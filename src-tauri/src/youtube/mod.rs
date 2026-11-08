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
    pub user_agent: String,
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
            .pool_max_idle_per_host(12)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .connect_timeout(std::time::Duration::from_secs(5))
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
            .connect_timeout(std::time::Duration::from_millis(2600))
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
) -> Arc<StreamDownloadTask> {
    let mut tasks = active_tasks().lock();
    if let Some(task) = tasks.get(video_id) {
        return Arc::clone(task);
    }

    let task = Arc::new(StreamDownloadTask {
        video_id: video_id.to_string(),
        upstream_url: Mutex::new(upstream_url.to_string()),
        user_agent: user_agent.to_string(),
        mime_type: mime_type.to_string(),
        content_length: AtomicU64::new(content_length),
        downloaded_bytes: AtomicU64::new(0),
        is_complete: AtomicBool::new(false),
        notify: Arc::new(Notify::new()),
        error: Mutex::new(None),
    });

    tasks.insert(video_id.to_string(), Arc::clone(&task));

    let task_worker = Arc::clone(&task);
    let vid = video_id.to_string();

    tauri::async_runtime::spawn(async move {
        run_stream_downloader(task_worker, vid).await;
    });

    task
}

async fn run_stream_downloader(task: Arc<StreamDownloadTask>, video_id: String) {
    let dir = cache_dir();
    let part_path = dir.join(format!("{}.part", video_id));

    let (mut file, mut cur_pos) = match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&part_path)
    {
        Ok(mut f) => {
            let existing_len = f.metadata().map(|m| m.len()).unwrap_or(0);
            let _ = f.seek(SeekFrom::Start(existing_len));
            if existing_len > 0 {
                task.downloaded_bytes.store(existing_len, Ordering::Release);
                task.notify.notify_waiters();
                crate::aethon_log!(
                    "Downloader resumed with {} pre-buffered bytes in {}",
                    existing_len,
                    part_path.display()
                );
            }
            (f, existing_len)
        }
        Err(e) => {
            *task.error.lock() = Some(format!("Failed to open part file: {}", e));
            task.notify.notify_waiters();
            return;
        }
    };

    let client = http_client();
    let mut total_len = task.content_length.load(Ordering::Acquire);
    if total_len == 0 {
        let cur_url = task.upstream_url.lock().clone();
        if let Ok(resp) = client
            .get(&cur_url)
            .header("User-Agent", &task.user_agent)
            .header("Range", "bytes=0-0")
            .send()
            .await
        {
            if let Some(cr) = resp.headers().get("content-range").and_then(|v| v.to_str().ok()) {
                if let Some(slash_idx) = cr.rfind('/') {
                    if let Ok(len) = cr[slash_idx + 1..].trim().parse::<u64>() {
                        total_len = len;
                        task.content_length.store(len, Ordering::Release);
                    }
                }
            }
        }
    }

    let mut failures = 0;
    let mut ytdlp_retries = 0;

    while total_len == 0 || cur_pos < total_len {
        let chunk_end = if total_len > 0 {
            (cur_pos + CHUNK_SIZE - 1).min(total_len - 1)
        } else {
            cur_pos + CHUNK_SIZE - 1
        };
        let cur_url = task.upstream_url.lock().clone();

        let req_fut = client
            .get(&cur_url)
            .header("User-Agent", &task.user_agent)
            .header("Range", format!("bytes={}-{}", cur_pos, chunk_end))
            .send();

        match req_fut.await {
            Ok(resp) if resp.status().is_success() => {
                failures = 0;
                if total_len == 0 {
                    if let Some(cr) = resp.headers().get("content-range").and_then(|v| v.to_str().ok()) {
                        if let Some(slash_idx) = cr.rfind('/') {
                            if let Ok(len) = cr[slash_idx + 1..].trim().parse::<u64>() {
                                total_len = len;
                                task.content_length.store(len, Ordering::Release);
                            }
                        }
                    }
                }
                if let Ok(bytes) = resp.bytes().await {
                    if bytes.is_empty() {
                        break;
                    }
                    let n = bytes.len() as u64;
                    if file.write_all(&bytes).is_err() {
                        break;
                    }
                    let _ = file.flush();
                    cur_pos += n;
                    task.downloaded_bytes.store(cur_pos, Ordering::Release);
                    task.notify.notify_waiters();
                } else {
                    failures += 1;
                }
            }
            Ok(resp) => {
                let status = resp.status();
                failures += 1;
                crate::aethon_log!(
                    "Upstream chunk HTTP error {} at byte {}/{} for {}. (#{})",
                    status,
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

                // If Google Video returned 403 Forbidden or 410 Gone, the signed URL may have expired.
                // Re-resolve via yt-dlp at most once to prevent runaway Python child processes from pegging CPU.
                let status_code = status.as_u16();
                if (status_code == 403 || status_code == 410) && ytdlp_retries < 1 {
                    ytdlp_retries += 1;
                    crate::aethon_log!(
                        "Upstream URL expired (status {}). Attempting single yt-dlp self-healing recovery for {}...",
                        status,
                        video_id
                    );
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
                        *task.upstream_url.lock() = refreshed;
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    } else {
                        *task.error.lock() = Some(format!("Failed past byte {}", cur_pos));
                        task.notify.notify_waiters();
                        break;
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(500 * failures as u64)).await;
                }
            }
            Err(e) => {
                failures += 1;
                crate::aethon_log!(
                    "Upstream chunk network error '{}' at byte {}/{} for {}. (#{})",
                    e,
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
                tokio::time::sleep(std::time::Duration::from_millis(500 * failures as u64)).await;
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
        enforce_disk_cache_limit();
    } else if cur_pos == 0 {
        let _ = fs::remove_file(&part_path);
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

        crate::aethon_log!("Local progressive stream chunk proxy listening on 127.0.0.1:{}", port);
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

    // Check if track is already fully on disk
    let dir = cache_dir();
    let completed_path = ["webm", "m4a", "opus"]
        .iter()
        .map(|ext| dir.join(format!("{}.{}", video_id, ext)))
        .find(|p| p.exists());

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

    // Case A: Finished file on disk
    if let Some(final_file) = completed_path {
        if let Ok(meta) = fs::metadata(&final_file) {
            let total_len = meta.len();
            if total_len > 0 && req_start < total_len {
                let final_end = req_end_opt.map(|e| e.min(total_len - 1)).unwrap_or(total_len - 1);
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
                if socket.write_all(resp_headers.as_bytes()).await.is_err() || method.eq_ignore_ascii_case("HEAD") {
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

    let mut total_len = task.content_length.load(Ordering::Acquire);
    if total_len == 0 {
        let notify = Arc::clone(&task.notify);
        let _ = tokio::time::timeout(std::time::Duration::from_millis(1500), notify.notified()).await;
        total_len = task.content_length.load(Ordering::Acquire);
    }

    if total_len > 0 && req_start >= total_len {
        let _ = socket
            .write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return;
    }

    let final_end = req_end_opt
        .map(|e| if total_len > 0 { e.min(total_len - 1) } else { e })
        .unwrap_or(if total_len > 0 { total_len - 1 } else { req_start + 524_288 });
    let content_len = final_end - req_start + 1;

    let total_len_str = if total_len > 0 {
        total_len.to_string()
    } else {
        "*".to_string()
    };

    let resp_headers = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        task.mime_type, req_start, final_end, total_len_str, content_len
    );

    if socket.write_all(resp_headers.as_bytes()).await.is_err() || method.eq_ignore_ascii_case("HEAD") {
        return;
    }

    let part_path = dir.join(format!("{}.part", video_id));
    let mut file = loop {
        if let Ok(f) = File::open(&part_path) {
            break f;
        }
        if task.error.lock().is_some() {
            let cur_url = task.upstream_url.lock().clone();
            if cur_url.starts_with("http") {
                let resp = format!(
                    "HTTP/1.1 307 Temporary Redirect\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    cur_url
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            }
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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
                _ => break,
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
            let _ = tokio::time::timeout(std::time::Duration::from_secs(12), notify.notified()).await;
        }
    }
}

pub struct YouTubeService;

impl YouTubeService {
    pub fn extract_clen_from_url(url: &str) -> Option<u64> {
        if let Some(idx) = url.find("clen=") {
            let rest = &url[idx + 5..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(val) = num_str.parse::<u64>() {
                return Some(val);
            }
        }
        None
    }

    pub fn extract_user_agent_for_url(url: &str) -> String {
        if url.contains("c=ANDROID_VR") {
            "com.google.android.apps.youtube.vr.oculus/1.60.19 (Linux; U; Android 14; Quest 3)".to_string()
        } else if url.contains("c=ANDROID_TESTSUITE") {
            "com.google.android.apps.youtube.testsuite/1.9 (Linux; U; Android 14)".to_string()
        } else if url.contains("c=IOS") || url.contains("c=MWEB") {
            "com.google.ios.youtube/20.03.02 (iPhone16,2; U; CPU iOS 18_2_1 like Mac OS X;)".to_string()
        } else if url.contains("c=TVHTML5") || url.contains("c=TV") {
            "Mozilla/5.0 (SMART-TV; Linux; Tizen 6.0) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/4.0 Chrome/76.0.3809.146 TV Safari/537.36".to_string()
        } else {
            "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0".to_string()
        }
    }

    pub fn extract_mime_from_url(url: &str) -> String {
        if let Some(idx) = url.find("mime=") {
            let rest = &url[idx + 5..];
            let raw: String = rest.chars().take_while(|&c| c != '&' && c != '#' && c != '?').collect();
            if let Ok(decoded) = urlencoding::decode(&raw) {
                let clean = decoded.split(';').next().unwrap_or("").trim().to_string();
                if !clean.is_empty() {
                    return clean;
                }
            }
        }
        if url.contains("audio%2Fwebm") || url.contains("audio/webm") {
            "audio/webm".to_string()
        } else if url.contains("audio%2Fmp4") || url.contains("audio/mp4") || url.contains("m4a") {
            "audio/mp4".to_string()
        } else {
            "audio/webm".to_string()
        }
    }

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
        if url.contains("googleusercontent.com") || url.contains("ggpht.com") {
            if let Some(eq_idx) = url.rfind('=') {
                let suffix = &url[eq_idx + 1..];
                if suffix.starts_with('w') || suffix.starts_with('s') {
                    return format!("{}=w544-h544-l90-rj", &url[..eq_idx]);
                }
            }
            return format!("{}=w544-h544-l90-rj", url);
        }
        if !video_id.is_empty() && (url.is_empty() || url.contains("default.jpg")) {
            return format!("https://i.ytimg.com/vi/{}/maxresdefault.jpg", video_id);
        }
        url.to_string()
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

    async fn get_or_fetch_web_visitor_data() -> Option<String> {
        if let Some(existing) = web_visitor_store().lock().clone() {
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
                Self::capture_web_visitor_data(&json);
            }
        }
        web_visitor_store().lock().clone()
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
                    Self::capture_web_visitor_data(&json_data);
                    Self::extract_innertube_tracks(&json_data, &mut results, limit);
                }
            }
        }

        if results.len() < limit {
            if let Ok(resp) = songs_res {
                if resp.status().is_success() {
                    if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                        Self::capture_web_visitor_data(&json_data);
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
        let vr_visitor_data = vr_visitor_store().lock().clone();
        let web_visitor_data = Self::get_or_fetch_web_visitor_data().await;

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
                            // If this was IOS, run a tail-byte probe past 1MB to detect Google Video SABR 403 ceiling
                            if !is_vr {
                                let clen = content_length.unwrap_or(0);
                                if clen > 1_048_576 {
                                    let probe_start = clen.saturating_sub(64);
                                    let probe_end = clen.saturating_sub(33);
                                    let probe_res = http_client()
                                        .get(&upstream_url)
                                        .header("User-Agent", ua)
                                        .header("Range", format!("bytes={}-{}", probe_start, probe_end))
                                        .send()
                                        .await;
                                    if let Ok(probe_resp) = probe_res {
                                        if probe_resp.status().as_u16() == 403 || !probe_resp.status().is_success() {
                                            crate::aethon_log!(
                                                "IOS stream failed tail-byte probe (status {}). Google SABR ceiling active. Skipping.",
                                                probe_resp.status()
                                            );
                                            continue;
                                        }
                                    } else {
                                        continue;
                                    }
                                }
                            }

                            // Pre-buffer 64 KB (bytes=0-65535) directly into <video_id>.part so mpv gets audio bytes in 0 ms
                            let prebuffer_res = http_client()
                                .get(&upstream_url)
                                .header("User-Agent", ua)
                                .header("Range", "bytes=0-65535")
                                .send()
                                .await;

                            let part_path = cache_dir().join(format!("{}.part", video_id));
                            let mut prebuffered_bytes: u64 = 0;
                            if let Ok(pre_resp) = prebuffer_res {
                                if pre_resp.status().is_success() {
                                    if let Ok(p_bytes) = pre_resp.bytes().await {
                                        if !p_bytes.is_empty() {
                                            if let Ok(mut f) = OpenOptions::new()
                                                .create(true)
                                                .write(true)
                                                .truncate(true)
                                                .open(&part_path)
                                            {
                                                let _ = f.write_all(&p_bytes);
                                                let _ = f.flush();
                                                prebuffered_bytes = p_bytes.len() as u64;
                                                crate::aethon_log!(
                                                    "Pre-buffered {} bytes directly into {} for 0ms startup",
                                                    prebuffered_bytes,
                                                    part_path.display()
                                                );
                                            }
                                        }
                                    }
                                }
                            }

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
        if video_id.is_empty() {
            return uri.to_string();
        }

        // Tier 0: Disk Cache Hit (< 5ms startup, offline playback)
        if let Some(cached_file) = check_disk_cache_hit(&video_id) {
            crate::aethon_log!("Tier 0 Disk Cache HIT for video_id={} -> {}", video_id, cached_file);
            return cached_file;
        }

        // If an active download task is already running, return proxy URL immediately
        if active_tasks().lock().contains_key(&video_id) {
            let port = ensure_chunk_proxy_server();
            return format!("http://127.0.0.1:{}/stream/{}", port, video_id);
        }

        // Check in-memory stream cache
        let now = now_unix();
        if let Some(cached) = stream_cache().lock().get(&video_id) {
            if cached.expires_at > now {
                crate::aethon_log!("Stream cache HIT for video_id={}", video_id);
                // Ensure progressive downloader is active
                get_or_spawn_downloader(
                    &video_id,
                    &cached.upstream_url,
                    cached.content_length.unwrap_or(0),
                    &cached.mime_type,
                    &cached.user_agent,
                );
                return cached.playable_url.clone();
            }
        }

        // Tier 1: Fast ~250ms InnerTube ANDROID_VR / ANDROID_TESTSUITE / probed IOS
        if let Some(entry) = Self::resolve_innertube_bounded_stream(&video_id).await {
            let playable = entry.playable_url.clone();
            get_or_spawn_downloader(
                &video_id,
                &entry.upstream_url,
                entry.content_length.unwrap_or(0),
                &entry.mime_type,
                &entry.user_agent,
            );
            stream_cache().lock().insert(video_id.clone(), entry);
            crate::aethon_log!(
                "Resolved Tier 1 InnerTube stream with progressive downloader for video_id={}",
                video_id
            );
            return playable;
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
                let ua = Self::extract_user_agent_for_url(&resolved);
                let mime = Self::extract_mime_from_url(&resolved);
                let clen = Self::extract_clen_from_url(&resolved).unwrap_or(0);
                get_or_spawn_downloader(&vid_owned, &resolved, clen, &mime, &ua);
                stream_cache().lock().insert(
                    vid_owned,
                    CachedStream {
                        playable_url: proxy_url.clone(),
                        upstream_url: resolved,
                        content_length: if clen > 0 { Some(clen) } else { None },
                        mime_type: mime,
                        user_agent: ua,
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
            .arg("youtube:player_client=tv,mweb,web_safari")
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
                .arg("youtube:player_client=tv,mweb,web_safari")
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

                let thumbnail = format!("https://i.ytimg.com/vi/{}/maxresdefault.jpg", id);
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

mod urlencoding {
    pub fn decode(s: &str) -> Result<String, ()> {
        let mut bytes = Vec::new();
        let mut chars = s.bytes();
        while let Some(b) = chars.next() {
            if b == b'%' {
                let h1 = chars.next().ok_or(())?;
                let h2 = chars.next().ok_or(())?;
                let hex_str = [h1, h2];
                let hex_s = std::str::from_utf8(&hex_str).map_err(|_| ())?;
                let val = u8::from_str_radix(hex_s, 16).map_err(|_| ())?;
                bytes.push(val);
            } else if b == b'+' {
                bytes.push(b' ');
            } else {
                bytes.push(b);
            }
        }
        String::from_utf8(bytes).map_err(|_| ())
    }
}
