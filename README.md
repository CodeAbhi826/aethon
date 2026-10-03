# Aethon 🎵

**Aethon** is an ultra-lightweight, GPU-accelerated, modern lossless music player for **Arch Linux** (KDE Plasma on Wayland), combining the structural UX of **ArchiveTune** (`moe.koiverse.archivetune`) with the clean, refined aesthetic of **Apple Music**.

---

## ✨ Features

- **Bit-Perfect Lossless Audio:** Powered by a headless `mpv` audio engine communicating over a Unix domain socket (`/tmp/aethon-mpv.sock`), directly outputting bit-perfect streams to PipeWire/ALSA with zero resample degradation.
- **Local Lossless Library:** Recursive tag scanner with [`lofty`](https://crates.io/crates/lofty) indexing FLAC, ALAC, WAV, and MP3 into a private offline SQLite database (`~/.local/share/aethon/aethon.db`).
- **Ad-Free YouTube Music Streaming:** Instant search and streaming via `/usr/bin/yt-dlp` and `mpv`, streaming high-bitrate Opus directly without third-party web overhead.
- **Dynamic Monet / Material 3 Expressive UI:** Real-time color palette extraction from album cover art using `@material/material-color-utilities`, creating silky smooth ambient backdrops with zero grain or noise.
- **Synchronized Karaoke Lyrics:** Integration with the community-driven **LRCLIB** database, featuring line-by-line highlighting and click-to-seek timestamp jumping.
- **10-Band Graphic Equalizer:** ISO standard frequency bands (31Hz – 16kHz) with real-time response curve visualization and presets (Flat, Bass Boost, Vocal, Electronic, Rock, Acoustic).
- **Physical Turntable Vinyl:** High-fidelity vinyl animation with realistic grooves, stationary specular light sheen, and rotating album center label.
- **60 FPS Live Audio Visualizer:** Smooth canvas-based audio wave spectrum synchronized with playback.
- **Retractable Navigation Rail:** Collapsible sidebar (<kbd>Ctrl+B</kbd>) with ArchiveTune's 5 core sections: **Home**, **Search**, **Library**, **Stats**, and **Settings**.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl + B</kbd> / <kbd>⌘ + B</kbd> | Toggle Retractable Sidebar |
| <kbd>Space</kbd> | Play / Pause Playback |
| <kbd>←</kbd> / <kbd>→</kbd> | Seek Backward / Forward 5 seconds |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Adjust Volume (+5% / -5%) |
| <kbd>Escape</kbd> | Close Fullscreen Now Playing / Equalizer Modal |

---

## 🛠️ Development & Building

### Prerequisites on Arch Linux
```bash
sudo pacman -S --needed mpv yt-dlp webkit2gtk-4.1 base-devel
```

### Run in Development
```bash
pnpm install
pnpm tauri dev
```

### Build Production Binary
```bash
pnpm tauri build
```
The output binary and packages will be located in `src-tauri/target/release/`.
