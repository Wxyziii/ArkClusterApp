<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader, Card, MapCard, StatusBadge, ConfirmActionDialog, EmptyState, Button, SafetyWarningPanel, LoadingState, BackendStatusBanner } from '$lib/components';
  import { maps as mockMaps } from '$lib/data/mock';
  import { api, loadWithFallback } from '$lib/api';
  import type { ArkMap } from '$lib/types';

  let view = $state<'cards' | 'table'>('cards');
  let dialogOpen = $state(false);
  let pending = $state<{ action: string; map: ArkMap } | null>(null);

  // Live data with mock fallback so the page works with the backend down.
  let maps = $state<ArkMap[]>(mockMaps);
  let loading = $state(true);
  let fromFallback = $state(false);
  let loadError = $state<string | null>(null);

  onMount(async () => {
    const res = await loadWithFallback(() => api.servers(), mockMaps);
    maps = res.data;
    fromFallback = res.fromFallback;
    loadError = res.error;
    loading = false;
  });

  let travelMaps = $derived(maps.filter((m) => m.assignment === 'Travel A' || m.assignment === 'Travel B'));

  function handle(action: string, map: ArkMap) {
    if (action === 'start' || action === 'backup') {
      // non-destructive — fire immediately (mock)
      return;
    }
    pending = { action, map };
    dialogOpen = true;
  }

  let isHomeStop = $derived(pending?.action === 'stop' && pending?.map.isHome);
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
    Control disabled in this phase. The manager reads configured unit status only; Start, Stop, Restart, RCON, and backup actions are not implemented.
  </SafetyWarningPanel>
</div>

{#if loading}
  <LoadingState label="Loading maps from manager…" rows={4} />
{:else}
{#if travelMaps.length === 0}
  <div class="mb-5"><EmptyState icon="🧭" title="No travel maps active" hint="Both travel slots are free. Players can start one with !travel mapname." /></div>
{/if}

{#if view === 'cards'}
  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
    {#each maps as map (map.id)}<MapCard {map} onaction={handle} />{/each}
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
  confirmLabel="{pending?.action === 'stop' ? 'Stop' : 'Restart'} map"
  requirePhrase={isHomeStop ? 'STOP HOME' : undefined}
>
  {#snippet body()}
    {#if isHomeStop}
      <SafetyWarningPanel tone="danger" title="This is the protected Home map">
        Home should normally stay online. Stopping it manually triggers a save + backup, then a full shutdown. Home is meant to leave only via <strong>Resource Standby</strong> (empty + resource pressure). It will auto-restart when resources recover or when requested.
      </SafetyWarningPanel>
    {:else if pending?.map.players}
      <p>⚠️ <strong>{pending.map.players} player(s)</strong> are connected to {pending.map.name}. They will be disconnected. A save + backup runs first.</p>
    {:else}
      <p>This runs a save + backup, then {pending?.action === 'stop' ? 'stops' : 'restarts'} <strong>{pending?.map.name}</strong> via <code class="font-mono text-[#8aa1ae]">{pending?.map.config.systemdUnit}</code>.</p>
    {/if}
  {/snippet}
</ConfirmActionDialog>
