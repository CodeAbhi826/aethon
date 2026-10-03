<script lang="ts">
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import { THEME_PRESETS } from '../theme';
  import CupertinoSwitch from '../components/CupertinoSwitch.svelte';

  type CategoryId =
    | 'appearance'
    | 'ambient'
    | 'audio'
    | 'lyrics'
    | 'canvas'
    | 'content'
    | 'behavior'
    | 'network'
    | 'storage'
    | 'backup';

  let activeCategory = $state<CategoryId>('appearance');
  let isScanning = $state(false);
  let isClearingCache = $state(false);
  let cacheClearSuccess = $state<string | null>(null);
  let importSuccess = $state<string | null>(null);

  const categories: { id: CategoryId; label: string; icon: string; desc: string }[] = [
    { id: 'appearance', label: 'Appearance', icon: 'palette', desc: 'Monet themes, accent tones & floating orbs' },
    { id: 'ambient', label: 'Standby & Ambient', icon: 'nightlight_round', desc: 'Idle screen, visualizers & glow' },
    { id: 'audio', label: 'Audio & Playback', icon: 'graphic_eq', desc: 'Bit-perfect PipeWire, gapless & replaygain' },
    { id: 'lyrics', label: 'Lyrics', icon: 'lyrics', desc: 'Synced LRCLIB provider & typography' },
    { id: 'canvas', label: 'Canvas & Visuals', icon: 'movie', desc: 'Animated background loops & spectrum' },
    { id: 'content', label: 'Content & Sources', icon: 'music_note', desc: 'YouTube Opus bitrates & filtering' },
    { id: 'behavior', label: 'Behavior & Privacy', icon: 'settings_suggest', desc: 'Tray minimize, background play & notifications' },
    { id: 'network', label: 'Network & DNS', icon: 'wifi_tethering', desc: 'Encrypted DNS (DoH), proxy & buffers' },
    { id: 'storage', label: 'Storage & Cache', icon: 'folder_open', desc: 'Stream-while-playing cache & library path' },
    { id: 'backup', label: 'Backup & Engine Stats', icon: 'backup', desc: 'Export preferences, playlists & system stats' },
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
          setTimeout(() => (importSuccess = null), 4000);
        } else {
          alert('Failed to parse settings JSON file.');
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
</script>

<div class="h-full flex flex-col md:flex-row overflow-hidden bg-[var(--md-sys-surface)]/60 text-[var(--md-sys-on-surface)]">
  
  <!-- Left Navigation Rail (Dual-Pane Category Selector) -->
  <aside class="w-full md:w-72 lg:w-80 shrink-0 border-r border-white/[0.06] bg-[var(--md-sys-surface-container-low)]/80 backdrop-blur-2xl flex flex-col overflow-y-auto p-4 space-y-1.5 scrollbar-none">
    <div class="px-3 py-3 mb-2 border-b border-white/[0.05]">
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
        class="w-full flex items-center gap-3.5 px-3.5 py-3 rounded-2xl text-left transition-all group relative {isActive ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] font-semibold shadow-sm' : 'hover:bg-white/[0.05] text-white/70 hover:text-white'}"
      >
        <span
          class="material-symbols-rounded text-[21px] transition-colors shrink-0 {isActive ? 'text-[var(--md-sys-primary)] font-semibold' : 'text-white/60 group-hover:text-white'}"
        >
          {cat.icon}
        </span>
        <div class="flex flex-col min-w-0 flex-1">
          <span class="text-sm truncate">{cat.label}</span>
          <span class="text-[11px] truncate opacity-65 font-normal">{cat.desc}</span>
        </div>
        {#if isActive}
          <div class="w-1.5 h-5 rounded-full bg-[var(--md-sys-primary)] shrink-0"></div>
        {/if}
      </button>
    {/each}

    <div class="pt-4 mt-auto border-t border-white/[0.05] px-3 text-[11px] text-white/40 flex items-center justify-between">
      <span>Aethon v0.1.0</span>
      <span class="font-mono text-emerald-400/80">Arch Linux Wayland</span>
    </div>
  </aside>

  <!-- Right Pane: Active Category Grouped Cards -->
  <main class="flex-1 overflow-y-auto p-6 md:p-8 space-y-6 max-w-4xl scroll-smooth">
    
    <!-- 1. APPEARANCE -->
    {#if activeCategory === 'appearance'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Appearance & Theming</h3>
          <p class="text-xs text-white/50 mt-1">Material 3 Dynamic Monet color engine, background acrylics, and palette selection</p>
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
            label="Ambient Floating Mesh Orbs"
            description="Render GPU-accelerated animated floating spheres in the background"
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

          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 pt-1">
            {#each Object.entries(THEME_PRESETS) as [key, preset]}
              {@const isSelected = settingsState.themePreset === key}
              <button
                type="button"
                onclick={() => selectPreset(key)}
                class="flex items-start gap-3 p-3.5 rounded-2xl text-left border transition-all {isSelected ? 'bg-white/[0.08] border-[var(--md-sys-primary)] shadow-sm shadow-[var(--md-sys-glow)]' : 'bg-black/20 border-white/[0.05] hover:border-white/[0.12] hover:bg-white/[0.03]'}"
              >
                <div
                  class="w-7 h-7 rounded-full shrink-0 border border-white/20 mt-0.5 shadow-inner"
                  style="background-color: {preset.hex};"
                ></div>
                <div class="min-w-0 flex-1">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-semibold text-white truncate">{preset.name}</span>
                    {#if isSelected}
                      <span class="material-symbols-rounded text-[var(--md-sys-primary)] text-[16px]">check_circle</span>
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
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm font-semibold text-white">Color Saturation & Intensity</p>
              <p class="text-xs text-white/50 mt-0.5">Control the prominence of accent tints across the application</p>
            </div>
            <select
              bind:value={settingsState.accentIntensity}
              onchange={() => {
                settingsState.updateActiveTheme();
                settingsState.scheduleSave();
              }}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none focus:border-[var(--md-sys-primary)]"
            >
              <option value="subtle">Subtle</option>
              <option value="balanced">Balanced</option>
              <option value="expressive">Expressive (M3 Default)</option>
            </select>
          </div>
        </div>
      </div>

    <!-- 2. STANDBY & AMBIENT -->
    {:else if activeCategory === 'ambient'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Standby & Ambient Mode</h3>
          <p class="text-xs text-white/50 mt-1">Configure screensaver, idle dimming, and serene fullscreen music visuals</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSwitch
            bind:checked={settingsState.standbyEnabled}
            label="Ambient Standby Mode"
            description="Automatically transition to serene ambient screen when music plays and app is idle"
            icon="nightlight_round"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex items-center justify-between py-2 border-t border-white/[0.05]">
            <div>
              <p class="text-sm font-semibold text-white">Idle Timeout Duration</p>
              <p class="text-xs text-white/50">Inactivity period before entering ambient standby</p>
            </div>
            <select
              bind:value={settingsState.idleTimeoutMinutes}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value={1}>1 Minute</option>
              <option value={3}>3 Minutes</option>
              <option value={5}>5 Minutes</option>
              <option value={10}>10 Minutes</option>
              <option value={15}>15 Minutes</option>
            </select>
          </div>

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

    <!-- 3. AUDIO & PLAYBACK -->
    {:else if activeCategory === 'audio'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Audio Engine & Playback</h3>
          <p class="text-xs text-white/50 mt-1">Bit-perfect PipeWire output, gapless buffering, and audio normalization</p>
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

          <CupertinoSwitch
            bind:checked={settingsState.replayGain}
            label="ReplayGain Volume Normalization"
            description="Prevent jarring volume differences between tracks and albums"
            icon="volume_up"
            onchange={() => settingsState.scheduleSave()}
          />

          {#if settingsState.replayGain}
            <div class="flex items-center justify-between py-2 pl-4 border-l-2 border-[var(--md-sys-primary)]">
              <div>
                <span class="text-xs font-semibold text-white">ReplayGain Calculation Mode</span>
                <p class="text-[11px] text-white/50">Track-level or full album dynamic range preservation</p>
              </div>
              <select
                bind:value={settingsState.replayGainMode}
                onchange={() => settingsState.scheduleSave()}
                class="bg-black/40 border border-white/10 text-xs text-white rounded-xl px-3 py-1.5 outline-none"
              >
                <option value="track">Track Gain</option>
                <option value="album">Album Gain (Audiophile)</option>
              </select>
            </div>
          {/if}

          <CupertinoSwitch
            bind:checked={settingsState.exclusiveDeviceMode}
            label="Bit-Perfect DAC Hardware Mode"
            description="Bypass desktop software resampling for native lossless sample rates"
            icon="album"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex items-center justify-between py-2 border-t border-white/[0.05]">
            <div>
              <p class="text-sm font-semibold text-white">Output Resampling Rate</p>
              <p class="text-xs text-white/50">Target sample rate for headless MPV engine</p>
            </div>
            <select
              bind:value={settingsState.resamplingRate}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value="auto">Native / Automatic (Bit-Perfect)</option>
              <option value="44100">44.1 kHz (CD Quality)</option>
              <option value="48000">48.0 kHz (Studio Quality)</option>
              <option value="96000">96.0 kHz (Hi-Res Audio)</option>
              <option value="192000">192.0 kHz (Ultra Hi-Res)</option>
            </select>
          </div>
        </div>
      </div>

    <!-- 4. LYRICS -->
    {:else if activeCategory === 'lyrics'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Synced Lyrics & Typography</h3>
          <p class="text-xs text-white/50 mt-1">Configure real-time lyrics provider, karaoke animation, and sizing</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between py-2">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-xl bg-indigo-500/20 text-indigo-300 flex items-center justify-center">
                <span class="material-symbols-rounded text-[20px]">lyrics</span>
              </div>
              <div>
                <p class="text-sm font-semibold text-white">Lyrics Provider Source</p>
                <p class="text-xs text-white/50">Primary synced lyrics database</p>
              </div>
            </div>
            <select
              bind:value={settingsState.lyricsSource}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value="lrclib">LRCLIB (Community Database - Default)</option>
              <option value="local_lrc">Local .lrc Sidecar Files</option>
              <option value="musixmatch">Musixmatch Fallback</option>
            </select>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.karaokeAnimation}
            label="Fluid Progressive Karaoke Animation"
            description="Smooth word-by-word active lyric animation during playback"
            icon="auto_awesome"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.autoScrollLyrics}
            label="Auto-Scroll Active Line"
            description="Keep the currently active sung line vertically centered in the sheet"
            icon="unfold_more"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.blurInactiveLines}
            label="Blur Inactive Lyric Lines"
            description="Apply progressive focal blur to past and upcoming lyric lines"
            icon="blur_on"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex items-center justify-between py-2 border-t border-white/[0.05]">
            <div>
              <p class="text-sm font-semibold text-white">Lyrics Typography Scale</p>
              <p class="text-xs text-white/50">Font size inside Now Playing lyrics panel</p>
            </div>
            <select
              bind:value={settingsState.lyricsFontSize}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value="compact">Compact (16px)</option>
              <option value="standard">Standard (22px)</option>
              <option value="large">Large (28px)</option>
              <option value="immersive">Immersive (36px)</option>
            </select>
          </div>
        </div>
      </div>

    <!-- 5. CANVAS & VISUALS -->
    {:else if activeCategory === 'canvas'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Canvas & Motion Visuals</h3>
          <p class="text-xs text-white/50 mt-1">Short looping video canvases and visualizer performance settings</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSwitch
            bind:checked={settingsState.canvasLoopsEnabled}
            label="Looping Motion Art & Canvases"
            description="Stream short video loops from YouTube / Spotify for supported tracks"
            icon="motion_photos_on"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex items-center justify-between py-2 border-t border-white/[0.05]">
            <div>
              <p class="text-sm font-semibold text-white">Canvas Quality & Resolution</p>
              <p class="text-xs text-white/50">Balance GPU rendering load against visual clarity</p>
            </div>
            <select
              bind:value={settingsState.canvasQuality}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value="1080p">1080p 60fps (High Fidelity)</option>
              <option value="720p">720p 30fps (Balanced)</option>
              <option value="low">Low Bandwidth (Eco)</option>
            </select>
          </div>

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

    <!-- 6. CONTENT & SOURCES -->
    {:else if activeCategory === 'content'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Content & Streaming Sources</h3>
          <p class="text-xs text-white/50 mt-1">Stream quality, explicit filtering, and search optimizations</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between py-2">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-xl bg-red-500/20 text-red-300 flex items-center justify-center">
                <span class="material-symbols-rounded text-[20px]">play_circle</span>
              </div>
              <div>
                <p class="text-sm font-semibold text-white">YouTube Music Stream Quality</p>
                <p class="text-xs text-white/50">Opus audio bitrate extracted via headless yt-dlp</p>
              </div>
            </div>
            <select
              bind:value={settingsState.youtubeStreamingQuality}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value="256k">Opus 256kbps (Lossless Master)</option>
              <option value="160k">Opus 160kbps (Standard High)</option>
              <option value="96k">Opus 96kbps (Data Saver)</option>
            </select>
          </div>

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
            description="Intelligently collapse redundant uploads from different YouTube channels"
            icon="filter_list"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.communityPlaylists}
            label="Include Curated YouTube Playlists"
            description="Show verified community playlists in search results"
            icon="playlist_play"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.sponsorBlockSkip}
            label="SponsorBlock Audio Segment Skipping"
            description="Automatically skip non-music intros, silence, and talking segments in music videos"
            icon="skip_next"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 7. BEHAVIOR & PRIVACY -->
    {:else if activeCategory === 'behavior'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Behavior & Privacy</h3>
          <p class="text-xs text-white/50 mt-1">System tray docking, background audio, and desktop integration</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
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
            description="Show native Wayland notifications with album art when tracks change"
            icon="notifications"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.mprisIntegration}
            label="MPRIS D-Bus Media Key Controls"
            description="Support standard Linux keyboard keys (Play/Pause, Next, Prev) and KDE Plasma widget"
            icon="keyboard"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.telemetryEnabled}
            label="Anonymous Telemetry & Crash Reports"
            description="Aethon operates strictly offline by default without tracking or ads"
            icon="security"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 8. NETWORK & DNS -->
    {:else if activeCategory === 'network'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Network & Encrypted DNS</h3>
          <p class="text-xs text-white/50 mt-1">Configure encrypted DNS-over-HTTPS (DoH), stream pre-buffering, and proxies</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between py-2">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-xl bg-cyan-500/20 text-cyan-300 flex items-center justify-center">
                <span class="material-symbols-rounded text-[20px]">lock</span>
              </div>
              <div>
                <p class="text-sm font-semibold text-white">DNS-over-HTTPS (DoH) Provider</p>
                <p class="text-xs text-white/50">Prevent ISP throttling and DNS poisoning of audio CDN streams</p>
              </div>
            </div>
            <select
              bind:value={settingsState.dohProvider}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value="cloudflare">Cloudflare (1.1.1.1 - Fastest)</option>
              <option value="quad9">Quad9 (9.9.9.9 - Privacy & Malware Block)</option>
              <option value="google">Google DNS (8.8.8.8)</option>
              <option value="system">System Default Resolver</option>
            </select>
          </div>

          <div class="flex items-center justify-between py-2 border-t border-white/[0.05]">
            <div>
              <p class="text-sm font-semibold text-white">Stream Readahead Buffer Length</p>
              <p class="text-xs text-white/50">Forward audio stream buffering for spotty internet connections</p>
            </div>
            <select
              bind:value={settingsState.bufferReadaheadSecs}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value={30}>30 Seconds (Low RAM)</option>
              <option value={60}>60 Seconds</option>
              <option value={120}>2 Minutes</option>
              <option value={180}>3 Minutes (Recommended)</option>
              <option value={300}>5 Minutes (Resilient)</option>
            </select>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.offlineModeOnly}
            label="Force Offline Mode Only"
            description="Block all internet requests and play solely from cached streams and local tracks"
            icon="wifi_off"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 9. STORAGE & CACHE -->
    {:else if activeCategory === 'storage'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Storage & Stream Caching</h3>
          <p class="text-xs text-white/50 mt-1">Stream-while-playing local caching, disk limits, and music directories</p>
        </div>

        <!-- Stream-while-playing Card -->
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

          <div class="flex items-center justify-between py-2 border-t border-white/[0.05]">
            <div>
              <p class="text-sm font-semibold text-white">Maximum Cache Disk Quota</p>
              <p class="text-xs text-white/50">LRU cache eviction policy triggers when quota is exceeded</p>
            </div>
            <select
              bind:value={settingsState.maxCacheSizeGb}
              onchange={() => settingsState.scheduleSave()}
              class="bg-black/40 border border-white/10 text-xs font-semibold text-white rounded-xl px-3 py-2 outline-none"
            >
              <option value={1}>1 GB</option>
              <option value={2}>2 GB</option>
              <option value={5}>5 GB (Recommended)</option>
              <option value={10}>10 GB</option>
              <option value={25}>25 GB</option>
            </select>
          </div>

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
              class="px-4 py-2 rounded-xl bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 text-xs font-semibold transition-all"
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

        <!-- Library Directory Management -->
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
              class="px-4 py-2 rounded-xl bg-white/[0.08] hover:bg-white/[0.14] text-xs font-semibold text-white transition-all flex items-center gap-2"
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
      </div>

    <!-- 10. BACKUP & ENGINE STATS -->
    {:else if activeCategory === 'backup'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Backup, Restore & Engine Diagnostics</h3>
          <p class="text-xs text-white/50 mt-1">Export your player data, review audio engine telemetry, or reset defaults</p>
        </div>

        <!-- Backup & Migration -->
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
              class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-90 text-xs font-semibold transition-opacity shrink-0 flex items-center gap-2"
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
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-emerald-400 text-[20px]">monitor_heart</span>
            <h4 class="font-semibold text-sm text-white">Engine Architecture & Telemetry</h4>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
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
              class="px-4 py-2 rounded-xl bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 text-xs font-semibold transition-all"
            >
              Reset to Defaults
            </button>
          </div>
        </div>
      </div>
    {/if}

  </main>
</div>
