# Plan: 008.3 - State Rune Migration for Page Components

## Task Description
Fix non-reactive state variables in all four page components that break `$derived` reactivity. After the Svelte 5 runes migration (008.1) and stores migration (008.2), page components declare local state as plain `let` variables. In Svelte 5 runes mode, these are NOT reactive. The `onMount` callbacks reassign them, but since they are not `$state()`, the reassignments do not trigger re-renders. All `$derived` values that depend on these variables are computed once at initialization and never update.

## Objective
Replace all non-reactive `let` state declarations in page components with `$state()` so that reassignments trigger proper reactivity and all `$derived` values update correctly.

## Problem Statement

### Root Cause
In Svelte 5 runes mode, a plain `let` variable is NOT reactive. Reassigning it does not trigger component re-renders. The Svelte 5 compiler treats `let x = value` as a regular JavaScript variable, not as reactive state. Only `$state()` declarations create reactive state.

### Affected Variables (12 total across 4 files)

Plan 008.2 (stores migration) already converted `error` to `$state<string | null>(null)` in each page, so it does not need migration here. The `loading` variable and all data variables still use plain `let` and need `$state()`.

| File | Variable | Type | Impact |
|------|----------|------|--------|
| `PlanList.svelte` | `planList` | `Plan[]` | All 4 `$derived` stats values (totalPlans, activePlans, totalTasks, completedTasks) are stale. Plan grid never renders. |
| `PlanList.svelte` | `loading` | `boolean` | Loading state never updates. UI never transitions from loading to data display. |
| `PlanDetail.svelte` | `plan` | `Plan \| null` | Breadcrumb, header, progress bar, markdown sections all use stale null data. Plan detail never renders. |
| `PlanDetail.svelte` | `tasks` | `Task[]` | All 3 `$derived` values (totalTasks, completedTasks, progressPercent) are stale. Task list never renders. |
| `PlanDetail.svelte` | `loading` | `boolean` | Loading state never updates. UI never transitions from loading to data display. |
| `TaskList.svelte` | `plan` | `Plan \| null` | Breadcrumb and header use stale null data. Task list never renders. |
| `TaskList.svelte` | `tasks` | `Task[]` | Kanban columns and list view use stale empty array. No tasks display. |
| `TaskList.svelte` | `loading` | `boolean` | Loading state never updates. UI never transitions from loading to data display. |
| `TaskDetail.svelte` | `task` | `Task \| null` | Breadcrumb, header, all tab content use stale null data. Task detail never renders. |
| `TaskDetail.svelte` | `activeTab` | `'definition' \| 'history'` | Tab switching buttons fire but UI never updates — tabs don't switch visually. |
| `TaskDetail.svelte` | `transitioning` | `boolean` | Status transition loading feedback never shows/hides. Select stays disabled or enabled incorrectly. |
| `TaskDetail.svelte` | `loading` | `boolean` | Loading state never updates. UI never transitions from loading to data display. |

### Why This Is Critical
- **Data never displays**: Every page fetches data from the API via `onMount`, assigns it to a non-reactive variable, and the UI never updates to show it.
- **UI interactions broken**: In `TaskDetail.svelte`, tab switching and status transition feedback are completely non-functional.
- **Loading state stuck**: The `loading` variable in each page never updates, so loading indicators never clear.
- **Silent failure**: The app appears to load but shows empty/null states because the compiler doesn't error on non-reactive variables.

## Solution Approach

### Decision: Local `$state()` vs. Store-Based Reactivity

**Decision**: Use component-local `$state()` declarations.

**Rationale**:
1. **Post-008.2 context**: Plan 008.2 (stores migration) has already been executed. All store imports and writes have been removed from page components. Pages now use local `loading` and `error` variables. The `error` variable is already declared as `$state<string | null>(null)` and needs no changes.
2. **Scope of this fix**: This plan addresses the critical reactivity bug. Converting `let x = value` to `let x = $state(value)` is a surgical, low-risk fix that directly addresses the bug.
3. **No cross-component sharing**: None of these variables are shared across components. Each page manages its own independent state.
4. **`error` already reactive**: Plan 008.2 declared `error` as `$state<string | null>(null)` in each page. It requires no migration here.

