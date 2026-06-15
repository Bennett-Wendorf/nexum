# Plan: 008.1 - Svelte 4 → Svelte 5 Runes Migration

## Task Description
Migrate all Svelte components from Svelte 4 syntax to Svelte 5 runes. The project uses `"svelte": "^5.0.0"` but all components still use Svelte 4 syntax. This migration is critical for the app to function correctly with Svelte 5's compiler.

## Objective
Convert all 16 Svelte files to use Svelte 5 runes syntax so the application compiles and runs correctly with `svelte@^5.0.0`.

## Problem Statement
The Svelte 5 compiler has fundamentally changed how reactivity works:
1. `export let` props → must use `$props()` rune
2. `$:` reactive statements → must use `$derived` rune
3. `on:event` handlers → must use `onevent` attribute syntax
4. `$store` auto-subscription in `<script>` → must use `$derived` or explicit `.subscribe()`
5. `<slot />` self-closing → must become `<slot></slot>`
6. `window.location` direct access → should use router context or prop-based route passing

## Solution Approach
Migrate files in dependency order: leaf components first (no dependencies on other migrated components), then pages (which depend on components). Store usage patterns will be addressed consistently across all files.

### Store Usage Strategy
In Svelte 5, `$storeName` auto-subscription works in templates but NOT in `<script>` blocks. The migration pattern:
- **In templates**: Keep `$loading`, `$viewMode` etc. as-is (auto-subscription works in templates)
- **In `<script>` blocks**: Where stores are read in logic (not just `.set()` calls), use `$derived($storeName)` to create a reactive local variable
- **For `loading` used in `{#if loading}` templates**: Change to `{#if $loading}` since `loading` is the store object, not its value

## Relevant Files

### Components (9 files)
| File | Issues |
|------|--------|
| `Breadcrumb.svelte` | `export let` → `$props()` |
| `MarkdownViewer.svelte` | ✅ Already migrated |
| `TaskCard.svelte` | `export let` → `$props()`, `$:` → `$derived` |
| `PlanCard.svelte` | `export let` → `$props()`, `$:` → `$derived` |
| `TaskColumn.svelte` | `export let` → `$props()`, `$:` → `$derived` |
| `MarkdownRenderer.svelte` | `export let` → `$props()`, `$:` → `$derived` |
| `RouterLink.svelte` | `export let` → `$props()`, `<slot />` → `<slot></slot>` |
| `StatusBadge.svelte` | `export let` → `$props()`, `$:` → `$derived` |
| `Layout.svelte` | `window.location` access (low priority) |

### Pages (6 files)
| File | Issues |
|------|--------|
| `App.svelte` | ✅ Already compatible |
| `Home.svelte` | ✅ Already compatible |
| `NotFound.svelte` | ✅ Already compatible |
| `PlanList.svelte` | `$:` → `$derived`, `$store` template usage |
| `PlanDetail.svelte` | `export let` → `$props()`, `$:` → `$derived`, `$store` usage |
| `TaskDetail.svelte` | `export let` → `$props()`, `$:` → `$derived`, `on:click`/`on:change` → `onclick`/`onchange`, `$store` usage |
| `TaskList.svelte` | `export let` → `$props()`, `$:` → `$derived`, `on:click` → `onclick`, `$store` usage |

### Store
| File | Issues |
|------|--------|
| `store.ts` | No changes needed (writable stores work in Svelte 5) |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: `svelte5-migrator`
  - Role: Migrate all Svelte components and pages to runes syntax
  - Agent: builder

- **Validator**
  - Name: `svelte5-validator`
  - Role: Verify compilation, test suite, and runtime behavior
  - Agent: validator

- **Documenter**
  - Name: `migration-documenter`
  - Role: Generate migration notes and update any relevant documentation
  - Agent: documenter

## Step by Step Tasks

