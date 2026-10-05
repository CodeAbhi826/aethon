import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Track, PlaybackState, LyricsResult, YouTubeTrack, TabType, Playlist } from './types';
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
  lyrics = $state<LyricsResult>({ synced: false, lines: [] });
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

  // Derived properties
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
        artwork: track.cover_art ? [
          { src: track.cover_art, sizes: '512x512', type: 'image/jpeg' }
        ] : [],
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
          // MPV has no file loaded yet. Do not wipe out restored track position/duration, and ensure is_playing is false on idle.
          this.playback = {
            ...event.payload,
            is_playing: false,
            position: this.currentTrack ? this.playback.position : event.payload.position,
            duration: this.currentTrack ? (this.playback.duration || (this.currentTrack.duration ?? 0)) : event.payload.duration,
          };
          return;
        }

        this.playback = event.payload;
        if (event.payload.is_playing || event.payload.position > 0.1) {
          this.isLoading = false;
        }

        if (this.pendingResumePosition !== null && (event.payload.is_playing || event.payload.position > 0)) {
          const target = this.pendingResumePosition;
          this.pendingResumePosition = null;
          this.seek(target);
          return;
        }

        if (this.currentTrack) {
          this.updateMediaSession(this.currentTrack);
          if (typeof localStorage !== 'undefined' && event.payload.position > 0 && this.pendingResumePosition === null) {
            try {
              localStorage.setItem('aethon_last_position', String(event.payload.position));
              localStorage.setItem('aethon_last_track_id', this.currentTrack.id);
            } catch {}
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
        this.playNext();
      });

      await listen('mpris-next', () => {
        this.playNext();
      });

      await listen('mpris-prev', () => {
        this.playPrevious();
      });

      await listen('mpris-playpause', () => {
        this.togglePlay();
      });

      await listen('mpris-play', () => {
        if (!this.playback.is_playing) {
          this.togglePlay();
        }
      });

      await listen('mpris-pause', () => {
        if (this.playback.is_playing) {
          this.togglePlay();
        }
      });

      await listen('tray-next', () => {
        this.playNext();
      });

      await listen('tray-prev', () => {
        this.playPrevious();
      });

      await listen('tray-mini-player', () => {
        this.toggleMiniPlayer();
      });

      // Initial state sync
      const state = await invoke<PlaybackState>('get_playback_state').catch(() => null);
      if (state) {
        this.playback = state;
      }
    } catch {
      // In browser preview / mock mode
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

  async playTrack(track: Track) {
    if (this.currentTrack && this.currentTrack.id !== track.id) {
      this.pendingResumePosition = null;
      this.history.unshift(this.currentTrack);
      if (this.history.length > 50) this.history.pop();
      if (typeof localStorage !== 'undefined') {
        try {
          localStorage.setItem('aethon_last_track_id', track.id);
          localStorage.setItem('aethon_last_position', '0');
        } catch {}
      }
    } else if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem('aethon_last_track_id', track.id);
      } catch {}
    }
    this.currentTrack = track;
    this.isLoading = true;
    this.updateMediaSession(track);
    this.updateMpris(track, true);
    this.notifyTrackChange(track);
    try {
      await invoke('play_track', { uri: track.file_path, trackId: track.id, track });
      this.fetchLyrics(track.title, track.artist, track.duration);
      this.loadPlayHistory();
      if (track.cover_art) {
        this.applyMonetColorFromImage(track.cover_art);
      } else {
        this.reapplyTheme();
      }
    } catch (e) {
      console.error('Play error:', e);
      this.isLoading = false;
    }
  }

  async playYouTube(yt: YouTubeTrack) {
    const syntheticTrack: Track = {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: yt.artist,
      album: 'Lossless Online Stream',
      duration: yt.duration,
      file_path: yt.stream_url,
      cover_art: yt.thumbnail,
      format: 'Opus 256k',
      play_count: 0,
      favorite: false,
      date_added: Date.now(),
    };
    if (this.currentTrack && this.currentTrack.id !== syntheticTrack.id) {
      this.pendingResumePosition = null;
      this.history.unshift(this.currentTrack);
      if (this.history.length > 50) this.history.pop();
      if (typeof localStorage !== 'undefined') {
        try {
          localStorage.setItem('aethon_last_track_id', syntheticTrack.id);
          localStorage.setItem('aethon_last_position', '0');
        } catch {}
      }
    } else if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem('aethon_last_track_id', syntheticTrack.id);
      } catch {}
    }
    this.currentTrack = syntheticTrack;
    this.isLoading = true;
    this.updateMediaSession(syntheticTrack);
    this.updateMpris(syntheticTrack, true);
    this.notifyTrackChange(syntheticTrack);
    try {
      await invoke('play_track', { uri: yt.stream_url, trackId: syntheticTrack.id, track: syntheticTrack });
      this.fetchLyrics(yt.title, yt.artist, yt.duration);
      this.loadPlayHistory();
      if (yt.thumbnail) {
        this.applyMonetColorFromImage(yt.thumbnail);
      } else {
        this.reapplyTheme();
      }
    } catch (e) {
      console.error('Stream play error:', e);
      this.isLoading = false;
    }
  }

  async togglePlay() {
    if (!this.playback.is_playing && this.currentTrack && (!this.playback.current_uri || this.playback.current_uri !== this.currentTrack.file_path)) {
      const resumePos = this.playback.position;
      if (resumePos > 0) {
        this.pendingResumePosition = resumePos;
      }
      await this.playTrack(this.currentTrack);
      if (resumePos > 0) {
        setTimeout(() => {
          if (this.pendingResumePosition !== null) {
            const target = this.pendingResumePosition;
            this.pendingResumePosition = null;
            this.seek(target);
          }
        }, 500);
      }
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
    } catch (e) {
      console.error('Toggle favorite error:', e);
    }
  }

  async fetchLyrics(title: string, artist: string, duration?: number) {
    try {
      const res = await invoke<LyricsResult>('get_lyrics', { title, artist, duration: duration || null });
      this.lyrics = res;
    } catch {
      this.lyrics = { synced: false, lines: [], plain_text: 'Lyrics unavailable for this track.' };
    }
  }

  async searchYouTube(query: string) {
    if (!query.trim()) return;
    this.isSearching = true;
    try {
      const res = await invoke<YouTubeTrack[]>('search_youtube', { query, limit: 12 });
      this.youtubeResults = res;
    } catch (e) {
      console.error('YouTube search error:', e);
    } finally {
      this.isSearching = false;
    }
  }

  addToQueue(track: Track) {
    this.queue = [...this.queue, track];
  }

  playNextInQueue(track: Track) {
    this.queue = [track, ...this.queue];
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

    // Autoplay Radio continuation (dynamic online stream recommendations)
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
              artist: yt.artist || 'Radio Pick',
              album: 'Autoplay Radio',
              duration: yt.duration || 210,
              file_path: yt.stream_url,
              cover_art: yt.thumbnail,
              format: 'Opus 256k',
              play_count: 0,
              favorite: false,
              date_added: Date.now(),
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
