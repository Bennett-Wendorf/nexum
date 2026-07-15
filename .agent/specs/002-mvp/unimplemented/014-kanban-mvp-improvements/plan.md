# Plan: 014 - Kanban Board MVP Improvements

## Task Description
Add three MVP features to the Kanban board (TaskList page):
1. A Yolo mode toggle button in the topbar that controls the server's `yolo_mode` config setting
2. Blocked dependency indicators on task cards showing unmet dependency wait status
3. Inline Approve/Request Changes buttons in the Manual Review column for manual task review actions

## Objective
Enhance the Kanban board with operational controls (Yolo toggle), dependency visibility (blocked indicators), and manual review workflow (inline approval buttons) to complete the MVP feature set.

## Problem Statement
- Users cannot toggle Yolo mode from the UI; they must edit the config file manually
- Task cards with unmet dependencies provide no visual indication of blocking status
- The Manual Review column requires users to navigate to the task detail page to approve or request changes, adding unnecessary friction

## Solution Approach
1. **Yolo toggle**: Add a `PATCH /api/v1/config` endpoint on the backend to update `yolo_mode`. On the frontend, add a toggle button in the TaskList topbar that fetches config on mount and calls the PATCH API on toggle.

2. **Blocked indicators**: Pass the full tasks list down to TaskCard so it can resolve dependency IDs to task names. Compute unmet dependencies (those not in `completed` or `abandoned` status). Render a 3px yellow left border and a "⌛ Waiting on: [names]" indicator below the card tags.

3. **Inline review actions**: Pass the column status down to TaskCard. In the Manual Review column, render two small inline buttons (Approve → `merge-queue`, Request Changes → `reviewing`) that call `transitionTaskStatus` via the API.

## Relevant Files

### Frontend (web/)
- `web/src/pages/TaskList.svelte` — Main kanban page; needs Yolo toggle button, config fetching, and passing tasks list down to columns
- `web/src/components/TaskCard.svelte` — Individual card; needs blocked indicator logic, inline review buttons, and new props (column status, tasks list)
- `web/src/components/TaskColumn.svelte` — Column wrapper; needs to pass column status and tasks list to TaskCard
- `web/src/lib/api.ts` — API client; needs new `patchConfig` function for yolo_mode toggle
- `web/src/lib/types.ts` — Types; needs `PatchConfigRequest` type for the PATCH endpoint
- `web/src/lib/statusColors.ts` — Status definitions; no changes needed but referenced for column status

### Backend (src/)
- `src/api/config.rs` — Config handlers; needs new `patch_config` handler for yolo_mode
- `src/api/mod.rs` — Router; needs new route registration for `PATCH /api/v1/config`
- `src/api/types.rs` — API types; needs `PatchConfigRequest` struct
- `src/api/tests.rs` — Tests; needs test for the new PATCH endpoint

### Reference
- `mockups/refined/kanban.html` — Visual reference for Yolo button, blocked indicators, and review buttons

### New Files (if needed)
None — all changes are additions to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: backend-builder
  - Role: Implement PATCH /api/v1/config endpoint for yolo_mode toggling
  - Agent: builder

- **Builder**
  - Name: frontend-builder
  - Role: Implement Yolo toggle UI, blocked indicators, and inline review buttons
  - Agent: builder

- **Validator**
  - Name: kanban-validator
  - Role: Verify all three features work correctly end-to-end
  - Agent: validator

- **Documenter**
  - Name: kanban-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Backend: PATCH /api/v1/config endpoint
- **Task ID**: backend-config-patch
- **Depends On**: none
- **Assigned To**: backend-builder
- **Agent**: builder
- **Actions**:
  - Add `PatchConfigRequest` struct to `src/api/types.rs` with optional `yolo_mode: Option<bool>` field
  - Add `patch_config` handler function to `src/api/config.rs` that:
    - Accepts `PATCH /api/v1/config` requests
    - Updates `state.config.preferences.yolo_mode` from the request body
    - Re-persists the config to disk (via `config::loader::save` or equivalent)
    - Returns the updated `ConfigResponse`
  - Register the new route in `src/api/mod.rs`: `.route("/api/v1/config", get(config::get_config).patch(config::patch_config))`
  - Add test in `src/api/tests.rs` for the PATCH endpoint
  - Update OpenAPI spec if applicable
