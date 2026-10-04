<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { TabType } from '../types';

  interface NavItem {
    id: TabType;
    label: string;
    icon: string;
    color?: string;
  }

  const browseItems: NavItem[] = [
    { id: 'home', label: 'Home', icon: 'home' },
    { id: 'search', label: 'Search & Explore', icon: 'search' },
    { id: 'library', label: 'Your Library', icon: 'library_music' },
    { id: 'stats', label: 'Listening Stats', icon: 'analytics' },
  ];
</script>

<aside
  class="h-full m3-acrylic border-r border-white/5 flex flex-col justify-between py-3 transition-all duration-300 ease-in-out z-20 shrink-0 select-none {playerState.sidebarCollapsed ? 'w-16 px-2' : 'w-56 px-3'}"
>
  <div class="space-y-4 overflow-y-auto scrollbar-none">
    
    <!-- Header label -->
    {#if !playerState.sidebarCollapsed}
      <div class="px-3 pt-1 text-[10px] font-bold uppercase tracking-wider text-[var(--md-sys-outline)] font-display">
        Menu
      </div>
    {/if}

    <!-- Main Navigation Items -->
    <div class="space-y-1">
      {#each browseItems as item}
        {@const isActive = playerState.activeTab === item.id}
        <button
          onclick={() => playerState.activeTab = item.id}
          title={playerState.sidebarCollapsed ? item.label : undefined}
          class="w-full flex items-center gap-3.5 px-3.5 py-2.5 rounded-full text-xs font-semibold transition-all duration-200 cursor-pointer {isActive ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] shadow-sm' : 'text-[var(--md-sys-on-surface-variant)] hover:bg-white/5 hover:text-white'}"
        >
          <span class="material-symbols-rounded text-xl shrink-0 {isActive ? 'filled' : ''}">
            {item.icon}
          </span>
          {#if !playerState.sidebarCollapsed}
            <span class="truncate">{item.label}</span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- Quick Actions -->
    <div class="pt-2 border-t border-white/5 space-y-1">
      {#if !playerState.sidebarCollapsed}
        <div class="px-3 pb-1 text-[10px] font-bold uppercase tracking-wider text-[var(--md-sys-outline)]/80 font-display">
          Quick Access
        </div>
      {/if}

      <button
        onclick={() => playerState.isNowPlayingOpen = true}
        title="Open Now Playing & Lyrics"
        class="w-full flex items-center gap-3.5 px-3.5 py-2 rounded-full text-xs font-semibold transition-all duration-200 text-[var(--md-sys-on-surface-variant)] hover:bg-white/5 hover:text-white cursor-pointer"
      >
        <span class="material-symbols-rounded text-xl shrink-0 text-[var(--md-sys-primary)]">
          lyrics
        </span>
        {#if !playerState.sidebarCollapsed}
          <span class="truncate">Live Lyrics</span>
        {/if}
      </button>

      <button
        onclick={() => playerState.isEqualizerOpen = true}
        title="Open Equalizer"
        class="w-full flex items-center gap-3.5 px-3.5 py-2 rounded-full text-xs font-semibold transition-all duration-200 text-[var(--md-sys-on-surface-variant)] hover:bg-white/5 hover:text-white cursor-pointer"
      >
        <span class="material-symbols-rounded text-xl shrink-0 text-cyan-400">
          tune
        </span>
        {#if !playerState.sidebarCollapsed}
          <span class="truncate">Equalizer</span>
        {/if}
      </button>
    </div>

  </div>

  <!-- Bottom: Settings & Sync -->
  <div class="pt-2 border-t border-white/5 space-y-1">
    <button
      onclick={() => playerState.activeTab = 'settings'}
      title="Settings"
      class="w-full flex items-center gap-3.5 px-3.5 py-2.5 rounded-full text-xs font-semibold transition-all duration-200 {playerState.activeTab === 'settings' ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)]' : 'text-[var(--md-sys-on-surface-variant)] hover:bg-white/5 hover:text-white'}"
    >
      <span class="material-symbols-rounded text-xl shrink-0 {playerState.activeTab === 'settings' ? 'filled' : ''}">
        settings
      </span>
      {#if !playerState.sidebarCollapsed}
        <span class="truncate">Settings</span>
      {/if}
    </button>
  </div>
</aside>
