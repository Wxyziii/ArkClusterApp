<script lang="ts">
  import { onMount } from 'svelte';
  import {
    PageHeader, Card, ModCard, ModTable, Button, StatusBadge, ConfirmActionDialog,
    SafetyWarningPanel, RestartRequiredBanner, TextInput
  } from '$lib/components';
  import { api, loadWithFallback, type ModLookupResponse, type ModsResponse } from '$lib/api';
  import { mods as mockMods } from '$lib/data/mock';
  import type { Mod } from '$lib/types';

  let input = $state('1428596566');
  let data = $state<ModsResponse>({ mods: [], mutable: false, testModId: '1428596566' });
  let mods = $state<Mod[]>([]);
  let lookup = $state<ModLookupResponse | null>(null);
  let confirmOpen = $state(false);
  let pending = $state<{ action: 'add' | 'update' | 'enable' | 'disable' | 'remove'; mod: Mod } | null>(null);
  let error = $state<string | null>(null);
  let fromFallback = $state(false);
  let working = $state(false);
  let lastResult = $state<string | null>(null);

  let active = $derived(mods.filter((m) => m.state === 'active'));
  let disabled = $derived(mods.filter((m) => m.state === 'disabled'));
  let problem = $derived(mods.filter((m) => m.state === 'downloading' || m.state === 'failed' || m.state === 'missing'));
  let restartNeeded = $derived(!!data.restartRequired || problem.length > 0);

  onMount(load);

  async function load() {
    const res = await loadWithFallback(() => api.mods(), {
      mods: mockMods,
      mutable: false,
      restartRequired: true,
      activeModsConfig: '',
      testModId: '1428596566'
    });
    data = res.data;
    mods = data.mods.map(normalizeMod);
    fromFallback = res.fromFallback;
    error = res.error;
  }

  function normalizeMod(raw: unknown, i: number): Mod {
    const r = raw as Record<string, unknown>;
    const workshopId = String(r.workshopId ?? r.id ?? '');
    const enabled = Boolean(Number(r.enabled ?? true));
    const installed = Boolean(Number(r.installed ?? true));
    const state = String(r.state ?? r.status ?? '');
    return {
      id: String(r.id ?? workshopId),
      name: String(r.name ?? `Workshop ${workshopId}`),
      workshopId,
      enabled,
      installed,
      loadOrder: Number(r.loadOrder ?? i + 1),
      lastUpdated: String(r.lastUpdated ?? 'unknown'),
      sizeMb: Number(r.sizeMb ?? 0),
      usedBy: Array.isArray(r.usedBy) ? (r.usedBy as string[]) : [],
      state: state === 'downloading' || state === 'failed' || state === 'missing'
        ? state
        : enabled && installed ? 'active' : installed ? 'disabled' : 'missing',
      progress: Number(r.progress ?? 0) || undefined
    };
  }

  async function findWorkshop() {
    working = true;
    error = null;
    lookup = null;
    try {
      lookup = await api.modLookup({ workshopId: input.trim() });
    } catch (e) {
      error = e instanceof Error ? e.message : 'Workshop lookup failed';
    } finally {
      working = false;
    }
  }

  function queue(action: 'add' | 'update' | 'enable' | 'disable' | 'remove', mod: Mod) {
    pending = { action, mod };
    confirmOpen = true;
  }

  function handle(action: string, mod: Mod) {
    if (action === 'up' || action === 'down') {
      error = 'Reorder endpoint is disabled until backend can safely rewrite ActiveMods.';
      return;
    }
    const next = action === 'retry' ? 'update' : action;
    if (next === 'add' || next === 'update' || next === 'enable' || next === 'disable' || next === 'remove') {
      queue(next, mod);
    }
  }

  async function applyPending() {
    if (!pending) return;
    working = true;
    error = null;
    lastResult = null;
    try {
      await api.modAction(pending.action, { workshopId: pending.mod.workshopId, confirm: true });
      lastResult = `${pending.action} recorded for ${pending.mod.workshopId}`;
      confirmOpen = false;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'mod action failed';
    } finally {
      working = false;
    }
  }

  function queueLookupAdd() {
    if (!lookup) return;
    queue('add', normalizeMod({ workshopId: lookup.workshopId, name: lookup.name, enabled: true, installed: false, status: 'missing' }, 0));
  }
</script>

