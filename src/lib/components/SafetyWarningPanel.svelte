<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    tone = 'warn',
    title,
    children
  }: { tone?: 'warn' | 'danger' | 'info'; title: string; children: Snippet } = $props();

  const map = {
    warn: { border: 'border-[#bfa15e]/40', bg: 'bg-[#bfa15e]/8', text: 'text-[#bfa15e]', icon: '⚠️' },
    danger: { border: 'border-[#b5544f]/40', bg: 'bg-[#b5544f]/8', text: 'text-[#b5544f]', icon: '🛑' },
    info: { border: 'border-[#8aa1ae]/40', bg: 'bg-[#8aa1ae]/8', text: 'text-[#8aa1ae]', icon: 'ℹ️' }
  };
  let s = $derived(map[tone]);
</script>

<div class="rounded-lg border {s.border} {s.bg} p-3">
  <p class="flex items-center gap-2 text-sm font-semibold {s.text}">
    <span>{s.icon}</span>{title}
  </p>
  <div class="mt-1.5 pl-6 text-xs text-[#8c8c8c]">{@render children()}</div>
</div>
