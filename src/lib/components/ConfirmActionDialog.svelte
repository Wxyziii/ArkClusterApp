<script lang="ts">
  import type { Snippet } from 'svelte';
  import Button from './Button.svelte';

  let {
    open = $bindable(false),
    title,
    tone = 'warn',
    confirmLabel = 'Confirm',
    requirePhrase,
    onconfirm,
    body
  }: {
    open?: boolean;
    title: string;
    tone?: 'warn' | 'danger';
    confirmLabel?: string;
    requirePhrase?: string;
    onconfirm?: () => void;
    body: Snippet;
  } = $props();

  let typed = $state('');
  let canConfirm = $derived(!requirePhrase || typed === requirePhrase);

  function close() {
    open = false;
    typed = '';
  }
  function confirm() {
    if (!canConfirm) return;
    onconfirm?.();
    close();
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm"
    onclick={close}
    onkeydown={(e) => e.key === 'Escape' && close()}
    role="presentation"
  >
    <div
      class="card-elevated w-full max-w-md overflow-hidden"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onkeydown={() => {}}
    >
      <header class="flex items-center gap-2 border-b border-[#2a2a2a] px-4 py-3">
        <span class="text-lg">{tone === 'danger' ? '⚠️' : '❗'}</span>
        <h3 class="text-sm font-semibold {tone === 'danger' ? 'text-[#b5544f]' : 'text-[#bfa15e]'}">{title}</h3>
      </header>
      <div class="space-y-3 px-4 py-4 text-sm text-[#ededed]">
        {@render body()}
        {#if requirePhrase}
          <div>
            <label for="confirm-phrase" class="mb-1 block text-xs text-[#8c8c8c]">
              Type <code class="rounded bg-[#0a0a0a] px-1.5 py-0.5 font-mono text-[#b5544f]">{requirePhrase}</code> to confirm
            </label>
            <input
              id="confirm-phrase"
              bind:value={typed}
              autocomplete="off"
              class="w-full rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-3 py-2 font-mono text-sm text-[#ededed] outline-none focus:border-[#b5544f]"
            />
          </div>
        {/if}
      </div>
      <footer class="flex justify-end gap-2 border-t border-[#2a2a2a] px-4 py-3">
        <Button variant="ghost" onclick={close}>Cancel</Button>
        <Button variant={tone === 'danger' ? 'danger' : 'warn'} disabled={!canConfirm} onclick={confirm}>
          {confirmLabel}
        </Button>
      </footer>
    </div>
  </div>
{/if}
