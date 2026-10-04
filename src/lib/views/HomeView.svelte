<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import type { Track, YouTubeTrack } from '../types';

  const moods = ['All', 'Relax', 'Workout', 'Focus', 'Energize', 'Commute', 'Party', 'Romance'] as const;
  type MoodType = typeof moods[number];
  let activeMood = $state<MoodType>('All');
  let moodTracks = $state<Track[]>([]);
  let isMoodLoading = $state(false);

  const moodQueries: Record<MoodType, string> = {
    'All': '',
    'Relax': 'Acoustic Chill Hindi Melodies',
    'Workout': 'High Energy Workout Hits Bass',
    'Focus': 'Instrumental Lo-Fi Study Classical Beats',
    'Energize': 'Upbeat Dance Pop High Energy',
    'Commute': 'Feel Good Acoustic Travel Songs',
    'Party': 'Club EDM Dance Party Bangers',
    'Romance': 'Soulful Romantic Hindi Love Songs',
  };

  const trendingTracks: Track[] = [
    {
      id: 'yt_fsiPzT50ZiM',
      title: 'Tum Hi Ho',
      artist: 'Arijit Singh',
      album: 'Aashiqui 2',
      duration: 262,
      file_path: 'https://www.youtube.com/watch?v=fsiPzT50ZiM',
      cover_art: 'https://i.ytimg.com/vi/fsiPzT50ZiM/hqdefault.jpg',
      format: 'Opus 256k',
      play_count: 98,
      favorite: true,
      date_added: Date.now() - 3600000,
    },
    {
      id: 'yt_BddP6PYo2gs',
      title: 'Kesariya',
      artist: 'Arijit Singh, Pritam',
      album: 'Brahmastra',
      duration: 268,
      file_path: 'https://www.youtube.com/watch?v=BddP6PYo2gs',
      cover_art: 'https://i.ytimg.com/vi/BddP6PYo2gs/hqdefault.jpg',
      format: 'Opus 256k',
      play_count: 85,
      favorite: true,
      date_added: Date.now() - 7200000,
    },
    {
      id: 'yt_E_SbwSe15y0',
      title: 'Ve Haaniyaan',
      artist: 'Avvy Sra, Danny, Sagar',
      album: 'Ve Haaniyaan',
      duration: 236,
      file_path: 'https://www.youtube.com/watch?v=E_SbwSe15y0',
      cover_art: 'https://i.ytimg.com/vi/E_SbwSe15y0/hqdefault.jpg',
      format: 'Opus 256k',
      play_count: 77,
      favorite: false,
      date_added: Date.now() - 14400000,
    },
    {
      id: 'yt_mH_LFkWxpI0',
      title: 'Lover',
      artist: 'Diljit Dosanjh',
      album: 'MoonChild Era',
      duration: 191,
      file_path: 'https://www.youtube.com/watch?v=mH_LFkWxpI0',
      cover_art: 'https://i.ytimg.com/vi/mH_LFkWxpI0/hqdefault.jpg',
      format: 'Opus 256k',
      play_count: 91,
      favorite: true,
      date_added: Date.now() - 28800000,
    },
    {
      id: 'yt_gvyUuxdRdR4',
      title: 'Raataan Lambiyan',
      artist: 'Jubin Nautiyal, Asees Kaur',
      album: 'Shershaah',
      duration: 230,
      file_path: 'https://www.youtube.com/watch?v=gvyUuxdRdR4',
      cover_art: 'https://i.ytimg.com/vi/gvyUuxdRdR4/hqdefault.jpg',
      format: 'Opus 256k',
      play_count: 64,
      favorite: false,
      date_added: Date.now() - 40000000,
    },
    {
      id: 'yt_tMgEYaclnQ0',
      title: 'On Top',
      artist: 'Karan Aujla',
      album: 'Four Me',
      duration: 184,
      file_path: 'https://www.youtube.com/watch?v=tMgEYaclnQ0',
      cover_art: 'https://i.ytimg.com/vi/tMgEYaclnQ0/hqdefault.jpg',
      format: 'Opus 256k',
      play_count: 82,
      favorite: true,
      date_added: Date.now() - 50000000,
    },
  ];

  let liveTrending = $state<Track[]>([]);
  let similarArtistTracks = $state<Track[]>([]);

  // Compute recommended artist based on history or library
  let recommendedArtist = $derived.by(() => {
    if (playerState.playHistoryTracks.length > 0) {
      return playerState.playHistoryTracks[0].artist.split(',')[0].trim();
    }
    if (playerState.currentTrack) {
      return playerState.currentTrack.artist.split(',')[0].trim();
    }
    if (playerState.library.length > 0) {
      return playerState.library[0].artist.split(',')[0].trim();
    }
    return 'Arijit Singh';
  });

  async function handleMoodChange(mood: MoodType) {
    activeMood = mood;
    if (mood === 'All') {
      moodTracks = [];
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
          artist: yt.artist || mood,
          album: `${mood} Session`,
          duration: yt.duration || 200,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 10 + idx,
          favorite: false,
          date_added: Date.now(),
        }));
      }
    } catch (e) {
      console.warn('Mood fetch failed:', e);
    } finally {
      isMoodLoading = false;
    }
  }

  async function loadSimilarArtist(artist: string) {
    try {
      const results = await invoke<YouTubeTrack[]>('search_youtube', { query: `${artist} top popular songs`, limit: 6 });
      if (results && results.length > 0) {
        similarArtistTracks = results.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: yt.artist || artist,
          album: `${artist} Dynamic Radio`,
          duration: yt.duration || 215,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 40 + idx * 3,
          favorite: false,
          date_added: Date.now(),
        }));
      }
    } catch (e) {
      console.warn('Similar artist tracks failed:', e);
    }
  }

  onMount(async () => {
    try {
      const results = await invoke<YouTubeTrack[]>('search_youtube', { query: 'trending indian songs', limit: 6 });
      if (results && results.length > 0) {
        liveTrending = results.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: yt.artist || 'Popular',
          album: 'Top Daily Charts',
          duration: yt.duration || 210,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 50 + idx * 5,
          favorite: false,
          date_added: Date.now(),
        }));
      }
    } catch (e) {
      console.warn('Could not load trending:', e);
    }
  });

  let lastLoadedArtist = '';
  $effect(() => {
    const artist = recommendedArtist;
    if (artist && artist !== lastLoadedArtist) {
      lastLoadedArtist = artist;
      loadSimilarArtist(artist);
    }
  });

  let displayQuickPicks = $derived(
    playerState.library.length > 0 
      ? playerState.library.slice(0, 6) 
      : (liveTrending.length > 0 ? liveTrending : trendingTracks)
  );

  let listenAgainTracks = $derived(
    playerState.playHistoryTracks.length > 0
      ? playerState.playHistoryTracks.slice(0, 8)
      : (playerState.library.length > 0 ? playerState.library.slice(0, 8) : trendingTracks)
  );

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
        <span>Welcome Back</span>
      </div>
      <h1 class="text-3xl md:text-4xl font-extrabold text-white tracking-tight font-display mt-0.5">{greeting}</h1>
    </div>

    <div class="hidden sm:flex items-center gap-2 text-xs font-medium text-white/50 bg-white/[0.04] px-3.5 py-1.5 rounded-full border border-white/[0.06]">
      <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
      <span>Lossless Engine Ready</span>
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
          {:else if mood === 'Party'}
            <span class="material-symbols-rounded text-sm">celebration</span>
          {:else if mood === 'Romance'}
            <span class="material-symbols-rounded text-sm">favorite</span>
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
        <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
          <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">tune</span>
          {activeMood} Mixes & Sessions
        </h2>
        {#if isMoodLoading}
          <span class="text-xs text-[var(--md-sys-primary)] animate-pulse">Curating tracks...</span>
        {:else}
          <span class="text-xs text-[var(--md-sys-on-surface-variant)]">{moodTracks.length} tracks</span>
        {/if}
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
        {#each moodTracks as track}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <button
            onclick={() => playerState.playTrack(track)}
            class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40 shadow-sm' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border cursor-pointer transition flex items-center gap-3.5 group text-left w-full gpu-card"
          >
            <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
              <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
              <div class="absolute inset-0 bg-black/40 {isCurrent ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition">
                <span class="material-symbols-rounded {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} text-xl filled">
                  {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                </span>
              </div>
            </div>

            <div class="min-w-0 flex-1">
              <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                {track.title}
              </div>
              <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
                <span class="truncate">{track.artist}</span>
              </div>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Listen Again (Recently Played from SQLite History) -->
  <section class="space-y-3.5">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">history</span>
        Listen Again
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">From your listening history</span>
    </div>

    <div class="overflow-x-auto scrollbar-none -mx-6 px-6 pb-2">
      <div class="flex items-center gap-4 min-w-max">
        {#each listenAgainTracks as track}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            class="w-44 p-3 rounded-2xl m3-acrylic-card hover:bg-white/[0.09] border border-white/[0.06] transition-all duration-300 group flex flex-col justify-between gpu-card"
          >
            <div class="relative w-full aspect-square rounded-xl overflow-hidden shadow-md mb-2.5 border border-white/10">
              <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500" />
              <button
                onclick={() => playerState.playTrack(track)}
                aria-label="Play {track.title}"
                class="absolute bottom-2 right-2 w-9 h-9 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] shadow-lg flex items-center justify-center opacity-0 group-hover:opacity-100 hover:scale-110 active:scale-95 transition-all duration-200 cursor-pointer"
              >
                <span class="material-symbols-rounded text-xl filled">
                  {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                </span>
              </button>
            </div>

            <div class="min-w-0">
              <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                {track.title}
              </div>
              <div class="text-[11px] text-white/50 truncate mt-0.5">
                {track.artist}
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </section>

  <!-- Similar to [Artist] / Recommended Radio Row -->
  {#if similarArtistTracks.length > 0}
    <section class="space-y-3.5 pt-1">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
          <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">radio</span>
          Similar to {recommendedArtist}
        </h2>
        <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Automated station discovery</span>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
        {#each similarArtistTracks as track}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <button
            onclick={() => playerState.playTrack(track)}
            class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40 shadow-sm' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border cursor-pointer transition flex items-center gap-3.5 group text-left w-full gpu-card"
          >
            <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
              <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
              <div class="absolute inset-0 bg-black/40 {isCurrent ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition">
                <span class="material-symbols-rounded {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} text-xl filled">
                  {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                </span>
              </div>
            </div>

            <div class="min-w-0 flex-1">
              <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
                {track.title}
              </div>
              <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
                <span class="truncate">{track.artist}</span>
              </div>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <!-- Quick Picks Grid -->
  <section class="space-y-3.5">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">bolt</span>
        Quick Picks
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">{displayQuickPicks.length} tracks ready</span>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
      {#each displayQuickPicks as track}
        {@const isCurrent = playerState.currentTrack?.id === track.id}
        <button
          onclick={() => playerState.playTrack(track)}
          class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40 shadow-sm' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border cursor-pointer transition flex items-center gap-3.5 group text-left w-full gpu-card"
        >
          <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
            <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
            <div class="absolute inset-0 bg-black/40 {isCurrent ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition">
              <span class="material-symbols-rounded {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} text-xl filled">
                {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
              </span>
            </div>
          </div>

          <div class="min-w-0 flex-1">
            <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
              {track.title}
            </div>
            <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
              <span class="truncate">{track.artist}</span>
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

  <!-- Dynamic Stations (Speed Dial) -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">dialpad</span>
        Dynamic Stations
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Instant station radios</span>
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
      {#each [
        { name: 'Arijit Singh', type: 'Artist Radio', img: 'https://i.ytimg.com/vi/fsiPzT50ZiM/hqdefault.jpg', query: 'Arijit Singh' },
        { name: 'Diljit Dosanjh', type: 'Punjabi Hits', img: 'https://i.ytimg.com/vi/mH_LFkWxpI0/hqdefault.jpg', query: 'Diljit Dosanjh' },
        { name: 'Romantic Melodies', type: 'Soulful Mix', img: 'https://i.ytimg.com/vi/BddP6PYo2gs/hqdefault.jpg', query: 'Bollywood Romantic Songs' },
        { name: 'Karan Aujla', type: 'Pop Radio', img: 'https://i.ytimg.com/vi/tMgEYaclnQ0/hqdefault.jpg', query: 'Karan Aujla' },
        { name: 'South Melodies', type: 'Regional Mix', img: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=160&auto=format&fit=crop', query: 'South Indian Hits' },
        { name: 'Lo-Fi Chill', type: 'Study Beats', img: 'https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=160&auto=format&fit=crop', query: 'Indian Lo-Fi Chill' },
      ] as dial}
        <button
          onclick={() => { playerState.activeTab = 'search'; playerState.searchYouTube(dial.query); }}
          class="p-3 rounded-2xl m3-acrylic-card hover:bg-white/10 border border-white/5 cursor-pointer transition text-center group flex flex-col items-center gpu-card"
        >
          <div class="w-16 h-16 rounded-full overflow-hidden mb-2 shadow-md border border-white/10 relative">
            <img loading="lazy" decoding="async" src={dial.img} alt={dial.name} class="w-full h-full object-cover group-hover:scale-110 transition duration-500" />
            <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition">
              <span class="material-symbols-rounded filled text-white text-xl">play_arrow</span>
            </div>
          </div>
          <div class="text-xs font-bold text-white truncate max-w-full">{dial.name}</div>
          <div class="text-[10px] text-[var(--md-sys-on-surface-variant)]">{dial.type}</div>
        </button>
      {/each}
    </div>
  </section>

  <!-- Top Daily Charts -->
  <section class="space-y-3.5 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">leaderboard</span>
        Top Charts • Regional Daily
      </h2>
      <button onclick={() => { playerState.activeTab = 'search'; playerState.searchYouTube('Top 50 Songs Daily'); }} class="text-xs font-bold text-[var(--md-sys-primary)] hover:underline">
        Explore Chart
      </button>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
      {#each (liveTrending.length > 0 ? liveTrending : trendingTracks).slice(0, 6) as track, idx}
        {@const isCurrent = playerState.currentTrack?.id === track.id}
        <button
          onclick={() => playerState.playTrack(track)}
          class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40 shadow-sm' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border cursor-pointer transition flex items-center gap-3.5 group text-left w-full gpu-card"
        >
          <div class="w-6 text-center font-heading font-black text-sm {idx === 0 ? 'text-amber-400' : idx === 1 ? 'text-slate-300' : idx === 2 ? 'text-amber-600' : 'text-white/40'} shrink-0">
            {idx + 1}
          </div>
          <div class="relative w-12 h-12 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
            <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
            <div class="absolute inset-0 bg-black/40 {isCurrent ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition">
              <span class="material-symbols-rounded {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} text-lg filled">
                {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
              </span>
            </div>
          </div>
          <div class="min-w-0 flex-1">
            <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition">
              {track.title}
            </div>
            <div class="text-[11px] text-[var(--md-sys-on-surface-variant)]/75 truncate mt-0.5 flex items-center gap-1.5">
              <span class="truncate">{track.artist}</span>
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

  <!-- Editorial Activity Mixes -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">auto_awesome</span>
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
          class="p-4 rounded-2xl bg-gradient-to-br {mood.gradient} border {mood.border} hover:scale-[1.02] hover:brightness-110 transition-all text-left group cursor-pointer h-28 flex flex-col justify-between shadow-sm gpu-card"
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
  /* Composite-only hardware-accelerated transforms avoiding repaint triggers */
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
