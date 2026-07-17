# Plan: 020.1 - Type Task.status as TaskStatusValue instead of String

## Task Description
The `Task.status` field in `src/persistence/schema.rs` is typed as `Option<String>` when it should be `Option<TaskStatusValue>`. The `TaskStatusValue` enum already exists in the same file with `Serialize`, `Deserialize`, `Display`, and `FromStr` implementations. Using `String` creates a dual source of truth where the markdown status string can drift from the typed enum, allowing invalid or misspelled status values to slip through.

## Objective
Change `Task.status` from `Option<String>` to `Option<TaskStatusValue>` so that the markdown-level task representation uses the same typed enum as the runtime `status.json` representation. This eliminates the dual source of truth and ensures type safety across the persistence layer.

## Problem Statement
- `Task.status` is `Option<String>` — allows any arbitrary string, including invalid status values
- `TaskStatusValue` enum already exists with proper `Display`/`FromStr` implementations
- Callers manually convert between `TaskStatusValue` and `String` (e.g., `TaskStatusValue::Backlog.to_string()`)
- Tests use hardcoded strings like `"backlog".to_string()` instead of typed enum variants
- The `update_task` endpoint converts enum → string → enum via string intermediaries

## Solution Approach
Change `Task.status` from `Option<String>` to `Option<TaskStatusValue>`. Update all callers to work with the enum directly. For markdown parsing, use `TaskStatusValue::from_str()` with a graceful fallback to `None` on parse error for backward compatibility with existing task.md files that may contain unrecognized status strings.

**Backward compatibility:** Existing task.md files with string status values will still parse correctly because `from_str()` handles all known kebab-case status strings. Unknown strings will fall back to `None` (matching current behavior for missing status lines) rather than causing parse errors.

## Relevant Files

### Existing Files
- `src/persistence/schema.rs` (line 134) — `Task.status` field; change type from `Option<String>` to `Option<TaskStatusValue>`
- `src/persistence/markdown.rs` (line 15) — imports; add `TaskStatusValue` to the `use super::schema` import
- `src/persistence/markdown.rs` (lines 243, 285, 300) — `parse_task_markdown()`; change status variable type and parsing logic
- `src/persistence/markdown.rs` (lines 473-475) — `render_task_markdown()`; use `.to_string()` on enum variant
- `src/persistence/operations.rs` (line 116) — `create_task()`; set enum directly instead of `.to_string()`
- `src/api/tasks.rs` (line 381) — `update_task()`; assign enum directly instead of `.to_string()`
- `src/persistence/tests.rs` (lines 210, 240, 257) — test fixtures; update status values to use enum variants

### New Files (if needed)
None.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: typed-status-builder
  - Role: Change Task.status type and update all callers
  - Agent: builder

- **Validator**
  - Name: typed-status-validator
  - Role: Verify implementation, run tests, check backward compatibility
  - Agent: validator

- **Documenter**
  - Name: typed-status-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Change Task.status type in schema.rs
- **Task ID**: change-status-type
- **Depends On**: none
- **Assigned To**: typed-status-builder
- **Agent**: builder
- **Actions**:
  - In `src/persistence/schema.rs`, line 134, change `pub status: Option<String>` to `pub status: Option<TaskStatusValue>`
  - Keep the `#[serde(default)]` attribute unchanged (it works for both types)
  - Update the documentation comment to reflect the typed nature: `/// Optional informational status for human-readable context in task.md. Typed as TaskStatusValue for consistency with status.json. Not authoritative — runtime status lives in status.json.`
- **Acceptance Criteria**:
  - `Task` struct compiles without errors
  - `#[serde(default)]` still ensures backward-compatible deserialization
  - Field is `Option<TaskStatusValue>` (not `Option<String>`)

### 2. Update imports in markdown.rs
- **Task ID**: update-imports
- **Depends On**: change-status-type
- **Assigned To**: typed-status-builder
- **Agent**: builder
- **Actions**:
  - In `src/persistence/markdown.rs`, line 15, add `TaskStatusValue` to the existing import: `use super::schema::{Plan, PlanStatus, Task, TaskReference, TaskStatusValue};`
