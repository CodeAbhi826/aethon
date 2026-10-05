<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import type { Track, YouTubeTrack } from '../types';

  let albumTracks = $state<Track[]>([]);
  let isLoading = $state(true);

  let album = $derived(playerState.selectedAlbum || { title: 'Album', artist: 'Artist' });

  // Check local library for tracks matching this album
  let localAlbumTracks = $derived(
    playerState.library.filter(
      (t) => t.album.toLowerCase() === album.title.toLowerCase()
    )
  );

  onMount(async () => {
    await fetchAlbumTracks();
  });

  $effect(() => {
    if (playerState.selectedAlbum) {
      fetchAlbumTracks();
    }
  });

  async function fetchAlbumTracks() {
    isLoading = true;
    try {
      const res = await invoke<YouTubeTrack[]>('search_youtube', {
        query: `${album.title} ${album.artist} full album tracks`,
        limit: 10,
      });

      if (res && res.length > 0) {
        albumTracks = res.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: yt.artist || album.artist,
          album: album.title,
          duration: yt.duration || 210,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail || album.cover,
          format: 'Opus 256k',
          play_count: 50 + idx,
          favorite: false,
          date_added: Date.now(),
        }));
      } else {
        albumTracks = localAlbumTracks;
      }
    } catch {
      albumTracks = localAlbumTracks;
    } finally {
      isLoading = false;
    }
  }

  function handlePlayAlbum() {
    const list = albumTracks.length > 0 ? albumTracks : localAlbumTracks;
    if (list.length > 0) {
      playerState.queue = list.slice(1);
      playerState.playTrack(list[0]);
    }
  }

  function handleAddAllToQueue() {
    const list = albumTracks.length > 0 ? albumTracks : localAlbumTracks;
    for (const track of list) {
      playerState.addToQueue(track);
    }
  }
</script>

<div class="px-6 py-6 space-y-8 max-w-7xl mx-auto pb-28 select-none">
  
  <!-- Back Button -->
  <button
    onclick={() => playerState.activeTab = 'library'}
    class="flex items-center gap-2 text-xs font-semibold text-[var(--md-sys-primary)] hover:underline cursor-pointer"
  >
    <span class="material-symbols-rounded text-base">arrow_back</span>
    <span>Back to Library</span>
  </button>

  <!-- Album Hero Banner -->
  <div class="p-6 md:p-8 rounded-3xl bg-gradient-to-br from-white/[0.06] to-white/[0.01] border border-white/10 flex flex-col md:flex-row items-center gap-7 shadow-2xl relative overflow-hidden">
    
    <!-- Ambient Backdrop Glow -->
    <div class="absolute -top-24 -left-24 w-80 h-80 rounded-full bg-[var(--md-sys-primary)]/15 blur-3xl pointer-events-none"></div>

    <div class="w-40 h-40 md:w-52 md:h-52 rounded-2xl overflow-hidden shadow-2xl border border-white/20 shrink-0 relative bg-black/40">
      {#if album.cover || albumTracks[0]?.cover_art}
        <img src={album.cover || albumTracks[0]?.cover_art} alt={album.title} class="w-full h-full object-cover" />
      {:else}
        <div class="w-full h-full flex items-center justify-center">
          <span class="material-symbols-rounded text-6xl text-white/40">album</span>
        </div>
      {/if}
    </div>

    <div class="flex-1 text-center md:text-left space-y-2.5 min-w-0">
      <div class="text-xs uppercase tracking-wider font-bold text-[var(--md-sys-primary)] font-display">
        Album Release
      </div>
      <h1 class="text-2xl md:text-4xl font-extrabold text-white tracking-tight font-display truncate">
        {album.title}
      </h1>
      <p class="text-sm text-[var(--md-sys-on-surface-variant)] flex items-center justify-center md:justify-start gap-2">
        <button
          onclick={() => playerState.openArtist(album.artist)}
          class="font-semibold text-white hover:text-[var(--md-sys-primary)] hover:underline transition"
        >
          {album.artist}
        </button>
        <span>•</span>
        <span>{albumTracks.length} tracks</span>
      </p>

      <div class="flex items-center justify-center md:justify-start gap-3 pt-3">
        <button
          onclick={handlePlayAlbum}
          class="flex items-center gap-2 px-5 py-2.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-bold hover:brightness-110 shadow-lg cursor-pointer transition"
        >
          <span class="material-symbols-rounded text-lg filled">play_arrow</span>
          <span>Play Album</span>
        </button>

        <button
          onclick={handleAddAllToQueue}
          class="flex items-center gap-2 px-4 py-2.5 rounded-full bg-white/10 hover:bg-white/15 text-white text-xs font-semibold border border-white/10 cursor-pointer transition"
        >
          <span class="material-symbols-rounded text-lg">queue_music</span>
          <span>Add to Queue</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Tracklist Section -->
  <section class="space-y-4">
    <div class="flex items-center justify-between text-xs font-bold uppercase tracking-wider text-[var(--md-sys-outline)] font-display pb-2 border-b border-white/5">
      <div class="flex items-center gap-6">
        <span class="w-6 text-center">#</span>
        <span>Title</span>
      </div>
      <span>Duration</span>
    </div>

    {#if isLoading}
      <div class="flex items-center justify-center py-12 text-[#9ea2b0]">
        <svg class="w-6 h-6 animate-spin [animation-timing-function:steps(8)] text-[var(--md-sys-primary)]" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <line x1="12" y1="2" x2="12" y2="6" stroke-width="2.5" stroke-linecap="round" class="opacity-100" />
          <line x1="19.07" y1="4.93" x2="16.24" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-90" />
          <line x1="22" y1="12" x2="18" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-75" />
          <line x1="19.07" y1="19.07" x2="16.24" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-60" />
          <line x1="12" y1="22" x2="12" y2="18" stroke-width="2.5" stroke-linecap="round" class="opacity-45" />
          <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-30" />
          <line x1="2" y1="12" x2="6" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-20" />
          <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-15" />
        </svg>
      </div>
    {:else}
      <div class="space-y-1.5">
        {#each albumTracks as track, idx}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            onkeydown={(e) => { if (e.key === 'Enter') playerState.playTrack(track); }}
            class="flex items-center justify-between p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/50 shadow-sm' : 'bg-white/[0.02] hover:bg-white/[0.06] border-transparent hover:border-white/5'} border transition-all cursor-pointer group"
          >
            <div class="flex items-center gap-6 min-w-0 flex-1 pr-4">
              <span class="w-6 text-center text-xs font-mono font-medium {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white/40'} shrink-0">
                {#if isCurrent && playerState.playback.is_playing}
                  <span class="material-symbols-rounded text-base text-[var(--md-sys-primary)] filled">equalizer</span>
                {:else}
                  {idx + 1}
                {/if}
              </span>

              <div class="min-w-0 flex-1">
                <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                  {track.title}
                </div>
                <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5">
                  {track.artist}
                </div>
              </div>
            </div>

            <div class="flex items-center gap-3 shrink-0">
              <span class="font-mono text-xs text-white/50">
                {Math.floor(track.duration / 60)}:{Math.floor(track.duration % 60) < 10 ? '0' : ''}{Math.floor(track.duration % 60)}
              </span>

              <button
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.addToQueue(track);
                }}
                class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 transition opacity-0 group-hover:opacity-100"
                title="Add to Up Next Queue"
              >
                <span class="material-symbols-rounded text-base">queue_music</span>
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

</div>
