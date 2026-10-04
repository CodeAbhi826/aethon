<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { Track } from '../types';

  let activeFilter = $state<'all' | 'favorites' | 'flac'>('all');

  let filteredTracks = $derived.by(() => {
    let list = playerState.library;
    if (activeFilter === 'favorites') {
      return list.filter((t) => t.favorite);
    }
    if (activeFilter === 'flac') {
      return list.filter((t) => t.format.toLowerCase().includes('flac'));
    }
    return list;
  });
</script>

<div class="p-6 space-y-6 max-w-7xl mx-auto pb-28">
  
  <!-- Header & Filters -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h2 class="font-heading font-bold text-2xl text-white">Your Music Library</h2>
      <p class="text-xs text-[#9ea2b0] mt-0.5">{playerState.library.length} tracks indexed locally</p>
    </div>

    <!-- Filter Pills (ArchiveTune style) -->
    <div class="flex items-center gap-2">
      <button
        onclick={() => activeFilter = 'all'}
        class="px-3.5 py-1.5 rounded-full text-xs font-medium transition-all {activeFilter === 'all' ? 'bg-white text-[#111215] font-semibold' : 'bg-[#1e2026] text-[#9ea2b0] hover:text-white border border-white/[0.05]'}"
      >
        All Songs
      </button>
      <button
        onclick={() => activeFilter = 'favorites'}
        class="px-3.5 py-1.5 rounded-full text-xs font-medium transition-all {activeFilter === 'favorites' ? 'bg-white text-[#111215] font-semibold' : 'bg-[#1e2026] text-[#9ea2b0] hover:text-white border border-white/[0.05]'}"
      >
        Favorites
      </button>
      <button
        onclick={() => activeFilter = 'flac'}
        class="px-3.5 py-1.5 rounded-full text-xs font-medium transition-all {activeFilter === 'flac' ? 'bg-white text-[#111215] font-semibold' : 'bg-[#1e2026] text-[#9ea2b0] hover:text-white border border-white/[0.05]'}"
      >
        Local Audio
      </button>
      <button
        onclick={() => playerState.scanLibrary()}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium bg-white/[0.06] hover:bg-white/[0.1] text-white border border-white/[0.06] transition-colors ml-2"
        title="Rescan"
      >
        <span class="material-symbols-rounded text-[16px]">sync</span>
        <span>Rescan</span>
      </button>
    </div>
  </div>

  <!-- ArchiveTune 2-Column Cards Grid -->
  {#if filteredTracks.length > 0}
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-3.5">
      {#each filteredTracks as track}
        {@const isCurrent = playerState.currentTrack?.id === track.id}
        <div
          role="button"
          tabindex="0"
          onclick={() => playerState.playTrack(track)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); playerState.playTrack(track); } }}
          class="flex items-center justify-between p-3 rounded-2xl bg-[#1e2026]/70 hover:bg-[#262830] border {isCurrent ? 'border-emerald-400/40 bg-[#262830]' : 'border-white/[0.05] hover:border-white/[0.1]'} transition-all cursor-pointer group shadow-sm"
        >
          <div class="flex items-center gap-3.5 min-w-0 flex-1">
            <div class="relative w-14 h-14 rounded-xl overflow-hidden shrink-0 bg-[#252730] border border-white/[0.06]">
              {#if track.cover_art}
                <img src={track.cover_art} alt="Cover" class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
              {:else}
                <div class="w-full h-full flex items-center justify-center">
                  <span class="material-symbols-rounded text-white/40">music_note</span>
                </div>
              {/if}
              <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
                <span class="material-symbols-rounded text-white text-[24px] filled">play_arrow</span>
              </div>
            </div>

            <div class="min-w-0 flex-1 pr-2">
              <h4 class="font-heading font-semibold text-sm text-white truncate {isCurrent ? 'text-emerald-300' : 'group-hover:text-white'}">
                {track.title}
              </h4>
              <div class="flex items-center gap-1.5 text-xs text-[#9ea2b0] truncate mt-1">
                <span class="truncate">{track.artist} • {track.album}</span>
                {#if track.duration > 0}
                  <span class="text-[#6b6e7c] shrink-0">•</span>
                  <span class="font-mono text-[11px] text-[#6b6e7c] shrink-0">
                    {Math.floor(track.duration / 60)}:{Math.floor(track.duration % 60) < 10 ? '0' : ''}{Math.floor(track.duration % 60)}
                  </span>
                {/if}
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2 shrink-0">
            <button
              onclick={(e) => {
                e.stopPropagation();
                playerState.toggleFavorite(track.id);
              }}
              class="w-8 h-8 rounded-full flex items-center justify-center text-[#9ea2b0] hover:text-rose-400 hover:bg-white/[0.05] transition-colors"
            >
              <span class="material-symbols-rounded text-[20px] {track.favorite ? 'text-rose-400 filled' : ''}">
                favorite
              </span>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="flex flex-col items-center justify-center py-24 text-center bg-[#18191e]/50 rounded-3xl border border-white/[0.04]">
      <span class="material-symbols-rounded text-[48px] text-[#6b6e7c] mb-3">folder_open</span>
      <h3 class="font-heading font-bold text-base text-white">No tracks in library</h3>
      <p class="text-xs text-[#9ea2b0] max-w-sm mt-1 mb-4">
        Place your FLAC, MP3, or ALAC audio files in ~/Music and click Rescan.
      </p>
      <button
        onclick={() => playerState.scanLibrary()}
        class="px-5 py-2.5 rounded-full bg-white text-[#111215] font-semibold text-xs hover:bg-[#e6e0e9] transition-all shadow-md"
      >
        Scan ~/Music Now
      </button>
    </div>
  {/if}

</div>
