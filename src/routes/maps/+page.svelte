<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader, Card, MapCard, StatusBadge, ConfirmActionDialog, EmptyState, Button, SafetyWarningPanel, LoadingState, BackendStatusBanner } from '$lib/components';
  import { maps as mockMaps } from '$lib/data/mock';
  import { api, loadWithFallback, type Capabilities } from '$lib/api';
  import type { ArkMap } from '$lib/types';

  let view = $state<'cards' | 'table'>('cards');
  let dialogOpen = $state(false);
  let pending = $state<{ action: string; map: ArkMap } | null>(null);

  // Live data with mock fallback so the page works with the backend down.
  let maps = $state<ArkMap[]>(mockMaps);
  let capabilities = $state<Capabilities | null>(null);
  let loading = $state(true);
  let fromFallback = $state(false);
  let loadError = $state<string | null>(null);
  let actionMessage = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  onMount(async () => {
    const [res, caps] = await Promise.all([
      loadWithFallback(() => api.servers(), mockMaps),
      loadWithFallback(() => api.capabilities(), null)
    ]);
    maps = res.data;
    if (caps.data) capabilities = caps.data;
    fromFallback = res.fromFallback || caps.fromFallback;
    loadError = res.error ?? caps.error;
    loading = false;
  });

  let travelMaps = $derived(maps.filter((m) => m.assignment === 'Travel A' || m.assignment === 'Travel B'));

  function handle(action: string, map: ArkMap) {
    pending = { action, map };
    dialogOpen = true;
  }

  let isHomeStop = $derived(pending?.action === 'stop' && pending?.map.isHome);
  let isHomeRestart = $derived(pending?.action === 'restart' && pending?.map.isHome);
  let confirmPhrase = $derived(isHomeStop ? 'STOP HOME' : isHomeRestart ? 'RESTART HOME' : undefined);

  async function runPending() {
    if (!pending) return;
    actionMessage = null;
    actionError = null;
    try {
      const reason = isHomeStop ? 'manual_admin_override' : 'manual_admin_action';
      const result = await api.serverAction(pending.map.id, pending.action as 'start' | 'stop' | 'restart' | 'backup', {
        confirm: true,
        strongConfirm: isHomeStop || isHomeRestart,
        adminOverride: false,
        reason
      });
      actionMessage = result.message;
      maps = await api.servers();
    } catch (e) {
      actionError = e instanceof Error ? e.message : 'action failed';
    }
  }
</script>

