//! ACP (Agent Communication Protocol) error types.
//!
//! This module defines all error variants that can occur when interacting
//! with ACP agent subprocesses via JSON-RPC over stdio.

/// Errors that can occur during ACP agent communication.
///
/// Each variant documents when it occurs and recommended handling strategies.
#[derive(thiserror::Error, Debug)]
pub enum ACPError {
    /// Failed to spawn the agent subprocess.
    ///
    /// Occurs when the operating system refuses to create the child process
    /// (e.g., executable not found, permission denied, or resource limits
    /// exceeded).
    ///
    /// **Handling**: Verify the agent binary path is correct and the binary
    /// has execute permissions. Check system resource limits if the error
    /// persists.
    #[error("failed to spawn agent subprocess '{agent_id}' (command: {command}): {source}")]
    SubprocessSpawn {
        agent_id: String,
        command: String,
        source: std::io::Error,
    },

    /// Agent subprocess exited unexpectedly.
    ///
    /// Occurs when the agent process terminates while a session is still
    /// active, either due to a crash, signal, or normal exit.
    ///
    /// **Handling**: Clean up the session state and notify any waiting
    /// callers. Consider restarting the agent if configured for
    /// auto-recovery.
    #[error("agent subprocess '{agent_id}' exited unexpectedly: exit_code={exit_code:?}")]
    SubprocessCrash {
        agent_id: String,
        exit_code: Option<i32>,
    },

    /// JSON-RPC error returned by the agent.
    ///
    /// Occurs when the agent responds with an error to a JSON-RPC request
    /// (e.g., invalid params, method not found, internal agent error).
    ///
    /// **Handling**: Inspect the error code and message to determine if the
    /// request can be retried. Standard JSON-RPC error codes (-32700 to
    /// -32090) indicate protocol-level issues; application-specific codes
    /// (<= -32000) are agent-defined.
    #[error("JSON-RPC error on method '{method}': code={code}, message={message}")]
    JsonRpcError {
        method: String,
        code: i32,
        message: String,
    },

    /// JSON-RPC transport error (stdio pipe broken).
    ///
    /// Occurs when the communication channel between this process and the
    /// agent subprocess is disrupted (e.g., pipe broken, I/O error on
    /// stdin/stdout).
    ///
    /// **Handling**: The session is no longer usable. Tear down the session
    /// and optionally restart the agent.
    #[error("JSON-RPC transport error: {source}")]
    JsonRpcTransport {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Referenced session does not exist.
    ///
    /// Occurs when an operation targets a session ID that has never been
    /// created or has already been cleaned up.
    ///
    /// **Handling**: Create a new session before retrying the operation,
    /// or verify the session ID is correct.
    #[error("session '{session_id}' not found")]
    SessionNotFound { session_id: String },

    /// Session already exists (unexpected duplicate).
    ///
    /// Occurs when attempting to create a session with an ID that is
    /// already in use. This typically indicates a bug in session ID
    /// generation.
    ///
    /// **Handling**: Generate a new unique session ID and retry.
    #[error("session '{session_id}' already exists")]
    SessionAlreadyExists { session_id: String },

    /// Session is not in active state.
    ///
    /// Occurs when an operation requires an active session but the session
    /// is in a different state (e.g., initializing, closed, or errored).
    ///
    /// **Handling**: Wait for the session to become active or create a new
    /// session.
    #[error("session '{session_id}' is not active")]
    SessionNotActive { session_id: String },

    /// Session exceeded its configured timeout.
    ///
    /// Occurs when a session has been idle or running beyond the allowed
    /// duration. The session will be automatically cleaned up.
    ///
    /// **Handling**: Create a new session and re-issue the request.
    #[error("session '{session_id}' exceeded timeout of {duration:?}")]
    SessionTimeout {
        session_id: String,
        duration: std::time::Duration,
    },

    /// ACP initialize handshake failed.
    ///
    /// Occurs when the agent rejects the initialization request, typically
    /// due to protocol version mismatch or capability incompatibility.
    ///
    /// **Handling**: Verify the agent supports the required ACP protocol
    /// version and capabilities. Update the agent or client as needed.
    #[error("ACP initialize failed for agent '{agent_id}': {reason}")]
    InitializeFailed { agent_id: String, reason: String },

    /// Permission request was explicitly denied.
    ///
    /// Occurs when the user or policy engine denies a permission request
    /// made by the agent (e.g., file access, tool execution).
    ///
    /// **Handling**: The requested operation cannot proceed. Inform the
    /// caller and consider alternative approaches that don't require the
    /// denied permission.
    #[error("permission denied for session '{session_id}': {action}")]
    PermissionDenied { session_id: String, action: String },

    /// Permission request timed out with no response.
    ///
    /// Occurs when a permission request is issued but no response is
    /// received within the allowed time window.
    ///
    /// **Handling**: Treat as a denial by default (fail-safe). The
    /// operation should not proceed without explicit permission.
    #[error("permission request '{request_id}' for session '{session_id}' timed out")]
    PermissionTimeout {
        session_id: String,
        request_id: String,
    },

    /// Event stream error.
    ///
    /// Occurs when the event stream (e.g., SSE or WebSocket connection)
    /// encounters an error during streaming of agent events.
    ///
    /// **Handling**: Reconnect the event stream if the connection is
    /// expected to be long-lived. Otherwise, close and clean up.
    #[error("event stream error: {source}")]
    EventStreamError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// File system I/O error.
    ///
    /// Occurs when a file or directory operation fails (e.g., read, write,
    /// create, delete). The `path` field indicates which file or directory
    /// was being accessed.
    ///
    /// **Handling**: Check file permissions, disk space, and path validity.
    #[error("I/O error at '{0}': {1}")]
    Io(std::path::PathBuf, std::io::Error),

    /// File system I/O error without an associated path.
    ///
    /// Occurs when a raw `std::io::Error` is converted via the `From`
    /// implementation. Prefer `ACPError::Io(path, error)` when the
    /// affected path is known.
    #[error("I/O error: {0}")]
    IoBare(std::io::Error),

    /// Configuration error.
    ///
    /// Occurs when a required configuration field is missing, malformed,
    /// or has an invalid value.
    ///
    /// **Handling**: Fix the configuration file or environment variable
    /// and restart the application.
    #[error("configuration error for field '{field}': {reason}")]
    ConfigError { field: String, reason: String },

    /// Generic operation timeout.
    ///
    /// Occurs when an operation (e.g., request/response, handshake, or
    /// other time-bound task) exceeds its allowed duration.
    ///
    /// **Handling**: Retry with a longer timeout if appropriate, or
    /// investigate why the operation is taking too long.
    #[error("operation '{operation}' timed out after {duration:?}")]
    Timeout {
        operation: String,
        duration: std::time::Duration,
    },
}

/// Ergonomic conversion from `std::io::Error` for use with the `?` operator.
///
/// Converts a raw I/O error into an `ACPError::IoBare`. Prefer using
/// `ACPError::Io(path, error)` directly when the affected path is known.
impl From<std::io::Error> for ACPError {
    fn from(e: std::io::Error) -> Self {
        ACPError::IoBare(e)
    }
}

/// Result type alias for ACP operations.
pub type Result<T> = std::result::Result<T, ACPError>;
