<script lang="ts">
  import { location } from 'svelte-spa-router';
  import { onMount } from 'svelte';
  import RouterLink from './RouterLink.svelte';
  import { listAgents, listRunningTasks } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { AgentRegistration, RunningTask } from '$lib/types';

  const RUNNING_TASKS_POLL_INTERVAL_MS = 5000;

  const currentPath = $derived($location);

  function isActive(path: string): boolean {
    return currentPath === path || currentPath.startsWith(path + '/');
  }

  let agents = $state<AgentRegistration[]>([]);
  let runningTasks = $state<RunningTask[]>([]);
  let agentsLoading = $state(false);
  let error = $state<string | null>(null);
  let errorCleanup: (() => void) | null = null;
  let agentsByType = $derived.by(() => agents.reduce<Record<string, AgentRegistration[]>>((groups, agent) => {
    const type = agent.type;
    if (!groups[type]) groups[type] = [];
    groups[type].push(agent);
    return groups;
  }, {}));

  $effect(() => {
    if (errorCleanup) {
      return errorCleanup;
    }
  });

  onMount(async () => {
    const controller = new AbortController();

    // --- Agent fetching ---
    agentsLoading = true;
    try {
      const response = await listAgents({ signal: controller.signal });
      agents = response.agents;
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') {
        // Silently ignore abort errors on unmount
      } else if (e instanceof Error) {
        errorCleanup = setError(
          () => { error = e.message; },
          () => { error = null; }
        );
      }
    } finally {
      agentsLoading = false;
    }

    // --- Running tasks polling setup ---
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchRunningTasks() {
      try {
        const tasks = await listRunningTasks({ signal: controller.signal });
        runningTasks = tasks;
      } catch (e) {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        // Silently ignore polling errors (network blips are expected)
        if (!controller.signal.aborted && e instanceof DOMException) {
          console.warn('Failed to fetch running tasks:', e);
        }
      }
    }

    // Initial fetch
    await fetchRunningTasks();

    // Poll every 5 seconds
    pollInterval = setInterval(fetchRunningTasks, RUNNING_TASKS_POLL_INTERVAL_MS);

    // Return cleanup function (Svelte 5 pattern)
    return () => {
      if (pollInterval) clearInterval(pollInterval);
      controller.abort();
    };
  });

  const totalCount = $derived(agents.length);

  // Extract unique agent roles from running tasks
  const activeRoles = $derived(
    new Set(
      runningTasks
        .map(t => t.agent?.role)
        .filter((r): r is string => Boolean(r))
    )
  );

  // Count agents whose type matches an active role
  const activeCount = $derived(
    agents.filter(agent => activeRoles.has(agent.type)).length
  );
  const idleCount = $derived(totalCount - activeCount);

  const utilizationPercent = $derived(totalCount > 0 ? (activeCount / totalCount) * 100 : 0);
  function isAgentActive(agent: AgentRegistration): boolean {
    return activeRoles.has(agent.type);
  }
</script>

<div class="flex h-screen bg-bg-primary overflow-hidden">
  <!-- Icon Sidebar (56px) -->
  <nav class="w-[56px] bg-bg-secondary border-r border-border-default flex flex-col items-center py-3 gap-1 flex-shrink-0">
    <RouterLink href="/" class="w-10 h-10 rounded-lg flex items-center justify-center cursor-pointer text-text-muted hover:bg-border-muted hover:text-text-secondary transition-colors {isActive('/') ? 'bg-accent-blue-subtle text-accent-blue' : ''}" title="Home">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/></svg>
    </RouterLink>
    <RouterLink href="/plans" class="w-10 h-10 rounded-lg flex items-center justify-center cursor-pointer text-text-muted hover:bg-border-muted hover:text-text-secondary transition-colors {isActive('/plans') ? 'bg-accent-blue-subtle text-accent-blue' : ''}" title="Plans">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
    </RouterLink>
  </nav>

  <!-- Agent Pool Sidebar (220px) -->
  <aside class="w-[220px] bg-bg-secondary border-r border-border-default flex flex-col flex-shrink-0 overflow-y-auto">
    <div class="p-3 flex items-center justify-between border-b border-border-default">
      <span class="text-xs font-semibold text-text-muted uppercase tracking-wider">Agent Pool</span>
    </div>
    <div class="p-2.5 border-b border-border-default">
      <div class="flex justify-between text-[11px] text-text-muted">
        <span><strong class="text-text-secondary">{activeCount}</strong> active</span>
        <span><strong class="text-text-secondary">{idleCount}</strong> idle</span>
      </div>
      <div class="h-1.5 bg-border-default rounded-sm overflow-hidden mt-1.5">
        <div class="h-full bg-accent-green rounded-sm" style="width: {utilizationPercent}%"></div>
      </div>
    </div>
    {#if agentsLoading}
      <div class="px-3.5 py-4 text-xs text-text-muted">Loading agents...</div>
    {:else if error}
      <div class="px-3.5 py-4 text-xs text-accent-red">{error}</div>
    {:else if agents.length === 0}
      <div class="px-3.5 py-4 text-xs text-text-muted italic">No agents configured</div>
    {:else}
      <div class="text-[10px] uppercase tracking-widest text-text-faint font-semibold px-3.5 py-2.5">
        Groups
      </div>
      {#each Object.entries(agentsByType) as [type, groupAgents] (type)}
        <div class="mb-2">
          <div class="flex items-center gap-1.5 px-3 pb-1">
            <span class="text-[10px] uppercase tracking-wider text-text-faint font-semibold">{type}</span>
            <span class="text-[10px] text-text-faint">({groupAgents.length})</span>
          </div>
          {#each groupAgents as agent (agent.name)}
            <div class="flex items-center gap-2 px-3.5 py-1.5 rounded-md hover:bg-border-muted transition-colors cursor-default">
              <div class="w-1.5 h-1.5 rounded-full flex-shrink-0 {isAgentActive(agent) ? 'bg-accent-green' : 'bg-accent-yellow'}"></div>
              <span class="text-xs text-text-secondary truncate" title="{agent.name}">{agent.name}</span>
            </div>
          {/each}
        </div>
      {/each}
    {/if}
  </aside>

  <!-- Main Content Area -->
  <main class="flex-1 overflow-y-auto bg-bg-primary">
    <slot></slot>
  </main>
</div>
