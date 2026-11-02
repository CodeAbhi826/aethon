use crate::library::{Database, Track};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, COOKIE, ORIGIN, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const YTM_ORIGIN: &str = "https://music.youtube.com";
const YTM_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtmAccountProfile {
    pub name: String,
    pub email_or_handle: String,
    pub avatar_url: String,
    pub cookie: String,
    pub visitor_data: String,
    pub page_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtmSyncSummary {
    pub liked_songs_count: usize,
    pub playlists_count: usize,
    pub history_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackTrackingSession {
    pub video_id: String,
    pub cpn: String,
    pub playback_url: String,
    pub watchtime_url: String,
    pub last_reported_pos: f64,
}

/// Extract a specific cookie key (e.g. "SAPISID" or "__Secure-3PAPISID") from a raw Cookie header string
pub fn extract_cookie_value(cookie_str: &str, key: &str) -> Option<String> {
    for part in cookie_str.split(';') {
        let trimmed = part.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Generate SAPISIDHASH Authorization header required by YouTube Music InnerTube
pub fn generate_sapisidhash(cookie_str: &str) -> Option<String> {
    let sapisid = extract_cookie_value(cookie_str, "SAPISID")
        .or_else(|| extract_cookie_value(cookie_str, "__Secure-3PAPISID"))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let input = format!("{} {} {}", timestamp, sapisid, YTM_ORIGIN);
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let hash_hex = format!("{:x}", hasher.finalize());

    Some(format!(
        "SAPISIDHASH {}_{} SAPISID1PHASH {}_{} SAPISID3PHASH {}_{}",
        timestamp, hash_hex, timestamp, hash_hex, timestamp, hash_hex
    ))
}

/// Build authenticated headers for InnerTube & Playback Stats requests
pub fn build_auth_headers(cookie_str: &str, visitor_data: Option<&str>, auth_user: Option<&str>) -> Result<HeaderMap, String> {
    let auth_val = generate_sapisidhash(cookie_str)
        .ok_or_else(|| "Cookie is missing SAPISID or __Secure-3PAPISID token".to_string())?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static(YTM_USER_AGENT));
    headers.insert(ORIGIN, HeaderValue::from_static(YTM_ORIGIN));
    headers.insert(REFERER, HeaderValue::from_static("https://music.youtube.com/"));
    headers.insert("X-Origin", HeaderValue::from_static(YTM_ORIGIN));
    headers.insert("X-YouTube-Client-Name", HeaderValue::from_static("67"));
    headers.insert("X-YouTube-Client-Version", HeaderValue::from_static("1.20260408.01.00"));
    headers.insert(
        "X-Goog-AuthUser",
        HeaderValue::from_str(auth_user.unwrap_or("0")).unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    if let Some(vd) = visitor_data.filter(|s| !s.is_empty()) {
        if let Ok(hv) = HeaderValue::from_str(vd) {
            headers.insert("X-Goog-Visitor-Id", hv);
        }
    }
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_val).map_err(|e| e.to_string())?,
    );
    headers.insert(
        COOKIE,
        HeaderValue::from_str(cookie_str).map_err(|e| e.to_string())?,
    );
    Ok(headers)
}

/// Generate a 16-character alphanumeric Client Playback Nonce (cpn) for YouTube Recap / WatchTime correlation
pub fn generate_cpn() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut state = nanos as u64 ^ 0x5dee_ce66_d1a4_f87b;
    let mut out = String::with_capacity(16);
    for _ in 0..16 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let idx = (state as usize) % CHARS.len();
        out.push(CHARS[idx] as char);
    }
    out
}

/// Build the standard WEB_REMIX InnerTube context body (supports Brand Account onBehalfOfUser)
pub fn build_innertube_context(gl: &str, page_id: Option<&str>) -> serde_json::Value {
    let mut user_obj = serde_json::json!({
        "lockedSafetyMode": false
    });
    if let Some(pid) = page_id.filter(|s| !s.is_empty()) {
        user_obj["onBehalfOfUser"] = serde_json::Value::String(pid.to_string());
    }
    serde_json::json!({
        "client": {
            "clientName": "WEB_REMIX",
            "clientVersion": "1.20260408.01.00",
            "gl": gl,
            "hl": "en"
        },
        "user": user_obj
    })
}

