<script lang="ts">
  import { playerState } from '../state.svelte';

  const currentTrack = $derived(playerState.currentTrack);
  const isPlaying = $derived(playerState.playback.is_playing);
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
    class="fixed bottom-6 right-6 z-50 w-80 rounded-2xl bg-[#141218]/95 backdrop-blur-2xl border border-white/15 p-3.5 shadow-2xl shadow-black/90 flex flex-col gap-2.5 animate-in slide-in-from-bottom-5 duration-200 select-none group"
    role="region"
    aria-label="Mini Player"
  >
    <!-- Top Row: Cover, Info, Controls -->
    <div class="flex items-center gap-3">
      <!-- Album Cover / Vinyl -->
      <div class="relative w-12 h-12 rounded-xl overflow-hidden shadow-md flex-shrink-0 bg-white/5 border border-white/10">
        {#if currentTrack?.cover_art}
          <img
            src={currentTrack.cover_art}
            alt={currentTrack.title}
            class="w-full h-full object-cover transition-transform duration-700 {isPlaying ? 'scale-105' : 'scale-100'}"
          />
        {:else}
          <div class="w-full h-full flex items-center justify-center text-white/40">
            <span class="material-symbols-rounded text-xl">music_note</span>
          </div>
        {/if}
      </div>

      <!-- Track Info -->
      <div class="min-w-0 flex-1">
        <h4 class="text-xs font-semibold text-white truncate leading-tight">
          {currentTrack ? currentTrack.title : 'Not Playing'}
        </h4>
        <p class="text-[11px] text-white/75 truncate leading-tight mt-0.5">
          {currentTrack ? currentTrack.artist : 'Select a song'}
        </p>
      </div>

      <!-- Close / Expand Mini Player -->
      <div class="flex items-center gap-1 flex-shrink-0">
        <button
          type="button"
          class="p-1 rounded-lg text-white/50 hover:text-white hover:bg-white/10 transition-colors"
          onclick={() => { playerState.isMiniPlayerOpen = false; playerState.isNowPlayingOpen = true; }}
          title="Expand Now Playing"
          aria-label="Expand"
        >
          <span class="material-symbols-rounded text-base">open_in_full</span>
        </button>
        <button
          type="button"
          class="p-1 rounded-lg text-white/50 hover:text-white hover:bg-white/10 transition-colors"
          onclick={() => { playerState.isMiniPlayerOpen = false; }}
          title="Close Mini Player"
          aria-label="Close"
        >
          <span class="material-symbols-rounded text-base">close</span>
        </button>
      </div>
    </div>

    <!-- Scrubber -->
    <button
      type="button"
      class="w-full h-1.5 rounded-full bg-white/10 cursor-pointer relative overflow-hidden group/bar p-0 border-0"
      onclick={handleSeek}
      aria-label="Audio progress scrubber"
    >
      <div
        class="h-full bg-[var(--md-sys-primary,#d0bcff)] rounded-full transition-[width] duration-150"
        style="width: {progressPercent}%;"
      ></div>
    </button>

    <!-- Bottom Controls & Time -->
    <div class="flex items-center justify-between text-[10px] text-white/50 px-0.5">
      <span>{formattedTime.current}</span>

      <div class="flex items-center gap-2">
        <button
          type="button"
          class="p-1 rounded-full text-white/70 hover:text-white hover:bg-white/10 transition-colors"
          onclick={() => playerState.playPrevious()}
          title="Previous Track"
          aria-label="Previous Track"
        >
          <span class="material-symbols-rounded text-base">skip_previous</span>
        </button>

        <button
          type="button"
          class="w-7 h-7 rounded-full bg-white text-black flex items-center justify-center hover:scale-105 active:scale-95 transition-transform"
          onclick={() => playerState.togglePlay()}
          title={playerState.isLoading ? 'Buffering stream...' : isPlaying ? 'Pause' : 'Play'}
          aria-label={playerState.isLoading ? 'Buffering stream...' : isPlaying ? 'Pause' : 'Play'}
        >
          {#if playerState.isLoading}
            <svg class="w-3.5 h-3.5 animate-spin [animation-timing-function:steps(8)] text-current" viewBox="0 0 24 24" fill="none" stroke="currentColor">
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
            {#if isPlaying}
              <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-current" xmlns="http://www.w3.org/2000/svg">
                <rect x="6" y="4" width="4" height="16" rx="1.5" />
                <rect x="14" y="4" width="4" height="16" rx="1.5" />
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-current translate-x-0.5" xmlns="http://www.w3.org/2000/svg">
                <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11.04-6.86a1 1 0 0 0 0-1.72L9.5 4.28A1 1 0 0 0 8 5.14z"/>
              </svg>
            {/if}
          {/if}
        </button>

        <button
          type="button"
          class="p-1 rounded-full text-white/70 hover:text-white hover:bg-white/10 transition-colors"
          onclick={() => playerState.playNext()}
          title="Next Track"
          aria-label="Next Track"
        >
          <span class="material-symbols-rounded text-base">skip_next</span>
        </button>
      </div>

      <span>{formattedTime.total}</span>
    </div>
  </div>
{/if}
