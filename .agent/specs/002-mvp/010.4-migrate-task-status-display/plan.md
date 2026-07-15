# Plan: 010.4 - Migrate Task Status Display

## Task Description
Replace all call sites of the standalone `task_status_to_string()` function with the `fmt::Display` implementation on `TaskStatusValue` (using `.to_string()`), then remove the now-redundant function.

## Objective
Eliminate the redundant `task_status_to_string` function by migrating all callers to use `TaskStatusValue::to_string()` via the `fmt::Display` trait, and then delete the function definition and all its re-exports.

## Problem Statement
The `TaskStatusValue` enum in `src/persistence/schema.rs` implements `fmt::Display` (lines 189-202), which produces the exact same kebab-case string output as the standalone `task_status_to_string` function (lines 223-234). However, the persistence and overlord modules still call `task_status_to_string(&x).to_string()` instead of `x.to_string()`. The API layer was already migrated in a previous plan, but `execution.rs` still uses the function (it uses `use crate::persistence::*` glob import). The function is actively used in 4 source files and re-exported from `overlord/mod.rs`.

## Solution Approach
1. Migrate each call site from `task_status_to_string(&x).to_string()` to `x.to_string()` (or `x.to_string()` when `x` is already a reference).
2. Remove the `task_status_to_string` import from each file that explicitly imports it.
3. Remove the re-export from `overlord/mod.rs`.
4. Delete the function definition from `schema.rs`.
5. Update the doc comment in `execution.rs` that references the function.

## Relevant Files

### Files to Modify
| File | Changes |
|------|---------|
| `src/api/execution.rs` | Replace call on line 52, update doc comment on line 43 |
| `src/persistence/schema.rs` | Delete function definition (lines 222-234) |
| `src/persistence/operations.rs` | Replace 4 calls (lines 191, 192, 253, 254) |
| `src/overlord/status_machine.rs` | Remove import (line 7), replace 4 calls (lines 130-131, 136-137) |
| `src/overlord/mod.rs` | Remove re-export (line 23) |
| `src/overlord/scheduler.rs` | Remove import (line 20), replace 2 calls (lines 246-247) |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: display-migration-builder
  - Role: Migrate all call sites and remove the function
  - Agent: builder

- **Validator**
  - Name: display-migration-validator
  - Role: Verify compilation, test suite, and no remaining references
  - Agent: validator

## Step by Step Tasks

### 1. Migrate `src/api/execution.rs`
- **Task ID**: migrate-execution-rs
- **Depends On**: none
- **Assigned To**: display-migration-builder
- **Agent**: builder
- **Actions**:
  - Replace line 52: `task_status_to_string(v).to_string()` → `v.to_string()`
  - Update doc comment on line 43: Remove reference to `[`task_status_to_string`]` and reference `.to_string()` / `fmt::Display` instead
  - Note: `execution.rs` uses `use crate::persistence::*;` glob import, so no explicit import to remove
- **Acceptance Criteria**:
  - `task_status_to_string` no longer appears in `execution.rs`
  - Code compiles

### 2. Migrate `src/persistence/operations.rs`
- **Task ID**: migrate-operations-rs
- **Depends On**: none
- **Assigned To**: display-migration-builder
- **Agent**: builder
- **Actions**:
  - Line 191: Replace `task_status_to_string(&old_status).to_string()` → `old_status.to_string()`
  - Line 192: Replace `task_status_to_string(&params.new_status).to_string()` → `params.new_status.to_string()`
  - Line 253: Replace `task_status_to_string(&old_status).to_string()` → `old_status.to_string()`
  - Line 254: Replace `task_status_to_string(&params.target_status).to_string()` → `params.target_status.to_string()`
  - Note: `operations.rs` uses `use super::schema::*;` glob import, so no explicit import to remove
- **Acceptance Criteria**:
  - `task_status_to_string` no longer appears in `operations.rs`
  - Code compiles

### 3. Migrate `src/overlord/status_machine.rs`
- **Task ID**: migrate-status-machine-rs
- **Depends On**: none
- **Assigned To**: display-migration-builder
- **Agent**: builder
- **Actions**:
  - Line 7: Remove `task_status_to_string` from the import: `use crate::persistence::{PlanStatus, TaskStatusValue};`
  - Line 130: Replace `task_status_to_string(from).to_string()` → `from.to_string()`
  - Line 131: Replace `task_status_to_string(to).to_string()` → `to.to_string()`
  - Line 136: Replace `task_status_to_string(from).to_string()` → `from.to_string()`
  - Line 137: Replace `task_status_to_string(to).to_string()` → `to.to_string()`
