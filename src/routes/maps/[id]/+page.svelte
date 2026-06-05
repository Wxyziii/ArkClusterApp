<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import {
    PageHeader, Card, StatusBadge, MapStateTimeline, PlayerList, Button,
    RestartRequiredBanner, ResourceCard, ActivityLogItem, EmptyState, PolicyCard,
    BackendStatusBanner, SafetyWarningPanel
  } from '$lib/components';
  import { maps, players, activityLog } from '$lib/data/mock';
  import { api, loadWithFallback, type Capabilities } from '$lib/api';
  import type { ArkMap, Player } from '$lib/types';
  import { stateTone, rconTone, systemdTone, fmtDuration, fmtMb } from '$lib/ui';

  let map = $state<ArkMap | undefined>(maps.find((m) => m.id === $page.params.id));
  let loadedPlayers = $state<Player[]>(players.filter((p) => p.map === map?.name));
  let capabilities = $state<Capabilities | null>(null);
  let actionMessage = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let fromFallback = $state(false);
  let loadError = $state<string | null>(null);

  onMount(async () => {
    const id = $page.params.id ?? '';
    const fallbackMap = maps.find((m) => m.id === id);
    if (!fallbackMap) return;
    const [res, caps] = await Promise.all([
      loadWithFallback(() => api.server(id), {
        server: fallbackMap,
        players: players.filter((p) => p.map === fallbackMap.name)
      }),
      loadWithFallback(() => api.capabilities(), null)
    ]);
    map = res.data.server;
    loadedPlayers = res.data.players;
    if (caps.data) capabilities = caps.data;
    fromFallback = res.fromFallback || caps.fromFallback;
    loadError = res.error ?? caps.error;
  });

  let mapPlayers = $derived(loadedPlayers);
  let mapLogs = $derived(map ? activityLog.filter((l) => l.targetMap === map!.name).slice(0, 6) : []);

  const allSteps = [
    'Requested', 'Starting systemd unit', 'Waiting for RCON', 'Ready for transfer',
    'Monitoring players', 'Saving', 'Backing up', 'Stopping', 'Offline / Resource Standby'
  ];
  // map current state -> timeline index
  function stepFor(state: string): number {
    const m: Record<string, number> = {
      Offline: 8, 'Resource Standby': 8, Starting: 1, Online: 4, Ready: 3,
      Draining: 4, Saving: 5, 'Backing Up': 6, Stopping: 7, Error: 1
    };
    return m[state] ?? 0;
  }

  async function runAction(action: 'start' | 'stop' | 'restart' | 'backup') {
    if (!map) return;
    if (map.isHome && action === 'stop') {
      const phrase = window.prompt('Type STOP HOME to request protected Home stop');
      if (phrase !== 'STOP HOME') return;
    } else if (action !== 'backup' && !window.confirm(`Request ${action} for ${map.name}?`)) {
      return;
    } else if (action === 'backup' && !window.confirm(`Create backup for ${map.name}?`)) {
      return;
    }
    actionMessage = null;
    actionError = null;
    try {
      const result = await api.serverAction(map.id, action, {
        confirm: true,
        strongConfirm: map.isHome && (action === 'stop' || action === 'restart'),
        adminOverride: false,
        reason: map.isHome && action === 'stop' ? 'manual_admin_override' : 'manual_admin_action'
      });
      actionMessage = result.message;
      const refreshed = await api.server(map.id);
      map = refreshed.server;
      loadedPlayers = refreshed.players;
    } catch (e) {
      actionError = e instanceof Error ? e.message : 'action failed';
    }
  }
</script>

