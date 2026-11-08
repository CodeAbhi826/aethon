<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import {
    sanitizeArtist,
    handleThumbnailError,
    handleThumbnailLoad,
    type Track,
    type YouTubeTrack,
    type HomeShelf,
  } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from '../components/ArtistLinks.svelte';

  // ArchiveTune / YouTube Music Mood Filter Chips (tap active chip to clear filter)
  const moods = [
    { label: 'Energize', query: 'upbeat high energy hits songs' },
    { label: 'Workout', query: 'workout gym phonk motivation hits' },
    { label: 'Relax', query: 'chill relaxing acoustic lofi songs' },
    { label: 'Commute', query: 'driving feel good pop indie hits' },
    { label: 'Focus', query: 'deep focus ambient study instrumental' },
    { label: 'Romance', query: 'romantic love songs acoustic hits' },
    { label: 'Party', query: 'party dance club electronic hits' },
    { label: 'Feel good', query: 'feel good happy upbeat hits' },
    { label: 'Sad', query: 'sad emotional heartbreak songs' },
    { label: 'Sleep', query: 'sleep ambient calm piano music' },
  ];

  let activeMood = $state<string | null>(null);
  let quickPickFeed = $state<Track[]>(playerState.homeQuickPickFeed);
  let chartTracks = $state<Track[]>(playerState.homeChartTracks);
  let artistRadioTracks = $state<Track[]>(playerState.homeArtistRadioTracks);
  let homeShelves = $state<{ title: string; tracks: Track[] }[]>([]);
  let secondSimilarShelf = $state<{ artist: string; tracks: Track[] } | null>(null);
  let isLoadingFeed = $state(!playerState.homeFeedLoaded);

  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  // Extract distinct recent artists from play history & library for ArchiveTune "Similar to ..." shelves
  let recentArtists = $derived.by(() => {
    const seen = new Set<string>();
    const out: string[] = [];
    const candidates = [
      ...playerState.playHistoryTracks,
      ...(playerState.currentTrack ? [playerState.currentTrack] : []),
      ...playerState.library,
    ];
    for (const t of candidates) {
      if (!t.artist) continue;
      const primary = sanitizeArtist(t.artist).split(',')[0].split('&')[0].trim();
      const lower = primary.toLowerCase();
      if (
        primary &&
        lower !== 'unknown' &&
        lower !== 'unknown artist' &&
        lower !== 'youtube music' &&
        !seen.has(lower)
      ) {
        seen.add(lower);
        out.push(primary);
      }
      if (out.length >= 3) break;
    }
    return out;
  });

  let seedArtist = $derived(recentArtists[0] ?? null);
  let seedTrack = $derived(playerState.playHistoryTracks[0] ?? playerState.currentTrack ?? null);

  function ytToTrack(yt: YouTubeTrack, albumLabel = 'YouTube Music'): Track {
    return {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: sanitizeArtist(yt.artist) || 'Unknown Artist',
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

  // 1. ArchiveTune "Quick picks" (4-row x 3-col Uniform Grid of 12 personalized tracks)
  let quickPicks = $derived.by(() => {
    const seen = new Set<string>();
    const list: Track[] = [];
    const homeShelfTracks = homeShelves.flatMap((s) => s.tracks);
    const source = activeMood
      ? quickPickFeed
      : [
          ...playerState.playHistoryTracks.slice(0, 2),
          ...quickPickFeed,
          ...homeShelfTracks,
          ...chartTracks,
        ];

    for (const t of source) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        list.push(t);
      }
      if (list.length >= 12) break;
    }
    return list;
  });

  // 2. ArchiveTune "Keep listening" (Clean square covers of recently played & active tracks — no overlay text tags)
  let keepListening = $derived.by(() => {
    const seen = new Set<string>();
    const list: Track[] = [];
    for (const t of [
      ...playerState.playHistoryTracks,
      ...playerState.library.slice(0, 8),
      ...quickPickFeed.slice(0, 6),
    ]) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        list.push(t);
      }
      if (list.length >= 12) break;
    }
    return list;
  });

  // 3. ArchiveTune "Forgotten favorites" (Liked / older history tracks not in the top 4 recent plays)
  let forgottenFavorites = $derived.by(() => {
    const recentIds = new Set(playerState.playHistoryTracks.slice(0, 4).map((t) => t.id));
    const seen = new Set<string>();
    const list: Track[] = [];
    const pool = [
      ...playerState.library.filter((t) => t.favorite),
      ...playerState.playHistoryTracks.slice(4),
      ...playerState.library,
      ...chartTracks,
    ];
    for (const t of pool) {
      if (!recentIds.has(t.id) && !seen.has(t.id)) {
        seen.add(t.id);
        list.push(t);
      }
      if (list.length >= 12) break;
    }
    return list;
  });

  async function loadHomeFeeds(customMoodQuery?: string) {
    isLoadingFeed = true;
    try {
      const seedVid =
        !customMoodQuery && seedTrack
          ? playerState.extractVideoId(seedTrack.file_path) || playerState.extractVideoId(seedTrack.id)
          : null;

      const primaryQuery =
        customMoodQuery ||
        (seedArtist ? `${seedArtist} top hits mix` : 'trending global hits songs');

      // Use RDAMVM Automix when a seed track is known + authenticated/anonymous FEmusic_home shelves
      const [automixRes, feedRes, shelvesRes, chartRes] = await Promise.all([
        seedVid
          ? invoke<YouTubeTrack[]>('get_automix_queue', {
              videoId: seedVid,
              cookie: settingsState.ytmCookie || null,
            }).catch(() => [])
          : Promise.resolve([]),
        invoke<YouTubeTrack[]>('search_youtube', { query: primaryQuery, limit: 24 }).catch(() => []),
        !customMoodQuery
          ? invoke<HomeShelf[]>('get_home_shelves', {
              cookie: settingsState.ytmCookie || null,
            }).catch(() => [])
          : Promise.resolve([]),
        chartTracks.length === 0
          ? invoke<YouTubeTrack[]>('search_youtube', {
              query: 'top music charts global hits',
              limit: 16,
            }).catch(() => [])
          : Promise.resolve([]),
      ]);

      const combinedFeed = [...(automixRes || []), ...(feedRes || [])];
      if (combinedFeed.length > 0) {
        const seen = new Set<string>();
        const tracks: Track[] = [];
        for (const yt of combinedFeed) {
          if (!seen.has(yt.id)) {
            seen.add(yt.id);
            tracks.push(ytToTrack(yt, 'Quick picks'));
          }
        }
        quickPickFeed = tracks;
        if (!customMoodQuery) {
          playerState.homeQuickPickFeed = tracks;
        }
      }

      if (shelvesRes && shelvesRes.length > 0) {
        homeShelves = shelvesRes.map((s) => ({
          title: s.title,
          tracks: s.tracks.map((yt) => ytToTrack(yt, s.title)),
        }));
      }

      if (chartRes && chartRes.length > 0) {
        const cTracks = chartRes.map((yt) => ytToTrack(yt, 'Charts'));
        chartTracks = cTracks;
        playerState.homeChartTracks = cTracks;
      }

      playerState.homeFeedLoaded = true;
    } finally {
      isLoadingFeed = false;
    }
  }

  let lastSeedKey = '';
  $effect(() => {
    const artists = recentArtists;
    const isConnected = settingsState.ytmConnected;
    const key = `${isConnected}:${artists.join('|')}`;
    if (key !== lastSeedKey) {
      lastSeedKey = key;
      if (artists.length > 0) {
        const first = artists[0];
        invoke<YouTubeTrack[]>('search_youtube', {
          query: `${first} popular songs`,
          limit: 14,
        })
          .then((res) => {
            if (res && res.length > 0) {
              const rTracks = res.map((yt) => ytToTrack(yt, `Similar to ${first}`));
              artistRadioTracks = rTracks;
              playerState.homeArtistRadioTracks = rTracks;
            }
          })
          .catch(() => {});

        if (artists.length > 1) {
          const second = artists[1];
          invoke<YouTubeTrack[]>('search_youtube', {
            query: `${second} best songs`,
            limit: 14,
          })
            .then((res) => {
              if (res && res.length > 0) {
                secondSimilarShelf = {
                  artist: second,
                  tracks: res.map((yt) => ytToTrack(yt, `Similar to ${second}`)),
                };
              }
            })
            .catch(() => {});
        }
      }
    }
  });

  function toggleMood(m: { label: string; query: string }) {
    if (activeMood === m.label) {
      activeMood = null;
      playerState.currentMood = 'All';
      loadHomeFeeds(undefined);
    } else {
      activeMood = m.label;
      playerState.currentMood = m.label;
      loadHomeFeeds(m.query);
    }
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

<div class="px-6 py-5 max-w-7xl mx-auto space-y-8 pb-28 select-none">
  <!-- ArchiveTune Mood Filter Chips Bar (Clean Top Row + Account Status Pill) -->
  <div class="flex items-center justify-between gap-3">
    <div class="flex items-center gap-2 overflow-x-auto scrollbar-none pb-0.5 flex-1">
      {#each moods as m}
        {@const isSelected = activeMood === m.label}
        <button
          type="button"
          onclick={() => toggleMood(m)}
          class="px-4 py-1.5 rounded-xl text-xs font-medium whitespace-nowrap transition-all duration-150 cursor-pointer border {isSelected
            ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] border-transparent font-semibold shadow-sm'
            : 'bg-white/[0.05] text-white/75 border-white/[0.06] hover:text-white hover:bg-white/[0.1]'}"
        >
          {m.label}
        </button>
      {/each}
    </div>

    {#if settingsState.ytmConnected}
      <button
        type="button"
        onclick={() => (playerState.activeTab = 'settings')}
        class="hidden sm:flex items-center gap-2 px-3 py-1 rounded-full bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.08] text-xs text-white/80 shrink-0 cursor-pointer"
        title="Connected to YouTube Music"
      >
        {#if settingsState.ytmAccountAvatar}
          <img src={settingsState.ytmAccountAvatar} alt="" class="w-4 h-4 rounded-full object-cover" />
        {:else}
          <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
        {/if}
        <span class="truncate max-w-[120px]">{settingsState.ytmAccountName || 'YouTube Music'}</span>
      </button>
    {/if}
  </div>

  <!-- 1. ArchiveTune "Quick picks" 3-Column x 4-Row Uniform Speed Grid -->
  <section class="space-y-3">
    <div class="flex items-center justify-between px-0.5">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">
        {activeMood ? `${activeMood}` : 'Quick picks'}
      </h2>
    </div>

    {#if isLoadingFeed && quickPicks.length === 0}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2.5">
        {#each Array(12) as _}
          <div class="h-16 rounded-2xl bg-white/[0.04] animate-pulse"></div>
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
            class="h-16 flex items-center justify-between px-2.5 rounded-2xl {isCurrent
              ? 'bg-[var(--md-sys-primary-container)]/35 ring-1 ring-[var(--md-sys-primary)]/35'
              : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group overflow-hidden"
          >
            <div class="flex items-center gap-3 min-w-0 flex-1 pr-2">
              <div class="relative w-12 h-12 rounded-xl overflow-hidden bg-white/5 shrink-0 shadow-sm">
                {#if track.cover_art}
                  <img
                    loading="lazy"
                    decoding="async"
                    src={track.cover_art}
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

              <!-- Strict single-line truncation on both Title and Artist so every card has identical height -->
              <div class="min-w-0 flex-1 overflow-hidden">
                <p
                  class="text-xs font-semibold {isCurrent
                    ? 'text-[var(--md-sys-primary)]'
                    : 'text-white'} truncate block w-full"
                >
                  {track.title}
                </p>
                <ArtistLinks
                  artist={track.artist}
                  class="text-[11px] text-white/55 mt-0.5"
                  buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                />
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
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                class="w-7 h-7 rounded-full hidden group-hover:flex items-center justify-center text-white/55 hover:text-white hover:bg-white/10 transition-colors"
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

  <!-- 2. ArchiveTune "Keep listening" Horizontal Shelf -->
  {#if keepListening.length > 0 && !activeMood}
    <section class="space-y-3">
      <div class="flex items-center justify-between px-0.5">
        <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">Keep listening</h2>
      </div>

      <div class="flex items-start gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
        {#each keepListening as track (track.id)}
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
                  onload={(e) => handleThumbnailLoad(e, track.id)}
                  onerror={(e) => handleThumbnailError(e, track.id)}
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
                <div class="w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                  {#if isCurrent && isBuffering}
                    <svg class="w-4 h-4 animate-spin archivetune-spinner-svg text-current" viewBox="0 0 24 24" fill="none">
                      <circle class="opacity-15" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                      <circle class="archivetune-spinner-arc" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
                    </svg>
                  {:else if isCurrent && playerState.playback.is_playing}
                    <div class="flex items-end gap-[2px] h-3">
                      <span class="w-[2.5px] h-full bg-current rounded-full animate-eq-1"></span>
                      <span class="w-[2.5px] h-full bg-current rounded-full animate-eq-2"></span>
                      <span class="w-[2.5px] h-full bg-current rounded-full animate-eq-3"></span>
                    </div>
                  {:else}
                    <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                  {/if}
                </div>
              </div>
            </div>

            <div class="px-0.5 overflow-hidden">
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
    </section>
  {/if}

  <!-- 3. ArchiveTune "Forgotten favorites" Shelf -->
  {#if forgottenFavorites.length >= 4 && !activeMood}
    <section class="space-y-3">
      <div class="flex items-center justify-between px-0.5">
        <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">Forgotten favorites</h2>
      </div>

      <div class="flex items-start gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
        {#each forgottenFavorites as track (track.id)}
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
                  onload={(e) => handleThumbnailLoad(e, track.id)}
                  onerror={(e) => handleThumbnailError(e, track.id)}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                />
              {/if}
              <div
                class="absolute inset-0 bg-black/30 {isCurrent
                  ? 'opacity-100'
                  : 'opacity-0 group-hover:opacity-100'} flex items-end justify-end p-2.5 transition-opacity"
              >
                <div class="w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                  <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                </div>
              </div>
            </div>

            <div class="px-0.5 overflow-hidden">
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
    </section>
  {/if}

  <!-- 4. ArchiveTune "Similar to [Artist]" Personalized Recommendations -->
  {#if seedArtist && artistRadioTracks.length > 0 && !activeMood}
    <section class="space-y-3">
      <div class="flex items-center justify-between px-0.5">
        <div>
          <p class="text-[10px] font-bold uppercase tracking-wider text-white/40">Similar to</p>
          <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">
            {seedArtist}
          </h2>
        </div>
      </div>

      <div class="flex items-start gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
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
                  onload={(e) => handleThumbnailLoad(e, track.id)}
                  onerror={(e) => handleThumbnailError(e, track.id)}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                />
              {/if}
              <div
                class="absolute inset-0 bg-black/30 {isCurrent
                  ? 'opacity-100'
                  : 'opacity-0 group-hover:opacity-100'} flex items-end justify-end p-2.5 transition-opacity"
              >
                <div class="w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                  <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                </div>
              </div>
            </div>

            <div class="px-0.5 overflow-hidden">
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
    </section>
  {/if}

  {#if secondSimilarShelf && secondSimilarShelf.tracks.length > 0 && !activeMood}
    <section class="space-y-3">
      <div class="flex items-center justify-between px-0.5">
        <div>
          <p class="text-[10px] font-bold uppercase tracking-wider text-white/40">Similar to</p>
          <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">
            {secondSimilarShelf.artist}
          </h2>
        </div>
      </div>

      <div class="flex items-start gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
        {#each secondSimilarShelf.tracks as track (track.id)}
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
                  onload={(e) => handleThumbnailLoad(e, track.id)}
                  onerror={(e) => handleThumbnailError(e, track.id)}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                />
              {/if}
              <div
                class="absolute inset-0 bg-black/30 {isCurrent
                  ? 'opacity-100'
                  : 'opacity-0 group-hover:opacity-100'} flex items-end justify-end p-2.5 transition-opacity"
              >
                <div class="w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                  <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                </div>
              </div>
            </div>

            <div class="px-0.5 overflow-hidden">
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
    </section>
  {/if}

  <!-- 5. Live YouTube Music Home Shelves (FEmusic_home: Mixed for you, New releases, Trending, Recommended videos, etc.) -->
  {#if homeShelves.length > 0 && !activeMood}
    {#each homeShelves as shelf (shelf.title)}
      <section class="space-y-3">
        <div class="flex items-center justify-between px-0.5">
          <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">{shelf.title}</h2>
        </div>

        <div class="flex items-start gap-4 overflow-x-auto scrollbar-none pb-2 -mx-6 px-6">
          {#each shelf.tracks as track (track.id)}
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
                    onload={(e) => handleThumbnailLoad(e, track.id)}
                    onerror={(e) => handleThumbnailError(e, track.id)}
                    class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                  />
                {/if}
                <div
                  class="absolute inset-0 bg-black/30 {isCurrent
                    ? 'opacity-100'
                    : 'opacity-0 group-hover:opacity-100'} flex items-end justify-end p-2.5 transition-opacity"
                >
                  <div class="w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] flex items-center justify-center shadow-lg">
                    <span class="material-symbols-rounded text-xl filled">play_arrow</span>
                  </div>
                </div>
              </div>

              <div class="px-0.5 overflow-hidden">
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
      </section>
    {/each}
  {/if}

  <!-- 6. Charts Shelf -->
  {#if chartTracks.length > 0 && !activeMood}
    <section class="space-y-3">
      <div class="flex items-center justify-between px-0.5">
        <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">Charts</h2>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-1.5">
        {#each chartTracks.slice(0, 10) as track, idx (track.id)}
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
            class="h-14 flex items-center justify-between px-2.5 rounded-2xl {isCurrent
              ? 'bg-[var(--md-sys-primary-container)]/35'
              : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group overflow-hidden"
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-2">
              <span class="w-5 text-center font-display font-bold text-xs {idx < 3 ? 'text-[var(--md-sys-primary)]' : 'text-white/40'} shrink-0">
                {idx + 1}
              </span>

              <div class="relative w-11 h-11 rounded-xl overflow-hidden bg-white/5 shrink-0">
                {#if track.cover_art}
                  <img
                    loading="lazy"
                    decoding="async"
                    src={track.cover_art}
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
                  class="text-[11px] text-white/50 mt-0.5"
                  buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                />
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
                <span class="material-symbols-rounded text-base">more_vert</span>
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
