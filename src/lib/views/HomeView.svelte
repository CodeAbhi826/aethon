<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist, type Track, type YouTubeTrack } from '../types';

  const moods = [
    { label: 'All', query: '' },
    { label: 'Energize', query: 'upbeat high energy hits songs' },
    { label: 'Relax', query: 'chill relaxing acoustic lofi songs' },
    { label: 'Workout', query: 'workout gym phonk motivation hits' },
    { label: 'Focus', query: 'deep focus ambient study instrumental' },
    { label: 'Commute', query: 'driving feel good pop indie hits' },
    { label: 'Romance', query: 'romantic love songs acoustic hits' },
    { label: 'Party', query: 'party dance club electronic hits' },
  ];

  let activeMood = $state('All');
  let quickPickFeed = $state<Track[]>(playerState.homeQuickPickFeed);
  let chartTracks = $state<Track[]>(playerState.homeChartTracks);
  let artistRadioTracks = $state<Track[]>(playerState.homeArtistRadioTracks);
  let isLoadingFeed = $state(!playerState.homeFeedLoaded);
  let isLoadingCharts = $state(!playerState.homeFeedLoaded);

  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  let seedArtist = $derived.by(() => {
    const fromHistory = playerState.playHistoryTracks.find(
      (t) => t.artist && t.artist.toLowerCase() !== 'unknown' && t.artist.toLowerCase() !== 'youtube music'
    )?.artist;
    if (fromHistory) return sanitizeArtist(fromHistory).split(',')[0].trim();

    if (
      playerState.currentTrack?.artist &&
      playerState.currentTrack.artist.toLowerCase() !== 'unknown'
    ) {
      return sanitizeArtist(playerState.currentTrack.artist).split(',')[0].trim();
    }

    const fromLib = playerState.library.find(
      (t) => t.artist && t.artist.toLowerCase() !== 'unknown'
    )?.artist;
    if (fromLib) return sanitizeArtist(fromLib).split(',')[0].trim();

    return null;
  });

  function ytToTrack(yt: YouTubeTrack, albumLabel = 'YouTube Music'): Track {
    return {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: sanitizeArtist(yt.artist),
      album: albumLabel,
      duration: yt.duration || 0,
      file_path: yt.stream_url,
      cover_art: yt.thumbnail,
      format: 'Opus 256k',
      play_count: 0,
      favorite: false,
      date_added: Math.floor(Date.now() / 1000),
    };
  }

  // Apple Music "Top Picks for You" Hero Cards (4 large editorial feature cards)
  let heroShowcase = $derived.by(() => {
    const pool: { tag: string; track: Track }[] = [];
    const seen = new Set<string>();

    if (playerState.playHistoryTracks.length > 0) {
      const t = playerState.playHistoryTracks[0];
      seen.add(t.id);
      pool.push({ tag: 'Heavy Rotation', track: t });
    }
    for (const t of quickPickFeed) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        const tag =
          pool.length === 0
            ? 'Featured Pick'
            : pool.length === 1
              ? 'Made for You'
              : pool.length === 2
                ? 'New Discovery'
                : 'Trending Now';
        pool.push({ tag, track: t });
      }
      if (pool.length >= 4) break;
    }
    return pool;
  });

  // ArchiveTune / YT Music 12-Track "Quick Picks" Grid
  let quickPicks = $derived.by(() => {
    const seen = new Set<string>();
    const list: Track[] = [];
    const source =
      activeMood === 'All'
        ? [...playerState.playHistoryTracks.slice(0, 3), ...quickPickFeed, ...chartTracks]
        : quickPickFeed;

    for (const t of source) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        list.push(t);
      }
      if (list.length >= 12) break;
    }
    return list;
  });

  // "Listen Again" Horizontal Carousel
  let listenAgain = $derived.by(() => {
    const seen = new Set<string>();
    const list: Track[] = [];
    for (const t of [
      ...playerState.playHistoryTracks,
      ...playerState.library.slice(0, 6),
      ...quickPickFeed,
    ]) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        list.push(t);
      }
      if (list.length >= 10) break;
    }
    return list;
  });

  async function loadHomeFeeds(customMoodQuery?: string, forceRefresh = false) {
    if (!forceRefresh && !customMoodQuery && playerState.homeFeedLoaded && playerState.homeQuickPickFeed.length > 0) {
      quickPickFeed = playerState.homeQuickPickFeed;
      chartTracks = playerState.homeChartTracks;
      artistRadioTracks = playerState.homeArtistRadioTracks;
      isLoadingFeed = false;
      isLoadingCharts = false;
      return;
    }

    isLoadingFeed = true;
    try {
      const primaryQuery =
        customMoodQuery ||
        (seedArtist ? `${seedArtist} popular songs mix` : 'top hits popular songs');

      const [feedRes, chartRes] = await Promise.all([
        invoke<YouTubeTrack[]>('search_youtube', { query: primaryQuery, limit: 15 }).catch(() => []),
        chartTracks.length === 0
          ? invoke<YouTubeTrack[]>('search_youtube', {
              query: 'global top hits chart songs',
              limit: 10,
            }).catch(() => [])
          : Promise.resolve([]),
      ]);

      if (feedRes && feedRes.length > 0) {
        const tracks = feedRes.map((yt) => ytToTrack(yt, 'Quick Picks'));
        quickPickFeed = tracks;
        if (!customMoodQuery) {
          playerState.homeQuickPickFeed = tracks;
        }
      }

      if (chartRes && chartRes.length > 0) {
        const cTracks = chartRes.map((yt) => ytToTrack(yt, 'Top Charts'));
        chartTracks = cTracks;
        playerState.homeChartTracks = cTracks;
      }
      playerState.homeFeedLoaded = true;
    } finally {
      isLoadingFeed = false;
      isLoadingCharts = false;
    }
  }

  let lastSeedArtist = '';
  $effect(() => {
    const artist = seedArtist;
    if (artist && artist !== lastSeedArtist) {
      lastSeedArtist = artist;
      invoke<YouTubeTrack[]>('search_youtube', {
        query: `${artist} best songs`,
        limit: 8,
      })
        .then((res) => {
          if (res && res.length > 0) {
            const rTracks = res.map((yt) => ytToTrack(yt, `${artist} Station`));
            artistRadioTracks = rTracks;
            playerState.homeArtistRadioTracks = rTracks;
          }
        })
        .catch(() => {});
    }
  });

  function selectMood(m: { label: string; query: string }) {
    activeMood = m.label;
    playerState.currentMood = m.label;
    loadHomeFeeds(m.query || undefined);
  }

  function formatDuration(sec: number): string {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }

  onMount(() => {
    loadHomeFeeds();
  });
