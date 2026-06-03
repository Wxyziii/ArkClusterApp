<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    title,
    subtitle,
    icon,
    elevated = false,
    actions,
    children,
    pad = true
  }: {
    title?: string;
    subtitle?: string;
    icon?: string;
    elevated?: boolean;
    actions?: Snippet;
    children: Snippet;
    pad?: boolean;
  } = $props();
</script>

<section class="{elevated ? 'card-elevated' : 'card'} flex flex-col">
  {#if title}
    <header class="flex items-start justify-between gap-3 border-b border-[#2a2a2a] px-4 py-3">
      <div class="min-w-0">
        <h2 class="flex items-center gap-2 text-sm font-semibold text-[#ededed]">
          {#if icon}<span class="text-base leading-none">{icon}</span>{/if}
          {title}
        </h2>
        {#if subtitle}<p class="mt-0.5 text-xs text-[#8c8c8c]">{subtitle}</p>{/if}
      </div>
      {#if actions}<div class="shrink-0">{@render actions()}</div>{/if}
    </header>
  {/if}
  <div class={pad ? 'p-4' : ''}>
    {@render children()}
  </div>
</section>
