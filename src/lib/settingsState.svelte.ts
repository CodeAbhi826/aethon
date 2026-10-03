import { invoke } from '@tauri-apps/api/core';
import { generateThemeFromHex, applyThemeTokens, THEME_PRESETS } from './theme';

export type ThemeListener = () => void;
const themeListeners: Set<ThemeListener> = new Set();
export function registerThemeListener(fn: ThemeListener): () => void {
  themeListeners.add(fn);
  return () => themeListeners.delete(fn);
}

class SettingsState {
  // 1. Appearance
  dynamicTheme = $state(true);
  dynamicBackground = $state(true);
  themePreset = $state('dynamic');
  showFloatingOrbs = $state(true);
  accentIntensity = $state<'subtle' | 'balanced' | 'expressive'>('expressive');

  // 2. Standby / Ambient
  standbyEnabled = $state(true);
  idleTimeoutMinutes = $state(5);
  fullscreenVisualizer = $state(true);
  ambientGlowStrength = $state(45);
  dimDisplayOnStandby = $state(false);

  // 3. Audio & Playback
  gaplessAudio = $state(true);
  replayGain = $state(false);
  replayGainMode = $state<'track' | 'album' | 'off'>('track');
  exclusiveDeviceMode = $state(false);
  resamplingRate = $state('auto');
  equalizerPreset = $state('flat');

  // 4. Lyrics
  lyricsSource = $state<'lrclib' | 'local_lrc' | 'musixmatch'>('lrclib');
  karaokeAnimation = $state(true);
  autoScrollLyrics = $state(true);
  lyricsFontSize = $state<'compact' | 'standard' | 'large' | 'immersive'>('standard');
  blurInactiveLines = $state(true);

  // 5. Canvas & Visuals
  canvasLoopsEnabled = $state(true);
  canvasQuality = $state<'1080p' | '720p' | 'low'>('1080p');
  reduceMotion = $state(false);
  spectrumVisualizer = $state(true);

  // 6. Content & Sources
  youtubeStreamingQuality = $state<'256k' | '160k' | '96k'>('256k');
  preferExplicit = $state(true);
  filterDuplicates = $state(true);
  communityPlaylists = $state(true);
  sponsorBlockSkip = $state(false);

  // 7. Behavior & Privacy
  closeToTray = $state(true);
  minimizeToTray = $state(false);
  launchOnStartup = $state(false);
  resumePlaybackOnLaunch = $state(true);
  trackChangeNotifications = $state(true);
  mprisIntegration = $state(true);
  telemetryEnabled = $state(false);

  // 8. Network & DNS
  dohProvider = $state<'cloudflare' | 'quad9' | 'google' | 'system'>('cloudflare');
  bufferReadaheadSecs = $state(180);
  offlineModeOnly = $state(false);
  proxyEnabled = $state(false);
  proxyAddress = $state('');

  // 9. Storage & Cache
  streamCachingEnabled = $state(true);
  maxCacheSizeGb = $state(5);
  libraryPath = $state('~/Music');
  autoScanOnStartup = $state(true);
  streamCacheSizeBytes = $state<number>(0);

  // 10. Account & Sync (YouTube Music / Google Account)
  ytmConnected = $state(false);
  ytmAccountName = $state('Guest Listener');
  ytmAccountEmail = $state('');
  ytmAccountAvatar = $state('');
  ytmCookie = $state('');
  ytmSyncPlaylists = $state(true);
  ytmSyncFavorites = $state(true);
  ytmSyncHistory = $state(true);
  ytmAutoSync = $state(true);
  savedAccounts = $state<Array<{ name: string; email: string; avatar: string; cookie: string }>>([
    {
      name: 'Abhinav (Personal)',
      email: 'abhinav@gmail.com',
      avatar: 'https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=120&auto=format&fit=crop',
      cookie: 'SAPISID=example_sapisid_token_1',
    },
    {
      name: 'Aethon Hi-Fi Studio',
      email: 'audiophile@studio.net',
      avatar: 'https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=120&auto=format&fit=crop',
      cookie: 'SAPISID=example_sapisid_token_2',
    }
  ]);

