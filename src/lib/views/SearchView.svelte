<script lang="ts">
  import { onDestroy } from 'svelte';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist, type Track, type YouTubeTrack } from '../types';

  let searchQuery = $state('');
  let activeFilter = $state<'all' | 'songs' | 'artists' | 'library'>('all');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let searchSeq = 0;

  // Instant 0ms local library matches
  let localMatches = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return [];
    return playerState.library
      .filter(
        (t) =>
          t.title.toLowerCase().includes(q) ||
          t.artist.toLowerCase().includes(q) ||
          (t.album && t.album.toLowerCase().includes(q))
      )
      .slice(0, 8);
  });

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
      if (seq === searchSeq && activeFilter !== 'library') {
        await playerState.searchYouTube(q);
      }
    }, 260);
  }

  onDestroy(() => {
    if (debounceTimer) clearTimeout(debounceTimer);
  });

  function handleSearch(e: Event) {
    e.preventDefault();
    if (debounceTimer) clearTimeout(debounceTimer);
    if (searchQuery.trim() && activeFilter !== 'library') {
      playerState.searchYouTube(searchQuery.trim());
    }
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
      format: 'Opus',
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

  const browseCategories = [
    { label: 'Top Hits', query: 'global top hits songs', color: 'from-rose-500/25 to-orange-500/10' },
    { label: 'Chill & Lo-Fi', query: 'lofi chill beats study', color: 'from-indigo-500/25 to-purple-500/10' },
    { label: 'Hip-Hop', query: 'hip hop rap essentials', color: 'from-amber-500/25 to-yellow-500/10' },
    { label: 'Electronic', query: 'electronic house melodic', color: 'from-cyan-500/25 to-blue-500/10' },
    { label: 'Pop', query: 'pop hits essentials', color: 'from-pink-500/25 to-rose-500/10' },
    { label: 'Rock & Alt', query: 'alternative rock classics', color: 'from-emerald-500/25 to-teal-500/10' },
    { label: 'R&B & Soul', query: 'rnb soul smooth hits', color: 'from-violet-500/25 to-fuchsia-500/10' },
    { label: 'Workout', query: 'workout gym energy songs', color: 'from-red-500/25 to-rose-500/10' },
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

<div class="p-6 max-w-6xl mx-auto space-y-6 pb-28 select-none">
  <!-- Search Input Bar -->
  <div class="space-y-3">
    <form onsubmit={handleSearch} class="relative max-w-2xl">
      <span class="material-symbols-rounded absolute left-4 top-1/2 -translate-y-1/2 text-white/40 text-xl pointer-events-none">
        search
      </span>
      <input
        type="text"
        bind:value={searchQuery}
        oninput={onSearchInput}
        placeholder="Songs, artists, or albums"
        class="w-full pl-11 pr-10 py-3 rounded-2xl bg-white/[0.06] border border-white/[0.08] text-white placeholder-white/35 focus:outline-none focus:border-white/25 focus:bg-white/[0.09] transition-all text-sm"
      />
      {#if searchQuery}
        <button
          type="button"
          onclick={() => {
            searchQuery = '';
            playerState.youtubeResults = [];
          }}
          class="absolute right-3 top-1/2 -translate-y-1/2 w-7 h-7 rounded-full flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
          aria-label="Clear search"
        >
          <span class="material-symbols-rounded text-base">close</span>
        </button>
      {/if}
    </form>

    {#if searchQuery.trim().length > 0}
      <div class="flex items-center gap-1.5">
        {#each [
          { id: 'all', label: 'All' },
          { id: 'songs', label: 'Songs' },
          { id: 'artists', label: 'Artists' },
          { id: 'library', label: `Library (${localMatches.length})` },
        ] as filter}
          <button
            type="button"
            onclick={() => (activeFilter = filter.id as any)}
            class="px-3.5 py-1.5 rounded-full text-xs font-medium transition-all cursor-pointer {activeFilter === filter.id
              ? 'bg-white text-black font-semibold'
              : 'bg-white/[0.05] text-white/65 hover:text-white hover:bg-white/10'}"
          >
            {filter.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Browse Categories (When Search Input is Empty) -->
  {#if !searchQuery.trim() && playerState.youtubeResults.length === 0 && !playerState.isSearching}
    <div class="space-y-3 pt-1">
      <h3 class="text-sm font-semibold text-white/80">Browse Categories</h3>
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {#each browseCategories as cat}
          <button
            type="button"
            onclick={() => {
              searchQuery = cat.label;
              playerState.searchYouTube(cat.query);
            }}
            class="h-20 p-4 rounded-2xl bg-gradient-to-br {cat.color} border border-white/[0.07] hover:border-white/20 text-left flex items-end justify-between transition-all cursor-pointer group"
          >
            <span class="font-display font-bold text-sm text-white group-hover:translate-x-0.5 transition-transform">
              {cat.label}
            </span>
            <span class="material-symbols-rounded text-white/40 group-hover:text-white/80 text-lg transition-colors">
              north_east
            </span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Instant Local Library Matches -->
  {#if localMatches.length > 0 && (activeFilter === 'all' || activeFilter === 'library')}
    <section class="space-y-2">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">In Your Library</h3>
      <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
        {#each localMatches as track (track.id)}
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
            class="flex items-center justify-between px-3 py-2.5 hover:bg-white/[0.05] transition-colors cursor-pointer group"
          >
            <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
              <div class="relative w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
                {#if track.cover_art}
                  <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover" />
                {:else}
                  <div class="w-full h-full flex items-center justify-center text-white/30">
                    <span class="material-symbols-rounded text-base">music_note</span>
                  </div>
                {/if}
                <div class="absolute inset-0 bg-black/45 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                  <span class="material-symbols-rounded text-white text-lg filled">
                    {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                  </span>
                </div>
              </div>

              <div class="min-w-0 flex-1">
                <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                  {track.title}
                </p>
                <p class="text-[11px] text-white/55 truncate mt-0.5">{track.artist}</p>
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              {#if track.duration > 0}
                <span class="text-[11px] font-mono text-white/40">{formatDuration(track.duration)}</span>
              {/if}
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
                aria-label="More options"
              >
                <span class="material-symbols-rounded text-base">more_horiz</span>
              </button>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Online Loading Indicator -->
  {#if playerState.isSearching && playerState.youtubeResults.length === 0}
    <div class="flex items-center justify-center py-14 text-white/50">
      <svg class="w-6 h-6 animate-spin text-[var(--md-sys-primary)]" viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <circle cx="12" cy="12" r="9" stroke-width="2.5" class="opacity-25" />
        <path d="M21 12a9 9 0 00-9-9" stroke-width="2.5" stroke-linecap="round" />
      </svg>
    </div>
  {:else if playerState.youtubeResults.length > 0 && activeFilter !== 'library'}
    {#if activeFilter === 'all'}
      <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        <!-- Top Result Card (Clean Apple Music / ArchiveTune style) -->
        {#if topResult}
          {@const topTrack = ytToTrack(topResult)}
          <div class="lg:col-span-5 space-y-2">
            <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Top Result</h3>
            <div
              role="button"
              tabindex="0"
              onmouseenter={() => playerState.prefetchStream(topResult.stream_url, topResult.title, topResult.artist)}
              onclick={() => playerState.playYouTube(topResult)}
              oncontextmenu={(e) => {
                e.preventDefault();
                playerState.openContextMenu(topTrack, e.clientX, e.clientY);
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter') playerState.playYouTube(topResult);
              }}
              class="p-5 rounded-3xl bg-white/[0.03] hover:bg-white/[0.06] border border-white/[0.07] transition-all cursor-pointer group flex flex-col justify-between gap-4"
            >
              <div class="flex items-center gap-4">
                <img
                  src={topResult.thumbnail}
                  alt={topResult.title}
                  class="w-20 h-20 rounded-2xl object-cover shadow-lg shrink-0"
                />
                <div class="min-w-0 flex-1">
                  <h4 class="font-display font-bold text-base text-white truncate">
                    {topResult.title}
                  </h4>
                  <p class="text-xs text-white/60 truncate mt-1">
                    {sanitizeArtist(topResult.artist)}
                    {#if topResult.duration > 0}
                      <span> • {formatDuration(topResult.duration)}</span>
                    {/if}
                  </p>
                </div>
              </div>

              <div class="flex items-center gap-2 pt-1">
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.playYouTube(topResult);
                  }}
                  class="px-4 py-2 rounded-full bg-white text-black font-semibold text-xs flex items-center gap-1.5 hover:scale-105 transition-transform cursor-pointer"
                >
                  <span class="material-symbols-rounded text-base filled">play_arrow</span>
                  <span>Play</span>
                </button>
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.addToQueue(topTrack);
                  }}
                  class="px-3.5 py-2 rounded-full bg-white/[0.07] hover:bg-white/[0.14] text-white text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
                >
                  <span class="material-symbols-rounded text-base">queue_music</span>
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

        <!-- Songs List -->
        <div class="lg:col-span-7 space-y-2">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">Songs</h3>
          <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
            {#each playerState.youtubeResults.slice(1, 7) as yt (yt.id)}
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
                class="flex items-center justify-between px-3 py-2.5 hover:bg-white/[0.05] transition-colors cursor-pointer group"
              >
                <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                  <div class="relative w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
                    <img loading="lazy" decoding="async" src={yt.thumbnail} alt={yt.title} class="w-full h-full object-cover" />
                    <div class="absolute inset-0 bg-black/45 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                      <span class="material-symbols-rounded text-white text-lg filled">
                        {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                      </span>
                    </div>
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                      {yt.title}
                    </p>
                    <p class="text-[11px] text-white/55 truncate mt-0.5">{sanitizeArtist(yt.artist)}</p>
                  </div>
                </div>

                <div class="flex items-center gap-2 shrink-0">
                  {#if yt.duration > 0}
                    <span class="text-[11px] font-mono text-white/40">{formatDuration(yt.duration)}</span>
                  {/if}
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      playerState.addToQueue(track);
                    }}
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
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
                    class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
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

    <!-- Full Songs List -->
    {#if activeFilter === 'all' || activeFilter === 'songs'}
      <div class="space-y-2 pt-2">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-white/45 px-1">
          {activeFilter === 'songs' ? 'All Songs' : 'More Results'}
        </h3>
        <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
          {#each activeFilter === 'songs' ? playerState.youtubeResults : playerState.youtubeResults.slice(7) as yt (yt.id)}
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
              class="flex items-center justify-between px-3 py-2.5 hover:bg-white/[0.05] transition-colors cursor-pointer group"
            >
              <div class="flex items-center gap-3 min-w-0 flex-1 pr-3">
                <div class="relative w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
                  <img loading="lazy" decoding="async" src={yt.thumbnail} alt={yt.title} class="w-full h-full object-cover" />
                  <div class="absolute inset-0 bg-black/45 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                    <span class="material-symbols-rounded text-white text-lg filled">
                      {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                    </span>
                  </div>
                </div>
                <div class="min-w-0 flex-1">
                  <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                    {yt.title}
                  </p>
                  <p class="text-[11px] text-white/55 truncate mt-0.5">{sanitizeArtist(yt.artist)}</p>
                </div>
              </div>

              <div class="flex items-center gap-2 shrink-0">
                {#if yt.duration > 0}
                  <span class="text-[11px] font-mono text-white/40">{formatDuration(yt.duration)}</span>
                {/if}
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    playerState.addToQueue(track);
                  }}
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
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
                  class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
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

