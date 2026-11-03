use crate::library::{Database, Playlist, Scanner, Track};
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
    pub mpris: Arc<crate::player::MprisManager>,
}

#[tauri::command]
pub async fn play_track(
    uri: String,
    track_id: Option<String>,
    track: Option<Track>,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::aethon_log!("play_track: uri={}, track_id={:?}", uri, track_id);
    let (t_title, t_artist) = if let Some(ref t) = track {
        (t.title.clone(), t.artist.clone())
    } else {
        (String::new(), String::new())
    };

    let full_uri = if uri.starts_with("watch?v=") {
        format!("https://www.youtube.com/{}", uri)
    } else if uri.starts_with('/') && uri.contains("watch?v=") {
        format!("https://www.youtube.com{}", uri)
    } else if let Some(stripped) = uri.strip_prefix("yt_") {
        format!("https://www.youtube.com/watch?v={}", stripped)
    } else if uri.len() == 11 && !uri.contains('/') && !uri.contains('.') {
        format!("https://www.youtube.com/watch?v={}", uri)
    } else {
        uri
    };

    if force_refresh.unwrap_or(false) {
        YouTubeService::invalidate_cached_stream(&full_uri);
    }

    // Mark engine as buffering before stream resolution so no stale player-state event clears isLoading
    {
        let mut s = state.mpv.state.write();
        s.current_uri = Some(full_uri.clone());
        s.media_title = if t_title.is_empty() { None } else { Some(t_title.clone()) };
        s.position = 0.0;
        s.is_buffering = true;
        s.is_playing = false;
    }

    let playable_uri =
        YouTubeService::resolve_playable_stream_async(&full_uri, &t_title, &t_artist).await;

    state
        .mpv
        .load_file_with_canonical(
            &playable_uri,
            Some(&full_uri),
            if t_title.is_empty() { None } else { Some(&t_title) },
        )
        .await
        .map_err(|e| e.to_string())?;

    if let Some(t) = track {
        let _ = state.db.upsert_track(&t);
        let _ = state.db.record_play(&t.id);
        let _ = state.mpris.update_metadata(
            &t.title,
            &t.artist,
            &t.album,
            t.cover_art.as_deref().unwrap_or(""),
            (t.duration * 1_000_000.0) as i64,
            true,
        ).await;
    } else if let Some(id) = track_id {
        let _ = state.db.record_play(&id);
    }
    Ok(())
}

