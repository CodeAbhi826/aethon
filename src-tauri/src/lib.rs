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
use tauri::{Emitter, Manager};

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

            let mpris = Arc::new(player::MprisManager::new(handle.clone(), &mpv.socket_path));

            // Native System Tray configuration
            let menu = tauri::menu::MenuBuilder::new(app)
                .text("tray_toggle_play", "Play / Pause")
                .text("tray_next", "Next Track")
                .text("tray_prev", "Previous Track")
                .separator()
                .text("tray_show", "Show / Hide Aethon")
                .text("tray_quit", "Quit Aethon")
                .build()?;

            let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
                tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).unwrap()
            });

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Aethon")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "tray_toggle_play" => {
                            if let Some(state) = app.try_state::<AppState>() {
                                let mpv = Arc::clone(&state.mpv);
                                tauri::async_runtime::spawn(async move {
                                    let _ = mpv.toggle_pause().await;
                                });
                            }
                        }
                        "tray_next" => {
                            let _ = app.emit("tray-next", ());
                        }
                        "tray_prev" => {
                            let _ = app.emit("tray-prev", ());
                        }
                        "tray_show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                        "tray_quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            app.manage(AppState { mpv, db, mpris });
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
            create_playlist,
            delete_playlist,
            get_playlists,
            add_to_playlist,
            remove_from_playlist,
            get_playlist_tracks,
            set_crossfade,
            set_replaygain,
            reveal_in_file_manager,
            download_track,
            get_play_history,
            send_desktop_notification,
            update_mpris_metadata,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
