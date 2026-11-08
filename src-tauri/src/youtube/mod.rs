use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Notify;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeShelf {
    pub title: String,
    pub tracks: Vec<YouTubeTrack>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CachedTrackMeta {
    video_id: String,
    content_length: u64,
    mime_type: String,
    ext: String,
    created_at: u64,
}

#[derive(Debug)]
pub struct StreamDownloadTask {
    pub video_id: String,
    pub upstream_url: Mutex<String>,
    pub user_agent: Mutex<String>,
    pub mime_type: String,
    pub content_length: AtomicU64,
    pub downloaded_bytes: AtomicU64,
    pub is_complete: AtomicBool,
    pub notify: Arc<Notify>,
    pub error: Mutex<Option<String>>,
}

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static PLAYER_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static STREAM_CACHE: OnceLock<Mutex<HashMap<String, CachedStream>>> = OnceLock::new();
static WEB_VISITOR_DATA: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static VR_VISITOR_DATA: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static CHUNK_PROXY_PORT: OnceLock<u16> = OnceLock::new();
static ACTIVE_TASKS: OnceLock<Mutex<HashMap<String, Arc<StreamDownloadTask>>>> = OnceLock::new();

static CACHE_ENABLED: AtomicBool = AtomicBool::new(true);
static MAX_CACHE_SIZE_BYTES: AtomicU64 = AtomicU64::new(10 * 1024 * 1024 * 1024); // 10 GB default

const CHUNK_SIZE: u64 = 524_288; // 512 KB bounded slices

pub fn configure_stream_cache(enabled: bool, max_size_gb: f64) {
    CACHE_ENABLED.store(enabled, Ordering::Relaxed);
    let bytes = (max_size_gb * 1024.0 * 1024.0 * 1024.0)
        .clamp(512.0 * 1024.0 * 1024.0, 500.0 * 1024.0 * 1024.0 * 1024.0) as u64;
    MAX_CACHE_SIZE_BYTES.store(bytes, Ordering::Relaxed);
    crate::aethon_log!(
        "Stream cache configured: enabled={}, max_gb={:.2}",
        enabled,
        max_size_gb
    );
}

fn cache_dir() -> PathBuf {
    let p = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aethon")
        .join("streams");
    let _ = fs::create_dir_all(&p);
    p
}

fn active_tasks() -> &'static Mutex<HashMap<String, Arc<StreamDownloadTask>>> {
    ACTIVE_TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_nodelay(true)
            .pool_max_idle_per_host(16)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .connect_timeout(std::time::Duration::from_millis(3500))
            .timeout(std::time::Duration::from_secs(12))
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
            .connect_timeout(std::time::Duration::from_millis(1800))
            .timeout(std::time::Duration::from_millis(2400))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn stream_cache() -> &'static Mutex<HashMap<String, CachedStream>> {
    STREAM_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn web_visitor_store() -> &'static Mutex<Option<String>> {
    WEB_VISITOR_DATA.get_or_init(|| Mutex::new(None))
}

fn vr_visitor_store() -> &'static Mutex<Option<String>> {
    VR_VISITOR_DATA.get_or_init(|| Mutex::new(None))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Extract `clen=<bytes>` directly from a `googlevideo.com` URL query string in 0 ms
