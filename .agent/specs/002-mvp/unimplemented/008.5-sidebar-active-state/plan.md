# Plan: 008.5 - Sidebar Active State

## Task Description
Fix the `Layout.svelte` component's `currentPath` derivation so that it reactively updates when SPA navigation occurs via `svelte-spa-router`. Currently, `currentPath` is computed once at mount using `window.location.href`, which is not a reactive Svelte source, causing the sidebar active state to become stale after any in-app navigation.

## Objective
Ensure the icon sidebar navigation highlights the correct active route after every SPA navigation event, including both programmatic navigation (`push()`) and browser back/forward navigation.

## Problem Statement
In `Layout.svelte` (line 2), `currentPath` is derived from `window.location.href`:

```svelte
const currentPath = $derived(new URL(window.location.href).pathname);
```

`$derived` in Svelte 5 runes tracks reactive dependencies. Since `window.location` is a browser global and not a Svelte reactive value, `$derived` computes `currentPath` once at component mount and never re-evaluates it.

This is especially problematic because the app uses **hash-based routing** (svelte-spa-router v4). With hash-based routing, navigating from `/` to `/plans` changes the URL from `http://localhost/#/` to `http://localhost/#/plans`. The `pathname` portion (`/`) never changes — only the hash fragment changes. `new URL(window.location.href).pathname` always returns `/`, so `currentPath` is always `/` regardless of the actual route.

The `isActive()` function uses `currentPath` to determine which sidebar icon to highlight, so after any `svelte-spa-router` navigation, the sidebar active state remains permanently stale.

## Solution Approach
Replace the `window.location.href` approach with `svelte-spa-router`'s exported `location` readable store. This store is updated reactively by the router on every navigation event and returns the current route path (e.g., `"/plans"`).

### Approach Evaluation

| Approach | Pros | Cons | Verdict |
|----------|------|------|---------|
| **(a) `location` store from svelte-spa-router** | Idiomatic, minimal change, reactive by design, works with hash routing | Requires importing from router package | **Selected** |
| **(b) Listen to `popstate` events** | No dependency on router internals | Manual state management, doesn't handle `push()` calls, complex, error-prone | Rejected |
| **(c) Pass path as prop from App.svelte** | Decouples Layout from router | Adds prop plumbing, requires changes to App.svelte, more surface area for bugs | Rejected |

### Why approach (a) is best:
1. **Minimal change**: Only two lines in `Layout.svelte` are modified (one import added, one line replaced)
2. **Idiomatic**: Uses the router's own reactive store, which is the intended pattern
3. **Complete coverage**: Handles all navigation types — `push()`, `pop()`, `replace()`, browser back/forward
4. **Hash-routing aware**: The `location` store returns the route path without the `#` prefix, which matches the format expected by `isActive()`
5. **SSR-safe**: Unlike `window.location`, the `location` store is managed by the router and won't cause errors in SSR contexts

### Implementation details:
In `Layout.svelte`:
- Import `{ location }` from `'svelte-spa-router'`
- Replace `const currentPath = $derived(new URL(window.location.href).pathname);` with `const currentPath = $derived($location);`
- The `$` auto-subscription syntax accesses the store reactively, and `$derived` will re-compute whenever `$location` changes
- The `isActive()` function requires no modification — `$location` returns the path string in the same format (e.g., `"/plans"`)

No changes are needed in `App.svelte` since `location` is a global store provided by `svelte-spa-router`, not a prop-passed value.

## Relevant Files
- `web/src/components/Layout.svelte` — Primary fix target; replace `window.location` with `$location` store
- `web/src/App.svelte` — No changes needed (for reference)
- `web/src/routes/index.ts` — No changes needed (for reference)
- `web/src/components/RouterLink.svelte` — No changes needed (for reference; uses `push()` for SPA navigation)

### New Files
None.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: sidebar-active-builder
  - Role: Implement the reactive location fix in Layout.svelte
  - Agent: builder

- **Validator**
  - Name: sidebar-active-validator
  - Role: Verify the fix works correctly and no regressions are introduced
  - Agent: validator

- **Documenter**
  - Name: sidebar-active-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Fix Layout.svelte reactive path tracking
- **Task ID**: fix-layout-current-path
- **Depends On**: none
- **Assigned To**: sidebar-active-builder
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
  - No references to `window.location` remain in the file

### 2. Final Validation
- **Task ID**: validate-sidebar-active-state
- **Depends On**: fix-layout-current-path
- **Assigned To**: sidebar-active-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npx svelte-check` — verify no typecheck errors
  - Run `cd web && npm run build` — verify the production build succeeds
  - Run `cd web && npm run dev` — verify the dev server starts without errors
  - Manual verification: Navigate between `/` and `/plans` and confirm the sidebar icon highlight updates correctly for each route
  - Manual verification: Use browser back/forward buttons and confirm the sidebar active state updates correctly
  - Confirm no console errors related to the `location` store subscription
  - Confirm no references to `window.location` exist in `Layout.svelte`

### 3. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-sidebar-active-state
- **Assigned To**: sidebar-active-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `Layout.svelte` uses `svelte-spa-router`'s `location` store instead of `window.location.href`
- The `currentPath` variable updates reactively on every SPA navigation event (push, pop, replace, browser back/forward)
- The `isActive()` function correctly highlights the active sidebar item after navigation
- No TypeScript or Svelte compilation errors
- The application builds and runs without errors
- Sidebar active state updates immediately after navigating between routes (e.g., `/` → `/plans`)
- No references to `window.location` remain in `Layout.svelte`

## Validation Commands
- `cd web && npx svelte-check` — Typecheck the Svelte project
- `cd web && npm run build` — Verify production build succeeds
- `cd web && npm run dev` — Start dev server for manual testing
- `grep -r "window.location" web/src/components/Layout.svelte` — Verify no remaining references to `window.location`

## Notes
- The `location` store from `svelte-spa-router` v4 returns the route path as a string (e.g., `"/plans"`) without the hash prefix. This matches the format previously obtained via `new URL(window.location.href).pathname` (though that approach was broken for hash routing since `pathname` doesn't include the hash fragment).
- The `$` prefix auto-subscribes to the Svelte store, and `$derived` will track `$location` as a reactive dependency, ensuring `currentPath` updates on every navigation.
- This fix is a minimal, surgical change — only two lines in `Layout.svelte` are modified (one import added, one line replaced).
- The app uses hash-based routing (svelte-spa-router v4). The `location` store correctly returns the route path from the hash fragment, which is exactly what `isActive()` needs.
- This resolves the known limitation documented in `app_docs/svelte5-runes-migration.md` (Known Limitations / Deferred Items, item #1).