- **Acceptance Criteria**:
  - `PATCH /api/v1/config` with `{"yolo_mode": true}` updates the config and returns the new config
  - `PATCH /api/v1/config` with `{"yolo_mode": false}` updates the config and returns the new config
  - Partial updates work (only `yolo_mode` field sent, other fields unchanged)
  - Config is persisted to disk after update
  - Test suite passes

### 2. Frontend: API layer for config patch
- **Task ID**: frontend-api-config
- **Depends On**: backend-config-patch
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - Add `PatchConfigRequest` interface to `web/src/lib/types.ts`: `{ yolo_mode?: boolean }`
  - Add `patchConfig(req: PatchConfigRequest): Promise<Config>` function to `web/src/lib/api.ts` that calls `PATCH /api/v1/config`
- **Acceptance Criteria**:
  - New types compile without errors
  - `patchConfig` function correctly serializes request and returns Config response

### 3. Frontend: Yolo mode toggle in topbar
- **Task ID**: frontend-yolo-toggle
- **Depends On**: frontend-api-config
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `TaskList.svelte`, add state for `yoloMode: boolean` (fetched on mount via `getConfig()`)
  - Add a toggle button in the topbar right section (next to Add Task button):
    - Label: "⚡ Yolo: ON" when true, "⚡ Yolo: OFF" when false
    - When ON: yellow/amber styling (e.g., `bg-accent-yellow-subtle text-accent-yellow border border-accent-yellow`)
    - When OFF: muted styling (e.g., `text-text-muted border border-border-default`)
    - On click: call `patchConfig({ yolo_mode: !yoloMode })` and update local state
  - Fetch config in `onMount` alongside plan/tasks fetch
- **Acceptance Criteria**:
  - Button appears in topbar next to Add Task button
  - Button shows correct initial state from server config
  - Clicking toggles yolo_mode on server and updates UI
  - ON state has yellow/amber styling, OFF state has muted styling
  - No visual regression of existing topbar elements

### 4. Frontend: Blocked dependency indicators on task cards
- **Task ID**: frontend-blocked-indicators
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `TaskList.svelte`, pass the full `tasks` array down to `TaskColumn` as a new prop `allTasks`
  - In `TaskColumn.svelte`, pass `allTasks` through to each `TaskCard`
  - In `TaskCard.svelte`, add new props: `allTasks: Task[]` and `columnStatus: string`
  - Implement blocked detection logic:
    ```ts
    const unmetDependencies = $derived(
      task.dependencies
        .map(depId => allTasks.find(t => t.id === depId))
        .filter(t => t && !['completed', 'abandoned'].includes(t.status.status))
        .map(t => t!.name)
    );
    const isBlocked = $derived(unmetDependencies.length > 0);
    ```
  - Render blocked indicator when `isBlocked`:
    - Add `border-l-[3px] border-accent-yellow` (or equivalent) to the card container
    - Below the tags/agent section, render: `⌛ Waiting on: {unmetDependencies.join(', ')}`
    - Style: small text, muted color, matching mockup style
- **Acceptance Criteria**:
  - Task cards with unmet dependencies show 3px yellow left border
  - "⌛ Waiting on: [task names]" text appears below card tags
  - Tasks with no unmet dependencies show no blocked indicator
  - Dependency IDs are correctly resolved to task names
  - Completed and abandoned dependencies are excluded from blocked list

### 5. Frontend: Inline review action buttons
- **Task ID**: frontend-review-buttons
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `TaskColumn.svelte`, pass the `status` prop through to `TaskCard` as `columnStatus`
  - In `TaskCard.svelte`, add `columnStatus` prop
  - When `columnStatus === 'waiting-manual-review'`, render two buttons below the card content:
    - **Approve** button: styled as small primary button (referencing mockup), on click calls `transitionTaskStatus(branch, planId, task.id, { status: 'merge-queue' })`
    - **Request Changes** button: styled as small secondary button, on click calls `transitionTaskStatus(branch, planId, task.id, { status: 'reviewing' })`
  - Both buttons: `font-size: 11px`, small padding, inline layout with gap
  - Add loading state handling (disable buttons during API call, show spinner or disabled state)
  - On successful transition, the card should re-render in the new column (via task list re-fetch or optimistic update)
