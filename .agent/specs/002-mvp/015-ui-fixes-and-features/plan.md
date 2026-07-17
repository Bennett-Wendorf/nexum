# Plan: 015 - Navigation Fixes, UI Completion, and Feature Enablement

## Task Description
Fix 6 identified navigation bugs, workflow gaps, and disabled features in the Nexum web frontend. The issues span from high-priority navigation regressions (plain `<a>` tags instead of `<RouterLink>` causing full page reloads) to medium-priority feature completion (Add Task button) and low-priority cleanup (label fixes, unused code).

## Objective
Produce a fully navigable SPA with no full-page reloads, a functional task creation workflow, a recoverable 404 page, and cleaned-up UI labels — restoring the expected Svelte SPA experience across the entire application.

## Problem Statement
The Nexum web frontend has several issues that degrade the user experience:
1. **Navigation breaks SPA routing**: Two card components use plain `<a href>` instead of `<RouterLink>`, causing full page reloads and breaking the SPA experience.
2. **Users stranded on 404**: The NotFound page provides no way to navigate back.
3. **Task creation blocked**: The "+ Add Task" button is disabled despite the API already supporting task creation.
4. **Misleading sidebar label**: "Dashboard" icon points to `/` which renders PlanList, not a dashboard.
5. **Dead buttons**: Two sidebar buttons are permanently disabled with "coming soon" labels.
6. **Dead code**: An API function is defined but never used.

## Solution Approach
Apply targeted, surgical fixes to each issue:
- Replace `<a href>` with `<RouterLink>` in PlanCard and TaskCard
- Add a "Go Home" `<RouterLink>` to NotFound
- Create a `CreateTaskForm` component and wire it into TaskList to enable the "+ Add Task" button
- Rename the sidebar icon tooltip from "Dashboard" to "Plans"
- Remove the two "coming soon" disabled buttons from the sidebar
- Remove the unused `getExecutionState` function from api.ts

## Relevant Files

### Existing Files to Modify
| File | Issue(s) Affected |
|------|-------------------|
| `web/src/components/PlanCard.svelte` | Issue 1 (line 13) |
| `web/src/components/TaskCard.svelte` | Issue 1 (line 57) |
| `web/src/pages/NotFound.svelte` | Issue 2 |
| `web/src/pages/TaskList.svelte` | Issue 3 (line 139) |
| `web/src/components/Layout.svelte` | Issue 4 (line 111), Issue 5 (lines 118, 121) |
| `web/src/lib/api.ts` | Issue 6 (line 164) |

### New Files
| File | Purpose |
|------|---------|
| `web/src/components/CreateTaskForm.svelte` | Form component for creating tasks (Issue 3) |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: frontend-builder
  - Role: Implement all navigation fixes, UI changes, and the CreateTaskForm component
  - Agent: builder

- **Validator**
  - Name: frontend-validator
  - Role: Verify all fixes work correctly, SPA routing is intact, and no regressions were introduced
  - Agent: validator

- **Documenter**
  - Name: change-log-writer
  - Role: Generate a changelog summarizing all changes made
  - Agent: documenter

## Step by Step Tasks

### 1. Fix PlanCard Navigation (Issue 1a)
- **Task ID**: fix-plan-card-navigation
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/components/PlanCard.svelte`:
    - Add `import RouterLink from './RouterLink.svelte';` to the script block (line 2 area)
    - On line 13, replace `<a href={href}` with `<RouterLink href={href}`
    - Replace the closing `</a>` on line 35 with `</RouterLink>`
  - Verify the component still compiles and the derived `href` variable is still correctly used
- **Acceptance Criteria**:
  - PlanCard uses `<RouterLink>` instead of `<a>` for navigation
  - Clicking a plan card navigates via SPA routing (no full page reload)
  - All existing classes and styling are preserved

### 2. Fix TaskCard Navigation (Issue 1b)
- **Task ID**: fix-task-card-navigation
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/components/TaskCard.svelte`:
    - Add `import RouterLink from './RouterLink.svelte';` to the script block
    - On line 57, replace `<a href={href}` with `<RouterLink href={href}`
    - Replace the closing `</a>` on line 75 with `</RouterLink>`
  - Verify the component still compiles
- **Acceptance Criteria**:
  - TaskCard uses `<RouterLink>` instead of `<a>` for navigation
  - Clicking a task card navigates via SPA routing (no full page reload)
  - All existing classes, styling, and the review action buttons (which use `e.stopPropagation()`) continue to work correctly

