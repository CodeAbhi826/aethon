<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import {
    sanitizeArtist,
    splitArtists,
    handleThumbnailError,
    handleThumbnailLoad,
    type Track,
    type YouTubeTrack,
  } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from '../components/ArtistLinks.svelte';

  let artistTracks = $state<Track[]>([]);
  let albumTracks = $state<Track[]>([]);
  let liveVideoTracks = $state<Track[]>([]);
  let isLoadingTracks = $state(true);
  let showAllSongs = $state(false);
  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  let artistName = $derived(playerState.selectedArtist || 'Artist');

  let localArtistTracks = $derived(
    playerState.library.filter((t) =>
      t.artist.toLowerCase().includes(artistName.toLowerCase())
    )
  );

  // Derive collaborating / similar artists from the artist's catalog
  let similarArtists = $derived.by(() => {
    const seen = new Set<string>([artistName.toLowerCase()]);
    const list: { name: string; avatar: string; id: string }[] = [];
    for (const t of [...artistTracks, ...albumTracks]) {
      const parts = splitArtists(t.artist);
      for (const p of parts) {
        const lower = p.toLowerCase();
        if (p && !seen.has(lower) && lower !== 'unknown artist') {
          seen.add(lower);
          list.push({ name: p, avatar: t.cover_art || '', id: t.id });
        }
        if (list.length >= 6) break;
      }
      if (list.length >= 6) break;
    }
    return list;
  });

  let lastFetchedArtist = '';
  $effect(() => {
    if (playerState.selectedArtist && playerState.selectedArtist !== lastFetchedArtist) {
      lastFetchedArtist = playerState.selectedArtist;
      showAllSongs = false;
      fetchArtistData();
    }
  });

  function ytToTrack(yt: YouTubeTrack, albumLabel: string): Track {
    return {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: sanitizeArtist(yt.artist) || artistName,
      album: albumLabel,
      duration: yt.duration || 0,
      file_path: yt.stream_url,
      cover_art: getHighResArtwork(yt.thumbnail, yt.id),
      format: 'Opus 256k',
      play_count: 0,
      favorite: false,
      date_added: Math.floor(Date.now() / 1000),
    };
  }

  async function fetchArtistData() {
    isLoadingTracks = true;
    try {
      const [songsRes, albumsRes, videosRes] = await Promise.all([
        invoke<YouTubeTrack[]>('search_youtube', {
          query: `${artistName} popular songs`,
          limit: 45,
        }).catch(() => []),
        invoke<YouTubeTrack[]>('search_youtube', {
          query: `${artistName} album singles`,
          limit: 16,
        }).catch(() => []),
        invoke<YouTubeTrack[]>('search_youtube', {
          query: `${artistName} official music video live`,
          limit: 12,
        }).catch(() => []),
      ]);

      if (songsRes && songsRes.length > 0) {
        artistTracks = songsRes.map((yt) => ytToTrack(yt, artistName));
      } else {
        artistTracks = localArtistTracks;
      }

      const songIds = new Set(artistTracks.map((t) => t.id));
      if (albumsRes && albumsRes.length > 0) {
        albumTracks = albumsRes
          .filter((yt) => !songIds.has(`yt_${yt.id}`))
          .map((yt) => ytToTrack(yt, `${artistName} • Releases`));
      } else {
        albumTracks = artistTracks.slice(6, 16);
      }

      if (videosRes && videosRes.length > 0) {
        liveVideoTracks = videosRes
          .map((yt) => ytToTrack(yt, `${artistName} • Videos`))
          .slice(0, 10);
      }
    } catch {
      artistTracks = localArtistTracks;
    } finally {
      isLoadingTracks = false;
    }
  }

  function handleShuffleAll() {
    const list = artistTracks.length > 0 ? artistTracks : localArtistTracks;
    if (list.length > 0) {
      const shuffled = [...list].sort(() => Math.random() - 0.5);
      playerState.queue = shuffled.slice(1);
      playerState.playTrack(shuffled[0]);
    }
  }

  async function handleStartRadio() {
    const list = artistTracks.length > 0 ? artistTracks : localArtistTracks;
    if (list.length > 0) {
      const seed = list[0];
      playerState.queue = list.slice(1, 10);
      await playerState.playTrack(seed);
      const vid =
        playerState.extractVideoId(seed.file_path) || playerState.extractVideoId(seed.id);
      if (vid) {
        playerState.populateAutomixQueue(vid, seed);
      }
    }
  }

  function formatDuration(sec: number): string {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }
</script>

