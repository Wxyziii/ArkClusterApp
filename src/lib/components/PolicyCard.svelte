<script lang="ts">
  type Row = { label: string; value: string | boolean; hint?: string };
  let { title, icon = '📋', rows }: { title: string; icon?: string; rows: Row[] } = $props();
</script>

<div class="card">
  <header class="border-b border-[#2a2a2a] px-4 py-3">
    <h3 class="flex items-center gap-2 text-sm font-semibold">{icon} {title}</h3>
  </header>
  <dl class="divide-y divide-[#2a2a2a]/60">
    {#each rows as r (r.label)}
      <div class="flex items-center justify-between gap-4 px-4 py-2.5">
        <dt class="text-xs text-[#8c8c8c]">{r.label}{#if r.hint}<span class="ml-1 text-[#5c5c5c]">· {r.hint}</span>{/if}</dt>
        <dd class="text-xs font-medium">
          {#if typeof r.value === 'boolean'}
            <span class={r.value ? 'text-[#7c9a82]' : 'text-[#5c5c5c]'}>{r.value ? 'Yes' : 'No'}</span>
          {:else}
            <span class="font-mono text-[#ededed]">{r.value}</span>
          {/if}
        </dd>
      </div>
    {/each}
  </dl>
</div>
