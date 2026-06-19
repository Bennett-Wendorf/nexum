<script lang="ts">
  import { location } from 'svelte-spa-router';
  import RouterLink from './RouterLink.svelte';

  const currentPath = $derived($location);

  function isActive(path: string): boolean {
    return currentPath === path || currentPath.startsWith(path + '/');
  }
</script>

<div class="flex h-screen bg-bg-primary overflow-hidden">
  <!-- Icon Sidebar (56px) -->
  <nav class="w-[56px] bg-bg-secondary border-r border-border-default flex flex-col items-center py-3 gap-1 flex-shrink-0">
    <RouterLink href="/" class="w-10 h-10 rounded-lg flex items-center justify-center cursor-pointer text-text-muted hover:bg-border-muted hover:text-text-secondary transition-colors {isActive('/') ? 'bg-accent-blue-subtle text-accent-blue' : ''}" title="Dashboard">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/></svg>
    </RouterLink>
    <RouterLink href="/plans" class="w-10 h-10 rounded-lg flex items-center justify-center cursor-pointer text-text-muted hover:bg-border-muted hover:text-text-secondary transition-colors {isActive('/plans') ? 'bg-accent-blue-subtle text-accent-blue' : ''}" title="Plans">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
    </RouterLink>
    <div class="w-8 h-[1px] bg-border-default my-2"></div>
    <a href="#" class="w-10 h-10 rounded-lg flex items-center justify-center cursor-pointer text-text-muted hover:bg-border-muted hover:text-text-secondary transition-colors" title="Agent Teams">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/></svg>
    </a>
    <a href="#" class="w-10 h-10 rounded-lg flex items-center justify-center cursor-pointer text-text-muted hover:bg-border-muted hover:text-text-secondary transition-colors" title="Settings">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37a1.724 1.724 0 002.573-1.066z"/><circle cx="12" cy="12" r="3"/></svg>
    </a>
  </nav>

  <!-- Agent Pool Sidebar (220px) -->
  <aside class="w-[220px] bg-bg-secondary border-r border-border-default flex flex-col flex-shrink-0 overflow-y-auto">
    <div class="p-3 flex items-center justify-between border-b border-border-default">
      <span class="text-xs font-semibold text-text-muted uppercase tracking-wider">Agent Pool</span>
    </div>
    <div class="p-2.5 border-b border-border-default">
      <div class="flex justify-between text-[11px] text-text-muted">
        <span><strong class="text-text-secondary">0</strong> active</span>
        <span><strong class="text-text-secondary">0</strong> idle</span>
      </div>
      <div class="h-1.5 bg-border-default rounded-sm overflow-hidden mt-1.5">
        <div class="h-full bg-accent-green rounded-sm" style="width: 0%"></div>
      </div>
    </div>
    <div class="text-[10px] uppercase tracking-widest text-text-faint font-semibold px-3.5 py-2.5">
      Groups
    </div>
    <div class="px-2.5 pb-2 text-xs text-text-muted italic">
      No agents configured
    </div>
  </aside>

  <!-- Main Content Area -->
  <main class="flex-1 overflow-y-auto bg-bg-primary">
    <slot></slot>
  </main>
</div>
