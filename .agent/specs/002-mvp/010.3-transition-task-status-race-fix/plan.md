# Plan: 010.3 - TOCTOU Race Fix: Per-Plan Locking for transition_task_status and claim_task

## Task Description
Fix a Time-of-Check-to-Time-of-Use (TOCTOU) race condition in the `transition_task_status` and `claim_task` handlers where execution state updates are not atomic with status file updates. After `update_task_status` atomically writes `status.json`, the handler reads the execution state, inserts the new status into `task_status_map`, and writes it back. A concurrent transition on the same plan can cause the `task_status_map` to lag behind the individual `status.json` file.

## Objective
Wrap the `transition_task_status` and `claim_task` handlers with the per-plan locking infrastructure established by spec 010.1, ensuring that `status.json` updates and `task_status_map` updates are serialized atomically under a single lock acquisition.

## Problem Statement

In `src/api/tasks.rs`, the `transition_task_status` handler (lines 442-517) performs **two separate persistence updates** with no coordination between them:

```rust
// Line 487-488: Atomically update status.json (has its own TOCTOU protection)
let updated_status = crate::persistence::update_task_status(&params)?;

// Lines 491-503: Read execution state, modify task_status_map, write back (NO protection)
let mut exec_state = read_execution_state(&state.repo_root, &branch, &plan_id, &plan_name)?;
exec_state.task_status_map.insert(resolved_task_id.clone(), new_status);
crate::persistence::update_execution_state(&state.repo_root, &branch, &plan_id, &plan_name, &exec_state)?;
```

Between `update_task_status` and `update_execution_state`, a concurrent handler can:

1. **Concurrent `transition_task_status` on a different task in the same plan**: Reads the execution state before the first handler writes back, makes its own `task_status_map` modification, writes back — overwriting the first handler's `task_status_map` update. The `task_status_map` entry for the first task is lost, even though its `status.json` was correctly updated.

2. **Concurrent `delete_task` on the same plan**: Reads the execution state, removes the task from `task_status_map`, writes back — overwriting the first handler's status update. The task's new status is lost from `task_status_map`, even though `status.json` has the correct value.

3. **Concurrent `claim_task` on a different task**: Same interleaving issue — the `task_status_map` update from one handler overwrites the other.

**The critical gap**: `update_task_status` has file-content-based TOCTOU protection for `status.json`, but the subsequent `read_execution_state` + `update_execution_state` pair has **no** concurrency protection at the persistence layer. The `update_execution_state` function performs a plain atomic write without verifying the file hasn't changed since it was read.

The same race exists in `claim_task` (lines 531-636):

```rust
// Line 584-592: Atomically update status.json
let updated_status = crate::persistence::update_task_status(&params)?;

// Lines 619-631: Read execution state, modify task_status_map, write back (NO protection)
let mut exec_state = read_execution_state(&state.repo_root, &branch, &plan_id, &plan_name)?;
exec_state.task_status_map.insert(resolved_task_id.clone(), TaskStatusValue::Running);
crate::persistence::update_execution_state(&state.repo_root, &branch, &plan_id, &plan_name, &exec_state)?;
```

**Impact**: The `task_status_map` in `execution.json` can become inconsistent with the individual `status.json` files. This affects:
- Dashboard views that read from `execution.json` for plan-level status summaries
- Dependency resolution logic that reads `task_status_map` to determine which tasks are completed
- Any consumer that trusts `task_status_map` as the source of truth for task statuses

## Solution Approach

Reuse the per-plan locking infrastructure from spec 010.1. Both `transition_task_status` and `claim_task` acquire the per-plan lock **once** before any mutations, and hold it through both the `status.json` update and the `task_status_map` update. This serializes all mutations on a given plan, preventing interleaving.

### Design Details

1. **Lock acquisition point**: After `resolve_plan_path` and `resolve_task_path` (which are read-only filesystem operations), before any persistence mutations.

2. **Lock scope** for `transition_task_status`:
   - `update_task_status` (status.json write with TOCTOU protection)
   - `read_execution_state` → `task_status_map.insert` → `update_execution_state` (execution state write)
   - `read_task` (final read for response)

3. **Lock scope** for `claim_task`:
   - `update_task_status` (status.json write with TOCTOU protection)
   - Agent lease re-read and re-write of status.json
   - `read_execution_state` → `task_status_map.insert` → `update_execution_state` (execution state write)

4. **No changes to persistence layer**: The per-plan lock operates at the API handler level, serializing callers. The persistence functions remain unchanged.

5. **Dependency on 010.1**: This spec assumes the per-plan lock infrastructure (`AppState.plan_locks`, `lock_plan()` middleware helper) is in place from spec 010.1.

### Why not merge into update_task_status?

The alternative of merging `task_status_map` maintenance into `update_task_status` would require:
- `update_task_status` to accept execution state parameters
- A more complex persistence function that atomically updates two files
- Handling the case where execution state doesn't exist yet

The per-plan lock is simpler, requires fewer code changes, and provides broader protection by serializing ALL plan-level mutations (not just status transitions).

## Relevant Files

