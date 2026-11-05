# Changelog

All notable changes, architectural upgrades, and performance fixes in **Aethon** are documented in this file.

---

## [v0.0.38] — 2026-11-06

### 🚀 60fps WebKitGTK GPU Compositor & UI Smoothness
- **WebKitGTK 100% GPU Compositing Engine (`src-tauri/src/main.rs`)**:
  - Activated `WEBKIT_FORCE_COMPOSITING_MODE=1` and removed conflicting `WEBKIT_DISABLE_COMPOSITING_MODE`.
  - Configured hardware-accelerated rendering and GPU layer compositing.
- **Micro-Stutter Eliminations & State Throttle (`src-tauri/src/player/mpv.rs` & `src/lib/state.svelte.ts`)**:
  - Throttled MPV `time-pos` property change IPC emissions to 10 Hz (100 ms) while maintaining 0 ms immediacy for play/pause/seek events.
  - In-place mutation of `playerState.playback` via `Object.assign` avoiding full Svelte proxy invalidation cascades.
  - Guarded `reconcilePlayingTrack` and `updateMediaSession` to only fire when track URI or play status genuinely changes.
  - Throttled `localStorage` position persistence from every tick down to $\ge 5.0$s delta.
  - Cached Home feeds in `playerState` (`homeQuickPickFeed`, `homeChartTracks`, `homeArtistRadioTracks`, `homeFeedLoaded`) eliminating blank-flash re-fetches on tab switches.
- **High-Performance Scrubber & Color Extraction (`BottomPlayer.svelte`, `NowPlayingSheet.svelte`, `src/lib/theme.ts`)**:
  - Replaced CPU layout reflows (`width: X%`) with hardware-accelerated `transform: scaleX(...)` on scrubber progress bars.
  - Replaced full-image color extraction with 24x24 offscreen canvas chroma-weighted sampling and RAM cache.
  - Removed full-window CSS `backdrop-blur-3xl` and heavy Gaussian blur in `NowPlayingSheet.svelte` in favor of a GPU dual radial gradient mesh.
- **Apple Music / ArchiveTune Synced Lyrics Enhancements (`NowPlayingSheet.svelte`)**:
  - Eliminated per-line `filter: blur(1.2px)` in favor of distance-graded CSS spring cascades (`cascade-lyric`), keeping rendering entirely on the GPU.
  - 60fps `requestAnimationFrame` spring auto-scroll (`animateScrollTo`) centering active lyrics smoothly.
  - Musical interlude rhythm indicators (`• • •` with `interlude-dot-1/2/3` pulsing) for intro breaks ($\ge 4.0$s) and mid-song breaks ($\ge 10.0$s).
  - Single circular ArchiveTune spinner (`archivetune-spinner-svg` + `archivetune-spinner-arc`) across buttons, cards, and lyrics loading skeleton (`isLoadingLyrics` + `lyricsRequestId`).

### 🎵 7-Provider ArchiveTune Lyrics Pipeline (`src-tauri/src/lyrics/mod.rs`)
- **Tier 0**: Local `.lrc` sidecars, embedded tags (`USLT`/`SYLT`/`LYRICS`), and persistent disk cache in `~/.cache/aethon/lyrics/`.
- **Tier 1**: LRCLIB with `±12s` duration filtering and NetEase CJK auto-merge (`romalrc` + `tlyric`).
- **Tier 2**: NetEase Cloud Music (`lrc` + `romalrc` + `tlyric`).
- **Tier 3**: SimpMusic / YouLyPlus YouTube `videoId` lookup.
- **Tier 4**: KuGou Music duration-matched search with Base64 decoded lyrics.
- **Tier 5**: YouTube Music Native InnerTube (`timedLyricsData` and `musicDescriptionShelfRenderer`).
- **Tier 6**: Lyrics.ovh plain-text fallback.

### 🌐 12-Script Phonetic Romanization Engine (`Romanizer` in `src-tauri/src/lyrics/mod.rs`)
- Full mathematical syllable decomposition for **Korean Hangul** (Revised Romanization).
- Hepburn romanization for **Japanese Kana** (Hiragana/Katakana with sokuon and digraphs).
- Unified ISCII offset mapping with language-specific rules:
  - **Malayalam**: Chillu letters (`ൻ`, `ർ`, `ൽ`, `ൾ`, `ൺ`, `ൿ`), retroflex `ഴ` (`zha`), and clusters `റ്റ`/`ന്റ`.
  - **Tamil**: Retroflex `ழ` (`zha`), `ற`, `ன/ண`.
  - **Devanagari** (Hindi, Marathi, Sanskrit) with nukta, halant, and schwa deletion.
  - **Telugu, Kannada, Bengali, Gujarati, Gurmukhi, Odia, and Cyrillic**.

---

## [v0.0.37] — 2026-11-05

