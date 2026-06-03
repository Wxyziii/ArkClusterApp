<script lang="ts">
  import { PageHeader, Card, StatusBadge, TravelStepper, Button, SafetyWarningPanel, TravelSlotCard } from '$lib/components';
  import { maps, travelA, travelB, homeMap, ramPct, thresholds } from '$lib/data/mock';
  import type { ArkMap } from '$lib/types';

  let destinations = $derived(maps.filter((m) => m.role === 'Travel-capable' || m.role === 'Home-capable'));
  let selected = $state<ArkMap | null>(maps.find((m) => m.alias === 'extinction') ?? null);

  // evaluate mock travel conditions for selected map
  let bothSlotsBusy = $derived(!!travelA?.players && !!travelB?.players);
  let slotFree = $derived(!travelA || !travelB || (travelA?.players === 0) || (travelB?.players === 0));
  let ramPressure = $derived(ramPct >= thresholds.ramPressurePct);
  let homeCanStandby = $derived(homeMap.players === 0 && homeMap.state !== 'Resource Standby');

  let result = $derived.by(() => {
    if (!selected) return { kind: 'none', label: '—', tone: 'gray' as const, reason: 'Select a destination map.' };
    if (selected.state === 'Online' || selected.state === 'Ready')
      return { kind: 'online', label: 'Already online', tone: 'green' as const, reason: `${selected.name} is already running — travel directly.` };
    if (selected.state === 'Starting')
      return { kind: 'starting', label: 'Starting', tone: 'amber' as const, reason: `${selected.name} is booting. RCON not ready yet.` };
    if (bothSlotsBusy)
      return { kind: 'blocked', label: 'Blocked — slots full', tone: 'red' as const, reason: 'Both travel slots have active players. Maps with players are never auto-stopped. Request will be queued.' };
    if (ramPressure && !homeCanStandby)
      return { kind: 'blocked', label: 'Blocked — RAM pressure', tone: 'red' as const, reason: `RAM at ${ramPct}% (≥ ${thresholds.ramPressurePct}% pressure). No safe way to free memory right now.` };
    if (ramPressure && homeCanStandby)
      return { kind: 'standby', label: 'Home Standby offered', tone: 'amber' as const, reason: 'RAM pressure is high but Home is empty. Home can enter Resource Standby to free memory, then this map can start.' };
    return { kind: 'ready', label: 'Ready to start', tone: 'green' as const, reason: `Resources OK and a travel slot is free. ${selected.name} can start now.` };
  });

  const sources = [
    { icon: '🎮', label: 'In-game chat', cmd: '!travel aberration', note: 'Anyone on the server' },
    { icon: '💬', label: 'Discord command', cmd: '/travel aberration', note: 'Anyone in the guild' },
    { icon: '🖥️', label: 'Web UI', cmd: 'manual request', note: 'This panel' }
  ];

  const conditions = $derived([
    { label: 'Map enabled', ok: selected?.role !== 'Disabled' },
    { label: 'Not already online', ok: selected?.state === 'Offline' },
    { label: 'Travel Slot A free', ok: !travelA || travelA.players === 0 },
    { label: 'Travel Slot B free', ok: !travelB || travelB.players === 0 },
    { label: 'Under max travel servers (2)', ok: slotFree },
    { label: 'RAM under pressure threshold', ok: !ramPressure },
    { label: 'CPU headroom', ok: true },
    { label: 'Disk space available', ok: true },
    { label: 'No backup in progress', ok: false }
  ]);
</script>

<PageHeader title="Travel" icon="🧭" subtitle="On-demand travel servers — start a map when resources allow">
  {#snippet actions()}<StatusBadge label={result.label} tone={result.tone} dot /> {/snippet}
</PageHeader>

<!-- slot overview -->
<div class="mb-5 grid grid-cols-1 gap-3 md:grid-cols-3">
  <TravelSlotCard slot="Home" map={homeMap} />
  <TravelSlotCard slot="Travel A" map={travelA} />
  <TravelSlotCard slot="Travel B" map={travelB} />
</div>

<div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
  <!-- left: destination + conditions + result -->
  <div class="space-y-5 lg:col-span-2">
    <Card title="Choose destination" icon="🗺️">
      <div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
        {#each destinations as d (d.id)}
          <button
            onclick={() => (selected = d)}
            class="rounded-lg border p-3 text-left transition-colors {selected?.id === d.id ? 'border-[#7c9a82] bg-[#222222]' : 'border-[#2a2a2a] bg-[#0a0a0a]/40 hover:bg-[#181818]'}"
          >
            <p class="text-sm font-medium text-[#ededed]">{d.name}</p>
            <p class="font-mono text-[10px] text-[#8c8c8c]">!travel {d.alias}</p>
            <div class="mt-1.5"><StatusBadge label={d.state} tone={d.state === 'Online' ? 'green' : d.state === 'Offline' ? 'gray' : 'amber'} size="sm" /></div>
          </button>
        {/each}
      </div>
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
        {#if result.kind === 'blocked'}
          <SafetyWarningPanel tone="danger" title={result.label}>{result.reason}</SafetyWarningPanel>
        {:else if result.kind === 'standby'}
          <SafetyWarningPanel tone="warn" title="Home can enter Resource Standby">{result.reason}</SafetyWarningPanel>
        {:else if result.kind === 'ready'}
          <SafetyWarningPanel tone="info" title="Ready to start">{result.reason}</SafetyWarningPanel>
        {:else}
          <SafetyWarningPanel tone="info" title={result.label}>{result.reason}</SafetyWarningPanel>
        {/if}
      </div>

      <div class="mt-4 flex flex-wrap gap-2">
        <Button variant="primary" disabled={result.kind === 'blocked' || result.kind === 'none'}>
          {result.kind === 'standby' ? 'Standby Home & start' : 'Start travel map'}
        </Button>
        {#if result.kind === 'blocked'}<Button variant="warn">Queue request</Button>{/if}
        <Button variant="ghost">Cancel</Button>
      </div>
    </Card>
  </div>

  <!-- right: stepper + sources + policy -->
  <div class="space-y-5">
    <Card title="Travel flow" icon="🪜">
      <TravelStepper current={result.kind === 'online' ? 6 : result.kind === 'ready' ? 1 : 2} blocked={result.kind === 'blocked'} blockedAt={2} />
    </Card>

    <Card title="Request sources" icon="📥">
      <ul class="space-y-2">
        {#each sources as s (s.label)}
          <li class="rounded-lg bg-[#0a0a0a]/40 p-2.5">
            <p class="flex items-center gap-2 text-xs font-medium">{s.icon} {s.label}</p>
            <p class="mt-1 font-mono text-[11px] text-[#7c9a82]">{s.cmd}</p>
            <p class="text-[10px] text-[#8c8c8c]">{s.note}</p>
          </li>
        {/each}
      </ul>
    </Card>

    <Card title="How travel works" icon="ℹ️">
      <ul class="space-y-2 text-xs text-[#8c8c8c]">
        <li class="flex gap-2"><span class="text-[#7c9a82]">●</span>Everyone can use <code class="text-[#7c9a82]">!travel</code> — no admin needed.</li>
        <li class="flex gap-2"><span class="text-[#bfa15e]">●</span>Stop / restart / config / mod actions stay <strong>admin-only</strong>.</li>
        <li class="flex gap-2"><span class="text-[#8aa1ae]">●</span>The manager listens to <strong>every running map's</strong> RCON chat — players may be split across Home, Travel A and Travel B.</li>
      </ul>
    </Card>
  </div>
</div>
