# Task Markdown Status

## Overview

Adds a `**Status:**` metadata line to rendered `task.md` files so the markdown output matches the design specification in `design/persistence.md`. The status in `task.md` is purely informational — a human-readable label for context. The authoritative runtime status continues to live exclusively in `status.json`.

## What Was Built

An optional `status` field was added to the `Task` struct, and the markdown renderer and parser were updated to emit and extract the `**Status:**` line. Callers that create or update tasks now populate this field so the rendered markdown stays in sync with the runtime status stored in `status.json`.

## Technical Implementation

### Files Modified

| File | Change |
|------|--------|
| `src/persistence/schema.rs` | Added `pub status: Option<String>` field to `Task` struct with `#[serde(default)]` for backward-compatible deserialization |
| `src/persistence/markdown.rs` | `render_task_markdown()` emits `**Status:** {status}` when present; `parse_task_markdown()` extracts status from task.md metadata |
| `src/persistence/operations.rs` | `create_task()` sets `status = Some("backlog")` on new tasks before rendering |
| `src/api/tasks.rs` | `update_task()` reads current runtime status from `status.json` and sets it on the task before re-rendering `task.md` |
| `src/persistence/tests.rs` | Updated existing tests with status assertions; added `test_render_task_markdown_no_status` for the `None` case |

### Key Details

- **`Task.status`** — `Option<String>` with `#[serde(default)]`. Existing serialized data without a status field deserializes cleanly as `None`.
- **Rendering** — `render_task_markdown()` outputs `**Status:** {status}\n` only when `task.status` is `Some`. The line is omitted entirely when `None`, preserving backward compatibility with existing task.md files.
- **Parsing** — `parse_task_markdown()` extracts the status value from `**Status:**` metadata lines using the same emphasis-split-aware logic used for other metadata fields (`Parent plan:`, `Dependencies:`). Files without a status line parse with `status: None`.
- **Task creation** — `create_task()` clones the task, sets `status = Some("backlog")`, and renders the clone. The `status.json` file still uses `TaskStatusValue::Backlog` as the authoritative status.
- **Task updates** — `update_task()` reads the current `TaskStatus` from `status.json`, converts the `TaskStatusValue` to its kebab-case string via `Display`, and sets it on the task before re-rendering. This preserves the status line across metadata updates.
- **Roundtrip** — parse → render → parse preserves the status value, verified by `test_task_markdown_roundtrip`.

### Design Rationale

- **Struct field over function parameter:** The `Task` struct is the canonical representation of `task.md` content. Adding status as a field keeps the struct as the single source of truth for what gets rendered.
- **`Option<String>` not `Option<TaskStatusValue>`:** Since `task.md` is a human-readable format, the status is just a string label. Using `String` avoids coupling the `Task` struct to the status enum.
- **Backward compatibility:** The optional field with `#[serde(default)]` ensures existing data without a status field deserializes without errors.

## Usage

No new API endpoints or commands are introduced. The status line appears automatically in `task.md` files:

- **New tasks** — `create_task` populates `**Status:** backlog` in the rendered markdown.
- **Updated tasks** — `update_task` preserves the current status from `status.json` in the re-rendered markdown.
- **Existing tasks** — Files without a status line parse correctly with `status: None` and render without a status line.

Example rendered `task.md` output:

```markdown
# TASK-001: implement-auth

**Parent plan:** PLAN-001
**Dependencies:** TASK-000
**Status:** backlog

## Description

Build auth system.
...
```

## Configuration

No configuration changes. The feature is always active and requires no environment variables or settings.
