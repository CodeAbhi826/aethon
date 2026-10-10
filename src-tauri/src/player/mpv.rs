use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub current_uri: Option<String>,
    pub is_playing: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
    pub codec: Option<String>,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            current_uri: None,
            is_playing: false,
            position: 0.0,
            duration: 0.0,
            volume: 100.0,
            codec: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
        }
    }
}

pub struct MpvEngine {
    process: parking_lot::Mutex<Option<Child>>,
    cmd_tx: mpsc::Sender<String>,
    pub socket_path: String,
    pub state: Arc<RwLock<PlaybackState>>,
}

impl Drop for MpvEngine {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.lock().take() {
            let _ = child.kill();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

impl MpvEngine {
    pub fn dummy() -> Self {
        let (cmd_tx, _cmd_rx) = mpsc::channel(1);
        Self {
            process: parking_lot::Mutex::new(None),
            cmd_tx,
            socket_path: String::new(),
            state: Arc::new(RwLock::new(PlaybackState::default())),
        }
    }

    pub async fn new(app: Option<AppHandle>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pid = std::process::id();
        let socket_path = format!("/tmp/aethon-mpv-{}.sock", pid);

        // 1. Remove old socket file if it exists
        if std::path::Path::new(&socket_path).exists() {
            let _ = std::fs::remove_file(&socket_path);
        }

        // 2. Locate mpv binary (handle AppImage PATH differences)
        let mpv_bin = if std::path::Path::new("/usr/bin/mpv").exists() {
            "/usr/bin/mpv"
        } else if std::path::Path::new("/usr/local/bin/mpv").exists() {
            "/usr/local/bin/mpv"
        } else {
            "mpv"
        };

        let (stdout_cfg, stderr_cfg) = if crate::is_verbose() {
            (Stdio::inherit(), Stdio::inherit())
        } else {
            (Stdio::null(), Stdio::null())
        };

        crate::aethon_log!("Spawning MPV engine: {} (socket: {})", mpv_bin, socket_path);

        // Prepare stream cache directory (~/.cache/aethon/streams/)
        let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        cache_dir.push("aethon");
        cache_dir.push("streams");
        let _ = std::fs::create_dir_all(&cache_dir);
        let cache_dir_str = cache_dir.to_string_lossy().to_string();

        // 3. Spawn headless MPV process
        let mut cmd = Command::new(mpv_bin);
        cmd.arg("--idle=yes")
            .arg("--no-video")
            .arg("--no-terminal")
            .arg(format!("--input-ipc-server={}", socket_path))
            .arg("--ytdl=yes")
            .arg("--ytdl-format=bestaudio/best")
            .arg("--keep-open=yes")
            .arg("--gapless-audio=yes")
            .arg("--cache=yes")
            .arg("--cache-on-disk=yes")
            .arg(format!("--demuxer-cache-dir={}", cache_dir_str))
            .arg("--demuxer-cache-unlink-files=no")
            .arg("--demuxer-max-bytes=16777216")
            .arg("--demuxer-max-back-bytes=8388608")
            .arg("--demuxer-readahead-secs=60")
            .arg("--ao=pipewire,pulse,alsa")
            .stdin(Stdio::null())
            .stdout(stdout_cfg)
            .stderr(stderr_cfg);

        // AppImage isolation: Clean host process environment completely so host mpv and yt-dlp use Arch Linux native host libraries and python runtime
        crate::sanitize_host_command(&mut cmd);

        let child = cmd.spawn()?;

        // 4. Wait up to 3 seconds for socket to be created
        let mut connected = false;
        let mut stream = None;
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(s) = UnixStream::connect(&socket_path).await {
                stream = Some(s);
                connected = true;
                break;
            }
        }

        if !connected {
            return Err("Failed to connect to mpv socket".into());
        }

        let stream = stream.unwrap();
        let (reader, mut writer) = stream.into_split();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(64);

        let state = Arc::new(RwLock::new(PlaybackState::default()));
        let state_clone = Arc::clone(&state);

