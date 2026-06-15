# Plan: 008.2 - Migrate Svelte Stores to Component-Local State

## Task Description
Replace global Svelte writable stores with component-local `$state` and prop-based data flow. The current `store.ts` module exports 10 writable stores that are mutated directly from components via `.set()`, creating tightly coupled components and unpredictable re-rendering behavior. This is an anti-pattern in Svelte 5 which favors component-local state and explicit data flow.

## Objective
Eliminate the global mutable store anti-pattern by migrating all store consumers to use component-local reactive state (`$state`/`let`), a lightweight service layer for error handling, and localStorage for persistent preferences.

## Problem Statement
The `store.ts` module exports writable stores (`plans`, `tasks`, `loading`, `error`, `viewMode`, `currentPlan`, `currentBranch`, `currentTask`, `runningTasks`, `agents`) that are mutated directly from components. Analysis reveals:

- **6 of 10 stores are completely unused**: `currentPlan`, `currentBranch`, `currentTask`, `runningTasks`, `agents`, and `tasks` are exported but no page or component reads or writes them.
- **`plans` store is written but never read**: `PlanList.svelte` calls `plans.set(planList)` but uses a local `planList` variable for rendering.
- **`loading` and `error` stores are shared but page-scoped**: All 4 pages use `loading.set()` and `setError()` but each page manages its own independent loading/error state — there's no cross-page dependency.
- **`viewMode` is single-page-scoped**: Only `TaskList.svelte` uses `viewMode`, and it's a per-page preference with no cross-page sharing.

This creates tight coupling, unpredictable re-renders, and dead code.

## Solution Approach
Three-pronged migration:

1. **Component-local state**: Replace `loading` store usage with local `let loading = false` in each page. Replace `error` store usage with local `let error = $state<string | null>(null)`.
2. **Error utility function**: Extract `setError()` and `clearError()` logic into a standalone utility module (`$lib/errorUtils.ts`) that returns a reactive error state object — no global store.
3. **localStorage persistence**: Replace `viewMode` store with localStorage-backed preference in `TaskList.svelte`.
4. **Delete `store.ts`**: After all consumers are migrated, remove the file entirely.

## Relevant Files

### Files to Modify
- `web/src/pages/PlanList.svelte` — Remove store imports, add local `loading`/`error` state
- `web/src/pages/PlanDetail.svelte` — Remove store imports, add local `loading`/`error` state
- `web/src/pages/TaskDetail.svelte` — Remove store imports, add local `loading`/`error` state
- `web/src/pages/TaskList.svelte` — Remove store imports, add local `loading`/`error` state, migrate `viewMode` to localStorage

### Files to Create
- `web/src/lib/errorUtils.ts` — Lightweight error handling utility (replaces `setError`/`clearError` from store)

### Files to Delete
- `web/src/lib/store.ts` — Removed after all consumers are migrated

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: stores-migration-builder
  - Role: Implement component-local state migration, create errorUtils utility, delete store.ts
  - Agent: builder

- **Validator**
  - Name: stores-migration-validator
  - Role: Verify all store references are eliminated, verify each page renders correctly, verify viewMode persistence works
  - Agent: validator

- **Documenter**
  - Name: stores-migration-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Create error handling utility module
- **Task ID**: create-error-utils
- **Depends On**: none
- **Assigned To**: stores-migration-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/lib/errorUtils.ts` with:
    - `createErrorState()` function that returns `{ value: $state<string | null>(null), setError(message: string): ReturnType<typeof setTimeout>, clearError(): void }`
    - The `setError` method sets the value and starts a 5-second auto-clear timeout
    - The `clearError` method clears the value and cancels any pending timeout
    - This pattern allows each component to create its own isolated error state
  - Export the function
- **Acceptance Criteria**:
  - `errorUtils.ts` compiles without errors
  - `createErrorState()` returns an object with `value`, `setError`, and `clearError`
  - The auto-clear timeout fires after 5 seconds
  - `clearError()` cancels the auto-clear timeout

### 2. Migrate PlanList.svelte
- **Task ID**: migrate-plan-list
- **Depends On**: create-error-utils
- **Assigned To**: stores-migration-builder
- **Agent**: builder
- **Actions**:
  - Remove `import { plans, loading, setError } from '$lib/store'`
  - Add `let loading = false` (local state)
  - Add `const errorState = createErrorState()` from `$lib/errorUtils`
  - Replace `loading.set(true/false)` with `loading = true/false`
  - Replace `setError(e.message)` with `errorState.setError(e.message)`
  - Remove the line `plans.set(planList)` (dead code — `plans` store is never read)
  - In the template, replace `{#if loading}` with the local variable (same syntax)
  - Add error toast/notification display using `$errorState.value` if needed (or keep errors silent as current behavior suggests)
- **Before**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import PlanCard from '../components/PlanCard.svelte';
  import { listPlans } from '$lib/api';
  import { plans, loading, setError } from '$lib/store';
  import type { Plan } from '$lib/types';
  
  let planList: Plan[] = [];
  
  onMount(async () => {
    loading.set(true);
    try {
      const response = await listPlans();
      planList = response.items;
      plans.set(planList);
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      }
    } finally {
      loading.set(false);
    }
  });
