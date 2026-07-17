<script lang="ts">
  import type { CreateTaskRequest } from '$lib/types';

  let { onSubmit, onCancel, parentPlan }: { onSubmit: (data: CreateTaskRequest) => Promise<void>; onCancel: () => void; parentPlan: string } = $props();

  // Form state
  let name = $state('');
  let description = $state('');
  let dependencies = $state('');
  let acceptanceCriteria = $state('');
  let filesToModify = $state('');
  let background = $state('');
  let notes = $state('');

  // Validation state
  let errors = $state<Record<string, string>>({});

  // Loading state
  let submitting = $state(false);

  function validate(): boolean {
    errors = {};
    let valid = true;

    const trimmedName = name.trim();
    const trimmedDescription = description.trim();

    if (!trimmedName) {
      errors.name = 'Task name is required';
      valid = false;
    }
    if (!trimmedDescription) {
      errors.description = 'Description is required';
      valid = false;
    }

    return valid;
  }

  function parseCommaSeparated(value: string): string[] {
    return value
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
  }

  function parseLineSeparated(value: string): string[] {
    return value
      .split('\n')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
  }

  async function handleSubmit() {
    if (!validate()) return;

    submitting = true;
    try {
      const data: CreateTaskRequest = {
        name: name.trim(),
        parent_plan: parentPlan,
        dependencies: parseCommaSeparated(dependencies),
        description: description.trim(),
        acceptance_criteria: parseLineSeparated(acceptanceCriteria),
        files_to_modify: parseCommaSeparated(filesToModify),
        background: background.trim() || undefined,
        notes: notes.trim() || undefined,
      };
      await onSubmit(data);
    } finally {
      submitting = false;
    }
  }
</script>

<div class="bg-bg-secondary border border-border-default rounded-lg p-6">
  <!-- Header -->
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-lg font-semibold text-text-primary">Create New Task</h2>
    <button type="button" aria-label="Close" class="text-text-muted hover:text-text-primary cursor-pointer" onclick={onCancel}>
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
    </button>
  </div>

  <!-- Form Fields -->
  <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
    <!-- Task Name -->
    <div class="mb-4">
      <label for="task-name" class="block text-text-secondary text-sm font-medium mb-1">Task Name</label>
      <input
        id="task-name"
        type="text"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="e.g., Implement login page"
        bind:value={name}
      />
      {#if errors.name}
        <p class="text-accent-red text-xs mt-1">{errors.name}</p>
      {/if}
    </div>

    <!-- Description -->
    <div class="mb-4">
      <label for="task-description" class="block text-text-secondary text-sm font-medium mb-1">Description</label>
      <textarea
        id="task-description"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full min-h-[80px] resize-y focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Describe what this task entails"
        bind:value={description}
      ></textarea>
      {#if errors.description}
        <p class="text-accent-red text-xs mt-1">{errors.description}</p>
      {/if}
    </div>

    <!-- Dependencies -->
    <div class="mb-4">
      <label for="dependencies" class="block text-text-secondary text-sm font-medium mb-1">Dependencies</label>
      <input
        id="dependencies"
        type="text"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Comma-separated task IDs (e.g., task-1, task-2)"
        bind:value={dependencies}
      />
    </div>

    <!-- Acceptance Criteria -->
    <div class="mb-4">
      <label for="acceptance-criteria" class="block text-text-secondary text-sm font-medium mb-1">Acceptance Criteria</label>
      <textarea
        id="acceptance-criteria"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full min-h-[80px] resize-y focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="One criterion per line"
        bind:value={acceptanceCriteria}
      ></textarea>
    </div>

    <!-- Files to Modify -->
    <div class="mb-4">
      <label for="files-to-modify" class="block text-text-secondary text-sm font-medium mb-1">Files to Modify</label>
      <input
        id="files-to-modify"
        type="text"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Comma-separated file paths (e.g., src/auth.ts, src/login.svelte)"
        bind:value={filesToModify}
      />
    </div>

    <!-- Background -->
    <div class="mb-4">
      <label for="task-background" class="block text-text-secondary text-sm font-medium mb-1">Background</label>
      <textarea
        id="task-background"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full min-h-[80px] resize-y focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Context or background information"
        bind:value={background}
      ></textarea>
    </div>

    <!-- Notes -->
    <div class="mb-4">
      <label for="notes" class="block text-text-secondary text-sm font-medium mb-1">Notes</label>
      <textarea
        id="notes"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full min-h-[80px] resize-y focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Additional notes"
        bind:value={notes}
      ></textarea>
    </div>

    <!-- Footer -->
    <div class="flex justify-end gap-3 mt-6">
      <button
        type="button"
        class="bg-bg-tertiary border border-border-default text-text-secondary hover:text-text-primary px-4 py-2 rounded-md text-sm font-medium transition-colors"
        onclick={onCancel}
      >
        Cancel
      </button>
      <button
        type="submit"
        class="bg-btn-green hover:bg-btn-green-hover text-white px-4 py-2 rounded-md text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={submitting}
      >
        {submitting ? 'Creating…' : 'Create'}
      </button>
    </div>
  </form>
</div>
