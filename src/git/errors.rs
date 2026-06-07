//! Error types for the git operations module.
//!
//! This module defines [`GitError`] which covers all error cases that can
//! arise from git subprocess execution, branch operations, worktree
//! lifecycle management, and merge coordination.

use std::io;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during git operations.
///
/// Each variant carries contextual information to help diagnose the problem.
#[derive(Debug, Error)]
pub enum GitError {
    /// The git subprocess failed with a non-zero exit code.
    #[error("git command failed (exit {exit_code}): {command}\nstdout: {stdout}\nstderr: {stderr}")]
    SubprocessFailure {
        command: String,
        exit_code: i32,
        stdout: String,
        stderr: String,
    },

    /// The git subprocess exceeded the configured timeout.
    #[error("git command timed out after {duration:?}: {command}")]
    Timeout {
        command: String,
        duration: Duration,
    },

    /// Merge produced conflicts that require resolution.
    #[error("merge conflict between '{branch}' and '{plan_branch}': conflicts in {conflicts:?}")]
    MergeConflict {
        branch: String,
        plan_branch: String,
        conflicts: Vec<String>,
    },

    /// Referenced branch does not exist.
    #[error("branch not found: {branch}")]
    BranchNotFound {
        branch: String,
    },

    /// Branch already exists (unexpected).
    #[error("branch already exists: {branch}")]
    BranchExists {
        branch: String,
    },

    /// Expected worktree does not exist at the given path.
    #[error("worktree not found at: {path}")]
    WorktreeNotFound {
        path: PathBuf,
    },

    /// Worktree already exists at the given path.
    #[error("worktree already exists at: {path}")]
    WorktreeExists {
        path: PathBuf,
    },

    /// Worktree is stale (lock file present).
    #[error("worktree is stale (lock file present): {path}")]
    WorktreeStale {
        path: PathBuf,
    },

    /// git binary not found in PATH.
    #[error("git binary not found in PATH")]
    GitNotInstalled,

    /// File system I/O error during worktree path operations.
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Dependency tasks have not yet been merged.
    #[error("task {task_id} cannot merge: unmet dependencies {unmet_deps:?}")]
    DependencyNotMet {
        task_id: String,
        unmet_deps: Vec<String>,
    },

    /// Failed to spawn the git subprocess.
    #[error("failed to spawn git subprocess: {0}")]
    SpawnFailed(#[source] io::Error),
}

/// Result type alias for git operations.
pub type Result<T> = std::result::Result<T, GitError>;
