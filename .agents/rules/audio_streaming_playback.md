---
description: Guardrails for instant audio stream resolution, MPV IPC state handling, and non-blocking playback
alwaysApply: true
---

# Audio Streaming & Engine Playback Guardrails

When working on audio streaming, MPV integration, or stream resolvers:

1. **Stream Resolution & Range Support:**
   - Google Video's `c=IOS` stream endpoint enforces Apple's HLS bounded chunking and rejects open-ended byte ranges (`bytes=0-`) with HTTP 403 Forbidden. Because MPV/FFmpeg demuxers issue `bytes=0-` requests, raw iOS stream URLs must not be passed directly to MPV.
   - Use stable `TVHTML5` extraction (via async `yt-dlp` without cookies) which solves the `n=` transform and yields full open-ended range compatible googlevideo streams.
   - Cache all resolved stream URLs in memory (`STREAM_CACHE`) for 4 hours to enable instant (<1ms) replays, seeking, and queue navigation.
   - Fall back smoothly to the canonical `https://www.youtube.com/watch?v={id}` URL if direct extraction times out, allowing MPV's native async demuxer to stream it.

2. **Zero-Stall Fallback Policy:**
   - Never invoke `yt-dlp` with `--cookies-from-browser` synchronously on the playback critical path; extracting browser cookies from SQLite takes 30–70s and causes playback timeouts.
   - When using CLI fallbacks, offload to asynchronous tasks and feed MPV direct YouTube watch URLs via its internal async demuxer rather than blocking frontend requests.

3. **Cold Boot & Idle State Management:**
   - An MPV instance in `idle-active: true` mode cannot resume playback with `cycle pause`. Always verify whether a track is physically loaded into the engine before issuing toggle commands.
   - If `currentTrack` is populated from persistent storage or play history but MPV is idle, route the play request through `playTrack(currentTrack)`.

4. **Continuous Audio Transition:**
   - Do not stop previous playback until the replacement stream is verified and ready to load.
   - Prevent race conditions where superseded track requests overwrite newer user selections.
