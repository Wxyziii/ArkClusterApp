<script lang="ts">
  import type { LogEvent } from '$lib/types';
  import StatusBadge from './StatusBadge.svelte';
  import { severityTone } from '$lib/ui';

  let { event, expandable = true }: { event: LogEvent; expandable?: boolean } = $props();
  let open = $state(false);

  const icon: Record<string, string> = {
    Map: '🗺️', Travel: '🧭', Governor: '🧠', RCON: '🔌', Discord: '💬',
    Config: '⚙️', Mod: '🧩', Backup: '💾', User: '👤'
  };
</script>

<div class="border-l-2 py-2 pl-3 transition-colors hover:bg-[#181818]/50
  {event.severity === 'error' ? 'border-l-[#b5544f]' : event.severity === 'warn' ? 'border-l-[#bfa15e]' : event.severity === 'success' ? 'border-l-[#7c9a82]' : 'border-l-[#8aa1ae]'}">
  <button
    class="flex w-full items-start gap-3 text-left"
    onclick={() => expandable && (open = !open)}
    disabled={!expandable}
  >
    <span class="mt-0.5 text-sm">{icon[event.source] ?? '•'}</span>
    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-2">
        <span class="font-mono text-[11px] text-[#8c8c8c]">{event.ts}</span>
        <StatusBadge label={event.source} tone={severityTone[event.severity]} size="sm" />
        {#if event.targetMap !== '—'}<span class="text-[11px] text-[#8aa1ae]">{event.targetMap}</span>{/if}
        <span class="text-[11px] text-[#5c5c5c]">· {event.actor}</span>
      </div>
      <p class="mt-0.5 text-xs text-[#ededed]">{event.message}</p>
      {#if open}
        <p class="mt-1.5 rounded bg-[#0a0a0a]/60 p-2 font-mono text-[11px] text-[#8c8c8c]">{event.detail}</p>
      {/if}
    </div>
    {#if expandable}<span class="mt-0.5 text-[10px] text-[#5c5c5c]">{open ? '▲' : '▼'}</span>{/if}
  </button>
</div>
