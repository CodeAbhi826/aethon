<script lang="ts">
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import { THEME_PRESETS } from '../theme';
  import CupertinoSwitch from '../components/CupertinoSwitch.svelte';
  import CupertinoSelect from '../components/CupertinoSelect.svelte';

  type CategoryId =
    | 'appearance'
    | 'audio'
    | 'lyrics_visuals'
    | 'library_storage'
    | 'account_system';

  let activeCategory = $state<CategoryId>('appearance');
  let isScanning = $state(false);
  let isClearingCache = $state(false);
  let cacheClearSuccess = $state<string | null>(null);
  let importSuccess = $state<string | null>(null);

  // Account & Sync state
  let rawTokenBlockInput = $state('');
  let isConnectingYtm = $state(false);
  let ytmConnectMsg = $state<string | null>(null);
  let isSyncingYtm = $state(false);
  let ytmSyncMsg = $state<string | null>(null);
  let showAdvancedLogin = $state(false);
  let showCookieHelp = $state(false);

  // Collapsible diagnostics
  let showTelemetry = $state(false);

  const categories: { id: CategoryId; label: string; icon: string; desc: string }[] = [
    { id: 'appearance', label: 'Appearance & Theme', icon: 'palette', desc: 'Monet themes, accent tones & ambient mode' },
    { id: 'audio', label: 'Audio & Playback', icon: 'graphic_eq', desc: 'Bit-perfect PipeWire, gapless & normalization' },
    { id: 'lyrics_visuals', label: 'Lyrics & Visuals', icon: 'lyrics', desc: 'Synced LRCLIB, typography & visualizer' },
    { id: 'library_storage', label: 'Library & Storage', icon: 'folder_open', desc: 'Local music path, rescan & stream cache' },
    { id: 'account_system', label: 'Account & System', icon: 'account_circle', desc: 'Cloud streaming sync, MPRIS & tray' },
  ];

  function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  async function handleRescan() {
    isScanning = true;
    await playerState.scanLibrary();
    isScanning = false;
  }

  async function handleClearCache() {
    isClearingCache = true;
    const freed = await settingsState.clearCache();
    isClearingCache = false;
    cacheClearSuccess = `Successfully cleared ${formatBytes(freed)} from stream cache.`;
    setTimeout(() => {
      cacheClearSuccess = null;
    }, 4000);
  }

  function handleExportConfig() {
    const jsonStr = settingsState.exportSettings();
    const blob = new Blob([jsonStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `aethon-settings-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function handleImportFile(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    const file = input.files[0];
    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      if (content) {
        const ok = settingsState.importSettings(content);
        if (ok) {
          importSuccess = 'Settings successfully imported and applied!';
          setTimeout(() => {
            importSuccess = null;
          }, 4000);
        }
      }
    };
    reader.readAsText(file);
  }

  function handleResetDefaults() {
    if (confirm('Are you sure you want to reset all Aethon settings to factory defaults?')) {
      settingsState.resetDefaults();
    }
  }

  function selectPreset(key: string) {
    settingsState.themePreset = key;
    if (key !== 'dynamic') {
      settingsState.dynamicTheme = false;
    } else {
      settingsState.dynamicTheme = true;
    }
    settingsState.updateActiveTheme();
    settingsState.scheduleSave();
  }

  async function handleOpenWebLogin() {
    try {
      await settingsState.openYtmLogin();
    } catch (e) {
      console.error('Failed to open login:', e);
    }
  }

  function handleAdvancedTokenConnect() {
    const raw = rawTokenBlockInput.trim();
    if (!raw) return;

    let cookieToUse = raw;
    if (raw.includes('***INNERTUBE COOKIE***')) {
      for (const line of raw.split('\n')) {
        const trimmed = line.trim();
        if (trimmed.startsWith('***INNERTUBE COOKIE*** =')) {
          cookieToUse = trimmed.substring('***INNERTUBE COOKIE*** ='.length).trim();
          break;
        }
      }
    }

    settingsState.connectYtmAccount(cookieToUse);
    ytmConnectMsg = 'Session credentials saved successfully!';
    rawTokenBlockInput = '';
    setTimeout(() => (ytmConnectMsg = null), 4000);
  }

  function handleDisconnectYtm() {
    settingsState.disconnectYtmAccount();
    ytmConnectMsg = 'Account disconnected.';
    setTimeout(() => (ytmConnectMsg = null), 3000);
  }

  async function handleRealSyncYtm() {
    isSyncingYtm = true;
    ytmSyncMsg = null;
    try {
      const count = await settingsState.syncYtmLibrary();
      await playerState.refreshLibrary();
      ytmSyncMsg = `Synced ${count} tracks and playlists to library!`;
    } catch (e) {
      ytmSyncMsg = `Sync error: ${e}`;
    } finally {
      isSyncingYtm = false;
      setTimeout(() => (ytmSyncMsg = null), 5000);
    }
  }
</script>

<div class="h-full flex flex-col md:flex-row overflow-hidden bg-[var(--md-sys-surface)]/60 text-[var(--md-sys-on-surface)] select-none">
  
  <!-- Left Navigation Rail -->
  <aside class="w-full md:w-72 lg:w-80 shrink-0 border-b md:border-b-0 md:border-r border-white/[0.06] bg-[var(--md-sys-surface-container-low)]/80 backdrop-blur-2xl flex md:flex-col flex-row items-center md:items-stretch overflow-x-auto md:overflow-y-auto p-2 md:p-4 gap-1.5 scrollbar-none z-10">
    <div class="px-3 py-3 mb-2 border-b border-white/[0.05] hidden md:block">
      <h2 class="font-display font-bold text-xl text-white tracking-tight flex items-center gap-2">
        <span class="material-symbols-rounded text-[var(--md-sys-primary)]">tune</span>
        Settings
      </h2>
      <p class="text-xs text-white/50 mt-0.5">Preferences & audio engine configuration</p>
    </div>

    {#each categories as cat}
      {@const isActive = activeCategory === cat.id}
      <button
        type="button"
        onclick={() => (activeCategory = cat.id)}
        class="flex items-center gap-2 md:gap-3.5 px-3 py-2 md:px-3.5 md:py-3 rounded-xl md:rounded-2xl text-left transition-all group relative shrink-0 md:shrink md:w-full cursor-pointer {isActive ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] font-semibold shadow-sm' : 'hover:bg-white/[0.05] text-white/70 hover:text-white'}"
      >
        <span
          class="material-symbols-rounded text-[19px] md:text-[21px] transition-colors shrink-0 {isActive ? 'text-[var(--md-sys-primary)] font-semibold' : 'text-white/60 group-hover:text-white'}"
        >
          {cat.icon}
        </span>
        <div class="flex flex-col min-w-0 flex-1">
          <span class="text-xs md:text-sm font-medium md:font-normal truncate">{cat.label}</span>
          <span class="text-[11px] truncate opacity-65 font-normal hidden md:inline">{cat.desc}</span>
        </div>
        {#if isActive}
          <div class="w-1.5 h-5 rounded-full bg-[var(--md-sys-primary)] shrink-0 hidden md:block"></div>
        {/if}
      </button>
    {/each}

    <div class="pt-4 mt-auto border-t border-white/[0.05] px-3 text-[11px] text-white/40 hidden md:flex items-center justify-between">
      <span>Aethon Audio Engine</span>
      <span class="font-mono text-emerald-400/80">PipeWire Bit-Perfect</span>
    </div>
  </aside>

  <!-- Right Pane: Active Category Content -->
  <main class="flex-1 overflow-y-auto p-6 md:p-8 space-y-6 max-w-4xl scroll-smooth pb-28">
    
    <!-- 1. APPEARANCE & THEME -->
    {#if activeCategory === 'appearance'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Appearance & Theme</h3>
          <p class="text-xs text-white/50 mt-1">Material 3 Dynamic Monet color engine, background acrylics, and ambient mode</p>
        </div>

        <!-- Dynamic Monet Card -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-[20px]">auto_awesome</span>
            <h4 class="font-semibold text-sm text-white">Monet Dynamic Palette</h4>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.dynamicTheme}
            label="Dynamic Monet Theming"
            description="Extract dynamic Material 3 tonal palette from current album art in real-time"
            icon="colorize"
            onchange={() => {
              settingsState.updateActiveTheme();
              settingsState.scheduleSave();
            }}
          />

          <CupertinoSwitch
            bind:checked={settingsState.dynamicBackground}
            label="Dynamic Background Mood Tinting"
            description="Adapt ambient canvas and surface gradients to match the current track's color mood"
            icon="wallpaper"
            onchange={() => {
              settingsState.updateActiveTheme();
              settingsState.scheduleSave();
            }}
          />

          <CupertinoSwitch
            bind:checked={settingsState.showFloatingOrbs}
            label="Ambient Background Mesh"
            description="Render smooth radial gradient color mesh in the background"
            icon="bubble_chart"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>

        <!-- Color Palette Presets -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between pb-2 border-b border-white/[0.05]">
            <div class="flex items-center gap-2">
              <span class="material-symbols-rounded text-purple-400 text-[20px]">palette</span>
              <h4 class="font-semibold text-sm text-white">Color Schemes & Accent Presets</h4>
            </div>
            <span class="text-xs font-mono text-white/50">Current: {settingsState.themePreset}</span>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-2 2xl:grid-cols-3 gap-3 pt-1">
            {#each Object.entries(THEME_PRESETS) as [key, preset]}
              {@const isSelected = settingsState.themePreset === key}
              <button
                type="button"
                onclick={() => selectPreset(key)}
                class="flex items-start gap-3 p-3.5 rounded-2xl text-left border transition-all cursor-pointer {isSelected ? 'bg-white/[0.08] border-[var(--md-sys-primary)] shadow-sm shadow-[var(--md-sys-glow)]' : 'bg-black/20 border-white/[0.05] hover:border-white/[0.12] hover:bg-white/[0.03]'}"
              >
                <div
                  class="w-7 h-7 rounded-full shrink-0 border border-white/20 mt-0.5 shadow-inner"
                  style="background-color: {preset.hex};"
                ></div>
                <div class="min-w-0 flex-1">
                  <div class="flex items-center justify-between gap-1">
                    <span class="text-xs font-semibold text-white whitespace-normal leading-tight">{preset.name}</span>
                    {#if isSelected}
                      <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-[16px] shrink-0 ml-1.5">check_circle</span>
                    {/if}
                  </div>
                  <p class="text-[11px] text-white/50 leading-tight mt-0.5 line-clamp-2">{preset.desc}</p>
                </div>
              </button>
            {/each}
          </div>
        </div>

        <!-- Accent Intensity -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSelect
            bind:value={settingsState.accentIntensity}
            options={[
              { value: 'subtle', label: 'Subtle', desc: 'Gentle, muted accent shades' },
              { value: 'balanced', label: 'Balanced', desc: 'Standard Material 3 balance' },
              { value: 'expressive', label: 'Expressive (M3 Default)', desc: 'High vibrancy & rich glowing tints' }
            ]}
            label="Color Saturation & Intensity"
            description="Control the prominence of accent tints across the application"
            icon="palette"
            onchange={() => {
              settingsState.updateActiveTheme();
              settingsState.scheduleSave();
            }}
          />
        </div>

        <!-- Ambient & Standby Mode -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-indigo-400 text-[20px]">nightlight_round</span>
            <h4 class="font-semibold text-sm text-white">Standby & Ambient Mode</h4>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.standbyEnabled}
            label="Ambient Standby Mode"
            description="Automatically transition to serene ambient screen when music plays and app is idle"
            icon="nightlight_round"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSelect
            bind:value={settingsState.idleTimeoutMinutes}
            options={[
              { value: 1, label: '1 Minute' },
              { value: 3, label: '3 Minutes' },
              { value: 5, label: '5 Minutes' },
              { value: 10, label: '10 Minutes' },
              { value: 15, label: '15 Minutes' }
            ]}
            label="Idle Timeout Duration"
            description="Inactivity period before entering ambient standby"
            icon="timer"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.fullscreenVisualizer}
            label="Live Spectrum Visualizer & Ambient Clock"
            description="Render smooth 60fps audio waveform and clock during standby"
            icon="waves"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="py-2 border-t border-white/[0.05] space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-sm font-semibold text-white">Ambient Glow Intensity</span>
              <span class="text-xs font-mono text-[var(--md-sys-primary)]">{settingsState.ambientGlowStrength}%</span>
            </div>
            <input
              type="range"
              min="10"
              max="100"
              step="5"
              bind:value={settingsState.ambientGlowStrength}
              oninput={() => settingsState.scheduleSave()}
              class="w-full h-1.5 bg-white/10 rounded-lg appearance-none cursor-pointer accent-[var(--md-sys-primary)]"
            />
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.dimDisplayOnStandby}
            label="OLED Burn-in Protection"
            description="Dim peak brightness and gently drift static UI elements while in standby"
            icon="brightness_low"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 2. AUDIO & PLAYBACK -->
    {:else if activeCategory === 'audio'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Audio & Playback</h3>
          <p class="text-xs text-white/50 mt-1">Bit-perfect PipeWire output, gapless buffering, normalization, and quality bitrates</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between py-1">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-xl bg-emerald-500/20 text-emerald-300 flex items-center justify-center">
                <span class="material-symbols-rounded text-[20px]">surround_sound</span>
              </div>
              <div>
                <p class="text-sm font-semibold text-white">Audio Output Sink</p>
                <p class="text-xs text-white/50">PipeWire Bit-Perfect Lossless on Arch Linux (0.8ms latency)</p>
              </div>
            </div>
            <span class="text-xs font-mono px-3 py-1 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-semibold">
              Active / Bit-Perfect
            </span>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.gaplessAudio}
            label="Seamless Gapless Audio Playback"
            description="Pre-decode and buffer the next queued track for true seamless gapless transitions"
            icon="queue_music"
            onchange={() => settingsState.scheduleSave()}
          />

          <!-- Crossfade Slider -->
          <div class="py-2 space-y-2 border-t border-white/[0.05]">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-xl bg-purple-500/20 text-purple-300 flex items-center justify-center">
                  <span class="material-symbols-rounded text-[20px]">graphic_eq</span>
                </div>
                <div>
                  <p class="text-sm font-semibold text-white">Crossfade Duration</p>
                  <p class="text-xs text-white/50">Blend smoothly between ending and starting tracks</p>
                </div>
              </div>
              <span class="text-xs font-mono px-2.5 py-1 rounded-full bg-white/10 text-white font-medium">
                {settingsState.crossfadeDuration === 0 ? 'Off (Gapless)' : `${settingsState.crossfadeDuration}s`}
              </span>
            </div>

            <div class="pt-1 px-1">
              <input
                type="range"
                min="0"
                max="10"
                step="0.5"
                bind:value={settingsState.crossfadeDuration}
                oninput={() => {
                  playerState.setCrossfade(settingsState.crossfadeDuration);
                  settingsState.scheduleSave();
                }}
                class="w-full h-1.5 bg-white/10 rounded-lg appearance-none cursor-pointer accent-[var(--md-sys-primary,#d0bcff)]"
              />
              <div class="flex justify-between text-[10px] text-white/40 mt-1">
                <span>0s (Gapless)</span>
                <span>5s</span>
                <span>10s</span>
              </div>
            </div>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.replayGain}
            label="ReplayGain Volume Normalization"
            description="Prevent jarring volume differences between tracks and albums"
            icon="volume_up"
            onchange={() => {
              playerState.setReplayGain(settingsState.replayGain ? settingsState.replayGainMode : 'no');
              settingsState.scheduleSave();
            }}
          />

          {#if settingsState.replayGain}
            <CupertinoSelect
              bind:value={settingsState.replayGainMode}
              options={[
                { value: 'track', label: 'Track Gain' },
                { value: 'album', label: 'Album Gain (Audiophile)' }
              ]}
              label="ReplayGain Calculation Mode"
              description="Track-level or full album dynamic range preservation"
              icon="tune"
              onchange={() => {
                playerState.setReplayGain(settingsState.replayGainMode);
                settingsState.scheduleSave();
              }}
            />
          {/if}

          <CupertinoSwitch
            bind:checked={settingsState.exclusiveDeviceMode}
            label="Bit-Perfect DAC Hardware Mode"
            description="Bypass desktop software resampling for native lossless sample rates"
            icon="album"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.showAudioQualityBadge}
            label="Show Audio Quality Badge"
            description="Display audio stream format and quality badge in the expanded Now Playing sheet"
            icon="verified"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSelect
            bind:value={settingsState.resamplingRate}
            options={[
              { value: 'auto', label: 'Native / Automatic (Bit-Perfect)' },
              { value: '44100', label: '44.1 kHz (CD Quality)' },
              { value: '48000', label: '48.0 kHz (Studio Quality)' },
              { value: '96000', label: '96.0 kHz (Hi-Res Audio)' },
              { value: '192000', label: '192.0 kHz (Ultra Hi-Res)' }
            ]}
            label="Output Resampling Rate"
            description="Target sample rate for headless MPV engine"
            icon="speed"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSelect
            bind:value={settingsState.youtubeStreamingQuality}
            options={[
              { value: '256k', label: 'Opus 256kbps (Lossless Master)' },
              { value: '160k', label: 'Opus 160kbps (Standard High)' },
              { value: '96k', label: 'Opus 96kbps (Data Saver)' }
            ]}
            label="Online Stream Audio Quality"
            description="High-fidelity audio stream bitrate extracted via lossless backend"
            icon="graphic_eq"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.preferExplicit}
            label="Prefer Explicit Audio Masters"
            description="Stream original uncensored album cuts rather than radio edits"
            icon="explicit"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.filterDuplicates}
            label="Deduplicate Search Results"
            description="Intelligently collapse redundant uploads from different online channels"
            icon="filter_list"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.sponsorBlockSkip}
            label="SponsorBlock Segment Skipping"
            description="Automatically skip non-music intros, silence, and talking segments in music videos"
            icon="skip_next"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 3. LYRICS & VISUALS -->
    {:else if activeCategory === 'lyrics_visuals'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Lyrics & Visuals</h3>
          <p class="text-xs text-white/50 mt-1">Configure synced LRCLIB lyrics, typography scaling, live spectrums, and looping canvases</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-teal-400 text-[20px]">lyrics</span>
            <h4 class="font-semibold text-sm text-white">Synced Lyrics Engine</h4>
          </div>

          <CupertinoSelect
            bind:value={settingsState.lyricsSource}
            options={[
              { value: 'lrclib', label: 'LRCLIB (Community Database - Default)' },
              { value: 'local_lrc', label: 'Local .lrc Sidecar Files' },
              { value: 'musixmatch', label: 'Musixmatch Fallback' }
            ]}
            label="Lyrics Provider Source"
            description="Primary synced lyrics database"
            icon="lyrics"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.wordByWordTracking}
            label="Word-by-Word Karaoke Tracking"
            description="Animate progressive syllable text fill instead of full-line highlighting"
            icon="auto_awesome"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.blurInactiveLines}
            label="Focus Line Blur"
            description="Keep active sung line in focus and progressively blur background lines"
            icon="blur_on"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.autoScrollLyrics}
            label="Auto-Scroll Active Line"
            description="Keep the currently active sung line vertically centered in the sheet"
            icon="unfold_more"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSelect
            bind:value={settingsState.lyricsFontSize}
            options={[
              { value: 'compact', label: 'Compact (16px)' },
              { value: 'standard', label: 'Standard (22px)' },
              { value: 'large', label: 'Large (28px)' },
              { value: 'immersive', label: 'Immersive (36px)' }
            ]}
            label="Lyrics Typography Scale"
            description="Font size inside Now Playing lyrics panel"
            icon="format_size"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-rose-400 text-[20px]">movie</span>
            <h4 class="font-semibold text-sm text-white">Visuals & Animations</h4>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.canvasLoopsEnabled}
            label="Looping Motion Art & Canvases"
            description="Stream short video loops from online sources for supported tracks"
            icon="motion_photos_on"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSelect
            bind:value={settingsState.canvasQuality}
            options={[
              { value: '1080p', label: '1080p 60fps (High Fidelity)' },
              { value: '720p', label: '720p 30fps (Balanced)' },
              { value: 'low', label: 'Low Bandwidth (Eco)' }
            ]}
            label="Canvas Quality & Resolution"
            description="Balance GPU rendering load against visual clarity"
            icon="hd"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.spectrumVisualizer}
            label="Spectrum Analyzer in Dock & Sheet"
            description="Render live graphic audio frequency bars across playback interfaces"
            icon="equalizer"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.reduceMotion}
            label="Battery Saver / Reduce Motion"
            description="Pause background canvas animations when operating on laptop battery"
            icon="battery_saver"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 4. LIBRARY & STORAGE -->
    {:else if activeCategory === 'library_storage'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Library & Storage</h3>
          <p class="text-xs text-white/50 mt-1">Local music directories, instant scanner, stream caching, and encrypted DNS</p>
        </div>

        <!-- Local Library Directory -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-rose-400 text-[20px]">folder</span>
            <h4 class="font-semibold text-sm text-white">Local Library Storage</h4>
          </div>

          <div class="flex items-center justify-between py-1">
            <div>
              <p class="text-sm font-semibold text-white">Music Collection Directory</p>
              <p class="text-xs font-mono text-white/50">{settingsState.libraryPath}</p>
            </div>
            <button
              type="button"
              onclick={handleRescan}
              disabled={isScanning}
              class="px-4 py-2 rounded-xl bg-white/[0.08] hover:bg-white/[0.14] text-xs font-semibold text-white transition-all flex items-center gap-2 cursor-pointer"
            >
              {#if isScanning}
                <span class="material-symbols-rounded animate-spin text-[16px]">sync</span>
                <span>Scanning...</span>
              {:else}
                <span class="material-symbols-rounded text-[16px]">refresh</span>
                <span>Rescan Library</span>
              {/if}
            </button>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.autoScanOnStartup}
            label="Auto-Scan on Application Startup"
            description="Silently index newly added FLAC, ALAC, MP3 and Opus files on launch"
            icon="autorenew"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>

        <!-- Stream Caching -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-emerald-400 text-[20px]">bolt</span>
            <h4 class="font-semibold text-sm text-white">Stream-While-Playing Caching</h4>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.streamCachingEnabled}
            label="Stream-While-Playing Disk Caching"
            description="MPV plays streams immediately while caching audio chunks to ~/.cache/aethon/streams/ for instant replay"
            icon="cloud_download"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSelect
            bind:value={settingsState.maxCacheSizeGb}
            options={[
              { value: 1, label: '1 GB' },
              { value: 2, label: '2 GB' },
              { value: 5, label: '5 GB (Recommended)' },
              { value: 10, label: '10 GB' },
              { value: 25, label: '25 GB' }
            ]}
            label="Maximum Cache Disk Quota"
            description="LRU cache eviction policy triggers when quota is exceeded"
            icon="storage"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex items-center justify-between py-3 px-4 rounded-2xl bg-black/25 border border-white/[0.05]">
            <div class="flex items-center gap-3">
              <span class="material-symbols-rounded text-white/60">cached</span>
              <div>
                <p class="text-xs font-semibold text-white">Current Cache Size</p>
                <p class="text-[11px] font-mono text-white/50">
                  {formatBytes(settingsState.streamCacheSizeBytes)} in ~/.cache/aethon/streams/
                </p>
              </div>
            </div>
            <button
              type="button"
              onclick={handleClearCache}
              disabled={isClearingCache}
              class="px-4 py-2 rounded-xl bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 text-xs font-semibold transition-all cursor-pointer"
            >
              {isClearingCache ? 'Clearing...' : 'Clear Stream Cache'}
            </button>
          </div>

          {#if cacheClearSuccess}
            <div class="p-3 rounded-xl bg-emerald-500/20 text-emerald-300 text-xs font-medium border border-emerald-500/30">
              {cacheClearSuccess}
            </div>
          {/if}
        </div>

        <!-- Network & DNS -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-cyan-400 text-[20px]">wifi_tethering</span>
            <h4 class="font-semibold text-sm text-white">Network & Encrypted DNS</h4>
          </div>

          <CupertinoSelect
            bind:value={settingsState.dohProvider}
            options={[
              { value: 'cloudflare', label: 'Cloudflare (1.1.1.1 - Fastest)' },
              { value: 'quad9', label: 'Quad9 (9.9.9.9 - Privacy & Malware Block)' },
              { value: 'google', label: 'Google DNS (8.8.8.8)' },
              { value: 'system', label: 'System Default Resolver' }
            ]}
            label="DNS-over-HTTPS (DoH) Provider"
            description="Prevent ISP throttling and DNS poisoning of audio CDN streams"
            icon="lock"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSelect
            bind:value={settingsState.bufferReadaheadSecs}
            options={[
              { value: 30, label: '30 Seconds (Low RAM)' },
              { value: 60, label: '60 Seconds' },
              { value: 120, label: '2 Minutes' },
              { value: 180, label: '3 Minutes (Recommended)' },
              { value: 300, label: '5 Minutes (Resilient)' }
            ]}
            label="Stream Readahead Buffer Length"
            description="Forward audio stream buffering for spotty internet connections"
            icon="cached"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.offlineModeOnly}
            label="Force Offline Mode Only"
            description="Block all internet requests and play solely from cached streams and local tracks"
            icon="wifi_off"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 5. ACCOUNT & SYSTEM -->
    {:else if activeCategory === 'account_system'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Account & System</h3>
          <p class="text-xs text-white/50 mt-1">Connect your streaming account, configure desktop integration, and manage backups</p>
        </div>

        <!-- YouTube Music Account Card -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between pb-2 border-b border-white/[0.05]">
            <div class="flex items-center gap-2">
              <span class="material-symbols-rounded text-rose-400 text-[20px]">account_circle</span>
              <h4 class="font-semibold text-sm text-white">Online Streaming Account</h4>
            </div>
            {#if settingsState.ytmConnected}
              <span class="text-xs font-semibold px-2.5 py-1 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 flex items-center gap-1.5">
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                Connected
              </span>
            {:else}
              <span class="text-xs font-medium px-2.5 py-1 rounded-full bg-white/[0.06] text-white/60">
                Not Connected
              </span>
            {/if}
          </div>

          {#if settingsState.ytmConnected}
            <!-- Connected Profile Info -->
            <div class="flex items-center justify-between p-4 rounded-2xl bg-white/[0.03] border border-white/[0.06]">
              <div class="flex items-center gap-3.5 min-w-0">
                <div class="w-12 h-12 rounded-full overflow-hidden border border-white/20 shrink-0 shadow-md">
                  <img src={settingsState.ytmAccountAvatar || 'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=120&auto=format&fit=crop'} alt={settingsState.ytmAccountName} class="w-full h-full object-cover" />
                </div>
                <div class="min-w-0">
                  <h5 class="text-sm font-bold text-white truncate">{settingsState.ytmAccountName}</h5>
                  <p class="text-xs text-white/60 font-mono truncate">{settingsState.ytmAccountEmail || 'authenticated@google.com'}</p>
                </div>
              </div>
              <div class="flex items-center gap-2 shrink-0">
                <button
                  type="button"
                  onclick={handleRealSyncYtm}
                  disabled={isSyncingYtm}
                  class="px-3.5 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-90 text-xs font-semibold transition-opacity flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
                >
                  <span class="material-symbols-rounded text-[16px] {isSyncingYtm ? 'animate-spin' : ''}">sync</span>
                  <span>{isSyncingYtm ? 'Syncing...' : 'Sync Library'}</span>
                </button>
                <button
                  type="button"
                  onclick={handleDisconnectYtm}
                  class="px-3 py-2 rounded-xl bg-rose-500/20 text-rose-300 hover:bg-rose-500/30 text-xs font-semibold transition-all cursor-pointer"
                >
                  Sign Out
                </button>
              </div>
            </div>

            <!-- YouTube Music Recap Telemetry Setting -->
            <div class="p-3.5 rounded-2xl bg-white/[0.02] border border-white/[0.06]">
              <CupertinoSwitch
                bind:checked={settingsState.ytmRecapTracking}
                label="Contribute to YouTube Music Recap"
                description="Report standard client playback & watch-time telemetry so plays in Aethon count towards your annual YouTube Music Recap stats"
                icon="insights"
                onchange={() => settingsState.scheduleSave()}
              />
            </div>
          {:else}
            <!-- Sign-in Primary CTA -->
            <div class="py-6 flex flex-col items-center justify-center text-center space-y-3">
              <div class="w-14 h-14 rounded-2xl bg-rose-500/10 text-rose-400 flex items-center justify-center border border-rose-500/20 shadow-md">
                <span class="material-symbols-rounded text-3xl">login</span>
              </div>
              <div class="max-w-md">
                <h5 class="text-base font-bold text-white">Sign In to YouTube Music</h5>
                <p class="text-xs text-white/60 mt-1">
                  Connect your Google account to automatically sync your playlists, liked songs, and track plays toward your annual YouTube Music Recap.
                </p>
              </div>
              <button
                type="button"
                onclick={handleOpenWebLogin}
                class="px-6 py-2.5 rounded-xl bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold transition-all shadow-lg shadow-rose-600/30 flex items-center gap-2 cursor-pointer active:scale-95"
              >
                <span class="material-symbols-rounded text-[18px]">open_in_new</span>
                <span>Sign in with YouTube Music</span>
              </button>
            </div>

            <!-- Advanced Manual Token Option -->
            <div class="border-t border-white/[0.05] pt-3">
              <button
                type="button"
                onclick={() => (showAdvancedLogin = !showAdvancedLogin)}
                class="text-xs text-white/50 hover:text-white/80 flex items-center gap-1.5 cursor-pointer font-medium"
              >
                <span class="material-symbols-rounded text-[16px] transition-transform duration-200 {showAdvancedLogin ? 'rotate-90' : ''}">chevron_right</span>
                <span>Advanced: Manual Session Token Paste</span>
              </button>

              {#if showAdvancedLogin}
                <div class="mt-3 p-4 rounded-2xl bg-black/30 border border-white/[0.06] space-y-3 animate-fade-in">
                  <textarea
                    bind:value={rawTokenBlockInput}
                    placeholder="Paste session token or cookie header..."
                    rows="3"
                    class="w-full bg-black/50 border border-white/10 rounded-xl p-3 text-xs text-white placeholder-white/25 outline-none focus:border-[var(--md-sys-primary)] transition-all font-mono resize-none"
                  ></textarea>
                  <button
                    type="button"
                    onclick={handleAdvancedTokenConnect}
                    disabled={!rawTokenBlockInput.trim()}
                    class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-90 text-xs font-semibold transition-all disabled:opacity-40 cursor-pointer"
                  >
                    Save Token
                  </button>
                </div>
              {/if}
            </div>
          {/if}

          {#if ytmConnectMsg}
            <div class="p-3 rounded-xl bg-emerald-500/20 text-emerald-300 text-xs font-medium border border-emerald-500/30">
              {ytmConnectMsg}
            </div>
          {/if}
          {#if ytmSyncMsg}
            <div class="p-3 rounded-xl bg-indigo-500/20 text-indigo-300 text-xs font-medium border border-indigo-500/30">
              {ytmSyncMsg}
            </div>
          {/if}
        </div>

        <!-- Saved Accounts Switcher -->
        {#if settingsState.savedAccounts.length > 0}
          <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
            <div class="flex items-center justify-between pb-2 border-b border-white/[0.05]">
              <div class="flex items-center gap-2">
                <span class="material-symbols-rounded text-cyan-400 text-[20px]">switch_account</span>
                <h4 class="font-semibold text-sm text-white">Saved Accounts</h4>
              </div>
              <span class="text-xs text-white/50">{settingsState.savedAccounts.length} saved</span>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              {#each settingsState.savedAccounts as acc, i}
                {@const isCurrent = settingsState.ytmConnected && settingsState.ytmCookie === acc.cookie}
                <button
                  type="button"
                  onclick={() => settingsState.switchAccount(i)}
                  class="flex items-center justify-between p-3.5 rounded-2xl border text-left transition-all cursor-pointer {isCurrent ? 'bg-[var(--md-sys-primary-container)]/30 border-[var(--md-sys-primary)] shadow-sm' : 'bg-black/20 border-white/[0.05] hover:bg-white/[0.04]'}"
                >
                  <div class="flex items-center gap-3 min-w-0">
                    <div class="w-10 h-10 rounded-full overflow-hidden border border-white/20 shrink-0">
                      <img src={acc.avatar} alt={acc.name} class="w-full h-full object-cover" />
                    </div>
                    <div class="min-w-0 flex-1">
                      <p class="text-xs font-bold text-white truncate">{acc.name}</p>
                      <p class="text-[11px] text-white/50 font-mono truncate">{acc.email}</p>
                    </div>
                  </div>
                  {#if isCurrent}
                    <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-[20px] shrink-0">check_circle</span>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Behavior & System Tray -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-indigo-400 text-[20px]">settings_suggest</span>
            <h4 class="font-semibold text-sm text-white">Behavior & Desktop Integration</h4>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.closeToTray}
            label="Close Window to System Tray"
            description="Closing the window keeps music playing seamlessly in the background"
            icon="close_fullscreen"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.minimizeToTray}
            label="Minimize Window to System Tray"
            description="Dock into Wayland status area instead of taskbar when minimized"
            icon="minimize"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.resumePlaybackOnLaunch}
            label="Resume Previous Queue on Launch"
            description="Restore last playing track and queue position when starting Aethon"
            icon="history"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.trackChangeNotifications}
            label="Desktop Notifications on Track Change"
            description="Show native desktop banner with album art and track details when song advances while unfocused"
            icon="notifications"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.mprisIntegration}
            label="Desktop & Lockscreen MPRIS2 Controls"
            description="Expose media controls, metadata, and album artwork to lockscreen and desktop widgets via D-Bus"
            icon="phonelink"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>

        <!-- Backup & Engine Diagnostics -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-cyan-400 text-[20px]">cloud_sync</span>
            <h4 class="font-semibold text-sm text-white">Data Backup & Migration</h4>
          </div>

          <div class="flex flex-col sm:flex-row items-center justify-between gap-4 py-2">
            <div>
              <p class="text-sm font-semibold text-white">Export Settings & Playlists</p>
              <p class="text-xs text-white/50">Save all player preferences and favorites as a portable JSON file</p>
            </div>
            <button
              type="button"
              onclick={handleExportConfig}
              class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-90 text-xs font-semibold transition-opacity shrink-0 flex items-center gap-2 cursor-pointer"
            >
              <span class="material-symbols-rounded text-[18px]">download</span>
              <span>Export JSON</span>
            </button>
          </div>

          <div class="flex flex-col sm:flex-row items-center justify-between gap-4 py-2 border-t border-white/[0.05]">
            <div>
              <p class="text-sm font-semibold text-white">Restore from Backup</p>
              <p class="text-xs text-white/50">Load saved preferences from an aethon-settings.json file</p>
            </div>
            <label class="px-4 py-2 rounded-xl bg-white/[0.08] hover:bg-white/[0.14] text-xs font-semibold text-white transition-all shrink-0 flex items-center gap-2 cursor-pointer">
              <span class="material-symbols-rounded text-[18px]">upload</span>
              <span>Import JSON</span>
              <input type="file" accept=".json" onchange={handleImportFile} class="hidden" />
            </label>
          </div>

          {#if importSuccess}
            <div class="p-3 rounded-xl bg-emerald-500/20 text-emerald-300 text-xs font-medium border border-emerald-500/30">
              {importSuccess}
            </div>
          {/if}
        </div>

        <!-- Audio Engine Diagnostics -->
        <div class="m3-acrylic-card rounded-3xl border border-white/[0.07] overflow-hidden transition-all">
          <button
            type="button"
            onclick={() => (showTelemetry = !showTelemetry)}
            class="w-full p-5 flex items-center justify-between text-left hover:bg-white/[0.03] transition-colors cursor-pointer select-none"
          >
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-xl bg-emerald-500/20 text-emerald-300 flex items-center justify-center">
                <span class="material-symbols-rounded text-[20px]">monitor_heart</span>
              </div>
              <div>
                <h4 class="font-semibold text-sm text-white">Engine Architecture & Telemetry</h4>
                <p class="text-xs text-white/50">PipeWire sink, Unix domain socket IPC, and SQLite status</p>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-[10px] uppercase font-mono px-2 py-0.5 rounded-full bg-white/[0.06] text-white/60 border border-white/10">
                {showTelemetry ? 'Expanded' : 'Diagnostics / Nerd Stats'}
              </span>
              <span class="material-symbols-rounded text-white/60 transition-transform duration-200 {showTelemetry ? 'rotate-180 text-emerald-400' : ''}">
                expand_more
              </span>
            </div>
          </button>

          {#if showTelemetry}
            <div class="p-5 pt-0 border-t border-white/[0.05] animate-fade-in mt-2">
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs pt-3">
                <div class="p-3 rounded-2xl bg-black/30 border border-white/[0.05]">
                  <span class="text-white/40 block text-[11px]">Audio Backend</span>
                  <span class="font-semibold text-white text-sm">Headless MPV IPC</span>
                  <span class="font-mono text-[11px] text-emerald-400 block mt-1">Unix Domain Socket Active</span>
                </div>
                <div class="p-3 rounded-2xl bg-black/30 border border-white/[0.05]">
                  <span class="text-white/40 block text-[11px]">Desktop Output Sink</span>
                  <span class="font-semibold text-white text-sm">PipeWire Audio Server</span>
                  <span class="font-mono text-[11px] text-emerald-400 block mt-1">Bit-Perfect / Lossless</span>
                </div>
                <div class="p-3 rounded-2xl bg-black/30 border border-white/[0.05]">
                  <span class="text-white/40 block text-[11px]">UI Composition & Acceleration</span>
                  <span class="font-semibold text-white text-sm">WebKitGTK on Wayland</span>
                  <span class="font-mono text-[11px] text-cyan-400 block mt-1">EGL / DMABuf Hardware GPU</span>
                </div>
                <div class="p-3 rounded-2xl bg-black/30 border border-white/[0.05]">
                  <span class="text-white/40 block text-[11px]">Local Metadata Store</span>
                  <span class="font-semibold text-white text-sm">SQLite Embedded</span>
                  <span class="font-mono text-[11px] text-purple-300 block mt-1">~/.local/share/aethon/aethon.db</span>
                </div>
              </div>
            </div>
          {/if}
        </div>

        <!-- Factory Reset Card -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-rose-500/20 bg-rose-500/[0.03] space-y-3">
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm font-semibold text-rose-300">Factory Reset</p>
              <p class="text-xs text-white/50">Restore all preferences, theme colors, and audio profiles to original defaults</p>
            </div>
            <button
              type="button"
              onclick={handleResetDefaults}
              class="px-4 py-2 rounded-xl bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 text-xs font-semibold transition-all cursor-pointer"
            >
              Reset to Defaults
            </button>
          </div>
        </div>
      </div>
    {/if}

  </main>
</div>
