<script lang="ts">
  import { onMount } from 'svelte';
  import {
    PageHeader, Card, StatusBadge, ResourceCard, TravelSlotCard, HomeProtectionCard,
    ResourceGovernorCard, ActivityLogItem, TravelStepper, Button, SafetyWarningPanel,
    BackendStatusBanner
  } from '$lib/components';
  import * as mock from '$lib/data/mock';
  import { thresholds } from '$lib/data/mock';
  import { api, loadWithFallback, type ClusterStatus } from '$lib/api';
  import type { ArkMap, Backup, LogEvent, ResourceSample, TravelRequest, Tone } from '$lib/types';

  // All state defaults to mock so the page renders instantly and survives a
  // backend outage; onMount overrides with live data where available.
  let maps = $state<ArkMap[]>(mock.maps);
  let resources = $state<ResourceSample>(mock.resources);
  let recentActivity = $state<LogEvent[]>(mock.recentActivity);
  let backups = $state<Backup[]>(mock.backups);
  let activeTravel = $state<TravelRequest>(mock.activeTravel);
  let status = $state<ClusterStatus | null>(null);
  let fromFallback = $state(false);
  let loadError = $state<string | null>(null);

  onMount(async () => {
    const [srv, rsrc, act, bk, st] = await Promise.all([
      loadWithFallback(() => api.servers(), mock.maps),
      loadWithFallback(() => api.resources(), null),
      loadWithFallback(() => api.activity(), null),
      loadWithFallback(() => api.backups(), null),
      loadWithFallback(() => api.status(), null)
    ]);
    maps = srv.data;
    if (rsrc.data) resources = rsrc.data.sample;
    if (act.data) recentActivity = act.data.recent;
    if (bk.data) backups = bk.data.backups;
    if (st.data) status = st.data;
    // Any failed call means we are (at least partly) on fallback data.
    fromFallback = [srv, rsrc, act, bk, st].some((r) => r.fromFallback);
    loadError = srv.error ?? rsrc.error ?? st.error;
  });

  let homeMap = $derived(maps.find((m) => m.isHome) ?? mock.homeMap);
  let travelA = $derived(maps.find((m) => m.assignment === 'Travel A') ?? null);
  let travelB = $derived(maps.find((m) => m.assignment === 'Travel B') ?? null);

  let ramPct = $derived(Math.round((resources.ramUsedGb / resources.ramTotalGb) * 100));
  let cpuPct = $derived(resources.cpuPct);
  let swapPct = $derived(Math.round((resources.swapUsedGb / resources.swapTotalGb) * 100));
  let diskPct = $derived(Math.round((resources.diskUsedGb / resources.diskTotalGb) * 100));

  let pl = $derived.by((): { label: string; tone: Tone } => {
    if (ramPct >= thresholds.ramEmergencyPct) return { label: 'Critical', tone: 'red' };
    if (ramPct >= thresholds.ramPressurePct) return { label: 'Resource Pressure', tone: 'amber' };
    if (ramPct >= thresholds.ramWarnPct) return { label: 'Warning', tone: 'amber' };
    return { label: 'Healthy', tone: 'green' };
  });

  let totalPlayers = $derived(status?.players ?? maps.reduce((n, m) => n + m.players, 0));
  let runningMaps = $derived(
    status?.runningMaps ??
      maps.filter((m) => m.state === 'Online' || m.state === 'Ready' || m.state === 'Starting').length
  );

  let runningBackup = $derived(backups.find((b) => b.status === 'running'));
  let failedBackup = $derived(backups.find((b) => b.status === 'failed'));
  let lastGood = $derived(backups.find((b) => b.status === 'success'));

  let clusterStatus = $derived([
    { label: 'Rust Manager', value: status?.manager.status ?? 'Online', tone: (status?.manager.tone ?? 'green') as Tone },
    { label: 'systemd Control', value: status?.systemd.status ?? 'Available', tone: (status?.systemd.tone ?? 'green') as Tone },
    { label: 'Tailscale', value: status?.tailscale.status ?? 'Connected', tone: (status?.tailscale.tone ?? 'cyan') as Tone },
    { label: 'Discord Bot', value: status?.discord.status ?? 'Online', tone: (status?.discord.tone ?? 'green') as Tone }
  ]);
</script>

