# Changelog

All notable changes, architectural upgrades, and performance fixes in **Aethon** are documented in this file.

---

## [v0.0.36] — 2026-11-04

### 🚀 Sub-Second Cold Playback Engine (`ChunkProxyServer`)
- **Localhost Bounded-Range Chunk Proxy (`src-tauri/src/youtube/mod.rs`)**:
  - Solved the root cause of `~9.5 – 11.2s` cold-track startup delays on desktop Linux: while mobile InnerTube clients (`ANDROID_VR`, `ANDROID_TESTSUITE`, `IOS`) return direct `googlevideo.com/videoplayback` URLs in `~250ms` without downloading `base.js` (2.5 MB) or spawning `/usr/bin/deno` (`~4.8s`), desktop `mpv` / `ffmpeg` sends an open-ended `Range: bytes=0-` HTTP header that triggers `HTTP 403 Forbidden` on Google Video's CDN.
  - Built an async Tokio TCP range-chunking proxy (`http://127.0.0.1:<port>/stream/<video_id>`) that accepts `mpv`'s open-ended `Range: bytes=START-` requests and fulfills them via sequential **512 KB bounded range requests (`Range: bytes=cur-chunk_end`)** with the matching mobile `User-Agent`—cutting cold-track time-to-sound from `~9.6s` down to **`~0.55s`**.
- **VisitorData Token Capture & Injection**: Automatically captures `responseContext.visitorData` from InnerTube search responses (or `/youtubei/v1/visitor_id`) and injects it into concurrent `ANDROID_VR` (`1.60.19`), `ANDROID_TESTSUITE` (`1.9`), and `IOS` (`20.03.02`) `/youtubei/v1/player` requests.
- **Canonical URI Preservation & Stream Error Auto-Recovery (`src-tauri/src/player/mpv.rs`, `src/lib/state.svelte.ts`)**:
  - Added `load_file_with_canonical(&playable_uri, Some(&uri))` so `playback.current_uri` retains the canonical track URI (`https://www.youtube.com/watch?v=...`) while `mpv` streams from the local chunk proxy—preventing Pause/Resume (`togglePlay`) from reloading the track from `0:00`.
  - Added `"stream-error"` event emission on `mpv` `end-file (reason=error)` to automatically invalidate stale cache entries (`forceRefresh: true`) and seamlessly re-resolve the stream.

### 🎨 Real Buffering Observers, Dual SVG Circular Rings & Editorial UI Remake
- **Accurate `is_buffering` State (`src-tauri/src/player/mpv.rs`, `src/lib/state.svelte.ts`)**:
  - Fixed the bug where `mpv.rs:load_file` immediately emitted `is_playing: true` at `t = 0.0s`, causing `state.svelte.ts` to clear `isLoading = false` within `<5ms` of clicking a track while audio was still loading silently.
  - Observed `mpv` properties `core-idle` (`id 8`) and `paused-for-cache` (`id 9`) alongside `start-file`, `file-loaded`, and `playback-restart` events so `isLoading` remains `true` until real audio samples play through PipeWire.
- **Dual SVG Circular Progress Rings & Scrubber Shimmer (`BottomPlayer.svelte`, `NowPlayingSheet.svelte`, `MiniPlayer.svelte`)**:
  - Added outer orbiting SVG circular arc rings + inner spinning SVG arcs on Play/Pause buttons, active track thumbnail SVG circular spinners that morph into 3-bar animated equalizers once playing, and indeterminate shimmer bars on the scrubber and dock edge.
- **Full Remake of `HomeView.svelte` & Online-Only `SearchView.svelte`**:
  - Remade `HomeView.svelte` with Apple Music *Top Picks for You* hero cards (`aspect-[4/4.6]`), ArchiveTune 3-column × 4-row *Quick Picks* speed grid, *Listen Again* platter carousel, *More Like [Artist]* station shelf, and *Top Charts* (`#1`–`#8`).
  - Remade `SearchView.svelte` to query the online streaming catalog exclusively (**zero local library tracks mixed into search results**), featuring a live SVG circular spinner inside the search input, Top Result hero card (`musicCardShelfRenderer`) + 4-Song Stack, circular Artists row, and shimmer skeleton placeholders.
- **Apple Music / ArchiveTune V10 Editorial Artwork & Spring Sheet Transitions (`NowPlayingSheet.svelte`, `QueueDrawer.svelte`, `MiniPlayer.svelte`, `+page.svelte`)**:
  - Replaced the CSS vinyl disc and center spindle hole with a full-bleed `rounded-[28px]` high-resolution squircle artwork showcase with spring play/pause scaling (`scale-100` vs `scale-[0.86]`) and dynamic color-matched ambient shadow glow.
  - Added hardware-accelerated `in:fly` / `out:fly` (`appleSheetEasing`) enter and exit animations on `NowPlayingSheet` and `QueueDrawer`, `in:scale` / `out:scale` on `MiniPlayer`, and tab transitions in `+page.svelte`.

