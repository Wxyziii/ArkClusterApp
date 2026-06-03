<script lang="ts">
  import type { ArkMap } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import { stateTone, fmtDuration, fmtMb } from '$lib/ui';

  let {
    slot,
    map
  }: { slot: 'Home' | 'Travel A' | 'Travel B'; map: ArkMap | null } = $props();

  let isHome = $derived(slot === 'Home');
</script>

<div class="card-elevated p-4 {isHome ? 'border-l-2 border-l-[#3a3a3a]' : ''}">
  <div class="flex items-center justify-between">
    <span class="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wide text-[#8c8c8c]">
      {#if isHome}🛡{/if}{slot}
    </span>
    {#if map}
      <StatusBadge label={map.state} tone={stateTone[map.state]} size="sm" dot pulse={map.state === 'Online'} />
    {:else}
      <StatusBadge label="Empty slot" tone="gray" size="sm" />
    {/if}
  </div>

  {#if map}
    <p class="mt-2 text-sm font-semibold text-[#ededed]">{map.name}</p>
    <div class="mt-2 space-y-1 text-xs text-[#8c8c8c]">
      <div class="flex justify-between"><span>Players</span><span class="tabular-nums text-[#ededed]">{map.players}/{map.maxPlayers}</span></div>
      <div class="flex justify-between"><span>Uptime</span><span class="tabular-nums">{fmtDuration(map.uptimeMins)}</span></div>
      <div class="flex justify-between"><span>Memory</span><span class="tabular-nums">{fmtMb(map.ramMb)}</span></div>
    </div>
    <p class="mt-2 border-t border-[#2a2a2a] pt-2 text-[11px] text-[#8c8c8c]">{map.nextAction}</p>
  {:else}
    <p class="mt-3 text-sm text-[#5c5c5c]">No travel server running</p>
    <p class="mt-1 text-[11px] text-[#5c5c5c]">Available — start with <code class="text-[#7c9a82]">!travel mapname</code></p>
  {/if}
</div>
