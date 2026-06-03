<script lang="ts">
  let { title = 'Something went wrong', detail }: { title?: string; detail?: string } = $props();
</script>

<div class="flex flex-col items-center justify-center rounded-lg border border-[#b5544f]/40 bg-[#b5544f]/8 px-6 py-8 text-center">
  <span class="mb-2 text-2xl">⛔</span>
  <p class="text-sm font-semibold text-[#b5544f]">{title}</p>
  {#if detail}<p class="mt-1 max-w-md font-mono text-xs text-[#8c8c8c]">{detail}</p>{/if}
</div>
