//! ACP session lifecycle management for builder tasks.
//!
//! This module manages the complete lifecycle of ACP agent sessions:
//! create → run → interact → complete → destroy.
//!
//! See `design/agent-harness-integration.md` for the full design.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::broadcast;

use crate::acp::events::{ACPEvent, CompletionStatus, EventStream};
use crate::acp::subprocess::{spawn_agent, AgentConfig, AgentProcess};
use crate::builder::dispatcher::TaskContext;
use crate::builder::errors::Result;
use crate::builder::event_bus::CompletionResult;

/// Handle to an active ACP session.
pub struct ACPSessionHandle {
    /// Unique session identifier.
    pub session_id: String,
    /// The agent subprocess.
    pub subprocess: AgentProcess,
    /// Event stream for this session.
    pub event_stream: EventStream,
    /// Timestamp when the session was created.
    pub started_at: DateTime<Utc>,
}

impl std::fmt::Debug for ACPSessionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ACPSessionHandle")
            .field("session_id", &self.session_id)
            .field("subprocess", &self.subprocess.agent_id)
            .field("started_at", &self.started_at)
            .finish()
    }
}

/// Manages ACP session lifecycle for builder tasks.
pub struct SessionManager {
    /// Repository root path.
    repo_root: PathBuf,
    /// Agent configuration for spawning subprocesses.
    agent_config: AgentConfig,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(repo_root: PathBuf, agent_config: AgentConfig) -> Self {
        Self { repo_root, agent_config }
    }

    /// Create an ACP session for a task.
    ///
    /// # Steps
    /// 1. Generate a unique session ID from task_id and current timestamp
    /// 2. Spawn agent subprocess in the worktree directory
    /// 3. Initialize event stream
    /// 4. Publish initial progress event
    /// 5. Return ACPSessionHandle
    pub async fn create_session(
        &self,
        task_context: &TaskContext,
        worktree_path: &std::path::Path,
    ) -> Result<ACPSessionHandle> {
        // Generate session ID
        let session_id = format!(
            "session-{}-{}",
            task_context.task_id,
            Utc::now().timestamp_millis()
        );

        // Spawn agent subprocess in worktree directory
        let subprocess = spawn_agent(&session_id, &self.agent_config, worktree_path)?;

        // Create event stream
        let event_stream = EventStream::with_default_capacity();

        // Publish initial progress event
        let _ = event_stream.publish(ACPEvent::Progress {
            session_id: session_id.clone(),
            message: format!("Task {} started", task_context.task_id),
            percentage: Some(0.0),
            timestamp: Utc::now().to_rfc3339(),
        });

        tracing::info!(
            session_id = %session_id,
            task_id = %task_context.task_id,
            "ACP session created"
        );

        Ok(ACPSessionHandle {
            session_id,
            subprocess,
            event_stream,
            started_at: Utc::now(),
        })
    }

    /// Get event receiver for a session.
    pub fn get_event_stream(&self, handle: &ACPSessionHandle) -> broadcast::Receiver<ACPEvent> {
        handle.event_stream.subscribe()
    }

    /// Send a follow-up message to the agent.
    ///
    /// Publishes the message as a `Progress` event on the session's event stream.
    pub async fn send_message(&self, handle: &ACPSessionHandle, message: &str) -> Result<()> {
        handle.event_stream.publish(ACPEvent::Progress {
            session_id: handle.session_id.clone(),
            message: message.to_string(),
            percentage: None,
            timestamp: Utc::now().to_rfc3339(),
        })?;

        Ok(())
    }

    /// Respond to a permission request from the agent.
    pub async fn handle_permission_request(
        &self,
        handle: &ACPSessionHandle,
        permission_id: &str,
        approve: bool,
    ) -> Result<()> {
        if approve {
            tracing::info!(
                session_id = %handle.session_id,
                permission_id = %permission_id,
                "Permission approved"
            );
        } else {
            tracing::warn!(
                session_id = %handle.session_id,
                permission_id = %permission_id,
                "Permission denied"
            );
        }

        Ok(())
    }

    /// Destroy an ACP session.
    ///
    /// Consumes the handle to gain ownership of the subprocess for cleanup.
    ///
    /// # Steps
    /// 1. Move the subprocess out of the handle
    /// 2. Call `shutdown()` on the subprocess (SIGKILL + 5s wait)
    /// 3. Resources are cleaned up by `AgentProcess::shutdown`
    pub async fn destroy_session(&self, handle: ACPSessionHandle) -> Result<()> {
        tracing::info!(
            session_id = %handle.session_id,
            "Destroying ACP session"
        );

        // Take ownership of the subprocess and shut it down
        let mut subprocess = handle.subprocess;
        subprocess.shutdown().await?;

        Ok(())
    }

    /// Check if the session has completed.
    ///
    /// Note: `AgentProcess::is_alive()` requires `&mut self`, so with only a
    /// shared reference to the handle we cannot check subprocess state directly.
    /// In practice, completion is detected via [`wait_for_completion`] monitoring
    /// the event stream for terminal events.
    pub fn is_completed(&self, _handle: &ACPSessionHandle) -> bool {
        false
    }

    /// Wait for session completion with timeout.
    ///
    /// # Steps
    /// 1. Subscribe to the session's event stream
    /// 2. Monitor for `Completion` or `Error` events
    /// 3. If timeout elapsed first, return `CompletionResult::Timeout`
    /// 4. If agent crashes (stream closed), return `CompletionResult::Crashed`
    /// 5. If completion event received, return appropriate result
    pub async fn wait_for_completion(
        &self,
        handle: &ACPSessionHandle,
        timeout: Duration,
    ) -> Result<CompletionResult> {
        let mut event_rx = handle.event_stream.subscribe();

        let result = tokio::time::timeout(timeout, async {
            loop {
                match event_rx.recv().await {
                    Ok(event) => match &event {
                        ACPEvent::Completion { status, .. } => match status {
                            CompletionStatus::Success => {
                                return CompletionResult::Completed;
                            }
                            CompletionStatus::Failed | CompletionStatus::Cancelled => {
                                return CompletionResult::Crashed { exit_code: None };
                            }
                        },
                        ACPEvent::Error { .. } => {
                            return CompletionResult::Crashed { exit_code: None };
                        }
                        _ => {
                            // Continue monitoring
                        }
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            session_id = %handle.session_id,
                            lagged = n,
                            "Event stream lagged during completion wait"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return CompletionResult::Crashed { exit_code: None };
                    }
                }
            }
        })
        .await;

        match result {
            Ok(completion) => Ok(completion),
            Err(_) => Ok(CompletionResult::Timeout(timeout)),
        }
    }
}
