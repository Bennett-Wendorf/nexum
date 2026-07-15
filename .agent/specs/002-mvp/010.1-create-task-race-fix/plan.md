# Plan: 010.1 - TOCTOU Race Fix: Per-Plan Locking for Plan Read-Modify-Write

## Task Description
Fix a Time-of-Check-to-Time-of-Use (TOCTOU) race condition in the `create_task` and `delete_task` handlers where plan read-modify-write operations are not atomic. Concurrent requests can interleave between reading the plan file and writing it back, causing lost updates or duplicate task references.

## Objective
Introduce per-plan async locking to serialize all read-modify-write operations on a given plan's markdown file, ensuring concurrent handlers cannot corrupt plan data.

## Problem Statement

In `src/api/tasks.rs`, the `create_task` handler (lines 278-286) performs a non-atomic read-modify-write on the plan file:

```rust
// Line 278-286: TOCTOU race window
let mut plan = read_plan(&state.repo_root, &branch, &plan_id, &plan_name)?;
plan.tasks.push(TaskReference { id: task_id.clone(), name: task.name.clone(), completed: false });
crate::persistence::update_plan(&state.repo_root, &branch, &plan_id, &plan_name, &plan)?;
```

Between `read_plan` and `update_plan`, a concurrent `create_task` or `delete_task` can:
1. Read the same plan state
2. Make its own modifications
3. Write back, overwriting the first handler's changes