  // Internal state
  isLoaded = $state(false);
  private saveTimeout: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    this.init();
  }

  async init() {
    await this.loadFromDb();
    this.updateActiveTheme();
    this.refreshCacheSize();
  }

  async loadFromDb() {
    try {
      const records = await invoke<Record<string, string>>('get_all_settings');
      if (records) {
        if ('dynamicTheme' in records) this.dynamicTheme = records['dynamicTheme'] === 'true';
        if ('dynamicBackground' in records) this.dynamicBackground = records['dynamicBackground'] === 'true';
        if ('themePreset' in records) this.themePreset = records['themePreset'];
        if ('showFloatingOrbs' in records) this.showFloatingOrbs = records['showFloatingOrbs'] === 'true';
        if ('accentIntensity' in records) this.accentIntensity = records['accentIntensity'] as any;

        if ('standbyEnabled' in records) this.standbyEnabled = records['standbyEnabled'] === 'true';
        if ('idleTimeoutMinutes' in records) this.idleTimeoutMinutes = Number(records['idleTimeoutMinutes']) || 5;
        if ('fullscreenVisualizer' in records) this.fullscreenVisualizer = records['fullscreenVisualizer'] === 'true';
        if ('ambientGlowStrength' in records) this.ambientGlowStrength = Number(records['ambientGlowStrength']) || 45;
        if ('dimDisplayOnStandby' in records) this.dimDisplayOnStandby = records['dimDisplayOnStandby'] === 'true';

        if ('gaplessAudio' in records) this.gaplessAudio = records['gaplessAudio'] === 'true';
        if ('replayGain' in records) this.replayGain = records['replayGain'] === 'true';
        if ('replayGainMode' in records) this.replayGainMode = records['replayGainMode'] as any;
        if ('exclusiveDeviceMode' in records) this.exclusiveDeviceMode = records['exclusiveDeviceMode'] === 'true';
        if ('resamplingRate' in records) this.resamplingRate = records['resamplingRate'];
        if ('equalizerPreset' in records) this.equalizerPreset = records['equalizerPreset'];

        if ('lyricsSource' in records) this.lyricsSource = records['lyricsSource'] as any;
        if ('karaokeAnimation' in records) this.karaokeAnimation = records['karaokeAnimation'] === 'true';
        if ('autoScrollLyrics' in records) this.autoScrollLyrics = records['autoScrollLyrics'] === 'true';
        if ('lyricsFontSize' in records) this.lyricsFontSize = records['lyricsFontSize'] as any;
        if ('blurInactiveLines' in records) this.blurInactiveLines = records['blurInactiveLines'] === 'true';

        if ('canvasLoopsEnabled' in records) this.canvasLoopsEnabled = records['canvasLoopsEnabled'] === 'true';
        if ('canvasQuality' in records) this.canvasQuality = records['canvasQuality'] as any;
        if ('reduceMotion' in records) this.reduceMotion = records['reduceMotion'] === 'true';
        if ('spectrumVisualizer' in records) this.spectrumVisualizer = records['spectrumVisualizer'] === 'true';

        if ('youtubeStreamingQuality' in records) this.youtubeStreamingQuality = records['youtubeStreamingQuality'] as any;
        if ('preferExplicit' in records) this.preferExplicit = records['preferExplicit'] === 'true';
        if ('filterDuplicates' in records) this.filterDuplicates = records['filterDuplicates'] === 'true';
        if ('communityPlaylists' in records) this.communityPlaylists = records['communityPlaylists'] === 'true';
        if ('sponsorBlockSkip' in records) this.sponsorBlockSkip = records['sponsorBlockSkip'] === 'true';

        if ('closeToTray' in records) this.closeToTray = records['closeToTray'] === 'true';
        if ('minimizeToTray' in records) this.minimizeToTray = records['minimizeToTray'] === 'true';
        if ('launchOnStartup' in records) this.launchOnStartup = records['launchOnStartup'] === 'true';
        if ('resumePlaybackOnLaunch' in records) this.resumePlaybackOnLaunch = records['resumePlaybackOnLaunch'] === 'true';
        if ('trackChangeNotifications' in records) this.trackChangeNotifications = records['trackChangeNotifications'] === 'true';
        if ('mprisIntegration' in records) this.mprisIntegration = records['mprisIntegration'] === 'true';
        if ('telemetryEnabled' in records) this.telemetryEnabled = records['telemetryEnabled'] === 'true';

        if ('dohProvider' in records) this.dohProvider = records['dohProvider'] as any;
        if ('bufferReadaheadSecs' in records) this.bufferReadaheadSecs = Number(records['bufferReadaheadSecs']) || 180;
        if ('offlineModeOnly' in records) this.offlineModeOnly = records['offlineModeOnly'] === 'true';
        if ('proxyEnabled' in records) this.proxyEnabled = records['proxyEnabled'] === 'true';
        if ('proxyAddress' in records) this.proxyAddress = records['proxyAddress'];

        if ('streamCachingEnabled' in records) this.streamCachingEnabled = records['streamCachingEnabled'] === 'true';
        if ('maxCacheSizeGb' in records) this.maxCacheSizeGb = Number(records['maxCacheSizeGb']) || 5;
        if ('libraryPath' in records) this.libraryPath = records['libraryPath'];
        if ('autoScanOnStartup' in records) this.autoScanOnStartup = records['autoScanOnStartup'] === 'true';

        if ('ytmConnected' in records) this.ytmConnected = records['ytmConnected'] === 'true';
        if ('ytmAccountName' in records) this.ytmAccountName = records['ytmAccountName'];
        if ('ytmAccountEmail' in records) this.ytmAccountEmail = records['ytmAccountEmail'];
        if ('ytmAccountAvatar' in records) this.ytmAccountAvatar = records['ytmAccountAvatar'];
        if ('ytmCookie' in records) this.ytmCookie = records['ytmCookie'];
        if ('ytmSyncPlaylists' in records) this.ytmSyncPlaylists = records['ytmSyncPlaylists'] === 'true';
        if ('ytmSyncFavorites' in records) this.ytmSyncFavorites = records['ytmSyncFavorites'] === 'true';
        if ('ytmSyncHistory' in records) this.ytmSyncHistory = records['ytmSyncHistory'] === 'true';
        if ('ytmAutoSync' in records) this.ytmAutoSync = records['ytmAutoSync'] === 'true';
        if ('savedAccounts' in records) {
          try {
            this.savedAccounts = JSON.parse(records['savedAccounts']);
          } catch {
            // Keep defaults
          }
        }
      }
    } catch {
      // In mock/preview environment
    } finally {
      this.isLoaded = true;
    }
  }

  /**
   * Persist current state to SQLite with 300ms debounce
   */
  scheduleSave() {
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }
    this.saveTimeout = setTimeout(async () => {
      try {
        const payload: Record<string, string> = {
          dynamicTheme: String(this.dynamicTheme),
          dynamicBackground: String(this.dynamicBackground),
          themePreset: this.themePreset,
          showFloatingOrbs: String(this.showFloatingOrbs),
          accentIntensity: this.accentIntensity,

          standbyEnabled: String(this.standbyEnabled),
          idleTimeoutMinutes: String(this.idleTimeoutMinutes),
          fullscreenVisualizer: String(this.fullscreenVisualizer),
          ambientGlowStrength: String(this.ambientGlowStrength),
          dimDisplayOnStandby: String(this.dimDisplayOnStandby),

          gaplessAudio: String(this.gaplessAudio),
          replayGain: String(this.replayGain),
          replayGainMode: this.replayGainMode,
          exclusiveDeviceMode: String(this.exclusiveDeviceMode),
          resamplingRate: this.resamplingRate,
          equalizerPreset: this.equalizerPreset,

          lyricsSource: this.lyricsSource,
          karaokeAnimation: String(this.karaokeAnimation),
          autoScrollLyrics: String(this.autoScrollLyrics),
          lyricsFontSize: this.lyricsFontSize,
          blurInactiveLines: String(this.blurInactiveLines),

          canvasLoopsEnabled: String(this.canvasLoopsEnabled),
          canvasQuality: this.canvasQuality,
          reduceMotion: String(this.reduceMotion),
          spectrumVisualizer: String(this.spectrumVisualizer),

          youtubeStreamingQuality: this.youtubeStreamingQuality,
          preferExplicit: String(this.preferExplicit),
          filterDuplicates: String(this.filterDuplicates),
          communityPlaylists: String(this.communityPlaylists),
          sponsorBlockSkip: String(this.sponsorBlockSkip),

          closeToTray: String(this.closeToTray),
          minimizeToTray: String(this.minimizeToTray),
          launchOnStartup: String(this.launchOnStartup),
          resumePlaybackOnLaunch: String(this.resumePlaybackOnLaunch),
          trackChangeNotifications: String(this.trackChangeNotifications),
          mprisIntegration: String(this.mprisIntegration),
          telemetryEnabled: String(this.telemetryEnabled),

          dohProvider: this.dohProvider,
          bufferReadaheadSecs: String(this.bufferReadaheadSecs),
          offlineModeOnly: String(this.offlineModeOnly),
          proxyEnabled: String(this.proxyEnabled),
          proxyAddress: this.proxyAddress,

          streamCachingEnabled: String(this.streamCachingEnabled),
          maxCacheSizeGb: String(this.maxCacheSizeGb),
          libraryPath: this.libraryPath,
          autoScanOnStartup: String(this.autoScanOnStartup),

          ytmConnected: String(this.ytmConnected),
          ytmAccountName: this.ytmAccountName,
          ytmAccountEmail: this.ytmAccountEmail,
          ytmAccountAvatar: this.ytmAccountAvatar,
          ytmCookie: this.ytmCookie,
          ytmSyncPlaylists: String(this.ytmSyncPlaylists),
          ytmSyncFavorites: String(this.ytmSyncFavorites),
          ytmSyncHistory: String(this.ytmSyncHistory),
          ytmAutoSync: String(this.ytmAutoSync),
          savedAccounts: JSON.stringify(this.savedAccounts),
        };

        await invoke('set_settings_batch', { settings: payload });
      } catch (err) {
        console.warn('Failed to persist settings:', err);
      }
    }, 300);
  }

  connectYtmAccount(cookie: string, name?: string, email?: string) {
    this.ytmCookie = cookie.trim();
    this.ytmConnected = true;
    this.ytmAccountName = name || 'Google Account';
    this.ytmAccountEmail = email || (cookie.includes('SAPISID') ? 'authenticated@google.com' : 'cookie-auth@user');
    this.ytmAccountAvatar = 'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=120&auto=format&fit=crop';

    if (!this.savedAccounts.some((a) => a.cookie === this.ytmCookie)) {
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
    this.scheduleSave();
  }

  switchAccount(index: number) {
    const acc = this.savedAccounts[index];
    if (acc) {
      this.ytmConnected = true;
      this.ytmAccountName = acc.name;
      this.ytmAccountEmail = acc.email;
      this.ytmAccountAvatar = acc.avatar;
      this.ytmCookie = acc.cookie;
      this.scheduleSave();
    }
  }

  updateActiveTheme() {
    if (!this.dynamicTheme || this.themePreset !== 'dynamic') {
      const presetKey = this.themePreset === 'dynamic' ? 'violet' : this.themePreset;
      const preset = THEME_PRESETS[presetKey] || THEME_PRESETS.violet;
      const tokens = generateThemeFromHex(preset.hex, {
        pureOled: this.themePreset === 'oled',
        dynamicBg: this.dynamicBackground,
        accentIntensity: this.accentIntensity,
      });
      applyThemeTokens(tokens);
    } else {
      for (const listener of themeListeners) {
        listener();
      }
    }
  }

  async refreshCacheSize() {
    try {
      const bytes = await invoke<number>('get_stream_cache_size');
      this.streamCacheSizeBytes = bytes || 0;
    } catch {
      this.streamCacheSizeBytes = 0;
    }
  }

  async clearCache(): Promise<number> {
    try {
      const freed = await invoke<number>('clear_stream_cache');
      await this.refreshCacheSize();
      return freed || 0;
    } catch {
      return 0;
    }
  }

  exportSettings(): string {
    const data = {
      version: '0.1.0',
      exportedAt: new Date().toISOString(),
      appearance: {
        dynamicTheme: this.dynamicTheme,
        dynamicBackground: this.dynamicBackground,
        themePreset: this.themePreset,
        showFloatingOrbs: this.showFloatingOrbs,
        accentIntensity: this.accentIntensity,
      },
      ambient: {
        standbyEnabled: this.standbyEnabled,
        idleTimeoutMinutes: this.idleTimeoutMinutes,
        fullscreenVisualizer: this.fullscreenVisualizer,
        ambientGlowStrength: this.ambientGlowStrength,
        dimDisplayOnStandby: this.dimDisplayOnStandby,
      },
      audio: {
        gaplessAudio: this.gaplessAudio,
        replayGain: this.replayGain,
        replayGainMode: this.replayGainMode,
        exclusiveDeviceMode: this.exclusiveDeviceMode,
        resamplingRate: this.resamplingRate,
        equalizerPreset: this.equalizerPreset,
      },
      lyrics: {
        lyricsSource: this.lyricsSource,
        karaokeAnimation: this.karaokeAnimation,
        autoScrollLyrics: this.autoScrollLyrics,
        lyricsFontSize: this.lyricsFontSize,
        blurInactiveLines: this.blurInactiveLines,
      },
      canvas: {
        canvasLoopsEnabled: this.canvasLoopsEnabled,
        canvasQuality: this.canvasQuality,
        reduceMotion: this.reduceMotion,
        spectrumVisualizer: this.spectrumVisualizer,
      },
      content: {
        youtubeStreamingQuality: this.youtubeStreamingQuality,
        preferExplicit: this.preferExplicit,
        filterDuplicates: this.filterDuplicates,
        communityPlaylists: this.communityPlaylists,
        sponsorBlockSkip: this.sponsorBlockSkip,
      },
      behavior: {
        closeToTray: this.closeToTray,
        minimizeToTray: this.minimizeToTray,
        launchOnStartup: this.launchOnStartup,
        resumePlaybackOnLaunch: this.resumePlaybackOnLaunch,
        trackChangeNotifications: this.trackChangeNotifications,
        mprisIntegration: this.mprisIntegration,
        telemetryEnabled: this.telemetryEnabled,
      },
      network: {
        dohProvider: this.dohProvider,
        bufferReadaheadSecs: this.bufferReadaheadSecs,
        offlineModeOnly: this.offlineModeOnly,
        proxyEnabled: this.proxyEnabled,
        proxyAddress: this.proxyAddress,
      },
      storage: {
        streamCachingEnabled: this.streamCachingEnabled,
        maxCacheSizeGb: this.maxCacheSizeGb,
        libraryPath: this.libraryPath,
        autoScanOnStartup: this.autoScanOnStartup,
      },
    };
    return JSON.stringify(data, null, 2);
  }

  importSettings(json: string): boolean {
    try {
      const data = JSON.parse(json);
      if (data.appearance) {
        if (data.appearance.dynamicTheme !== undefined) this.dynamicTheme = Boolean(data.appearance.dynamicTheme);
        if (data.appearance.dynamicBackground !== undefined) this.dynamicBackground = Boolean(data.appearance.dynamicBackground);
        if (data.appearance.themePreset) this.themePreset = data.appearance.themePreset;
        if (data.appearance.showFloatingOrbs !== undefined) this.showFloatingOrbs = Boolean(data.appearance.showFloatingOrbs);
        if (data.appearance.accentIntensity) this.accentIntensity = data.appearance.accentIntensity;
      }
      if (data.ambient) {
        if (data.ambient.standbyEnabled !== undefined) this.standbyEnabled = Boolean(data.ambient.standbyEnabled);
        if (data.ambient.idleTimeoutMinutes !== undefined) this.idleTimeoutMinutes = Number(data.ambient.idleTimeoutMinutes) || 5;
        if (data.ambient.fullscreenVisualizer !== undefined) this.fullscreenVisualizer = Boolean(data.ambient.fullscreenVisualizer);
        if (data.ambient.ambientGlowStrength !== undefined) this.ambientGlowStrength = Number(data.ambient.ambientGlowStrength) || 45;
        if (data.ambient.dimDisplayOnStandby !== undefined) this.dimDisplayOnStandby = Boolean(data.ambient.dimDisplayOnStandby);
      }
      if (data.audio) {
        if (data.audio.gaplessAudio !== undefined) this.gaplessAudio = Boolean(data.audio.gaplessAudio);
        if (data.audio.replayGain !== undefined) this.replayGain = Boolean(data.audio.replayGain);
        if (data.audio.replayGainMode) this.replayGainMode = data.audio.replayGainMode;
        if (data.audio.exclusiveDeviceMode !== undefined) this.exclusiveDeviceMode = Boolean(data.audio.exclusiveDeviceMode);
        if (data.audio.resamplingRate) this.resamplingRate = data.audio.resamplingRate;
        if (data.audio.equalizerPreset) this.equalizerPreset = data.audio.equalizerPreset;
      }
      if (data.lyrics) {
        if (data.lyrics.lyricsSource) this.lyricsSource = data.lyrics.lyricsSource;
        if (data.lyrics.karaokeAnimation !== undefined) this.karaokeAnimation = Boolean(data.lyrics.karaokeAnimation);
        if (data.lyrics.autoScrollLyrics !== undefined) this.autoScrollLyrics = Boolean(data.lyrics.autoScrollLyrics);
        if (data.lyrics.lyricsFontSize) this.lyricsFontSize = data.lyrics.lyricsFontSize;
        if (data.lyrics.blurInactiveLines !== undefined) this.blurInactiveLines = Boolean(data.lyrics.blurInactiveLines);
      }
      if (data.canvas) {
        if (data.canvas.canvasLoopsEnabled !== undefined) this.canvasLoopsEnabled = Boolean(data.canvas.canvasLoopsEnabled);
        if (data.canvas.canvasQuality) this.canvasQuality = data.canvas.canvasQuality;
        if (data.canvas.reduceMotion !== undefined) this.reduceMotion = Boolean(data.canvas.reduceMotion);
        if (data.canvas.spectrumVisualizer !== undefined) this.spectrumVisualizer = Boolean(data.canvas.spectrumVisualizer);
      }
      if (data.content) {
        if (data.content.youtubeStreamingQuality) this.youtubeStreamingQuality = data.content.youtubeStreamingQuality;
        if (data.content.preferExplicit !== undefined) this.preferExplicit = Boolean(data.content.preferExplicit);
        if (data.content.filterDuplicates !== undefined) this.filterDuplicates = Boolean(data.content.filterDuplicates);
        if (data.content.communityPlaylists !== undefined) this.communityPlaylists = Boolean(data.content.communityPlaylists);
        if (data.content.sponsorBlockSkip !== undefined) this.sponsorBlockSkip = Boolean(data.content.sponsorBlockSkip);
      }
      if (data.behavior) {
        if (data.behavior.closeToTray !== undefined) this.closeToTray = Boolean(data.behavior.closeToTray);
        if (data.behavior.minimizeToTray !== undefined) this.minimizeToTray = Boolean(data.behavior.minimizeToTray);
        if (data.behavior.launchOnStartup !== undefined) this.launchOnStartup = Boolean(data.behavior.launchOnStartup);
        if (data.behavior.resumePlaybackOnLaunch !== undefined) this.resumePlaybackOnLaunch = Boolean(data.behavior.resumePlaybackOnLaunch);
        if (data.behavior.trackChangeNotifications !== undefined) this.trackChangeNotifications = Boolean(data.behavior.trackChangeNotifications);
        if (data.behavior.mprisIntegration !== undefined) this.mprisIntegration = Boolean(data.behavior.mprisIntegration);
        if (data.behavior.telemetryEnabled !== undefined) this.telemetryEnabled = Boolean(data.behavior.telemetryEnabled);
      }
      if (data.network) {
        if (data.network.dohProvider) this.dohProvider = data.network.dohProvider;
        if (data.network.bufferReadaheadSecs !== undefined) this.bufferReadaheadSecs = Number(data.network.bufferReadaheadSecs) || 180;
        if (data.network.offlineModeOnly !== undefined) this.offlineModeOnly = Boolean(data.network.offlineModeOnly);
        if (data.network.proxyEnabled !== undefined) this.proxyEnabled = Boolean(data.network.proxyEnabled);
        if (data.network.proxyAddress !== undefined) this.proxyAddress = data.network.proxyAddress;
      }
      if (data.storage) {
        if (data.storage.streamCachingEnabled !== undefined) this.streamCachingEnabled = Boolean(data.storage.streamCachingEnabled);
        if (data.storage.maxCacheSizeGb !== undefined) this.maxCacheSizeGb = Number(data.storage.maxCacheSizeGb) || 5;
        if (data.storage.libraryPath) this.libraryPath = data.storage.libraryPath;
        if (data.storage.autoScanOnStartup !== undefined) this.autoScanOnStartup = Boolean(data.storage.autoScanOnStartup);
      }

      this.updateActiveTheme();
      this.scheduleSave();
      return true;
    } catch {
      return false;
    }
  }

  resetDefaults() {
    this.dynamicTheme = true;
    this.dynamicBackground = true;
    this.themePreset = 'dynamic';
    this.showFloatingOrbs = true;
    this.accentIntensity = 'expressive';

    this.standbyEnabled = true;
    this.idleTimeoutMinutes = 5;
    this.fullscreenVisualizer = true;
    this.ambientGlowStrength = 45;
    this.dimDisplayOnStandby = false;

    this.gaplessAudio = true;
    this.replayGain = false;
    this.replayGainMode = 'track';
    this.exclusiveDeviceMode = false;
    this.resamplingRate = 'auto';
    this.equalizerPreset = 'flat';

    this.lyricsSource = 'lrclib';
    this.karaokeAnimation = true;
    this.autoScrollLyrics = true;
    this.lyricsFontSize = 'standard';
    this.blurInactiveLines = true;

    this.canvasLoopsEnabled = true;
    this.canvasQuality = '1080p';
    this.reduceMotion = false;
    this.spectrumVisualizer = true;

    this.youtubeStreamingQuality = '256k';
    this.preferExplicit = true;
    this.filterDuplicates = true;
    this.communityPlaylists = true;
    this.sponsorBlockSkip = false;

    this.closeToTray = true;
    this.minimizeToTray = false;
    this.launchOnStartup = false;
    this.resumePlaybackOnLaunch = true;
    this.trackChangeNotifications = true;
    this.mprisIntegration = true;
    this.telemetryEnabled = false;

    this.dohProvider = 'cloudflare';
    this.bufferReadaheadSecs = 180;
    this.offlineModeOnly = false;
    this.proxyEnabled = false;
    this.proxyAddress = '';

    this.streamCachingEnabled = true;
    this.maxCacheSizeGb = 5;
    this.libraryPath = '~/Music';
    this.autoScanOnStartup = true;

    this.updateActiveTheme();
    this.scheduleSave();
  }
}

export const settingsState = new SettingsState();
