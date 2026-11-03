<script lang="ts">
  import { onDestroy } from 'svelte';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist, type Track, type YouTubeTrack } from '../types';

  let searchQuery = $state('');
  let activeFilter = $state<'all' | 'songs' | 'artists'>('all');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let searchSeq = 0;

  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  function onSearchInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    const q = searchQuery.trim();
    if (!q) {
      playerState.youtubeResults = [];
      playerState.isSearching = false;
      return;
    }
    const seq = ++searchSeq;
    debounceTimer = setTimeout(async () => {
      if (seq === searchSeq) {
        await playerState.searchYouTube(q);
      }
    }, 240);
  }

  onDestroy(() => {
    if (debounceTimer) clearTimeout(debounceTimer);
  });

  function handleSearch(e: Event) {
    e.preventDefault();
    if (debounceTimer) clearTimeout(debounceTimer);
    if (searchQuery.trim()) {
      playerState.searchYouTube(searchQuery.trim());
    }
  }

  function runQuickSearch(label: string, query: string) {
    searchQuery = label;
    playerState.searchYouTube(query);
  }

  function ytToTrack(yt: YouTubeTrack): Track {
    return {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: sanitizeArtist(yt.artist),
      album: 'YouTube Music',
      duration: yt.duration || 0,
      file_path: yt.stream_url,
      cover_art: yt.thumbnail,
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
    { label: 'New Releases', query: 'new music releases hits' },
    { label: 'Chill', query: 'chill lofi acoustic vibes' },
    { label: 'Phonk', query: 'drift phonk gym hits' },
    { label: 'Synthwave', query: 'synthwave retrowave night drive' },
    { label: 'Pop', query: 'pop chart hits' },
    { label: 'Hip-Hop', query: 'hip hop rap essentials' },
    { label: 'Indie', query: 'indie alternative essentials' },
  ];

  const browseCategories = [
    { label: 'Top Charts', sub: 'Global 100', query: 'global top hits songs', color: 'from-rose-500/30 to-orange-600/15' },
    { label: 'Chill & Lo-Fi', sub: 'Study & Relax', query: 'lofi chill beats study', color: 'from-indigo-500/30 to-purple-600/15' },
    { label: 'Hip-Hop', sub: 'Rap & Trap', query: 'hip hop rap essentials', color: 'from-amber-500/30 to-yellow-600/15' },
    { label: 'Electronic', sub: 'House & Techno', query: 'electronic house melodic', color: 'from-cyan-500/30 to-blue-600/15' },
    { label: 'Pop Essentials', sub: 'Hitmakers', query: 'pop hits essentials', color: 'from-pink-500/30 to-rose-600/15' },
    { label: 'Rock & Alt', sub: 'Anthems', query: 'alternative rock classics', color: 'from-emerald-500/30 to-teal-600/15' },
    { label: 'R&B & Soul', sub: 'Late Night', query: 'rnb soul smooth hits', color: 'from-violet-500/30 to-fuchsia-600/15' },
    { label: 'Workout', sub: 'High Energy', query: 'workout gym energy songs', color: 'from-red-500/30 to-rose-600/15' },
  ];

  let topResult = $derived(
    playerState.youtubeResults.length > 0 ? playerState.youtubeResults[0] : null
  );

  let derivedArtists = $derived.by(() => {
    const seen = new Set<string>();
    const list: { name: string; avatar: string }[] = [];
    for (const yt of playerState.youtubeResults) {
      const cleanName = sanitizeArtist(yt.artist);
      if (cleanName && !seen.has(cleanName.toLowerCase())) {
        seen.add(cleanName.toLowerCase());
        list.push({ name: cleanName, avatar: yt.thumbnail });
      }
      if (list.length >= 6) break;
    }
    return list;
  });
</script>

<div class="px-6 py-5 max-w-6xl mx-auto space-y-6 pb-28 select-none">
  <!-- Search Header & Input Bar with Live SVG Circular Progress Ring -->
  <div class="space-y-3.5">
    <h1 class="font-display font-bold text-2xl sm:text-3xl text-white tracking-tight">Search</h1>

    <form onsubmit={handleSearch} class="relative max-w-2xl">
      <span class="material-symbols-rounded absolute left-4 top-1/2 -translate-y-1/2 text-white/45 text-xl pointer-events-none">
        search
      </span>
      <input
        type="text"
        bind:value={searchQuery}
        oninput={onSearchInput}
        placeholder="Search Apple Music & YouTube Music catalog..."
        class="w-full pl-11 pr-12 py-3 rounded-2xl bg-white/[0.06] border border-white/[0.08] text-white placeholder-white/35 focus:outline-none focus:border-white/25 focus:bg-white/[0.09] transition-all text-sm"
      />

      <div class="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-1">
        {#if playerState.isSearching}
          <svg class="w-5 h-5 animate-spin text-[var(--md-sys-primary)] mr-1" viewBox="0 0 36 36" fill="none">
            <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
            <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.2" stroke-dasharray="52 100" stroke-linecap="round" />
          </svg>
        {/if}
        {#if searchQuery}
          <button
            type="button"
            onclick={() => {
              searchQuery = '';
              playerState.youtubeResults = [];
            }}
            class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
            aria-label="Clear search"
          >
            <span class="material-symbols-rounded text-base">close</span>
          </button>
        {/if}
      </div>
    </form>

    {#if searchQuery.trim().length > 0 || playerState.youtubeResults.length > 0}
      <div class="flex items-center gap-1.5">
        {#each [
          { id: 'all', label: 'Top Results' },
          { id: 'songs', label: 'Songs' },
          { id: 'artists', label: 'Artists' },
        ] as filter}
          <button
            type="button"
            onclick={() => (activeFilter = filter.id as any)}
            class="px-4 py-1.5 rounded-full text-xs font-medium transition-all cursor-pointer {activeFilter ===
            filter.id
              ? 'bg-white text-black font-semibold shadow-sm'
              : 'bg-white/[0.05] text-white/65 hover:text-white hover:bg-white/10'}"
          >
            {filter.label}
          </button>
        {/each}
      </div>
    {:else}
      <!-- Quick Suggestion Chips -->
      <div class="flex items-center gap-2 overflow-x-auto scrollbar-none pb-0.5">
        {#each quickTags as tag}
          <button
            type="button"
            onclick={() => runQuickSearch(tag.label, tag.query)}
            class="px-3.5 py-1.5 rounded-full text-xs font-medium bg-white/[0.05] hover:bg-white/[0.1] text-white/75 hover:text-white whitespace-nowrap transition-colors cursor-pointer"
          >
            {tag.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Browse Categories (When Search Input is Empty) -->
  {#if !searchQuery.trim() && playerState.youtubeResults.length === 0 && !playerState.isSearching}
    <div class="space-y-3 pt-1">
      <h2 class="font-display font-bold text-base text-white/90">Browse Categories</h2>
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3.5">
        {#each browseCategories as cat}
          <button
            type="button"
            onclick={() => runQuickSearch(cat.label, cat.query)}
            class="h-24 p-4 rounded-2xl bg-gradient-to-br {cat.color} border border-white/[0.08] hover:border-white/25 text-left flex flex-col justify-between transition-all duration-200 hover:-translate-y-0.5 cursor-pointer group"
          >
            <span class="text-[10px] font-semibold uppercase tracking-wider text-white/55">
              {cat.sub}
            </span>
            <div class="flex items-end justify-between w-full">
              <span class="font-display font-bold text-sm sm:text-base text-white">
                {cat.label}
              </span>
              <span class="material-symbols-rounded text-white/45 group-hover:text-white text-lg transition-colors">
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
        {#each Array(4) as _}
          <div class="h-12 rounded-2xl bg-white/[0.04] animate-pulse"></div>
        {/each}
      </div>
    </div>
  {:else if playerState.youtubeResults.length > 0}
    {#if activeFilter === 'all'}
      <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        <!-- Left Column: Apple Music / ArchiveTune "Top Result" Hero Card -->
        {#if topResult}
          {@const topTrack = ytToTrack(topResult)}
          {@const isTopCurrent = playerState.currentTrack?.id === topTrack.id}
          <div class="lg:col-span-5 space-y-2.5">
            <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Top Result</h3>
            <div
              role="button"
              tabindex="0"
              onmouseenter={() =>
                playerState.prefetchStream(topResult.stream_url, topResult.title, topResult.artist)}
              onclick={() => playerState.playYouTube(topResult)}
              oncontextmenu={(e) => {
                e.preventDefault();
                playerState.openContextMenu(topTrack, e.clientX, e.clientY);
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter') playerState.playYouTube(topResult);
              }}
              class="p-5 rounded-3xl bg-white/[0.04] hover:bg-white/[0.07] border border-white/[0.08] transition-all cursor-pointer group flex flex-col justify-between gap-5 shadow-xl"
            >
              <div class="flex items-center gap-4">
                <div class="relative w-24 h-24 rounded-2xl overflow-hidden shadow-lg shrink-0 bg-white/5">
                  <img
                    src={topResult.thumbnail}
                    alt={topResult.title}
                    class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                  />
                  {#if isTopCurrent && (isBuffering || playerState.playback.is_playing)}
                    <div class="absolute inset-0 bg-black/45 flex items-center justify-center">
                      {#if isBuffering}
                        <svg class="w-7 h-7 animate-spin text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                          <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                          <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.2" stroke-dasharray="52 100" stroke-linecap="round" />
                        </svg>
                      {:else}
                        <div class="flex items-end gap-[3px] h-4">
                          <span class="w-[3px] bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                          <span class="w-[3px] bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                          <span class="w-[3px] bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>

                <div class="min-w-0 flex-1">
                  <h4 class="font-display font-bold text-base sm:text-lg {isTopCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                    {topResult.title}
                  </h4>
                  <p class="text-xs text-white/60 truncate mt-1">
                    Song • {sanitizeArtist(topResult.artist)}
                    {#if topResult.duration > 0}
                      <span> • {formatDuration(topResult.duration)}</span>
                    {/if}
                  </p>
                </div>
              </div>

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.playYouTube(topResult);
                  }}
                  class="px-5 py-2 rounded-full bg-white text-black font-semibold text-xs flex items-center gap-1.5 hover:scale-105 active:scale-95 transition-transform cursor-pointer"
                >
                  {#if isTopCurrent && isBuffering}
                    <svg class="w-4 h-4 animate-spin text-black" viewBox="0 0 36 36" fill="none">
                      <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.25" stroke-width="3.4" />
                      <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.4" stroke-dasharray="52 100" stroke-linecap="round" />
                    </svg>
                    <span>Loading</span>
                  {:else}
                    <span class="material-symbols-rounded text-base filled">
                      {isTopCurrent && playerState.playback.is_playing ? 'pause' : 'play_arrow'}
                    </span>
                    <span>{isTopCurrent && playerState.playback.is_playing ? 'Playing' : 'Play'}</span>
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
                >
                  <span class="material-symbols-rounded text-base">more_horiz</span>
                </button>
              </div>
            </div>
          </div>
        {/if}

        <!-- Right Column: 4-Song Stack (Apple Music / YT Music Desktop Layout) -->
        <div class="lg:col-span-7 space-y-2.5">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Songs</h3>
          <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
            {#each playerState.youtubeResults.slice(1, 5) as yt (yt.id)}
              {@const track = ytToTrack(yt)}
              {@const isCurrent = playerState.currentTrack?.id === track.id}
              <div
                role="button"
                tabindex="0"
                onmouseenter={() => playerState.prefetchStream(yt.stream_url, yt.title, yt.artist)}
                onclick={() => playerState.playYouTube(yt)}
                oncontextmenu={(e) => {
                  e.preventDefault();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                onkeydown={(e) => {
                  if (e.key === 'Enter') playerState.playYouTube(yt);
                }}
                class="flex items-center justify-between px-3 py-2.5 {isCurrent ? 'bg-white/[0.08]' : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group"
              >
                <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                  <div class="relative w-11 h-11 rounded-xl overflow-hidden bg-white/5 shrink-0">
                    <img loading="lazy" decoding="async" src={yt.thumbnail} alt={yt.title} class="w-full h-full object-cover" />
                    <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                      {#if isCurrent && isBuffering}
                        <svg class="w-5 h-5 animate-spin text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                          <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                          <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.2" stroke-dasharray="52 100" stroke-linecap="round" />
                        </svg>
                      {:else if isCurrent && playerState.playback.is_playing}
                        <div class="flex items-end gap-[2.5px] h-3.5">
                          <span class="w-[3px] bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                          <span class="w-[3px] bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                          <span class="w-[3px] bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                        </div>
                      {:else}
                        <span class="material-symbols-rounded text-white text-lg filled">play_arrow</span>
                      {/if}
                    </div>
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                      {yt.title}
                    </p>
                    <p class="text-[11px] text-white/55 truncate mt-0.5">{sanitizeArtist(yt.artist)}</p>
                  </div>
                </div>

                <div class="flex items-center gap-1.5 shrink-0">
                  {#if yt.duration > 0}
                    <span class="text-[11px] font-mono text-white/40">{formatDuration(yt.duration)}</span>
                  {/if}
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      playerState.addToQueue(track);
                    }}
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
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
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
                    title="More options"
                  >
                    <span class="material-symbols-rounded text-base">more_horiz</span>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/if}

    <!-- Artists Row -->
    {#if (activeFilter === 'all' || activeFilter === 'artists') && derivedArtists.length > 0}
      <div class="space-y-2.5 pt-2">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Artists</h3>
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
          {#each derivedArtists as artist}
            <button
              type="button"
              onclick={() => playerState.openArtist(artist.name)}
              class="p-3 rounded-2xl bg-white/[0.02] hover:bg-white/[0.06] border border-white/[0.05] flex flex-col items-center text-center gap-2.5 transition-all cursor-pointer group"
            >
              <img
                loading="lazy"
                decoding="async"
                src={artist.avatar}
                alt={artist.name}
                class="w-16 h-16 rounded-full object-cover shadow-md group-hover:scale-105 transition-transform"
              />
              <span class="text-xs font-semibold text-white truncate w-full">{artist.name}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Full Songs Catalog List -->
    {#if activeFilter === 'all' || activeFilter === 'songs'}
      <div class="space-y-2.5 pt-2">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">
          {activeFilter === 'songs' ? 'All Songs' : 'More Songs'}
        </h3>
        <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
          {#each activeFilter === 'songs' ? playerState.youtubeResults : playerState.youtubeResults.slice(5) as yt (yt.id)}
            {@const track = ytToTrack(yt)}
            {@const isCurrent = playerState.currentTrack?.id === track.id}
            <div
              role="button"
              tabindex="0"
              onmouseenter={() => playerState.prefetchStream(yt.stream_url, yt.title, yt.artist)}
              onclick={() => playerState.playYouTube(yt)}
              oncontextmenu={(e) => {
                e.preventDefault();
                playerState.openContextMenu(track, e.clientX, e.clientY);
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter') playerState.playYouTube(yt);
              }}
              class="flex items-center justify-between px-3 py-2.5 {isCurrent ? 'bg-white/[0.08]' : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group"
            >
              <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                <div class="relative w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
                  <img loading="lazy" decoding="async" src={yt.thumbnail} alt={yt.title} class="w-full h-full object-cover" />
                  <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                    {#if isCurrent && isBuffering}
                      <svg class="w-4 h-4 animate-spin text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                        <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                        <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.2" stroke-dasharray="52 100" stroke-linecap="round" />
                      </svg>
                    {:else if isCurrent && playerState.playback.is_playing}
                      <div class="flex items-end gap-[2px] h-3">
                        <span class="w-[2.5px] bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                        <span class="w-[2.5px] bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                        <span class="w-[2.5px] bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                      </div>
                    {:else}
                      <span class="material-symbols-rounded text-white text-lg filled">play_arrow</span>
                    {/if}
                  </div>
                </div>
                <div class="min-w-0 flex-1">
                  <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                    {yt.title}
                  </p>
                  <p class="text-[11px] text-white/55 truncate mt-0.5">{sanitizeArtist(yt.artist)}</p>
                </div>
              </div>

              <div class="flex items-center gap-1.5 shrink-0">
                {#if yt.duration > 0}
                  <span class="text-[11px] font-mono text-white/40">{formatDuration(yt.duration)}</span>
                {/if}
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.addToQueue(track);
                  }}
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
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
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
                  title="More options"
                >
                  <span class="material-symbols-rounded text-base">more_horiz</span>
                </button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  @keyframes eqBar1 {
    0%, 100% { height: 35%; }
    50% { height: 100%; }
  }
  @keyframes eqBar2 {
    0%, 100% { height: 95%; }
    40% { height: 30%; }
  }
  @keyframes eqBar3 {
    0%, 100% { height: 50%; }
    65% { height: 95%; }
  }
  .animate-eq-1 {
    animation: eqBar1 0.75s ease-in-out infinite;
  }
  .animate-eq-2 {
    animation: eqBar2 0.65s ease-in-out infinite;
  }
  .animate-eq-3 {
    animation: eqBar3 0.8s ease-in-out infinite;
  }
</style>
