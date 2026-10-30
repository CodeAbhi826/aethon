<script lang="ts">
  import { playerState } from '../state.svelte';
  import { sanitizeArtist, type Track } from '../types';

  let searchQuery = $state('');

  const exploreTags = [
    'Lo-Fi Chill Beats',
    'Synthwave & Retrowave',
    'Electronic / Dance',
    'Acoustic Unplugged',
    'Modern Rock',
    'Hip-Hop & R&B',
    'Deep Ambient Focus',
    'Classical & Piano',
    'Jazz & Blues Melodies',
  ];

  // Instant local library filtering
  let localResults = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return [];
    return playerState.library.filter(
      (t) =>
        t.title.toLowerCase().includes(q) ||
        t.artist.toLowerCase().includes(q) ||
        (t.album && t.album.toLowerCase().includes(q))
    ).slice(0, 10);
  });

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

  function convertYtToTrack(yt: any): Track {
    return {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: sanitizeArtist(yt.artist),
      album: 'Lossless Online Stream',
      duration: yt.duration,
      file_path: yt.stream_url,
      cover_art: yt.thumbnail,
      format: 'Opus 256k',
      play_count: 0,
      favorite: false,
      date_added: Math.floor(Date.now() / 1000),
    };
  }
</script>

<div class="p-6 space-y-6 max-w-7xl mx-auto pb-28 select-none">
  
  <!-- Search Input Bar -->
  <div class="relative max-w-2xl">
    <div class="flex items-center gap-3 px-4 py-3 rounded-2xl bg-[#1e2026] border border-white/[0.08] focus-within:border-[var(--md-sys-primary)] shadow-lg transition-colors">
      <span class="material-symbols-rounded text-[#9ea2b0] text-[22px]">search</span>
      <input
        type="text"
        bind:value={searchQuery}
        onkeydown={handleKeydown}
        placeholder="Search local library or stream online..."
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
        class="px-3.5 py-1.5 rounded-xl bg-[var(--md-sys-primary)] hover:brightness-110 text-xs font-semibold text-[var(--md-sys-on-primary)] transition-all cursor-pointer shadow-sm"
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
        class="px-4 py-2 rounded-full text-xs font-medium bg-[#1e2026] hover:bg-[#282a33] text-[#9ea2b0] hover:text-white border border-white/[0.05] transition-all shrink-0 hover:scale-105 cursor-pointer"
      >
        {tag}
      </button>
    {/each}
  </div>

  <!-- Instant Local Results (if query matches local library) -->
  {#if localResults.length > 0}
    <div class="space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="font-heading font-bold text-base text-white flex items-center gap-2">
          <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-lg">folder_open</span>
          Local Library Matches
        </h3>
        <span class="text-xs text-white/50">{localResults.length} found</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
        {#each localResults as track}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); playerState.playTrack(track); } }}
            oncontextmenu={(e) => { e.preventDefault(); playerState.openContextMenu(track, e.clientX, e.clientY); }}
            class="flex items-center gap-3.5 p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/50 shadow-sm' : 'bg-[#1e2026]/70 hover:bg-[#262830] border-white/[0.05] hover:border-white/[0.1]'} border transition-all cursor-pointer group shadow-sm"
          >
            <div class="relative w-14 h-14 rounded-xl overflow-hidden shrink-0 bg-[#252730] border border-white/[0.06]">
              <img loading="lazy" decoding="async" src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
              <div class="absolute inset-0 bg-black/40 {isCurrent && playerState.playback.is_playing ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
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
              <h4 class="font-heading font-semibold text-xs {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition-colors">
                {track.title}
              </h4>
              <div class="flex items-center gap-1.5 text-[11px] text-[#9ea2b0]/80 truncate mt-0.5">
                <span class="truncate">{sanitizeArtist(track.artist)}</span>
                {#if track.duration > 0}
                  <span class="text-white/30 shrink-0">•</span>
                  <span class="font-mono text-[10px] text-white/50 shrink-0">
                    {Math.floor(track.duration / 60)}:{Math.floor(track.duration % 60) < 10 ? '0' : ''}{Math.floor(track.duration % 60)}
                  </span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Online Stream Search Results -->
  {#if playerState.isSearching}
    <div class="flex items-center justify-center py-20 text-[#9ea2b0]">
      <div class="flex flex-col items-center gap-3">
        <svg class="w-8 h-8 animate-spin [animation-timing-function:steps(8)] text-[var(--md-sys-primary)]" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <line x1="12" y1="2" x2="12" y2="6" stroke-width="2.5" stroke-linecap="round" class="opacity-100" />
          <line x1="19.07" y1="4.93" x2="16.24" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-90" />
          <line x1="22" y1="12" x2="18" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-75" />
          <line x1="19.07" y1="19.07" x2="16.24" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-60" />
          <line x1="12" y1="22" x2="12" y2="18" stroke-width="2.5" stroke-linecap="round" class="opacity-45" />
          <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" stroke-width="2.5" stroke-linecap="round" class="opacity-30" />
          <line x1="2" y1="12" x2="6" y2="12" stroke-width="2.5" stroke-linecap="round" class="opacity-20" />
          <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" stroke-width="2.5" stroke-linecap="round" class="opacity-15" />
        </svg>
        <p class="text-xs font-mono">Curating lossless online streams...</p>
      </div>
    </div>
  {:else if playerState.youtubeResults.length > 0}
    <div class="space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="font-heading font-bold text-base text-white flex items-center gap-2">
          <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-lg">stream</span>
          Online Stream Results
        </h3>
        <span class="text-xs text-white/50">{playerState.youtubeResults.length} tracks</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
        {#each playerState.youtubeResults as yt}
          {@const isCurrent = playerState.currentTrack?.id === `yt_${yt.id}`}
          {@const syntheticTrack = convertYtToTrack(yt)}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playYouTube(yt)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); playerState.playYouTube(yt); } }}
            oncontextmenu={(e) => { e.preventDefault(); playerState.openContextMenu(syntheticTrack, e.clientX, e.clientY); }}
            class="flex items-center gap-3.5 p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container)]/25 border-[var(--md-sys-primary)]/50 shadow-sm' : 'bg-[#1e2026]/70 hover:bg-[#262830] border-white/[0.05] hover:border-white/[0.1]'} border transition-all cursor-pointer group shadow-sm"
          >
            <div class="relative w-16 h-16 rounded-xl overflow-hidden shrink-0 bg-[#252730] border border-white/[0.06]">
              <img loading="lazy" decoding="async" src={yt.thumbnail} alt={yt.title} class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
              <div class="absolute inset-0 bg-black/40 {isCurrent && playerState.playback.is_playing ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
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
              <h4 class="font-heading font-semibold text-xs {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate group-hover:text-[var(--md-sys-primary)] transition-colors">
                {yt.title}
              </h4>
              <div class="flex items-center gap-1.5 text-[11px] text-[#9ea2b0]/80 truncate mt-0.5">
                <span class="truncate">{sanitizeArtist(yt.artist)}</span>
                {#if yt.duration > 0}
                  <span class="text-white/30 shrink-0">•</span>
                  <span class="font-mono text-[10px] text-white/50 shrink-0">
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
    <!-- Browse & Explore Categories -->
    <div class="space-y-4 pt-2">
      <div class="flex items-center justify-between">
        <h3 class="text-xs font-bold uppercase tracking-wider text-[var(--md-sys-on-surface-variant)] flex items-center gap-2">
          <span class="material-symbols-rounded text-sm text-[var(--md-sys-primary)]">grid_view</span>
          Browse Genres & Vibes
        </h3>
        <span class="text-xs text-[var(--md-sys-on-surface-variant)]">Curated station categories</span>
      </div>

      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3.5">
        {#each [
          { title: 'Global Hot Hits', query: 'Global Top 50 Hits', bg: 'bg-gradient-to-br from-amber-600/30 to-orange-950/40', border: 'border-orange-500/20', icon: 'trending_up', text: 'text-orange-400' },
          { title: 'Lo-Fi Chill Beats', query: 'Lo-Fi Chill Study Beats', bg: 'bg-gradient-to-br from-cyan-600/30 to-blue-950/40', border: 'border-cyan-500/20', icon: 'nightlife', text: 'text-cyan-400' },
          { title: 'Synthwave & Retro', query: 'Synthwave Retrowave 80s', bg: 'bg-gradient-to-br from-purple-600/30 to-indigo-950/40', border: 'border-purple-500/20', icon: 'music_note', text: 'text-purple-400' },
          { title: 'Deep Ambient Focus', query: 'Ambient Instrumental Sleep Focus', bg: 'bg-gradient-to-br from-emerald-600/30 to-teal-950/40', border: 'border-emerald-500/20', icon: 'graphic_eq', text: 'text-emerald-400' },
          { title: 'Electronic & EDM', query: 'Electronic Dance Club Hits', bg: 'bg-gradient-to-br from-rose-600/30 to-pink-950/40', border: 'border-rose-500/20', icon: 'bolt', text: 'text-rose-400' },
          { title: 'Acoustic & Unplugged', query: 'Acoustic Guitar Sessions', bg: 'bg-gradient-to-br from-yellow-600/30 to-amber-950/40', border: 'border-yellow-500/20', icon: 'piano', text: 'text-yellow-400' },
          { title: 'Indie & Alternative', query: 'Indie Alternative Pop Rock', bg: 'bg-gradient-to-br from-sky-600/30 to-slate-950/40', border: 'border-sky-500/20', icon: 'audiotrack', text: 'text-sky-400' },
          { title: 'Classical Masters', query: 'Classical Piano Symphony Pieces', bg: 'bg-gradient-to-br from-teal-600/30 to-emerald-950/40', border: 'border-teal-500/20', icon: 'favorite', text: 'text-teal-400' },
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
              <div class="font-heading font-bold text-xs text-white">{cat.title}</div>
              <div class="text-[10px] text-white/50 mt-0.5">Explore station</div>
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/if}

</div>
