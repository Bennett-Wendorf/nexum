# Git Operations Module

## Overview

The `src/git/` module provides a complete, typed Rust API for interacting with git repositories through async subprocess calls to the system `git` binary. It is the sole mechanism for all git operations within Nexum — no git libraries are used.

The module supports Nexum's parallel task execution workflow: each task gets its own isolated worktree and ephemeral branch, and completed tasks are merged into a plan branch in dependency order. After merge, branches and worktrees are cleaned up automatically.

## What Was Built

Six submodules work together to provide the full git operations lifecycle:

| Submodule | Purpose |
|---|---|
| `subprocess.rs` | Async git subprocess wrapper with builder pattern, timeout enforcement, and output capture |
| `errors.rs` | 11 `thiserror`-based error variants covering all failure modes |
| `branch.rs` | Branch creation, deletion, checkout, listing, and task workspace setup/teardown |
| `worktree.rs` | Worktree spawn, removal (with stale fallback), listing, and existence checks |
| `merge.rs` | Branch merging with conflict detection, plus dependency-ordered merge coordination |
| `tests.rs` | 25 async unit and integration tests using temporary git repositories |

All public types and functions are re-exported through `mod.rs` for ergonomic access as `nexum::git::*`.

## Technical Implementation

### Files

| File | Lines | Description |
|---|---|---|
| `src/git/mod.rs` | 91 | Module root, re-exports, doc comments |
| `src/git/subprocess.rs` | 239 | `GitCommand` builder, `GitOutput`, `git()` convenience function |
| `src/git/errors.rs` | 96 | `GitError` enum (12 variants), `Result<T>` type alias |
| `src/git/branch.rs` | 272 | Branch operations, task workspace setup/teardown |
| `src/git/worktree.rs` | 303 | Worktree lifecycle, `WorktreeInfo` struct |
| `src/git/merge.rs` | 404 | Merge operations, `MergePlan`, dependency-ordered merge |
| `src/git/tests.rs` | 392 | 25 tests across all submodules |

### Dependencies

- `tokio` (process feature) — async subprocess spawning
- `thiserror` — error derive macro
- `tempfile` (dev-dependency) — isolated test repositories
- `tracing` — warning logs for fallback operations

---

## 1. Git Subprocess Wrapper Architecture

The subprocess wrapper (`subprocess.rs`) is the foundation of all git operations. Every git command flows through it.

### `GitCommand` Builder

A fluent builder that accumulates arguments, environment variables, and an optional timeout before executing:

```rust
let output = GitCommand::new(&repo_root)
    .args(&["log", "--oneline", "-5"])
    .env("GIT_AUTHOR_NAME", "Nexum")
    .timeout(Duration::from_secs(10))
    .execute()
    .await?;
```

Key design decisions:
- **Working directory**: Always set to `repo_root` — critical for correct git behavior.
- **Output capture**: Both `stdout` and `stderr` are captured as UTF-8 strings (lossy conversion).
- **Timeout enforcement**: Uses `tokio::time::timeout` with a spawned wait task. On timeout, the wait task is aborted, which drops the child handle and terminates the git subprocess.
- **Default timeout**: 30 seconds (`DEFAULT_TIMEOUT`), configurable per-command.

### `GitOutput`

```rust
pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
```

Provides a `success()` method (exit code == 0).

### `git()` Convenience Function

For simple commands, use the one-liner:

```rust
let output = git(&repo_root, &["rev-parse", "HEAD"]).await?;
```

This creates a `GitCommand` with default 30-second timeout.

---

## 2. Branch Strategy: Plan Branches, Task Branches, Worktree Isolation

Nexum's branch strategy (defined in `design/persistence.md`) is implemented as follows:

### Plan Branches

Plan branches (e.g., `feature/auth-overhaul`) are the source of truth for a plan. They are created from a base branch and may optionally be pushed to remote:

```rust
create_plan_branch(&repo_root, "feature/auth-overhaul", "main", push_to_remote: true).await?;
```

This function checks for pre-existing branches (returns `GitError::BranchExists` if present) before creating.

### Task Branches

Task branches follow the `task/<TASK_ID>` naming convention (e.g., `task/TASK-001`). They are:
- **Local-only** — never pushed to remote
- **Ephemeral** — deleted after merge

```rust
let branch_name = create_task_branch(&repo_root, "TASK-001", "feature/auth-overhaul").await?;
// Returns "task/TASK-001"
```

