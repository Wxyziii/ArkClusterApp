<script lang="ts">
  import { PageHeader, Card, ModCard, ModTable, Button, StatusBadge, ConfirmActionDialog, SafetyWarningPanel, RestartRequiredBanner } from '$lib/components';
  import { mods } from '$lib/data/mock';
  import type { Mod } from '$lib/types';

  let newId = $state('');
  let confirmOpen = $state(false);
  let pending = $state<{ action: string; mod: Mod } | null>(null);

  let active = $derived(mods.filter((m) => m.state === 'active'));
  let disabled = $derived(mods.filter((m) => m.state === 'disabled'));
  let problem = $derived(mods.filter((m) => m.state === 'downloading' || m.state === 'failed' || m.state === 'missing'));
  let restartNeeded = $derived(mods.some((m) => m.state === 'downloading' || m.state === 'failed'));

  function handle(action: string, mod: Mod) {
    if (action === 'remove') {
      pending = { action, mod };
      confirmOpen = true;
    }
    // enable/disable/retry/reorder = immediate (mock)
  }
</script>

<PageHeader title="Mods" icon="🧩" subtitle="Steam Workshop mod manager — load order, downloads, enable/disable">
  {#snippet actions()}
    <div class="flex gap-2">
      <input bind:value={newId} placeholder="Workshop ID…" class="w-36 rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-3 py-2 text-xs text-[#ededed] outline-none focus:border-[#3a3a3a]" />
      <Button variant="primary" size="sm" disabled={!newId.trim()}>Add mod</Button>
    </div>
  {/snippet}
</PageHeader>

{#if restartNeeded}<div class="mb-5"><RestartRequiredBanner reason="Mod load order or downloads changed." /></div>{/if}

<div class="mb-5">
  <SafetyWarningPanel tone="warn" title="Disable vs Remove">
    <strong>Disable</strong> = remove from active load order but keep files (instant re-enable). <strong>Remove</strong> = drop from load order <em>and delete files</em> from disk — requires stronger confirmation.
  </SafetyWarningPanel>
</div>

<!-- counts -->
<div class="mb-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
  {#each [{ l: 'Active', v: active.length, t: 'green' }, { l: 'Disabled', v: disabled.length, t: 'gray' }, { l: 'Installed', v: mods.filter((m) => m.installed).length, t: 'cyan' }, { l: 'Problems', v: problem.length, t: 'red' }] as s (s.l)}
    <div class="card-elevated p-3 text-center">
      <p class="text-2xl font-bold tabular-nums">{s.v}</p>
      <p class="mt-0.5 text-xs text-[#8c8c8c]">{s.l}</p>
    </div>
  {/each}
</div>

<div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
  <div class="space-y-5 lg:col-span-2">
    <Card title="Active load order" icon="🔢"><ModTable {mods} /></Card>

    {#if problem.length}
      <Card title="Needs attention" icon="⚠️">
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {#each problem as mod (mod.id)}<ModCard {mod} onaction={handle} />{/each}
        </div>
      </Card>
    {/if}
  </div>

  <div class="space-y-5">
    <Card title="All mods" icon="📦">
      <div class="space-y-3">
        {#each mods as mod (mod.id)}<ModCard {mod} onaction={handle} />{/each}
      </div>
    </Card>
  </div>
</div>

<ConfirmActionDialog
  bind:open={confirmOpen}
  title="Remove {pending?.mod.name}?"
  tone="danger"
  confirmLabel="Delete files & remove"
  requirePhrase="DELETE"
>
  {#snippet body()}
    <SafetyWarningPanel tone="danger" title="This deletes mod files from disk">
      {pending?.mod.name} (Workshop {pending?.mod.workshopId}) will be removed from the active load order <strong>and its files deleted</strong>. Maps using it ({pending?.mod.usedBy.join(', ') || 'none'}) will need a restart. To keep files, use <strong>Disable</strong> instead.
    </SafetyWarningPanel>
  {/snippet}
</ConfirmActionDialog>