### 🎤 5-Provider Synced Lyrics & Multi-Script Romanizer (`src-tauri/src/lyrics/mod.rs`)
- **Multi-Provider Fallback Pipeline**: Queries **Local `.lrc` / Embedded ID3-Vorbis tags** → **LRCLIB (Exact + Duration-Scored Search)** → **NetEase Cloud Music (`lrc` + `romalrc` + `tlyric`)** → **KuGou Synced LRC** → **SimpMusic VideoID API**.
- **Built-In Multi-Script Phonetic Romanizer (`Romanizer::romanize_line`)**: Algorithmic transliteration for **Korean Hangul**, **Japanese Hiragana & Katakana**, **Hindi / Devanagari**, and **Cyrillic**, wired to the `Aあ Romanized` toggle in `NowPlayingSheet.svelte`.
- **Fixed YouTube Recap & Cloud Sync IPC Contracts**: Aligned `PlaybackTrackingSession` (`init_recap_playback_tracking` / `ping_recap_watchtime`) and `YtmSyncSummary` (`sync_ytm_library`) between Rust (`commands.rs`) and TypeScript (`state.svelte.ts`, `settingsState.svelte.ts`).

---

## [v0.0.35] — 2026-11-03

- **Open-Range Stream Diagnostics**: Identified Google Video's `403 Forbidden` behavior on open-ended `Range: bytes=0-` requests vs `206 Partial Content` on bounded range chunks (`Range: bytes=0-1000000`).
- **Zero-Stall `yt-dlp` Fallback Policy**: Added `--no-config` to all `yt-dlp` invocations, eliminating 33–73 second stalls caused by external browser cookie extraction (`--cookies-from-browser`).
- **4-Hour In-Memory Stream Cache (`STREAM_CACHE`) & Prefetching**: Added sub-millisecond (`<1ms`) cache lookups for replayed songs and automatic background prefetching (`prefetch_stream`) for upcoming queue items and hovered tracks.
- **Cold Boot Idle Recovery**: Fixed cold-boot idle playback state restoration in `state.svelte.ts` and `mpv.rs`.

---

## [v0.0.33 – v0.0.34] — 2026-10-30

- **YouTube Music Account Sync & Recap Tracking (`src-tauri/src/youtube/auth_sync.rs`)**: Added one-click Google WebView login (`SAPISIDHASH`), cloud sync for Liked Songs (`VLLM`), Saved Playlists, and History, plus `/api/stats/playback` and `/api/stats/watchtime` Recap heartbeat reporting.
- **Official Aethon Soundwave Branding & System Tray**: Added `static/aethon_logo.svg` and native Linux system tray menu (`Play/Pause`, `Next`, `Previous`, `Mini-Player`, `Show/Hide`, `Quit`).

---

## [v0.0.21 – v0.0.32] — 2026-10-10

- **RAM & WebKitGTK Texture Optimizations**: Tuned glibc malloc arenas, WebKit image decoding (`loading="lazy" decoding="async"`), and MPV demuxer cache sizing.
- **Two-Pass InnerTube Search**: Parsed `musicCardShelfRenderer` top-hit hero cards and `musicResponsiveListItemRenderer` tracks with metadata noise stripping.
- **AppImage Host Sanitization**: Added `sanitize_host_command` to strip `LD_LIBRARY_PATH` and `PYTHONHOME` when spawning host `mpv`, `yt-dlp`, `python3`, and `notify-send`.

---

## [v0.0.11 – v0.0.20] — 2026-10-05

- **Queue Drawer, Offline Downloader & Mini-Player**: Added slide-over Up Next queue drawer, `yt-dlp` audio downloader (`~/Music/Aethon Downloads`), and picture-in-picture Mini-Player.
- **10-Band Studio Equalizer, 60 FPS Visualizer & Material 3 Monet Theme**: Added 10-band ISO graphic equalizer (`equalizer` lavfi filter chain), reactive 60 FPS canvas visualizer, and `@material/material-color-utilities` dynamic palette extraction.

---

## [v0.0.1 – v0.0.10] — 2026-10-03

- **Initial Release**: Tauri v2 + Svelte 5 desktop architecture, headless `mpv` JSON IPC engine over Unix domain sockets, recursive `lofty` audio scanner with SQLite indexing (`~/.local/share/aethon/aethon.db`), and MPRIS2 D-Bus desktop media controls (`mpris_bridge.py`).
