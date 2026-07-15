# Plan: 012 - TaskDetail Execution Features

## Task Description
Add task execution action buttons (Execute and Abandon) and a Baby Step Mode indicator banner to the TaskDetail page. These features enable users to directly transition tasks to "running" or "abandoned" status via buttons, and provide visual feedback when Baby Step Mode is active.

## Objective
- Add "▶ Execute" and "Abandon" action buttons to the TaskDetail topbar
- Add a Baby Step Mode banner/indicator that displays when `yolo_mode` is disabled in config
- Replace the current status dropdown with the new action buttons for primary transitions
- Handle loading states and error states during status transitions

## Problem Statement
The TaskDetail page currently only offers a status dropdown for transitioning tasks. Users need quick-access action buttons for the two most common transitions (execute and abandon). Additionally, users need visual feedback when Baby Step Mode is enabled so they know their transitions will require confirmation.

## Solution Approach
1. Add Execute and Abandon buttons to the topbar area, styled as primary (blue) and danger (red) respectively
2. Fetch the config on mount to determine if Baby Step Mode is active (`yolo_mode === false`)
3. Render a Baby Step Mode banner at the top of the task content area when applicable
4. Reuse the existing `handleTransition` function for button click handlers
5. Maintain the existing status dropdown as a fallback for other transitions

## Relevant Files
- `web/src/pages/TaskDetail.svelte` — Primary file to modify; add buttons and banner
- `web/src/lib/api.ts` — Already has `getConfig()` and `transitionTaskStatus()` functions
- `web/src/lib/types.ts` — Already has `Config` type with `yolo_mode` field and `TransitionTaskStatusRequest`
- `mockups/refined/task-detail.html` — Reference for intended UI layout and styling

### New Files (if needed)
None — all changes are modifications to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: task-detail-builder
  - Role: Implement Execute/Abandon buttons and Baby Step Mode banner in TaskDetail.svelte
  - Agent: builder

- **Validator**
  - Name: task-detail-validator
  - Role: Verify implementation meets criteria, test button interactions and banner visibility
  - Agent: validator

- **Documenter**
  - Name: task-detail-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Add Execute and Abandon buttons to TaskDetail topbar
