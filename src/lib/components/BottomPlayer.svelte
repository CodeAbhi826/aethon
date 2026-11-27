<script lang="ts">
  import { playerState } from '../state.svelte';
  import { handleThumbnailError, handleThumbnailLoad } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from './ArtistLinks.svelte';

  let showVolumeSlider = $state(false);
  let volumeWrapEl = $state<HTMLDivElement>();
  let lastNonZeroVolume = $state(85);
  let isLoading = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  function handleSeek(e: Event) {
    if (!playerState.currentTrack) return;
    const val = Number((e.target as HTMLInputElement).value);
    playerState.seek(val);
  }

  function handleVolume(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    if (val > 0) lastNonZeroVolume = val;
    playerState.setVolume(val);
  }

  function toggleMute() {
    if (playerState.playback.volume > 0) {
      lastNonZeroVolume = playerState.playback.volume;
      playerState.setVolume(0);
    } else {
      playerState.setVolume(lastNonZeroVolume || 85);
    }
  }
</script>

<svelte:window
  onclick={(e) => {
    if (showVolumeSlider && volumeWrapEl && !volumeWrapEl.contains(e.target as Node)) {
      showVolumeSlider = false;
    }
  }}
  onkeydown={(e) => {
    if (showVolumeSlider && e.key === 'Escape') {
      e.stopPropagation();
      showVolumeSlider = false;
    }
  }}
/>