### 3. Add Recovery Link to NotFound Page (Issue 2)
- **Task ID**: fix-not-found-page
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent: builder
- **Actions**:
  - In `web/src/pages/NotFound.svelte`:
    - Add `import RouterLink from '../components/RouterLink.svelte';` to the script block
    - After the `<p class="text-text-muted mt-2">Page not found</p>` line (line 7), add a `<RouterLink>` with a "Go Home" link pointing to `/`
    - Style it as a button or prominent link: `class="inline-block mt-4 px-4 py-2 bg-btn-green text-white rounded-md text-sm font-medium hover:bg-btn-green-hover transition-colors"`
    - Suggested markup:
      ```svelte
      <RouterLink href="/" class="inline-block mt-4 px-4 py-2 bg-btn-green text-white rounded-md text-sm font-medium hover:bg-btn-green-hover transition-colors">
        Go Home
      </RouterLink>
      ```
- **Acceptance Criteria**:
  - NotFound page displays a "Go Home" link
  - Clicking "Go Home" navigates to `/` via SPA routing
  - The link is visually prominent and styled consistently with other buttons in the app

### 4. Create CreateTaskForm Component (Issue 3a)
- **Task ID**: create-task-form
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/CreateTaskForm.svelte`:
    - Follow the same pattern as `CreatePlanForm.svelte` (modal dialog with form fields, validation, loading state)
    - Props: `{ onSubmit, onCancel }` where `onSubmit: (data: CreateTaskRequest) => Promise<void>` and `onCancel: () => void`
    - Form fields:
      - **Task Name** (required, text input) — maps to `CreateTaskRequest.name`
      - **Description** (required, textarea) — maps to `CreateTaskRequest.description`
      - **Dependencies** (optional, comma-separated text input) — maps to `CreateTaskRequest.dependencies` (array of task IDs, parse from comma-separated string)
      - **Acceptance Criteria** (optional, textarea, one per line) — maps to `CreateTaskRequest.acceptance_criteria` (array of strings, split by newline)
      - **Files to Modify** (optional, comma-separated text input) — maps to `CreateTaskRequest.files_to_modify`
      - **Background** (optional, textarea) — maps to `CreateTaskRequest.background`
      - **Notes** (optional, textarea) — maps to `CreateTaskRequest.notes`
    - Validation:
      - Name is required (non-empty)
      - Description is required (non-empty)
      - Dependencies: if provided, split by comma and trim each entry; filter out empty strings
      - Acceptance criteria: if provided, split by newline and trim each entry; filter out empty strings
      - Files to modify: if provided, split by comma and trim each entry; filter out empty strings
    - `parent_plan` is NOT a form field — it will be filled in by the caller (TaskList) using the current `planId`
    - Use the same modal styling pattern as CreatePlanForm (fixed overlay, centered, backdrop blur, click-outside-to-close)
    - Include Escape key handler via `$effect` (same pattern as PlanList.svelte lines 35-44)
  - Import `type { CreateTaskRequest } from '$lib/types'`
- **Acceptance Criteria**:
  - CreateTaskForm.svelte exists and compiles
  - Form validates required fields (name, description)
  - Form parses multi-value fields (dependencies, acceptance_criteria, files_to_modify) from comma/newline separated input
  - Form matches the styling and interaction pattern of CreatePlanForm
  - Submitting calls `onSubmit` with a properly formed `CreateTaskRequest` (with `parent_plan` to be filled by caller)

### 5. Wire Up Add Task in TaskList (Issue 3b)
- **Task ID**: wire-add-task
- **Depends On**: create-task-form
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/pages/TaskList.svelte`:
    - Add `import CreateTaskForm from '../components/CreateTaskForm.svelte';` to the script block
    - Add `import { createTask } from '$lib/api';` to the existing import on line 8
    - Add `import type { CreateTaskRequest } from '$lib/types';` to the existing import on line 10
    - Add `let showCreateTaskForm = $state(false);` state variable
    - Add Escape key handler `$effect` (same pattern as PlanList.svelte lines 35-44) for `showCreateTaskForm`
    - On line 139, replace the disabled button:
      ```svelte
      <!-- BEFORE: -->
      <button class="px-3 py-1.5 bg-btn-green border border-btn-green text-white rounded-md text-sm hover:bg-btn-green-hover transition-colors" disabled title="Coming soon">
        + Add Task
      </button>
      <!-- AFTER: -->
      <button type="button" class="px-3 py-1.5 bg-btn-green border border-btn-green text-white rounded-md text-sm hover:bg-btn-green-hover transition-colors cursor-pointer" onclick={() => showCreateTaskForm = true}>
        + Add Task
      </button>
      ```
    - After the closing `</div>` of the main content (after line 187), add the modal overlay with `showCreateTaskForm`:
      ```svelte
      {#if showCreateTaskForm}
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-bg-primary/80 backdrop-blur-sm" onclick={() => showCreateTaskForm = false}>
          <div class="bg-bg-secondary border border-border-default rounded-lg shadow-2xl w-full max-w-lg mx-4" onclick={(e) => e.stopPropagation()}>
            <CreateTaskForm
              onSubmit={async (data: CreateTaskRequest) => {
                try {
                  const requestData: CreateTaskRequest = {
                    ...data,
                    parent_plan: planId,
                  };
                  const newTask = await createTask(branch, planId, requestData);
                  tasks = [...tasks, newTask];
                  showCreateTaskForm = false;
                } catch (e) {
                  if (e instanceof Error) {
                    errorCleanup = setError(
                      () => { error = e.message; },
                      () => { error = null; }
                    );
                  }
                }
              }}
              onCancel={() => showCreateTaskForm = false}
            />
          </div>
        </div>
      {/if}
      ```
