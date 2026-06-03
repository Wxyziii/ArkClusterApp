<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title, subtitle, icon, actions }: { title: string; subtitle?: string; icon?: string; actions?: Snippet } = $props();
</script>

<div class="mb-5 flex flex-wrap items-end justify-between gap-3">
  <div>
    <h1 class="flex items-center gap-2 text-xl font-bold text-[#ededed]">
      {#if icon}<span>{icon}</span>{/if}{title}
    </h1>
    {#if subtitle}<p class="mt-1 text-sm text-[#8c8c8c]">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="flex flex-wrap gap-2">{@render actions()}</div>{/if}
</div>
