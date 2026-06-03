<script lang="ts">
  import { PageHeader, Card, DiscordStatusCard, StatusBadge, SafetyWarningPanel } from '$lib/components';
  import { discordCommands, discordEvents, alertSettings } from '$lib/data/mock';

  let alerts = $state(structuredClone(alertSettings));
  let everyone = $derived(discordCommands.filter((c) => c.access === 'Everyone'));
  let admin = $derived(discordCommands.filter((c) => c.access === 'Admin'));
</script>

<PageHeader title="Discord Bot" icon="💬" subtitle="Status bot integration — commands, permissions, alerts" />

<div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
  <div class="space-y-5">
    <DiscordStatusCard />

    <Card title="Permission model" icon="🔐">
      <p class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-[#7c9a82]">Everyone</p>
      <div class="mb-3 flex flex-wrap gap-1.5">
        <StatusBadge label="travel" tone="green" size="sm" />
        <StatusBadge label="status" tone="green" size="sm" />
        <StatusBadge label="maps" tone="green" size="sm" />
        <StatusBadge label="players" tone="green" size="sm" />
      </div>
      <p class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-[#bfa15e]">Admin only</p>
      <div class="flex flex-wrap gap-1.5">
        <StatusBadge label="restart" tone="amber" size="sm" />
        <StatusBadge label="stop" tone="amber" size="sm" />
        <StatusBadge label="config edit" tone="amber" size="sm" />
        <StatusBadge label="mod add/remove" tone="amber" size="sm" />
        <StatusBadge label="backup/restore" tone="amber" size="sm" />
      </div>
    </Card>
  </div>

  <div class="space-y-5 lg:col-span-2">
    <Card title="Commands" icon="⌨️">
      <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {#each [...everyone, ...admin] as c (c.cmd)}
          <div class="flex items-center justify-between rounded-lg bg-[#0a0a0a]/40 px-3 py-2">
            <div class="min-w-0">
              <p class="font-mono text-xs text-[#7c9a82]">{c.cmd}</p>
              <p class="text-[11px] text-[#8c8c8c]">{c.desc}</p>
            </div>
            <StatusBadge label={c.access} tone={c.access === 'Admin' ? 'amber' : 'green'} size="sm" />
          </div>
        {/each}
      </div>
    </Card>

    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2">
      <Card title="Recent Discord events" icon="📨">
        <ul class="space-y-2">
          {#each discordEvents as e (e.id)}
            <li class="flex items-start gap-2 text-xs">
              <span class="font-mono text-[11px] text-[#8c8c8c]">{e.ts}</span>
              <StatusBadge label={e.kind} tone={e.kind === 'alert' ? 'amber' : e.kind === 'travel' ? 'cyan' : 'gray'} size="sm" />
              <span class="text-[#ededed]">{e.text}</span>
            </li>
          {/each}
        </ul>
      </Card>

      <Card title="Alert settings" icon="🔔">
        <ul class="space-y-2">
          {#each alerts as a, i (a.key)}
            <li class="flex items-center justify-between text-xs">
              <span class="text-[#8c8c8c]">{a.label}</span>
              <button
                onclick={() => (alerts[i].enabled = !alerts[i].enabled)}
                class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors {a.enabled ? 'bg-[#3a3a3a]' : 'bg-[#2a2a2a]'}"
                role="switch" aria-checked={a.enabled} aria-label="Toggle {a.label} alert"
              >
                <span class="inline-block h-3.5 w-3.5 transform rounded-full bg-[#ededed] transition-transform {a.enabled ? 'translate-x-[1.125rem]' : 'translate-x-1'}"></span>
              </button>
            </li>
          {/each}
        </ul>
      </Card>
    </div>

    <SafetyWarningPanel tone="info" title="Bot relays RCON across all maps">
      The bot announces readiness and alerts from every running map (Home, Travel A, Travel B), because players may be split across maps. Admin commands still require the Discord admin role.
    </SafetyWarningPanel>
  </div>
</div>
