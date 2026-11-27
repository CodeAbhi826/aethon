<script lang="ts">
  import { onMount, tick } from 'svelte';

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
  let openUpward = $state(false);
  let highlightedIndex = $state(-1);
  let containerEl = $state<HTMLDivElement>();
  let triggerEl = $state<HTMLButtonElement>();
  let popoverEl = $state<HTMLDivElement>();

  let selectedOption = $derived(
    options.find((opt) => opt.value === value) || options[0]
  );

  function checkDirection() {
    if (containerEl) {
      const rect = containerEl.getBoundingClientRect();
      const spaceBelow = window.innerHeight - rect.bottom - 88; // account for BottomPlayer height
      openUpward = spaceBelow < 240 && rect.top > 260;
    }
  }

  async function openMenu() {
    checkDirection();
    const curIdx = options.findIndex((o) => o.value === value);
    highlightedIndex = curIdx >= 0 ? curIdx : 0;
    isOpen = true;
    await tick();
    if (popoverEl) {
      const selected = popoverEl.querySelector('[aria-selected="true"]') as HTMLElement;
      selected?.scrollIntoView({ block: 'nearest' });
    }
  }

  async function toggleOpen() {
    if (disabled) return;
    if (!isOpen) {
      await openMenu();
    } else {
      isOpen = false;
    }
  }

  function selectOption(opt: SelectOption) {
    if (disabled) return;
    value = opt.value;
    isOpen = false;
    onchange?.(opt.value);
    triggerEl?.focus();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === 'Escape') {
      if (isOpen) {
        e.preventDefault();
        e.stopPropagation();
        isOpen = false;
        triggerEl?.focus();
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (!isOpen) {
        openMenu();
      } else if (options.length > 0) {
        highlightedIndex = (highlightedIndex + 1) % options.length;
        const btns = popoverEl?.querySelectorAll<HTMLElement>('[role="option"]');
        btns?.[highlightedIndex]?.scrollIntoView({ block: 'nearest' });
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (!isOpen) {
        openMenu();
      } else if (options.length > 0) {
        highlightedIndex = (highlightedIndex - 1 + options.length) % options.length;
        const btns = popoverEl?.querySelectorAll<HTMLElement>('[role="option"]');
        btns?.[highlightedIndex]?.scrollIntoView({ block: 'nearest' });
      }
    } else if ((e.key === 'Enter' || e.key === ' ') && isOpen) {
      e.preventDefault();
      const target = options[highlightedIndex];
      if (target) selectOption(target);
    }
  }

  onMount(() => {
    function handleClickOutside(e: MouseEvent) {
      if (containerEl && !containerEl.contains(e.target as Node)) {
        isOpen = false;
      }
    }
    // Using capture phase ensures any other open dropdown closes immediately on click
    window.addEventListener('click', handleClickOutside, { capture: true });
    return () => window.removeEventListener('click', handleClickOutside, { capture: true });
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
        bind:this={triggerEl}
        type="button"
        onclick={toggleOpen}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-label={label || 'Select option'}
        {disabled}
        class="flex items-center justify-between gap-2.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-[var(--md-sys-on-surface)] border border-white/10 hover:border-white/25 focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-2 transition-colors shadow-sm cursor-pointer select-none"
        style="background-color: var(--md-sys-surface-container-low, #181422); outline-color: var(--md-sys-primary);"
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
          bind:this={popoverEl}
          class="absolute right-0 {openUpward ? 'bottom-full mb-2' : 'top-full mt-2'} w-64 max-h-60 overflow-y-auto z-50 rounded-2xl border border-white/15 p-1.5 shadow-2xl space-y-1 animate-dropdown scrollbar-none"
          style="background-color: var(--md-sys-surface-container-high, #1c1726);"
          role="listbox"
        >
          {#each options as opt, optIdx}
            {@const isSelected = opt.value === value}
            {@const isHighlighted = optIdx === highlightedIndex}
            <button
              type="button"
              role="option"
              aria-selected={isSelected}
              onclick={(e) => { e.stopPropagation(); selectOption(opt); }}
              class="w-full flex items-center justify-between gap-3 px-3 py-2 rounded-xl text-xs text-left transition-colors cursor-pointer {isSelected ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] font-semibold shadow-sm' : isHighlighted ? 'bg-white/[0.1] text-white' : 'text-white/80 hover:text-white hover:bg-white/[0.08]'}"
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
      bind:this={triggerEl}
      type="button"
      onclick={toggleOpen}
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      aria-label={label || 'Select option'}
      {disabled}
      class="flex items-center justify-between gap-2.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-[var(--md-sys-on-surface)] border border-white/10 hover:border-white/25 focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-2 transition-colors shadow-sm cursor-pointer select-none"
      style="background-color: var(--md-sys-surface-container-low, #181422); outline-color: var(--md-sys-primary);"
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
        bind:this={popoverEl}
        class="absolute right-0 {openUpward ? 'bottom-full mb-2' : 'top-full mt-2'} w-64 max-h-60 overflow-y-auto z-50 rounded-2xl border border-white/15 p-1.5 shadow-2xl space-y-1 animate-dropdown scrollbar-none"
        style="background-color: var(--md-sys-surface-container-high, #1c1726);"
        role="listbox"
      >
        {#each options as opt, optIdx}
          {@const isSelected = opt.value === value}
          {@const isHighlighted = optIdx === highlightedIndex}
          <button
            type="button"
            role="option"
            aria-selected={isSelected}
            onclick={(e) => { e.stopPropagation(); selectOption(opt); }}
            class="w-full flex items-center justify-between gap-3 px-3 py-2 rounded-xl text-xs text-left transition-colors cursor-pointer {isSelected ? 'bg-[var(--md-sys-primary-container)] text-[var(--md-sys-on-primary-container)] font-semibold shadow-sm' : isHighlighted ? 'bg-white/[0.1] text-white' : 'text-white/80 hover:text-white hover:bg-white/[0.08]'}"
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
