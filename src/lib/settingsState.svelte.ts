import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { generateThemeFromHex, applyThemeTokens, THEME_PRESETS } from './theme';
import type { YtmSyncSummary } from './types';

export type ThemeListener = () => void;
const themeListeners: Set<ThemeListener> = new Set();

export function registerThemeListener(fn: ThemeListener) {
  themeListeners.add(fn);
}

export function notifyThemeListeners() {
  for (const fn of themeListeners) {
    fn();
  }
}

export interface SavedYtmAccount {
  name: string;
  email: string;
  avatar: string;
  cookie: string;
}

class SettingsState {
  // 1. Appearance & Material You
  dynamicTheme = $state(true);
  themePreset = $state<'dynamic' | 'violet' | 'crimson' | 'emerald' | 'amber' | 'ocean' | 'rose'>('dynamic');
  dynamicBackground = $state<'ambient' | 'glass' | 'solid' | 'oled'>('ambient');
  accentIntensity = $state<'subtle' | 'standard' | 'vibrant'>('standard');
  blurStrength = $state(48);
  uiScale = $state(100);
  compactMode = $state(false);
  fontStyle = $state<'outfit' | 'inter' | 'system'>('outfit');

  // 2. Now Playing & Lyrics
  defaultNowPlayingView = $state<'auto' | 'turntable' | 'lyrics'>('auto');
  vinylSpinSpeed = $state<'slow' | 'normal' | 'fast'>('normal');
  showVisualizer = $state(true);
  visualizerStyle = $state<'bars' | 'wave' | ' orbs'>('bars');
  visualizerSensitivity = $state(75);
  showAudioQualityBadge = $state(true);
  lyricsFontSize = $state<'small' | 'medium' | 'large' | 'xlarge'>('large');
  lyricsAlignment = $state<'left' | 'center'>('left');
  blurInactiveLines = $state(true);
  wordByWordTracking = $state(true);
  lyricOffsetSeconds = $state(0.0);
  autoScrollLyrics = $state(true);
  romanizeLyricsByDefault = $state(true);

  // 3. Audio & Playback Engine
  audioBackend = $state<'pipewire' | 'pulse' | 'alsa'>('pipewire');
  exclusiveAudio = $state(false);
  gaplessPlayback = $state(true);
  crossfadeDuration = $state(0);
  replayGainMode = $state<'off' | 'track' | 'album'>('off');
  replayGainPreamp = $state(0);
  audioNormalization = $state(false);
  streamQuality = $state<'low' | 'normal' | 'high' | 'lossless'>('lossless');
  downloadQuality = $state<'128k' | '256k' | '320k' | 'flac'>('320k');
  rememberPlaybackPosition = $state(true);
  autoplaySimilar = $state(true);

  // 4. Library & Storage
  libraryPath = $state('~/Music');
  downloadPath = $state('~/Music/Aethon Downloads');
  autoScanOnStartup = $state(true);
  watchFolderChanges = $state(true);
  skipShortTracks = $state(true);
  minTrackDurationSec = $state(15);
  maxStreamCacheMB = $state(1024);

  // 5. Desktop & System Integration
  closeToTray = $state(true);
  startMinimized = $state(false);
  mprisIntegration = $state(true);
  trackChangeNotifications = $state(true);
  discordRichPresence = $state(false);
  hardwareAcceleration = $state(true);
  DiscordClientId = $state('1240000000000000000');

  // 6. YouTube Music & Cloud Sync
  ytmConnected = $state(false);
  ytmAccountName = $state('Guest Listener');
  ytmAccountEmail = $state('');
  ytmAccountAvatar = $state('');
  ytmCookie = $state('');
  ytmSyncPlaylists = $state(true);
  ytmSyncFavorites = $state(true);
  ytmSyncHistory = $state(true);
  ytmAutoSync = $state(true);
  ytmRecapTracking = $state(true);
  isSyncingYtm = $state(false);
  lastSyncSummary = $state<YtmSyncSummary | null>(null);
  savedAccounts = $state<SavedYtmAccount[]>([]);

  private saveTimer: ReturnType<typeof setTimeout> | null = null;
  private initialized = false;

  constructor() {
    this.loadFromBackend();
    this.initYtmAuthListener();
  }

  async initYtmAuthListener() {
    try {
      await listen<{
        cookie: string;
        name: string;
        email: string;
        avatar: string;
      }>('ytm-auth-success', async (event) => {
        const { cookie, name, email, avatar } = event.payload;
        this.connectYtmAccount(cookie, name, email, avatar);
        if (this.ytmAutoSync) {
          await this.syncYtmLibrary();
        }
      });
    } catch {
      // Browser preview fallback
    }
  }

