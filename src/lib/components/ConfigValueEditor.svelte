<script lang="ts">
  import type { ConfigField } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';

  let { field = $bindable() }: { field: ConfigField } = $props();

  let invalid = $derived(
    field.type === 'number' &&
      typeof field.value === 'number' &&
      ((field.min != null && field.value < field.min) || (field.max != null && field.value > field.max))
  );
</script>

<div class="rounded-lg border border-[#2a2a2a] bg-[#0a0a0a]/40 p-3 {invalid ? 'border-[#b5544f]/50' : ''}">
  <div class="flex items-center justify-between gap-2">
    <label for="cfg-{field.key}" class="text-xs font-medium text-[#ededed]">{field.label}</label>
    {#if field.restartRequired}<StatusBadge label="restart" tone="amber" size="sm" />{/if}
  </div>
  <p class="mt-0.5 mb-2 font-mono text-[10px] text-[#5c5c5c]">{field.key}</p>

  {#if field.type === 'bool'}
    <button
      id="cfg-{field.key}"
      onclick={() => (field.value = !field.value)}
      class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {field.value ? 'bg-[#3a3a3a]' : 'bg-[#2a2a2a]'}"
      role="switch"
      aria-checked={field.value as boolean}
      aria-label="Toggle {field.label}"
    >
      <span class="inline-block h-4 w-4 transform rounded-full bg-[#ededed] transition-transform {field.value ? 'translate-x-6' : 'translate-x-1'}"></span>
    </button>
    <span class="ml-2 text-xs text-[#8c8c8c]">{field.value ? 'On' : 'Off'}</span>
  {:else if field.type === 'enum'}
    <select
      id="cfg-{field.key}"
      bind:value={field.value}
      class="w-full rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-2.5 py-1.5 text-sm text-[#ededed] outline-none focus:border-[#3a3a3a]"
    >
      {#each field.options ?? [] as opt (opt)}<option value={opt}>{opt}</option>{/each}
    </select>
  {:else}
    <div class="flex items-center gap-2">
      <input
        id="cfg-{field.key}"
        type="number"
        bind:value={field.value}
        min={field.min}
        max={field.max}
        step={field.step}
        class="w-full rounded-lg border bg-[#0a0a0a] px-2.5 py-1.5 font-mono text-sm text-[#ededed] outline-none focus:border-[#3a3a3a] {invalid ? 'border-[#b5544f]' : 'border-[#2a2a2a]'}"
      />
      <span class="whitespace-nowrap text-[10px] text-[#5c5c5c]">{field.min}–{field.max}</span>
    </div>
  {/if}

  <p class="mt-1.5 text-[11px] text-[#8c8c8c]">{field.hint}</p>
  {#if invalid}<p class="mt-1 text-[11px] font-medium text-[#b5544f]">Value out of allowed range ({field.min}–{field.max}).</p>{/if}
</div>
