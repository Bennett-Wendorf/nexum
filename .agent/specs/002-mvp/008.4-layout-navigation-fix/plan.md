# Plan: 008.4 - Layout Navigation Fix

## Task Description
Fix the `Layout.svelte` component's `currentPath` derivation so that it reactively updates when SPA navigation occurs via `svelte-spa-router`. Currently, `currentPath` is computed once at mount using `window.location.href`, which is not a reactive Svelte source, causing the sidebar active state to become stale after any in-app navigation.

## Objective
Ensure the sidebar navigation highlights the correct active route after every SPA navigation event.

## Problem Statement
In `Layout.svelte` (line 2), `currentPath` is derived from `window.location.href`:

```svelte
const currentPath = $derived(new URL(window.location.href).pathname);
```

`$derived` in Svelte 5 runes tracks reactive dependencies. Since `window.location` is a browser global and not a Svelte reactive value, `$derived` computes `currentPath` once at component mount and never re-evaluates it. The `isActive()` function uses `currentPath` to determine which sidebar icon to highlight, so after any `svelte-spa-router` navigation, the sidebar active state remains stale.

## Solution Approach
Replace the `window.location.href` approach with `svelte-spa-router`'s exported `location` readable store. This store is updated reactively by the router on every navigation event.

In `Layout.svelte`:
- Import `{ location }` from `'svelte-spa-router'`
- Replace `const currentPath = $derived(new URL(window.location.href).pathname);` with `const currentPath = $derived($location);`
- The `$` auto-subscription syntax accesses the store reactively, and `$derived` will re-compute whenever `$location` changes

No changes are needed in `App.svelte` since `location` is a global store provided by `svelte-spa-router`, not a prop-passed value.

## Relevant Files
- `web/src/components/Layout.svelte` — Primary fix target; replace `window.location` with `$location` store
- `web/src/App.svelte` — No changes needed (for reference)
- `web/src/routes/index.ts` — No changes needed (for reference)

### New Files
None.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: layout-fix-builder
  - Role: Implement the reactive location fix in Layout.svelte
  - Agent: builder

- **Validator**
  - Name: layout-fix-validator
  - Role: Verify the fix works correctly and no regressions are introduced
  - Agent: validator

- **Documenter**
  - Name: layout-fix-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Fix Layout.svelte reactive path tracking
- **Task ID**: fix-layout-current-path
- **Depends On**: none
- **Assigned To**: layout-fix-builder
- **Agent**: builder
- **Actions**:
  - Open `web/src/components/Layout.svelte`
  - In the `<script>` block, add `import { location } from 'svelte-spa-router';` at the top
  - Replace line 2: `const currentPath = $derived(new URL(window.location.href).pathname);` with `const currentPath = $derived($location);`
  - The resulting script block should look like:
    ```svelte
    <script lang="ts">
      import { location } from 'svelte-spa-router';

      const currentPath = $derived($location);

      function isActive(path: string): boolean {
        return currentPath === path || currentPath.startsWith(path + '/');
      }
    </script>
    ```
- **Acceptance Criteria**:
  - `Layout.svelte` imports `location` from `svelte-spa-router`
  - `currentPath` is derived from `$location` instead of `window.location.href`
  - The `isActive()` function remains unchanged and still works with the new `currentPath`
  - No TypeScript compilation errors

### 2. Final Validation
- **Task ID**: validate-layout-fix
- **Depends On**: fix-layout-current-path
- **Assigned To**: layout-fix-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npx svelte-check` — verify no typecheck errors
  - Run `cd web && npm run build` — verify the build succeeds
  - Run `cd web && npm run dev` — verify the dev server starts
  - Manual verification: Navigate between `/` and `/plans` and confirm the sidebar icon highlight updates correctly
  - Confirm no console errors related to the `location` store subscription

### 3. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-layout-fix
- **Assigned To**: layout-fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `Layout.svelte` uses `svelte-spa-router`'s `location` store instead of `window.location.href`
- The `currentPath` variable updates reactively on every SPA navigation event
- The `isActive()` function correctly highlights the active sidebar item after navigation
- No TypeScript or Svelte compilation errors
- The application builds and runs without errors
- Sidebar active state updates immediately after navigating between routes (e.g., `/` → `/plans`)

## Validation Commands
- `cd web && npx svelte-check` — Typecheck the Svelte project
- `cd web && npm run build` — Verify production build succeeds
- `cd web && npm run dev` — Start dev server for manual testing

## Notes
- The `location` store from `svelte-spa-router` returns the path as a string (e.g., `"/plans"`) which matches the format previously obtained via `new URL(window.location.href).pathname`. The `isActive()` function logic does not need modification.
- The `$` prefix auto-subscribes to the Svelte store, and `$derived` will track `$location` as a reactive dependency, ensuring `currentPath` updates on every navigation.
- This fix is a minimal, surgical change — only two lines in `Layout.svelte` are modified (one import added, one line replaced).
