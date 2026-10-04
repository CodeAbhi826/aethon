<script lang="ts">
  import { playerState } from '../state.svelte';

  let name = $state('');
  let description = $state('');
  let isSubmitting = $state(false);

  async function handleCreate() {
    if (!name.trim()) return;
    isSubmitting = true;
    const pl = await playerState.createPlaylist(name, description);
    isSubmitting = false;
    if (pl) {
      name = '';
      description = '';
      playerState.isNewPlaylistModalOpen = false;
    }
  }

  function handleClose() {
    name = '';
    description = '';
    playerState.isNewPlaylistModalOpen = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if playerState.isNewPlaylistModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
    <div
      class="w-full max-w-sm rounded-3xl bg-[#1c1b1f]/95 border border-white/10 p-6 shadow-2xl shadow-black/80 flex flex-col gap-4 animate-in zoom-in-95 duration-200"
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-playlist-title"
    >
      <div class="flex items-center justify-between">
        <h3 id="new-playlist-title" class="text-base font-semibold text-white">New Playlist</h3>
        <button
          type="button"
          class="p-1 rounded-full text-white/50 hover:text-white hover:bg-white/10 transition-colors"
          onclick={handleClose}
          aria-label="Close"
        >
          <span class="material-symbols-rounded text-lg">close</span>
        </button>
      </div>

      <div class="flex flex-col gap-3">
        <div>
          <label for="playlist-name-input" class="text-[11px] font-medium text-white/60 mb-1 block">Playlist Name</label>
          <input
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
          class="px-4 py-2 rounded-xl text-xs font-medium text-white/70 hover:text-white hover:bg-white/10 transition-colors"
          onclick={handleClose}
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={!name.trim() || isSubmitting}
          class="px-5 py-2 rounded-xl text-xs font-medium bg-[var(--md-sys-primary,#d0bcff)] text-black hover:opacity-90 disabled:opacity-40 transition-opacity flex items-center gap-1.5"
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
