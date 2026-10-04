<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  async function handleMinimize() {
    try {
      await invoke('window_minimize');
    } catch (e) {
      console.error('Minimize error:', e);
    }
  }

  async function handleMaximize() {
    try {
      await invoke('window_toggle_maximize');
    } catch (e) {
      console.error('Maximize error:', e);
    }
  }

  async function handleClose() {
    try {
      await invoke('window_close');
    } catch (e) {
      console.error('Close error:', e);
    }
  }
</script>

<header class="h-12 w-full flex items-center justify-between px-3 bg-[#111215]/80 backdrop-blur-xl border-b border-white/[0.06] select-none z-30 shrink-0">
  <!-- Left: App Brand -->
  <div class="flex items-center gap-2 px-1">
    <div class="w-6 h-6 rounded-md bg-gradient-to-tr from-purple-500 to-indigo-400 flex items-center justify-center shadow-sm">
      <span class="material-symbols-rounded text-white text-[16px]">graphic_eq</span>
    </div>
    <span class="font-heading font-bold text-sm tracking-wide text-white">Aethon</span>
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
