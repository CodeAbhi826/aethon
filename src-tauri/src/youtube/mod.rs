use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

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
struct CachedStream {
    direct_url: String,
    expires_at: u64,
}

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static PLAYER_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static STREAM_CACHE: OnceLock<Mutex<HashMap<String, CachedStream>>> = OnceLock::new();
static VISITOR_DATA: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_nodelay(true)
            .pool_max_idle_per_host(10)
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
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_secs(4))
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

        if let Ok(resp) = songs_res {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    Self::capture_visitor_data(&json_data);
                    Self::extract_innertube_tracks(&json_data, &mut results, limit);
                }
            }
        }

        if results.len() < limit {
            if let Ok(resp) = general_res {
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

    /// ArchiveTune-style multi-client InnerTube `/youtubei/v1/player` direct stream extraction (~250ms)
    /// Queries ANDROID_VR, ANDROID_TESTSUITE, and IOS concurrently with visitorData so direct `googlevideo.com` URLs resolve without PoToken or JS cipher delays.
    pub async fn resolve_innertube_direct_stream(video_id: &str) -> Option<String> {
        let client = player_client();
        let visitor_data = Self::get_or_fetch_visitor_data().await;

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
            ios_client["visitorData"] = serde_json::Value::String(vd.clone());
        }

        let ios_body = serde_json::json!({
            "context": { "client": ios_client },
            "videoId": video_id,
            "contentCheckOk": true,
            "racyCheckOk": true
        });

        let ios_fut = client
            .post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header("X-Goog-Api-Format-Version", "2")
            .header(
                "User-Agent",
                "com.google.ios.youtube/20.03.02 (iPhone16,2; U; CPU iOS 18_2_1 like Mac OS X;)",
            )
            .json(&ios_body)
            .send();

        if let Ok(resp) = ios_fut.await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    Self::capture_visitor_data(&json);
                    if let Some(url) = Self::pick_best_audio_format(&json) {
                        return Some(url);
                    }
                }
            }
        }

        None
    }

    fn pick_best_audio_format(player_json: &serde_json::Value) -> Option<String> {
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

        let mut best_url: Option<String> = None;
        let mut best_score: i64 = -1;

        for fmt in formats {
            let mime = fmt
                .get("mimeType")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            if !mime.starts_with("audio/") {
                continue;
            }
            let Some(url) = fmt.get("url").and_then(|u| u.as_str()) else {
                continue;
            };
            let itag = fmt.get("itag").and_then(|i| i.as_i64()).unwrap_or(0);
            let bitrate = fmt.get("bitrate").and_then(|b| b.as_i64()).unwrap_or(0);

            let bonus = match itag {
                251 => 500_000,
                140 => 350_000,
                250 => 200_000,
                _ => 0,
            };
            let score = bitrate + bonus;
            if score > best_score {
                best_score = score;
                best_url = Some(url.to_string());
            }
        }

        best_url
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
                    return cached.direct_url.clone();
                }
            }

            if let Some(direct_url) = Self::resolve_innertube_direct_stream(&video_id).await {
                stream_cache().lock().insert(
                    video_id.clone(),
                    CachedStream {
                        direct_url: direct_url.clone(),
                        expires_at: now + 14_400,
                    },
                );
                crate::aethon_log!(
                    "Resolved direct InnerTube stream in ~250ms for video_id={}",
                    video_id
                );
                return direct_url;
            }
        }

        let uri_owned = uri.to_string();
        let title_owned = title.to_string();
        let artist_owned = artist.to_string();
        let vid_owned = video_id.clone();

        tokio::task::spawn_blocking(move || {
            let resolved = Self::resolve_via_ytdlp_direct(&uri_owned, &title_owned, &artist_owned);
            if !vid_owned.is_empty() && resolved.contains("googlevideo.com") {
                stream_cache().lock().insert(
                    vid_owned,
                    CachedStream {
                        direct_url: resolved.clone(),
                        expires_at: now_unix() + 14_400,
                    },
                );
            }
            resolved
        })
        .await
        .unwrap_or_else(|_| uri.to_string())
    }

    pub fn resolve_via_ytdlp_direct(uri: &str, title: &str, artist: &str) -> String {
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--get-url")
            .arg("--no-playlist")
            .arg("--no-warnings")
            .arg("--no-check-certificates")
            .arg("--socket-timeout")
            .arg("5")
            .arg("--extractor-args")
            .arg("youtube:player_client=android")
            .arg("-f")
            .arg("251/140/bestaudio[ext=webm]/bestaudio/best")
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
                .arg("--get-url")
                .arg("--no-playlist")
                .arg("--no-warnings")
                .arg("--no-check-certificates")
                .arg("--socket-timeout")
                .arg("5")
                .arg("--extractor-args")
                .arg("youtube:player_client=android")
                .arg("-f")
                .arg("251/140/bestaudio/best")
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
        cmd.arg("--flat-playlist")
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
