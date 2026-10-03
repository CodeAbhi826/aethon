<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { playerState } from '../state.svelte';

  const appWindow = typeof window !== 'undefined' ? getCurrentWindow() : null;

  async function handleMinimize() {
    await appWindow?.minimize();
  }

  async function handleMaximize() {
    await appWindow?.toggleMaximize();
  }

  async function handleClose() {
    await appWindow?.close();
  }

  function toggleSidebar() {
    playerState.sidebarCollapsed = !playerState.sidebarCollapsed;
  }
</script>

<header data-tauri-drag-region class="h-12 w-full flex items-center justify-between px-3 bg-[#111215]/80 backdrop-blur-xl border-b border-white/[0.06] select-none z-30 shrink-0">
  <!-- Left: Sidebar toggle + App Brand -->
  <div class="flex items-center gap-2">
    <button
      onclick={toggleSidebar}
      title="Toggle Sidebar (Ctrl+B)"
      class="w-8 h-8 rounded-lg flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] transition-colors"
    >
      <span class="material-symbols-rounded text-[20px]">
        {playerState.sidebarCollapsed ? 'menu' : 'dock_to_left'}
      </span>
    </button>

    <div class="flex items-center gap-2 px-2 py-1">
      <div class="w-6 h-6 rounded-md bg-gradient-to-tr from-purple-500 to-indigo-400 flex items-center justify-center shadow-sm">
        <span class="material-symbols-rounded text-white text-[16px]">graphic_eq</span>
      </div>
      <span class="font-heading font-bold text-sm tracking-wide text-white">Aethon</span>
    </div>
  </div>

  <!-- Center: Pure clean drag region (no music pill) -->
  <div data-tauri-drag-region class="flex-1 h-full mx-4 cursor-default"></div>

  <!-- Right: Window traffic controls (Linux / Wayland custom frame) -->
  <div class="flex items-center gap-1">
    <button
      onclick={handleMinimize}
      aria-label="Minimize"
      class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] transition-colors"
    >
      <span class="material-symbols-rounded text-[16px]">remove</span>
    </button>
    <button
      onclick={handleMaximize}
      aria-label="Maximize"
      class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-white/[0.08] transition-colors"
    >
      <span class="material-symbols-rounded text-[15px]">crop_square</span>
    </button>
    <button
      onclick={handleClose}
      aria-label="Close"
      class="w-7 h-7 rounded-md flex items-center justify-center text-[#9ea2b0] hover:text-white hover:bg-red-500/80 transition-colors"
    >
      <span class="material-symbols-rounded text-[16px]">close</span>
    </button>
  </div>
</header>
