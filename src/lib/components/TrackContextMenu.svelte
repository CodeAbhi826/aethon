<script lang="ts">
  import { tick } from 'svelte';
  import { playerState } from '../state.svelte';
  import { getHighResArtwork } from '../artwork';
  import { splitArtists } from '../types';

  const menu = $derived(playerState.activeContextMenu);
  const track = $derived(menu?.track);

  let menuRef: HTMLDivElement | null = $state(null);
  let adjustedX = $state(0);
  let adjustedY = $state(0);

  $effect(() => {
    if (menu) {
      // Initial optimistic clamp, then refine with actual rendered menu dimensions
      adjustedX = Math.max(12, Math.min(menu.x, window.innerWidth - 236));
      adjustedY = Math.max(12, Math.min(menu.y, window.innerHeight - 360));
      tick().then(() => {
        if (!menuRef || !menu) return;
        const rect = menuRef.getBoundingClientRect();
        const maxX = Math.max(12, window.innerWidth - rect.width - 12);
        const maxY = Math.max(12, window.innerHeight - rect.height - 12);
        adjustedX = Math.max(12, Math.min(menu.x, maxX));
        adjustedY = Math.max(12, Math.min(menu.y, maxY));
        menuRef.focus();
      });
    }
  });

  function handleAction(action: () => void) {
    action();
    playerState.closeContextMenu();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!menu) return;
    if (e.key === 'Escape') {
      e.stopPropagation();
      playerState.closeContextMenu();
      return;
    }
    if (menuRef && ['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) {
      const items = Array.from(menuRef.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)'));
      if (items.length === 0) return;
      e.preventDefault();
      const idx = items.indexOf(document.activeElement as HTMLButtonElement);
      if (e.key === 'Home') items[0].focus();
      else if (e.key === 'End') items[items.length - 1].focus();
      else if (e.key === 'ArrowDown') items[(idx + 1) % items.length].focus();
      else if (e.key === 'ArrowUp') items[(idx - 1 + items.length) % items.length].focus();
    }
  }

  function handleBackdrop() {
    playerState.closeContextMenu();
  }

  function isLocalTrackPath(fp?: string | null): boolean {
    if (!fp) return false;
    return fp.startsWith('/') || /^[A-Za-z]:\\/.test(fp);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if menu && track}
  <!-- Backdrop click catcher -->
  <button
    type="button"
    class="fixed inset-0 z-[70] bg-transparent border-0 cursor-default"
    onclick={handleBackdrop}
    oncontextmenu={(e) => { e.preventDefault(); playerState.closeContextMenu(); }}
    aria-label="Close context menu"
  ></button>

  <div
    bind:this={menuRef}
    class="fixed z-[70] w-56 max-h-[calc(100vh-24px)] overflow-y-auto py-1.5 rounded-2xl border border-white/10 shadow-2xl shadow-black/80 flex flex-col gap-0.5 text-xs text-white/90 animate-in focus:outline-none"
    style="left: {adjustedX}px; top: {adjustedY}px; background-color: var(--md-sys-surface-container-high, #1e1e24);"
    role="menu"
    tabindex="-1"
  >
    <!-- Header Track Info -->
    <div class="px-3 py-2 border-b border-white/5 flex items-center gap-2.5">
      {#if track.cover_art}
        <img
          src={getHighResArtwork(track.cover_art, track.id)}
          alt={track.title}
          class="w-8 h-8 rounded-lg object-cover flex-shrink-0"
        />
      {:else}
        <div class="w-8 h-8 rounded-lg bg-white/5 flex items-center justify-center flex-shrink-0">
          <span class="material-symbols-rounded text-base text-white/40">music_note</span>
        </div>
      {/if}
      <div class="min-w-0 flex-1">
        <p class="font-medium text-white truncate text-xs">{track.title}</p>
        <p class="text-[10px] text-white/60 truncate">{track.artist}</p>
      </div>
    </div>

    <!-- Actions -->
    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 cursor-pointer"
      onclick={() => handleAction(() => playerState.playTrack(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">play_arrow</span>
      <span>Play Now</span>
    </button>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 cursor-pointer"
      onclick={() => handleAction(() => playerState.playNextInQueue(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">playlist_play</span>
      <span>Play Next</span>
    </button>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 cursor-pointer"
      onclick={() => handleAction(() => playerState.addToQueue(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">queue_music</span>
      <span>Add to Queue</span>
    </button>

    <div class="h-px bg-white/5 my-0.5"></div>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 cursor-pointer"
      onclick={() => handleAction(() => playerState.openAddToPlaylist(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">playlist_add</span>
      <span>Add to Playlist…</span>
    </button>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 cursor-pointer"
      onclick={() => handleAction(() => playerState.toggleFavorite(track.id, track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base {track.favorite ? 'text-rose-400 filled' : 'text-white/70'}">
        {track.favorite ? 'favorite' : 'favorite_border'}
      </span>
      <span>{track.favorite ? 'Remove Favorite' : 'Favorite'}</span>
    </button>

    {#if track.artist}
      {#each splitArtists(track.artist) as art}
        <button
          type="button"
          class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 min-w-0 cursor-pointer"
          onclick={() => handleAction(() => playerState.openArtist(art))}
          role="menuitem"
        >
          <span class="material-symbols-rounded text-base text-white/70 shrink-0">person</span>
          <span class="truncate">Go to {art}</span>
        </button>
      {/each}
    {/if}

    {#if track.album && track.album !== 'Unknown' && track.album !== 'Lossless Online Stream' && track.album !== 'YouTube Music Stream'}
      <button
        type="button"
        class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 min-w-0 cursor-pointer"
        onclick={() => handleAction(() => playerState.openAlbum(track.album, track.artist, track.cover_art))}
        role="menuitem"
      >
        <span class="material-symbols-rounded text-base text-white/70 shrink-0">album</span>
        <span class="truncate">Go to Album</span>
      </button>
    {/if}

    <div class="h-px bg-white/5 my-0.5"></div>

    <!-- Download Track -->
    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 disabled:opacity-40 cursor-pointer disabled:cursor-not-allowed"
      disabled={playerState.downloadingTrackIds.includes(track.id)}
      onclick={() => handleAction(() => playerState.downloadTrack(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">
        {playerState.downloadingTrackIds.includes(track.id) ? 'sync' : 'download'}
      </span>
      <span>{playerState.downloadingTrackIds.includes(track.id) ? 'Downloading…' : 'Download Offline'}</span>
    </button>

    {#if isLocalTrackPath(track.file_path)}
      <button
        type="button"
        class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 cursor-pointer"
        onclick={() => handleAction(() => playerState.revealInFileManager(track.file_path))}
        role="menuitem"
      >
        <span class="material-symbols-rounded text-base text-white/70">folder_open</span>
        <span>Reveal in File Manager</span>
      </button>
    {/if}
  </div>
{/if}
