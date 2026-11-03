<script lang="ts">
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
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
    if (window.innerWidth < 840) {
      playerState.sidebarCollapsed = true;
    }

    const handleResize = () => {
      if (window.innerWidth < 780 && !playerState.sidebarCollapsed) {
        playerState.sidebarCollapsed = true;
      }
    };
    window.addEventListener('resize', handleResize);

    const handleKeyDown = (e: KeyboardEvent) => {
      const activeTag = (document.activeElement?.tagName || '').toLowerCase();
      if (
        activeTag === 'input' ||
        activeTag === 'textarea' ||
        activeTag === 'button' ||
        activeTag === 'select'
      ) {
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        playerState.sidebarCollapsed = !playerState.sidebarCollapsed;
      } else if (e.code === 'Space') {
        e.preventDefault();
        playerState.togglePlay();
      } else if (e.code === 'ArrowLeft') {
        playerState.seek(Math.max(0, playerState.playback.position - 5));
      } else if (e.code === 'ArrowRight') {
        const maxDur = playerState.playback.duration || (playerState.currentTrack?.duration ?? 0);
        playerState.seek(
          maxDur > 0
            ? Math.min(maxDur, playerState.playback.position + 5)
            : playerState.playback.position + 5
        );
      } else if (e.code === 'ArrowUp') {
        playerState.setVolume(Math.min(100, playerState.playback.volume + 5));
      } else if (e.code === 'ArrowDown') {
        playerState.setVolume(Math.max(0, playerState.playback.volume - 5));
      } else if (e.key === 'Escape') {
        if (playerState.activeContextMenu) {
          playerState.closeContextMenu();
        } else if (playerState.isAddToPlaylistModalOpen) {
          playerState.isAddToPlaylistModalOpen = false;
        } else if (playerState.isNewPlaylistModalOpen) {
          playerState.isNewPlaylistModalOpen = false;
        } else if (playerState.isQueueOpen) {
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
  {#if settingsState.showFloatingOrbs}
    <div class="ambient-mesh fixed inset-0 pointer-events-none overflow-hidden z-0"></div>
  {/if}

  <TitleBar />

  <div class="flex-1 flex overflow-hidden relative">
    <NavigationRail />

    <main class="flex-1 overflow-y-auto relative scroll-smooth">
      {#key playerState.activeTab}
        <div in:fly={{ y: 8, duration: 220, opacity: 0 }} class="min-h-full">
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
        </div>
      {/key}
    </main>
  </div>

  <BottomPlayer />
  <NowPlayingSheet />
  <EqualizerModal />
  <QueueDrawer />
  <TrackContextMenu />
  <NewPlaylistModal />
  <AddToPlaylistModal />
  <MiniPlayer />
</div>
