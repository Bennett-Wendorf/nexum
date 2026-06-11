//! Git operations module.
//!
//! This module provides abstractions for interacting with git repositories
//! through subprocess management. All git interactions within Nexum flow
//! through this module, ensuring consistent error handling and timeout
//! enforcement.
//!
//! # Branch Strategy
//!
//! The branch strategy is defined in [`design/persistence.md`](../../design/persistence.md)
//! (section "4. Branch strategy"):
//!
//! - Each plan has one branch (e.g., `feature/auth-overhaul`) — this is the
//!   source of truth for the plan.
//! - Each task gets its own **local branch + worktree** (`task/TASK-001`)
//!   spawned from the plan's branch.
//! - Tasks work in complete isolation — no cross-talk between worktrees.
//! - When a task finishes, the overlord merges its branch into the plan's
//!   branch.
//! - The dependency graph dictates merge order — `TASK-003` can't merge until
//!   `TASK-001` and `TASK-002` are merged.
//! - After merge, the task branch is deleted and the worktree is cleaned up.
//! - **Task branches are never pushed to remote** — they are a local, ephemeral
//!   concept for parallelization only.

mod branch;
mod errors;
mod merge;
mod subprocess;
mod worktree;

// ── Re-exported types ────────────────────────────────────────────────────────
#[allow(unused_imports)]
pub use errors::{GitError, Result};
#[allow(unused_imports)]
pub use subprocess::{GitCommand, GitOutput};
#[allow(unused_imports)]
pub use worktree::WorktreeInfo;

// ── Re-exported functions: Branch operations ─────────────────────────────────
#[allow(unused_imports)]
pub use branch::{
    branch_exists, checkout_branch, create_branch, create_plan_branch, create_task_branch,
    current_branch, delete_branch, is_task_branch, list_local_branches, task_branch_name,
};

// ── Re-exported functions: Worktree operations ───────────────────────────────
#[allow(unused_imports)]
pub use worktree::{
    list_worktrees, remove_worktree, remove_worktree_force, spawn_worktree, worktree_exists,
    worktree_path, worktrees_dir,
};

// ── Re-exported functions: Merge operations ──────────────────────────────────
#[allow(unused_imports)]
pub use merge::{
    abort_merge, has_merge_conflicts, is_merging, list_conflicted_files, merge_branch,
    merge_task_branch,
};

// ── Re-exported functions: Coordination ──────────────────────────────────────
#[allow(unused_imports)]
pub use merge::{
    cleanup_merged_task, determine_merge_order, execute_merge_sequence, next_mergeable_tasks,
};

// ── Re-exported functions: Workspace ─────────────────────────────────────────
#[allow(unused_imports)]
pub use branch::{setup_task_workspace, teardown_task_workspace};

// ── Re-exported functions: Subprocess ────────────────────────────────────────
#[allow(unused_imports)]
pub use subprocess::git;

// ── Re-exported types: Merge ─────────────────────────────────────────────────
#[allow(unused_imports)]
pub use merge::MergePlan;

#[cfg(test)]
mod tests;
