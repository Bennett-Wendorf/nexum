//! ACP event types and streaming.
//!
//! This module defines the event types used by the Agent Communication Protocol
//! for streaming real-time updates from agent subprocesses, as well as the
//! `EventStream` broadcast channel that delivers those events to subscribers.

use super::errors::Result;

/// Status of a completed ACP task.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionStatus {
    /// The task completed successfully.
    Success,
    /// The task failed with an error.
    Failed,
    /// The task was cancelled before completion.
    Cancelled,
}

/// All ACP event types streamed from agent subprocesses.
///
/// Each event carries a `session_id` and an ISO 8601 timestamp so that
/// consumers can correlate events and determine their chronological order.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ACPEvent {
    /// Periodic progress update from the agent.
    Progress {
        session_id: String,
        message: String,
        percentage: Option<f32>,
        timestamp: String,
    },
    /// The agent invoked a tool.
    ToolCall {
        session_id: String,
        tool_name: String,
        arguments: serde_json::Value,
        call_id: String,
        timestamp: String,
    },
    /// Result from a tool invocation.
    ToolResult {
        session_id: String,
        call_id: String,
        success: bool,
        output: Option<String>,
        error: Option<String>,
        timestamp: String,
    },
    /// The agent is asking a question that requires user input.
    Question {
        session_id: String,
        question_id: String,
        content: String,
        options: Option<Vec<String>>,
        timestamp: String,
    },
    /// The agent requires permission before performing an action.
    PermissionRequest {
        session_id: String,
        request_id: String,
        /// Action type, e.g. `"file-write"`, `"command-execution"`, `"network-request"`.
        action: String,
        details: serde_json::Value,
        timestamp: String,
    },
    /// The agent task has completed (successfully, failed, or cancelled).
    Completion {
        session_id: String,
        status: CompletionStatus,
        summary: Option<String>,
        artifacts: Option<Vec<String>>,
        timestamp: String,
    },
    /// An error occurred during agent execution.
    Error {
        session_id: String,
        error_code: String,
        message: String,
        timestamp: String,
    },
}

/// Manages event delivery via a [`tokio::sync::broadcast`] channel.
///
/// Multiple subscribers can listen for events. Lagging subscribers that fall
/// behind the channel buffer receive
/// [`tokio::sync::broadcast::error::RecvError::Lagged`] and should decide
/// whether to skip missed events or disconnect.
///
/// Default channel capacity is **1024** events.
pub struct EventStream {
    sender: tokio::sync::broadcast::Sender<ACPEvent>,
}

impl EventStream {
    /// Create a new `EventStream` with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }

    /// Create a new `EventStream` with the default capacity of 1024.
    pub fn with_default_capacity() -> Self {
        Self::new(1024)
    }

    /// Subscribe to the event stream.
    ///
    /// Returns a receiver that yields `ACPEvent` values. Callers should
    /// handle [`tokio::sync::broadcast::error::RecvError::Lagged`] to
    /// gracefully recover from missed events.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ACPEvent> {
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers.
    ///
    /// Returns an error if all subscribers have dropped (no active listeners).
    pub fn publish(&self, event: ACPEvent) -> Result<()> {
        self.sender
            .send(event)
            .map(|_| ())
            .map_err(|e| super::errors::ACPError::EventStreamError {
                source: Box::new(e),
            })
    }

    /// Get a clone of the broadcast sender.
    ///
    /// Useful for registering the session with external event buses
    /// that need their own sender reference.
    pub fn sender(&self) -> tokio::sync::broadcast::Sender<ACPEvent> {
        self.sender.clone()
    }
}

/// Returns `true` if the event signals the end of a session.
///
/// Terminal events are `Completion` and `Error`.
pub fn is_terminal_event(event: &ACPEvent) -> bool {
    matches!(event, ACPEvent::Completion { .. } | ACPEvent::Error { .. })
}

/// Returns `true` if the event requires a response from the client.
///
/// Events that require a response are `PermissionRequest` and `Question`.
pub fn requires_response(event: &ACPEvent) -> bool {
    matches!(
        event,
        ACPEvent::PermissionRequest { .. } | ACPEvent::Question { .. }
    )
}

/// Extract the session ID from any ACP event variant.
///
/// All ACP events carry a `session_id` field, so this function covers every
/// variant of the enum.
pub fn session_id(event: &ACPEvent) -> &str {
    match event {
        ACPEvent::Progress { session_id, .. } => session_id,
        ACPEvent::ToolCall { session_id, .. } => session_id,
        ACPEvent::ToolResult { session_id, .. } => session_id,
        ACPEvent::Question { session_id, .. } => session_id,
        ACPEvent::PermissionRequest { session_id, .. } => session_id,
        ACPEvent::Completion { session_id, .. } => session_id,
        ACPEvent::Error { session_id, .. } => session_id,
    }
}

