<script lang="ts">
  import { discord } from '$lib/data/mock';
  import StatusBadge from './StatusBadge.svelte';

  let { showHeartbeat = true }: { showHeartbeat?: boolean } = $props();
</script>

<div class="card">
  <header class="flex items-center justify-between border-b border-[#2a2a2a] px-4 py-3">
    <h2 class="flex items-center gap-2 text-sm font-semibold">💬 Discord Bot</h2>
    <StatusBadge label={discord.online ? 'Online' : 'Offline'} tone={discord.online ? 'green' : 'red'} dot pulse={discord.online} />
  </header>
  <dl class="divide-y divide-[#2a2a2a]/60 text-xs">
    <div class="flex justify-between px-4 py-2.5"><dt class="text-[#8c8c8c]">Guild</dt><dd class="font-medium">{discord.guild}</dd></div>
    <div class="flex justify-between px-4 py-2.5"><dt class="text-[#8c8c8c]">Status channel</dt><dd class="font-mono text-[#8aa1ae]">{discord.statusChannel}</dd></div>
    <div class="flex justify-between px-4 py-2.5"><dt class="text-[#8c8c8c]">Permissions</dt><dd><StatusBadge label={discord.permissionsOk ? 'OK' : 'Missing'} tone={discord.permissionsOk ? 'green' : 'red'} size="sm" /></dd></div>
    {#if showHeartbeat}
      <div class="flex justify-between px-4 py-2.5"><dt class="text-[#8c8c8c]">Last heartbeat</dt><dd class="font-mono text-[#8c8c8c]">{discord.lastHeartbeat}</dd></div>
    {/if}
  </dl>
</div>