<PageHeader title="Dashboard" icon="🛰️" subtitle="At-a-glance health of the smart ARK cluster">
  {#snippet actions()}
    <Button variant="default" size="sm" href="/resources">Resources</Button>
    <Button variant="primary" size="sm" href="/travel">Start Travel Map</Button>
  {/snippet}
</PageHeader>

{#if fromFallback}<BackendStatusBanner error={loadError} />{/if}

<!-- top status strip -->
<div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-5">
  <div class="card-elevated col-span-2 flex flex-col justify-center p-4 lg:col-span-1">
    <p class="text-xs text-[#8c8c8c]">Cluster state</p>
    <p class="mt-1 flex items-center gap-2 text-lg font-bold"
      class:text-[#7c9a82]={pl.tone === 'green'} class:text-[#bfa15e]={pl.tone === 'amber'} class:text-[#b5544f]={pl.tone === 'red'}>
      {pl.label}
    </p>
    <p class="mt-1 text-[11px] text-[#8c8c8c]">{totalPlayers} players · {runningMaps} maps online</p>
  </div>
  {#each clusterStatus as s (s.label)}
    <div class="card-elevated flex flex-col justify-center p-4">
      <p class="text-xs text-[#8c8c8c]">{s.label}</p>
      <div class="mt-1.5"><StatusBadge label={s.value} tone={s.tone} dot pulse={s.tone !== 'cyan'} /></div>
    </div>
  {/each}
</div>

<!-- system health -->
<Card title="System Health" icon="📊">
  {#snippet actions()}<StatusBadge label={pl.label} tone={pl.tone} dot />{/snippet}
  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
    <ResourceCard label="RAM" icon="🧠" pct={ramPct} detail="{resources.ramUsedGb} / {resources.ramTotalGb} GB" warn={70} danger={88} />
    <ResourceCard label="CPU" icon="⚡" pct={cpuPct} detail="8 cores · load high" warn={75} danger={90} />
    <ResourceCard label="Swap" icon="🔁" pct={swapPct} detail="{resources.swapUsedGb} / {resources.swapTotalGb} GB" warn={30} danger={60} />
    <ResourceCard label="Disk" icon="🗄️" pct={diskPct} detail="{resources.diskUsedGb} / {resources.diskTotalGb} GB" warn={80} danger={92} />
  </div>
  <p class="mt-3 text-xs text-[#8c8c8c]">ARK process memory total: <span class="font-bold text-[#ededed]">{resources.arkProcMemGb} GB</span> across running maps.</p>
</Card>

<!-- running maps -->
<div class="mt-5">
  <h2 class="mb-3 flex items-center gap-2 text-sm font-semibold text-[#8c8c8c]">🗺️ Running Maps & Slots</h2>
  <div class="grid grid-cols-1 gap-3 md:grid-cols-3">
    <TravelSlotCard slot="Home" map={homeMap} />
    <TravelSlotCard slot="Travel A" map={travelA} />
    <TravelSlotCard slot="Travel B" map={travelB} />
  </div>
</div>

<!-- two-col: home protection + governor -->
<div class="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-2">
  <HomeProtectionCard home={homeMap} />
  <ResourceGovernorCard />
</div>

<!-- active travel + backup status -->
<div class="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-2">
  <Card title="Active Travel" icon="🧭">
    {#snippet actions()}<StatusBadge label={activeTravel.result} tone={activeTravel.result === 'Blocked' ? 'red' : 'amber'} dot />{/snippet}
    <div class="mb-3 grid grid-cols-2 gap-2 text-xs">
      <div><p class="text-[#8c8c8c]">Requested map</p><p class="font-medium text-[#ededed]">{activeTravel.map}</p></div>
      <div><p class="text-[#8c8c8c]">Requested by</p><p class="font-medium text-[#ededed]">{activeTravel.requestedBy}</p></div>
      <div><p class="text-[#8c8c8c]">Source</p><p class="font-mono text-[#8aa1ae]">{activeTravel.sourceRaw}</p></div>
      <div><p class="text-[#8c8c8c]">From map</p><p class="font-medium text-[#ededed]">{activeTravel.sourceMap}</p></div>
    </div>
    <SafetyWarningPanel tone="danger" title="Request blocked">{activeTravel.reason}</SafetyWarningPanel>
    <div class="mt-3"><TravelStepper current={activeTravel.step} blocked={true} blockedAt={2} /></div>
  </Card>

  <Card title="Backup Status" icon="💾">
    {#snippet actions()}<Button size="sm" variant="ghost" href="/backups">All backups</Button>{/snippet}
    <div class="space-y-3">
      <div class="rounded-lg border border-[#7c9a82]/30 bg-[#7c9a82]/5 p-3">
        <p class="text-[11px] uppercase text-[#8c8c8c]">Last successful</p>
        <p class="mt-0.5 text-sm font-medium">{lastGood?.map} · {lastGood?.created}</p>
        <p class="text-[11px] text-[#8c8c8c]">reason: {lastGood?.reason}</p>
      </div>
      {#if runningBackup}
        <div class="rounded-lg border border-[#bfa15e]/30 bg-[#bfa15e]/5 p-3">
          <p class="flex items-center gap-2 text-sm font-medium text-[#bfa15e]"><span class="h-2 w-2 rounded-full bg-[#bfa15e] live-dot"></span>Backup running</p>
          <p class="text-[11px] text-[#8c8c8c]">{runningBackup.map} · {runningBackup.progress}% · {runningBackup.reason}</p>
        </div>
      {/if}
      {#if failedBackup}
        <div class="rounded-lg border border-[#b5544f]/30 bg-[#b5544f]/5 p-3">
          <p class="flex items-center gap-2 text-sm font-medium text-[#b5544f]">⛔ Failed backup</p>
          <p class="mt-0.5 text-[11px] text-[#8c8c8c]">{failedBackup.map} · {failedBackup.created}</p>
          <p class="mt-1 font-mono text-[10px] text-[#b5544f]">{failedBackup.error}</p>
        </div>
      {/if}
    </div>
  </Card>
</div>

<!-- recent activity -->
<div class="mt-5">
  <Card title="Recent Activity" icon="📜">
    {#snippet actions()}<Button size="sm" variant="ghost" href="/logs">View all logs</Button>{/snippet}
    <div class="divide-y divide-[#2a2a2a]/40">
      {#each recentActivity as e (e.id)}<ActivityLogItem event={e} />{/each}
    </div>
  </Card>
</div>
