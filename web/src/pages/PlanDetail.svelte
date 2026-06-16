<script lang="ts">
  import { onMount, onCleanup } from 'svelte';
  import Breadcrumb from '../components/Breadcrumb.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import MarkdownRenderer from '../components/MarkdownRenderer.svelte';
  import RouterLink from '../components/RouterLink.svelte';
  import { getPlan, listTasks } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { Plan, Task } from '$lib/types';
  
  let { branch, planId }: { branch: string; planId: string } = $props();
  
  let loading = $state(false);
  let error = $state<string | null>(null);
  
  let plan = $state<Plan | null>(null);
  let tasks = $state<Task[]>([]);
  
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
  
  const totalTasks = $derived(tasks.length);
  const completedTasks = $derived(tasks.filter(t => t.status.status === 'completed').length);
  const progressPercent = $derived(totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0);
  const createdDate = $derived(plan ? new Date(plan.created).toLocaleDateString() : '');
  
  const breadcrumbItems = $derived([
    { label: 'Plans', href: '/plans' },
    { label: plan?.name ?? planId },
  ]);
</script>

<div class="p-6">
  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="text-text-muted">Loading plan...</div>
    </div>
  {:else if !plan}
    <div class="flex flex-col items-center justify-center py-12 text-text-muted">
      <p class="text-lg">Plan not found</p>
    </div>
  {:else}
    <!-- Top Bar -->
    <div class="flex items-center gap-4 mb-4">
      <RouterLink href="/plans" class="text-text-muted hover:text-text-secondary text-sm">← Back</RouterLink>
      <Breadcrumb items={breadcrumbItems} />
    </div>
    
    <!-- Header -->
    <div class="flex items-start justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold text-text-primary">{plan.name}</h1>
        <div class="flex items-center gap-3 mt-2">
          <StatusBadge status={plan.status} type="plan" />
          <span class="text-sm text-text-muted font-mono">{plan.branch}</span>
          <span class="text-sm text-text-muted">{createdDate}</span>
        </div>
      </div>
      <RouterLink href={`/plans/${branch}/${planId}/tasks`} class="px-3 py-1.5 border border-border-default text-text-secondary rounded-md text-sm hover:bg-border-default transition-colors">
        Kanban Board →
      </RouterLink>
    </div>
    
    <!-- Progress Bar -->
    {#if totalTasks > 0}
      <div class="mb-6">
        <div class="flex items-center gap-2 mb-1">
          <span class="text-xs text-text-muted">Progress</span>
          <span class="text-xs text-text-secondary font-semibold">{progressPercent}%</span>
        </div>
        <div class="h-2 bg-border-muted rounded-full overflow-hidden">
          <div class="h-full bg-accent-green rounded-full" style="width: {progressPercent}%"></div>
        </div>
      </div>
    {/if}
    
    <!-- Markdown Sections -->
    <div class="space-y-6 mb-8">
      {#if plan.goal}
        <div>
          <h2 class="text-base font-semibold text-text-secondary mb-2">Goal</h2>
          <MarkdownRenderer content={plan.goal} />
        </div>
      {/if}
      
      {#if plan.scope}
        <div>
          <h2 class="text-base font-semibold text-text-secondary mb-2">Scope</h2>
          <MarkdownRenderer content={plan.scope} />
        </div>
      {/if}
      
      {#if plan.background}
        <div>
          <h2 class="text-base font-semibold text-text-secondary mb-2">Background</h2>
          <MarkdownRenderer content={plan.background} />
        </div>
      {/if}
    </div>
    
    <!-- Task List -->
    <div>
      <h2 class="text-base font-semibold text-text-secondary mb-3">Tasks ({totalTasks})</h2>
      <div class="bg-bg-secondary border border-border-default rounded-lg overflow-hidden">
        {#each tasks as task (task.id)}
          <RouterLink href={`/plans/${branch}/${planId}/tasks/${task.id}`} class="flex items-center gap-3 p-3 border-b border-border-muted hover:bg-bg-tertiary transition-colors last:border-b-0">
            <StatusBadge status={task.status.status} type="task" />
            <span class="flex-1 text-sm text-text-secondary">{task.name}</span>
            {#if task.status.agent}
              <span class="text-xs text-accent-blue bg-accent-blue-subtle px-2 py-0.5 rounded">{task.status.agent.role}</span>
            {/if}
          </RouterLink>
        {/each}
        
        {#if tasks.length === 0}
          <p class="p-4 text-sm text-text-muted text-center italic">No tasks yet</p>
        {/if}
      </div>
    </div>
  {/if}
</div>
