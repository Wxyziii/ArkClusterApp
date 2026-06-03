<script lang="ts">
  let {
    steps,
    current
  }: { steps: string[]; current: number } = $props();
</script>

<ol class="relative ml-3 space-y-4 border-l border-[#2a2a2a]">
  {#each steps as step, i (step)}
    {@const done = i < current}
    {@const active = i === current}
    <li class="ml-5">
      <span
        class="absolute -left-[7px] flex h-3.5 w-3.5 items-center justify-center rounded-full border-2
        {done ? 'border-[#7c9a82] bg-[#7c9a82]' : active ? 'border-[#bfa15e] bg-[#bfa15e]/30 live-dot' : 'border-[#2a2a2a] bg-[#0a0a0a]'}"
      ></span>
      <p class="text-xs font-medium {active ? 'text-[#bfa15e]' : done ? 'text-[#ededed]' : 'text-[#5c5c5c]'}">
        {step}
        {#if active}<span class="ml-1 text-[10px] text-[#bfa15e]">● in progress</span>{/if}
        {#if done}<span class="ml-1 text-[10px] text-[#7c9a82]">✓</span>{/if}
      </p>
    </li>
  {/each}
</ol>
