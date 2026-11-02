<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist, type Track, type YouTubeTrack } from '../types';

  let onlineDiscovery = $state<Track[]>([]);
  let isLoadingFeed = $state(false);
  let activeMood = $state<string>('All');

  const moods = [
    { label: 'All', query: '' },
    { label: 'Chill', query: 'chill lofi acoustic songs' },
    { label: 'Focus', query: 'deep focus ambient instrumental' },
    { label: 'Energy', query: 'upbeat energetic hits' },
    { label: 'Workout', query: 'workout gym motivation songs' },
    { label: 'Late Night', query: 'late night synthwave rnb' },
    { label: 'Local', query: '__LOCAL__' },
  ];

  // Combine user history + local library + personalized online discovery (Apple Music "Home" / ArchiveTune Quick Picks)
  let quickPicks = $derived.by(() => {
    if (activeMood === 'Local') {
      return playerState.library.slice(0, 12);
    }
    if (activeMood !== 'All' && onlineDiscovery.length > 0) {
      return onlineDiscovery.slice(0, 12);
    }
    const seen = new Set<string>();
    const combined: Track[] = [];
    for (const t of [
      ...playerState.playHistoryTracks.slice(0, 4),
      ...playerState.library.slice(0, 4),
      ...onlineDiscovery,
    ]) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        combined.push(t);
      }
      if (combined.length >= 12) break;
    }
    return combined;
  });

  let recentlyPlayed = $derived.by(() => {
    const list =
      playerState.playHistoryTracks.length > 0
        ? playerState.playHistoryTracks
        : playerState.library;
    return list.slice(0, 8);
  });

  async function fetchDiscoveryFeed(customQuery?: string) {
    if (activeMood === 'Local') return;
    isLoadingFeed = true;
    try {
      let seedQuery = customQuery || '';
      if (!seedQuery) {
        const seedArtist =
          playerState.playHistoryTracks[0]?.artist ||
          playerState.library[0]?.artist ||
          '';
        seedQuery =
          seedArtist && seedArtist !== 'Unknown Artist'
            ? `${sanitizeArtist(seedArtist)} mix`
            : 'popular hits songs';
      }

      const res = await invoke<YouTubeTrack[]>('search_youtube', {
        query: seedQuery,
        limit: 12,
      });

      if (res && res.length > 0) {
        onlineDiscovery = res.map((yt) => ({
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
        }));
        if (onlineDiscovery[0]) {
          playerState.prefetchStream(
            onlineDiscovery[0].file_path,
            onlineDiscovery[0].title,
            onlineDiscovery[0].artist
          );
        }
      }
    } catch (e) {
      console.warn('Discovery feed error:', e);
    } finally {
      isLoadingFeed = false;
    }
  }

  function selectMood(mood: { label: string; query: string }) {
    activeMood = mood.label;
    playerState.currentMood = mood.label;
    if (mood.query && mood.query !== '__LOCAL__') {
      fetchDiscoveryFeed(mood.query);
    } else if (mood.label === 'All') {
      fetchDiscoveryFeed();
    }
  }

  function formatDuration(sec: number): string {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }

  onMount(() => {
    fetchDiscoveryFeed();
  });
</script>

