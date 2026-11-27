<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import {
    sanitizeArtist,
    splitArtists,
    handleThumbnailError,
    handleThumbnailLoad,
    type Track,
  } from '../types';
  import { getHighResArtwork } from '../artwork';
  import ArtistLinks from '../components/ArtistLinks.svelte';

  type Period = 'week' | 'month' | 'half_year' | 'year' | 'all';
  let activePeriod = $state<Period>('all');
  let periodTracks = $state<Track[] | null>(null);
  let statsSeq = 0;
  let isBuffering = $derived(playerState.isLoading || Boolean(playerState.playback.is_buffering));

  const periods: { id: Period; label: string; seconds: number }[] = [
    { id: 'week', label: '1 Week', seconds: 7 * 86400 },
    { id: 'month', label: '1 Month', seconds: 30 * 86400 },
    { id: 'half_year', label: '6 Months', seconds: 180 * 86400 },
    { id: 'year', label: '1 Year', seconds: 365 * 86400 },
    { id: 'all', label: 'All Time', seconds: 0 },
  ];

  $effect(() => {
    const p = periods.find((x) => x.id === activePeriod);
    const sinceEpoch = !p || p.seconds === 0 ? 0 : Math.floor(Date.now() / 1000) - p.seconds;
    const seq = ++statsSeq;
    invoke<Track[]>('get_listening_stats', { sinceEpoch })
      .then((res) => {
        if (seq === statsSeq && Array.isArray(res)) {
          periodTracks = res;
        }
      })
      .catch(() => {
        if (seq === statsSeq) {
          periodTracks = null;
        }
      });
  });

  // Combine library and playHistoryTracks without duplicates as fallback / baseline
  let allTrackedSongs = $derived.by(() => {
    const historyIds = new Set(playerState.playHistoryTracks.map((h) => h.id));
    const map = new Map<string, Track>();
    for (const t of [...playerState.playHistoryTracks, ...playerState.library]) {
      const normalizedCount = Math.max(t.play_count || 0, historyIds.has(t.id) ? 1 : 0);
      const existing = map.get(t.id);
      if (existing) {
        if (normalizedCount > existing.play_count) {
          map.set(t.id, { ...t, play_count: normalizedCount });
        }
      } else {
        map.set(t.id, { ...t, play_count: normalizedCount });
      }
    }
    return Array.from(map.values());
  });

  let filteredTracks = $derived.by(() => {
    if (periodTracks !== null) {
      if (activePeriod === 'all' && periodTracks.length === 0) {
        return allTrackedSongs;
      }
      return periodTracks;
    }
    const p = periods.find((x) => x.id === activePeriod);
    if (!p || p.seconds === 0) return allTrackedSongs;
    const cutoff = Math.floor(Date.now() / 1000) - p.seconds;
    return allTrackedSongs.filter((t) => (t.date_added || 0) >= cutoff);
  });

  let totalPlays = $derived(
    filteredTracks.reduce((sum, t) => sum + Math.max(t.play_count || 0, 0), 0)
  );

  let totalSecondsListened = $derived(
    filteredTracks.reduce((sum, t) => sum + (t.duration > 0 ? t.duration : 0) * Math.max(t.play_count || 0, 0), 0)
  );

  let formattedListeningTime = $derived.by(() => {
    const totalMins = Math.round(totalSecondsListened / 60);
    if (totalMins < 60) return { main: `${totalMins}`, unit: 'mins' };
    const hrs = Math.floor(totalMins / 60);
    const mins = totalMins % 60;
    return { main: `${hrs}h ${mins}m`, unit: 'listened' };
  });

  let playedTracksSorted = $derived(
    [...filteredTracks]
      .filter((t) => (t.play_count || 0) > 0)
      .sort((a, b) => (b.play_count || 0) - (a.play_count || 0))
  );

  let mostPlayedSongs = $derived(playedTracksSorted.slice(0, 15));

  let allArtistsRanked = $derived.by(() => {
    const map = new Map<
      string,
      { name: string; plays: number; minutes: number; cover?: string; id: string }
    >();
    for (const t of filteredTracks) {
      const plays = Math.max(t.play_count || 0, 0);
      if (plays <= 0) continue;
      const primary = splitArtists(sanitizeArtist(t.artist))[0] || '';
      if (!primary || primary.toLowerCase() === 'unknown artist' || primary.toLowerCase() === 'various artists') continue;
      const key = primary.toLowerCase();
      const mins = ((t.duration > 0 ? t.duration : 0) * plays) / 60;
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
    return Array.from(map.values()).sort((a, b) => b.plays - a.plays || b.minutes - a.minutes);
  });

  let topArtists = $derived(allArtistsRanked.slice(0, 12));

  const IGNORED_ALBUM_LABELS = new Set([
    'unknown',
    'unknown album',
    'youtube music',
    'youtube music stream',
    'youtube music history',
    'liked songs (youtube music)',
    'lossless online stream',
    'aethon downloads',
    'up next • automix',
  ]);

  let topAlbums = $derived.by(() => {
    const map = new Map<
      string,
      { title: string; artist: string; plays: number; cover?: string; id: string }
    >();
    for (const t of filteredTracks) {
      const plays = Math.max(t.play_count || 0, 0);
      if (plays <= 0) continue;
      const alb = (t.album || '').trim();
      if (!alb || IGNORED_ALBUM_LABELS.has(alb.toLowerCase())) continue;
      const primaryArtist = splitArtists(sanitizeArtist(t.artist))[0] || 'Unknown Artist';
      const key = `${alb.toLowerCase()}::${primaryArtist.toLowerCase()}`;
      const existing = map.get(key);
      if (existing) {
        existing.plays += plays;
        if (!existing.cover && t.cover_art) existing.cover = t.cover_art;
      } else {
        map.set(key, {
          title: alb,
          artist: primaryArtist,
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
          class="px-3.5 py-1.5 rounded-xl text-xs font-medium whitespace-nowrap transition-colors border cursor-pointer {activePeriod === p.id
            ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] border-transparent font-semibold shadow-sm'
            : 'bg-white/[0.05] border-white/[0.06] text-white/65 hover:text-white hover:bg-white/10'}"
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
      <p class="text-[11px] text-white/45 mt-1">{playedTracksSorted.length} {playedTracksSorted.length === 1 ? 'unique track' : 'unique tracks'}</p>
    </div>

    <div class="p-4 sm:p-5 rounded-3xl m3-acrylic-card border border-white/[0.07]">
      <div class="flex items-center gap-2 text-white/55 text-xs font-medium mb-2">
        <span class="material-symbols-rounded text-amber-400 text-[18px]">interpreter_mode</span>
        <span>Artists</span>
      </div>
      <div class="font-display font-bold text-2xl sm:text-3xl text-white">
        {allArtistsRanked.length}
      </div>
      <p class="text-[11px] text-white/45 mt-1 truncate">
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
      <div class="font-display font-bold text-base sm:text-lg {settingsState.ytmConnected && settingsState.ytmRecapTracking ? 'text-emerald-400' : 'text-white/80'} truncate">
        {settingsState.ytmConnected
          ? settingsState.ytmRecapTracking
            ? 'Live Reporting Active'
            : 'Tracking Paused'
          : 'Local Mode'}
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
        Most played songs ({playedTracksSorted.length})
      </h2>
      {#if mostPlayedSongs.length > 0}
        <button
          type="button"
          onclick={() => {
            playerState.queue = mostPlayedSongs.slice(1);
            playerState.playTrack(mostPlayedSongs[0]);
          }}
          class="px-3.5 py-1 rounded-full bg-white/[0.06] hover:bg-white/[0.12] text-xs font-medium text-white/80 cursor-pointer transition-colors"
        >
          Play all
        </button>
      {/if}
    </div>

    {#if mostPlayedSongs.length > 0}
      <div class="divide-y divide-white/[0.04] rounded-2xl bg-white/[0.02] border border-white/[0.05] overflow-hidden">
        {#each mostPlayedSongs as track, idx (track.id)}
          {@const isCurrent = playerState.currentTrack?.id === track.id}
          {@const durStr = formatDuration(track.duration)}
          {@const mins = track.duration > 0 ? Math.max(1, Math.round((track.duration * Math.max(track.play_count, 1)) / 60)) : 0}
          <div
            role="button"
            tabindex="0"
            onclick={() => playerState.playTrack(track)}
            oncontextmenu={(e) => {
              e.preventDefault();
              playerState.openContextMenu(track, e.clientX, e.clientY);
            }}
            onkeydown={(e) => {
              if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                e.preventDefault();
                playerState.playTrack(track);
              }
            }}
            class="h-14 flex items-center justify-between px-3.5 transition-colors cursor-pointer group overflow-hidden {isCurrent ? '' : 'hover:bg-white/[0.05]'}"
            style={isCurrent ? 'background-color: color-mix(in srgb, var(--md-sys-primary-container) 35%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-primary) 35%, transparent);' : ''}
          >
            <div class="flex items-center gap-3.5 min-w-0 flex-1 pr-3">
              <span class="w-5 text-center font-display font-bold text-xs {idx < 3 ? 'text-[var(--md-sys-primary)]' : 'text-white/40'} shrink-0">
                {idx + 1}
              </span>

              <div class="relative w-10 h-10 rounded-xl overflow-hidden bg-white/5 shrink-0">
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
                {:else}
                  <div class="w-full h-full flex items-center justify-center text-white/30">
                    <span class="material-symbols-rounded text-lg">music_note</span>
                  </div>
                {/if}
                <div class="absolute inset-0 bg-black/50 {isCurrent ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'} flex items-center justify-center transition-opacity">
                  {#if isCurrent && isBuffering}
                    <svg class="w-4 h-4 archivetune-spinner-svg text-[var(--md-sys-primary)]" viewBox="0 0 36 36" fill="none">
                      <circle cx="18" cy="18" r="14" stroke="currentColor" stroke-opacity="0.2" stroke-width="3.2" />
                      <circle class="archivetune-spinner-arc" cx="18" cy="18" r="14" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
                    </svg>
                  {:else if isCurrent && playerState.playback.is_playing}
                    <div class="flex items-end gap-[2px] h-3.5">
                      <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-1"></span>
                      <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-2"></span>
                      <span class="w-[2.5px] h-full bg-[var(--md-sys-primary)] rounded-full animate-eq-3"></span>
                    </div>
                  {:else}
                    <span class="material-symbols-rounded text-white text-lg filled">play_arrow</span>
                  {/if}
                </div>
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

            <div class="flex items-center gap-2 shrink-0">
              <div class="text-right">
                <span class="text-xs font-semibold text-white/85 block">
                  {Math.max(track.play_count, 1)} {Math.max(track.play_count, 1) === 1 ? 'play' : 'plays'}
                </span>
                {#if mins > 0 || durStr}
                  <span class="text-[10px] font-mono text-white/40 block">
                    {mins > 0 ? `${mins} min` : ''}{mins > 0 && durStr ? ' • ' : ''}{durStr}
                  </span>
                {/if}
              </div>
              <button
                type="button"
                onclick={(e) => {
                  e.stopPropagation();
                  playerState.openContextMenu(track, e.clientX, e.clientY);
                }}
                class="w-7 h-7 rounded-full opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 flex items-center justify-center text-white/50 hover:text-white hover:bg-white/10 transition-opacity cursor-pointer"
                aria-label="More options"
                title="More options"
              >
                <span class="material-symbols-rounded text-base">more_vert</span>
              </button>
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
        Most played artists ({allArtistsRanked.length})
      </h2>

      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3.5">
        {#each topArtists as artist, idx}
          <button
            type="button"
            onclick={() => playerState.openArtist(artist.name)}
            class="p-4 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] flex flex-col items-center text-center gap-2.5 transition-colors cursor-pointer group relative"
          >
            <span class="absolute top-2.5 left-2.5 z-10 px-1.5 py-0.5 rounded-md bg-black/60 font-display font-bold text-[11px] {idx < 3 ? 'text-[var(--md-sys-primary)]' : 'text-white/60'}">
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
                {artist.plays} {artist.plays === 1 ? 'play' : 'plays'}{artist.minutes >= 1 ? ` • ${Math.round(artist.minutes)}m` : ''}
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

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5">
        {#each topAlbums as album}
          <button
            type="button"
            onclick={() => playerState.openAlbum(album.title, album.artist, album.cover)}
            class="p-3 rounded-2xl bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.06] flex items-center gap-3 text-left transition-colors cursor-pointer group overflow-hidden"
          >
            <div class="w-14 h-14 rounded-xl overflow-hidden bg-white/5 shrink-0 flex items-center justify-center">
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
              {:else}
                <span class="material-symbols-rounded text-2xl text-white/30">album</span>
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
