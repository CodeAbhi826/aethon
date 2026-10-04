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

<footer class="h-20 w-full m3-acrylic-dock px-3 sm:px-6 md:px-8 py-2 flex items-center justify-between z-30 shrink-0 select-none gap-2 sm:gap-4">
  
  <!-- Left: Track Details & Album Thumbnail -->
  <div class="flex items-center gap-2.5 sm:gap-3.5 min-w-0 max-w-[180px] sm:max-w-[260px] md:max-w-[320px] shrink">
    {#if playerState.currentTrack}
      <button
        onclick={() => playerState.isNowPlayingOpen = true}
        class="cursor-pointer relative group w-11 h-11 sm:w-14 sm:h-14 rounded-xl sm:rounded-2xl overflow-hidden shadow-lg shrink-0 border border-white/10"
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
          <span class="material-symbols-rounded text-white text-lg sm:text-xl">open_in_full</span>
        </div>
      </button>

      <div class="min-w-0 flex-1">
        <button
          onclick={() => playerState.isNowPlayingOpen = true}
          class="text-xs sm:text-sm font-bold text-white truncate hover:underline font-display text-left block w-full"
        >
          {playerState.currentTrack.title}
        </button>
        <div class="text-[11px] sm:text-xs text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5">
          <button
            onclick={() => playerState.openArtist(playerState.currentTrack?.artist || '')}
            class="hover:underline hover:text-white transition text-left"
          >
            {playerState.currentTrack.artist}
          </button>
        </div>
        <div class="flex items-center gap-1.5 mt-0.5 hidden sm:flex">
          <span class="text-[9px] font-bold uppercase tracking-wider px-2 py-0.5 rounded-full bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)]">
            {playerState.currentTrack.format || 'Opus 256k'}
          </span>
        </div>
      </div>

      <button
        onclick={() => playerState.currentTrack && playerState.toggleFavorite(playerState.currentTrack.id)}
        class="p-1.5 sm:p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] transition shrink-0 hidden xs:inline-flex"
        title="Favorite"
      >
        <span class="material-symbols-rounded text-lg sm:text-xl {playerState.currentTrack.favorite ? 'text-rose-400 filled' : ''}">
          favorite
        </span>
      </button>
    {:else}
      <div class="flex items-center gap-2.5 sm:gap-3 text-[var(--md-sys-on-surface-variant)]">
        <div class="w-11 h-11 sm:w-14 sm:h-14 rounded-xl sm:rounded-2xl bg-white/5 border border-white/5 flex items-center justify-center shrink-0">
          <span class="material-symbols-rounded text-lg sm:text-xl opacity-40">music_note</span>
        </div>
        <div class="text-xs min-w-0">
          <div class="font-bold text-white/70 truncate">No track playing</div>
          <div class="text-[10px] text-white/40 truncate hidden sm:block">Select a song to start listening</div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Center: Playback Transport & Precision Scrubber -->
  <div class="flex flex-col items-center max-w-xl flex-1 px-2 min-w-0">
    <div class="flex items-center gap-2.5 sm:gap-4 mb-1">
      <button
        onclick={() => playerState.toggleShuffle()}
        class="p-1 rounded-full {playerState.shuffleMode ? 'text-[var(--md-sys-primary)]' : 'text-[var(--md-sys-on-surface-variant)]/60 hover:text-white'} transition cursor-pointer"
        title={playerState.shuffleMode ? 'Shuffle On' : 'Shuffle Off'}
      >
        <span class="material-symbols-rounded text-base sm:text-lg">shuffle</span>
      </button>

      <button
        onclick={() => playerState.playPrevious()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white transition cursor-pointer"
        title="Previous Track"
      >
        <span class="material-symbols-rounded text-xl sm:text-2xl">skip_previous</span>
      </button>

      <button
        onclick={() => playerState.togglePlay()}
        class="w-9 h-9 sm:w-11 sm:h-11 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg shadow-[var(--md-sys-glow)] hover:scale-105 active:scale-95 transition cursor-pointer"
        title={playerState.isLoading ? 'Buffering stream...' : playerState.playback.is_playing ? 'Pause' : 'Play'}
      >
        {#if playerState.isLoading}
          <div class="w-4 h-4 sm:w-5 sm:h-5 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
        {:else}
          <span class="material-symbols-rounded filled text-xl sm:text-2xl">
            {playerState.playback.is_playing ? 'pause' : 'play_arrow'}
          </span>
        {/if}
      </button>

      <button
        onclick={() => playerState.playNext()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white transition cursor-pointer"
        title="Next Track"
      >
        <span class="material-symbols-rounded text-xl sm:text-2xl">skip_next</span>
      </button>

      <button
        onclick={() => playerState.toggleRepeat()}
        class="p-1 rounded-full {playerState.repeatMode !== 'off' ? 'text-[var(--md-sys-primary)]' : 'text-[var(--md-sys-on-surface-variant)]/60 hover:text-white'} transition cursor-pointer"
        title={`Repeat: ${playerState.repeatMode}`}
      >
        <span class="material-symbols-rounded text-base sm:text-lg">
          {playerState.repeatMode === 'one' ? 'repeat_one' : 'repeat'}
        </span>
      </button>
    </div>

    <!-- Scrubber -->
    <div class="flex items-center gap-2 w-full text-[10px] sm:text-[11px] text-[var(--md-sys-on-surface-variant)] font-mono">
      <span class="w-7 sm:w-8 text-right shrink-0">{playerState.formattedTime.current}</span>
      <input
        type="range"
        min="0"
        max={playerState.playback.duration || (playerState.currentTrack?.duration ?? 100)}
        step="0.5"
        value={playerState.playback.position}
        oninput={handleSeek}
        class="w-full h-1 bg-white/10 rounded-lg appearance-none cursor-pointer"
      />
      <span class="w-7 sm:w-8 shrink-0">{playerState.formattedTime.total}</span>
    </div>
  </div>

  <!-- Right: Queue, Lyrics, Equalizer, Volume, Expand -->
  <div class="flex items-center justify-end gap-1 sm:gap-2.5 shrink-0">
    <button
      onclick={() => playerState.isQueueOpen = !playerState.isQueueOpen}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition relative cursor-pointer"
      title="Playing Next Queue"
    >
      <span class="material-symbols-rounded text-xl {playerState.isQueueOpen ? 'text-[var(--md-sys-primary)]' : ''}">queue_music</span>
      {#if playerState.queue.length > 0}
        <span class="absolute top-1 right-1 w-3.5 h-3.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-[8px] font-bold flex items-center justify-center">
          {playerState.queue.length > 9 ? '9+' : playerState.queue.length}
        </span>
      {/if}
    </button>

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
      onclick={() => playerState.toggleMiniPlayer()}
      class="p-2 rounded-full hover:bg-white/10 {playerState.isMiniPlayerOpen ? 'text-[var(--md-sys-primary)]' : 'text-[var(--md-sys-on-surface-variant)] hover:text-white'} transition"
      title="Mini Player"
    >
      <span class="material-symbols-rounded text-xl">picture_in_picture_alt</span>
    </button>

    <button
      onclick={() => playerState.isNowPlayingOpen = !playerState.isNowPlayingOpen}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition"
      title="Expand Player"
    >
      <span class="material-symbols-rounded text-xl">open_in_full</span>
    </button>
  </div>

</footer>
