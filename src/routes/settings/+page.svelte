<script lang="ts">
  import { PageHeader, Card, StatusBadge, Button, SafetyWarningPanel, TailscaleStatusBadge } from '$lib/components';
  import { cluster, maps, thresholds } from '$lib/data/mock';
  import type { Snippet } from 'svelte';

  let open = $state<Record<string, boolean>>({ cluster: true, tailscale: true });
  function toggle(k: string) { open[k] = !open[k]; }
</script>

{#snippet section(id: string, title: string, icon: string, body: Snippet)}
  <div class="card overflow-hidden">
    <button class="flex w-full items-center justify-between px-4 py-3 text-left hover:bg-[#181818]" onclick={() => toggle(id)}>
      <h2 class="flex items-center gap-2 text-sm font-semibold">{icon} {title}</h2>
      <span class="text-xs text-[#5c5c5c]">{open[id] ? '▲' : '▼'}</span>
    </button>
    {#if open[id]}<div class="border-t border-[#2a2a2a] p-4">{@render body()}</div>{/if}
  </div>
{/snippet}

{#snippet row(label: string, body: Snippet)}
  <div class="flex flex-wrap items-center justify-between gap-2 border-b border-[#2a2a2a]/50 py-2.5 last:border-0">
    <span class="text-xs text-[#8c8c8c]">{label}</span>
    <div>{@render body()}</div>
  </div>
{/snippet}

<PageHeader title="Settings" icon="🔧" subtitle="Cluster, access, policy and security configuration (mock)" />

<div class="mb-5">
  <SafetyWarningPanel tone="danger" title="Private dashboard — do not expose publicly">
    This UI is built for private <strong>Tailscale</strong> access only. Never port-forward it or bind to a public address. Secrets are always masked. Dangerous actions require confirmation.
  </SafetyWarningPanel>
</div>

<div class="space-y-4">
  {#snippet clusterBody()}
    {#snippet v1()}<input value={cluster.name} class="rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-2.5 py-1.5 text-xs text-[#ededed]" />{/snippet}
    {@render row('Cluster name', v1)}
    {#snippet v2()}<code class="font-mono text-xs text-[#8aa1ae]">{cluster.id}</code>{/snippet}
    {@render row('Cluster ID', v2)}
    {#snippet v3()}<code class="font-mono text-xs text-[#8c8c8c]">{cluster.directory}</code>{/snippet}
    {@render row('Cluster directory', v3)}
    {#snippet v4()}
      <select class="rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-2.5 py-1.5 text-xs text-[#ededed]">
        {#each maps.filter((m) => m.config.canBeHome) as m (m.id)}<option selected={m.isHome}>{m.name}</option>{/each}
      </select>
    {/snippet}
    {@render row('Home map', v4)}
  {/snippet}
  {@render section('cluster', 'Cluster settings', '🛰️', clusterBody)}

  {#snippet tsBody()}
    {#snippet v1()}<StatusBadge label="Private only" tone="cyan" />{/snippet}
    {@render row('Private access mode', v1)}
    {#snippet v2()}<TailscaleStatusBadge />{/snippet}
    {@render row('Tailscale status', v2)}
    {#snippet v3()}<code class="font-mono text-xs text-[#8c8c8c]">100.84.x.x:8088</code>{/snippet}
    {@render row('Dashboard bind address', v3)}
    <p class="mt-2 rounded-lg bg-[#b5544f]/8 px-3 py-2 text-[11px] text-[#b5544f]">⚠️ Binding to 0.0.0.0 or a public IP is blocked by policy.</p>
  {/snippet}
  {@render section('tailscale', 'Tailscale / private access', '🔒', tsBody)}

  {#snippet mapBody()}
    <div class="overflow-x-auto">
      <table class="w-full text-xs">
        <thead><tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]"><th class="py-2 pr-3 font-medium">Map</th><th class="py-2 pr-3 font-medium">Alias</th><th class="py-2 pr-3 font-medium">Enabled</th><th class="py-2 pr-3 font-medium">Home-capable</th><th class="py-2 font-medium">Travel-capable</th></tr></thead>
        <tbody class="divide-y divide-[#2a2a2a]/50">
          {#each maps as m (m.id)}
            <tr><td class="py-2 pr-3 font-medium text-[#ededed]">{m.name}</td><td class="py-2 pr-3 font-mono text-[#8c8c8c]">{m.alias}</td>
              <td class="py-2 pr-3"><StatusBadge label={m.role === 'Disabled' ? 'No' : 'Yes'} tone={m.role === 'Disabled' ? 'gray' : 'green'} size="sm" /></td>
              <td class="py-2 pr-3">{m.config.canBeHome ? '✓' : '—'}</td><td class="py-2">{m.role === 'Travel-capable' || m.role === 'Home-capable' ? '✓' : '—'}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/snippet}
  {@render section('maps', 'Map configuration', '🗺️', mapBody)}

  {#snippet travelBody()}
    {#snippet v1()}<StatusBadge label={String(thresholds.maxTravel)} tone="cyan" />{/snippet}
    {@render row('Max travel servers', v1)}
    {#snippet v2()}<StatusBadge label="Yes" tone="green" />{/snippet}
    {@render row('Allow everyone to use travel', v2)}
    {#snippet v3()}<span class="text-xs text-[#ededed]">Queue when full</span>{/snippet}
    {@render row('Queue behavior', v3)}
    {#snippet v4()}<span class="text-xs text-[#ededed]">Block + notify in chat/Discord</span>{/snippet}
    {@render row('Blocked behavior', v4)}
    {#snippet v5()}<span class="text-xs text-[#ededed]">{thresholds.emptyShutdownMins} min</span>{/snippet}
    {@render row('Empty shutdown timer', v5)}
  {/snippet}
  {@render section('travel', 'Travel policy', '🧭', travelBody)}

  {#snippet govBody()}
    {#snippet v1()}<span class="font-mono text-xs">{thresholds.ramWarnPct}/{thresholds.ramPressurePct}/{thresholds.ramEmergencyPct}%</span>{/snippet}
    {@render row('RAM thresholds (warn/pressure/emergency)', v1)}
    {#snippet on()}<StatusBadge label="Enabled" tone="green" size="sm" />{/snippet}
    {@render row('Home Resource Standby', on)}
    {@render row('Never stop maps with players', on)}
    {@render row('Prefer active-player maps', on)}
    {@render row('Auto-restart Home when recovered', on)}
  {/snippet}
  {@render section('governor', 'Resource governor policy', '🧠', govBody)}

  {#snippet backupBody()}
    {#snippet on()}<StatusBadge label="On" tone="green" size="sm" />{/snippet}
    {@render row('Backup before shutdown', on)}
    {@render row('Backup before config changes', on)}
    {@render row('Backup before mod changes', on)}
    {#snippet ret()}<span class="text-xs text-[#ededed]">14 daily · 6 weekly · prune 60d</span>{/snippet}
    {@render row('Retention', ret)}
  {/snippet}
  {@render section('backup', 'Backup policy', '💾', backupBody)}

  {#snippet cfgBody()}
    {#snippet on()}<StatusBadge label="On" tone="green" size="sm" />{/snippet}
    {@render row('Safe mode (form editor)', on)}
    {@render row('Raw editor enabled', on)}
    {@render row('Require backup before save', on)}
    {@render row('Restart warnings', on)}
  {/snippet}
  {@render section('configpolicy', 'Config editor policy', '⚙️', cfgBody)}

  {#snippet modBody()}
    {#snippet on()}<StatusBadge label="Allowed" tone="green" size="sm" />{/snippet}
    {@render row('Allow add / download', on)}
    {@render row('Allow disable', on)}
    {#snippet confirm()}<StatusBadge label="Requires confirmation" tone="amber" size="sm" />{/snippet}
    {@render row('Allow remove (delete files)', confirm)}
    {#snippet onw()}<StatusBadge label="On" tone="green" size="sm" />{/snippet}
    {@render row('Restart-required warnings', onw)}
  {/snippet}
  {@render section('modpolicy', 'Mod policy', '🧩', modBody)}

  {#snippet secBody()}
    {#snippet v1()}<span class="text-xs text-[#ededed]">Marcel (admin)</span>{/snippet}
    {@render row('Admin user', v1)}
    {#snippet v2()}<code class="font-mono text-xs text-[#8c8c8c]">••••••••••••••••</code>{/snippet}
    {@render row('API token', v2)}
    {#snippet v3()}<code class="font-mono text-xs text-[#8c8c8c]">@ark-admin</code>{/snippet}
    {@render row('Discord admin role', v3)}
    {#snippet v4()}<StatusBadge label="Enabled" tone="green" size="sm" />{/snippet}
    {@render row('Audit logging', v4)}
    <div class="mt-3 flex gap-2"><Button size="sm" variant="ghost">Rotate token</Button><Button size="sm" variant="ghost">Reveal once</Button></div>
  {/snippet}
  {@render section('security', 'Access / security (placeholder)', '🛡️', secBody)}
</div>
