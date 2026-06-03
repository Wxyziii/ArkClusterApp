<script lang="ts">
  let { label = 'Loading…', rows = 3 }: { label?: string; rows?: number } = $props();
</script>

<div class="space-y-3" aria-busy="true" aria-label={label}>
  {#each Array(rows) as _, i (i)}
    <div class="h-10 w-full animate-pulse rounded-lg bg-[#181818]" style="animation-delay: {i * 120}ms"></div>
  {/each}
  <p class="text-center text-xs text-[#8c8c8c]">{label}</p>
</div>
