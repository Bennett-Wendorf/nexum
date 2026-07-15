# Plan: 013 - TaskDetail Monitoring Features

## Task Description
Add two MVP monitoring features to the TaskDetail page: (1) an "Agent Output" tab that shows real-time execution logs from the agent during task execution, and (2) a "Permissions" sidebar section that displays what permissions the agent has for this task. These features give users visibility into agent activity and the constraints under which the agent operates.

## Objective
- Add an "Agent Output" tab (between Task Definition and Status History) that displays timestamped, color-coded log lines from agent execution
- Implement real-time polling for log updates when the task is in a running state
- Add a "Permissions" section to the sidebar showing permission levels (auto/manual/blocked) for five operation categories
- Ensure both features integrate cleanly with the existing TaskDetail layout and Svelte 5 runes patterns

## Problem Statement
Users currently have no visibility into what the agent is doing during task execution. They cannot see the agent's activity log, nor can they understand what permissions the agent has. This lack of transparency makes it difficult to monitor progress, debug issues, and understand why certain actions may be blocked or require approval.

## Solution Approach
1. **New types**: Define `LogEntry` and `TaskPermissions` types in `types.ts`
2. **New API function**: Add `getTaskLogs()` to `api.ts` for fetching execution logs
3. **Agent Output tab**: Create a new tab component that renders timestamped log lines with severity-based color coding, with polling when task is running
4. **Permissions sidebar**: Add a new sidebar section rendering permission categories with colored badges (green=auto, yellow=manual, red=blocked)
5. **Data strategy**: For MVP, permissions data will come from the `AgentRegistration.tool_permissions` field fetched via the existing `listAgents()` API, or from a new task-level endpoint if the backend provides it. Log data will come from a new `GET /plans/{branch}/{planId}/tasks/{taskId}/logs` endpoint.

## Relevant Files
- `web/src/pages/TaskDetail.svelte` — Primary file to modify; add new tab and sidebar section
- `web/src/lib/api.ts` — Add new `getTaskLogs()` function
- `web/src/lib/types.ts` — Add `LogEntry`, `TaskPermissions` types
- `mockups/refined/task-detail.html` — Reference for intended UI layout and styling
- `web/src/lib/statusColors.ts` — Reference for existing color pattern conventions

### New Files (if needed)
- `web/src/components/AgentLog.svelte` — Reusable log display component (optional; may be inlined in TaskDetail for MVP)
- `web/src/components/PermissionBadge.svelte` — Reusable permission level badge component (optional; may be inlined for MVP)

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: monitoring-builder
  - Role: Implement Agent Output tab and Permissions sidebar section in TaskDetail
  - Agent: builder

- **Validator**
  - Name: monitoring-validator
  - Role: Verify implementation meets criteria, test polling behavior, verify UI styling
  - Agent: validator

- **Documenter**
  - Name: monitoring-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Add types for log entries and permissions
- **Task ID**: add-monitoring-types
- **Depends On**: none
- **Assigned To**: monitoring-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/lib/types.ts`, add the following types:
    ```typescript
    export interface LogEntry {
      timestamp: string;  // ISO datetime string
      message: string;
      severity: 'info' | 'warn' | 'error' | 'success';
    }
    
    export type PermissionLevel = 'auto' | 'manual' | 'blocked';
    
    export interface TaskPermissions {
      file_writes: PermissionLevel;
      terminal_commands: PermissionLevel;
      network_requests: PermissionLevel;
      git_operations: PermissionLevel;
      package_installs: PermissionLevel;
    }
    ```
  - Extend the `Task` interface to optionally include `permissions?: TaskPermissions` and `logs?: LogEntry[]` fields (or keep them separate if the backend serves them via different endpoints)
- **Acceptance Criteria**:
  - New types are properly defined in `types.ts`
  - Types follow existing naming conventions (camelCase, PascalCase for interfaces)
  - TypeScript compilation passes with no errors

### 2. Add API function for fetching task logs
- **Task ID**: add-task-logs-api
- **Depends On**: add-monitoring-types
- **Assigned To**: monitoring-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/lib/api.ts`, add:
    ```typescript
    export async function getTaskLogs(
      branch: string,
      planId: string,
      taskId: string,
    ): Promise<LogEntry[]> {
      return request('GET', `/plans/${branch}/${planId}/tasks/${taskId}/logs`);
    }
    ```
  - Import `LogEntry` type at the top of the file
  - For MVP, if the backend endpoint doesn't exist yet, add a comment noting the expected API contract
