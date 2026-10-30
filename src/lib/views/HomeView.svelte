<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist, type Track, type YouTubeTrack } from '../types';

  const moods = ['All', 'Relax', 'Focus', 'Energize', 'Workout', 'Commute', 'Local Only'] as const;
  type MoodType = typeof moods[number];
  let activeMood = $state<MoodType>('All');
  let moodTracks = $state<Track[]>([]);
  let isMoodLoading = $state(false);

  const moodQueries: Record<Exclude<MoodType, 'All' | 'Local Only'>, string> = {
    'Relax': 'Chill Acoustic Melodies',
    'Focus': 'Instrumental Lo-Fi Study Classical Beats',
    'Energize': 'Upbeat High Energy Pop Electronic Hits',
    'Workout': 'High Energy Workout Bass Motivation',
    'Commute': 'Feel Good Travel Acoustic Hits',
  };

  let liveTrending = $state<Track[]>([]);
  let similarArtistTracks = $state<Track[]>([]);

  function cleanArtistName(raw: string): string {
    return sanitizeArtist(raw).split(',')[0].trim();
  }

  // Compute recommended artist dynamically based on history, current track, or local library
  let recommendedArtist = $derived.by(() => {
    const fromHistory = playerState.playHistoryTracks.find(t => t.artist && t.artist.trim().toLowerCase() !== 'unknown')?.artist;
    if (fromHistory) return cleanArtistName(fromHistory);

    if (playerState.currentTrack?.artist && playerState.currentTrack.artist.trim().toLowerCase() !== 'unknown') {
      return cleanArtistName(playerState.currentTrack.artist);
    }

    const fromLibrary = playerState.library.find(t => t.artist && t.artist.trim().toLowerCase() !== 'unknown')?.artist;
    if (fromLibrary) return cleanArtistName(fromLibrary);

    return null;
  });

  async function handleMoodChange(mood: MoodType) {
    activeMood = mood;
    if (mood === 'All') {
      moodTracks = [];
      return;
    }
    if (mood === 'Local Only') {
      moodTracks = playerState.library.slice(0, 12);
      return;
    }
    isMoodLoading = true;
    try {
      const q = moodQueries[mood];
      const results = await invoke<YouTubeTrack[]>('search_youtube', { query: q, limit: 6 });
      if (results && results.length > 0) {
        moodTracks = results.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: sanitizeArtist(yt.artist) || mood,
          album: `${mood} Session`,
          duration: yt.duration || 200,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 10 + idx,
          favorite: false,
          date_added: Math.floor(Date.now() / 1000),
        }));
      }
    } catch (e) {
      console.warn('Mood fetch failed:', e);
    } finally {
      isMoodLoading = false;
    }
  }

  let isLoadingSimilar = false;
  async function loadSimilarArtist(artist: string) {
    if (!artist || !artist.trim() || artist.trim().toLowerCase() === 'unknown' || isLoadingSimilar) return;
    isLoadingSimilar = true;
    try {
      const results = await invoke<YouTubeTrack[]>('search_youtube', { query: `${artist} top songs radio`, limit: 6 });
      if (results && results.length > 0) {
        similarArtistTracks = results.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: sanitizeArtist(yt.artist) || artist,
          album: `${artist} Radio`,
          duration: yt.duration || 215,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 40 + idx * 3,
          favorite: false,
          date_added: Math.floor(Date.now() / 1000),
        }));
      }
    } catch (e) {
      console.warn('Similar artist tracks failed:', e);
    } finally {
      isLoadingSimilar = false;
    }
  }

  onMount(async () => {
    try {
      const results = await invoke<YouTubeTrack[]>('search_youtube', { query: 'Global Top Trending Hits', limit: 6 });
      if (results && results.length > 0) {
        liveTrending = results.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: sanitizeArtist(yt.artist) || 'Top Chart',
          album: 'Daily Hot Hits',
          duration: yt.duration || 210,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 50 + idx * 5,
          favorite: false,
          date_added: Math.floor(Date.now() / 1000),
        }));
      }
    } catch (e) {
      console.warn('Could not load trending:', e);
    }
  });

  let lastLoadedArtist = '';
  let artistDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const artist = recommendedArtist;
    if (artist && artist !== lastLoadedArtist) {
      if (artistDebounceTimer) {
        clearTimeout(artistDebounceTimer);
      }
      artistDebounceTimer = setTimeout(() => {
        if (artist && artist !== lastLoadedArtist) {
          lastLoadedArtist = artist;
          loadSimilarArtist(artist);
        }
      }, 800);
    }
    return () => {
      if (artistDebounceTimer) {
        clearTimeout(artistDebounceTimer);
      }
    };
  });

  let displayQuickPicks = $derived(
    playerState.library.length > 0 
      ? playerState.library.slice(0, 6) 
      : liveTrending.slice(0, 6)
  );

  let listenAgainTracks = $derived(
    playerState.playHistoryTracks.length > 0
      ? playerState.playHistoryTracks.slice(0, 8)
      : (playerState.library.length > 0 ? playerState.library.slice(0, 8) : liveTrending.slice(0, 8))
  );

  // Dynamic stations based on library artists or high-fidelity genres
  let dynamicStations = $derived.by(() => {
    const artists = Array.from(new Set(
      [...playerState.playHistoryTracks, ...playerState.library]
        .map(t => cleanArtistName(t.artist))
        .filter(a => a && a.toLowerCase() !== 'unknown')
    )).slice(0, 3);

    const stations = artists.map(a => ({
      name: a,
      type: 'Artist Station',
      img: 'https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=160&auto=format&fit=crop',
      query: `${a} top songs`,
    }));

    const defaults = [
      { name: 'Lo-Fi Chill', type: 'Study & Sleep', img: 'https://images.unsplash.com/photo-1518609878373-06d740f60d8b?w=160&auto=format&fit=crop', query: 'Lo-Fi Chill Beats' },
      { name: 'Synthwave', type: 'Retro Neon', img: 'https://images.unsplash.com/photo-1508700115892-45ecd05ae2ad?w=160&auto=format&fit=crop', query: 'Synthwave Chillwave' },
      { name: 'Acoustic', type: 'Unplugged Coffee', img: 'https://images.unsplash.com/photo-1510915361894-db8b60106cb1?w=160&auto=format&fit=crop', query: 'Acoustic Guitar Hits' },
      { name: 'Electronic', type: 'Club & Dance', img: 'https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=160&auto=format&fit=crop', query: 'Electronic Dance Anthems' },
      { name: 'Deep Focus', type: 'Ambient Strings', img: 'https://images.unsplash.com/photo-1445985543469-433ec003388b?w=160&auto=format&fit=crop', query: 'Deep Focus Ambient Instrumental' },
    ];

    return [...stations, ...defaults].slice(0, 6);
  });

  const greeting = $derived.by(() => {
    const hours = new Date().getHours();
    if (hours < 12) return 'Good morning';
    if (hours < 18) return 'Good afternoon';
    return 'Good evening';
  });
