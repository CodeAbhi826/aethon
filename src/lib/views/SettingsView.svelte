<script lang="ts">
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import { THEME_PRESETS } from '../theme';
  import CupertinoSwitch from '../components/CupertinoSwitch.svelte';
  import CupertinoSelect from '../components/CupertinoSelect.svelte';

  type CategoryId = 'account' | 'appearance' | 'audio' | 'lyrics' | 'storage';

  let activeCategory = $state<CategoryId>('account');
  let isScanning = $state(false);
  let scanResultMsg = $state<string | null>(null);
  let isClearingCache = $state(false);
  let cacheClearSuccess = $state<string | null>(null);
  let importSuccess = $state<string | null>(null);
  let importError = $state<string | null>(null);
  let avatarFailed = $state(false);

  $effect(() => {
    const _av = settingsState.ytmAccountAvatar;
    avatarFailed = false;
  });

  $effect(() => {
    if (activeCategory === 'storage') {
      settingsState.refreshCacheSize();
    }
  });

  let loginMode = $state<'direct' | 'advanced'>('direct');
  let rawTokenBlockInput = $state('');
  let isConnectingYtm = $state(false);
  let ytmConnectMsg = $state<string | null>(null);
  let ytmErrorMsg = $state<string | null>(null);
  let isSyncingYtm = $state(false);
  let ytmSyncMsg = $state<string | null>(null);

  const categories: { id: CategoryId; label: string; icon: string; desc: string }[] = [
    { id: 'account', label: 'YouTube Music & Recap', icon: 'account_circle', desc: 'One-click login, cloud sync & Recap tracking' },
    { id: 'appearance', label: 'Appearance & Theme', icon: 'palette', desc: 'Monet dynamic colors, OLED & presets' },
    { id: 'audio', label: 'Audio & Playback', icon: 'graphic_eq', desc: 'PipeWire output, ReplayGain & equalizer' },
    { id: 'lyrics', label: 'Lyrics & Visuals', icon: 'lyrics', desc: 'Synced LRCLIB karaoke, font size & spectrum' },
    { id: 'storage', label: 'Library, System & Backup', icon: 'folder_open', desc: 'Folders, cache, tray, MPRIS & backup' },
  ];

  function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  async function handleWebviewLogin() {
    ytmErrorMsg = null;
    ytmConnectMsg = 'Opening Google Sign-In window — once you log in, Aethon will automatically connect and close the popup.';
    try {
      await settingsState.openWebLogin();
    } catch (e: any) {
      ytmErrorMsg = `Could not open login window: ${e}`;
    }
  }

  async function handleAdvancedCookieLogin() {
    const raw = rawTokenBlockInput.trim();
    if (!raw) return;
    isConnectingYtm = true;
    ytmErrorMsg = null;
    ytmConnectMsg = null;

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

    try {
      const profile = await settingsState.connectWithCookie(cookieToUse);
      ytmConnectMsg = `Signed in as ${profile.name} (${profile.email_or_handle})!`;
      rawTokenBlockInput = '';
      if (settingsState.ytmAutoSync) {
        await handleSyncYtm();
      }
    } catch (e: any) {
      ytmErrorMsg = `Authentication failed: ${e}`;
    } finally {
      isConnectingYtm = false;
    }
  }

  async function handleSyncYtm() {
    isSyncingYtm = true;
    ytmSyncMsg = null;
    ytmErrorMsg = null;
    try {
      const summary = await settingsState.syncYtmLibrary();
      await playerState.loadLibrary();
      await playerState.loadPlaylists();
      await playerState.loadPlayHistory();
      playerState.homeFeedLoaded = false;
      ytmSyncMsg = `Synced ${summary.liked_songs_count} liked songs, ${summary.playlists_count} cloud playlists, and ${summary.history_count} history items!`;
    } catch (e: any) {
      ytmErrorMsg = `Sync failed: ${e}`;
    } finally {
      isSyncingYtm = false;
    }
  }

  function selectPreset(key: string) {
    settingsState.themePreset = key;
    settingsState.dynamicTheme = key === 'dynamic';
    settingsState.updateActiveTheme();
    settingsState.scheduleSave();
  }

  async function handleRescan() {
    if (isScanning) return;
    isScanning = true;
    scanResultMsg = null;
    try {
      await playerState.scanLibrary(settingsState.libraryPath);
      scanResultMsg = `Library scan complete (${playerState.library.length} tracks indexed).`;
    } catch (e: any) {
      scanResultMsg = `Scan failed: ${e}`;
    } finally {
      isScanning = false;
    }
  }

  async function handleClearCache() {
    if (isClearingCache) return;
    isClearingCache = true;
    try {
      const freed = await settingsState.clearCache();
      cacheClearSuccess = `Cleared ${formatBytes(freed)} from stream cache.`;
      setTimeout(() => (cacheClearSuccess = null), 4000);
    } finally {
      isClearingCache = false;
    }
  }

  function handleExportConfig() {
    const jsonStr = settingsState.exportSettings();
    const blob = new Blob([jsonStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `aethon-settings-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  function handleImportFile(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    importSuccess = null;
    importError = null;
    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      if (content && settingsState.importSettings(content)) {
        importSuccess = 'Settings imported and applied!';
        setTimeout(() => (importSuccess = null), 4000);
      } else {
        importError = 'Invalid settings JSON file.';
        setTimeout(() => (importError = null), 4000);
      }
      input.value = '';
    };
    reader.readAsText(input.files[0]);
  }
</script>

<div class="min-h-full flex flex-col md:flex-row text-[var(--md-sys-on-surface)] select-none">
  <aside
    class="w-full md:w-72 lg:w-80 shrink-0 border-b md:border-b-0 md:border-r border-white/[0.06] bg-[var(--md-sys-surface-container-low)] flex md:flex-col flex-row items-center md:items-stretch overflow-x-auto md:overflow-y-auto p-2 md:p-4 gap-1.5 scrollbar-none z-10 md:sticky md:top-0 md:self-start md:max-h-screen"
    aria-label="Settings Categories"
  >
    <div class="px-3 py-3 mb-2 border-b border-white/[0.05] hidden md:block">
      <h2 class="font-display font-bold text-xl text-white tracking-tight flex items-center gap-2">
        <span class="material-symbols-rounded text-[var(--md-sys-primary)]">tune</span>
        Settings
      </h2>
      <p class="text-xs text-white/50 mt-0.5">Account sync, audio & appearance</p>
    </div>

    <div class="flex md:flex-col flex-row gap-1.5" role="tablist" aria-label="Settings Categories">
    {#each categories as cat}
      {@const isActive = activeCategory === cat.id}
      <button
        type="button"
        role="tab"
        aria-selected={isActive}
        onclick={() => (activeCategory = cat.id)}
        class="flex items-center gap-2 md:gap-3.5 px-3 py-2 md:px-3.5 md:py-3 rounded-xl md:rounded-2xl text-left transition-colors group shrink-0 md:shrink md:w-full cursor-pointer {isActive
          ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] font-semibold shadow-sm'
          : 'hover:bg-white/[0.05] text-white/70 hover:text-white'}"
      >
        <span class="material-symbols-rounded text-[20px] shrink-0 {isActive ? 'text-[var(--md-sys-on-primary-container)]' : 'text-white/60'}">{cat.icon}</span>
        <div class="flex flex-col min-w-0 flex-1">
          <span class="text-xs md:text-sm truncate">{cat.label}</span>
          <span class="text-[11px] truncate opacity-65 font-normal hidden md:inline">{cat.desc}</span>
        </div>
      </button>
    {/each}
    </div>
  </aside>

  <div class="flex-1 px-6 py-5 md:p-8 space-y-6 max-w-4xl pb-28">
    {#if activeCategory === 'account'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white tracking-tight">YouTube Music Account & Recap Sync</h3>
          <p class="text-xs text-white/50 mt-1">Sign in to sync Liked Songs, Playlists, and register listening time for your year-end YouTube Recap</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          {#if settingsState.ytmConnected}
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-2xl bg-black/25 border border-white/[0.06]">
              <div class="flex items-center gap-3.5 min-w-0 flex-1">
                {#if settingsState.ytmAccountAvatar && !avatarFailed}
                  <img
                    src={settingsState.ytmAccountAvatar}
                    alt="Avatar"
                    referrerpolicy="no-referrer"
                    onerror={() => (avatarFailed = true)}
                    class="w-12 h-12 rounded-full object-cover border-2 shrink-0"
                    style="border-color: color-mix(in srgb, var(--md-sys-primary) 50%, transparent);"
                  />
                {:else}
                  <div class="w-12 h-12 rounded-full bg-white/10 flex items-center justify-center shrink-0">
                    <span class="material-symbols-rounded text-2xl">person</span>
                  </div>
                {/if}
                <div class="min-w-0 flex-1">
                  <h5 class="text-sm font-bold text-white truncate">{settingsState.ytmAccountName}</h5>
                  <p class="text-xs text-white/50 font-mono truncate">{settingsState.ytmAccountEmail}</p>
                  <span class="text-[10px] font-mono text-emerald-400">SAPISIDHASH Session Active</span>
                </div>
              </div>

              <div class="flex items-center gap-2 shrink-0">
                <button
                  type="button"
                  onclick={handleSyncYtm}
                  disabled={isSyncingYtm || settingsState.isSyncingYtm}
                  class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold flex items-center gap-2 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <span class="material-symbols-rounded text-[18px] {isSyncingYtm || settingsState.isSyncingYtm ? 'animate-spin' : ''}">sync</span>
                  <span>{isSyncingYtm || settingsState.isSyncingYtm ? 'Syncing...' : 'Sync Now'}</span>
                </button>
                <button
                  type="button"
                  onclick={() => {
                    ytmConnectMsg = null;
                    ytmSyncMsg = null;
                    settingsState.disconnectYtmAccount();
                  }}
                  class="px-3.5 py-2 rounded-xl bg-white/[0.06] hover:bg-rose-500/20 text-white/70 hover:text-rose-300 border border-white/10 text-xs font-semibold cursor-pointer transition-colors"
                >
                  Sign Out
                </button>
              </div>
            </div>
          {:else}
            <div class="space-y-4">
              <div class="flex flex-wrap items-center gap-1.5 p-1 rounded-xl bg-black/40 border border-white/[0.08] w-fit max-w-full" role="tablist">
                <button
                  type="button"
                  role="tab"
                  aria-selected={loginMode === 'direct'}
                  onclick={() => (loginMode = 'direct')}
                  class="px-3.5 py-1.5 rounded-lg text-xs font-semibold cursor-pointer transition-colors {loginMode === 'direct' ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)]' : 'text-white/60 hover:text-white'}"
                >
                  One-Click Google Login
                </button>
                <button
                  type="button"
                  role="tab"
                  aria-selected={loginMode === 'advanced'}
                  onclick={() => (loginMode = 'advanced')}
                  class="px-3.5 py-1.5 rounded-lg text-xs font-semibold cursor-pointer transition-colors {loginMode === 'advanced' ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)]' : 'text-white/60 hover:text-white'}"
                >
                  Paste Cookie / Token
                </button>
              </div>

              {#if loginMode === 'direct'}
                <div class="p-4 rounded-2xl bg-black/30 border border-white/[0.07] space-y-3">
                  <p class="text-xs text-white/70">
                    Opens an embedded Google sign-in window. As soon as you finish signing into YouTube Music, Aethon automatically captures your session and closes the login window.
                  </p>
                  <button
                    type="button"
                    onclick={handleWebviewLogin}
                    class="px-5 py-2.5 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold flex items-center gap-2 cursor-pointer shadow-md"
                  >
                    <span class="material-symbols-rounded text-[18px]">login</span>
                    <span>Sign in with Google</span>
                  </button>
                </div>
              {:else}
                <div class="p-4 rounded-2xl bg-black/30 border border-white/[0.07] space-y-3">
                  <textarea
                    aria-label="Google Cookie Header"
                    bind:value={rawTokenBlockInput}
                    placeholder="Paste Cookie header containing SAPISID=... or __Secure-3PAPISID=..."
                    rows="3"
                    class="w-full bg-black/50 border border-white/10 rounded-xl p-3 text-xs text-white font-mono focus:outline-none focus:border-[var(--md-sys-primary)]"
                  ></textarea>
                  <button
                    type="button"
                    onclick={handleAdvancedCookieLogin}
                    disabled={!rawTokenBlockInput.trim() || isConnectingYtm}
                    class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                  >
                    {isConnectingYtm ? 'Verifying...' : 'Verify & Connect'}
                  </button>
                </div>
              {/if}
            </div>
          {/if}

          {#if ytmConnectMsg && !settingsState.ytmConnected}
            <div role="status" class="p-3 rounded-xl bg-emerald-500/20 text-emerald-300 text-xs border border-emerald-500/30 select-text">{ytmConnectMsg}</div>
          {/if}
          {#if ytmSyncMsg}
            <div role="status" class="p-3 rounded-xl bg-indigo-500/20 text-indigo-300 text-xs border border-indigo-500/30 select-text">{ytmSyncMsg}</div>
          {/if}
          {#if ytmErrorMsg}
            <div role="alert" class="p-3 rounded-xl bg-rose-500/20 text-rose-300 text-xs border border-rose-500/30 select-text">{ytmErrorMsg}</div>
          {/if}
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSwitch
            bind:checked={settingsState.ytmRecapTracking}
            label="Register Plays for YouTube History & Year-End Recap"
            description="Send authenticated /api/stats/playback and 30s /api/stats/watchtime beacons so songs count toward your YouTube Music Recap"
            icon="insights"
            onchange={() => settingsState.scheduleSave()}
          />
          <CupertinoSwitch
            bind:checked={settingsState.ytmAutoSync}
            label="Auto-Sync Library on Sign In"
            description="Automatically fetch Liked Songs, Playlists, and History when signing in"
            icon="sync"
            onchange={() => settingsState.scheduleSave()}
          />
          <CupertinoSelect
            bind:value={settingsState.regionCode}
            options={[
              { value: 'US', label: 'United States (US)' },
              { value: 'IN', label: 'India (IN)' },
              { value: 'GB', label: 'United Kingdom (GB)' },
              { value: 'CA', label: 'Canada (CA)' },
              { value: 'AU', label: 'Australia (AU)' },
              { value: 'DE', label: 'Germany (DE)' },
              { value: 'JP', label: 'Japan (JP)' },
            ]}
            label="Catalog Region / Country Code"
            description="Used for YouTube Music search and chart discovery"
            icon="public"
            onchange={() => {
              playerState.homeFeedLoaded = false;
              settingsState.scheduleSave();
            }}
          />
        </div>
      </div>

    {:else if activeCategory === 'appearance'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white tracking-tight">Appearance & Theme</h3>
          <p class="text-xs text-white/50 mt-1">Customize Monet dynamic colors, Pure Obsidian OLED mode, and motion</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSwitch
            bind:checked={settingsState.dynamicTheme}
            label="Dynamic Monet Theming"
            description="Extract Material 3 tonal palette from current album artwork"
            icon="colorize"
            onchange={() => {
              if (settingsState.dynamicTheme) {
                settingsState.themePreset = 'dynamic';
              } else if (settingsState.themePreset === 'dynamic') {
                settingsState.themePreset = 'violet';
              }
              settingsState.updateActiveTheme();
              settingsState.scheduleSave();
            }}
          />
          <CupertinoSwitch
            bind:checked={settingsState.dynamicBackground}
            label="Dynamic Background Mood Tinting"
            description="Tint surface containers with the active album's palette"
            icon="wallpaper"
            onchange={() => { settingsState.updateActiveTheme(); settingsState.scheduleSave(); }}
          />
          <CupertinoSwitch
            bind:checked={settingsState.showFloatingOrbs}
            label="Ambient Background Mesh"
            description="Render radial gradient mesh backdrop"
            icon="bubble_chart"
            onchange={() => settingsState.scheduleSave()}
          />
          <CupertinoSwitch
            bind:checked={settingsState.reduceMotion}
            label="Reduce Motion"
            description="Disable view transitions and animated visual effects"
            icon="motion_photos_off"
            onchange={() => { settingsState.updateActiveTheme(); settingsState.scheduleSave(); }}
          />
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {#each Object.entries(THEME_PRESETS) as [key, preset]}
            {@const isSelected = settingsState.themePreset === key}
            <button
              type="button"
              onclick={() => selectPreset(key)}
              class="flex items-start gap-3 p-3.5 rounded-2xl text-left border transition-colors cursor-pointer {isSelected ? 'bg-white/[0.08] border-[var(--md-sys-primary)]' : 'bg-black/20 border-white/[0.05] hover:bg-white/[0.04] hover:border-white/15'}"
            >
              <div class="w-6 h-6 rounded-full shrink-0 border border-white/20" style="background-color: {preset.hex};"></div>
              <div class="min-w-0 flex-1">
                <div class="text-xs font-semibold text-white">{preset.name}</div>
                <p class="text-[10px] text-white/50 mt-0.5">{preset.desc}</p>
              </div>
            </button>
          {/each}
        </div>
      </div>

    {:else if activeCategory === 'audio'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white tracking-tight">Audio & Playback</h3>
          <p class="text-xs text-white/50 mt-1">Configure PipeWire / MPV ReplayGain normalization, crossfade, and DSP equalizer</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSwitch
            bind:checked={settingsState.replayGain}
            label="ReplayGain Volume Normalization"
            description="Normalize loudness across tracks or albums via MPV"
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
                { value: 'album', label: 'Album Gain' },
              ]}
              label="ReplayGain Mode"
              icon="tune"
              onchange={() => {
                playerState.setReplayGain(settingsState.replayGainMode);
                settingsState.scheduleSave();
              }}
            />
          {/if}
          <CupertinoSelect
            bind:value={settingsState.crossfadeDuration}
            options={[
              { value: 0, label: 'Off (Gapless)' },
              { value: 2, label: '2 Seconds' },
              { value: 4, label: '4 Seconds' },
              { value: 6, label: '6 Seconds' },
              { value: 8, label: '8 Seconds' },
            ]}
            label="Track Crossfade"
            description="Smoothly blend transitions between consecutive songs"
            icon="graphic_eq"
            onchange={() => {
              playerState.setCrossfade(settingsState.crossfadeDuration);
              settingsState.scheduleSave();
            }}
          />
          <CupertinoSwitch
            bind:checked={settingsState.showAudioQualityBadge}
            label="Show Audio Codec & Sample Rate Badge"
            description="Display real codec and kHz readout in Now Playing"
            icon="verified"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex items-center justify-between pt-2 border-t border-white/[0.06]">
            <div>
              <p class="text-sm font-semibold text-white">10-Band Graphic Equalizer</p>
              <p class="text-xs text-white/50">Adjust frequency bands and DSP presets live</p>
            </div>
            <button
              type="button"
              onclick={() => (playerState.isEqualizerOpen = true)}
              class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold cursor-pointer"
            >
              Open Equalizer
            </button>
          </div>
        </div>
      </div>

    {:else if activeCategory === 'lyrics'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white tracking-tight">Lyrics & Visuals</h3>
          <p class="text-xs text-white/50 mt-1">Synced karaoke scrolling and Now Playing visualizers</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSwitch
            bind:checked={settingsState.autoScrollLyrics}
            label="Auto-Scroll Synced Lyrics"
            description="Automatically center the active lyric line during playback"
            icon="vertical_align_center"
            onchange={() => settingsState.scheduleSave()}
          />
          <CupertinoSwitch
            bind:checked={settingsState.spectrumVisualizer}
            label="Now Playing Audio Visualizer"
            description="Render GPU-composited waveform bars under the artwork"
            icon="equalizer"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    {:else if activeCategory === 'storage'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white tracking-tight">Library, System & Backup</h3>
          <p class="text-xs text-white/50 mt-1">Manage local music folders, stream cache, desktop integration & settings backup</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex flex-col sm:flex-row sm:items-end justify-between gap-3">
            <div class="flex-1 min-w-0">
              <label for="library-path-input" class="text-sm font-semibold text-white block">Local Music Folder Path</label>
              <input
                id="library-path-input"
                type="text"
                bind:value={settingsState.libraryPath}
                onchange={() => settingsState.scheduleSave()}
                class="mt-1.5 w-full bg-black/40 border border-white/10 rounded-xl px-3 py-2 text-xs font-mono text-white focus:outline-none focus:border-[var(--md-sys-primary)]"
              />
            </div>
            <button
              type="button"
              onclick={handleRescan}
              disabled={isScanning}
              class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] text-xs font-semibold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed shrink-0"
            >
              {isScanning ? 'Scanning...' : 'Scan Folder'}
            </button>
          </div>
          {#if scanResultMsg}
            <div role="status" class="p-3 rounded-xl bg-white/[0.06] text-white/80 text-xs">{scanResultMsg}</div>
          {/if}

          <CupertinoSwitch
            bind:checked={settingsState.streamCachingEnabled}
            label="Cache Streamed Tracks to Disk"
            description="Store played songs locally for instant zero-buffering replay and offline playback"
            icon="offline_pin"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex items-center justify-between py-3 px-4 rounded-2xl bg-black/25 border border-white/[0.05]">
            <div>
              <p class="text-xs font-semibold text-white">Stream Cache ({formatBytes(settingsState.streamCacheSizeBytes)})</p>
              <p class="text-[11px] font-mono text-white/50">~/.cache/aethon/streams/</p>
            </div>
            <button
              type="button"
              onclick={handleClearCache}
              disabled={isClearingCache}
              class="px-4 py-2 rounded-xl bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 text-xs font-semibold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {isClearingCache ? 'Clearing...' : 'Clear Cache'}
            </button>
          </div>
          {#if cacheClearSuccess}
            <div role="status" class="p-3 rounded-xl bg-emerald-500/20 text-emerald-300 text-xs">{cacheClearSuccess}</div>
          {/if}

          <CupertinoSwitch
            bind:checked={settingsState.closeToTray}
            label="Close Window to System Tray"
            description="Keep music playing in background when window is closed"
            icon="close_fullscreen"
            onchange={() => settingsState.scheduleSave()}
          />
          <CupertinoSwitch
            bind:checked={settingsState.mprisIntegration}
            label="Desktop & Lockscreen MPRIS2 Controls"
            description="Expose D-Bus media controls and cover art to Wayland / Sway / Plasma"
            icon="phonelink"
            onchange={() => settingsState.scheduleSave()}
          />

          <div class="flex flex-wrap items-center justify-between gap-2 pt-3 border-t border-white/10">
            <div class="flex items-center gap-2">
              <button
                type="button"
                onclick={handleExportConfig}
                class="px-4 py-2 rounded-xl bg-white/10 hover:bg-white/15 text-xs font-semibold text-white cursor-pointer transition-colors"
              >
                Export Settings JSON
              </button>
              <label class="px-4 py-2 rounded-xl bg-white/10 hover:bg-white/15 text-xs font-semibold text-white cursor-pointer transition-colors focus-within:ring-2 focus-within:ring-white/40">
                Import Settings JSON
                <input type="file" accept=".json" onchange={handleImportFile} class="sr-only" />
              </label>
            </div>
            <button
              type="button"
              onclick={() => {
                settingsState.resetDefaults();
                importSuccess = 'Settings reset to defaults.';
                setTimeout(() => (importSuccess = null), 4000);
              }}
              class="px-4 py-2 rounded-xl bg-white/[0.05] hover:bg-rose-500/15 text-xs font-semibold text-white/70 hover:text-rose-300 border border-white/10 cursor-pointer transition-colors"
            >
              Reset Defaults
            </button>
          </div>
          {#if importSuccess}
            <div role="status" class="p-3 rounded-xl bg-emerald-500/20 text-emerald-300 text-xs">{importSuccess}</div>
          {/if}
          {#if importError}
            <div role="alert" class="p-3 rounded-xl bg-rose-500/20 text-rose-300 text-xs">{importError}</div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