  async loadFromBackend() {
    try {
      const map = await invoke<Record<string, string>>('get_all_settings');
      if (map) {
        if (map.dynamicTheme !== undefined) this.dynamicTheme = map.dynamicTheme === 'true';
        if (map.themePreset) this.themePreset = map.themePreset as any;
        if (map.dynamicBackground) this.dynamicBackground = map.dynamicBackground as any;
        if (map.accentIntensity) this.accentIntensity = map.accentIntensity as any;
        if (map.blurStrength) this.blurStrength = parseInt(map.blurStrength, 10) || 48;
        if (map.uiScale) this.uiScale = parseInt(map.uiScale, 10) || 100;
        if (map.compactMode !== undefined) this.compactMode = map.compactMode === 'true';
        if (map.fontStyle) this.fontStyle = map.fontStyle as any;

        if (map.defaultNowPlayingView) this.defaultNowPlayingView = map.defaultNowPlayingView as any;
        if (map.vinylSpinSpeed) this.vinylSpinSpeed = map.vinylSpinSpeed as any;
        if (map.showVisualizer !== undefined) this.showVisualizer = map.showVisualizer === 'true';
        if (map.visualizerStyle) this.visualizerStyle = map.visualizerStyle as any;
        if (map.visualizerSensitivity) this.visualizerSensitivity = parseInt(map.visualizerSensitivity, 10) || 75;
        if (map.showAudioQualityBadge !== undefined) this.showAudioQualityBadge = map.showAudioQualityBadge === 'true';
        if (map.lyricsFontSize) this.lyricsFontSize = map.lyricsFontSize as any;
        if (map.lyricsAlignment) this.lyricsAlignment = map.lyricsAlignment as any;
        if (map.blurInactiveLines !== undefined) this.blurInactiveLines = map.blurInactiveLines === 'true';
        if (map.wordByWordTracking !== undefined) this.wordByWordTracking = map.wordByWordTracking === 'true';
        if (map.lyricOffsetSeconds) this.lyricOffsetSeconds = parseFloat(map.lyricOffsetSeconds) || 0.0;
        if (map.autoScrollLyrics !== undefined) this.autoScrollLyrics = map.autoScrollLyrics === 'true';

        if (map.audioBackend) this.audioBackend = map.audioBackend as any;
        if (map.exclusiveAudio !== undefined) this.exclusiveAudio = map.exclusiveAudio === 'true';
        if (map.gaplessPlayback !== undefined) this.gaplessPlayback = map.gaplessPlayback === 'true';
        if (map.crossfadeDuration) this.crossfadeDuration = parseFloat(map.crossfadeDuration) || 0;
        if (map.replayGainMode) this.replayGainMode = map.replayGainMode as any;
        if (map.replayGainPreamp) this.replayGainPreamp = parseFloat(map.replayGainPreamp) || 0;
        if (map.audioNormalization !== undefined) this.audioNormalization = map.audioNormalization === 'true';
        if (map.streamQuality) this.streamQuality = map.streamQuality as any;
        if (map.downloadQuality) this.downloadQuality = map.downloadQuality as any;
        if (map.rememberPlaybackPosition !== undefined) this.rememberPlaybackPosition = map.rememberPlaybackPosition === 'true';
        if (map.autoplaySimilar !== undefined) this.autoplaySimilar = map.autoplaySimilar === 'true';

        if (map.libraryPath) this.libraryPath = map.libraryPath;
        if (map.downloadPath) this.downloadPath = map.downloadPath;
        if (map.autoScanOnStartup !== undefined) this.autoScanOnStartup = map.autoScanOnStartup === 'true';
        if (map.watchFolderChanges !== undefined) this.watchFolderChanges = map.watchFolderChanges === 'true';
        if (map.skipShortTracks !== undefined) this.skipShortTracks = map.skipShortTracks === 'true';
        if (map.minTrackDurationSec) this.minTrackDurationSec = parseInt(map.minTrackDurationSec, 10) || 15;
        if (map.maxStreamCacheMB) this.maxStreamCacheMB = parseInt(map.maxStreamCacheMB, 10) || 1024;

        if (map.closeToTray !== undefined) this.closeToTray = map.closeToTray === 'true';
        if (map.startMinimized !== undefined) this.startMinimized = map.startMinimized === 'true';
        if (map.mprisIntegration !== undefined) this.mprisIntegration = map.mprisIntegration === 'true';
        if (map.trackChangeNotifications !== undefined) this.trackChangeNotifications = map.trackChangeNotifications === 'true';
        if (map.discordRichPresence !== undefined) this.discordRichPresence = map.discordRichPresence === 'true';
        if (map.hardwareAcceleration !== undefined) this.hardwareAcceleration = map.hardwareAcceleration === 'true';

        if (map.ytmConnected !== undefined) this.ytmConnected = map.ytmConnected === 'true';
        if (map.ytmAccountName) this.ytmAccountName = map.ytmAccountName;
        if (map.ytmAccountEmail) this.ytmAccountEmail = map.ytmAccountEmail;
        if (map.ytmAccountAvatar) this.ytmAccountAvatar = map.ytmAccountAvatar;
        if (map.ytmCookie) this.ytmCookie = map.ytmCookie;
        if (map.ytmSyncPlaylists !== undefined) this.ytmSyncPlaylists = map.ytmSyncPlaylists === 'true';
        if (map.ytmSyncFavorites !== undefined) this.ytmSyncFavorites = map.ytmSyncFavorites === 'true';
        if (map.ytmSyncHistory !== undefined) this.ytmSyncHistory = map.ytmSyncHistory === 'true';
        if (map.ytmAutoSync !== undefined) this.ytmAutoSync = map.ytmAutoSync === 'true';
        if (map.ytmRecapTracking !== undefined) this.ytmRecapTracking = map.ytmRecapTracking === 'true';

        if (map.savedAccounts) {
          try {
            this.savedAccounts = JSON.parse(map.savedAccounts);
          } catch {
            this.savedAccounts = [];
          }
        }
      }
    } catch {
      // Fallback to defaults
    } finally {
      this.initialized = true;
      this.applyVisualSettings();
    }
  }

