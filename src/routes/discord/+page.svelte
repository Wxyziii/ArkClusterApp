<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader, Card, StatusBadge, Toggle, Button, SafetyWarningPanel } from '$lib/components';
  import { api, loadWithFallback, type DiscordStatusResponse } from '$lib/api';
  import { discordCommands, discordEvents, alertSettings } from '$lib/data/mock';

  const fallback: DiscordStatusResponse = {
    status: {
      online: false,
      guild: '',
      statusChannel: '',
      lastHeartbeat: 'unknown',
      permissionsOk: false,
      implemented: false,
      dashboard: { category: 'ARK Cluster', channels: ['ark-status', 'ark-travel', 'ark-players', 'ark-logs', 'ark-admin'], stateFile: '/var/lib/ark-cluster-discord-bot/state.json' }
    },
    commands: discordCommands,
    events: discordEvents,
    alertSettings
  };

  let data = $state<DiscordStatusResponse>(fallback);
  let alerts = $state(structuredClone(alertSettings));
  let error = $state<string | null>(null);
  let fromFallback = $state(false);

  let service = $derived(data.status.service);
  let channels = $derived(data.status.dashboard?.channels ?? []);
  let commands = $derived(data.commands.map((c) => ({
    cmd: c.cmd ?? c.name ?? 'command',
    desc: c.desc ?? '',
    access: c.access ?? 'User'
  })));

  onMount(load);

  async function load() {
    const res = await loadWithFallback(() => api.discordStatus(), fallback);
    data = res.data;
    alerts = structuredClone(data.alertSettings ?? alertSettings);
    fromFallback = res.fromFallback;
    error = res.error;
  }
</script>

<PageHeader title="Discord Bot" icon="💬" subtitle="Live bot service, dashboard channels, slash commands">
  {#snippet actions()}
    <StatusBadge label={data.status.online ? 'online' : 'offline'} tone={data.status.online ? 'green' : 'red'} dot pulse={data.status.online} />
    <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
  {/snippet}
</PageHeader>

{#if fromFallback}<div class="mb-5"><SafetyWarningPanel tone="warn" title="Fallback Discord data">Backend unavailable: {error}</SafetyWarningPanel></div>{/if}

<div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
  <div class="space-y-5">
    <Card title="Service" icon="🤖">
      <div class="space-y-2 text-xs">
        <div class="flex items-center justify-between rounded-lg bg-[#0a0a0a]/40 p-2"><span class="text-[#8c8c8c]">Gateway</span><StatusBadge label={data.status.online ? 'online' : 'offline'} tone={data.status.online ? 'green' : 'red'} size="sm" /></div>
        <div class="flex items-center justify-between rounded-lg bg-[#0a0a0a]/40 p-2"><span class="text-[#8c8c8c]">systemd</span><StatusBadge label={service?.activeState ?? 'unknown'} tone={service?.active ? 'green' : 'gray'} size="sm" /></div>
        <div class="flex items-center justify-between rounded-lg bg-[#0a0a0a]/40 p-2"><span class="text-[#8c8c8c]">enabled</span><StatusBadge label={service?.enabled ? 'yes' : 'unknown'} tone={service?.enabled ? 'green' : 'gray'} size="sm" /></div>
        <div class="flex items-center justify-between rounded-lg bg-[#0a0a0a]/40 p-2"><span class="text-[#8c8c8c]">permissions</span><StatusBadge label={data.status.permissionsOk ? 'ok' : 'check'} tone={data.status.permissionsOk ? 'green' : 'amber'} size="sm" /></div>
      </div>
      <p class="mt-3 font-mono text-[11px] text-[#8c8c8c]">guild {data.status.guild || 'not configured'}</p>
    </Card>

    <Card title="Dashboard" icon="📌">
      <p class="text-xs text-[#8c8c8c]">Category: <span class="text-[#ededed]">{data.status.dashboard?.category ?? 'ARK Cluster'}</span></p>
      <ul class="mt-3 space-y-1.5">
        {#each channels as channel (channel)}
          <li class="rounded-lg bg-[#0a0a0a]/40 px-2 py-1.5 font-mono text-xs text-[#8aa1ae]">#{channel}</li>
        {/each}
      </ul>
      <p class="mt-3 truncate font-mono text-[10px] text-[#5c5c5c]">{data.status.dashboard?.stateFile}</p>
    </Card>

    <Card title="Alert settings" icon="🔔">
      <ul class="space-y-2.5">
        {#each alerts as a, i (a.key)}
          <li class="flex items-center justify-between text-xs">
            <span class="text-[#8c8c8c]">{a.label}</span>
            <Toggle bind:checked={alerts[i].enabled} label="Toggle {a.label} alert" />
          </li>
        {/each}
      </ul>
    </Card>
  </div>

  <div class="space-y-5 lg:col-span-2">
    <Card title="Commands" icon="⌨️">
      <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {#each commands as c (c.cmd)}
          <div class="flex items-center justify-between gap-2 rounded-lg bg-[#0a0a0a]/40 px-3 py-2">
            <div class="min-w-0">
              <p class="font-mono text-xs text-[#7c9a82]">{c.cmd}</p>
              <p class="text-[11px] text-[#8c8c8c]">{c.desc}</p>
            </div>
            <StatusBadge label={c.access} tone={c.access === 'Admin' ? 'amber' : 'green'} size="sm" />
          </div>
        {/each}
      </div>
    </Card>

    <Card title="Recent Discord events" icon="📨">
      <ul class="space-y-2">
        {#each data.events as e, i (e.id ?? i)}
          <li class="flex flex-wrap items-start gap-2 text-xs">
            <span class="font-mono text-[11px] text-[#8c8c8c]">{e.ts ?? 'recent'}</span>
            <StatusBadge label={e.kind ?? 'event'} tone={e.kind === 'alert' ? 'amber' : e.kind === 'travel' ? 'cyan' : 'gray'} size="sm" />
            <span class="text-[#ededed]">{e.text ?? 'Discord event'}</span>
          </li>
        {/each}
      </ul>
    </Card>
  </div>
</div>
