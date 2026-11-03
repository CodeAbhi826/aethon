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
    pub media_title: Option<String>,
    pub is_playing: bool,
    pub is_buffering: bool,
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
            media_title: None,
            is_playing: false,
            is_buffering: false,
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
    pub app_handle: Option<AppHandle>,
}

impl Drop for MpvEngine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl MpvEngine {
    pub fn shutdown(&self) {
        if let Some(mut child) = self.process.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if !self.socket_path.is_empty() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }

    pub fn dummy() -> Self {
        let (cmd_tx, _cmd_rx) = mpsc::channel(1);
        Self {
            process: parking_lot::Mutex::new(None),
            cmd_tx,
            socket_path: String::new(),
            state: Arc::new(RwLock::new(PlaybackState::default())),
            app_handle: None,
        }
    }

    pub async fn new(
        app: Option<AppHandle>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pid = std::process::id();
        let sock_buf = crate::runtime_socket_dir().join(format!("aethon-mpv-{}.sock", pid));
        let socket_path = sock_buf.to_string_lossy().to_string();

        if std::path::Path::new(&socket_path).exists() {
            let _ = std::fs::remove_file(&socket_path);
        }

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

        // Low-latency RAM-buffered MPV configuration for <1s stream startup
        let mut cmd = Command::new(mpv_bin);
        let mut ytdl_raw = "no-playlist=,no-check-certificates=".to_string();
        if let Some(cfg) = dirs::config_dir() {
            if cfg.join("BraveSoftware/Brave-Browser").exists() {
                ytdl_raw.push_str(",cookies-from-browser=brave");
            } else if let Some(home) = dirs::home_dir() {
                if home.join(".mozilla/firefox").exists() {
                    ytdl_raw.push_str(",cookies-from-browser=firefox");
                }
            }
        }

        cmd.arg("--idle=yes")
            .arg("--no-video")
            .arg("--no-terminal")
            .arg(format!("--input-ipc-server={}", socket_path))
            .arg("--ytdl=yes")
            .arg("--ytdl-format=251/140/bestaudio[ext=webm]/bestaudio/best")
            .arg(format!("--ytdl-raw-options={}", ytdl_raw))
            .arg("--keep-open=yes")
            .arg("--gapless-audio=yes")
            .arg("--cache=yes")
            .arg("--cache-on-disk=no")
            .arg("--cache-pause=yes")
            .arg("--cache-pause-wait=0.15")
            .arg("--cache-pause-initial=yes")
            .arg("--demuxer-max-bytes=33554432")
            .arg("--demuxer-max-back-bytes=8388608")
            .arg("--demuxer-readahead-secs=20")
            .arg("--audio-buffer=0.15")
            .arg("--demuxer-lavf-o=http_persistent=1,reconnect=1,reconnect_streamed=1,reconnect_delay_max=2")
            .arg("--ao=pipewire,pulse,alsa")
            .stdin(Stdio::null())
            .stdout(stdout_cfg)
            .stderr(stderr_cfg);

        crate::sanitize_host_command(&mut cmd);

        let child = cmd.spawn()?;

        let mut connected = false;
        let mut stream = None;
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(60)).await;
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

