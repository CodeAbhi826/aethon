<script lang="ts">
  import { playerState } from '../state.svelte';
  import { onMount } from 'svelte';

  const menu = $derived(playerState.activeContextMenu);
  const track = $derived(menu?.track);

  let menuRef: HTMLDivElement | null = $state(null);
  let adjustedX = $state(0);
  let adjustedY = $state(0);

  $effect(() => {
    if (menu) {
      // Adjust menu position so it doesn't clip off the screen
      const width = 230;
      const height = 340;
      const maxX = window.innerWidth - width - 12;
      const maxY = window.innerHeight - height - 12;

      adjustedX = Math.min(menu.x, maxX);
      adjustedY = Math.min(menu.y, maxY);
    }
  });

  function handleAction(action: () => void) {
    action();
    playerState.closeContextMenu();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      playerState.closeContextMenu();
    }
  }

  function handleBackdrop(e: MouseEvent) {
    playerState.closeContextMenu();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if menu && track}
  <!-- Backdrop click catcher -->
  <button
    type="button"
    class="fixed inset-0 z-50 bg-transparent border-0 cursor-default"
    onclick={handleBackdrop}
    aria-label="Close context menu"
  ></button>

  <div
    bind:this={menuRef}
    class="fixed z-50 w-56 py-1.5 rounded-2xl bg-[#1e1e24]/95 backdrop-blur-2xl border border-white/10 shadow-2xl shadow-black/80 flex flex-col gap-0.5 text-xs text-white/90 animate-in fade-in zoom-in-95 duration-150"
    style="left: {adjustedX}px; top: {adjustedY}px;"
    role="menu"
    tabindex="-1"
  >
    <!-- Header Track Info -->
    <div class="px-3 py-2 border-b border-white/5 flex items-center gap-2.5">
      {#if track.cover_art}
        <img src={track.cover_art} alt={track.title} class="w-8 h-8 rounded-lg object-cover flex-shrink-0" />
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
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
      onclick={() => handleAction(() => playerState.playTrack(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">play_arrow</span>
      <span>Play Now</span>
    </button>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
      onclick={() => handleAction(() => playerState.playNextInQueue(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">playlist_play</span>
      <span>Play Next</span>
    </button>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
      onclick={() => handleAction(() => playerState.addToQueue(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">queue_music</span>
      <span>Add to Queue</span>
    </button>

    <div class="h-px bg-white/5 my-0.5"></div>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
      onclick={() => handleAction(() => playerState.openAddToPlaylist(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">playlist_add</span>
      <span>Add to Playlist…</span>
    </button>

    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
      onclick={() => handleAction(() => playerState.toggleFavorite(track.id))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70 {track.favorite ? 'text-rose-400 fill-1' : ''}">
        {track.favorite ? 'favorite' : 'favorite_border'}
      </span>
      <span>{track.favorite ? 'Remove Favorite' : 'Favorite'}</span>
    </button>

    {#if track.artist}
      <button
        type="button"
        class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
        onclick={() => handleAction(() => playerState.openArtist(track.artist))}
        role="menuitem"
      >
        <span class="material-symbols-rounded text-base text-white/70">person</span>
        <span>Go to Artist</span>
      </button>
    {/if}

    {#if track.album && track.album !== 'Unknown' && track.album !== 'YouTube Music Stream'}
      <button
        type="button"
        class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
        onclick={() => handleAction(() => playerState.openAlbum(track.album, track.artist, track.cover_art))}
        role="menuitem"
      >
        <span class="material-symbols-rounded text-base text-white/70">album</span>
        <span>Go to Album</span>
      </button>
    {/if}

    <div class="h-px bg-white/5 my-0.5"></div>

    <!-- Download Track -->
    <button
      type="button"
      class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90 disabled:opacity-40"
      disabled={playerState.downloadingTrackIds.includes(track.id)}
      onclick={() => handleAction(() => playerState.downloadTrack(track))}
      role="menuitem"
    >
      <span class="material-symbols-rounded text-base text-white/70">
        {playerState.downloadingTrackIds.includes(track.id) ? 'sync' : 'download'}
      </span>
      <span>{playerState.downloadingTrackIds.includes(track.id) ? 'Downloading…' : 'Download Offline'}</span>
    </button>

    {#if !track.file_path.startsWith('http://') && !track.file_path.startsWith('https://')}
      <button
        type="button"
        class="px-3 py-2 flex items-center gap-2.5 hover:bg-white/10 transition-colors text-left text-white/90"
        onclick={() => handleAction(() => playerState.revealInFileManager(track.file_path))}
        role="menuitem"
      >
        <span class="material-symbols-rounded text-base text-white/70">folder_open</span>
        <span>Reveal in File Manager</span>
      </button>
    {/if}
  </div>
{/if}