### Files to Modify
| File | Changes |
|------|---------|
| `src/api/tasks.rs` | Wrap `transition_task_status` (lines 442-517) and `claim_task` (lines 531-636) with per-plan lock acquisition |

### Files for Reference (No Changes — provided by 010.1)
| File | Why |
|------|-----|
| `src/api/types.rs` | Contains `AppState` with `plan_locks` field (added by 010.1) |
| `src/api/middleware.rs` | Contains `lock_plan()` helper (added by 010.1) |
| `src/persistence/operations.rs` | Contains `update_task_status`, `read_execution_state`, `update_execution_state` |
| `src/persistence/schema.rs` | Contains `ExecutionState`, `TaskStatusValue` types |

### Files for Reference (Out of Scope — Overlord Path)
| File | Why |
|------|-----|
| `src/overlord/scheduler.rs` | Overlord's `transition_task_status` also has a gap (updates status.json but not task_status_map); addressed in follow-up |
| `src/builder/merge_coordinator.rs` | `update_execution_state` method calls persistence separately from overlord's `transition_task_status`; addressed in follow-up |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: `transition-lock-builder`
  - Role: Wrap `transition_task_status` and `claim_task` handlers with per-plan lock
  - Agent: builder

- **Validator**
  - Name: `transition-lock-validator`
  - Role: Verify lock acquisition, scope, and correctness
  - Agent: validator

- **Documenter**
  - Name: `transition-lock-documenter`
  - Role: Generate documentation for the transition race condition fix
  - Agent: documenter

## Step by Step Tasks

### 1. Wrap transition_task_status with per-plan lock
- **Task ID**: wrap-transition-task-status-lock
- **Depends On**: none (depends on 010.1 infrastructure being in place)
- **Assigned To**: transition-lock-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/tasks.rs`, `transition_task_status` handler (lines 442-517):
    - Ensure import: `use crate::api::middleware::lock_plan;` (if not already imported from 010.1)
    - After `resolve_task_path` (line 452) and BEFORE `read_task_status` (line 455), insert:
      ```rust
      // Acquire per-plan lock to serialize status.json and task_status_map updates.
      // This prevents TOCTOU races where a concurrent handler reads execution state
      // between our status.json write and task_status_map update, causing the map
      // to lag behind the individual status file.
      // See spec 010.1 for lock infrastructure details.
      let _plan_guard = lock_plan(&state, &plan_id).await;
      ```
    - The `_plan_guard` keeps the lock held through:
      - `read_task_status` (validation read)
      - `update_task_status` (status.json write)
      - `read_execution_state` → `task_status_map.insert` → `update_execution_state` (execution state write)
      - `read_task` (final read for response)
    - The guard is dropped automatically at end of function scope
  - Update the function doc comment to mention the per-plan lock:
    - Add: "Acquires the per-plan lock to serialize `status.json` and `task_status_map` updates atomically."
- **Acceptance Criteria**:
  - `transition_task_status` acquires the per-plan lock after path resolution and before any persistence operations
  - The lock guard is held through all status.json and execution state mutations
  - The lock guard is dropped at end of function scope (implicit via Rust drop semantics)
  - `cargo check` passes

### 2. Wrap claim_task with per-plan lock
- **Task ID**: wrap-claim-task-lock
- **Depends On**: none (depends on 010.1 infrastructure being in place; independent of Task 1)
- **Assigned To**: transition-lock-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/tasks.rs`, `claim_task` handler (lines 531-636):
    - After `resolve_task_path` (line 541) and BEFORE `read_task_status` (line 544), insert:
      ```rust
      // Acquire per-plan lock to serialize status.json and task_status_map updates.
      // This prevents TOCTOU races where a concurrent handler reads execution state
      // between our status.json write and task_status_map update.
      // See spec 010.1 for lock infrastructure details.
      let _plan_guard = lock_plan(&state, &plan_id).await;
      ```
    - The `_plan_guard` keeps the lock held through:
      - `read_task_status` (validation read)
      - `update_task_status` (status.json write)
      - Agent lease re-read and re-write of status.json (TOCTOU verification)
      - `read_execution_state` → `task_status_map.insert` → `update_execution_state` (execution state write)
      - `read_task` (final read for response)
    - The guard is dropped automatically at end of function scope
  - Update the function doc comment to mention the per-plan lock:
    - Add: "Acquires the per-plan lock to serialize `status.json` and `task_status_map` updates atomically."
- **Acceptance Criteria**:
  - `claim_task` acquires the per-plan lock after path resolution and before any persistence operations
  - The lock guard is held through all status.json and execution state mutations
  - The lock guard is dropped at end of function scope (implicit via Rust drop semantics)
  - `cargo check` passes

