<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { TabType } from '../types';

  interface NavItem {
    id: TabType;
    label: string;
    icon: string;
  }

  const discoverItems: NavItem[] = [
    { id: 'home', label: 'Home', icon: 'home' },
    { id: 'search', label: 'Explore & Search', icon: 'explore' },
  ];

  const libraryItems: NavItem[] = [
    { id: 'library', label: 'Your Library', icon: 'library_music' },
    { id: 'stats', label: 'Listening Stats', icon: 'insights' },
  ];
</script>

<!-- ArchiveTune Seamless GPU Sidebar: Icons stay locked in a fixed 44px left track while labels glide smoothly -->
<aside
  class="h-full m3-acrylic border-r border-white/5 flex flex-col justify-between py-3 px-2.5 z-20 shrink-0 select-none overflow-hidden will-change-[width] transition-[width] duration-300 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.sidebarCollapsed
    ? 'w-[64px]'
    : 'w-[220px]'}"
>
  <div class="space-y-3 overflow-y-auto overflow-x-hidden scrollbar-none">
    <!-- Top Header: Sidebar Collapse Toggle (Locked to 44px Left Track) -->
    <div class="flex items-center pb-1">
      <button
        type="button"
        onclick={() => (playerState.sidebarCollapsed = !playerState.sidebarCollapsed)}
        title={playerState.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        aria-label={playerState.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        class="w-11 h-10 rounded-xl text-[var(--md-sys-on-surface-variant)] hover:text-white hover:bg-white/10 active:scale-95 transition-colors duration-150 cursor-pointer flex items-center justify-center shrink-0"
      >
        <span class="material-symbols-rounded text-xl">
          {playerState.sidebarCollapsed ? 'dock_to_right' : 'dock_to_left'}
        </span>
      </button>
    </div>

    <!-- Discover Group -->
    <div class="space-y-1">
      <div class="h-5 flex items-center px-3 text-[10px] font-bold uppercase tracking-wider text-[var(--md-sys-outline)] font-display overflow-hidden select-none">
        <span
          class="whitespace-nowrap transition-all duration-300 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.sidebarCollapsed
            ? 'opacity-0 -translate-x-2 pointer-events-none'
            : 'opacity-100 translate-x-0'}"
        >
          Discover
        </span>
      </div>

      {#each discoverItems as item}
        {@const isActive = playerState.activeTab === item.id}
        <button
          type="button"
          onclick={() => (playerState.activeTab = item.id)}
          title={playerState.sidebarCollapsed ? item.label : undefined}
          class="w-full h-11 flex items-center rounded-full text-xs font-semibold transition-colors duration-200 cursor-pointer overflow-hidden {isActive
            ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] shadow-sm'
            : 'text-[var(--md-sys-on-surface-variant)] hover:bg-white/5 hover:text-white'}"
        >
          <span class="w-11 h-11 flex items-center justify-center shrink-0">
            <span class="material-symbols-rounded text-xl {isActive ? 'filled' : ''}">
              {item.icon}
            </span>
          </span>
          <span
            class="whitespace-nowrap truncate pr-3.5 transition-all duration-300 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.sidebarCollapsed
              ? 'opacity-0 -translate-x-2.5 max-w-0 pr-0 pointer-events-none'
              : 'opacity-100 translate-x-0 max-w-[148px]'}"
          >
            {item.label}
          </span>
        </button>
      {/each}
    </div>

    <!-- Library Group -->
    <div class="pt-2 border-t border-white/5 space-y-1">
      <div class="h-5 flex items-center px-3 text-[10px] font-bold uppercase tracking-wider text-[var(--md-sys-outline)] font-display overflow-hidden select-none">
        <span
          class="whitespace-nowrap transition-all duration-300 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.sidebarCollapsed
            ? 'opacity-0 -translate-x-2 pointer-events-none'
            : 'opacity-100 translate-x-0'}"
        >
          Library
        </span>
      </div>

      {#each libraryItems as item}
        {@const isActive = playerState.activeTab === item.id}
        <button
          type="button"
          onclick={() => (playerState.activeTab = item.id)}
          title={playerState.sidebarCollapsed ? item.label : undefined}
          class="w-full h-11 flex items-center rounded-full text-xs font-semibold transition-colors duration-200 cursor-pointer overflow-hidden {isActive
            ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] shadow-sm'
            : 'text-[var(--md-sys-on-surface-variant)] hover:bg-white/5 hover:text-white'}"
        >
          <span class="w-11 h-11 flex items-center justify-center shrink-0">
            <span class="material-symbols-rounded text-xl {isActive ? 'filled' : ''}">
              {item.icon}
            </span>
          </span>
          <span
            class="whitespace-nowrap truncate pr-3.5 transition-all duration-300 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.sidebarCollapsed
              ? 'opacity-0 -translate-x-2.5 max-w-0 pr-0 pointer-events-none'
              : 'opacity-100 translate-x-0 max-w-[148px]'}"
          >
            {item.label}
          </span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Bottom: Settings -->
  <div class="pt-2 border-t border-white/5 space-y-1">
    <button
      type="button"
      onclick={() => (playerState.activeTab = 'settings')}
      title={playerState.sidebarCollapsed ? 'Settings' : undefined}
      class="w-full h-11 flex items-center rounded-full text-xs font-semibold transition-colors duration-200 cursor-pointer overflow-hidden {playerState.activeTab === 'settings'
        ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] shadow-sm'
        : 'text-[var(--md-sys-on-surface-variant)] hover:bg-white/5 hover:text-white'}"
    >
      <span class="w-11 h-11 flex items-center justify-center shrink-0">
        <span class="material-symbols-rounded text-xl {playerState.activeTab === 'settings' ? 'filled' : ''}">
          settings
        </span>
      </span>
      <span
        class="whitespace-nowrap truncate pr-3.5 transition-all duration-300 [transition-timing-function:cubic-bezier(0.22,1,0.36,1)] {playerState.sidebarCollapsed
          ? 'opacity-0 -translate-x-2.5 max-w-0 pr-0 pointer-events-none'
          : 'opacity-100 translate-x-0 max-w-[148px]'}"
      >
        Settings
      </span>
    </button>
  </div>
</aside>
