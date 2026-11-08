// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    unsafe {
        if std::env::var("MALLOC_ARENA_MAX").is_err() {
            std::env::set_var("MALLOC_ARENA_MAX", "2");
        }
        // Force hardware GPU acceleration and zero-copy DMABUF compositing on Intel/Mesa GPUs
        if std::env::var("WEBKIT_FORCE_COMPOSITING_MODE").is_err() {
            std::env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");
        }
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "0");
        }
    }

    aethon_lib::run()
}