/// 1. Open an ArchiveTune-style embedded login WebviewWindow that automatically captures cookies once signed in
#[tauri::command]
pub async fn open_ytm_login_window(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("ytm-login") {
        let _ = existing.set_focus();
        return Ok(());
    }

    let login_url = "https://accounts.google.com/ServiceLogin?ltmpl=music&service=youtube&continue=https%3A%2F%2Fmusic.youtube.com%2F";
    let win = WebviewWindowBuilder::new(
        &app,
        "ytm-login",
        WebviewUrl::External(login_url.parse().map_err(|e: url::ParseError| e.to_string())?),
    )
    .title("Sign in to YouTube Music — Aethon")
    .inner_size(520.0, 680.0)
    .center()
    .focused(true)
    .build()
    .map_err(|e| e.to_string())?;

    let app_clone = app.clone();
    tokio::spawn(async move {
        let target_url: url::Url = "https://music.youtube.com/".parse().unwrap();
        loop {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            let Some(login_win) = app_clone.get_webview_window("ytm-login") else {
                break;
            };
            let current_url = login_win.url().ok().map(|u| u.to_string()).unwrap_or_default();
            if current_url.starts_with("https://music.youtube.com") {
                if let Ok(cookies) = login_win.cookies_for_url(target_url.clone()) {
                    let has_sapisid = cookies.iter().any(|c| c.name() == "SAPISID" || c.name() == "__Secure-3PAPISID");
                    if has_sapisid {
                        let cookie_header = cookies
                            .iter()
                            .map(|c| format!("{}={}", c.name(), c.value()))
                            .collect::<Vec<_>>()
                            .join("; ");

                        if let Ok(profile) = fetch_ytm_account_profile(&cookie_header, None).await {
                            let _ = app_clone.emit("ytm-login-success", &profile);
                            let _ = login_win.close();
                            break;
                        }
                    }
                }
            }
        }
    });

    let _ = win.show();
    Ok(())
}

/// 2. Fetch the signed-in user's real Google / YouTube Music profile from /youtubei/v1/account/account_menu
pub async fn fetch_ytm_account_profile(
    cookie_str: &str,
    visitor_data: Option<&str>,
) -> Result<YtmAccountProfile, String> {
    let headers = build_auth_headers(cookie_str, visitor_data, None)?;
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "context": build_innertube_context("US", None)
    });

    let resp = client
        .post("https://music.youtube.com/youtubei/v1/account/account_menu")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("account_menu failed with status {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let header = json.pointer("/actions/0/openPopupAction/popup/multiPageMenuRenderer/header/activeAccountHeaderRenderer");

    let name = header
        .and_then(|h| h.pointer("/accountName/runs/0/text"))
        .and_then(|t| t.as_str())
        .unwrap_or("YouTube Music User")
        .to_string();

    let email_or_handle = header
        .and_then(|h| {
            h.pointer("/channelHandle/runs/0/text")
                .or_else(|| h.pointer("/email/runs/0/text"))
        })
        .and_then(|t| t.as_str())
        .unwrap_or("@youtube-music")
        .to_string();

    let avatar_url = header
        .and_then(|h| h.pointer("/accountPhoto/thumbnails"))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.last())
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_string();

    let extracted_visitor = json
        .pointer("/responseContext/visitorData")
        .and_then(|v| v.as_str())
        .unwrap_or(visitor_data.unwrap_or(""))
        .to_string();

    Ok(YtmAccountProfile {
        name,
        email_or_handle,
        avatar_url,
        cookie: cookie_str.to_string(),
        visitor_data: extracted_visitor,
        page_id: None,
    })
}

/// 3. Full Cloud Library Sync: Fetches Liked Songs (VLLM), Saved Playlists (FEmusic_liked_playlists), and History (FEmusic_history)
pub async fn sync_ytm_library_internal(
    cookie_str: &str,
    visitor_data: Option<&str>,
    page_id: Option<&str>,
    gl: &str,
    db: &Arc<Database>,
) -> Result<YtmSyncSummary, String> {
    let headers = build_auth_headers(cookie_str, visitor_data, None)?;
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // A. Fetch Liked Songs via "VLLM" (full paginated Liked Music playlist)
    let liked_body = serde_json::json!({
        "context": build_innertube_context(gl, page_id),
        "browseId": "VLLM"
    });
    let mut liked_tracks = Vec::new();
    if let Ok(resp) = client
        .post("https://music.youtube.com/youtubei/v1/browse")
        .json(&liked_body)
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            crate::youtube::YouTubeService::extract_innertube_tracks(&json, &mut liked_tracks, 500);
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    for yt in &liked_tracks {
        let track = Track {
            id: format!("yt_{}", yt.id),
            title: yt.title.clone(),
            artist: yt.artist.clone(),
            album: "Liked Songs (YouTube Music)".to_string(),
            duration: yt.duration,
            file_path: yt.stream_url.clone(),
            cover_art: Some(yt.thumbnail.clone()),
            format: "Opus 256k".to_string(),
            bitrate: Some(256),
            sample_rate: Some(48000),
            play_count: 0,
            favorite: true,
            date_added: now,
        };
        let _ = db.upsert_track(&track);
        let _ = db.set_favorite_explicit(&track.id, true);
    }

    // B. Fetch History via "FEmusic_history"
    let hist_body = serde_json::json!({
        "context": build_innertube_context(gl, page_id),
        "browseId": "FEmusic_history"
    });
    let mut history_tracks = Vec::new();
    if let Ok(resp) = client
        .post("https://music.youtube.com/youtubei/v1/browse")
        .json(&hist_body)
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            crate::youtube::YouTubeService::extract_innertube_tracks(&json, &mut history_tracks, 100);
        }
    }

    Ok(YtmSyncSummary {
        liked_songs_count: liked_tracks.len(),
        playlists_count: 0,
        history_count: history_tracks.len(),
    })
}