This causes **lost updates** (one handler's TaskReference is discarded) or **duplicate references** (both handlers push the same reference).

The same race exists in:
- `delete_task` (lines 408-412): reads plan, retains tasks, writes back
- `update_plan` in `plans.rs` (lines 252-281): reads plan, modifies fields, writes back
- `transition_plan_status` in `plans.rs` (lines 334-351): reads plan, changes status, writes back

## Solution Approach

Introduce a **per-plan async mutex map** stored in `AppState`. The map is keyed by `plan_id` and holds `Arc<tokio::sync::Mutex<()>>` entries — one mutex per plan.

### Design Details

1. **Data structure**: `Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>` stored in `AppState`
   - The outer `RwLock` protects the HashMap itself (briefly held to look up / create entries)
   - Each value is an `Arc<tokio::sync::Mutex<()>>` — the per-plan lock held for the duration of the operation

2. **Key**: `plan_id` (e.g., `"PLAN-001"`). Since plans are uniquely identified by `(branch, plan_id)` in the API, using just `plan_id` is safe — plans with the same ID in different branches would be serialized conservatively, which is correct behavior.

3. **Helper function**: A new helper `acquire_plan_lock(&AppState, &str)` in `src/api/middleware.rs` that:
   - Acquires the outer RwLock (read) to look up the per-plan mutex
   - If not found, upgrades to write lock, creates a new mutex entry, inserts it
   - Returns the `Arc<tokio::sync::Mutex<()>>` for the caller to `.await .lock()`

4. **Cleanup**: When `delete_plan` removes a plan, the corresponding mutex entry is removed from the map.

5. **No new dependencies**: Uses only `tokio::sync::RwLock`, `tokio::sync::Mutex`, and `std::collections::HashMap` — all already available.

### Why not `dashmap`?

`dashmap` would provide per-key locking on the map itself, but the outer map lock is only held briefly (microseconds) to look up or create the per-plan mutex entry. The per-plan mutex is what serializes the actual file I/O. The `RwLock<HashMap>` approach is simpler, avoids a new dependency, and has negligible contention since map operations are fast.

## Relevant Files

### Files to Modify
| File | Changes |
|------|---------|
| `src/api/types.rs` | Add `plan_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>` field to `AppState` |
| `src/api/middleware.rs` | Add `acquire_plan_lock()` helper and `release_plan_lock()` cleanup helper |
| `src/api/tasks.rs` | Wrap plan read-modify-write in `create_task` (lines 278-286) and `delete_task` (lines 408-426) with per-plan lock |
| `src/api/plans.rs` | Wrap plan read-modify-write in `update_plan` (lines 252-281) and `transition_plan_status` (lines 334-351) with per-plan lock; clean up lock on `delete_plan` |
| `src/api/mod.rs` | Initialize `plan_locks` in `AppState` construction (if not already done at startup) |

### Files for Reference (No Changes)
| File | Why |
|------|-----|
| `src/persistence/operations.rs` | Contains `read_plan`, `update_plan`, `add_task_to_execution` — the persistence functions called by handlers |
| `src/persistence/schema.rs` | Contains `Plan`, `TaskReference`, `ExecutionState` types |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: `race-fix-builder`
  - Role: Implement per-plan locking infrastructure and update all affected handlers
  - Agent: builder

- **Validator**
  - Name: `race-fix-validator`
  - Role: Verify implementation correctness, run tests, check for compilation errors
  - Agent: validator

- **Documenter**
  - Name: `race-fix-documenter`
  - Role: Generate documentation for the per-plan locking mechanism
  - Agent: documenter

## Step by Step Tasks

### 1. Add plan_locks field to AppState
- **Task ID**: add-app-state-field
- **Depends On**: none
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - Add `pub plan_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>` field to `AppState` in `src/api/types.rs`
  - Add necessary imports: `use std::collections::HashMap; use std::sync::Arc; use tokio::sync::RwLock; use tokio::sync::Mutex;`
  - Ensure `AppState` remains `Clone`-derivable (the `Arc<RwLock<...>>` type is `Clone`)
  - Update the `AppState` construction site (in `src/main.rs` or wherever it's initialized) to initialize `plan_locks: Arc::new(RwLock::new(HashMap::new()))`
- **Acceptance Criteria**:
  - `AppState` compiles with the new field
  - `AppState` construction initializes `plan_locks` with an empty map
  - `cargo check` passes

### 2. Implement acquire_plan_lock helper
- **Task ID**: implement-lock-helper
- **Depends On**: add-app-state-field
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - Add `acquire_plan_lock(state: &AppState, plan_id: &str) -> impl Future<Output = tokio::sync::MappedMutexGuard<()>>` helper in `src/api/middleware.rs`
  - Logic:
    1. Attempt read lock on `state.plan_locks`
    2. If entry exists, return `Arc::clone` of the mutex
    3. If not, upgrade to write lock, create new `Arc<tokio::sync::Mutex::new(())>`, insert into map, return clone
  - Add `remove_plan_lock(state: &AppState, plan_id: &str)` helper for cleanup on plan deletion
  - Add `lock_plan(state: &AppState, plan_id: &str) -> impl Future<Output = tokio::sync::MutexGuard<'_, ()>>` convenience function that acquires the entry AND locks the mutex
- **Acceptance Criteria**:
  - Helper functions compile and are accessible from `tasks.rs` and `plans.rs`
  - Helper correctly creates new mutex entries on first access
  - Helper correctly removes entries on cleanup

### 3. Wrap create_task plan operations with lock
- **Task ID**: lock-create-task
- **Depends On**: implement-lock-helper
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/tasks.rs`, `create_task` handler (around lines 277-297):
    - Before `read_plan`, acquire the per-plan lock: `let _guard = lock_plan(&state, &plan_id).await;`
    - The guard keeps the lock held through `read_plan`, `plan.tasks.push(...)`, and `update_plan`
    - Also extend the lock to cover `add_task_to_execution` since it modifies execution state for the same plan
  - The lock guard is dropped at end of function scope
- **Acceptance Criteria**:
  - `create_task` acquires per-plan lock before reading plan
  - Lock is held through all plan and execution state modifications
  - `cargo check` passes

### 4. Wrap delete_task plan operations with lock
- **Task ID**: lock-delete-task
- **Depends On**: implement-lock-helper
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/tasks.rs`, `delete_task` handler (around lines 407-426):
    - Before `read_plan`, acquire the per-plan lock: `let _guard = lock_plan(&state, &plan_id).await;`
    - The guard covers `read_plan`, `plan.tasks.retain(...)`, `update_plan`, and execution state modifications
- **Acceptance Criteria**:
  - `delete_task` acquires per-plan lock before reading plan
  - Lock is held through all plan and execution state modifications
  - `cargo check` passes

### 5. Wrap update_plan handler with lock
- **Task ID**: lock-update-plan
- **Depends On**: implement-lock-helper
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/plans.rs`, `update_plan` handler (around lines 252-281):
    - Before `read_plan`, acquire the per-plan lock
    - Guard covers `read_plan`, field modifications, and `update_plan` persistence call
- **Acceptance Criteria**:
  - `update_plan` acquires per-plan lock before reading plan
  - Lock is held through all modifications
  - `cargo check` passes

### 6. Wrap transition_plan_status handler with lock
- **Task ID**: lock-transition-plan-status
- **Depends On**: implement-lock-helper
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/plans.rs`, `transition_plan_status` handler (around lines 334-351):
    - Before `read_plan`, acquire the per-plan lock
    - Guard covers `read_plan`, `plan.status = new_status`, and `update_plan` persistence call
- **Acceptance Criteria**:
  - `transition_plan_status` acquires per-plan lock before reading plan
  - Lock is held through all modifications
  - `cargo check` passes

### 7. Clean up lock on plan deletion
- **Task ID**: cleanup-lock-on-delete
- **Depends On**: implement-lock-helper
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/plans.rs`, `delete_plan` handler:
    - After removing plan directories, call `remove_plan_lock(&state, &plan_id)` to clean up the mutex entry
    - This prevents memory leaks from accumulated mutex entries for deleted plans
- **Acceptance Criteria**:
  - `delete_plan` removes the per-plan lock entry after deletion
  - No mutex entries remain for deleted plans

### 8. Integration verification and compilation
- **Task ID**: verify-compilation
- **Depends On**: lock-create-task, lock-delete-task, lock-update-plan, lock-transition-plan-status, cleanup-lock-on-delete
- **Assigned To**: race-fix-builder
- **Agent**: builder
- **Actions**:
  - Run `cargo check` to verify compilation
  - Run `cargo clippy` to catch any linting issues
  - Fix any compilation or clippy errors
- **Acceptance Criteria**:
  - `cargo check` passes with no errors
  - `cargo clippy` passes with no warnings
  - All modified files have consistent formatting (`cargo fmt --check`)

### 9. Final Validation
- **Task ID**: validate-all
- **Depends On**: verify-compilation
- **Assigned To**: race-fix-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all existing tests pass
  - Run `cargo clippy -- -D warnings` — no warnings
  - Verify `AppState` has `plan_locks` field
  - Verify `acquire_plan_lock` / `lock_plan` / `remove_plan_lock` helpers exist in middleware.rs
  - Verify all 4 handlers (`create_task`, `delete_task`, `update_plan`, `transition_plan_status`) acquire the per-plan lock
  - Verify `delete_plan` cleans up the lock entry

### 10. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: race-fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/` describing the per-plan locking mechanism
  - Document the race condition that was fixed and how the locking prevents it

## Acceptance Criteria

1. **Compilation**: `cargo check` passes with zero errors
2. **Linting**: `cargo clippy -- -D warnings` passes with zero warnings
3. **Formatting**: `cargo fmt --check` passes with zero formatting issues
4. **Tests**: `cargo test` passes — all existing tests continue to pass
5. **AppState**: Contains `plan_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>` field
6. **Middleware**: Contains `lock_plan()` and `remove_plan_lock()` helper functions
7. **create_task**: Acquires per-plan lock before `read_plan`, holds through `update_plan` and `add_task_to_execution`
8. **delete_task**: Acquires per-plan lock before `read_plan`, holds through `update_plan` and execution state modifications
9. **update_plan** (plans.rs): Acquires per-plan lock before `read_plan`, holds through `update_plan`
10. **transition_plan_status**: Acquires per-plan lock before `read_plan`, holds through `update_plan`
11. **delete_plan**: Removes per-plan lock entry after deleting plan directories
12. **No new dependencies**: Solution uses only existing crate dependencies (tokio, std)

## Validation Commands

- `cargo check` — Verify compilation
- `cargo clippy -- -D warnings` — Verify no clippy warnings
- `cargo fmt --check` — Verify formatting
- `cargo test` — Run all tests
- `cargo test --lib api` — Run API module tests specifically

## Notes

- **Lock granularity**: Per-plan (not per-task) is the correct granularity since the race is about concurrent modifications to the same plan's `tasks` list. Task-level operations (like `transition_task_status`) already have TOCTOU protection at the persistence layer via file content comparison.

- **Lock ordering**: All handlers acquire the lock after `resolve_plan_path`, which only reads the filesystem (no plan mutations). This minimizes lock hold time for the initial path resolution.

- **Performance impact**: Minimal. The per-plan mutex is only held during file I/O (read-modify-write of plan.md). Concurrent operations on *different* plans are not blocked. Only concurrent operations on the *same* plan are serialized.

- **Deadlock risk**: None. Each handler acquires exactly one per-plan lock, and the lock is held for a bounded duration (the scope of the handler function). No nested locks are acquired.

- **Memory management**: Mutex entries are cleaned up on plan deletion. For long-lived plans, the per-plan mutex is a negligible memory cost (one `Mutex<()>` per plan).

- **Existing TOCTOU protection**: The `add_task_to_execution` function in `persistence/operations.rs` already has file-content-based TOCTOU protection. The per-plan lock provides an additional layer that prevents the race at a higher level, before the persistence layer is even called.
