<script lang="ts">
  import type { ConfigField } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import Toggle from './Toggle.svelte';
  import Select from './Select.svelte';
  import NumberInput from './NumberInput.svelte';

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
    <div class="flex items-center gap-2">
      <Toggle id="cfg-{field.key}" bind:checked={field.value as boolean} label="Toggle {field.label}" />
      <span class="text-xs text-[#8c8c8c]">{field.value ? 'On' : 'Off'}</span>
    </div>
  {:else if field.type === 'enum'}
    <Select id="cfg-{field.key}" bind:value={field.value as string} options={field.options ?? []} size="sm" />
  {:else}
    <div class="flex items-center gap-2">
      <NumberInput id="cfg-{field.key}" bind:value={field.value as number} min={field.min} max={field.max} step={field.step} {invalid} class="w-full" />
      <span class="whitespace-nowrap text-[10px] text-[#5c5c5c]">{field.min}–{field.max}</span>
    </div>
  {/if}

  <p class="mt-1.5 text-[11px] text-[#8c8c8c]">{field.hint}</p>
  {#if invalid}<p class="mt-1 text-[11px] font-medium text-[#b5544f]">Value out of allowed range ({field.min}–{field.max}).</p>{/if}
</div>
