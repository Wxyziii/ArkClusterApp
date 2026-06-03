<script lang="ts">
  import type { Mod } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import Button from './Button.svelte';
  import type { Tone } from '$lib/types';

  let { mods, onaction }: { mods: Mod[]; onaction?: (a: string, m: Mod) => void } = $props();

  const stateMeta: Record<string, { tone: Tone; label: string }> = {
    active: { tone: 'green', label: 'Active' },
    disabled: { tone: 'gray', label: 'Disabled' },
    downloading: { tone: 'amber', label: 'Downloading' },
    failed: { tone: 'red', label: 'Failed' },
    missing: { tone: 'red', label: 'Missing' }
  };
  let active = $derived(mods.filter((m) => m.enabled && m.installed).sort((a, b) => a.loadOrder - b.loadOrder));
</script>

<div class="overflow-x-auto">
  <table class="w-full text-xs">
    <thead>
      <tr class="border-b border-[#2a2a2a] text-left text-[#8c8c8c]">
        <th class="px-3 py-2 font-medium">Order</th>
        <th class="px-3 py-2 font-medium">Mod</th>
        <th class="px-3 py-2 font-medium">Workshop ID</th>
        <th class="px-3 py-2 font-medium">State</th>
        <th class="px-3 py-2 font-medium">Used by</th>
        <th class="px-3 py-2 text-right font-medium">Reorder</th>
      </tr>
    </thead>
    <tbody class="divide-y divide-[#2a2a2a]/50">
      {#each active as mod, i (mod.id)}
        <tr class="hover:bg-[#181818]/50">
          <td class="px-3 py-2 font-mono text-[#7c9a82]">{mod.loadOrder}</td>
          <td class="px-3 py-2 font-medium text-[#ededed]">{mod.name}</td>
          <td class="px-3 py-2 font-mono text-[11px] text-[#8c8c8c]">{mod.workshopId}</td>
          <td class="px-3 py-2"><StatusBadge label={stateMeta[mod.state].label} tone={stateMeta[mod.state].tone} size="sm" /></td>
          <td class="px-3 py-2 text-[#8c8c8c]">{mod.usedBy.length}</td>
          <td class="px-3 py-2">
            <div class="flex justify-end gap-1">
              <Button size="sm" variant="ghost" disabled={i === 0} onclick={() => onaction?.('up', mod)} title="Move up">▲</Button>
              <Button size="sm" variant="ghost" disabled={i === active.length - 1} onclick={() => onaction?.('down', mod)} title="Move down">▼</Button>
            </div>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="mt-2 px-3 text-[11px] text-[#8c8c8c]">Load order matters — later mods override earlier ones. Reordering requires a map restart.</p>
