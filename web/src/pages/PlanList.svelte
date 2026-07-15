<script lang="ts">
  import { onMount } from 'svelte';
  import PlanCard from '../components/PlanCard.svelte';
  import CreatePlanForm from '../components/CreatePlanForm.svelte';
  import { listPlans, createPlan } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { Plan, CreatePlanRequest } from '$lib/types';
  
  const ACTIVE_PLAN_STATUSES = ['approved', 'planning', 'reviewing', 'queued'] as const;

  let planList = $state<Plan[]>([]);
  let totalFromApi = $state(0);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let errorCleanup = $state<(() => void) | null>(null);
  let showCreateForm = $state(false);

  async function loadPlans() {
    loading = true;
    try {
      const response = await listPlans();
      planList = response.items;
      totalFromApi = response.total;
    } catch (e) {
      if (e instanceof Error) {
        errorCleanup = setError(() => { error = e.message; }, () => { error = null; });
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => loadPlans());

  $effect(() => {
    if (!showCreateForm) return;
    function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        showCreateForm = false;
      }
    }
    document.addEventListener('keydown', handleKeydown);
    return () => document.removeEventListener('keydown', handleKeydown);
  });

  $effect(() => {
    if (errorCleanup) {
      return errorCleanup;
    }
  });

  const totalPlans = $derived(totalFromApi);
  const activePlans = $derived(planList.filter(p => ACTIVE_PLAN_STATUSES.includes(p.status)).length);
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
    <button type="button" class="px-4 py-2 bg-btn-green hover:bg-btn-green-hover text-white rounded-md text-sm font-medium transition-colors cursor-pointer" onclick={() => showCreateForm = true}>
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
  
  {#if error}
    <div class="mb-4 p-3 bg-accent-red-subtle border border-accent-red/30 rounded-md text-accent-red text-sm">
      {error}
    </div>
  {/if}
  
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

{#if showCreateForm}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-bg-primary/80 backdrop-blur-sm" onclick={() => showCreateForm = false}>
    <div class="bg-bg-secondary border border-border-default rounded-lg shadow-2xl w-full max-w-lg mx-4" onclick={(e) => e.stopPropagation()}>
      <CreatePlanForm
        onSubmit={async (data: CreatePlanRequest) => {
          try {
            const newPlan = await createPlan(data);
            planList.unshift(newPlan);
            totalFromApi += 1;
            showCreateForm = false;
          } catch (e) {
            if (e instanceof Error) {
              errorCleanup = setError(
                () => { error = e.message; },
                () => { error = null; }
              );
            }
          }
        }}
        onCancel={() => showCreateForm = false}
      />
    </div>
  </div>
{/if}
