use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub mod auth_sync;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub thumbnail: String,
    pub stream_url: String,
}

#[derive(Debug, Clone)]
pub struct CachedStream {
    pub playable_url: String,
    pub upstream_url: String,
    pub content_length: Option<u64>,
    pub mime_type: String,
    pub user_agent: String,
    pub expires_at: u64,
}

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static PLAYER_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static STREAM_CACHE: OnceLock<Mutex<HashMap<String, CachedStream>>> = OnceLock::new();
static VISITOR_DATA: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static CHUNK_PROXY_PORT: OnceLock<u16> = OnceLock::new();

const CHUNK_SIZE: u64 = 524_288; // 512 KB bounded slices

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_nodelay(true)
            .pool_max_idle_per_host(12)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_secs(4))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn player_client() -> &'static reqwest::Client {
    PLAYER_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_nodelay(true)
            .pool_max_idle_per_host(12)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_millis(2600))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn stream_cache() -> &'static Mutex<HashMap<String, CachedStream>> {
    STREAM_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn visitor_store() -> &'static Mutex<Option<String>> {
    VISITOR_DATA.get_or_init(|| Mutex::new(None))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Starts a lightweight localhost HTTP range-chunking proxy (`127.0.0.1:<port>/stream/<video_id>`).
/// Converts MPV's open-ended Range requests into bounded 512KB slices to bypass Google Video's 403 Forbidden firewall.
fn ensure_chunk_proxy_server() -> u16 {
    *CHUNK_PROXY_PORT.get_or_init(|| {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("Failed to bind local stream chunk proxy");
        let _ = std_listener.set_nonblocking(true);
        let port = std_listener
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(0);

        tauri::async_runtime::spawn(async move {
            let Ok(listener) = TcpListener::from_std(std_listener) else {
                return;
            };
            while let Ok((socket, _)) = listener.accept().await {
                tokio::spawn(handle_proxy_client(socket));
            }
        });

        crate::aethon_log!("Local bounded-range chunk proxy listening on 127.0.0.1:{}", port);
        port
    })
}

async fn handle_proxy_client(mut socket: tokio::net::TcpStream) {
    let _ = socket.set_nodelay(true);
    let mut buf = [0u8; 4096];
    let Ok(n) = socket.read(&mut buf).await else {
        return;
    };
    if n == 0 {
        return;
    }

    let req_str = String::from_utf8_lossy(&buf[..n]);
    let mut lines = req_str.lines();
    let Some(first_line) = lines.next() else {
        return;
    };
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1];
    let video_id = path
        .strip_prefix("/stream/")
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .trim();

    let Some(entry) = stream_cache().lock().get(video_id).cloned() else {
        let _ = socket
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return;
    };

    // Parse Range header from MPV (e.g., "Range: bytes=0-" or "Range: bytes=1048576-")
    let mut req_start: u64 = 0;
    let mut req_end_opt: Option<u64> = None;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("range:") {
            if let Some(bytes_part) = rest.trim().strip_prefix("bytes=") {
                let mut bounds = bytes_part.split('-');
                if let Some(s) = bounds.next().and_then(|v| v.trim().parse::<u64>().ok()) {
                    req_start = s;
                }
                if let Some(e) = bounds
                    .next()
                    .and_then(|v| v.trim().parse::<u64>().ok())
                {
                    req_end_opt = Some(e);
                }
            }
        }
    }

    let client = http_client();
    let first_chunk_end = match (req_end_opt, entry.content_length) {
        (Some(re), Some(total)) => re.min(req_start + CHUNK_SIZE - 1).min(total.saturating_sub(1)),
        (Some(re), None) => re.min(req_start + CHUNK_SIZE - 1),
        (None, Some(total)) => (req_start + CHUNK_SIZE - 1).min(total.saturating_sub(1)),
        (None, None) => req_start + CHUNK_SIZE - 1,
    };

    // Fetch first bounded chunk to verify upstream 206 & discover total length if not already known
    let Ok(first_resp) = client
        .get(&entry.upstream_url)
        .header("User-Agent", &entry.user_agent)
        .header("Range", format!("bytes={}-{}", req_start, first_chunk_end))
        .send()
        .await
    else {
        return;
    };

    if !first_resp.status().is_success() {
        let _ = socket
            .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return;
    }

    let total_len = entry
        .content_length
        .or_else(|| {
            first_resp
                .headers()
                .get("content-range")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.rsplit('/').next())
                .and_then(|t| t.trim().parse::<u64>().ok())
        })
        .unwrap_or(0);

    if total_len == 0 || req_start >= total_len {
        let _ = socket
            .write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return;
    }

    // Update cached content_length if newly discovered
    if entry.content_length.is_none() {
        if let Some(cached) = stream_cache().lock().get_mut(video_id) {
            cached.content_length = Some(total_len);
        }
    }

    let final_end = req_end_opt
        .map(|e| e.min(total_len - 1))
        .unwrap_or(total_len - 1);
    let content_len = final_end - req_start + 1;

    let resp_headers = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        entry.mime_type, req_start, final_end, total_len, content_len
    );

    if socket.write_all(resp_headers.as_bytes()).await.is_err() {
        return;
    }
    if method.eq_ignore_ascii_case("HEAD") {
        return;
    }

    // Write first bounded chunk payload to MPV immediately
    let Ok(first_bytes) = first_resp.bytes().await else {
        return;
    };
    if first_bytes.is_empty() || socket.write_all(&first_bytes).await.is_err() {
        return;
    }

    let mut cur_pos = req_start + (first_bytes.len() as u64);

    // Stream remaining bounded 512KB chunks seamlessly until final_end or MPV closes socket
    while cur_pos <= final_end {
        let chunk_end = (cur_pos + CHUNK_SIZE - 1).min(final_end);
        let Ok(resp) = client
            .get(&entry.upstream_url)
            .header("User-Agent", &entry.user_agent)
            .header("Range", format!("bytes={}-{}", cur_pos, chunk_end))
            .send()
            .await
        else {
            break;
        };

        if !resp.status().is_success() {
            break;
        }

        let Ok(bytes) = resp.bytes().await else {
            break;
        };
        if bytes.is_empty() {
            break;
        }
        if socket.write_all(&bytes).await.is_err() {
            break;
        }
        cur_pos += bytes.len() as u64;
    }
}

