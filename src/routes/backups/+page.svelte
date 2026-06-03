<script lang="ts">
  import { PageHeader, Card, BackupTable, Button, StatusBadge, ConfirmActionDialog, SafetyWarningPanel, PolicyCard } from '$lib/components';
  import { backups, backupPolicy } from '$lib/data/mock';
  import type { Backup } from '$lib/types';

  let confirmOpen = $state(false);
  let pending = $state<{ action: string; backup: Backup } | null>(null);

  let filter = $state<'all' | 'success' | 'running' | 'failed'>('all');
  let shown = $derived(filter === 'all' ? backups : backups.filter((b) => b.status === filter));

  function handle(action: string, backup: Backup) {
    if (action === 'restore' || action === 'delete') {
      pending = { action, backup };
      confirmOpen = true;
    }
  }
  let isRestore = $derived(pending?.action === 'restore');
</script>

<PageHeader title="Backups" icon="💾" subtitle="Save, config, mod and cluster-data backups">
  {#snippet actions()}<Button variant="primary" size="sm">Backup now</Button>{/snippet}
</PageHeader>

<div class="mb-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
  {#each [{ l: 'Total', v: backups.length, t: 'cyan' }, { l: 'Success', v: backups.filter((b) => b.status === 'success').length, t: 'green' }, { l: 'Running', v: backups.filter((b) => b.status === 'running' || b.status === 'verifying').length, t: 'amber' }, { l: 'Failed', v: backups.filter((b) => b.status === 'failed').length, t: 'red' }] as s (s.l)}
    <div class="card-elevated p-3 text-center"><p class="text-2xl font-bold tabular-nums">{s.v}</p><p class="mt-0.5 text-xs text-[#8c8c8c]">{s.l}</p></div>
  {/each}
</div>

{#if backups.some((b) => b.status === 'failed')}
  <div class="mb-5"><SafetyWarningPanel tone="danger" title="A backup failed">Review the failed backup below. Auto-shutdown will retry; persistent failures may indicate disk problems.</SafetyWarningPanel></div>
{/if}

<div class="grid grid-cols-1 gap-5 lg:grid-cols-4">
  <div class="lg:col-span-3">
    <Card title="Recent backups" icon="🗂️" pad={false}>
      {#snippet actions()}
        <div class="flex gap-1">
          {#each ['all', 'success', 'running', 'failed'] as f (f)}
            <button class="rounded px-2 py-1 text-[11px] capitalize {filter === f ? 'bg-[#222222] text-[#7c9a82]' : 'text-[#8c8c8c] hover:text-[#ededed]'}" onclick={() => (filter = f as typeof filter)}>{f}</button>
          {/each}
        </div>
      {/snippet}
      <BackupTable backups={shown} onaction={handle} />
    </Card>
  </div>

  <PolicyCard title="Backup policy" icon="🛡️" rows={[
    { label: 'Before shutdown', value: backupPolicy.beforeShutdown },
    { label: 'Before config save', value: backupPolicy.beforeConfigSave },
    { label: 'Before mod changes', value: backupPolicy.beforeModChange },
    { label: 'Retention', value: backupPolicy.retention }
  ]} />
</div>

<ConfirmActionDialog
  bind:open={confirmOpen}
  title="{isRestore ? 'Restore' : 'Delete'} backup — {pending?.backup.map}?"
  tone="danger"
  confirmLabel={isRestore ? 'Restore backup' : 'Delete backup'}
  requirePhrase={isRestore ? 'RESTORE' : undefined}
>
  {#snippet body()}
    {#if isRestore}
      <SafetyWarningPanel tone="danger" title="Restore overwrites current world data">
        Restoring <strong>{pending?.backup.map}</strong> ({pending?.backup.created}) will stop the map, replace its current save with this backup, then restart. Current progress since this backup will be lost. A safety backup of the current state is taken first.
      </SafetyWarningPanel>
    {:else}
      <p>Permanently delete the <strong>{pending?.backup.type}</strong> backup of <strong>{pending?.backup.map}</strong> from {pending?.backup.created}? This cannot be undone.</p>
    {/if}
  {/snippet}
</ConfirmActionDialog>
