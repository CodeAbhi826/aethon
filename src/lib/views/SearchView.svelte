<script lang="ts">
  import { onDestroy } from 'svelte';
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

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let searchSeq = 0;
  let isLoadingMore = $state(false);
  let hasMoreResults = $state(true);
  let currentPage = $state(1);

  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  function buildFilterQuery(q: string, filter: typeof playerState.searchFilter): string {
    if (filter === 'videos') return `${q} official music video`;
    if (filter === 'albums') return `${q} full album`;
    if (filter === 'songs') return `${q} audio`;
    return q;
  }

  function onSearchInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    const q = playerState.searchQuery.trim();
    if (!q) {
      ++searchSeq;
      playerState.clearSearch();
      hasMoreResults = true;
      currentPage = 1;
      return;
    }
    const seq = ++searchSeq;
    debounceTimer = setTimeout(async () => {
      if (seq === searchSeq) {
        currentPage = 1;
        hasMoreResults = true;
        await playerState.searchYouTube(buildFilterQuery(q, playerState.searchFilter), 55);
      }
    }, 240);
  }

  onDestroy(() => {
    if (debounceTimer) clearTimeout(debounceTimer);
  });

  function handleSearch(e: Event) {
    e.preventDefault();
    if (debounceTimer) clearTimeout(debounceTimer);
    const q = playerState.searchQuery.trim();
    if (q) {
      ++searchSeq;
      currentPage = 1;
      hasMoreResults = true;
      playerState.searchYouTube(buildFilterQuery(q, playerState.searchFilter), 55);
    }
  }

  function switchFilter(nextFilter: typeof playerState.searchFilter) {
    if (playerState.searchFilter === nextFilter) return;
    playerState.searchFilter = nextFilter;
    const q = playerState.searchQuery.trim();
    if (q && nextFilter !== 'artists') {
      if (debounceTimer) clearTimeout(debounceTimer);
      ++searchSeq;
      currentPage = 1;
      hasMoreResults = true;
      playerState.searchYouTube(buildFilterQuery(q, nextFilter), 55);
    }
  }

  async function loadMoreResults() {
    const q = playerState.searchQuery.trim();
    if (!q || isLoadingMore || playerState.isSearching || !hasMoreResults) return;
    const seq = searchSeq;
    isLoadingMore = true;
    try {
      const nextPage = currentPage + 1;
      const suffix =
        nextPage === 2
          ? 'official audio songs'
          : nextPage === 3
            ? 'full album tracks'
            : `popular tracks ${nextPage}`;
      const more = await invoke<YouTubeTrack[]>('search_youtube', {
        query: `${q} ${suffix}`,
        limit: 45,
      }).catch(() => []);
      if (seq !== searchSeq || playerState.searchQuery.trim() !== q) return;
      if (more && more.length > 0) {
        const seen = new Set(playerState.youtubeResults.map((r) => r.id));
        const unique = more.filter((r) => !seen.has(r.id));
        if (unique.length > 0) {
          currentPage = nextPage;
          playerState.youtubeResults = [...playerState.youtubeResults, ...unique];
        } else {
          hasMoreResults = false;
        }
      } else {
        hasMoreResults = false;
      }
    } finally {
      isLoadingMore = false;
    }
  }

  function runQuickSearch(label: string, query: string) {
    if (debounceTimer) clearTimeout(debounceTimer);
    ++searchSeq;
    playerState.searchQuery = label;
    playerState.searchFilter = 'all';
    currentPage = 1;
    hasMoreResults = true;
    playerState.searchYouTube(query, 55);
  }

  function ytToTrack(yt: YouTubeTrack): Track {
    return {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: sanitizeArtist(yt.artist) || 'Unknown Artist',
      album: 'YouTube Music',
      duration: yt.duration || 0,
      file_path: yt.stream_url,
      cover_art: getHighResArtwork(yt.thumbnail, yt.id),
      format: 'Opus 256k',
      play_count: 0,
      favorite: false,
      date_added: Math.floor(Date.now() / 1000),
    };
  }

  function formatDuration(sec: number): string {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }

  const quickTags = [
    { label: 'Trending', query: 'global trending songs' },
    { label: 'New releases', query: 'new music releases hits' },
    { label: 'Chill', query: 'chill lofi acoustic vibes' },
    { label: 'Phonk', query: 'drift phonk gym hits' },
    { label: 'Synthwave', query: 'synthwave retrowave night drive' },
    { label: 'Pop', query: 'pop chart hits' },
    { label: 'Hip-Hop', query: 'hip hop rap essentials' },
    { label: 'Indie', query: 'indie alternative essentials' },
  ];

  const browseCategories = [
    { label: 'Charts', sub: 'Global 100', query: 'global top hits songs', color: 'from-rose-500/25 to-orange-600/10' },
    { label: 'Chill', sub: 'Study & Relax', query: 'lofi chill beats study', color: 'from-indigo-500/25 to-purple-600/10' },
    { label: 'Hip-Hop', sub: 'Rap & Trap', query: 'hip hop rap essentials', color: 'from-amber-500/25 to-yellow-600/10' },
    { label: 'Electronic', sub: 'House & Techno', query: 'electronic house melodic', color: 'from-cyan-500/25 to-blue-600/10' },
    { label: 'Pop', sub: 'Hitmakers', query: 'pop hits essentials', color: 'from-pink-500/25 to-rose-600/10' },
    { label: 'Rock & Alt', sub: 'Anthems', query: 'alternative rock classics', color: 'from-emerald-500/25 to-teal-600/10' },
    { label: 'R&B & Soul', sub: 'Late Night', query: 'rnb soul smooth hits', color: 'from-violet-500/25 to-fuchsia-600/10' },
    { label: 'Workout', sub: 'High Energy', query: 'workout gym energy songs', color: 'from-red-500/25 to-rose-600/10' },
  ];

  let topResult = $derived(
    playerState.youtubeResults.length > 0 ? playerState.youtubeResults[0] : null
  );

  let derivedArtists = $derived.by(() => {
    const seen = new Set<string>();
    const list: { name: string; avatar: string; id: string }[] = [];
    for (const yt of playerState.youtubeResults) {
      for (const part of splitArtists(sanitizeArtist(yt.artist))) {
        const cleanName = part.trim();
        const lower = cleanName.toLowerCase();
        if (cleanName && lower !== 'unknown artist' && lower !== 'various artists' && !seen.has(lower)) {
          seen.add(lower);
          list.push({ name: cleanName, avatar: yt.thumbnail, id: yt.id });
        }
        if (list.length >= 6) break;
      }
      if (list.length >= 6) break;
    }
    return list;
  });