### Pattern

Every affected `let` declaration follows the same transformation:

```ts
// Before (non-reactive)
let variableName: Type = initialValue;
// or
let variableName = initialValue;

// After (reactive)
let variableName = $state<Type>(initialValue);
```

Note: The explicit type annotation moves from the variable declaration to the `$state()` generic parameter. This is the idiomatic Svelte 5 pattern.

## Relevant Files

### Files to Modify (4)
| File | Variables to Fix | `$derived` Values Affected |
|------|------------------|---------------------------|
| `web/src/pages/PlanList.svelte` | `planList`, `loading` | `totalPlans`, `activePlans`, `totalTasks`, `completedTasks` |
| `web/src/pages/PlanDetail.svelte` | `plan`, `tasks`, `loading` | `totalTasks`, `completedTasks`, `progressPercent`, `breadcrumbItems` |
| `web/src/pages/TaskList.svelte` | `plan`, `tasks`, `loading` | `breadcrumbItems` |
| `web/src/pages/TaskDetail.svelte` | `task`, `activeTab`, `transitioning`, `loading` | `breadcrumbItems` |

### Files NOT Modified
- Components (no local state — all use `$props()` and `$derived`)
- `types.ts` (unchanged)

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: `state-rune-builder`
  - Role: Convert all non-reactive `let` declarations to `$state()` in 4 page files
  - Agent: builder

- **Validator**
  - Name: `state-rune-validator`
  - Role: Verify compilation, type-checking, and runtime reactivity
  - Agent: validator

## Step by Step Tasks

### 1. Migrate PlanList.svelte
- **Task ID**: migrate-plan-list-state
- **Depends On**: none
- **Assigned To**: state-rune-builder
- **Agent**: builder
- **Actions**:

  #### `planList` variable
  - **Before**:
    ```ts
    let planList: Plan[] = [];
    ```
  - **After**:
    ```ts
    let planList = $state<Plan[]>([]);
    ```
  - **Rationale**: `planList` is assigned in `onMount` (`planList = response.items`). Without `$state()`, this reassignment doesn't trigger reactivity. All 4 `$derived` values depend on `planList`.

  #### `loading` variable
  - **Before**:
    ```ts
    let loading = false;
    ```
  - **After**:
    ```ts
    let loading = $state(false);
    ```
  - **Rationale**: `loading` is set to `true` before the `onMount` fetch and `false` in the `finally` block. Without `$state()`, the loading indicator never clears.

  #### `error` variable (no change needed)
  - Already declared as `let error = $state<string | null>(null)` by plan 008.2. No migration needed.

  #### Template references (no change needed)
  - `planList.length` and `{#each planList as plan}` — These will work correctly once `planList` is reactive.

- **Acceptance Criteria**:
  - `planList` is declared with `$state<Plan[]>([])`
  - `loading` is declared with `$state(false)`
  - File compiles without errors
  - `$derived` values (totalPlans, activePlans, totalTasks, completedTasks) update when `planList` changes
  - Loading indicator clears after data loads

### 2. Migrate PlanDetail.svelte
- **Task ID**: migrate-plan-detail-state
- **Depends On**: none
- **Assigned To**: state-rune-builder
- **Agent**: builder
- **Actions**:

  #### `plan` variable
  - **Before**:
    ```ts
    let plan: Plan | null = null;
    ```
  - **After**:
    ```ts
    let plan = $state<Plan | null>(null);
    ```
  - **Rationale**: `plan` is assigned in `onMount` (`plan = await getPlan(...)`). Without `$state()`, breadcrumb and header use stale null data.

  #### `tasks` variable
  - **Before**:
    ```ts
    let tasks: Task[] = [];
    ```
  - **After**:
    ```ts
    let tasks = $state<Task[]>([]);
    ```
  - **Rationale**: `tasks` is assigned in `onMount` (`tasks = response.items`). Without `$state()`, all 3 `$derived` values (totalTasks, completedTasks, progressPercent) are stale and the task list never renders.

  #### `loading` variable
  - **Before**:
    ```ts
    let loading = false;
    ```
  - **After**:
    ```ts
    let loading = $state(false);
    ```
  - **Rationale**: `loading` is set to `true` before the `onMount` fetch and `false` in the `finally` block. Without `$state()`, the loading indicator never clears.

  #### `error` variable (no change needed)
  - Already declared as `$state<string | null>(null)` by plan 008.2. No migration needed.