- **Acceptance Criteria**:
  - "+ Add Task" button is enabled and clickable
  - Clicking the button opens the CreateTaskForm modal
  - Submitting a valid task creates the task via API and adds it to the local `tasks` array
  - The new task appears immediately in both kanban and list views
  - Error handling displays API errors via the existing error mechanism
  - Modal closes on cancel, Escape key, or clicking outside

### 6. Fix Dashboard Label (Issue 4)
- **Task ID**: fix-dashboard-label
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/components/Layout.svelte` line 111:
    - Change `title="Dashboard"` to `title="Plans"`
  - Rationale: The `/` route maps to `PlanList` (see `routes/index.ts` line 9), so the label should reflect that.
- **Acceptance Criteria**:
  - Sidebar icon tooltip reads "Plans" instead of "Dashboard"
  - No visual or behavioral changes to the icon itself

### 7. Remove Coming Soon Buttons (Issue 5)
- **Task ID**: remove-coming-soon-buttons
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/components/Layout.svelte`:
    - Remove lines 118-123 (the two disabled buttons for "Agent Teams" and "Settings")
    - Specifically, remove:
      ```svelte
      <button type="button" disabled class="..." title="Agent Teams (coming soon)">
        <svg ...></svg>
      </button>
      <button type="button" disabled class="..." title="Settings (coming soon)">
        <svg ...></svg>
      </button>
      ```
    - The separator `<div class="w-8 h-[1px] bg-border-default my-2"></div>` on line 117 should remain (it separates the main nav items from the now-removed items; since there are no items below it, consider removing the separator too)
    - **Decision**: Remove the separator div (line 117) as well since there are no items below it.
- **Acceptance Criteria**:
  - Sidebar no longer shows disabled "Agent Teams" and "Settings" buttons
  - Sidebar layout is clean with no orphaned separator
  - No layout shift or visual regression

