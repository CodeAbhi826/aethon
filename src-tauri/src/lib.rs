pub mod commands;
pub mod library;
pub mod lyrics;
pub mod player;
pub mod youtube;

use commands::*;
use library::Database;
use player::MpvEngine;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let db = Arc::new(Database::new().expect("Failed to initialize SQLite database"));

            let mpv = match tauri::async_runtime::block_on(async {
                MpvEngine::new(Some(handle.clone())).await
            }) {
                Ok(engine) => Arc::new(engine),
                Err(e) => {
                    eprintln!("[Aethon Warning] First attempt to start MPV failed: {:?}. Retrying...", e);
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    let retry_engine = tauri::async_runtime::block_on(async {
                        MpvEngine::new(Some(handle.clone())).await
                    }).expect("Fatal: Could not initialize audio engine (is mpv installed?)");
                    Arc::new(retry_engine)
                }
            };

            app.manage(AppState { mpv, db });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            play_track,
            toggle_play,
            pause_track,
            resume_track,
            seek_track,
            set_volume,
            set_equalizer,
            get_playback_state,
            scan_library,
            get_library_tracks,
            toggle_favorite,
            get_lyrics,
            search_youtube,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