- **Acceptance Criteria**:
  - `getTaskLogs` function is exported from `api.ts`
  - Function signature matches the pattern of other API functions (branch, planId, taskId params)
  - Returns typed `LogEntry[]`
  - TypeScript compilation passes

### 3. Add Agent Output tab to TaskDetail
- **Task ID**: add-agent-output-tab
- **Depends On**: add-monitoring-types, add-task-logs-api
- **Assigned To**: monitoring-builder
- **Agent**: builder
- **Actions**:
  - In `TaskDetail.svelte`:
    - Import `getTaskLogs` from `$lib/api`
    - Import `LogEntry` from `$lib/types`
    - Add state: `let logEntries = $state<LogEntry[]>([]);`
    - Add state: `let logLoading = $state(false);`
    - Add state: `let logPollInterval: ReturnType<typeof setInterval> | null = $state(null);`
    - Update `activeTab` type to include `'agent-output'`: `let activeTab = $state<'definition' | 'agent-output' | 'history'>('definition');`
    - Add a new tab button between "Task Definition" and "Status History" in the tab bar
    - Add the Agent Output tab content section (between definition and history blocks)
    - Implement a `fetchLogs()` async function that calls `getTaskLogs(branch, planId, taskId)` and updates `logEntries`
    - Call `fetchLogs()` in the `onMount` block
    - Implement polling logic: when task status is 'running', start an interval that calls `fetchLogs()` every 3 seconds
    - Clean up the interval in an `$effect` cleanup or `onDestroy`
    - Render log entries as a scrollable list with:
      - Monospace font (`font-mono`)
      - Dark background (`bg-bg-primary`)
      - Border (`border border-border-default rounded-lg`)
      - Max height with overflow scroll (`max-h-96 overflow-y-auto`)
      - Each line shows: timestamp (in `text-text-faint`) + severity indicator + message
      - Color coding per severity:
        - info: `text-accent-blue`
        - success: `text-accent-green`
        - warn: `text-accent-yellow`
        - error: `text-accent-red`
      - Default message color: `text-text-muted`
      - Auto-scroll to bottom when new entries arrive
    - Show a loading state while initial logs are being fetched
    - Show a message "No logs available" when the task has not started or logs are empty
    - Show "Task is not running — showing last known log" when task is not in 'running' status
  - Tab button styling: match existing tab buttons (use the same conditional class pattern)
- **Acceptance Criteria**:
  - "Agent Output" tab appears between "Task Definition" and "Status History"
  - Tab button styling matches existing tabs (blue underline when active)
  - Log entries render with monospace font in a dark scrollable container
  - Timestamps are displayed in faint color
  - Messages are color-coded by severity (blue=info, green=success, yellow=warn, red=error)
  - Polling starts when task status is 'running' and stops otherwise
  - Polling interval is cleaned up when component unmounts or task status changes
  - Auto-scroll to bottom works when new log entries arrive
  - Empty state shows appropriate message
  - Loading state is shown during initial fetch

