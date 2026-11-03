import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  sanitizeArtist,
  type Track,
  type PlaybackState,
  type LyricsResult,
  type YouTubeTrack,
  type TabType,
  type Playlist,
  type PlaybackTrackingSession,
} from './types';
import { extractAndApplyThemeFromImage, generateThemeFromHex, applyThemeTokens } from './theme';
import { settingsState, registerThemeListener } from './settingsState.svelte';

class PlayerState {
  currentTrack = $state<Track | null>(null);
  playback = $state<PlaybackState>({
    current_uri: null,
    is_playing: false,
    position: 0,
    duration: 0,
    volume: 85,
  });
  queue = $state<Track[]>([]);
  library = $state<Track[]>([]);
  playlists = $state<Playlist[]>([]);
  selectedPlaylist = $state<Playlist | null>(null);
  playlistTracks = $state<Track[]>([]);
  playHistoryTracks = $state<Track[]>([]);
  currentMood = $state<string>('All');
  lyrics = $state<LyricsResult>({ synced: false, lines: [], provider: '', has_romanized: false });
  youtubeResults = $state<YouTubeTrack[]>([]);
  isSearching = $state(false);
  activeTab = $state<TabType>('home');
  isNowPlayingOpen = $state(false);
  isEqualizerOpen = $state(false);
  isQueueOpen = $state(false);
  isMiniPlayerOpen = $state(false);
  activeContextMenu = $state<{ track: Track; x: number; y: number } | null>(null);
  isNewPlaylistModalOpen = $state(false);
  isAddToPlaylistModalOpen = $state(false);
  targetTrackForPlaylist = $state<Track | null>(null);
  downloadingTrackIds = $state<string[]>([]);
  sidebarCollapsed = $state(false);
  accentColor = $state('#d0bcff');
  equalizerBands = $state<number[]>([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
  isLoading = $state(false);
  shuffleMode = $state(false);
  repeatMode = $state<'off' | 'all' | 'one'>('off');
  autoplayRadio = $state(true);
  history = $state<Track[]>([]);
  selectedArtist = $state<string | null>(null);
  selectedAlbum = $state<{ title: string; artist: string; cover?: string; year?: string } | null>(null);
  lastMprisPlaying = false;
  pendingResumePosition: number | null = null;
  currentRecapSession = $state<PlaybackTrackingSession | null>(null);
  lastRecapReportedPos = 0;

  formattedTime = $derived.by(() => {
    const cur = this.formatSeconds(this.playback.position);
    const dur = this.formatSeconds(this.playback.duration || (this.currentTrack?.duration ?? 0));
    return { current: cur, total: dur };
  });

  progressPercent = $derived.by(() => {
    const total = this.playback.duration || (this.currentTrack?.duration ?? 0);
    if (!total || total <= 0) return 0;
    return Math.min(100, Math.max(0, (this.playback.position / total) * 100));
  });

  constructor() {
    this.initTauriListeners();
    this.initMediaSession();
    this.loadLibrary();
    this.loadPlaylists();
    this.loadPlayHistory();
    registerThemeListener(() => {
      this.reapplyTheme();
    });
  }

  initMediaSession() {
    if (typeof navigator !== 'undefined' && 'mediaSession' in navigator) {
      navigator.mediaSession.setActionHandler('play', () => this.togglePlay());
      navigator.mediaSession.setActionHandler('pause', () => this.togglePlay());
      navigator.mediaSession.setActionHandler('previoustrack', () => this.playPrevious());
      navigator.mediaSession.setActionHandler('nexttrack', () => this.playNext());
      navigator.mediaSession.setActionHandler('seekto', (details) => {
        if (details.seekTime !== undefined) {
          this.seek(details.seekTime);
        }
      });
    }
  }

  updateMediaSession(track: Track) {
    if (typeof navigator !== 'undefined' && 'mediaSession' in navigator) {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: track.title,
        artist: track.artist,
        album: track.album || 'Aethon',
        artwork: track.cover_art
          ? [{ src: track.cover_art, sizes: '512x512', type: 'image/jpeg' }]
          : [],
      });
      navigator.mediaSession.playbackState = this.playback.is_playing ? 'playing' : 'paused';
    }
  }

