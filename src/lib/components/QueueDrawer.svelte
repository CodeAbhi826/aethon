<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from './ArtistLinks.svelte';

  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  function appleDrawerEasing(t: number): number {
    return 1 - Math.pow(1 - t, 3.5);
  }

  function handlePlayNow(index: number) {
    const track = playerState.queue[index];
    if (track) {
      playerState.removeFromQueue(index);
      playerState.playTrack(track);
    }
  }

  function handleMoveUp(index: number) {
    if (index <= 0) return;
    const item = playerState.queue[index];
    const newQueue = [...playerState.queue];
    newQueue[index] = newQueue[index - 1];
    newQueue[index - 1] = item;
    playerState.queue = newQueue;
  }

  function handleMoveDown(index: number) {
    if (index >= playerState.queue.length - 1) return;
    const item = playerState.queue[index];
    const newQueue = [...playerState.queue];
    newQueue[index] = newQueue[index + 1];
    newQueue[index + 1] = item;
    playerState.queue = newQueue;
  }
</script>

{#if playerState.isQueueOpen}
  <!-- Backdrop with smooth enter + exit fade -->
  <div
    role="button"
    tabindex="0"
    transition:fade={{ duration: 240 }}
    onclick={() => (playerState.isQueueOpen = false)}
    onkeydown={(e) => {
      if (e.key === 'Escape') playerState.isQueueOpen = false;
    }}
    class="fixed inset-0 bg-black/75 z-50"
  ></div>

  <!-- Sliding Queue Panel with Apple spring enter + exit -->
  <div
    in:fly={{ x: 400, duration: 340, opacity: 1, easing: appleDrawerEasing }}
    out:fly={{ x: 400, duration: 280, opacity: 1, easing: appleDrawerEasing }}
    class="fixed top-0 right-0 h-full w-full sm:w-[390px] m3-acrylic border-l border-white/10 z-50 shadow-2xl flex flex-col justify-between overflow-hidden select-none will-change-transform"
  >
    <!-- Header -->
    <div class="px-5 py-4 border-b border-white/[0.06] flex items-center justify-between">
      <div>
        <h2 class="font-display font-bold text-base text-white">Playing Next</h2>
        <p class="text-[11px] text-white/45">
          {playerState.queue.length} {playerState.queue.length === 1 ? 'song' : 'songs'}
        </p>
      </div>

      <div class="flex items-center gap-1.5">
        {#if playerState.queue.length > 0}
          <button
            onclick={() => playerState.clearQueue()}
            class="px-2.5 py-1 rounded-full text-xs font-medium text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          >
            Clear
          </button>
        {/if}
        <button
          onclick={() => (playerState.isQueueOpen = false)}
          class="w-8 h-8 rounded-full flex items-center justify-center text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
        >
          <span class="material-symbols-rounded text-xl">close</span>
        </button>
      </div>
    </div>

    <!-- Queue Body -->
    <div class="flex-1 overflow-y-auto p-4 space-y-5 scrollbar-thin">
      {#if playerState.currentTrack}
        <div class="space-y-2">
          <span class="text-[11px] font-semibold uppercase tracking-wider text-white/40 px-1">Now Playing</span>
          <div class="p-2.5 rounded-2xl bg-white/[0.06] border border-white/[0.08] flex items-center gap-3">
            <div class="relative w-11 h-11 rounded-xl overflow-hidden shrink-0 bg-white/5">
              {#if playerState.currentTrack.cover_art}
                <img
                  src={getHighResArtwork(playerState.currentTrack.cover_art, playerState.currentTrack.id)}
                  alt={playerState.currentTrack.title}
                  class="w-full h-full object-cover"
                  onerror={(e) => {
                    const target = e.currentTarget as HTMLImageElement;
                    if (target.src.includes('maxresdefault.jpg')) {
                      target.src = target.src.replace('maxresdefault.jpg', 'hqdefault.jpg');
                    }
                  }}
                />
              {:else}
                <div class="w-full h-full flex items-center justify-center">
                  <span class="material-symbols-rounded text-white/40">music_note</span>
                </div>
              {/if}
              <div class="absolute inset-0 bg-black/45 flex items-center justify-center">
                {#if isBuffering}
                  <svg class="w-5 h-5 animate-spin text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                    <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                    <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.2" stroke-dasharray="52 100" stroke-linecap="round" />
                  </svg>
                {:else}
                  <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-lg filled">
                    {playerState.playback.is_playing ? 'equalizer' : 'pause'}
                  </span>
                {/if}
              </div>
            </div>

            <div class="min-w-0 flex-1">
              <div class="text-xs font-semibold text-white truncate">
                {playerState.currentTrack.title}
              </div>
              <ArtistLinks
                artist={playerState.currentTrack.artist}
                class="mt-0.5 block max-w-full"
                buttonClass="text-[11px] text-white/55 hover:text-white transition-colors cursor-pointer text-left"
              />
            </div>
          </div>
        </div>
      {/if}

      <!-- Autoplay Toggle Pill -->
      <div class="flex items-center justify-between px-3 py-2.5 rounded-2xl bg-white/[0.03] border border-white/[0.05]">
        <div class="flex items-center gap-2.5">
          <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-lg">all_inclusive</span>
          <span class="text-xs font-medium text-white">Autoplay similar songs</span>
        </div>
        <button
          onclick={() => (playerState.autoplayRadio = !playerState.autoplayRadio)}
          class="w-9 h-5 rounded-full transition-colors relative cursor-pointer {playerState.autoplayRadio
            ? 'bg-[var(--md-sys-primary)]'
            : 'bg-white/20'}"
          aria-label="Toggle Autoplay"
        >
          <div
            class="w-3.5 h-3.5 rounded-full bg-black absolute top-[3px] transition-transform {playerState.autoplayRadio
              ? 'left-[18px]'
              : 'left-[3px]'}"
          ></div>
        </button>
      </div>

      <!-- Up Next List -->
      <div class="space-y-1.5">
        {#if playerState.queue.length > 0}
          {#each playerState.queue as track, idx}
            <div
              role="listitem"
              oncontextmenu={(e) => {
                e.preventDefault();
                playerState.openContextMenu(track, e.clientX, e.clientY);
              }}
              class="flex items-center justify-between p-2 rounded-xl hover:bg-white/[0.05] transition-colors group"
            >
              <div class="flex items-center gap-3 min-w-0 flex-1 pr-2">
                <button
                  type="button"
                  onclick={() => handlePlayNow(idx)}
                  class="relative w-10 h-10 rounded-lg overflow-hidden shrink-0 bg-white/5 cursor-pointer"
                >
                  {#if track.cover_art}
                    <img
                      loading="lazy"
                      decoding="async"
                      src={getHighResArtwork(track.cover_art, track.id)}
                      alt={track.title}
                      class="w-full h-full object-cover"
                      onerror={(e) => {
                        const target = e.currentTarget as HTMLImageElement;
                        if (target.src.includes('maxresdefault.jpg')) {
                          target.src = target.src.replace('maxresdefault.jpg', 'hqdefault.jpg');
                        }
                      }}
                    />
                  {:else}
                    <div class="w-full h-full flex items-center justify-center">
                      <span class="material-symbols-rounded text-white/40 text-sm">music_note</span>
                    </div>
                  {/if}
                  <div class="absolute inset-0 bg-black/45 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
                    <span class="material-symbols-rounded text-white text-base filled">play_arrow</span>
                  </div>
                </button>

                <div class="min-w-0 flex-1">
                  <div class="text-xs font-semibold text-white truncate">{track.title}</div>
                  <ArtistLinks
                    artist={track.artist}
                    class="mt-0.5 block max-w-full"
                    buttonClass="text-[11px] text-white/50 hover:text-white transition-colors cursor-pointer text-left"
                  />
                </div>
              </div>

              <div class="flex items-center gap-0.5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onclick={() => handleMoveUp(idx)}
                  disabled={idx === 0}
                  class="p-1 rounded text-white/50 hover:text-white disabled:opacity-20 cursor-pointer"
                  title="Move Up"
                >
                  <span class="material-symbols-rounded text-sm">arrow_upward</span>
                </button>
                <button
                  onclick={() => handleMoveDown(idx)}
                  disabled={idx === playerState.queue.length - 1}
                  class="p-1 rounded text-white/50 hover:text-white disabled:opacity-20 cursor-pointer"
                  title="Move Down"
                >
                  <span class="material-symbols-rounded text-sm">arrow_downward</span>
                </button>
                <button
                  onclick={() => playerState.removeFromQueue(idx)}
                  class="p-1 rounded text-white/50 hover:text-rose-400 cursor-pointer"
                  title="Remove"
                >
                  <span class="material-symbols-rounded text-sm">close</span>
                </button>
              </div>
            </div>
          {/each}
        {:else}
          <div class="py-12 text-center text-white/40 text-xs">
            Queue is empty
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
