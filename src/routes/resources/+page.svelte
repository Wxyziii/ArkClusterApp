<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader, Card, ResourceCard, StatusBadge, PolicyCard, ResourceGovernorCard, SafetyWarningPanel } from '$lib/components';
  import BackendStatusBanner from '$lib/components/BackendStatusBanner.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import * as mock from '$lib/data/mock';
  import { api, loadWithFallback, type ResourcesResponse } from '$lib/api';
  import { fmtMb } from '$lib/ui';
  import type { ArkMap, ResourceSample, Tone } from '$lib/types';

  let resources = $state<ResourceSample>(mock.resources);
  let maps = $state<ArkMap[]>(mock.maps);
  let response = $state<ResourcesResponse | null>(null);
  let fromFallback = $state(false);
  let loadError = $state<string | null>(null);
  let loading = $state(true);

  onMount(async () => {
    const [rsrc, srv] = await Promise.all([
      loadWithFallback(() => api.resources(), null),
      loadWithFallback(() => api.servers(), mock.maps)
    ]);
    if (rsrc.data) {
      response = rsrc.data;
      resources = rsrc.data.sample;
    }
    maps = srv.data;
    fromFallback = rsrc.fromFallback || srv.fromFallback;
    loadError = rsrc.error ?? srv.error;
    loading = false;
  });

  let ramPct = $derived(response?.derived.ramPct ?? Math.round((resources.ramUsedGb / resources.ramTotalGb) * 100));
  let cpuPct = $derived(response?.derived.cpuPct ?? resources.cpuPct);
  let swapPct = $derived(response?.derived.swapPct ?? Math.round((resources.swapUsedGb / resources.swapTotalGb) * 100));
  let diskPct = $derived(response?.derived.diskPct ?? Math.round((resources.diskUsedGb / resources.diskTotalGb) * 100));
  let thresholds = $derived(response?.thresholds ?? mock.thresholds);
  let governor = $derived(response?.governor ?? mock.governor);
  let pl = $derived.by((): { label: string; tone: Tone } => {
    if (response?.derived.pressure) return response.derived.pressure as { label: string; tone: Tone };
    if (ramPct >= mock.thresholds.ramEmergencyPct) return { label: 'Critical', tone: 'red' };
    if (ramPct >= mock.thresholds.ramPressurePct) return { label: 'Resource Pressure', tone: 'amber' };
    if (ramPct >= mock.thresholds.ramWarnPct) return { label: 'Warning', tone: 'amber' };
    return { label: 'Healthy', tone: 'green' };
  });
  let running = $derived(maps.filter((m) => m.ramMb > 0));
  let runningMaps = $derived(maps.filter((m) => m.state === 'Online' || m.state === 'Ready' || m.state === 'Starting').length);
</script>

<PageHeader title="Resources" icon="📊" subtitle="Live system load and the resource governor policy">
  {#snippet actions()}<StatusBadge label={pl.label} tone={pl.tone} dot pulse={pl.tone !== 'green'} />{/snippet}
</PageHeader>

{#if !loading || fromFallback}
  <BackendStatusBanner
    error={loadError}
    connected={!fromFallback && !!response}
    dataSource={resources.source}
    systemdStatus={null}
  />
{/if}

{#if pl.tone !== 'green'}
  <div class="mb-5">
    <SafetyWarningPanel tone={pl.tone === 'red' ? 'danger' : 'warn'} title="{pl.label}: RAM at {ramPct}%">
      The resource governor may place Home in Resource Standby if it is empty, to protect active travel-map players. Maps with players are never stopped.
    </SafetyWarningPanel>
  </div>
{/if}

<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
  <ResourceCard label="RAM" icon="🧠" pct={ramPct} detail="{resources.ramUsedGb} / {resources.ramTotalGb} GB" warn={thresholds.ramWarnPct} danger={thresholds.ramEmergencyPct} />
  <ResourceCard label="CPU" icon="⚡" pct={cpuPct} detail="load {resources.load1} / {resources.load5} / {resources.load15}" warn={75} danger={90} />
  <ResourceCard label="Swap" icon="🔁" pct={swapPct} detail="{resources.swapUsedGb} / {resources.swapTotalGb} GB" warn={30} danger={60} />
  <ResourceCard label="Disk" icon="🗄️" pct={diskPct} detail="{resources.diskFreeGb} GB free" warn={80} danger={92} />
</div>

<div class="mt-5">
  <Card title="Per-ARK-process memory" icon="🧩">
    {#if running.length === 0}
      <p class="text-xs text-[#8c8c8c]">No ARK processes currently running.</p>
    {:else}
      <div class="space-y-3">
        {#each running as m (m.id)}
          <div>
            <div class="mb-1 flex items-center justify-between text-xs">
              <span class="font-medium">{m.name} <span class="text-[#8c8c8c]">· {m.assignment}</span></span>
              <span class="tabular-nums text-[#8c8c8c]">{fmtMb(m.ramMb)}</span>
            </div>
            <ProgressBar value={Math.round((m.ramMb / (resources.ramTotalGb * 1024)) * 100)} tone="accent" height="h-1.5" />
          </div>
        {/each}
      </div>
      <p class="mt-3 border-t border-[#2a2a2a] pt-2 text-xs text-[#8c8c8c]">ARK total: <span class="font-bold text-[#ededed]">{resources.arkProcMemGb} GB</span> · {runningMaps} maps online · source {resources.source}</p>
    {/if}
  </Card>
</div>

<div class="mt-5 grid grid-cols-1 gap-5 lg:grid-cols-3">
  <div class="lg:col-span-2"><ResourceGovernorCard /></div>
  <PolicyCard title="Governor policy" icon="🛡️" rows={[
    { label: 'Warning RAM', value: `${thresholds.ramWarnPct}%` },
    { label: 'Pressure RAM', value: `${thresholds.ramPressurePct}%` },
    { label: 'Emergency RAM', value: `${thresholds.ramEmergencyPct}%` },
    { label: 'Max travel servers', value: String(thresholds.maxTravel) },
    { label: 'Empty travel shutdown', value: `${thresholds.emptyShutdownMins} min` },
    { label: 'Never stop maps w/ players', value: governor.policy.neverStopWithPlayers },
    { label: 'Home Resource Standby', value: governor.policy.homeStandbyEnabled },
    { label: 'Home stops only when empty', value: governor.policy.homeStopsOnlyWhenEmpty },
    { label: 'Prefer active-player maps', value: governor.policy.preferActivePlayerMaps },
    { label: 'Auto-restart Home', value: governor.policy.autoRestartHome }
  ]} />
</div>
