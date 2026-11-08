<script lang="ts">
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import {
    sanitizeArtist,
    handleThumbnailError,
    handleThumbnailLoad,
    type Track,
    type Playlist,
  } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from '../components/ArtistLinks.svelte';

  type FilterChip = 'all' | 'playlists' | 'songs' | 'albums' | 'artists' | 'downloaded';
  let activeFilter = $state<FilterChip>('all');
  let viewMode = $state<'grid' | 'list'>('list');
  let searchQuery = $state('');
  let sortBy = $state<'date' | 'title' | 'artist' | 'plays'>('date');
  let selectedAutoPlaylist = $state<'liked' | 'downloaded' | 'top50' | 'cached' | null>(null);
  let isSyncingCloud = $state(false);

  // ArchiveTune Auto-Playlists
  let likedTracks = $derived(
    playerState.library.filter((t) => t.favorite)
  );
  let downloadedTracks = $derived(
    playerState.library.filter(
      (t) =>
        t.file_path.includes('Aethon Downloads') ||
        t.id.startsWith('dl_') ||
        (!t.file_path.startsWith('http://') && !t.file_path.startsWith('https://'))
    )
  );
  let top50Tracks = $derived(
    [...playerState.library, ...playerState.playHistoryTracks]
      .filter((t, idx, arr) => arr.findIndex((x) => x.id === t.id) === idx && t.play_count > 0)
      .sort((a, b) => b.play_count - a.play_count)
      .slice(0, 50)
  );
  let historyCachedTracks = $derived.by(() => {
    const seen = new Set<string>();
    const list: Track[] = [];
    for (const t of [...playerState.playHistoryTracks, ...playerState.library]) {
      if (!seen.has(t.id)) {
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
      subtitle: `${likedTracks.length} songs`,
      icon: 'favorite',
      gradient: 'from-rose-500/35 via-pink-600/20 to-purple-900/20',
      iconColor: 'text-rose-400',
      tracks: likedTracks,
    },
    {
      id: 'downloaded' as const,
      title: 'Offline & Local',
      subtitle: `${downloadedTracks.length} songs`,
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

  let artistGroups = $derived.by(() => {
    const map = new Map<string, { name: string; count: number; cover?: string; id: string }>();
    for (const t of [...playerState.library, ...playerState.playHistoryTracks]) {
      const a = sanitizeArtist(t.artist).split(',')[0].trim() || 'Unknown Artist';
      const existing = map.get(a.toLowerCase());
      if (existing) {
        existing.count += 1;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(a.toLowerCase(), { name: a, count: 1, cover: t.cover_art, id: t.id });
      }
    }
    return Array.from(map.values()).sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
  });

  let albumGroups = $derived.by(() => {
    const map = new Map<string, { title: string; artist: string; count: number; cover?: string; id: string }>();
    for (const t of [...playerState.library, ...playerState.playHistoryTracks]) {
      const alb = t.album || 'YouTube Music';
      const key = alb.toLowerCase();
      const existing = map.get(key);
      if (existing) {
        existing.count += 1;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(key, {
          title: alb,
          artist: sanitizeArtist(t.artist),
          count: 1,
          cover: t.cover_art,
          id: t.id,
        });
      }
    }
    return Array.from(map.values());
  });

  let displayTracks = $derived.by(() => {
    const seen = new Set<string>();
    let list: Track[] = [];
    const source =
      activeFilter === 'downloaded'
        ? downloadedTracks
        : [...playerState.library, ...playerState.playHistoryTracks];

    for (const t of source) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        list.push(t);
      }
    }

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
    isSyncingCloud = true;
    try {
      await settingsState.syncYtmLibrary();
      await playerState.refreshLibrary();
    } finally {
      isSyncingCloud = false;
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
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-medium bg-white/[0.06] hover:bg-white/[0.12] border border-white/[0.08] text-white/85 transition-colors cursor-pointer"
        title={settingsState.ytmConnected ? 'Sync YouTube Music Cloud Library' : 'Connect Google Account to sync YouTube Music'}
      >
        <span class="material-symbols-rounded text-base {isSyncingCloud ? 'animate-spin' : ''}">
          cloud_sync
        </span>
        <span>{settingsState.ytmConnected ? (isSyncingCloud ? 'Syncing...' : 'Sync YTM') : 'Connect YTM'}</span>
      </button>

      <button
        type="button"
        onclick={() => playerState.scanLibrary()}
        class="w-8 h-8 rounded-xl bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer"
        title="Scan local ~/Music folder"
      >
        <span class="material-symbols-rounded text-base">folder_open</span>
      </button>
    </div>
  </div>

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
            class="px-4 py-1.5 rounded-xl text-xs font-medium transition-all cursor-pointer shrink-0 {activeFilter === chip.id
              ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] font-semibold shadow-sm'
              : 'bg-white/[0.05] text-white/70 hover:text-white hover:bg-white/10'}"
          >
            {chip.label}
          </button>
        {/each}
      </div>

      <div class="flex items-center gap-2">
        <div class="relative">
          <span class="material-symbols-rounded absolute left-3 top-1/2 -translate-y-1/2 text-white/40 text-sm">search</span>
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search in library..."
            class="w-44 sm:w-56 pl-8 pr-3 py-1.5 rounded-xl bg-white/[0.05] border border-white/[0.08] text-white text-xs placeholder:text-white/35 focus:outline-none focus:border-[var(--md-sys-primary)]/50"
          />
        </div>

        <select
          bind:value={sortBy}
          class="bg-white/[0.05] border border-white/[0.08] rounded-xl px-2.5 py-1.5 text-xs text-white/80 focus:outline-none cursor-pointer"
        >
          <option value="date" class="bg-[#181520]">Recently added</option>
          <option value="plays" class="bg-[#181520]">Most played</option>
          <option value="title" class="bg-[#181520]">Name</option>
          <option value="artist" class="bg-[#181520]">Artist</option>
        </select>

        <button
          type="button"
          onclick={() => (viewMode = viewMode === 'grid' ? 'list' : 'grid')}
          class="w-8 h-8 rounded-xl bg-white/[0.05] hover:bg-white/[0.1] flex items-center justify-center text-white/70 hover:text-white cursor-pointer"
          title="Toggle Grid / List view"
        >
          <span class="material-symbols-rounded text-base">
            {viewMode === 'grid' ? 'view_list' : 'grid_view'}
          </span>
        </button>
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
          <div class="flex-1 text-center sm:text-left space-y-2">
            <span class="text-[10px] font-bold uppercase tracking-wider text-white/55">Auto-Playlist</span>
            <h2 class="font-display font-bold text-2xl sm:text-3xl text-white">{activeAuto.title}</h2>
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
                  <span>Play</span>
                </button>
                <button
                  type="button"
                  onclick={() => {
                    const shuffled = [...activeAuto.tracks].sort(() => Math.random() - 0.5);
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

        <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
          {#each activeAuto.tracks as track, idx (track.id)}
            {@const isCurrent = playerState.currentTrack?.id === track.id}
            <div
              role="button"
              tabindex="0"
              onclick={() => playerState.playTrack(track)}
              oncontextmenu={(e) => handleContextMenu(e, track)}
              onkeydown={(e) => {
                if (e.key === 'Enter') playerState.playTrack(track);
              }}
              class="h-14 flex items-center justify-between px-3.5 {isCurrent ? 'bg-white/[0.08]' : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group overflow-hidden"
            >
              <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                <span class="w-5 text-center font-mono text-xs text-white/35 shrink-0">{idx + 1}</span>
                <div class="w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
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
                  {/if}
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
                {#if track.duration > 0}
                  <span class="font-mono text-[11px] text-white/40">{formatTime(track.duration)}</span>
                {/if}
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.toggleFavorite(track.id);
                  }}
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-rose-400 cursor-pointer"
                >
                  <span class="material-symbols-rounded text-base {track.favorite ? 'text-rose-400 filled' : ''}">favorite</span>
                </button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

  <!-- USER / CLOUD PLAYLIST DETAIL VIEW -->
  {:else if playerState.selectedPlaylist}
    {@const pl = playerState.selectedPlaylist}
    <div class="space-y-5">
      <div class="p-6 rounded-3xl bg-white/[0.03] border border-white/[0.08] flex flex-col sm:flex-row items-center gap-6">
        <div class="w-24 h-24 rounded-2xl bg-[var(--md-sys-primary-container)]/40 border border-white/10 flex items-center justify-center shrink-0 overflow-hidden">
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
        <div class="flex-1 text-center sm:text-left space-y-2">
          <h2 class="font-display font-bold text-2xl sm:text-3xl text-white">{pl.name}</h2>
          {#if pl.description}
            <p class="text-xs text-white/55">{pl.description}</p>
          {/if}
          <p class="text-[11px] text-white/40">{playerState.playlistTracks.length} songs</p>
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
                  const shuffled = [...playerState.playlistTracks].sort(() => Math.random() - 0.5);
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

      <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
        {#each playerState.playlistTracks as track, idx (track.id)}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => handleContextMenu(e, track)}
            onkeydown={(e) => {
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="h-14 flex items-center justify-between px-3.5 hover:bg-white/[0.05] transition-colors group cursor-pointer overflow-hidden"
          >
            <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
              <span class="w-5 text-center font-mono text-xs text-white/35 shrink-0">{idx + 1}</span>
              <div class="w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
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
                {/if}
              </div>
              <div class="min-w-0 flex-1 overflow-hidden">
                <p class="text-xs font-semibold text-white truncate block w-full">{track.title}</p>
                <ArtistLinks
                  artist={track.artist}
                  class="text-[11px] text-white/55 mt-0.5"
                  buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                />
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              {#if track.duration > 0}
                <span class="text-xs font-mono text-white/40">{formatTime(track.duration)}</span>
              {/if}
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.removeFromPlaylist(pl.id, track.id);
                }}
                class="p-1 rounded-full text-white/40 hover:text-rose-400 hover:bg-white/10 transition-colors cursor-pointer"
                title="Remove from playlist"
              >
                <span class="material-symbols-rounded text-base">close</span>
              </button>
            </div>
          </div>
        {/each}
      </div>
    </div>

  {:else}
    <!-- 1. ArchiveTune Pinned Auto-Playlists + User/Cloud Playlists Grid -->
    {#if activeFilter === 'all' || activeFilter === 'playlists'}
      <section class="space-y-3">
        {#if activeFilter === 'all'}
          <h2 class="font-display font-bold text-base text-white/90 px-0.5">Playlists & Collections</h2>
        {/if}
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3.5">
          {#each autoPlaylists as auto}
            <button
              type="button"
              onclick={() => (selectedAutoPlaylist = auto.id)}
              class="p-4 rounded-2xl bg-gradient-to-br {auto.gradient} border border-white/[0.08] hover:border-white/25 flex flex-col justify-between aspect-[4/3.6] text-left transition-all hover:-translate-y-0.5 cursor-pointer group"
            >
              <div class="w-10 h-10 rounded-xl bg-black/30 border border-white/10 flex items-center justify-center group-hover:scale-105 transition-transform">
                <span class="material-symbols-rounded text-2xl {auto.iconColor} filled">{auto.icon}</span>
              </div>
              <div class="overflow-hidden w-full">
                <h3 class="font-display font-bold text-xs sm:text-sm text-white truncate">{auto.title}</h3>
                <p class="text-[11px] text-white/60 truncate mt-0.5">{auto.subtitle}</p>
              </div>
            </button>
          {/each}

          {#each playerState.playlists as pl (pl.id)}
            <div
              role="button"
              tabindex="0"
              onclick={() => playerState.selectPlaylist(pl)}
              onkeydown={(e) => {
                if (e.key === 'Enter') playerState.selectPlaylist(pl);
              }}
              class="p-4 rounded-2xl bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.07] flex flex-col justify-between aspect-[4/3.6] text-left transition-all hover:-translate-y-0.5 cursor-pointer group relative"
            >
              <div class="w-10 h-10 rounded-xl bg-[var(--md-sys-primary-container)]/40 text-[var(--md-sys-primary)] flex items-center justify-center">
                <span class="material-symbols-rounded text-xl">queue_music</span>
              </div>

              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.deletePlaylist(pl.id);
                }}
                class="absolute top-3 right-3 p-1 rounded-full text-white/35 hover:text-rose-400 hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
                title="Delete playlist"
              >
                <span class="material-symbols-rounded text-sm">delete</span>
              </button>

              <div class="overflow-hidden w-full">
                <h4 class="font-display font-bold text-xs sm:text-sm text-white truncate">{pl.name}</h4>
                <p class="text-[11px] text-white/50 truncate mt-0.5">
                  {pl.description?.includes('YouTube Music') ? 'YouTube Music Cloud' : `${pl.track_count} songs`}
                </p>
              </div>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    <!-- 2. Artists View -->
    {#if activeFilter === 'artists'}
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 lg:grid-cols-6 gap-4">
        {#each artistGroups as artist}
          <button
            type="button"
            onclick={() => playerState.openArtist(artist.name)}
            class="flex flex-col items-center p-3.5 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] transition-all group text-center cursor-pointer"
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

    <!-- 3. Albums View -->
    {:else if activeFilter === 'albums'}
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
        {#each albumGroups as album}
          <button
            type="button"
            onclick={() => playerState.openAlbum(album.title, album.artist, album.cover)}
            class="flex flex-col p-3 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] transition-all group text-left cursor-pointer"
          >
            <div class="aspect-square rounded-xl overflow-hidden mb-2.5 bg-white/5 shadow-md border border-white/10 flex items-center justify-center">
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

    <!-- 4. Songs List / Grid (Shown in 'all', 'songs', and 'downloaded') -->
    {:else if activeFilter === 'all' || activeFilter === 'songs' || activeFilter === 'downloaded'}
      <section class="space-y-3">
        {#if activeFilter === 'all'}
          <div class="flex items-center justify-between px-0.5">
            <h2 class="font-display font-bold text-base text-white/90">Songs ({displayTracks.length})</h2>
            {#if displayTracks.length > 0}
              <button
                type="button"
                onclick={() => {
                  const shuffled = [...displayTracks].sort(() => Math.random() - 0.5);
                  playerState.queue = shuffled.slice(1);
                  playerState.playTrack(shuffled[0]);
                }}
                class="px-3.5 py-1 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-xs font-medium text-white/80 flex items-center gap-1.5 cursor-pointer"
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
                <div
                  role="button"
                  tabindex="0"
                  onclick={() => playerState.playTrack(track)}
                  oncontextmenu={(e) => handleContextMenu(e, track)}
                  onkeydown={(e) => {
                    if (e.key === 'Enter') playerState.playTrack(track);
                  }}
                  class="group cursor-pointer space-y-2"
                >
                  <div class="relative aspect-square rounded-2xl overflow-hidden bg-white/5 ring-1 ring-white/[0.07]">
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
                    {/if}
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
                <div
                  role="button"
                  tabindex="0"
                  onclick={() => playerState.playTrack(track)}
                  oncontextmenu={(e) => handleContextMenu(e, track)}
                  onkeydown={(e) => {
                    if (e.key === 'Enter') playerState.playTrack(track);
                  }}
                  class="h-14 flex items-center justify-between px-3.5 {isCurrent
                    ? 'bg-[var(--md-sys-primary-container)]/30'
                    : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group overflow-hidden"
                >
                  <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-3">
                    <div class="relative w-10 h-10 rounded-lg overflow-hidden shrink-0 bg-white/5">
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
                    {#if track.duration > 0}
                      <span class="font-mono text-[11px] text-white/45">{formatTime(track.duration)}</span>
                    {/if}

                    <button
                      type="button"
                      onclick={(e) => {
                        e.stopPropagation();
                        playerState.toggleFavorite(track.id);
                      }}
                      class="w-8 h-8 rounded-full flex items-center justify-center text-white/40 hover:text-rose-400 hover:bg-white/10 transition-colors cursor-pointer"
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
                      title="More options"
                    >
                      <span class="material-symbols-rounded text-base">more_vert</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </section>
    {/if}
  {/if}
</div>
