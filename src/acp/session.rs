//! ACP session lifecycle manager.
//!
//! This module manages the full lifecycle of an ACP session: spawning the
//! agent subprocess, creating the session via JSON-RPC, running the event
//! loop, handling interactions, and cleaning up resources on destruction.

use std::path::Path;
use std::pin::Pin;
use std::time::{Duration, Instant};

use super::client::{ACPClient, MessageType, SessionCreateParams};
use super::errors::{ACPError, Result};
use super::events::{ACPEvent, EventStream, is_terminal_event, log_event, requires_response};
use super::subprocess::{AgentConfig, AgentProcess, spawn_agent};

#[derive(Debug, Clone, PartialEq)]
pub enum AgentRole {
    Builder,
    Reviewer,
    Planner,
    SecurityConsultant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Created,
    Interacting,
    Completing,
    Destroyed,
    Error,
}

/// A running ACP session managing an agent subprocess and its JSON-RPC communication.
pub struct ACPSession {
    /// Unique session identifier returned by the agent.
    pub session_id: String,
    /// The running agent subprocess.
    pub agent_process: AgentProcess,
    /// JSON-RPC client for sending requests to the agent.
    pub client: ACPClient,
    /// Current lifecycle state of the session.
    pub state: SessionState,
    /// ID of the task this session is working on.
    pub task_id: String,
    /// Role assigned to the agent.
    pub role: AgentRole,
    /// Timestamp when the session was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Maximum idle duration before the session times out.
    pub timeout: Duration,
    /// Last time an event was received from the agent.
    pub heartbeat: Instant,
    /// Background task reading stdout from the agent.
    reader_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ACPSession {
    /// Create a new ACP session by spawning an agent subprocess, initializing
    /// the protocol handshake, and creating a session via `sessions/create`.
    ///
    /// Events emitted during initialization are buffered in the mpsc channel
    /// and will be consumed by `run_event_loop()`.
    pub async fn create(
        task_id: &str,
        role: AgentRole,
        prompt: &str,
        worktree_path: &Path,
        agent_config: &AgentConfig,
        timeout: Duration,
        _event_stream: &EventStream,
    ) -> Result<Self> {
        // 1. Spawn agent subprocess
        let agent_id = &agent_config.name;
        let mut agent_process = spawn_agent(agent_id, agent_config, worktree_path)?;

        // 2. Create client (mpsc channel created internally)
        let stdin = agent_process.stdin.take().expect("stdin should be available");
        let client = ACPClient::new(agent_id.to_string(), stdin);

        // 3. Take stdout and spawn reader task
        let stdout = agent_process.stdout.take().expect("stdout should be available");
        let reader_handle = client.spawn_reader(stdout);

        // 4. Initialize the client (protocol handshake)
        client.initialize().await?;

        // 5. Create ACP session via sessions/create
        let params = SessionCreateParams {
            prompt: prompt.to_string(),
            working_directory: Some(worktree_path.to_string_lossy().to_string()),
            tool_permissions: None,
            timeout_seconds: Some(timeout.as_secs()),
        };
        let create_result = client.sessions_create(params).await?;

        // 6. Return the session
        // Events during initialize/sessions_create are buffered in the mpsc channel
        Ok(Self {
            session_id: create_result.session_id,
            agent_process,
            client,
            state: SessionState::Created,
            task_id: task_id.to_string(),
            role,
            created_at: chrono::Utc::now(),
            timeout,
            heartbeat: Instant::now(),
            reader_handle: Some(reader_handle),
        })
    }

    /// Send a follow-up message to the session.
    pub async fn send_message(&self, content: &str, message_type: MessageType) -> Result<()> {
        match self.state {
            SessionState::Created | SessionState::Interacting => {}
            _ => {
                return Err(ACPError::SessionNotActive {
                    session_id: self.session_id.clone(),
                })
            }
        }
        self.client
            .sessions_message(&self.session_id, content, Some(message_type))
            .await?;
        Ok(())
    }

    /// Respond to a permission request from the agent.
    pub async fn respond_to_permission(&self, request_id: &str, approved: bool) -> Result<()> {
        self.client.permission_respond(request_id, approved).await
    }

    /// Respond to a question from the agent.
    pub async fn respond_to_question(&self, question_id: &str, answer: &str) -> Result<()> {
        let payload = serde_json::json!({
            "questionId": question_id,
            "answer": answer,
        });
        let content = serde_json::to_string(&payload).map_err(|e| ACPError::JsonRpcTransport {
            source: Box::new(e),
        })?;
        self.client
            .sessions_message(&self.session_id, &content, Some(MessageType::FollowUp))
            .await
    }

    /// Check whether the session heartbeat is stale beyond the given threshold.
    pub fn check_heartbeat(&self, max_stale: Duration) -> bool {
        self.heartbeat.elapsed() > max_stale
    }

    /// Signal that the session is completing.
    pub async fn complete(&mut self) -> Result<()> {
        self.state = SessionState::Completing;
        tracing::info!(session_id = %self.session_id, "Session completing");
        Ok(())
    }

    /// Destroy the session: send destroy request, shutdown subprocess, cancel reader.
    pub async fn destroy(&mut self) -> Result<()> {
        let _ = self.client.sessions_destroy(&self.session_id).await;
        self.state = SessionState::Destroyed;
        self.agent_process.shutdown().await?;
        if let Some(handle) = self.reader_handle.take() {
            handle.abort();
        }
        tracing::info!(session_id = %self.session_id, "Session destroyed");
        Ok(())
    }

    /// Run the event loop, processing events from the agent subprocess.
    ///
    /// Uses a resettable idle timeout that resets on each received event.
    /// Returns when a terminal event is received, a response-requiring event
    /// is received, or an error occurs (subprocess crash, timeout).
    pub async fn run_event_loop(&mut self, event_stream: &EventStream) -> Result<Option<ACPEvent>> {
        // Take ownership of the mpsc receiver from the client
        let mut rx = self
            .client
            .take_event_receiver()
            .expect("event receiver should be available");

        // Create a pinned, resettable idle timeout
        let mut timeout_sleep: Pin<Box<tokio::time::Sleep>> =
            Box::pin(tokio::time::sleep(self.timeout));

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Some(event) => {
                            // Publish to central event stream
                            event_stream.publish(event.clone())?;

                            // Update heartbeat
                            self.heartbeat = Instant::now();

                            // Log the event
                            log_event(&event);

                            // Reset the idle timeout on each event
                            let wake_at = tokio::time::Instant::now() + self.timeout;
                            timeout_sleep.as_mut().reset(wake_at);

                            // Check if terminal or requires response
                            if is_terminal_event(&event) {
                                return Ok(Some(event));
                            }
                            if requires_response(&event) {
                                return Ok(Some(event));
                            }
                        }
                        None => {
                            // mpsc channel closed — subprocess crashed
                            return Err(ACPError::SubprocessCrash {
                                agent_id: self.agent_process.agent_id.clone(),
                                exit_code: None,
                            });
                        }
                    }
                }
                // Check for idle timeout
                _ = &mut timeout_sleep => {
                    self.state = SessionState::Error;
                    return Err(ACPError::SessionTimeout {
                        session_id: self.session_id.clone(),
                        duration: self.timeout,
                    });
                }
            }
        }
    }
}
