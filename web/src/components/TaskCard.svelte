<script lang="ts">
  import StatusBadge from './StatusBadge.svelte';
  import RouterLink from './RouterLink.svelte';
  import type { Task } from '$lib/types';
  import { transitionTaskStatus } from '$lib/api';
  
  let { task, planId, branch, tasksMap, columnStatus, onTaskTransition }: { 
    task: Task; 
    planId: string; 
    branch: string;
    tasksMap: Map<string, Task>;
    columnStatus: string;
    onTaskTransition: (taskId: string, newStatus: string) => void;
  } = $props();
  
  let reviewLoading = $state(false);
  
  const ACTIVE_STATUSES = ['backlog', 'queued', 'running', 'reviewing', 'waiting-manual-review', 'merge-queue'];
  
  const href = $derived(`/plans/${branch}/${planId}/tasks/${task.id}`);
  const agentName = $derived(task.status.agent?.role ?? null);
  const taskStatus = $derived(task.status.status);
  
  const unmetDependencies = $derived(
    task.dependencies
      .map(depId => tasksMap.get(depId))
      .filter((t): t is Task => t !== undefined && ACTIVE_STATUSES.includes(t.status.status))
      .map((t) => t.name)
  );
  const isBlocked = $derived(unmetDependencies.length > 0);
  
  async function handleApprove(e: Event): Promise<void> {
    reviewLoading = true;
    try {
      await transitionTaskStatus(branch, planId, task.id, { status: 'merge-queue' });
      onTaskTransition(task.id, 'merge-queue');
    } catch (err) {
      console.error('Failed to approve task:', err);
    } finally {
      reviewLoading = false;
    }
  }

  async function handleRequestChanges(e: Event): Promise<void> {
    reviewLoading = true;
    try {
      await transitionTaskStatus(branch, planId, task.id, { status: 'reviewing' });
      onTaskTransition(task.id, 'reviewing');
    } catch (err) {
      console.error('Failed to request changes:', err);
    } finally {
      reviewLoading = false;
    }
  }
</script>

<div class="bg-bg-tertiary border border-border-default rounded-md p-2.5 transition-colors hover:border-accent-blue {isBlocked && 'border-l-[3px] border-l-accent-yellow'}">
  <RouterLink href={href} class="block">
    <div class="flex items-start justify-between gap-1 mb-1.5">
      <h4 class="text-[13px] font-semibold text-text-secondary leading-snug flex-1">{task.name}</h4>
      <StatusBadge status={taskStatus} type="task" />
    </div>
    
    {#if agentName}
      <div class="flex items-center gap-1.5 mt-1.5">
        <div class="w-4 h-4 rounded-full bg-accent-blue-subtle flex items-center justify-center text-[10px] text-accent-blue font-bold">
          {agentName.charAt(0).toUpperCase()}
        </div>
        <span class="text-[11px] text-text-muted">{agentName}</span>
      </div>
    {/if}
    
    {#if isBlocked}
      <div class="text-[10px] text-text-muted mt-1.5">⌛ Waiting on: {unmetDependencies.join(', ')}</div>
    {/if}
  </RouterLink>
  
  {#if columnStatus === 'waiting-manual-review'}
    <div class="flex items-center gap-2 mt-2 pt-2 border-t border-border-default">
      <button 
        class="text-xs px-2 py-1 bg-accent-green border border-accent-green text-white rounded font-medium hover:bg-accent-green/80 transition-colors disabled:opacity-50"
        onclick={handleApprove}
        disabled={reviewLoading}
      >
        {reviewLoading ? '...' : '✓ Approve'}
      </button>
      <button 
        class="text-xs px-2 py-1 bg-accent-yellow-subtle border border-accent-yellow text-accent-yellow rounded font-medium hover:bg-accent-yellow/20 transition-colors disabled:opacity-50"
        onclick={handleRequestChanges}
        disabled={reviewLoading}
      >
        {reviewLoading ? '...' : '↩ Request Changes'}
      </button>
    </div>
  {/if}
</div>
