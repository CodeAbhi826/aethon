<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { Track, Playlist } from '../types';

  type SubTab = 'tracks' | 'artists' | 'albums' | 'playlists' | 'favorites' | 'downloaded';
  let activeTab = $state<SubTab>('tracks');
  let searchQuery = $state('');

  // Track sorting
  let sortBy = $state<'title' | 'artist' | 'date' | 'duration'>('title');

  // Artists grouping
  let artistGroups = $derived.by(() => {
    const map = new Map<string, { name: string; count: number; cover?: string }>();
    for (const t of playerState.library) {
      const a = t.artist || 'Unknown Artist';
      const existing = map.get(a);
      if (existing) {
        existing.count += 1;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(a, { name: a, count: 1, cover: t.cover_art });
      }
    }
    return Array.from(map.values()).sort((a, b) => a.name.localeCompare(b.name));
  });

  // Albums grouping
  let albumGroups = $derived.by(() => {
    const map = new Map<string, { title: string; artist: string; count: number; cover?: string }>();
    for (const t of playerState.library) {
      const alb = t.album || 'Unknown Album';
      const key = `${alb}::${t.artist}`;
      const existing = map.get(key);
      if (existing) {
        existing.count += 1;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(key, { title: alb, artist: t.artist, count: 1, cover: t.cover_art });
      }
    }
    return Array.from(map.values()).sort((a, b) => a.title.localeCompare(b.title));
  });

  // Filtered tracks based on sub-tab and search
  let displayTracks = $derived.by(() => {
    let list = playerState.library;

    if (activeTab === 'favorites') {
      list = list.filter((t) => t.favorite);
    } else if (activeTab === 'downloaded') {
      list = list.filter((t) => t.file_path.includes('Aethon Downloads') || t.id.startsWith('dl_'));
    }

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter((t) => t.title.toLowerCase().includes(q) || t.artist.toLowerCase().includes(q) || t.album.toLowerCase().includes(q));
    }

    const sorted = [...list];
    if (sortBy === 'title') {
      sorted.sort((a, b) => a.title.localeCompare(b.title));
    } else if (sortBy === 'artist') {
      sorted.sort((a, b) => a.artist.localeCompare(b.artist));
    } else if (sortBy === 'date') {
      sorted.sort((a, b) => b.date_added - a.date_added);
    } else if (sortBy === 'duration') {
      sorted.sort((a, b) => b.duration - a.duration);
    }

    return sorted;
  });

  function handleContextMenu(e: MouseEvent, track: Track) {
    e.preventDefault();
    playerState.openContextMenu(track, e.clientX, e.clientY);
  }

  function formatTime(sec: number) {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }
</script>

