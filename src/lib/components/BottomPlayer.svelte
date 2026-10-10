<script lang="ts">
  import { playerState } from '../state.svelte';
  import { sanitizeArtist } from '../types';

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
            onclick={() => playerState.openArtist(sanitizeArtist(playerState.currentTrack?.artist || ''))}
            class="hover:underline hover:text-white transition text-left"
          >
            {sanitizeArtist(playerState.currentTrack.artist)}
          </button>
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
        class="w-9 h-9 sm:w-11 sm:h-11 {playerState.playback.is_playing ? 'rounded-full' : 'rounded-[12px] sm:rounded-[14px]'} bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center active:scale-90 hover:scale-105 transition-all duration-300 [transition-timing-function:cubic-bezier(0.34,1.56,0.64,1)] cursor-pointer select-none shadow-md shadow-black/30 hover:shadow-black/40"
        title={playerState.isLoading ? 'Buffering stream...' : playerState.playback.is_playing ? 'Pause' : 'Play'}
      >
        {#if playerState.isLoading}
          <svg class="w-4 h-4 sm:w-5 sm:h-5 animate-spin [animation-timing-function:steps(8)] text-current" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <line x1="12" y1="2" x2="12" y2="6" stroke-width="2.5" stroke-linecap="round" class="opacity-100" />
            <line x1="19.07" y1="4.93" x2="16.24" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-90" />
            <line x1="22" y1="12" x2="18" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-75" />
            <line x1="19.07" y1="19.07" x2="16.24" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-60" />
            <line x1="12" y1="22" x2="12" y2="18" stroke-width="2.5" stroke-linecap="round" class="opacity-45" />
            <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-30" />
            <line x1="2" y1="12" x2="6" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-20" />
            <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-15" />
          </svg>
        {:else}
          {#if playerState.playback.is_playing}
            <!-- ArchiveTune pause.xml (960x960) -->
            <svg viewBox="0 0 960 960" class="w-5 h-5 sm:w-6 sm:h-6 fill-current transition-transform duration-200" xmlns="http://www.w3.org/2000/svg">
              <path d="M640,760Q607,760 583.5,736.5Q560,713 560,680L560,280Q560,247 583.5,223.5Q607,200 640,200L640,200Q673,200 696.5,223.5Q720,247 720,280L720,680Q720,713 696.5,736.5Q673,760 640,760ZM320,760Q287,760 263.5,736.5Q240,713 240,680L240,280Q240,247 263.5,223.5Q287,200 320,200L320,200Q353,200 376.5,223.5Q400,247 400,280L400,680Q400,713 376.5,736.5Q353,760 320,760Z"/>
            </svg>
          {:else}
            <!-- ArchiveTune play.xml (960x960) centered -->
            <svg viewBox="0 0 960 960" class="w-5 h-5 sm:w-6 sm:h-6 fill-current transition-transform duration-200" xmlns="http://www.w3.org/2000/svg">
              <path d="M320,687L320,273Q320,256 332,244.5Q344,233 360,233Q365,233 370.5,234.5Q376,236 381,239L707,446Q716,452 720.5,461Q725,470 725,480Q725,490 720.5,499Q716,508 707,514L381,721Q376,724 370.5,725.5Q365,727 360,727Q344,727 332,715.5Q320,704 320,687Z"/>
            </svg>
          {/if}
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

  <!-- Right: Queue, Equalizer, Volume, Expand -->
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