  async updateMpris(track: Track, isPlaying: boolean) {
    if (!settingsState.mprisIntegration) return;
    try {
      await invoke('update_mpris_metadata', {
        title: track.title,
        artist: track.artist,
        album: track.album || 'Aethon',
        cover: track.cover_art || '',
        durationUs: Math.round((track.duration || this.playback.duration || 0) * 1_000_000),
        isPlaying,
      });
    } catch (e) {
      console.warn('update_mpris_metadata error:', e);
    }
  }

  async notifyTrackChange(track: Track) {
    if (!settingsState.trackChangeNotifications) return;
    if (typeof document !== 'undefined' && !document.hasFocus()) {
      try {
        await invoke('send_desktop_notification', {
          title: track.title,
          artist: track.artist,
          coverArt: track.cover_art || null,
        });
      } catch (e) {
        console.warn('send_desktop_notification error:', e);
      }
    }
  }

  async loadPlayHistory() {
    try {
      const history = await invoke<Track[]>('get_play_history', { limit: 20 });
      if (history && history.length > 0) {
        this.playHistoryTracks = history;
        if (!this.currentTrack) {
          const lastTrack = history[0];
          this.currentTrack = lastTrack;
          this.playback.duration = lastTrack.duration || 0;
          this.playback.is_playing = false;

          if (typeof localStorage !== 'undefined') {
            const savedTrackId = localStorage.getItem('aethon_last_track_id');
            const savedPos = localStorage.getItem('aethon_last_position');
            if (savedPos && (!savedTrackId || savedTrackId === lastTrack.id)) {
              const pos = parseFloat(savedPos);
              if (!isNaN(pos) && pos >= 0 && (!lastTrack.duration || pos <= lastTrack.duration)) {
                this.playback.position = pos;
              }
            }
          }

          if (lastTrack.cover_art) {
            this.applyMonetColorFromImage(lastTrack.cover_art);
          }
        }
      }
    } catch (e) {
      console.warn('get_play_history error:', e);
    }
  }

  async initTauriListeners() {
    try {
      await listen<PlaybackState>('player-state', (event) => {
        if (!event.payload.current_uri) {
          this.playback = {
            ...event.payload,
            is_playing: false,
            position: this.currentTrack ? this.playback.position : event.payload.position,
            duration: this.currentTrack
              ? this.playback.duration || (this.currentTrack.duration ?? 0)
              : event.payload.duration,
          };
          return;
        }

        this.playback = event.payload;
        if (event.payload.is_playing || event.payload.position > 0.05) {
          this.isLoading = false;
        }

        // Automatic Track Reconciliation between MPV backend and Svelte UI
        if (event.payload.current_uri && event.payload.is_playing) {
          this.reconcilePlayingTrack(event.payload.current_uri, event.payload.media_title);
        }

        if (
          this.pendingResumePosition !== null &&
          event.payload.position > 0.05 &&
          event.payload.duration > 0
        ) {
          const target = this.pendingResumePosition;
          this.pendingResumePosition = null;
          this.seek(target);
          return;
        }

        if (this.currentTrack) {
          this.updateMediaSession(this.currentTrack);
          if (
            typeof localStorage !== 'undefined' &&
            event.payload.position > 0 &&
            this.pendingResumePosition === null
          ) {
            try {
              localStorage.setItem('aethon_last_position', String(event.payload.position));
              localStorage.setItem('aethon_last_track_id', this.currentTrack.id);
            } catch {}
          }
        }

        // Official YouTube Music Recap watch-time heartbeat (reported every ~25s of playback)
        if (
          event.payload.is_playing &&
          this.currentRecapSession &&
          settingsState.ytmConnected &&
          settingsState.ytmRecapTracking &&
          settingsState.ytmCookie
        ) {
          const pos = event.payload.position;
          if (pos - this.lastRecapReportedPos >= 25) {
            const startSec = this.lastRecapReportedPos;
            const endSec = pos;
            this.lastRecapReportedPos = pos;
            invoke('ping_recap_watchtime', {
              cookie: settingsState.ytmCookie,
              visitorData: null,
              session: this.currentRecapSession,
              startSec,
              endSec,
              isPlaying: true,
            }).catch((err) => console.warn('ping_recap_watchtime error:', err));
          }
        }

        if (settingsState.mprisIntegration) {
          if (this.playback.is_playing !== this.lastMprisPlaying) {
            this.lastMprisPlaying = this.playback.is_playing;
            invoke('update_mpris_status', {
              isPlaying: this.playback.is_playing,
              positionUs: Math.round(this.playback.position * 1_000_000),
            }).catch(() => {});
          }
        }
      });

      await listen('track-ended', () => {
        if (
          this.currentRecapSession &&
          settingsState.ytmConnected &&
          settingsState.ytmRecapTracking &&
          settingsState.ytmCookie
        ) {
          const endSec =
            this.playback.duration || (this.currentTrack ? this.currentTrack.duration : 0);
          if (endSec > this.lastRecapReportedPos) {
            invoke('ping_recap_watchtime', {
              cookie: settingsState.ytmCookie,
              visitorData: null,
              session: this.currentRecapSession,
              startSec: this.lastRecapReportedPos,
              endSec,
              isPlaying: false,
            }).catch(() => {});
          }
        }
        this.currentRecapSession = null;
        this.lastRecapReportedPos = 0;
        this.playNext();
      });

      await listen('mpris-next', () => this.playNext());
      await listen('mpris-prev', () => this.playPrevious());
      await listen('mpris-playpause', () => this.togglePlay());
      await listen('mpris-play', () => {
        if (!this.playback.is_playing) this.togglePlay();
      });
      await listen('mpris-pause', () => {
        if (this.playback.is_playing) this.togglePlay();
      });
      await listen('tray-next', () => this.playNext());
      await listen('tray-prev', () => this.playPrevious());
      await listen('tray-playpause', () => this.togglePlay());
      await listen('tray-mini-player', () => this.toggleMiniPlayer());

      const state = await invoke<PlaybackState>('get_playback_state').catch(() => null);
      if (state) {
        if (!state.current_uri) {
          this.playback = {
            ...state,
            is_playing: false,
            position: this.currentTrack ? this.playback.position : state.position,
            duration: this.currentTrack
              ? this.playback.duration || (this.currentTrack.duration ?? 0)
              : state.duration,
          };
        } else {
          this.playback = state;
        }
      }
    } catch {
      // Browser preview mode
    }
  }

