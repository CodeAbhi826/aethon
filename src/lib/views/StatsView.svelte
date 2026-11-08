<script lang="ts">
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import {
    sanitizeArtist,
    handleThumbnailError,
    handleThumbnailLoad,
    type Track,
  } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from '../components/ArtistLinks.svelte';

  type Period = 'week' | 'month' | 'half_year' | 'year' | 'all';
  let activePeriod = $state<Period>('all');

  const periods: { id: Period; label: string; seconds: number }[] = [
    { id: 'week', label: '1 Week', seconds: 7 * 86400 },
    { id: 'month', label: '1 Month', seconds: 30 * 86400 },
    { id: 'half_year', label: '6 Months', seconds: 180 * 86400 },
    { id: 'year', label: '1 Year', seconds: 365 * 86400 },
    { id: 'all', label: 'All Time', seconds: 0 },
  ];

  // Combine library and playHistoryTracks without duplicates
  let allTrackedSongs = $derived.by(() => {
    const map = new Map<string, Track>();
    for (const t of [...playerState.playHistoryTracks, ...playerState.library]) {
      const existing = map.get(t.id);
      if (existing) {
        if (t.play_count > existing.play_count) {
          map.set(t.id, t);
        }
      } else {
        map.set(t.id, {
          ...t,
          play_count: Math.max(t.play_count || 0, playerState.playHistoryTracks.some((h) => h.id === t.id) ? 1 : 0),
        });
      }
    }
    return Array.from(map.values());
  });

  let filteredTracks = $derived.by(() => {
    const p = periods.find((x) => x.id === activePeriod);
    if (!p || p.seconds === 0) return allTrackedSongs;
    const cutoff = Math.floor(Date.now() / 1000) - p.seconds;
    const within = allTrackedSongs.filter((t) => (t.date_added || 0) >= cutoff);
    return within.length > 0 ? within : allTrackedSongs;
  });

  let totalPlays = $derived(
    filteredTracks.reduce((sum, t) => sum + Math.max(t.play_count || 0, 0), 0)
  );

  let totalSecondsListened = $derived(
    filteredTracks.reduce((sum, t) => sum + (t.duration || 210) * Math.max(t.play_count || 0, 0), 0)
  );

  let formattedListeningTime = $derived.by(() => {
    const totalMins = Math.round(totalSecondsListened / 60);
    if (totalMins < 60) return { main: `${totalMins}`, unit: 'mins' };
    const hrs = Math.floor(totalMins / 60);
    const mins = totalMins % 60;
    return { main: `${hrs}h ${mins}m`, unit: 'listened' };
  });

  let mostPlayedSongs = $derived(
    [...filteredTracks]
      .filter((t) => (t.play_count || 0) > 0)
      .sort((a, b) => (b.play_count || 0) - (a.play_count || 0))
      .slice(0, 15)
  );

  let topArtists = $derived.by(() => {
    const map = new Map<
      string,
      { name: string; plays: number; minutes: number; cover?: string; id: string }
    >();
    for (const t of filteredTracks) {
      const plays = Math.max(t.play_count || 0, 0);
      if (plays <= 0) continue;
      const primary = sanitizeArtist(t.artist).split(',')[0].trim();
      if (!primary || primary.toLowerCase() === 'unknown artist') continue;
      const key = primary.toLowerCase();
      const mins = ((t.duration || 210) * plays) / 60;
      const existing = map.get(key);
      if (existing) {
        existing.plays += plays;
        existing.minutes += mins;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(key, {
          name: primary,
          plays,
          minutes: mins,
          cover: t.cover_art,
          id: t.id,
        });
      }
    }
    return Array.from(map.values())
      .sort((a, b) => b.plays - a.plays || b.minutes - a.minutes)
      .slice(0, 12);
  });

  let topAlbums = $derived.by(() => {
    const map = new Map<
      string,
      { title: string; artist: string; plays: number; cover?: string; id: string }
    >();
    for (const t of filteredTracks) {
      const plays = Math.max(t.play_count || 0, 0);
      if (plays <= 0) continue;
      const alb = t.album || 'YouTube Music';
      const key = alb.toLowerCase();
      const existing = map.get(key);
      if (existing) {
        existing.plays += plays;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(key, {
          title: alb,
          artist: sanitizeArtist(t.artist),
          plays,
          cover: t.cover_art,
          id: t.id,
        });
      }
    }
    return Array.from(map.values())
      .sort((a, b) => b.plays - a.plays)
      .slice(0, 8);
  });

  function formatDuration(sec: number): string {
    if (!sec || sec <= 0) return '';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }
</script>

