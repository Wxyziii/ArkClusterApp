<script lang="ts">
  type Option = string | { value: string; label: string };
  let {
    value = $bindable(''),
    options,
    placeholder = 'Select…',
    disabled = false,
    id,
    size = 'md',
    class: klass = ''
  }: {
    value?: string;
    options: Option[];
    placeholder?: string;
    disabled?: boolean;
    id?: string;
    size?: 'sm' | 'md';
    class?: string;
  } = $props();

  let open = $state(false);
  let root: HTMLDivElement;

  const norm = $derived(options.map((o) => (typeof o === 'string' ? { value: o, label: o } : o)));
  const selected = $derived(norm.find((o) => o.value === value));
  const pad = $derived(size === 'sm' ? 'px-2.5 py-1 text-xs' : 'px-3 py-2 text-sm');

  function pick(v: string) {
    value = v;
    open = false;
  }
  function onWindowClick(e: MouseEvent) {
    if (root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} />

<div bind:this={root} class="relative {klass}">
  <button
    {id}
    type="button"
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={() => (open = !open)}
    class="flex w-full items-center justify-between gap-2 rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] text-left text-[#ededed] outline-none transition-colors hover:border-[#3a3a3a] focus:border-[#3a3a3a] disabled:cursor-not-allowed disabled:opacity-40 {pad}"
  >
    <span class="truncate {selected ? '' : 'text-[#5c5c5c]'}">{selected ? selected.label : placeholder}</span>
    <span class="text-[10px] text-[#8c8c8c] transition-transform {open ? 'rotate-180' : ''}">▾</span>
  </button>
  {#if open}
    <ul
      role="listbox"
      class="absolute z-50 mt-1 max-h-60 w-full overflow-y-auto rounded-lg border border-[#2a2a2a] bg-[#181818] py-1 shadow-xl"
    >
      {#each norm as opt (opt.value)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={opt.value === value}
            onclick={() => pick(opt.value)}
            class="flex w-full items-center justify-between gap-2 px-3 py-1.5 text-left text-sm transition-colors hover:bg-[#222222] {opt.value === value ? 'text-[#7c9a82]' : 'text-[#ededed]'}"
          >
            <span class="truncate">{opt.label}</span>
            {#if opt.value === value}<span class="text-xs">✓</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
