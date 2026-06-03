<script lang="ts">
  import type { Tone } from '$lib/types';
  let {
    value,
    tone = 'green',
    striped = false,
    height = 'h-2'
  }: { value: number; tone?: Tone; striped?: boolean; height?: string } = $props();

  const fill: Record<Tone, string> = {
    green: 'bg-[#7c9a82]', amber: 'bg-[#bfa15e]', red: 'bg-[#b5544f]',
    gray: 'bg-[#5c5c5c]', cyan: 'bg-[#8aa1ae]', accent: 'bg-[#3a3a3a]'
  };
  let clamped = $derived(Math.max(0, Math.min(100, value)));
</script>

<div class="w-full overflow-hidden rounded-full bg-[#0a0a0a] border border-[#2a2a2a] {height}">
  <div
    class="h-full rounded-full transition-all duration-500 {fill[tone]} {striped ? 'striped' : ''}"
    style="width: {clamped}%"
  ></div>
</div>
