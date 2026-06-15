<script lang="ts">
  import { onMount } from 'svelte';
  import Breadcrumb from '../components/Breadcrumb.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import MarkdownRenderer from '../components/MarkdownRenderer.svelte';
  import RouterLink from '../components/RouterLink.svelte';
  import { getTask, transitionTaskStatus } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { Task } from '$lib/types';
  
  let { branch, planId, taskId }: { branch: string; planId: string; taskId: string } = $props();
  
  let task: Task | null = null;
  let activeTab: 'definition' | 'history' = 'definition';
  let loading = false;
  let transitioning = false;
  let error = $state<string | null>(null);
  
  onMount(async () => {
    loading = true;
    try {
      task = await getTask(branch, planId, taskId);
    } catch (e) {
      if (e instanceof Error) {
        setError(() => { error = e.message; }, () => { error = null; });
      }
    } finally {
      loading = false;
    }
  });
  
  const breadcrumbItems = $derived([
    { label: 'Plans', href: '/plans' },
    { label: task?.name ?? taskId, href: `/plans/${branch}/${planId}` },
    { label: 'Tasks', href: `/plans/${branch}/${planId}/tasks` },
    { label: task?.name ?? taskId },
  ]);
  
  async function handleTransition(newStatus: string): Promise<void> {
    if (!newStatus) return;
    if (!task) return;
    transitioning = true;
    try {
      task = await transitionTaskStatus(branch, planId, taskId, { status: newStatus });
    } catch (e) {
      if (e instanceof Error) {
        setError(() => { error = e.message; }, () => { error = null; });
      }
    } finally {
      transitioning = false;
    }
  }

  function handleStatusChange(e: Event): void {
    handleTransition((e.target as HTMLSelectElement).value);
  }
</script>

