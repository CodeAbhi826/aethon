<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { Playlist } from '../types';

  const track = $derived(playerState.targetTrackForPlaylist);
  const playlists = $derived(playerState.playlists);

  let addedPlaylistId = $state<string | null>(null);

  async function handleSelectPlaylist(pl: Playlist) {
    if (!track) return;
    addedPlaylistId = pl.id;
    await playerState.addToPlaylist(pl.id, track);
    setTimeout(() => {
      addedPlaylistId = null;
      playerState.isAddToPlaylistModalOpen = false;
      playerState.targetTrackForPlaylist = null;
    }, 400);
  }

  function handleCreateNew() {
    playerState.isAddToPlaylistModalOpen = false;
    playerState.isNewPlaylistModalOpen = true;
  }

  function handleClose() {
    playerState.isAddToPlaylistModalOpen = false;
    playerState.targetTrackForPlaylist = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if playerState.isAddToPlaylistModalOpen && track}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
    <div
      class="w-full max-w-sm rounded-3xl bg-[#1c1b1f]/95 border border-white/10 p-5 shadow-2xl shadow-black/80 flex flex-col gap-4 animate-in zoom-in-95 duration-200"
      role="dialog"
      aria-modal="true"
      aria-labelledby="add-to-playlist-title"
    >
      <div class="flex items-center justify-between">
        <div>
          <h3 id="add-to-playlist-title" class="text-base font-semibold text-white">Add to Playlist</h3>
          <p class="text-xs text-white/50 truncate max-w-[240px]">{track.title} • {track.artist}</p>
        </div>
        <button
          type="button"
          class="p-1 rounded-full text-white/50 hover:text-white hover:bg-white/10 transition-colors"
          onclick={handleClose}
          aria-label="Close"
        >
          <span class="material-symbols-rounded text-lg">close</span>
        </button>
      </div>

      <!-- Playlists List -->
      <div class="max-h-64 overflow-y-auto flex flex-col gap-1 pr-1 custom-scrollbar">
        <button
          type="button"
          class="w-full px-3 py-2.5 rounded-xl hover:bg-white/10 transition-colors flex items-center gap-3 text-left group border border-dashed border-white/15"
          onclick={handleCreateNew}
        >
          <div class="w-9 h-9 rounded-lg bg-[var(--md-sys-primary,#d0bcff)]/20 text-[var(--md-sys-primary,#d0bcff)] flex items-center justify-center">
            <span class="material-symbols-rounded text-lg">add</span>
          </div>
          <div>
            <p class="text-xs font-medium text-white">New Playlist</p>
            <p class="text-[10px] text-white/40">Create a new collection</p>
          </div>
        </button>

        {#if playlists.length === 0}
          <div class="py-6 text-center text-xs text-white/40">
            No existing playlists. Create one above!
          </div>
        {:else}
          {#each playlists as pl (pl.id)}
            <button
              type="button"
              class="w-full px-3 py-2 rounded-xl hover:bg-white/10 transition-colors flex items-center justify-between text-left group"
              onclick={() => handleSelectPlaylist(pl)}
            >
              <div class="flex items-center gap-3 min-w-0">
                <div class="w-9 h-9 rounded-lg bg-white/5 flex items-center justify-center flex-shrink-0 text-white/60">
                  <span class="material-symbols-rounded text-lg">queue_music</span>
                </div>
                <div class="min-w-0">
                  <p class="text-xs font-medium text-white truncate">{pl.name}</p>
                  <p class="text-[10px] text-white/40">{pl.track_count} {pl.track_count === 1 ? 'song' : 'songs'}</p>
                </div>
              </div>

              {#if addedPlaylistId === pl.id}
                <span class="material-symbols-rounded text-emerald-400 text-lg animate-in zoom-in">check_circle</span>
              {:else}
                <span class="material-symbols-rounded text-white/20 group-hover:text-white/60 text-lg transition-colors">add</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}