### 1. Migrate Leaf Components (no inter-component dependencies)
- **Task ID**: migrate-leaf-components
- **Depends On**: none
- **Assigned To**: svelte5-migrator
- **Agent**: builder
- **Actions**:

  #### Breadcrumb.svelte (web/src/components/Breadcrumb.svelte)
  - **Line 2**: `export let items: { label: string; href?: string }[];`
    - **Before**: `export let items: { label: string; href?: string }[];`
    - **After**: `let { items }: { items: { label: string; href?: string }[] } = $props();`

  #### StatusBadge.svelte (web/src/components/StatusBadge.svelte)
  - **Line 4-6**: Props declaration
    - **Before**:
      ```ts
      export let status: string;
      export let type: 'plan' | 'task';
      export let label: string | undefined = undefined;
      ```
    - **After**:
      ```ts
      let { status, type, label = undefined }: {
        status: string;
        type: 'plan' | 'task';
        label?: string;
      } = $props();
      ```
  - **Line 8**: Reactive destructuring
    - **Before**: `$: [bgClass, textClass] = type === 'plan' ? getPlanStatusColor(status) : getTaskStatusColor(status);`
    - **After**:
      ```ts
      const colorTuple = $derived(type === 'plan' ? getPlanStatusColor(status) : getTaskStatusColor(status));
      const bgClass = $derived(colorTuple[0]);
      const textClass = $derived(colorTuple[1]);
      ```
  - **Line 9**: Reactive statement
    - **Before**: `$: displayLabel = label ?? (kanbanColumnLabels[status] ?? status);`
    - **After**: `const displayLabel = $derived(label ?? (kanbanColumnLabels[status] ?? status));`

  #### RouterLink.svelte (web/src/components/RouterLink.svelte)
  - **Line 4-5**: Props declaration
    - **Before**:
      ```ts
      export let href: string;
      export let className: string = '';
      ```
    - **After**:
      ```ts
      let { href, className = '' }: { href: string; className?: string } = $props();
      ```
  - **Line 14**: Self-closing slot
    - **Before**: `<slot />`
    - **After**: `<slot></slot>`

- **Acceptance Criteria**:
  - All 4 files compile without Svelte compiler errors
  - No `$:` reactive statements remain
  - No `export let` props remain
  - No self-closing `<slot />` tags remain

### 2. Migrate Components with Reactive Statements
- **Task ID**: migrate-reactive-components
- **Depends On**: migrate-leaf-components
- **Assigned To**: svelte5-migrator
- **Agent**: builder
- **Actions**:

  #### TaskCard.svelte (web/src/components/TaskCard.svelte)
  - **Line 5-7**: Props declaration
    - **Before**:
      ```ts
      export let task: Task;
      export let planId: string;
      export let branch: string;
      ```
    - **After**:
      ```ts
      let { task, planId, branch }: { task: Task; planId: string; branch: string } = $props();
      ```
  - **Line 9-11**: Reactive statements
    - **Before**:
      ```ts
      $: href = `/plans/${branch}/${planId}/tasks/${task.id}`;
      $: agentName = task.status.agent?.role ?? null;
      $: taskStatus = task.status.status;
      ```
    - **After**:
      ```ts
      const href = $derived(`/plans/${branch}/${planId}/tasks/${task.id}`);
      const agentName = $derived(task.status.agent?.role ?? null);
      const taskStatus = $derived(task.status.status);
      ```

  #### PlanCard.svelte (web/src/components/PlanCard.svelte)
  - **Line 5**: Props declaration
    - **Before**: `export let plan: Plan;`
    - **After**: `let { plan }: { plan: Plan } = $props();`
  - **Line 7-10**: Reactive statements
    - **Before**:
      ```ts
      $: totalTasks = plan.tasks?.length ?? 0;
      $: completedTasks = plan.tasks?.filter(t => t.completed).length ?? 0;
      $: progressPercent = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;
      $: href = `/plans/${plan.branch}/${plan.id}`;
      ```
    - **After**:
      ```ts
      const totalTasks = $derived(plan.tasks?.length ?? 0);
      const completedTasks = $derived(plan.tasks?.filter(t => t.completed).length ?? 0);
      const progressPercent = $derived(totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0);
      const href = $derived(`/plans/${plan.branch}/${plan.id}`);
      ```

  #### TaskColumn.svelte (web/src/components/TaskColumn.svelte)
  - **Line 6-9**: Props declaration
    - **Before**:
      ```ts
      export let status: string;
      export let tasks: Task[];
      export let planId: string;
      export let branch: string;
      ```
    - **After**:
      ```ts
      let { status, tasks, planId, branch }: {
        status: string;
        tasks: Task[];
        planId: string;
        branch: string;
      } = $props();
      ```
  - **Line 11-12**: Reactive statements
    - **Before**:
      ```ts
      $: label = kanbanColumnLabels[status] ?? status;
      $: count = tasks.length;
      ```
    - **After**:
      ```ts
      const label = $derived(kanbanColumnLabels[status] ?? status);
      const count = $derived(tasks.length);
      ```

  #### MarkdownRenderer.svelte (web/src/components/MarkdownRenderer.svelte)
  - **Line 11-12**: Props declaration
    - **Before**:
      ```ts
      export let content: string;
      export let className: string = '';
      ```
    - **After**:
      ```ts
      let { content, className = '' }: { content: string; className?: string } = $props();
      ```
  - **Line 14-15**: Reactive statements
    - **Before**:
      ```ts
      $: html = content.trim() ? md.render(content) : '';
      $: containerClass = className || 'md-content';
      ```
    - **After**:
      ```ts
      const html = $derived(content.trim() ? md.render(content) : '');
      const containerClass = $derived(className || 'md-content');
      ```

