<script lang="ts">
  import { scale, fly } from 'svelte/transition';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from './ArtistLinks.svelte';

  const currentTrack = $derived(playerState.currentTrack);
  const isPlaying = $derived(playerState.playback.is_playing);
  const isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));
  const progressPercent = $derived(playerState.progressPercent);
  const formattedTime = $derived(playerState.formattedTime);

  function handleSeek(e: MouseEvent) {
    const bar = e.currentTarget as HTMLElement;
    const rect = bar.getBoundingClientRect();
    const pos = (e.clientX - rect.left) / rect.width;
    const duration = playerState.playback.duration || (currentTrack?.duration ?? 0);
    playerState.seek(pos * duration);
  }
</script>

{#if playerState.isMiniPlayerOpen}
  <div
    in:scale={{ start: 0.9, duration: 240, opacity: 0 }}
    out:scale={{ start: 0.9, duration: 180, opacity: 0 }}
    class="fixed bottom-6 right-6 z-50 w-80 rounded-2xl bg-[#141218] border border-white/15 p-3.5 shadow-2xl shadow-black/90 flex flex-col gap-2.5 select-none group origin-bottom-right"
    style="background-color: #141218;"
    role="region"
    aria-label="Mini Player"
  >
    <div class="flex items-center gap-3">
      <div class="relative w-12 h-12 rounded-xl overflow-hidden shadow-md flex-shrink-0 bg-white/5 border border-white/10">
        {#if currentTrack?.cover_art}
          <img
            src={getHighResArtwork(currentTrack.cover_art, currentTrack.id)}
            alt={currentTrack.title}
            class="w-full h-full object-cover transition-transform duration-700 {isPlaying ? 'scale-105' : 'scale-100'}"
            onerror={(e) => {
              const target = e.currentTarget as HTMLImageElement;
              if (target.src.includes('maxresdefault.jpg')) {
                target.src = target.src.replace('maxresdefault.jpg', 'hqdefault.jpg');
              }
            }}
          />
        {:else}
          <div class="w-full h-full flex items-center justify-center text-white/40">
            <span class="material-symbols-rounded text-xl">music_note</span>
          </div>
        {/if}
      </div>

      <div class="min-w-0 flex-1">
        <h4 class="text-xs font-semibold text-white truncate leading-tight">
          {currentTrack ? currentTrack.title : 'Not Playing'}
        </h4>
        <ArtistLinks
          artist={currentTrack ? currentTrack.artist : ''}
          class="mt-0.5 block max-w-full"
          buttonClass="text-[11px] text-white/70 hover:text-white transition-colors cursor-pointer text-left leading-tight"
        />
      </div>

      <div class="flex items-center gap-1 flex-shrink-0">
        <button
          type="button"
          class="p-1 rounded-lg text-white/50 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          onclick={() => {
            playerState.isMiniPlayerOpen = false;
            playerState.isNowPlayingOpen = true;
          }}
          title="Expand Now Playing"
        >
          <span class="material-symbols-rounded text-base">open_in_full</span>
        </button>
        <button
          type="button"
          class="p-1 rounded-lg text-white/50 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          onclick={() => {
            playerState.isMiniPlayerOpen = false;
          }}
          title="Close Mini Player"
        >
          <span class="material-symbols-rounded text-base">close</span>
        </button>
      </div>
    </div>

    <button
      type="button"
      class="w-full h-1.5 rounded-full bg-white/10 cursor-pointer relative overflow-hidden p-0 border-0"
      onclick={handleSeek}
      aria-label="Audio progress scrubber"
    >
      <div
        class="h-full bg-[var(--md-sys-primary,#d0bcff)] rounded-full transition-[width] duration-150"
        style="width: {progressPercent}%;"
      ></div>
    </button>

    <div class="flex items-center justify-between text-[10px] text-white/50 px-0.5">
      <span>{formattedTime.current}</span>

      <div class="flex items-center gap-2">
        <button
          type="button"
          class="p-1 rounded-full text-white/70 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          onclick={() => playerState.playPrevious()}
          title="Previous Track"
        >
          <span class="material-symbols-rounded text-base">skip_previous</span>
        </button>

        <button
          type="button"
          class="mini-squircle-btn w-9 h-9 {isPlaying ? 'is-playing' : ''} bg-white text-black flex items-center justify-center active:scale-90 hover:scale-105 cursor-pointer select-none shadow-md"
          onclick={() => playerState.togglePlay()}
          title={isBuffering ? 'Buffering...' : isPlaying ? 'Pause' : 'Play'}
        >
          {#if isBuffering}
            <svg class="w-4 h-4 animate-spin text-black" viewBox="0 0 36 36" fill="none">
              <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.25" stroke-width="3.4" />
              <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.4" stroke-dasharray="52 100" stroke-linecap="round" />
            </svg>
          {:else if isPlaying}
            <svg viewBox="0 0 960 960" class="w-5 h-5 fill-current" xmlns="http://www.w3.org/2000/svg">
              <path d="M640,760Q607,760 583.5,736.5Q560,713 560,680L560,280Q560,247 583.5,223.5Q607,200 640,200L640,200Q673,200 696.5,223.5Q720,247 720,280L720,680Q720,713 696.5,736.5Q673,760 640,760ZM320,760Q287,760 263.5,736.5Q240,713 240,680L240,280Q240,247 263.5,223.5Q287,200 320,200L320,200Q353,200 376.5,223.5Q400,247 400,280L400,680Q400,713 376.5,736.5Q353,760 320,760Z" />
            </svg>
          {:else}
            <svg viewBox="42.5 0 960 960" class="w-5 h-5 fill-current" xmlns="http://www.w3.org/2000/svg">
              <path d="M320,687L320,273Q320,256 332,244.5Q344,233 360,233Q365,233 370.5,234.5Q376,236 381,239L707,446Q716,452 720.5,461Q725,470 725,480Q725,490 720.5,499Q716,508 707,514L381,721Q376,724 370.5,725.5Q365,727 360,727Q344,727 332,715.5Q320,704 320,687Z" />
            </svg>
          {/if}
        </button>

        <button
          type="button"
          class="p-1 rounded-full text-white/70 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          onclick={() => playerState.playNext()}
          title="Next Track"
        >
          <span class="material-symbols-rounded text-base">skip_next</span>
        </button>
      </div>

      <span>{formattedTime.total}</span>
    </div>
  </div>
{/if}

<style>
  .mini-squircle-btn {
    border-radius: 50%;
    transition: border-radius 260ms cubic-bezier(0.2, 0, 0, 1), transform 150ms cubic-bezier(0.2, 0, 0, 1);
    will-change: border-radius, transform;
  }
  .mini-squircle-btn.is-playing {
    border-radius: 28%;
  }
</style>
