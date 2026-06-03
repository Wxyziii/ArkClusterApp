<script lang="ts">
  import ProgressBar from './ProgressBar.svelte';
  import { barTone } from '$lib/ui';
  let {
    label,
    pct,
    detail,
    icon,
    warn = 70,
    danger = 88
  }: { label: string; pct: number; detail: string; icon: string; warn?: number; danger?: number } = $props();
  let tone = $derived(barTone(pct, warn, danger));
</script>

<div class="card-elevated p-4">
  <div class="flex items-center justify-between">
    <span class="flex items-center gap-2 text-xs font-medium text-[#8c8c8c]">
      <span class="text-sm">{icon}</span>{label}
    </span>
    <span
      class="text-lg font-bold tabular-nums"
      class:text-[#7c9a82]={tone === 'green'}
      class:text-[#bfa15e]={tone === 'amber'}
      class:text-[#b5544f]={tone === 'red'}
    >{pct}%</span>
  </div>
  <div class="mt-3"><ProgressBar value={pct} {tone} /></div>
  <p class="mt-2 text-xs text-[#8c8c8c] tabular-nums">{detail}</p>
</div>