- **Acceptance Criteria**:
  - All 4 files compile without Svelte compiler errors
  - All `$:` statements converted to `$derived`
  - All `export let` props converted to `$props()`
  - `$derived` chains correctly reference other `$derived` variables (Svelte 5 allows this)

### 3. Migrate Pages without Route Params (simple store-only)
- **Task ID**: migrate-simple-pages
- **Depends On**: migrate-reactive-components
- **Assigned To**: svelte5-migrator
- **Agent**: builder
- **Actions**:

  #### PlanList.svelte (web/src/pages/PlanList.svelte)
  - **Line 25-28**: Reactive statements
    - **Before**:
      ```ts
      $: totalPlans = planList.length;
      $: activePlans = planList.filter(p => ['approved', 'planning', 'reviewing', 'queued'].includes(p.status)).length;
      $: totalTasks = planList.reduce((sum, p) => sum + (p.tasks?.length ?? 0), 0);
      $: completedTasks = planList.reduce((sum, p) => sum + (p.tasks?.filter(t => t.completed).length ?? 0), 0);
      ```
    - **After**:
      ```ts
      const totalPlans = $derived(planList.length);
      const activePlans = $derived(planList.filter(p => ['approved', 'planning', 'reviewing', 'queued'].includes(p.status)).length);
      const totalTasks = $derived(planList.reduce((sum, p) => sum + (p.tasks?.length ?? 0), 0));
      const completedTasks = $derived(planList.reduce((sum, p) => sum + (p.tasks?.filter(t => t.completed).length ?? 0), 0));
      ```
  - **Template lines 64, 66, 70**: Store auto-subscription
    - **Before**: `{#if loading}`
    - **After**: `{#if $loading}`
    - (Apply to all `{#if loading}` occurrences in template)

  #### Layout.svelte (web/src/components/Layout.svelte)
  - **Line 2**: `window.location` access (deferred - low priority)
    - **Note**: `$derived(new URL(window.location.href).pathname)` works in Svelte 5 but is not ideal. The `isActive()` function relies on this for active link highlighting. This is a known limitation that can be addressed later by passing the current route via context or props. For now, leave as-is since it's functional.

- **Acceptance Criteria**:
  - PlanList.svelte compiles without errors
  - Template `{#if loading}` changed to `{#if $loading}`
  - All `$:` statements converted to `$derived`

