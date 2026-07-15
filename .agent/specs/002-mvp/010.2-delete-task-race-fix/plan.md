# Plan: 010.2 - TOCTOU Race Fix: Per-Plan Locking for delete_task Execution State

## Task Description
Wrap the `delete_task` handler in `src/api/tasks.rs` with the per-plan locking infrastructure established by spec 010.1, ensuring that both plan file modifications and execution state modifications are serialized atomically under a single lock acquisition.

## Objective
Eliminate the TOCTOU race condition in `delete_task` where three separate read-modify-write cycles (task directory removal, plan task list update, execution state update) can interleave with concurrent handlers, corrupting the plan's task list or the execution state's `task_status_map`.

## Problem Statement

In `src/api/tasks.rs`, the `delete_task` handler (lines 392-429) performs **three separate read-modify-write cycles** with no locking:

```rust
// Line 404: Remove task directory (filesystem mutation)
crate::persistence::remove_dir_all(&task_dir)?;

// Lines 408-412: Read plan, modify task list, write back
let mut plan = read_plan(...)?;
plan.tasks.retain(|t| t.id != resolved_task_id);
update_plan(...)?;

// Lines 415-426: Read execution state, modify task_status_map, write back
let mut exec_state = read_execution_state(...)?;
exec_state.tasks.retain(|t| *t != resolved_task_id);
exec_state.task_status_map.remove(&resolved_task_id);
update_execution_state(...)?;
```

Between any of these cycles, a concurrent handler can:

1. **Concurrent `delete_task` on same plan**: Reads the plan before the first handler writes back, then writes back its own modifications, overwriting the first handler's changes. The plan's task list loses one handler's deletion, or the execution state retains a task that was deleted from the plan.

2. **Concurrent `create_task` on same plan**: Reads the plan, adds a task reference, writes back — overwriting the first handler's deletion. The deleted task reappears in the plan's task list.

3. **Concurrent `transition_task_status` on same plan**: Reads execution state, updates `task_status_map`, writes back — overwriting the first handler's deletion from `task_status_map`. The deleted task retains a stale status entry.

**The critical gap**: while `add_task_to_execution` in the persistence layer has file-content-based TOCTOU protection, the `delete_task` handler uses plain `read_execution_state` + `update_execution_state` with **no** concurrency protection. This means the execution state race is completely unprotected at the persistence layer.

## Solution Approach

Reuse the per-plan locking infrastructure from spec 010.1. The `delete_task` handler acquires the per-plan lock **once** at the beginning of its mutation section, and holds it through all three read-modify-write cycles (task directory removal, plan update, execution state update). This serializes all mutations on a given plan, preventing interleaving.

This spec focuses specifically on the `delete_task` handler and its interaction with execution state. It depends on the infrastructure established by 010.1 (`AppState.plan_locks`, `lock_plan()` helper in middleware).

### Design Details

1. **Lock acquisition point**: After `resolve_plan_path` and `resolve_task_path` (which are read-only filesystem operations), before `remove_dir_all`.

2. **Lock scope**: The guard covers:
   - `remove_dir_all` (task directory removal)
   - `read_plan` → `plan.tasks.retain` → `update_plan` (plan file modification)
   - `read_execution_state` → `exec_state.tasks.retain` + `task_status_map.remove` → `update_execution_state` (execution state modification)

3. **No changes needed elsewhere**: The lock infrastructure, AppState field, and middleware helpers are all provided by 010.1. This spec only modifies `delete_task` in `tasks.rs`.

4. **Consistency guarantee**: Under the lock, a deleted task is removed from the plan's task list AND the execution state's `task_status_map` atomically. No handler can observe an intermediate state where the task is deleted from one but not the other.

## Relevant Files

### Files to Modify
| File | Changes |
|------|---------|
| `src/api/tasks.rs` | Wrap `delete_task` handler (lines 392-429) with per-plan lock acquisition |

### Files for Reference (No Changes — provided by 010.1)
| File | Why |
|------|-----|
| `src/api/types.rs` | Contains `AppState` with `plan_locks` field (added by 010.1) |
| `src/api/middleware.rs` | Contains `lock_plan()` helper (added by 010.1) |
| `src/persistence/operations.rs` | Contains `read_plan`, `update_plan`, `read_execution_state`, `update_execution_state` |
| `src/persistence/schema.rs` | Contains `Plan`, `TaskReference`, `ExecutionState` types |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: `delete-task-lock-builder`
  - Role: Wrap `delete_task` handler with per-plan lock
  - Agent: builder

- **Validator**
  - Name: `delete-task-lock-validator`
  - Role: Verify lock acquisition, scope, and correctness
  - Agent: validator

- **Documenter**
  - Name: `delete-task-lock-documenter`
  - Role: Generate documentation for the delete_task race fix
  - Agent: documenter

## Step by Step Tasks