        // Task: Write commands to mpv socket
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let mut line = cmd;
                if !line.ends_with('\n') {
                    line.push('\n');
                }
                if let Err(e) = writer.write_all(line.as_bytes()).await {
                    eprintln!("[mpv IPC write error]: {:?}", e);
                    break;
                }
            }
        });

        // Task: Read events and property changes from mpv socket
        let app_handle = app.clone();
        tokio::spawn(async move {
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            while let Ok(n) = buf_reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                let trimmed = line.trim();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(event) = val.get("event").and_then(|e| e.as_str()) {
                        match event {
                            "property-change" => {
                                if let Some(name) = val.get("name").and_then(|n| n.as_str()) {
                                    let mut s = state_clone.write();
                                    let data = val.get("data");
                                    match name {
                                        "time-pos" => {
                                            if let Some(t) = data.and_then(|d| d.as_f64()) {
                                                s.position = t;
                                            }
                                        }
                                        "duration" => {
                                            if let Some(d) = data.and_then(|d| d.as_f64()) {
                                                s.duration = d;
                                            }
                                        }
                                        "pause" => {
                                            if let Some(p) = data.and_then(|d| d.as_bool()) {
                                                s.is_playing = s.current_uri.is_some() && !p;
                                            }
                                        }
                                        "volume" => {
                                            if let Some(v) = data.and_then(|d| d.as_f64()) {
                                                s.volume = v;
                                            }
                                        }
                                        "audio-codec-name" => {
                                            s.codec = data.and_then(|d| d.as_str()).map(String::from);
                                        }
                                        "audio-bitrate" => {
                                            s.bitrate = data.and_then(|d| d.as_i64());
                                        }
                                        "audio-params" => {
                                            if let Some(obj) = data.and_then(|d| d.as_object()) {
                                                s.sample_rate = obj.get("samplerate").and_then(|sr| sr.as_i64());
                                                s.channels = obj.get("channel-count").and_then(|c| c.as_i64());
                                            }
                                        }
                                        _ => {}
                                    }

                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit("player-state", &*s);
                                    }
                                }
                            }
                            "end-file" => {
                                let reason = val.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                                crate::aethon_log!("MPV end-file event received: reason={}", reason);
                                if reason == "eof" {
                                    let mut s = state_clone.write();
                                    s.is_playing = false;
                                    s.position = 0.0;
                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit("player-state", &*s);
                                        let _ = handle.emit("track-ended", ());
                                    }
                                } else if reason == "error" {
                                    let mut s = state_clone.write();
                                    s.is_playing = false;
                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit("player-state", &*s);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                line.clear();
            }
        });

        let engine = Self {
            process: parking_lot::Mutex::new(Some(child)),
            cmd_tx,
            socket_path,
            state,
        };

        // Register property observers
        engine.send_cmd(r#"{"command": ["observe_property", 1, "time-pos"]}"#).await?;
        engine.send_cmd(r#"{"command": ["observe_property", 2, "duration"]}"#).await?;
        engine.send_cmd(r#"{"command": ["observe_property", 3, "pause"]}"#).await?;
        engine.send_cmd(r#"{"command": ["observe_property", 4, "volume"]}"#).await?;
        engine.send_cmd(r#"{"command": ["observe_property", 5, "audio-codec-name"]}"#).await?;
        engine.send_cmd(r#"{"command": ["observe_property", 6, "audio-params"]}"#).await?;
        engine.send_cmd(r#"{"command": ["observe_property", 7, "audio-bitrate"]}"#).await?;

        Ok(engine)
    }

    pub async fn send_cmd(&self, json_cmd: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crate::aethon_log!("MPV IPC >> {}", json_cmd);
        self.cmd_tx.send(json_cmd.to_string()).await?;
        Ok(())
    }

    pub async fn load_file(&self, uri: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        {
            let mut s = self.state.write();
            s.current_uri = Some(uri.to_string());
            s.position = 0.0;
            s.is_playing = true;
        }
        let msg = serde_json::json!({
            "command": ["loadfile", uri, "replace"]
        });
        self.send_cmd(&msg.to_string()).await?;
        let play_msg = serde_json::json!({
            "command": ["set_property", "pause", false]
        });
        self.send_cmd(&play_msg.to_string()).await?;
        Ok(())
    }

    pub async fn play(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg = serde_json::json!({
            "command": ["set_property", "pause", false]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn pause(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg = serde_json::json!({
            "command": ["set_property", "pause", true]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn toggle_pause(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg = serde_json::json!({
            "command": ["cycle", "pause"]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn seek(&self, seconds: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg = serde_json::json!({
            "command": ["seek", seconds, "absolute"]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn set_volume(&self, vol: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let clamped = vol.clamp(0.0, 100.0);
        let msg = serde_json::json!({
            "command": ["set_property", "volume", clamped]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn set_equalizer(&self, gains: &[f64]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let is_flat = gains.iter().all(|&g| g.abs() < 0.05);
        if is_flat {
            crate::aethon_log!("set_equalizer: flat preset active, bypassing audio filters");
            let msg = serde_json::json!({
                "command": ["set_property", "af", ""]
            });
            return self.send_cmd(&msg.to_string()).await;
        }

        let freqs = [31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
        let mut filters = Vec::new();
        for (i, &f) in freqs.iter().enumerate() {
            let gain = gains.get(i).copied().unwrap_or(0.0).clamp(-12.0, 12.0);
            if gain.abs() >= 0.05 {
                filters.push(format!("equalizer=f={:.1}:width_type=o:width=1.0:g={:.1}", f, gain));
            }
        }
        let filter_chain = if filters.is_empty() {
            String::new()
        } else {
            filters.join(",")
        };
        let msg = serde_json::json!({
            "command": ["set_property", "af", filter_chain]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn set_replaygain(&self, mode: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let val = match mode {
            "track" | "album" => mode,
            _ => "no",
        };
        let msg = serde_json::json!({
            "command": ["set_property", "replaygain", val]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn set_crossfade(&self, seconds: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if seconds <= 0.05 {
            let msg = serde_json::json!({
                "command": ["set_property", "gapless-audio", "yes"]
            });
            return self.send_cmd(&msg.to_string()).await;
        }
        let msg = serde_json::json!({
            "command": ["set_property", "gapless-audio", "weak"]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub fn get_state(&self) -> PlaybackState {
        self.state.read().clone()
    }
}
