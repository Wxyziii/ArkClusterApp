<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader, Card, StatusBadge, Button, SafetyWarningPanel } from '$lib/components';
  import { api, loadWithFallback, type Capabilities, type RuntimeStatus } from '$lib/api';
  import { cluster as mockCluster, maps as mockMaps } from '$lib/data/mock';
  import type { ArkMap, Tone } from '$lib/types';

  type SettingsData = Record<string, any>;

  const fallbackSettings: SettingsData = {
    cluster: mockCluster,
    privateAccess: { bindAddress: '100.68.7.42', port: 8788, bindPrivate: true, note: 'Private/Tailscale/LAN access only.' },
    travelPolicy: { maxTravelServers: 2, slots: [] },
    resourcePolicy: { ramWarnPct: 70, ramPressurePct: 82, ramEmergencyPct: 92, emptyShutdownMins: 180, homeStandbyEnabled: true },
    backupPolicy: { beforeShutdown: true, beforeConfigSave: true, beforeModChange: true, retention: '14 daily' },
    configModPolicy: { configWritable: false, modsMutable: false },
    security: { authScheme: 'Bearer token', tokenMasked: '••••••••', note: 'Token is never returned by the API.' }
  };

  let settings = $state<SettingsData>(fallbackSettings);
  let caps = $state<Capabilities | null>(null);
  let runtime = $state<RuntimeStatus | null>(null);
  let maps = $state<ArkMap[]>(mockMaps);
  let error = $state<string | null>(null);
  let fromFallback = $state(false);

  let capabilityRows = $derived(caps ? Object.entries(caps)
    .filter(([_, v]) => typeof v === 'object' && v && 'enabled' in v)
    .map(([key, value]) => ({ key, value: value as { enabled: boolean; available: boolean; reason: string } })) : []);

  onMount(load);

  async function load() {
    const [settingsRes, capsRes, runtimeRes, mapsRes] = await Promise.all([
      loadWithFallback(() => api.settings(), fallbackSettings),
      loadWithFallback(() => api.capabilities(), null),
      loadWithFallback(() => api.runtime(), null),
      loadWithFallback(() => api.servers(), mockMaps)
    ]);
    settings = settingsRes.data;
    caps = capsRes.data;
    runtime = runtimeRes.data;
    maps = mapsRes.data;
    fromFallback = settingsRes.fromFallback || capsRes.fromFallback || runtimeRes.fromFallback || mapsRes.fromFallback;
    error = settingsRes.error ?? capsRes.error ?? runtimeRes.error ?? mapsRes.error;
  }

  function boolTone(v: boolean): Tone {
    return v ? 'green' : 'gray';
  }
</script>

