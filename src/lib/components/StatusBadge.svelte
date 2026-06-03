<script lang="ts">
  import type { Tone } from '$lib/types';

  let {
    label,
    tone = 'gray',
    dot = false,
    pulse = false,
    size = 'md'
  }: { label: string; tone?: Tone; dot?: boolean; pulse?: boolean; size?: 'sm' | 'md' } = $props();

  const tones: Record<Tone, string> = {
    green: 'text-[#7c9a82] bg-[#7c9a82]/12 border-[#7c9a82]/30',
    amber: 'text-[#bfa15e] bg-[#bfa15e]/12 border-[#bfa15e]/30',
    red: 'text-[#b5544f] bg-[#b5544f]/12 border-[#b5544f]/30',
    gray: 'text-[#5c5c5c] bg-[#5c5c5c]/12 border-[#5c5c5c]/30',
    cyan: 'text-[#8aa1ae] bg-[#8aa1ae]/12 border-[#8aa1ae]/30',
    accent: 'text-[#ededed] bg-[#3a3a3a]/25 border-[#3a3a3a]/50'
  };
  const dotColor: Record<Tone, string> = {
    green: 'bg-[#7c9a82]', amber: 'bg-[#bfa15e]', red: 'bg-[#b5544f]',
    gray: 'bg-[#5c5c5c]', cyan: 'bg-[#8aa1ae]', accent: 'bg-[#7c9a82]'
  };
</script>

<span
  class="inline-flex items-center gap-1.5 rounded-full border font-medium whitespace-nowrap {tones[tone]} {size === 'sm' ? 'px-2 py-0.5 text-[10px]' : 'px-2.5 py-1 text-xs'}"
>
  {#if dot}
    <span class="h-1.5 w-1.5 rounded-full {dotColor[tone]} {pulse ? 'live-dot' : ''}"></span>
  {/if}
  {label}
</span>