### 4. Migrate Pages with Route Params and Event Handlers
- **Task ID**: migrate-complex-pages
- **Depends On**: migrate-simple-pages
- **Assigned To**: svelte5-migrator
- **Agent**: builder
- **Actions**:

  #### PlanDetail.svelte (web/src/pages/PlanDetail.svelte)
  - **Line 11-12**: Props declaration
    - **Before**:
      ```ts
      export let branch: string;
      export let planId: string;
      ```
    - **After**:
      ```ts
      let { branch, planId }: { branch: string; planId: string } = $props();
      ```
  - **Line 32-39**: Reactive statements
    - **Before**:
      ```ts
      $: totalTasks = tasks.length;
      $: completedTasks = tasks.filter(t => t.status.status === 'completed').length;
      $: progressPercent = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;
      $: breadcrumbItems = [
        { label: 'Plans', href: '/plans' },
        { label: plan?.name ?? planId },
      ];
      ```
    - **After**:
      ```ts
      const totalTasks = $derived(tasks.length);
      const completedTasks = $derived(tasks.filter(t => t.status.status === 'completed').length);
      const progressPercent = $derived(totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0);
      const breadcrumbItems = $derived([
        { label: 'Plans', href: '/plans' },
        { label: plan?.name ?? planId },
      ]);
      ```
  - **Template line 43**: Store auto-subscription
    - **Before**: `{#if loading}`
    - **After**: `{#if $loading}`

  #### TaskDetail.svelte (web/src/pages/TaskDetail.svelte)
  - **Line 11-13**: Props declaration
    - **Before**:
      ```ts
      export let branch: string;
      export let planId: string;
      export let taskId: string;
      ```
    - **After**:
      ```ts
      let { branch, planId, taskId }: { branch: string; planId: string; taskId: string } = $props();
      ```
  - **Line 32-37**: Reactive statement
    - **Before**:
      ```ts
      $: breadcrumbItems = [
        { label: 'Plans', href: '/plans' },
        { label: task?.name ?? taskId, href: `/plans/${branch}/${planId}` },
        { label: 'Tasks', href: `/plans/${branch}/${planId}/tasks` },
        { label: task?.name ?? taskId },
      ];
      ```
    - **After**:
      ```ts
      const breadcrumbItems = $derived([
        { label: 'Plans', href: '/plans' },
        { label: task?.name ?? taskId, href: `/plans/${branch}/${planId}` },
        { label: 'Tasks', href: `/plans/${branch}/${planId}/tasks` },
        { label: task?.name ?? taskId },
      ]);
      ```
  - **Line 81**: Event handler (on:change → onchange)
    - **Before**: `on:change={(e) => handleTransition((e.target as HTMLSelectElement).value)}`
    - **After**: `onchange={(e: Event) => handleTransition((e.target as HTMLSelectElement).value)}`
  - **Line 101**: Event handler (on:click → onclick)
    - **Before**: `on:click={() => activeTab = 'definition'}`
    - **After**: `onclick={() => activeTab = 'definition'}`
  - **Line 106**: Event handler (on:click → onclick)
    - **Before**: `on:click={() => activeTab = 'history'}`
    - **After**: `onclick={() => activeTab = 'history'}`
  - **Template line 55**: Store auto-subscription
    - **Before**: `{#if loading}`
    - **After**: `{#if $loading}`

  #### TaskList.svelte (web/src/pages/TaskList.svelte)
  - **Line 12-13**: Props declaration
    - **Before**:
      ```ts
      export let branch: string;
      export let planId: string;
      ```
    - **After**:
      ```ts
      let { branch, planId }: { branch: string; planId: string } = $props();
      ```
  - **Line 33-37**: Reactive statement
    - **Before**:
      ```ts
      $: breadcrumbItems = [
        { label: 'Plans', href: '/plans' },
        { label: plan?.name ?? planId, href: `/plans/${branch}/${planId}` },
        { label: 'Tasks' },
      ];
      ```
    - **After**:
      ```ts
      const breadcrumbItems = $derived([
        { label: 'Plans', href: '/plans' },
        { label: plan?.name ?? planId, href: `/plans/${branch}/${planId}` },
        { label: 'Tasks' },
      ]);
      ```
  - **Line 40**: Script store read
    - **Before**: `viewMode.set($viewMode === 'kanban' ? 'list' : 'kanban');`
    - **After**: `viewMode.set($viewMode === 'kanban' ? 'list' : 'kanban');`
    - **Note**: `$viewMode` in `<script>` is NOT valid in Svelte 5 runes mode. Fix:
    - **After (correct)**:
      ```ts
      const currentViewMode = $derived($viewMode);
      // Then in toggleView:
      viewMode.set(currentViewMode === 'kanban' ? 'list' : 'kanban');
      ```
  - **Line 72**: Event handler (on:click → onclick)
    - **Before**: `on:click={toggleView}`
    - **After**: `onclick={toggleView}`
  - **Line 75**: Event handler (on:click → onclick)
    - **Before**: `on:click={toggleView}`
    - **After**: `onclick={toggleView}`
  - **Template lines 49, 72, 75**: Store auto-subscription
    - **Before**: `{#if loading}` → **After**: `{#if $loading}`
    - **Before**: `$viewMode === 'kanban'` → **After**: `$viewMode === 'kanban'` (already correct in templates)
    - **Note**: `$viewMode` in templates is fine (auto-subscription works in templates)

