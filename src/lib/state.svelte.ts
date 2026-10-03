import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Track, PlaybackState, LyricsResult, YouTubeTrack, TabType } from './types';

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
  lyrics = $state<LyricsResult>({ synced: false, lines: [] });
  youtubeResults = $state<YouTubeTrack[]>([]);
  isSearching = $state(false);
  activeTab = $state<TabType>('home');
  isNowPlayingOpen = $state(false);
  isEqualizerOpen = $state(false);
  sidebarCollapsed = $state(false);
  accentColor = $state('#d0bcff');
  equalizerBands = $state<number[]>([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

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
  }

  async initTauriListeners() {
    try {
      await listen<PlaybackState>('player-state', (event) => {
        this.playback = event.payload;
      });

      await listen('track-ended', () => {
        this.playNext();
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
    this.currentTrack = track;
    try {
      await invoke('play_track', { uri: track.file_path, trackId: track.id });
      this.fetchLyrics(track.title, track.artist, track.duration);
      if (track.cover_art) {
        this.applyMonetColorFromImage(track.cover_art);
      }
    } catch (e) {
      console.error('Play error:', e);
    }
  }

  async playYouTube(yt: YouTubeTrack) {
    const syntheticTrack: Track = {
      id: `yt_${yt.id}`,
      title: yt.title,
      artist: yt.artist,
      album: 'YouTube Music Stream',
      duration: yt.duration,
      file_path: yt.stream_url,
      cover_art: yt.thumbnail,
      format: 'Opus 256k',
      play_count: 0,
      favorite: false,
      date_added: Date.now(),
    };
    this.currentTrack = syntheticTrack;
    try {
      await invoke('play_track', { uri: yt.stream_url, trackId: null });
      this.fetchLyrics(yt.title, yt.artist, yt.duration);
      if (yt.thumbnail) {
        this.applyMonetColorFromImage(yt.thumbnail);
      }
    } catch (e) {
      console.error('YouTube play error:', e);
    }
  }

  async togglePlay() {
    try {
      await invoke('toggle_play');
    } catch (e) {
      console.error('Toggle play error:', e);
    }
  }

  async seek(seconds: number) {
    try {
      await invoke('seek_track', { position: seconds });
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

  playNext() {
    if (this.queue.length > 0) {
      const next = this.queue.shift();
      if (next) this.playTrack(next);
    } else if (this.library.length > 0 && this.currentTrack) {
      const idx = this.library.findIndex((t) => t.id === this.currentTrack?.id);
      if (idx !== -1 && idx < this.library.length - 1) {
        this.playTrack(this.library[idx + 1]);
      }
    }
  }

  playPrevious() {
    if (this.playback.position > 3) {
      this.seek(0);
      return;
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

  private applyMonetColorFromImage(url: string) {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.src = url;
    img.onload = () => {
      try {
        const canvas = document.createElement('canvas');
        canvas.width = 64;
        canvas.height = 64;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        ctx.drawImage(img, 0, 0, 64, 64);
        const data = ctx.getImageData(0, 0, 64, 64).data;
        let r = 0, g = 0, b = 0, count = 0;
        for (let i = 0; i < data.length; i += 16) {
          if (data[i + 3] > 128) {
            r += data[i];
            g += data[i + 1];
            b += data[i + 2];
            count++;
          }
        }
        if (count > 0) {
          r = Math.round(r / count);
          g = Math.round(g / count);
          b = Math.round(b / count);
          const hex = `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
          this.accentColor = hex;
          document.documentElement.style.setProperty('--at-accent', hex);
          document.documentElement.style.setProperty('--at-accent-container', `rgba(${r}, ${g}, ${b}, 0.35)`);
          document.documentElement.style.setProperty('--at-on-accent-container', '#ffffff');
        }
      } catch (e) {
        console.warn('Canvas color extraction error:', e);
      }
    };
  }
}

export const playerState = new PlayerState();
