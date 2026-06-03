<script lang="ts">
  let { reason = 'Configuration or mod changes are pending.' }: { reason?: string } = $props();
</script>

<div class="flex items-center gap-3 rounded-lg border border-[#bfa15e]/40 bg-[#bfa15e]/8 px-3 py-2">
  <span class="text-base">🔄</span>
  <div class="min-w-0">
    <p class="text-xs font-semibold text-[#bfa15e]">Restart required</p>
    <p class="text-xs text-[#8c8c8c]">{reason} Changes apply after the next map restart.</p>
  </div>
</div>
