<script lang="ts">
  import { onMount } from 'svelte';
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import {
    sanitizeArtist,
    splitArtists,
    handleThumbnailError,
    handleThumbnailLoad,
    type Track,
  } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from '../components/ArtistLinks.svelte';
  import CupertinoSelect from '../components/CupertinoSelect.svelte';

  type FilterChip = 'all' | 'playlists' | 'songs' | 'albums' | 'artists' | 'downloaded';
  let activeFilter = $state<FilterChip>('all');
  let viewMode = $state<'grid' | 'list'>('list');
  let searchQuery = $state('');
  let sortBy = $state<'date' | 'title' | 'artist' | 'plays'>('date');
  let selectedAutoPlaylist = $state<'liked' | 'downloaded' | 'top50' | 'cached' | null>(null);
  let isSyncingCloud = $state(false);
  let isScanningLocal = $state(false);
  let libraryNotice = $state<string | null>(null);
  let confirmDeletePlaylistId = $state<string | null>(null);

  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));
  let cachedSet = $derived(new Set(settingsState.cachedVideoIds));
  let downloadingSet = $derived(new Set(playerState.downloadingTrackIds));

  onMount(() => {
    settingsState.refreshCachedVideoIds();
  });

  function isLocalFile(t: Track): boolean {
    const fp = (t.file_path || '').trim();
    if (!fp) return false;
    return !fp.startsWith('http://') && !fp.startsWith('https://') && !fp.startsWith('watch?v=');
  }

  function isStreamCached(t: Track): boolean {
    const vid = playerState.extractVideoId(t.file_path) || playerState.extractVideoId(t.id);
    return vid ? cachedSet.has(vid) : false;
  }

  function getTrackOfflineState(t: Track): 'downloading' | 'local' | 'cached' | null {
    if (downloadingSet.has(t.id)) return 'downloading';
    if (isLocalFile(t) || t.id.startsWith('dl_') || (t.file_path || '').includes('Aethon Downloads')) {
      return 'local';
    }
    if (isStreamCached(t)) return 'cached';
    return null;
  }

  function fisherYates<T>(arr: T[]): T[] {
    const copy = [...arr];
    for (let i = copy.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [copy[i], copy[j]] = [copy[j], copy[i]];
    }
    return copy;
  }

  let allUniqueTracks = $derived.by(() => {
    const map = new Map<string, Track>();
    for (const t of [...playerState.library, ...playerState.playHistoryTracks]) {
      const existing = map.get(t.id);
      if (!existing || (t.play_count || 0) > (existing.play_count || 0)) {
        map.set(t.id, t);
      }
    }
    return Array.from(map.values());
  });

  // ArchiveTune Auto-Playlists
  let likedTracks = $derived(
    playerState.library.filter((t) => t.favorite)
  );
  let downloadedTracks = $derived(
    allUniqueTracks.filter((t) => {
      const state = getTrackOfflineState(t);
      return state === 'local' || state === 'cached';
    })
  );
  let top50Tracks = $derived(
    allUniqueTracks
      .filter((t) => (t.play_count || 0) > 0)
      .sort((a, b) => b.play_count - a.play_count)
      .slice(0, 50)
  );
  let historyCachedTracks = $derived.by(() => {
    const seen = new Set<string>();
    const list: Track[] = [];
    for (const t of [...playerState.playHistoryTracks, ...playerState.library]) {
      if (!seen.has(t.id) && (isStreamCached(t) || isLocalFile(t) || playerState.playHistoryTracks.some((h) => h.id === t.id))) {
        seen.add(t.id);
        list.push(t);
      }
      if (list.length >= 100) break;
    }
    return list;
  });

  let autoPlaylists = $derived([
    {
      id: 'liked' as const,
      title: 'Liked Songs',
      subtitle: `${likedTracks.length} ${likedTracks.length === 1 ? 'song' : 'songs'}`,
      icon: 'favorite',
      gradient: 'from-rose-500/35 via-pink-600/20 to-purple-900/20',
      iconColor: 'text-rose-400',
      tracks: likedTracks,
    },
    {
      id: 'downloaded' as const,
      title: 'Offline & Cached',
      subtitle: `${downloadedTracks.length} ${downloadedTracks.length === 1 ? 'song' : 'songs'}`,
      icon: 'download_done',
      gradient: 'from-emerald-500/35 via-teal-600/20 to-cyan-900/20',
      iconColor: 'text-emerald-400',
      tracks: downloadedTracks,
    },
    {
      id: 'top50' as const,
      title: 'My Top 50',
      subtitle: `${top50Tracks.length} most played`,
      icon: 'trending_up',
      gradient: 'from-amber-500/35 via-orange-600/20 to-red-900/20',
      iconColor: 'text-amber-400',
      tracks: top50Tracks,
    },
    {
      id: 'cached' as const,
      title: 'Cached & History',
      subtitle: `${historyCachedTracks.length} recent songs`,
      icon: 'history',
      gradient: 'from-indigo-500/35 via-violet-600/20 to-blue-900/20',
      iconColor: 'text-indigo-400',
      tracks: historyCachedTracks,
    },
  ]);

  let filteredAutoPlaylists = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return autoPlaylists;
    return autoPlaylists.filter((a) => a.title.toLowerCase().includes(q));
  });

  let filteredUserPlaylists = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    let list = q
      ? playerState.playlists.filter((p) => p.name.toLowerCase().includes(q))
      : [...playerState.playlists];
    if (sortBy === 'title') list.sort((a, b) => a.name.localeCompare(b.name));
    return list;
  });

  let artistGroups = $derived.by(() => {
    const map = new Map<string, { name: string; count: number; plays: number; cover?: string; id: string }>();
    for (const t of allUniqueTracks) {
      const a = splitArtists(sanitizeArtist(t.artist))[0]?.trim() || 'Unknown Artist';
      if (a.toLowerCase() === 'unknown artist' || a.toLowerCase() === 'various artists') continue;
      const existing = map.get(a.toLowerCase());
      if (existing) {
        existing.count += 1;
        existing.plays += t.play_count || 0;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(a.toLowerCase(), { name: a, count: 1, plays: t.play_count || 0, cover: t.cover_art, id: t.id });
      }
    }
    let list = Array.from(map.values());
    const q = searchQuery.trim().toLowerCase();
    if (q) list = list.filter((a) => a.name.toLowerCase().includes(q));
    if (sortBy === 'title' || sortBy === 'artist') {
      list.sort((a, b) => a.name.localeCompare(b.name));
    } else if (sortBy === 'plays') {
      list.sort((a, b) => b.plays - a.plays || b.count - a.count);
    } else {
      list.sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
    }
    return list;
  });

  const IGNORED_ALBUMS = new Set([
    'unknown',
    'unknown album',
    'youtube music',
    'youtube music stream',
    'youtube music history',
    'liked songs (youtube music)',
    'lossless online stream',
    'aethon downloads',
    'up next • automix',
  ]);

  let albumGroups = $derived.by(() => {
    const map = new Map<string, { title: string; artist: string; count: number; plays: number; cover?: string; id: string }>();
    for (const t of allUniqueTracks) {
      const alb = (t.album || '').trim();
      if (!alb || IGNORED_ALBUMS.has(alb.toLowerCase())) continue;
      const primaryArtist = splitArtists(sanitizeArtist(t.artist))[0] || 'Unknown Artist';
      const key = `${alb.toLowerCase()}::${primaryArtist.toLowerCase()}`;
      const existing = map.get(key);
      if (existing) {
        existing.count += 1;
        existing.plays += t.play_count || 0;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(key, {
          title: alb,
          artist: primaryArtist,
          count: 1,
          plays: t.play_count || 0,
          cover: t.cover_art,
          id: t.id,
        });
      }
    }
    let list = Array.from(map.values());
    const q = searchQuery.trim().toLowerCase();
    if (q) list = list.filter((al) => al.title.toLowerCase().includes(q) || al.artist.toLowerCase().includes(q));
    if (sortBy === 'title') {
      list.sort((a, b) => a.title.localeCompare(b.title));
    } else if (sortBy === 'artist') {
      list.sort((a, b) => a.artist.localeCompare(b.artist));
    } else if (sortBy === 'plays') {
      list.sort((a, b) => b.plays - a.plays || b.count - a.count);
    } else {
      list.sort((a, b) => b.count - a.count || a.title.localeCompare(b.title));
    }
    return list;
  });

  let displayTracks = $derived.by(() => {
    let list = activeFilter === 'downloaded' ? [...downloadedTracks] : [...allUniqueTracks];

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(
        (t) =>
          t.title.toLowerCase().includes(q) ||
          t.artist.toLowerCase().includes(q) ||
          (t.album || '').toLowerCase().includes(q)
      );
    }

    if (sortBy === 'title') {
      list.sort((a, b) => a.title.localeCompare(b.title));
    } else if (sortBy === 'artist') {
      list.sort((a, b) => sanitizeArtist(a.artist).localeCompare(sanitizeArtist(b.artist)));
    } else if (sortBy === 'plays') {
      list.sort((a, b) => b.play_count - a.play_count);
    } else {
      list.sort((a, b) => b.date_added - a.date_added);
    }

    return list;
  });

  async function handleCloudSync() {
    if (!settingsState.ytmConnected) {
      playerState.activeTab = 'settings';
      return;
    }
    if (isSyncingCloud) return;
    isSyncingCloud = true;
    libraryNotice = null;
    try {
      const summary = await settingsState.syncYtmLibrary();
      await playerState.refreshLibrary();
      await settingsState.refreshCachedVideoIds();
      libraryNotice = `Synced ${summary.liked_songs_count} liked songs & ${summary.playlists_count} playlists.`;
      setTimeout(() => (libraryNotice = null), 4000);
    } catch (e: any) {
      libraryNotice = `Cloud sync failed: ${e}`;
    } finally {
      isSyncingCloud = false;
    }
  }

  async function handleLocalScan() {
    if (isScanningLocal) return;
    isScanningLocal = true;
    libraryNotice = null;
    try {
      await playerState.scanLibrary(settingsState.libraryPath);
      await settingsState.refreshCachedVideoIds();
      libraryNotice = `Local scan complete (${playerState.library.length} library tracks).`;
      setTimeout(() => (libraryNotice = null), 4000);
    } catch (e: any) {
      libraryNotice = `Folder scan failed: ${e}`;
    } finally {
      isScanningLocal = false;
    }
  }

  function handleContextMenu(e: MouseEvent, track: Track) {
    e.preventDefault();
    playerState.openContextMenu(track, e.clientX, e.clientY);
  }

  function formatTime(sec: number) {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }
