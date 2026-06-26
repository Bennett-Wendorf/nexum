//! Builder workflow module.
//!
//! This module orchestrates the end-to-end builder workflow: dispatching
//! tasks to isolated worktrees, running agent sessions, collecting results,
//! and merging completed task branches back into the plan branch.

pub mod dispatcher;
pub mod errors;
pub mod worktree_manager;
