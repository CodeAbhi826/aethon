<script lang="ts">
  import { tick } from 'svelte';
  import { playerState } from '../state.svelte';

  let name = $state('');
  let description = $state('');
  let isSubmitting = $state(false);
  let errorMsg = $state<string | null>(null);
  let nameInputEl: HTMLInputElement | null = $state(null);

  $effect(() => {
    if (playerState.isNewPlaylistModalOpen) {
      tick().then(() => nameInputEl?.focus());
    }
  });

  async function handleCreate() {
    if (!name.trim() || isSubmitting) return;
    isSubmitting = true;
    errorMsg = null;
    try {
      const pl = await playerState.createPlaylist(name, description);
      if (pl) {
        if (playerState.targetTrackForPlaylist) {
          await playerState.addToPlaylist(pl.id, playerState.targetTrackForPlaylist);
          playerState.targetTrackForPlaylist = null;
        }
        name = '';
        description = '';
        playerState.isNewPlaylistModalOpen = false;
      } else {
        errorMsg = 'Failed to create playlist.';
      }
    } finally {
      isSubmitting = false;
    }
  }

  function handleClose() {
    name = '';
    description = '';
    errorMsg = null;
    playerState.targetTrackForPlaylist = null;
    playerState.isNewPlaylistModalOpen = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!playerState.isNewPlaylistModalOpen) return;
    if (e.key === 'Escape') {
      e.stopPropagation();
      handleClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if playerState.isNewPlaylistModalOpen}
  <div
    class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-black/75 animate-in"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) handleClose(); }}
  >
    <div
      class="w-full max-w-sm rounded-3xl border border-white/10 p-6 shadow-2xl shadow-black/80 flex flex-col gap-4 animate-in"
      style="background-color: var(--md-sys-surface-container, #1c1b1f);"
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-playlist-title"
    >
      <div class="flex items-center justify-between">
        <h3 id="new-playlist-title" class="text-base font-semibold text-white">New Playlist</h3>
        <button
          type="button"
          class="p-1 rounded-full text-white/50 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          onclick={handleClose}
          aria-label="Close"
        >
          <span class="material-symbols-rounded text-lg">close</span>
        </button>
      </div>

      {#if errorMsg}
        <p class="text-xs text-rose-400 bg-rose-500/10 border border-rose-500/20 rounded-xl px-3 py-2" role="alert">
          {errorMsg}
        </p>
      {/if}

      <div class="flex flex-col gap-3">
        <div>
          <label for="playlist-name-input" class="text-[11px] font-medium text-white/60 mb-1 block">Playlist Name</label>
          <input
            bind:this={nameInputEl}
            id="playlist-name-input"
            type="text"
            bind:value={name}
            placeholder="My Playlist"
            class="w-full px-3.5 py-2 rounded-xl bg-white/5 border border-white/10 text-white text-sm focus:outline-none focus:border-[var(--md-sys-primary,#d0bcff)] transition-colors"
            onkeydown={(e) => { if (e.key === 'Enter') handleCreate(); }}
          />
        </div>

        <div>
          <label for="playlist-desc-input" class="text-[11px] font-medium text-white/60 mb-1 block">Description (Optional)</label>
          <input
            id="playlist-desc-input"
            type="text"
            bind:value={description}
            placeholder="Give your playlist a mood or description"
            class="w-full px-3.5 py-2 rounded-xl bg-white/5 border border-white/10 text-white text-sm focus:outline-none focus:border-[var(--md-sys-primary,#d0bcff)] transition-colors"
            onkeydown={(e) => { if (e.key === 'Enter') handleCreate(); }}
          />
        </div>
      </div>

      <div class="flex items-center justify-end gap-2.5 mt-2">
        <button
          type="button"
          class="px-4 py-2 rounded-xl text-xs font-medium text-white/70 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          onclick={handleClose}
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={!name.trim() || isSubmitting}
          class="px-5 py-2 rounded-xl text-xs font-medium bg-[var(--md-sys-primary,#d0bcff)] text-[var(--md-sys-on-primary,#141218)] hover:opacity-90 disabled:opacity-40 disabled:cursor-not-allowed transition-opacity flex items-center gap-1.5 cursor-pointer"
          onclick={handleCreate}
        >
          {#if isSubmitting}
            <span class="material-symbols-rounded text-xs animate-spin">sync</span>
          {/if}
          <span>Create</span>
        </button>
      </div>
    </div>
  </div>
{/if}