- **Acceptance Criteria**:
  - `plan` is declared with `$state<Plan | null>(null)`
  - `tasks` is declared with `$state<Task[]>([])`
  - `loading` is declared with `$state(false)`
  - File compiles without errors
  - `$derived` values update when `plan` or `tasks` change
  - Plan detail renders correctly after data loads
  - Loading indicator clears after data loads

### 3. Migrate TaskList.svelte
- **Task ID**: migrate-task-list-state
- **Depends On**: none
- **Assigned To**: state-rune-builder
- **Agent**: builder
- **Actions**:

  #### `plan` variable
  - **Before**:
    ```ts
    let plan: Plan | null = null;
    ```
  - **After**:
    ```ts
    let plan = $state<Plan | null>(null);
    ```
  - **Rationale**: `plan` is assigned in `onMount`. Breadcrumb and header depend on it.

  #### `tasks` variable
  - **Before**:
    ```ts
    let tasks: Task[] = [];
    ```
  - **After**:
    ```ts
    let tasks = $state<Task[]>([]);
    ```
  - **Rationale**: `tasks` is assigned in `onMount`. Kanban columns and list view iterate over `tasks`. Without `$state()`, both views render empty.

  #### `loading` variable
  - **Before**:
    ```ts
    let loading = false;
    ```
  - **After**:
    ```ts
    let loading = $state(false);
    ```
  - **Rationale**: `loading` is set to `true` before the `onMount` fetch and `false` in the `finally` block. Without `$state()`, the loading indicator never clears.

  #### `error` variable (no change needed)
  - Already declared as `$state<string | null>(null)` by plan 008.2. No migration needed.

- **Acceptance Criteria**:
  - `plan` is declared with `$state<Plan | null>(null)`
  - `tasks` is declared with `$state<Task[]>([])`
  - `loading` is declared with `$state(false)`
  - File compiles without errors
  - Both kanban and list views render tasks correctly after data loads
  - Loading indicator clears after data loads

### 4. Migrate TaskDetail.svelte
- **Task ID**: migrate-task-detail-state
- **Depends On**: none
- **Assigned To**: state-rune-builder
- **Agent**: builder
- **Actions**:

  #### `task` variable
  - **Before**:
    ```ts
    let task: Task | null = null;
    ```
  - **After**:
    ```ts
    let task = $state<Task | null>(null);
    ```
  - **Rationale**: `task` is assigned in `onMount` and reassigned in `handleTransition`. Breadcrumb, header, and all tab content depend on it.

  #### `activeTab` variable
  - **Before**:
    ```ts
    let activeTab: 'definition' | 'history' = 'definition';
    ```
  - **After**:
    ```ts
    let activeTab = $state<'definition' | 'history'>('definition');
    ```
  - **Rationale**: `activeTab` is reassigned by tab button click handlers (`onclick={() => activeTab = 'definition'}`). Without `$state()`, tab switching has no visual effect. Tab button classes (`activeTab === 'definition' ? ...`) never update.

  #### `transitioning` variable
  - **Before**:
    ```ts
    let transitioning = false;
    ```
  - **After**:
    ```ts
    let transitioning = $state(false);
    ```
  - **Rationale**: `transitioning` is set to `true` at the start of `handleTransition` and `false` in the `finally` block. It controls the `disabled` attribute on the status select element. Without `$state()`, the select never visually disables/enables during transitions.

  #### `loading` variable
  - **Before**:
    ```ts
    let loading = false;
    ```
  - **After**:
    ```ts
    let loading = $state(false);
    ```
  - **Rationale**: `loading` is set to `true` before the `onMount` fetch and `false` in the `finally` block. Without `$state()`, the loading indicator never clears.

  #### `error` variable (no change needed)
  - Already declared as `$state<string | null>(null)` by plan 008.2. No migration needed.

  #### `handleTransition` function (no change needed)
  - The function reassigns `task` and `transitioning`. These reassignments will now trigger reactivity since both variables are `$state()`.

  #### Tab buttons (no change needed)
  - `onclick={() => activeTab = 'definition'}` and `onclick={() => activeTab = 'history'}` — These will now work correctly since `activeTab` is reactive.

