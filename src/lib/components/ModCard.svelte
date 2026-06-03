<script lang="ts">
  import type { Mod } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Button from './Button.svelte';
  import { fmtMb } from '$lib/ui';
  import type { Tone } from '$lib/types';

  let { mod, onaction }: { mod: Mod; onaction?: (a: string, m: Mod) => void } = $props();

  const stateMeta: Record<string, { tone: Tone; label: string }> = {
    active: { tone: 'green', label: 'Active' },
    disabled: { tone: 'gray', label: 'Disabled (files kept)' },
    downloading: { tone: 'amber', label: 'Downloading' },
    failed: { tone: 'red', label: 'Download failed' },
    missing: { tone: 'red', label: 'Missing / deleted' }
  };
  let m = $derived(stateMeta[mod.state]);
</script>

<div class="card-elevated p-3">
  <div class="flex items-start justify-between gap-2">
    <div class="min-w-0">
      <p class="flex items-center gap-2 text-sm font-medium text-[#ededed]">
        {#if mod.enabled && mod.installed}<span class="font-mono text-[10px] text-[#7c9a82]">#{mod.loadOrder}</span>{/if}
        <span class="truncate">{mod.name}</span>
      </p>
      <p class="mt-0.5 font-mono text-[11px] text-[#8c8c8c]">Workshop {mod.workshopId}</p>
    </div>
    <StatusBadge label={m.label} tone={m.tone} size="sm" dot={mod.state === 'downloading'} pulse={mod.state === 'downloading'} />
  </div>

  {#if mod.state === 'downloading' && mod.progress != null}
    <div class="mt-2"><ProgressBar value={mod.progress} tone="amber" striped height="h-1.5" /></div>
  {/if}
  {#if mod.state === 'failed'}
    <p class="mt-2 rounded bg-[#b5544f]/8 px-2 py-1 text-[10px] text-[#b5544f]">SteamCMD download failed — retry or remove.</p>
  {/if}
  {#if mod.state === 'missing'}
    <p class="mt-2 rounded bg-[#b5544f]/8 px-2 py-1 text-[10px] text-[#b5544f]">Files missing on disk — referenced but not installed.</p>
  {/if}

  <div class="mt-2 grid grid-cols-2 gap-x-3 gap-y-1 text-[11px] text-[#8c8c8c]">
    <span>Updated: {mod.lastUpdated}</span>
    <span>Size: {fmtMb(mod.sizeMb)}</span>
    <span class="col-span-2 truncate">Maps: {mod.usedBy.length ? mod.usedBy.join(', ') : '—'}</span>
  </div>

  <div class="mt-3 flex flex-wrap gap-1.5 border-t border-[#2a2a2a] pt-2">
    {#if mod.enabled}
      <Button size="sm" variant="warn" onclick={() => onaction?.('disable', mod)} title="Remove from load order, keep files">Disable</Button>
    {:else if mod.installed}
      <Button size="sm" variant="primary" onclick={() => onaction?.('enable', mod)}>Enable</Button>
    {/if}
    {#if mod.state === 'failed'}
      <Button size="sm" variant="default" onclick={() => onaction?.('retry', mod)}>Retry download</Button>
    {/if}
    <Button size="sm" variant="danger" onclick={() => onaction?.('remove', mod)} title="Remove from load order AND delete files — strong confirmation">Remove</Button>
  </div>
</div>
