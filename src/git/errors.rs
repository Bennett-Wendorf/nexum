//! Error types for the git operations module.
//!
//! This module defines [`GitError`] which covers all error cases that can
//! arise from spawning git subprocesses, capturing their output, and
//! enforcing execution timeouts.

use thiserror::Error;

/// Errors that can occur during git subprocess operations.
///
/// Each variant carries contextual information to help diagnose the problem,
/// such as command output, exit codes, or timeout durations.
#[derive(Debug, Error)]
pub enum GitError {
    /// The git subprocess failed with a non-zero exit code.
    #[error("git command failed with exit code {exit_code}: stdout={stdout}, stderr={stderr}")]
    SubprocessFailure {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },

    /// The git subprocess exceeded the configured timeout.
    #[error("git command timed out after {timeout:?}: {command}")]
    Timeout {
        timeout: std::time::Duration,
        command: String,
    },

    /// Failed to spawn the git subprocess.
    #[error("failed to spawn git subprocess: {0}")]
    SpawnFailed(#[source] std::io::Error),
}

/// Result type alias for git operations.
pub type Result<T> = std::result::Result<T, GitError>;
