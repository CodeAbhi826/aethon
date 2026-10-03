<script lang="ts">
  import { playerState } from '../state.svelte';

  let showVolumeSlider = $state(false);

  function handleSeek(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    playerState.seek(val);
  }

  function handleVolume(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    playerState.setVolume(val);
  }
</script>

<div class="h-20 w-full px-4 pb-2 z-20 shrink-0">
  <div class="h-full w-full bg-[#18191e]/90 backdrop-blur-3xl border border-white/[0.08] rounded-2xl px-4 flex items-center justify-between shadow-2xl relative overflow-hidden">
    
    <!-- Top thin progress bar for ambient progress -->
    <div
      class="absolute top-0 left-0 h-[2px] bg-gradient-to-r from-purple-400 via-indigo-400 to-emerald-400 transition-all duration-200"
      style="width: {playerState.progressPercent}%;"
    ></div>

    <!-- Left: Track info + Cover Art -->
    <div class="flex items-center gap-3 w-1/4 min-w-[200px]">
      {#if playerState.currentTrack}
        <button
          onclick={() => playerState.isNowPlayingOpen = true}
          class="relative w-12 h-12 rounded-xl overflow-hidden shadow-md group shrink-0 border border-white/[0.08]"
        >
          {#if playerState.currentTrack.cover_art}
            <img src={playerState.currentTrack.cover_art} alt="Cover" class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
          {:else}
            <div class="w-full h-full bg-[#262830] flex items-center justify-center">
              <span class="material-symbols-rounded text-[#9ea2b0] text-[20px]">music_note</span>
            </div>
          {/if}
          <div class="absolute inset-0 bg-black/30 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
            <span class="material-symbols-rounded text-white text-[20px]">expand_less</span>
          </div>
        </button>

        <div class="flex flex-col min-w-0 pr-2">
          <div class="flex items-center gap-1.5">
            <span class="font-heading font-semibold text-sm text-white truncate hover:underline cursor-pointer" onclick={() => playerState.isNowPlayingOpen = true}>
              {playerState.currentTrack.title}
            </span>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-xs text-[#9ea2b0] truncate">{playerState.currentTrack.artist}</span>
            <span class="text-[10px] font-medium px-1.5 py-0.2 rounded bg-white/[0.07] text-[#9ea2b0] tracking-wide uppercase shrink-0">
              {playerState.currentTrack.format || 'Lossless'}
            </span>
          </div>
        </div>

        <button
          onclick={() => playerState.currentTrack && playerState.toggleFavorite(playerState.currentTrack.id)}
          class="text-[#9ea2b0] hover:text-rose-400 transition-colors ml-1"
          title="Favorite"
        >
          <span class="material-symbols-rounded text-[20px] {playerState.currentTrack.favorite ? 'text-rose-400 filled' : ''}">
            favorite
          </span>
        </button>
      {:else}
        <div class="flex items-center gap-3 text-[#6b6e7c]">
          <div class="w-12 h-12 rounded-xl bg-white/[0.03] border border-white/[0.04] flex items-center justify-center">
            <span class="material-symbols-rounded text-[22px]">music_note</span>
          </div>
          <span class="text-xs">No media playing</span>
        </div>
      {/if}
    </div>

    <!-- Center: Playback controls + Main scrubber -->
    <div class="flex flex-col items-center justify-center flex-1 max-w-xl px-4">
      <div class="flex items-center gap-5 mb-1">
        <button
          onclick={() => playerState.playPrevious()}
          class="text-[#9ea2b0] hover:text-white transition-colors"
          title="Previous Track"
        >
          <span class="material-symbols-rounded text-[22px]">skip_previous</span>
        </button>

        <button
          onclick={() => playerState.togglePlay()}
          class="w-10 h-10 rounded-full bg-white text-[#111215] flex items-center justify-center shadow-lg hover:scale-105 active:scale-95 transition-all"
          title={playerState.playback.is_playing ? 'Pause' : 'Play'}
        >
          <span class="material-symbols-rounded text-[26px] filled">
            {playerState.playback.is_playing ? 'pause' : 'play_arrow'}
          </span>
        </button>

        <button
          onclick={() => playerState.playNext()}
          class="text-[#9ea2b0] hover:text-white transition-colors"
          title="Next Track"
        >
          <span class="material-symbols-rounded text-[22px]">skip_next</span>
        </button>
      </div>

      <!-- Scrubber bar -->
      <div class="w-full flex items-center gap-2.5 text-[11px] font-mono text-[#6b6e7c]">
        <span class="w-8 text-right">{playerState.formattedTime.current}</span>
        <input
          type="range"
          min="0"
          max={playerState.playback.duration || (playerState.currentTrack?.duration ?? 100)}
          step="0.5"
          value={playerState.playback.position}
          oninput={handleSeek}
          class="flex-1 h-1.5 bg-white/[0.1] rounded-lg appearance-none cursor-pointer focus:outline-none"
        />
        <span class="w-8">{playerState.formattedTime.total}</span>
      </div>
    </div>

    <!-- Right: Lyrics, Equalizer, Volume, Expand -->
    <div class="flex items-center justify-end gap-3 w-1/4 min-w-[200px]">
      <button
        onclick={() => playerState.isNowPlayingOpen = true}
        class="text-[#9ea2b0] hover:text-white transition-colors p-1.5 rounded-lg hover:bg-white/[0.05]"
        title="Lyrics"
      >
        <span class="material-symbols-rounded text-[20px]">lyrics</span>
      </button>

      <button
        onclick={() => playerState.isEqualizerOpen = true}
        class="text-[#9ea2b0] hover:text-white transition-colors p-1.5 rounded-lg hover:bg-white/[0.05]"
        title="10-Band Equalizer"
      >
        <span class="material-symbols-rounded text-[20px]">tune</span>
      </button>

      <!-- Volume control -->
      <div class="relative flex items-center">
        <button
          onclick={() => showVolumeSlider = !showVolumeSlider}
          class="text-[#9ea2b0] hover:text-white transition-colors p-1.5 rounded-lg hover:bg-white/[0.05]"
          title="Volume"
        >
          <span class="material-symbols-rounded text-[20px]">
            {playerState.playback.volume === 0 ? 'volume_off' : playerState.playback.volume < 50 ? 'volume_down' : 'volume_up'}
          </span>
        </button>
        
        {#if showVolumeSlider}
          <div class="absolute bottom-10 right-0 bg-[#22232a] border border-white/[0.1] p-2.5 rounded-xl shadow-2xl flex items-center gap-2 w-36 z-30">
            <input
              type="range"
              min="0"
              max="100"
              value={playerState.playback.volume}
              oninput={handleVolume}
              class="w-full h-1.5 bg-white/[0.15] rounded-lg appearance-none cursor-pointer"
            />
            <span class="text-[10px] font-mono text-[#9ea2b0] w-6">{Math.round(playerState.playback.volume)}%</span>
          </div>
        {/if}
      </div>

      <button
        onclick={() => playerState.isNowPlayingOpen = !playerState.isNowPlayingOpen}
        class="text-[#9ea2b0] hover:text-white transition-colors p-1.5 rounded-lg hover:bg-white/[0.05]"
        title="Full Player"
      >
        <span class="material-symbols-rounded text-[20px]">open_in_full</span>
      </button>
    </div>

  </div>
</div>
