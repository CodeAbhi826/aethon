<script lang="ts">
  import { playerState } from '../state.svelte';
  import { sanitizeArtist } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from './ArtistLinks.svelte';

  let showVolumeSlider = $state(false);
  let isLoading = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  function handleSeek(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    playerState.seek(val);
  }

  function handleVolume(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    playerState.setVolume(val);
  }
</script>

<footer class="h-20 w-full m3-acrylic-dock px-3 sm:px-6 md:px-8 py-2 flex items-center justify-between z-30 shrink-0 select-none gap-2 sm:gap-4 relative">
  <!-- Top Thin Indeterminate Buffering Shimmer Line -->
  {#if isLoading}
    <div class="absolute top-0 left-0 right-0 h-[2px] overflow-hidden bg-white/5 pointer-events-none">
      <div class="h-full w-1/3 bg-gradient-to-r from-transparent via-[var(--md-sys-primary)] to-transparent animate-shimmer-bar"></div>
    </div>
  {/if}

  <!-- Left: Track Details & Album Thumbnail -->
  <div class="flex items-center gap-2.5 sm:gap-3.5 min-w-0 max-w-[180px] sm:max-w-[260px] md:max-w-[320px] shrink">
    {#if playerState.currentTrack}
      <button
        onclick={() => (playerState.isNowPlayingOpen = true)}
        class="cursor-pointer relative group w-11 h-11 sm:w-14 sm:h-14 rounded-xl sm:rounded-2xl overflow-hidden shadow-lg shrink-0 border border-white/10"
        title="Expand Now Playing"
      >
        {#if playerState.currentTrack.cover_art}
          <img
            src={getHighResArtwork(playerState.currentTrack.cover_art, playerState.currentTrack.id)}
            class="w-full h-full object-cover group-hover:scale-105 transition duration-300"
            alt="Artwork"
            onerror={(e) => {
              const target = e.currentTarget as HTMLImageElement;
              if (target.src.includes('maxresdefault.jpg')) {
                target.src = target.src.replace('maxresdefault.jpg', 'hqdefault.jpg');
              }
            }}
          />
        {:else}
          <div class="w-full h-full bg-[var(--md-sys-surface-container)] flex items-center justify-center">
            <span class="material-symbols-rounded text-[var(--md-sys-on-surface-variant)] text-xl">music_note</span>
          </div>
        {/if}

        <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition">
          <span class="material-symbols-rounded text-white text-lg sm:text-xl">keyboard_arrow_up</span>
        </div>
      </button>

      <div class="min-w-0 flex-1">
        <button
          onclick={() => (playerState.isNowPlayingOpen = true)}
          class="text-xs sm:text-sm font-bold text-white truncate hover:underline font-display text-left block w-full cursor-pointer"
        >
          {playerState.currentTrack.title}
        </button>
        <div class="text-[11px] sm:text-xs text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
          {#if isLoading}
            <span class="text-[10px] font-medium text-[var(--md-sys-primary)] animate-pulse">Loading stream...</span>
          {:else}
            <ArtistLinks
              artist={playerState.currentTrack.artist}
              class="max-w-full"
              buttonClass="hover:underline hover:text-white transition text-left truncate cursor-pointer"
            />
          {/if}
        </div>
      </div>

      <button
        onclick={() =>
          playerState.currentTrack && playerState.toggleFavorite(playerState.currentTrack.id)}
        class="p-1.5 sm:p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] transition shrink-0 inline-flex cursor-pointer"
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
          <div class="font-bold text-white/70 truncate">Not Playing</div>
          <div class="text-[10px] text-white/40 truncate hidden sm:block">Select a song</div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Center: Playback Transport & Precision Scrubber -->
  <div class="flex flex-col items-center max-w-xl flex-1 px-2 min-w-0">
    <div class="flex items-center gap-2.5 sm:gap-4 mb-1.5">
      <button
        onclick={() => playerState.toggleShuffle()}
        class="p-1 rounded-full {playerState.shuffleMode
          ? 'text-[var(--md-sys-primary)]'
          : 'text-[var(--md-sys-on-surface-variant)]/60 hover:text-white'} transition cursor-pointer"
        title={playerState.shuffleMode ? 'Shuffle On' : 'Shuffle Off'}
      >
        <span class="material-symbols-rounded text-base sm:text-lg">shuffle</span>
      </button>

      <button
        onclick={() => playerState.playPrevious()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white active:scale-90 transition cursor-pointer"
        title="Previous Track"
      >
        <span class="material-symbols-rounded text-xl sm:text-2xl filled">skip_previous</span>
      </button>

      <!-- Play / Pause Button with Single ArchiveTune Circular Spinner -->
      <div class="relative flex items-center justify-center">
        <button
          onclick={() => playerState.togglePlay()}
          class="squircle-morph-btn relative w-9 h-9 sm:w-11 sm:h-11 {playerState.playback.is_playing ? 'is-playing' : ''} bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center active:scale-90 hover:scale-105 cursor-pointer select-none shadow-md shadow-black/30"
          title={isLoading ? 'Buffering stream...' : playerState.playback.is_playing ? 'Pause' : 'Play'}
        >
          {#if isLoading}
            <svg class="w-5 h-5 archivetune-spinner-svg text-current" viewBox="0 0 36 36" fill="none">
              <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3" />
              <circle
                class="archivetune-spinner-arc-secondary"
                cx="18"
                cy="18"
                r="14"
                stroke="currentColor"
                stroke-width="3"
              />
              <circle
                class="archivetune-spinner-arc"
                cx="18"
                cy="18"
                r="14"
                stroke="currentColor"
                stroke-width="3"
              />
            </svg>
          {:else if playerState.playback.is_playing}
            <svg viewBox="0 0 960 960" class="w-5 h-5 sm:w-6 sm:h-6 fill-current" xmlns="http://www.w3.org/2000/svg">
              <path d="M640,760Q607,760 583.5,736.5Q560,713 560,680L560,280Q560,247 583.5,223.5Q607,200 640,200L640,200Q673,200 696.5,223.5Q720,247 720,280L720,680Q720,713 696.5,736.5Q673,760 640,760ZM320,760Q287,760 263.5,736.5Q240,713 240,680L240,280Q240,247 263.5,223.5Q287,200 320,200L320,200Q353,200 376.5,223.5Q400,247 400,280L400,680Q400,713 376.5,736.5Q353,760 320,760Z" />
            </svg>
          {:else}
            <svg viewBox="42.5 0 960 960" class="w-5 h-5 sm:w-6 sm:h-6 fill-current" xmlns="http://www.w3.org/2000/svg">
              <path d="M320,687L320,273Q320,256 332,244.5Q344,233 360,233Q365,233 370.5,234.5Q376,236 381,239L707,446Q716,452 720.5,461Q725,470 725,480Q725,490 720.5,499Q716,508 707,514L381,721Q376,724 370.5,725.5Q365,727 360,727Q344,727 332,715.5Q320,704 320,687Z" />
            </svg>
          {/if}
        </button>
      </div>

      <button
        onclick={() => playerState.playNext()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white active:scale-90 transition cursor-pointer"
        title="Next Track"
      >
        <span class="material-symbols-rounded text-xl sm:text-2xl filled">skip_next</span>
      </button>

      <button
        onclick={() => playerState.toggleRepeat()}
        class="p-1 rounded-full {playerState.repeatMode !== 'off'
          ? 'text-[var(--md-sys-primary)]'
          : 'text-[var(--md-sys-on-surface-variant)]/60 hover:text-white'} transition cursor-pointer"
        title={`Repeat: ${playerState.repeatMode}`}
      >
        <span class="material-symbols-rounded text-base sm:text-lg">
          {playerState.repeatMode === 'one' ? 'repeat_one' : 'repeat'}
        </span>
      </button>
    </div>

    <!-- Scrubber with Active Progress Fill & Buffering Shimmer -->
    <div class="flex items-center gap-2.5 w-full text-[10px] sm:text-[11px] text-[var(--md-sys-on-surface-variant)] font-mono">
      <span class="w-8 text-right shrink-0">{playerState.formattedTime.current}</span>
      <div class="relative flex-1 h-1.5 bg-white/10 hover:h-2 rounded-full overflow-hidden group transition-all">
        {#if isLoading}
          <div class="absolute inset-0 bg-gradient-to-r from-transparent via-[var(--md-sys-primary)]/60 to-transparent animate-shimmer-bar"></div>
        {:else}
          <div
            class="absolute inset-0 bg-[var(--md-sys-primary)] rounded-full origin-left will-change-transform"
            style="transform: scaleX({playerState.progressPercent / 100});"
          ></div>
        {/if}
        <input
          type="range"
          min="0"
          max={playerState.playback.duration || playerState.currentTrack?.duration || Math.max(playerState.playback.position, 100)}
          step="0.5"
          value={playerState.playback.position}
          oninput={handleSeek}
          class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
        />
      </div>
      <span class="w-8 shrink-0">{playerState.formattedTime.total}</span>
    </div>
  </div>

  <!-- Right: Queue, Equalizer, Volume, Expand -->
  <div class="flex items-center justify-end gap-1 sm:gap-2 shrink-0">
    <button
      onclick={() => (playerState.isQueueOpen = !playerState.isQueueOpen)}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition relative cursor-pointer"
      title="Playing Next"
    >
      <span class="material-symbols-rounded text-xl {playerState.isQueueOpen ? 'text-[var(--md-sys-primary)]' : ''}">
        queue_music
      </span>
      {#if playerState.queue.length > 0}
        <span class="absolute top-1 right-1 w-3.5 h-3.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-[8px] font-bold flex items-center justify-center">
          {playerState.queue.length > 9 ? '9+' : playerState.queue.length}
        </span>
      {/if}
    </button>

    <button
      onclick={() => (playerState.isEqualizerOpen = true)}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition cursor-pointer"
      title="Equalizer"
    >
      <span class="material-symbols-rounded text-xl">tune</span>
    </button>

    <!-- Volume -->
    <div class="relative flex items-center">
      <button
        onclick={() => (showVolumeSlider = !showVolumeSlider)}
        class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition cursor-pointer"
        title="Volume"
      >
        <span class="material-symbols-rounded text-xl">
          {playerState.playback.volume === 0
            ? 'volume_off'
            : playerState.playback.volume < 50
              ? 'volume_down'
              : 'volume_up'}
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
            class="w-full h-1 bg-white/20 rounded-lg appearance-none cursor-pointer accent-[var(--md-sys-primary)]"
          />
          <span class="text-[10px] font-mono text-[var(--md-sys-primary)] w-7 text-right">
            {Math.round(playerState.playback.volume)}%
          </span>
        </div>
      {/if}
    </div>

    <button
      onclick={() => playerState.toggleMiniPlayer()}
      class="p-2 rounded-full hover:bg-white/10 {playerState.isMiniPlayerOpen
        ? 'text-[var(--md-sys-primary)]'
        : 'text-[var(--md-sys-on-surface-variant)] hover:text-white'} transition cursor-pointer"
      title="Mini Player"
    >
      <span class="material-symbols-rounded text-xl">picture_in_picture_alt</span>
    </button>

    <button
      onclick={() => (playerState.isNowPlayingOpen = !playerState.isNowPlayingOpen)}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition cursor-pointer"
      title="Expand Player"
    >
      <span class="material-symbols-rounded text-xl">open_in_full</span>
    </button>
  </div>
</footer>

<style>
  @keyframes shimmerBar {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(300%);
    }
  }
  .animate-shimmer-bar {
    animation: shimmerBar 1.25s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }
  .squircle-morph-btn {
    border-radius: 50%;
    transition: border-radius 260ms cubic-bezier(0.2, 0, 0, 1), transform 150ms cubic-bezier(0.2, 0, 0, 1);
    will-change: border-radius, transform;
  }
  .squircle-morph-btn.is-playing {
    border-radius: 28%;
  }
</style>