- **Acceptance Criteria**:
  - `task_status_to_string` no longer appears in `status_machine.rs`
  - Code compiles

### 4. Migrate `src/overlord/scheduler.rs`
- **Task ID**: migrate-scheduler-rs
- **Depends On**: none
- **Assigned To**: display-migration-builder
- **Agent**: builder
- **Actions**:
  - Line 20: Remove the import line `use crate::persistence::task_status_to_string;`
  - Line 246: Replace `task_status_to_string(&status.status).to_string()` → `status.status.to_string()`
  - Line 247: Replace `task_status_to_string(&params.new_status).to_string()` → `params.new_status.to_string()`
- **Acceptance Criteria**:
  - `task_status_to_string` no longer appears in `scheduler.rs`
  - Code compiles

### 5. Remove re-export from `src/overlord/mod.rs`
- **Task ID**: remove-overlord-reexport
- **Depends On**: migrate-status-machine-rs, migrate-scheduler-rs
- **Assigned To**: display-migration-builder
- **Agent**: builder
- **Actions**:
  - Delete line 23: `pub use crate::persistence::task_status_to_string;`
- **Acceptance Criteria**:
  - `task_status_to_string` no longer appears in `mod.rs`
  - Code compiles

### 6. Delete function from `src/persistence/schema.rs`
- **Task ID**: delete-function-schema-rs
- **Depends On**: migrate-execution-rs, migrate-operations-rs
- **Assigned To**: display-migration-builder
- **Agent**: builder
- **Actions**:
  - Delete lines 222-234 (the doc comment and function body):
    ```
    /// Convert a TaskStatusValue to its kebab-case string representation.
    pub fn task_status_to_string(status: &TaskStatusValue) -> &'static str {
        match status {
            TaskStatusValue::Backlog => "backlog",
            TaskStatusValue::Queued => "queued",
            TaskStatusValue::Running => "running",
            TaskStatusValue::Reviewing => "reviewing",
            TaskStatusValue::WaitingManualReview => "waiting-manual-review",
            TaskStatusValue::MergeQueue => "merge-queue",
            TaskStatusValue::Abandoned => "abandoned",
            TaskStatusValue::Completed => "completed",
        }
    }
    ```
  - Remove the blank line after the function if it leaves a double blank line
- **Acceptance Criteria**:
  - `task_status_to_string` no longer appears in `schema.rs`
  - Code compiles

### 7. Final Validation
- **Task ID**: validate-all
- **Depends On**: delete-function-schema-rs, remove-overlord-reexport
- **Assigned To**: display-migration-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must compile with no errors
  - Run `cargo test` — all tests must pass
  - Run `grep -r 'task_status_to_string' src/` — must return zero results
  - Verify `fmt::Display` implementation for `TaskStatusValue` still exists and is unchanged

### 8. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `task_status_to_string` function is completely removed from `src/persistence/schema.rs`
- `task_status_to_string` re-export is removed from `src/overlord/mod.rs`
- All 6 call sites across 4 files have been migrated to use `.to_string()` via `fmt::Display`
- All imports of `task_status_to_string` have been removed from `status_machine.rs` and `scheduler.rs`
- `cargo check` passes with no errors
- `cargo test` passes with no failures
- `grep -r 'task_status_to_string' src/` returns zero matches

## Validation Commands
- `cargo check` — Verify the project compiles
- `cargo test` — Run the full test suite
- `grep -r 'task_status_to_string' src/` — Confirm no remaining references (should return nothing)
- `cargo clippy` — Check for any new lint warnings

## Notes
- The `fmt::Display` implementation for `TaskStatusValue` (lines 189-202) must NOT be removed — it is the replacement mechanism.
- The `plan_status_to_string` function in `status_machine.rs` (line 176) is a separate concern and is out of scope for this plan.
- `execution.rs` uses a glob import (`use crate::persistence::*`) so no import line needs removal; only the call site and doc comment need updating.
- `operations.rs` also uses a glob import (`use super::schema::*`) so no import line needs removal.
- The `.to_string()` call on a `&TaskStatusValue` reference works because `Display` is implemented for the type and `to_string()` is available through the `ToString` auto-trait derived from `Display`.
