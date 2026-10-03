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

<footer class="h-20 w-full m3-acrylic-dock px-4 md:px-8 py-2.5 flex items-center justify-between z-30 shrink-0 select-none">
  
  <!-- Left: Track Details & Album Thumbnail -->
  <div class="flex items-center gap-3.5 min-w-[200px] md:min-w-[280px] max-w-[320px]">
    {#if playerState.currentTrack}
      <button
        onclick={() => playerState.isNowPlayingOpen = true}
        class="cursor-pointer relative group w-13 h-13 rounded-2xl overflow-hidden shadow-lg shrink-0 border border-white/10"
        title="Click to Expand Now Playing"
      >
        {#if playerState.currentTrack.cover_art}
          <img src={playerState.currentTrack.cover_art} class="w-full h-full object-cover group-hover:scale-105 transition" alt="Artwork">
        {:else}
          <div class="w-full h-full bg-[var(--md-sys-surface-container)] flex items-center justify-center">
            <span class="material-symbols-rounded text-[var(--md-sys-on-surface-variant)] text-xl">music_note</span>
          </div>
        {/if}
        <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition">
          <span class="material-symbols-rounded text-white text-xl">open_in_full</span>
        </div>
      </button>

      <div class="min-w-0 flex-1">
        <button
          onclick={() => playerState.isNowPlayingOpen = true}
          class="text-sm font-bold text-white truncate hover:underline font-display text-left block w-full"
        >
          {playerState.currentTrack.title}
        </button>
        <div class="text-xs text-[var(--md-sys-on-surface-variant)] truncate">
          {playerState.currentTrack.artist}
        </div>
        <div class="flex items-center gap-1.5 mt-0.5">
          <span class="text-[9px] font-bold uppercase tracking-wider px-2 py-0.5 rounded-full bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)]">
            {playerState.currentTrack.format || 'Lossless'}
          </span>
        </div>
      </div>

      <button
        onclick={() => playerState.currentTrack && playerState.toggleFavorite(playerState.currentTrack.id)}
        class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] transition shrink-0"
        title="Favorite"
      >
        <span class="material-symbols-rounded text-xl {playerState.currentTrack.favorite ? 'text-rose-400 filled' : ''}">
          favorite
        </span>
      </button>
    {:else}
      <div class="flex items-center gap-3 text-[var(--md-sys-on-surface-variant)]">
        <div class="w-13 h-13 rounded-2xl bg-white/5 border border-white/5 flex items-center justify-center">
          <span class="material-symbols-rounded text-xl opacity-40">music_note</span>
        </div>
        <div class="text-xs">
          <div class="font-bold text-white/70">No track playing</div>
          <div class="text-[10px] text-white/40">Select a song to start listening</div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Center: Playback Transport & Precision Scrubber -->
  <div class="flex flex-col items-center max-w-xl w-full px-4">
    <div class="flex items-center gap-5 mb-1">
      <button
        onclick={() => playerState.playPrevious()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white transition"
        title="Previous Track"
      >
        <span class="material-symbols-rounded text-2xl">skip_previous</span>
      </button>

      <button
        onclick={() => playerState.togglePlay()}
        class="w-11 h-11 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg shadow-[var(--md-sys-glow)] hover:scale-105 active:scale-95 transition"
        title={playerState.playback.is_playing ? 'Pause' : 'Play'}
      >
        <span class="material-symbols-rounded filled text-2xl">
          {playerState.playback.is_playing ? 'pause' : 'play_arrow'}
        </span>
      </button>

      <button
        onclick={() => playerState.playNext()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white transition"
        title="Next Track"
      >
        <span class="material-symbols-rounded text-2xl">skip_next</span>
      </button>
    </div>

    <!-- Scrubber -->
    <div class="flex items-center gap-2.5 w-full text-[11px] text-[var(--md-sys-on-surface-variant)] font-mono">
      <span class="w-8 text-right">{playerState.formattedTime.current}</span>
      <input
        type="range"
        min="0"
        max={playerState.playback.duration || (playerState.currentTrack?.duration ?? 100)}
        step="0.5"
        value={playerState.playback.position}
        oninput={handleSeek}
        class="w-full h-1 bg-white/10 rounded-lg appearance-none cursor-pointer"
      />
      <span class="w-8">{playerState.formattedTime.total}</span>
    </div>
  </div>

  <!-- Right: Lyrics, Equalizer, Volume, Expand -->
  <div class="flex items-center justify-end gap-3 min-w-[200px]">
    <button
      onclick={() => playerState.isNowPlayingOpen = true}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition"
      title="Lyrics"
    >
      <span class="material-symbols-rounded text-xl">lyrics</span>
    </button>

    <button
      onclick={() => playerState.isEqualizerOpen = true}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition"
      title="10-Band Equalizer"
    >
      <span class="material-symbols-rounded text-xl">tune</span>
    </button>

    <!-- Volume -->
    <div class="relative flex items-center">
      <button
        onclick={() => showVolumeSlider = !showVolumeSlider}
        class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition"
        title="Volume"
      >
        <span class="material-symbols-rounded text-xl">
          {playerState.playback.volume === 0 ? 'volume_off' : playerState.playback.volume < 50 ? 'volume_down' : 'volume_up'}
        </span>
      </button>

      {#if showVolumeSlider}
        <div class="absolute bottom-12 right-0 m3-acrylic-card border border-white/10 p-3 rounded-2xl shadow-2xl flex items-center gap-2.5 w-40 z-50">
          <input
            type="range"
            min="0"
            max="100"
            value={playerState.playback.volume}
            oninput={handleVolume}
            class="w-full h-1 bg-white/20 rounded-lg appearance-none cursor-pointer"
          />
          <span class="text-[10px] font-mono text-[var(--md-sys-primary)] w-7 text-right">
            {Math.round(playerState.playback.volume)}%
          </span>
        </div>
      {/if}
    </div>

    <button
      onclick={() => playerState.isNowPlayingOpen = !playerState.isNowPlayingOpen}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition"
      title="Expand Player"
    >
      <span class="material-symbols-rounded text-xl">open_in_full</span>
    </button>
  </div>

</footer>
