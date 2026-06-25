//! Error types for the Builder Workflow module.
//!
//! This module defines [`BuilderError`] which wraps all error types from
//! the dependency modules (git, acp, persistence, overlord) and adds
//! builder-specific error variants for workflow failures, merge conflicts,
//! timeouts, agent crashes, and permission denials.

use std::time::Duration;

use thiserror::Error;

use crate::acp::ACPError;
use crate::git::GitError;
use crate::overlord::OverlordError;
use crate::persistence::PersistenceError;

/// Errors that can occur during builder workflow operations.
///
/// Each variant carries contextual information to help diagnose the problem.
/// Wrapped errors use `#[from]` for ergonomic `?` operator compatibility.
#[derive(Debug, Error)]
pub enum BuilderError {
    /// Failed to dispatch a task to a builder agent.
    #[error("Task dispatch error: {0}")]
    TaskDispatchError(String),

    /// Wrapped git worktree error.
    #[error("Worktree error: {0}")]
    WorktreeError(#[from] GitError),

    /// Wrapped ACP session error.
    #[error("Session error: {0}")]
    SessionError(#[from] ACPError),

    /// Wrapped git merge error.
    #[error("Merge error: {0}")]
    MergeError(#[from] GitError),

    /// Merge conflict detected during task branch merge.
    #[error("Merge conflict for task '{task_id}' on branch '{branch}': conflicts in {conflicts:?}")]
    MergeConflict {
        task_id: String,
        branch: String,
        conflicts: Vec<String>,
    },

    /// Task exceeded its configured timeout.
    #[error("Task '{task_id}' timed out: elapsed {elapsed:?}, limit {limit:?}")]
    TimeoutError {
        task_id: String,
        elapsed: Duration,
        limit: Duration,
    },

    /// Agent subprocess crashed unexpectedly.
    #[error("Agent crashed for task '{task_id}': exit_code={exit_code:?}")]
    AgentCrash {
        task_id: String,
        exit_code: Option<i32>,
    },

    /// Wrapped persistence layer error.
    #[error("Persistence error: {0}")]
    PersistenceError(#[from] PersistenceError),

    /// Wrapped overlord error.
    #[error("Overlord error: {0}")]
    OverlordError(#[from] OverlordError),

    /// Agent permission request was denied.
    #[error("Permission denied for task '{task_id}': {resource}")]
    PermissionDenied {
        task_id: String,
        resource: String,
    },

    /// General workflow error.
    #[error("Workflow error: {0}")]
    WorkflowError(String),
}

/// Result type alias for builder operations.
pub type Result<T> = std::result::Result<T, BuilderError>;
