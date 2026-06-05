<script lang="ts">
  import { onMount } from 'svelte';
  import {
    PageHeader, Card, StatusBadge, TravelStepper, Button, SafetyWarningPanel, TravelSlotCard
  } from '$lib/components';
  import { api, ApiError, loadWithFallback, type TravelDecision, type TravelState } from '$lib/api';
  import { maps as mockMaps, travelA as mockTravelA, travelB as mockTravelB, homeMap as mockHomeMap } from '$lib/data/mock';
  import type { ArkMap, Tone } from '$lib/types';

  let maps = $state<ArkMap[]>(mockMaps);
  let travel = $state<TravelState>({
    enabled: false,
    idleShutdownSecs: 10800,
    idleShutdownProduction: true,
    maxTravelServers: 2,
    homeResourceStandby: true,
    slots: {
      home: mockHomeMap ?? undefined,
      travelA: mockTravelA ?? undefined,
      travelB: mockTravelB ?? undefined
    },
    recent: [],
    queue: []
  });
  let selected = $state('ragnarok');
  let decision = $state<TravelDecision | null>(null);
  let error = $state<string | null>(null);
  let fromFallback = $state(false);
  let working = $state(false);

  let destinations = $derived(maps.filter((m) => m.role === 'Travel-capable' || m.role === 'Home-capable'));
  let selectedMap = $derived(destinations.find((m) => m.alias === selected || m.id === selected) ?? destinations[0] ?? null);
  let slotFree = $derived(!travel.slots.travelA || !travel.slots.travelB || travel.slots.travelA.players === 0 || travel.slots.travelB.players === 0);
  let result = $derived(decision ? {
    label: decision.status,
    tone: decision.accepted ? 'green' as Tone : 'amber' as Tone,
    reason: decision.reason
  } : travel.enabled ? {
    label: slotFree ? 'Ready to request' : 'Queue likely',
    tone: slotFree ? 'green' as Tone : 'amber' as Tone,
    reason: travel.blockReason ?? `${travel.maxTravelServers} travel slots. Empty maps shut down after ${Math.round(travel.idleShutdownSecs / 60)} min.`
  } : {
    label: 'Disabled',
    tone: 'gray' as Tone,
    reason: travel.blockReason ?? 'Travel scheduler disabled in manager config.'
  });

  let conditions = $derived([
    { label: 'Travel scheduler enabled', ok: travel.enabled },
    { label: `Max travel servers ${travel.maxTravelServers}`, ok: travel.maxTravelServers > 0 },
    { label: 'Travel Slot A free/empty', ok: !travel.slots.travelA || travel.slots.travelA.players === 0 },
    { label: 'Travel Slot B free/empty', ok: !travel.slots.travelB || travel.slots.travelB.players === 0 },
    { label: 'Home standby policy enabled', ok: travel.homeResourceStandby },
    { label: '3h idle shutdown production rule', ok: travel.idleShutdownProduction }
  ]);

  onMount(load);

  async function load() {
    const [serverRes, travelRes] = await Promise.all([
      loadWithFallback(() => api.servers(), mockMaps),
      loadWithFallback(() => api.travel(), travel)
    ]);
    maps = serverRes.data;
    travel = travelRes.data;
    fromFallback = serverRes.fromFallback || travelRes.fromFallback;
    error = serverRes.error ?? travelRes.error;
    selected = selectedMap?.alias ?? destinations[0]?.alias ?? selected;
  }

  async function requestTravel() {
    if (!selectedMap) return;
    working = true;
    error = null;
    decision = null;
    try {
      decision = await api.travelRequest({ map: selectedMap.alias, source: 'web_ui', actor: 'Marcel' });
      await load();
    } catch (e) {
      if (e instanceof ApiError && e.payload && typeof e.payload === 'object') {
        decision = e.payload as TravelDecision;
      }
      error = e instanceof Error ? e.message : 'travel request failed';
    } finally {
      working = false;
    }
  }
</script>

