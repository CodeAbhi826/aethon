<script lang="ts">
  interface Props {
    checked: boolean;
    disabled?: boolean;
    label?: string;
    description?: string;
    icon?: string;
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    label,
    description,
    icon,
    onchange,
  }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      toggle();
    }
  }
</script>

<div class="flex items-center justify-between py-2.5 px-3 rounded-2xl hover:bg-white/[0.03] transition-colors group select-none {disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}" onclick={toggle} onkeydown={handleKeyDown} role="presentation">
  {#if label || description || icon}
    <div class="flex items-center gap-3.5 flex-1 pr-4 min-w-0">
      {#if icon}
        <div
          class="w-8 h-8 rounded-xl flex items-center justify-center shrink-0 transition-colors"
          style="background-color: {checked ? 'var(--md-sys-primary-container)' : 'rgba(255, 255, 255, 0.05)'}; color: {checked ? 'var(--md-sys-on-primary-container)' : 'rgba(255, 255, 255, 0.7)'};"
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
  {/if}

  <!-- Cupertino Toggle Switch -->
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={label || 'Toggle switch'}
    {disabled}
    onclick={(e) => { e.stopPropagation(); toggle(); }}
    class="relative inline-flex h-7 w-[48px] shrink-0 cursor-pointer rounded-full p-[2px] transition-colors duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--md-sys-primary)]/70"
    style="
      background-color: {checked ? 'var(--md-sys-primary)' : 'rgba(255, 255, 255, 0.16)'};
      transition: background-color 0.35s cubic-bezier(0.16, 1, 0.3, 1);
    "
  >
    <span
      class="pointer-events-none inline-block h-6 w-6 rounded-full bg-white shadow-md transform"
      style="
        transform: {checked ? 'translateX(20px)' : 'translateX(0px)'};
        transition: transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
        box-shadow: 0 3px 8px rgba(0, 0, 0, 0.32), 0 1px 2px rgba(0, 0, 0, 0.18);
      "
    ></span>
  </button>
</div>