</script>
```
- **After**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import PlanCard from '../components/PlanCard.svelte';
  import { listPlans } from '$lib/api';
  import { createErrorState } from '$lib/errorUtils';
  import type { Plan } from '$lib/types';
  
  let loading = false;
  let planList: Plan[] = [];
  const errorState = createErrorState();
  
  onMount(async () => {
    loading = true;
    try {
      const response = await listPlans();
      planList = response.items;
    } catch (e) {
      if (e instanceof Error) {
        errorState.setError(e.message);
      }
    } finally {
      loading = false;
    }
  });
</script>
```
- **Acceptance Criteria**:
  - No imports from `$lib/store`
  - Page loads and renders plans correctly
  - Loading state works during data fetch
  - Error state is set on failure

### 3. Migrate PlanDetail.svelte
- **Task ID**: migrate-plan-detail
- **Depends On**: create-error-utils
- **Assigned To**: stores-migration-builder
- **Agent**: builder
- **Actions**:
  - Remove `import { loading, setError } from '$lib/store'`
  - Add `let loading = false` (local state)
  - Add `const errorState = createErrorState()` from `$lib/errorUtils`
  - Replace `loading.set(true/false)` with `loading = true/false`
  - Replace `setError(e.message)` with `errorState.setError(e.message)`
- **Before**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  // ... other imports ...
  import { loading, setError } from '$lib/store';
  
  let plan: Plan | null = null;
  let tasks: Task[] = [];
  
  onMount(async () => {
    loading.set(true);
    try {
      plan = await getPlan(branch, planId);
      const response = await listTasks(branch, planId);
      tasks = response.items;
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      }
    } finally {
      loading.set(false);
    }
  });
</script>
```
- **After**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  // ... other imports ...
  import { createErrorState } from '$lib/errorUtils';
  
  let loading = false;
  let plan: Plan | null = null;
  let tasks: Task[] = [];
  const errorState = createErrorState();
  
  onMount(async () => {
    loading = true;
    try {
      plan = await getPlan(branch, planId);
      const response = await listTasks(branch, planId);
      tasks = response.items;
    } catch (e) {
      if (e instanceof Error) {
        errorState.setError(e.message);
      }
    } finally {
      loading = false;
    }
  });
</script>
```
- **Acceptance Criteria**:
  - No imports from `$lib/store`
  - Page loads and renders plan detail correctly
  - Loading state works during data fetch
  - Error state is set on failure

### 4. Migrate TaskDetail.svelte
- **Task ID**: migrate-task-detail
- **Depends On**: create-error-utils
- **Assigned To**: stores-migration-builder
- **Agent**: builder
- **Actions**:
  - Remove `import { loading, setError } from '$lib/store'`
  - Add `let loading = false` (local state)
  - Add `const errorState = createErrorState()` from `$lib/errorUtils`
  - Replace `loading.set(true/false)` with `loading = true/false`
  - Replace `setError(e.message)` with `errorState.setError(e.message)` in both `onMount` and `handleTransition`
- **Before**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  // ... other imports ...
  import { loading, setError } from '$lib/store';
  
  let task: Task | null = null;
  
  onMount(async () => {
    loading.set(true);
    try {
      task = await getTask(branch, planId, taskId);
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      }
    } finally {
      loading.set(false);
    }
  });
  
  async function handleTransition(newStatus: string): Promise<void> {
    // ...
    try {
      task = await transitionTaskStatus(branch, planId, taskId, { status: newStatus });
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      }
    }
    // ...
  }
</script>
```
- **After**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  // ... other imports ...
  import { createErrorState } from '$lib/errorUtils';
  
  let loading = false;
  let task: Task | null = null;
  const errorState = createErrorState();
  
  onMount(async () => {
    loading = true;
    try {
      task = await getTask(branch, planId, taskId);
    } catch (e) {
      if (e instanceof Error) {
        errorState.setError(e.message);
      }
    } finally {
      loading = false;
    }
  });
  
  async function handleTransition(newStatus: string): Promise<void> {
    // ...
    try {
      task = await transitionTaskStatus(branch, planId, taskId, { status: newStatus });
    } catch (e) {
      if (e instanceof Error) {
        errorState.setError(e.message);
      }
    }
    // ...
  }
</script>
```
- **Acceptance Criteria**:
  - No imports from `$lib/store`
  - Page loads and renders task detail correctly
  - Loading state works during data fetch
  - Error state is set on both initial load and status transition failure