<PageHeader title="Settings" icon="🔧" subtitle="Live cluster, access, policy, runtime, and safety configuration">
  {#snippet actions()}<Button size="sm" variant="ghost" onclick={load}>Refresh</Button>{/snippet}
</PageHeader>

<div class="mb-5">
  <SafetyWarningPanel tone="danger" title="Private dashboard">
    Bind stays private/Tailscale/LAN only. Do not port-forward this UI publicly. API token remains masked.
  </SafetyWarningPanel>
</div>
{#if fromFallback}<div class="mb-5"><SafetyWarningPanel tone="warn" title="Fallback settings">Backend unavailable: {error}</SafetyWarningPanel></div>{/if}

<div class="grid grid-cols-1 gap-5 xl:grid-cols-3">
  <div class="space-y-5 xl:col-span-2">
    <Card title="Cluster" icon="🛰️">
      <div class="grid grid-cols-1 gap-2 text-xs md:grid-cols-2">
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3"><p class="text-[#8c8c8c]">Name</p><p class="text-[#ededed]">{settings.cluster?.name}</p></div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3"><p class="text-[#8c8c8c]">ID</p><p class="font-mono text-[#ededed]">{settings.cluster?.id}</p></div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3 md:col-span-2"><p class="text-[#8c8c8c]">Directory</p><p class="truncate font-mono text-[#ededed]">{settings.cluster?.directory}</p></div>
      </div>
    </Card>

    <Card title="Private access" icon="🔒">
      <div class="grid grid-cols-1 gap-2 text-xs md:grid-cols-3">
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3"><p class="text-[#8c8c8c]">Bind</p><p class="font-mono text-[#ededed]">{settings.privateAccess?.bindAddress}:{settings.privateAccess?.port}</p></div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3"><p class="text-[#8c8c8c]">Private</p><StatusBadge label={settings.privateAccess?.bindPrivate ? 'yes' : 'check'} tone={settings.privateAccess?.bindPrivate ? 'green' : 'amber'} size="sm" /></div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3"><p class="text-[#8c8c8c]">Auth</p><p class="text-[#ededed]">{settings.security?.authScheme}</p></div>
      </div>
      <p class="mt-3 text-xs text-[#8c8c8c]">{settings.privateAccess?.note}</p>
    </Card>

    <Card title="Maps" icon="🗺️" pad={false}>
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead><tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]"><th class="p-2">Map</th><th class="p-2">Alias</th><th class="p-2">Assignment</th><th class="p-2">State</th><th class="p-2">Unit</th></tr></thead>
          <tbody class="divide-y divide-[#2a2a2a]/50">
            {#each maps as m (m.id)}
              <tr><td class="p-2 font-medium text-[#ededed]">{m.name}</td><td class="p-2 font-mono text-[#8c8c8c]">{m.alias}</td><td class="p-2">{m.assignment}</td><td class="p-2"><StatusBadge label={m.state} tone={m.state === 'Online' ? 'green' : m.state === 'Starting' ? 'amber' : 'gray'} size="sm" /></td><td class="p-2 truncate font-mono text-[#8c8c8c]">{m.config.systemdUnit}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </Card>

    <Card title="Policies" icon="🧠">
      <div class="grid grid-cols-1 gap-2 text-xs md:grid-cols-2">
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3">
          <p class="font-medium text-[#ededed]">Travel</p>
          <p class="text-[#8c8c8c]">Max servers: {settings.travelPolicy?.maxTravelServers}</p>
          <p class="text-[#8c8c8c]">Empty shutdown: {settings.resourcePolicy?.emptyShutdownMins} min</p>
        </div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3">
          <p class="font-medium text-[#ededed]">Resource governor</p>
          <p class="text-[#8c8c8c]">RAM {settings.resourcePolicy?.ramWarnPct}/{settings.resourcePolicy?.ramPressurePct}/{settings.resourcePolicy?.ramEmergencyPct}%</p>
          <p class="text-[#8c8c8c]">Home standby: {settings.resourcePolicy?.homeStandbyEnabled ? 'on' : 'off'}</p>
        </div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3">
          <p class="font-medium text-[#ededed]">Backups</p>
          <p class="text-[#8c8c8c]">Before shutdown: {String(settings.backupPolicy?.beforeShutdown)}</p>
          <p class="text-[#8c8c8c]">Before config/mods: {String(settings.backupPolicy?.beforeConfigSave)} / {String(settings.backupPolicy?.beforeModChange)}</p>
        </div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-3">
          <p class="font-medium text-[#ededed]">Config & mods</p>
          <p class="text-[#8c8c8c]">Config writes: {String(settings.configModPolicy?.configWritable)}</p>
          <p class="text-[#8c8c8c]">Mod mutations: {String(settings.configModPolicy?.modsMutable)}</p>
        </div>
      </div>
    </Card>
  </div>

  <div class="space-y-5">
    <Card title="Runtime" icon="🧰">
      <ul class="space-y-2 text-xs">
        {#each [
          ['SteamCMD', runtime?.steamcmd],
          ['ARK server', runtime?.arkServer],
          ['Shared config', runtime?.sharedConfig],
          ['Cluster dir', runtime?.clusterDir],
          ['Backup root', runtime?.backupRoot]
        ] as row (row[0])}
          {@const item = row[1] as { ok?: boolean; message?: string; path?: string } | undefined}
          <li class="rounded-lg bg-[#0a0a0a]/40 p-2">
            <div class="flex items-center justify-between gap-2"><span>{row[0]}</span><StatusBadge label={item?.ok ? 'ok' : 'check'} tone={boolTone(!!item?.ok)} size="sm" /></div>
            <p class="mt-1 truncate font-mono text-[10px] text-[#8c8c8c]">{item?.path ?? item?.message ?? 'unknown'}</p>
          </li>
        {/each}
      </ul>
    </Card>

    <Card title="Capabilities" icon="✅">
      <ul class="space-y-2 text-xs">
        {#each capabilityRows as row (row.key)}
          <li class="rounded-lg bg-[#0a0a0a]/40 p-2">
            <div class="flex items-center justify-between gap-2"><span class="capitalize">{row.key}</span><StatusBadge label={row.value.enabled ? 'enabled' : 'off'} tone={boolTone(row.value.enabled)} size="sm" /></div>
            <p class="mt-1 text-[#8c8c8c]">{row.value.reason}</p>
          </li>
        {/each}
      </ul>
    </Card>

    <Card title="Security" icon="🛡️">
      <div class="space-y-2 text-xs text-[#8c8c8c]">
        <div class="rounded-lg bg-[#0a0a0a]/40 p-2">Token: <code>{settings.security?.tokenMasked}</code></div>
        <div class="rounded-lg bg-[#0a0a0a]/40 p-2">{settings.security?.note}</div>
      </div>
    </Card>
  </div>
</div>
