# Plan: 020 - Add status line to task.md rendering

## Task Description
The `render_task_markdown` function in `src/persistence/markdown.rs` does not output a `**Status:**` line in the rendered `task.md` output, despite the design template in `design/persistence.md:61` specifying that `task.md` should include `**Status:** backlog | queued | running | ...`.

The root cause is that the `Task` struct in `src/persistence/schema.rs` does not have a status field — status lives in `status.json`, not `task.md`. The renderer therefore has no status data to output.

## Objective
Add a `**Status:**` line to the rendered `task.md` output so that it matches the design specification. The status in `task.md` is purely informational (human-readable context); the authoritative status continues to live in `status.json`.

## Problem Statement
- `render_task_markdown()` omits the `**Status:**` metadata line
- The `Task` struct has no status field
- All callers of `render_task_markdown` have no status data to pass
- The design template explicitly requires this line

## Solution Approach
Add an optional `status: Option<String>` field to the `Task` struct with `#[serde(default)]` for backward compatibility. Update `render_task_markdown` to output `**Status:** {status}` when the field is present. Update callers to populate the status field when appropriate.

**Why struct field over function parameter:** The `Task` struct is the canonical representation of task.md content. Adding status as a field keeps the struct as the single source of truth for what gets rendered. A function parameter would create a split between the struct and the rendered output, making it harder to reason about what task.md contains.

**Backward compatibility:** The field is `Option<String>` with `#[serde(default)]`, so existing serialized `Task` data (in JSON or markdown) that lacks a status field will deserialize cleanly with `status: None`.

## Relevant Files

### Existing Files
- `src/persistence/schema.rs` (lines 106-131) — `Task` struct definition; add `status: Option<String>` field
- `src/persistence/markdown.rs` (lines 367-421) — `render_task_markdown()` function; add status output line
- `src/persistence/markdown.rs` (lines 192-364) — `parse_task_markdown()` function; add status parsing
- `src/persistence/operations.rs` (lines 103-133) — `create_task()` function; populate status when creating tasks
- `src/api/tasks.rs` (lines 321-389) — `update_task()` endpoint; read status and populate before re-rendering
- `src/persistence/tests.rs` (lines 147-165) — `test_render_task_markdown`; update test to include status

### New Files (if needed)
None.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: schema-builder
  - Role: Add status field to Task struct and update all related code
  - Agent: builder

- **Validator**
  - Name: schema-validator
  - Role: Verify implementation meets criteria, run tests, check backward compatibility
  - Agent: validator

- **Documenter**
  - Name: schema-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Add status field to Task struct
- **Task ID**: add-status-field
- **Depends On**: none
- **Assigned To**: schema-builder
- **Agent**: builder
- **Actions**:
  - Add `pub status: Option<String>` field to the `Task` struct in `src/persistence/schema.rs` after the `notes` field
  - Annotate with `#[serde(default)]` for backward-compatible deserialization
  - Add documentation comment: `/// Optional informational status string for human-readable context in task.md. Not authoritative — runtime status lives in status.json.`
- **Acceptance Criteria**:
  - `Task` struct compiles without errors
  - `#[serde(default)]` ensures backward-compatible deserialization of existing data lacking status
  - Field is `Option<String>` (not `Option<TaskStatusValue>`) since it's informational text, not runtime state

