<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import {
    sanitizeArtist,
    handleThumbnailError,
    handleThumbnailLoad,
    type Track,
    type YouTubeTrack,
  } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from '../components/ArtistLinks.svelte';

  let albumTracks = $state<Track[]>([]);
  let isLoading = $state(false);
  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));
  let fetchSeq = 0;

  let album = $derived(playerState.selectedAlbum || { title: 'Album', artist: 'Artist' });

  // Check local library for tracks matching this album
  let localAlbumTracks = $derived(
    playerState.library.filter(
      (t) => (t.album || '').toLowerCase() === album.title.toLowerCase()
    )
  );

  let lastFetchedAlbum = '';
  $effect(() => {
    if (!playerState.selectedAlbum) {
      isLoading = false;
      return;
    }
    const cur = `${album.title}::${album.artist}`;
    if (cur !== lastFetchedAlbum) {
      lastFetchedAlbum = cur;
      fetchAlbumTracks(album.title, album.artist, album.cover);
    }
  });

  async function fetchAlbumTracks(title: string, artist: string, cover?: string) {
    const seq = ++fetchSeq;
    albumTracks = [];
    if (localAlbumTracks.length > 0 && !cover?.startsWith('http')) {
      albumTracks = localAlbumTracks;
      isLoading = false;
      return;
    }
    isLoading = true;
    try {
      const res = await invoke<YouTubeTrack[]>('search_youtube', {
        query: `${title} ${artist} full album tracks`,
        limit: 12,
      });

      if (seq !== fetchSeq) return;
      if (res && res.length > 0) {
        albumTracks = res.map((yt) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: sanitizeArtist(yt.artist) || sanitizeArtist(artist) || artist || 'YouTube Music',
          album: title,
          duration: yt.duration || 0,
          file_path: yt.stream_url,
          cover_art: getHighResArtwork(yt.thumbnail || cover, yt.id),
          format: 'Opus 256k',
          play_count: 0,
          favorite: false,
          date_added: Math.floor(Date.now() / 1000),
        }));
      } else {
        albumTracks = localAlbumTracks;
      }
    } catch {
      if (seq === fetchSeq) {
        albumTracks = localAlbumTracks;
      }
    } finally {
      if (seq === fetchSeq) {
        isLoading = false;
      }
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

  function formatDuration(sec: number): string {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }
</script>

<div class="px-6 py-5 space-y-8 max-w-7xl mx-auto pb-28 select-none">
  
  <!-- Back Button -->
  <button
    type="button"
    onclick={() => (playerState.activeTab = 'home')}
    class="flex items-center gap-1.5 text-xs font-medium text-white/60 hover:text-white cursor-pointer transition-colors"
  >
    <span class="material-symbols-rounded text-base">arrow_back</span>
    <span>Back</span>
  </button>

  <!-- Album Hero Banner -->
  <div class="p-6 md:p-8 rounded-3xl m3-acrylic-card border border-white/[0.08] flex flex-col md:flex-row items-center gap-7 shadow-2xl relative overflow-hidden">
    
    <!-- GPU Ambient Radial Accent (Zero CPU blur) -->
    <div
      class="absolute -top-24 -left-24 w-80 h-80 rounded-full pointer-events-none"
      style="background: radial-gradient(circle, color-mix(in srgb, var(--md-sys-primary) 18%, transparent) 0%, transparent 70%);"
    ></div>

    <div class="w-40 h-40 md:w-52 md:h-52 rounded-2xl overflow-hidden shadow-2xl border border-white/15 shrink-0 relative bg-black/40">
      {#if album.cover || albumTracks[0]?.cover_art}
        {@const heroVid = albumTracks[0]?.id}
        <img
          src={getHighResArtwork(album.cover || albumTracks[0]?.cover_art, heroVid)}
          alt={album.title}
          onload={(e) => handleThumbnailLoad(e, heroVid)}
          onerror={(e) => handleThumbnailError(e, heroVid)}
          class="w-full h-full object-cover"
        />
      {:else}
        <div class="w-full h-full flex items-center justify-center">
          <span class="material-symbols-rounded text-6xl text-white/40">album</span>
        </div>
      {/if}
    </div>

    <div class="w-full md:w-auto flex-1 text-center md:text-left space-y-2.5 min-w-0">
      <div class="text-xs uppercase tracking-wider font-bold text-[var(--md-sys-primary)] font-display">
        Album Release
      </div>
      <h1 class="text-2xl md:text-4xl font-extrabold text-white tracking-tight font-display truncate">
        {album.title}
      </h1>
      <div class="text-sm text-[var(--md-sys-on-surface-variant)] flex items-center justify-center md:justify-start gap-2 min-w-0">
        <div class="min-w-0 truncate">
          <ArtistLinks
            artist={album.artist}
            class="text-sm"
            buttonClass="font-semibold text-white hover:text-[var(--md-sys-primary)] hover:underline transition-colors cursor-pointer"
          />
        </div>
        <span class="shrink-0">•</span>
        <span class="shrink-0 whitespace-nowrap">
          {#if isLoading}
            Loading tracks…
          {:else}
            {albumTracks.length} {albumTracks.length === 1 ? 'track' : 'tracks'}
          {/if}
        </span>
      </div>

      <div class="flex items-center justify-center md:justify-start gap-3 pt-3">
        <button
          type="button"
          disabled={isLoading || albumTracks.length === 0}
          onclick={handlePlayAlbum}
          class="flex items-center gap-2 px-5 py-2.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-bold hover:opacity-95 active:scale-95 disabled:opacity-40 disabled:cursor-not-allowed shadow-lg cursor-pointer transition-[transform,opacity]"
        >
          <span class="material-symbols-rounded text-lg filled">play_arrow</span>
          <span>Play Album</span>
        </button>

        <button
          type="button"
          disabled={isLoading || albumTracks.length === 0}
          onclick={handleAddAllToQueue}
          class="flex items-center gap-2 px-4 py-2.5 rounded-full bg-white/10 hover:bg-white/15 disabled:opacity-40 disabled:cursor-not-allowed text-white text-xs font-semibold border border-white/10 cursor-pointer transition-colors"
        >
          <span class="material-symbols-rounded text-lg">queue_music</span>
          <span>Add to Queue</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Tracklist Section -->
  <section class="space-y-3">
    <div class="flex items-center justify-between px-3.5 text-xs font-bold uppercase tracking-wider text-[var(--md-sys-outline)] font-display pb-2 border-b border-white/5">
      <div class="flex items-center gap-4">
        <span class="w-5 text-center">#</span>
        <span>Title</span>
      </div>
      <span>Duration</span>
    </div>

    {#if isLoading}
      <div class="space-y-2">
        {#each Array(6) as _}
          <div class="h-14 rounded-2xl bg-white/[0.04] animate-pulse"></div>
        {/each}
      </div>
    {:else if albumTracks.length === 0}
      <div class="py-16 text-center rounded-2xl bg-white/[0.02] border border-white/[0.05] space-y-2">
        <span class="material-symbols-rounded text-4xl text-white/25">album</span>
        <p class="text-sm font-medium text-white/60">No tracks found for this album</p>
        <p class="text-xs text-white/35">Try searching directly in Explore & Search</p>
      </div>
    {:else}
      <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
        {#each albumTracks as track, idx (track.id)}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                e.preventDefault();
                playerState.playTrack(track);
              }
            }}
            class="h-14 flex items-center justify-between px-3.5 transition-colors cursor-pointer group overflow-hidden {isCurrent ? '' : 'hover:bg-white/[0.05]'}"
            style={isCurrent ? 'background-color: color-mix(in srgb, var(--md-sys-primary-container) 35%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-primary) 35%, transparent);' : ''}
          >
            <div class="flex items-center gap-4 min-w-0 flex-1 pr-4">
              <span class="w-5 text-center text-xs font-mono font-medium {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white/40'} shrink-0 flex items-center justify-center">
                {#if isCurrent && isBuffering}
                  <svg class="w-4 h-4 archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                    <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                    <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
                  </svg>
                {:else if isCurrent && playerState.playback.is_playing}
                  <div class="flex items-end gap-[2px] h-3.5">
                    <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                    <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                    <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                  </div>
                {:else}
                  {idx + 1}
                {/if}
              </span>

              <div class="min-w-0 flex-1 overflow-hidden">
                <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition-colors">
                  {track.title}
                </div>
                <ArtistLinks
                  artist={track.artist}
                  class="mt-0.5 text-[11px] text-[var(--md-sys-on-surface-variant)] opacity-75"
                  buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                />
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              {#if track.duration > 0}
                <span class="font-mono text-[11px] text-white/45 group-hover:hidden group-focus-within:hidden">
                  {formatDuration(track.duration)}
                </span>
              {/if}

              <div class="hidden group-hover:flex group-focus-within:flex items-center gap-1">
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.addToQueue(track);
                  }}
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/50 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
                  aria-label="Add to Up Next Queue"
                  title="Add to Up Next Queue"
                >
                  <span class="material-symbols-rounded text-base">queue_music</span>
                </button>
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.openContextMenu(track, e.clientX, e.clientY);
                  }}
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/50 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
                  aria-label="More options"
                  title="More options"
                >
                  <span class="material-symbols-rounded text-base">more_vert</span>
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

</div>
