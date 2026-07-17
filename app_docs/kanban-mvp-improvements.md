# Kanban Board MVP Improvements

## Overview

Four features added to the Kanban board (TaskList page) to complete the MVP feature set:

1. **PATCH /api/v1/config endpoint** — Backend API for toggling `yolo_mode`
2. **Yolo mode toggle UI** — Toggle button in the TaskList topbar
3. **Blocked dependency indicators** — Visual indicators on task cards with unmet dependencies
4. **Inline review action buttons** — Approve/Request Changes buttons in the Manual Review column

---

## Feature 1: PATCH /api/v1/config Endpoint

### Description

A new PATCH endpoint allows clients to update server configuration settings. Currently, only `yolo_mode` is supported as a patchable field. The endpoint reads the current config, serializes it to TOML, writes it to disk, then updates the in-memory state — ensuring persistence before mutation.

### API Endpoint

```
PATCH /api/v1/config
Content-Type: application/json

{ "yolo_mode": true }
```

**Response** (200 OK):

```json
{
  "server_host": "127.0.0.1",
  "server_port": 3000,
  "max_parallel": 4,
  "default_timeout_seconds": 300,
  "log_level": "info",
  "yolo_mode": true,
  "auth_enabled": false,
  "auth_require_read": false,
  "auth_keys_count": 0
}
```

Partial updates are supported — only the fields present in the request body are modified.

### Technical Implementation

- **Handler**: `patch_config` in `src/api/config.rs`
  1. Resolves the config file path via `config::loader::config_path()`
  2. Reads the current in-memory config under a read lock
  3. Serializes to TOML (no lock needed)
  4. Writes to disk with `tokio::fs::write`
  5. Only after disk write succeeds, acquires a write lock and updates `cfg.preferences.yolo_mode`
  6. Returns the updated `ConfigResponse`
- **Types**: `PatchConfigRequest` struct in `src/api/types.rs` with `yolo_mode: Option<bool>`
- **Route registration**: Added `.patch(config::patch_config)` to the `/api/v1/config` route in `src/api/mod.rs`
- **Tests**: Three tests in `src/api/tests.rs`:
  - `test_patch_config_yolo_mode_on` — enables yolo mode and verifies persistence
  - `test_patch_config_yolo_mode_off` — disables yolo mode
  - `test_patch_config_partial_update` — verifies other fields remain unchanged

### Files Changed

| File | Change |
|------|--------|
| `src/api/config.rs` | Added `patch_config` handler |
| `src/api/types.rs` | Added `PatchConfigRequest` struct |
| `src/api/mod.rs` | Registered PATCH route |
| `src/api/tests.rs` | Added 3 PATCH config tests |

---

## Feature 2: Yolo Mode Toggle UI

### Description

A toggle button in the TaskList page topbar lets users enable/disable Yolo mode without editing config files. The button shows the current state fetched from the server on page load.

### UI Behavior

- **Location**: Topbar, between the view mode toggle and the "Add Task" button
- **Label**: `⚡ Yolo: ON` (when enabled) or `⚡ Yolo: OFF` (when disabled)
- **Styling when ON**: Yellow/amber background (`bg-accent-yellow-subtle`), yellow text, yellow border, hover deepens background
- **Styling when OFF**: Muted text (`text-text-muted`), default border, hover brightens text and border
- **Loading state**: Button is disabled during the API call (`disabled:opacity-50`)
- **Interaction**: Clicking calls `PATCH /api/v1/config` with the inverted `yolo_mode` value, then updates local state from the response

### Technical Implementation

- **State**: `yoloMode` and `yoloLoading` reactive state variables in `TaskList.svelte`
- **Mount**: `getConfig()` called in `onMount` alongside plan and tasks fetching
- **Toggle**: `toggleYolo()` function calls `patchConfig({ yolo_mode: !yoloMode })` and updates state from the response
- **API layer**: `patchConfig(req: PatchConfigRequest): Promise<Config>` added to `web/src/lib/api.ts`
- **Types**: `PatchConfigRequest` interface added to `web/src/lib/types.ts`

### Files Changed

| File | Change |
|------|--------|
| `web/src/pages/TaskList.svelte` | Added Yolo toggle button, state, and toggle handler |
| `web/src/lib/api.ts` | Added `patchConfig` function |
| `web/src/lib/types.ts` | Added `PatchConfigRequest` interface |

