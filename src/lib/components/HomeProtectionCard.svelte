<script lang="ts">
  import type { ArkMap } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import { stateTone } from '$lib/ui';

  let { home }: { home: ArkMap } = $props();
  let inStandby = $derived(home.state === 'Resource Standby');

  const points = [
    { icon: '⭐', text: 'Home is preferred online and starts by default.' },
    { icon: '🛡', text: 'Home is protected from normal travel rotation.' },
    { icon: '💤', text: 'Home can enter Resource Standby only when empty and resources are needed by active travel maps.' },
    { icon: '♻️', text: 'Home auto-restarts when resources recover, when requested, or when travel maps empty.' }
  ];
</script>

<div class="card overflow-hidden">
  <header class="flex items-center justify-between border-b border-[#2a2a2a] bg-[#3a3a3a]/8 px-4 py-3">
    <h2 class="flex items-center gap-2 text-sm font-semibold">🛡 Home Protection Policy</h2>
    <StatusBadge label={home.state} tone={stateTone[home.state]} dot />
  </header>
  <div class="p-4">
    <p class="text-sm font-medium text-[#ededed]">{home.name} <span class="text-[#8c8c8c]">— protected Home map</span></p>

    {#if inStandby}
      <div class="mt-3 rounded-lg border border-[#5c5c5c]/40 bg-[#5c5c5c]/8 p-3">
        <p class="flex items-center gap-2 text-xs font-semibold text-[#8c8c8c]">💤 Currently in Resource Standby</p>
        <p class="mt-1 text-xs text-[#8c8c8c]">Home was empty and travel maps had active players under RAM pressure. It was saved + backed up, then stopped. It will auto-restart when resources recover or Home is requested.</p>
      </div>
    {/if}

    <ul class="mt-3 space-y-2">
      {#each points as p (p.text)}
        <li class="flex gap-2.5 text-xs text-[#8c8c8c]">
          <span class="mt-0.5">{p.icon}</span><span>{p.text}</span>
        </li>
      {/each}
    </ul>
  </div>
</div>