pub fn extract_clen_from_url(url: &str) -> Option<u64> {
    for sep in ["&clen=", "?clen=", "/clen/"] {
        if let Some(idx) = url.find(sep) {
            let rest = &url[idx + sep.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(val) = digits.parse::<u64>() {
                if val > 0 {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Match the required `User-Agent` to the `c=` client parameter inside a `googlevideo.com` URL
pub fn extract_user_agent_for_url(url: &str, default_ua: &str) -> String {
    if url.contains("c=ANDROID_VR") {
        "com.google.android.apps.youtube.vr.oculus/1.60.19 (Linux; U; Android 14; en_US; Quest 3; Build/UP1A.231005.007.A1; Cronet/132.0.6808.3)".to_string()
    } else if url.contains("c=ANDROID_TESTSUITE") {
        "com.google.android.youtube/1.9 (Linux; U; Android 14; en_US)".to_string()
    } else if url.contains("c=ANDROID") {
        "com.google.android.youtube/19.45.38 (Linux; U; Android 14; en_US)".to_string()
    } else if url.contains("c=IOS") {
        "com.google.ios.youtube/20.03.02 (iPhone16,2; U; CPU iOS 18_2_1 like Mac OS X;)".to_string()
    } else if url.contains("c=MWEB") || url.contains("c=WEB_SAFARI") {
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.2 Mobile/15E148 Safari/604.1".to_string()
    } else if !default_ua.is_empty() {
        default_ua.to_string()
    } else {
        "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0".to_string()
    }
}

/// Extract audio MIME type from a `googlevideo.com` URL
pub fn extract_mime_from_url(url: &str, default_mime: &str) -> String {
    if url.contains("mime=audio%2Fmp4") || url.contains("mime=audio/mp4") || url.contains("itag=140") {
        "audio/mp4".to_string()
    } else if url.contains("mime=audio%2Fwebm") || url.contains("mime=audio/webm") || url.contains("itag=251") || url.contains("itag=250") {
        "audio/webm".to_string()
    } else if !default_mime.is_empty() {
        default_mime.to_string()
    } else {
        "audio/webm".to_string()
    }
}

fn check_disk_cache_hit(video_id: &str) -> Option<String> {
    if !CACHE_ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    let dir = cache_dir();
    for ext in &["webm", "m4a", "opus"] {
        let media_path = dir.join(format!("{}.{}", video_id, ext));
        let meta_path = dir.join(format!("{}.json", video_id));
        if media_path.exists() {
            if let Ok(meta_bytes) = fs::read(&meta_path) {
                if let Ok(meta) = serde_json::from_slice::<CachedTrackMeta>(&meta_bytes) {
                    if let Ok(file_meta) = fs::metadata(&media_path) {
                        if file_meta.len() >= meta.content_length && meta.content_length > 65_536 {
                            crate::aethon_log!(
                                "Tier 0 disk cache HIT for {}: {} ({} bytes)",
                                video_id,
                                media_path.display(),
                                file_meta.len()
                            );
                            return Some(media_path.to_string_lossy().to_string());
                        }
                    }
                }
            } else if let Ok(file_meta) = fs::metadata(&media_path) {
                if file_meta.len() > 500_000 {
                    crate::aethon_log!(
                        "Tier 0 disk cache HIT (unmeta) for {}: {} ({} bytes)",
                        video_id,
                        media_path.display(),
                        file_meta.len()
                    );
                    return Some(media_path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn enforce_disk_cache_limit() {
    let max_bytes = MAX_CACHE_SIZE_BYTES.load(Ordering::Relaxed);
    let dir = cache_dir();
    let mut files = Vec::new();
    let mut total_size: u64 = 0;

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    let len = meta.len();
                    total_size += len;
                    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    files.push((path, len, mtime));
                }
            }
        }
    }

    if total_size > max_bytes {
        files.sort_by_key(|k| k.2);
        for (path, len, _) in files {
            if total_size <= max_bytes {
                break;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".part") {
                    continue;
                }
            }
            if fs::remove_file(&path).is_ok() {
                total_size = total_size.saturating_sub(len);
                crate::aethon_log!(
                    "LRU evicted cache file: {} (freed {} bytes)",
                    path.display(),
                    len
                );
            }
        }
    }
}

fn get_or_spawn_downloader(
    video_id: &str,
    upstream_url: &str,
    content_length: u64,
    mime_type: &str,
    user_agent: &str,
    initial_bytes: Option<Vec<u8>>,
) -> Arc<StreamDownloadTask> {
    let mut tasks = active_tasks().lock();
    if let Some(task) = tasks.get(video_id) {
        if task.error.lock().is_none() {
            return Arc::clone(task);
        }
        tasks.remove(video_id);
    }

    let resolved_clen = if content_length > 0 {
        content_length
    } else {
        extract_clen_from_url(upstream_url).unwrap_or(0)
    };
    let resolved_ua = extract_user_agent_for_url(upstream_url, user_agent);
    let resolved_mime = extract_mime_from_url(upstream_url, mime_type);

    let task = Arc::new(StreamDownloadTask {
        video_id: video_id.to_string(),
        upstream_url: Mutex::new(upstream_url.to_string()),
        user_agent: Mutex::new(resolved_ua),
        mime_type: resolved_mime,
        content_length: AtomicU64::new(resolved_clen),
        downloaded_bytes: AtomicU64::new(0),
        is_complete: AtomicBool::new(false),
        notify: Arc::new(Notify::new()),
        error: Mutex::new(None),
    });

    tasks.insert(video_id.to_string(), Arc::clone(&task));

    let task_worker = Arc::clone(&task);
    let vid = video_id.to_string();

    tauri::async_runtime::spawn(async move {
        run_stream_downloader(task_worker, vid, initial_bytes).await;
    });

    task
}

async fn fetch_chunk_with_fallback(
    client: &reqwest::Client,
    base_url: &str,
    ua: &str,
    cur_pos: u64,
    chunk_end: u64,
) -> Option<(bytes::Bytes, Option<u64>)> {
    let is_mobile_client = base_url.contains("c=ANDROID") || base_url.contains("c=IOS");

    // Strategy 1: Mobile clients prefer `&range=start-end` query param; web/TV clients prefer `Range: bytes=start-end` header
    let url_with_range = if base_url.contains('?') {
        format!("{}&range={}-{}", base_url, cur_pos, chunk_end)
    } else {
        format!("{}?range={}-{}", base_url, cur_pos, chunk_end)
    };

    let attempts: [(&str, bool); 2] = if is_mobile_client {
        [(url_with_range.as_str(), true), (base_url, true)]
    } else {
        [(base_url, true), (url_with_range.as_str(), false)]
    };

    for (target_url, send_range_hdr) in attempts {
        let mut req = client.get(target_url).header("User-Agent", ua);
        if send_range_hdr {
            req = req.header("Range", format!("bytes={}-{}", cur_pos, chunk_end));
        }
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                let mut total_from_hdr = None;
                if let Some(cr) = resp
                    .headers()
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                {
                    if let Some(slash_idx) = cr.rfind('/') {
                        if let Ok(len) = cr[slash_idx + 1..].trim().parse::<u64>() {
                            if len > 0 {
                                total_from_hdr = Some(len);
                            }
                        }
                    }
                }
                if total_from_hdr.is_none() && cur_pos == 0 {
                    if let Some(cl) = resp
                        .headers()
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        if cl > CHUNK_SIZE {
                            total_from_hdr = Some(cl);
                        }
                    }
                }
                if let Ok(b) = resp.bytes().await {
                    if !b.is_empty() {
                        return Some((b, total_from_hdr));
                    }
                }
            }
        }
    }
    None
}

async fn run_stream_downloader(
    task: Arc<StreamDownloadTask>,
    video_id: String,
    initial_bytes: Option<Vec<u8>>,
) {
    let dir = cache_dir();
    let part_path = dir.join(format!("{}.part", video_id));

    let mut file = match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&part_path)
    {
        Ok(f) => f,
        Err(e) => {
            *task.error.lock() = Some(format!("Failed to open part file: {}", e));
            task.notify.notify_waiters();
            active_tasks().lock().remove(&video_id);
            return;
        }
    };

    let mut cur_pos: u64 = 0;

    // Write pre-verified initial bytes immediately so MPV has 0ms wait on connect
    if let Some(init_buf) = initial_bytes {
        if !init_buf.is_empty() && file.write_all(&init_buf).is_ok() {
            let _ = file.flush();
            cur_pos = init_buf.len() as u64;
            task.downloaded_bytes.store(cur_pos, Ordering::Release);
            task.notify.notify_waiters();
        }
    }

    let client = http_client();
    let mut total_len = task.content_length.load(Ordering::Acquire);
    if total_len == 0 {
        let cur_url = task.upstream_url.lock().clone();
        if let Some(clen) = extract_clen_from_url(&cur_url) {
            total_len = clen;
            task.content_length.store(clen, Ordering::Release);
            task.notify.notify_waiters();
        }
    }

    let mut failures = 0;

    while total_len == 0 || cur_pos < total_len {
        let chunk_end = if total_len > 0 {
            (cur_pos + CHUNK_SIZE - 1).min(total_len - 1)
        } else {
            cur_pos + CHUNK_SIZE - 1
        };
        let cur_url = task.upstream_url.lock().clone();
        let cur_ua = task.user_agent.lock().clone();

        if let Some((bytes, hdr_total)) =
            fetch_chunk_with_fallback(client, &cur_url, &cur_ua, cur_pos, chunk_end).await
        {
            failures = 0;
            if total_len == 0 {
                if let Some(ht) = hdr_total.or_else(|| extract_clen_from_url(&cur_url)) {
                    total_len = ht;
                    task.content_length.store(ht, Ordering::Release);
                }
            }
            let n = bytes.len() as u64;
            if file.write_all(&bytes).is_err() {
                break;
            }
            let _ = file.flush();
            cur_pos += n;
            task.downloaded_bytes.store(cur_pos, Ordering::Release);
            task.notify.notify_waiters();

            // If server returned fewer bytes than requested and total_len wasn't known, we reached EOF
            let requested_len = chunk_end - (cur_pos - n) + 1;
            if total_len == 0 && n < requested_len {
                total_len = cur_pos;
                task.content_length.store(total_len, Ordering::Release);
                break;
            }
        } else {
            failures += 1;
            crate::aethon_log!(
                "Upstream chunk error at byte {}/{} for {}. Attempting self-healing recovery (#{})...",
                cur_pos,
                total_len,
                video_id,
                failures
            );
            if failures > 3 {
                *task.error.lock() = Some(format!("Failed past byte {}", cur_pos));
                task.notify.notify_waiters();
                break;
            }
            let v = video_id.clone();
            let refreshed = tokio::task::spawn_blocking(move || {
                YouTubeService::resolve_via_ytdlp_direct(
                    &format!("https://www.youtube.com/watch?v={}", v),
                    "",
                    "",
                )
            })
            .await
            .unwrap_or_default();

            if refreshed.contains("googlevideo.com") {
                let new_ua = extract_user_agent_for_url(&refreshed, "");
                if total_len == 0 {
                    if let Some(clen) = extract_clen_from_url(&refreshed) {
                        total_len = clen;
                        task.content_length.store(clen, Ordering::Release);
                    }
                }
                *task.upstream_url.lock() = refreshed;
                *task.user_agent.lock() = new_ua;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                continue;
            } else {
                *task.error.lock() = Some("yt-dlp refresh returned non-googlevideo URL".to_string());
                task.notify.notify_waiters();
                break;
            }
        }
    }

    if cur_pos >= total_len && total_len > 0 {
        task.is_complete.store(true, Ordering::Release);
        task.notify.notify_waiters();
        let ext = if task.mime_type.contains("mp4") || task.mime_type.contains("m4a") {
            "m4a"
        } else {
            "webm"
        };
        let final_media = dir.join(format!("{}.{}", video_id, ext));
        let final_meta = dir.join(format!("{}.json", video_id));

        let _ = fs::rename(&part_path, &final_media);
        let meta = CachedTrackMeta {
            video_id: video_id.clone(),
            content_length: total_len,
            mime_type: task.mime_type.clone(),
            ext: ext.to_string(),
            created_at: now_unix(),
        };
        if let Ok(json_bytes) = serde_json::to_vec(&meta) {
            let _ = fs::write(&final_meta, json_bytes);
        }
        crate::aethon_log!(
            "Track successfully cached to disk: {} ({} bytes)",
            final_media.display(),
            total_len
        );
        active_tasks().lock().remove(&video_id);
        enforce_disk_cache_limit();
    } else {
        active_tasks().lock().remove(&video_id);
    }
}

/// Starts a lightweight localhost HTTP range-chunking proxy (`127.0.0.1:<port>/stream/<video_id>`).
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

        crate::aethon_log!(
            "Local progressive stream chunk proxy listening on 127.0.0.1:{}",
            port
        );
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

    if video_id.is_empty() {
        let _ = socket
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return;
    }

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
                if let Some(e) = bounds.next().and_then(|v| v.trim().parse::<u64>().ok()) {
                    req_end_opt = Some(e);
                }
            }
        }
    }

    // Check if track is already fully on disk
    let dir = cache_dir();
    let completed_path = ["webm", "m4a", "opus"]
        .iter()
        .map(|ext| dir.join(format!("{}.{}", video_id, ext)))
        .find(|p| p.exists());

    // Case A: Finished file on disk
    if let Some(final_file) = completed_path {
        if let Ok(meta) = fs::metadata(&final_file) {
            let total_len = meta.len();
            if total_len > 0 && req_start < total_len {
                let final_end = req_end_opt
                    .map(|e| e.min(total_len - 1))
                    .unwrap_or(total_len - 1);
                let content_len = final_end - req_start + 1;
                let mime = if final_file.extension().and_then(|e| e.to_str()) == Some("m4a") {
                    "audio/mp4"
                } else {
                    "audio/webm"
                };

                let resp_headers = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    mime, req_start, final_end, total_len, content_len
                );
                if socket.write_all(resp_headers.as_bytes()).await.is_err()
                    || method.eq_ignore_ascii_case("HEAD")
                {
                    return;
                }

                if let Ok(mut f) = File::open(&final_file) {
                    if f.seek(SeekFrom::Start(req_start)).is_ok() {
                        let mut cur_pos = req_start;
                        let mut chunk_buf = vec![0u8; 65536];
                        while cur_pos <= final_end {
                            let to_read = ((final_end - cur_pos + 1) as usize).min(chunk_buf.len());
                            let n = match f.read(&mut chunk_buf[..to_read]) {
                                Ok(n) if n > 0 => n,
                                _ => break,
                            };
                            if socket.write_all(&chunk_buf[..n]).await.is_err() {
                                break;
                            }
                            cur_pos += n as u64;
                        }
                    }
                }
                return;
            }
        }
    }

    // Case B: In-progress download task
    let task_opt = active_tasks().lock().get(video_id).cloned();
    let Some(task) = task_opt else {
        let _ = socket
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return;
    };

    // CRITICAL FIX: Wait for total_len > 0 and initial bytes before responding!
    // Never return 416 Range Not Satisfiable while total_len is still 0 (being resolved).
    let wait_start = std::time::Instant::now();
    let mut total_len = task.content_length.load(Ordering::Acquire);
    while total_len == 0 && wait_start.elapsed() < std::time::Duration::from_secs(8) {
        if task.error.lock().is_some() {
            let _ = socket
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await;
            return;
        }
        let notify = Arc::clone(&task.notify);
        let _ = tokio::time::timeout(std::time::Duration::from_millis(250), notify.notified()).await;
        total_len = task.content_length.load(Ordering::Acquire);
    }

    if total_len == 0 || req_start >= total_len {
        let _ = socket
            .write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return;
    }

    let final_end = req_end_opt
        .map(|e| e.min(total_len - 1))
        .unwrap_or(total_len - 1);
    let content_len = final_end - req_start + 1;

    let resp_headers = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        task.mime_type, req_start, final_end, total_len, content_len
    );

    if socket.write_all(resp_headers.as_bytes()).await.is_err()
        || method.eq_ignore_ascii_case("HEAD")
    {
        return;
    }

    let part_path = dir.join(format!("{}.part", video_id));
    let mut file = loop {
        if let Ok(f) = File::open(&part_path) {
            break f;
        }
        let mut found = None;
        for ext in &["webm", "m4a", "opus"] {
            let done_p = dir.join(format!("{}.{}", video_id, ext));
            if let Ok(f) = File::open(&done_p) {
                found = Some(f);
                break;
            }
        }
        if let Some(f) = found {
            break f;
        }
        if task.error.lock().is_some() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    };

    let mut cur_pos = req_start;
    let mut chunk_buf = vec![0u8; 65536];

    while cur_pos <= final_end {
        let dl_bytes = task.downloaded_bytes.load(Ordering::Acquire);
        let complete = task.is_complete.load(Ordering::Acquire);

        if cur_pos < dl_bytes {
            let to_read = ((dl_bytes - cur_pos) as usize)
                .min(chunk_buf.len())
                .min((final_end - cur_pos + 1) as usize);
            if file.seek(SeekFrom::Start(cur_pos)).is_err() {
                break;
            }
            let n = match file.read(&mut chunk_buf[..to_read]) {
                Ok(n) if n > 0 => n,
                _ => {
                    // Check if .part was atomically renamed to .webm/.m4a on completion
                    let mut reopened = None;
                    for ext in &["webm", "m4a", "opus"] {
                        let done_p = dir.join(format!("{}.{}", video_id, ext));
                        if let Ok(f) = File::open(&done_p) {
                            reopened = Some(f);
                            break;
                        }
                    }
                    if let Some(mut new_f) = reopened {
                        if new_f.seek(SeekFrom::Start(cur_pos)).is_ok() {
                            if let Ok(n2) = new_f.read(&mut chunk_buf[..to_read]) {
                                if n2 > 0 {
                                    file = new_f;
                                    if socket.write_all(&chunk_buf[..n2]).await.is_err() {
                                        break;
                                    }
                                    cur_pos += n2 as u64;
                                    continue;
                                }
                            }
                        }
                    }
                    break;
                }
            };
            if socket.write_all(&chunk_buf[..n]).await.is_err() {
                break;
            }
            cur_pos += n as u64;
        } else if complete {
            break;
        } else {
            if task.error.lock().is_some() {
                break;
            }
            let notify = Arc::clone(&task.notify);
            let _ =
                tokio::time::timeout(std::time::Duration::from_secs(10), notify.notified()).await;
        }
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
            active_tasks().lock().remove(&vid);
        }
    }

    pub fn upgrade_thumbnail_url(url: &str, video_id: &str) -> String {
        let trimmed = url.trim();
        if trimmed.contains("lh3.googleusercontent.com") || trimmed.contains("yt3.ggpht.com") {
            if let Some(eq_idx) = trimmed.rfind('=') {
                let suffix = &trimmed[eq_idx + 1..];
                if suffix.starts_with('w') || suffix.starts_with('s') {
                    return format!("{}=w544-h544-l90-rj", &trimmed[..eq_idx]);
                }
            }
            return trimmed.to_string();
        }
        // Preserve signed ?sqp= and valid i.ytimg.com URLs; only fallback to mqdefault.jpg (which never 404s) when empty or bare /default.jpg
        if !trimmed.is_empty() && !trimmed.ends_with("/default.jpg") {
            return trimmed.to_string();
        }
        if !video_id.is_empty() {
            return format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", video_id);
        }
        trimmed.to_string()
    }

    fn capture_web_visitor_data(json: &serde_json::Value) {
        if let Some(vd) = json
            .pointer("/responseContext/visitorData")
            .and_then(|v| v.as_str())
        {
            if !vd.is_empty() {
                *web_visitor_store().lock() = Some(vd.to_string());
            }
        }
    }

    fn capture_vr_visitor_data(json: &serde_json::Value) {
        if let Some(vd) = json
            .pointer("/responseContext/visitorData")
            .and_then(|v| v.as_str())
        {
            if !vd.is_empty() {
                *vr_visitor_store().lock() = Some(vd.to_string());
            }
        }
    }

    pub async fn search(query: &str, limit: usize, gl: &str) -> Vec<YouTubeTrack> {
        let max_results = limit.clamp(1, 80);
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

    fn find_continuation_token(val: &serde_json::Value) -> Option<String> {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(tok) = map
                    .get("nextContinuationData")
                    .and_then(|n| n.get("continuation"))
                    .and_then(|v| v.as_str())
                {
                    if !tok.is_empty() {
                        return Some(tok.to_string());
                    }
                }
                for v in map.values() {
                    if let Some(found) = Self::find_continuation_token(v) {
                        return Some(found);
                    }
                }
                None
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(found) = Self::find_continuation_token(item) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
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

        let videos_body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "gl": gl,
                    "hl": "en"
                }
            },
            "query": query,
            "params": "EgWKAQIIA2oSEAQQAxAJEAUQChAVEA4QERAQ"
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

        let videos_fut = client
            .post("https://music.youtube.com/youtubei/v1/search?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
            )
            .json(&videos_body)
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

        let (songs_res, videos_res, general_res) = tokio::join!(songs_fut, videos_fut, general_fut);

        let mut results = Vec::new();
        let mut continuation_token: Option<String> = None;

        // Prioritize Songs catalog first (like ArchiveTune), then General top results, then Videos
        if let Ok(resp) = songs_res {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    Self::capture_web_visitor_data(&json_data);
                    continuation_token = Self::find_continuation_token(&json_data);
                    Self::extract_innertube_tracks(&json_data, &mut results, limit);
                }
            }
        }

        if results.len() < limit {
            if let Ok(resp) = general_res {
                if resp.status().is_success() {
                    if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                        Self::capture_web_visitor_data(&json_data);
                        Self::extract_innertube_tracks(&json_data, &mut results, limit);
                    }
                }
            }
        }

        if results.len() < limit {
            if let Ok(resp) = videos_res {
                if resp.status().is_success() {
                    if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                        Self::capture_web_visitor_data(&json_data);
                        Self::extract_innertube_tracks(&json_data, &mut results, limit);
                    }
                }
            }
        }

        // Follow InnerTube continuation token if caller requested more songs (e.g. limit > 25)
        let mut pages = 0;
        while results.len() < limit && continuation_token.is_some() && pages < 2 {
            pages += 1;
            let ctok = continuation_token.take().unwrap();
            let cont_url = format!(
                "https://music.youtube.com/youtubei/v1/search?ctoken={}&continuation={}&type=next&prettyPrint=false",
                ctok, ctok
            );
            let cont_body = serde_json::json!({
                "context": {
                    "client": {
                        "clientName": "WEB_REMIX",
                        "clientVersion": "1.20260408.01.00",
                        "gl": gl,
                        "hl": "en"
                    }
                }
            });
            if let Ok(c_resp) = client
                .post(&cont_url)
                .header("Content-Type", "application/json")
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
                )
                .json(&cont_body)
                .send()
                .await
            {
                if c_resp.status().is_success() {
                    if let Ok(c_json) = c_resp.json::<serde_json::Value>().await {
                        continuation_token = Self::find_continuation_token(&c_json);
                        Self::extract_innertube_tracks(&c_json, &mut results, limit);
                    }
                }
            }
        }

        Some(results)
    }

    /// Fetch authentic YouTube Music Home Screen shelves (`FEmusic_home` + 2 continuations)
    /// Uses authenticated SAPISIDHASH headers when connected to a Google Account (ArchiveTune parity)!
    pub async fn fetch_home_shelves(cookie: Option<&str>, gl: &str) -> Vec<HomeShelf> {
        let region = if gl.trim().is_empty() { "IN" } else { gl.trim() };
        let client = http_client();
        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "gl": region,
                    "hl": "en"
                }
            },
            "browseId": "FEmusic_home"
        });

        let auth_headers = cookie
            .filter(|c| !c.trim().is_empty())
            .and_then(|c| auth_sync::build_auth_headers(c, None, None).ok());

        let mut req = client
            .post("https://music.youtube.com/youtubei/v1/browse?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
            );
        if let Some(ref hdrs) = auth_headers {
            req = req.headers(hdrs.clone());
        }

        let mut shelves: Vec<HomeShelf> = Vec::new();
        if let Ok(resp) = req.json(&body).send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    Self::capture_web_visitor_data(&json_data);
                    Self::extract_home_shelves_recursive(&json_data, &mut shelves);

                    let mut next_ctok = Self::find_continuation_token(&json_data);
                    let mut pages = 0;
                    while let Some(ctok) = next_ctok.take() {
                        if pages >= 2 {
                            break;
                        }
                        pages += 1;
                        let cont_url = format!(
                            "https://music.youtube.com/youtubei/v1/browse?ctoken={}&continuation={}&type=next&prettyPrint=false",
                            ctok, ctok
                        );
                        let mut c_req = client
                            .post(&cont_url)
                            .header("Content-Type", "application/json")
                            .header(
                                "User-Agent",
                                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
                            );
                        if let Some(ref hdrs) = auth_headers {
                            c_req = c_req.headers(hdrs.clone());
                        }
                        if let Ok(c_resp) = c_req.json(&body).send().await {
                            if c_resp.status().is_success() {
                                if let Ok(c_json) = c_resp.json::<serde_json::Value>().await {
                                    next_ctok = Self::find_continuation_token(&c_json);
                                    Self::extract_home_shelves_recursive(&c_json, &mut shelves);
                                }
                            }
                        }
                    }
                }
            }
        }

        shelves
    }

    /// ArchiveTune Native Automix / Up Next Queue (`/youtubei/v1/next` with `RDAMVM{video_id}`)
    pub async fn fetch_automix_queue(
        video_id: &str,
        cookie: Option<&str>,
        gl: &str,
    ) -> Vec<YouTubeTrack> {
        let clean_vid = video_id.trim().strip_prefix("yt_").unwrap_or(video_id.trim());
        if clean_vid.is_empty() {
            return Vec::new();
        }
        let region = if gl.trim().is_empty() { "IN" } else { gl.trim() };
        let client = http_client();

        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "gl": region,
                    "hl": "en"
                }
            },
            "videoId": clean_vid,
            "playlistId": format!("RDAMVM{}", clean_vid),
            "isAudioOnly": true,
            "enablePersistentPlaylistPanel": true
        });

        let mut req = client
            .post("https://music.youtube.com/youtubei/v1/next?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
            );
        if let Some(c) = cookie.filter(|s| !s.trim().is_empty()) {
            if let Ok(hdrs) = auth_sync::build_auth_headers(c, None, None) {
                req = req.headers(hdrs);
            }
        }

        let mut results = Vec::new();
        if let Ok(resp) = req.json(&body).send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    Self::capture_web_visitor_data(&json_data);
                    Self::extract_playlist_panel_tracks(&json_data, clean_vid, &mut results, 28);
                }
            }
        }

        results
    }

    fn extract_playlist_panel_tracks(
        value: &serde_json::Value,
        exclude_video_id: &str,
        results: &mut Vec<YouTubeTrack>,
        limit: usize,
    ) {
        if results.len() >= limit {
            return;
        }
        if let Some(map) = value.as_object() {
            if let Some(panel) = map.get("playlistPanelVideoRenderer") {
                let vid = panel
                    .get("videoId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();

                if !vid.is_empty() && vid != exclude_video_id && !results.iter().any(|r| r.id == vid) {
                    let title = panel
                        .pointer("/title/runs/0/text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    let mut artist = String::new();
                    if let Some(runs) = panel
                        .pointer("/longBylineText/runs")
                        .or_else(|| panel.pointer("/shortBylineText/runs"))
                        .and_then(|r| r.as_array())
                    {
                        let mut artist_names: Vec<String> = Vec::new();
                        for r in runs {
                            if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                                let tr = t.trim();
                                if tr == "•" || tr == "·" || tr == "|" {
                                    if !artist_names.is_empty() {
                                        break;
                                    }
                                    continue;
                                }
                                if !Self::is_metadata_noise_run(tr) && tr != "," && tr != "&" {
                                    let clean_p = tr
                                        .trim_matches(|c: char| c == ',' || c == ' ')
                                        .to_string();
                                    if !clean_p.is_empty()
                                        && !artist_names.iter().any(|a| a.eq_ignore_ascii_case(&clean_p))
                                    {
                                        artist_names.push(clean_p);
                                    }
                                }
                            }
                        }
                        artist = artist_names.join(", ");
                    }

                    let duration = panel
                        .pointer("/lengthText/runs/0/text")
                        .and_then(|v| v.as_str())
                        .and_then(Self::parse_duration_text)
                        .unwrap_or(0.0);

                    let raw_thumb = panel
                        .pointer("/thumbnail/thumbnails")
                        .and_then(|th| th.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|t| t.get("url"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("");

                    if !title.is_empty() {
                        let thumbnail = Self::upgrade_thumbnail_url(raw_thumb, &vid);
                        let stream_url = format!("https://www.youtube.com/watch?v={}", vid);
                        results.push(YouTubeTrack {
                            id: vid,
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
                Self::extract_playlist_panel_tracks(v, exclude_video_id, results, limit);
            }
        } else if let Some(arr) = value.as_array() {
            for v in arr {
                Self::extract_playlist_panel_tracks(v, exclude_video_id, results, limit);
            }
        }
    }

    /// Fetch tracks inside a YouTube Music cloud playlist (`VL...`, `PL...`, `VLLM`)
    pub async fn fetch_cloud_playlist_tracks(
        browse_id: &str,
        cookie: Option<&str>,
        gl: &str,
    ) -> Vec<YouTubeTrack> {
        let clean_id = if browse_id.starts_with("VL") || browse_id.starts_with("FE") {
            browse_id.to_string()
        } else {
            format!("VL{}", browse_id)
        };
        let region = if gl.trim().is_empty() { "IN" } else { gl.trim() };
        let client = http_client();
        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20260408.01.00",
                    "gl": region,
                    "hl": "en"
                }
            },
            "browseId": clean_id
        });

        let mut req = client
            .post("https://music.youtube.com/youtubei/v1/browse?prettyPrint=false")
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
            );
        if let Some(c) = cookie.filter(|s| !s.trim().is_empty()) {
            if let Ok(hdrs) = auth_sync::build_auth_headers(c, None, None) {
                req = req.headers(hdrs);
            }
        }

        let mut results = Vec::new();
        if let Ok(resp) = req.json(&body).send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    Self::extract_innertube_tracks(&json_data, &mut results, 200);
                }
            }
        }
        results
    }

    fn extract_home_shelves_recursive(
        value: &serde_json::Value,
        shelves: &mut Vec<HomeShelf>,
    ) {
        if let Some(map) = value.as_object() {
            for key in ["musicCarouselShelfRenderer", "musicShelfRenderer"] {
                if let Some(shelf_obj) = map.get(key) {
                    let title = shelf_obj
                        .pointer("/header/musicCarouselShelfBasicHeaderRenderer/title/runs/0/text")
                        .or_else(|| shelf_obj.pointer("/title/runs/0/text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    let mut tracks = Vec::new();
                    Self::extract_innertube_tracks(shelf_obj, &mut tracks, 20);
                    if !title.is_empty() && tracks.len() >= 3 {
                        if let Some(existing) = shelves.iter_mut().find(|s| s.title.eq_ignore_ascii_case(&title)) {
                            for t in tracks {
                                if !existing.tracks.iter().any(|et| et.id == t.id) {
                                    existing.tracks.push(t);
                                }
                            }
                        } else {
                            shelves.push(HomeShelf { title, tracks });
                        }
                    }
                }
            }
            for v in map.values() {
                Self::extract_home_shelves_recursive(v, shelves);
            }
        } else if let Some(arr) = value.as_array() {
            for v in arr {
                Self::extract_home_shelves_recursive(v, shelves);
            }
        }
    }

    /// Fast ~250ms InnerTube `/youtubei/v1/player` extraction + 64KB prebuffer & tail verification
    async fn resolve_innertube_bounded_stream(
        video_id: &str,
    ) -> Option<(CachedStream, Vec<u8>)> {
        let client = player_client();
        let vr_visitor_data = vr_visitor_store().lock().clone();
        let web_visitor_data = web_visitor_store().lock().clone();

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

        if let Some(ref vd) = vr_visitor_data {
            vr_client["visitorData"] = serde_json::Value::String(vd.clone());
            ts_client["visitorData"] = serde_json::Value::String(vd.clone());
        }
        if let Some(ref vd) = web_visitor_data {
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

        for (resp_res, ua, is_vr) in [
            (vr_res, vr_ua, true),
            (ts_res, ts_ua, true),
            (ios_res, ios_ua, false),
        ] {
            if let Ok(resp) = resp_res {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if is_vr {
                            Self::capture_vr_visitor_data(&json);
                        } else {
                            Self::capture_web_visitor_data(&json);
                        }
                        if let Some((upstream_url, content_length, mime_type)) =
                            Self::pick_best_audio_format(&json)
                        {
                            let clen = content_length
                                .or_else(|| extract_clen_from_url(&upstream_url))
                                .unwrap_or(0);

                            // If this was IOS, run a tail-byte probe past 1MB to detect Google Video SABR 403 ceiling
                            if !is_vr && clen > 1_048_576 {
                                let probe_start = clen.saturating_sub(64);
                                let probe_end = clen.saturating_sub(33);
                                if fetch_chunk_with_fallback(
                                    player_client(),
                                    &upstream_url,
                                    ua,
                                    probe_start,
                                    probe_end,
                                )
                                .await
                                .is_none()
                                {
                                    crate::aethon_log!(
                                        "IOS stream failed tail-byte probe for {}. Skipping to next resolver.",
                                        video_id
                                    );
                                    continue;
                                }
                            }

                            // Pre-fetch the first 64 KB chunk to verify the stream is playable and give MPV 0ms initial bytes
                            let first_end = if clen > 0 {
                                65_535u64.min(clen - 1)
                            } else {
                                65_535u64
                            };
                            if let Some((first_bytes, hdr_total)) = fetch_chunk_with_fallback(
                                player_client(),
                                &upstream_url,
                                ua,
                                0,
                                first_end,
                            )
                            .await
                            {
                                let verified_clen = if clen > 0 {
                                    Some(clen)
                                } else {
                                    hdr_total
                                };
                                let port = ensure_chunk_proxy_server();
                                let playable_url =
                                    format!("http://127.0.0.1:{}/stream/{}", port, video_id);
                                return Some((
                                    CachedStream {
                                        playable_url,
                                        upstream_url,
                                        content_length: verified_clen,
                                        mime_type,
                                        user_agent: ua.to_string(),
                                        expires_at: now_unix() + 14_400,
                                    },
                                    first_bytes.to_vec(),
                                ));
                            }
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
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| extract_clen_from_url(url));
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
        if video_id.is_empty() {
            return uri.to_string();
        }

        // Tier 0: Disk Cache Hit (< 2ms startup, offline playback)
        if let Some(cached_file) = check_disk_cache_hit(&video_id) {
            crate::aethon_log!(
                "Tier 0 Disk Cache HIT for video_id={} -> {}",
                video_id,
                cached_file
            );
            return cached_file;
        }

        // If an active healthy download task is already running, return proxy URL immediately
        if let Some(existing_task) = active_tasks().lock().get(&video_id).cloned() {
            if existing_task.error.lock().is_none() {
                let port = ensure_chunk_proxy_server();
                return format!("http://127.0.0.1:{}/stream/{}", port, video_id);
            }
        }

        // Check in-memory stream cache
        let now = now_unix();
        let cached_opt = stream_cache().lock().get(&video_id).cloned();
        if let Some(cached) = cached_opt {
            if cached.expires_at > now {
                crate::aethon_log!("Stream cache HIT for video_id={}", video_id);
                get_or_spawn_downloader(
                    &video_id,
                    &cached.upstream_url,
                    cached.content_length.unwrap_or(0),
                    &cached.mime_type,
                    &cached.user_agent,
                    None,
                );
                return cached.playable_url;
            }
        }

        // Tier 1: Fast InnerTube ANDROID_VR / ANDROID_TESTSUITE / verified IOS with 64KB prebuffer
        if let Some((entry, init_bytes)) =
            Self::resolve_innertube_bounded_stream(&video_id).await
        {
            let playable = entry.playable_url.clone();
            get_or_spawn_downloader(
                &video_id,
                &entry.upstream_url,
                entry.content_length.unwrap_or(0),
                &entry.mime_type,
                &entry.user_agent,
                Some(init_bytes),
            );
            stream_cache().lock().insert(video_id.clone(), entry);
            crate::aethon_log!(
                "Resolved Tier 1 InnerTube stream with 64KB prebuffer for video_id={}",
                video_id
            );
            return playable;
        }

        // Tier 2 Fallback: `yt-dlp --no-config -g`
        let uri_owned = uri.to_string();
        let title_owned = title.to_string();
        let artist_owned = artist.to_string();
        let vid_owned = video_id.clone();

        let resolved = tokio::task::spawn_blocking(move || {
            Self::resolve_via_ytdlp_direct(&uri_owned, &title_owned, &artist_owned)
        })
        .await
        .unwrap_or_else(|_| uri.to_string());

        if !vid_owned.is_empty() && resolved.contains("googlevideo.com") {
            let clen = extract_clen_from_url(&resolved).unwrap_or(0);
            let ua = extract_user_agent_for_url(&resolved, "");
            let mime = extract_mime_from_url(&resolved, "audio/webm");
            let port = ensure_chunk_proxy_server();
            let proxy_url = format!("http://127.0.0.1:{}/stream/{}", port, vid_owned);

            let task = get_or_spawn_downloader(&vid_owned, &resolved, clen, &mime, &ua, None);

            // Wait up to 1200ms for the first chunk or content_length to be verified
            let start = std::time::Instant::now();
            while task.downloaded_bytes.load(Ordering::Acquire) == 0
                && start.elapsed() < std::time::Duration::from_millis(1200)
            {
                if task.error.lock().is_some() {
                    break;
                }
                let notify = Arc::clone(&task.notify);
                let _ = tokio::time::timeout(
                    std::time::Duration::from_millis(200),
                    notify.notified(),
                )
                .await;
            }

            if task.error.lock().is_none() && task.content_length.load(Ordering::Acquire) > 0 {
                stream_cache().lock().insert(
                    vid_owned,
                    CachedStream {
                        playable_url: proxy_url.clone(),
                        upstream_url: resolved,
                        content_length: Some(task.content_length.load(Ordering::Acquire)),
                        mime_type: mime,
                        user_agent: ua,
                        expires_at: now_unix() + 14_400,
                    },
                );
                return proxy_url;
            }
        }

        resolved
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
            .arg("youtube:player_client=tv,android_vr,mweb,web_safari")
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
        if !query.is_empty() && query != "audio" {
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
                .arg("youtube:player_client=tv,android_vr,mweb,web_safari")
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

    pub fn extract_innertube_tracks(
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
                            let mut artist_names: Vec<String> = Vec::new();
                            let mut hit_bullet = false;
                            for r in runs {
                                if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                                    let trimmed = t.trim();
                                    if let Some(d) = Self::parse_duration_text(trimmed) {
                                        duration = d;
                                        continue;
                                    }
                                    if trimmed == "•" || trimmed == "·" || trimmed == "|" {
                                        if !artist_names.is_empty() {
                                            hit_bullet = true;
                                        }
                                        continue;
                                    }
                                    if !hit_bullet && !Self::is_metadata_noise_run(trimmed) {
                                        if trimmed == "," || trimmed == "&" || trimmed == "and" {
                                            continue;
                                        }
                                        let cleaned_part = trimmed
                                            .trim_matches(|c: char| c == ',' || c == ' ')
                                            .to_string();
                                        if !cleaned_part.is_empty()
                                            && !artist_names.iter().any(|a| a.eq_ignore_ascii_case(&cleaned_part))
                                        {
                                            artist_names.push(cleaned_part);
                                        }
                                    }
                                }
                            }
                            if !artist_names.is_empty() {
                                artist = Self::strip_artist_metadata_suffixes(&artist_names.join(", "))
                                    .replace(" ,", ",")
                                    .replace(",  ", ", ");
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

            if let Some(two_row) = map.get("musicTwoRowItemRenderer") {
                let video_id = two_row
                    .pointer("/navigationEndpoint/watchEndpoint/videoId")
                    .or_else(|| two_row.pointer("/thumbnailOverlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let title = two_row
                    .pointer("/title/runs/0/text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();

                let mut artist = String::from("YouTube Music");
                if let Some(sub_runs) = two_row.pointer("/subtitle/runs").and_then(|r| r.as_array()) {
                    let mut artist_names = Vec::new();
                    for r in sub_runs {
                        if let Some(t) = r.get("text").and_then(|s| s.as_str()) {
                            let tr = t.trim();
                            if tr == "•" || tr == "·" || tr == "|" {
                                if !artist_names.is_empty() {
                                    break;
                                }
                                continue;
                            }
                            if !Self::is_metadata_noise_run(tr) && tr != "," && tr != "&" {
                                let clean_p = tr.trim_matches(|c: char| c == ',' || c == ' ').to_string();
                                if !clean_p.is_empty() && !artist_names.iter().any(|a: &String| a.eq_ignore_ascii_case(&clean_p)) {
                                    artist_names.push(clean_p);
                                }
                            }
                        }
                    }
                    if !artist_names.is_empty() {
                        artist = artist_names.join(", ");
                    }
                }

                if !video_id.is_empty() && !title.is_empty() && video_id.len() == 11 {
                    let raw_thumb = two_row
                        .pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
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
                            duration: 0.0,
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

                let thumbnail = format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", id);
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