- **Acceptance Criteria**:
  - Approve and Request Changes buttons appear only in Manual Review column
  - Approve button transitions task to `merge-queue` status
  - Request Changes button transitions task to `reviewing` status
  - Buttons are styled per mockup (small, inline, proper colors)
  - After transition, task card moves to appropriate column
  - No buttons appear in other columns

### 6. Integration: Wire props through component hierarchy
- **Task ID**: frontend-prop-wiring
- **Depends On**: frontend-blocked-indicators, frontend-review-buttons
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - Ensure `TaskList.svelte` passes `tasks` to `TaskColumn` as `allTasks`
  - Ensure `TaskColumn.svelte` passes `allTasks` and `status` (as `columnStatus`) to each `TaskCard`
  - Update prop signatures in `TaskColumn` and `TaskCard` accordingly:
    - `TaskColumn`: `{ status, tasks, planId, branch, allTasks }`
    - `TaskCard`: `{ task, planId, branch, allTasks, columnStatus }`
  - Verify no TypeScript compilation errors
- **Acceptance Criteria**:
  - All components compile without type errors
  - Props flow correctly from TaskList → TaskColumn → TaskCard
  - No runtime errors in the browser

### 7. Final Validation
- **Task ID**: validate-all
- **Depends On**: backend-config-patch, frontend-yolo-toggle, frontend-blocked-indicators, frontend-review-buttons, frontend-prop-wiring
- **Assigned To**: kanban-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all backend tests pass including new PATCH config test
  - Run `cd web && npm run check` — Svelte type checking passes
  - Run `cd web && npm run build` — production build succeeds
  - Manual verification checklist:
    - Yolo toggle button appears in topbar with correct initial state
    - Yolo toggle button toggles ON/OFF and updates server config
    - Task cards with unmet dependencies show yellow left border and "⌛ Waiting on:" text
    - Task cards without unmet dependencies show no blocked indicator
    - Manual Review column cards show Approve and Request Changes buttons
    - Approve button moves task to Merge Queue column
    - Request Changes button moves task to Reviewing column
    - No visual regressions in existing kanban board layout

### 8. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: kanban-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/` covering:
    - PATCH /api/v1/config endpoint
    - Yolo mode toggle UI feature
    - Blocked dependency indicators
    - Inline review action buttons

## Acceptance Criteria
- Yolo mode toggle button in topbar correctly toggles `yolo_mode` in server config via API
- Toggle button shows ON/OFF state with appropriate styling (yellow/amber when ON, muted when OFF)
- Task cards with unmet dependencies display a 3px yellow left border highlight
- Blocked tasks show "⌛ Waiting on: [dependency task names]" below card tags
- Dependency names are correctly resolved from the tasks list
- Manual Review column task cards display inline Approve and Request Changes buttons
- Approve button transitions task to `merge-queue` status
- Request Changes button transitions task to `reviewing` status
- All backend tests pass
- Frontend builds without errors
- No visual regressions in existing kanban board

## Validation Commands
- `cargo test` — Run all backend tests
- `cd web && npm run check` — Svelte type checking
- `cd web && npm run build` — Production build
- `curl -X PATCH http://localhost:PORT/api/v1/config -H 'Content-Type: application/json' -d '{"yolo_mode": true}'` — Test PATCH endpoint

## Notes
- The `config::loader` module may need a `save` or `persist` function if one doesn't exist for writing config changes back to disk. If not, the PATCH endpoint may need to write to the config file directly using the loader's serialization logic.
- The `TaskStatus` type has both `dependencies` and `dependent_tasks` fields. For blocked indicators, we use `task.dependencies` (the task's own dependency list) to find which dependencies are unmet.
- When a task transitions status via the review buttons, the TaskList page needs to re-render. Options: re-fetch tasks after transition, or use optimistic UI update. Re-fetching is simpler and safer for MVP.
- The mockup shows the Yolo button in the topbar between the mode toggle and Add Task button. Match this layout.
- The `transitionTaskStatus` API call returns the updated task, which can be used for optimistic UI updates if desired.
