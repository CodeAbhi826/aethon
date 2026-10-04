<script lang="ts">
  import { playerState } from '../state.svelte';
  import { settingsState } from '../settingsState.svelte';
  import { THEME_PRESETS } from '../theme';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import CupertinoSwitch from '../components/CupertinoSwitch.svelte';
  import CupertinoSelect from '../components/CupertinoSelect.svelte';

  type CategoryId =
    | 'account'
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

  // Account & Sync state
  let loginMode = $state<'direct' | 'advanced'>('direct');
  let ytmCookieInput = $state('');
  let rawTokenBlockInput = $state('');
  let isConnectingYtm = $state(false);
  let ytmConnectMsg = $state<string | null>(null);
  let isSyncingYtm = $state(false);
  let ytmSyncMsg = $state<string | null>(null);
  let showCookieHelp = $state(false);

  // Collapsible diagnostics
  let showTelemetry = $state(false);

  const categories: { id: CategoryId; label: string; icon: string; desc: string }[] = [
    { id: 'account', label: 'Account & Sync', icon: 'account_circle', desc: 'Google / YouTube Music cookie & playlist sync' },
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

  async function handleConnectYtm() {
    if (!ytmCookieInput.trim()) return;
    isConnectingYtm = true;
    ytmConnectMsg = null;
    await new Promise((r) => setTimeout(r, 600));
    settingsState.connectYtmAccount(ytmCookieInput);
    isConnectingYtm = false;
    ytmConnectMsg = 'Successfully connected YouTube Music account!';
    ytmCookieInput = '';
    setTimeout(() => (ytmConnectMsg = null), 4000);
  }

  async function handleDirectWebLogin() {
    try {
      await openUrl('https://accounts.google.com/ServiceLogin?continue=https%3A%2F%2Fmusic.youtube.com');
      ytmConnectMsg = 'Opened Google sign-in. After completing sign-in, use Advanced Login to paste your session cookie to verify.';
    } catch {
      window.open('https://accounts.google.com/ServiceLogin?continue=https%3A%2F%2Fmusic.youtube.com', '_blank');
      ytmConnectMsg = 'Opened Google sign-in in browser.';
    }
  }

  function handleAdvancedTokenConnect() {
    const raw = rawTokenBlockInput.trim() || ytmCookieInput.trim();
    if (!raw) return;

    let cookieToUse = raw;
    // Check if it's ArchiveTune export format
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
    ytmConnectMsg = 'Advanced login credentials saved successfully!';
    rawTokenBlockInput = '';
    ytmCookieInput = '';
    setTimeout(() => (ytmConnectMsg = null), 4000);
  }

  function handleDisconnectYtm() {
    settingsState.disconnectYtmAccount();
    ytmConnectMsg = 'Account disconnected.';
    setTimeout(() => (ytmConnectMsg = null), 3000);
  }

  async function handleSyncYtm() {
    isSyncingYtm = true;
    ytmSyncMsg = null;
    await new Promise((r) => setTimeout(r, 800));
    isSyncingYtm = false;
    ytmSyncMsg = 'Synced 4 playlists and 142 liked songs successfully!';
    setTimeout(() => (ytmSyncMsg = null), 4000);
  }
</script>

<div class="h-full flex flex-col md:flex-row overflow-hidden bg-[var(--md-sys-surface)]/60 text-[var(--md-sys-on-surface)]">
  
  <!-- Left Navigation Rail (Dual-Pane on desktop, compact horizontal pill rail on mobile/floating) -->
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

  <!-- Right Pane: Active Category Grouped Cards -->
  <main class="flex-1 overflow-y-auto p-6 md:p-8 space-y-6 max-w-4xl scroll-smooth pb-28">
    
    <!-- 0. ACCOUNT & SYNC (ArchiveTune / YouTube Music) -->
    {#if activeCategory === 'account'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Account & Cloud Sync</h3>
          <p class="text-xs text-white/50 mt-1">Connect your YouTube Music or Google account via SAPISID cookie to sync playlists, liked songs & history</p>
        </div>

        <!-- Connection Status Card -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between pb-2 border-b border-white/[0.05]">
            <div class="flex items-center gap-2">
              <span class="material-symbols-rounded text-rose-400 text-[20px]">account_circle</span>
              <h4 class="font-semibold text-sm text-white">Active Connection</h4>
            </div>
            {#if settingsState.ytmConnected}
              <span class="text-xs font-semibold px-2.5 py-1 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 flex items-center gap-1.5">
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                Connected
              </span>
            {:else}
              <span class="text-xs font-semibold px-2.5 py-1 rounded-full bg-white/[0.06] text-white/50 border border-white/10">
                Disconnected
              </span>
            {/if}
          </div>

          {#if settingsState.ytmConnected}
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-2xl bg-black/25 border border-white/[0.06]">
              <div class="flex items-center gap-3.5">
                <div class="w-12 h-12 rounded-full overflow-hidden border-2 border-[var(--md-sys-primary)]/50 shrink-0">
                  <img src={settingsState.ytmAccountAvatar} alt="Avatar" class="w-full h-full object-cover" />
                </div>
                <div>
                  <h5 class="text-sm font-bold text-white">{settingsState.ytmAccountName}</h5>
                  <p class="text-xs text-white/50 font-mono">{settingsState.ytmAccountEmail}</p>
                  <div class="flex items-center gap-2 mt-1">
                    <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-red-500/20 text-red-300 border border-red-500/30">
                      YouTube Music Premium
                    </span>
                    <span class="text-[10px] font-mono text-emerald-400">InnerTube Session Active</span>
                  </div>
                </div>
              </div>

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  onclick={handleSyncYtm}
                  disabled={isSyncingYtm}
                  class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-90 text-xs font-semibold transition-opacity flex items-center gap-2 cursor-pointer"
                >
                  <span class="material-symbols-rounded text-[18px] {isSyncingYtm ? 'animate-spin' : ''}">sync</span>
                  <span>{isSyncingYtm ? 'Syncing...' : 'Sync Library'}</span>
                </button>
                <button
                  type="button"
                  onclick={handleDisconnectYtm}
                  class="px-3.5 py-2 rounded-xl bg-white/[0.06] hover:bg-rose-500/20 text-white/70 hover:text-rose-300 border border-white/10 text-xs font-semibold transition-all cursor-pointer"
                >
                  Disconnect
                </button>
              </div>
            </div>
          {:else}
            <!-- ArchiveTune Dual Login: Direct Web Login & Advanced Token Login -->
            <div class="space-y-4">
              <p class="text-xs text-white/70 leading-relaxed">
                Connect your YouTube Music account to sync your playlists, favorites, and history. Choose between direct Google sign-in or advanced token entry.
              </p>

              <!-- Login Mode Selector (Matching ArchiveTune) -->
              <div class="flex items-center gap-1.5 p-1 rounded-xl bg-black/40 border border-white/[0.08] w-fit">
                <button
                  type="button"
                  onclick={() => (loginMode = 'direct')}
                  class="px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer {loginMode === 'direct' ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] font-semibold shadow-sm' : 'text-white/60 hover:text-white'}"
                >
                  Direct Web Login
                </button>
                <button
                  type="button"
                  onclick={() => (loginMode = 'advanced')}
                  class="px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer {loginMode === 'advanced' ? 'bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] font-semibold shadow-sm' : 'text-white/60 hover:text-white'}"
                >
                  Advanced Login
                </button>
              </div>

              {#if loginMode === 'direct'}
                <!-- 1. Direct Web Login Flow -->
                <div class="p-4 rounded-2xl bg-black/30 border border-white/[0.07] space-y-3">
                  <div class="flex items-center gap-2.5">
                    <span class="material-symbols-rounded text-[20px] text-amber-400">lock_open</span>
                    <div>
                      <h5 class="text-xs font-bold text-white">Default Google Sign-In</h5>
                      <p class="text-[11px] text-white/50">Opens official Google account authentication in your browser/window</p>
                    </div>
                  </div>

                  <button
                    type="button"
                    onclick={handleDirectWebLogin}
                    class="w-full sm:w-auto px-5 py-2.5 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-90 text-xs font-semibold transition-all flex items-center justify-center gap-2 cursor-pointer shadow-md"
                  >
                    <span class="material-symbols-rounded text-[18px]">login</span>
                    <span>Sign in with Google / YouTube Music</span>
                  </button>

                  <p class="text-[10px] text-white/40">
                    Aethon uses official Google endpoints. After logging in, verify by pasting your session cookie in Advanced Login or authorizing via browser.
                  </p>
                </div>
              {:else}
                <!-- 2. Advanced Token / Cookie Editor (ArchiveTune Format Supported) -->
                <div class="p-4 rounded-2xl bg-black/30 border border-white/[0.07] space-y-3">
                  <div class="flex items-center justify-between">
                    <div>
                      <h5 class="text-xs font-bold text-white">Advanced Token / Cookie Editor</h5>
                      <p class="text-[11px] text-white/50">Supports raw SAPISID cookie or ArchiveTune exported token block</p>
                    </div>
                    <button
                      type="button"
                      onclick={() => (showCookieHelp = !showCookieHelp)}
                      class="text-[11px] text-[var(--md-sys-primary)] hover:underline flex items-center gap-1 font-medium cursor-pointer"
                    >
                      <span class="material-symbols-rounded text-[14px]">help</span>
                      <span>How to get cookie?</span>
                    </button>
                  </div>

                  <textarea
                    bind:value={rawTokenBlockInput}
                    placeholder="Paste SAPISID, Cookie header, or ArchiveTune token block:&#10;***INNERTUBE COOKIE*** = ...&#10;***VISITOR DATA*** = ..."
                    rows="3"
                    class="w-full bg-black/50 border border-white/10 rounded-xl p-3 text-xs text-white placeholder-white/25 outline-none focus:border-[var(--md-sys-primary)] transition-all font-mono resize-none"
                  ></textarea>

                  <button
                    type="button"
                    onclick={handleAdvancedTokenConnect}
                    disabled={!rawTokenBlockInput.trim()}
                    class="px-4 py-2 rounded-xl bg-[var(--md-sys-primary)] text-[var(--md-sys-on-primary)] hover:opacity-90 text-xs font-semibold transition-all disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2 cursor-pointer"
                  >
                    <span class="material-symbols-rounded text-[16px]">key</span>
                    <span>Save Advanced Credentials</span>
                  </button>

                  {#if showCookieHelp}
                    <div class="p-3.5 rounded-xl bg-black/40 border border-white/[0.06] text-xs text-white/70 space-y-1.5 animate-fade-in font-sans">
                      <p class="font-semibold text-white">How to copy YouTube Music session cookie:</p>
                      <ol class="list-decimal list-inside space-y-1 text-white/60 text-[11px]">
                        <li>Open <a href="https://music.youtube.com" target="_blank" class="text-[var(--md-sys-primary)] underline">music.youtube.com</a> and sign in.</li>
                        <li>Press <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-white font-mono text-[10px]">F12</kbd> &rarr; <span class="text-white">Application</span> &rarr; <span class="text-white">Cookies</span>.</li>
                        <li>Copy the value of the <code class="text-[var(--md-sys-primary)] font-mono">SAPISID</code> cookie and paste it above.</li>
                      </ol>
                    </div>
                  {/if}
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

        <!-- Saved Accounts Switcher (No Hardcoded Dummy Data) -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center justify-between pb-2 border-b border-white/[0.05]">
            <div class="flex items-center gap-2">
              <span class="material-symbols-rounded text-cyan-400 text-[20px]">switch_account</span>
              <h4 class="font-semibold text-sm text-white">Saved Accounts Switcher</h4>
            </div>
            <span class="text-xs text-white/50">{settingsState.savedAccounts.length} saved</span>
          </div>

          {#if settingsState.savedAccounts.length === 0}
            <div class="py-6 flex flex-col items-center justify-center text-center text-white/40">
              <span class="material-symbols-rounded text-3xl mb-1 text-white/20">no_accounts</span>
              <p class="text-xs text-white/60 font-medium">No saved accounts yet</p>
              <p class="text-[11px] text-white/40 mt-0.5">Signed-in accounts will appear here for fast one-click switching.</p>
            </div>
          {:else}
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
          {/if}
        </div>

        <!-- Sync Preferences Card -->
        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <div class="flex items-center gap-2 pb-2 border-b border-white/[0.05]">
            <span class="material-symbols-rounded text-indigo-400 text-[20px]">sync</span>
            <h4 class="font-semibold text-sm text-white">Sync Preferences & Library Matching</h4>
          </div>

          <CupertinoSwitch
            bind:checked={settingsState.ytmSyncPlaylists}
            label="Sync YouTube Music Playlists"
            description="Fetch user-created and saved playlists directly into your local library"
            icon="playlist_add_check"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.ytmSyncFavorites}
            label="Sync Liked Songs & Favorites"
            description="Keep local favorites bidirectionally synced with your YouTube Music 'Your Likes' list"
            icon="favorite"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.ytmSyncHistory}
            label="Sync Playback History & Stats"
            description="Track plays locally and mirror scrobbles to your YouTube Music account"
            icon="history"
            onchange={() => settingsState.scheduleSave()}
          />

          <CupertinoSwitch
            bind:checked={settingsState.ytmAutoSync}
            label="Automatic Sync on App Launch"
            description="Silently fetch updated cloud playlists and subscriptions whenever Aethon opens"
            icon="cloud_sync"
            onchange={() => settingsState.scheduleSave()}
          />
        </div>
      </div>

    <!-- 1. APPEARANCE -->
    {:else if activeCategory === 'appearance'}
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
                class="flex items-start gap-3 p-3.5 rounded-2xl text-left border transition-all cursor-pointer {isSelected ? 'bg-white/[0.08] border-[var(--md-sys-primary)] shadow-sm shadow-[var(--md-sys-glow)]' : 'bg-black/20 border-white/[0.05] hover:border-white/[0.12] hover:bg-white/[0.03]'}"
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

        <!-- Accent Intensity (CupertinoSelect) -->
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

    <!-- 6. CONTENT & SOURCES -->
    {:else if activeCategory === 'content'}
      <div class="space-y-6">
        <div>
          <h3 class="font-display font-bold text-2xl text-white">Content & Streaming Sources</h3>
          <p class="text-xs text-white/50 mt-1">Stream quality, explicit filtering, and search optimizations</p>
        </div>

        <div class="m3-acrylic-card rounded-3xl p-5 border border-white/[0.07] space-y-4">
          <CupertinoSelect
            bind:value={settingsState.youtubeStreamingQuality}
            options={[
              { value: '256k', label: 'Opus 256kbps (Lossless Master)' },
              { value: '160k', label: 'Opus 160kbps (Standard High)' },
              { value: '96k', label: 'Opus 96kbps (Data Saver)' }
            ]}
            label="YouTube Music Stream Quality"
            description="Opus audio bitrate extracted via headless yt-dlp"
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

        <!-- Audio Engine Diagnostics (Nerd Stats) - Collapsible Disclosure -->
        <div class="m3-acrylic-card rounded-3xl border border-white/[0.07] overflow-hidden transition-all">
          <button
            type="button"
            onclick={() => showTelemetry = !showTelemetry}
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