</script>

<div class="px-6 py-5 max-w-7xl mx-auto space-y-6 pb-28 select-none">
  <!-- ArchiveTune Search Bar ("Search YouTube Music") -->
  <div class="space-y-3.5">
    <form onsubmit={handleSearch} class="relative max-w-2xl">
      <span class="material-symbols-rounded absolute left-4 top-1/2 -translate-y-1/2 text-white/45 text-xl pointer-events-none">
        search
      </span>
      <input
        type="text"
        aria-label="Search YouTube Music"
        bind:value={playerState.searchQuery}
        oninput={onSearchInput}
        placeholder="Search YouTube Music"
        class="w-full pl-11 pr-14 py-3 rounded-2xl bg-white/[0.06] border border-white/[0.08] text-white placeholder-white/40 focus:outline-none focus:border-[var(--md-sys-primary)] focus:bg-white/[0.09] transition-colors text-sm"
      />

      <div class="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-1">
        {#if playerState.isSearching}
          <svg class="w-5 h-5 archivetune-spinner-svg text-[var(--md-sys-primary)] mr-1" viewBox="0 0 36 36" fill="none">
            <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
            <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
          </svg>
        {/if}
        {#if playerState.searchQuery || playerState.youtubeResults.length > 0}
          <button
            type="button"
            onclick={() => {
              if (debounceTimer) clearTimeout(debounceTimer);
              ++searchSeq;
              currentPage = 1;
              hasMoreResults = true;
              playerState.clearSearch();
            }}
            class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
            aria-label="Clear search"
            title="Clear search"
          >
            <span class="material-symbols-rounded text-base">close</span>
          </button>
        {/if}
      </div>
    </form>

    {#if playerState.searchQuery.trim().length > 0 || playerState.youtubeResults.length > 0}
      <!-- ArchiveTune Search Filter Chips -->
      <div class="flex items-center gap-1.5 overflow-x-auto scrollbar-none">
        {#each [
          { id: 'all', label: 'All' },
          { id: 'songs', label: 'Songs' },
          { id: 'videos', label: 'Videos' },
          { id: 'albums', label: 'Albums' },
          { id: 'artists', label: 'Artists' },
        ] as filter}
          <button
            type="button"
            onclick={() => switchFilter(filter.id as typeof playerState.searchFilter)}
            class="px-4 py-1.5 rounded-xl text-xs font-medium transition-colors border cursor-pointer shrink-0 {playerState.searchFilter === filter.id
              ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] border-transparent font-semibold shadow-sm'
              : 'bg-white/[0.05] border-white/[0.06] text-white/65 hover:text-white hover:bg-white/10'}"
          >
            {filter.label}
          </button>
        {/each}
      </div>
    {:else}
      <div class="flex items-center gap-2 overflow-x-auto scrollbar-none pb-0.5">
        {#each quickTags as tag}
          <button
            type="button"
            onclick={() => runQuickSearch(tag.label, tag.query)}
            class="px-4 py-1.5 rounded-xl text-xs font-medium bg-white/[0.05] border border-white/[0.06] hover:bg-white/[0.1] text-white/75 hover:text-white whitespace-nowrap transition-colors cursor-pointer"
          >
            {tag.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Moods & Genres Browse Grid (When Search Input is Empty) -->
  {#if !playerState.searchQuery.trim() && playerState.youtubeResults.length === 0 && !playerState.isSearching}
    <div class="space-y-3 pt-1">
      <h2 class="font-display font-bold text-base text-white/90 tracking-tight">Moods & genres</h2>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-3.5">
        {#each browseCategories as cat}
          <button
            type="button"
            onclick={() => runQuickSearch(cat.label, cat.query)}
            class="h-24 p-4 rounded-2xl bg-gradient-to-br {cat.color} border border-white/[0.08] hover:border-white/25 text-left flex flex-col justify-between transition-[transform,border-color] duration-200 hover:-translate-y-0.5 cursor-pointer group"
          >
            <span class="text-[10px] font-semibold uppercase tracking-wider text-white/55 truncate w-full">
              {cat.sub}
            </span>
            <div class="flex items-end justify-between w-full gap-2">
              <span class="font-display font-bold text-sm sm:text-base text-white truncate">
                {cat.label}
              </span>
              <span class="material-symbols-rounded text-white/45 group-hover:text-white text-lg transition-colors shrink-0">
                north_east
              </span>
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Shimmer Skeletons While Searching -->
  {#if playerState.isSearching && playerState.youtubeResults.length === 0}
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 pt-2">
      <div class="lg:col-span-5 h-52 rounded-3xl bg-white/[0.04] animate-pulse"></div>
      <div class="lg:col-span-7 space-y-2.5">
        {#each Array(6) as _}
          <div class="h-14 rounded-2xl bg-white/[0.04] animate-pulse"></div>
        {/each}
      </div>
    </div>
  {:else if playerState.searchError && playerState.youtubeResults.length === 0}
    <div role="alert" class="py-16 text-center rounded-3xl bg-white/[0.02] border border-rose-500/20 space-y-2">
      <span class="material-symbols-rounded text-4xl text-rose-400">cloud_off</span>
      <p class="text-sm font-medium text-white/80">Search failed</p>
      <p class="text-xs text-white/45 max-w-md mx-auto">{playerState.searchError}</p>
    </div>
  {:else if playerState.searchQuery.trim() && !playerState.isSearching && playerState.youtubeResults.length === 0}
    <div class="py-16 text-center rounded-3xl bg-white/[0.02] border border-white/[0.05] space-y-2">
      <span class="material-symbols-rounded text-4xl text-white/25">search_off</span>
      <p class="text-sm font-medium text-white/70">No results for "{playerState.searchQuery}"</p>
      <p class="text-xs text-white/40">Try another song title, artist name, or mood tag</p>
    </div>
  {:else if playerState.youtubeResults.length > 0}
    <div class="space-y-6 transition-opacity {playerState.isSearching ? 'opacity-60' : 'opacity-100'}">
    {#if playerState.searchFilter === 'all'}
      <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        <!-- Left Column: Top Result Hero Card -->
        {#if topResult}
          {@const topTrack = ytToTrack(topResult)}
          {@const isTopCurrent = playerState.currentTrack?.id === topTrack.id}
          {@const topThumb = getHighResArtwork(topResult.thumbnail, topResult.id)}
          <div class="lg:col-span-5 space-y-2.5">
            <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Top result</h3>
            <div
              role="button"
              tabindex="0"
              onclick={() => (isTopCurrent ? playerState.togglePlay() : playerState.playYouTube(topResult))}
              oncontextmenu={(e) => {
                e.preventDefault();
                playerState.openContextMenu(topTrack, e.clientX, e.clientY);
              }}
              onkeydown={(e) => {
                if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                  e.preventDefault();
                  isTopCurrent ? playerState.togglePlay() : playerState.playYouTube(topResult);
                }
              }}
              class="p-5 rounded-3xl m3-acrylic-card hover:bg-white/[0.07] border border-white/[0.08] transition-colors cursor-pointer group flex flex-col justify-between gap-5 shadow-lg"
            >
              <div class="flex items-center gap-4">
                <div class="relative w-24 h-24 rounded-2xl overflow-hidden shadow-lg shrink-0 bg-white/5">
                  {#if topThumb}
                    <img
                      src={topThumb}
                      alt={topResult.title}
                      onload={(e) => handleThumbnailLoad(e, topResult.id)}
                      onerror={(e) => handleThumbnailError(e, topResult.id)}
                      class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                    />
                  {:else}
                    <div class="w-full h-full flex items-center justify-center text-white/30">
                      <span class="material-symbols-rounded text-3xl">music_note</span>
                    </div>
                  {/if}
                  {#if isTopCurrent && (isBuffering || playerState.playback.is_playing)}
                    <div class="absolute inset-0 bg-black/45 flex items-center justify-center">
                      {#if isBuffering}
                        <svg class="w-6 h-6 archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                          <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                          <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
                        </svg>
                      {:else}
                        <div class="flex items-end gap-[3px] h-4">
                          <span class="w-[3px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                          <span class="w-[3px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                          <span class="w-[3px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>

                <div class="min-w-0 flex-1 overflow-hidden">
                  <h4 class="font-display font-bold text-base sm:text-lg {isTopCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate block w-full">
                    {topResult.title}
                  </h4>
                  <div class="text-xs text-white/60 mt-1 flex items-center gap-1.5 min-w-0">
                    <span class="shrink-0">Song</span>
                    {#if topTrack.artist}
                      <span class="shrink-0">•</span>
                      <div class="min-w-0 truncate">
                        <ArtistLinks
                          artist={topTrack.artist}
                          class="text-xs text-white/75"
                          buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                        />
                      </div>
                    {/if}
                    {#if topResult.duration > 0}
                      <span class="shrink-0">•</span>
                      <span class="shrink-0 font-mono">{formatDuration(topResult.duration)}</span>
                    {/if}
                  </div>
                </div>
              </div>

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    if (isTopCurrent) {
                      playerState.togglePlay();
                    } else {
                      playerState.playYouTube(topResult);
                    }
                  }}
                  class="px-5 py-2 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] font-semibold text-xs flex items-center gap-1.5 hover:scale-105 active:scale-95 transition-transform cursor-pointer"
                >
                  {#if isTopCurrent && isBuffering}
                    <svg class="w-4 h-4 archivetune-spinner-svg text-current" viewBox="0 0 36 36" fill="none">
                      <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.25" stroke-width="3.4" />
                      <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.4" stroke-linecap="round" />
                    </svg>
                    <span>Loading</span>
                  {:else}
                    <span class="material-symbols-rounded text-base filled">
                      {isTopCurrent && playerState.playback.is_playing ? 'pause' : 'play_arrow'}
                    </span>
                    <span>{isTopCurrent && playerState.playback.is_playing ? 'Pause' : 'Play'}</span>
                  {/if}
                </button>

                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.addToQueue(topTrack);
                  }}
                  class="px-3.5 py-2 rounded-full bg-white/[0.07] hover:bg-white/[0.14] text-white text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
                >
                  <span class="material-symbols-rounded text-base">playlist_add</span>
                  <span>Queue</span>
                </button>

                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.openContextMenu(topTrack, e.clientX, e.clientY);
                  }}
                  class="ml-auto w-8 h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.14] text-white/65 hover:text-white flex items-center justify-center transition-colors cursor-pointer"
                  aria-label="More options"
                  title="More options"
                >
                  <span class="material-symbols-rounded text-base">more_vert</span>
                </button>
              </div>
            </div>
          </div>
        {/if}

        <!-- Right Column: Top Songs Stack -->
        <div class="lg:col-span-7 space-y-2.5">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Songs</h3>
          <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
            {#each playerState.youtubeResults.slice(1, 5) as yt (yt.id)}
              {@const track = ytToTrack(yt)}
              {@const isCurrent = playerState.currentTrack?.id === track.id}
              {@const thumb = getHighResArtwork(yt.thumbnail, yt.id)}
              <div
                role="button"
                tabindex="0"
                onclick={() => playerState.playYouTube(yt)}
                oncontextmenu={(e) => {
                  e.preventDefault();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                onkeydown={(e) => {
                  if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                    e.preventDefault();
                    playerState.playYouTube(yt);
                  }
                }}
                class="h-14 flex items-center justify-between px-3 transition-colors cursor-pointer group overflow-hidden {isCurrent ? '' : 'hover:bg-white/[0.05]'}"
                style={isCurrent ? 'background-color: color-mix(in srgb, var(--md-sys-primary-container) 35%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-primary) 35%, transparent);' : ''}
              >
                <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                  <div class="relative w-10 h-10 rounded-xl overflow-hidden bg-white/5 shrink-0">
                    {#if thumb}
                      <img
                        loading="lazy"
                        decoding="async"
                        src={thumb}
                        alt={yt.title}
                        onload={(e) => handleThumbnailLoad(e, yt.id)}
                        onerror={(e) => handleThumbnailError(e, yt.id)}
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
                      {yt.title}
                    </p>
                    <ArtistLinks artist={track.artist} class="text-[11px] text-white/55 mt-0.5" buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left" />
                  </div>
                </div>

                <div class="flex items-center gap-1.5 shrink-0">
                  {#if yt.duration > 0}
                    <span class="text-[11px] font-mono text-white/40 group-hover:hidden group-focus-within:hidden">{formatDuration(yt.duration)}</span>
                  {/if}
                  <div class="hidden group-hover:flex group-focus-within:flex items-center gap-1">
                    <button
                      type="button"
                      onclick={(e) => {
                        e.stopPropagation();
                        playerState.addToQueue(track);
                      }}
                      class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
                      aria-label="Add to Queue"
                      title="Add to Queue"
                    >
                      <span class="material-symbols-rounded text-base">playlist_add</span>
                    </button>
                    <button
                      type="button"
                      onclick={(e) => {
                        e.stopPropagation();
                        playerState.openContextMenu(track, e.clientX, e.clientY);
                      }}
                      class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
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
        </div>
      </div>
    {/if}

    <!-- Artists Row -->
    {#if playerState.searchFilter === 'all' || playerState.searchFilter === 'artists'}
      {#if derivedArtists.length > 0}
        <div class="space-y-2.5 pt-2">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Artists</h3>
          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
            {#each derivedArtists as artist}
              {@const artistThumb = getHighResArtwork(artist.avatar, artist.id)}
              <button
                type="button"
                onclick={() => playerState.openArtist(artist.name)}
                class="p-3 rounded-2xl bg-white/[0.02] hover:bg-white/[0.06] border border-white/[0.05] flex flex-col items-center text-center gap-2.5 transition-colors cursor-pointer group"
              >
                <div class="w-16 h-16 rounded-full overflow-hidden bg-white/5 shadow-md flex items-center justify-center">
                  {#if artistThumb}
                    <img
                      loading="lazy"
                      decoding="async"
                      src={artistThumb}
                      alt={artist.name}
                      onload={(e) => handleThumbnailLoad(e, artist.id)}
                      onerror={(e) => handleThumbnailError(e, artist.id)}
                      class="w-full h-full object-cover group-hover:scale-105 transition-transform"
                    />
                  {:else}
                    <span class="material-symbols-rounded text-2xl text-white/30">person</span>
                  {/if}
                </div>
                <span class="text-xs font-semibold text-white truncate w-full">{artist.name}</span>
              </button>
            {/each}
          </div>
        </div>
      {:else if playerState.searchFilter === 'artists'}
        <div class="py-12 text-center rounded-2xl bg-white/[0.02] border border-white/[0.05] space-y-1.5">
          <span class="material-symbols-rounded text-3xl text-white/25">person_off</span>
          <p class="text-xs font-medium text-white/55">No artists found in these results</p>
        </div>
      {/if}
    {/if}

    <!-- Full Songs / Videos / Albums List + Load More -->
    {#if playerState.searchFilter !== 'artists'}
      <div class="space-y-3 pt-2">
        <div class="flex items-center justify-between px-1">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45">
            {playerState.searchFilter === 'songs'
              ? `Songs (${playerState.youtubeResults.length})`
              : playerState.searchFilter === 'videos'
                ? `Music Videos (${playerState.youtubeResults.length})`
                : playerState.searchFilter === 'albums'
                  ? `Albums & Singles (${playerState.youtubeResults.length})`
                  : `More songs (${playerState.youtubeResults.length})`}
          </h3>
        </div>
        <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
          {#each playerState.searchFilter === 'all' ? playerState.youtubeResults.slice(5) : playerState.youtubeResults as yt (yt.id)}
            {@const track = ytToTrack(yt)}
            {@const isCurrent = playerState.currentTrack?.id === track.id}
            {@const thumb = getHighResArtwork(yt.thumbnail, yt.id)}
            <div
              role="button"
              tabindex="0"
              onclick={() => playerState.playYouTube(yt)}
              oncontextmenu={(e) => {
                e.preventDefault();
                playerState.openContextMenu(track, e.clientX, e.clientY);
              }}
              onkeydown={(e) => {
                if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                  e.preventDefault();
                  playerState.playYouTube(yt);
                }
              }}
              class="h-14 flex items-center justify-between px-3 transition-colors cursor-pointer group overflow-hidden {isCurrent ? '' : 'hover:bg-white/[0.05]'}"
              style={isCurrent ? 'background-color: color-mix(in srgb, var(--md-sys-primary-container) 35%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-primary) 35%, transparent);' : ''}
            >
              <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                <div class="relative w-10 h-10 rounded-xl overflow-hidden bg-white/5 shrink-0">
                  {#if thumb}
                    <img
                      loading="lazy"
                      decoding="async"
                      src={thumb}
                      alt={yt.title}
                      onload={(e) => handleThumbnailLoad(e, yt.id)}
                      onerror={(e) => handleThumbnailError(e, yt.id)}
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
                    {yt.title}
                  </p>
                  <ArtistLinks artist={track.artist} class="text-[11px] text-white/55 mt-0.5" buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left" />
                </div>
              </div>

              <div class="flex items-center gap-1.5 shrink-0">
                {#if yt.duration > 0}
                  <span class="text-[11px] font-mono text-white/40 group-hover:hidden group-focus-within:hidden">{formatDuration(yt.duration)}</span>
                {/if}
                <div class="hidden group-hover:flex group-focus-within:flex items-center gap-1">
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      playerState.addToQueue(track);
                    }}
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
                    aria-label="Add to Queue"
                    title="Add to Queue"
                  >
                    <span class="material-symbols-rounded text-base">playlist_add</span>
                  </button>
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      playerState.openContextMenu(track, e.clientX, e.clientY);
                    }}
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
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

        {#if hasMoreResults}
          <div class="flex justify-center pt-2">
            <button
              type="button"
              onclick={loadMoreResults}
              disabled={isLoadingMore}
              class="px-5 py-2.5 rounded-full bg-white/[0.06] hover:bg-white/[0.12] border border-white/[0.08] text-xs font-semibold text-white/85 flex items-center gap-2 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {#if isLoadingMore}
                <svg class="w-4 h-4 archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                  <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                  <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
                </svg>
                <span>Loading more songs...</span>
              {:else}
                <span class="material-symbols-rounded text-base">expand_more</span>
                <span>Show more results</span>
              {/if}
            </button>
          </div>
        {/if}
      </div>
    {/if}
    </div>
  {/if}
</div>
