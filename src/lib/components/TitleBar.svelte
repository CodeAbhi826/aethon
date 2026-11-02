<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  async function handleMinimize() {
    try {
      await invoke('window_minimize');
    } catch {}
  }

  async function handleMaximize() {
    try {
      await invoke('window_toggle_maximize');
    } catch {}
  }

  async function handleClose() {
    try {
      await invoke('window_close');
    } catch {}
  }
</script>

<header class="h-11 w-full flex items-center justify-between px-3.5 bg-[#0e0f13]/85 backdrop-blur-xl border-b border-white/[0.05] select-none z-30 shrink-0">
  <div class="flex items-center gap-2">
    <img src="/aethon_logo.svg" alt="Aethon" class="w-5 h-5 rounded-md object-contain" />
    <span class="font-heading font-bold text-xs tracking-wide text-white/90">Aethon</span>
  </div>

  <div data-tauri-drag-region class="flex-1 h-full mx-4 cursor-default"></div>

  <div class="flex items-center gap-1">
    <button
      onclick={handleMinimize}
      aria-label="Minimize"
      class="w-7 h-7 rounded-md flex items-center justify-center text-white/55 hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
    >
      <span class="material-symbols-rounded text-[15px]">remove</span>
    </button>
    <button
      onclick={handleMaximize}
      aria-label="Maximize"
      class="w-7 h-7 rounded-md flex items-center justify-center text-white/55 hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
    >
      <span class="material-symbols-rounded text-[14px]">crop_square</span>
    </button>
    <button
      onclick={handleClose}
      aria-label="Close"
      class="w-7 h-7 rounded-md flex items-center justify-center text-white/55 hover:text-white hover:bg-red-500/80 transition-colors cursor-pointer"
    >
      <span class="material-symbols-rounded text-[15px]">close</span>
    </button>
  </div>
</header>

