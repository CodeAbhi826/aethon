# Aethon 🎵

**Aethon** (`v0.0.37`) is an ultra-lightweight, GPU-accelerated, lossless desktop music player for **Arch Linux** (KDE Plasma on Wayland), blending the streaming & cloud architecture of **[ArchiveTune](https://github.com/rukamori/ArchiveTune)** (`moe.koiverse.archivetune`) with the editorial minimalism and spring physics of **Apple Music**.

---

## ✨ Core Architecture & Features

- **Sub-Second (`~0.55s`) Cold-Start Streaming via Local Bounded-Range Chunk Proxy:**
  - Desktop `mpv` / `ffmpeg` normally sends open-ended `Range: bytes=0-` headers, which Google Video's CDN blocks with `HTTP 403 Forbidden` unless `yt-dlp` downloads the 2.5 MB `base.js` player and spawns `deno` (~4.8s) to solve the `n=` cipher (causing 9.5–11.2s startup delays).
  - Aethon bypasses this bottleneck completely by querying YouTube's native InnerTube `/youtubei/v1/player` (`ANDROID_VR`, `ANDROID_TESTSUITE`, `IOS`) in `~250ms` and routing `mpv` through a built-in **Localhost Bounded-Range Chunk Proxy (`http://127.0.0.1:<port>/stream/<video_id>`)** that fetches 512 KB bounded chunks (`Range: bytes=start-end`)—matching Android ExoPlayer's traffic pattern for **`~0.55s` cold playback** and **`<0.15s` cached/prefetched playback**.
- **Bit-Perfect PipeWire / ALSA Audio Engine:**
  - Headless `mpv` IPC engine over `$XDG_RUNTIME_DIR/aethon-mpv-{pid}.sock` with 120ms RAM ring-buffering (`--cache-on-disk=no --cache-pause-wait=0.12`), 10-band ISO graphic equalizer, ReplayGain, and gapless playback.
  - Real-time `core-idle`, `paused-for-cache`, and `playback-restart` property observers drive dual **SVG Circular Buffering Rings** and indeterminate shimmer bars with zero false-pause states.
- **5-Provider Synced Lyrics & Multi-Script Phonetic Romanizer (`Aあ`):**
  - Automatic fallback chain modeled after [rukamori/lyrics](https://github.com/rukamori/lyrics):
    1. **Local `.lrc` sidecar files & Embedded ID3 (`USLT`/`SYLT`) / Vorbis (`LYRICS`) tags** via [`lofty`](https://crates.io/crates/lofty)
    2. **[LRCLIB](https://lrclib.net/docs)** (Exact match + duration-scored fuzzy search)
    3. **NetEase Cloud Music** (`lrc` synced lyrics + `romalrc` official romanized timestamps + `tlyric` translations)
    4. **KuGou Synced Lyrics** (Base64-decoded synced `.lrc`)
    5. **SimpMusic / YouLyPlus VideoID API**
  - Built-in Rust **`Romanizer`** (`Aあ Romanized` toggle) providing real-time phonetic transliteration for **Korean Hangul** (Revised Romanization syllable block decomposition), **Japanese Hiragana & Katakana** (Hepburn romanization with `っ` gemination and digraphs), **Hindi / Devanagari** (consonants, matras, virama, and word-final schwa deletion), and **Cyrillic**.
- **YouTube Music Account Sync & Official Year-End Recap Tracking:**
  - One-click **Sign in with Google** embedded Tauri `WebviewWindow` automatically captures `SAPISID` / `__Secure-3PAPISID` session cookies, generates `SAPISIDHASH` headers, and syncs **Liked Songs** (`VLLM`), **Saved Playlists**, and **Listening History**.
  - Reports `/api/stats/playback` on track start and `/api/stats/watchtime` heartbeats every 25 seconds with a 16-character `cpn` (Client Playback Nonce) so every song and minute listened counts toward your official **YouTube Music History** and **Year-End YouTube Recap**.
- **Apple Music & ArchiveTune V10 Editorial UI:**
  - **Editorial Floating Squircle Artwork Showcase:** High-resolution `rounded-[28px]` album cover showcase with Apple spring scale physics (`scale-100` with dynamic color-matched ambient glow when playing vs. `scale-[0.86]` when paused).
  - **Remade Homepage & Online-Only Search:** Apple Music *Top Picks for You* hero cards, ArchiveTune 3×4 *Quick Picks* speed grid, *Listen Again* carousel, *More Like [Artist]* station, *Top Charts* (`#1`–`#8`), and dedicated online catalog search (`musicCardShelfRenderer` Top Result + 4-Song Stack + Artists).
  - **iOS / macOS Spring Sheet Transitions:** Hardware-accelerated `in:fly` / `out:fly` (`appleSheetEasing`) open and close animations on `NowPlayingSheet`, `QueueDrawer`, `MiniPlayer`, and tab routing.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl + B</kbd> / <kbd>⌘ + B</kbd> | Toggle Retractable Navigation Rail |
| <kbd>Space</kbd> | Play / Pause Active Track |
| <kbd>←</kbd> / <kbd>→</kbd> | Seek Backward / Forward 5 seconds |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Adjust Volume (+5% / -5%) |
| <kbd>Escape</kbd> | Dismiss Context Menu / Modal / Queue Drawer / Now Playing Sheet |

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
> [!IMPORTANT]
> Aethon is compiled via the remote GitHub Actions CI pipeline to ensure reproducible AppImage packaging across Linux environments.

To update the local installed AppImage directly from the latest CI build:
```bash
aethon-update
```

---

## 📜 Changelog

For a complete record of updates, performance enhancements, and bug fixes from version 0.0.1 to 0.0.37, see [CHANGELOG.md](file:///home/abhin/Projects/music/CHANGELOG.md).
