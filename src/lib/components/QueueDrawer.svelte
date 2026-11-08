<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { playerState } from '../state.svelte';
  import { handleThumbnailError, handleThumbnailLoad } from '../types';
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

  function shuffleQueue() {
    if (playerState.queue.length <= 1) return;
    playerState.queue = [...playerState.queue].sort(() => Math.random() - 0.5);
  }

  function toggleAutomix() {
    playerState.autoplayRadio = !playerState.autoplayRadio;
    if (playerState.autoplayRadio && playerState.queue.length <= 2 && playerState.currentTrack) {
      playerState.populateAutomixQueue(null, playerState.currentTrack);
    }
  }

  function formatDuration(sec: number): string {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }
</script>

{#if playerState.isQueueOpen}
  <!-- Backdrop -->
  <div
    role="button"
    tabindex="0"
    transition:fade={{ duration: 220 }}
    onclick={() => (playerState.isQueueOpen = false)}
    onkeydown={(e) => {
      if (e.key === 'Escape') playerState.isQueueOpen = false;
    }}
    class="fixed inset-0 bg-black/70 z-50"
  ></div>

  <!-- ArchiveTune Up Next & Automix Radio Sheet -->
  <div
    in:fly={{ x: 420, duration: 320, opacity: 1, easing: appleDrawerEasing }}
    out:fly={{ x: 420, duration: 250, opacity: 1, easing: appleDrawerEasing }}
    class="fixed top-0 right-0 h-full w-full sm:w-[410px] m3-acrylic border-l border-white/10 z-50 shadow-2xl flex flex-col justify-between overflow-hidden select-none will-change-transform"
  >
    <!-- Header -->
    <div class="px-5 py-4 border-b border-white/[0.06] flex items-center justify-between">
      <div>
        <h2 class="font-display font-bold text-base text-white">Playing Next</h2>
        <p class="text-[11px] text-white/45">
          {playerState.queue.length} {playerState.queue.length === 1 ? 'song' : 'songs'} in queue
        </p>
      </div>

      <div class="flex items-center gap-1">
        {#if playerState.queue.length > 1}
          <button
            type="button"
            onclick={shuffleQueue}
            class="w-8 h-8 rounded-full flex items-center justify-center text-white/65 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
            title="Shuffle Queue"
          >
            <span class="material-symbols-rounded text-[18px]">shuffle</span>
          </button>
        {/if}
        {#if playerState.queue.length > 0}
          <button
            type="button"
            onclick={() => playerState.clearQueue()}
            class="px-2.5 py-1 rounded-full text-xs font-medium text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          >
            Clear
          </button>
        {/if}
        <button
          type="button"
          onclick={() => (playerState.isQueueOpen = false)}
          class="w-8 h-8 rounded-full flex items-center justify-center text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
        >
          <span class="material-symbols-rounded text-xl">close</span>
        </button>
      </div>
    </div>

    <!-- Queue Body -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4 scrollbar-none">
      {#if playerState.currentTrack}
        <div class="space-y-2">
          <span class="text-[10px] font-bold uppercase tracking-wider text-white/40 px-1">Now Playing</span>
          <div class="p-2.5 rounded-2xl bg-[var(--md-sys-primary-container)]/35 border border-[var(--md-sys-primary)]/30 flex items-center gap-3">
            <div class="relative w-12 h-12 rounded-xl overflow-hidden shrink-0 bg-white/5">
              {#if playerState.currentTrack.cover_art}
                <img
                  src={getHighResArtwork(playerState.currentTrack.cover_art, playerState.currentTrack.id)}
                  alt={playerState.currentTrack.title}
                  onload={(e) => handleThumbnailLoad(e, playerState.currentTrack?.id)}
                  onerror={(e) => handleThumbnailError(e, playerState.currentTrack?.id)}
                  class="w-full h-full object-cover"
                />
              {:else}
                <div class="w-full h-full flex items-center justify-center">
                  <span class="material-symbols-rounded text-white/40">music_note</span>
                </div>
              {/if}
              <div class="absolute inset-0 bg-black/45 flex items-center justify-center">
                {#if isBuffering}
                  <svg class="w-5 h-5 archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                    <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                    <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
                  </svg>
                {:else}
                  <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-lg filled">
                    {playerState.playback.is_playing ? 'equalizer' : 'pause'}
                  </span>
                {/if}
              </div>
            </div>

            <div class="min-w-0 flex-1 overflow-hidden">
              <div class="text-xs font-semibold text-white truncate">
                {playerState.currentTrack.title}
              </div>
              <ArtistLinks
                artist={playerState.currentTrack.artist}
                class="mt-0.5 text-[11px] text-[var(--md-sys-primary)]"
                buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
              />
            </div>
          </div>
        </div>
      {/if}

      <!-- ArchiveTune Automix Radio Toggle Bar -->
      <div class="flex items-center justify-between px-3.5 py-2.5 rounded-2xl bg-white/[0.04] border border-white/[0.06]">
        <div class="flex items-center gap-2.5">
          <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-lg">radio</span>
          <div>
            <span class="text-xs font-semibold text-white block">Automix similar songs</span>
            <span class="text-[10px] text-white/45 block">Infinite YouTube Music RDAMVM radio queue</span>
          </div>
        </div>
        <button
          type="button"
          onclick={toggleAutomix}
          class="w-10 h-5 rounded-full transition-colors relative cursor-pointer {playerState.autoplayRadio
            ? 'bg-[var(--md-sys-primary)]'
            : 'bg-white/20'}"
          aria-label="Toggle Automix"
        >
          <div
            class="w-3.5 h-3.5 rounded-full bg-black absolute top-[3px] transition-transform {playerState.autoplayRadio
              ? 'left-[22px]'
              : 'left-[3px]'}"
          ></div>
        </button>
      </div>

      <!-- Up Next List -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between px-1">
          <span class="text-[10px] font-bold uppercase tracking-wider text-white/40">
            Up Next {playerState.autoplayRadio ? '• Automix' : ''}
          </span>
          {#if playerState.currentTrack && playerState.queue.length < 5 && !playerState.isLoadingAutomix}
            <button
              type="button"
              onclick={() => playerState.populateAutomixQueue(null, playerState.currentTrack)}
              class="text-[11px] font-semibold text-[var(--md-sys-primary)] hover:underline cursor-pointer"
            >
              + Load Radio Mix
            </button>
          {/if}
        </div>

        {#if playerState.isLoadingAutomix && playerState.queue.length === 0}
          <div class="space-y-2 pt-1">
            {#each Array(6) as _}
              <div class="h-12 rounded-xl bg-white/[0.04] animate-pulse"></div>
            {/each}
          </div>
        {:else if playerState.queue.length > 0}
          {#each playerState.queue as track, idx (track.id + '_' + idx)}
            <div
              role="listitem"
              oncontextmenu={(e) => {
                e.preventDefault();
                playerState.openContextMenu(track, e.clientX, e.clientY);
              }}
              class="h-14 flex items-center justify-between px-2 rounded-xl hover:bg-white/[0.06] transition-colors group overflow-hidden"
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
                      onload={(e) => handleThumbnailLoad(e, track.id)}
                      onerror={(e) => handleThumbnailError(e, track.id)}
                      class="w-full h-full object-cover"
                    />
                  {:else}
                    <div class="w-full h-full flex items-center justify-center">
                      <span class="material-symbols-rounded text-white/40 text-sm">music_note</span>
                    </div>
                  {/if}
                  <div class="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
                    <span class="material-symbols-rounded text-white text-base filled">play_arrow</span>
                  </div>
                </button>

                <div class="min-w-0 flex-1 overflow-hidden">
                  <button
                    type="button"
                    onclick={() => handlePlayNow(idx)}
                    class="text-xs font-semibold text-white hover:text-[var(--md-sys-primary)] truncate block w-full text-left cursor-pointer"
                  >
                    {track.title}
                  </button>
                  <ArtistLinks
                    artist={track.artist}
                    class="mt-0.5 text-[11px] text-white/50"
                    buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                  />
                </div>
              </div>

              <div class="flex items-center gap-1 shrink-0">
                {#if track.duration > 0}
                  <span class="text-[10px] font-mono text-white/35 group-hover:hidden">
                    {formatDuration(track.duration)}
                  </span>
                {/if}
                <div class="hidden group-hover:flex items-center gap-0.5">
                  <button
                    type="button"
                    onclick={() => handleMoveUp(idx)}
                    disabled={idx === 0}
                    class="p-1 rounded text-white/50 hover:text-white disabled:opacity-20 cursor-pointer"
                    title="Move Up"
                  >
                    <span class="material-symbols-rounded text-sm">arrow_upward</span>
                  </button>
                  <button
                    type="button"
                    onclick={() => handleMoveDown(idx)}
                    disabled={idx === playerState.queue.length - 1}
                    class="p-1 rounded text-white/50 hover:text-white disabled:opacity-20 cursor-pointer"
                    title="Move Down"
                  >
                    <span class="material-symbols-rounded text-sm">arrow_downward</span>
                  </button>
                  <button
                    type="button"
                    onclick={() => playerState.removeFromQueue(idx)}
                    class="p-1 rounded text-white/50 hover:text-rose-400 cursor-pointer"
                    title="Remove"
                  >
                    <span class="material-symbols-rounded text-sm">close</span>
                  </button>
                </div>
              </div>
            </div>
          {/each}
        {:else}
          <div class="py-12 text-center space-y-2.5">
            <p class="text-white/40 text-xs">Queue is empty</p>
            {#if playerState.currentTrack}
              <button
                type="button"
                onclick={() => playerState.populateAutomixQueue(null, playerState.currentTrack)}
                class="px-4 py-1.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold cursor-pointer"
              >
                Start Automix Radio
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
