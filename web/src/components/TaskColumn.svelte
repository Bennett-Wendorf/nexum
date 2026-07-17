<script lang="ts">
  import TaskCard from './TaskCard.svelte';
  import { kanbanColumnLabels } from '$lib/statusColors';
  import type { Task } from '$lib/types';
  
  let { status, tasks, planId, branch, tasksMap, onTaskTransition }: { 
    status: string; 
    tasks: Task[]; 
    planId: string; 
    branch: string; 
    tasksMap: Map<string, Task>;
    onTaskTransition: (taskId: string, newStatus: string) => void;
  } = $props();
  
  const label = $derived(kanbanColumnLabels[status] ?? status);
  const count = $derived(tasks.length);
</script>

<div class="min-w-[260px] w-[260px] bg-bg-secondary border border-border-default rounded-lg flex flex-col flex-shrink-0">
  <!-- Header -->
  <div class="p-3 border-b border-border-default flex items-center justify-between">
    <h3 class="text-sm font-semibold text-text-secondary">{label}</h3>
    <span class="text-xs text-text-muted bg-border-default px-1.5 py-0.5 rounded-md">{count}</span>
  </div>
  
  <!-- Body -->
  <div class="p-2 overflow-y-auto flex-1 space-y-2">
    {#each tasks as task (task.id)}
      <TaskCard {task} {planId} {branch} {tasksMap} columnStatus={status} {onTaskTransition} />
    {/each}
    
    {#if tasks.length === 0}
      <p class="text-xs text-text-faint text-center py-4 italic">No tasks</p>
    {/if}
  </div>
</div>
