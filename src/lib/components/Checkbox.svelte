<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    checked = $bindable(false),
    disabled = false,
    label,
    children
  }: { checked?: boolean; disabled?: boolean; label?: string; children?: Snippet } = $props();
</script>

<button
  type="button"
  {disabled}
  role="checkbox"
  aria-checked={checked}
  aria-label={label}
  onclick={() => (checked = !checked)}
  class="group inline-flex items-center gap-2 text-xs text-[#8c8c8c] disabled:cursor-not-allowed disabled:opacity-40"
>
  <span
    class="flex h-4 w-4 items-center justify-center rounded border transition-colors {checked
      ? 'border-[#7c9a82]/50 bg-[#3a3a3a] text-[#ededed]'
      : 'border-[#2a2a2a] bg-[#0a0a0a] group-hover:border-[#3a3a3a]'}"
  >
    {#if checked}
      <svg viewBox="0 0 12 12" class="h-3 w-3" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M2.5 6.5 5 9l4.5-5.5" />
      </svg>
    {/if}
  </span>
  {#if children}{@render children()}{:else if label}{label}{/if}
</button>