<div class="px-6 py-5 space-y-7 max-w-7xl mx-auto pb-28 select-none">
  <button
    type="button"
    onclick={() => (playerState.activeTab = 'home')}
    class="flex items-center gap-1.5 text-xs font-medium text-white/60 hover:text-white cursor-pointer transition-colors"
  >
    <span class="material-symbols-rounded text-base">arrow_back</span>
    <span>Back</span>
  </button>

  <!-- ArchiveTune Artist Hero Header -->
  <div class="p-6 sm:p-8 rounded-3xl m3-acrylic-card border border-white/[0.08] flex flex-col sm:flex-row items-center gap-6 relative overflow-hidden">
    <div class="w-28 h-28 sm:w-36 sm:h-36 rounded-full overflow-hidden shadow-2xl ring-2 ring-white/15 shrink-0 bg-white/5">
      {#if artistTracks[0]?.cover_art}
        <img
          src={getHighResArtwork(artistTracks[0].cover_art, artistTracks[0].id)}
          alt={artistName}
          onload={(e) => handleThumbnailLoad(e, artistTracks[0]?.id)}
          onerror={(e) => handleThumbnailError(e, artistTracks[0]?.id)}
          class="w-full h-full object-cover"
        />
      {:else}
        <div class="w-full h-full flex items-center justify-center">
          <span class="material-symbols-rounded text-4xl text-white/30">person</span>
        </div>
      {/if}
    </div>

    <div class="flex-1 text-center sm:text-left space-y-3 min-w-0">
      <div>
        <span class="text-[10px] font-bold uppercase tracking-wider text-[var(--md-sys-primary)]">Artist</span>
        <h1 class="text-2xl sm:text-4xl font-bold text-white tracking-tight font-display truncate mt-0.5">
          {artistName}
        </h1>
        <p class="text-xs text-white/50 mt-1">
          {artistTracks.length} songs available in catalog
        </p>
      </div>

      <!-- ArchiveTune Shuffle & Radio Action Pills -->
      <div class="flex items-center justify-center sm:justify-start gap-2.5 pt-1">
        <button
          type="button"
          onclick={handleShuffleAll}
          class="flex items-center gap-1.5 px-5 py-2.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold hover:scale-105 cursor-pointer transition-transform shadow-md"
        >
          <span class="material-symbols-rounded text-base">shuffle</span>
          <span>Shuffle</span>
        </button>

        <button
          type="button"
          onclick={handleStartRadio}
          class="flex items-center gap-1.5 px-5 py-2.5 rounded-full bg-white/[0.08] hover:bg-white/[0.15] border border-white/10 text-white text-xs font-semibold cursor-pointer transition-colors"
        >
          <span class="material-symbols-rounded text-base">radio</span>
          <span>Radio</span>
        </button>
      </div>
    </div>
  </div>

  <!-- 1. ArchiveTune "Songs" Section with "Show all" Expansion (No 10-Song Cutoff!) -->
  <section class="space-y-3">
    <div class="flex items-center justify-between px-1">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white">Songs</h2>
      {#if artistTracks.length > 6}
        <button
          type="button"
          onclick={() => (showAllSongs = !showAllSongs)}
          class="px-3.5 py-1 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-xs font-semibold text-[var(--md-sys-primary)] cursor-pointer transition-colors"
        >
          {showAllSongs ? 'Show less' : `Show all (${artistTracks.length})`}
        </button>
      {/if}
    </div>

    {#if isLoadingTracks}
      <div class="space-y-2">
        {#each Array(6) as _}
          <div class="h-14 rounded-2xl bg-white/[0.04] animate-pulse"></div>
        {/each}
      </div>
    {:else}
      <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
        {#each showAllSongs ? artistTracks : artistTracks.slice(0, 6) as track, idx (track.id)}
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
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="h-14 flex items-center justify-between px-3.5 {isCurrent
              ? 'bg-[var(--md-sys-primary-container)]/35'
              : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group overflow-hidden"
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-3">
              <span class="w-5 text-center text-xs font-mono text-white/35 shrink-0">
                {idx + 1}
              </span>
              <div class="relative w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
                <img
                  loading="lazy"
                  decoding="async"
                  src={getHighResArtwork(track.cover_art, track.id)}
                  alt={track.title}
                  onload={(e) => handleThumbnailLoad(e, track.id)}
                  onerror={(e) => handleThumbnailError(e, track.id)}
                  class="w-full h-full object-cover"
                />
                <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                  {#if isCurrent && isBuffering}
                    <svg class="w-4 h-4 archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                      <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                      <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
                    </svg>
                  {:else}
                    <span class="material-symbols-rounded text-white text-lg filled">
                      {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                    </span>
                  {/if}
                </div>
              </div>

              <div class="min-w-0 flex-1 overflow-hidden">
                <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate block w-full">
                  {track.title}
                </div>
                <ArtistLinks
                  artist={track.artist}
                  class="mt-0.5 text-[11px] text-white/50"
                  buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                />
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              {#if track.duration > 0}
                <span class="font-mono text-[11px] text-white/40">
                  {formatDuration(track.duration)}
                </span>
              {/if}
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 transition opacity-0 group-hover:opacity-100 cursor-pointer"
                title="More options"
              >
                <span class="material-symbols-rounded text-base">more_vert</span>
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- 2. ArchiveTune "Albums & Singles" Horizontal Shelf -->
  {#if albumTracks.length > 0}
    <section class="space-y-3">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white px-1">Albums & Singles</h2>
      <div class="flex items-start gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
        {#each albumTracks as track (track.id)}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="w-36 sm:w-40 shrink-0 group cursor-pointer space-y-2"
          >
            <div class="relative aspect-square rounded-2xl overflow-hidden bg-white/5 shadow-md ring-1 ring-white/[0.07]">
              {#if track.cover_art}
                <img
                  loading="lazy"
                  decoding="async"
                  src={getHighResArtwork(track.cover_art, track.id)}
                  alt={track.title}
                  onload={(e) => handleThumbnailLoad(e, track.id)}
                  onerror={(e) => handleThumbnailError(e, track.id)}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                />
              {/if}
              <div class="absolute inset-0 bg-black/30 opacity-0 group-hover:opacity-100 flex items-end justify-end p-2.5 transition-opacity">
                <div class="w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                  <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                </div>
              </div>
            </div>
            <div class="px-0.5 overflow-hidden">
              <p class="text-xs font-semibold text-white truncate block w-full">{track.title}</p>
              <ArtistLinks
                artist={track.artist}
                class="text-[11px] text-white/50 mt-0.5"
                buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
              />
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- 3. ArchiveTune "Videos & Live Performances" Shelf -->
  {#if liveVideoTracks.length > 0}
    <section class="space-y-3">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white px-1">Videos & Live</h2>
      <div class="flex items-start gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
        {#each liveVideoTracks as track (track.id)}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="w-44 sm:w-52 shrink-0 group cursor-pointer space-y-2"
          >
            <div class="relative aspect-video rounded-2xl overflow-hidden bg-white/5 shadow-md ring-1 ring-white/[0.07]">
              {#if track.cover_art}
                <img
                  loading="lazy"
                  decoding="async"
                  src={getHighResArtwork(track.cover_art, track.id)}
                  alt={track.title}
                  onload={(e) => handleThumbnailLoad(e, track.id)}
                  onerror={(e) => handleThumbnailError(e, track.id)}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                />
              {/if}
              <div class="absolute inset-0 bg-black/35 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
                <div class="w-10 h-10 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                  <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                </div>
              </div>
            </div>
            <div class="px-0.5 overflow-hidden">
              <p class="text-xs font-semibold text-white truncate block w-full">{track.title}</p>
              <p class="text-[11px] text-white/50 truncate block w-full mt-0.5">{formatDuration(track.duration) || artistName}</p>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- 4. ArchiveTune "Fans might also like" Similar Artists Row -->
  {#if similarArtists.length > 0}
    <section class="space-y-3">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white px-1">Fans might also like</h2>
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3.5">
        {#each similarArtists as sim}
          <button
            type="button"
            onclick={() => playerState.openArtist(sim.name)}
            class="p-3.5 rounded-2xl bg-white/[0.02] hover:bg-white/[0.06] border border-white/[0.05] flex flex-col items-center text-center gap-2.5 transition-all cursor-pointer group"
          >
            <div class="w-16 h-16 rounded-full overflow-hidden bg-white/5 shadow-md">
              {#if sim.avatar}
                <img
                  loading="lazy"
                  decoding="async"
                  src={getHighResArtwork(sim.avatar, sim.id)}
                  alt={sim.name}
                  onload={(e) => handleThumbnailLoad(e, sim.id)}
                  onerror={(e) => handleThumbnailError(e, sim.id)}
                  class="w-16 h-16 rounded-full object-cover group-hover:scale-105 transition-transform"
                />
              {/if}
            </div>
            <span class="text-xs font-semibold text-white truncate w-full">{sim.name}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>
