<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { nav } from '$lib/nav';
  import { api, type ClusterStatus } from '$lib/api';
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();

  let sidebarOpen = $state(false);
  let status = $state<ClusterStatus | null>(null);
  let shellError = $state<string | null>(null);
  let current = $derived($page.url.pathname);

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
      status = await api.status();
      shellError = null;
    } catch (e) {
      shellError = e instanceof Error ? e.message : 'manager unavailable';
    }
  }
</script>

<div class="app-shell">
  <aside class:open={sidebarOpen}>
    <div class="brand">
      <div class="brand-mark">A</div>
      <div>
        <p>ARK Cluster</p>
        <span>{status?.cluster.id ?? 'private manager'}</span>
      </div>
    </div>

    <nav>
      {#each nav as item (item.href)}
        <a href={item.href} class:active={isActive(item.href)} onclick={() => (sidebarOpen = false)}>
          <span>{item.icon}</span>
          {item.label}
        </a>
      {/each}
    </nav>

    <div class="private-note">
      <strong>Private access</strong>
      <span>{status?.tailscale.bindAddress ?? 'Tailscale/LAN only'}</span>
    </div>
  </aside>

  {#if sidebarOpen}
    <button class="scrim" aria-label="Close menu" onclick={() => (sidebarOpen = false)}></button>
  {/if}

  <div class="main-pane">
    <header>
      <button class="menu-button" aria-label="Toggle menu" onclick={() => (sidebarOpen = !sidebarOpen)}>Menu</button>
      <div class="cluster-title">
        <p>{status?.cluster.name ?? 'ARK Cluster'}</p>
        <span>{status?.dataMode ?? 'live'} data</span>
      </div>
      <div class="top-status">
        <span class:error={!!shellError}>{shellError ? 'API error' : (status?.manager.status ?? 'Checking')}</span>
        <span>{status?.systemd.status ?? 'systemd unknown'}</span>
        <span>{status?.resourcePressure.label ?? 'resources unknown'}</span>
      </div>
    </header>

    <main>
      {@render children()}
    </main>
  </div>
</div>
