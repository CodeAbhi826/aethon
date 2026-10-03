<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { Track } from '../types';

  // Sample quick picks & keep listening if library is empty
  const sampleTracks: Track[] = [
    {
      id: 'sample_1',
      title: 'Bling-Bang-Bang-Born',
      artist: 'Creepy Nuts',
      album: 'THE FIRST TAKE',
      duration: 178,
      file_path: 'sample_1',
      cover_art: 'https://images.unsplash.com/photo-1614613535308-eb5fbd3d2c17?w=400&q=80',
      format: 'FLAC 96k',
      play_count: 42,
      favorite: true,
      date_added: Date.now() - 3600000,
    },
    {
      id: 'sample_2',
      title: 'Idol (アイドル)',
      artist: 'YOASOBI',
      album: 'THE BOOK 3',
      duration: 213,
      file_path: 'sample_2',
      cover_art: 'https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=400&q=80',
      format: 'FLAC',
      play_count: 38,
      favorite: true,
      date_added: Date.now() - 7200000,
    },
    {
      id: 'sample_3',
      title: 'Subtitle',
      artist: 'Official HIGE DANDISM',
      album: 'Rejoice',
      duration: 305,
      file_path: 'sample_3',
      cover_art: 'https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=400&q=80',
      format: 'ALAC',
      play_count: 29,
      favorite: false,
      date_added: Date.now() - 14400000,
    },
    {
      id: 'sample_4',
      title: 'Sparkle (Movie ver.)',
      artist: 'RADWIMPS',
      album: 'Your Name OST',
      duration: 537,
      file_path: 'sample_4',
      cover_art: 'https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=400&q=80',
      format: 'FLAC 96k',
      play_count: 51,
      favorite: true,
      date_added: Date.now() - 28800000,
    },
  ];

  let displayTracks = $derived(
    playerState.library.length > 0 ? playerState.library.slice(0, 8) : sampleTracks
  );
</script>

<div class="p-6 space-y-8 max-w-7xl mx-auto">
  
  <!-- Hero / Quick Picks (ArchiveTune skeleton) -->
  <section>
    <div class="flex items-center justify-between mb-4">
      <div>
        <h2 class="font-heading font-bold text-xl text-white">Quick Picks</h2>
        <p class="text-xs text-[#9ea2b0]">Jump back into your frequent listening</p>
      </div>
      <button
        onclick={() => playerState.activeTab = 'library'}
        class="text-xs font-semibold text-[#9ea2b0] hover:text-white transition-colors"
      >
        View All
      </button>
    </div>

    <!-- 2x2 or 4x1 Quick Picks Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {#each displayTracks.slice(0, 4) as track}
        <div
          onclick={() => playerState.playTrack(track)}
          class="flex items-center gap-3.5 p-3 rounded-2xl bg-[#1e2026]/70 hover:bg-[#262830] border border-white/[0.05] hover:border-white/[0.1] transition-all cursor-pointer group shadow-sm"
        >
          <div class="relative w-14 h-14 rounded-xl overflow-hidden shrink-0 bg-[#252730] border border-white/[0.06]">
            {#if track.cover_art}
              <img src={track.cover_art} alt="Cover" class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
            {:else}
              <div class="w-full h-full flex items-center justify-center">
                <span class="material-symbols-rounded text-[#9ea2b0]">music_note</span>
              </div>
            {/if}
            <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
              <span class="material-symbols-rounded text-white text-[24px] filled">play_arrow</span>
            </div>
          </div>

          <div class="min-w-0 flex-1">
            <h4 class="font-heading font-semibold text-sm text-white truncate group-hover:text-emerald-300 transition-colors">
              {track.title}
            </h4>
            <p class="text-xs text-[#9ea2b0] truncate mt-0.5">{track.artist}</p>
            <span class="inline-block text-[10px] font-mono px-1.5 py-0.2 rounded bg-white/[0.06] text-[#9ea2b0] mt-1">
              {track.format}
            </span>
          </div>
        </div>
      {/each}
    </div>
  </section>

  <!-- Keep Listening Carousel -->
  <section>
    <div class="flex items-center justify-between mb-4">
      <h2 class="font-heading font-bold text-xl text-white">Keep Listening</h2>
      <span class="text-xs text-[#6b6e7c]">Albums & Playlists</span>
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
      {#each displayTracks as track}
        <div
          onclick={() => playerState.playTrack(track)}
          class="flex flex-col p-3 rounded-2xl bg-[#1e2026]/50 hover:bg-[#24262f] border border-white/[0.04] hover:border-white/[0.08] transition-all cursor-pointer group"
        >
          <div class="relative w-full aspect-square rounded-xl overflow-hidden mb-3 bg-[#262830] border border-white/[0.06] shadow-md">
            {#if track.cover_art}
              <img src={track.cover_art} alt="Cover" class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
            {:else}
              <div class="w-full h-full flex items-center justify-center">
                <span class="material-symbols-rounded text-white/40 text-[32px]">album</span>
              </div>
            {/if}
            <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
              <div class="w-10 h-10 rounded-full bg-white text-[#111215] flex items-center justify-center shadow-lg">
                <span class="material-symbols-rounded text-[24px] filled">play_arrow</span>
              </div>
            </div>
          </div>

          <h4 class="font-heading font-semibold text-xs text-white truncate">{track.title}</h4>
          <p class="text-[11px] text-[#9ea2b0] truncate mt-0.5">{track.artist}</p>
        </div>
      {/each}
    </div>
  </section>

  <!-- Speed Dial / Top Artists -->
  <section>
    <h2 class="font-heading font-bold text-lg text-white mb-3">Featured Artists</h2>
    <div class="flex items-center gap-3 overflow-x-auto pb-2 scrollbar-none">
      {#each ['YOASOBI', 'Official HIGE DANDISM', 'RADWIMPS', 'Creepy Nuts', 'Aimer', 'Kenshi Yonezu', 'Vaundy'] as artist}
        <button
          onclick={() => {
            playerState.activeTab = 'search';
            playerState.searchYouTube(artist);
          }}
          class="flex items-center gap-2.5 px-4 py-2 rounded-full bg-[#1e2026] hover:bg-[#282a33] border border-white/[0.06] text-xs font-medium text-white transition-all shrink-0 hover:scale-105"
        >
          <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span>{artist}</span>
        </button>
      {/each}
    </div>
  </section>

</div>
