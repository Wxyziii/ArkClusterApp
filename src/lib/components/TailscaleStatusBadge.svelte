<script lang="ts">
  import StatusBadge from './StatusBadge.svelte';
  let { connected = true, compact = false }: { connected?: boolean; compact?: boolean } = $props();
</script>

{#if compact}
  <StatusBadge label={connected ? 'Tailscale' : 'Tailscale off'} tone={connected ? 'cyan' : 'red'} dot pulse={connected} size="sm" />
{:else}
  <span class="inline-flex items-center gap-2 rounded-lg border border-[#8aa1ae]/30 bg-[#8aa1ae]/8 px-2.5 py-1 text-xs text-[#8aa1ae]">
    <span class="h-1.5 w-1.5 rounded-full bg-[#8aa1ae] {connected ? 'live-dot' : ''}"></span>
    {connected ? 'Private · Tailscale' : 'Tailscale disconnected'}
  </span>
{/if}