<div class="p-6">
  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="text-text-muted">Loading task...</div>
    </div>
  {:else if !task}
    <div class="flex flex-col items-center justify-center py-12 text-text-muted">
      <p class="text-lg">Task not found</p>
    </div>
  {:else}
    <!-- Top Bar -->
    <div class="flex items-center gap-4 mb-4">
      <RouterLink href={`/plans/${branch}/${planId}/tasks`} class="text-text-muted hover:text-text-secondary text-sm">← Back</RouterLink>
      <Breadcrumb items={breadcrumbItems} />
    </div>
    
    <!-- Header -->
    <div class="flex items-start justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold text-text-primary">{task.name}</h1>
        <div class="flex items-center gap-3 mt-2">
          <StatusBadge status={task.status.status} type="task" />
          <span class="text-sm text-text-muted">{task.dependencies.length} dependencies</span>
          <span class="text-sm text-text-muted">{task.files_to_modify.length} files</span>
        </div>
      </div>
      <select class="px-3 py-1.5 border border-border-default bg-bg-secondary text-accent-blue rounded-md text-sm cursor-pointer"
              onchange={handleStatusChange}
              disabled={transitioning}>
        <option value="">Change status...</option>
        <option value="queued">Queued</option>
        <option value="running">Running</option>
        <option value="reviewing">Reviewing</option>
        <option value="waiting-manual-review">Manual Review</option>
        <option value="merge-queue">Merge Queue</option>
        <option value="completed">Completed</option>
        <option value="abandoned">Abandoned</option>
      </select>
    </div>
    
    <div class="flex gap-6">
      <!-- Main Content -->
      <div class="flex-1 min-w-0">
        <!-- Tabs -->
        <div class="flex border-b border-border-default mb-6">
          <button class="px-4 py-2.5 text-sm border-b-2 border-transparent {activeTab === 'definition' ? 'text-accent-blue border-b-2 border-accent-blue' : 'text-text-muted hover:text-text-secondary'} transition-colors"
                  onclick={() => activeTab = 'definition'}>
            Task Definition
          </button>
          <button class="px-4 py-2.5 text-sm border-b-2 border-transparent {activeTab === 'history' ? 'text-accent-blue border-b-2 border-accent-blue' : 'text-text-muted hover:text-text-secondary'} transition-colors"
                  onclick={() => activeTab = 'history'}>
            Status History
          </button>
        </div>
        
        <!-- Definition Tab -->
        {#if activeTab === 'definition'}
          <div class="space-y-6">
            {#if task.description}
              <div>
                <h2 class="text-sm font-semibold text-text-secondary mb-2">Description</h2>
                <MarkdownRenderer content={task.description} />
              </div>
            {/if}
            
            {#if task.acceptance_criteria.length > 0}
              <div>
                <h2 class="text-sm font-semibold text-text-secondary mb-2">Acceptance Criteria</h2>
                <div>
                  <ul class="list-disc pl-5 space-y-1">
                    {#each task.acceptance_criteria as criterion (criterion)}
                      <li class="text-text-secondary text-sm">{criterion}</li>
                    {/each}
                  </ul>
                </div>
              </div>
            {/if}
            
            {#if task.background}
              <div>
                <h2 class="text-sm font-semibold text-text-secondary mb-2">Background</h2>
                <MarkdownRenderer content={task.background} />
              </div>
            {/if}
            
            {#if task.notes}
              <div>
                <h2 class="text-sm font-semibold text-text-secondary mb-2">Notes</h2>
                <MarkdownRenderer content={task.notes} />
              </div>
            {/if}
            
            {#if task.files_to_modify.length > 0}
              <div>
                <h2 class="text-sm font-semibold text-text-secondary mb-2">Files to Modify</h2>
                <div class="bg-bg-secondary border border-border-default rounded-lg p-3">
                  {#each task.files_to_modify as file (file)}
                    <div class="flex items-center gap-2 py-1.5 text-sm">
                      <svg class="w-4 h-4 text-accent-blue flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9a2 2 0 00-2-2h-5l-5 5v9a2 2 0 002 2zM9 5H7a2 2 0 00-2 2v4a2 2 0 002 2h2l3-3H9V7a2 2 0 00-2-2z"/></svg>
                      <span class="text-text-secondary font-mono text-xs">{file}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        
        <!-- History Tab -->
        {:else}
          <div class="pl-4 relative border-l-2 border-border-default">
            {#each task.status.transitions as transition (transition.at)}
              <div class="mb-4 pl-4 relative">
                <div class="absolute -left-[9px] top-1 w-2 h-2 rounded-full bg-border-default border-2 bg-bg-primary"></div>
                <div class="text-sm">
                  <span class="text-text-muted">{transition.from}</span>
                  <span class="text-border-default mx-1">→</span>
                  <span class="text-text-secondary font-semibold">{transition.to}</span>
                </div>
                <div class="text-xs text-text-faint mt-0.5">
                  by {transition.by} · {new Date(transition.at).toLocaleString()}
                </div>
              </div>
            {/each}
            
            {#if task.status.transitions.length === 0}
              <p class="text-text-muted italic text-sm">No transitions yet</p>
            {/if}
          </div>
        {/if}
      </div>
      
      <!-- Sidebar -->
      <div class="w-56 flex-shrink-0 space-y-6">
        <!-- Agent Assignment -->
        <div>
          <h3 class="text-xs font-semibold text-text-muted uppercase tracking-wider mb-3">Assigned Agent</h3>
          {#if task.status.agent}
            <div class="flex items-center gap-2.5 p-2.5 bg-bg-tertiary rounded-md">
              <div class="w-8 h-8 rounded-full bg-accent-blue-subtle flex items-center justify-center text-sm font-bold text-accent-blue">
                {task.status.agent.role.charAt(0).toUpperCase()}
              </div>
              <div>
                <div class="text-sm font-semibold text-text-secondary">{task.status.agent.role}</div>
                <div class="text-[11px] text-text-muted">PID: {task.status.agent.pid}</div>
              </div>
            </div>
          {:else}
            <p class="text-xs text-text-faint italic">Unassigned</p>
          {/if}
        </div>
        
        <!-- Dependencies -->
        <div>
          <h3 class="text-xs font-semibold text-text-muted uppercase tracking-wider mb-3">Dependencies</h3>
          {#if task.dependencies.length > 0}
            <div class="space-y-1">
              {#each task.dependencies as depId (depId)}
                <div class="text-xs text-text-muted font-mono bg-bg-secondary px-2 py-1 rounded">
                  {depId}
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-xs text-text-faint italic">No dependencies</p>
          {/if}
        </div>
        
        <!-- Status Info -->
        <div>
          <h3 class="text-xs font-semibold text-text-muted uppercase tracking-wider mb-3">Status Info</h3>
          <div class="text-xs space-y-1 text-text-muted">
            <div>Attempts: <span class="text-text-secondary">{task.status.attempts}</span></div>
            {#if task.status.started_at}
              <div>Started: <span class="text-text-secondary">{new Date(task.status.started_at).toLocaleString()}</span></div>
            {/if}
            {#if task.status.completed_at}
              <div>Completed: <span class="text-text-secondary">{new Date(task.status.completed_at).toLocaleString()}</span></div>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