<div class="px-6 py-5 space-y-7 max-w-7xl mx-auto pb-28 select-none">
  <!-- ArchiveTune Stats Header + Time Period Filter Chips -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h1 class="font-display font-bold text-2xl sm:text-3xl text-white tracking-tight">Stats</h1>
      <p class="text-xs text-white/50 mt-0.5">Your personal listening activity, top artists & Recap metrics</p>
    </div>

    <div class="flex items-center gap-1.5 overflow-x-auto scrollbar-none">
      {#each periods as p}
        <button
          type="button"
          onclick={() => (activePeriod = p.id)}
          class="px-3.5 py-1.5 rounded-xl text-xs font-medium whitespace-nowrap transition-all cursor-pointer {activePeriod === p.id
            ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] font-semibold shadow-sm'
            : 'bg-white/[0.05] text-white/65 hover:text-white hover:bg-white/10'}"
        >
          {p.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- 4 Summary Hero Cards -->
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-3.5">
    <div class="p-4 sm:p-5 rounded-3xl m3-acrylic-card border border-white/[0.07]">
      <div class="flex items-center gap-2 text-white/55 text-xs font-medium mb-2">
        <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-[18px]">schedule</span>
        <span>Listening Time</span>
      </div>
      <div class="font-display font-bold text-2xl sm:text-3xl text-white">
        {formattedListeningTime.main}
      </div>
      <p class="text-[11px] text-white/45 mt-1">{formattedListeningTime.unit}</p>
    </div>

    <div class="p-4 sm:p-5 rounded-3xl m3-acrylic-card border border-white/[0.07]">
      <div class="flex items-center gap-2 text-white/55 text-xs font-medium mb-2">
        <span class="material-symbols-rounded text-emerald-400 text-[18px]">music_note</span>
        <span>Songs Played</span>
      </div>
      <div class="font-display font-bold text-2xl sm:text-3xl text-white">
        {totalPlays}
      </div>
      <p class="text-[11px] text-white/45 mt-1">{mostPlayedSongs.length} unique tracks</p>
    </div>

    <div class="p-4 sm:p-5 rounded-3xl m3-acrylic-card border border-white/[0.07]">
      <div class="flex items-center gap-2 text-white/55 text-xs font-medium mb-2">
        <span class="material-symbols-rounded text-amber-400 text-[18px]">Interpreter_mode</span>
        <span>Artists</span>
      </div>
      <div class="font-display font-bold text-2xl sm:text-3xl text-white">
        {topArtists.length}
      </div>
      <p class="text-[11px] text-white/45 mt-1">
        {topArtists[0] ? `#1 ${topArtists[0].name}` : 'Start listening'}
      </p>
    </div>

    <button
      type="button"
      onclick={() => (playerState.activeTab = 'settings')}
      class="p-4 sm:p-5 rounded-3xl m3-acrylic-card border border-white/[0.07] text-left hover:border-white/20 transition-colors cursor-pointer"
    >
      <div class="flex items-center gap-2 text-white/55 text-xs font-medium mb-2">
        <span class="material-symbols-rounded text-rose-400 text-[18px]">insights</span>
        <span>YouTube Recap</span>
      </div>
      <div class="font-display font-bold text-base sm:text-lg {settingsState.ytmConnected ? 'text-emerald-400' : 'text-white/80'} truncate">
        {settingsState.ytmConnected ? 'Live Reporting Active' : 'Local Mode'}
      </div>
      <p class="text-[11px] text-white/45 mt-1 truncate">
        {settingsState.ytmConnected
          ? `Connected as ${settingsState.ytmAccountName}`
          : 'Tap to connect Google Account'}
      </p>
    </button>
  </div>

  <!-- 1. ArchiveTune "Most played songs" Ranked Shelf -->
  <section class="space-y-3">
    <div class="flex items-center justify-between px-0.5">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight">
        Most played songs ({mostPlayedSongs.length})
      </h2>
      {#if mostPlayedSongs.length > 0}
        <button
          type="button"
          onclick={() => {
            playerState.queue = mostPlayedSongs.slice(1);
            playerState.playTrack(mostPlayedSongs[0]);
          }}
          class="px-3.5 py-1 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-xs font-medium text-white/80 cursor-pointer"
        >
          Play all
        </button>
      {/if}
    </div>

    {#if mostPlayedSongs.length > 0}
      <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
        {#each mostPlayedSongs as track, idx (track.id)}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          {@const mins = Math.max(1, Math.round(((track.duration || 210) * Math.max(track.play_count, 1)) / 60))}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter') playerState.playTrack(track);
            }}
            class="h-14 flex items-center justify-between px-3.5 {isCurrent
              ? 'bg-[var(--md-sys-primary-container)]/35'
              : 'hover:bg-white/[0.05]'} transition-colors cursor-pointer group overflow-hidden"
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-3">
              <span class="w-5 text-center font-display font-bold text-xs {idx < 3 ? 'text-[var(--md-sys-primary)]' : 'text-white/40'} shrink-0">
                {idx + 1}
              </span>

              <div class="relative w-10 h-10 rounded-lg overflow-hidden bg-white/5 shrink-0">
                {#if track.cover_art}
                  <img
                    loading="lazy"
                    decoding="async"
                    src={getHighResArtwork(track.cover_art, track.id)}
                    alt={track.title}
                    onload={(e) => handleThumbnailLoad(e, track.id)}
                    onerror={(e) => handleThumbnailError(e, track.id)}
                    class="w-full h-full object-cover"
                  />
                {/if}
              </div>

              <div class="min-w-0 flex-1 overflow-hidden">
                <p class="text-xs font-semibold {isCurrent ? 'text-[var(--md-sys-primary)]' : 'text-white'} truncate block w-full">
                  {track.title}
                </p>
                <ArtistLinks
                  artist={track.artist}
                  class="text-[11px] text-white/55 mt-0.5"
                  buttonClass="hover:underline hover:text-white transition-colors cursor-pointer text-left"
                />
              </div>
            </div>

            <div class="flex items-center gap-3 shrink-0">
              <div class="text-right">
                <span class="text-xs font-semibold text-white/85 block">
                  {Math.max(track.play_count, 1)} {Math.max(track.play_count, 1) === 1 ? 'play' : 'plays'}
                </span>
                <span class="text-[10px] font-mono text-white/40 block">
                  {mins} min • {formatDuration(track.duration)}
                </span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="p-10 rounded-3xl bg-white/[0.02] border border-white/[0.05] text-center text-xs text-white/45">
        Play songs to populate your personal listening rankings.
      </div>
    {/if}
  </section>

  <!-- 2. ArchiveTune "Most played artists" Circular Podium Grid -->
  {#if topArtists.length > 0}
    <section class="space-y-3">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight px-0.5">
        Most played artists ({topArtists.length})
      </h2>

      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3.5">
        {#each topArtists as artist, idx}
          <button
            type="button"
            onclick={() => playerState.openArtist(artist.name)}
            class="p-4 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] flex flex-col items-center text-center gap-2.5 transition-all cursor-pointer group relative"
          >
            <span class="absolute top-2.5 left-3 font-display font-bold text-xs {idx < 3 ? 'text-[var(--md-sys-primary)]' : 'text-white/35'}">
              #{idx + 1}
            </span>
            <div class="w-20 h-20 rounded-full overflow-hidden bg-white/5 shadow-md ring-1 ring-white/10">
              {#if artist.cover}
                <img
                  loading="lazy"
                  decoding="async"
                  src={getHighResArtwork(artist.cover, artist.id)}
                  alt={artist.name}
                  onload={(e) => handleThumbnailLoad(e, artist.id)}
                  onerror={(e) => handleThumbnailError(e, artist.id)}
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform"
                />
              {:else}
                <div class="w-full h-full flex items-center justify-center text-white/35">
                  <span class="material-symbols-rounded text-2xl">person</span>
                </div>
              {/if}
            </div>
            <div class="overflow-hidden w-full">
              <h4 class="text-xs font-semibold text-white truncate w-full">{artist.name}</h4>
              <p class="text-[10px] text-white/50 mt-0.5">
                {artist.plays} {artist.plays === 1 ? 'play' : 'plays'} • {Math.max(1, Math.round(artist.minutes))}m
              </p>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <!-- 3. ArchiveTune "Most played albums" Shelf -->
  {#if topAlbums.length > 0}
    <section class="space-y-3">
      <h2 class="font-display font-bold text-lg sm:text-xl text-white tracking-tight px-0.5">
        Most played albums
      </h2>

      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3.5">
        {#each topAlbums as album}
          <button
            type="button"
            onclick={() => playerState.openAlbum(album.title, album.artist, album.cover)}
            class="p-3 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] flex items-center gap-3 text-left transition-all cursor-pointer group overflow-hidden"
          >
            <div class="w-14 h-14 rounded-xl overflow-hidden bg-white/5 shrink-0">
              {#if album.cover}
                <img
                  loading="lazy"
                  decoding="async"
                  src={getHighResArtwork(album.cover, album.id)}
                  alt={album.title}
                  onload={(e) => handleThumbnailLoad(e, album.id)}
                  onerror={(e) => handleThumbnailError(e, album.id)}
                  class="w-full h-full object-cover"
                />
              {/if}
            </div>
            <div class="min-w-0 flex-1 overflow-hidden">
              <h4 class="text-xs font-semibold text-white truncate">{album.title}</h4>
              <p class="text-[11px] text-white/50 truncate mt-0.5">{album.artist}</p>
              <span class="text-[10px] font-mono text-[var(--md-sys-primary)] mt-1 inline-block">
                {album.plays} {album.plays === 1 ? 'play' : 'plays'}
              </span>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>
