<script lang="ts">
  import { onMount } from 'svelte';
  import {
    PageHeader, Card, Button, StatusBadge, ConfirmActionDialog, SafetyWarningPanel,
    RestartRequiredBanner, Select, TextInput, Textarea
  } from '$lib/components';
  import { api, loadWithFallback, type ConfigResponse } from '$lib/api';
  import { configFields, rawGameIni, rawGusIni } from '$lib/data/mock';
  import type { ConfigField } from '$lib/types';

  type Density = 'compact' | 'comfortable';
  type Mode = 'fields' | 'raw' | 'history';
  type ConfigVersion = {
    id?: string;
    ts?: string;
    file?: string;
    reason?: string;
    status?: string;
    actor?: string;
    backupPath?: string;
  };

  let mode = $state<Mode>('fields');
  let density = $state<Density>('compact');
  let data = $state<ConfigResponse | null>(null);
  let fields = $state<ConfigField[]>(structuredClone(configFields));
  let gameIni = $state(rawGameIni);
  let gusIni = $state(rawGusIni);
  let versions = $state<ConfigVersion[]>([]);
  let fromFallback = $state(false);
  let error = $state<string | null>(null);
  let applyOpen = $state(false);
  let pending = $state<{ file: string; key: string; value: string } | null>(null);
  let rawFile = $state<'Game.ini' | 'GameUserSettings.ini'>('GameUserSettings.ini');
  let rawKey = $state('ServerMessage');
  let rawValue = $state('');
  let saving = $state(false);
  let lastResult = $state<string | null>(null);

  let groups = $derived([...new Set(fields.map((f) => f.group))]);
  let writable = $derived(!!data?.writable && !fromFallback);
  let restartNeeded = $derived(!!data?.restartRequired);
  let rowPad = $derived(density === 'compact' ? 'py-1.5' : 'py-2.5');

  onMount(load);

  async function load() {
    const [cfg, hist] = await Promise.all([
      loadWithFallback(() => api.config(), {
        fields: configFields,
        gameIni: rawGameIni,
        gameUserSettingsIni: rawGusIni,
        restartRequired: true,
        writable: false
      }),
      loadWithFallback(() => api.configVersions(), { versions: [] })
    ]);
    data = cfg.data;
    fields = cfg.data.fields ?? [];
    gameIni = cfg.data.gameIni;
    gusIni = cfg.data.gameUserSettingsIni;
    versions = hist.data.versions as ConfigVersion[];
    fromFallback = cfg.fromFallback || hist.fromFallback;
    error = cfg.error ?? hist.error;
  }

  function queueField(field: ConfigField) {
    pending = {
      file: 'GameUserSettings.ini',
      key: field.key,
      value: String(field.value)
    };
    applyOpen = true;
  }

  function fieldToString(field: ConfigField) {
    return String(field.value);
  }

  function updateField(field: ConfigField, value: string) {
    field.value = field.type === 'number' ? Number(value) : field.type === 'bool' ? value === 'true' : value;
    fields = fields;
  }

  function queueRaw() {
    pending = { file: rawFile, key: rawKey.trim(), value: rawValue };
    applyOpen = true;
  }

  async function applyPending() {
    if (!pending) return;
    saving = true;
    error = null;
    try {
      await api.configApply({ ...pending, confirm: true, reason: 'web_ui_config_apply' });
      lastResult = `Applied ${pending.key} to ${pending.file}`;
      applyOpen = false;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'config apply failed';
    } finally {
      saving = false;
    }
  }
</script>