  async loadLibrary() {
    try {
      const tracks = await invoke<Track[]>('get_library_tracks');
      this.library = tracks;
    } catch (e) {
      console.warn('Error loading library from SQLite:', e);
    }
  }

  async refreshLibrary() {
    await this.loadLibrary();
    await this.loadPlaylists();
    await this.loadPlayHistory();
  }

  async loadPlaylists() {
    try {
      const pls = await invoke<Playlist[]>('get_playlists');
      this.playlists = pls;
    } catch (e) {
      console.warn('Error loading playlists from SQLite:', e);
    }
  }

  async createPlaylist(name: string, description?: string) {
    if (!name.trim()) return null;
    try {
      const pl = await invoke<Playlist>('create_playlist', {
        name: name.trim(),
        description: description?.trim() || null,
      });
      this.playlists = [pl, ...this.playlists];
      return pl;
    } catch (e) {
      console.error('Create playlist error:', e);
      return null;
    }
  }

  async deletePlaylist(id: string) {
    try {
      await invoke('delete_playlist', { playlistId: id });
      this.playlists = this.playlists.filter((p) => p.id !== id);
      if (this.selectedPlaylist?.id === id) {
        this.selectedPlaylist = null;
        this.playlistTracks = [];
      }
    } catch (e) {
      console.error('Delete playlist error:', e);
    }
  }

  async selectPlaylist(playlist: Playlist) {
    this.selectedPlaylist = playlist;
    try {
      const tracks = await invoke<Track[]>('get_playlist_tracks', { playlistId: playlist.id });
      this.playlistTracks = tracks;
    } catch (e) {
      console.error('Error fetching playlist tracks:', e);
      this.playlistTracks = [];
    }
  }

  async addToPlaylist(playlistId: string, track: Track) {
    try {
      await invoke('add_to_playlist', { playlistId, trackId: track.id });
      await this.loadPlaylists();
      if (this.selectedPlaylist?.id === playlistId) {
        await this.selectPlaylist(this.selectedPlaylist);
      }
    } catch (e) {
      console.error('Error adding track to playlist:', e);
    }
  }

  async removeFromPlaylist(playlistId: string, trackId: string) {
    try {
      await invoke('remove_from_playlist', { playlistId, trackId });
      await this.loadPlaylists();
      if (this.selectedPlaylist?.id === playlistId) {
        this.playlistTracks = this.playlistTracks.filter((t) => t.id !== trackId);
      }
    } catch (e) {
      console.error('Error removing track from playlist:', e);
    }
  }