/// Log an ACP event using the `tracing` crate at the appropriate level.
///
/// | Event              | Level   |
/// |--------------------|---------|
/// | Progress           | `info`  |
/// | ToolCall           | `debug` |
/// | ToolResult         | `debug` |
/// | Question           | `info`  |
/// | PermissionRequest  | `info`  |
/// | Completion         | `info`  |
/// | Error              | `error` |
pub fn log_event(event: &ACPEvent) {
    match event {
        ACPEvent::Progress {
            session_id,
            message,
            percentage,
            ..
        } => {
            tracing::info!(
                session_id = %session_id,
                message = %message,
                percentage = percentage,
                "ACP progress update"
            );
        }
        ACPEvent::ToolCall {
            session_id,
            tool_name,
            arguments,
            call_id,
            ..
        } => {
            tracing::debug!(
                session_id = %session_id,
                tool_name = %tool_name,
                call_id = %call_id,
                arguments = ?arguments,
                "ACP tool call"
            );
        }
        ACPEvent::ToolResult {
            session_id,
            call_id,
            success,
            output,
            error,
            ..
        } => {
            tracing::debug!(
                session_id = %session_id,
                call_id = %call_id,
                success = success,
                output = output,
                error = error,
                "ACP tool result"
            );
        }
        ACPEvent::Question {
            session_id,
            question_id,
            content,
            ..
        } => {
            tracing::info!(
                session_id = %session_id,
                question_id = %question_id,
                content = %content,
                "ACP question"
            );
        }
        ACPEvent::PermissionRequest {
            session_id,
            request_id,
            action,
            ..
        } => {
            tracing::info!(
                session_id = %session_id,
                request_id = %request_id,
                action = %action,
                "ACP permission request"
            );
        }
        ACPEvent::Completion {
            session_id,
            status,
            summary,
            artifacts,
            ..
        } => {
            tracing::info!(
                session_id = %session_id,
                status = ?status,
                summary = summary,
                artifacts = ?artifacts,
                "ACP completion"
            );
        }
        ACPEvent::Error {
            session_id,
            error_code,
            message,
            ..
        } => {
            tracing::error!(
                session_id = %session_id,
                error_code = %error_code,
                message = %message,
                "ACP error"
            );
        }
    }
}

/// Generate an ISO 8601 timestamp string for the current UTC time.
fn now_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Helper to create a `Progress` event with the current timestamp.
pub fn progress_event(session_id: String, message: String, percentage: Option<f32>) -> ACPEvent {
    ACPEvent::Progress {
        session_id,
        message,
        percentage,
        timestamp: now_timestamp(),
    }
}

/// Helper to create a `ToolCall` event with the current timestamp.
pub fn tool_call_event(
    session_id: String,
    tool_name: String,
    arguments: serde_json::Value,
    call_id: String,
) -> ACPEvent {
    ACPEvent::ToolCall {
        session_id,
        tool_name,
        arguments,
        call_id,
        timestamp: now_timestamp(),
    }
}

/// Helper to create a `ToolResult` event with the current timestamp.
pub fn tool_result_event(
    session_id: String,
    call_id: String,
    success: bool,
    output: Option<String>,
    error: Option<String>,
) -> ACPEvent {
    ACPEvent::ToolResult {
        session_id,
        call_id,
        success,
        output,
        error,
        timestamp: now_timestamp(),
    }
}

/// Helper to create a `Question` event with the current timestamp.
pub fn question_event(
    session_id: String,
    question_id: String,
    content: String,
    options: Option<Vec<String>>,
) -> ACPEvent {
    ACPEvent::Question {
        session_id,
        question_id,
        content,
        options,
        timestamp: now_timestamp(),
    }
}

/// Helper to create a `PermissionRequest` event with the current timestamp.
pub fn permission_request_event(
    session_id: String,
    request_id: String,
    action: String,
    details: serde_json::Value,
) -> ACPEvent {
    ACPEvent::PermissionRequest {
        session_id,
        request_id,
        action,
        details,
        timestamp: now_timestamp(),
    }
}

/// Helper to create a `Completion` event with the current timestamp.
pub fn completion_event(
    session_id: String,
    status: CompletionStatus,
    summary: Option<String>,
    artifacts: Option<Vec<String>>,
) -> ACPEvent {
    ACPEvent::Completion {
        session_id,
        status,
        summary,
        artifacts,
        timestamp: now_timestamp(),
    }
}

/// Helper to create an `Error` event with the current timestamp.
pub fn error_event(session_id: String, error_code: String, message: String) -> ACPEvent {
    ACPEvent::Error {
        session_id,
        error_code,
        message,
        timestamp: now_timestamp(),
    }
}
