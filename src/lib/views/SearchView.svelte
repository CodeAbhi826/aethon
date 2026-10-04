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
              <h4 class="font-heading font-semibold text-sm text-white truncate group-hover:text-[var(--md-sys-primary)] transition-colors">
                {yt.title}
              </h4>
              <div class="flex items-center gap-1.5 text-xs text-[#9ea2b0] truncate mt-1">
                <span class="truncate">{yt.artist}</span>
                {#if yt.duration > 0}
                  <span class="text-[#6b6e7c] shrink-0">•</span>
                  <span class="font-mono text-[11px] text-[#6b6e7c] shrink-0">
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
    <!-- Apple Music style Browse & Explore Categories -->
    <div class="space-y-4 pt-2">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
          <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">grid_view</span>
          Browse Categories
        </h3>
        <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Discover music by mood & genre</span>
      </div>

      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3.5">
        {#each [
          { title: 'Top Charts India', query: 'Top 50 India Songs', bg: 'bg-gradient-to-br from-amber-600/30 to-orange-950/40', border: 'border-orange-500/20', icon: 'trending_up', text: 'text-orange-400' },
          { title: 'Bollywood Romance', query: 'Bollywood Romantic Songs', bg: 'bg-gradient-to-br from-rose-600/30 to-pink-950/40', border: 'border-rose-500/20', icon: 'favorite', text: 'text-rose-400' },
          { title: 'Punjabi Hits', query: 'Punjabi Hits 2026', bg: 'bg-gradient-to-br from-purple-600/30 to-indigo-950/40', border: 'border-purple-500/20', icon: 'music_note', text: 'text-purple-400' },
          { title: 'Lo-Fi Chill Beats', query: 'Indian Lo-Fi Chill Beats', bg: 'bg-gradient-to-br from-cyan-600/30 to-blue-950/40', border: 'border-cyan-500/20', icon: 'nightlife', text: 'text-cyan-400' },
          { title: 'Indie India', query: 'Indian Indie Songs', bg: 'bg-gradient-to-br from-emerald-600/30 to-teal-950/40', border: 'border-emerald-500/20', icon: 'graphic_eq', text: 'text-emerald-400' },
          { title: 'South Indian Hits', query: 'South Indian Hits', bg: 'bg-gradient-to-br from-yellow-600/30 to-amber-950/40', border: 'border-yellow-500/20', icon: 'audiotrack', text: 'text-yellow-400' },
          { title: 'Workout Energy', query: 'Gym Workout Music Bollywood', bg: 'bg-gradient-to-br from-red-600/30 to-rose-950/40', border: 'border-red-500/20', icon: 'bolt', text: 'text-red-400' },
          { title: 'Acoustic & Unplugged', query: 'Acoustic Hindi Unplugged', bg: 'bg-gradient-to-br from-sky-600/30 to-slate-950/40', border: 'border-sky-500/20', icon: 'piano', text: 'text-sky-400' },
        ] as cat}
          <button
            onclick={() => handleSearch(cat.query)}
            class="p-4 rounded-2xl {cat.bg} border {cat.border} hover:scale-[1.02] hover:brightness-110 transition-all text-left group cursor-pointer relative overflow-hidden flex flex-col justify-between h-28 shadow-sm"
          >
            <div class="flex items-center justify-between">
              <span class="material-symbols-rounded {cat.text} text-2xl group-hover:scale-110 transition-transform">
                {cat.icon}
              </span>
              <span class="material-symbols-rounded text-white/20 group-hover:text-white/60 text-lg transition-colors">
                arrow_forward
              </span>
            </div>
            <div>
              <div class="font-heading font-bold text-sm text-white">{cat.title}</div>
              <div class="text-[10px] text-white/50 mt-0.5">Explore curated playlist</div>
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/if}

</div>