#[tauri::command]
pub async fn prefetch_stream(
    uri: String,
    title: Option<String>,
    artist: Option<String>,
) -> Result<(), String> {
    let t = title.unwrap_or_default();
    let a = artist.unwrap_or_default();
    let _ = YouTubeService::resolve_playable_stream_async(&uri, &t, &a).await;
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
    state
        .mpv
        .set_equalizer(&bands)
        .await
        .map_err(|e| e.to_string())
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
    let tracks = tokio::task::spawn_blocking(move || Scanner::scan_directory(p, &db))
        .await
        .map_err(|e| e.to_string())?;

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
    state
        .db
        .toggle_favorite(&track_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_lyrics(
    title: String,
    artist: String,
    duration: Option<f64>,
    file_path: Option<String>,
    video_id: Option<String>,
) -> Result<LyricsResult, String> {
    crate::aethon_log!("get_lyrics: title='{}', artist='{}'", title, artist);
    LyricsService::fetch_lyrics(
        &title,
        &artist,
        duration,
        file_path.as_deref(),
        video_id.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_youtube(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<YouTubeTrack>, String> {
    crate::aethon_log!("search_youtube: query='{}'", query);
    let gl = state
        .db
        .get_setting("ytmRegion")
        .ok()
        .flatten()
        .unwrap_or_else(|| "IN".to_string());
    Ok(YouTubeService::search(&query, limit.unwrap_or(15), &gl).await)
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
    state
        .db
        .set_setting(&key, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_settings_batch(
    settings: HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::aethon_log!("set_settings_batch: {} settings keys", settings.len());
    state
        .db
        .set_settings_batch(&settings)
        .map_err(|e| e.to_string())
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

#[tauri::command]
pub fn create_playlist(
    name: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<Playlist, String> {
    crate::aethon_log!("create_playlist: name='{}'", name);
    state
        .db
        .create_playlist(&name, description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_playlist(playlist_id: String, state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("delete_playlist: id='{}'", playlist_id);
    state
        .db
        .delete_playlist(&playlist_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    state.db.get_playlists().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_to_playlist(
    playlist_id: String,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::aethon_log!("add_to_playlist: pl='{}', track='{}'", playlist_id, track_id);
    state
        .db
        .add_to_playlist(&playlist_id, &track_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_playlist(
    playlist_id: String,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::aethon_log!(
        "remove_from_playlist: pl='{}', track='{}'",
        playlist_id,
        track_id
    );
    state
        .db
        .remove_from_playlist(&playlist_id, &track_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_playlist_tracks(
    playlist_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Track>, String> {
    state
        .db
        .get_playlist_tracks(&playlist_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_crossfade(seconds: f64, state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("set_crossfade: {:.1}s", seconds);
    state
        .mpv
        .set_crossfade(seconds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_replaygain(mode: String, state: State<'_, AppState>) -> Result<(), String> {
    crate::aethon_log!("set_replaygain: {}", mode);
    state
        .mpv
        .set_replaygain(&mode)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reveal_in_file_manager(path: String) -> Result<(), String> {
    crate::aethon_log!("reveal_in_file_manager: path='{}'", path);
    if path.starts_with("http://") || path.starts_with("https://") {
        return Ok(());
    }
    let p = std::path::Path::new(&path);
    let target = if p.is_dir() {
        p
    } else {
        p.parent().unwrap_or(p)
    };
    let mut cmd = std::process::Command::new("xdg-open");
    crate::sanitize_host_command(&mut cmd);
    let _ = cmd.arg(target).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn download_track(
    id: String,
    title: String,
    artist: String,
    url: String,
    thumbnail: Option<String>,
    state: State<'_, AppState>,
) -> Result<Track, String> {
    crate::aethon_log!(
        "download_track: id='{}', title='{}', artist='{}'",
        id,
        title,
        artist
    );
    let db = Arc::clone(&state.db);

    tokio::task::spawn_blocking(move || {
        let music_dir = dirs::audio_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("Music")
            })
            .join("Aethon Downloads");
        let _ = std::fs::create_dir_all(&music_dir);

        let sanitized_title = title.replace('/', "-").replace('\\', "-");
        let out_path = music_dir.join(format!("{}.mp3", sanitized_title));
        let out_template = music_dir.join("%(title)s [%(id)s].%(ext)s");

        let mut cmd = std::process::Command::new("yt-dlp");
        cmd.arg("-x")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0")
            .arg("--embed-thumbnail")
            .arg("--add-metadata")
            .arg("--no-playlist")
            .arg("--print")
            .arg("after_move:filepath")
            .arg("-o")
            .arg(out_template.to_string_lossy().to_string())
            .arg("--")
            .arg(&url);

        crate::sanitize_host_command(&mut cmd);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("yt-dlp download failed: {}", err));
        }

        let printed = String::from_utf8_lossy(&output.stdout);
        let actual_path = printed
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .map(|l| PathBuf::from(l.trim()))
            .filter(|p| p.exists())
            .unwrap_or(out_path);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut track = Track {
            id: format!("dl_{}", id),
            title: title.clone(),
            artist: artist.clone(),
            album: "Aethon Downloads".to_string(),
            duration: 0.0,
            file_path: actual_path.to_string_lossy().to_string(),
            cover_art: thumbnail,
            format: "MP3 320k".to_string(),
            bitrate: Some(320),
            sample_rate: Some(44100),
            play_count: 0,
            favorite: false,
            date_added: now,
        };

        if let Ok(tagged) = lofty::probe::Probe::open(&actual_path).and_then(|p| p.read()) {
            use lofty::file::AudioFile;
            let props = tagged.properties();
            track.duration = props.duration().as_secs_f64();
            track.bitrate = props.audio_bitrate().map(|b| b as i64);
            track.sample_rate = props.sample_rate().map(|s| s as i64);
        }

        let _ = db.upsert_track(&track);
        Ok(track)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_play_history(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<Track>, String> {
    state
        .db
        .get_play_history(limit.unwrap_or(20))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_desktop_notification(
    title: String,
    artist: String,
    cover_art: Option<String>,
) -> Result<(), String> {
    let mut icon_arg = "multimedia-audio-player".to_string();
    if let Some(ref icon) = cover_art {
        if icon.starts_with("http://") || icon.starts_with("https://") {
            let cache_path = crate::runtime_socket_dir()
                .join(format!("aethon_notif_art_{}.jpg", std::process::id()));
            if let Ok(resp) = reqwest::get(icon).await {
                if let Ok(bytes) = resp.bytes().await {
                    if std::fs::write(&cache_path, bytes).is_ok() {
                        icon_arg = cache_path.to_string_lossy().to_string();
                    }
                }
            }
        } else if std::path::Path::new(icon).exists() {
            icon_arg = icon.clone();
        }
    }
    let mut cmd = std::process::Command::new("notify-send");
    crate::sanitize_host_command(&mut cmd);
    cmd.arg("-a").arg("Aethon");
    cmd.arg("-c").arg("media");
    cmd.arg("-i").arg(&icon_arg);
    cmd.arg(&title).arg(format!("{} • Aethon", artist));
    let _ = cmd.spawn();
    Ok(())
}

#[tauri::command]
pub async fn update_mpris_metadata(
    title: String,
    artist: String,
    album: String,
    cover: String,
    duration_us: i64,
    is_playing: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .mpris
        .update_metadata(&title, &artist, &album, &cover, duration_us, is_playing)
        .await
}

#[tauri::command]
pub async fn update_mpris_status(
    is_playing: bool,
    position_us: i64,
    seeked: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .mpris
        .update_status(is_playing, position_us, seeked.unwrap_or(false))
        .await
}

#[tauri::command]
pub async fn open_ytm_login(app: tauri::AppHandle) -> Result<(), String> {
    crate::youtube::auth_sync::open_ytm_login_window(app).await
}

#[tauri::command]
pub async fn fetch_ytm_profile(
    cookie: String,
    visitor_data: Option<String>,
) -> Result<crate::youtube::auth_sync::YtmAccountProfile, String> {
    crate::youtube::auth_sync::fetch_ytm_account_profile(&cookie, visitor_data.as_deref()).await
}

#[tauri::command]
pub async fn sync_ytm_library(
    cookie: String,
    visitor_data: Option<String>,
    page_id: Option<String>,
    gl: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::youtube::auth_sync::YtmSyncSummary, String> {
    let gl_str = gl.unwrap_or_else(|| "IN".to_string());
    crate::youtube::auth_sync::sync_ytm_library_internal(
        &cookie,
        visitor_data.as_deref(),
        page_id.as_deref(),
        &gl_str,
        &state.db,
    )
    .await
}

#[tauri::command]
pub async fn set_ytm_like(
    cookie: String,
    visitor_data: Option<String>,
    page_id: Option<String>,
    video_id: String,
    like: bool,
) -> Result<(), String> {
    crate::youtube::auth_sync::set_ytm_like_status(
        &cookie,
        visitor_data.as_deref(),
        page_id.as_deref(),
        &video_id,
        like,
    )
    .await
}

#[tauri::command]
pub async fn init_recap_playback_tracking(
    cookie: String,
    visitor_data: Option<String>,
    page_id: Option<String>,
    video_id: String,
) -> Result<crate::youtube::auth_sync::PlaybackTrackingSession, String> {
    crate::youtube::auth_sync::init_recap_playback_tracking(
        &cookie,
        visitor_data.as_deref(),
        page_id.as_deref(),
        &video_id,
    )
    .await
}

#[tauri::command]
pub async fn ping_recap_watchtime(
    cookie: String,
    visitor_data: Option<String>,
    session: crate::youtube::auth_sync::PlaybackTrackingSession,
    start_sec: f64,
    end_sec: f64,
    is_playing: bool,
) -> Result<(), String> {
    crate::youtube::auth_sync::ping_recap_watchtime(
        &cookie,
        visitor_data.as_deref(),
        &session,
        start_sec,
        end_sec,
        is_playing,
    )
    .await
}
