<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { Track } from '../types';

  const sampleTracks: Track[] = [
    {
      id: 'sample_1',
      title: 'Midnight City',
      artist: 'M83',
      album: 'Hurry Up, We\'re Dreaming',
      duration: 244,
      file_path: 'sample_1',
      cover_art: 'https://images.unsplash.com/photo-1614613535308-eb5fbd3d2c17?w=400&auto=format&fit=crop',
      format: 'FLAC 96k',
      play_count: 52,
      favorite: true,
      date_added: Date.now() - 3600000,
    },
    {
      id: 'sample_2',
      title: 'Get Lucky',
      artist: 'Daft Punk ft. Pharrell',
      album: 'Random Access Memories',
      duration: 248,
      file_path: 'sample_2',
      cover_art: 'https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=400&auto=format&fit=crop',
      format: 'FLAC',
      play_count: 48,
      favorite: true,
      date_added: Date.now() - 7200000,
    },
    {
      id: 'sample_3',
      title: 'Resonance',
      artist: 'HOME',
      album: 'Odyssey',
      duration: 212,
      file_path: 'sample_3',
      cover_art: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=400&auto=format&fit=crop',
      format: 'ALAC',
      play_count: 39,
      favorite: false,
      date_added: Date.now() - 14400000,
    },
    {
      id: 'sample_4',
      title: 'Blinding Lights',
      artist: 'The Weeknd',
      album: 'After Hours',
      duration: 200,
      file_path: 'sample_4',
      cover_art: 'https://images.unsplash.com/photo-1508700115892-45ecd05ae2ad?w=400&auto=format&fit=crop',
      format: 'FLAC 96k',
      play_count: 61,
      favorite: true,
      date_added: Date.now() - 28800000,
    },
    {
      id: 'sample_5',
      title: 'Clair de Lune',
      artist: 'Claude Debussy',
      album: 'Suite Bergamasque',
      duration: 304,
      file_path: 'sample_5',
      cover_art: 'https://images.unsplash.com/photo-1507838153414-b4b713384a76?w=400&auto=format&fit=crop',
      format: 'FLAC 192k',
      play_count: 34,
      favorite: false,
      date_added: Date.now() - 40000000,
    },
    {
      id: 'sample_6',
      title: 'After Dark',
      artist: 'Mr.Kitty',
      album: 'Time',
      duration: 259,
      file_path: 'sample_6',
      cover_art: 'https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=400&auto=format&fit=crop',
      format: 'Opus 256k',
      play_count: 45,
      favorite: true,
      date_added: Date.now() - 50000000,
    },
  ];

  let displayTracks = $derived(
    playerState.library.length > 0 ? playerState.library.slice(0, 6) : sampleTracks
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
        <button
          onclick={() => playerState.playTrack(track)}
          class="p-3 rounded-2xl m3-acrylic-card hover:bg-white/10 border border-white/5 cursor-pointer transition flex items-center gap-3.5 group text-left w-full"
        >
          <div class="relative w-14 h-14 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
            <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
            <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition">
              <span class="material-symbols-rounded text-white text-xl filled">play_arrow</span>
            </div>
          </div>

          <div class="min-w-0 flex-1">
            <div class="text-xs font-bold text-white truncate group-hover:text-[var(--md-sys-primary)] transition">
              {track.title}
            </div>
            <div class="text-[11px] text-[var(--md-sys-on-surface-variant)] truncate mt-0.5">
              {track.artist}
            </div>
            <div class="flex items-center gap-1.5 mt-1">
              <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-[var(--md-sys-surface-container)] text-[var(--md-sys-primary)]">
                {track.format}
              </span>
            </div>
          </div>
        </button>
      {/each}
    </div>
  </section>

  <!-- Speed Dial (From Mockup) -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">dialpad</span>
        Speed Dial
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Frequently played stations & artists</span>
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
      {#each [
        { name: 'M83 Radio', type: 'Station', img: 'https://images.unsplash.com/photo-1614613535308-eb5fbd3d2c17?w=160&auto=format&fit=crop' },
        { name: 'Daft Punk', type: 'Artist Mix', img: 'https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=160&auto=format&fit=crop' },
        { name: 'Chillwave Synth', type: 'Mood Radio', img: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=160&auto=format&fit=crop' },
        { name: 'The Weeknd', type: 'Artist Mix', img: 'https://images.unsplash.com/photo-1508700115892-45ecd05ae2ad?w=160&auto=format&fit=crop' },
        { name: 'Piano Masters', type: 'Classical', img: 'https://images.unsplash.com/photo-1507838153414-b4b713384a76?w=160&auto=format&fit=crop' },
        { name: 'Dark Wave', type: 'Synth', img: 'https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=160&auto=format&fit=crop' },
      ] as dial, i}
        <button
          onclick={() => playerState.playTrack(sampleTracks[i % sampleTracks.length])}
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

  <!-- Keep Listening -->
  <section class="space-y-3 pt-2">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">resume</span>
        Keep Listening
      </h2>
      <button onclick={() => playerState.activeTab = 'library'} class="text-xs font-bold text-[var(--md-sys-primary)] hover:underline">
        View All
      </button>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each displayTracks.slice(0, 3) as track}
        <button
          onclick={() => playerState.playTrack(track)}
          class="p-4 rounded-3xl m3-acrylic-card hover:bg-white/10 border border-white/5 cursor-pointer transition flex items-center gap-4 group text-left w-full"
        >
          <div class="w-16 h-16 rounded-2xl overflow-hidden shadow-lg border border-white/10 shrink-0">
            <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
          </div>
          <div class="min-w-0 flex-1">
            <div class="text-xs font-bold uppercase tracking-wider text-[var(--md-sys-primary)] font-display">Album</div>
            <div class="text-sm font-extrabold text-white truncate font-display">{track.title}</div>
            <div class="text-xs text-[var(--md-sys-on-surface-variant)] truncate">{track.artist}</div>
          </div>
        </button>
      {/each}
    </div>
  </section>

</div>
