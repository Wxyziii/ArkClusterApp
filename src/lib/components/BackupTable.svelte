<script lang="ts">
  import type { Backup } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Button from './Button.svelte';
  import { fmtMb } from '$lib/ui';
  import type { Tone } from '$lib/types';

  let { backups, onaction }: { backups: Backup[]; onaction?: (a: string, b: Backup) => void } = $props();

  const statusTone: Record<string, Tone> = { success: 'green', running: 'amber', failed: 'red', verifying: 'cyan' };
  const typeIcon: Record<string, string> = { save: '💾', config: '⚙️', mod: '🧩', 'cluster data': '🗄️' };
</script>

<div class="overflow-x-auto">
  <table class="w-full text-xs">
    <thead>
      <tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]">
        <th class="px-3 py-2 font-medium">Map</th>
        <th class="px-3 py-2 font-medium">Type</th>
        <th class="px-3 py-2 font-medium">Reason</th>
        <th class="px-3 py-2 font-medium">Size</th>
        <th class="px-3 py-2 font-medium">Created</th>
        <th class="px-3 py-2 font-medium">Status</th>
        <th class="px-3 py-2 text-right font-medium">Actions</th>
      </tr>
    </thead>
    <tbody class="divide-y divide-[#2a2a2a]/50">
      {#each backups as b (b.id)}
        <tr class="align-top hover:bg-[#181818]/50">
          <td class="px-3 py-2.5 font-medium text-[#ededed]">{b.map}</td>
          <td class="px-3 py-2.5 text-[#8c8c8c]">{typeIcon[b.type]} {b.type}</td>
          <td class="px-3 py-2.5"><span class="rounded bg-[#0a0a0a]/60 px-1.5 py-0.5 text-[10px] text-[#8c8c8c]">{b.reason}</span></td>
          <td class="px-3 py-2.5 tabular-nums text-[#8c8c8c]">{fmtMb(b.sizeMb)}</td>
          <td class="px-3 py-2.5 font-mono text-[11px] text-[#8c8c8c]">{b.created}</td>
          <td class="px-3 py-2.5">
            <StatusBadge label={b.status} tone={statusTone[b.status]} size="sm" dot pulse={b.status === 'running'} />
            {#if (b.status === 'running' || b.status === 'verifying') && b.progress != null}
              <div class="mt-1.5 w-24"><ProgressBar value={b.progress} tone={statusTone[b.status]} striped height="h-1.5" /></div>
            {/if}
            {#if b.status === 'failed' && b.error}
              <p class="mt-1 max-w-xs font-mono text-[10px] text-[#b5544f]">{b.error}</p>
            {/if}
          </td>
          <td class="px-3 py-2.5">
            <div class="flex justify-end gap-1">
              <Button size="sm" variant="ghost" disabled={b.status !== 'success'} onclick={() => onaction?.('view', b)}>View</Button>
              <Button size="sm" variant="warn" disabled={b.status !== 'success'} title={b.status !== 'success' ? 'Only successful backups can be restored' : 'Restore — strong confirmation required'} onclick={() => onaction?.('restore', b)}>Restore</Button>
              <Button size="sm" variant="danger" disabled={b.status === 'running'} onclick={() => onaction?.('delete', b)}>Delete</Button>
            </div>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