### 2. Update render_task_markdown to output status line
- **Task ID**: update-renderer
- **Depends On**: add-status-field
- **Assigned To**: schema-builder
- **Agent**: builder
- **Actions**:
  - In `render_task_markdown()` in `src/persistence/markdown.rs`, add a status output line after the dependencies line (around line 380)
  - Output format: `**Status:** {status}\n` when `task.status` is `Some`
  - Skip the line entirely when `task.status` is `None` (to maintain backward compatibility with existing task.md files that don't have status)
  - The status line should appear between the dependencies line and the blank line before `## Description`
- **Acceptance Criteria**:
  - When `task.status` is `Some("backlog")`, output contains `**Status:** backlog`
  - When `task.status` is `None`, output does not contain a Status line
  - Status line appears in the metadata section (after dependencies, before the blank line)

### 3. Update parse_task_markdown to extract status
- **Task ID**: update-parser
- **Depends On**: add-status-field
- **Assigned To**: schema-builder
- **Agent: builder
- **Actions**:
  - In `parse_task_markdown()` in `src/persistence/markdown.rs`, add a `mut status: Option<String> = None` variable
  - Add extraction logic in the metadata parsing section: `extract_metadata_value(&text, "Status:")`
  - Include `status` in the returned `Task` struct construction
- **Acceptance Criteria**:
  - Parsing a task.md with `**Status:** backlog` produces a `Task` with `status: Some("backlog")`
  - Parsing a task.md without a status line produces a `Task` with `status: None`
  - Roundtrip: parse → render → parse produces the same status value

### 4. Update create_task to populate status
- **Task ID**: update-create-task
- **Depends On**: update-renderer
- **Assigned To**: schema-builder
- **Agent**: builder
- **Actions**:
  - In `create_task()` in `src/persistence/operations.rs`, set `task.status` to `Some("backlog".to_string())` before calling `render_task_markdown()`
  - Since `Task` is passed by reference, create a mutable clone, set the status, and render the clone
  - Alternatively, pass status into the function and set it on the task before rendering
- **Acceptance Criteria**:
  - Newly created tasks have `**Status:** backlog` in their task.md
  - The status.json file still uses `TaskStatusValue::Backlog` as before

### 5. Update update_task endpoint to preserve status
- **Task ID**: update-api-endpoint
- **Depends On**: update-renderer
- **Assigned To**: schema-builder
- **Agent**: builder
- **Actions**:
  - In `update_task()` in `src/api/tasks.rs`, read the current task status from `status.json` before re-rendering
  - Convert the `TaskStatusValue` to a string using its `Display` implementation
  - Set `task.status = Some(status_string)` before calling `render_task_markdown()`
  - The status read happens after line 380 where `read_task_status` is already called — reuse that data
- **Acceptance Criteria**:
  - Updating a task's metadata preserves the current status in the re-rendered task.md
  - Status string matches the `Display` output of `TaskStatusValue` (e.g., "backlog", "running", "completed")

### 6. Update tests
- **Task ID**: update-tests
- **Depends On**: update-renderer, update-parser
- **Assigned To**: schema-builder
- **Agent**: builder
- **Actions**:
  - Update `test_render_task_markdown` in `src/persistence/tests.rs` to include `status: Some("backlog".into())` in the test `Task`
  - Add assertion: `assert!(md.contains("**Status:** backlog"))`
  - Add a new test case for `status: None` that asserts no Status line appears
  - Verify existing tests still pass (backward compatibility)
- **Acceptance Criteria**:
  - All existing tests pass
  - New assertions verify status line presence/absence correctly
  - Test coverage includes both `Some` and `None` status cases

### 7. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-status-field, update-renderer, update-parser, update-create-task, update-api-endpoint, update-tests
- **Assigned To**: schema-validator
- **Agent**: validator
- **Checks**:
  - `cargo check` — No compilation errors
  - `cargo test` — All tests pass
  - `cargo clippy` — No warnings
  - Verify `render_task_markdown` output matches the design template format in `design/persistence.md`
  - Verify backward compatibility: deserializing a Task without status field works
  - Verify roundtrip: parse → render → parse preserves all fields including status

### 8. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: schema-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `Task` struct has `status: Option<String>` field with `#[serde(default)]`
- `render_task_markdown()` outputs `**Status:** {status}` when status is present
- `parse_task_markdown()` extracts status from task.md files that include it
- `create_task()` populates status as "backlog" for new tasks
- `update_task()` preserves current status when re-rendering task.md
- All existing tests pass
- New tests verify status line rendering (both present and absent cases)
- Backward compatible: existing task.md files without status line parse correctly
- Roundtrip (parse → render → parse) preserves status value

## Validation Commands
- `cargo check` — Verify compilation
- `cargo test` — Run all tests
- `cargo clippy` — Check for warnings
- `cargo test test_render_task_markdown` — Specifically verify the rendering test
- `cargo test parse_task_markdown` — Verify parsing test

## Notes
- The status in `task.md` is purely informational. Runtime orchestration, status transitions, and agent leases all continue to use `status.json` exclusively.
- Using `Option<String>` (not `Option<TaskStatusValue>`) avoids coupling the Task struct to the status enum, since task.md is a human-readable format where the status is just a string label.
- The `TaskStatusValue` enum already implements `Display`, so converting runtime status to a string for the task.md output is trivial.
- Callers that don't set status (e.g., tests, ad-hoc Task construction) will produce task.md without a status line, which is acceptable for backward compatibility.
- The parsing of status from existing task.md files is idempotent — files without status will parse with `status: None`, and files with status will parse with the correct value.
