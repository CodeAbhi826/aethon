<script lang="ts">
  import { onMount } from 'svelte';
  import { playerState } from '$lib/state.svelte';
  import { settingsState } from '$lib/settingsState.svelte';
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
  import ArtistView from '$lib/views/ArtistView.svelte';
  import AlbumView from '$lib/views/AlbumView.svelte';
  import QueueDrawer from '$lib/components/QueueDrawer.svelte';
  import TrackContextMenu from '$lib/components/TrackContextMenu.svelte';
  import NewPlaylistModal from '$lib/components/NewPlaylistModal.svelte';
  import AddToPlaylistModal from '$lib/components/AddToPlaylistModal.svelte';
  import MiniPlayer from '$lib/components/MiniPlayer.svelte';

  onMount(() => {
    // Load local SQLite tracks on mount
    playerState.loadLibrary();

    // Auto-collapse sidebar in compact / floating window modes (< 840px)
    if (window.innerWidth < 840) {
      playerState.sidebarCollapsed = true;
    }

    const handleResize = () => {
      if (window.innerWidth < 780 && !playerState.sidebarCollapsed) {
        playerState.sidebarCollapsed = true;
      }
    };
    window.addEventListener('resize', handleResize);

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
        if (playerState.isQueueOpen) {
          playerState.isQueueOpen = false;
        } else if (playerState.isEqualizerOpen) {
          playerState.isEqualizerOpen = false;
        } else if (playerState.isNowPlayingOpen) {
          playerState.isNowPlayingOpen = false;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('resize', handleResize);
    };
  });
</script>

<div class="h-screen w-screen flex flex-col bg-[var(--md-sys-surface)] text-[var(--md-sys-on-surface)] overflow-hidden relative font-sans">
  
  <!-- Ambient Animated Floating Mesh Background (From Mockup) -->
  {#if settingsState.showFloatingOrbs}
    <div class="fixed inset-0 pointer-events-none overflow-hidden z-0">
      <div class="ambient-orb orb-1 w-[600px] h-[600px] -top-32 -left-32" style="background: radial-gradient(circle, var(--md-sys-mesh-1) 0%, transparent 70%);"></div>
      <div class="ambient-orb orb-2 w-[700px] h-[700px] top-1/4 -right-48" style="background: radial-gradient(circle, var(--md-sys-mesh-2) 0%, transparent 70%);"></div>
      <div class="ambient-orb orb-3 w-[500px] h-[500px] -bottom-32 left-1/3" style="background: radial-gradient(circle, var(--md-sys-mesh-3) 0%, transparent 70%);"></div>
    </div>
  {/if}

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
      {:else if playerState.activeTab === 'artist'}
        <ArtistView />
      {:else if playerState.activeTab === 'album'}
        <AlbumView />
      {/if}
    </main>
  </div>

  <!-- Bottom Mini-Player Bar -->
  <BottomPlayer />

  <!-- Fullscreen Now Playing Overlay Sheet -->
  <NowPlayingSheet />

  <!-- 10-Band Equalizer Modal -->
  <EqualizerModal />

  <!-- Up Next Queue Drawer -->
  <QueueDrawer />

  <!-- Track Context Menu -->
  <TrackContextMenu />

  <!-- Playlist Modals -->
  <NewPlaylistModal />
  <AddToPlaylistModal />

  <!-- Desktop Mini Player -->
  <MiniPlayer />

</div>