        let app_handle = app.clone();
        tokio::spawn(async move {
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            let mut mpv_paused = false;
            let mut mpv_core_idle = true;
            let mut mpv_paused_for_cache = false;

            while let Ok(n) = buf_reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                let trimmed = line.trim();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(event) = val.get("event").and_then(|e| e.as_str()) {
                        match event {
                            "start-file" => {
                                let mut s = state_clone.write();
                                s.is_buffering = true;
                                s.is_playing = false;
                                s.position = 0.0;
                                if let Some(ref handle) = app_handle {
                                    let _ = handle.emit("player-state", &*s);
                                }
                            }
                            "file-loaded" | "playback-restart" => {
                                mpv_core_idle = false;
                                mpv_paused_for_cache = false;
                                let mut s = state_clone.write();
                                s.is_buffering = false;
                                s.is_playing = s.current_uri.is_some() && !mpv_paused;
                                if let Some(ref handle) = app_handle {
                                    let _ = handle.emit("player-state", &*s);
                                }
                            }
                            "property-change" => {
                                if let Some(name) = val.get("name").and_then(|n| n.as_str()) {
                                    let mut s = state_clone.write();
                                    let data = val.get("data");
                                    match name {
                                        "time-pos" => {
                                            if let Some(t) = data.and_then(|d| d.as_f64()) {
                                                s.position = t;
                                                if t > 0.03 && !mpv_paused {
                                                    s.is_buffering = false;
                                                    s.is_playing = true;
                                                }
                                            }
                                        }
                                        "duration" => {
                                            if let Some(d) = data.and_then(|d| d.as_f64()) {
                                                if d > 0.0 {
                                                    s.duration = d;
                                                }
                                            }
                                        }
                                        "pause" => {
                                            if let Some(p) = data.and_then(|d| d.as_bool()) {
                                                mpv_paused = p;
                                                if p {
                                                    s.is_playing = false;
                                                } else if !s.is_buffering && !mpv_core_idle {
                                                    s.is_playing = s.current_uri.is_some();
                                                }
                                            }
                                        }
                                        "core-idle" => {
                                            if let Some(idle) = data.and_then(|d| d.as_bool()) {
                                                mpv_core_idle = idle;
                                                if !idle && !mpv_paused {
                                                    s.is_buffering = false;
                                                    s.is_playing = s.current_uri.is_some();
                                                }
                                            }
                                        }
                                        "paused-for-cache" => {
                                            if let Some(caching) = data.and_then(|d| d.as_bool()) {
                                                mpv_paused_for_cache = caching;
                                                if caching && s.current_uri.is_some() {
                                                    s.is_buffering = true;
                                                } else if !caching && !mpv_core_idle && !mpv_paused {
                                                    s.is_buffering = false;
                                                    s.is_playing = s.current_uri.is_some();
                                                }
                                            }
                                        }
                                        "path" => {
                                            if let Some(p) = data.and_then(|d| d.as_str()) {
                                                if s.current_uri.is_none() && !p.contains("googlevideo.com") {
                                                    s.current_uri = Some(p.to_string());
                                                }
                                            }
                                        }
                                        "media-title" => {
                                            if let Some(mt) = data.and_then(|d| d.as_str()) {
                                                if !mt.is_empty() {
                                                    s.media_title = Some(mt.to_string());
                                                }
                                            }
                                        }
                                        "volume" => {
                                            if let Some(v) = data.and_then(|d| d.as_f64()) {
                                                s.volume = v;
                                            }
                                        }
                                        "audio-codec-name" => {
                                            s.codec =
                                                data.and_then(|d| d.as_str()).map(String::from);
                                        }
                                        "audio-bitrate" => {
                                            s.bitrate = data.and_then(|d| d.as_i64());
                                        }
                                        "audio-params" => {
                                            if let Some(obj) = data.and_then(|d| d.as_object()) {
                                                s.sample_rate = obj
                                                    .get("samplerate")
                                                    .and_then(|sr| sr.as_i64());
                                                s.channels = obj
                                                    .get("channel-count")
                                                    .and_then(|c| c.as_i64());
                                            }
                                        }
                                        _ => {}
                                    }

                                    let _ = mpv_paused_for_cache;
                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit("player-state", &*s);
                                    }
                                }
                            }
                            "end-file" => {
                                let reason =
                                    val.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                                crate::aethon_log!("MPV end-file event received: reason={}", reason);
                                if reason == "eof" {
                                    let mut s = state_clone.write();
                                    s.is_playing = false;
                                    s.is_buffering = false;
                                    s.position = 0.0;
                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit("player-state", &*s);
                                        let _ = handle.emit("track-ended", ());
                                    }
                                } else if reason == "error" {
                                    let failed_uri = {
                                        let mut s = state_clone.write();
                                        s.is_playing = false;
                                        s.is_buffering = false;
                                        s.current_uri.clone()
                                    };
                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit("player-state", &*state_clone.read());
                                        if let Some(uri) = failed_uri {
                                            let _ = handle.emit("stream-error", uri);
                                        }
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
            app_handle: app,
        };

        engine
            .send_cmd(r#"{"command": ["observe_property", 1, "time-pos"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 2, "duration"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 3, "pause"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 4, "volume"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 5, "audio-codec-name"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 6, "audio-params"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 7, "audio-bitrate"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 8, "core-idle"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 9, "paused-for-cache"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 10, "path"]}"#)
            .await?;
        engine
            .send_cmd(r#"{"command": ["observe_property", 11, "media-title"]}"#)
            .await?;

        Ok(engine)
    }

    pub async fn send_cmd(
        &self,
        json_cmd: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crate::aethon_log!("MPV IPC >> {}", json_cmd);
        self.cmd_tx.send(json_cmd.to_string()).await?;
        Ok(())
    }

    pub async fn load_file_with_canonical(
        &self,
        playable_uri: &str,
        canonical_uri: Option<&str>,
        title: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        {
            let mut s = self.state.write();
            s.current_uri = Some(canonical_uri.unwrap_or(playable_uri).to_string());
            if let Some(t) = title {
                s.media_title = Some(t.to_string());
            }
            s.position = 0.0;
            s.is_buffering = true;
            s.is_playing = false;
            if let Some(ref handle) = self.app_handle {
                let _ = handle.emit("player-state", &*s);
            }
        }
        // Silence previous audio stream immediately so old tracks don't continue or overlap
        let _ = self.send_cmd(r#"{"command": ["stop"]}"#).await;
        let msg = serde_json::json!({
            "command": ["loadfile", playable_uri, "replace"]
        });
        self.send_cmd(&msg.to_string()).await?;
        let play_msg = serde_json::json!({
            "command": ["set_property", "pause", false]
        });
        self.send_cmd(&play_msg.to_string()).await?;
        Ok(())
    }

    pub async fn load_file(
        &self,
        uri: &str,
        title: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.load_file_with_canonical(uri, None, title).await
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
        let (is_eof, uri_opt, title_opt) = {
            let s = self.state.read();
            let at_end = s.duration > 0.0 && s.position >= (s.duration - 0.5);
            (at_end, s.current_uri.clone(), s.media_title.clone())
        };

        if is_eof {
            if let Some(uri) = uri_opt {
                return self.load_file_with_canonical(&uri, None, title_opt.as_deref()).await;
            } else {
                let _ = self.seek(0.0).await;
            }
        }

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

    pub async fn set_volume(
        &self,
        vol: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let clamped = vol.clamp(0.0, 100.0);
        let msg = serde_json::json!({
            "command": ["set_property", "volume", clamped]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn set_equalizer(
        &self,
        gains: &[f64],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let is_flat = gains.iter().all(|&g| g.abs() < 0.05);
        if is_flat {
            let msg = serde_json::json!({
                "command": ["set_property", "af", ""]
            });
            return self.send_cmd(&msg.to_string()).await;
        }

        let freqs = [
            31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
        ];
        let mut filters = Vec::new();
        for (i, &f) in freqs.iter().enumerate() {
            let gain = gains.get(i).copied().unwrap_or(0.0).clamp(-12.0, 12.0);
            if gain.abs() >= 0.05 {
                filters.push(format!(
                    "equalizer=f={:.1}:width_type=o:width=1.0:g={:.1}",
                    f, gain
                ));
            }
        }
        let filter_chain = filters.join(",");
        let msg = serde_json::json!({
            "command": ["set_property", "af", filter_chain]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn set_replaygain(
        &self,
        mode: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let val = match mode {
            "track" | "album" => mode,
            _ => "no",
        };
        let msg = serde_json::json!({
            "command": ["set_property", "replaygain", val]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub async fn set_crossfade(
        &self,
        seconds: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let val = if seconds <= 0.05 { "yes" } else { "weak" };
        let msg = serde_json::json!({
            "command": ["set_property", "gapless-audio", val]
        });
        self.send_cmd(&msg.to_string()).await
    }

    pub fn get_state(&self) -> PlaybackState {
        self.state.read().clone()
    }
}
