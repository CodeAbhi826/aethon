<script lang="ts">
  import { onMount } from 'svelte';

  export interface SelectOption<T = any> {
    value: T;
    label: string;
    desc?: string;
    icon?: string;
  }

  interface Props<T = any> {
    value: T;
    options: SelectOption<T>[];
    label?: string;
    description?: string;
    icon?: string;
    disabled?: boolean;
    onchange?: (value: T) => void;
  }

  let {
    value = $bindable(),
    options = [],
    label,
    description,
    icon,
    disabled = false,
    onchange,
  }: Props = $props();

  let isOpen = $state(false);
  let containerEl = $state<HTMLDivElement>();

  let selectedOption = $derived(
    options.find((opt) => opt.value === value) || options[0]
  );

  function toggleOpen(e: MouseEvent) {
    e.stopPropagation();
    if (disabled) return;
    isOpen = !isOpen;
  }

  function selectOption(opt: SelectOption) {
    if (disabled) return;
    value = opt.value;
    isOpen = false;
    onchange?.(opt.value);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === 'Escape') {
      isOpen = false;
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (!isOpen) {
        isOpen = true;
      } else {
        const curIdx = options.findIndex((o) => o.value === value);
        if (curIdx < options.length - 1) {
          selectOption(options[curIdx + 1]);
        }
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (isOpen) {
        const curIdx = options.findIndex((o) => o.value === value);
        if (curIdx > 0) {
          selectOption(options[curIdx - 1]);
        }
      }
    }
  }

  onMount(() => {
    function handleClickOutside(e: MouseEvent) {
      if (containerEl && !containerEl.contains(e.target as Node)) {
        isOpen = false;
      }
    }
    window.addEventListener('click', handleClickOutside);
    return () => window.removeEventListener('click', handleClickOutside);
  });
</script>

{#if label || description || icon}
  <!-- Labeled row style matching CupertinoSwitch -->
  <div
    bind:this={containerEl}
    class="relative flex items-center justify-between py-2.5 px-3 rounded-2xl hover:bg-white/[0.03] transition-colors group select-none {disabled ? 'opacity-50 cursor-not-allowed' : ''}"
    onkeydown={handleKeyDown}
    role="presentation"
  >
    <div class="flex items-center gap-3.5 flex-1 pr-4 min-w-0">
      {#if icon}
        <div
          class="w-8 h-8 rounded-xl flex items-center justify-center shrink-0 transition-colors bg-white/[0.05] text-white/70 group-hover:text-white"
        >
          <span class="material-symbols-rounded text-[19px]">{icon}</span>
        </div>
      {/if}
      <div class="flex flex-col min-w-0">
        <span class="text-sm font-semibold text-white/95 group-hover:text-white transition-colors truncate">{label}</span>
        {#if description}
          <span class="text-xs text-white/50 leading-relaxed mt-0.5 line-clamp-2">{description}</span>
        {/if}
      </div>
    </div>

    <!-- Dropdown Trigger Button -->
    <div class="relative shrink-0">
      <button
        type="button"
        onclick={toggleOpen}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        {disabled}
        class="flex items-center justify-between gap-2.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-white bg-black/40 hover:bg-black/60 border border-white/10 hover:border-[var(--md-sys-primary)]/60 focus:border-[var(--md-sys-primary)] focus:ring-2 focus:ring-[var(--md-sys-primary)]/30 backdrop-blur-md transition-all shadow-sm cursor-pointer select-none"
      >
        <span class="truncate max-w-[180px]">{selectedOption?.label ?? 'Select...'}</span>
        <span
          class="material-symbols-rounded text-[18px] text-white/60 transition-transform duration-200 {isOpen ? 'rotate-180 text-[var(--md-sys-primary)]' : ''}"
        >
          expand_more
        </span>
      </button>

      <!-- Sleek Frosted Glass Popover Menu -->
      {#if isOpen}
        <div
          class="absolute right-0 top-full mt-2 w-64 max-h-60 overflow-y-auto z-50 rounded-2xl bg-[#181520]/95 backdrop-blur-2xl border border-white/15 p-1.5 shadow-2xl space-y-1 animate-dropdown scrollbar-none"
          role="listbox"
        >
          {#each options as opt}
            {@const isSelected = opt.value === value}
            <button
              type="button"
              role="option"
              aria-selected={isSelected}
              onclick={(e) => { e.stopPropagation(); selectOption(opt); }}
              class="w-full flex items-center justify-between gap-3 px-3 py-2 rounded-xl text-xs text-left transition-all {isSelected ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] font-semibold shadow-sm' : 'text-white/80 hover:text-white hover:bg-white/[0.08]'}"
            >
              <div class="flex flex-col min-w-0 flex-1">
                <span class="truncate {isSelected ? 'text-[var(--md-sys-on-primary-container)]' : 'text-white'}">{opt.label}</span>
                {#if opt.desc}
                  <span class="text-[10px] opacity-60 leading-tight truncate">{opt.desc}</span>
                {/if}
              </div>
              {#if isSelected}
                <span class="material-symbols-rounded text-[17px] text-[var(--md-sys-primary)] shrink-0">check</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{:else}
  <!-- Standalone Trigger & Popover Button -->
  <div
    bind:this={containerEl}
    class="relative inline-block select-none"
    onkeydown={handleKeyDown}
    role="presentation"
  >
    <button
      type="button"
      onclick={toggleOpen}
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      {disabled}
      class="flex items-center justify-between gap-2.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-white bg-black/40 hover:bg-black/60 border border-white/10 hover:border-[var(--md-sys-primary)]/60 focus:border-[var(--md-sys-primary)] focus:ring-2 focus:ring-[var(--md-sys-primary)]/30 backdrop-blur-md transition-all shadow-sm cursor-pointer select-none"
    >
      <span class="truncate max-w-[180px]">{selectedOption?.label ?? 'Select...'}</span>
      <span
        class="material-symbols-rounded text-[18px] text-white/60 transition-transform duration-200 {isOpen ? 'rotate-180 text-[var(--md-sys-primary)]' : ''}"
      >
        expand_more
      </span>
    </button>

    {#if isOpen}
      <div
        class="absolute right-0 top-full mt-2 w-64 max-h-60 overflow-y-auto z-50 rounded-2xl bg-[#181520]/95 backdrop-blur-2xl border border-white/15 p-1.5 shadow-2xl space-y-1 animate-dropdown scrollbar-none"
        role="listbox"
      >
        {#each options as opt}
          {@const isSelected = opt.value === value}
          <button
            type="button"
            role="option"
            aria-selected={isSelected}
            onclick={(e) => { e.stopPropagation(); selectOption(opt); }}
            class="w-full flex items-center justify-between gap-3 px-3 py-2 rounded-xl text-xs text-left transition-all {isSelected ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] font-semibold shadow-sm' : 'text-white/80 hover:text-white hover:bg-white/[0.08]'}"
          >
            <div class="flex flex-col min-w-0 flex-1">
              <span class="truncate {isSelected ? 'text-[var(--md-sys-on-primary-container)]' : 'text-white'}">{opt.label}</span>
              {#if opt.desc}
                <span class="text-[10px] opacity-60 leading-tight truncate">{opt.desc}</span>
              {/if}
            </div>
            {#if isSelected}
              <span class="material-symbols-rounded text-[17px] text-[var(--md-sys-primary)] shrink-0">check</span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  @keyframes dropdownFade {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  .animate-dropdown {
    animation: dropdownFade 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }
</style>