<PageHeader title="Config Editor" icon="⚙️" subtitle="Live shared Game.ini / GameUserSettings.ini editor">
  {#snippet actions()}
    <StatusBadge label={writable ? 'writes enabled' : 'read-only'} tone={writable ? 'green' : 'gray'} dot />
    <Select bind:value={density} options={['compact', 'comfortable']} size="sm" />
    <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
  {/snippet}
</PageHeader>

{#if restartNeeded}<div class="mb-4"><RestartRequiredBanner reason="Some config changes may require ARK restart." /></div>{/if}
{#if fromFallback}<div class="mb-4"><SafetyWarningPanel tone="warn" title="Fallback data">Backend config unavailable: {error}</SafetyWarningPanel></div>{/if}
{#if error && !fromFallback}<div class="mb-4"><SafetyWarningPanel tone="danger" title="Config error">{error}</SafetyWarningPanel></div>{/if}
{#if lastResult}<div class="mb-4"><SafetyWarningPanel tone="info" title="Applied">{lastResult}</SafetyWarningPanel></div>{/if}

<div class="mb-4 card-elevated flex flex-wrap items-center gap-2 p-2">
  {#each [{ id: 'fields', label: 'Compact fields' }, { id: 'raw', label: 'Raw INI' }, { id: 'history', label: 'History' }] as tab}
    <button
      class="rounded-md px-3 py-1.5 text-xs {mode === tab.id ? 'bg-[#222222] text-[#7c9a82]' : 'text-[#8c8c8c] hover:bg-[#181818]'}"
      onclick={() => (mode = tab.id as Mode)}
    >
      {tab.label}
    </button>
  {/each}
  <span class="ml-auto text-[11px] text-[#8c8c8c]">{data?.shared?.sharedConfigDir ?? 'shared config dir unavailable'}</span>
</div>

{#if mode === 'fields'}
  <div class="space-y-4">
    {#each groups as group}
      <Card title={group} icon="🎚️" pad={false}>
        <div class="divide-y divide-[#2a2a2a]/50">
          {#each fields.filter((f) => f.group === group) as field, i (field.key)}
            <div class="grid grid-cols-1 gap-2 px-3 {rowPad} md:grid-cols-[minmax(180px,260px)_1fr_auto] md:items-center">
              <div>
                <p class="text-xs font-medium text-[#ededed]">{field.label}</p>
                <p class="font-mono text-[10px] text-[#8c8c8c]">{field.key}</p>
              </div>
              <div>
                {#if field.type === 'bool'}
                  <select
                    value={fieldToString(field)}
                    onchange={(e) => updateField(field, (e.currentTarget as HTMLSelectElement).value)}
                    class="w-full rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-2.5 py-1 text-xs text-[#ededed] outline-none focus:border-[#3a3a3a]"
                  >
                    <option value="true">true</option>
                    <option value="false">false</option>
                  </select>
                {:else if field.type === 'enum' && field.options}
                  <select
                    value={fieldToString(field)}
                    onchange={(e) => updateField(field, (e.currentTarget as HTMLSelectElement).value)}
                    class="w-full rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-2.5 py-1 text-xs text-[#ededed] outline-none focus:border-[#3a3a3a]"
                  >
                    {#each field.options as opt (opt)}<option value={opt}>{opt}</option>{/each}
                  </select>
                {:else}
                  <input
                    type={field.type === 'number' ? 'number' : 'text'}
                    value={fieldToString(field)}
                    min={field.min}
                    max={field.max}
                    step={field.step}
                    oninput={(e) => updateField(field, (e.currentTarget as HTMLInputElement).value)}
                    class="w-full rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-2.5 py-1 text-xs text-[#ededed] outline-none focus:border-[#3a3a3a]"
                  />
                {/if}
                {#if density === 'comfortable'}<p class="mt-1 text-[11px] text-[#8c8c8c]">{field.hint}</p>{/if}
              </div>
              <Button size="sm" variant="ghost" disabled={!writable} onclick={() => queueField(field)}>Apply</Button>
            </div>
          {/each}
        </div>
      </Card>
    {/each}
  </div>
{:else if mode === 'raw'}
  <div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
    <Card title="GameUserSettings.ini" icon="📄">
      <Textarea value={gusIni} rows={18} disabled mono />
    </Card>
    <Card title="Game.ini" icon="📄">
      <Textarea value={gameIni} rows={18} disabled mono />
    </Card>
  </div>
  <div class="mt-4">
    <Card title="Safe key apply" icon="🛠️">
      <div class="grid grid-cols-1 gap-3 md:grid-cols-[180px_1fr_1fr_auto]">
        <Select bind:value={rawFile} options={['GameUserSettings.ini', 'Game.ini']} />
        <TextInput bind:value={rawKey} placeholder="Key" />
        <TextInput bind:value={rawValue} placeholder="Value" />
        <Button variant="primary" disabled={!writable || !rawKey.trim()} onclick={queueRaw}>Preview/apply</Button>
      </div>
      <p class="mt-2 text-[11px] text-[#8c8c8c]">Password keys are rejected/masked by backend. Full raw-file overwrite is intentionally not exposed.</p>
    </Card>
  </div>
{:else}
  <Card title="Config snapshots" icon="🕓" pad={false}>
    <div class="overflow-x-auto">
      <table class="w-full text-xs">
        <thead><tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]"><th class="p-2">Time</th><th class="p-2">File</th><th class="p-2">Reason</th><th class="p-2">Status</th></tr></thead>
        <tbody class="divide-y divide-[#2a2a2a]/50">
          {#each versions as row}
            <tr><td class="p-2 font-mono">{row.ts ?? '—'}</td><td class="p-2">{row.file ?? '—'}</td><td class="p-2">{row.reason ?? '—'}</td><td class="p-2"><StatusBadge label={row.status ?? 'snapshot'} tone="cyan" size="sm" /></td></tr>
          {:else}
            <tr><td colspan="4" class="p-4 text-center text-[#8c8c8c]">No config snapshots yet.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </Card>
{/if}

<ConfirmActionDialog bind:open={applyOpen} title="Apply config change?" tone="warn" confirmLabel={saving ? 'Applying…' : 'Apply'} onconfirm={applyPending}>
  {#snippet body()}
    <p class="text-sm">Apply <code>{pending?.key}</code> in <strong>{pending?.file}</strong>.</p>
    <SafetyWarningPanel tone="info" title="Backend safety">Backend writes a snapshot and masks secrets. Restart may be needed.</SafetyWarningPanel>
  {/snippet}
</ConfirmActionDialog>