  openContextMenu(track: Track, x: number, y: number) {
    this.activeContextMenu = { track, x, y };
  }

  closeContextMenu() {
    this.activeContextMenu = null;
  }

  openAddToPlaylist(track: Track) {
    this.targetTrackForPlaylist = track;
    this.isAddToPlaylistModalOpen = true;
  }

  async downloadTrack(track: Track) {
    if (this.downloadingTrackIds.includes(track.id)) return;
    this.downloadingTrackIds = [...this.downloadingTrackIds, track.id];
    try {
      const downloaded = await invoke<Track>('download_track', {
        id: track.id,
        title: track.title,
        artist: track.artist,
        url: track.file_path,
        thumbnail: track.cover_art || null,
      });
      if (!this.library.some((t) => t.id === downloaded.id)) {
        this.library = [downloaded, ...this.library];
      }
    } catch (e) {
      console.error('Download track error:', e);
    } finally {
      this.downloadingTrackIds = this.downloadingTrackIds.filter((id) => id !== track.id);
    }
  }

  async revealInFileManager(filePath: string) {
    try {
      await invoke('reveal_in_file_manager', { path: filePath });
    } catch (e) {
      console.error('Reveal in file manager error:', e);
    }
  }

  async setCrossfade(seconds: number) {
    try {
      await invoke('set_crossfade', { seconds });
    } catch (e) {
      console.error('Set crossfade error:', e);
    }
  }

  async setReplayGain(mode: string) {
    try {
      await invoke('set_replaygain', { mode });
    } catch (e) {
      console.error('Set replaygain error:', e);
    }
  }

  toggleMiniPlayer() {
    this.isMiniPlayerOpen = !this.isMiniPlayerOpen;
  }

  async scanLibrary(path?: string) {
    try {
      const tracks = await invoke<Track[]>('scan_library', { path: path || null });
      this.library = tracks;
      return tracks;
    } catch (e) {
      console.error('Scan error:', e);
      return [];
    }
  }

  prefetchStream(uri: string, title?: string, artist?: string) {
    if (!uri || (!uri.includes('youtube.com/') && !uri.includes('youtu.be/'))) return;
    invoke('prefetch_stream', {
      uri,
      title: title || null,
      artist: artist || null,
    }).catch(() => {});
  }

  prefetchNextInQueue() {
    const next = this.queue[0];
    if (next && next.file_path) {
      this.prefetchStream(next.file_path, next.title, next.artist);
    }
  }

  extractVideoId(uri: string | null | undefined): string | null {
    if (!uri) return null;
    const m = uri.match(/(?:v=|youtu\.be\/|\/embed\/|\/watch\?v=)([0-9A-Za-z_-]{11})/);
    if (m) return m[1];
    if (uri.startsWith('yt_')) return uri.replace('yt_', '');
    return null;
  }

  isSameTrack(track: Track | null | undefined, uri: string | null | undefined): boolean {
    if (!track || !uri) return false;
    if (track.file_path === uri) return true;
    const v1 = this.extractVideoId(track.file_path) || this.extractVideoId(track.id);
    const v2 = this.extractVideoId(uri);
    if (v1 && v2 && v1 === v2) return true;
    return false;
  }

  reconcilePlayingTrack(uri: string, mediaTitle?: string | null) {
    if (this.isSameTrack(this.currentTrack, uri)) {
      return;
    }

    const candidate = [
      ...this.queue,
      ...this.playHistoryTracks,
      ...this.history,
      ...this.library,
    ].find((t) => this.isSameTrack(t, uri));

    if (candidate) {
      this.currentTrack = candidate;
    } else {
      const vid = this.extractVideoId(uri);
      const title =
        mediaTitle && mediaTitle.trim()
          ? mediaTitle.trim()
          : vid
            ? `YouTube Track (${vid})`
            : 'Audio Stream';
      const synthetic: Track = {
        id: vid ? `yt_${vid}` : uri,
        title,
        artist: 'Unknown Artist',
        album: 'Aethon Stream',
        duration: this.playback.duration || 0,
        file_path: uri,
        cover_art: vid ? `https://i.ytimg.com/vi/${vid}/hqdefault.jpg` : '',
        format: 'Opus',
        play_count: 1,
        favorite: false,
        date_added: Math.floor(Date.now() / 1000),
      };
      this.currentTrack = synthetic;
    }

    if (this.currentTrack) {
      const vid =
        this.extractVideoId(this.currentTrack.file_path) ||
        this.extractVideoId(this.currentTrack.id);
      this.fetchLyrics(
        this.currentTrack.title,
        this.currentTrack.artist,
        this.currentTrack.duration,
        this.currentTrack.file_path,
        vid
      );
      if (this.currentTrack.cover_art) {
        this.applyMonetColorFromImage(this.currentTrack.cover_art);
      }
      this.updateMediaSession(this.currentTrack);
      this.updateMpris(this.currentTrack, true);
      this.notifyTrackChange(this.currentTrack);
    }
  }

