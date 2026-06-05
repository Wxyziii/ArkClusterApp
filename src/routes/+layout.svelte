<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { nav } from '$lib/nav';
  import { cluster, pressureLevel, discord } from '$lib/data/mock';
  import { api } from '$lib/api';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import TailscaleStatusBadge from '$lib/components/TailscaleStatusBadge.svelte';
  import type { Snippet } from 'svelte';
  import type { Tone } from '$lib/types';

  let { children }: { children: Snippet } = $props();

  let sidebarOpen = $state(false);
  let userMenuOpen = $state(false);
  const initialPressure = pressureLevel();
  let pl = $derived(pressureLevel());
  let current = $derived($page.url.pathname);
  let clusterName = $state(cluster.name);
  let clusterId = $state(cluster.id);
  let managerTone = $state<Tone>('green');
  let managerLabel = $state('Rust mgr');
  let discordTone = $state<Tone>(discord.online ? 'green' : 'red');
  let discordLabel = $state('Discord');
  let pressureLabel = $state(initialPressure.label);
  let pressureTone = $state<Tone>(initialPressure.tone);

  // group nav
  const groups = [...new Set(nav.map((n) => n.group))];
  function isActive(href: string) {
    return href === '/' ? current === '/' : current.startsWith(href);
  }

  onMount(() => {
    refreshShell();
    const id = window.setInterval(refreshShell, 30000);
    return () => window.clearInterval(id);
  });

  async function refreshShell() {
    try {
      const s = await api.status();
      clusterName = s.cluster.name;
      clusterId = s.cluster.id;
      managerTone = s.manager.tone as Tone;
      managerLabel = s.manager.status;
      discordTone = s.discord.tone as Tone;
      discordLabel = s.discord.status;
      pressureLabel = s.resourcePressure.label;
      pressureTone = s.resourcePressure.tone as Tone;
    } catch {
      pressureLabel = pl.label;
      pressureTone = pl.tone;
    }
  }
</script>

<div class="ark-grid flex h-screen overflow-hidden text-[#ededed]">
  <!-- Sidebar -->
  <aside
    class="fixed inset-y-0 left-0 z-40 w-60 transform border-r border-[#2a2a2a] bg-[#121212] transition-transform lg:static lg:translate-x-0 {sidebarOpen ? 'translate-x-0' : '-translate-x-full'}"
  >
    <div class="flex h-14 items-center gap-2 border-b border-[#2a2a2a] px-4">
      <span class="text-xl">🦖</span>
      <div class="leading-tight">
        <p class="text-sm font-bold">ARK Cluster</p>
        <p class="text-[10px] text-[#8c8c8c]">Smart Manager</p>
      </div>
    </div>
    <nav class="flex flex-col gap-4 overflow-y-auto px-3 py-4" style="height: calc(100vh - 3.5rem)">
      {#each groups as g (g)}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-wider text-[#5c5c5c]">{g}</p>
          <ul class="space-y-0.5">
            {#each nav.filter((n) => n.group === g) as item (item.href)}
              <li>
                <a
                  href={item.href}
                  onclick={() => (sidebarOpen = false)}
                  class="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                  {isActive(item.href)
                    ? 'bg-[#1c1c1c] font-medium text-[#ededed] border-l-2 border-[#ededed] pl-2'
                    : 'text-[#8c8c8c] hover:bg-[#181818] hover:text-[#ededed]'}"
                >
                  <span>{item.icon}</span>{item.label}
                </a>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
      <div class="mt-auto rounded-lg border border-[#2a2a2a] bg-[#141414] p-2.5 text-[10px] text-[#8c8c8c]">
        🔒 Private cluster — Tailscale only. Do not expose this UI publicly.
      </div>
    </nav>
  </aside>

  {#if sidebarOpen}
    <button class="fixed inset-0 z-30 bg-black/50 lg:hidden" onclick={() => (sidebarOpen = false)} aria-label="Close menu"></button>
  {/if}

  <!-- Main -->
  <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
    <!-- Topbar -->
    <header class="z-20 flex h-14 shrink-0 items-center gap-3 border-b border-[#2a2a2a] bg-[#0a0a0a]/90 px-4 backdrop-blur">
      <button class="lg:hidden" onclick={() => (sidebarOpen = !sidebarOpen)} aria-label="Toggle menu">☰</button>
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold">{clusterName}</p>
        <p class="hidden font-mono text-[10px] text-[#8c8c8c] sm:block">{clusterId}</p>
      </div>

      <div class="ml-auto flex items-center gap-2">
        <div class="hidden items-center gap-2 md:flex">
          <TailscaleStatusBadge compact />
          <StatusBadge label={managerLabel} tone={managerTone} dot size="sm" />
          <StatusBadge label={discordLabel} tone={discordTone} dot size="sm" />
        </div>
        <StatusBadge label={pressureLabel} tone={pressureTone} dot pulse={pressureTone !== 'green'} />

        <!-- user menu -->
        <div class="relative">
          <button
            onclick={() => (userMenuOpen = !userMenuOpen)}
            class="flex items-center gap-2 rounded-lg border border-[#2a2a2a] bg-[#181818] px-2 py-1.5 text-xs hover:bg-[#222222]"
          >
            <span class="flex h-6 w-6 items-center justify-center rounded-full bg-[#3a3a3a] text-[10px] font-bold">M</span>
            <span class="hidden sm:inline">Marcel</span>
            <span class="text-[10px]">▾</span>
          </button>
          {#if userMenuOpen}
            <div class="absolute right-0 mt-2 w-48 rounded-lg border border-[#2a2a2a] bg-[#181818] py-1 text-xs shadow-xl">
              <div class="border-b border-[#2a2a2a] px-3 py-2">
                <p class="font-medium">Marcel</p>
                <p class="text-[10px] text-[#8c8c8c]">Admin · authenticated (mock)</p>
              </div>
              <button class="block w-full px-3 py-2 text-left text-[#8c8c8c] hover:bg-[#222222]">Profile (placeholder)</button>
              <button class="block w-full px-3 py-2 text-left text-[#8c8c8c] hover:bg-[#222222]">Switch to read-only</button>
              <button class="block w-full px-3 py-2 text-left text-[#b5544f] hover:bg-[#222222]">Sign out</button>
            </div>
          {/if}
        </div>
      </div>
    </header>

    <main class="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">
      {@render children()}
    </main>
  </div>
</div>