pub struct YouTubeService;

impl YouTubeService {
    pub fn extract_video_id(uri: &str) -> Option<String> {
        if let Some(idx) = uri.find("v=") {
            let rest = &uri[idx + 2..];
            let id: String = rest
                .chars()
                .take_while(|&c| c != '&' && c != '#' && c != '?')
                .collect();
            if id.len() == 11 {
                return Some(id);
            }
        }
        if let Some(idx) = uri.find("youtu.be/") {
            let rest = &uri[idx + 9..];
            let id: String = rest
                .chars()
                .take_while(|&c| c != '&' && c != '#' && c != '?')
                .collect();
            if id.len() == 11 {
                return Some(id);
            }
        }
        if let Some(idx) = uri.find("/stream/") {
            let rest = &uri[idx + 8..];
            let id: String = rest
                .chars()
                .take_while(|&c| c != '&' && c != '#' && c != '?')
                .collect();
            if id.len() == 11 {
                return Some(id);
            }
        }
        if uri.len() == 11
            && uri
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Some(uri.to_string());
        }
        None
    }

    pub fn invalidate_cached_stream(uri: &str) {
        if let Some(vid) = Self::extract_video_id(uri) {
            stream_cache().lock().remove(&vid);
        }
    }

    pub fn upgrade_thumbnail_url(url: &str, video_id: &str) -> String {
        if url.contains("lh3.googleusercontent.com") || url.contains("yt3.ggpht.com") {
            if let Some(eq_idx) = url.rfind('=') {
                let suffix = &url[eq_idx + 1..];
                if suffix.starts_with('w') || suffix.starts_with('s') {
                    return format!("{}=w544-h544-l90-rj", &url[..eq_idx]);
                }
            }
            return url.to_string();
        }
        if !video_id.is_empty() && (url.is_empty() || url.contains("default.jpg")) {
            return format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id);
        }
        url.to_string()
    }

    fn capture_visitor_data(json: &serde_json::Value) {
        if let Some(vd) = json
            .pointer("/responseContext/visitorData")
            .and_then(|v| v.as_str())
        {
            if !vd.is_empty() {
                *visitor_store().lock() = Some(vd.to_string());
            }
        }
    }

    async fn get_or_fetch_visitor_data() -> Option<String> {
        if let Some(existing) = visitor_store().lock().clone() {
            return Some(existing);
        }
        let client = player_client();
        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "hl": "en",
                    "gl": "IN"
                }
            }
        });
        if let Ok(resp) = client
            .post("https://music.youtube.com/youtubei/v1/visitor_id?prettyPrint=false")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                Self::capture_visitor_data(&json);
            }
        }
        visitor_store().lock().clone()
    }

    pub async fn search(query: &str, limit: usize, gl: &str) -> Vec<YouTubeTrack> {
        let max_results = limit.clamp(1, 50);
        let region = if gl.trim().is_empty() { "IN" } else { gl.trim() };

        if let Some(tracks) = Self::search_innertube(query, max_results, region).await {
            if !tracks.is_empty() {
                return tracks;
            }
        }

        let q = query.to_string();
        tokio::task::spawn_blocking(move || Self::search_ytdlp(&q, max_results))
            .await
            .unwrap_or_default()
    }

    pub async fn search_innertube(
        query: &str,
        limit: usize,
        gl: &str,
    ) -> Option<Vec<YouTubeTrack>> {
        let client = http_client();

        let songs_body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "gl": gl,
                    "hl": "en"
                }
            },
            "query": query,
            "params": "EgWKAQIIAWoSEAQQAxAJEAUQChAVEA4QERAQ"
        });

        let general_body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "gl": gl,
                    "hl": "en"
                }
            },
            "query": query
        });

        let songs_fut = client
            .post("https://music.youtube.com/youtubei/v1/search?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
            )
            .json(&songs_body)
            .send();

        let general_fut = client
            .post("https://music.youtube.com/youtubei/v1/search?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
            )
            .json(&general_body)
            .send();

        let (songs_res, general_res) = tokio::join!(songs_fut, general_fut);

        let mut results = Vec::new();

        // Extract general shelf first so `musicCardShelfRenderer` (Top Result) lands at index 0
        if let Ok(resp) = general_res {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    Self::capture_visitor_data(&json_data);
                    Self::extract_innertube_tracks(&json_data, &mut results, limit);
                }
            }
        }

        if results.len() < limit {
            if let Ok(resp) = songs_res {
                if resp.status().is_success() {
                    if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                        Self::capture_visitor_data(&json_data);
                        Self::extract_innertube_tracks(&json_data, &mut results, limit);
                    }
                }
            }
        }

        Some(results)
    }

    /// Fast ~250ms InnerTube `/youtubei/v1/player` extraction + bounded-range verification
    async fn resolve_innertube_bounded_stream(video_id: &str) -> Option<CachedStream> {
        let client = player_client();
        let visitor_data = Self::get_or_fetch_visitor_data().await;

        let vr_ua = "com.google.android.apps.youtube.vr.oculus/1.60.19 (Linux; U; Android 14; en_US; Quest 3; Build/UP1A.231005.007.A1; Cronet/132.0.6808.3)";
        let ts_ua = "com.google.android.youtube/1.9 (Linux; U; Android 14; en_US)";
        let ios_ua = "com.google.ios.youtube/20.03.02 (iPhone16,2; U; CPU iOS 18_2_1 like Mac OS X;)";

        let mut vr_client = serde_json::json!({
            "clientName": "ANDROID_VR",
            "clientVersion": "1.60.19",
            "androidSdkVersion": 34,
            "osName": "Android",
            "osVersion": "14",
            "deviceMake": "Oculus",
            "deviceModel": "Quest 3",
            "hl": "en",
            "gl": "IN"
        });
        let mut ts_client = serde_json::json!({
            "clientName": "ANDROID_TESTSUITE",
            "clientVersion": "1.9",
            "androidSdkVersion": 34,
            "osName": "Android",
            "osVersion": "14",
            "hl": "en",
            "gl": "IN"
        });
        let mut ios_client = serde_json::json!({
            "clientName": "IOS",
            "clientVersion": "20.03.02",
            "deviceMake": "Apple",
            "deviceModel": "iPhone16,2",
            "osName": "iPhone",
            "osVersion": "18.2.1.22C161",
            "hl": "en",
            "gl": "IN"
        });

        if let Some(ref vd) = visitor_data {
            vr_client["visitorData"] = serde_json::Value::String(vd.clone());
            ts_client["visitorData"] = serde_json::Value::String(vd.clone());
            ios_client["visitorData"] = serde_json::Value::String(vd.clone());
        }

        let vr_body = serde_json::json!({
            "context": { "client": vr_client },
            "videoId": video_id,
            "contentCheckOk": true,
            "racyCheckOk": true
        });
        let ts_body = serde_json::json!({
            "context": { "client": ts_client },
            "videoId": video_id,
            "contentCheckOk": true,
            "racyCheckOk": true
        });
        let ios_body = serde_json::json!({
            "context": { "client": ios_client },
            "videoId": video_id,
            "contentCheckOk": true,
            "racyCheckOk": true
        });

        let vr_fut = client
            .post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header("X-Goog-Api-Format-Version", "2")
            .header("User-Agent", vr_ua)
            .json(&vr_body)
            .send();

        let ts_fut = client
            .post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header("X-Goog-Api-Format-Version", "2")
            .header("User-Agent", ts_ua)
            .json(&ts_body)
            .send();

        let ios_fut = client
            .post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header("X-Goog-Api-Format-Version", "2")
            .header("User-Agent", ios_ua)
            .json(&ios_body)
            .send();

        let (vr_res, ts_res, ios_res) = tokio::join!(vr_fut, ts_fut, ios_fut);

        for (resp_res, ua) in [(vr_res, vr_ua), (ts_res, ts_ua), (ios_res, ios_ua)] {
            if let Ok(resp) = resp_res {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        Self::capture_visitor_data(&json);
                        if let Some((upstream_url, content_length, mime_type)) =
                            Self::pick_best_audio_format(&json)
                        {
                            let port = ensure_chunk_proxy_server();
                            let playable_url =
                                format!("http://127.0.0.1:{}/stream/{}", port, video_id);
                            return Some(CachedStream {
                                playable_url,
                                upstream_url,
                                content_length,
                                mime_type,
                                user_agent: ua.to_string(),
                                expires_at: now_unix() + 14_400,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn pick_best_audio_format(
        player_json: &serde_json::Value,
    ) -> Option<(String, Option<u64>, String)> {
        let status = player_json
            .pointer("/playabilityStatus/status")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        if status != "OK" {
            return None;
        }

        let formats = player_json
            .pointer("/streamingData/adaptiveFormats")
            .and_then(|f| f.as_array())?;

        let mut best: Option<(String, Option<u64>, String)> = None;
        let mut best_score: i64 = -1;

        for fmt in formats {
            let mime_raw = fmt
                .get("mimeType")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            if !mime_raw.starts_with("audio/") {
                continue;
            }
            let Some(url) = fmt.get("url").and_then(|u| u.as_str()) else {
                continue;
            };
            let itag = fmt.get("itag").and_then(|i| i.as_i64()).unwrap_or(0);
            let bitrate = fmt.get("bitrate").and_then(|b| b.as_i64()).unwrap_or(0);
            let content_len = fmt
                .get("contentLength")
                .and_then(|c| c.as_str())
                .and_then(|s| s.parse::<u64>().ok());
            let clean_mime = mime_raw
                .split(';')
                .next()
                .unwrap_or("audio/webm")
                .trim()
                .to_string();

            let bonus = match itag {
                251 => 500_000,
                140 => 350_000,
                250 => 200_000,
                _ => 0,
            };
            let score = bitrate + bonus;
            if score > best_score {
                best_score = score;
                best = Some((url.to_string(), content_len, clean_mime));
            }
        }

        best
    }

    pub async fn resolve_playable_stream_async(
        uri: &str,
        title: &str,
        artist: &str,
    ) -> String {
        if !uri.contains("youtube.com/watch?v=") && !uri.contains("youtu.be/") {
            return uri.to_string();
        }

        let video_id = Self::extract_video_id(uri).unwrap_or_default();

        if !video_id.is_empty() {
            let now = now_unix();
            if let Some(cached) = stream_cache().lock().get(&video_id) {
                if cached.expires_at > now {
                    crate::aethon_log!("Stream cache HIT for video_id={}", video_id);
                    return cached.playable_url.clone();
                }
            }

            // Tier 1: ~0.25s InnerTube ANDROID_VR / ANDROID_TESTSUITE / IOS + Local Bounded-Range Chunk Proxy
            if let Some(entry) = Self::resolve_innertube_bounded_stream(&video_id).await {
                let playable = entry.playable_url.clone();
                stream_cache().lock().insert(video_id.clone(), entry);
                crate::aethon_log!(
                    "Resolved sub-second bounded-range stream via local proxy for video_id={}",
                    video_id
                );
                return playable;
            }
        }

        // Tier 2 Fallback: `yt-dlp --no-config -g` in blocking thread pool
        let uri_owned = uri.to_string();
        let title_owned = title.to_string();
        let artist_owned = artist.to_string();
        let vid_owned = video_id.clone();

        tokio::task::spawn_blocking(move || {
            let resolved = Self::resolve_via_ytdlp_direct(&uri_owned, &title_owned, &artist_owned);
            if !vid_owned.is_empty() && resolved.contains("googlevideo.com") {
                let port = ensure_chunk_proxy_server();
                let proxy_url = format!("http://127.0.0.1:{}/stream/{}", port, vid_owned);
                stream_cache().lock().insert(
                    vid_owned,
                    CachedStream {
                        playable_url: proxy_url.clone(),
                        upstream_url: resolved,
                        content_length: None,
                        mime_type: "audio/webm".to_string(),
                        user_agent: "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0".to_string(),
                        expires_at: now_unix() + 14_400,
                    },
                );
                return proxy_url;
            }
            resolved
        })
        .await
        .unwrap_or_else(|_| uri.to_string())
    }

    pub fn resolve_via_ytdlp_direct(uri: &str, title: &str, artist: &str) -> String {
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--no-config")
            .arg("--get-url")
            .arg("--no-playlist")
            .arg("--no-warnings")
            .arg("--no-check-certificates")
            .arg("--socket-timeout")
            .arg("5")
            .arg("-f")
            .arg("251/140/bestaudio[ext=webm]/bestaudio/best")
            .arg("--extractor-args")
            .arg("youtube:player_client=tv,android_vr,ios;player_skip=webpage,configs")
            .arg("--")
            .arg(uri);
        crate::sanitize_host_command(&mut cmd);

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                if let Some(first_url) = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .find(|l| l.starts_with("http"))
                {
                    return first_url.to_string();
                }
            }
        }

        let query = format!("{} {} audio", title, artist).trim().to_string();
        if !query.is_empty() {
            let mut alt_cmd = Command::new("yt-dlp");
            alt_cmd
                .arg("--no-config")
                .arg("--get-url")
                .arg("--no-playlist")
                .arg("--no-warnings")
                .arg("--no-check-certificates")
                .arg("--socket-timeout")
                .arg("5")
                .arg("-f")
                .arg("251/140/bestaudio/best")
                .arg("--extractor-args")
                .arg("youtube:player_client=tv,android_vr;player_skip=webpage,configs")
                .arg("--")
                .arg(format!("ytsearch1:{}", query));
            crate::sanitize_host_command(&mut alt_cmd);

            if let Ok(alt_out) = alt_cmd.output() {
                if alt_out.status.success() {
                    if let Some(first_url) = String::from_utf8_lossy(&alt_out.stdout)
                        .lines()
                        .map(str::trim)
                        .find(|l| l.starts_with("http"))
                    {
                        return first_url.to_string();
                    }
                }
            }
        }

        uri.to_string()
    }

    pub fn parse_duration_text(text: &str) -> Option<f64> {
        let trimmed = text.trim();
        if !trimmed.contains(':') || !trimmed.chars().all(|c| c.is_ascii_digit() || c == ':') {
            return None;
        }
        let parts: Vec<&str> = trimmed.split(':').collect();
        match parts.len() {
            2 => {
                let m = parts[0].parse::<f64>().ok()?;
                let s = parts[1].parse::<f64>().ok()?;
                Some(m * 60.0 + s)
            }
            3 => {
                let h = parts[0].parse::<f64>().ok()?;
                let m = parts[1].parse::<f64>().ok()?;
                let s = parts[2].parse::<f64>().ok()?;
                Some(h * 3600.0 + m * 60.0 + s)
            }
            _ => None,
        }
    }

    fn is_metadata_noise_run(text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty()
            || trimmed == "•"
            || trimmed == "·"
            || trimmed == "|"
            || trimmed == "-"
        {
            return true;
        }
        let lower = trimmed.to_lowercase();
        if matches!(
            lower.as_str(),
            "song"
                | "video"
                | "artist"
                | "album"
                | "single"
                | "ep"
                | "station"
                | "playlist"
                | "chart"
                | "episode"
                | "podcast"
        ) {
            return true;
        }
        if trimmed.contains(':') && trimmed.chars().all(|c| c.is_ascii_digit() || c == ':') {
            return true;
        }
        if trimmed.len() == 4
            && (trimmed.starts_with("19") || trimmed.starts_with("20"))
            && trimmed.chars().all(|c| c.is_ascii_digit())
        {
            return true;
        }
        if lower.ends_with("views")
            || lower.ends_with("plays")
            || lower.ends_with("subscribers")
            || lower.ends_with("likes")
            || lower.contains("views ")
            || lower.contains("plays ")
        {
            return true;
        }
        false
    }

    pub fn strip_artist_metadata_suffixes(raw: &str) -> String {
        let mut s = raw.trim().to_string();
        for sep in [" • ", " · ", " | "] {
            if let Some(parts) = s.split(sep).next() {
                s = parts.trim().to_string();
            }
        }
        s
    }

    fn extract_innertube_tracks(
        value: &serde_json::Value,
        results: &mut Vec<YouTubeTrack>,
        limit: usize,
    ) {
        if results.len() >= limit {
            return;
        }

        if let Some(map) = value.as_object() {
            if let Some(item) = map.get("musicResponsiveListItemRenderer") {
                let mut video_id = item
                    .pointer("/playlistItemData/videoId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let mut title = String::new();
                let mut artist = String::from("YouTube Music");
                let mut duration = 0.0;

                if let Some(flex_cols) = item.get("flexColumns").and_then(|c| c.as_array()) {
                    if let Some(col0) = flex_cols.first() {
                        if let Some(runs) = col0
                            .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
                            .and_then(|r| r.as_array())
                        {
                            if let Some(run0) = runs.first() {
                                title = run0
                                    .get("text")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if video_id.is_empty() {
                                    if let Some(vid) = run0
                                        .pointer("/navigationEndpoint/watchEndpoint/videoId")
                                        .and_then(|s| s.as_str())
                                    {
                                        video_id = vid.to_string();
                                    }
                                }
                            }
                        }
                    }

                    if flex_cols.len() > 1 {
                        if let Some(runs) = flex_cols[1]
                            .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
                            .and_then(|r| r.as_array())
                        {
                            let mut artist_parts = Vec::new();
                            for r in runs {
                                if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                                    if let Some(d) = Self::parse_duration_text(t) {
                                        duration = d;
                                    } else if !Self::is_metadata_noise_run(t) {
                                        artist_parts.push(t.trim());
                                    }
                                }
                            }
                            if !artist_parts.is_empty() {
                                artist =
                                    Self::strip_artist_metadata_suffixes(&artist_parts.join(" "))
                                        .replace(" ,", ",");
                            }
                        }
                    }
                }

                if duration == 0.0 {
                    if let Some(fixed_cols) = item.get("fixedColumns").and_then(|c| c.as_array()) {
                        for col in fixed_cols {
                            if let Some(runs) = col
                                .pointer("/musicResponsiveListItemFixedColumnRenderer/text/runs")
                                .and_then(|r| r.as_array())
                            {
                                for r in runs {
                                    if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                                        if let Some(d) = Self::parse_duration_text(t) {
                                            duration = d;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !video_id.is_empty() && !title.is_empty() {
                    let raw_thumb = item
                        .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                        .and_then(|th| th.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|t| t.get("url"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("");

                    let thumbnail = Self::upgrade_thumbnail_url(raw_thumb, &video_id);
                    let stream_url = format!("https://www.youtube.com/watch?v={}", video_id);

                    if !results.iter().any(|r| r.id == video_id) {
                        results.push(YouTubeTrack {
                            id: video_id,
                            title,
                            artist,
                            duration,
                            thumbnail,
                            stream_url,
                        });
                    }
                }
            }

            for v in map.values() {
                Self::extract_innertube_tracks(v, results, limit);
            }
        } else if let Some(arr) = value.as_array() {
            for v in arr {
                Self::extract_innertube_tracks(v, results, limit);
            }
        }
    }

    pub fn search_ytdlp(query: &str, limit: usize) -> Vec<YouTubeTrack> {
        let max_results = limit.clamp(1, 50);
        let search_query = format!("ytsearch{}:{}", max_results, query);

        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--no-config")
            .arg("--flat-playlist")
            .arg("--print")
            .arg("%(id)s||%(title)s||%(uploader)s||%(duration)s")
            .arg("--")
            .arg(&search_query);

        crate::sanitize_host_command(&mut cmd);

        let output = match cmd.output() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        if !output.status.success() {
            return Vec::new();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split("||").collect();
            if parts.len() >= 3 {
                let id = parts[0].trim().to_string();
                let title = parts[1].trim().to_string();
                let artist = parts[2].trim().to_string();
                let duration = parts
                    .get(3)
                    .and_then(|d| d.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);

                let thumbnail = format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", id);
                let stream_url = format!("https://www.youtube.com/watch?v={}", id);

                if !id.is_empty() && id != "NA" {
                    results.push(YouTubeTrack {
                        id,
                        title,
                        artist,
                        duration,
                        thumbnail,
                        stream_url,
                    });
                }
            }
        }

        results
    }
}
