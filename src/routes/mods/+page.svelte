<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type ModLookupResponse, type ModsResponse } from '$lib/api';
  import type { Mod } from '$lib/types';

  let input = $state('1428596566');
  let data = $state<ModsResponse | null>(null);
  let mods = $state<Mod[]>([]);
  let lookup = $state<ModLookupResponse | null>(null);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let loading = $state(true);
  let working = $state(false);

  const active = $derived(mods.filter((m) => m.state === 'active'));
  const missing = $derived(mods.filter((m) => m.state === 'missing' || m.state === 'failed'));

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      data = await api.mods();
      mods = data.mods.map(normalizeMod);
    } catch (e) {
      error = e instanceof Error ? e.message : 'API request failed';
    } finally {
      loading = false;
    }
  }

  function normalizeMod(raw: unknown, i: number): Mod {
    const r = raw as Record<string, unknown>;
    const workshopId = String(r.workshopId ?? r.id ?? '');
    const enabled = Boolean(Number(r.enabled ?? true));
    const installed = Boolean(Number(r.installed ?? false));
    const status = String(r.state ?? r.status ?? '');
    return {
      id: String(r.id ?? workshopId),
      name: String(r.name ?? '') || `Workshop ID ${workshopId}`,
      workshopId,
      enabled,
      installed,
      loadOrder: Number(r.loadOrder ?? i + 1),
      lastUpdated: String(r.lastUpdated ?? 'unknown'),
      sizeMb: Number(r.sizeMb ?? 0),
      usedBy: Array.isArray(r.usedBy) ? (r.usedBy as string[]) : [],
      state: status === 'failed' || status === 'missing' || status === 'downloading'
        ? status
        : enabled && installed ? 'active' : installed ? 'disabled' : 'missing'
    };
  }

  async function findWorkshop() {
    working = true;
    error = null;
    message = null;
    lookup = null;
    try {
      lookup = await api.modLookup({ workshopId: input.trim() });
    } catch (e) {
      error = e instanceof Error ? e.message : 'Workshop lookup failed';
    } finally {
      working = false;
    }
  }

  async function modAction(action: 'add' | 'update' | 'enable' | 'disable' | 'remove', workshopId: string) {
    working = true;
    error = null;
    message = null;
    try {
      await api.modAction(action, { workshopId, confirm: true });
      message = `${action} recorded for ${workshopId}`;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'mod action failed';
    } finally {
      working = false;
    }
  }
</script>

<section class="page">
  <div class="page-head">
    <div>
      <h1>Mods Manager</h1>
      <p>Live mod records and ActiveMods policy. No Steam metadata is guessed.</p>
    </div>
    <div class="toolbar">
      <button class="button" onclick={load} disabled={loading}>{loading ? 'Refreshing' : 'Refresh'}</button>
    </div>
  </div>

  {#if error}<div class="notice error">{error}</div>{/if}
  {#if message}<div class="notice">{message}</div>{/if}

  <div class="grid cols-4">
    <div class="panel"><div class="panel-body metric"><span>Known mods</span><strong>{mods.length}</strong><span>sqlite/config records</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Active</span><strong>{active.length}</strong><span>enabled and installed</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Missing/problems</span><strong>{missing.length}</strong><span>needs admin attention</span></div></div>
    <div class="panel"><div class="panel-body metric"><span>Mutations</span><strong>{data?.mutable ? 'Enabled' : 'Disabled'}</strong><span>manager policy</span></div></div>
  </div>

  <div class="grid cols-2">
    <div class="panel">
      <div class="panel-head"><h2>Workshop Lookup</h2><span class="chip">{lookup?.metadataSource ?? 'not queried'}</span></div>
      <div class="panel-body grid">
        <div class="form-row" style="grid-template-columns: 1fr auto">
          <input class="field" bind:value={input} placeholder="Workshop URL or ID" />
          <button class="button primary" onclick={findWorkshop} disabled={!input.trim() || working}>Lookup</button>
        </div>
        {#if lookup}
          <div class="notice">
            <strong>{lookup.name ?? `Workshop ID ${lookup.workshopId}`}</strong>
            <div class="muted mono">{lookup.url}</div>
            <div class="muted">{lookup.reason ?? (lookup.metadataAvailable ? 'metadata available' : 'metadata unavailable')}</div>
            <div class="toolbar" style="justify-content:flex-start;margin-top:10px">
              <a class="button" href={lookup.url}>Open Steam</a>
              <button class="button primary" disabled={!data?.mutable || working} onclick={() => modAction('add', lookup!.workshopId)}>Add record</button>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="panel">
      <div class="panel-head"><h2>Backend Policy</h2><span class="chip {data?.mutable ? 'green' : 'amber'}">{data?.mutable ? 'write enabled' : 'read-only'}</span></div>
      <div class="panel-body grid">
        <div class="notice">Active config: <span class="mono">{data?.activeModsConfig ?? 'not reported'}</span></div>
        <div class="notice">SteamCMD required: {data?.steamcmdRequired === undefined ? 'not reported' : data.steamcmdRequired ? 'yes' : 'no'}</div>
        <div class="notice">Restart required: {data?.restartRequired ? 'yes' : 'not reported'}</div>
      </div>
    </div>
  </div>

  <div class="panel">
    <div class="panel-head"><h2>Known Mods</h2><span class="chip">{mods.length} rows</span></div>
    <div class="table-wrap">
      <table>
        <thead><tr><th>Order</th><th>Workshop</th><th>Name</th><th>State</th><th>Installed</th><th>Actions</th></tr></thead>
        <tbody>
          {#each mods as mod (mod.id)}
            <tr>
              <td>{mod.loadOrder}</td>
              <td class="mono">{mod.workshopId}</td>
              <td>{mod.name}</td>
              <td><span class="chip {mod.state === 'active' ? 'green' : mod.state === 'missing' || mod.state === 'failed' ? 'red' : 'amber'}">{mod.state}</span></td>
              <td>{mod.installed ? 'yes' : 'no'}</td>
              <td>
                <div class="toolbar" style="justify-content:flex-start">
                  <button class="button" disabled={!data?.mutable || working} onclick={() => modAction('update', mod.workshopId)}>Update</button>
                  <button class="button" disabled={!data?.mutable || working} onclick={() => modAction(mod.enabled ? 'disable' : 'enable', mod.workshopId)}>{mod.enabled ? 'Disable' : 'Enable'}</button>
                  <button class="button" disabled={!data?.mutable || working} onclick={() => modAction('remove', mod.workshopId)}>Remove</button>
                </div>
              </td>
            </tr>
          {:else}
            <tr><td colspan="6">No mod records returned.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</section>