  applyVisualSettings() {
    if (typeof document === 'undefined') return;
    document.documentElement.style.fontSize = `${(this.uiScale / 100) * 16}px`;

    if (!this.dynamicTheme || this.themePreset !== 'dynamic') {
      const hex = THEME_PRESETS[this.themePreset] || '#d0bcff';
      const tokens = generateThemeFromHex(hex, {
        dynamicBg: this.dynamicBackground,
        accentIntensity: this.accentIntensity,
        pureOled: this.dynamicBackground === 'oled',
      });
      applyThemeTokens(tokens);
    } else {
      notifyThemeListeners();
    }
  }

  scheduleSave() {
    this.applyVisualSettings();
    if (!this.initialized) return;
    if (this.saveTimer) clearTimeout(this.saveTimer);

    this.saveTimer = setTimeout(async () => {
      try {
        const payload: Record<string, string> = {
          dynamicTheme: String(this.dynamicTheme),
          themePreset: this.themePreset,
          dynamicBackground: this.dynamicBackground,
          accentIntensity: this.accentIntensity,
          blurStrength: String(this.blurStrength),
          uiScale: String(this.uiScale),
          compactMode: String(this.compactMode),
          fontStyle: this.fontStyle,

          defaultNowPlayingView: this.defaultNowPlayingView,
          vinylSpinSpeed: this.vinylSpinSpeed,
          showVisualizer: String(this.showVisualizer),
          visualizerStyle: this.visualizerStyle,
          visualizerSensitivity: String(this.visualizerSensitivity),
          showAudioQualityBadge: String(this.showAudioQualityBadge),
          lyricsFontSize: this.lyricsFontSize,
          lyricsAlignment: this.lyricsAlignment,
          blurInactiveLines: String(this.blurInactiveLines),
          wordByWordTracking: String(this.wordByWordTracking),
          lyricOffsetSeconds: String(this.lyricOffsetSeconds),
          autoScrollLyrics: String(this.autoScrollLyrics),

          audioBackend: this.audioBackend,
          exclusiveAudio: String(this.exclusiveAudio),
          gaplessPlayback: String(this.gaplessPlayback),
          crossfadeDuration: String(this.crossfadeDuration),
          replayGainMode: this.replayGainMode,
          replayGainPreamp: String(this.replayGainPreamp),
          audioNormalization: String(this.audioNormalization),
          streamQuality: this.streamQuality,
          downloadQuality: this.downloadQuality,
          rememberPlaybackPosition: String(this.rememberPlaybackPosition),
          autoplaySimilar: String(this.autoplaySimilar),

          libraryPath: this.libraryPath,
          downloadPath: this.downloadPath,
          autoScanOnStartup: String(this.autoScanOnStartup),
          watchFolderChanges: String(this.watchFolderChanges),
          skipShortTracks: String(this.skipShortTracks),
          minTrackDurationSec: String(this.minTrackDurationSec),
          maxStreamCacheMB: String(this.maxStreamCacheMB),

          closeToTray: String(this.closeToTray),
          startMinimized: String(this.startMinimized),
          mprisIntegration: String(this.mprisIntegration),
          trackChangeNotifications: String(this.trackChangeNotifications),
          discordRichPresence: String(this.discordRichPresence),
          hardwareAcceleration: String(this.hardwareAcceleration),

          ytmConnected: String(this.ytmConnected),
          ytmAccountName: this.ytmAccountName,
          ytmAccountEmail: this.ytmAccountEmail,
          ytmAccountAvatar: this.ytmAccountAvatar,
          ytmCookie: this.ytmCookie,
          ytmSyncPlaylists: String(this.ytmSyncPlaylists),
          ytmSyncFavorites: String(this.ytmSyncFavorites),
          ytmSyncHistory: String(this.ytmSyncHistory),
          ytmAutoSync: String(this.ytmAutoSync),
          ytmRecapTracking: String(this.ytmRecapTracking),
          savedAccounts: JSON.stringify(this.savedAccounts),
        };

        await invoke('set_settings_batch', { settings: payload });
      } catch (err) {
        console.warn('Failed to persist settings:', err);
      }
    }, 300);
  }

