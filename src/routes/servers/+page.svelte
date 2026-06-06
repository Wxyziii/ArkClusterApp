<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Capabilities, type TravelState } from '$lib/api';
  import type { ArkMap } from '$lib/types';

  let maps = $state<ArkMap[]>([]);
  let capabilities = $state<Capabilities | null>(null);
  let travel = $state<TravelState | null>(null);
  let selected = $state<ArkMap | null>(null);
  let actionReason = $state('web_ui_admin_action');
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let loading = $state(true);
  let working = $state(false);

  const configured = $derived(maps.filter((m) => m.configured));
  const officialMissing = $derived(maps.filter((m) => !m.configured));
  const destinations = $derived(maps.filter((m) => m.configured && !m.isHome));

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      const [srv, caps, tr] = await Promise.all([api.servers(), api.capabilities(), api.travel()]);
      maps = srv;
      capabilities = caps;
      travel = tr;
      selected = selected ? (srv.find((m) => m.id === selected?.id) ?? null) : (srv.find((m) => m.configured) ?? null);
    } catch (e) {
      error = e instanceof Error ? e.message : 'API request failed';
    } finally {
      loading = false;
    }
  }

  function selectMap(map: ArkMap) {
    selected = map;
    message = null;
  }

  async function runAction(action: 'start' | 'stop' | 'restart' | 'backup') {
    if (!selected) return;
    working = true;
    error = null;
    message = null;
    try {
      const res = await api.serverAction(selected.id, action, {
        confirm: true,
        strongConfirm: action === 'stop' && selected.isHome,
        reason: actionReason
      });
      message = `${res.operation} ${res.result}: ${res.message}`;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'action failed';
    } finally {
      working = false;
    }
  }

  async function requestTravel(map: ArkMap) {
    working = true;
    error = null;
    message = null;
    try {
      const decision = await api.travelRequest({ map: map.id, source: 'web', actor: 'web-ui' });
      message = `${decision.status}: ${decision.reason}`;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'travel request failed';
    } finally {
      working = false;
    }
  }

  function players(map: ArkMap) {
    return map.playerCountSource === 'rcon' ? String(map.players) : 'unknown';
  }
</script>

<section class="page">
  <div class="page-head">
    <div>
      <h1>Server Manager</h1>
      <p>Configured slots, official maps, systemd state, RCON availability, and guarded actions.</p>
    </div>
    <div class="toolbar">
      <button class="button" onclick={load} disabled={loading}>{loading ? 'Refreshing' : 'Refresh'}</button>
    </div>
  </div>

  {#if error}<div class="notice error">{error}</div>{/if}
  {#if message}<div class="notice">{message}</div>{/if}

  <div class="grid cols-4">
    <div class="panel"><div class="panel-body metric"><span>Configured</span><strong>{configured.length}</strong><span>cluster maps</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Official missing</span><strong>{officialMissing.length}</strong><span>shown as unavailable</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Travel scheduler</span><strong>{travel?.enabled ? 'On' : 'Off'}</strong><span>{travel?.maxTravelServers ?? 0} max on-demand</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Systemd control</span><strong>{capabilities?.systemdControl.available ? 'Available' : 'Disabled'}</strong><span>{capabilities?.systemdControl.reason ?? 'unknown'}</span></div></div>
  </div>

  <div class="grid cols-2">
    <div class="panel">
      <div class="panel-head"><h2>Maps</h2><span class="chip">{maps.length} official/configured rows</span></div>
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>Map</th><th>State</th><th>Slot</th><th>Players</th><th>Max</th><th>RCON</th></tr>
          </thead>
          <tbody>
            {#each maps as map (map.id)}
              <tr onclick={() => selectMap(map)} style="cursor:pointer">
                <td><strong>{map.name}</strong><div class="muted mono">{map.id}</div></td>
                <td><span class="chip {map.state === 'Online' ? 'green' : map.configured ? 'amber' : 'red'}">{map.state}</span></td>
                <td>{map.slotRole}</td>
                <td>{players(map)}</td>
                <td>{map.maxPlayers ?? 'unknown'}</td>
                <td>{map.rcon}</td>
              </tr>
            {:else}
              <tr><td colspan="6">No server rows returned.</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>

    <div class="panel">
      <div class="panel-head"><h2>Selected Server</h2><span class="chip">{selected?.state ?? 'none'}</span></div>
      <div class="panel-body grid">
        {#if selected}
          <div class="grid cols-2">
            <div class="metric"><span>Map</span><strong>{selected.name}</strong><span>{selected.config.arkMapName}</span></div>
            <div class="metric"><span>Systemd</span><strong>{selected.systemd}</strong><span>{selected.config.systemdUnit || 'not configured'}</span></div>
            <div class="metric"><span>Players</span><strong>{players(selected)}</strong><span>{selected.playerCountSource}</span></div>
            <div class="metric"><span>Max players</span><strong>{selected.maxPlayers ?? 'Unknown'}</strong><span>{selected.maxPlayersSource}</span></div>
          </div>
          <label class="muted" for="reason">Action reason</label>
          <input id="reason" class="field" bind:value={actionReason} />
          <div class="toolbar" style="justify-content:flex-start">
            <button class="button primary" disabled={!selected.configured || working || !capabilities?.systemdControl.available} onclick={() => runAction('start')}>Start</button>
            <button class="button" disabled={!selected.configured || working || !capabilities?.systemdControl.available} onclick={() => runAction('restart')}>Restart</button>
            <button class="button" disabled={!selected.configured || working || !capabilities?.systemdControl.available} onclick={() => runAction('stop')}>Stop</button>
            <button class="button" disabled={!selected.configured || working || !capabilities?.backup.available} onclick={() => runAction('backup')}>Backup</button>
          </div>
          <div class="notice">Actions are sent to the backend guard. Player counts are unknown unless RCON is connected, so active on-demand slots are treated conservatively.</div>
        {:else}
          <p class="muted">Select a configured map.</p>
        {/if}
      </div>
    </div>
  </div>

  <div class="panel">
    <div class="panel-head"><h2>On-Demand Requests</h2><span class="chip">{travel?.enabled ? 'enabled' : 'disabled'}</span></div>
    <div class="panel-body grid">
      {#if travel?.blockReason}<div class="notice warn">{travel.blockReason}</div>{/if}
      <div class="table-wrap">
        <table>
          <thead><tr><th>Destination</th><th>State</th><th>Slot</th><th>Action</th></tr></thead>
          <tbody>
            {#each destinations as map (map.id)}
              <tr>
                <td><strong>{map.name}</strong><div class="muted">{map.alias}</div></td>
                <td>{map.state}</td>
                <td>{map.slotRole}</td>
                <td><button class="button" disabled={!travel?.enabled || working} onclick={() => requestTravel(map)}>Request</button></td>
              </tr>
            {:else}
              <tr><td colspan="4">No configured destinations.</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  </div>
</section>