Helper functions:
- `task_branch_name("TASK-001")` → `"task/TASK-001"`
- `is_task_branch("task/TASK-001")` → `true`
- `is_task_branch("feature/auth")` → `false`

### Worktree Isolation

Each task gets its own isolated filesystem via `git worktree add`. Worktrees are stored under `<repo_root>/.worktrees/<TASK_ID>/`. Multiple builder agents can work in parallel without cross-talk because each has its own working directory, while sharing the same git object database.

---

## 3. Merge Workflow: Merge, Conflict Detection, Dependency Ordering

### Basic Merge

`merge_branch()` merges a source branch into a target branch:

1. Records the current branch for restoration
2. Checks out the target branch
3. Runs `git merge --no-ff <source>` (always creates a merge commit for audit trail)
4. On success, restores the original branch
5. On failure (exit code 1), checks for conflicts via `git diff --name-only --diff-filter=U`

```rust
let output = merge_branch(&repo_root, "task/TASK-001", "feature/auth-overhaul").await?;
```

### Conflict Detection

When a merge fails with exit code 1, the module distinguishes between two cases:

- **Merge conflict**: `git diff --name-only --diff-filter=U` returns conflicted files → `GitError::MergeConflict` with branch names and file list
- **Other failure**: No conflicts detected → `GitError::SubprocessFailure`

Conflict detection helpers:
- `has_merge_conflicts(&repo_root)` → `bool`
- `list_conflicted_files(&repo_root)` → `Vec<String>`
- `is_merging(&repo_root)` → checks for `.git/MERGE_HEAD` file

### Merge Abort

`abort_merge()` cancels an in-progress merge. It checks `is_merging()` first, so it's safe to call even when no merge is active (no-op).

### Task Branch Merge

`merge_task_branch()` is the primary merge entry point for the Overlord:

```rust
merge_task_branch(&repo_root, "TASK-001", "feature/auth-overhaul").await?;
```

It validates the task branch exists, checks out the plan branch, merges with `--no-ff`, and restores the original branch on completion.

---

## 4. Worktree Lifecycle: Spawn, Execute, Merge, Cleanup

The complete worktree lifecycle:

### Setup

`setup_task_workspace()` atomically creates both the task branch and worktree:

```rust
let wt_path = setup_task_workspace(&repo_root, "TASK-001", "feature/auth-overhaul").await?;
// Returns path like /repo/.worktrees/TASK-001/
```

On failure, it cleans up partial state (e.g., deletes the branch if worktree spawn fails).

### Execute

Builder agents work inside the worktree directory returned by `setup_task_workspace()`. The worktree is a fully functional git working directory checked out to the task branch.

### Merge

The Overlord calls `merge_task_branch()` to merge the task branch into the plan branch. This operates on the main repository, not the worktree.

### Cleanup

After merge, `cleanup_merged_task()` removes resources:

```rust
cleanup_merged_task(&repo_root, "TASK-001").await?;
```

This deletes the task branch and removes the worktree. If normal worktree removal fails (stale lock file), it falls back to force removal, then to manual `fs::remove_dir_all()`.

`teardown_task_workspace()` provides a complete teardown for task failure cases:

```rust
teardown_task_workspace(&repo_root, "TASK-001").await?;
```

---

## 5. Rust API (Public Functions and Types)

### Types

| Type | Description |
|---|---|
| `GitError` | 12 error variants (see section 6) |
| `Result<T>` | `std::result::Result<T, GitError>` |
| `GitCommand` | Builder for git subprocess commands |
| `GitOutput` | Captured stdout, stderr, exit code |
| `WorktreeInfo` | Worktree path, branch name, bare flag |
| `MergePlan` | Dependency-ordered merge coordination state |

### Branch Operations

| Function | Signature |
|---|---|
| `create_branch` | `async fn(&Path, &str, &str) -> Result<()>` |
| `create_task_branch` | `async fn(&Path, &str, &str) -> Result<String>` |
| `create_plan_branch` | `async fn(&Path, &str, &str, bool) -> Result<()>` |
| `delete_branch` | `async fn(&Path, &str) -> Result<()>` (no-op if missing) |
| `checkout_branch` | `async fn(&Path, &str) -> Result<()>` |
| `list_local_branches` | `async fn(&Path) -> Result<Vec<String>>` |
| `branch_exists` | `async fn(&Path, &str) -> Result<bool>` |
| `current_branch` | `async fn(&Path) -> Result<String>` |
| `task_branch_name` | `fn(&str) -> String` (sync) |
| `is_task_branch` | `fn(&str) -> bool` (sync) |

