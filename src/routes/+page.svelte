<script lang="ts">
  import { onMount } from 'svelte';
  import { playerState } from '$lib/state.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import NavigationRail from '$lib/components/NavigationRail.svelte';
  import BottomPlayer from '$lib/components/BottomPlayer.svelte';
  import NowPlayingSheet from '$lib/components/NowPlayingSheet.svelte';
  import EqualizerModal from '$lib/components/EqualizerModal.svelte';

  import HomeView from '$lib/views/HomeView.svelte';
  import SearchView from '$lib/views/SearchView.svelte';
  import LibraryView from '$lib/views/LibraryView.svelte';
  import StatsView from '$lib/views/StatsView.svelte';
  import SettingsView from '$lib/views/SettingsView.svelte';

  onMount(() => {
    // Load local SQLite tracks on mount
    playerState.loadLibrary();

    // Global keyboard shortcuts
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeTag = (document.activeElement?.tagName || '').toLowerCase();
      if (activeTag === 'input' || activeTag === 'textarea') return;

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        playerState.sidebarCollapsed = !playerState.sidebarCollapsed;
      } else if (e.code === 'Space') {
        e.preventDefault();
        playerState.togglePlay();
      } else if (e.code === 'ArrowLeft') {
        playerState.seek(Math.max(0, playerState.playback.position - 5));
      } else if (e.code === 'ArrowRight') {
        playerState.seek(playerState.playback.position + 5);
      } else if (e.code === 'ArrowUp') {
        playerState.setVolume(Math.min(100, playerState.playback.volume + 5));
      } else if (e.code === 'ArrowDown') {
        playerState.setVolume(Math.max(0, playerState.playback.volume - 5));
      } else if (e.key === 'Escape') {
        if (playerState.isEqualizerOpen) {
          playerState.isEqualizerOpen = false;
        } else if (playerState.isNowPlayingOpen) {
          playerState.isNowPlayingOpen = false;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<div class="h-screen w-screen flex flex-col bg-[#111215] text-white overflow-hidden relative font-sans">
  
  <!-- Subtle Ambient Glow Mesh -->
  <div
    class="absolute -top-32 -right-32 w-96 h-96 rounded-full blur-[120px] opacity-20 pointer-events-none transition-colors duration-1000"
    style="background: radial-gradient(circle, {playerState.accentColor} 0%, transparent 70%);"
  ></div>

  <!-- Custom TitleBar with drag region and controls -->
  <TitleBar />

  <!-- Middle: Sidebar + Content Area -->
  <div class="flex-1 flex overflow-hidden relative">
    <NavigationRail />

    <main class="flex-1 overflow-y-auto relative scroll-smooth">
      {#if playerState.activeTab === 'home'}
        <HomeView />
      {:else if playerState.activeTab === 'search'}
        <SearchView />
      {:else if playerState.activeTab === 'library'}
        <LibraryView />
      {:else if playerState.activeTab === 'stats'}
        <StatsView />
      {:else if playerState.activeTab === 'settings'}
        <SettingsView />
      {/if}
    </main>
  </div>

  <!-- Bottom Mini-Player Bar -->
  <BottomPlayer />

  <!-- Fullscreen Now Playing Overlay Sheet -->
  <NowPlayingSheet />

  <!-- 10-Band Equalizer Modal -->
  <EqualizerModal />

</div>