- **Task ID**: add-execution-buttons
- **Depends On**: none
- **Assigned To**: task-detail-builder
- **Agent**: builder
- **Actions**:
  - Import `getConfig` from `$lib/api` in TaskDetail.svelte
  - Add `config` state variable: `let config = $state<Config | null>(null);`
  - In the `onMount` block, call `getConfig()` alongside `getTask()` to fetch config on page load
  - Add two new button elements to the topbar area (inside the header's right-side div, replacing or alongside the status dropdown):
    - Execute button: styled with `bg-accent-blue border border-accent-blue text-white hover:bg-accent-blue/80 rounded-md text-sm px-3 py-1.5 transition-colors`
    - Abandon button: styled with `bg-accent-red border border-accent-red text-white hover:bg-accent-red/80 rounded-md text-sm px-3 py-1.5 transition-colors`
  - Wire Execute button to call `handleTransition('running')`
  - Wire Abandon button to call `handleTransition('abandoned')`
  - Disable both buttons when `transitioning` is true
  - Show loading indicator (spinner or text) on buttons when transitioning
  - Keep the existing status dropdown for other transitions (or replace it entirely — builder's call based on mockup alignment)
- **Acceptance Criteria**:
  - Execute button appears in the topbar with ▶ icon and "Execute" label
  - Abandon button appears next to Execute button
  - Execute button uses blue (primary) styling
  - Abandon button uses red (danger) styling
  - Both buttons are disabled during a transition (loading state)
  - Clicking Execute transitions task to "running" status
  - Clicking Abandon transitions task to "abandoned" status
  - Error handling works correctly for failed transitions

### 2. Add Baby Step Mode banner
- **Task ID**: add-baby-step-banner
- **Depends On**: add-execution-buttons
- **Assigned To**: task-detail-builder
- **Agent**: builder
- **Actions**:
  - Derive `babyStepMode` from config: `const babyStepMode = $derived(config !== null && !config.yolo_mode);`
  - Add a banner element at the top of the task content area (before the tabs, inside the main content div)
  - Banner styling per mockup: yellow-themed with train icon (🚂)
    - Background: `bg-accent-yellow-subtle` (or equivalent subtle yellow)
    - Border: `border-accent-yellow` with subtle opacity
    - Text color: `text-accent-yellow`
    - Layout: flex row with icon on left, text on right
    - Padding: `p-3` or similar
    - Border radius: `rounded-md`
  - Banner text: "Baby Step Mode: Each status transition requires your confirmation before proceeding."
  - Banner only renders when `babyStepMode` is true
- **Acceptance Criteria**:
  - Banner appears at the top of task content when Baby Step Mode is active
  - Banner has a train icon (🚂) and explanatory text
  - Banner uses yellow accent colors consistent with the existing design system
  - Banner does not appear when yolo_mode is true (Baby Step Mode off)
  - Banner does not appear while config is still loading

### 3. Refine topbar layout
- **Task ID**: refine-topbar-layout
- **Depends On**: add-execution-buttons
- **Assigned To**: task-detail-builder
- **Agent**: builder
- **Actions**:
  - Adjust the header layout to accommodate the new buttons alongside the existing elements
  - Ensure the buttons are right-aligned in the header (matching the mockup's topbar-right layout)
  - Consider removing the status dropdown in favor of the buttons, or keeping it as a secondary option
  - Ensure responsive layout works (buttons don't overflow on narrow screens)
  - Add appropriate spacing (gap) between buttons
- **Acceptance Criteria**:
  - Buttons are properly positioned in the header area
  - Layout is clean and matches the mockup intent
  - No layout overflow or wrapping issues

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-execution-buttons, add-baby-step-banner, refine-topbar-layout
- **Assigned To**: task-detail-validator
- **Agent**: validator
- **Checks**:
  - Verify Execute button transitions task to "running" correctly
  - Verify Abandon button transitions task to "abandoned" correctly
  - Verify loading state disables buttons during transition
  - Verify Baby Step Mode banner appears when config.yolo_mode is false
  - Verify Baby Step Mode banner does not appear when config.yolo_mode is true
  - Verify error handling works for failed API calls
  - Verify the page still renders correctly when config fetch fails (graceful degradation)
  - Run the dev server and visually inspect the UI

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: task-detail-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- Execute button transitions task to "running" status via PATCH API call
- Abandon button transitions task to "abandoned" status via PATCH API call
- Both buttons show loading state during transition (disabled + visual feedback)
- Baby Step Mode banner displays when `yolo_mode` config is `false`
- Baby Step Mode banner is hidden when `yolo_mode` config is `true`
- Config fetch failure does not break the page (banner simply doesn't show)
- Error handling for failed transitions follows existing patterns (setError utility)
- UI matches the mockup intent for layout, colors, and iconography
- No regressions in existing TaskDetail functionality (tabs, breadcrumb, sidebar, etc.)

## Validation Commands
- `cd web && npm run dev` — Start dev server for visual inspection
- `cd web && npm run check` — Run Svelte type checking
- `cd web && npm run lint` — Run ESLint if configured

## Notes
- The existing `handleTransition` function already handles the API call, loading state, and error handling — reuse it directly
- The `getConfig()` API returns a `Config` object with `yolo_mode: boolean`; when `false`, Baby Step Mode is active
- Baby Step Mode means the overlord requires user confirmation before each status transition — the banner is purely informational for the MVP
- The status dropdown can be kept as a fallback for other transitions (queued, reviewing, etc.) or removed entirely — builder should decide based on UX best practices
- The Execute button should probably only be shown/enabled when the task is in a status that can be executed (e.g., "queued" or "pending"), but for MVP, showing it always is acceptable
- Button styling should use the existing Tailwind color tokens (`accent-blue`, `accent-red`, `accent-yellow`) to maintain consistency
- The mockup uses green for the Execute button, but the requirements specify blue (primary) — follow the requirements
- Consider adding a small spinner SVG or loading text inside buttons during transition for better UX