<footer class="h-20 w-full m3-acrylic-dock px-3 sm:px-6 md:px-8 py-2 flex items-center justify-between z-30 shrink-0 select-none gap-2 sm:gap-4 relative">
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
            onload={(e) => handleThumbnailLoad(e, playerState.currentTrack?.id)}
            onerror={(e) => handleThumbnailError(e, playerState.currentTrack?.id)}
            class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
            alt="Artwork"
          />
        {:else}
          <div class="w-full h-full bg-[var(--md-sys-surface-container)] flex items-center justify-center">
            <span class="material-symbols-rounded text-[var(--md-sys-on-surface-variant)] text-xl">music_note</span>
          </div>
        {/if}

        <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
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
        <div class="text-[11px] sm:text-xs text-[var(--md-sys-on-surface-variant)] opacity-75 truncate mt-0.5 flex items-center gap-1.5">
          {#if isLoading}
            <span class="text-[10px] font-medium text-[var(--md-sys-primary)] animate-pulse">Loading stream...</span>
          {:else}
            <ArtistLinks
              artist={playerState.currentTrack.artist}
              class="max-w-full"
              buttonClass="hover:underline hover:text-white transition-colors text-left cursor-pointer"
            />
          {/if}
        </div>
      </div>

      <button
        type="button"
        onclick={() =>
          playerState.currentTrack && playerState.toggleFavorite(playerState.currentTrack.id, playerState.currentTrack)}
        class="p-1.5 sm:p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] transition-colors shrink-0 inline-flex cursor-pointer"
        aria-label={playerState.currentTrack.favorite ? 'Remove favorite' : 'Favorite'}
        aria-pressed={playerState.currentTrack.favorite}
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
        type="button"
        onclick={() => playerState.toggleShuffle()}
        class="p-1 rounded-full {playerState.shuffleMode
          ? 'text-[var(--md-sys-primary)]'
          : 'text-[var(--md-sys-on-surface-variant)] opacity-60 hover:opacity-100 hover:text-white'} transition-colors cursor-pointer"
        aria-label={playerState.shuffleMode ? 'Shuffle On' : 'Shuffle Off'}
        aria-pressed={playerState.shuffleMode}
        title={playerState.shuffleMode ? 'Shuffle On' : 'Shuffle Off'}
      >
        <span class="material-symbols-rounded text-base sm:text-lg">shuffle</span>
      </button>

      <button
        type="button"
        onclick={() => playerState.playPrevious()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white active:scale-90 transition-transform cursor-pointer"
        aria-label="Previous Track"
        title="Previous Track"
      >
        <span class="material-symbols-rounded text-xl sm:text-2xl filled">skip_previous</span>
      </button>

      <!-- Play / Pause Button with Apple Music Morphing Variable-Sweep Loader -->
      <div class="relative flex items-center justify-center">
        <button
          type="button"
          onclick={() => playerState.togglePlay()}
          class="squircle-morph-btn relative w-9 h-9 sm:w-11 sm:h-11 {playerState.playback.is_playing ? 'is-playing' : ''} bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center active:scale-90 hover:scale-105 cursor-pointer select-none shadow-md shadow-black/30"
          aria-label={isLoading ? 'Buffering stream' : playerState.playback.is_playing ? 'Pause' : 'Play'}
          title={isLoading ? 'Buffering stream...' : playerState.playback.is_playing ? 'Pause' : 'Play'}
        >
          {#if isLoading}
            <svg class="w-5 h-5 sm:w-6 sm:h-6 archivetune-spinner-svg text-current" viewBox="0 0 36 36" fill="none">
              <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.16" stroke-width="3.2" />
              <circle
                cx="18"
                cy="18"
                r="14"
                stroke="currentColor"
                stroke-opacity="0.35"
                stroke-width="3.2"
                stroke-dasharray="18 70"
                stroke-dashoffset="-12"
                stroke-linecap="round"
              />
              <circle
                class="archivetune-spinner-arc"
                cx="18"
                cy="18"
                r="14"
                stroke="currentColor"
                stroke-width="3.4"
                stroke-linecap="round"
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
        type="button"
        onclick={() => playerState.playNext()}
        class="p-1 rounded-full text-[var(--md-sys-on-surface-variant)] hover:text-white active:scale-90 transition-transform cursor-pointer"
        aria-label="Next Track"
        title="Next Track"
      >
        <span class="material-symbols-rounded text-xl sm:text-2xl filled">skip_next</span>
      </button>

      <button
        type="button"
        onclick={() => playerState.toggleRepeat()}
        class="p-1 rounded-full {playerState.repeatMode !== 'off'
          ? 'text-[var(--md-sys-primary)]'
          : 'text-[var(--md-sys-on-surface-variant)] opacity-60 hover:opacity-100 hover:text-white'} transition-colors cursor-pointer"
        aria-label={`Repeat: ${playerState.repeatMode}`}
        title={`Repeat: ${playerState.repeatMode}`}
      >
        <span class="material-symbols-rounded text-base sm:text-lg">
          {playerState.repeatMode === 'one' ? 'repeat_one' : 'repeat'}
        </span>
      </button>
    </div>

    <!-- Scrubber with GPU scaleX Progress Fill -->
    <div class="flex items-center gap-2.5 w-full text-[10px] sm:text-[11px] text-[var(--md-sys-on-surface-variant)] font-mono">
      <span class="w-8 text-right shrink-0">{playerState.formattedTime.current}</span>
      <div class="relative flex-1 h-1.5 bg-white/10 rounded-full overflow-hidden group">
        <div
          class="absolute inset-0 bg-[var(--md-sys-primary)] rounded-full origin-left will-change-transform"
          style="transform: scaleX({playerState.progressPercent / 100});"
        ></div>
        <input
          type="range"
          min="0"
          max={playerState.playback.duration || playerState.currentTrack?.duration || Math.max(playerState.playback.position, 100)}
          step="0.5"
          value={playerState.playback.position}
          disabled={!playerState.currentTrack}
          oninput={handleSeek}
          aria-label="Seek audio position"
          class="absolute inset-0 w-full h-full opacity-0 cursor-pointer disabled:cursor-default"
        />
      </div>
      <span class="w-8 shrink-0">{playerState.formattedTime.total}</span>
    </div>
  </div>

  <!-- Right: Queue, Equalizer, Volume, Expand -->
  <div class="flex items-center justify-end gap-1 sm:gap-2 shrink-0">
    <button
      type="button"
      onclick={() => (playerState.isQueueOpen = !playerState.isQueueOpen)}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition-colors relative cursor-pointer"
      aria-label="Playing Next Queue"
      aria-expanded={playerState.isQueueOpen}
      title="Playing Next"
    >
      <span class="material-symbols-rounded text-xl {playerState.isQueueOpen ? 'text-[var(--md-sys-primary)]' : ''}">
        queue_music
      </span>
      {#if playerState.queue.length > 0}
        <span class="absolute top-1 right-1 min-w-[14px] h-3.5 px-0.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-[8px] font-bold flex items-center justify-center">
          {playerState.queue.length > 9 ? '9+' : playerState.queue.length}
        </span>
      {/if}
    </button>

    <button
      type="button"
      onclick={() => (playerState.isEqualizerOpen = true)}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition-colors cursor-pointer"
      aria-label="Equalizer"
      aria-expanded={playerState.isEqualizerOpen}
      title="Equalizer"
    >
      <span class="material-symbols-rounded text-xl">tune</span>
    </button>

    <!-- Volume -->
    <div bind:this={volumeWrapEl} class="relative flex items-center">
      <button
        type="button"
        onclick={() => (showVolumeSlider = !showVolumeSlider)}
        class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition-colors cursor-pointer"
        aria-label="Volume"
        aria-expanded={showVolumeSlider}
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
        <div class="absolute bottom-12 right-0 m3-acrylic-card border border-white/10 p-3 rounded-2xl shadow-2xl flex items-center gap-2 w-48 z-50">
          <button
            type="button"
            onclick={toggleMute}
            aria-label={playerState.playback.volume === 0 ? 'Unmute' : 'Mute'}
            class="p-1 rounded-lg text-white/70 hover:text-white hover:bg-white/10 transition-colors cursor-pointer shrink-0"
          >
            <span class="material-symbols-rounded text-base">
              {playerState.playback.volume === 0 ? 'volume_off' : 'volume_up'}
            </span>
          </button>
          <input
            type="range"
            min="0"
            max="100"
            value={playerState.playback.volume}
            oninput={handleVolume}
            aria-label="Volume"
            class="w-full h-1.5 bg-white/20 rounded-lg cursor-pointer accent-[var(--md-sys-primary)]"
          />
          <span class="text-[10px] font-mono text-[var(--md-sys-primary)] w-7 text-right shrink-0">
            {Math.round(playerState.playback.volume)}%
          </span>
        </div>
      {/if}
    </div>

    <button
      type="button"
      onclick={() => playerState.toggleMiniPlayer()}
      class="p-2 rounded-full hover:bg-white/10 {playerState.isMiniPlayerOpen
        ? 'text-[var(--md-sys-primary)]'
        : 'text-[var(--md-sys-on-surface-variant)] hover:text-white'} transition-colors cursor-pointer"
      aria-label="Mini Player"
      aria-pressed={playerState.isMiniPlayerOpen}
      title="Mini Player"
    >
      <span class="material-symbols-rounded text-xl">picture_in_picture_alt</span>
    </button>

    <button
      type="button"
      onclick={() => (playerState.isNowPlayingOpen = !playerState.isNowPlayingOpen)}
      class="p-2 rounded-full hover:bg-white/10 text-[var(--md-sys-on-surface-variant)] hover:text-white transition-colors cursor-pointer"
      aria-label="Expand Now Playing"
      aria-expanded={playerState.isNowPlayingOpen}
      title="Expand Player"
    >
      <span class="material-symbols-rounded text-xl">open_in_full</span>
    </button>
  </div>
</footer>

<style>
  .squircle-morph-btn {
    border-radius: 50%;
    transition: border-radius 240ms cubic-bezier(0.2, 0, 0, 1), transform 140ms cubic-bezier(0.2, 0, 0, 1);
    will-change: transform;
  }
  .squircle-morph-btn.is-playing {
    border-radius: 28%;
  }
</style>