{#if !map}
  <EmptyState icon="🚫" title="Map not found" hint="No configured map with that id." />
  <div class="mt-4"><Button href="/maps" variant="default">← Back to Maps</Button></div>
{:else}
  <div class="mb-4"><a href="/maps" class="text-xs text-[#8c8c8c] hover:text-[#7c9a82]">← Maps</a></div>
  {#if fromFallback}<BackendStatusBanner error={loadError} />{/if}
  <div class="mb-5">
    <SafetyWarningPanel tone="warn" title="Read-only status">
      Control disabled in this phase. Unit state below is read from systemd when available; no ARK control or RCON commands are sent.
    </SafetyWarningPanel>
  </div>
  {#if actionMessage}
    <div class="mb-5"><SafetyWarningPanel tone="info" title="Action complete">{actionMessage}</SafetyWarningPanel></div>
  {/if}
  {#if actionError}
    <div class="mb-5"><SafetyWarningPanel tone="danger" title="Action blocked or failed">{actionError}</SafetyWarningPanel></div>
  {/if}

  <!-- status header -->
  <div class="card mb-5 overflow-hidden">
    <div class="flex flex-wrap items-center justify-between gap-4 border-b border-[#2a2a2a] p-4">
      <div>
        <h1 class="flex items-center gap-2 text-xl font-bold">
          {map.name}
          {#if map.isHome}<StatusBadge label="🛡 Protected Home" tone="accent" />{/if}
        </h1>
        <p class="mt-1 font-mono text-xs text-[#8c8c8c]">{map.config.arkMapName} · {map.assignment} · {map.role}</p>
      </div>
      <div class="flex flex-wrap items-center gap-2">
      <StatusBadge label={map.state} tone={stateTone[map.state]} dot pulse={map.state === 'Online'} />
        <StatusBadge label="RCON {map.rcon}" tone={rconTone[map.rcon]} dot />
        <StatusBadge label={map.systemd} tone={systemdTone(map.systemd)} />
        {#if map.restartRequired}<StatusBadge label="Restart required" tone="amber" />{/if}
      </div>
    </div>
    <div class="grid grid-cols-2 divide-x divide-[#2a2a2a] text-center sm:grid-cols-4">
      <div class="p-3"><p class="text-[10px] uppercase text-[#8c8c8c]">Players</p><p class="text-lg font-bold tabular-nums">{map.players}/{map.maxPlayers}</p></div>
      <div class="p-3"><p class="text-[10px] uppercase text-[#8c8c8c]">Uptime</p><p class="text-lg font-bold tabular-nums">{fmtDuration(map.uptimeMins)}</p></div>
      <div class="p-3"><p class="text-[10px] uppercase text-[#8c8c8c]">Memory</p><p class="text-lg font-bold tabular-nums">{fmtMb(map.ramMb)}</p></div>
      <div class="p-3"><p class="text-[10px] uppercase text-[#8c8c8c]">Idle</p><p class="text-lg font-bold tabular-nums">{fmtDuration(map.idleMins)}</p></div>
    </div>
  </div>

  {#if map.restartRequired}
    <div class="mb-5"><RestartRequiredBanner reason="GameUserSettings.ini changed on {map.name}." /></div>
  {/if}

  <div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
    <!-- left: timeline + players + logs -->
    <div class="space-y-5 lg:col-span-2">
      <Card title="State Timeline" icon="⏱️">
        <MapStateTimeline steps={allSteps} current={stepFor(map.state)} />
      </Card>

      <Card title="Players ({mapPlayers.length})" icon="🧍">
        <PlayerList players={mapPlayers} showMap={false} />
      </Card>

      <Card title="Map Logs" icon="📜">
        {#if mapLogs.length}
          <div class="divide-y divide-[#2a2a2a]/40">{#each mapLogs as e (e.id)}<ActivityLogItem event={e} />{/each}</div>
        {:else}
          <EmptyState icon="📜" title="No recent events for this map" />
        {/if}
      </Card>
    </div>

    <!-- right: resources, save/backup, config -->
    <div class="space-y-5">
      <Card title="Resource Usage" icon="📊">
        <div class="space-y-3">
          <ResourceCard label="RAM" icon="🧠" pct={map.ramEstimateMb ? Math.round((map.ramMb / map.ramEstimateMb) * 100) : 0} detail="{fmtMb(map.ramMb)} / est {fmtMb(map.ramEstimateMb)}" />
          <ResourceCard label="CPU" icon="⚡" pct={map.cpuPct} detail="process load" />
          <div class="grid grid-cols-2 gap-2 text-xs">
            <div class="rounded-lg bg-[#0a0a0a]/60 p-2"><p class="text-[#8c8c8c]">Save size</p><p class="font-bold">{fmtMb(map.saveSizeMb)}</p></div>
            <div class="rounded-lg bg-[#0a0a0a]/60 p-2"><p class="text-[#8c8c8c]">Uptime</p><p class="font-bold">{fmtDuration(map.uptimeMins)}</p></div>
          </div>
        </div>
      </Card>

      <Card title="Save / Backup" icon="💾">
        <dl class="space-y-2 text-xs">
          <div class="flex justify-between"><dt class="text-[#8c8c8c]">Last backup</dt><dd class="font-mono">{map.lastBackup}</dd></div>
          <div class="flex justify-between"><dt class="text-[#8c8c8c]">Backup size</dt><dd>{fmtMb(map.saveSizeMb)}</dd></div>
          <div class="flex justify-between"><dt class="text-[#8c8c8c]">Reason</dt><dd>{map.isHome ? 'resource standby' : 'scheduled'}</dd></div>
        </dl>
        <div class="mt-3 flex gap-2">
          <Button size="sm" variant="ghost" disabled={!capabilities?.backup.enabled} title={capabilities?.backup.reason ?? 'Backup capability unavailable'} onclick={() => runAction('backup')}>Backup now</Button>
          <Button size="sm" variant="ghost" href="/backups">History</Button>
        </div>
      </Card>

      <Card title="Guarded Actions" icon="🛡️">
        <div class="flex flex-wrap gap-2">
          <Button size="sm" variant="primary" disabled={!capabilities?.systemdControl.enabled} title={capabilities?.systemdControl.reason ?? 'Systemd control unavailable'} onclick={() => runAction('start')}>Start</Button>
          <Button size="sm" variant="warn" disabled={!capabilities?.systemdControl.enabled} title={capabilities?.systemdControl.reason ?? 'Systemd control unavailable'} onclick={() => runAction('restart')}>Restart</Button>
          <Button size="sm" variant="danger" disabled={!capabilities?.systemdControl.enabled} title={capabilities?.systemdControl.reason ?? 'Systemd control unavailable'} onclick={() => runAction('stop')}>Stop</Button>
        </div>
        <p class="mt-2 text-xs text-[#8c8c8c]">Actions are sent only for configured units and are blocked by backend guard rules.</p>
      </Card>

      <PolicyCard title="systemd & config" icon="⚙️" rows={[
        { label: 'Unit', value: map.systemdDetail?.unit ?? map.config.systemdUnit },
        { label: 'Active / sub state', value: `${map.systemdDetail?.activeState ?? 'unknown'} · ${map.systemdDetail?.subState ?? 'unknown'}` },
        { label: 'Main PID', value: map.systemdDetail?.mainPid ? String(map.systemdDetail.mainPid) : '—' },
        { label: 'Since', value: map.systemdDetail?.since ?? '—' },
        { label: 'ARK map', value: map.config.arkMapName },
        { label: 'Ports (query/rcon/game)', value: `${map.config.queryPort} · ${map.config.rconPort} · ${map.config.gamePort}` },
        { label: 'Slot priority', value: String(map.config.slotPriority) },
        { label: 'Auto-shutdown', value: map.config.autoShutdownEnabled },
        { label: 'Can be Home', value: map.config.canBeHome },
        { label: 'Auto-stop when empty', value: map.config.canAutoStopWhenEmpty },
        { label: 'Can enter Standby', value: map.config.canEnterStandby }
      ]} />

      <Card title="Mods ({map.config.modList.length})" icon="🧩">
        <ul class="space-y-1.5 text-xs">
          {#each map.config.modList as mod (mod)}
            <li class="flex items-center gap-2 text-[#8c8c8c]"><span class="text-[#7c9a82]">●</span>{mod}</li>
          {/each}
        </ul>
        <div class="mt-3"><Button size="sm" variant="ghost" href="/mods">Manage mods</Button></div>
      </Card>
    </div>
  </div>
{/if}