### Worktree Operations

| Function | Signature |
|---|---|
| `spawn_worktree` | `async fn(&Path, &str, &str) -> Result<PathBuf>` |
| `remove_worktree` | `async fn(&Path, &str) -> Result<()>` |
| `remove_worktree_force` | `async fn(&Path, &str) -> Result<()>` |
| `list_worktrees` | `async fn(&Path) -> Result<Vec<WorktreeInfo>>` |
| `worktree_exists` | `async fn(&Path, &str) -> Result<bool>` |
| `worktree_path` | `fn(&Path, &str) -> PathBuf` (sync) |
| `worktrees_dir` | `fn(&Path) -> PathBuf` (sync) |

### Merge Operations

| Function | Signature |
|---|---|
| `merge_branch` | `async fn(&Path, &str, &str) -> Result<GitOutput>` |
| `merge_task_branch` | `async fn(&Path, &str, &str) -> Result<()>` |
| `abort_merge` | `async fn(&Path) -> Result<()>` |
| `has_merge_conflicts` | `async fn(&Path) -> Result<bool>` |
| `list_conflicted_files` | `async fn(&Path) -> Result<Vec<String>>` |
| `is_merging` | `async fn(&Path) -> Result<bool>` |

### Coordination

| Function | Signature |
|---|---|
| `determine_merge_order` | `fn(&MergePlan) -> Result<Vec<String>>` (sync) |
| `next_mergeable_tasks` | `fn(&MergePlan) -> Result<Vec<String>>` (sync) |
| `execute_merge_sequence` | `async fn(&mut MergePlan) -> Result<Vec<String>>` |
| `cleanup_merged_task` | `async fn(&Path, &str) -> Result<()>` |

### Workspace

| Function | Signature |
|---|---|
| `setup_task_workspace` | `async fn(&Path, &str, &str) -> Result<PathBuf>` |
| `teardown_task_workspace` | `async fn(&Path, &str) -> Result<()>` |

### Subprocess

| Function | Signature |
|---|---|
| `git` | `async fn(&Path, &[&str]) -> Result<GitOutput>` |

---

## 6. Error Handling and Common Failure Modes

The `GitError` enum has 12 variants, each carrying contextual information:

### Subprocess Errors

| Variant | When It Occurs | Context |
|---|---|---|
| `SubprocessFailure` | Git exits with non-zero code | command, exit_code, stdout, stderr |
| `Timeout` | Command exceeds configured timeout | command, duration |
| `SpawnFailed` | Cannot spawn git process | io::Error |
| `GitNotInstalled` | git binary not in PATH | (none) |

### Branch Errors

| Variant | When It Occurs | Context |
|---|---|---|
| `BranchNotFound` | Referenced branch doesn't exist | branch name |
| `BranchExists` | Plan branch already exists | branch name |

### Merge Errors

| Variant | When It Occurs | Context |
|---|---|---|
| `MergeConflict` | Merge produces conflicts | branch, plan_branch, conflicted files |
| `DependencyNotMet` | Task dependencies not yet merged | task_id, unmet dependency list |

### Worktree Errors

| Variant | When It Occurs | Context |
|---|---|---|
| `WorktreeNotFound` | Expected worktree doesn't exist | path |
| `WorktreeExists` | Worktree already at target path | path |
| `WorktreeStale` | Lock file present | path |

### I/O Errors

| Variant | When It Occurs | Context |
|---|---|---|
| `Io` | Filesystem operation fails | path, source io::Error |

### Common Failure Modes

1. **Stale worktrees**: If a builder agent is killed mid-operation, the worktree may have a lock file. `remove_worktree()` handles this by falling back to `--force`, then to manual `fs::remove_dir_all()`.

2. **Merge conflicts**: Expected during parallel development. The module detects and reports them but does not auto-resolve — conflict resolution is the Overlord's responsibility.

3. **Missing branches**: `delete_branch()` treats non-existent branches as no-op, making cleanup idempotent.

4. **Partial setup failure**: `setup_task_workspace()` cleans up the branch if worktree spawn fails, preventing orphaned branches.

---

## 7. Dependency-Ordered Merge Algorithm

### `MergePlan` Structure

