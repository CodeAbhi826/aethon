<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import type { Track, YouTubeTrack } from '../types';

  const trendingIndiaTracks: Track[] = [
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
      id: 'yt_k3g_Wj3Fda4',
      title: 'Ve Haaniyaan',
      artist: 'Avvy Sra, Danny, Sagar',
      album: 'Ve Haaniyaan',
      duration: 236,
      file_path: 'https://www.youtube.com/watch?v=k3g_Wj3Fda4',
      cover_art: 'https://i.ytimg.com/vi/k3g_Wj3Fda4/hqdefault.jpg',
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

  onMount(async () => {
    try {
      const results = await invoke<YouTubeTrack[]>('search_youtube', { query: 'trending indian songs', limit: 6 });
      if (results && results.length > 0) {
        liveTrending = results.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: yt.artist || 'Trending India',
          album: 'YouTube Music India Charts',
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
      console.warn('Could not load live India trending:', e);
    }
  });

  let displayTracks = $derived(
    playerState.library.length > 0 
      ? playerState.library.slice(0, 6) 
      : (liveTrending.length > 0 ? liveTrending : trendingIndiaTracks)
  );
</script>

<div class="px-6 py-6 space-y-8 max-w-7xl mx-auto pb-28">
  
  <!-- Friendly Greeting Header -->
  <div>
    <div class="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[var(--md-sys-primary)] font-display">
      <span class="material-symbols-rounded text-base">waving_hand</span>
      <span>Welcome Back</span>
    </div>
    <h1 class="text-3xl md:text-4xl font-extrabold text-white tracking-tight font-display mt-0.5">Good evening</h1>
  </div>

  <!-- Quick Picks (ArchiveTune Homescreen.kt Skeleton) -->
  <section class="space-y-3.5">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">bolt</span>
        Quick Picks
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">{displayTracks.length} tracks ready</span>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
      {#each displayTracks as track}
        {@const isCurrent = playerState.currentTrack?.id === track.id}
        <button
          onclick={() => playerState.playTrack(track)}
          class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40 shadow-sm' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border cursor-pointer transition flex items-center gap-3.5 group text-left w-full"
        >
          <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
            <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
            <div class="absolute inset-0 bg-black/40 {isCurrent ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition">
              <span class="material-symbols-rounded {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} text-xl filled">
                {isCurrent && playerState.playback.isPlaying ? 'equalizer' : 'play_arrow'}
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

  <!-- Speed Dial (ArchiveTune Style Artist & Station Radios) -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">dialpad</span>
        Speed Dial
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Instant station radios</span>
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
      {#each [
        { name: 'Arijit Singh', type: 'Artist Radio', img: 'https://i.ytimg.com/vi/fsiPzT50ZiM/hqdefault.jpg', query: 'Arijit Singh' },
        { name: 'Diljit Dosanjh', type: 'Punjabi Hits', img: 'https://i.ytimg.com/vi/mH_LFkWxpI0/hqdefault.jpg', query: 'Diljit Dosanjh' },
        { name: 'Bollywood Love', type: 'Romance Mix', img: 'https://i.ytimg.com/vi/BddP6PYo2gs/hqdefault.jpg', query: 'Bollywood Romantic Songs' },
        { name: 'Karan Aujla', type: 'Punjabi Pop', img: 'https://i.ytimg.com/vi/tMgEYaclnQ0/hqdefault.jpg', query: 'Karan Aujla' },
        { name: 'South Hits', type: 'Regional Mix', img: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=160&auto=format&fit=crop', query: 'South Indian Hits' },
        { name: 'Lo-Fi India', type: 'Chill Beats', img: 'https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=160&auto=format&fit=crop', query: 'Indian Lo-Fi Chill' },
      ] as dial}
        <button
          onclick={() => { playerState.activeTab = 'search'; playerState.searchYouTube(dial.query); }}
          class="p-3 rounded-2xl m3-acrylic-card hover:bg-white/10 border border-white/5 cursor-pointer transition text-center group flex flex-col items-center"
        >
          <div class="w-16 h-16 rounded-full overflow-hidden mb-2 shadow-md border border-white/10 relative">
            <img src={dial.img} alt={dial.name} class="w-full h-full object-cover group-hover:scale-110 transition duration-500" />
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

  <!-- India Top Charts (Ranked 1 to 6) -->
  <section class="space-y-3.5 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">leaderboard</span>
        Top Charts • India Daily
      </h2>
      <button onclick={() => { playerState.activeTab = 'search'; playerState.searchYouTube('Top 50 India Songs'); }} class="text-xs font-bold text-[var(--md-sys-primary)] hover:underline">
        Explore Chart
      </button>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
      {#each (liveTrending.length > 0 ? liveTrending : trendingIndiaTracks).slice(0, 6) as track, idx}
        {@const isCurrent = playerState.currentTrack?.id === track.id}
        <button
          onclick={() => playerState.playTrack(track)}
          class="p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/40 shadow-sm' : 'm3-acrylic-card hover:bg-white/10 border-white/5'} border cursor-pointer transition flex items-center gap-3.5 group text-left w-full"
        >
          <div class="w-6 text-center font-heading font-black text-sm {idx === 0 ? 'text-amber-400' : idx === 1 ? 'text-slate-300' : idx === 2 ? 'text-amber-600' : 'text-white/40'} shrink-0">
            {idx + 1}
          </div>
          <div class="relative w-12 h-12 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
            <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
            <div class="absolute inset-0 bg-black/40 {isCurrent ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition">
              <span class="material-symbols-rounded {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} text-lg filled">
                {isCurrent && playerState.playback.isPlaying ? 'equalizer' : 'play_arrow'}
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

  <!-- Mood & Activity Mixes (Apple Music Style) -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">genres</span>
        Mood & Activity Mixes
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Tailored listening</span>
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-4 gap-3.5">
      {#each [
        { title: 'Late Night Chill', desc: 'Mellow soothing Hindi & acoustic', query: 'Late Night Hindi Chill Songs', gradient: 'from-indigo-600/30 to-violet-950/40', border: 'border-indigo-500/20', icon: 'dark_mode' },
        { title: 'Punjabi Energy', desc: 'High octane bhangra & bass', query: 'Punjabi Workout Energy Hits', gradient: 'from-red-600/30 to-amber-950/40', border: 'border-red-500/20', icon: 'local_fire_department' },
        { title: 'Deep Focus & Study', desc: 'Instrumental sitar & lo-fi beats', query: 'Indian Classical Study Focus Lo-Fi', gradient: 'from-teal-600/30 to-emerald-950/40', border: 'border-teal-500/20', icon: 'psychology' },
        { title: 'Acoustic Coffeehouse', desc: 'Warm guitars & unplugged vocals', query: 'Bollywood Acoustic Unplugged', gradient: 'from-amber-600/30 to-orange-950/40', border: 'border-amber-500/20', icon: 'coffee' },
      ] as mood}
        <button
          onclick={() => { playerState.activeTab = 'search'; playerState.searchYouTube(mood.query); }}
          class="p-4 rounded-2xl bg-gradient-to-br {mood.gradient} border {mood.border} hover:scale-[1.02] hover:brightness-110 transition-all text-left group cursor-pointer h-28 flex flex-col justify-between shadow-sm"
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
