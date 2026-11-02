use crate::commands::AppState;
use parking_lot::Mutex;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

const MPRIS_SCRIPT: &str = include_str!("../../scripts/mpris_bridge.py");

pub struct MprisManager {
    child: Mutex<Option<Child>>,
    cmd_tx: mpsc::Sender<String>,
    ipc_socket_path: String,
    bridge_script_path: String,
}

impl Drop for MprisManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl MprisManager {
    pub fn shutdown(&self) {
        if let Some(mut c) = self.child.lock().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = std::fs::remove_file(&self.ipc_socket_path);
        let _ = std::fs::remove_file(&self.bridge_script_path);
    }

    pub fn new(app: AppHandle, _mpv_socket: &str) -> Self {
        let pid = std::process::id();
        let runtime_dir = crate::runtime_socket_dir();
        let ipc_socket_path = runtime_dir
            .join(format!("aethon-mpris-{}.sock", pid))
            .to_string_lossy()
            .to_string();
        let bridge_script_path = runtime_dir
            .join(format!("aethon_mpris_bridge_{}.py", pid))
            .to_string_lossy()
            .to_string();

        if std::path::Path::new(&ipc_socket_path).exists() {
            let _ = std::fs::remove_file(&ipc_socket_path);
        }

        let _ = std::fs::write(&bridge_script_path, MPRIS_SCRIPT);

        let std_listener = match std::os::unix::net::UnixListener::bind(&ipc_socket_path) {
            Ok(l) => {
                let _ = l.set_nonblocking(true);
                Some(l)
            }
            Err(e) => {
                crate::aethon_log!(
                    "Failed to bind MPRIS IPC socket {}: {:?}",
                    ipc_socket_path,
                    e
                );
                None
            }
        };

        let mut cmd = Command::new("python3");
        cmd.arg("-O")
            .arg(&bridge_script_path)
            .arg(&ipc_socket_path)
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(if crate::is_verbose() {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if crate::is_verbose() {
                Stdio::inherit()
            } else {
                Stdio::null()
            });

        crate::sanitize_host_command(&mut cmd);
        cmd.env("PYTHONOPTIMIZE", "2");
        cmd.env("PYTHONDONTWRITEBYTECODE", "1");

        let child = match cmd.spawn() {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("[Aethon] Could not spawn MPRIS Python bridge: {:?}", e);
                None
            }
        };

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(32);

        if let Some(std_l) = std_listener {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let listener = match tokio::net::UnixListener::from_std(std_l) {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("[Aethon] MPRIS UnixListener from_std failed: {:?}", e);
                        return;
                    }
                };

                while let Ok((stream, _)) = listener.accept().await {
                    let (reader, mut writer) = stream.into_split();
                    let app_inner = app_handle.clone();

                    let mut buf_reader = BufReader::new(reader);
                    let mut line = String::new();
                    loop {
                        tokio::select! {
                            msg_opt = cmd_rx.recv() => {
                                match msg_opt {
                                    Some(mut msg) => {
                                        if !msg.ends_with('\n') {
                                            msg.push('\n');
                                        }
                                        if writer.write_all(msg.as_bytes()).await.is_err() {
                                            break;
                                        }
                                    }
                                    None => return,
                                }
                            }
                            read_res = buf_reader.read_line(&mut line) => {
                                match read_res {
                                    Ok(0) | Err(_) => break,
                                    Ok(_) => {
                                        let trimmed = line.trim();
                                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                                            if let Some(event) = v.get("event").and_then(|e| e.as_str()) {
                                                match event {
                                                    "next" => { let _ = app_inner.emit("mpris-next", ()); }
                                                    "prev" => { let _ = app_inner.emit("mpris-prev", ()); }
                                                    "playpause" => { let _ = app_inner.emit("mpris-playpause", ()); }
                                                    "play" => { let _ = app_inner.emit("mpris-play", ()); }
                                                    "pause" | "stop" => { let _ = app_inner.emit("mpris-pause", ()); }
                                                    "seek" => {
                                                        if let Some(offset) = v.get("offset").and_then(|o| o.as_f64()) {
                                                            if let Some(app_state) = app_inner.try_state::<AppState>() {
                                                                let mpv = Arc::clone(&app_state.mpv);
                                                                tauri::async_runtime::spawn(async move {
                                                                    let cur = mpv.state.read().position;
                                                                    let _ = mpv.seek((cur + offset).max(0.0)).await;
                                                                });
                                                            }
                                                        }
                                                    }
                                                    "set_position" => {
                                                        if let Some(pos) = v.get("position").and_then(|p| p.as_f64()) {
                                                            if let Some(app_state) = app_inner.try_state::<AppState>() {
                                                                let mpv = Arc::clone(&app_state.mpv);
                                                                tauri::async_runtime::spawn(async move {
                                                                    let _ = mpv.seek(pos.max(0.0)).await;
                                                                });
                                                            }
                                                        }
                                                    }
                                                    "volume" => {
                                                        if let Some(vol) = v.get("volume").and_then(|p| p.as_f64()) {
                                                            if let Some(app_state) = app_inner.try_state::<AppState>() {
                                                                let mpv = Arc::clone(&app_state.mpv);
                                                                tauri::async_runtime::spawn(async move {
                                                                    let _ = mpv.set_volume(vol).await;
                                                                });
                                                            }
                                                        }
                                                    }
                                                    "raise" => {
                                                        if let Some(win) = app_inner.get_webview_window("main") {
                                                            let _ = win.show();
                                                            let _ = win.set_focus();
                                                        }
                                                    }
                                                    "quit" => {
                                                        crate::shutdown_daemons(&app_inner);
                                                        app_inner.exit(0);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                        line.clear();
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        Self {
            child: Mutex::new(child),
            cmd_tx,
            ipc_socket_path,
            bridge_script_path,
        }
    }

    pub async fn update_metadata(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        cover: &str,
        duration_us: i64,
        is_playing: bool,
    ) -> Result<(), String> {
        let msg = serde_json::json!({
            "action": "update",
            "title": title,
            "artist": artist,
            "album": album,
            "art_url": cover,
            "duration_us": duration_us,
            "is_playing": is_playing,
        });
        self.cmd_tx
            .send(msg.to_string())
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_status(
        &self,
        is_playing: bool,
        position_us: i64,
        seeked: bool,
    ) -> Result<(), String> {
        let msg = serde_json::json!({
            "action": "status",
            "is_playing": is_playing,
            "position_us": position_us,
            "seeked": seeked,
        });
        self.cmd_tx
            .send(msg.to_string())
            .await
            .map_err(|e| e.to_string())
    }
}


