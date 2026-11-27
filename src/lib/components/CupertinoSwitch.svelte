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

  const uid = Math.random().toString(36).slice(2, 9);
  const switchId = `cupertino-switch-${uid}`;
  const descId = `cupertino-switch-desc-${uid}`;

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<div class="flex items-center justify-between py-2.5 px-3 rounded-2xl hover:bg-white/[0.03] transition-colors group select-none {disabled ? 'opacity-50' : ''}">
  {#if label || description || icon}
    <label for={switchId} class="flex items-center gap-3.5 flex-1 pr-4 min-w-0 {disabled ? 'cursor-not-allowed' : 'cursor-pointer'}">
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
          <span id={descId} class="text-xs text-white/50 leading-relaxed mt-0.5 line-clamp-2">{description}</span>
        {/if}
      </div>
    </label>
  {/if}

  <!-- Cupertino Toggle Switch -->
  <button
    id={switchId}
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={label || 'Toggle switch'}
    aria-describedby={description ? descId : undefined}
    {disabled}
    onclick={toggle}
    class="relative inline-flex h-7 w-[48px] shrink-0 rounded-full p-[2px] focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-2 {disabled ? 'cursor-not-allowed' : 'cursor-pointer'}"
    style="
      background-color: {checked ? 'var(--md-sys-primary)' : 'rgba(255, 255, 255, 0.16)'};
      outline-color: var(--md-sys-primary);
      transition: background-color 0.35s cubic-bezier(0.16, 1, 0.3, 1);
    "
  >
    <span
      class="pointer-events-none inline-block h-6 w-6 rounded-full shadow-md"
      style="
        background-color: {checked ? 'var(--md-sys-on-primary, #ffffff)' : '#ffffff'};
        transform: {checked ? 'translateX(20px)' : 'translateX(0px)'};
        transition: transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
        box-shadow: 0 3px 8px rgba(0, 0, 0, 0.32), 0 1px 2px rgba(0, 0, 0, 0.18);
      "
    ></span>
  </button>
</div>
