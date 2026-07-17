<script module lang="ts">
  import type { PermissionLevel, LogEntry } from '$lib/types';

  const PERMISSION_BADGE_CLASSES: Record<PermissionLevel, string> = {
    auto: 'bg-accent-green-subtle text-accent-green',
    manual: 'bg-accent-yellow-subtle text-accent-yellow',
    blocked: 'bg-accent-red-subtle text-accent-red',
  };

  const SEVERITY_COLORS: Record<LogEntry['severity'], string> = {
    info: 'text-accent-blue',
    success: 'text-accent-green',
    warn: 'text-accent-yellow',
    error: 'text-accent-red',
  };
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import Breadcrumb from '../components/Breadcrumb.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import MarkdownRenderer from '../components/MarkdownRenderer.svelte';
  import RouterLink from '../components/RouterLink.svelte';
  import { getTask, transitionTaskStatus, getConfig, getTaskLogs, listAgents } from '$lib/api';
  import { setError } from '$lib/errorUtils';
  import type { Task, Config, LogEntry, TaskPermissions, PermissionLevel } from '$lib/types';
  
  const LOG_POLL_INTERVAL_MS = 3000;

  interface LogEntryWithTime extends LogEntry {
    formattedTime: string;
  }

  const PERMISSION_KEYS: Array<keyof TaskPermissions> = [
    'file_writes',
    'terminal_commands',
    'network_requests',
    'git_operations',
    'package_installs',
  ];

  let { branch, planId, taskId }: { branch: string; planId: string; taskId: string } = $props();
  
  const DEFAULT_TAB: 'definition' | 'agent-output' | 'history' = 'definition';

  let task = $state<Task | null>(null);
  let config = $state<Config | null>(null);
  let activeTab = $state<typeof DEFAULT_TAB>(DEFAULT_TAB);
  let loading = $state(false);
  let executing = $state(false);
  let abandoning = $state(false);
  let error = $state<string | null>(null);
  let errorCleanup: (() => void) | null = null;
  let loadFailed = $state(false);
  let logEntries = $state<LogEntryWithTime[]>([]);
  let logLoading = $state(false);
  let logError = $state<string | null>(null);
  let permissions = $state<TaskPermissions | null>(null);

  const SPINNER_SVG = `<svg class="animate-spin h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
</svg>`;

  // Format timestamp for display
  function formatLogTime(timestamp: string): string {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
  }

  async function loadPermissions(): Promise<void> {
    // TODO: Implement granular mapping of agent tool_permissions to permission categories.
    // For now, permissions are not displayed to avoid showing fabricated values.
    permissions = null;
  }

  onMount(async () => {
    loading = true;
    try {
      [task, config] = await Promise.all([
        getTask(branch, planId, taskId),
        getConfig(),
      ]);
      // Load permissions from agent config
      await loadPermissions();
      // Fetch logs after task data is loaded, but only if status suggests logs exist
      const taskStatus = task?.status.status;
      if (taskStatus === 'running' || taskStatus === 'reviewing' || taskStatus === 'completed' || taskStatus === 'abandoned') {
        await fetchLogs();
      }
    } catch (e) {
      if (e instanceof Error) {
        errorCleanup = setError(() => { error = e.message; }, () => { error = null; });
        loadFailed = true;
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

  const breadcrumbItems = $derived([
    { label: 'Plans', href: '/plans' },
    { label: task?.name ?? taskId, href: `/plans/${branch}/${planId}` },
    { label: 'Tasks', href: `/plans/${branch}/${planId}/tasks` },
    { label: task?.name ?? taskId },
  ]);

  const babyStepMode = $derived(config !== null && !config.yolo_mode && !loading);

  const isTaskRunning = $derived(task?.status.status === 'running');

  async function handleTransition(newStatus: string): Promise<void> {
    if (!newStatus) return;
    if (!task) return;
    if (newStatus === 'running') executing = true;
    if (newStatus === 'abandoned') abandoning = true;
    try {
      task = await transitionTaskStatus(branch, planId, taskId, { status: newStatus });
    } catch (e) {
      if (e instanceof Error) {
        errorCleanup = setError(() => { error = e.message; }, () => { error = null; });
      }
    } finally {
      executing = false;
      abandoning = false;
    }
  }

  function handleStatusChange(e: Event): void {
    const target = e.target as HTMLSelectElement | null;
    if (target instanceof HTMLSelectElement) {
      handleTransition(target.value);
    }
  }

  async function fetchLogs(signal?: AbortSignal): Promise<void> {
    if (!task) return;
    try {
      logLoading = true;
      logError = null;
      const entries = await getTaskLogs(branch, planId, taskId, { signal });
      logEntries = entries.map(e => ({ ...e, formattedTime: formatLogTime(e.timestamp) }));
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      if (e instanceof Error) {
        logError = e.message;
      }
    } finally {
      logLoading = false;
    }
  }

  // Ref for auto-scrolling the log container
  let logContainer: HTMLDivElement | undefined;

  function scrollToBottom(): void {
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  }

  $effect(() => {
    // Auto-scroll when log entries change
    if (logEntries.length > 0) {
      queueMicrotask(scrollToBottom);
    }
  });

  $effect(() => {
    // Manage polling interval — only poll when task is running AND user is viewing the Agent Output tab
    if (isTaskRunning && task && activeTab === 'agent-output') {
      const controller = new AbortController();
      const interval = setInterval(() => fetchLogs(controller.signal), LOG_POLL_INTERVAL_MS);
      return () => {
        clearInterval(interval);
        controller.abort();
      };
    }
  });
</script>

<div class="p-6">
  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="text-text-muted">Loading task...</div>
    </div>
  {:else if loadFailed}
    <div class="flex flex-col items-center justify-center py-12 text-text-muted">
      <p class="text-lg">Failed to load task</p>
    </div>
  {:else if !task}
    <div class="flex flex-col items-center justify-center py-12 text-text-muted">
      <p class="text-lg">Task not found</p>
    </div>
  {:else}
    {#if error}
      <div class="mb-4 p-3 bg-accent-red-subtle border border-accent-red/30 rounded-md text-accent-red text-sm">
        {error}
      </div>
    {/if}
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
      <div class="flex flex-col items-end gap-2">
        <div class="flex items-center gap-2">
          <button type="button" class="bg-accent-blue border border-accent-blue text-white hover:bg-accent-blue/80 rounded-md text-sm px-3 py-1.5 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  onclick={() => handleTransition('running')}
                  disabled={executing}>
            {#if executing}
              <span class="inline-flex items-center gap-1.5">
                {@html SPINNER_SVG}
                Executing...
              </span>
            {:else}
              ▶ Execute
            {/if}
          </button>
          <button type="button" class="bg-accent-red border border-accent-red text-white hover:bg-accent-red/80 rounded-md text-sm px-3 py-1.5 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  title="Abandon this task. This action cannot be undone."
                  onclick={() => handleTransition('abandoned')}
                  disabled={abandoning}>
            {#if abandoning}
              <span class="inline-flex items-center gap-1.5">
                {@html SPINNER_SVG}
                Abandoning...
              </span>
            {:else}
              Abandon
            {/if}
          </button>
        </div>
        <select class="px-3 py-1.5 border border-border-default bg-bg-secondary text-accent-blue rounded-md text-sm cursor-pointer"
                onchange={handleStatusChange}
                disabled={executing || abandoning}>
          <option value="">Change status...</option>
          <option value="queued">Queued</option>
          <option value="reviewing">Reviewing</option>
          <option value="waiting-manual-review">Manual Review</option>
          <option value="merge-queue">Merge Queue</option>
          <option value="completed">Completed</option>
        </select>
      </div>
    </div>
    
    <div class="flex gap-6">
      <!-- Main Content -->
      <div class="flex-1 min-w-0">
        <!-- Baby Step Mode Banner -->
        {#if babyStepMode}
          <div class="flex items-start gap-3 p-3 mb-4 bg-yellow-500/10 border border-yellow-500/30 rounded-md text-yellow-400">
            <span class="text-lg flex-shrink-0">🚂</span>
            <span class="text-sm">Baby Step Mode: Each status transition requires your confirmation before proceeding.</span>
          </div>
        {/if}
        
        <!-- Tabs -->
        <div class="flex border-b border-border-default mb-6">
          <button type="button" aria-label="Show task definition" class="px-4 py-2.5 text-sm border-b border-transparent {activeTab === 'definition' ? 'border-b-2 text-accent-blue border-accent-blue' : 'text-text-muted hover:text-text-secondary'} transition-colors"
                  onclick={() => activeTab = 'definition'}>
            Task Definition
          </button>
          <button type="button" aria-label="Show agent output" class="px-4 py-2.5 text-sm border-b border-transparent {activeTab === 'agent-output' ? 'border-b-2 text-accent-blue border-accent-blue' : 'text-text-muted hover:text-text-secondary'} transition-colors"
                  onclick={() => activeTab = 'agent-output'}>
            Agent Output
          </button>
          <button type="button" aria-label="Show status history" class="px-4 py-2.5 text-sm border-b border-transparent {activeTab === 'history' ? 'border-b-2 text-accent-blue border-accent-blue' : 'text-text-muted hover:text-text-secondary'} transition-colors"
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

        <!-- Agent Output Tab -->
        {:else if activeTab === 'agent-output'}
          <div>
            {#if logLoading && logEntries.length === 0}
              <div class="flex items-center justify-center py-8">
                <div class="text-text-muted text-sm">Loading logs...</div>
              </div>
            {:else if logEntries.length === 0}
              <div class="flex flex-col items-center justify-center py-8 text-text-muted">
                <p class="text-sm">No logs available</p>
                {#if task?.status.status === 'backlog'}
                  <p class="text-xs text-text-faint mt-1">Logs will appear once the task starts executing</p>
                {/if}
              </div>
            {:else}
              <div class="space-y-3">
                {#if !isTaskRunning}
                  <div class="text-xs text-text-faint italic">Task is not running — showing last known log</div>
                {/if}
                {#if logError}
                  <div class="p-2 bg-accent-red-subtle border border-accent-red/30 rounded-md text-accent-red text-xs">
                    Failed to fetch logs: {logError}
                  </div>
                {/if}
                <div class="bg-bg-primary border border-border-default rounded-lg max-h-96 overflow-y-auto font-mono text-xs"
                     bind:this={logContainer}>
                  <div class="p-3 space-y-1">
                    {#each logEntries as entry, i (i)}
                      <div class="flex items-start gap-2">
                        <span class="text-text-faint flex-shrink-0 w-16">{entry.formattedTime}</span>
                        <span class={SEVERITY_COLORS[entry.severity]}>{entry.message}</span>
                      </div>
                    {/each}
                  </div>
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

        {#if permissions}
          <!-- Permissions -->
          <div>
            <h3 class="text-xs font-semibold text-text-muted uppercase tracking-wider mb-3">Permissions</h3>
            {#each PERMISSION_KEYS as key}
              <div class="flex items-center justify-between py-2 text-sm border-b border-border-muted">
                <span class="text-text-secondary">{key.replace('_', ' ').replace(/^\w/, c => c.toUpperCase())}</span>
                <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold {PERMISSION_BADGE_CLASSES[permissions[key]]}">
                  {permissions[key]}
                </span>
              </div>
            {/each}
          </div>
        {/if}

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
