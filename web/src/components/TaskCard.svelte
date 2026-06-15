<script lang="ts">
  import StatusBadge from './StatusBadge.svelte';
  import type { Task } from '$lib/types';
  
  let { task, planId, branch }: { task: Task; planId: string; branch: string } = $props();
  
  const href = $derived(`/plans/${branch}/${planId}/tasks/${task.id}`);
  const agentName = $derived(task.status.agent?.role ?? null);
  const taskStatus = $derived(task.status.status);
</script>

<a href={href} class="block bg-bg-tertiary border border-border-default rounded-md p-2.5 cursor-pointer transition-colors hover:border-accent-blue">
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
</a>