</script>

<div class="px-6 py-6 space-y-8 max-w-7xl mx-auto pb-28 contain-gpu select-none">
  
  <!-- Friendly Greeting Header -->
  <div class="flex items-center justify-between">
    <div>
      <div class="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[var(--md-sys-primary)] font-display">
        <span class="material-symbols-rounded text-base">waving_hand</span>
        <span>Welcome to Aethon</span>
      </div>
      <h1 class="text-3xl md:text-4xl font-extrabold text-white tracking-tight font-display mt-0.5">{greeting}</h1>
    </div>
  </div>

  <!-- Mood & Activity Filter Chips (Editorial Fluid Design) -->
  <div class="overflow-x-auto scrollbar-none pb-1 -mx-6 px-6">
    <div class="flex items-center gap-2.5 min-w-max">
      {#each moods as mood}
        {@const isActive = activeMood === mood}
        <button
          onclick={() => handleMoodChange(mood)}
          class="px-4 py-2 rounded-full text-xs font-semibold tracking-wide transition-all duration-300 cursor-pointer flex items-center gap-1.5 {isActive ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] shadow-md shadow-[var(--md-sys-primary)]/20 scale-[1.02]' : 'bg-white/[0.06] text-white/80 hover:bg-white/[0.12] hover:text-white border border-white/[0.06]'}"
        >
          {#if mood === 'Relax'}
            <span class="material-symbols-rounded text-sm">spa</span>
          {:else if mood === 'Workout'}
            <span class="material-symbols-rounded text-sm">fitness_center</span>
          {:else if mood === 'Focus'}
            <span class="material-symbols-rounded text-sm">psychology</span>
          {:else if mood === 'Energize'}
            <span class="material-symbols-rounded text-sm">local_fire_department</span>
          {:else if mood === 'Commute'}
            <span class="material-symbols-rounded text-sm">directions_car</span>
          {:else if mood === 'Local Only'}
            <span class="material-symbols-rounded text-sm">folder_special</span>
          {:else}
            <span class="material-symbols-rounded text-sm">all_inclusive</span>
          {/if}
          <span>{mood}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- If a specific mood is active, display the tailored mood stream -->
  {#if activeMood !== 'All'}
    <section class="space-y-3.5 pt-1 animate-fade-in">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
          <span class="material-symbols-rounded text-lg text-[var(--md-sys-primary)]">tune</span>
          {activeMood} Mixes & Tracks
        </h2>
        {#if isMoodLoading}
          <span class="text-xs text-[var(--md-sys-primary)] animate-pulse">Curating tracks...</span>
        {:else}
          <span class="text-xs text-[var(--md-sys-on-surface-variant)]">{moodTracks.length} tracks</span>
        {/if}
      </div>

      {#if moodTracks.length === 0 && !isMoodLoading}
        <div class="p-8 text-center text-white/50 text-xs border border-white/5 rounded-2xl bg-white/[0.02]">
          No tracks available for this category.
        </div>
      {:else}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
          {#each moodTracks as track}
            {@const isCurrent = playerState.currentTrack?.id === track.id}
            <button
              onclick={() => playerState.playTrack(track)}
              oncontextmenu={(e) => { e.preventDefault(); playerState.openContextMenu(track, e.clientX, e.clientY); }}
              class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border shadow-md shadow-black/30 hover:shadow-black/50 hover:scale-[1.02] transition-all duration-200 cursor-pointer flex items-center gap-3.5 group text-left w-full gpu-card"
            >
              <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
                <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition duration-300" />
                <div class="absolute inset-0 bg-black/30 backdrop-blur-[2px] {isCurrent && playerState.playback.is_playing ? 'opacity-100 bg-black/40' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity duration-200">
                  <div class="w-8 h-8 rounded-full bg-black/60 backdrop-blur-md border border-white/20 text-white flex items-center justify-center shadow-md">
                    {#if isCurrent && playerState.playback.is_playing}
                      <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-current" xmlns="http://www.w3.org/2000/svg">
                        <rect x="6" y="4" width="4" height="16" rx="1.5" />
                        <rect x="14" y="4" width="4" height="16" rx="1.5" />
                      </svg>
                    {:else}
                      <svg viewBox="2.27 0 24 24" class="w-3.5 h-3.5 fill-current" xmlns="http://www.w3.org/2000/svg">
                        <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11.04-6.86a1 1 0 0 0 0-1.72L9.5 4.28A1 1 0 0 0 8 5.14z"/>
                      </svg>
                    {/if}
                  </div>
                </div>
              </div>

              <div class="min-w-0 flex-1">
                <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                  {track.title}
                </div>
                <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
                  <span class="truncate">{sanitizeArtist(track.artist)}</span>
                </div>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  <!-- Listen Again (Recently Played from SQLite History) -->
  {#if listenAgainTracks.length > 0}
    <section class="space-y-3.5">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
          <span class="material-symbols-rounded text-lg text-[var(--md-sys-primary)]">history</span>
          Listen Again
        </h2>
        <span class="text-xs text-[var(--md-sys-on-surface-variant)]">From your listening history</span>
      </div>

      <div class="overflow-x-auto scrollbar-none -mx-6 px-6 pb-2">
        <div class="flex items-center gap-4 min-w-max">
          {#each listenAgainTracks as track}
            {@const isCurrent = playerState.currentTrack?.id === track.id}
            <div
              role="button"
              tabindex="0"
              onclick={() => playerState.playTrack(track)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') playerState.playTrack(track); }}
              oncontextmenu={(e) => { e.preventDefault(); playerState.openContextMenu(track, e.clientX, e.clientY); }}
              class="w-44 p-3 rounded-2xl m3-acrylic-card {isCurrent ? 'border-[var(--md-sys-primary)]/40 bg-[var(--md-sys-primary-container)]/15' : 'border-white/[0.06] hover:bg-white/[0.09]'} border shadow-md shadow-black/30 hover:shadow-black/50 hover:scale-[1.02] transition-all duration-200 group flex flex-col justify-between gpu-card cursor-pointer"
            >
              <div class="relative w-full aspect-square rounded-xl overflow-hidden shadow-md mb-2.5 border border-white/10">
                <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500" />
                <div class="absolute inset-0 bg-black/20 {isCurrent && playerState.playback.is_playing ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} transition-opacity duration-200 flex items-center justify-center">
                  <div class="w-11 h-11 rounded-full bg-black/60 backdrop-blur-md border border-white/20 text-white shadow-xl flex items-center justify-center hover:scale-110 active:scale-95 transition-all duration-200">
                    {#if isCurrent && playerState.playback.is_playing}
                      <svg viewBox="0 0 24 24" class="w-5 h-5 fill-current" xmlns="http://www.w3.org/2000/svg">
                        <rect x="6" y="4" width="4" height="16" rx="1.5" />
                        <rect x="14" y="4" width="4" height="16" rx="1.5" />
                      </svg>
                    {:else}
                      <svg viewBox="2.27 0 24 24" class="w-5 h-5 fill-current" xmlns="http://www.w3.org/2000/svg">
                        <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11.04-6.86a1 1 0 0 0 0-1.72L9.5 4.28A1 1 0 0 0 8 5.14z"/>
                      </svg>
                    {/if}
                  </div>
                </div>
              </div>

              <div class="min-w-0">
                <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                  {track.title}
                </div>
                <div class="text-[11px] text-white/50 truncate mt-0.5">
                  {sanitizeArtist(track.artist)}
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </section>
  {/if}

  <!-- Similar to [Artist] / Recommended Radio Row -->
  {#if recommendedArtist && similarArtistTracks.length > 0}
    <section class="space-y-3.5 pt-1">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
          <span class="material-symbols-rounded text-lg text-[var(--md-sys-primary)]">radio</span>
          Similar to {recommendedArtist}
        </h2>
        <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Automated station discovery</span>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
        {#each similarArtistTracks as track}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <button
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => { e.preventDefault(); playerState.openContextMenu(track, e.clientX, e.clientY); }}
            class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border shadow-md shadow-black/30 hover:shadow-black/50 hover:scale-[1.02] transition-all duration-200 cursor-pointer flex items-center gap-3.5 group text-left w-full gpu-card"
          >
            <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
              <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition duration-300" />
              <div class="absolute inset-0 bg-black/30 backdrop-blur-[2px] {isCurrent && playerState.playback.is_playing ? 'opacity-100 bg-black/40' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity duration-200">
                <div class="w-8 h-8 rounded-full bg-black/60 backdrop-blur-md border border-white/20 text-white flex items-center justify-center shadow-md">
                  {#if isCurrent && playerState.playback.is_playing}
                    <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-current" xmlns="http://www.w3.org/2000/svg">
                      <rect x="6" y="4" width="4" height="16" rx="1.5" />
                      <rect x="14" y="4" width="4" height="16" rx="1.5" />
                    </svg>
                  {:else}
                    <svg viewBox="2.27 0 24 24" class="w-3.5 h-3.5 fill-current" xmlns="http://www.w3.org/2000/svg">
                      <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11.04-6.86a1 1 0 0 0 0-1.72L9.5 4.28A1 1 0 0 0 8 5.14z"/>
                    </svg>
                  {/if}
                </div>
              </div>
            </div>

            <div class="min-w-0 flex-1">
              <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                {track.title}
              </div>
              <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
                <span class="truncate">{sanitizeArtist(track.artist)}</span>
              </div>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Top Picks Grid -->
  {#if displayQuickPicks.length > 0}
    <section class="space-y-3.5">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
          <span class="material-symbols-rounded text-lg text-[var(--md-sys-primary)]">bolt</span>
          Top Picks
        </h2>
        <span class="text-xs text-[var(--md-sys-on-surface-variant)]">{displayQuickPicks.length} tracks ready</span>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
        {#each displayQuickPicks as track}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <button
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => { e.preventDefault(); playerState.openContextMenu(track, e.clientX, e.clientY); }}
            class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border shadow-md shadow-black/30 hover:shadow-black/50 hover:scale-[1.02] transition-all duration-200 cursor-pointer flex items-center gap-3.5 group text-left w-full gpu-card"
          >
            <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
              <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition duration-300" />
              <div class="absolute inset-0 bg-black/30 backdrop-blur-[2px] {isCurrent && playerState.playback.is_playing ? 'opacity-100 bg-black/40' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity duration-200">
                <div class="w-8 h-8 rounded-full bg-black/60 backdrop-blur-md border border-white/20 text-white flex items-center justify-center shadow-md">
                  {#if isCurrent && playerState.playback.is_playing}
                    <svg viewBox="0 0 24 24" class="w-3.5 h-3.5 fill-current" xmlns="http://www.w3.org/2000/svg">
                      <rect x="6" y="4" width="4" height="16" rx="1.5" />
                      <rect x="14" y="4" width="4" height="16" rx="1.5" />
                    </svg>
                  {:else}
                    <svg viewBox="2.27 0 24 24" class="w-3.5 h-3.5 fill-current" xmlns="http://www.w3.org/2000/svg">
                      <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11.04-6.86a1 1 0 0 0 0-1.72L9.5 4.28A1 1 0 0 0 8 5.14z"/>
                    </svg>
                  {/if}
                </div>
              </div>
            </div>

            <div class="min-w-0 flex-1">
              <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                {track.title}
              </div>
              <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
                <span class="truncate">{sanitizeArtist(track.artist)}</span>
                {#if track.duration > 0}
                  <span class="text-white/30 shrink-0">•</span>
                  <span class="font-mono text-[10px] text-white/50 shrink-0">
                    {Math.floor(track.duration / 60)}:{Math.floor(track.duration % 60) < 10 ? '0' : ''}{Math.floor(track.duration % 60)}
                  </span>
                {/if}
              </div>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Dynamic Stations (Speed Dial) -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
        <span class="material-symbols-rounded text-lg text-[var(--md-sys-primary)]">dialpad</span>
        Dynamic Stations
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Instant station radios</span>
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
      {#each dynamicStations as dial}
        <button
          onclick={() => { playerState.activeTab = 'search'; playerState.searchYouTube(dial.query); }}
          class="p-3 rounded-2xl m3-acrylic-card hover:bg-white/10 border border-white/5 cursor-pointer shadow-md shadow-black/30 hover:shadow-black/50 hover:scale-[1.02] transition-all duration-200 text-center group flex flex-col items-center gpu-card"
        >
          <div class="w-16 h-16 rounded-full overflow-hidden mb-2 shadow-md border border-white/10 relative">
            <img loading="lazy" decoding="async" src={dial.img} alt={dial.name} class="w-full h-full object-cover group-hover:scale-110 transition duration-500" />
            <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity duration-200">
              <div class="w-8 h-8 rounded-full bg-black/60 backdrop-blur-md border border-white/20 text-white flex items-center justify-center shadow-md">
                <svg viewBox="2.27 0 24 24" class="w-4 h-4 fill-current" xmlns="http://www.w3.org/2000/svg">
                  <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11.04-6.86a1 1 0 0 0 0-1.72L9.5 4.28A1 1 0 0 0 8 5.14z"/>
                </svg>
              </div>
            </div>
          </div>
          <div class="text-xs font-bold text-white truncate max-w-full">{dial.name}</div>
          <div class="text-[10px] text-[var(--md-sys-on-surface-variant)]">{dial.type}</div>
        </button>
      {/each}
    </div>
  </section>

  <!-- Editorial Activity Mixes -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
        <span class="material-symbols-rounded text-lg text-[var(--md-sys-primary)]">auto_awesome</span>
        Curated Activity Mixes
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Tailored listening</span>
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-4 gap-3.5">
      {#each [
        { title: 'Late Night Chill', desc: 'Mellow soothing acoustic & vocals', query: 'Late Night Acoustic Chill Songs', gradient: 'from-indigo-600/30 to-violet-950/40', border: 'border-indigo-500/20', icon: 'dark_mode' },
        { title: 'High Energy Beat', desc: 'Punchy bass & motivating rhythms', query: 'Workout Energy Motivation Bass Hits', gradient: 'from-red-600/30 to-amber-950/40', border: 'border-red-500/20', icon: 'local_fire_department' },
        { title: 'Deep Focus & Study', desc: 'Instrumental strings & gentle lo-fi', query: 'Classical Study Focus Lo-Fi Instrumental', gradient: 'from-teal-600/30 to-emerald-950/40', border: 'border-teal-500/20', icon: 'psychology' },
        { title: 'Acoustic Coffeehouse', desc: 'Warm guitars & unplugged vocals', query: 'Acoustic Guitar Unplugged Coffeehouse', gradient: 'from-amber-600/30 to-orange-950/40', border: 'border-amber-500/20', icon: 'coffee' },
      ] as mood}
        <button
          onclick={() => { playerState.activeTab = 'search'; playerState.searchYouTube(mood.query); }}
          class="p-4 rounded-2xl bg-gradient-to-br {mood.gradient} border {mood.border} shadow-md shadow-black/30 hover:shadow-black/50 hover:scale-[1.02] transition-all duration-200 text-left group cursor-pointer h-28 flex flex-col justify-between gpu-card"
        >
          <div class="flex items-center justify-between">
            <span class="material-symbols-rounded text-white/80 text-xl group-hover:scale-110 transition-transform">
              {mood.icon}
            </span>
            <span class="material-symbols-rounded text-white/30 group-hover:text-white/80 text-base transition-colors">
              play_circle
            </span>
          </div>
          <div>
            <div class="font-heading font-bold text-xs text-white truncate">{mood.title}</div>
            <div class="text-[10px] text-white/60 truncate mt-0.5">{mood.desc}</div>
          </div>
        </button>
      {/each}
    </div>
  </section>

</div>

<style>
  .contain-gpu {
    contain: layout style;
  }
  .gpu-card {
    will-change: transform;
    transform: translate3d(0, 0, 0);
    contain: layout style paint;
    transition-timing-function: cubic-bezier(0.16, 1, 0.3, 1);
  }
  .animate-fade-in {
    animation: fadeIn 0.28s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translate3d(0, 6px, 0);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0);
    }
  }
</style>
