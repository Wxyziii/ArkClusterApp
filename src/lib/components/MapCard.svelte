<script lang="ts">
  import type { ArkMap } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import Button from './Button.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import { stateTone, rconTone, fmtDuration, fmtMb, barTone } from '$lib/ui';

  let { map, onaction }: { map: ArkMap; onaction?: (action: string, map: ArkMap) => void } = $props();

  // action availability rules
  let isOffline = $derived(map.state === 'Offline' || map.state === 'Resource Standby');
  let ramPct = $derived(map.ramEstimateMb ? Math.round((map.ramMb / map.ramEstimateMb) * 100) : 0);
</script>

<div class="card-elevated flex flex-col p-4 transition-colors hover:border-[#3a3a3a]/50">
  <div class="flex items-start justify-between gap-2">
    <div class="min-w-0">
      <h3 class="flex items-center gap-2 truncate text-sm font-semibold text-[#ededed]">
        {map.name}
        {#if map.isHome}<span class="rounded bg-[#3a3a3a]/30 px-1.5 py-0.5 text-[10px] font-bold text-[#7c9a82]">🛡 HOME</span>{/if}
      </h3>
      <p class="mt-0.5 font-mono text-[11px] text-[#8c8c8c]">{map.assignment} · {map.alias}</p>
    </div>
    <StatusBadge label={map.state} tone={stateTone[map.state]} dot pulse={map.state === 'Online' || map.state === 'Ready'} />
  </div>

  <div class="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 text-xs">
    <div class="flex justify-between"><span class="text-[#8c8c8c]">Players</span><span class="font-medium tabular-nums">{map.players}/{map.maxPlayers}</span></div>
    <div class="flex justify-between"><span class="text-[#8c8c8c]">Uptime</span><span class="tabular-nums">{fmtDuration(map.uptimeMins)}</span></div>
    <div class="flex justify-between"><span class="text-[#8c8c8c]">RAM</span><span class="tabular-nums">{fmtMb(map.ramMb)}</span></div>
    <div class="flex justify-between"><span class="text-[#8c8c8c]">CPU</span><span class="tabular-nums">{map.cpuPct}%</span></div>
  </div>

  {#if map.ramMb > 0}
    <div class="mt-2"><ProgressBar value={ramPct} tone={barTone(ramPct)} height="h-1.5" /></div>
  {/if}

  <div class="mt-3 flex flex-wrap gap-1.5">
    <StatusBadge label="Read-only status" tone="cyan" size="sm" />
    <StatusBadge label="RCON {map.rcon}" tone={rconTone[map.rcon]} size="sm" />
    {#if map.restartRequired}<StatusBadge label="Restart req." tone="amber" size="sm" />{/if}
    {#if map.protected}<StatusBadge label="Protected" tone="accent" size="sm" />{/if}
  </div>

  <p class="mt-3 rounded-md bg-[#0a0a0a]/60 px-2.5 py-1.5 text-[11px] text-[#8c8c8c]">
    <span class="text-[#8aa1ae]">▸ next:</span> {map.nextAction}
  </p>

  <div class="mt-3 flex flex-wrap gap-1.5 border-t border-[#2a2a2a] pt-3">
    <Button size="sm" variant="ghost" href="/maps/{map.id}">Details</Button>
    {#if isOffline}
      <Button size="sm" variant="primary" disabled title="Control disabled in this phase">Start</Button>
    {:else}
      <Button
        size="sm"
        variant="warn"
        disabled
        title="Control disabled in this phase"
      >Restart</Button>
      <Button
        size="sm"
        variant="danger"
        disabled
        title="Control disabled in this phase"
      >Stop</Button>
    {/if}
    <Button size="sm" variant="ghost" disabled title="Backup action disabled in this phase">Backup</Button>
  </div>
</div>