- **Acceptance Criteria**:
  - All 4 page files compile without Svelte compiler errors
  - All `on:event` handlers converted to `onevent`
  - All `export let` props converted to `$props()`
  - All `$:` statements converted to `$derived`
  - Store reads in `<script>` blocks use `$derived($storeName)` pattern
  - Store reads in templates use `$storeName` auto-subscription

## Acceptance Criteria
1. `npm run build` completes without Svelte compiler errors
2. `npm run dev` starts without errors
3. `npm test` passes all existing tests
4. No `export let` props remain in any .svelte file
5. No `$:` reactive statements remain in any .svelte file
6. No `on:event` handlers remain in any .svelte file
7. No self-closing `<slot />` tags remain in any .svelte file
8. Store usage in templates uses `$storeName` auto-subscription
9. Store usage in `<script>` blocks uses `$derived($storeName)` pattern
10. All pages render correctly in the browser

## Validation Commands
- `cd web && npm run build` - Verify Svelte compilation succeeds
- `cd web && npm run dev` - Verify dev server starts without errors
- `cd web && npm test` - Run unit test suite
- `cd web && npm run test:run` - Run tests in non-watch mode
- Grep for remaining Svelte 4 patterns:
  - `grep -r 'export let' web/src/**/*.svelte` - Should return nothing
  - `grep -r '\$:' web/src/**/*.svelte` - Should return nothing (except `$derived`/`$effect`/`$state`)
  - `grep -r 'on:click\|on:change\|on:input\|on:submit' web/src/**/*.svelte` - Should return nothing
  - `grep -r '<slot />' web/src/**/*.svelte` - Should return nothing

## Notes

### Migration Order Rationale
1. **Leaf components first**: Breadcrumb, StatusBadge, RouterLink have no dependencies on other components being migrated, so they can be done in any order.
2. **Components with reactive statements second**: TaskCard, PlanCard, TaskColumn, MarkdownRenderer depend on StatusBadge (already migrated) and have `$:` statements.
3. **Simple pages third**: PlanList has no route params, only store usage and reactive statements.
4. **Complex pages last**: PlanDetail, TaskDetail, TaskList have route params, event handlers, and store usage — most complex changes.

### Known Limitations (deferred)
- **Layout.svelte `window.location` usage**: The `isActive()` function uses `window.location.href` to determine active nav links. In Svelte 5, this works but is not ideal for SSR. Can be addressed later by passing the current route path via Svelte context API or props from the router.

### `$derived` Chain References
Svelte 5 allows `$derived` variables to reference other `$derived` variables. For example in PlanCard.svelte:
```ts
const totalTasks = $derived(plan.tasks?.length ?? 0);
const completedTasks = $derived(plan.tasks?.filter(t => t.completed).length ?? 0);
const progressPercent = $derived(totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0);
```
`progressPercent` references `totalTasks` and `completedTasks` which are both `$derived` — this is valid in Svelte 5.

### TypeScript Type Annotations for `$props()`
When using `$props()` with TypeScript, the type annotation goes on the destructured object:
```ts
let { task, planId, branch }: { task: Task; planId: string; branch: string } = $props();
```
NOT:
```ts
// WRONG - this doesn't work with $props()
let { task }: { task: Task } = $props();
let { planId }: { planId: string } = $props();
let { branch }: { branch: string } = $props();
```

### Store Pattern Summary
| Context | Svelte 4 | Svelte 5 |
|---------|----------|----------|
| Template read | `$storeName` | `$storeName` (same) |
| Template conditional | `{#if $loading}` | `{#if $loading}` (same) |
| Script `.set()` call | `loading.set(true)` | `loading.set(true)` (same) |
| Script read (reactive) | `$loading` | `$derived($loading)` |
| Script read (non-reactive) | `$loading` | `loading` (get value once) or `$derived($loading)` |
