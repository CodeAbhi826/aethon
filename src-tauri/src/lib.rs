pub mod commands;
pub mod library;
pub mod lyrics;
pub mod player;
pub mod youtube;

use commands::*;
use library::Database;
use player::MpvEngine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Manager;

pub static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

#[macro_export]
macro_rules! aethon_log {
    ($($arg:tt)*) => {
        if $crate::is_verbose() {
            eprintln!("\x1b[36m[Aethon Verbose]\x1b[0m {}", format_args!($($arg)*));
        }
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let verbose_enabled = std::env::args().any(|arg| {
        arg == "--verbose" || arg == "-v" || arg == "--debug"
    });
    VERBOSE.store(verbose_enabled, Ordering::Relaxed);

    if verbose_enabled {
        eprintln!("\x1b[32m[Aethon]\x1b[0m Verbose debug logging enabled via CLI flag (--verbose / -v)");
    }

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
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    match tauri::async_runtime::block_on(async {
                        MpvEngine::new(Some(handle.clone())).await
                    }) {
                        Ok(retry_engine) => Arc::new(retry_engine),
                        Err(err) => {
                            eprintln!("[Aethon Error] MPV engine initialization failed: {:?}. Launching in safe mode.", err);
                            Arc::new(MpvEngine::dummy())
                        }
                    }
                }
            };

            app.manage(AppState { mpv, db });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app_handle = window.app_handle();
                if let Some(state) = app_handle.try_state::<AppState>() {
                    let is_close_to_tray = state
                        .db
                        .get_setting("closeToTray")
                        .or_else(|_| state.db.get_setting("close_to_tray"))
                        .ok()
                        .flatten()
                        .map(|val| val == "true")
                        .unwrap_or(true);

                    if is_close_to_tray {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                }
            }
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
            get_all_settings,
            get_setting,
            set_setting,
            set_settings_batch,
            get_stream_cache_size,
            clear_stream_cache,
            show_window,
            window_minimize,
            window_toggle_maximize,
            window_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
