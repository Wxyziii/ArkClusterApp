<script lang="ts">
  let {
    current = 0,
    blocked = false,
    blockedAt = -1
  }: { current?: number; blocked?: boolean; blockedAt?: number } = $props();

  const steps = [
    { t: 'Request received', d: 'From in-game chat, Discord, or Web UI' },
    { t: 'Resource check', d: 'RAM / CPU / disk pressure evaluated' },
    { t: 'Slot selected', d: 'Travel A / Travel B chosen if free' },
    { t: 'Starting map (systemd)', d: 'ark-server@travel-x.service start' },
    { t: 'Waiting for RCON', d: 'Server boots, RCON becomes ready' },
    { t: 'Transfer available', d: 'Players may travel to the map' },
    { t: 'Monitoring idle timer', d: 'Auto-shutdown countdown when empty' },
    { t: 'Save + backup', d: 'Before shutdown' },
    { t: 'Auto-shutdown when empty', d: 'Slot released' }
  ];
</script>

<ol class="space-y-1">
  {#each steps as s, i (s.t)}
    {@const done = i < current}
    {@const active = i === current && !blocked}
    {@const isBlocked = blocked && i === blockedAt}
    <li class="flex items-start gap-3 rounded-lg px-2 py-1.5 {active ? 'bg-[#bfa15e]/8' : isBlocked ? 'bg-[#b5544f]/8' : ''}">
      <span
        class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-[11px] font-bold
        {done ? 'bg-[#7c9a82] text-[#0a0a0a]' : isBlocked ? 'bg-[#b5544f] text-white' : active ? 'bg-[#bfa15e] text-[#0a0a0a] live-dot' : 'bg-[#181818] text-[#5c5c5c] border border-[#2a2a2a]'}"
      >
        {#if done}✓{:else if isBlocked}✕{:else}{i + 1}{/if}
      </span>
      <div class="min-w-0 pt-0.5">
        <p class="text-xs font-medium {done ? 'text-[#ededed]' : active ? 'text-[#bfa15e]' : isBlocked ? 'text-[#b5544f]' : 'text-[#5c5c5c]'}">{s.t}</p>
        <p class="text-[11px] text-[#8c8c8c]">{s.d}</p>
      </div>
    </li>
  {/each}
</ol>
