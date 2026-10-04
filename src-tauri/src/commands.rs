use crate::library::{Database, Scanner, Track};
use crate::lyrics::{LyricsResult, LyricsService};
use crate::player::{MpvEngine, PlaybackState};
use crate::youtube::{YouTubeService, YouTubeTrack};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub mpv: Arc<MpvEngine>,
    pub db: Arc<Database>,
}

#[tauri::command]
pub async fn play_track(
    uri: String,
    track_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::aethon_log!("play_track: uri={}, track_id={:?}", uri, track_id);
    state.mpv.load_file(&uri).await.map_err(|e| e.to_string())?;
    if let Some(id) = track_id {
        let _ = state.db.record_play(&id);
    }
    Ok(())
}

#[tauri::command]
pub async fn toggle_play(state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("toggle_play invoked");
    state.mpv.toggle_pause().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_track(state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("pause_track invoked");
    state.mpv.pause().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume_track(state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("resume_track invoked");
    state.mpv.play().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_track(position: f64, state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("seek_track: position={:.2}s", position);
    state.mpv.seek(position).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_volume(volume: f64, state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("set_volume: volume={:.1}%", volume);
    state.mpv.set_volume(volume).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_equalizer(bands: Vec<f64>, state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("set_equalizer: bands={:?}", bands);
    state.mpv.set_equalizer(&bands).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_playback_state(state: State<'_, AppState>) -> PlaybackState {
    state.mpv.get_state()
}

#[tauri::command]
pub async fn scan_library(
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Track>, String> {
    crate::aethon_log!("scan_library: path={:?}", path);
    let p = path.map(std::path::PathBuf::from);
    let db = Arc::clone(&state.db);
    let tracks = tokio::task::spawn_blocking(move || {
        Scanner::scan_directory(p, &db)
    }).await.map_err(|e| e.to_string())?;

    crate::aethon_log!("scan_library finished: {} tracks indexed", tracks.len());
    Ok(tracks)
}

#[tauri::command]
pub fn get_library_tracks(state: State<'_, AppState>) -> Result<Vec<Track>, String> {
    state.db.get_all_tracks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_favorite(track_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    crate::aethon_log!("toggle_favorite: track_id={}", track_id);
    state.db.toggle_favorite(&track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_lyrics(
    title: String,
    artist: String,
    duration: Option<f64>,
) -> Result<LyricsResult, String> {
    crate::aethon_log!("get_lyrics: title='{}', artist='{}'", title, artist);
    LyricsService::fetch_lyrics(&title, &artist, duration)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_youtube(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<YouTubeTrack>, String> {
    crate::aethon_log!("search_youtube: query='{}'", query);
    Ok(YouTubeService::search(&query, limit.unwrap_or(10)).await)
}

#[tauri::command]
pub fn get_all_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    state.db.get_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_setting(key: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    state.db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("set_setting: {}={}", key, value);
    state.db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_settings_batch(
    settings: HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::aethon_log!("set_settings_batch: {} settings keys", settings.len());
    state.db.set_settings_batch(&settings).map_err(|e| e.to_string())
}

fn calculate_dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    total += calculate_dir_size(&entry.path());
                }
            }
        }
    }
    total
}

fn remove_dir_contents(path: &std::path::Path) -> u64 {
    let mut freed = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(meta) = entry.metadata() {
                let len = meta.len();
                if meta.is_file() {
                    if std::fs::remove_file(&p).is_ok() {
                        freed += len;
                    }
                } else if meta.is_dir() {
                    freed += remove_dir_contents(&p);
                    let _ = std::fs::remove_dir(&p);
                }
            }
        }
    }
    freed
}

#[tauri::command]
pub fn get_stream_cache_size() -> Result<u64, String> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aethon")
        .join("streams");
    if !cache_dir.exists() {
        return Ok(0);
    }
    Ok(calculate_dir_size(&cache_dir))
}

#[tauri::command]
pub fn clear_stream_cache() -> Result<u64, String> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aethon")
        .join("streams");
    if !cache_dir.exists() {
        return Ok(0);
    }
    let freed_bytes = remove_dir_contents(&cache_dir);
    crate::aethon_log!("clear_stream_cache: freed {} bytes", freed_bytes);
    Ok(freed_bytes)
}

#[tauri::command]
pub fn show_window(window: tauri::Window) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn window_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn window_toggle_maximize(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn window_close(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