---

## Feature 3: Blocked Dependency Indicators

### Description

Task cards that have unmet dependencies (dependencies not yet `completed` or `abandoned`) display a visual indicator showing which tasks are blocking them.

### UI Behavior

- **Left border**: 3px yellow left border (`border-l-[3px] border-l-accent-yellow`) on the card container
- **Text indicator**: `⌛ Waiting on: [dependency task names]` rendered below the card content in small muted text (`text-[10px]`)
- **Only shown** when the task has at least one unmet dependency
- **Dependency resolution**: Uses a `Map` lookup (`tasksMap`) for O(1) dependency ID-to-task resolution

### Technical Implementation

- **Data flow**: `TaskList.svelte` creates a `tasksMap` (`Map<string, Task>`) derived from the tasks array, passes it through `TaskColumn` to each `TaskCard`
- **Blocked detection** (in `TaskCard.svelte`):
  ```ts
  const unmetDependencies = $derived(
    task.dependencies
      .map(depId => tasksMap.get(depId))
      .filter((t): t is Task => t !== undefined && !['completed', 'abandoned'].includes(t.status.status))
      .map((t) => t.name)
  );
  const isBlocked = $derived(unmetDependencies.length > 0);
  ```
- **Rendering**: Conditional `border-l-[3px] border-l-accent-yellow` class on the card div, conditional `⌛ Waiting on:` text block

### Files Changed

| File | Change |
|------|--------|
| `web/src/pages/TaskList.svelte` | Added `tasksMap` derived state, passes it to `TaskColumn` |
| `web/src/components/TaskColumn.svelte` | Accepts `tasksMap` prop, passes it to `TaskCard` |
| `web/src/components/TaskCard.svelte` | Added `tasksMap` prop, blocked detection logic, and indicator rendering |

---

## Feature 4: Inline Review Action Buttons

### Description

Task cards in the Manual Review column (`waiting-manual-review` status) display inline Approve and Request Changes buttons, allowing users to transition tasks without navigating to the task detail page.

### UI Behavior

- **Approve button**: Green styling (`bg-accent-green`, `border-accent-green`, white text), labeled `✓ Approve`. Transitions the task to `merge-queue` status.
- **Request Changes button**: Yellow styling (`bg-accent-yellow-subtle`, `border-accent-yellow`, yellow text), labeled `↩ Request Changes`. Transitions the task back to `reviewing` status.
- **Loading state**: Both buttons show `...` and become disabled during the API call
- **Event handling**: `e.stopPropagation()` prevents the card link from navigating away
- **Post-transition**: Triggers a full task list re-fetch via the `onTaskTransition` callback, causing the card to re-render in the new column
- **Only shown** in the `waiting-manual-review` column

### Technical Implementation

- **Props**: `TaskCard` receives `columnStatus` and `onTaskTransition` props from `TaskColumn`
- **Callbacks**: `TaskList.svelte` defines `handleTaskTransition` which re-fetches tasks via `listTasks()` and updates the local `tasks` state
- **API calls**: Both buttons call `transitionTaskStatus(branch, planId, task.id, { status: '...' })`
- **Conditional rendering**: `{#if columnStatus === 'waiting-manual-review'}` wraps the button group

### Files Changed

| File | Change |
|------|--------|
| `web/src/pages/TaskList.svelte` | Added `onTaskTransition` callback, passes it to `TaskColumn` |
| `web/src/components/TaskColumn.svelte` | Accepts `onTaskTransition` prop, passes `columnStatus` and callback to `TaskCard` |
| `web/src/components/TaskCard.svelte` | Added `columnStatus` and `onTaskTransition` props, review buttons with handlers |

---

## Configuration

No new configuration options are required. The `yolo_mode` setting is persisted in the server's TOML config file and can be toggled via the UI or the PATCH API endpoint.

## Validation

Run these commands to verify the features:

```bash
# Backend tests
cargo test

# Frontend type checking
cd web && npm run check

# Frontend production build
cd web && npm run build

# API endpoint test
curl -X PATCH http://localhost:PORT/api/v1/config \
  -H 'Content-Type: application/json' \
  -d '{"yolo_mode": true}'
```
