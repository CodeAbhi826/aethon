<script lang="ts">
  import { playerState } from '../state.svelte';
  import type { TabType } from '../types';

  const navItems: { id: TabType; label: string; icon: string }[] = [
    { id: 'home', label: 'Home', icon: 'home' },
    { id: 'search', label: 'Search', icon: 'search' },
    { id: 'library', label: 'Library', icon: 'library_music' },
    { id: 'stats', label: 'Stats', icon: 'leaderboard' },
    { id: 'settings', label: 'Settings', icon: 'settings' },
  ];
</script>

<aside
  class="h-full bg-[#141519]/70 backdrop-blur-2xl border-r border-white/[0.06] flex flex-col justify-between py-4 transition-all duration-300 ease-out z-20 shrink-0 {playerState.sidebarCollapsed ? 'w-16 px-2' : 'w-52 px-3'}"
>
  <!-- Main Nav Buttons -->
  <div class="flex flex-col gap-1.5">
    {#each navItems as item}
      {@const isActive = playerState.activeTab === item.id}
      <button
        onclick={() => playerState.activeTab = item.id}
        title={playerState.sidebarCollapsed ? item.label : undefined}
        class="flex items-center gap-3.5 px-3 py-2.5 rounded-2xl text-sm font-medium transition-all duration-150 {isActive ? 'bg-[#e6e0e9] text-[#1d192b] font-semibold shadow-md' : 'text-[#9ea2b0] hover:text-white hover:bg-white/[0.05]'}"
      >
        <span class="material-symbols-rounded text-[22px] {isActive ? 'filled' : ''}">
          {item.icon}
        </span>
        {#if !playerState.sidebarCollapsed}
          <span class="truncate">{item.label}</span>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Bottom actions / Library scan indicator -->
  <div class="flex flex-col gap-2 pt-4 border-t border-white/[0.06]">
    <button
      onclick={() => playerState.scanLibrary()}
      title="Rescan Local Music Library"
      class="flex items-center gap-3.5 px-3 py-2 rounded-xl text-xs font-medium text-[#9ea2b0] hover:text-white hover:bg-white/[0.05] transition-colors"
    >
      <span class="material-symbols-rounded text-[18px]">sync</span>
      {#if !playerState.sidebarCollapsed}
        <span class="truncate">Sync Library</span>
      {/if}
    </button>
  </div>
</aside>