<PageHeader title="Travel" icon="🧭" subtitle="Live on-demand map requests and travel slot policy">
  {#snippet actions()}
    <StatusBadge label={result.label} tone={result.tone} dot />
    <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
  {/snippet}
</PageHeader>

{#if fromFallback}<div class="mb-5"><SafetyWarningPanel tone="warn" title="Fallback travel">Backend unavailable: {error}</SafetyWarningPanel></div>{/if}
{#if error && !fromFallback}<div class="mb-5"><SafetyWarningPanel tone={decision ? 'warn' : 'danger'} title="Travel response">{error}</SafetyWarningPanel></div>{/if}

<div class="mb-5 grid grid-cols-1 gap-3 md:grid-cols-3">
  <TravelSlotCard slot="Home" map={travel.slots.home ?? null} />
  <TravelSlotCard slot="Travel A" map={travel.slots.travelA ?? null} />
  <TravelSlotCard slot="Travel B" map={travel.slots.travelB ?? null} />
</div>

<div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
  <div class="space-y-5 lg:col-span-2">
    <Card title="Choose destination" icon="🗺️">
      <div class="grid grid-cols-1 gap-3 md:grid-cols-[1fr_auto]">
        <select
          value={selected}
          onchange={(e) => (selected = (e.currentTarget as HTMLSelectElement).value)}
          class="w-full rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-3 py-2 text-sm text-[#ededed] outline-none focus:border-[#3a3a3a]"
        >
          {#each destinations as d (d.id)}
            <option value={d.alias}>{d.name} (!travel {d.alias})</option>
          {/each}
        </select>
        <Button variant="primary" disabled={!travel.enabled || !selectedMap || working} onclick={requestTravel}>
          {working ? 'Requesting…' : 'Request travel'}
        </Button>
      </div>

      {#if selectedMap}
        <div class="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-4">
          <div class="rounded-lg bg-[#0a0a0a]/40 p-2 text-xs"><p class="text-[#8c8c8c]">State</p><StatusBadge label={selectedMap.state} tone={selectedMap.state === 'Online' ? 'green' : selectedMap.state === 'Starting' ? 'amber' : 'gray'} size="sm" /></div>
          <div class="rounded-lg bg-[#0a0a0a]/40 p-2 text-xs"><p class="text-[#8c8c8c]">Players</p><p class="text-[#ededed]">{selectedMap.players}/{selectedMap.maxPlayers}</p></div>
          <div class="rounded-lg bg-[#0a0a0a]/40 p-2 text-xs"><p class="text-[#8c8c8c]">Assignment</p><p class="text-[#ededed]">{selectedMap.assignment}</p></div>
          <div class="rounded-lg bg-[#0a0a0a]/40 p-2 text-xs"><p class="text-[#8c8c8c]">Unit</p><p class="truncate font-mono text-[#ededed]">{selectedMap.config.systemdUnit}</p></div>
        </div>
      {/if}
    </Card>

    <Card title="Resource & slot checks" icon="✅">
      <ul class="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
        {#each conditions as c (c.label)}
          <li class="flex items-center gap-2 rounded-lg bg-[#0a0a0a]/40 px-3 py-2 text-xs">
            <span class={c.ok ? 'text-[#7c9a82]' : 'text-[#b5544f]'}>{c.ok ? '✓' : '✕'}</span>
            <span class={c.ok ? 'text-[#ededed]' : 'text-[#8c8c8c]'}>{c.label}</span>
          </li>
        {/each}
      </ul>

      <div class="mt-4">
        <SafetyWarningPanel tone={result.tone === 'green' ? 'info' : result.tone === 'red' ? 'danger' : 'warn'} title={result.label}>{result.reason}</SafetyWarningPanel>
      </div>
    </Card>
  </div>

  <div class="space-y-5">
    <Card title="Travel flow" icon="🪜">
      <TravelStepper current={decision?.accepted ? 3 : 2} blocked={!!decision && !decision.accepted} blockedAt={2} />
    </Card>

    <Card title="Recent requests" icon="📥">
      <ul class="space-y-2">
        {#each travel.recent.slice(0, 8) as item, i (i)}
          {@const r = item as Record<string, unknown>}
          <li class="rounded-lg bg-[#0a0a0a]/40 p-2 text-xs">
            <p class="font-mono text-[11px] text-[#8c8c8c]">{String(r.ts ?? r.at ?? 'recent')}</p>
            <p class="text-[#ededed]">{String(r.requestedMap ?? r.map ?? r.target ?? 'travel request')}</p>
            <p class="text-[#8c8c8c]">{String(r.reason ?? r.status ?? '')}</p>
          </li>
        {:else}
          <li class="text-sm text-[#8c8c8c]">No travel decisions recorded.</li>
        {/each}
      </ul>
    </Card>
  </div>
</div>
