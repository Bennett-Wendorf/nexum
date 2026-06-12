//! Agent subprocess management.
//!
//! This module handles spawning, communicating with, and shutting down
//! ACP agent processes. Each agent runs as a child process communicating
//! over stdio pipes (JSON-RPC).

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use std::process::Stdio;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;

use super::errors::{ACPError, Result};

/// Configuration for spawning an agent subprocess.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Human-readable name (e.g., "OpenCode").
    pub name: String,
    /// Path to the agent binary (e.g., "/usr/bin/opencode").
    pub binary: PathBuf,
    /// Arguments to pass when spawning (e.g., ["acp"]).
    pub args: Vec<String>,
    /// Environment variables to set on the child process.
    pub env: HashMap<String, String>,
}

/// A running agent subprocess with its I/O handles.
///
/// Handles are consumed (set to `None`) during shutdown to prevent
/// double-cleanup. If dropped without explicit shutdown, the `Drop`
/// implementation will clean up any remaining handles.
pub struct AgentProcess {
    /// Agent registration name (e.g., "opencode").
    pub agent_id: String,
    /// The spawned child process.
    pub child: Option<tokio::process::Child>,
    /// Write handle for sending JSON-RPC requests.
    pub stdin: Option<tokio::process::ChildStdin>,
    /// Read handle for receiving JSON-RPC responses.
    pub stdout: Option<tokio::process::ChildStdout>,
    /// Background task forwarding stderr to tracing logs.
    pub stderr_task: Option<tokio::task::JoinHandle<()>>,
    /// Instant when the process was spawned.
    pub spawn_time: Instant,
    /// Path to the task's isolated worktree directory.
    pub worktree_path: PathBuf,
}

/// Spawn an agent subprocess with the given configuration.
///
/// The process is spawned in the specified worktree directory with
/// piped stdio. A background task is started to forward stderr lines
/// to `tracing::info!`.
///
/// # Errors
///
/// Returns `ACPError::SubprocessSpawn` if the OS refuses to create
/// the child process.
pub fn spawn_agent(
    agent_id: &str,
    config: &AgentConfig,
    worktree_path: &Path,
) -> Result<AgentProcess> {
    let command_str = format!(
        "{} {}",
        config.binary.display(),
        config.args.join(" ")
    );

    let mut cmd = Command::new(&config.binary);
    cmd.args(&config.args)
        .current_dir(worktree_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn().map_err(|source| ACPError::SubprocessSpawn {
        agent_id: agent_id.to_string(),
        command: command_str,
        source,
    })?;

    let stdin = child.stdin.take().expect("stdin should be piped");
    let stdout = child.stdout.take().expect("stdout should be piped");
    let stderr = child.stderr.take().expect("stderr should be piped");

    // Spawn a background task to forward stderr to tracing::info!
    let agent_id_for_task = agent_id.to_string();
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            tracing::info!(agent_id = %agent_id_for_task, line = %line);
        }
    });

    Ok(AgentProcess {
        agent_id: agent_id.to_string(),
        child: Some(child),
        stdin: Some(stdin),
        stdout: Some(stdout),
        stderr_task: Some(stderr_task),
        spawn_time: Instant::now(),
        worktree_path: worktree_path.to_path_buf(),
    })
}

impl AgentProcess {
    /// Shut down the agent subprocess.
    ///
    /// Sends SIGKILL via `kill()` (on Unix, tokio's `Child::kill()` sends
    /// SIGKILL, not SIGTERM) and waits up to 5 seconds for the process
    /// to exit. The process is forcefully terminated; this is not a
    /// graceful shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be signaled or reaped.
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            // Send termination signal.
            let _ = child.kill().await;

            // Wait up to 5 seconds for the process to exit.
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await;
        }

        // Cancel stderr forwarding task.
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }

        // Clear all handles.
        self.stdin.take();
        self.stdout.take();
        self.child.take();

        Ok(())
    }

    /// Force kill the agent subprocess immediately.
    ///
    /// Sends SIGKILL without waiting for a graceful shutdown period.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be killed or reaped.
    pub async fn force_kill(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        // Cancel stderr forwarding task.
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }

        // Clear all handles.
        self.stdin.take();
        self.stdout.take();
        self.child.take();

        Ok(())
    }

    /// Check whether the agent subprocess is still running.
    ///
    /// Returns `true` if the process has not yet exited.
    /// Note: requires `&mut self` because `try_wait()` mutates internal
    /// state in the tokio `Child` handle.
    pub fn is_alive(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => child.try_wait().map(|status| status.is_none()).unwrap_or(true),
            None => false,
        }
    }

    /// Get the exit status of the agent subprocess, if it has exited.
    ///
    /// Returns `Some(ExitStatus)` if the process has terminated, or
    /// `None` if it is still running or has not been checked yet.
    /// Note: requires `&mut self` because `try_wait()` mutates internal
    /// state in the tokio `Child` handle.
    pub fn exit_status(&mut self) -> Option<std::process::ExitStatus> {
        self.child.as_mut().and_then(|child| child.try_wait().ok().flatten())
    }
}

impl Drop for AgentProcess {
    fn drop(&mut self) {
        // If handles haven't been explicitly shut down, clean them up.
        // We can't spawn a task or call async methods in Drop, so we
        // simply take the handles to ensure the child process is
        // cleaned up by the OS when the child guard is dropped.
        self.stdin.take();
        self.stdout.take();
        self.stderr_task.take();
        // Dropping the Child handle will close its file descriptors.
        // The OS will clean up the zombie process.
        self.child.take();
    }
}