### 5. Migrate TaskList.svelte
- **Task ID**: migrate-task-list
- **Depends On**: create-error-utils
- **Assigned To**: stores-migration-builder
- **Agent**: builder
- **Actions**:
  - Remove `import { loading, setError, viewMode } from '$lib/store'`
  - Add `let loading = false` (local state)
  - Add `const errorState = createErrorState()` from `$lib/errorUtils`
  - Replace `loading.set(true/false)` with `loading = true/false`
  - Replace `setError(e.message)` with `errorState.setError(e.message)`
  - Replace `$viewMode` store with localStorage-backed local state:
    - `let viewMode = $state<'kanban' | 'list'>(localStorage.getItem('nexum-viewMode') as 'kanban' | 'list' ?? 'kanban')`
    - Update `toggleView()` to write to localStorage: `localStorage.setItem('nexum-viewMode', viewMode)`
    - Replace `$viewMode` references in template with `viewMode`
- **Before**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  // ... other imports ...
  import { loading, setError, viewMode } from '$lib/store';
  
  function toggleView(): void {
    viewMode.set($viewMode === 'kanban' ? 'list' : 'kanban');
  }
</script>

<!-- Template uses $viewMode -->
<button on:click={toggleView}>Kanban</button>
{#if $viewMode === 'kanban'}
```
- **After**:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  // ... other imports ...
  import { createErrorState } from '$lib/errorUtils';
  
  let loading = false;
  let viewMode = $state<'kanban' | 'list'>(
    localStorage.getItem('nexum-viewMode') as 'kanban' | 'list' ?? 'kanban'
  );
  const errorState = createErrorState();
  
  function toggleView(): void {
    viewMode = viewMode === 'kanban' ? 'list' : 'kanban';
    localStorage.setItem('nexum-viewMode', viewMode);
  }
</script>

<!-- Template uses viewMode directly -->
<button on:click={toggleView}>Kanban</button>
{#if viewMode === 'kanban'}
```
- **Acceptance Criteria**:
  - No imports from `$lib/store`
  - Page loads and renders task list correctly
  - Loading state works during data fetch
  - View mode toggle works and persists across page reloads
  - Error state is set on failure

### 6. Delete store.ts
- **Task ID**: delete-store
- **Depends On**: migrate-plan-list, migrate-plan-detail, migrate-task-detail, migrate-task-list
- **Assigned To**: stores-migration-builder
- **Agent**: builder
- **Actions**:
  - Verify no remaining imports of `$lib/store` exist in the codebase (grep for `from '\$lib/store'`)
  - Delete `web/src/lib/store.ts`
- **Acceptance Criteria**:
  - `grep "from.*store" web/src/` returns no results (except `svelte/store` if any)
  - `store.ts` file no longer exists
  - Project builds without errors

### 7. Final Validation
- **Task ID**: validate-all
- **Depends On**: delete-store
- **Assigned To**: stores-migration-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npm run check` — TypeScript/Svelte type checking passes
  - Run `cd web && npm run build` — Production build succeeds
  - Run `cd web && npm test` — All existing tests pass
  - Run `grep -r "\$lib/store" web/src/` — No remaining imports
  - Run `grep -r "from.*store" web/src/` — No remaining imports (except `svelte/store` in errorUtils if needed)
  - Verify `store.ts` is deleted
  - Manual verification: each page renders correctly with loading states
  - Manual verification: viewMode toggle persists in TaskList after reload

### 8. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: stores-migration-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/` describing the migration
  - Document the `errorUtils.ts` API

## Acceptance Criteria
1. `web/src/lib/store.ts` is deleted
2. No files in `web/src/` import from `$lib/store`
3. All 4 page files use component-local `loading` state
4. All 4 page files use `createErrorState()` from `$lib/errorUtils` for error handling
5. `TaskList.svelte` uses localStorage for `viewMode` persistence
6. `web/src/lib/errorUtils.ts` exists and exports `createErrorState()`
7. `npm run check` passes with no type errors
8. `npm run build` succeeds
9. All existing tests pass
10. No regression in UI behavior (loading states, error display, view mode toggle)

## Validation Commands
- `cd web && npm run check` — Type check
- `cd web && npm run build` — Production build
- `cd web && npm test` — Run test suite
- `grep -r "\$lib/store" web/src/` — Verify no remaining store imports
- `grep -r "from.*store" web/src/` — Verify no remaining store references
- `test -f web/src/lib/store.ts && echo "FAIL: store.ts still exists" || echo "PASS: store.ts deleted"`

## Notes
- **Why not Svelte Context API?** The stores being replaced are not shared across component hierarchies — each page manages its own independent loading/error state. Context adds overhead without benefit here.
- **Why not a service layer?** A service layer would be appropriate for shared business logic (e.g., data fetching, authentication). Error handling is a cross-cutting concern that's best handled by a lightweight factory function (`createErrorState`) that each component instantiates locally.
- **Unused stores**: `currentPlan`, `currentBranch`, `currentTask`, `runningTasks`, `agents`, and `tasks` are deleted without migration since no code references them. If future features need these, they should be re-introduced with proper architecture (e.g., a service layer or context).
- **Error display**: The current codebase does not appear to have a global error toast/notification component. The `setError()` function sets a store value but there's no visible UI that reads `$error`. The migration preserves this behavior — errors are captured but not displayed. If error display is needed, a separate plan should address it.
- **viewMode persistence key**: Using `nexum-viewMode` as the localStorage key to avoid collisions.
