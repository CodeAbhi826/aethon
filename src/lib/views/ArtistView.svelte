<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import type { Track, YouTubeTrack } from '../types';

  let artistTracks = $state<Track[]>([]);
  let isLoadingTracks = $state(true);

  let artistName = $derived(playerState.selectedArtist || 'Artist');

  // Find local tracks by this artist
  let localArtistTracks = $derived(
    playerState.library.filter(
      (t) => t.artist.toLowerCase().includes(artistName.toLowerCase())
    )
  );

  onMount(async () => {
    await fetchArtistData();
  });

  $effect(() => {
    if (playerState.selectedArtist) {
      fetchArtistData();
    }
  });

  async function fetchArtistData() {
    isLoadingTracks = true;
    try {
      const res = await invoke<YouTubeTrack[]>('search_youtube', {
        query: `${artistName} top tracks`,
        limit: 8,
      });

      if (res && res.length > 0) {
        artistTracks = res.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: yt.artist || artistName,
          album: 'Top Releases',
          duration: yt.duration || 210,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 100 - idx * 5,
          favorite: false,
          date_added: Date.now(),
        }));
      } else {
        artistTracks = localArtistTracks;
      }
    } catch {
      artistTracks = localArtistTracks;
    } finally {
      isLoadingTracks = false;
    }
  }

  function handlePlayAll() {
    const list = artistTracks.length > 0 ? artistTracks : localArtistTracks;
    if (list.length > 0) {
      playerState.queue = list.slice(1);
      playerState.playTrack(list[0]);
    }
  }

  function handlePlayRadio() {
    playerState.searchYouTube(`${artistName} Radio Mix`);
    playerState.activeTab = 'search';
  }
</script>

<div class="px-6 py-6 space-y-8 max-w-7xl mx-auto pb-28 select-none">
  
  <!-- Back Button -->
  <button
    onclick={() => playerState.activeTab = 'home'}
    class="flex items-center gap-2 text-xs font-semibold text-[var(--md-sys-primary)] hover:underline cursor-pointer"
  >
    <span class="material-symbols-rounded text-base">arrow_back</span>
    <span>Back to Home</span>
  </button>

  <!-- Artist Hero Banner -->
  <div class="p-6 md:p-8 rounded-3xl bg-gradient-to-br from-white/[0.06] to-white/[0.01] border border-white/10 flex flex-col md:flex-row items-center gap-6 shadow-2xl relative overflow-hidden">
    
    <!-- Ambient Backdrop Glow -->
    <div class="absolute -top-24 -left-24 w-80 h-80 rounded-full bg-[var(--md-sys-primary)]/15 blur-3xl pointer-events-none"></div>

    <div class="w-32 h-32 md:w-40 md:h-40 rounded-full overflow-hidden shadow-2xl border-2 border-white/20 shrink-0 relative bg-black/40">
      {#if artistTracks[0]?.cover_art}
        <img src={artistTracks[0].cover_art} alt={artistName} class="w-full h-full object-cover" />
      {:else}
        <div class="w-full h-full flex items-center justify-center">
          <span class="material-symbols-rounded text-5xl text-white/40">person</span>
        </div>
      {/if}
    </div>

    <div class="flex-1 text-center md:text-left space-y-2.5 min-w-0">
      <div class="flex items-center justify-center md:justify-start gap-2">
        <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-sm filled">verified</span>
        <span class="text-xs uppercase tracking-wider font-bold text-[var(--md-sys-primary)] font-display">Verified Artist</span>
      </div>
      <h1 class="text-3xl md:text-5xl font-black text-white tracking-tight font-display truncate">
        {artistName}
      </h1>
      <p class="text-xs text-[var(--md-sys-on-surface-variant)]/75">
        Popular on Aethon • Curated Top Hits & Regional Discography
      </p>

      <div class="flex items-center justify-center md:justify-start gap-3 pt-2">
        <button
          onclick={handlePlayAll}
          class="flex items-center gap-2 px-5 py-2.5 rounded-full bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-bold hover:brightness-110 shadow-lg cursor-pointer transition"
        >
          <span class="material-symbols-rounded text-lg filled">play_arrow</span>
          <span>Play Top Songs</span>
        </button>

        <button
          onclick={handlePlayRadio}
          class="flex items-center gap-2 px-4 py-2.5 rounded-full bg-white/10 hover:bg-white/15 text-white text-xs font-semibold border border-white/10 cursor-pointer transition"
        >
          <span class="material-symbols-rounded text-lg">radio</span>
          <span>Artist Radio</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Top Tracks Section -->
  <section class="space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2 font-display">
        <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">trending_up</span>
        Top Tracks
      </h2>
      <span class="text-xs text-[var(--md-sys-on-surface-variant)]/60">{artistTracks.length} popular tracks</span>
    </div>

    {#if isLoadingTracks}
      <div class="flex items-center justify-center py-12 text-[#9ea2b0]">
        <svg class="w-6 h-6 animate-spin [animation-timing-function:steps(8)] text-[var(--md-sys-primary)]" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <line x1="12" y1="2" x2="12" y2="6" stroke-width="2.5" stroke-linecap="round" class="opacity-100" />
          <line x1="19.07" y1="4.93" x2="16.24" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-90" />
          <line x1="22" y1="12" x2="18" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-75" />
          <line x1="19.07" y1="19.07" x2="16.24" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-60" />
          <line x1="12" y1="22" x2="12" y2="18" stroke-width="2.5" stroke-linecap="round" class="opacity-45" />
          <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-30" />
          <line x1="2" y1="12" x2="6" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-20" />
          <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-15" />
        </svg>
      </div>
    {:else}
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        {#each artistTracks as track, idx}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            onkeydown={(e) => { if (e.key === 'Enter') playerState.playTrack(track); }}
            class="flex items-center justify-between p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/50 shadow-sm' : 'bg-white/[0.03] hover:bg-white/[0.07] border-white/[0.05]'} border transition-all cursor-pointer group"
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-2">
              <span class="w-5 text-center text-xs font-mono font-bold {idx < 3 ? 'text-amber-400' : 'text-white/40'} shrink-0">
                {idx + 1}
              </span>
              <div class="relative w-12 h-12 rounded-xl overflow-hidden shadow shrink-0 border border-white/10">
                <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition" />
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
            </div>

            <button
              onclick={(e) => {
                e.stopPropagation();
                playerState.addToQueue(track);
              }}
              class="w-8 h-8 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 transition shrink-0 opacity-0 group-hover:opacity-100"
              title="Add to Up Next Queue"
            >
              <span class="material-symbols-rounded text-lg">queue_music</span>
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

</div>
