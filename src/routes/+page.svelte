<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type ClusterStatus, type ResourcesResponse } from '$lib/api';
  import type { ArkMap, Backup, LogEvent } from '$lib/types';

  let status = $state<ClusterStatus | null>(null);
  let maps = $state<ArkMap[]>([]);
  let resources = $state<ResourcesResponse | null>(null);
  let activity = $state<LogEvent[]>([]);
  let backups = $state<Backup[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  const configuredMaps = $derived(maps.filter((m) => m.configured));
  const unavailableMaps = $derived(maps.filter((m) => !m.configured));
  const onlineMaps = $derived(maps.filter((m) => ['Online', 'Ready', 'Starting'].includes(m.state)));

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      const [st, srv, res, act, bk] = await Promise.all([
        api.status(),
        api.servers(),
        api.resources(),
        api.activity(),
        api.backups()
      ]);
      status = st;
      maps = srv;
      resources = res;
      activity = act.recent;
      backups = bk.backups;
    } catch (e) {
      error = e instanceof Error ? e.message : 'API request failed';
    } finally {
      loading = false;
    }
  }

  function cap(value: number | null) {
    return value === null ? 'unknown' : String(value);
  }
</script>

<section class="page">
  <div class="page-head">
    <div>
      <h1>Dashboard</h1>
      <p>Live manager, host, map, backup, and activity state.</p>
    </div>
    <div class="toolbar">
      <button class="button" onclick={load} disabled={loading}>{loading ? 'Refreshing' : 'Refresh'}</button>
      <a class="button primary" href="/servers">Server Manager</a>
    </div>
  </div>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  <div class="grid cols-4">
    <div class="panel"><div class="panel-body metric"><span>Manager</span><strong>{status?.manager.status ?? 'Unavailable'}</strong><span>{status?.dataMode ?? 'live'} mode</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Online maps</span><strong>{onlineMaps.length}</strong><span>{configuredMaps.length} configured</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Players</span><strong>{status?.players ?? 'Unknown'}</strong><span>{status?.playerCountSource ?? 'unavailable'}</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>RAM pressure</span><strong>{status?.resourcePressure.ramPct ?? resources?.derived.ramPct ?? 0}%</strong><span>{status?.resourcePressure.source ?? resources?.source ?? 'unavailable'}</span></div></div>
  </div>

  <div class="grid cols-2">
    <div class="panel">
      <div class="panel-head">
        <h2>Cluster Status</h2>
        <span class="chip {status?.tailscale.bindPrivate ? 'green' : 'red'}">{status?.tailscale.status ?? 'unknown'}</span>
      </div>
      <div class="panel-body grid">
        <div class="grid cols-3">
          <div class="metric"><span>Systemd</span><strong>{status?.systemd.status ?? 'Unknown'}</strong><span>{status?.systemd.source ?? 'systemd'}</span></div>
          <div class="metric"><span>Discord</span><strong>{status?.discord.status ?? 'Unknown'}</strong><span>{status?.discord.source ?? 'manager'}</span></div>
          <div class="metric"><span>Backups</span><strong>{backups.length}</strong><span>sqlite records</span></div>
        </div>
        <div class="notice">Private bind: {status?.tailscale.bindAddress ?? 'unavailable'}. Tailscale connection is not asserted unless a runtime source reports it.</div>
      </div>
    </div>

    <div class="panel">
      <div class="panel-head">
        <h2>Resource Governor</h2>
        <span class="chip cyan">{resources?.governor.decision ?? 'unknown'}</span>
      </div>
      <div class="panel-body">
        <p class="muted">{resources?.governor.why ?? 'Resource policy unavailable until API responds.'}</p>
        <div class="grid cols-3" style="margin-top:12px">
          <div class="metric"><span>CPU</span><strong>{resources?.derived.cpuPct ?? 0}%</strong><span>host sample</span></div>
          <div class="metric"><span>Disk</span><strong>{resources?.derived.diskPct ?? 0}%</strong><span>{resources?.sample.diskFreeGb ?? 0} GB free</span></div>
          <div class="metric"><span>Load</span><strong>{resources?.loadAverage.one ?? 0}</strong><span>1 minute</span></div>
        </div>
      </div>
    </div>
  </div>

  <div class="panel">
    <div class="panel-head">
      <h2>Servers</h2>
      <span class="chip">{unavailableMaps.length} official maps not configured</span>
    </div>
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>Map</th><th>State</th><th>Slot</th><th>Players</th><th>Max players</th><th>Systemd</th></tr>
        </thead>
        <tbody>
          {#each maps.slice(0, 12) as map (map.id)}
            <tr>
              <td><strong>{map.name}</strong><div class="muted mono">{map.config.arkMapName}</div></td>
              <td><span class="chip {map.state === 'Online' ? 'green' : map.state === 'Unavailable' ? 'red' : 'amber'}">{map.state}</span></td>
              <td>{map.slotRole}</td>
              <td>{map.playerCountSource === 'rcon' ? map.players : 'unknown'}</td>
              <td>{cap(map.maxPlayers)}</td>
              <td class="mono">{map.systemd}</td>
            </tr>
          {:else}
            <tr><td colspan="6">No server data returned.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  <div class="grid cols-2">
    <div class="panel">
      <div class="panel-head"><h2>Recent Activity</h2><span class="chip">sqlite</span></div>
      <div class="panel-body grid">
        {#each activity as item (item.id)}
          <div class="notice"><strong>{item.message}</strong><div class="muted">{item.ts} · {item.source} · {item.detail}</div></div>
        {:else}
          <p class="muted">No activity rows are present.</p>
        {/each}
      </div>
    </div>

    <div class="panel">
      <div class="panel-head"><h2>Recent Backups</h2><a class="button" href="/backups">Open</a></div>
      <div class="panel-body grid">
        {#each backups.slice(0, 5) as backup (backup.id)}
          <div class="notice"><strong>{backup.map}</strong><div class="muted">{backup.createdAt ?? backup.created} · {backup.status} · {backup.sizeMb} MB</div></div>
        {:else}
          <p class="muted">No backup rows are present.</p>
        {/each}
      </div>
    </div>
  </div>
</section>