</script>

<div class="px-6 py-5 max-w-7xl mx-auto space-y-9 pb-28 select-none">
  <!-- Top Header + Mood Filter Bar (Apple Music / ArchiveTune style) -->
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="font-display font-bold text-2xl sm:text-3xl text-white tracking-tight">Home</h1>

      <div class="flex items-center gap-2">
        {#if quickPicks.length > 0}
          <button
            type="button"
            onclick={() => {
              const shuffled = [...quickPicks].sort(() => Math.random() - 0.5);
              playerState.queue = shuffled.slice(1);
              playerState.playTrack(shuffled[0]);
            }}
            class="px-3.5 py-1.5 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-white/85 text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <span class="material-symbols-rounded text-base">shuffle</span>
            <span>Shuffle All</span>
          </button>
        {/if}
        <button
          type="button"
          onclick={() => loadHomeFeeds(moods.find((m) => m.label === activeMood)?.query, true)}
          class="w-8 h-8 rounded-full bg-white/[0.06] hover:bg-white/[0.12] flex items-center justify-center text-white/65 hover:text-white transition-colors cursor-pointer"
          title="Refresh Home Feed"
        >
          <span class="material-symbols-rounded text-base {isLoadingFeed ? 'animate-spin' : ''}">
            refresh
          </span>
        </button>
      </div>
    </div>

    <div class="flex items-center gap-2 overflow-x-auto scrollbar-none pb-0.5">
      {#each moods as m}
        <button
          type="button"
          onclick={() => selectMood(m)}
          class="px-4 py-1.5 rounded-full text-xs font-medium whitespace-nowrap transition-all duration-200 cursor-pointer {activeMood ===
          m.label
            ? 'bg-white text-black font-semibold shadow-sm scale-[1.02]'
            : 'bg-white/[0.05] text-white/70 hover:text-white hover:bg-white/[0.1]'}"
        >
          {m.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- 1. Apple Music "Top Picks for You" Editorial Hero Shelf -->
  {#if isLoadingFeed && heroShowcase.length === 0}
    <section class="space-y-3">
      <div class="h-5 w-40 rounded-lg bg-white/[0.06] animate-pulse"></div>
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {#each Array(4) as _}
          <div class="aspect-[4/4.6] rounded-[24px] bg-white/[0.04] animate-pulse"></div>
        {/each}
      </div>
    </section>
  {:else if heroShowcase.length > 0 && activeMood === 'All'}
    <section class="space-y-3.5">
      <div class="flex items-center justify-between px-0.5">
        <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">Top Picks for You</h2>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {#each heroShowcase as item (item.track.id)}
          {@const track = item.track}
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
            class="group relative aspect-[4/4.6] rounded-[24px] overflow-hidden bg-white/[0.03] ring-1 ring-white/[0.08] hover:ring-white/20 shadow-xl cursor-pointer transition-all duration-300 hover:-translate-y-1 flex flex-col justify-between"
          >
            <!-- Full-Bleed Artwork + Ambient Frosted Footer -->
            {#if track.cover_art}
              <img
                src={track.cover_art}
                alt={track.title}
                class="absolute inset-0 w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
              />
            {/if}
            <div class="absolute inset-0 bg-gradient-to-t from-black/90 via-black/30 to-black/10"></div>

            <div class="relative z-10 p-4 flex items-center justify-between">
              <span class="px-2.5 py-1 rounded-full text-[10px] font-semibold uppercase tracking-wider bg-black/75 text-white/85 border border-white/10">
                {item.tag}
              </span>
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                class="w-7 h-7 rounded-full bg-black/75 text-white/75 hover:text-white flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
                aria-label="More options"
              >
                <span class="material-symbols-rounded text-base">more_horiz</span>
              </button>
            </div>

            <div class="relative z-10 p-4 flex items-end justify-between gap-3">
              <div class="min-w-0 flex-1">
                <h3 class="font-display font-bold text-base text-white line-clamp-1">{track.title}</h3>
                <p class="text-xs text-white/70 truncate mt-0.5">{sanitizeArtist(track.artist)}</p>
              </div>

              <!-- Floating Play / SVG Circular Loader / Equalizer Pill -->
              <div class="w-10 h-10 rounded-full bg-white text-black flex items-center justify-center shadow-lg shrink-0 group-hover:scale-105 transition-transform">
                {#if isCurrent && isBuffering}
                  <svg class="w-5 h-5 animate-spin archivetune-spinner-svg text-black" viewBox="0 0 24 24" fill="none">
                    <circle class="opacity-15" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                    <circle class="archivetune-spinner-arc" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                  </svg>
                {:else if isCurrent && playerState.playback.is_playing}
                  <div class="flex items-end gap-[2.5px] h-3.5">
                    <span class="w-[3px] h-full bg-black rounded-full animate-eq-1"></span>
                    <span class="w-[3px] h-full bg-black rounded-full animate-eq-2"></span>
                    <span class="w-[3px] h-full bg-black rounded-full animate-eq-3"></span>
                  </div>
                {:else}
                  <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- 2. ArchiveTune / YouTube Music "Quick Picks" 3-Column x 4-Row Speed Grid -->
  <section class="space-y-3.5">
    <div class="flex items-center justify-between px-0.5">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">
        {activeMood === 'All' ? 'Quick Picks' : `${activeMood} Picks`}
      </h2>

      {#if quickPicks.length > 0}
        <button
          type="button"
          onclick={() => {
            playerState.queue = quickPicks.slice(1);
            playerState.playTrack(quickPicks[0]);
          }}
          class="px-3.5 py-1 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-xs font-medium text-white/80 hover:text-white transition-colors cursor-pointer"
        >
          Play All
        </button>
      {/if}
    </div>

    {#if isLoadingFeed && quickPicks.length === 0}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2.5">
        {#each Array(9) as _}
          <div class="h-14 rounded-2xl bg-white/[0.04] animate-pulse"></div>
        {/each}
      </div>
    {:else if quickPicks.length > 0}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-4 gap-y-1.5">
        {#each quickPicks as track (track.id)}
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
            class="flex items-center justify-between p-2 rounded-2xl {isCurrent
              ? 'bg-white/[0.1] ring-1 ring-white/15'
              : 'hover:bg-white/[0.05]'} transition-all cursor-pointer group"
          >
            <div class="flex items-center gap-3 min-w-0 flex-1 pr-2">
              <div class="relative w-12 h-12 rounded-xl overflow-hidden bg-white/5 shrink-0 shadow-sm">
                {#if track.cover_art}
                  <img
                    loading="lazy"
                    decoding="async"
                    src={track.cover_art}
                    alt={track.title}
                    class="w-full h-full object-cover"
                  />
                {:else}
                  <div class="w-full h-full flex items-center justify-center text-white/30">
                    <span class="material-symbols-rounded text-lg">music_note</span>
                  </div>
                {/if}

                <div
                  class="absolute inset-0 bg-black/50 {isCurrent
                    ? 'opacity-100'
                    : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity"
                >
                  {#if isCurrent && isBuffering}
                    <svg class="w-5 h-5 animate-spin archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 24 24" fill="none">
                      <circle class="opacity-15" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                      <circle class="archivetune-spinner-arc" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                    </svg>
                  {:else if isCurrent && playerState.playback.is_playing}
                    <div class="flex items-end gap-[2.5px] h-3.5">
                      <span class="w-[3px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                      <span class="w-[3px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                      <span class="w-[3px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                    </div>
                  {:else}
                    <span class="material-symbols-rounded text-white text-xl filled">play_arrow</span>
                  {/if}
                </div>
              </div>

              <div class="min-w-0 flex-1">
                <p
                  class="text-xs font-semibold {isCurrent
                    ? 'text-[var(--md-sys-primary)]'
                    : 'text-white'} truncate"
                >
                  {track.title}
                </p>
                <p class="text-[11px] text-white/55 truncate mt-0.5">
                  {sanitizeArtist(track.artist)}
                </p>
              </div>
            </div>

            <div class="flex items-center gap-1 shrink-0">
              {#if track.duration > 0}
                <span class="text-[11px] font-mono text-white/35 group-hover:hidden">
                  {formatDuration(track.duration)}
                </span>
              {/if}
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.addToQueue(track);
                }}
                class="w-7 h-7 rounded-full hidden group-hover:flex items-center justify-center text-white/55 hover:text-white hover:bg-white/10 transition-colors"
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
                class="w-7 h-7 rounded-full hidden group-hover:flex items-center justify-center text-white/55 hover:text-white hover:bg-white/10 transition-colors"
                title="More options"
              >
                <span class="material-symbols-rounded text-base">more_horiz</span>
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- 3. "Listen Again" Horizontal Platter Carousel -->
  {#if listenAgain.length > 0}
    <section class="space-y-3.5">
      <div class="flex items-center justify-between px-0.5">
        <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">Listen Again</h2>
      </div>

      <div class="flex items-center gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
        {#each listenAgain as track (track.id)}
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
            class="w-36 sm:w-40 shrink-0 group cursor-pointer space-y-2"
          >
            <div class="relative aspect-square rounded-2xl overflow-hidden bg-white/5 shadow-md ring-1 ring-white/[0.07]">
              {#if track.cover_art}
                <img
                  loading="lazy"
                  decoding="async"
                  src={track.cover_art}
                  alt={track.title}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                />
              {:else}
                <div class="w-full h-full flex items-center justify-center text-white/30">
                  <span class="material-symbols-rounded text-3xl">music_note</span>
                </div>
              {/if}

              <div
                class="absolute inset-0 bg-black/30 {isCurrent
                  ? 'opacity-100'
                  : 'opacity-0 group-hover:opacity-100'} flex items-end justify-end p-2.5 transition-opacity"
              >
                <div class="w-9 h-9 rounded-full bg-white text-black flex items-center justify-center shadow-lg">
                  {#if isCurrent && isBuffering}
                    <svg class="w-4 h-4 animate-spin archivetune-spinner-svg text-black" viewBox="0 0 24 24" fill="none">
                      <circle class="opacity-15" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                      <circle class="archivetune-spinner-arc" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                    </svg>
                  {:else if isCurrent && playerState.playback.is_playing}
                    <div class="flex items-end gap-[2px] h-3">
                      <span class="w-[2.5px] h-full bg-black rounded-full animate-eq-1"></span>
                      <span class="w-[2.5px] h-full bg-black rounded-full animate-eq-2"></span>
                      <span class="w-[2.5px] h-full bg-black rounded-full animate-eq-3"></span>
                    </div>
                  {:else}
                    <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                  {/if}
                </div>
              </div>
            </div>

            <div class="px-0.5">
              <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                {track.title}
              </p>
              <p class="text-[11px] text-white/50 truncate mt-0.5">{sanitizeArtist(track.artist)}</p>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- 4. "More Like [Artist]" Personalized Artist Station Shelf -->
  {#if seedArtist && artistRadioTracks.length > 0}
    <section class="space-y-3.5">
      <div class="flex items-center justify-between px-0.5">
        <div class="flex items-center gap-2.5">
          <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">
            More Like {seedArtist}
          </h2>
        </div>
        <button
          type="button"
          onclick={() => {
            playerState.queue = artistRadioTracks.slice(1);
            playerState.playTrack(artistRadioTracks[0]);
          }}
          class="px-3.5 py-1 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-xs font-medium text-white/80 hover:text-white transition-colors cursor-pointer"
        >
          Start Station
        </button>
      </div>

      <div class="flex items-center gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
        {#each artistRadioTracks as track (track.id)}
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
            class="w-36 sm:w-40 shrink-0 group cursor-pointer space-y-2"
          >
            <div class="relative aspect-square rounded-2xl overflow-hidden bg-white/5 shadow-md ring-1 ring-white/[0.07]">
              {#if track.cover_art}
                <img
                  loading="lazy"
                  decoding="async"
                  src={track.cover_art}
                  alt={track.title}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                />
              {/if}
              <div
                class="absolute inset-0 bg-black/30 {isCurrent
                  ? 'opacity-100'
                  : 'opacity-0 group-hover:opacity-100'} flex items-end justify-end p-2.5 transition-opacity"
              >
                <div class="w-9 h-9 rounded-full bg-white text-black flex items-center justify-center shadow-lg">
                  {#if isCurrent && isBuffering}
                    <svg class="w-4 h-4 animate-spin archivetune-spinner-svg text-black" viewBox="0 0 24 24" fill="none">
                      <circle class="opacity-15" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                      <circle class="archivetune-spinner-arc" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                    </svg>
                  {:else if isCurrent && playerState.playback.is_playing}
                    <div class="flex items-end gap-[2px] h-3">
                      <span class="w-[2.5px] h-full bg-black rounded-full animate-eq-1"></span>
                      <span class="w-[2.5px] h-full bg-black rounded-full animate-eq-2"></span>
                      <span class="w-[2.5px] h-full bg-black rounded-full animate-eq-3"></span>
                    </div>
                  {:else}
                    <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                  {/if}
                </div>
              </div>
            </div>

            <div class="px-0.5">
              <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                {track.title}
              </p>
              <p class="text-[11px] text-white/50 truncate mt-0.5">{sanitizeArtist(track.artist)}</p>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- 5. "Top Charts" Ranked List (Apple Music / ArchiveTune Charts) -->
  {#if chartTracks.length > 0}
    <section class="space-y-3.5">
      <div class="flex items-center justify-between px-0.5">
        <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">Top Charts</h2>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-1.5">
        {#each chartTracks.slice(0, 8) as track, idx (track.id)}
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
            class="flex items-center justify-between p-2 rounded-2xl {isCurrent
              ? 'bg-white/[0.1]'
              : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group"
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-2">
              <span class="w-5 text-center font-display font-bold text-xs {idx < 3 ? 'text-[var(--md-sys-primary)]' : 'text-white/40'} shrink-0">
                {idx + 1}
              </span>

              <div class="relative w-11 h-11 rounded-xl overflow-hidden bg-white/5 shrink-0">
                {#if track.cover_art}
                  <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover" />
                {/if}
                <div class="absolute inset-0 bg-black/45 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                  {#if isCurrent && isBuffering}
                    <svg class="w-4 h-4 animate-spin archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 24 24" fill="none">
                      <circle class="opacity-15" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                      <circle class="archivetune-spinner-arc" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                    </svg>
                  {:else if isCurrent && playerState.playback.is_playing}
                    <div class="flex items-end gap-[2px] h-3">
                      <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                      <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                      <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                    </div>
                  {:else}
                    <span class="material-symbols-rounded text-white text-lg filled">play_arrow</span>
                  {/if}
                </div>
              </div>

              <div class="min-w-0 flex-1">
                <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                  {track.title}
                </p>
                <p class="text-[11px] text-white/50 truncate mt-0.5">{sanitizeArtist(track.artist)}</p>
              </div>
            </div>

            <div class="flex items-center gap-1.5 shrink-0">
              {#if track.duration > 0}
                <span class="text-[11px] font-mono text-white/35 group-hover:hidden">
                  {formatDuration(track.duration)}
                </span>
              {/if}
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                class="w-7 h-7 rounded-full hidden group-hover:flex items-center justify-center text-white/55 hover:text-white hover:bg-white/10"
                title="More options"
              >
                <span class="material-symbols-rounded text-base">more_horiz</span>
              </button>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  @keyframes eqBar1 {
    0%, 100% { transform: scaleY(0.35); }
    50% { transform: scaleY(1); }
  }
  @keyframes eqBar2 {
    0%, 100% { transform: scaleY(0.95); }
    40% { transform: scaleY(0.3); }
  }
  @keyframes eqBar3 {
    0%, 100% { transform: scaleY(0.5); }
    65% { transform: scaleY(0.95); }
  }
  .animate-eq-1 {
    transform-origin: bottom;
    will-change: transform;
    animation: eqBar1 0.75s ease-in-out infinite;
  }
  .animate-eq-2 {
    transform-origin: bottom;
    will-change: transform;
    animation: eqBar2 0.65s ease-in-out infinite;
  }
  .animate-eq-3 {
    transform-origin: bottom;
    will-change: transform;
    animation: eqBar3 0.8s ease-in-out infinite;
  }
</style>