<div class="p-6 space-y-6 max-w-7xl mx-auto pb-28">

  <!-- Header -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
    <div>
      <h2 class="font-heading font-bold text-2xl text-white">Your Music Library</h2>
      <p class="text-xs text-white/50 mt-0.5">
        {playerState.library.length} indexed songs • {playerState.playlists.length} playlists
      </p>
    </div>

    <!-- Actions (Rescan / New Playlist) -->
    <div class="flex items-center gap-2">
      <button
        onclick={() => playerState.isNewPlaylistModalOpen = true}
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-full text-xs font-medium bg-[var(--md-sys-primary,#d0bcff)] text-black hover:opacity-90 transition-opacity"
      >
        <span class="material-symbols-rounded text-base">playlist_add</span>
        <span>New Playlist</span>
      </button>

      <button
        onclick={() => playerState.scanLibrary()}
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-full text-xs font-medium bg-white/10 hover:bg-white/15 text-white transition-colors"
        title="Rescan ~/Music"
      >
        <span class="material-symbols-rounded text-base">sync</span>
        <span>Rescan</span>
      </button>
    </div>
  </div>

  <!-- Sub-Tabs Navigation (ArchiveTune & Apple Music Style) -->
  <div class="flex flex-wrap items-center gap-1.5 border-b border-white/10 pb-3">
    {#each [
      { id: 'tracks', label: 'Tracks', icon: 'music_note' },
      { id: 'artists', label: 'Artists', icon: 'person' },
      { id: 'albums', label: 'Albums', icon: 'album' },
      { id: 'playlists', label: 'Playlists', icon: 'queue_music' },
      { id: 'favorites', label: 'Favorites', icon: 'favorite' },
      { id: 'downloaded', label: 'Downloaded', icon: 'download_done' }
    ] as tab}
      <button
        onclick={() => { activeTab = tab.id as SubTab; playerState.selectedPlaylist = null; }}
        class="flex items-center gap-1.5 px-4 py-2 rounded-full text-xs font-medium transition-all {activeTab === tab.id && !playerState.selectedPlaylist ? 'bg-white text-black font-semibold shadow-sm' : 'bg-white/5 text-white/70 hover:text-white hover:bg-white/10'}"
      >
        <span class="material-symbols-rounded text-base">{tab.icon}</span>
        <span>{tab.label}</span>
      </button>
    {/each}
  </div>

  <!-- PLAYLIST DETAIL VIEW (If a playlist is open) -->
  {#if playerState.selectedPlaylist}
    {@const pl = playerState.selectedPlaylist}
    <div class="space-y-6 animate-in fade-in duration-200">
      <div class="flex items-center gap-3">
        <button
          onclick={() => playerState.selectedPlaylist = null}
          class="p-2 rounded-full hover:bg-white/10 text-white/70 hover:text-white transition-colors"
          title="Back to Playlists"
        >
          <span class="material-symbols-rounded text-xl">arrow_back</span>
        </button>
        <div>
          <h3 class="text-xl font-bold text-white">{pl.name}</h3>
          {#if pl.description}
            <p class="text-xs text-white/60">{pl.description}</p>
          {/if}
          <p class="text-[11px] text-white/40 mt-0.5">{playerState.playlistTracks.length} tracks</p>
        </div>
      </div>

      <!-- Playlist Controls -->
      {#if playerState.playlistTracks.length > 0}
        <div class="flex items-center gap-3">
          <button
            onclick={() => {
              if (playerState.playlistTracks.length > 0) {
                playerState.playTrack(playerState.playlistTracks[0]);
                playerState.queue = playerState.playlistTracks.slice(1);
              }
            }}
            class="flex items-center gap-2 px-5 py-2.5 rounded-full bg-[var(--md-sys-primary,#d0bcff)] text-black text-xs font-semibold hover:opacity-90 transition-opacity"
          >
            <span class="material-symbols-rounded text-base">play_arrow</span>
            <span>Play All</span>
          </button>

          <button
            onclick={() => {
              if (playerState.playlistTracks.length > 0) {
                const shuffled = [...playerState.playlistTracks].sort(() => Math.random() - 0.5);
                playerState.playTrack(shuffled[0]);
                playerState.queue = shuffled.slice(1);
              }
            }}
            class="flex items-center gap-2 px-4 py-2.5 rounded-full bg-white/10 hover:bg-white/15 text-white text-xs font-medium transition-colors"
          >
            <span class="material-symbols-rounded text-base">shuffle</span>
            <span>Shuffle</span>
          </button>
        </div>
      {/if}

      <!-- Playlist Tracks -->
      <div class="space-y-1">
        {#each playerState.playlistTracks as track (track.id)}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => handleContextMenu(e, track)}
            onkeydown={(e) => { if (e.key === 'Enter') playerState.playTrack(track); }}
            class="flex items-center justify-between p-2.5 rounded-xl hover:bg-white/10 transition-colors group cursor-pointer"
          >
            <div class="flex items-center gap-3 min-w-0 flex-1">
              {#if track.cover_art}
                <img src={track.cover_art} alt={track.title} class="w-10 h-10 rounded-lg object-cover flex-shrink-0" />
              {:else}
                <div class="w-10 h-10 rounded-lg bg-white/5 flex items-center justify-center flex-shrink-0 text-white/40">
                  <span class="material-symbols-rounded text-lg">music_note</span>
                </div>
              {/if}
              <div class="min-w-0">
                <p class="text-xs font-medium text-white truncate">{track.title}</p>
                <p class="text-[11px] text-white/60 truncate">{track.artist}</p>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <span class="text-xs font-mono text-white/40">{formatTime(track.duration)}</span>
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.removeFromPlaylist(pl.id, track.id);
                }}
                class="p-1 rounded-full text-white/40 hover:text-rose-400 hover:bg-white/10 transition-colors"
                title="Remove from playlist"
              >
                <span class="material-symbols-rounded text-base">close</span>
              </button>
            </div>
          </div>
        {/each}

        {#if playerState.playlistTracks.length === 0}
          <div class="py-12 text-center text-xs text-white/40">
            This playlist is empty. Right-click any track and choose "Add to Playlist" to add music.
          </div>
        {/if}
      </div>
    </div>

  <!-- SUB-TAB: PLAYLISTS -->
  {:else if activeTab === 'playlists'}
    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
      <!-- Create New Card -->
      <button
        type="button"
        onclick={() => playerState.isNewPlaylistModalOpen = true}
        class="aspect-square rounded-2xl border border-dashed border-white/20 hover:border-white/40 bg-white/5 hover:bg-white/10 flex flex-col items-center justify-center gap-2 text-white/70 hover:text-white transition-all group"
      >
        <div class="w-12 h-12 rounded-full bg-[var(--md-sys-primary,#d0bcff)]/20 text-[var(--md-sys-primary,#d0bcff)] flex items-center justify-center group-hover:scale-110 transition-transform">
          <span class="material-symbols-rounded text-2xl">add</span>
        </div>
        <span class="text-xs font-medium">New Playlist</span>
      </button>

      {#each playerState.playlists as pl (pl.id)}
        <div
          role="button"
          tabindex="0"
          onclick={() => playerState.selectPlaylist(pl)}
          onkeydown={(e) => { if (e.key === 'Enter') playerState.selectPlaylist(pl); }}
          class="aspect-square rounded-2xl bg-white/5 hover:bg-white/10 border border-white/10 p-4 flex flex-col justify-between group transition-all cursor-pointer relative"
        >
          <div class="w-12 h-12 rounded-xl bg-[var(--md-sys-primary,#d0bcff)]/20 text-[var(--md-sys-primary,#d0bcff)] flex items-center justify-center">
            <span class="material-symbols-rounded text-2xl">queue_music</span>
          </div>

          <button
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              playerState.deletePlaylist(pl.id);
            }}
            class="absolute top-3 right-3 p-1.5 rounded-full text-white/30 hover:text-rose-400 hover:bg-white/10 transition-colors opacity-0 group-hover:opacity-100"
            title="Delete playlist"
          >
            <span class="material-symbols-rounded text-base">delete</span>
          </button>

          <div>
            <h4 class="text-xs font-semibold text-white truncate">{pl.name}</h4>
            <p class="text-[10px] text-white/50 mt-0.5">{pl.track_count} {pl.track_count === 1 ? 'song' : 'songs'}</p>
          </div>
        </div>
      {/each}
    </div>

  <!-- SUB-TAB: ARTISTS -->
  {:else if activeTab === 'artists'}
    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
      {#each artistGroups as artist}
        <button
          type="button"
          onclick={() => playerState.openArtist(artist.name)}
          class="flex flex-col items-center p-3 rounded-2xl bg-white/5 hover:bg-white/10 border border-white/10 transition-all group text-center"
        >
          <div class="w-24 h-24 rounded-full overflow-hidden mb-3 bg-white/5 shadow-md border border-white/10 flex items-center justify-center">
            {#if artist.cover}
              <img src={artist.cover} alt={artist.name} class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
            {:else}
              <span class="material-symbols-rounded text-3xl text-white/40">person</span>
            {/if}
          </div>
          <h4 class="text-xs font-semibold text-white truncate max-w-full">{artist.name}</h4>
          <p class="text-[10px] text-white/50">{artist.count} {artist.count === 1 ? 'track' : 'tracks'}</p>
        </button>
      {/each}
    </div>

  <!-- SUB-TAB: ALBUMS -->
  {:else if activeTab === 'albums'}
    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
      {#each albumGroups as album}
        <button
          type="button"
          onclick={() => playerState.openAlbum(album.title, album.artist, album.cover)}
          class="flex flex-col p-3 rounded-2xl bg-white/5 hover:bg-white/10 border border-white/10 transition-all group text-left"
        >
          <div class="aspect-square rounded-xl overflow-hidden mb-3 bg-white/5 shadow-md border border-white/10 flex items-center justify-center">
            {#if album.cover}
              <img src={album.cover} alt={album.title} class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
            {:else}
              <span class="material-symbols-rounded text-3xl text-white/40">album</span>
            {/if}
          </div>
          <h4 class="text-xs font-semibold text-white truncate max-w-full">{album.title}</h4>
          <p class="text-[10px] text-white/50 truncate max-w-full mt-0.5">{album.artist}</p>
        </button>
      {/each}
    </div>

  <!-- SUB-TABS: TRACKS / FAVORITES / DOWNLOADED -->
  {:else}
    <!-- Search & Sort Bar -->
    <div class="flex items-center justify-between gap-3">
      <div class="relative flex-1 max-w-sm">
        <span class="material-symbols-rounded absolute left-3 top-1/2 -translate-y-1/2 text-white/40 text-base">search</span>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Filter tracks..."
          class="w-full pl-9 pr-3 py-1.5 rounded-full bg-white/5 border border-white/10 text-white text-xs placeholder:text-white/40 focus:outline-none focus:border-[var(--md-sys-primary,#d0bcff)]"
        />
      </div>

      <div class="flex items-center gap-2 text-xs text-white/60">
        <span>Sort:</span>
        <select
          bind:value={sortBy}
          class="bg-white/5 border border-white/10 rounded-lg px-2 py-1 text-xs text-white focus:outline-none"
        >
          <option value="title" class="bg-[#1e1e24]">Title</option>
          <option value="artist" class="bg-[#1e1e24]">Artist</option>
          <option value="date" class="bg-[#1e1e24]">Recently Added</option>
          <option value="duration" class="bg-[#1e1e24]">Duration</option>
        </select>
      </div>
    </div>

    <!-- Tracks Grid -->
    {#if displayTracks.length > 0}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {#each displayTracks as track (track.id)}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => handleContextMenu(e, track)}
            onkeydown={(e) => { if (e.key === 'Enter') playerState.playTrack(track); }}
            class="flex items-center justify-between p-3 rounded-2xl {isCurrent ? 'bg-[var(--md-sys-primary-container,#4f378b)]/30 border-[var(--md-sys-primary,#d0bcff)]/40 shadow-sm' : 'bg-white/5 hover:bg-white/10 border-white/5'} border transition-all cursor-pointer group"
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1">
              <div class="relative w-12 h-12 rounded-xl overflow-hidden shrink-0 bg-white/5 border border-white/10">
                {#if track.cover_art}
                  <img src={track.cover_art} alt={track.title} class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
                {:else}
                  <div class="w-full h-full flex items-center justify-center text-white/40">
                    <span class="material-symbols-rounded text-xl">music_note</span>
                  </div>
                {/if}
                <div class="absolute inset-0 bg-black/40 {isCurrent ? 'opacity-100 bg-black/50' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                  <span class="material-symbols-rounded {isCurrent ? 'text-[var(--md-sys-primary,#d0bcff)]' : 'text-white'} text-xl">
                    {isCurrent && playerState.playback.is_playing ? 'equalizer' : 'play_arrow'}
                  </span>
                </div>
              </div>

              <div class="min-w-0 flex-1 pr-2">
                <h4 class="font-semibold text-xs {isCurrent ? 'text-[var(--md-sys-primary,#d0bcff)]' : 'text-white'} truncate">
                  {track.title}
                </h4>
                <p class="text-[11px] text-white/75 truncate mt-0.5">
                  {track.artist} <span class="text-white/30">•</span> {track.album}
                </p>
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              {#if track.duration > 0}
                <span class="font-mono text-[11px] text-white/50">{formatTime(track.duration)}</span>
              {/if}

              <!-- Favorite Toggle -->
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.toggleFavorite(track.id);
                }}
                class="w-8 h-8 rounded-full flex items-center justify-center text-white/40 hover:text-rose-400 hover:bg-white/10 transition-colors"
                title="Favorite"
              >
                <span class="material-symbols-rounded text-lg {track.favorite ? 'text-rose-400 fill-1' : ''}">
                  {track.favorite ? 'favorite' : 'favorite_border'}
                </span>
              </button>

              <!-- Context Menu Button -->
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  handleContextMenu(e, track);
                }}
                class="w-8 h-8 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/10 transition-colors"
                title="More Options"
              >
                <span class="material-symbols-rounded text-lg">more_vert</span>
              </button>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="flex flex-col items-center justify-center py-20 text-center bg-white/5 rounded-3xl border border-white/5">
        <span class="material-symbols-rounded text-5xl text-white/30 mb-3">folder_open</span>
        <h3 class="font-bold text-sm text-white">No tracks found</h3>
        <p class="text-xs text-white/50 max-w-sm mt-1 mb-4">
          {#if activeTab === 'favorites'}
            You haven't marked any tracks as favorite yet.
          {:else if activeTab === 'downloaded'}
            No downloaded tracks found. Click "Download Offline" on any song to save it.
          {:else}
            Place your audio files in ~/Music and click Rescan.
          {/if}
        </p>
      </div>
    {/if}
  {/if}

</div>
