<script lang="ts">
  import { governor, thresholds, ramPct } from '$lib/data/mock';
  import StatusBadge from './StatusBadge.svelte';
  import { pressureLevel } from '$lib/data/mock';

  let pl = pressureLevel();
</script>

<div class="card overflow-hidden">
  <header class="flex items-center justify-between border-b border-[#2a2a2a] px-4 py-3">
    <h2 class="flex items-center gap-2 text-sm font-semibold">🧠 Resource Governor</h2>
    <StatusBadge label={pl.label} tone={pl.tone} dot pulse={pl.tone !== 'green'} />
  </header>
  <div class="space-y-3 p-4">
    <div class="rounded-lg border border-[#8aa1ae]/30 bg-[#8aa1ae]/8 p-3">
      <p class="text-[11px] font-semibold uppercase tracking-wide text-[#8aa1ae]">Current decision</p>
      <p class="mt-1 text-sm font-medium text-[#ededed]">{governor.decision}</p>
      <p class="mt-1.5 text-xs text-[#8c8c8c]"><span class="text-[#8c8c8c]/70">why:</span> {governor.why}</p>
    </div>

    <div class="grid grid-cols-3 gap-2 text-center">
      <div class="rounded-lg bg-[#0a0a0a]/60 p-2">
        <p class="text-[10px] uppercase text-[#8c8c8c]">Warn</p>
        <p class="text-sm font-bold text-[#bfa15e]">{thresholds.ramWarnPct}%</p>
      </div>
      <div class="rounded-lg bg-[#0a0a0a]/60 p-2">
        <p class="text-[10px] uppercase text-[#8c8c8c]">Pressure</p>
        <p class="text-sm font-bold text-[#bfa15e]">{thresholds.ramPressurePct}%</p>
      </div>
      <div class="rounded-lg bg-[#0a0a0a]/60 p-2">
        <p class="text-[10px] uppercase text-[#8c8c8c]">Emergency</p>
        <p class="text-sm font-bold text-[#b5544f]">{thresholds.ramEmergencyPct}%</p>
      </div>
    </div>
    <p class="text-center text-xs text-[#8c8c8c]">Current RAM: <span class="font-bold text-[#ededed]">{ramPct}%</span></p>

    <div class="border-t border-[#2a2a2a] pt-2">
      <p class="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-[#8c8c8c]">Active policy</p>
      <div class="flex flex-wrap gap-1.5">
        <StatusBadge label="Max travel: 2" tone="cyan" size="sm" />
        <StatusBadge label="Never stop maps w/ players" tone="green" size="sm" />
        <StatusBadge label="Home standby enabled" tone="green" size="sm" />
        <StatusBadge label="Prefer active maps" tone="green" size="sm" />
        <StatusBadge label="Auto-restart Home" tone="green" size="sm" />
      </div>
    </div>
  </div>
</div>
