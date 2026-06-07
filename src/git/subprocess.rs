//! Git subprocess wrapper.
//!
//! This module provides the sole mechanism for interacting with git within
//! the Nexum codebase. All git operations must go through this wrapper to
//! ensure consistent error handling, timeout enforcement, and output capture.
//!
//! # Usage
//!
//! Use the [`GitCommand`] builder for full control:
//!
//! ```rust,ignore
//! let output = GitCommand::new(&repo_root)
//!     .args(&["status", "--porcelain"])
//!     .timeout(Duration::from_secs(10))
//!     .execute()
//!     .await?;
//! ```
//!
//! Or use the [`git`] convenience function for simple commands:
//!
//! ```rust,ignore
//! let output = git(&repo_root, &["status", "--porcelain"]).await?;
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use super::errors::{GitError, Result};

/// Default timeout for git subprocess operations (30 seconds).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Builder for constructing and executing git commands.
///
/// Uses the builder pattern to accumulate command arguments, environment
/// variables, and timeout settings before executing the git subprocess.
///
/// # Example
///
/// ```rust,ignore
/// let output = GitCommand::new(&repo_root)
///     .args(&["log", "--oneline", "-5"])
///     .env("GIT_AUTHOR_NAME", "Nexum")
///     .timeout(Duration::from_secs(30))
///     .execute()
///     .await?;
/// ```
pub struct GitCommand {
    /// The root directory of the git repository (used as working directory).
    repo_root: std::path::PathBuf,

    /// Git command arguments (e.g., `["status", "--porcelain"]`).
    args: Vec<String>,

    /// Optional execution timeout. When `None`, no timeout is enforced.
    timeout: Option<Duration>,

    /// Environment variables to set on the subprocess.
    env_vars: HashMap<String, String>,
}

impl GitCommand {
    /// Create a new `GitCommand` builder targeting the given repository root.
    ///
    /// The `repo_root` path is used as the working directory for the git
    /// subprocess.
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            args: Vec::new(),
            timeout: None,
            env_vars: HashMap::new(),
        }
    }

    /// Add multiple command arguments.
    ///
    /// Arguments are appended to any previously added arguments.
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args
            .extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Add a single command argument.
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Set an optional execution timeout.
    ///
    /// When a timeout is configured, [`GitCommand::execute`] will use
    /// `tokio::time::timeout` to enforce the limit. If the git subprocess
    /// does not complete within the specified duration, a [`GitError::Timeout`]
    /// is returned.
    ///
    /// By default, no timeout is enforced.
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Set an environment variable on the subprocess.
    ///
    /// Variables set here are applied to the git subprocess in addition
    /// to the inherited environment.
    pub fn env(mut self, key: &str, val: &str) -> Self {
        self.env_vars.insert(key.to_string(), val.to_string());
        self
    }

    /// Execute the git command and capture its output.
    ///
    /// Returns [`GitOutput`] on success (exit code 0), or a [`GitError`]
    /// on failure, timeout, or spawn error.
    ///
    /// # Errors
    ///
    /// - [`GitError::SpawnFailed`] if the subprocess could not be spawned.
    /// - [`GitError::Timeout`] if the command exceeded the configured timeout.
    /// - [`GitError::SubprocessFailure`] if git exited with a non-zero code.
    pub async fn execute(self) -> Result<GitOutput> {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.repo_root);
        cmd.args(&self.args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        for (key, val) in &self.env_vars {
            cmd.env(key, val);
        }

        let command_desc = format!(
            "git {}",
            self.args.join(" ")
        );

        let child = cmd.spawn().map_err(GitError::SpawnFailed)?;
        let mut child = tokio::process::Child::from(child);

        let output = if let Some(timeout_dur) = self.timeout {
            // Spawn the wait in a separate task so we can kill the child
            // on timeout without ownership conflicts.
            let (tx, rx) = tokio::sync::oneshot::channel();
            let wait_handle = tokio::spawn(async move {
                let _ = tx.send(child.wait_with_output().await);
            });

            match tokio::time::timeout(timeout_dur, rx).await {
                Ok(Ok(Ok(output))) => output,
                Ok(Ok(Err(e))) => return Err(GitError::SpawnFailed(e)),
                Ok(Err(_)) => {
                    // Channel closed unexpectedly; abort the wait task
                    wait_handle.abort();
                    return Err(GitError::SpawnFailed(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "wait channel closed unexpectedly",
                    )));
                }
                Err(_) => {
                    // Timeout — abort the wait task (which drops the child,
                    // terminating the git subprocess)
                    wait_handle.abort();
                    return Err(GitError::Timeout {
                        duration: timeout_dur,
                        command: command_desc,
                    });
                }
            }
        } else {
            child.wait_with_output().await.map_err(GitError::SpawnFailed)?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if exit_code != 0 {
            return Err(GitError::SubprocessFailure {
                command: command_desc,
                stdout,
                stderr,
                exit_code,
            });
        }

        Ok(GitOutput {
            stdout,
            stderr,
            exit_code,
        })
    }
}

/// Output captured from a git subprocess.
pub struct GitOutput {
    /// Standard output from the git command.
    pub stdout: String,

    /// Standard error from the git command.
    pub stderr: String,

    /// Exit code of the git subprocess.
    pub exit_code: i32,
}

impl GitOutput {
    /// Returns `true` if the command succeeded (exit code == 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Convenience function for executing simple git commands.
///
/// Spawns a git subprocess in the given repository root with the specified
/// arguments, using a default 30-second timeout.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `args` - Git command arguments (e.g., `["status", "--porcelain"]`).
///
/// # Example
///
/// ```rust,ignore
/// let output = git(&repo_root, &["rev-parse", "HEAD"]).await?;
/// println!("Current commit: {}", output.stdout.trim());
/// ```
pub async fn git(repo_root: &Path, args: &[&str]) -> Result<GitOutput> {
    GitCommand::new(repo_root)
        .args(args)
        .timeout(DEFAULT_TIMEOUT)
        .execute()
        .await
}