<div class="p-6 max-w-6xl mx-auto space-y-8 pb-28 select-none">
  <!-- Top Mood Filter Chips -->
  <div class="flex items-center justify-between gap-4">
    <div class="flex items-center gap-1.5 overflow-x-auto scrollbar-none pb-1">
      {#each moods as m}
        <button
          type="button"
          onclick={() => selectMood(m)}
          class="px-3.5 py-1.5 rounded-full text-xs font-medium whitespace-nowrap transition-all cursor-pointer {activeMood ===
          m.label
            ? 'bg-white text-black font-semibold shadow-sm'
            : 'bg-white/[0.05] text-white/65 hover:text-white hover:bg-white/[0.1]'}"
        >
          {m.label}
        </button>
      {/each}
    </div>

    <button
      type="button"
      onclick={() => fetchDiscoveryFeed()}
      class="w-8 h-8 rounded-full bg-white/[0.05] hover:bg-white/[0.1] flex items-center justify-center text-white/60 hover:text-white transition-colors cursor-pointer shrink-0"
      title="Refresh recommendations"
    >
      <span class="material-symbols-rounded text-base {isLoadingFeed ? 'animate-spin' : ''}">
        refresh
      </span>
    </button>
  </div>

  <!-- Quick Picks (2-Column Editorial Compact List) -->
  <section class="space-y-3">
    <div class="flex items-center justify-between px-0.5">
      <h2 class="font-display font-bold text-lg text-white">Quick Picks</h2>
      {#if quickPicks.length > 0}
        <button
          type="button"
          onclick={() => {
            playerState.queue = quickPicks.slice(1);
            playerState.playTrack(quickPicks[0]);
          }}
          class="text-xs font-medium text-white/55 hover:text-white transition-colors cursor-pointer"
        >
          Play All
        </button>
      {/if}
    </div>

    {#if isLoadingFeed && quickPicks.length === 0}
      <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
        {#each Array(6) as _}
          <div class="h-14 rounded-2xl bg-white/[0.03] animate-pulse"></div>
        {/each}
      </div>
    {:else if quickPicks.length > 0}
      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-1.5">
        {#each quickPicks as track (track.id)}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onmouseenter={() =>
              playerState.prefetchStream(track.file_path, track.title, track.artist)}
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="flex items-center justify-between p-2 rounded-2xl {isCurrent
              ? 'bg-white/[0.09]'
              : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group"
          >
            <div class="flex items-center gap-3 min-w-0 flex-1 pr-2">
              <div class="relative w-11 h-11 rounded-xl overflow-hidden bg-white/5 shrink-0">
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
                  class="absolute inset-0 bg-black/45 {isCurrent
                    ? 'opacity-100'
                    : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity"
                >
                  <span class="material-symbols-rounded text-white text-xl filled">
                    {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                  </span>
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
                  playerState.addToQueue(track);
                }}
                class="w-7 h-7 rounded-full hidden group-hover:flex items-center justify-center text-white/50 hover:text-white hover:bg-white/10 transition-colors"
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
                class="w-7 h-7 rounded-full hidden group-hover:flex items-center justify-center text-white/50 hover:text-white hover:bg-white/10 transition-colors"
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

  <!-- Recently Played Carousel (Apple Music Square Cards) -->
  {#if recentlyPlayed.length > 0}
    <section class="space-y-3">
      <div class="flex items-center justify-between px-0.5">
        <h2 class="font-display font-bold text-lg text-white">Recently Played</h2>
      </div>

      <div class="grid grid-cols-2 sm:grid-cols-4 md:grid-cols-6 gap-4">
        {#each recentlyPlayed.slice(0, 6) as track (track.id)}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onmouseenter={() =>
              playerState.prefetchStream(track.file_path, track.title, track.artist)}
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="group cursor-pointer space-y-2"
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
                  <span class="material-symbols-rounded text-xl filled">
                    {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                  </span>
                </div>
              </div>
            </div>

            <div class="px-0.5">
              <p class="text-xs font-semibold text-white truncate">{track.title}</p>
              <p class="text-[11px] text-white/50 truncate">{sanitizeArtist(track.artist)}</p>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Personal Stations -->
  <section class="space-y-3">
    <h2 class="font-display font-bold text-lg text-white px-0.5">Stations for You</h2>
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-3.5">
      {#each [
        { title: 'Discovery Station', subtitle: 'Fresh picks', query: 'new music discovery hits', gradient: 'from-violet-600/30 to-indigo-900/20' },
        { title: 'Favorites Mix', subtitle: 'Your top replays', query: 'essential popular songs mix', gradient: 'from-rose-600/30 to-pink-900/20' },
        { title: 'Chill Lounge', subtitle: 'Downtempo & acoustic', query: 'chill acoustic downtempo', gradient: 'from-teal-600/30 to-emerald-900/20' },
        { title: 'Late Night Drive', subtitle: 'Synth & electronic', query: 'night drive synthwave rnb', gradient: 'from-amber-600/30 to-orange-900/20' },
      ] as station}
        <button
          type="button"
          onclick={() => {
            playerState.searchYouTube(station.query);
            playerState.activeTab = 'search';
          }}
          class="p-4 h-28 rounded-2xl bg-gradient-to-br {station.gradient} border border-white/[0.07] hover:border-white/20 flex flex-col justify-between text-left transition-all cursor-pointer group"
        >
          <span class="material-symbols-rounded text-white/70 text-xl">radio</span>
          <div>
            <p class="font-display font-bold text-sm text-white">{station.title}</p>
            <p class="text-[11px] text-white/55 mt-0.5">{station.subtitle}</p>
          </div>
        </button>
      {/each}
    </div>
  </section>
</div>