<PageHeader title="Mods" icon="🧩" subtitle="Live Steam Workshop records, load order, enable/disable policy">
  {#snippet actions()}
    <StatusBadge label={data.mutable ? 'writes enabled' : 'read-only'} tone={data.mutable ? 'green' : 'gray'} dot />
    <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
  {/snippet}
</PageHeader>

{#if restartNeeded}<div class="mb-5"><RestartRequiredBanner reason="Mod changes require map restart after ActiveMods changes." /></div>{/if}
{#if fromFallback}<div class="mb-5"><SafetyWarningPanel tone="warn" title="Fallback mods">Backend unavailable: {error}</SafetyWarningPanel></div>{/if}
{#if error && !fromFallback}<div class="mb-5"><SafetyWarningPanel tone="danger" title="Mod error">{error}</SafetyWarningPanel></div>{/if}
{#if lastResult}<div class="mb-5"><SafetyWarningPanel tone="info" title="Recorded">{lastResult}</SafetyWarningPanel></div>{/if}

<div class="mb-5 grid grid-cols-1 gap-4 lg:grid-cols-[1fr_360px]">
  <Card title="Workshop lookup" icon="🔎">
    <div class="flex flex-wrap gap-2">
      <TextInput bind:value={input} placeholder="Workshop URL or ID" class="min-w-64 flex-1" />
      <Button variant="primary" disabled={!input.trim() || working} onclick={findWorkshop}>{working ? 'Checking…' : 'Lookup'}</Button>
    </div>
    <p class="mt-2 text-[11px] text-[#8c8c8c]">Test mod: {data.testModId ?? '1428596566'}. Lookup parses Steam Workshop URLs and numeric IDs.</p>
    {#if lookup}
      <div class="mt-3 rounded-lg border border-[#2a2a2a] bg-[#0a0a0a]/40 p-3 text-xs">
        <div class="flex flex-wrap items-start justify-between gap-2">
          <div>
            <p class="font-medium text-[#ededed]">{lookup.name}</p>
            <p class="font-mono text-[#8c8c8c]">Workshop {lookup.workshopId}</p>
            <p class="text-[#8c8c8c]">{lookup.game}</p>
          </div>
          <StatusBadge label={lookup.mutable ? 'install allowed' : 'install disabled'} tone={lookup.mutable ? 'green' : 'gray'} size="sm" />
        </div>
        {#if lookup.disabledReason}<p class="mt-2 text-[#bfa15e]">{lookup.disabledReason}</p>{/if}
        <div class="mt-3 flex flex-wrap gap-2">
          <a class="inline-flex items-center rounded-lg border border-[#2a2a2a] px-2.5 py-1 text-xs text-[#8aa1ae] hover:bg-[#181818]" href={lookup.url}>Open Steam</a>
          <Button size="sm" variant="primary" disabled={!data.mutable} onclick={queueLookupAdd}>Add to records</Button>
        </div>
      </div>
    {/if}
  </Card>

  <Card title="Backend policy" icon="🛡️">
    <ul class="space-y-2 text-xs text-[#8c8c8c]">
      <li class="rounded-lg bg-[#0a0a0a]/40 p-2">Active config: {data.activeModsConfig ?? 'not reported'}</li>
      <li class="rounded-lg bg-[#0a0a0a]/40 p-2">SteamCMD required: {data.steamcmdRequired ? 'yes' : 'not reported'}</li>
      <li class="rounded-lg bg-[#0a0a0a]/40 p-2">Mutations: {data.mutable ? 'enabled' : 'disabled by manager config'}</li>
    </ul>
  </Card>
</div>

<div class="space-y-5">
  <Card title="Mods · load order" icon="🔢">
    {#snippet actions()}
      <span class="text-[11px] text-[#8c8c8c]">{active.length} active · {disabled.length} disabled · {problem.length} problems</span>
    {/snippet}
    <ModTable {mods} onaction={handle} />
  </Card>

  <Card title="All known mods" icon="📦">
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {#each mods as mod (mod.id)}<ModCard {mod} onaction={handle} />{:else}<p class="text-sm text-[#8c8c8c]">No mod records yet.</p>{/each}
    </div>
  </Card>
</div>

<ConfirmActionDialog
  bind:open={confirmOpen}
  title="{pending?.action ?? 'change'} {pending?.mod.name ?? 'mod'}?"
  tone={pending?.action === 'remove' ? 'danger' : 'warn'}
  confirmLabel={working ? 'Working…' : 'Confirm'}
  requirePhrase={pending?.action === 'remove' ? 'DELETE' : undefined}
  onconfirm={applyPending}
>
  {#snippet body()}
    <SafetyWarningPanel tone={pending?.action === 'remove' ? 'danger' : 'warn'} title="Backend policy applies">
      {pending?.mod.name} (Workshop {pending?.mod.workshopId}) will be sent to manager as <strong>{pending?.action}</strong>. If mod management is disabled, backend returns a policy error and no files change.
    </SafetyWarningPanel>
  {/snippet}
</ConfirmActionDialog>
