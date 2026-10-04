use parking_lot::Mutex;
use std::process::{Child, Command, Stdio};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

const MPRIS_SCRIPT: &str = include_str!("../../scripts/mpris_bridge.py");

pub struct MprisManager {
    child: Mutex<Option<Child>>,
    cmd_tx: mpsc::Sender<String>,
    ipc_socket_path: String,
}

impl Drop for MprisManager {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.lock().take() {
            let _ = c.kill();
        }
        let _ = std::fs::remove_file(&self.ipc_socket_path);
    }
}

impl MprisManager {
    pub fn dummy() -> Self {
        let (cmd_tx, _rx) = mpsc::channel(1);
        Self {
            child: Mutex::new(None),
            cmd_tx,
            ipc_socket_path: String::new(),
        }
    }

    pub fn new(app: AppHandle, mpv_socket: &str) -> Self {
        let pid = std::process::id();
        let ipc_socket_path = format!("/tmp/aethon-mpris-{}.sock", pid);
        if std::path::Path::new(&ipc_socket_path).exists() {
            let _ = std::fs::remove_file(&ipc_socket_path);
        }

        // 1. Write the embedded script to /tmp/aethon_mpris_bridge.py
        let bridge_script_path = "/tmp/aethon_mpris_bridge.py";
        let _ = std::fs::write(bridge_script_path, MPRIS_SCRIPT);

        // 2. Bind Unix listener before spawning python process
        let std_listener = match std::os::unix::net::UnixListener::bind(&ipc_socket_path) {
            Ok(l) => {
                let _ = l.set_nonblocking(true);
                Some(l)
            }
            Err(e) => {
                crate::aethon_log!("Failed to bind MPRIS IPC socket {}: {:?}", ipc_socket_path, e);
                None
            }
        };

        // 3. Spawn python3 bridge
        let mut cmd = Command::new("python3");
        cmd.arg(bridge_script_path)
            .arg(&ipc_socket_path)
            .arg(mpv_socket)
            .stdin(Stdio::null())
            .stdout(if crate::is_verbose() { Stdio::inherit() } else { Stdio::null() })
            .stderr(if crate::is_verbose() { Stdio::inherit() } else { Stdio::null() });

        // Clean AppImage env vars so python uses native system libs
        if let Ok(ld) = std::env::var("LD_LIBRARY_PATH") {
            let filtered: Vec<&str> = ld
                .split(':')
                .filter(|p| !p.contains(".mount_") && !p.is_empty())
                .collect();
            if filtered.is_empty() {
                cmd.env_remove("LD_LIBRARY_PATH");
            } else {
                cmd.env("LD_LIBRARY_PATH", filtered.join(":"));
            }
        }
        if let Ok(py) = std::env::var("PYTHONPATH") {
            if py.contains(".mount_") {
                cmd.env_remove("PYTHONPATH");
            }
        }
        if let Ok(pyh) = std::env::var("PYTHONHOME") {
            if pyh.contains(".mount_") {
                cmd.env_remove("PYTHONHOME");
            }
        }

        let child = match cmd.spawn() {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("[Aethon] Could not spawn MPRIS Python bridge: {:?}", e);
                None
            }
        };

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(32);

        // 4. Handle incoming connection and command dispatching
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

                // Wait for python bridge to connect
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let (reader, mut writer) = stream.into_split();

                        // Task A: Write commands to Python bridge
                        tauri::async_runtime::spawn(async move {
                            while let Some(msg) = cmd_rx.recv().await {
                                let mut line = msg;
                                if !line.ends_with('\n') {
                                    line.push('\n');
                                }
                                if writer.write_all(line.as_bytes()).await.is_err() {
                                    break;
                                }
                            }
                        });

                        // Task B: Read events from Python bridge (next, prev, etc.)
                        let mut buf_reader = BufReader::new(reader);
                        let mut line = String::new();
                        while let Ok(n) = buf_reader.read_line(&mut line).await {
                            if n == 0 {
                                break;
                            }
                            let trimmed = line.trim();
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                                if let Some(event) = v.get("event").and_then(|e| e.as_str()) {
                                    match event {
                                        "next" => {
                                            let _ = app_handle.emit("mpris-next", ());
                                        }
                                        "prev" => {
                                            let _ = app_handle.emit("mpris-prev", ());
                                        }
                                        "playpause" => {
                                            let _ = app_handle.emit("mpris-playpause", ());
                                        }
                                        "raise" => {
                                            if let Some(win) = app_handle.get_webview_window("main") {
                                                let _ = win.show();
                                                let _ = win.set_focus();
                                            }
                                        }
                                        "quit" => {
                                            app_handle.exit(0);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            line.clear();
                        }
                    }
                    Err(e) => {
                        eprintln!("[Aethon] MPRIS IPC listener accept error: {:?}", e);
                    }
                }
            });
        }

        Self {
            child: Mutex::new(child),
            cmd_tx,
            ipc_socket_path,
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

    pub async fn update_status(&self, is_playing: bool, position_us: i64) -> Result<(), String> {
        let msg = serde_json::json!({
            "action": "status",
            "is_playing": is_playing,
            "position_us": position_us,
        });
        self.cmd_tx
            .send(msg.to_string())
            .await
            .map_err(|e| e.to_string())
    }
}
