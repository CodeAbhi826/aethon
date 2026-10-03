use crate::library::{Database, Scanner, Track};
use crate::lyrics::{LyricsResult, LyricsService};
use crate::player::{MpvEngine, PlaybackState};
use crate::youtube::{YouTubeService, YouTubeTrack};
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
    tokio::task::spawn_blocking(move || {
        YouTubeService::search(&query, limit.unwrap_or(10))
    }).await.map_err(|e| e.to_string())
}
