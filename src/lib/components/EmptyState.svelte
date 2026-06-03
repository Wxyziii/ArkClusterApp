<script lang="ts">
  let { icon = '🗺️', title, hint }: { icon?: string; title: string; hint?: string } = $props();
</script>

<div class="flex flex-col items-center justify-center rounded-lg border border-dashed border-[#2a2a2a] bg-[#0a0a0a]/40 px-6 py-10 text-center">
  <span class="mb-2 text-3xl opacity-60">{icon}</span>
  <p class="text-sm font-medium text-[#ededed]">{title}</p>
  {#if hint}<p class="mt-1 max-w-sm text-xs text-[#8c8c8c]">{hint}</p>{/if}
</div>