<PageHeader title="Maps" icon="🗺️" subtitle="All configured ARK maps, roles, slots and live state">
  {#snippet actions()}
    <div class="flex rounded-lg border border-[#2a2a2a] p-0.5">
      <button class="rounded px-2.5 py-1 text-xs {view === 'cards' ? 'bg-[#222222] text-[#7c9a82]' : 'text-[#8c8c8c]'}" onclick={() => (view = 'cards')}>Cards</button>
      <button class="rounded px-2.5 py-1 text-xs {view === 'table' ? 'bg-[#222222] text-[#7c9a82]' : 'text-[#8c8c8c]'}" onclick={() => (view = 'table')}>Table</button>
    </div>
  {/snippet}
</PageHeader>

{#if fromFallback}<BackendStatusBanner error={loadError} />{/if}

<div class="mb-5">
  <SafetyWarningPanel tone="warn" title="Read-only status">
    Controls are capability-gated. Systemd actions use configured units only; backup uses configured safe paths only.
  </SafetyWarningPanel>
</div>

{#if actionMessage}
  <div class="mb-5"><SafetyWarningPanel tone="info" title="Action complete">{actionMessage}</SafetyWarningPanel></div>
{/if}
{#if actionError}
  <div class="mb-5"><SafetyWarningPanel tone="danger" title="Action blocked or failed">{actionError}</SafetyWarningPanel></div>
{/if}

{#if loading}
  <LoadingState label="Loading maps from manager…" rows={4} />
{:else}
{#if travelMaps.length === 0}
  <div class="mb-5"><EmptyState icon="🧭" title="No travel maps active" hint="Both travel slots are free. Players can start one with !travel mapname." /></div>
{/if}

{#if view === 'cards'}
  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
    {#each maps as map (map.id)}<MapCard {map} {capabilities} onaction={handle} />{/each}
  </div>
{:else}
  <Card pad={false}>
    <div class="overflow-x-auto">
      <table class="w-full text-xs">
        <thead>
          <tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]">
            <th class="px-3 py-2.5 font-medium">Map</th>
            <th class="px-3 py-2.5 font-medium">Role</th>
            <th class="px-3 py-2.5 font-medium">Slot</th>
            <th class="px-3 py-2.5 font-medium">State</th>
            <th class="px-3 py-2.5 font-medium">Players</th>
            <th class="px-3 py-2.5 font-medium">RCON</th>
            <th class="px-3 py-2.5 font-medium">systemd</th>
            <th class="px-3 py-2.5 text-right font-medium"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[#2a2a2a]/50">
          {#each maps as map (map.id)}
            <tr class="hover:bg-[#181818]/50">
              <td class="px-3 py-2.5 font-medium text-[#ededed]">{map.name}{#if map.isHome}<span class="ml-1.5 text-[10px] text-[#7c9a82]">🛡</span>{/if}</td>
              <td class="px-3 py-2.5 text-[#8c8c8c]">{map.role}</td>
              <td class="px-3 py-2.5 text-[#8c8c8c]">{map.assignment}</td>
              <td class="px-3 py-2.5"><StatusBadge label={map.state} tone={map.state === 'Online' ? 'green' : map.state === 'Offline' || map.state === 'Resource Standby' ? 'gray' : map.state === 'Error' ? 'red' : 'amber'} size="sm" /></td>
              <td class="px-3 py-2.5 tabular-nums">{map.players}/{map.maxPlayers}</td>
              <td class="px-3 py-2.5"><StatusBadge label={map.rcon} tone={map.rcon === 'Connected' ? 'cyan' : 'gray'} size="sm" /></td>
              <td class="px-3 py-2.5 font-mono text-[10px] text-[#8c8c8c]">{map.systemd}</td>
              <td class="px-3 py-2.5 text-right"><Button size="sm" variant="ghost" href="/maps/{map.id}">Details</Button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </Card>
{/if}
{/if}

<ConfirmActionDialog
  bind:open={dialogOpen}
  title="{pending?.action === 'stop' ? 'Stop' : 'Restart'} {pending?.map.name}?"
  tone={isHomeStop ? 'danger' : 'warn'}
  confirmLabel="{pending?.action === 'backup' ? 'Back up' : pending?.action === 'start' ? 'Start' : pending?.action === 'stop' ? 'Stop' : 'Restart'} map"
  requirePhrase={confirmPhrase}
  onconfirm={runPending}
>
  {#snippet body()}
    {#if isHomeStop}
      <SafetyWarningPanel tone="danger" title="This is the protected Home map">
        Home stop is blocked unless the backend config explicitly allows it and this request uses a strong admin override reason.
      </SafetyWarningPanel>
    {:else if pending?.action === 'backup'}
      <p>Create a real backup from configured safe paths only for <strong>{pending.map.name}</strong>. Restore/delete remain disabled.</p>
    {:else if pending?.map.players}
      <p>⚠️ <strong>{pending.map.players} player(s)</strong> are connected to {pending.map.name}. They will be disconnected. A save + backup runs first.</p>
    {:else}
      <p>This requests <strong>{pending?.action}</strong> for <strong>{pending?.map.name}</strong> via configured unit <code class="font-mono text-[#8aa1ae]">{pending?.map.config.systemdUnit}</code>.</p>
    {/if}
  {/snippet}
</ConfirmActionDialog>