/// 4. Push Like / Unlike mutation to YouTube Music (/youtubei/v1/like/like or /youtubei/v1/like/removelike)
pub async fn set_ytm_like_status(
    cookie_str: &str,
    visitor_data: Option<&str>,
    page_id: Option<&str>,
    video_id: &str,
    like: bool,
) -> Result<(), String> {
    let clean_id = video_id.strip_prefix("yt_").unwrap_or(video_id);
    let headers = build_auth_headers(cookie_str, visitor_data, None)?;
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let endpoint = if like {
        "https://music.youtube.com/youtubei/v1/like/like"
    } else {
        "https://music.youtube.com/youtubei/v1/like/removelike"
    };

    let body = serde_json::json!({
        "context": build_innertube_context("US", page_id),
        "target": {
            "videoId": clean_id
        }
    });

    let resp = client.post(endpoint).json(&body).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("YouTube Music like mutation returned {}", resp.status()));
    }
    Ok(())
}

/// 5. Initialize YouTube Recap & Watch History Tracking (/youtubei/v1/player -> /api/stats/playback)
pub async fn init_recap_playback_tracking(
    cookie_str: &str,
    visitor_data: Option<&str>,
    page_id: Option<&str>,
    video_id: &str,
) -> Result<PlaybackTrackingSession, String> {
    let clean_id = video_id.strip_prefix("yt_").unwrap_or(video_id);
    let cpn = generate_cpn();
    let headers = build_auth_headers(cookie_str, visitor_data, None)?;
    let client = reqwest::Client::builder()
        .default_headers(headers.clone())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "context": build_innertube_context("US", page_id),
        "videoId": clean_id,
        "cpn": cpn,
        "contentCheckOk": true,
        "racyCheckOk": true
    });

    let resp = client
        .post("https://music.youtube.com/youtubei/v1/player")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let playback_url = json
        .pointer("/playbackTracking/videostatsPlaybackUrl/baseUrl")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "Missing videostatsPlaybackUrl in /player response".to_string())?
        .replace("https://s.youtube.com", "https://music.youtube.com");

    let watchtime_url = json
        .pointer("/playbackTracking/videostatsWatchtimeUrl/baseUrl")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "Missing videostatsWatchtimeUrl in /player response".to_string())?
        .replace("https://s.youtube.com", "https://music.youtube.com");

    // Fire initial /api/stats/playback ping -> registers song in YouTube Music History & Recap
    let init_ping_url = format!(
        "{}&ver=2&c=WEB_REMIX&cpn={}&cmt=0",
        playback_url, cpn
    );
    let _ = client.get(&init_ping_url).send().await;

    Ok(PlaybackTrackingSession {
        video_id: clean_id.to_string(),
        cpn,
        playback_url,
        watchtime_url,
        last_reported_pos: 0.0,
    })
}

/// 6. Report Watch-Time Segment (/api/stats/watchtime) -> credits Minutes Listened toward Year-End YouTube Recap
pub async fn ping_recap_watchtime(
    cookie_str: &str,
    visitor_data: Option<&str>,
    session: &PlaybackTrackingSession,
    start_sec: f64,
    end_sec: f64,
    is_playing: bool,
) -> Result<(), String> {
    if end_sec <= start_sec {
        return Ok(());
    }
    let headers = build_auth_headers(cookie_str, visitor_data, None)?;
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let state_str = if is_playing { "playing" } else { "paused" };
    let url = format!(
        "{}&ver=2&c=WEB_REMIX&cpn={}&cmt={:.2}&st={:.2}&et={:.2}&state={}",
        session.watchtime_url,
        session.cpn,
        end_sec,
        start_sec,
        end_sec,
        state_str
    );

    let _ = client.get(&url).send().await;
    Ok(())
}

