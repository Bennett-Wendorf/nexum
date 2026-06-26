//! Builder workflow module.
//!
//! This module orchestrates the end-to-end builder workflow: dispatching
//! tasks to isolated worktrees, running agent sessions, collecting results,
//! and merging completed task branches back into the plan branch.

pub mod dispatcher;
pub mod error_recovery;
pub mod errors;
pub mod event_bus;
pub mod heartbeat;
pub mod merge_coordinator;
pub mod session_manager;
pub mod worktree_manager;
