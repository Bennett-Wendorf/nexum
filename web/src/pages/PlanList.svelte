<script lang="ts">
  import { onMount } from 'svelte';
  import PlanCard from '../components/PlanCard.svelte';
  import { listPlans } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { Plan } from '$lib/types';
  
  const ACTIVE_PLAN_STATUSES = ['approved', 'planning', 'reviewing', 'queued'] as const;

  let planList = $state<Plan[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  
  onMount(async () => {
    loading = true;
    try {
      const response = await listPlans();
      planList = response.items;
    } catch (e) {
      if (e instanceof Error) {
        setError(() => { error = e.message; }, () => { error = null; });
      }
    } finally {
      loading = false;
    }
  });
  
  const totalPlans = $derived(planList.length);
  const activePlans = $derived(planList.filter(p => Array.from(ACTIVE_PLAN_STATUSES).includes(p.status)).length);
  const totalTasks = $derived(planList.reduce((sum, p) => sum + (p.tasks?.length ?? 0), 0));
  const completedTasks = $derived(planList.reduce((sum, p) => sum + (p.tasks?.filter(t => t.completed).length ?? 0), 0));
</script>

<div class="p-6">
  <!-- Page Header -->
  <div class="flex justify-between items-center mb-8">
    <div>
      <h1 class="text-2xl font-bold text-text-primary">Plans</h1>
      <p class="text-text-muted text-sm mt-1">Manage your AI agent orchestration plans</p>
    </div>
    <button disabled class="px-4 py-2 bg-bg-secondary border border-border-default text-text-muted rounded-md text-sm font-medium cursor-not-allowed" title="Coming soon">
      + New Plan
    </button>
  </div>
  
  <!-- Stats Row -->
  <div class="grid grid-cols-4 gap-4 mb-8">
    <div class="bg-bg-secondary border border-border-default rounded-lg p-5">
      <div class="text-xs text-text-muted uppercase tracking-wider">Total Plans</div>
      <div class="text-3xl font-bold mt-2 text-text-primary">{totalPlans}</div>
    </div>
    <div class="bg-bg-secondary border border-border-default rounded-lg p-5">
      <div class="text-xs text-text-muted uppercase tracking-wider">Active</div>
      <div class="text-3xl font-bold mt-2 text-accent-blue">{activePlans}</div>
    </div>
    <div class="bg-bg-secondary border border-border-default rounded-lg p-5">
      <div class="text-xs text-text-muted uppercase tracking-wider">Total Tasks</div>
      <div class="text-3xl font-bold mt-2 text-text-primary">{totalTasks}</div>
    </div>
    <div class="bg-bg-secondary border border-border-default rounded-lg p-5">
      <div class="text-xs text-text-muted uppercase tracking-wider">Completed</div>
      <div class="text-3xl font-bold mt-2 text-accent-green">{completedTasks}</div>
    </div>
  </div>
  
  <!-- Loading State -->
  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="text-text-muted">Loading plans...</div>
    </div>
  
  <!-- Empty State -->
  {:else if planList.length === 0}
    <div class="flex flex-col items-center justify-center py-12 text-text-muted">
      <p class="text-lg mb-2">No plans yet</p>
      <p class="text-sm">Create your first plan to get started</p>
    </div>
  
  <!-- Plan Grid -->
  {:else}
    <div class="grid grid-cols-2 gap-4">
      {#each planList as plan (plan.id)}
        <PlanCard {plan} />
      {/each}
    </div>
  {/if}
</div>
