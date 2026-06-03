<script lang="ts">
  // Small banner shown when a page is rendering local mock fallback data
  // because the Rust backend was unreachable or returned an error.
  let { error = null }: { error?: string | null } = $props();
</script>

<div
  class="mb-4 flex items-center gap-2 rounded-lg border border-[#bfa15e]/40 bg-[#bfa15e]/8 px-3 py-2 text-xs text-[#bfa15e]"
  role="status"
>
  <span class="h-2 w-2 rounded-full bg-[#bfa15e]"></span>
  <span class="font-medium">Showing mock data — backend unavailable.</span>
  {#if error}<span class="font-mono text-[10px] text-[#8c8c8c]">({error})</span>{/if}
  <span class="text-[10px] text-[#8c8c8c]">Start the Rust manager and set VITE_ARK_API_* to go live.</span>
</div>
