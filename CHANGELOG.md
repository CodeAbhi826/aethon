# Changelog

All notable changes to **Aethon** are documented in this file. The project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.0.35] — 2026-11-04

### Playback Engine & Stream Architecture
* **Open-Range Compatible Stream Resolver:**
  * Diagnosed and fixed the Google Video HTTP 403 Forbidden failure: resolved streams now use `TVHTML5` extraction with full open-ended range request (`Range: bytes=0-` $\rightarrow$ `206 Partial Content`) support, allowing MPV and FFmpeg to demux audio continuously without stream termination.
  * Removed `--extractor-args "youtube:player_client=android"` which was failing due to YouTube's recent SABR experiment rollout.
  * Eliminated synchronous `--cookies-from-browser` extraction from both `yt-dlp` and `mpv`, removing 33–73 second cold stalls caused by Linux encrypted SQLite cookie parsing.
* **In-Memory Stream Caching (`STREAM_CACHE`):**
  * Implemented an in-memory stream cache with a 4-hour TTL (`CachedStream`).
  * Replaying songs, scrubbing, and navigating the queue now resolve in **`<1ms`** without touching the network.
* **Continuous Audio & Smooth Track Transitions:**
  * Replaced premature engine stops with atomic stream replacement (`loadfile replace`) in MPV so previous tracks play smoothly until replacement streams are ready.
* **Cold Boot & Idle State Recovery:**
  * Updated `togglePlay()` in `state.svelte.ts` and `toggle_pause()` in `mpv.rs` to detect when the engine is in an idle state without a loaded file, properly routing initial play requests through `playTrack()`.

### Visuals, Buffering & UI Polish
* **SVG Circular Buffering Indicators & Shimmer Bars:**
  * Replaced generic loading spinners with smooth SVG indeterminate circular progress rings across `BottomPlayer`, `NowPlayingSheet`, `MiniPlayer`, `HomeView`, and `QueueDrawer`.
  * Preserved `isLoading = true` during stream resolution so the UI never prematurely flips to the "Pause" state before real audio frames are decoded.
* **Squircle Morphing Stabilization:**
  * Fixed a visual glitch where the play/pause button morphed into a sharp square during transitions. Replaced clamped large pixel radii with exact width-matched radii (`rounded-[18px]` $\leftrightarrow$ `rounded-[12px]`) and non-overshooting cubic-bezier easing (`cubic-bezier(0.2, 0, 0, 1)`).
* **Track Reconciliation & Metadata Synchronization:**
  * Added bidirectional reconciliation between MPV IPC events (`path`, `media-title`) and Svelte reactive state (`currentTrack`) to keep track titles, artists, and album artwork perfectly synchronized during radio and autoplay continuation.

### Branding & System Integration
* **Desktop & Tray Icon Polish:**
  * Replaced default boilerplate assets with the official Aethon purple soundwave brand icon.
  * Converted system tray icon to 32-bit RGBA PNG for flawless rendering across KDE Plasma Wayland panels.
  * Added active MPRIS playback indicators to the taskbar application entry.

---

## [0.0.34] — 2026-11-03

### Branding & Tray Integration
* **Official Brand Identity:** Packaged and integrated the official Aethon soundwave logo as the window, taskbar, desktop launcher, and notification icon.
* **Tray Audio Controls:** Added Play/Pause toggle, Next/Previous controls, and Mini-Player switcher directly to the Linux system tray menu.

---

## [0.0.33] — 2026-11-03

### Core Features & Account Sync
* **YouTube Music Sync:** Added settings and IPC commands to synchronize user playlists, liked songs, and library tracks directly from YouTube Music.
* **Playback Tracking & Recap Heartbeat:** Implemented official YouTube Music Recap watch-time heartbeat logging, reporting listening sessions every ~25 seconds of playback.
* **UI Overhaul:** Refreshed `HomeView.svelte` with Apple Music-inspired "Top Picks" hero showcase, ArchiveTune 3-column quick picks speed grid, and dynamic regional exploration tags.

---

## [0.0.30 – 0.0.32] — 2026-11-02

### Search & Engine Hardening
* **Expanded Two-Pass Search:** Implemented two-pass search combining InnerTube `musicCardShelfRenderer` with standard results, expanding discovery limits up to 50 tracks.
* **Play Button Alignment:** Pixel-aligned and centered play/pause vector icons across all card overlays and list items.
* **Environment Sanitization:** Implemented `sanitize_host_command` to scrub Tauri/AppImage environment variables (`PYTHONHOME`, `PYTHONPATH`, `LD_LIBRARY_PATH`) when invoking host binaries (`mpv`, `yt-dlp`, `python3`).

---

## [0.0.26 – 0.0.29] — 2026-11-01

### Player Refinements & Micro-Interactions
* **Spring Animations:** Implemented ArchiveTune fluid spring micro-interactions on play/pause buttons and sheet opening/closing.
* **EOF Detection:** Filtered MPV `end-file` events specifically for reason `eof` to prevent cascade skipping on stream timeouts or socket disconnects.

---

## [0.0.21 – 0.0.25] — 2026-10-31

### Resource & Performance Optimization
* **RAM & Texture Optimization:** Calibrated WebKit texture allocations, glibc malloc arenas, and MPV demuxer cache thresholds (`--cache-on-disk=no`, `--cache-pause-wait=0.15`) for minimal memory footprint on low-power devices.
* **Frosted Glass Backgrounds:** Replaced expensive dynamic blurs with optimized static gradient meshes and GPU-accelerated acrylic backdrops.

---

## [0.0.18 – 0.0.20] — 2026-10-30

### Library & Playback Features
* **Up Next Queue Drawer:** Slide-out interactive queue drawer with drag-and-drop reordering, track removal, and Autoplay Radio toggle.
* **Offline Downloads:** Tag-preserving audio downloader with Lofty metadata injection.
* **Mini-Player:** Compact picture-in-picture floating player with essential transport controls.
* **Context Menus:** Complete track action menu (Add to Playlist, Go to Artist, Go to Album, Play Next, Add to Queue, Download).

---

## [0.0.15 – 0.0.17] — 2026-10-29

### Audio DSP & Visualizations
* **10-Band Studio Graphic Equalizer:** ISO standard frequencies (31Hz – 16kHz) with real-time response curve rendering and custom preset selection.
* **Vinyl Turntable:** Interactive vinyl record animation with realistic groove reflection and rotating center label.
* **60 FPS Spectrum Visualizer:** Live canvas-based wave visualizer synchronized with playback.

---

## [0.0.11 – 0.0.14] — 2026-10-28

### Lyrics & Theming
* **Multi-Provider Lyrics Pipeline:** Synchronized karaoke lyrics engine supporting LRCLIB with line-by-line glow highlighting and click-to-seek navigation.
* **Dynamic Material 3 Monet Theming:** Automated palette extraction from album cover art using `@material/material-color-utilities`.

---

## [0.0.1 – 0.0.10] — 2026-10-27

### Initial Release
* **Initial Release of Aethon Lossless Music Player:**
  * Core Tauri v2 + Svelte 5 frontend with Tailwind CSS.
  * Headless MPV audio engine communicating over local Unix domain sockets.
  * Recursive local file scanner indexing FLAC, ALAC, WAV, and MP3 into SQLite via Lofty.
  * MPRIS2 D-Bus service bridge for Linux desktop controls.