### ⚡ ArchiveTune Progressive Disk-Cache Architecture (`src-tauri/src/youtube/mod.rs`)
- **Root-Cause SABR 403 Cutoff Resolution**:
  - Uncovered the exact cause of the ~28–60s playback cutoff: YouTube's mobile InnerTube streams (`IOS`) enforce a strict SABR byte-range ceiling (~20–23% of content length, or ~912 KB – 1022 KB), returning `HTTP 403 Forbidden` for any HTTP Range requests beyond that ceiling.
  - Resolved **Visitor Token Poisoning**: `WEB_REMIX` visitor tokens sent to `ANDROID_VR` caused `ANDROID_VR` to return `LOGIN_REQUIRED`. Completely segregated `WEB_VISITOR_DATA` and `VR_VISITOR_DATA` stores so `ANDROID_VR` only receives VR visitor tokens.
  - Implemented **Tail-Byte Probing**: When falling back to `IOS`, Aethon proactively probes a 32-byte slice past the 1 MB boundary (`content_length - 64..content_length - 33`). If Google Video returns `403 Forbidden`, the stream is rejected before playback starts, cleanly falling through to `yt-dlp` (`android_vr,tv,mweb,web_safari` without `player_skip=webpage,configs`).
- **Four-Tier Stream Resolution Engine**:
  - **Tier 0 (<5 ms Disk Hit)**: Instant local playback from `~/.cache/aethon/streams/<video_id>.webm` (or `.m4a`) if cached. Zero network latency, instant scrubbing, 100% offline-ready.
  - **Tier 1 (~250 ms Fast InnerTube)**: Concurrent `ANDROID_VR` / `ANDROID_TESTSUITE` / verified `IOS` resolution with segregated visitor data.
  - **Tier 2 (Self-Healing Progressive Downloader)**: Asynchronous background downloader (`run_stream_downloader`) streams 512 KB slices (`&range={start}-{end}` query param + `Range:` header fallback) into `~/.cache/aethon/streams/<video_id>.part`. Automatically self-heals expired or 403'd chunks via `yt-dlp` and resumes writing at the exact byte offset without dropping MPV's socket.
  - **Tier 3 (Progressive Read-Through Proxy)**: Reads directly from `.part` / `.webm` and delivers chunks to MPV. When the playback read cursor meets the network write cursor, it awaits `tokio::sync::Notify` instead of returning POSIX EOF, preventing premature track end while keeping time-to-sound under 300 ms.
- **Cache Management & Quota Enforcement (`enforce_disk_cache_limit`)**:
  - Automatic LRU eviction based on user-configured `maxCacheSizeGb` (default 10 GB).
  - Atomically renames `.part` to `.webm` / `.m4a` and stores metadata JSON on completion.

### 🛠️ 10 Additional Deep-Dive Audit Bug Fixes
1. **Visitor Token Segregation (`youtube/mod.rs`)**: Split `WEB_VISITOR_DATA` and `VR_VISITOR_DATA` to prevent `ANDROID_VR` authorization failure.
2. **Database Primary Key Collision Fix (`library/db.rs`)**: Fixed SQLite `UNIQUE constraint failed: tracks.id` in `upsert_track` and `upsert_tracks_batch` by deleting stale conflicting `file_path` rows and using `ON CONFLICT(id) DO UPDATE SET file_path = excluded.file_path, ...`.
3. **YouTube Music Cloud Playlists & History Persistence (`youtube/auth_sync.rs`)**: Added `FEmusic_liked_playlists` browse extraction + `db.upsert_cloud_playlist()`, and persisted synced `history_tracks` into SQLite `tracks` and `play_history`.
4. **MPV Keep-Open & EOF-Reached Observation (`player/mpv.rs`)**: Changed `--keep-open=yes` to `--keep-open=no` and registered MPV property observer for `"eof-reached"` (`id 12`), ensuring `track-ended` always fires reliably across streaming and cached tracks.
5. **Toggle Pause Replay Bug (`player/mpv.rs`)**: Simplified `MpvEngine::toggle_pause` to atomic `cycle pause`, preventing raw YouTube web URLs from being dispatched to MPV on pause/resume.
6. **Repeat One Track Unload Fix (`src/lib/state.svelte.ts`)**: Changed `playNext()` in `repeatMode === 'one'` from `this.seek(0)` to `await this.playTrack(this.currentTrack)` so tracks replay seamlessly from the local disk cache (<5 ms) after MPV unloads at EOF.
7. **Local Stream URL Track Reconciliation (`src/lib/state.svelte.ts`)**: Updated `extractVideoId`, `isSameTrack`, and `reconcilePlayingTrack` to recognize `/stream/`, `/streams/`, and loopback proxy URLs (`127.0.0.1:`).
8. **Stream Cache Settings Wiring & Region Code Prioritization (`commands.rs`)**: Wired `streamCachingEnabled` and `maxCacheSizeGb` to `youtube::configure_stream_cache()`, and prioritized `regionCode` over `ytmRegion` in `search_youtube`.
9. **Startup ReplayGain & Crossfade MPV Initialization (`src/lib/settingsState.svelte.ts`)**: Wired startup `invoke('set_replaygain', ...)` and `invoke('set_crossfade', ...)` on application launch.
10. **AlbumView Artist Sanitization & Timestamp Normalization (`src/lib/views/AlbumView.svelte`)**: Normalized `date_added` to Unix seconds (`Math.floor(Date.now() / 1000)`) and applied `sanitizeArtist()`.

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
