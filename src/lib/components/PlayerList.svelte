<script lang="ts">
  import type { Player } from '$lib/types';
  import EmptyState from './EmptyState.svelte';
  import { fmtDuration } from '$lib/ui';

  let { players, showMap = true }: { players: Player[]; showMap?: boolean } = $props();
</script>

{#if players.length === 0}
  <EmptyState icon="🧍" title="No players connected" hint="This map has no active survivors right now." />
{:else}
  <div class="overflow-x-auto">
    <table class="w-full text-xs">
      <thead>
        <tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]">
          <th class="px-3 py-2 font-medium">Player</th>
          <th class="px-3 py-2 font-medium">Lvl</th>
          <th class="px-3 py-2 font-medium">Tribe</th>
          <th class="px-3 py-2 font-medium">Connected</th>
          {#if showMap}<th class="px-3 py-2 font-medium">Map</th>{/if}
        </tr>
      </thead>
      <tbody class="divide-y divide-[#2a2a2a]/50">
        {#each players as p (p.name)}
          <tr class="hover:bg-[#181818]/50">
            <td class="px-3 py-2 font-medium text-[#ededed]">{p.name}</td>
            <td class="px-3 py-2 tabular-nums text-[#7c9a82]">{p.level}</td>
            <td class="px-3 py-2 text-[#8c8c8c]">{p.tribe}</td>
            <td class="px-3 py-2 tabular-nums text-[#8c8c8c]">{fmtDuration(p.connectedMins)}</td>
            {#if showMap}<td class="px-3 py-2 text-[#8aa1ae]">{p.map}</td>{/if}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
