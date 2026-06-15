<script lang="ts">
  import StatusBadge from './StatusBadge.svelte';
  import type { Plan } from '$lib/types';
  
  let { plan }: { plan: Plan } = $props();
  
  const totalTasks = $derived(plan.tasks?.length ?? 0);
  const completedTasks = $derived(plan.tasks?.filter(t => t.completed).length ?? 0);
  const progressPercent = $derived(totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0);
  const href = $derived(`/plans/${plan.branch}/${plan.id}`);
</script>

<a href={href} class="block bg-bg-tertiary border border-border-default rounded-lg p-3 cursor-pointer transition-colors hover:border-accent-blue">
  <div class="flex items-start justify-between gap-2 mb-2">
    <h3 class="font-semibold text-text-secondary text-sm leading-tight">{plan.name}</h3>
    <StatusBadge status={plan.status} type="plan" />
  </div>
  
  <div class="flex items-center gap-2 mb-2">
    <span class="text-xs text-text-muted font-mono">{plan.branch}</span>
  </div>
  
  {#if plan.goal}
    <p class="text-xs text-text-muted line-clamp-2 mb-2">{plan.goal}</p>
  {/if}
  
  {#if totalTasks > 0}
    <div class="flex items-center gap-2">
      <div class="flex-1 h-2 bg-border-muted rounded-full overflow-hidden">
        <div class="h-full bg-accent-green rounded-full transition-all" style="width: {progressPercent}%"></div>
      </div>
      <span class="text-[11px] text-text-muted">{completedTasks}/{totalTasks}</span>
    </div>
  {/if}
</a>