### 3. Integration verification and compilation
- **Task ID**: verify-compilation
- **Depends On**: wrap-transition-task-status-lock, wrap-claim-task-lock
- **Assigned To**: transition-lock-builder
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

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: verify-compilation
- **Assigned To**: transition-lock-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all existing tests pass
  - Run `cargo clippy -- -D warnings` — no warnings
  - Verify `transition_task_status` acquires per-plan lock via `lock_plan(&state, &plan_id).await`
  - Verify `claim_task` acquires per-plan lock via `lock_plan(&state, &plan_id).await`
  - Verify lock acquisition in `transition_task_status` occurs AFTER `resolve_plan_path` and `resolve_task_path` (read-only operations)
  - Verify lock acquisition in `transition_task_status` occurs BEFORE `read_task_status`, `update_task_status`, and `read_execution_state` (all mutations)
  - Verify lock acquisition in `claim_task` occurs AFTER `resolve_plan_path` and `resolve_task_path` (read-only operations)
  - Verify lock acquisition in `claim_task` occurs BEFORE `read_task_status`, `update_task_status`, and `read_execution_state` (all mutations)
  - Verify the lock guard variable is named `_plan_guard` (with underscore prefix to suppress unused variable warning)
  - Verify the lock scope covers both `status.json` and `task_status_map` mutations in each handler
  - Verify the lock guard is dropped at end of function scope (no explicit drop needed)

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: transition-lock-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/` describing the transition_task_status and claim_task race condition fix
  - Document how the per-plan lock prevents interleaving of status.json and task_status_map mutations
  - Document the consistency guarantee: under the lock, status.json and task_status_map are updated atomically from the perspective of concurrent handlers

## Acceptance Criteria

1. **Compilation**: `cargo check` passes with zero errors
2. **Linting**: `cargo clippy -- -D warnings` passes with zero warnings
3. **Formatting**: `cargo fmt --check` passes with zero formatting issues
4. **Tests**: `cargo test` passes — all existing tests continue to pass
5. **transition_task_status lock**: Acquires per-plan lock via `lock_plan(&state, &plan_id).await` after path resolution, before any persistence operations
6. **transition_task_status scope**: Lock guard covers `update_task_status` (status.json) AND `update_execution_state` (task_status_map)
7. **claim_task lock**: Acquires per-plan lock via `lock_plan(&state, &plan_id).await` after path resolution, before any persistence operations
8. **claim_task scope**: Lock guard covers `update_task_status` (status.json), agent lease re-write, AND `update_execution_state` (task_status_map)
9. **No new dependencies**: Solution uses only the lock infrastructure from 010.1
10. **No new files**: Only `src/api/tasks.rs` is modified

## Validation Commands

- `cargo check` — Verify compilation
- `cargo clippy -- -D warnings` — Verify no clippy warnings
- `cargo fmt --check` — Verify formatting
- `cargo test` — Run all tests
- `cargo test --lib api` — Run API module tests specifically

## Notes

- **Dependency on 010.1**: This spec assumes 010.1 has been implemented. The `AppState.plan_locks` field, `lock_plan()` middleware helper, and `AppState` initialization must all be in place. If 010.1 has not been executed, this spec must wait.

- **Lock granularity**: Per-plan locking is the correct granularity because the race condition involves concurrent access to a single plan's `status.json` files and its `execution.json` `task_status_map`. Both are scoped to a single plan.

- **Lock ordering**: The lock is acquired after `resolve_plan_path` and `resolve_task_path`, which only perform read-only filesystem operations (checking directory existence, reading filenames). This minimizes lock hold time for path resolution.

- **Performance impact**: Minimal. The per-plan mutex is only held during file I/O (status.json write + execution state read-modify-write). Concurrent operations on *different* plans are not blocked. Only concurrent operations on the *same* plan are serialized.

- **Deadlock risk**: None. Each handler acquires exactly one per-plan lock, held for a bounded duration (the scope of the handler function). No nested locks are acquired.

- **Consistency guarantee**: Under the lock, `status.json` and `task_status_map` are updated atomically from the perspective of concurrent handlers. No handler can observe an intermediate state where `status.json` has been updated but `task_status_map` has not (or vice versa).

- **Out of scope — Overlord path**: The overlord's `transition_task_status` method (`src/overlord/scheduler.rs`) and `merge_coordinator.update_execution_state` (`src/builder/merge_coordinator.rs`) also have a gap between status.json update and task_status_map update. However, the overlord runs in a single-threaded context with a merge lock that serializes concurrent merge operations. This gap is a lower priority and should be addressed in a follow-up spec. The per-plan lock infrastructure from 010.1 could be extended to the overlord if needed, but that requires the overlord to have access to `AppState.plan_locks`, which is currently scoped to the API module.

- **Why a separate spec from 010.1 and 010.2**: While 010.1 covers the plan file locking infrastructure and 010.2 covers `delete_task` execution state locking, this spec focuses specifically on the `transition_task_status` and `claim_task` handlers — the most frequently called status mutation paths. These handlers have a unique race pattern: they update TWO different files (status.json and execution.json) in sequence, which is a different risk profile than the single-file races covered by 010.1 and 010.2.

- **Relation to code review finding**: This spec directly addresses the code review finding: "TOCTOU race in `transition_task_status` — execution state update is not atomic." The per-plan lock ensures that the `status.json` write and `task_status_map` write are serialized, preventing the `task_status_map` from lagging behind `status.json`.