- **Acceptance Criteria**:
  - `TaskStatusValue` is available in the markdown module scope
  - No unused import warnings

### 3. Update parse_task_markdown to use TaskStatusValue
- **Task ID**: update-parser
- **Depends On**: update-imports
- **Assigned To**: typed-status-builder
- **Agent**: builder
- **Actions**:
  - Change `let mut status: Option<String> = None;` (line 243) to `let mut status: Option<TaskStatusValue> = None;`
  - In the pending metadata label handler (around line 285), change `status = Some(value)` to:
    ```rust
    status = TaskStatusValue::from_str(&value).ok();
    ```
  - In the direct metadata extraction (around line 300), change `status = Some(value.to_string())` to:
    ```rust
    status = TaskStatusValue::from_str(value).ok();
    ```
  - The `.ok()` call converts `Result<TaskStatusValue, String>` to `Option<TaskStatusValue>`, gracefully falling back to `None` for unrecognized status strings
- **Acceptance Criteria**:
  - Parsing a task.md with `**Status:** backlog` produces `Task { status: Some(TaskStatusValue::Backlog) }`
  - Parsing a task.md with an unknown status (e.g., `**Status:** unknown`) produces `Task { status: None }` (graceful fallback)
  - Parsing a task.md without a status line produces `Task { status: None }`
  - Roundtrip: parse → render → parse preserves the status enum variant

### 4. Update render_task_markdown to use enum Display
- **Task ID**: update-renderer
- **Depends On**: change-status-type
- **Assigned To**: typed-status-builder
- **Agent**: builder
- **Actions**:
  - In `render_task_markdown()` (lines 473-475), the existing code already uses `{status}` which calls `Display`. Since `TaskStatusValue` implements `Display`, the format string `**Status:** {}\n` will work correctly with the enum variant.
  - The current code:
    ```rust
    if let Some(ref status) = task.status {
        out.push_str(&format!("**Status:** {}\n", status));
    }
    ```
  - This code works as-is because `TaskStatusValue` implements `fmt::Display`. No changes needed to the render logic, but verify the compile succeeds.
- **Acceptance Criteria**:
  - When `task.status` is `Some(TaskStatusValue::Backlog)`, output contains `**Status:** backlog`
  - When `task.status` is `Some(TaskStatusValue::Running)`, output contains `**Status:** running`
  - When `task.status` is `None`, output does not contain a Status line
  - Status strings match the kebab-case Display output of `TaskStatusValue`

### 5. Update create_task to use enum directly
- **Task ID**: update-create-task
- **Depends On**: change-status-type
- **Assigned To**: typed-status-builder
- **Agent**: builder
- **Actions**:
  - In `create_task()` in `src/persistence/operations.rs` (line 116), change:
    ```rust
    task_with_status.status = Some(TaskStatusValue::Backlog.to_string());
    ```
    to:
    ```rust
    task_with_status.status = Some(TaskStatusValue::Backlog);
    ```
- **Acceptance Criteria**:
  - Newly created tasks have `**Status:** backlog` in their task.md (via Display)
  - The status.json file still uses `TaskStatusValue::Backlog` as before
  - No `.to_string()` conversion needed

### 6. Update update_task endpoint to use enum directly
- **Task ID**: update-api-endpoint
- **Depends On**: change-status-type
- **Assigned To**: typed-status-builder
- **Agent**: builder
- **Actions**:
  - In `update_task()` in `src/api/tasks.rs` (line 381), change:
    ```rust
    task.status = Some(status.status.to_string());
    ```
    to:
    ```rust
    task.status = Some(status.status);
    ```
  - `status.status` is already a `TaskStatusValue`, so no conversion needed
- **Acceptance Criteria**:
  - Updating a task's metadata preserves the current status enum variant in the re-rendered task.md
  - No string conversion intermediary

