# Task Detail Execution Features

## Overview

Adds task execution action buttons (**Execute** and **Abandon**) and a **Baby Step Mode** indicator banner to the TaskDetail page. These features let users directly transition tasks to "running" or "abandoned" status via prominent buttons, and provide visual feedback when Baby Step Mode is active (requiring confirmation before each transition).

## What Was Built

Three features were implemented in `TaskDetail.svelte`:

1. **Execute Button** — Transitions the task to `running` status with a blue (primary) styled button.
2. **Abandon Button** — Transitions the task to `abandoned` status with a red (danger) styled button. Shows a confirmation dialog before proceeding.
3. **Baby Step Mode Banner** — A yellow-themed banner that appears at the top of the task content area when `yolo_mode` is `false` in the server config, informing the user that each status transition requires confirmation.

The existing status dropdown was retained as a fallback for other transitions (queued, reviewing, merge-queue, completed, etc.). Note: the `running` and `abandoned` options were removed from the dropdown since they are now handled by the dedicated Execute and Abandon buttons.

**Note:** The status dropdown pre-existed in the codebase; it was not part of this feature. It was retained and modified (removing `running` and `abandoned` options) to avoid duplication with the new buttons.

## Technical Implementation

### Files Modified

- **`web/src/pages/TaskDetail.svelte`** — Primary file; all changes are here.

### Imports Added

- `getConfig` from `$lib/api` — Fetches server configuration to determine Baby Step Mode status.
- `Config` type from `$lib/types` — TypeScript interface for the config response.

### State Management

| State Variable | Type | Purpose |
|---|---|---|
| `config` | `Config \| null` | Holds server config fetched on mount |
| `babyStepMode` | `$derived` boolean | `true` when `config.yolo_mode === false` |
| `executing` | boolean | Guard flag for the Execute button during in-flight transitions |
| `abandoning` | boolean | Guard flag for the Abandon button during in-flight transitions |

Both `getTask()` and `getConfig()` are fetched in parallel via `Promise.all` inside `onMount`.

### Key Functions

- **`handleTransition(newStatus: string)`** — Reused for both buttons. Calls `transitionTaskStatus()` via PATCH to `/plans/{branch}/{planId}/tasks/{taskId}/status`, sets the appropriate loading guard (`executing` or `abandoning`), and handles errors via `setError()` utility. Errors during transitions are displayed in a red error banner at the top of the content area.

### API Calls

| Function | Method | Endpoint |
|---|---|---|
| `getConfig()` | `GET` | `/config` |
| `transitionTaskStatus()` | `PATCH` | `/plans/{branch}/{planId}/tasks/{taskId}/status` |

### Config Type

```typescript
interface Config {
  server_host: string;
  server_port: number;
  max_parallel: number;
  default_timeout_seconds: number;
  log_level: string;
  yolo_mode: boolean;   // false = Baby Step Mode active
}
```

## Usage

### Execute Button
- Click **▶ Execute** to transition the task to `running` status.
- During transition, the button shows a spinner with "Executing..." text and is disabled.

### Abandon Button
- Click **Abandon** to transition the task to `abandoned` status.
- A confirmation dialog is shown before the transition proceeds.
- During transition, the button shows a spinner with "Abandoning..." text and is disabled.

### Baby Step Mode Banner
- Appears automatically when the server config has `yolo_mode: false`.
- Displays: "🚂 Baby Step Mode: Each status transition requires your confirmation before proceeding."
- Hidden when `yolo_mode: true` or while config is still loading.
- Also checks `!loading` to prevent a brief flash of the banner on initial page load before the config is fetched.

### Status Dropdown (Fallback)
- Remains available for transitions not covered by the buttons (queued, reviewing, manual-review, merge-queue, completed).
- The `running` and `abandoned` options were removed from the dropdown since they are handled by the dedicated Execute and Abandon buttons.

### Error Handling
- Errors during transitions are displayed in a red error banner at the top of the content area.

## UI Details

### Button Styling

- **Execute**: `bg-accent-blue border border-accent-blue text-white hover:bg-accent-blue/80 rounded-md text-sm px-3 py-1.5`
- **Abandon**: `bg-accent-red border border-accent-red text-white hover:bg-accent-red/80 rounded-md text-sm px-3 py-1.5`
- Both share: `disabled:opacity-50 disabled:cursor-not-allowed transition-colors`

### Banner Styling

- Background: `bg-yellow-500/10`
- Border: `border-yellow-500/30`
- Text: `text-yellow-400`
- Layout: Flex row with train emoji (🚂) icon on the left, text on the right
- Padding: `p-3`, `rounded-md`

### Topbar Layout

The header's right-side area contains a vertical flex column (`flex-col items-end gap-2`):
1. A horizontal row with the Execute and Abandon buttons
2. The status dropdown select element below them

### Loading States

Loading states use per-button granularity:
- The **Execute** button is only disabled when `executing` is `true` (its own transition is in progress).
- The **Abandon** button is only disabled when `abandoning` is `true` (its own transition is in progress).
- The status dropdown is disabled when either `executing` or `abandoning` is active.

Both buttons display an inline SVG spinner (`animate-spin`) with contextual text ("Executing..." / "Abandoning...") when their respective guard flag is `true`.

## Configuration

No new configuration options were introduced. The Baby Step Mode banner visibility is driven entirely by the existing `yolo_mode` field in the server config (fetched from `GET /config`).

- **`yolo_mode: true`** → Banner hidden (YOLO mode: transitions proceed without confirmation)
- **`yolo_mode: false`** → Banner shown (Baby Step Mode: confirmation required)

## Known Issues

- The error display (red error banner for transition failures) was added during code review; it was missing from the initial implementation.
