# Per-Plan Locking — TOCTOU Race Fix

## Overview

The per-plan locking mechanism prevents **Time-of-Check-to-Time-of-Use (TOCTOU)** race conditions in the Nexum REST API. Without it, concurrent requests modifying the same plan's data (e.g., creating two tasks simultaneously) could interleave their read-modify-write cycles, causing lost updates or duplicate entries in the plan's task list.

## The Race Condition

Several API handlers perform a non-atomic **read → modify → write** cycle on a plan's markdown file (`plan.md`). For example, in `create_task`:

```rust
// Race window: concurrent handler can read the same state here
let mut plan = read_plan(&state.repo_root, &branch, &plan_id, &plan_name)?;
plan.tasks.push(TaskReference { id: task_id, name: task.name, completed: false });
// ... another handler reads the same plan state, pushes its own reference ...
crate::persistence::update_plan(&state.repo_root, &branch, &plan_id, &plan_name, &plan)?;
// Writes back, overwriting the other handler's changes — lost update
```

Two concurrent `create_task` calls on the same plan could both read the plan with 3 tasks, each push their own task reference (now 4 in memory), and both write back — resulting in only one of the two new tasks being persisted.

**Affected handlers before the fix:**
- `create_task` — appends a `TaskReference` to the plan's task list
- `delete_task` — removes a `TaskReference` from the plan's task list
- `update_plan` — modifies plan fields (name, goal, scope, background)
- `transition_plan_status` — changes the plan's status field

## How the Locking Mechanism Works

### Data Structure

The lock map lives in `AppState` (defined in `src/api/types.rs`):

```rust
pub struct AppState {
    pub repo_root: PathBuf,
    pub config: config::Config,
    pub orchestrator: Option<Arc<WorkflowOrchestrator>>,
    /// Per-plan async mutex map for serializing read-modify-write operations.
    pub plan_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}
```

**Two-level locking design:**

| Level | Type | Purpose | Hold Time |
|-------|------|---------|-----------|
| Outer | `tokio::sync::RwLock` | Protects the `HashMap` itself (lookups, insertions, deletions) | Microseconds — only during map access |
| Inner | `tokio::sync::Mutex<()>` (per plan) | Serializes read-modify-write on a specific plan's files | Milliseconds — covers file I/O |

The outer `RwLock` allows concurrent lookups of different plans' mutexes. The inner per-plan `Mutex<()>` is what actually serializes handlers working on the same plan.

### Helper Functions

Both helpers live in `src/api/middleware.rs`:

#### `lock_plan(state: &AppState, plan_id: &str) -> Arc<tokio::sync::Mutex<()>>`

Acquires the per-plan mutex for the given `plan_id`. Uses a **read-first** strategy:

1. Acquires a **read lock** on the outer `RwLock` to look up the plan's mutex
2. If the entry exists, clones the `Arc` and returns it
3. If not found, drops the read lock, acquires a **write lock**, creates a new `Arc<tokio::sync::Mutex::new(())>`, inserts it into the map, and returns it

The caller then locks the returned mutex: `let _guard = mutex.lock().await;`

#### `remove_plan_lock(state: &AppState, plan_id: &str)`

Removes the per-plan mutex entry from the map. Called by `delete_plan` to prevent memory leaks from accumulated mutex entries for deleted plans.

### Lock Acquisition Pattern

Every protected handler follows this pattern:

```rust
// 1. Resolve plan path (read-only, no lock needed)
let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

// 2. Acquire the per-plan lock
let plan_mutex = lock_plan(&state, &plan_id).await;
let _plan_guard = plan_mutex.lock().await;

// 3. Read-modify-write (protected by the lock)
let mut plan = read_plan(...)?;
plan.tasks.push(TaskReference { ... });
update_plan(...)?;

// 4. Lock guard dropped at end of scope → mutex released
```

The lock is acquired **after** `resolve_plan_path` (which only reads the filesystem) and **before** any read-modify-write on plan data files. The guard's scope covers the entire critical section.

## Protected Handlers

| Handler | File | Lock Scope |
|---------|------|------------|
| `create_task` | `src/api/tasks.rs` | `read_plan` → `plan.tasks.push(...)` → `update_plan` → `add_task_to_execution` |
| `delete_task` | `src/api/tasks.rs` | `read_plan` → `plan.tasks.retain(...)` → `update_plan` → execution state update |
| `update_plan` | `src/api/plans.rs` | `read_plan` → field modifications → `update_plan` |
| `transition_plan_status` | `src/api/plans.rs` | `read_plan` → `plan.status = ...` → `update_plan` |
| `delete_plan` | `src/api/plans.rs` | No lock needed (deletes the plan entirely); calls `remove_plan_lock` for cleanup |

**Not locked** (intentionally):
- `list_tasks`, `get_task`, `list_plans`, `get_plan` — read-only operations
- `update_task` — modifies only the task's own markdown file, not the plan
- `transition_task_status`, `claim_task` — already have TOCTOU protection at the persistence layer via file-content comparison

## Performance Characteristics

### Granularity
Locks are **per-plan**, not global. Concurrent operations on different plans proceed in parallel without blocking each other. Only concurrent operations on the **same** plan are serialized.

### Overhead
- **Outer RwLock**: Held for microseconds during HashMap lookup/insertion. Read-heavy (most calls find an existing entry), so multiple handlers can look up different plan locks concurrently.
- **Inner Mutex**: Held during file I/O (read plan.md → modify in memory → write plan.md). Typically a few milliseconds on SSD storage.
- **No new dependencies**: Uses only `tokio::sync::RwLock`, `tokio::sync::Mutex`, and `std::collections::HashMap` — all already present in the dependency tree.

### Deadlock Risk
**None.** Each handler acquires exactly one per-plan lock, held for a bounded duration (the scope of the handler). No nested locks are acquired, so deadlock is impossible.

## Memory Management

- Mutex entries are created lazily on first access to a plan.
- Entries are cleaned up when `delete_plan` removes a plan (via `remove_plan_lock`).
- For long-lived plans, each entry costs approximately the size of one `tokio::sync::Mutex<()>` — negligible.
- Without cleanup, deleted plans would leave orphaned mutex entries in the map. The `remove_plan_lock` call in `delete_plan` prevents this leak.

## Files Modified

| File | Changes |
|------|---------|
| `src/api/types.rs` | Added `plan_locks` field to `AppState`; added `use std::collections::HashMap` and `use tokio::sync::RwLock` imports |
| `src/api/middleware.rs` | Added `lock_plan()` and `remove_plan_lock()` helper functions |
| `src/api/tasks.rs` | `create_task` and `delete_task` acquire per-plan lock before plan read-modify-write |
| `src/api/plans.rs` | `update_plan` and `transition_plan_status` acquire per-plan lock; `delete_plan` calls `remove_plan_lock` for cleanup |
