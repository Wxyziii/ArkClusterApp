<script lang="ts">
  import StatusBadge from './StatusBadge.svelte';

  let {
    online = false,
    guild = '',
    statusChannel = '',
    source = 'unavailable'
  }: {
    online?: boolean;
    guild?: string;
    statusChannel?: string;
    source?: string;
  } = $props();
</script>

<div class="card">
  <header class="flex items-center justify-between border-b border-[#2a2a2a] px-4 py-3">
    <h2 class="text-sm font-semibold">Discord Service</h2>
    <StatusBadge label={online ? 'active' : 'inactive'} tone={online ? 'green' : 'gray'} size="sm" dot />
  </header>
  <dl class="divide-y divide-[#2a2a2a]/60 text-xs">
    <div class="flex justify-between px-4 py-2.5"><dt class="text-[#8c8c8c]">Guild</dt><dd>{guild || 'not reported'}</dd></div>
    <div class="flex justify-between px-4 py-2.5"><dt class="text-[#8c8c8c]">Status channel</dt><dd>{statusChannel || 'not reported'}</dd></div>
    <div class="flex justify-between px-4 py-2.5"><dt class="text-[#8c8c8c]">Source</dt><dd>{source}</dd></div>
  </dl>
</div>
