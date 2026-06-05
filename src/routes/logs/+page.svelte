<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader, Card, ActivityLogItem, StatusBadge, EmptyState } from '$lib/components';
  import { activityLog as mockActivity } from '$lib/data/mock';
  import { api, loadWithFallback } from '$lib/api';
  import type { LogEvent } from '$lib/types';

  const filters = ['All', 'Map', 'Travel', 'Governor', 'RCON', 'Discord', 'Config', 'Mod', 'Backup', 'User', 'Errors'];
  let active = $state('All');
  let activityLog = $state<LogEvent[]>(mockActivity);
  let source = $state('mock');

  onMount(async () => {
    const res = await loadWithFallback(() => api.activity(), { activity: mockActivity, recent: mockActivity.slice(0, 5) });
    activityLog = res.data.activity;
    source = res.fromFallback ? 'mock' : 'sqlite';
  });

  let shown = $derived.by(() => {
    if (active === 'All') return activityLog;
    if (active === 'Errors') return activityLog.filter((l) => l.severity === 'error');
    return activityLog.filter((l) => l.source === active);
  });
</script>

<PageHeader title="Activity / Logs" icon="📜" subtitle="Cluster audit timeline — every automated and admin action">
  {#snippet actions()}
    <StatusBadge label="{activityLog.length} events" tone="cyan" />
    <StatusBadge label={source} tone={source === 'sqlite' ? 'green' : 'gray'} />
  {/snippet}
</PageHeader>

<div class="mb-4 flex flex-wrap gap-1.5">
  {#each filters as f (f)}
    <button
      class="rounded-full border px-3 py-1 text-xs transition-colors {active === f ? 'border-[#7c9a82] bg-[#222222] text-[#7c9a82]' : 'border-[#2a2a2a] text-[#8c8c8c] hover:bg-[#181818]'}"
      onclick={() => (active = f)}
    >{f}</button>
  {/each}
</div>

<Card pad={false}>
  {#if shown.length}
    <div class="divide-y divide-[#2a2a2a]/40 p-2">
      {#each shown as e (e.id)}<ActivityLogItem event={e} />{/each}
    </div>
  {:else}
    <div class="p-6"><EmptyState icon="📭" title="No events for this filter" hint="Try a different category." /></div>
  {/if}
</Card>
