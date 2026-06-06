<script lang="ts">
  import type { Capabilities } from '$lib/api';

  let {
    error = null,
    connected = false,
    dataSource = null,
    systemdStatus = null,
    capabilities = null
  }: {
    error?: string | null;
    connected?: boolean;
    dataSource?: string | null;
    systemdStatus?: string | null;
    capabilities?: Capabilities | null;
  } = $props();
</script>

<div
  class="mb-4 flex flex-wrap items-center gap-2 rounded-lg border px-3 py-2 text-xs {connected
    ? 'border-[#7c9a82]/40 bg-[#7c9a82]/8 text-[#7c9a82]'
    : 'border-[#bfa15e]/40 bg-[#bfa15e]/8 text-[#bfa15e]'}"
  role="status"
>
  <span class="h-2 w-2 rounded-full {connected ? 'bg-[#7c9a82]' : 'bg-[#bfa15e]'}"></span>
  {#if connected}
    <span class="font-medium">Backend connected.</span>
    <span class="text-[10px] text-[#8c8c8c]">Auth: Bearer token accepted.</span>
    {#if dataSource}<span class="text-[10px] text-[#8c8c8c]">Data source: {dataSource}.</span>{/if}
    {#if systemdStatus}<span class="text-[10px] text-[#8c8c8c]">systemd: {systemdStatus}.</span>{/if}
    {#if capabilities}
      <span class="text-[10px] text-[#8c8c8c]">
        ops: systemd {capabilities.systemdControl.enabled ? 'on' : 'off'}, backups {capabilities.backup.enabled ? 'on' : 'off'}, RCON {capabilities.rcon.enabled ? 'on' : 'off'}.
      </span>
    {/if}
  {:else}
    <span class="font-medium">Backend unavailable.</span>
    {#if error}<span class="font-mono text-[10px] text-[#8c8c8c]">({error})</span>{/if}
    <span class="text-[10px] text-[#8c8c8c]">Start the Rust manager and provide the frontend API token.</span>
  {/if}
</div>
