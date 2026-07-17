# Task Status Typed

## Overview

The `Task.status` field in the persistence layer was changed from `Option<String>` to `Option<TaskStatusValue>`. This ensures the markdown-level task representation (`task.md`) uses the same typed enum as the runtime `status.json` representation, eliminating a dual source of truth and enforcing type safety across the persistence layer.

## What Was Built

The `TaskStatusValue` enum already existed in `schema.rs` with `Serialize`, `Deserialize`, `Display`, and `FromStr` implementations — it was used by `TaskStatus` (the `status.json` representation) but not by `Task` (the `task.md` representation). Previously, callers had to manually convert between `TaskStatusValue` and `String` (e.g., `TaskStatusValue::Backlog.to_string()`), and tests used hardcoded strings like `"backlog".to_string()`.

The change unifies both representations under the same typed enum, removing all intermediate string conversions in the persistence and API layers.

## Technical Implementation

### Files Modified

| File | Change |
|------|--------|
| `src/persistence/schema.rs` | `Task.status` field type changed from `Option<String>` to `Option<TaskStatusValue>` (line 135). Documentation comment updated to reflect typed nature. |
| `src/persistence/markdown.rs` | Added `TaskStatusValue` to the `use super::schema` import (line 16). Updated `parse_task_markdown()` to parse status via `TaskStatusValue::from_str(&value).ok()` with graceful fallback to `None` for unknown strings (lines 286, 301). |
| `src/persistence/operations.rs` | `create_task()` now assigns `Some(TaskStatusValue::Backlog)` directly instead of `Some(TaskStatusValue::Backlog.to_string())` (line 116). |
| `src/api/tasks.rs` | `update_task()` now assigns `task.status = Some(status.status.clone())` directly (enum, no string conversion) instead of `Some(status.status.to_string())` (line 381). |
| `src/persistence/tests.rs` | Test fixtures updated to use enum variants (`TaskStatusValue::Backlog`, `TaskStatusValue::Running`) instead of string literals. New test `test_parse_task_markdown_unknown_status` added to verify graceful fallback for unrecognized status strings. |

### Key Changes

- **Schema**: `Task.status: Option<TaskStatusValue>` with `#[serde(default)]` — backward-compatible deserialization still works.
- **Parsing**: `parse_task_markdown()` uses `TaskStatusValue::from_str(&value).ok()` — recognized kebab-case strings (e.g., `"backlog"`, `"running"`) are parsed into enum variants; unrecognized strings fall back to `None`.
- **Rendering**: `render_task_markdown()` uses `format!("**Status:** {}\n", status)` — works as-is because `TaskStatusValue` implements `fmt::Display` with kebab-case output.
- **Operations**: `create_task()` and `update_task()` assign enum variants directly without `.to_string()` conversions.

### Backward Compatibility

Existing `task.md` files with string status values parse correctly:
- **Known statuses** (e.g., `"backlog"`, `"running"`) → parsed into the corresponding `TaskStatusValue` variant.
- **Unknown statuses** → gracefully fall back to `None`, matching the current behavior for missing status lines. No parse errors are raised.

### Validation Results

- `cargo check` — No compilation errors
- `cargo test` — All tests pass (including new `test_parse_task_markdown_unknown_status`)
- `cargo clippy` — No warnings
- No remaining `.to_string()` calls on `TaskStatusValue` in the persistence layer
- No remaining `Option<String>` status assignments in task-related code
- Roundtrip (parse → render → parse) preserves status enum variant

## Usage

No external API changes. The feature is transparent to callers:

- **Creating tasks**: New tasks are created with `status: Some(TaskStatusValue::Backlog)` automatically by `create_task()`.
- **Updating tasks**: The `update_task` endpoint preserves the current status enum variant in the re-rendered `task.md`.
- **Parsing existing files**: Files with unrecognized status strings parse without error (status becomes `None`).

## Configuration

No new configuration options or environment variables introduced.
