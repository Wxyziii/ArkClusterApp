<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    variant = 'default',
    size = 'md',
    disabled = false,
    title,
    onclick,
    href,
    children
  }: {
    variant?: 'default' | 'primary' | 'danger' | 'ghost' | 'warn';
    size?: 'sm' | 'md';
    disabled?: boolean;
    title?: string;
    onclick?: () => void;
    href?: string;
    children: Snippet;
  } = $props();

  const variants = {
    default: 'bg-[#181818] border-[#2a2a2a] text-[#ededed] hover:bg-[#222222]',
    primary: 'bg-[#3a3a3a] border-[#7c9a82]/40 text-[#ededed] hover:bg-[#5b8c62]',
    danger: 'bg-[#b5544f]/15 border-[#b5544f]/40 text-[#b5544f] hover:bg-[#b5544f]/25',
    warn: 'bg-[#bfa15e]/15 border-[#bfa15e]/40 text-[#bfa15e] hover:bg-[#bfa15e]/25',
    ghost: 'bg-transparent border-transparent text-[#8c8c8c] hover:bg-[#222222] hover:text-[#ededed]'
  };
  const sizes = { sm: 'px-2.5 py-1 text-xs', md: 'px-3.5 py-2 text-sm' };
</script>

{#if href && !disabled}
  <a
    {href}
    {title}
    class="inline-flex items-center justify-center gap-1.5 rounded-lg border font-medium transition-colors {variants[variant]} {sizes[size]}"
  >
    {@render children()}
  </a>
{:else}
  <button
    {title}
    {disabled}
    {onclick}
    class="inline-flex items-center justify-center gap-1.5 rounded-lg border font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40 {variants[variant]} {sizes[size]}"
  >
    {@render children()}
  </button>
{/if}