### 1. Wrap delete_task with per-plan lock
- **Task ID**: wrap-delete-task-lock
- **Depends On**: none (depends on 010.1 infrastructure being in place)
- **Assigned To**: delete-task-lock-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/tasks.rs`, `delete_task` handler (lines 392-429):
    - Add import: `use crate::api::middleware::lock_plan;` (if not already imported)
    - After `resolve_task_path` (line 401) and before `remove_dir_all` (line 404), insert:
      ```rust
      // Acquire per-plan lock to serialize plan and execution state mutations
      let _plan_guard = lock_plan(&state, &plan_id).await;
      ```
    - The `_plan_guard` keeps the lock held through:
      - `remove_dir_all` (line 404)
      - `read_plan` → `plan.tasks.retain` → `update_plan` (lines 408-412)
      - `read_execution_state` → modifications → `update_execution_state` (lines 415-426)
    - The guard is dropped automatically at end of function scope
  - Add a doc comment above the lock acquisition explaining the purpose:
    ```rust
    // Acquire per-plan lock to serialize all mutations on this plan.
    // This prevents TOCTOU races between concurrent delete_task, create_task,
    // and other handlers that read-modify-write the plan file or execution state.
    // See spec 010.1 for lock infrastructure details.
    let _plan_guard = lock_plan(&state, &plan_id).await;
    ```
- **Acceptance Criteria**:
  - `delete_task` acquires the per-plan lock after path resolution and before any mutations
  - The lock guard is held through all three mutation phases (directory removal, plan update, execution state update)
  - The lock guard is dropped at end of function scope (implicit via Rust drop semantics)
  - `cargo check` passes

### 2. Integration verification and compilation
- **Task ID**: verify-compilation
- **Depends On**: wrap-delete-task-lock
- **Assigned To**: delete-task-lock-builder
- **Agent**: builder
- **Actions**:
  - Run `cargo check` to verify compilation
  - Run `cargo clippy` to catch any linting issues
  - Run `cargo fmt` to ensure consistent formatting
  - Fix any compilation, clippy, or formatting issues
- **Acceptance Criteria**:
  - `cargo check` passes with no errors
  - `cargo clippy` passes with no warnings
  - `cargo fmt --check` passes with no formatting issues

### 3. Final Validation
- **Task ID**: validate-all
- **Depends On**: verify-compilation
- **Assigned To**: delete-task-lock-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all existing tests pass
  - Run `cargo clippy -- -D warnings` — no warnings
  - Verify `delete_task` acquires per-plan lock via `lock_plan(&state, &plan_id).await`
  - Verify lock acquisition occurs AFTER `resolve_plan_path` and `resolve_task_path` (read-only operations)
  - Verify lock acquisition occurs BEFORE `remove_dir_all`, `read_plan`, and `read_execution_state` (all mutations)
  - Verify the lock guard variable is named `_plan_guard` (or similar, with underscore prefix to suppress unused variable warning)
  - Verify the lock scope covers all three mutation phases (directory removal, plan update, execution state update)

### 4. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: delete-task-lock-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/` describing the delete_task race condition fix
  - Document how the per-plan lock prevents interleaving of plan and execution state mutations

## Acceptance Criteria

1. **Compilation**: `cargo check` passes with zero errors
2. **Linting**: `cargo clippy -- -D warnings` passes with zero warnings
3. **Formatting**: `cargo fmt --check` passes with zero formatting issues
4. **Tests**: `cargo test` passes — all existing tests continue to pass
5. **Lock acquisition**: `delete_task` calls `lock_plan(&state, &plan_id).await` after path resolution
6. **Lock scope**: The lock guard covers all three mutation phases:
   - Task directory removal (`remove_dir_all`)
   - Plan file modification (`read_plan` → `retain` → `update_plan`)
   - Execution state modification (`read_execution_state` → `retain` + `remove` → `update_execution_state`)
7. **No new dependencies**: Solution uses only the lock infrastructure from 010.1
8. **No new files**: Only `src/api/tasks.rs` is modified

## Validation Commands

- `cargo check` — Verify compilation
- `cargo clippy -- -D warnings` — Verify no clippy warnings
- `cargo fmt --check` — Verify formatting
- `cargo test` — Run all tests
- `cargo test --lib api` — Run API module tests specifically

## Notes

- **Dependency on 010.1**: This spec assumes 010.1 has been implemented. The `AppState.plan_locks` field, `lock_plan()` middleware helper, and `AppState` initialization must all be in place. If 010.1 has not been executed, this spec must wait.

- **Lock granularity**: Per-plan locking is the correct granularity because the race condition involves concurrent access to the plan's `tasks` list and the plan's execution state. Both are scoped to a single plan.

- **Lock ordering**: The lock is acquired after `resolve_plan_path` and `resolve_task_path`, which only perform read-only filesystem operations (checking directory existence, reading filenames). This minimizes lock hold time for path resolution.

- **Performance impact**: Minimal. The per-plan mutex is only held during file I/O (three read-modify-write cycles). Concurrent operations on *different* plans are not blocked.

- **Deadlock risk**: None. The handler acquires exactly one per-plan lock, held for a bounded duration (the scope of the handler function). No nested locks are acquired.

- **Consistency guarantee**: Under the lock, the task is removed from the plan's task list AND the execution state's `task_status_map` atomically. No concurrent handler can observe an inconsistent intermediate state.

- **Other handlers**: `transition_task_status` and `claim_task` also modify execution state without per-plan locking. These are outside the scope of this spec but should be addressed in a follow-up spec if needed. The `create_task` handler's execution state update (`add_task_to_execution`) already has file-content-based TOCTOU protection at the persistence layer.

- **Why a separate spec from 010.1**: While 010.1 covers the plan file locking for `create_task` and `delete_task`, this spec focuses specifically on the execution state aspect of `delete_task` — the fact that `read_execution_state` + `update_execution_state` has no concurrency protection at the persistence layer (unlike `add_task_to_execution`). This spec ensures the lock scope explicitly covers both plan and execution state mutations in `delete_task`.
