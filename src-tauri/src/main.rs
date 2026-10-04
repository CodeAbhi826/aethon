// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    unsafe {
        if std::env::var("MALLOC_ARENA_MAX").is_err() {
            std::env::set_var("MALLOC_ARENA_MAX", "2");
        }
    }

    aethon_lib::run()
}
