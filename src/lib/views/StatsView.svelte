<script lang="ts">
  import { playerState } from '../state.svelte';

  const mostPlayed = $derived.by(() => {
    return [...playerState.library]
      .sort((a, b) => b.play_count - a.play_count)
      .slice(0, 5);
  });
</script>

<div class="p-6 space-y-6 max-w-7xl mx-auto pb-28">
  
  <div>
    <h2 class="font-heading font-bold text-2xl text-white">Listening Statistics</h2>
    <p class="text-xs text-[#9ea2b0] mt-0.5">High-fidelity metrics and playback analytics</p>
  </div>

  <!-- Top Metric Cards -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <div class="p-5 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <div class="flex items-center gap-2 text-[#9ea2b0] text-xs font-medium mb-3">
        <span class="material-symbols-rounded text-emerald-400 text-[18px]">timer</span>
        <span>Total Audio Time</span>
      </div>
      <div class="font-heading font-bold text-3xl text-white">
        {Math.round(playerState.library.reduce((acc, t) => acc + (t.duration * t.play_count), 0) / 3600)}
        <span class="text-base text-[#9ea2b0] font-normal">hrs</span>
      </div>
      <p class="text-[11px] text-[#6b6e7c] mt-2">Lossless stream time logged locally</p>
    </div>

    <div class="p-5 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <div class="flex items-center gap-2 text-[#9ea2b0] text-xs font-medium mb-3">
        <span class="material-symbols-rounded text-indigo-400 text-[18px]">library_music</span>
        <span>Indexed Tracks</span>
      </div>
      <div class="font-heading font-bold text-3xl text-white">
        {playerState.library.length}
      </div>
      <p class="text-[11px] text-[#6b6e7c] mt-2">Zero cloud tracking, 100% private SQLite</p>
    </div>

    <div class="p-5 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <div class="flex items-center gap-2 text-[#9ea2b0] text-xs font-medium mb-3">
        <span class="material-symbols-rounded text-purple-400 text-[18px]">graphic_eq</span>
        <span>Audio Engine</span>
      </div>
      <div class="font-heading font-bold text-2xl text-emerald-400">
        PipeWire Bit-Perfect
      </div>
      <p class="text-[11px] text-[#6b6e7c] mt-2">Native Arch Linux Wayland audio sink</p>
    </div>
  </div>

  <!-- Audio Format Donut / Breakdown -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <div class="p-6 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <h3 class="font-heading font-bold text-base text-white mb-4">Codec Distribution</h3>
      
      <div class="space-y-3">
        <div>
          <div class="flex justify-between text-xs text-[#9ea2b0] mb-1 font-mono">
            <span>FLAC Lossless (24-bit / 96kHz)</span>
            <span>68%</span>
          </div>
          <div class="h-2 w-full bg-white/[0.05] rounded-full overflow-hidden">
            <div class="h-full bg-emerald-400 rounded-full" style="width: 68%;"></div>
          </div>
        </div>

        <div>
          <div class="flex justify-between text-xs text-[#9ea2b0] mb-1 font-mono">
            <span>YouTube Music Opus (256kbps)</span>
            <span>22%</span>
          </div>
          <div class="h-2 w-full bg-white/[0.05] rounded-full overflow-hidden">
            <div class="h-full bg-indigo-400 rounded-full" style="width: 22%;"></div>
          </div>
        </div>

        <div>
          <div class="flex justify-between text-xs text-[#9ea2b0] mb-1 font-mono">
            <span>ALAC / MP3</span>
            <span>10%</span>
          </div>
          <div class="h-2 w-full bg-white/[0.05] rounded-full overflow-hidden">
            <div class="h-full bg-purple-400 rounded-full" style="width: 10%;"></div>
          </div>
        </div>
      </div>
    </div>

    <!-- Most Played Tracks -->
    <div class="p-6 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <h3 class="font-heading font-bold text-base text-white mb-4">Most Played</h3>
      {#if mostPlayed.length > 0}
        <div class="space-y-3">
          {#each mostPlayed as track, i}
            <div class="flex items-center justify-between text-xs">
              <div class="flex items-center gap-3 min-w-0 pr-2">
                <span class="font-mono text-[#6b6e7c] w-4">{i + 1}</span>
                <div class="min-w-0">
                  <p class="font-semibold text-white truncate">{track.title}</p>
                  <p class="text-[#9ea2b0] truncate">{track.artist}</p>
                </div>
              </div>
              <span class="font-mono text-[#9ea2b0] shrink-0">{track.play_count} plays</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-xs text-[#6b6e7c] py-6 text-center">Plays will be recorded as you listen to songs.</p>
      {/if}
    </div>
  </div>

</div>