### 7. Update test fixtures
- **Task ID**: update-tests
- **Depends On**: update-parser, update-renderer
- **Assigned To**: typed-status-builder
- **Agent**: builder
- **Actions**:
  - In `test_render_task_markdown` (line 210), change `status: Some("backlog".to_string())` to `status: Some(TaskStatusValue::Backlog)`
  - In `test_task_markdown_roundtrip` (line 240), change `status: Some("running".to_string())` to `status: Some(TaskStatusValue::Running)`
  - The assertion on line 257 (`assert_eq!(parsed.status, task.status)`) works as-is since both sides are now `Option<TaskStatusValue>`
  - Add a new test `test_parse_task_markdown_unknown_status` that verifies graceful fallback:
    - Create a task.md with `**Status:** unknown-status`
    - Parse it and assert `status` is `None`
- **Acceptance Criteria**:
  - All existing tests pass
  - Status assertions use enum variants, not strings
  - New test verifies backward compatibility with unknown status values
  - Roundtrip test still passes (parse → render → parse preserves enum variant)

### 8. Final Validation
- **Task ID**: validate-all
- **Depends On**: change-status-type, update-imports, update-parser, update-renderer, update-create-task, update-api-endpoint, update-tests
- **Assigned To**: typed-status-validator
- **Agent**: validator
- **Checks**:
  - `cargo check` — No compilation errors
  - `cargo test` — All tests pass
  - `cargo clippy` — No warnings
  - Verify no remaining `.to_string()` calls on `TaskStatusValue` in the persistence layer
  - Verify no remaining `Option<String>` status assignments in task-related code
  - Verify backward compatibility: deserializing a Task without status field works
  - Verify roundtrip: parse → render → parse preserves status enum variant
  - Grep for any remaining `status: Some("...")` patterns that should use enum variants

### 9. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: typed-status-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `Task.status` is `Option<TaskStatusValue>` with `#[serde(default)]`
- `render_task_markdown()` outputs `**Status:** {kebab-case-status}` when status is present (via Display)
- `parse_task_markdown()` extracts status as `TaskStatusValue` from task.md files
- `parse_task_markdown()` gracefully falls back to `None` for unrecognized status strings
- `create_task()` sets `status: Some(TaskStatusValue::Backlog)` directly (no `.to_string()`)
- `update_task()` sets `task.status = Some(status.status)` directly (enum, no string conversion)
- All existing tests pass
- Tests use `TaskStatusValue` enum variants instead of string literals
- Backward compatible: existing task.md files parse correctly (known statuses → enum, unknown → None)
- Roundtrip (parse → render → parse) preserves status enum variant
- No remaining `.to_string()` calls on `TaskStatusValue` in persistence or API task handlers

## Validation Commands
- `cargo check` — Verify compilation
- `cargo test` — Run all tests
- `cargo clippy` — Check for warnings
- `cargo test test_render_task_markdown` — Specifically verify the rendering test
- `cargo test test_task_markdown_roundtrip` — Verify roundtrip test
- `rg 'TaskStatusValue.*to_string\(\)' src/` — Verify no remaining unnecessary string conversions
- `rg 'status: Some\("' src/` — Verify no remaining string status assignments

## Notes
- The status in `task.md` remains purely informational. Runtime orchestration, status transitions, and agent leases continue to use `status.json` exclusively.
- Using `Option<TaskStatusValue>` (not `Option<String>`) ensures type safety and eliminates the dual source of truth between `task.md` and `status.json`.
- The `TaskStatusValue` enum already implements `Display`, so `render_task_markdown` requires no logic changes — the `format!("**Status:** {}\n", status)` call works with the enum directly.
- The `TaskStatusValue` enum already implements `FromStr`, so `parse_task_markdown` uses `.from_str().ok()` for parsing with graceful fallback.
- Backward compatibility is critical: existing task.md files with unrecognized status strings must not cause parse failures. The `.ok()` fallback handles this.
- The web frontend (`web/`) uses `task.status.status` which refers to the `TaskStatusResponse.status: String` field in the API response type — this is unaffected by the persistence-layer change since the API serialization layer handles the conversion.
