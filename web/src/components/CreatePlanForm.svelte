<script lang="ts">
  import type { CreatePlanRequest } from '$lib/types';

  let { onSubmit, onCancel }: { onSubmit: (data: CreatePlanRequest) => Promise<void>; onCancel: () => void } = $props();

  // Form state
  let name = $state('');
  let branch = $state('');
  let goal = $state('');
  let scope = $state('');
  let background = $state('');

  // Validation state
  let errors = $state<Record<string, string>>({});

  // Loading state
  let submitting = $state(false);

  function validate(): boolean {
    errors = {};
    let valid = true;

    const trimmedName = name.trim();
    const trimmedBranch = branch.trim();
    const trimmedGoal = goal.trim();

    if (!trimmedName) {
      errors.name = 'Plan name is required';
      valid = false;
    }
    if (!trimmedBranch) {
      errors.branch = 'Branch is required';
      valid = false;
    } else if (!/^[a-zA-Z0-9_\/.-]+$/.test(trimmedBranch)) {
      errors.branch = 'Invalid branch name (alphanumeric, -, _, ., / only)';
      valid = false;
    }
    if (!trimmedGoal) {
      errors.goal = 'Goal is required';
      valid = false;
    }

    return valid;
  }

  async function handleSubmit() {
    if (!validate()) return;

    submitting = true;
    try {
      const data: CreatePlanRequest = {
        name: name.trim(),
        branch: branch.trim(),
        goal: goal.trim(),
        scope: scope.trim() || undefined,
        background: background.trim() || undefined,
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
    <h2 class="text-lg font-semibold text-text-primary">Create New Plan</h2>
    <button type="button" aria-label="Close" class="text-text-muted hover:text-text-primary cursor-pointer" onclick={onCancel}>
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
    </button>
  </div>

  <!-- Form Fields -->
  <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
    <!-- Plan Name -->
    <div class="mb-4">
      <label for="plan-name" class="block text-text-secondary text-sm font-medium mb-1">Plan Name</label>
      <input
        id="plan-name"
        type="text"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="e.g., Implement authentication"
        bind:value={name}
      />
      {#if errors.name}
        <p class="text-accent-red text-xs mt-1">{errors.name}</p>
      {/if}
    </div>

    <!-- Branch -->
    <div class="mb-4">
      <label for="branch" class="block text-text-secondary text-sm font-medium mb-1">Branch</label>
      <input
        id="branch"
        type="text"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="e.g., main"
        bind:value={branch}
      />
      {#if errors.branch}
        <p class="text-accent-red text-xs mt-1">{errors.branch}</p>
      {/if}
    </div>

    <!-- Goal -->
    <div class="mb-4">
      <label for="goal" class="block text-text-secondary text-sm font-medium mb-1">Goal</label>
      <textarea
        id="goal"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full min-h-[80px] resize-y focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Describe what this plan aims to achieve"
        bind:value={goal}
      ></textarea>
      {#if errors.goal}
        <p class="text-accent-red text-xs mt-1">{errors.goal}</p>
      {/if}
    </div>

    <!-- Scope -->
    <div class="mb-4">
      <label for="scope" class="block text-text-secondary text-sm font-medium mb-1">Scope</label>
      <textarea
        id="scope"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full min-h-[80px] resize-y focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Define the boundaries of this plan"
        bind:value={scope}
      ></textarea>
    </div>

    <!-- Background -->
    <div class="mb-4">
      <label for="background" class="block text-text-secondary text-sm font-medium mb-1">Background</label>
      <textarea
        id="background"
        class="bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full min-h-[80px] resize-y focus:border-accent-blue focus:outline-none transition-colors"
        placeholder="Context or background information"
        bind:value={background}
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