### 4. Add Permissions sidebar section
- **Task ID**: add-permissions-sidebar
- **Depends On**: add-monitoring-types
- **Assigned To**: monitoring-builder
- **Agent**: builder
- **Actions**:
  - In `TaskDetail.svelte`:
    - Import `TaskPermissions`, `PermissionLevel` from `$lib/types`
    - Determine the data source for permissions:
      - Option A (preferred): If the backend returns permissions as part of the task response, use `task.permissions`
      - Option B (MVP fallback): Fetch from `listAgents()` and match the assigned agent's `tool_permissions` field
      - For MVP, implement Option B with a comment noting Option A is the target
    - Add a new sidebar section titled "Permissions" (use the same section header pattern: `text-xs font-semibold text-text-muted uppercase tracking-wider mb-3`)
    - Add the section after "Dependencies" and before "Status Info" in the sidebar
    - Render five permission categories as rows:
      - File writes
      - Terminal commands
      - Network requests
      - Git operations
      - Package installs
    - Each row layout: label on left, colored badge on right
    - Badge styling per permission level:
      - auto: `bg-accent-green-subtle text-accent-green` (green)
      - manual: `bg-accent-yellow-subtle text-accent-yellow` (yellow)
      - blocked: `bg-accent-red-subtle text-accent-red` (red)
    - Badge format: `inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold`
    - Row styling: `flex items-center justify-between py-2 text-sm border-b border-border-muted`
    - Label styling: `text-text-secondary`
    - If permissions data is unavailable, show "Permissions not configured" in `text-text-faint italic`
  - Consider creating a small helper function or derived value to map permission level strings to color classes
- **Acceptance Criteria**:
  - "Permissions" section appears in the sidebar between "Dependencies" and "Status Info"
  - Section header uses the same uppercase tracking-wider style as other sidebar sections
  - Five permission categories are displayed (File writes, Terminal commands, Network requests, Git operations, Package installs)
  - Each row has the label on the left and a colored badge on the right
  - Badge colors match the specification: green=auto, yellow=manual, red=blocked
  - Badge styling uses the existing subtle background tint pattern (e.g., `accent-green-subtle`)
  - Graceful fallback when permissions data is unavailable
  - Layout is consistent with other sidebar sections

### 5. Handle polling lifecycle and edge cases
- **Task ID**: handle-polling-lifecycle
- **Depends On**: add-agent-output-tab
- **Assigned To**: monitoring-builder
- **Agent**: builder
- **Actions**:
  - Implement a `$derived` or computed check: `const isTaskRunning = $derived(task?.status.status === 'running');`
  - Use `$effect` to manage the polling interval:
    ```typescript
    $effect(() => {
      if (isTaskRunning && task) {
        const interval = setInterval(() => fetchLogs(), 3000);
        logPollInterval = interval;
        return () => clearInterval(interval);
      }
    });
    ```
  - Ensure the interval is cleared when:
    - Task status changes away from 'running'
    - Component is destroyed (use `$effect` cleanup)
    - User navigates away from the Agent Output tab (optional optimization)
  - Handle API errors in `fetchLogs()` gracefully:
    - Don't crash the page
    - Optionally show an error indicator in the log view
    - Continue polling (errors may be transient)
  - Implement auto-scroll: use a `use:action` directive or a ref-based approach to scroll to bottom when `logEntries.length` changes
- **Acceptance Criteria**:
  - Polling starts automatically when task status is 'running'
  - Polling stops when task status changes away from 'running'
  - Polling interval is cleaned up on component destroy
  - API errors during polling don't crash the page
  - Log view auto-scrolls to bottom when new entries arrive
  - No memory leaks from uncleared intervals

### 6. Refine UI styling and mockup alignment
- **Task ID**: refine-ui-styling
- **Depends On**: add-agent-output-tab, add-permissions-sidebar
- **Assigned To**: monitoring-builder
- **Agent**: builder
- **Actions**:
  - Compare the rendered UI against `mockups/refined/task-detail.html`
  - Ensure the Agent Output tab log display matches the mockup's `.exec-log` styling:
    - Monospace font
    - Dark background
    - Border
    - Timestamp + message format
  - Ensure the Permissions section matches the mockup's `.perm-row` and `.perm-value` styling:
    - Label + badge layout
    - Color coding
    - Border separators between rows
  - Adjust spacing, padding, and border-radius to match existing patterns
  - Ensure responsive behavior (no overflow issues on narrow screens)
  - Test dark mode compatibility (all colors should be visible in the dark theme)
