<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Capabilities } from '$lib/api';
  import type { Backup, LogEvent } from '$lib/types';

  let backups = $state<Backup[]>([]);
  let activity = $state<LogEvent[]>([]);
  let capabilities = $state<Capabilities | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let source = $state('sqlite');
  let filter = $state('All');

  const filters = ['All', 'Systemd', 'Backup', 'Config', 'Mod', 'Travel', 'RCON', 'Discord', 'Errors'];
  const visibleActivity = $derived(
    activity.filter((row) => {
      if (filter === 'All') return true;
      if (filter === 'Errors') return row.severity === 'error' || row.severity === 'warn';
      return row.source === filter;
    })
  );

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      const [bk, act, caps] = await Promise.all([api.backups(), api.activity(), api.capabilities()]);
      backups = bk.backups;
      activity = act.activity;
      source = act.source;
      capabilities = caps;
    } catch (e) {
      error = e instanceof Error ? e.message : 'API request failed';
    } finally {
      loading = false;
    }
  }
</script>

<section class="page">
  <div class="page-head">
    <div>
      <h1>Backups/Logs</h1>
      <p>SQLite backup records and audit/activity rows from the manager.</p>
    </div>
    <div class="toolbar">
      <button class="button" onclick={load} disabled={loading}>{loading ? 'Refreshing' : 'Refresh'}</button>
    </div>
  </div>

  {#if error}<div class="notice error">{error}</div>{/if}

  <div class="grid cols-4">
    <div class="panel"><div class="panel-body metric"><span>Backups</span><strong>{backups.length}</strong><span>sqlite rows</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Activity</span><strong>{activity.length}</strong><span>{source}</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Backup writes</span><strong>{capabilities?.backup.available ? 'Available' : 'Disabled'}</strong><span>{capabilities?.backup.reason ?? 'unknown'}</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Config writes</span><strong>{capabilities?.configWrites.available ? 'Available' : 'Disabled'}</strong><span>{capabilities?.configWrites.reason ?? 'unknown'}</span></div></div>
  </div>

  <div class="panel">
    <div class="panel-head"><h2>Backup Records</h2><span class="chip">{backups.length} rows</span></div>
    <div class="table-wrap">
      <table>
        <thead><tr><th>Created</th><th>Map</th><th>Type</th><th>Status</th><th>Size</th><th>Path</th></tr></thead>
        <tbody>
          {#each backups as backup (backup.id)}
            <tr>
              <td class="mono">{backup.createdAt ?? backup.created}</td>
              <td>{backup.map}</td>
              <td>{backup.type}</td>
              <td><span class="chip {backup.status === 'success' ? 'green' : backup.status === 'failed' ? 'red' : 'amber'}">{backup.status}</span></td>
              <td>{backup.sizeMb} MB</td>
              <td class="mono">{backup.path ?? ''}</td>
            </tr>
          {:else}
            <tr><td colspan="6">No backup rows are present.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  <div class="panel">
    <div class="panel-head">
      <h2>Activity Log</h2>
      <select style="max-width:180px" bind:value={filter}>
        {#each filters as option}<option>{option}</option>{/each}
      </select>
    </div>
    <div class="table-wrap">
      <table>
        <thead><tr><th>Time</th><th>Severity</th><th>Source</th><th>Actor</th><th>Target</th><th>Message</th></tr></thead>
        <tbody>
          {#each visibleActivity as row (row.id)}
            <tr>
              <td class="mono">{row.ts}</td>
              <td><span class="chip {row.severity === 'error' ? 'red' : row.severity === 'warn' ? 'amber' : 'green'}">{row.severity}</span></td>
              <td>{row.source}</td>
              <td>{row.actor}</td>
              <td>{row.targetMap}</td>
              <td><strong>{row.message}</strong><div class="muted">{row.detail}</div></td>
            </tr>
          {:else}
            <tr><td colspan="6">No activity rows match this filter.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</section>
