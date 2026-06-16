<script lang="ts">
  import { onMount, onCleanup } from 'svelte';
  import Breadcrumb from '../components/Breadcrumb.svelte';
  import TaskColumn from '../components/TaskColumn.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import RouterLink from '../components/RouterLink.svelte';
  import { kanbanColumns } from '$lib/statusColors';
  import { getPlan, listTasks } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { Plan, Task } from '$lib/types';
  
  let { branch, planId }: { branch: string; planId: string } = $props();
  
  let plan = $state<Plan | null>(null);
  let tasks = $state<Task[]>([]);

  let loading = $state(false);
  let error = $state<string | null>(null);
  const savedViewMode = localStorage.getItem('nexum-viewMode');
  let viewMode = $state<'kanban' | 'list'>(
    (savedViewMode === 'kanban' || savedViewMode === 'list') ? savedViewMode : 'kanban'
  );
  
  onMount(async () => {
    loading = true;
    try {
      plan = await getPlan(branch, planId);
      const response = await listTasks(branch, planId);
      tasks = response.items;
    } catch (e) {
      if (e instanceof Error) {
        const cleanup = setError(() => { error = e.message; }, () => { error = null; });
        onCleanup(cleanup);
      }
    } finally {
      loading = false;
    }
  });
  
  const breadcrumbItems = $derived([
    { label: 'Plans', href: '/plans' },
    { label: plan?.name ?? planId, href: `/plans/${branch}/${planId}` },
    { label: 'Tasks' },
  ]);
  
  function toggleView(): void {
    viewMode = viewMode === 'kanban' ? 'list' : 'kanban';
    localStorage.setItem('nexum-viewMode', viewMode);
  }
  
  const tasksByStatus = $derived(
    tasks.reduce<Record<string, Task[]>>((acc, t) => {
      const s = t.status.status;
      if (!acc[s]) acc[s] = [];
      acc[s].push(t);
      return acc;
    }, {})
  );
</script>

<div class="p-6">
  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="text-text-muted">Loading tasks...</div>
    </div>
  {:else if !plan}
    <div class="flex flex-col items-center justify-center py-12 text-text-muted">
      <p class="text-lg">Plan not found</p>
    </div>
  {:else}
    <!-- Top Bar -->
    <div class="flex items-center gap-4 mb-4">
      <RouterLink href={`/plans/${branch}/${planId}`} class="text-text-muted hover:text-text-secondary text-sm">← Back</RouterLink>
      <Breadcrumb items={breadcrumbItems} />
    </div>
    
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-bold text-text-primary">{plan.name} — Tasks</h1>
      </div>
      <div class="flex items-center gap-3">
        <!-- View Mode Toggle -->
        <div class="flex border border-border-default rounded-md overflow-hidden">
          <button class="px-3 py-1 text-xs { viewMode === 'kanban' ? 'bg-border-default text-text-secondary' : 'text-text-muted' }" onclick={toggleView}>
            Kanban
          </button>
          <button class="px-3 py-1 text-xs { viewMode === 'list' ? 'bg-border-default text-text-secondary' : 'text-text-muted' }" onclick={toggleView}>
            List
          </button>
        </div>
        <button class="px-3 py-1.5 bg-btn-green border border-btn-green text-white rounded-md text-sm hover:bg-btn-green-hover transition-colors">
          + Add Task
        </button>
      </div>
    </div>
    
    <!-- Kanban View -->
    {#if viewMode === 'kanban'}
      <div class="flex gap-4 overflow-x-auto pb-4">
        {#each kanbanColumns as status (status)}
          <TaskColumn status={status} tasks={tasksByStatus[status] ?? []} {planId} {branch} />
        {/each}
      </div>
    
    <!-- List View -->
    {:else}
      <div class="bg-bg-secondary border border-border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-6 px-4 py-2.5 bg-bg-tertiary border-b border-border-default text-xs font-semibold text-text-muted uppercase tracking-wider">
          <div class="col-span-2">Task</div>
          <div>Status</div>
          <div>Agent</div>
          <div>Dependencies</div>
          <div>Actions</div>
        </div>
        {#each tasks as task (task.id)}
          <RouterLink href={`/plans/${branch}/${planId}/tasks/${task.id}`} class="grid grid-cols-6 px-4 py-3 border-b border-border-muted items-center hover:bg-bg-tertiary transition-colors last:border-b-0">
            <div class="col-span-2 text-sm text-text-secondary font-medium">{task.name}</div>
            <div>
              <StatusBadge status={task.status.status} type="task" />
            </div>
            <div>
              {#if task.status.agent}
                <span class="text-xs text-accent-blue bg-accent-blue-subtle px-2 py-0.5 rounded">{task.status.agent.role}</span>
              {:else}
                <span class="text-xs text-text-faint">—</span>
              {/if}
            </div>
            <div class="text-xs text-text-muted">{task.dependencies.length} deps</div>
            <div></div>
          </RouterLink>
        {/each}
        
        {#if tasks.length === 0}
          <p class="p-4 text-sm text-text-muted text-center italic">No tasks yet</p>
        {/if}
      </div>
    {/if}
  {/if}
</div>
