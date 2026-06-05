<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader, Card, Button, StatusBadge, SafetyWarningPanel } from '$lib/components';
  import { api, loadWithFallback, type MaintenanceStatus } from '$lib/api';

  type Job = { id?: string; ts?: string; kind?: string; status?: string; reason?: string; detail?: string };

  let status = $state<MaintenanceStatus>({
    enabled: false,
    steamAppId: '376030',
    installPath: '/srv/ark/server',
    safeCommand: 'steamcmd +force_install_dir /srv/ark/server +login anonymous +app_update 376030 validate +quit',
    jobs: []
  });
  let jobs = $derived(status.jobs as Job[]);
  let fromFallback = $state(false);
  let error = $state<string | null>(null);
  let running = $state(false);
  let lastResult = $state<string | null>(null);

  onMount(load);

  async function load() {
    const res = await loadWithFallback(() => api.maintenance(), status);
    status = res.data;
    fromFallback = res.fromFallback;
    error = res.error;
  }

  async function dryRun() {
    running = true;
    error = null;
    lastResult = null;
    try {
      const res = await api.maintenanceDryRun();
      lastResult = `${res.status ?? 'dry run'} · ${res.detail ?? 'ready'}`;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'maintenance dry-run failed';
    } finally {
      running = false;
    }
  }
</script>

<PageHeader title="Maintenance" icon="🛠️" subtitle="ARK server update readiness and maintenance job history">
  {#snippet actions()}
    <StatusBadge label={status.enabled ? 'enabled' : 'disabled'} tone={status.enabled ? 'green' : 'gray'} dot />
    <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
  {/snippet}
</PageHeader>

{#if fromFallback}<div class="mb-4"><SafetyWarningPanel tone="warn" title="Fallback data">Backend unavailable: {error}</SafetyWarningPanel></div>{/if}
{#if error && !fromFallback}<div class="mb-4"><SafetyWarningPanel tone="danger" title="Maintenance error">{error}</SafetyWarningPanel></div>{/if}
{#if lastResult}<div class="mb-4"><SafetyWarningPanel tone="info" title="Dry-run result">{lastResult}</SafetyWarningPanel></div>{/if}

<div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
  <div class="space-y-5 lg:col-span-2">
    <Card title="ARK update command" icon="🧰">
      <div class="space-y-3">
        <div class="grid grid-cols-1 gap-2 text-xs sm:grid-cols-3">
          <div class="rounded-lg bg-[#0a0a0a]/40 p-3">
            <p class="text-[#8c8c8c]">Steam app</p>
            <p class="mt-1 font-mono text-[#ededed]">{status.steamAppId}</p>
          </div>
          <div class="rounded-lg bg-[#0a0a0a]/40 p-3">
            <p class="text-[#8c8c8c]">Install path</p>
            <p class="mt-1 truncate font-mono text-[#ededed]">{status.installPath}</p>
          </div>
          <div class="rounded-lg bg-[#0a0a0a]/40 p-3">
            <p class="text-[#8c8c8c]">Mode</p>
            <p class="mt-1 text-[#ededed]">{status.enabled ? 'dry-run allowed' : 'read-only'}</p>
          </div>
        </div>
        <pre class="overflow-x-auto rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] p-3 text-xs text-[#8aa1ae]">{status.safeCommand}</pre>
        <div class="flex flex-wrap gap-2">
          <Button variant="primary" disabled={!status.enabled || running} onclick={dryRun}>{running ? 'Checking…' : 'Dry-run update'}</Button>
          <Button variant="ghost" href="/backups">Review backups</Button>
        </div>
      </div>
    </Card>

    <Card title="Maintenance jobs" icon="🧾" pad={false}>
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead><tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]"><th class="p-2">Time</th><th class="p-2">Kind</th><th class="p-2">Status</th><th class="p-2">Reason</th><th class="p-2">Detail</th></tr></thead>
          <tbody class="divide-y divide-[#2a2a2a]/50">
            {#each jobs as job (job.id)}
              <tr><td class="p-2 font-mono">{job.ts ?? '—'}</td><td class="p-2">{job.kind ?? 'ark_update'}</td><td class="p-2"><StatusBadge label={job.status ?? 'unknown'} tone="cyan" size="sm" /></td><td class="p-2">{job.reason ?? '—'}</td><td class="p-2 text-[#8c8c8c]">{job.detail ?? '—'}</td></tr>
            {:else}
              <tr><td colspan="5" class="p-4 text-center text-[#8c8c8c]">No maintenance jobs yet.</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </Card>
  </div>

  <Card title="Safety policy" icon="🛡️">
    <ul class="space-y-2 text-xs text-[#8c8c8c]">
      <li class="rounded-lg bg-[#0a0a0a]/40 p-2">Dry-run never starts SteamCMD.</li>
      <li class="rounded-lg bg-[#0a0a0a]/40 p-2">Real updates stay behind backend confirmation and service policy.</li>
      <li class="rounded-lg bg-[#0a0a0a]/40 p-2">Backups should be checked before server update.</li>
    </ul>
  </Card>
</div>