```rust
pub struct MergePlan {
    pub repo_root: PathBuf,          // Repository root path
    pub plan_branch: String,          // Target plan branch
    pub pending_tasks: Vec<String>,   // Tasks waiting to merge
    pub merged_tasks: Vec<String>,    // Tasks already merged
    pub dependencies: HashMap<String, Vec<String>>, // task_id -> [dep task IDs]
}
```

The Overlord populates this structure from the task dependency graph defined in `task.md` files and `status.json`.

### Kahn's Algorithm (Topological Sort)

`determine_merge_order()` uses Kahn's algorithm (BFS-based topological sort) with O(V + E) complexity:

1. Build an in-degree map for pending tasks only (ignoring already-merged tasks)
2. Build an adjacency list of pending-task dependencies
3. Seed a queue with tasks that have zero in-degree (no unmet dependencies)
4. Process the queue: for each task, decrement in-degree of dependents; enqueue those that reach zero
5. If the result doesn't include all pending tasks, a circular dependency exists → error

**Example**: Given dependencies `TASK-002 → [TASK-001]` and `TASK-003 → [TASK-001, TASK-002]`:
- Initial queue: `[TASK-001]` (no dependencies)
- Process TASK-001 → TASK-002's in-degree drops to 0 → enqueue TASK-002
- Process TASK-002 → TASK-003's in-degree drops to 0 → enqueue TASK-003
- Result: `[TASK-001, TASK-002, TASK-003]`

### `next_mergeable_tasks()`

Returns tasks whose dependencies are ALL in the `merged_tasks` list. These tasks can be merged in parallel. This enables the Overlord to merge independent tasks simultaneously.

### `execute_merge_sequence()`

Orchestrates the full merge process:

1. Call `next_mergeable_tasks()` to find ready tasks
2. For each ready task, call `merge_task_branch()` to merge into the plan branch
3. On success, move the task from `pending_tasks` to `merged_tasks`
4. Repeat until no tasks are mergeable or all are merged
5. If tasks remain unmerged (blocked dependencies), log a warning

### Usage Example

```rust
let mut plan = MergePlan {
    repo_root: PathBuf::from("/repo"),
    plan_branch: "feature/auth-overhaul".to_string(),
    pending_tasks: vec!["TASK-001".into(), "TASK-002".into(), "TASK-003".into()],
    merged_tasks: vec![],
    dependencies: {
        let mut m = HashMap::new();
        m.insert("TASK-002".into(), vec!["TASK-001".into()]);
        m.insert("TASK-003".into(), vec!["TASK-001".into(), "TASK-002".into()]);
        m
    },
};

let merged = execute_merge_sequence(&mut plan).await?;
// merged = ["TASK-001", "TASK-002", "TASK-003"]
```

---

## Usage

### Typical Overlord Workflow

```rust
use nexum::git::*;

// 1. Create plan branch
create_plan_branch(&repo_root, "feature/auth-overhaul", "main", true).await?;

// 2. For each task, set up workspace
for task_id in &tasks {
    let wt_path = setup_task_workspace(&repo_root, task_id, "feature/auth-overhaul").await?;
    // Give wt_path to builder agent
}

// 3. After tasks complete, merge in dependency order
let mut plan = MergePlan { /* ... */ };
let merged = execute_merge_sequence(&mut plan).await?;

// 4. Clean up each merged task
for task_id in &merged {
    cleanup_merged_task(&repo_root, task_id).await?;
}
```

### Direct Subprocess Usage

```rust
use nexum::git::{GitCommand, GitOutput};

let output: GitOutput = GitCommand::new(&repo_root)
    .args(&["log", "--oneline", "-5"])
    .timeout(std::time::Duration::from_secs(10))
    .execute()
    .await?;

println!("Last 5 commits:\n{}", output.stdout);
```

### Quick Git Command

```rust
use nexum::git::git;

let output = git(&repo_root, &["status", "--porcelain"]).await?;
println!("Repo status: {}", output.stdout);
```

## Configuration

- **Default timeout**: 30 seconds (`DEFAULT_TIMEOUT` in `subprocess.rs`), configurable per-command via `GitCommand::timeout()`.
- **Worktree directory**: `<repo_root>/.worktrees/` — should be added to `.gitignore`.
- **Git binary**: Must be available in `PATH`; otherwise `GitError::GitNotInstalled` is returned.
- **Environment variables**: Can be injected per-command via `GitCommand::env(key, val)` (e.g., `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`).
