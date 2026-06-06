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

pub use errors::{GitError, Result};
pub use subprocess::{GitCommand, GitOutput};
pub use worktree::WorktreeInfo;

// ── Re-exported functions: Branch operations ─────────────────────────────────

pub use branch::create_branch;
pub use branch::create_plan_branch;
pub use branch::create_task_branch;
pub use branch::delete_branch;
pub use branch::checkout_branch;
pub use branch::list_local_branches;
pub use branch::branch_exists;
pub use branch::current_branch;
pub use branch::task_branch_name;
pub use branch::is_task_branch;

// ── Re-exported functions: Worktree operations ───────────────────────────────

pub use worktree::spawn_worktree;
pub use worktree::remove_worktree;
pub use worktree::remove_worktree_force;
pub use worktree::list_worktrees;
pub use worktree::worktree_exists;
pub use worktree::worktree_path;
pub use worktree::worktrees_dir;

// ── Re-exported functions: Merge operations ──────────────────────────────────

pub use merge::merge_branch;
pub use merge::merge_task_branch;
pub use merge::abort_merge;
pub use merge::has_merge_conflicts;
pub use merge::list_conflicted_files;
pub use merge::is_merging;

// ── Re-exported functions: Coordination ──────────────────────────────────────

pub use merge::determine_merge_order;
pub use merge::next_mergeable_tasks;
pub use merge::execute_merge_sequence;
pub use merge::cleanup_merged_task;

// ── Re-exported functions: Workspace ─────────────────────────────────────────

pub use worktree::setup_task_workspace;
pub use worktree::teardown_task_workspace;

// ── Re-exported functions: Subprocess ────────────────────────────────────────

pub use subprocess::git;

// ── Re-exported types: Merge ─────────────────────────────────────────────────

pub use merge::MergePlan;