</script>

<div class="px-6 py-5 space-y-6 max-w-7xl mx-auto pb-28 select-none">
  <!-- ArchiveTune Library Header -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      {#if playerState.selectedPlaylist || selectedAutoPlaylist}
        <button
          type="button"
          onclick={() => {
            playerState.selectedPlaylist = null;
            selectedAutoPlaylist = null;
          }}
          aria-label="Back to Library"
          class="w-9 h-9 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white/75 hover:text-white transition-colors cursor-pointer"
        >
          <span class="material-symbols-rounded text-xl">arrow_back</span>
        </button>
      {/if}
      <div>
        <h1 class="font-display font-bold text-2xl sm:text-3xl text-white tracking-tight">Library</h1>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <button
        type="button"
        onclick={() => (playerState.isNewPlaylistModalOpen = true)}
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-semibold bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-95 transition-opacity cursor-pointer"
      >
        <span class="material-symbols-rounded text-base">add</span>
        <span>New playlist</span>
      </button>

      <button
        type="button"
        onclick={handleCloudSync}
        disabled={isSyncingCloud}
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-medium bg-white/[0.06] hover:bg-white/[0.12] border border-white/[0.08] text-white/85 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
        title={settingsState.ytmConnected ? 'Sync YouTube Music Cloud Library' : 'Connect Google Account to sync YouTube Music'}
      >
        <span class="material-symbols-rounded text-base {isSyncingCloud ? 'animate-spin' : ''}">
          cloud_sync
        </span>
        <span>{settingsState.ytmConnected ? (isSyncingCloud ? 'Syncing...' : 'Sync YTM') : 'Connect YTM'}</span>
      </button>

      <button
        type="button"
        onclick={handleLocalScan}
        disabled={isScanningLocal}
        class="w-8 h-8 rounded-xl bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
        aria-label="Scan local music folder"
        title={`Scan ${settingsState.libraryPath}`}
      >
        <span class="material-symbols-rounded text-base {isScanningLocal ? 'animate-spin' : ''}">
          {isScanningLocal ? 'sync' : 'folder_open'}
        </span>
      </button>
    </div>
  </div>

  {#if libraryNotice}
    <div role="status" class="px-4 py-2.5 rounded-2xl bg-white/[0.06] border border-white/[0.08] text-xs text-white/85 flex items-center justify-between">
      <span>{libraryNotice}</span>
      <button type="button" onclick={() => (libraryNotice = null)} aria-label="Dismiss notice" class="text-white/50 hover:text-white cursor-pointer">
        <span class="material-symbols-rounded text-sm">close</span>
      </button>
    </div>
  {/if}

  <!-- ArchiveTune Filter Chips Row -->
  {#if !playerState.selectedPlaylist && !selectedAutoPlaylist}
    <div class="flex items-center justify-between gap-3 flex-wrap">
      <div class="flex items-center gap-1.5 overflow-x-auto scrollbar-none">
        {#each [
          { id: 'all', label: 'All' },
          { id: 'playlists', label: 'Playlists' },
          { id: 'songs', label: 'Songs' },
          { id: 'albums', label: 'Albums' },
          { id: 'artists', label: 'Artists' },
          { id: 'downloaded', label: 'Downloaded' },
        ] as chip}
          <button
            type="button"
            onclick={() => (activeFilter = chip.id as FilterChip)}
            class="px-4 py-1.5 rounded-xl text-xs font-medium transition-colors border cursor-pointer shrink-0 {activeFilter === chip.id
              ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] border-transparent font-semibold shadow-sm'
              : 'bg-white/[0.05] border-white/[0.06] text-white/70 hover:text-white hover:bg-white/10'}"
          >
            {chip.label}
          </button>
        {/each}
      </div>

      <div class="flex items-center gap-2">
        <div class="relative">
          <span class="material-symbols-rounded absolute left-3 top-1/2 -translate-y-1/2 text-white/40 text-sm pointer-events-none">search</span>
          <input
            type="text"
            aria-label="Search in library"
            bind:value={searchQuery}
            placeholder="Search in library..."
            class="w-44 sm:w-56 pl-8 pr-8 py-1.5 rounded-xl bg-white/[0.05] border border-white/[0.08] text-white text-xs placeholder:text-white/35 focus:outline-none focus:border-[var(--md-sys-primary)]"
          />
          {#if searchQuery}
            <button
              type="button"
              onclick={() => (searchQuery = '')}
              aria-label="Clear library search"
              class="absolute right-2 top-1/2 -translate-y-1/2 text-white/45 hover:text-white cursor-pointer"
            >
              <span class="material-symbols-rounded text-sm">close</span>
            </button>
          {/if}
        </div>

        <CupertinoSelect
          bind:value={sortBy}
          options={[
            { value: 'date', label: 'Recently added' },
            { value: 'plays', label: 'Most played' },
            { value: 'title', label: 'Name' },
            { value: 'artist', label: 'Artist' },
          ]}
          label=""
        />

        {#if activeFilter === 'all' || activeFilter === 'songs' || activeFilter === 'downloaded'}
          <button
            type="button"
            onclick={() => (viewMode = viewMode === 'grid' ? 'list' : 'grid')}
            class="w-8 h-8 rounded-xl bg-white/[0.05] hover:bg-white/[0.1] flex items-center justify-center text-white/70 hover:text-white cursor-pointer transition-colors"
            aria-label={viewMode === 'grid' ? 'Switch to list view' : 'Switch to grid view'}
            title="Toggle Grid / List view"
          >
            <span class="material-symbols-rounded text-base">
              {viewMode === 'grid' ? 'view_list' : 'grid_view'}
            </span>
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <!-- AUTO-PLAYLIST DETAIL VIEW (Liked / Offline / Top 50 / Cached) -->
  {#if selectedAutoPlaylist}
    {@const activeAuto = autoPlaylists.find((a) => a.id === selectedAutoPlaylist)}
    {#if activeAuto}
      <div class="space-y-5">
        <div class="p-6 rounded-3xl bg-gradient-to-br {activeAuto.gradient} border border-white/10 flex flex-col sm:flex-row items-center gap-6">
          <div class="w-24 h-24 rounded-2xl bg-black/30 border border-white/10 flex items-center justify-center shrink-0">
            <span class="material-symbols-rounded text-5xl {activeAuto.iconColor} filled">{activeAuto.icon}</span>
          </div>
          <div class="w-full sm:w-auto flex-1 min-w-0 text-center sm:text-left space-y-2">
            <span class="text-[10px] font-bold uppercase tracking-wider text-white/55">Auto-Playlist</span>
            <h2 class="font-display font-bold text-2xl sm:text-3xl text-white truncate">{activeAuto.title}</h2>
            <p class="text-xs text-white/65">{activeAuto.subtitle}</p>
            {#if activeAuto.tracks.length > 0}
              <div class="flex items-center justify-center sm:justify-start gap-2.5 pt-1">
                <button
                  type="button"
                  onclick={() => {
                    playerState.queue = activeAuto.tracks.slice(1);
                    playerState.playTrack(activeAuto.tracks[0]);
                  }}
                  class="px-5 py-2 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
                >
                  <span class="material-symbols-rounded text-base filled">play_arrow</span>
                  <span>Play All</span>
                </button>
                <button
                  type="button"
                  onclick={() => {
                    const shuffled = fisherYates(activeAuto.tracks);
                    playerState.queue = shuffled.slice(1);
                    playerState.playTrack(shuffled[0]);
                  }}
                  class="px-4 py-2 rounded-full bg-white/10 hover:bg-white/15 text-white text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
                >
                  <span class="material-symbols-rounded text-base">shuffle</span>
                  <span>Shuffle</span>
                </button>
              </div>
            {/if}
          </div>
        </div>

        {#if activeAuto.tracks.length === 0}
          <div class="py-16 text-center rounded-2xl bg-white/[0.02] border border-white/[0.05] space-y-2">
            <span class="material-symbols-rounded text-4xl text-white/25">{activeAuto.icon}</span>
            <p class="text-sm font-medium text-white/65">No tracks in {activeAuto.title} yet</p>
            <p class="text-xs text-white/40">Play, favorite, or download songs to populate this collection</p>
          </div>
        {:else}
          <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
            {#each activeAuto.tracks as track, idx (track.id)}
              {@const isCurrent = playerState.currentTrack?.id === track.id}
              {@const offlineState = getTrackOfflineState(track)}
              <div
                role="button"
                tabindex="0"
                onclick={() => playerState.playTrack(track)}
                oncontextmenu={(e) => handleContextMenu(e, track)}
                onkeydown={(e) => {
                  if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                    e.preventDefault();
                    playerState.playTrack(track);
                  }
                }}
                class="h-14 flex items-center justify-between px-3.5 transition-colors cursor-pointer group overflow-hidden {isCurrent ? '' : 'hover:bg-white/[0.05]'}"
                style={isCurrent ? 'background-color: color-mix(in srgb, var(--md-sys-primary-container) 35%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-primary) 35%, transparent);' : ''}
              >
                <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                  <span class="w-5 text-center font-mono text-xs text-white/35 shrink-0">{idx + 1}</span>
                  <div class="relative w-10 h-10 rounded-xl overflow-hidden bg-white/5 shrink-0">
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
                      <div class="w-full h-full flex items-center justify-center text-white/30">
                        <span class="material-symbols-rounded text-lg">music_note</span>
                      </div>
                    {/if}
                    <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
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
                        <span class="material-symbols-rounded text-white text-lg filled">play_arrow</span>
                      {/if}
                    </div>
                  </div>
                  <div class="min-w-0 flex-1 overflow-hidden">
                    <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate block w-full">
                      {track.title}
                    </p>
                    <ArtistLinks
                      artist={track.artist}
                      class="text-[11px] text-white/55 mt-0.5"
                      buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                    />
                  </div>
                </div>
                <div class="flex items-center gap-2 shrink-0">
                  {#if offlineState === 'downloading'}
                    <span class="px-2 py-0.5 rounded-full text-[10px] font-medium bg-amber-500/15 text-amber-300 border border-amber-500/25">Downloading…</span>
                  {:else if offlineState === 'local'}
                    <span class="material-symbols-rounded text-emerald-400 text-base" title="Offline / Local File">offline_pin</span>
                  {:else if offlineState === 'cached'}
                    <span class="material-symbols-rounded text-sky-400 text-base" title="Cached on Disk">download_done</span>
                  {/if}
                  {#if track.duration > 0}
                    <span class="font-mono text-[11px] text-white/40">{formatTime(track.duration)}</span>
                  {/if}
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      playerState.toggleFavorite(track.id, track);
                    }}
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-rose-400 cursor-pointer"
                    aria-label={track.favorite ? 'Remove favorite' : 'Favorite'}
                  >
                    <span class="material-symbols-rounded text-base {track.favorite ? 'text-rose-400 filled' : ''}">favorite</span>
                  </button>
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      handleContextMenu(e, track);
                    }}
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white cursor-pointer"
                    aria-label="More options"
                    title="More options"
                  >
                    <span class="material-symbols-rounded text-base">more_vert</span>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

  <!-- USER / CLOUD PLAYLIST DETAIL VIEW -->
  {:else if playerState.selectedPlaylist}
    {@const pl = playerState.selectedPlaylist}
    {@const isCloudPlaylist = pl.id.startsWith('VL') || pl.id.startsWith('PL') || pl.id.startsWith('RD') || Boolean(pl.description?.includes('YouTube Music'))}
    <div class="space-y-5">
      <div class="p-6 rounded-3xl bg-white/[0.03] border border-white/[0.08] flex flex-col sm:flex-row items-center gap-6">
        <div
          class="w-24 h-24 rounded-2xl border border-white/10 flex items-center justify-center shrink-0 overflow-hidden"
          style="background-color: color-mix(in srgb, var(--md-sys-primary-container) 40%, transparent);"
        >
          {#if playerState.playlistTracks[0]?.cover_art}
            <img
              src={getHighResArtwork(playerState.playlistTracks[0].cover_art, playerState.playlistTracks[0].id)}
              alt={pl.name}
              onload={(e) => handleThumbnailLoad(e, playerState.playlistTracks[0]?.id)}
              onerror={(e) => handleThumbnailError(e, playerState.playlistTracks[0]?.id)}
              class="w-full h-full object-cover"
            />
          {:else}
            <span class="material-symbols-rounded text-4xl text-[var(--md-sys-primary)]">queue_music</span>
          {/if}
        </div>
        <div class="w-full sm:w-auto flex-1 min-w-0 text-center sm:text-left space-y-2">
          <h2 class="font-display font-bold text-2xl sm:text-3xl text-white truncate">{pl.name}</h2>
          {#if pl.description}
            <p class="text-xs text-white/55 truncate">{pl.description}</p>
          {/if}
          <p class="text-[11px] text-white/40">
            {playerState.isLoadingPlaylist ? 'Loading playlist…' : `${playerState.playlistTracks.length} ${playerState.playlistTracks.length === 1 ? 'song' : 'songs'}`}
          </p>
          {#if playerState.playlistTracks.length > 0}
            <div class="flex items-center justify-center sm:justify-start gap-2.5 pt-1">
              <button
                type="button"
                onclick={() => {
                  playerState.queue = playerState.playlistTracks.slice(1);
                  playerState.playTrack(playerState.playlistTracks[0]);
                }}
                class="px-5 py-2 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
              >
                <span class="material-symbols-rounded text-base filled">play_arrow</span>
                <span>Play All</span>
              </button>
              <button
                type="button"
                onclick={() => {
                  const shuffled = fisherYates(playerState.playlistTracks);
                  playerState.queue = shuffled.slice(1);
                  playerState.playTrack(shuffled[0]);
                }}
                class="px-4 py-2 rounded-full bg-white/10 hover:bg-white/15 text-white text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
              >
                <span class="material-symbols-rounded text-base">shuffle</span>
                <span>Shuffle</span>
              </button>
            </div>
          {/if}
        </div>
      </div>

      {#if playerState.isLoadingPlaylist}
        <div class="space-y-2">
          {#each Array(6) as _}
            <div class="h-14 rounded-2xl bg-white/[0.04] animate-pulse"></div>
          {/each}
        </div>
      {:else if playerState.playlistTracks.length === 0}
        <div class="py-16 text-center rounded-2xl bg-white/[0.02] border border-white/[0.05] space-y-2">
          <span class="material-symbols-rounded text-4xl text-white/25">queue_music</span>
          <p class="text-sm font-medium text-white/65">This playlist is empty</p>
          <p class="text-xs text-white/40">Right-click any song and choose "Add to Playlist…"</p>
        </div>
      {:else}
        <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
          {#each playerState.playlistTracks as track, idx (track.id)}
            {@const isCurrent = playerState.currentTrack?.id === track.id}
            {@const offlineState = getTrackOfflineState(track)}
            <div
              role="button"
              tabindex="0"
              onclick={() => playerState.playTrack(track)}
              oncontextmenu={(e) => handleContextMenu(e, track)}
              onkeydown={(e) => {
                if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                  e.preventDefault();
                  playerState.playTrack(track);
                }
              }}
              class="h-14 flex items-center justify-between px-3.5 transition-colors group cursor-pointer overflow-hidden {isCurrent ? '' : 'hover:bg-white/[0.05]'}"
              style={isCurrent ? 'background-color: color-mix(in srgb, var(--md-sys-primary-container) 35%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-primary) 35%, transparent);' : ''}
            >
              <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                <span class="w-5 text-center font-mono text-xs text-white/35 shrink-0">{idx + 1}</span>
                <div class="relative w-10 h-10 rounded-xl overflow-hidden bg-white/5 shrink-0">
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
                    <div class="w-full h-full flex items-center justify-center text-white/30">
                      <span class="material-symbols-rounded text-lg">music_note</span>
                    </div>
                  {/if}
                  <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
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
                      <span class="material-symbols-rounded text-white text-lg filled">play_arrow</span>
                    {/if}
                  </div>
                </div>
                <div class="min-w-0 flex-1 overflow-hidden">
                  <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate block w-full">{track.title}</p>
                  <ArtistLinks
                    artist={track.artist}
                    class="text-[11px] text-white/55 mt-0.5"
                    buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                  />
                </div>
              </div>

              <div class="flex items-center gap-2 shrink-0">
                {#if offlineState === 'local'}
                  <span class="material-symbols-rounded text-emerald-400 text-base" title="Offline / Local File">offline_pin</span>
                {:else if offlineState === 'cached'}
                  <span class="material-symbols-rounded text-sky-400 text-base" title="Cached on Disk">download_done</span>
                {/if}
                {#if track.duration > 0}
                  <span class="text-xs font-mono text-white/40">{formatTime(track.duration)}</span>
                {/if}
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.toggleFavorite(track.id, track);
                  }}
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-rose-400 cursor-pointer"
                  aria-label={track.favorite ? 'Remove favorite' : 'Favorite'}
                >
                  <span class="material-symbols-rounded text-base {track.favorite ? 'text-rose-400 filled' : ''}">favorite</span>
                </button>
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    handleContextMenu(e, track);
                  }}
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white cursor-pointer"
                  aria-label="More options"
                  title="More options"
                >
                  <span class="material-symbols-rounded text-base">more_vert</span>
                </button>
                {#if !isCloudPlaylist}
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      playerState.removeFromPlaylist(pl.id, track.id);
                    }}
                    class="p-1 rounded-full text-white/40 hover:text-rose-400 hover:bg-white/10 transition-colors cursor-pointer"
                    aria-label="Remove from playlist"
                    title="Remove from playlist"
                  >
                    <span class="material-symbols-rounded text-base">close</span>
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

  {:else}
    <!-- 1. ArchiveTune Pinned Auto-Playlists + User/Cloud Playlists Grid -->
    {#if activeFilter === 'all' || activeFilter === 'playlists'}
      <section class="space-y-3">
        {#if activeFilter === 'all'}
          <h2 class="font-display font-bold text-base text-white/90 px-0.5 tracking-tight">Playlists & Collections</h2>
        {/if}
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3.5">
          {#each filteredAutoPlaylists as auto}
            <button
              type="button"
              onclick={() => (selectedAutoPlaylist = auto.id)}
              class="p-4 rounded-2xl bg-gradient-to-br {auto.gradient} border border-white/[0.08] hover:border-white/25 flex flex-col justify-between min-h-[124px] text-left transition-[transform,border-color] hover:-translate-y-0.5 cursor-pointer group"
            >
              <div class="w-10 h-10 rounded-xl bg-black/30 border border-white/10 flex items-center justify-center group-hover:scale-105 transition-transform">
                <span class="material-symbols-rounded text-2xl {auto.iconColor} filled">{auto.icon}</span>
              </div>
              <div class="overflow-hidden w-full pt-3">
                <h3 class="font-display font-bold text-xs sm:text-sm text-white truncate">{auto.title}</h3>
                <p class="text-[11px] text-white/60 truncate mt-0.5">{auto.subtitle}</p>
              </div>
            </button>
          {/each}

          {#each filteredUserPlaylists as pl (pl.id)}
            <div
              role="button"
              tabindex="0"
              onclick={() => playerState.selectPlaylist(pl)}
              onkeydown={(e) => {
                if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                  e.preventDefault();
                  playerState.selectPlaylist(pl);
                }
              }}
              class="p-4 rounded-2xl bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.07] flex flex-col justify-between min-h-[124px] text-left transition-[transform,background-color,border-color] hover:-translate-y-0.5 cursor-pointer group relative"
            >
              <div
                class="w-10 h-10 rounded-xl text-[var(--md-sys-primary)] flex items-center justify-center"
                style="background-color: color-mix(in srgb, var(--md-sys-primary-container) 40%, transparent);"
              >
                <span class="material-symbols-rounded text-xl">queue_music</span>
              </div>

              {#if confirmDeletePlaylistId === pl.id}
                <div class="absolute top-2.5 right-2.5 flex items-center gap-1 bg-black/80 rounded-full px-2 py-0.5 border border-rose-500/40">
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      confirmDeletePlaylistId = null;
                      playerState.deletePlaylist(pl.id);
                    }}
                    class="text-[10px] font-bold text-rose-400 hover:text-rose-300 cursor-pointer"
                  >
                    Delete
                  </button>
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      confirmDeletePlaylistId = null;
                    }}
                    class="text-[10px] text-white/60 hover:text-white cursor-pointer"
                  >
                    Cancel
                  </button>
                </div>
              {:else}
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    confirmDeletePlaylistId = pl.id;
                  }}
                  aria-label="Delete playlist {pl.name}"
                  class="absolute top-3 right-3 p-1 rounded-full text-white/35 hover:text-rose-400 hover:bg-white/10 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity cursor-pointer"
                  title="Delete playlist"
                >
                  <span class="material-symbols-rounded text-sm">delete</span>
                </button>
              {/if}

              <div class="overflow-hidden w-full pt-3">
                <h4 class="font-display font-bold text-xs sm:text-sm text-white truncate">{pl.name}</h4>
                <p class="text-[11px] text-white/50 truncate mt-0.5">
                  {pl.description?.includes('YouTube Music') ? 'YouTube Music Cloud' : `${pl.track_count} ${pl.track_count === 1 ? 'song' : 'songs'}`}
                </p>
              </div>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    <!-- 2. Artists View -->
    {#if activeFilter === 'artists'}
      {#if artistGroups.length === 0}
        <div class="py-16 text-center rounded-2xl bg-white/[0.02] border border-white/[0.05] space-y-2">
          <span class="material-symbols-rounded text-4xl text-white/25">person</span>
          <p class="text-sm font-medium text-white/65">No artists found</p>
        </div>
      {:else}
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 lg:grid-cols-6 gap-4">
          {#each artistGroups as artist}
            <button
              type="button"
              onclick={() => playerState.openArtist(artist.name)}
              class="flex flex-col items-center p-3.5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] transition-colors group text-center cursor-pointer"
            >
              <div class="w-24 h-24 rounded-full overflow-hidden mb-3 bg-white/5 shadow-md border border-white/10 flex items-center justify-center">
                {#if artist.cover}
                  <img
                    loading="lazy"
                    decoding="async"
                    src={getHighResArtwork(artist.cover, artist.id)}
                    alt={artist.name}
                    onload={(e) => handleThumbnailLoad(e, artist.id)}
                    onerror={(e) => handleThumbnailError(e, artist.id)}
                    class="w-full h-full object-cover group-hover:scale-105 transition-transform"
                  />
                {:else}
                  <span class="material-symbols-rounded text-3xl text-white/40">person</span>
                {/if}
              </div>
              <h4 class="text-xs font-semibold text-white truncate w-full">{artist.name}</h4>
              <p class="text-[10px] text-white/45 mt-0.5">{artist.count} {artist.count === 1 ? 'song' : 'songs'}</p>
            </button>
          {/each}
        </div>
      {/if}

    <!-- 3. Albums View -->
    {:else if activeFilter === 'albums'}
      {#if albumGroups.length === 0}
        <div class="py-16 text-center rounded-2xl bg-white/[0.02] border border-white/[0.05] space-y-2">
          <span class="material-symbols-rounded text-4xl text-white/25">album</span>
          <p class="text-sm font-medium text-white/65">No albums found</p>
        </div>
      {:else}
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
          {#each albumGroups as album}
            <button
              type="button"
              onclick={() => playerState.openAlbum(album.title, album.artist, album.cover)}
              class="flex flex-col p-3 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] transition-colors group text-left cursor-pointer"
            >
              <div class="aspect-square rounded-2xl overflow-hidden mb-2.5 bg-white/5 shadow-md border border-white/10 flex items-center justify-center">
                {#if album.cover}
                  <img
                    loading="lazy"
                    decoding="async"
                    src={getHighResArtwork(album.cover, album.id)}
                    alt={album.title}
                    onload={(e) => handleThumbnailLoad(e, album.id)}
                    onerror={(e) => handleThumbnailError(e, album.id)}
                    class="w-full h-full object-cover group-hover:scale-105 transition-transform"
                  />
                {:else}
                  <span class="material-symbols-rounded text-3xl text-white/40">album</span>
                {/if}
              </div>
              <h4 class="text-xs font-semibold text-white truncate w-full">{album.title}</h4>
              <p class="text-[11px] text-white/50 truncate w-full mt-0.5">{album.artist}</p>
            </button>
          {/each}
        </div>
      {/if}

    <!-- 4. Songs List / Grid (Shown in 'all', 'songs', and 'downloaded') -->
    {:else if activeFilter === 'all' || activeFilter === 'songs' || activeFilter === 'downloaded'}
      <section class="space-y-3">
        {#if activeFilter === 'all' || activeFilter === 'songs' || activeFilter === 'downloaded'}
          <div class="flex items-center justify-between px-0.5">
            <h2 class="font-display font-bold text-base text-white/90 tracking-tight">
              {activeFilter === 'downloaded' ? 'Offline & Cached' : 'Songs'} ({displayTracks.length})
            </h2>
            {#if displayTracks.length > 0}
              <button
                type="button"
                onclick={() => {
                  const shuffled = fisherYates(displayTracks);
                  playerState.queue = shuffled.slice(1);
                  playerState.playTrack(shuffled[0]);
                }}
                class="px-3.5 py-1 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-xs font-medium text-white/80 flex items-center gap-1.5 cursor-pointer transition-colors"
              >
                <span class="material-symbols-rounded text-sm">shuffle</span>
                <span>Shuffle all</span>
              </button>
            {/if}
          </div>
        {/if}

        {#if displayTracks.length > 0}
          {#if viewMode === 'grid'}
            <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
              {#each displayTracks as track (track.id)}
                {@const isCurrent = playerState.currentTrack?.id === track.id}
                {@const offlineState = getTrackOfflineState(track)}
                <div
                  role="button"
                  tabindex="0"
                  onclick={() => playerState.playTrack(track)}
                  oncontextmenu={(e) => handleContextMenu(e, track)}
                  onkeydown={(e) => {
                    if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                      e.preventDefault();
                      playerState.playTrack(track);
                    }
                  }}
                  class="group cursor-pointer space-y-2"
                >
                  <div class="relative aspect-square rounded-2xl overflow-hidden bg-white/5 ring-1 {isCurrent ? 'ring-[var(--md-sys-primary)]' : 'ring-white/[0.07]'}">
                    {#if track.cover_art}
                      <img
                        loading="lazy"
                        decoding="async"
                        src={getHighResArtwork(track.cover_art, track.id)}
                        alt={track.title}
                        onload={(e) => handleThumbnailLoad(e, track.id)}
                        onerror={(e) => handleThumbnailError(e, track.id)}
                        class="w-full h-full object-cover group-hover:scale-105 transition-transform"
                      />
                    {:else}
                      <div class="w-full h-full flex items-center justify-center text-white/30">
                        <span class="material-symbols-rounded text-3xl">music_note</span>
                      </div>
                    {/if}
                    {#if offlineState === 'local' || offlineState === 'cached'}
                      <span class="absolute top-2 left-2 px-1.5 py-0.5 rounded-md bg-black/70 text-[9px] font-semibold {offlineState === 'local' ? 'text-emerald-400' : 'text-sky-400'}">
                        {offlineState === 'local' ? 'Local' : 'Cached'}
                      </span>
                    {/if}
                    <div class="absolute inset-0 bg-black/30 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-end justify-end p-2.5 transition-opacity">
                      <div class="w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                        <span class="material-symbols-rounded text-xl filled">
                          {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                        </span>
                      </div>
                    </div>
                  </div>
                  <div class="overflow-hidden">
                    <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate block w-full">
                      {track.title}
                    </p>
                    <ArtistLinks
                      artist={track.artist}
                      class="text-[11px] text-white/50 mt-0.5"
                      buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                    />
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
              {#each displayTracks as track (track.id)}
                {@const isCurrent = playerState.currentTrack?.id === track.id}
                {@const offlineState = getTrackOfflineState(track)}
                <div
                  role="button"
                  tabindex="0"
                  onclick={() => playerState.playTrack(track)}
                  oncontextmenu={(e) => handleContextMenu(e, track)}
                  onkeydown={(e) => {
                    if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                      e.preventDefault();
                      playerState.playTrack(track);
                    }
                  }}
                  class="h-14 flex items-center justify-between px-3.5 transition-colors cursor-pointer group overflow-hidden {isCurrent ? '' : 'hover:bg-white/[0.05]'}"
                  style={isCurrent ? 'background-color: color-mix(in srgb, var(--md-sys-primary-container) 35%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-primary) 35%, transparent);' : ''}
                >
                  <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-3">
                    <div class="relative w-10 h-10 rounded-xl overflow-hidden shrink-0 bg-white/5">
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
                        <div class="w-full h-full flex items-center justify-center text-white/40">
                          <span class="material-symbols-rounded text-lg">music_note</span>
                        </div>
                      {/if}
                      <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
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
                          <span class="material-symbols-rounded text-white text-lg filled">play_arrow</span>
                        {/if}
                      </div>
                    </div>

                    <div class="min-w-0 flex-1 overflow-hidden">
                      <h4 class="font-semibold text-xs {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate block w-full">
                        {track.title}
                      </h4>
                      <ArtistLinks
                        artist={track.artist}
                        class="text-[11px] text-white/55 mt-0.5"
                        buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                      />
                    </div>
                  </div>

                  <div class="flex items-center gap-2 shrink-0">
                    {#if offlineState === 'downloading'}
                      <span class="px-2 py-0.5 rounded-full text-[10px] font-medium bg-amber-500/15 text-amber-300 border border-amber-500/25">Downloading…</span>
                    {:else if offlineState === 'local'}
                      <span class="material-symbols-rounded text-emerald-400 text-base" title="Offline / Local File">offline_pin</span>
                    {:else if offlineState === 'cached'}
                      <span class="material-symbols-rounded text-sky-400 text-base" title="Cached on Disk">download_done</span>
                    {/if}

                    {#if track.duration > 0}
                      <span class="font-mono text-[11px] text-white/45">{formatTime(track.duration)}</span>
                    {/if}

                    <button
                      type="button"
                      onclick={(e) => {
                        e.stopPropagation();
                        playerState.toggleFavorite(track.id, track);
                      }}
                      class="w-8 h-8 rounded-full flex items-center justify-center text-white/40 hover:text-rose-400 hover:bg-white/10 transition-colors cursor-pointer"
                      aria-label={track.favorite ? 'Remove favorite' : 'Favorite'}
                      title="Favorite"
                    >
                      <span class="material-symbols-rounded text-base {track.favorite ? 'text-rose-400 filled' : ''}">
                        favorite
                      </span>
                    </button>

                    <button
                      type="button"
                      onclick={(e) => {
                        e.stopPropagation();
                        handleContextMenu(e, track);
                      }}
                      class="w-8 h-8 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
                      aria-label="More options"
                      title="More options"
                    >
                      <span class="material-symbols-rounded text-base">more_vert</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <div class="py-16 text-center rounded-2xl bg-white/[0.02] border border-white/[0.05] space-y-2">
            <span class="material-symbols-rounded text-4xl text-white/25">
              {activeFilter === 'downloaded' ? 'download_done' : 'music_note'}
            </span>
            <p class="text-sm font-medium text-white/65">
              {searchQuery.trim()
                ? `No matching tracks for "${searchQuery}"`
                : activeFilter === 'downloaded'
                  ? 'No offline or cached tracks yet'
                  : 'Your library is empty'}
            </p>
            <p class="text-xs text-white/40">
              {activeFilter === 'downloaded'
                ? 'Play or download tracks to cache them locally'
                : 'Scan a local music folder or connect YouTube Music above'}
            </p>
          </div>
        {/if}
      </section>
    {/if}
  {/if}
</div>
