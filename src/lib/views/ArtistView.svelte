<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { sanitizeArtist, type Track, type YouTubeTrack } from '../types';

  let artistTracks = $state<Track[]>([]);
  let isLoadingTracks = $state(true);
  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  let artistName = $derived(playerState.selectedArtist || 'Artist');

  let localArtistTracks = $derived(
    playerState.library.filter((t) =>
      t.artist.toLowerCase().includes(artistName.toLowerCase())
    )
  );

  let lastFetchedArtist = '';
  $effect(() => {
    if (playerState.selectedArtist && playerState.selectedArtist !== lastFetchedArtist) {
      lastFetchedArtist = playerState.selectedArtist;
      fetchArtistData();
    }
  });

  async function fetchArtistData() {
    isLoadingTracks = true;
    try {
      const res = await invoke<YouTubeTrack[]>('search_youtube', {
        query: `${artistName} songs`,
        limit: 10,
      });

      if (res && res.length > 0) {
        artistTracks = res.map((yt, idx) => ({
          id: `yt_${yt.id}`,
          title: yt.title,
          artist: sanitizeArtist(yt.artist) || artistName,
          album: 'Top Songs',
          duration: yt.duration || 0,
          file_path: yt.stream_url,
          cover_art: yt.thumbnail,
          format: 'Opus 256k',
          play_count: 100 - idx * 5,
          favorite: false,
          date_added: Math.floor(Date.now() / 1000),
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
    playerState.searchYouTube(`${artistName} mix`);
    playerState.activeTab = 'search';
  }
</script>

<div class="px-6 py-6 space-y-6 max-w-6xl mx-auto pb-28 select-none">
  <button
    onclick={() => (playerState.activeTab = 'home')}
    class="flex items-center gap-1.5 text-xs font-medium text-white/60 hover:text-white cursor-pointer transition-colors"
  >
    <span class="material-symbols-rounded text-base">arrow_back</span>
    <span>Back</span>
  </button>

  <div class="p-6 rounded-3xl bg-white/[0.03] border border-white/[0.07] flex flex-col sm:flex-row items-center gap-6 relative overflow-hidden">
    <div class="w-28 h-28 sm:w-36 sm:h-36 rounded-full overflow-hidden shadow-xl ring-1 ring-white/15 shrink-0 bg-white/5">
      {#if artistTracks[0]?.cover_art}
        <img src={artistTracks[0].cover_art} alt={artistName} class="w-full h-full object-cover" />
      {:else}
        <div class="w-full h-full flex items-center justify-center">
          <span class="material-symbols-rounded text-4xl text-white/30">person</span>
        </div>
      {/if}
    </div>

    <div class="flex-1 text-center sm:text-left space-y-3 min-w-0">
      <h1 class="text-2xl sm:text-4xl font-bold text-white tracking-tight font-display truncate">
        {artistName}
      </h1>

      <div class="flex items-center justify-center sm:justify-start gap-2.5">
        <button
          onclick={handlePlayAll}
          class="flex items-center gap-1.5 px-5 py-2 rounded-full bg-white text-black text-xs font-semibold hover:scale-105 cursor-pointer transition-transform"
        >
          <span class="material-symbols-rounded text-base filled">play_arrow</span>
          <span>Play</span>
        </button>

        <button
          onclick={handlePlayRadio}
          class="flex items-center gap-1.5 px-4 py-2 rounded-full bg-white/[0.07] hover:bg-white/[0.14] text-white text-xs font-medium cursor-pointer transition-colors"
        >
          <span class="material-symbols-rounded text-base">radio</span>
          <span>Radio</span>
        </button>
      </div>
    </div>
  </div>

  <section class="space-y-2.5">
    <h2 class="font-display font-bold text-base text-white px-1">Top Songs</h2>

    {#if isLoadingTracks}
      <div class="space-y-2">
        {#each Array(6) as _}
          <div class="h-14 rounded-2xl bg-white/[0.04] animate-pulse"></div>
        {/each}
      </div>
    {:else}
      <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
        {#each artistTracks as track, idx}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onmouseenter={() => playerState.prefetchStream(track.file_path, track.title, track.artist)}
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="flex items-center justify-between px-3.5 py-2.5 {isCurrent ? 'bg-white/[0.08]' : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group"
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-2">
              <span class="w-5 text-center text-xs font-mono text-white/35 shrink-0">
                {idx + 1}
              </span>
              <div class="relative w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
                <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover" />
                <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                  {#if isCurrent && isBuffering}
                    <svg class="w-4 h-4 animate-spin text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                      <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                      <circle cx="18" cy="18" r="13" stroke="currentColor" stroke-width="3.2" stroke-dasharray="52 100" stroke-linecap="round" />
                    </svg>
                  {:else}
                    <span class="material-symbols-rounded text-white text-lg filled">
                      {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                    </span>
                  {/if}
                </div>
              </div>

              <div class="min-w-0 flex-1">
                <div class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate">
                  {track.title}
                </div>
                <div class="text-[11px] text-white/50 truncate mt-0.5">
                  {track.artist}
                </div>
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              {#if track.duration > 0}
                <span class="font-mono text-[11px] text-white/40">
                  {Math.floor(track.duration / 60)}:{Math.floor(track.duration % 60) < 10 ? '0' : ''}{Math.floor(track.duration % 60)}
                </span>
              {/if}
              <button
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                class="w-7 h-7 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 transition opacity-0 group-hover:opacity-100"
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
</div>
