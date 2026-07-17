# 015 — Navigation Fixes, UI Completion, and Feature Enablement

## Overview

This spec restored a fully navigable single-page application (SPA) experience by fixing six issues that caused full-page reloads, stranded users, blocked workflows, misleading labels, dead UI buttons, and unused code.

---

## Issues Fixed

### 1. Navigation breaks SPA routing
**Problem:** `PlanCard` and `TaskCard` used plain `<a href>` elements, causing full page reloads and breaking the SPA experience.

**Fix:** Replaced `<a href>` with `<RouterLink>` in both components so clicks are intercepted by `svelte-spa-router` and navigate client-side.

### 2. Users stranded on 404
**Problem:** The `NotFound` page provided no way to navigate back to the application.

**Fix:** Added a styled "Go Home" `<RouterLink>` button pointing to `/` so users can recover from a 404 without refreshing.

### 3. Task creation blocked
**Problem:** The "+ Add Task" button in `TaskList` was permanently disabled with a "Coming soon" tooltip, despite the API already supporting task creation.

**Fix:** Created a `CreateTaskForm` modal component and wired it into `TaskList`. The button is now enabled and opens a form with fields for task name, description, dependencies, acceptance criteria, files to modify, background, and notes. On submit, the form calls `createTask()` via the API and appends the new task to the local state.

### 4. Misleading sidebar label
**Problem:** The sidebar home icon had `title="Dashboard"` but the `/` route renders `PlanList`, not a dashboard.

**Fix:** Renamed the tooltip to `title="Plans"` to accurately reflect the destination.

### 5. Dead buttons in sidebar
**Problem:** Two permanently disabled buttons ("Agent Teams" and "Settings") with "coming soon" tooltips cluttered the sidebar.

**Fix:** Removed both buttons and the orphaned separator `<div>` that separated them from the main nav items.

### 6. Dead code in api.ts
**Problem:** The `getExecutionState` function was defined in `api.ts` but never imported or used anywhere.

**Fix:** Removed the `getExecutionState` function and the unused `ExecutionState` type from the import statement.

---

## Files Modified

| File | Change |
|------|--------|
| `web/src/components/PlanCard.svelte` | Added `RouterLink` import; replaced `<a href>` with `<RouterLink>` |
| `web/src/components/TaskCard.svelte` | Added `RouterLink` import; replaced `<a href>` with `<RouterLink>` |
| `web/src/pages/NotFound.svelte` | Added `RouterLink` import and a "Go Home" recovery link |
| `web/src/components/Layout.svelte` | Changed home icon tooltip from "Dashboard" to "Plans"; removed two "coming soon" disabled buttons and the separator div |
| `web/src/lib/api.ts` | Removed `getExecutionState` function; removed `ExecutionState` from imports |
| `web/src/pages/TaskList.svelte` | Imported `CreateTaskForm` and `createTask`; added `showCreateTaskForm` state with Escape-key handler; replaced disabled "+ Add Task" button with a clickable one; added modal overlay rendering `CreateTaskForm` |

## New Files

| File | Purpose |
|------|---------|
| `web/src/components/CreateTaskForm.svelte` | Modal form component for creating tasks. Validates required fields (name, description), parses comma-separated dependencies and files-to-modify, and parses newline-separated acceptance criteria. Accepts `onSubmit` and `onCancel` callback props. |

---

## Technical Details

### RouterLink Navigation (PlanCard, TaskCard)
Both components import `RouterLink` from `./RouterLink.svelte` and wrap the card content with `<RouterLink href={href}>`. The `href` is a `$derived` value computed from the plan/task ID and branch. Review action buttons inside `TaskCard` use `e.stopPropagation()` to prevent the RouterLink from intercepting clicks on approve/request-changes buttons.

### CreateTaskForm Component
- **Required fields:** `name`, `description` — validated before submit
- **Optional fields:** `dependencies` (comma-separated task IDs), `acceptance_criteria` (newline-separated strings), `files_to_modify` (comma-separated paths), `background`, `notes`
- `parent_plan` is set by the caller (`TaskList`) using the current `planId` prop
- Form follows the same modal styling pattern as `CreatePlanForm` (fixed overlay, backdrop blur, click-outside-to-close)

### TaskList Integration
- `showCreateTaskForm` is a `$state(false)` variable toggled by the "+ Add Task" button click
- An `$effect` watches `showCreateTaskForm` and registers an Escape-key listener to close the modal
- On successful submit, the new task is appended to the local `tasks` array and the modal closes
- Errors are handled via the existing `setError` utility

### Sidebar Cleanup
`Layout.svelte` sidebar now contains only the two functional navigation links (home icon → `/`, plans icon → `/plans`). The "Agent Teams" and "Settings" placeholder buttons and the separator div between them have been removed.

### API Cleanup
`api.ts` no longer exports `getExecutionState` or imports `ExecutionState`. The remaining exported functions cover plan CRUD, task CRUD, monitoring (task logs, running tasks), health check, config, and agent listing.