- **Acceptance Criteria**:
  - `task` is declared with `$state<Task | null>(null)`
  - `activeTab` is declared with `$state<'definition' | 'history'>('definition')`
  - `transitioning` is declared with `$state(false)`
  - `loading` is declared with `$state(false)`
  - File compiles without errors
  - Tab switching works visually (active tab highlights correctly)
  - Status select disables during transitions and re-enables after
  - Loading indicator clears after data loads

### 5. Final Validation
- **Task ID**: validate-reactivity
- **Depends On**: migrate-plan-list-state, migrate-plan-detail-state, migrate-task-list-state, migrate-task-detail-state
- **Assigned To**: state-rune-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npm run check` — TypeScript/Svelte type checking passes
  - Run `cd web && npm run build` — Production build succeeds
  - Run `cd web && npm test` — All existing tests pass
  - Run `grep -rn 'let planList\|let plan:\|let tasks:\|let task:\|let activeTab\|let transitioning\|let loading = false' web/src/pages/` — Should return nothing (all converted to `$state()`)
  - Run `grep -rn '\$state<' web/src/pages/` — Should show 12 `$state()` data variable matches (plus 4 `error` variables already migrated by 008.2 = 16 total)

## Acceptance Criteria
1. All 12 non-reactive `let` declarations in page components are converted to `$state()`
2. `npm run check` passes with no type errors
3. `npm run build` succeeds
4. All existing tests pass
5. No `$derived` values in page components depend on non-reactive variables
6. Tab switching in TaskDetail works visually
7. Status transition feedback (disabled select) works in TaskDetail
8. All pages render data correctly after `onMount` fetches complete
9. Loading indicators clear on all pages after data loads
10. No regression in component behavior

## Validation Commands
- `cd web && npm run check` — Type check
- `cd web && npm run build` — Production build
- `cd web && npm test` — Run test suite
- `grep -rn 'let planList\|let plan:\|let tasks:\|let task:\|let activeTab\|let transitioning\|let loading = false' web/src/pages/` — Verify no plain `let` state variables remain
- `grep -rn '\$state<' web/src/pages/` — Verify all 12 `$state()` declarations exist (plus 4 pre-existing `error` variables)
- `grep -c '\$derived' web/src/pages/*.svelte` — Verify `$derived` count is unchanged (14 total)

## Notes

### Execution Order
All 4 page migrations are independent and can be done in parallel or any order. There are no inter-page dependencies.

### `$state()` Type Syntax
The idiomatic Svelte 5 pattern places the type annotation on the `$state()` generic, not the variable:
```ts
// Correct (idiomatic)
let planList = $state<Plan[]>([]);

// Also valid but less idiomatic
let planList: Plan[] = $state([]);
```
This plan uses the first form consistently.

### Relationship to Other Plans
- **008.1 (Svelte 5 Runes Migration)**: Already completed. Converted `$:` to `$derived`, `export let` to `$props()`, `on:event` to `onevent`.
- **008.2 (Stores Migration)**: Already completed. Removed all store imports and writes from page components, introducing local `loading` and `error` variables. The `error` variable was declared as `$state<string | null>(null)` and needs no further migration.
- **Execution order**: 008.1 → 008.2 → 008.3 (this plan)

### Reactivity Verification
To verify reactivity works at runtime, a validator can:
1. Open any page and observe that data renders after the loading state clears
2. In TaskDetail, click tab buttons and verify the active tab updates visually
3. In TaskDetail, change the status select and verify it disables during the API call
