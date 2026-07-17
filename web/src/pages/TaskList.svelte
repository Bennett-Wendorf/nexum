<script lang="ts">
  import { onMount } from 'svelte';
  import Breadcrumb from '../components/Breadcrumb.svelte';
  import TaskColumn from '../components/TaskColumn.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import RouterLink from '../components/RouterLink.svelte';
  import { kanbanColumns } from '$lib/statusColors';
  import { getPlan, listTasks, getConfig, patchConfig, createTask } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { Plan, Task, Config, CreateTaskRequest } from '$lib/types';
  import CreateTaskForm from '../components/CreateTaskForm.svelte';
  
  let { branch, planId }: { branch: string; planId: string } = $props();
  
  let plan = $state<Plan | null>(null);
  let tasks = $state<Task[]>([]);
  let yoloMode = $state(false);
  let yoloLoading = $state(false);

  let loading = $state(false);
  let error = $state<string | null>(null);
  let errorCleanup: (() => void) | null = null;
  let showCreateTaskForm = $state(false);
  let viewMode: 'kanban' | 'list' = (() => {
    const saved = localStorage.getItem('nexum-viewMode');
    return (saved === 'kanban' || saved === 'list') ? saved : 'kanban';
  })();
  
  onMount(async () => {
    loading = true;
    try {
      plan = await getPlan(branch, planId);
      const response = await listTasks(branch, planId);
      tasks = response.items;
      const config = await getConfig();
      yoloMode = config.yolo_mode;
    } catch (e) {
      if (e instanceof Error) {
        errorCleanup = setError(() => { error = e.message; }, () => { error = null; });
      }
    } finally {
      loading = false;
    }
  });

  $effect(() => {
    if (errorCleanup) {
      return errorCleanup;
    }
  });

  $effect(() => {
    if (showCreateTaskForm) {
      function handler(e: KeyboardEvent) {
        if (e.key === 'Escape') {
          showCreateTaskForm = false;
        }
      }
      window.addEventListener('keydown', handler);
      return () => window.removeEventListener('keydown', handler);
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

  async function toggleYolo(): Promise<void> {
    yoloLoading = true;
    try {
      const config = await patchConfig({ yolo_mode: !yoloMode });
      yoloMode = config.yolo_mode;
    } catch (e) {
      if (e instanceof Error) {
        errorCleanup = setError(() => { error = e.message; }, () => { error = null; });
      }
    } finally {
      yoloLoading = false;
    }
  }
  
  const tasksByStatus = $derived(
    tasks.reduce<Record<string, Task[]>>((acc, t) => {
      const s = t.status.status;
      if (!acc[s]) acc[s] = [];
      acc[s].push(t);
      return acc;
    }, {})
  );
  const tasksMap = $derived(new Map(tasks.map(t => [t.id, t])));

  async function handleTaskTransition(taskId: string, newStatus: string): Promise<void> {
    try {
      const response = await listTasks(branch, planId);
      tasks = response.items;
    } catch (e) {
      if (e instanceof Error) {
        errorCleanup = setError(() => { error = e.message; }, () => { error = null; });
      }
    }
  }
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
          <button type="button" class="px-3 py-1 text-xs { viewMode === 'kanban' ? 'bg-border-default text-text-secondary' : 'text-text-muted' }" onclick={toggleView}>
            Kanban
          </button>
          <button type="button" class="px-3 py-1 text-xs { viewMode === 'list' ? 'bg-border-default text-text-secondary' : 'text-text-muted' }" onclick={toggleView}>
            List
          </button>
        </div>
        <!-- Yolo Toggle -->
        <button
          class="px-3 py-1.5 rounded-md text-sm font-medium border transition-colors disabled:opacity-50 cursor-pointer
            {yoloMode
              ? 'bg-accent-yellow-subtle text-accent-yellow border-accent-yellow hover:bg-accent-yellow/20'
              : 'text-text-muted border-border-default hover:text-text-secondary hover:border-border-strong'}"
          onclick={toggleYolo}
          disabled={yoloLoading}
        >
          ⚡ Yolo: {yoloMode ? 'ON' : 'OFF'}
        </button>
        <button type="button" class="px-3 py-1.5 bg-btn-green border border-btn-green text-white rounded-md text-sm hover:bg-btn-green-hover transition-colors cursor-pointer" onclick={() => showCreateTaskForm = true}>
          + Add Task
        </button>
      </div>
    </div>
    
    <!-- Kanban View -->
    {#if viewMode === 'kanban'}
      <div class="flex gap-4 overflow-x-auto pb-4">
        {#each kanbanColumns as status (status)}
          <TaskColumn status={status} tasks={tasksByStatus[status] ?? []} {planId} {branch} tasksMap={tasksMap} onTaskTransition={handleTaskTransition} />
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

    {#if showCreateTaskForm}
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-bg-primary/80 backdrop-blur-sm" onclick={() => showCreateTaskForm = false}>
        <div class="bg-bg-secondary border border-border-default rounded-lg shadow-2xl w-full max-w-lg mx-4" onclick={(e) => e.stopPropagation()}>
          <CreateTaskForm
            parentPlan={planId}
            onSubmit={async (data: CreateTaskRequest) => {
              try {
                const newTask = await createTask(branch, planId, data);
                tasks = [...tasks, newTask];
                showCreateTaskForm = false;
              } catch (e) {
                if (e instanceof Error) {
                  errorCleanup = setError(
                    () => { error = e.message; },
                    () => { error = null; }
                  );
                }
              }
            }}
            onCancel={() => showCreateTaskForm = false}
          />
        </div>
      </div>
    {/if}
  {/if}
</div>
