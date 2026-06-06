<script lang="ts">
  import StatusBadge from './StatusBadge.svelte';
  import type { Tone } from '$lib/types';

  let {
    decision = 'unavailable',
    why = 'Resource governor data unavailable.',
    tone = 'gray'
  }: {
    decision?: string;
    why?: string;
    tone?: Tone;
  } = $props();
</script>

<div class="card overflow-hidden">
  <header class="flex items-center justify-between border-b border-[#2a2a2a] px-4 py-3">
    <h2 class="text-sm font-semibold">Resource Governor</h2>
    <StatusBadge label={decision} {tone} size="sm" />
  </header>
  <div class="p-4 text-sm text-[#8c8c8c]">
    {why}
  </div>
</div>
