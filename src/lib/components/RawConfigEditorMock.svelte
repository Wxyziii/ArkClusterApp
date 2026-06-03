<script lang="ts">
  import StatusBadge from './StatusBadge.svelte';
  let {
    filename,
    value = $bindable(),
    original
  }: { filename: string; value: string; original: string } = $props();

  let dirty = $derived(value !== original);
  // crude mock syntax warning: lines without '=' that aren't section headers/blank
  let warnings = $derived(
    value
      .split('\n')
      .map((l, i) => ({ l: l.trim(), i }))
      .filter(({ l }) => l && !l.startsWith('[') && !l.startsWith(';') && !l.includes('='))
      .map(({ i }) => i + 1)
  );
</script>

<div class="card overflow-hidden">
  <header class="flex items-center justify-between border-b border-[#2a2a2a] px-4 py-2.5">
    <span class="flex items-center gap-2 font-mono text-xs text-[#ededed]">📄 {filename}</span>
    <div class="flex items-center gap-2">
      {#if dirty}<StatusBadge label="unsaved" tone="amber" size="sm" />{/if}
      {#if warnings.length}<StatusBadge label="{warnings.length} syntax warn" tone="red" size="sm" />{:else}<StatusBadge label="valid" tone="green" size="sm" />{/if}
    </div>
  </header>
  <textarea
    bind:value
    spellcheck="false"
    rows="14"
    class="w-full resize-y bg-[#0a0a0a] p-3 font-mono text-xs leading-relaxed text-[#ededed] outline-none"
  ></textarea>
  {#if warnings.length}
    <p class="border-t border-[#2a2a2a] bg-[#b5544f]/8 px-4 py-2 font-mono text-[11px] text-[#b5544f]">
      ⚠ Possible syntax issue on line(s): {warnings.join(', ')} — expected key=value.
    </p>
  {/if}
  {#if dirty}
    <div class="border-t border-[#2a2a2a] bg-[#0a0a0a]/60 p-3">
      <p class="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-[#8c8c8c]">Diff preview (mock)</p>
      <pre class="overflow-x-auto rounded bg-[#0a0a0a] p-2 font-mono text-[11px]"><span class="text-[#b5544f]">- {original.split('\n').length} lines (saved)</span>
<span class="text-[#7c9a82]">+ {value.split('\n').length} lines (editing)</span></pre>
    </div>
  {/if}
</div>
