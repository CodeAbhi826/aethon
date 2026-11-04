---
description: Guardrails for instant audio stream resolution, MPV IPC state handling, and non-blocking playback
alwaysApply: true
---

# Audio Streaming & Engine Playback Guardrails

When working on audio streaming, MPV integration, or stream resolvers:

1. **Sub-Second Stream Resolution (<1s Target):**
   - Prioritize verified InnerTube mobile endpoints (`IOS 20.03.02`) that deliver direct googlevideo Opus (itag 251) streams in <300ms.
   - Set network timeouts for InnerTube calls to at least 3500ms to tolerate mobile/international TLS handshakes.
   - Never query deprecated or bot-blocked clients (e.g., `ANDROID_VR`, `ANDROID_TESTSUITE`) that trigger `LOGIN_REQUIRED`.

2. **Zero-Stall Fallback Policy:**
   - Never invoke `yt-dlp` with `--cookies-from-browser` synchronously on the playback critical path; extracting browser cookies from SQLite takes 30–70s and causes playback timeouts.
   - When using CLI fallbacks, offload to asynchronous tasks and feed MPV direct YouTube watch URLs via its internal async demuxer rather than blocking frontend requests.

3. **Cold Boot & Idle State Management:**
   - An MPV instance in `idle-active: true` mode cannot resume playback with `cycle pause`. Always verify whether a track is physically loaded into the engine before issuing toggle commands.
   - If `currentTrack` is populated from persistent storage or play history but MPV is idle, route the play request through `playTrack(currentTrack)`.

4. **Continuous Audio Transition:**
   - Do not stop previous playback until the replacement stream is verified and ready to load.
   - Prevent race conditions where superseded track requests overwrite newer user selections.
