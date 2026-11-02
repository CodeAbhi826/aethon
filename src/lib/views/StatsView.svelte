<script lang="ts">
  import { playerState } from '../state.svelte';

  const mostPlayed = $derived.by(() => {
    return [...playerState.library]
      .sort((a, b) => b.play_count - a.play_count)
      .slice(0, 5);
  });

  const codecBreakdown = $derived.by(() => {
    const total = playerState.library.length;
    if (total === 0) {
      return [
        { label: 'FLAC / ALAC / WAV (Lossless)', pct: 0, color: 'bg-emerald-400' },
        { label: 'Opus / OGG / AAC', pct: 0, color: 'bg-indigo-400' },
        { label: 'MP3 / Other', pct: 0, color: 'bg-purple-400' },
      ];
    }
    let lossless = 0;
    let modernLossy = 0;
    let mp3Other = 0;
    for (const t of playerState.library) {
      const f = (t.format || '').toUpperCase();
      if (/FLAC|ALAC|WAV/.test(f)) lossless++;
      else if (/OPUS|OGG|AAC|M4A/.test(f)) modernLossy++;
      else mp3Other++;
    }
    return [
      { label: 'FLAC / ALAC / WAV (Lossless)', pct: Math.round((lossless / total) * 100), color: 'bg-emerald-400' },
      { label: 'Opus / OGG / AAC', pct: Math.round((modernLossy / total) * 100), color: 'bg-indigo-400' },
      { label: 'MP3 / Other', pct: Math.round((mp3Other / total) * 100), color: 'bg-purple-400' },
    ];
  });
</script>

<div class="p-6 space-y-6 max-w-7xl mx-auto pb-28">
  <div>
    <h2 class="font-heading font-bold text-2xl text-white">Listening Statistics</h2>
    <p class="text-xs text-[#9ea2b0] mt-0.5">High-fidelity metrics and playback analytics</p>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <div class="p-5 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <div class="flex items-center gap-2 text-[#9ea2b0] text-xs font-medium mb-3">
        <span class="material-symbols-rounded text-emerald-400 text-[18px]">timer</span>
        <span>Total Audio Time</span>
      </div>
      <div class="font-heading font-bold text-3xl text-white">
        {(playerState.library.reduce((acc, t) => acc + (t.duration * t.play_count), 0) / 3600).toFixed(1)}
        <span class="text-base text-[#9ea2b0] font-normal">hrs</span>
      </div>
      <p class="text-[11px] text-[#6b6e7c] mt-2">Total playback time logged locally</p>
    </div>

    <div class="p-5 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <div class="flex items-center gap-2 text-[#9ea2b0] text-xs font-medium mb-3">
        <span class="material-symbols-rounded text-indigo-400 text-[18px]">library_music</span>
        <span>Indexed Tracks</span>
      </div>
      <div class="font-heading font-bold text-3xl text-white">
        {playerState.library.length}
      </div>
      <p class="text-[11px] text-[#6b6e7c] mt-2">Local & favorited tracks in SQLite</p>
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

  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <div class="p-6 rounded-3xl bg-[#1e2026] border border-white/[0.06] shadow-sm">
      <h3 class="font-heading font-bold text-base text-white mb-4">Live Library Codec Distribution</h3>
      <div class="space-y-3">
        {#each codecBreakdown as item}
          <div>
            <div class="flex justify-between text-xs text-[#9ea2b0] mb-1 font-mono">
              <span>{item.label}</span>
              <span>{item.pct}%</span>
            </div>
            <div class="h-2 w-full bg-white/[0.05] rounded-full overflow-hidden">
              <div class="h-full {item.color} rounded-full transition-all duration-300" style="width: {item.pct}%;"></div>
            </div>
          </div>
        {/each}
      </div>
    </div>

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


