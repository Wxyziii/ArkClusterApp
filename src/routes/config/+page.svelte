<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type ConfigResponse } from '$lib/api';
  import type { ConfigField } from '$lib/types';

  type Mode = 'fields' | 'raw' | 'history';

  let data = $state<ConfigResponse | null>(null);
  let versions = $state<Record<string, unknown>[]>([]);
  let mode = $state<Mode>('fields');
  let rawFile = $state<'GameUserSettings.ini' | 'Game.ini'>('GameUserSettings.ini');
  let rawKey = $state('');
  let rawValue = $state('');
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let loading = $state(true);
  let saving = $state(false);

  const fields = $derived(data?.fields ?? []);
  const groups = $derived([...new Set(fields.map((f) => f.group))]);
  const writable = $derived(!!data?.writable);

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      const [cfg, hist] = await Promise.all([api.config(), api.configVersions()]);
      data = cfg;
      versions = hist.versions;
      if (!rawKey && cfg.fields[0]) {
        rawKey = cfg.fields[0].key;
        rawValue = String(cfg.fields[0].value);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'API request failed';
    } finally {
      loading = false;
    }
  }

  function updateField(field: ConfigField, value: string) {
    field.value = field.type === 'number' ? Number(value) : field.type === 'bool' ? value === 'true' : value;
    data = data ? { ...data, fields } : data;
  }

  async function apply(file: string, key: string, value: string) {
    saving = true;
    error = null;
    message = null;
    try {
      await api.configApply({ file, key, value, confirm: true, reason: 'web_ui_config_apply' });
      message = `Applied ${key} to ${file}`;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'config apply failed';
    } finally {
      saving = false;
    }
  }

  function fieldFile(field: ConfigField) {
    return field.group.startsWith('Game.ini') ? 'Game.ini' : 'GameUserSettings.ini';
  }
</script>

<section class="page">
  <div class="page-head">
    <div>
      <h1>Config Editor</h1>
      <p>Masked shared Game.ini and GameUserSettings.ini, with key-level writes only.</p>
    </div>
    <div class="toolbar">
      <button class="button" onclick={load} disabled={loading}>{loading ? 'Refreshing' : 'Refresh'}</button>
    </div>
  </div>

  {#if error}<div class="notice error">{error}</div>{/if}
  {#if message}<div class="notice">{message}</div>{/if}
  {#if data?.restartRequired}<div class="notice warn">Config changes may require ARK server restart.</div>{/if}

  <div class="panel">
    <div class="panel-head">
      <h2>Config Source</h2>
      <span class="chip {writable ? 'green' : 'amber'}">{writable ? 'writes enabled' : 'read-only'}</span>
    </div>
    <div class="panel-body grid cols-2">
      <div class="notice">Shared directory: <span class="mono">{data?.shared?.sharedConfigDir ?? 'unavailable'}</span></div>
      <div class="notice">Masked: {data?.shared?.masked ? 'yes' : 'unknown'}</div>
    </div>
  </div>

  <div class="toolbar" style="justify-content:flex-start">
    <button class="button {mode === 'fields' ? 'primary' : ''}" onclick={() => (mode = 'fields')}>Fields</button>
    <button class="button {mode === 'raw' ? 'primary' : ''}" onclick={() => (mode = 'raw')}>Raw INI</button>
    <button class="button {mode === 'history' ? 'primary' : ''}" onclick={() => (mode = 'history')}>History</button>
  </div>

  {#if mode === 'fields'}
    <div class="grid">
      {#each groups as group (group)}
        <div class="panel">
          <div class="panel-head"><h2>{group}</h2><span class="chip">{fields.filter((f) => f.group === group).length} keys</span></div>
          <div class="panel-body grid">
            {#each fields.filter((f) => f.group === group) as field (field.group + field.key)}
              <div class="form-row">
                <div>
                  <strong>{field.label}</strong>
                  <div class="muted mono">{field.key}</div>
                </div>
                {#if field.type === 'bool'}
                  <select value={String(field.value)} onchange={(e) => updateField(field, (e.currentTarget as HTMLSelectElement).value)}>
                    <option value="true">true</option>
                    <option value="false">false</option>
                  </select>
                {:else}
                  <input
                    class="field"
                    type={field.type === 'number' ? 'number' : 'text'}
                    value={String(field.value)}
                    oninput={(e) => updateField(field, (e.currentTarget as HTMLInputElement).value)}
                  />
                {/if}
                <button class="button" disabled={!writable || saving} onclick={() => apply(fieldFile(field), field.key, String(field.value))}>Apply</button>
              </div>
            {:else}
              <p class="muted">No editable keys parsed from this config file.</p>
            {/each}
          </div>
        </div>
      {:else}
        <div class="notice">No config keys were parsed. Use Raw INI to inspect the masked files.</div>
      {/each}
    </div>
  {:else if mode === 'raw'}
    <div class="grid cols-2">
      <div class="panel">
        <div class="panel-head"><h2>GameUserSettings.ini</h2></div>
        <div class="panel-body"><textarea readonly value={data?.gameUserSettingsIni ?? ''}></textarea></div>
      </div>
      <div class="panel">
        <div class="panel-head"><h2>Game.ini</h2></div>
        <div class="panel-body"><textarea readonly value={data?.gameIni ?? ''}></textarea></div>
      </div>
    </div>
    <div class="panel">
      <div class="panel-head"><h2>Key Apply</h2><span class="chip {writable ? 'green' : 'amber'}">{writable ? 'enabled' : 'disabled'}</span></div>
      <div class="panel-body">
        <div class="form-row">
          <select bind:value={rawFile}>
            <option>GameUserSettings.ini</option>
            <option>Game.ini</option>
          </select>
          <input class="field" bind:value={rawKey} placeholder="Key" />
          <button class="button primary" disabled={!writable || saving || !rawKey.trim()} onclick={() => apply(rawFile, rawKey.trim(), rawValue)}>Apply</button>
        </div>
        <div style="margin-top:10px"><input class="field" bind:value={rawValue} placeholder="Value" /></div>
      </div>
    </div>
  {:else}
    <div class="panel">
      <div class="panel-head"><h2>Snapshots</h2><span class="chip">{versions.length} rows</span></div>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Time</th><th>File</th><th>Reason</th><th>Status</th><th>Backup path</th></tr></thead>
          <tbody>
            {#each versions as row}
              <tr>
                <td class="mono">{String(row.ts ?? '')}</td>
                <td>{String(row.file ?? '')}</td>
                <td>{String(row.reason ?? '')}</td>
                <td>{String(row.status ?? '')}</td>
                <td class="mono">{String(row.backupPath ?? '')}</td>
              </tr>
            {:else}
              <tr><td colspan="5">No snapshots returned.</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</section>