- **Acceptance Criteria**:
  - Agent Output tab visual style matches the mockup
  - Permissions sidebar section visual style matches the mockup
  - Consistent spacing and alignment with existing UI elements
  - No layout overflow or wrapping issues
  - All elements visible in dark mode

### 7. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-monitoring-types, add-task-logs-api, add-agent-output-tab, add-permissions-sidebar, handle-polling-lifecycle, refine-ui-styling
- **Assigned To**: monitoring-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npm run check` — Verify TypeScript compilation passes
  - Run `cd web && npm run dev` — Start dev server for visual inspection
  - Verify Agent Output tab appears and renders correctly
  - Verify log entries display with correct timestamp format and color coding
  - Verify polling works when task status is 'running'
  - Verify polling stops when task status changes
  - Verify Permissions sidebar section displays all five categories
  - Verify permission badges use correct colors (green/yellow/red)
  - Verify empty states render correctly (no logs, no permissions)
  - Verify error states don't crash the page
  - Verify auto-scroll works in the log view
  - Verify no regressions in existing TaskDetail functionality
  - Visually compare against the mockup for alignment

### 8. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: monitoring-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- Agent Output tab is present between Task Definition and Status History tabs
- Agent Output tab displays timestamped log entries with monospace font in a dark scrollable container
- Log entries are color-coded by severity: blue (info), green (success), yellow (warn), red (error)
- Real-time polling occurs every 3 seconds when task status is 'running'
- Polling stops when task status changes away from 'running'
- Polling interval is properly cleaned up on component destroy
- Auto-scroll to bottom works when new log entries arrive
- Permissions sidebar section displays five categories: File writes, Terminal commands, Network requests, Git operations, Package installs
- Permission badges use correct colors: green (auto), yellow (manual), red (blocked)
- Permission badges use the existing subtle background tint pattern
- Empty/error states are handled gracefully without crashing the page
- UI styling matches the mockup reference
- No regressions in existing TaskDetail functionality
- TypeScript compilation passes with no errors

## Validation Commands
- `cd web && npm run check` — Run Svelte type checking
- `cd web && npm run dev` — Start dev server for visual inspection
- `cd web && npm run lint` — Run ESLint if configured

## Notes
- **Backend dependency**: The `getTaskLogs()` API function assumes a `GET /plans/{branch}/{planId}/tasks/{taskId}/logs` endpoint exists. If the backend doesn't have this endpoint yet, the builder should add a TODO comment and potentially mock the data for frontend development.
- **Permissions data source**: For MVP, permissions can come from the `AgentRegistration.tool_permissions` array (fetched via `listAgents()`). The builder should map the agent's tool permissions to the five permission categories. A more robust approach would be to have the backend return task-specific permissions, which should be noted as a future improvement.
- **Log polling optimization**: Consider adding a `lastLogId` or `since` parameter to the logs API to fetch only new entries instead of the full log on each poll. This is a future optimization.
- **Svelte 5 runes**: All state management should use `$state`, `$derived`, and `$effect` (not the legacy `$:` syntax or writable stores).
- **Tab ordering**: The tab order should be: Task Definition → Agent Output → Status History.
- **Sidebar ordering**: The sidebar section order should be: Agent Assignment → Dependencies → Permissions → Status Info.
- **Color tokens**: Use the existing Tailwind custom color tokens (`accent-blue`, `accent-green`, `accent-yellow`, `accent-red`, `accent-green-subtle`, etc.) for consistency.
- **Accessibility**: Ensure log entries and permission badges have sufficient contrast in the dark theme.
