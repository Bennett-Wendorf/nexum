# Task Detail Monitoring Features

## Overview

Two monitoring features were added to the TaskDetail page to give users visibility into agent activity and the constraints under which the agent operates:

1. **Agent Output tab** — Displays real-time, timestamped, color-coded execution logs from the agent during task execution.
2. **Permissions sidebar** — Shows what permissions the assigned agent has for this task across five operation categories.

These features address the prior lack of transparency during task execution, making it easier to monitor progress, debug issues, and understand why certain actions may be blocked or require approval.

---

## What Was Built

### Agent Output Tab

A new tab positioned between "Task Definition" and "Status History" in the TaskDetail page. It renders execution logs in a dark, monospace, scrollable container with:

- **Timestamped entries** — Each log line shows a formatted time (HH:MM:SS, 24-hour).
- **Severity-based color coding** — `info` (blue), `success` (green), `warn` (yellow), `error` (red).
- **Real-time polling** — When the task status is `running`, logs are re-fetched every 3 seconds.
- **Auto-scroll** — The log container scrolls to the bottom when new entries arrive.
- **Graceful states** — Loading spinner during initial fetch, "No logs available" for unstarted tasks, and a notice ("Task is not running — showing last known log") when the task is not active.

### Permissions Sidebar Section

A new sidebar section placed between "Dependencies" and "Status Info". It displays five permission categories:

| Category | Description |
|---|---|
| File writes | Permission to modify files |
| Terminal commands | Permission to run shell commands |
| Network requests | Permission to make network calls |
| Git operations | Permission to perform git actions |
| Package installs | Permission to install packages |

Each category shows a colored badge: **green** (`auto`), **yellow** (`manual`), or **red** (`blocked`). If permissions data is unavailable, it displays "Permissions not configured".

---

## Technical Implementation

### Files Modified

| File | Changes |
|---|---|
| `web/src/lib/types.ts` | Added `LogEntry`, `PermissionLevel`, and `TaskPermissions` types |
| `web/src/lib/api.ts` | Added `getTaskLogs()` function; imported `LogEntry` type |
| `web/src/pages/TaskDetail.svelte` | Added Agent Output tab, Permissions sidebar, polling logic, and auto-scroll |

### New Types (`types.ts`)

```typescript
export interface LogEntry {
  timestamp: string;       // ISO datetime string
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

### New API Function (`api.ts`)

```typescript
export async function getTaskLogs(
  branch: string,
  planId: string,
  taskId: string,
  options?: { signal?: AbortSignal },
): Promise<LogEntry[]>
```

- Calls `GET /plans/{branch}/{planId}/tasks/{taskId}/logs`.
- Supports `AbortSignal` for cancellation during polling lifecycle.
- **Note**: The backend endpoint may not exist yet — a TODO comment documents the expected API contract.

### Key Implementation Details (`TaskDetail.svelte`)

- **Module-level constants** — `PERMISSION_BADGE_CLASSES` and `SEVERITY_COLORS` maps define Tailwind class strings for badges and log severity colors.
- **State management** — Uses Svelte 5 runes (`$state`, `$derived`, `$effect`) throughout. No legacy `$:` stores.
- **Extended log type** — `LogEntryWithTime` extends `LogEntry` with a `formattedTime` string for display.
- **Polling logic** — A `$effect` watches `isTaskRunning` (a `$derived` value). When true, it starts a 3-second `setInterval` calling `fetchLogs()` with an `AbortController` for clean cancellation.
- **Auto-scroll** — A `$effect` on `logEntries.length` triggers `queueMicrotask(scrollToBottom)` to scroll the log container to the bottom after new entries arrive.
- **Permission loading** — `loadPermissions()` calls `listAgents()`, matches the assigned agent by `role`, and maps `tool_permissions` to the five categories. Falls back to all `manual` if no permissions are defined.

---

## Usage

### Viewing Agent Output

1. Navigate to a task's detail page (`/plans/{branch}/{planId}/tasks/{taskId}`).
2. Click the **Agent Output** tab (between Task Definition and Status History).
3. If the task is running, logs update automatically every 3 seconds.
4. If the task is not running, the last known logs are displayed with a notice.

### Viewing Permissions

The Permissions section appears in the right sidebar by default — no interaction required. It displays the five permission categories with color-coded badges based on the assigned agent's configuration.

---

## Configuration

No additional configuration is required. The features use existing Tailwind CSS custom color tokens (`accent-blue`, `accent-green`, `accent-yellow`, `accent-red`, and their `-subtle` variants) for consistent styling.

The polling interval is hardcoded to **3000ms** (`LOG_POLL_INTERVAL_MS` constant in `TaskDetail.svelte`).

---

## Known Limitations

1. **Permissions data source is an MVP fallback** — Permissions are derived from `AgentRegistration.tool_permissions` via `listAgents()`. If the agent has any `tool_permissions`, all categories are set to `auto`; otherwise, all are set to `manual`. This does not provide granular per-category control. The target design is for the backend to return task-specific permissions.

2. **Backend log endpoint may not exist yet** — `getTaskLogs()` calls `GET /plans/{branch}/{planId}/tasks/{taskId}/logs`, which may not be implemented on the backend. If the endpoint is missing, log fetching will fail and display an error indicator in the Agent Output tab.

3. **Full log fetch on each poll** — Every poll retrieves the complete log history rather than incremental updates. This works for MVP but is inefficient for long-running tasks with many log entries.

---

## Future Improvements

1. **Granular permission mapping** — Replace the all-or-nothing permission logic with a proper mapping from `tool_permissions` to individual categories (e.g., `file_writes` maps to specific tool permission strings).

2. **Task-level permissions endpoint** — Have the backend return task-specific permissions rather than inferring them from agent configuration.

3. **ARIA tab roles** — Add `role="tab"`, `role="tabpanel"`, `aria-selected`, and `aria-controls` attributes to the tab bar for improved accessibility.

4. **Incremental log fetching** — Add a `since` or `lastLogId` parameter to the logs API so only new entries are fetched during polling, reducing bandwidth and improving performance.

5. **Log filtering/search** — Allow users to filter logs by severity level or search for specific messages.

6. **Log persistence** — Consider caching logs locally (e.g., IndexedDB) to reduce API calls and provide offline access to previously fetched logs.
