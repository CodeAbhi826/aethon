<script lang="ts">
  import { playerState } from '../state.svelte';

  let dynamicMonetEnabled = $state(true);
  let gaplessEnabled = $state(true);
  let replayGainEnabled = $state(false);
  let isScanning = $state(false);

  async function handleRescan() {
    isScanning = true;
    await playerState.scanLibrary();
    isScanning = false;
  }
</script>

<div class="p-6 space-y-6 max-w-4xl mx-auto">
  
  <div>
    <h2 class="font-heading font-bold text-2xl text-white">Settings</h2>
    <p class="text-xs text-[#9ea2b0] mt-0.5">Preferences, audio engine, and playback parameters</p>
  </div>

  <!-- Appearance Section (ArchiveTune skeleton) -->
  <section class="bg-[#1e2026] rounded-3xl p-5 border border-white/[0.06] space-y-4">
    <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
      <span class="material-symbols-rounded text-purple-400 text-[20px]">palette</span>
      <h3 class="font-heading font-bold text-sm text-white">Appearance</h3>
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Dynamic Monet Theme</p>
        <p class="text-xs text-[#9ea2b0]">Extract dynamic accent colors from current album cover art</p>
      </div>
      <input
        type="checkbox"
        bind:checked={dynamicMonetEnabled}
        class="w-5 h-5 rounded accent-purple-500 cursor-pointer"
      />
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Current Accent Palette</p>
        <p class="text-xs text-[#9ea2b0]">Extracted HCT / Monet tone</p>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-6 h-6 rounded-full border border-white/20 shadow-sm" style="background-color: {playerState.accentColor};"></div>
        <span class="font-mono text-xs text-[#9ea2b0]">{playerState.accentColor}</span>
      </div>
    </div>
  </section>

  <!-- Audio & Playback Section -->
  <section class="bg-[#1e2026] rounded-3xl p-5 border border-white/[0.06] space-y-4">
    <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
      <span class="material-symbols-rounded text-emerald-400 text-[20px]">graphic_eq</span>
      <h3 class="font-heading font-bold text-sm text-white">Audio & Playback</h3>
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Audio Output Sink</p>
        <p class="text-xs text-[#9ea2b0]">PipeWire (Bit-Perfect Lossless on Arch Linux)</p>
      </div>
      <span class="text-xs font-mono px-2.5 py-1 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
        Active (0.8ms latency)
      </span>
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Gapless Audio Playback</p>
        <p class="text-xs text-[#9ea2b0]">Pre-buffer next track for seamless audio transitions</p>
      </div>
      <input
        type="checkbox"
        bind:checked={gaplessEnabled}
        class="w-5 h-5 rounded accent-emerald-500 cursor-pointer"
      />
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">ReplayGain Volume Normalization</p>
        <p class="text-xs text-[#9ea2b0]">Prevent sudden volume spikes between different albums</p>
      </div>
      <input
        type="checkbox"
        bind:checked={replayGainEnabled}
        class="w-5 h-5 rounded accent-emerald-500 cursor-pointer"
      />
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Headless MPV IPC Engine</p>
        <p class="text-xs text-[#9ea2b0]">Unix domain socket at /tmp/aethon-mpv.sock</p>
      </div>
      <span class="text-xs font-mono text-[#9ea2b0]">Running</span>
    </div>
  </section>

  <!-- Player & Content Section -->
  <section class="bg-[#1e2026] rounded-3xl p-5 border border-white/[0.06] space-y-4">
    <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
      <span class="material-symbols-rounded text-indigo-400 text-[20px]">lyrics</span>
      <h3 class="font-heading font-bold text-sm text-white">Player & Content</h3>
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Synced Lyrics Source</p>
        <p class="text-xs text-[#9ea2b0]">LRCLIB Community Database</p>
      </div>
      <span class="text-xs font-semibold px-2.5 py-1 rounded-full bg-indigo-500/20 text-indigo-300 border border-indigo-500/30">
        LRCLIB (Active)
      </span>
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">YouTube Streaming Quality</p>
        <p class="text-xs text-[#9ea2b0]">Highest available Opus stream via yt-dlp</p>
      </div>
      <span class="text-xs font-mono text-[#9ea2b0]">Opus ~256kbps</span>
    </div>
  </section>

  <!-- Storage & Library Section -->
  <section class="bg-[#1e2026] rounded-3xl p-5 border border-white/[0.06] space-y-4">
    <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
      <span class="material-symbols-rounded text-rose-400 text-[20px]">folder</span>
      <h3 class="font-heading font-bold text-sm text-white">Storage & Library</h3>
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Library Directory</p>
        <p class="text-xs text-[#9ea2b0]">~/Music</p>
      </div>
      <button
        onclick={handleRescan}
        disabled={isScanning}
        class="px-4 py-2 rounded-xl bg-white/[0.08] hover:bg-white/[0.14] text-xs font-semibold text-white transition-colors"
      >
        {isScanning ? 'Scanning...' : 'Rescan Directory'}
      </button>
    </div>

    <div class="flex items-center justify-between py-1">
      <div>
        <p class="text-sm font-semibold text-white">Local SQLite Database</p>
        <p class="text-xs text-[#9ea2b0]">~/.local/share/aethon/aethon.db</p>
      </div>
      <span class="text-xs font-mono text-[#9ea2b0]">Encrypted / Offline</span>
    </div>
  </section>

</div>