### 8. Remove Unused API Function (Issue 6)
- **Task ID**: remove-unused-api
- **Depends On**: none
- **Assigned To**: frontend-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/lib/api.ts`:
    - Remove the `getExecutionState` function (lines 164-169):
      ```ts
      export async function getExecutionState(
        branch: string,
        planId: string,
      ): Promise<ExecutionState> {
        return request('GET', `/plans/${branch}/${planId}/execution`);
      }
      ```
    - Remove the `getExecutionState` import from any files that import it (grep for usage — should be none per the issue description)
    - Consider also removing `ExecutionState` from the import line 3 if it's no longer used anywhere in the file. Check if `ExecutionState` is referenced elsewhere in api.ts — it's only used in the `getExecutionState` function return type, so it can be removed from the import.
  - Verify no other files import `getExecutionState` from `api.ts`
- **Acceptance Criteria**:
  - `getExecutionState` is removed from api.ts
  - `ExecutionState` is removed from the import line if no longer needed
  - No compilation errors
  - No other files break from the removal

### 9. Final Validation
- **Task ID**: validate-all
- **Depends On**: fix-plan-card-navigation, fix-task-card-navigation, fix-not-found-page, create-task-form, wire-add-task, fix-dashboard-label, remove-coming-soon-buttons, remove-unused-api
- **Assigned To**: frontend-validator
- **Agent**: validator
- **Checks**:
  - Verify `grep -r "<a href" web/src/components/PlanCard.svelte web/src/components/TaskCard.svelte` returns no results (both now use RouterLink)
  - Verify `grep "RouterLink" web/src/components/PlanCard.svelte` shows the import and usage
  - Verify `grep "RouterLink" web/src/components/TaskCard.svelte` shows the import and usage
  - Verify NotFound.svelte contains a `RouterLink` to `/`
  - Verify CreateTaskForm.svelte exists and contains proper form fields
  - Verify TaskList.svelte contains `showCreateTaskForm` state and the CreateTaskForm modal
  - Verify TaskList.svelte line 139 button is no longer `disabled`
  - Verify Layout.svelte no longer contains "Dashboard" in the sidebar tooltip
  - Verify Layout.svelte no longer contains "coming soon" buttons
  - Verify api.ts no longer contains `getExecutionState`
  - Run `cd web && npm run check` (or equivalent Svelte type-check) to verify no compilation errors
  - Run `cd web && npm run build` to verify the app builds successfully
  - Manual verification: Navigate through the app — click plan cards, task cards, 404 page, create a task, verify SPA routing works throughout

### 10. Documentation
- **Task ID**: generate-changelog
- **Depends On**: validate-all
- **Assigned To**: change-log-writer
- **Agent**: documenter
- **Actions**:
  - Create a changelog entry summarizing all 6 issues fixed with:
    - Issue number and description
    - Files modified
    - Specific changes made
    - New files created

## Acceptance Criteria
- All navigation links use `<RouterLink>` — no plain `<a href>` in card components
- NotFound page has a functional "Go Home" recovery link
- "+ Add Task" button is fully functional with a proper form and API integration
- Sidebar labels are accurate ("Plans" not "Dashboard")
- No disabled "coming soon" buttons remain in the sidebar
- No unused API functions remain in api.ts
- All changes compile without errors
- `npm run build` succeeds
- SPA routing works correctly throughout the application

## Validation Commands
- `grep -rn "<a href" web/src/components/PlanCard.svelte web/src/components/TaskCard.svelte` — Should return no results
- `grep -rn "RouterLink" web/src/components/PlanCard.svelte` — Should show import and usage
- `grep -rn "RouterLink" web/src/components/TaskCard.svelte` — Should show import and usage
- `grep -rn "Go Home" web/src/pages/NotFound.svelte` — Should show the recovery link
- `grep -rn "showCreateTaskForm" web/src/pages/TaskList.svelte` — Should show the state and modal
- `grep -rn "Dashboard" web/src/components/Layout.svelte` — Should return no results
- `grep -rn "coming soon" web/src/components/Layout.svelte` — Should return no results
- `grep -rn "getExecutionState" web/src/lib/api.ts` — Should return no results
- `cd web && npm run check` — Should pass with no errors
- `cd web && npm run build` — Should succeed

## Notes
- **Ordering rationale**: Tasks 1-2 (navigation fixes) are independent and can be done in parallel. Task 3 (NotFound) is also independent. Tasks 4-5 (CreateTaskForm + wire up) are dependent on each other. Tasks 6-8 (label fix, button removal, API cleanup) are all independent. All can proceed in parallel except 4→5 dependency.
- **CreateTaskForm pattern**: The form follows the exact same modal pattern as CreatePlanForm. The key difference is that `parent_plan` is not a user input — it's filled in by the caller (TaskList) using the current `planId` prop.
- **SPA routing**: The `RouterLink` component wraps a plain `<a>` tag but intercepts clicks with `push()` from `svelte-spa-router`. It preserves modifier-key behavior (Ctrl+click opens in new tab). Both PlanCard and TaskCard need to import it from the same relative path as other components use it (`'./RouterLink.svelte'`).
- **Task dependencies input**: The CreateTaskForm accepts dependencies as comma-separated task IDs. The form parses them into a string array. Users should know the task IDs they want to depend on — this is a simple UX suitable for MVP.
- **Dead code removal**: `getExecutionState` was defined for future use (monitoring execution state). Removing it keeps the codebase clean. If needed later, it can be re-added easily.
