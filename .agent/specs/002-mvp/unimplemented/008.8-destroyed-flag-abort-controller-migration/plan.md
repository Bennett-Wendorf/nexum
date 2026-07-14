# Plan: 008.8 - Destroyed flag to AbortController migration

## Task Description
Refactor `Layout.svelte` to replace the `$state` `destroyed` flag pattern with a single `AbortController` that governs the entire `onMount` lifecycle. This requires propagating an `AbortSignal` through the API layer (`request`, `listAgents`, `listRunningTasks`) and simplifying the component's cleanup logic.

## Objective
Eliminate the `destroyed` reactive state variable from `Layout.svelte` and replace it with idiomatic Svelte 5 lifecycle management using `AbortController` and `$effect` cleanup.

## Problem Statement
`Layout.svelte` uses a `$state` `destroyed` flag as a guard against post-unmount state mutations. This pattern:
- Mixes lifecycle concerns into business logic (every async callback checks `if (destroyed) return`)
- Leaks lifecycle state into the component's reactive surface
- Makes the code harder to reason about and maintain
- Is non-idiomatic for Svelte 5, which provides `$effect` cleanup and `AbortController` as the standard patterns

The `destroyed` flag is checked in three places:
1. After `listAgents()` resolves (line 35)
2. After `listRunningTasks()` resolves (line 64)
3. In the polling error handler (line 68)

## Solution Approach
1. Add an optional `signal` parameter to the `request()` base function in `api.ts`
2. Propagate the signal through `listAgents()` and `listRunningTasks()`
3. In `Layout.svelte`, create a single `AbortController` at the start of `onMount` and pass its signal to all API calls
4. Use the `onMount` cleanup return function + the aborted signal to prevent any post-unmount state mutations
5. Remove the `destroyed` flag entirely
6. Keep the `$effect`-based error cleanup pattern (it's already idiomatic)

## Relevant Files

### Files to Modify
- `web/src/lib/api.ts` — Add `signal` parameter to `request()`, `listAgents()`, `listRunningTasks()`
- `web/src/components/Layout.svelte` — Replace `destroyed` flag with `AbortController`-based lifecycle management

### Reference Files (read-only)
- `web/src/lib/errorUtils.ts` — `setError()` pattern is already correct, no changes needed
- `web/src/pages/PlanList.svelte` — Reference for idiomatic Svelte 5 patterns (though it doesn't use AbortController, its `$effect` error cleanup pattern is correct)

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: api-signal-builder
  - Role: Add signal support to the API layer and refactor Layout.svelte
  - Agent: builder

- **Validator**
  - Name: lifecycle-validator
  - Role: Verify the migration is correct and no post-unmount mutations occur
  - Agent: validator

- **Documenter**
  - Name: refactor-documenter
  - Role: Document the pattern change for future reference
  - Agent: documenter

## Step by Step Tasks

### 1. Add signal parameter to request() in api.ts
- **Task ID**: add-signal-to-request
- **Depends On**: none
- **Assigned To**: api-signal-builder
- **Agent**: builder
- **Actions**:
  - Add an optional `signal?: AbortSignal` parameter to the `request()` function signature
  - Pass the signal through to the `fetch()` call: add `signal` to the fetch options object
  - Ensure that `AbortError` exceptions propagate correctly (fetch does this natively)
- **Acceptance Criteria**:
  - `request()` function signature includes `signal?: AbortSignal`
  - The signal is passed to `fetch(url, { ..., signal })`
  - Existing callers that don't pass a signal continue to work unchanged

### 2. Propagate signal through listAgents() and listRunningTasks()
- **Task ID**: propagate-signal-api
- **Depends On**: add-signal-to-request
- **Assigned To**: api-signal-builder
- **Agent**: builder
- **Actions**:
  - Add `signal?: AbortSignal` parameter to `listAgents()`
  - Add `signal?: AbortSignal` parameter to `listRunningTasks()`
  - Pass the signal through to the `request()` call in each function
- **Acceptance Criteria**:
  - Both functions accept an optional `signal` parameter
  - Both functions pass the signal to `request()`
  - Existing callers (without signal) continue to work unchanged

### 3. Refactor Layout.svelte to use AbortController
- **Task ID**: refactor-layout-abort-controller
- **Depends On**: propagate-signal-api
- **Assigned To**: api-signal-builder
- **Agent**: builder
- **Actions**:
  - Remove `let destroyed = $state(false);`
  - Inside `onMount`, create `const controller = new AbortController()` at the top
  - Pass `controller.signal` to `listAgents({ signal: controller.signal })`
  - Pass `controller.signal` to `listRunningTasks({ signal: controller.signal })` — note: the current `fetchRunningTasks` inner function creates its own `fetchController`; this should be removed in favor of the single `controller`
  - In the `listAgents` catch block, check `controller.signal.aborted` instead of `destroyed` (or simply let the `AbortError` propagate silently)
  - In the `fetchRunningTasks` function, remove the inner `fetchController` and use the outer `controller.signal` instead
  - In the `onMount` cleanup return function, replace `destroyed = true` with `controller.abort()`
  - Remove all `if (destroyed) return` guards — the aborted signal handles this naturally
  - Keep the `$effect` error cleanup pattern unchanged
- **Acceptance Criteria**:
  - No `destroyed` variable exists in the component
  - A single `AbortController` governs the entire `onMount` lifecycle
  - All API calls receive the controller's signal
  - The cleanup function calls `controller.abort()` instead of setting `destroyed = true`
  - The inner `fetchController` is removed (replaced by the single controller)
  - Component behavior is functionally identical before and after the change

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: refactor-layout-abort-controller
- **Assigned To**: lifecycle-validator
- **Agent**: validator
- **Checks**:
  - `grep -r "destroyed" web/src/components/Layout.svelte` returns no matches
  - `grep -r "AbortController" web/src/components/Layout.svelte` shows exactly one instantiation
  - `cd web && npx svelte-check` passes with no errors
  - The application builds successfully: `cd web && npm run build`
  - Manual verification: navigate away from a page while API calls are in-flight; confirm no errors from stale state mutations in the console

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: refactor-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- The `destroyed` flag is completely removed from `Layout.svelte`
- A single `AbortController` manages the entire `onMount` lifecycle
- The `request()` function in `api.ts` accepts an optional `signal` parameter
- `listAgents()` and `listRunningTasks()` accept and propagate the signal
- All API calls in `Layout.svelte` receive the abort signal
- The `$effect` error cleanup pattern remains unchanged (it's already idiomatic)
- `svelte-check` passes with no errors
- The application builds and runs without regressions
- No post-unmount state mutations occur (verified by absence of console errors)

## Validation Commands
- `cd web && npx svelte-check` — Type-check the Svelte project
- `cd web && npm run build` — Build the application
- `grep -rn "destroyed" web/src/components/Layout.svelte` — Should return no matches
- `grep -n "AbortController" web/src/components/Layout.svelte` — Should show exactly one instantiation

## Notes
- The inner `fetchController` in `fetchRunningTasks` was used to cancel in-flight fetches between poll cycles. The single `AbortController` approach replaces this: when the component unmounts, `controller.abort()` cancels all in-flight requests. Between poll cycles, the fetches are short-lived enough that cancellation between cycles is unnecessary. If this becomes a concern, a per-poll `AbortController` can be added back later, but it's not needed for the unmount-guard use case.
- The `$effect` pattern for error cleanup (`errorCleanup`) is already idiomatic Svelte 5 and should be preserved as-is.
- Other components that call `listAgents()` or `listRunningTasks()` will need to be audited if they exist, but a grep search shows `Layout.svelte` is the only caller of these functions.