  async openYtmLogin() {
    try {
      await invoke('open_ytm_login');
    } catch (e) {
      console.error('Failed to open YTM login window:', e);
    }
  }

  async syncYtmLibrary(): Promise<number> {
    if (!this.ytmCookie || !this.ytmConnected) return 0;
    this.isSyncingYtm = true;
    try {
      const res = await invoke<YtmSyncSummary | number>('sync_ytm_library', {
        cookie: this.ytmCookie,
        visitorData: null,
        pageId: null,
        gl: 'US',
      });
      if (typeof res === 'number') {
        return res;
      }
      this.lastSyncSummary = res;
      return (
        (res.liked_songs_count || 0) +
        (res.playlists_count || 0) +
        (res.history_count || 0)
      );
    } catch (err) {
      console.error('Failed to sync YTM library:', err);
      return 0;
    } finally {
      this.isSyncingYtm = false;
    }
  }

  connectYtmAccount(cookie: string, name?: string, email?: string, avatar?: string) {
    this.ytmCookie = cookie.trim();
    this.ytmConnected = true;
    this.ytmAccountName = name || 'Google Account';
    this.ytmAccountEmail =
      email || (cookie.includes('SAPISID') ? 'authenticated@google.com' : 'cookie-auth@user');
    this.ytmAccountAvatar =
      avatar || 'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=120&auto=format&fit=crop';

    const existingIdx = this.savedAccounts.findIndex((a) => a.cookie === this.ytmCookie);
    if (existingIdx >= 0) {
      this.savedAccounts[existingIdx] = {
        name: this.ytmAccountName,
        email: this.ytmAccountEmail,
        avatar: this.ytmAccountAvatar,
        cookie: this.ytmCookie,
      };
    } else {
      this.savedAccounts.push({
        name: this.ytmAccountName,
        email: this.ytmAccountEmail,
        avatar: this.ytmAccountAvatar,
        cookie: this.ytmCookie,
      });
    }
    this.scheduleSave();
  }

  disconnectYtmAccount() {
    this.ytmConnected = false;
    this.ytmAccountName = 'Guest Listener';
    this.ytmAccountEmail = '';
    this.ytmAccountAvatar = '';
    this.ytmCookie = '';
    this.lastSyncSummary = null;
    this.scheduleSave();
  }

  switchAccount(account: SavedYtmAccount) {
    this.ytmConnected = true;
    this.ytmAccountName = account.name;
    this.ytmAccountEmail = account.email;
    this.ytmAccountAvatar = account.avatar;
    this.ytmCookie = account.cookie;
    this.scheduleSave();
  }

  removeSavedAccount(email: string) {
    this.savedAccounts = this.savedAccounts.filter((a) => a.email !== email);
    this.scheduleSave();
  }

  resetAllSettings() {
    this.dynamicTheme = true;
    this.themePreset = 'dynamic';
    this.dynamicBackground = 'ambient';
    this.accentIntensity = 'standard';
    this.blurStrength = 48;
    this.uiScale = 100;
    this.compactMode = false;
    this.fontStyle = 'outfit';
    this.defaultNowPlayingView = 'auto';
    this.showVisualizer = true;
    this.showAudioQualityBadge = true;
    this.lyricsFontSize = 'large';
    this.blurInactiveLines = true;
    this.wordByWordTracking = true;
    this.lyricOffsetSeconds = 0.0;
    this.gaplessPlayback = true;
    this.crossfadeDuration = 0;
    this.replayGainMode = 'off';
    this.scheduleSave();
  }
}

export const settingsState = new SettingsState();