  async playTrack(track: Track) {
    let filePath = track.file_path;
    if (filePath && filePath.startsWith('watch?v=')) {
      filePath = `https://www.youtube.com/${filePath}`;
    } else if (filePath && filePath.startsWith('/watch?v=')) {
      filePath = `https://www.youtube.com${filePath}`;
    } else if (track.id && track.id.startsWith('yt_') && (!filePath || !filePath.startsWith('http'))) {
      filePath = `https://www.youtube.com/watch?v=${track.id.replace('yt_', '')}`;
    }

    const sanitized: Track = {
      ...track,
      file_path: filePath,
      artist: sanitizeArtist(track.artist),
    };
    if (this.currentTrack && this.currentTrack.id !== sanitized.id) {
      this.pendingResumePosition = null;
      this.history.unshift(this.currentTrack);
      if (this.history.length > 50) this.history.pop();
      if (typeof localStorage !== 'undefined') {
        try {
          localStorage.setItem('aethon_last_track_id', sanitized.id);
          localStorage.setItem('aethon_last_position', '0');
        } catch {}
      }
    } else if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem('aethon_last_track_id', sanitized.id);
      } catch {}
    }

    this.currentTrack = sanitized;
    this.isLoading = true;
    this.updateMediaSession(sanitized);
    this.updateMpris(sanitized, true);
    this.notifyTrackChange(sanitized);

    let videoId: string | null = null;
    if (sanitized.id.startsWith('yt_')) {
      videoId = sanitized.id.replace('yt_', '');
    } else if (
      sanitized.file_path.startsWith('http://') ||
      sanitized.file_path.startsWith('https://')
    ) {
      const match = sanitized.file_path.match(/(?:v=|\/)([0-9A-Za-z_-]{11})/);
      if (match) videoId = match[1];
    }

    // Fire lyrics lookup & theme extraction in parallel with stream playback so nothing blocks audio startup
    this.fetchLyrics(
      sanitized.title,
      sanitized.artist,
      sanitized.duration,
      sanitized.file_path,
      videoId
    );
    if (sanitized.cover_art) {
      this.applyMonetColorFromImage(sanitized.cover_art);
    } else {
      this.reapplyTheme();
    }

    if (
      videoId &&
      settingsState.ytmConnected &&
      settingsState.ytmRecapTracking &&
      settingsState.ytmCookie
    ) {
      invoke<PlaybackTrackingSession>('init_recap_playback_tracking', {
        cookie: settingsState.ytmCookie,
        visitorData: null,
        pageId: null,
        videoId,
      })
        .then((session) => {
          this.currentRecapSession = session;
          this.lastRecapReportedPos = 0;
        })
        .catch((err) => {
          console.warn('init_recap_playback_tracking error:', err);
          this.currentRecapSession = null;
        });
    } else {
      this.currentRecapSession = null;
      this.lastRecapReportedPos = 0;
    }

    try {
      await invoke('play_track', {
        uri: sanitized.file_path,
        trackId: sanitized.id,
        track: sanitized,
      });
      this.loadPlayHistory();
      this.prefetchNextInQueue();
    } catch (e) {
      console.error('Play error:', e);
      this.isLoading = false;
    }
  }

  async playYouTube(yt: YouTubeTrack) {
    const syntheticTrack: Track = {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: sanitizeArtist(yt.artist),
      album: 'YouTube Music',
      duration: yt.duration,
      file_path: yt.stream_url,
      cover_art: yt.thumbnail,
      format: 'Opus 256k',
      play_count: 0,
      favorite: false,
      date_added: Math.floor(Date.now() / 1000),
    };
    await this.playTrack(syntheticTrack);
  }

  async togglePlay() {
    // If track is at the very end or cold boot, replay from beginning
    const isAtEnd =
      this.playback.duration > 0 &&
      this.playback.position >= Math.max(1, this.playback.duration - 0.5);

    if (isAtEnd && this.currentTrack) {
      await this.seek(0);
      try {
        await invoke('toggle_play');
      } catch {}
      return;
    }

    // Only reload if MPV has no active stream loaded (e.g. cold boot with restored track from localStorage)
    if (!this.playback.is_playing && this.currentTrack && !this.playback.current_uri) {
      const resumePos = this.playback.position;
      if (resumePos > 0) {
        this.pendingResumePosition = resumePos;
      }
      await this.playTrack(this.currentTrack);
      return;
    }
    try {
      await invoke('toggle_play');
    } catch (e) {
      console.error('Toggle play error:', e);
    }
  }

  async seek(seconds: number) {
    this.pendingResumePosition = null;
    this.playback.position = seconds;
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem('aethon_last_position', String(seconds));
        if (this.currentTrack) {
          localStorage.setItem('aethon_last_track_id', this.currentTrack.id);
        }
      } catch {}
    }
    try {
      await invoke('seek_track', { position: seconds });
      if (settingsState.mprisIntegration) {
        invoke('update_mpris_status', {
          isPlaying: this.playback.is_playing,
          positionUs: Math.round(seconds * 1_000_000),
          seeked: true,
        }).catch(() => {});
      }
    } catch (e) {
      console.error('Seek error:', e);
    }
  }

  async setVolume(vol: number) {
    this.playback.volume = vol;
    try {
      await invoke('set_volume', { volume: vol });
    } catch (e) {
      console.error('Volume error:', e);
    }
  }

  async setEqualizer(bands: number[]) {
    this.equalizerBands = bands;
    try {
      await invoke('set_equalizer', { bands });
    } catch (e) {
      console.error('Equalizer error:', e);
    }
  }

  async toggleFavorite(trackId: string) {
    try {
      const fav = await invoke<boolean>('toggle_favorite', { trackId });
      const t = this.library.find((item) => item.id === trackId);
      if (t) t.favorite = fav;
      if (this.currentTrack?.id === trackId) {
        this.currentTrack.favorite = fav;
      }
      if (trackId.startsWith('yt_') && settingsState.ytmConnected && settingsState.ytmCookie) {
        const videoId = trackId.replace('yt_', '');
        invoke('set_ytm_like', {
          cookie: settingsState.ytmCookie,
          visitorData: null,
          pageId: null,
          videoId,
          like: fav,
        }).catch((err) => console.warn('set_ytm_like error:', err));
      }
    } catch (e) {
      console.error('Toggle favorite error:', e);
    }
  }

  async fetchLyrics(
    title: string,
    artist: string,
    duration?: number,
    filePath?: string,
    videoId?: string | null
  ) {
    try {
      const res = await invoke<LyricsResult>('get_lyrics', {
        title,
        artist,
        duration: duration || null,
        filePath: filePath || null,
        videoId: videoId || null,
      });
      this.lyrics = res;
    } catch {
      this.lyrics = {
        synced: false,
        lines: [],
        plain_text: undefined,
        provider: '',
        has_romanized: false,
      };
    }
  }

  async searchYouTube(query: string) {
    if (!query.trim()) return;
    this.isSearching = true;
    try {
      const res = await invoke<YouTubeTrack[]>('search_youtube', { query, limit: 25 });
      this.youtubeResults = res;
      if (res.length > 0) {
        this.prefetchStream(res[0].stream_url, res[0].title, res[0].artist);
      }
    } catch (e) {
      console.error('YouTube search error:', e);
    } finally {
      this.isSearching = false;
    }
  }

  addToQueue(track: Track) {
    const wasEmpty = this.queue.length === 0;
    this.queue = [...this.queue, track];
    if (wasEmpty) {
      this.prefetchNextInQueue();
    }
  }

  playNextInQueue(track: Track) {
    this.queue = [track, ...this.queue];
    this.prefetchNextInQueue();
  }

  removeFromQueue(idx: number) {
    this.queue = this.queue.filter((_, i) => i !== idx);
  }

  clearQueue() {
    this.queue = [];
  }

  toggleShuffle() {
    this.shuffleMode = !this.shuffleMode;
  }

  toggleRepeat() {
    if (this.repeatMode === 'off') this.repeatMode = 'all';
    else if (this.repeatMode === 'all') this.repeatMode = 'one';
    else this.repeatMode = 'off';
  }

  openArtist(artistName: string) {
    if (!artistName) return;
    this.selectedArtist = artistName;
    this.activeTab = 'artist';
  }

  openAlbum(title: string, artist: string, cover?: string, year?: string) {
    if (!title) return;
    this.selectedAlbum = { title, artist, cover, year };
    this.activeTab = 'album';
  }

  async playNext() {
    if (this.repeatMode === 'one' && this.currentTrack) {
      this.seek(0);
      return;
    }

    if (this.queue.length > 0) {
      const next = this.queue[0];
      this.queue = this.queue.slice(1);
      if (next) {
        await this.playTrack(next);
        return;
      }
    }

    if (this.autoplayRadio && this.currentTrack) {
      try {
        const query = `${this.currentTrack.artist} songs`;
        const res = await invoke<YouTubeTrack[]>('search_youtube', { query, limit: 6 });
        if (res && res.length > 0) {
          const suggestions: Track[] = res
            .filter((yt) => `yt_${yt.id}` !== this.currentTrack?.id)
            .map((yt) => ({
              id: `yt_${yt.id}`,
              title: yt.title,
              artist: sanitizeArtist(yt.artist) || 'Radio',
              album: 'Autoplay',
              duration: yt.duration || 210,
              file_path: yt.stream_url,
              cover_art: yt.thumbnail,
              format: 'Opus 256k',
              play_count: 0,
              favorite: false,
              date_added: Math.floor(Date.now() / 1000),
            }));

          if (suggestions.length > 0) {
            const nextTrack = suggestions[0];
            this.queue = suggestions.slice(1);
            await this.playTrack(nextTrack);
            return;
          }
        }
      } catch (e) {
        console.warn('Autoplay radio suggestion failed:', e);
      }
    }

    if (this.library.length > 0 && this.currentTrack) {
      const idx = this.library.findIndex((t) => t.id === this.currentTrack?.id);
      if (this.shuffleMode) {
        const randomIdx = Math.floor(Math.random() * this.library.length);
        this.playTrack(this.library[randomIdx]);
      } else if (idx !== -1 && idx < this.library.length - 1) {
        this.playTrack(this.library[idx + 1]);
      } else if (this.repeatMode === 'all') {
        this.playTrack(this.library[0]);
      }
    }
  }

  playPrevious() {
    if (this.playback.position > 3) {
      this.seek(0);
      return;
    }
    if (this.history.length > 0) {
      const prev = this.history.shift();
      if (prev) {
        this.playTrack(prev);
        return;
      }
    }
    if (this.library.length > 0 && this.currentTrack) {
      const idx = this.library.findIndex((t) => t.id === this.currentTrack?.id);
      if (idx > 0) {
        this.playTrack(this.library[idx - 1]);
      }
    }
  }

  private formatSeconds(sec: number): string {
    const s = Math.floor(sec || 0);
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return `${m}:${rem < 10 ? '0' : ''}${rem}`;
  }

  reapplyTheme() {
    if (!settingsState.dynamicTheme || settingsState.themePreset !== 'dynamic') {
      return;
    }
    const cover = this.currentTrack?.cover_art;
    if (cover) {
      this.applyMonetColorFromImage(cover);
    } else {
      const tokens = generateThemeFromHex('#d0bcff', {
        dynamicBg: settingsState.dynamicBackground,
        accentIntensity: settingsState.accentIntensity,
        pureOled: false,
      });
      applyThemeTokens(tokens);
      this.accentColor = '#d0bcff';
    }
  }

  async applyMonetColorFromImage(url: string) {
    if (!settingsState.dynamicTheme || settingsState.themePreset !== 'dynamic') {
      return;
    }
    try {
      const hex = await extractAndApplyThemeFromImage(url, {
        dynamicBg: settingsState.dynamicBackground,
        accentIntensity: settingsState.accentIntensity,
        pureOled: false,
      });
      this.accentColor = hex;
    } catch (e) {
      console.warn('Monet color extraction failed:', e);
    }
  }
}

export const playerState = new PlayerState();

