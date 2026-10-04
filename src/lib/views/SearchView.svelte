<script lang="ts">
  import { playerState } from '../state.svelte';

  let searchQuery = $state('');

  const exploreTags = [
    'Trending India',
    'Arijit Singh',
    'Bollywood Hits',
    'Punjabi Pop',
    'South Indian Hits',
    'Diljit Dosanjh',
    'Shreya Ghoshal',
    'Indie India',
    'Lo-Fi Beats',
  ];

  function handleSearch(q?: string) {
    const query = q || searchQuery;
    if (!query.trim()) return;
    searchQuery = query;
    playerState.searchYouTube(query);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleSearch();
    }
  }
</script>

<div class="p-6 space-y-6 max-w-7xl mx-auto pb-28">
  
  <!-- Search Input Bar -->
  <div class="relative max-w-2xl">
    <div class="flex items-center gap-3 px-4 py-3 rounded-2xl bg-[#1e2026] border border-white/[0.08] focus-within:border-emerald-400/60 shadow-lg transition-colors">
      <span class="material-symbols-rounded text-[#9ea2b0] text-[22px]">search</span>
      <input
        type="text"
        bind:value={searchQuery}
        onkeydown={handleKeydown}
        placeholder="Search songs, albums, artists, or YouTube streaming..."
        class="bg-transparent border-none outline-none text-sm text-white placeholder-[#6b6e7c] w-full"
      />
      {#if searchQuery}
        <button
          onclick={() => searchQuery = ''}
          class="text-[#9ea2b0] hover:text-white"
        >
          <span class="material-symbols-rounded text-[18px]">close</span>
        </button>
      {/if}
      <button
        onclick={() => handleSearch()}
        class="px-3.5 py-1.5 rounded-xl bg-white/[0.08] hover:bg-white/[0.14] text-xs font-semibold text-white transition-colors"
      >
        Search
      </button>
    </div>
  </div>

  <!-- Explore / Suggestion Capsules -->
  <div class="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-none">
    {#each exploreTags as tag}
      <button
        onclick={() => handleSearch(tag)}
        class="px-4 py-2 rounded-full text-xs font-medium bg-[#1e2026] hover:bg-[#282a33] text-[#9ea2b0] hover:text-white border border-white/[0.05] transition-all shrink-0 hover:scale-105"
      >
        {tag}
      </button>
    {/each}
  </div>

  <!-- Search Results -->
  {#if playerState.isSearching}
    <div class="flex items-center justify-center py-20 text-[#9ea2b0]">
      <div class="flex flex-col items-center gap-3">
        <div class="w-8 h-8 border-2 border-emerald-400 border-t-transparent rounded-full animate-spin"></div>
        <p class="text-xs font-mono">Fetching streams via yt-dlp & MPV...</p>
      </div>
    </div>
  {:else if playerState.youtubeResults.length > 0}
    <div>
      <h3 class="font-heading font-bold text-lg text-white mb-4">YouTube Music Results</h3>
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
        {#each playerState.youtubeResults as yt}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playYouTube(yt)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); playerState.playYouTube(yt); } }}
            class="flex items-center gap-3.5 p-3 rounded-2xl bg-[#1e2026]/70 hover:bg-[#262830] border border-white/[0.05] hover:border-white/[0.1] transition-all cursor-pointer group shadow-sm"
          >
            <div class="relative w-16 h-16 rounded-xl overflow-hidden shrink-0 bg-[#252730] border border-white/[0.06]">
              <img src={yt.thumbnail} alt={yt.title} class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
              <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity">
                <span class="material-symbols-rounded text-white text-[24px] filled">play_arrow</span>
              </div>
            </div>

            <div class="min-w-0 flex-1">
              <h4 class="font-heading font-semibold text-sm text-white truncate group-hover:text-emerald-300 transition-colors">
                {yt.title}
              </h4>
              <p class="text-xs text-[#9ea2b0] truncate mt-0.5">{yt.artist}</p>
              <div class="flex items-center gap-2 mt-1">
                <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-red-500/15 text-red-300 font-semibold border border-red-500/20">
                  YouTube Music • Opus 256k
                </span>
                {#if yt.duration > 0}
                  <span class="text-[10px] font-mono text-[#6b6e7c]">
                    {Math.floor(yt.duration / 60)}:{Math.floor(yt.duration % 60) < 10 ? '0' : ''}{Math.floor(yt.duration % 60)}
                  </span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="flex flex-col items-center justify-center py-20 text-[#6b6e7c]">
      <span class="material-symbols-rounded text-[48px] mb-3 opacity-40">travel_explore</span>
      <p class="text-sm font-medium">Search songs, artists, and YouTube Music streams</p>
      <p class="text-xs text-[#4c4f5e] mt-1">High Quality Opus 48kHz audio streamed directly via InnerTube & MPV.</p>
    </div>
  {/if}

</div>
