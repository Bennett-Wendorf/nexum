# Plan: 006 - ACP Client

## Task Description
Implement the complete ACP (Agent Client Protocol) client module for nexum. This module handles all communication between nexum and ACP-compatible agent subprocesses via JSON-RPC 2.0 over stdio. It includes: `jsonrpsee` client setup for JSON-RPC communication, subprocess spawning and lifecycle management via `tokio::process`, ACP session lifecycle (create/message/destroy), event streaming for real-time agent progress, permission request handling, error recovery for crashed agents, and per-role session configuration. The module provides a typed Rust API that the Overlord and Builder workflow can invoke to manage agent sessions throughout their lifecycle.

## Objective
Create a fully functional `src/acp/` module that:
- Spawns agent subprocesses with stdio pipes for JSON-RPC communication
- Establishes a `jsonrpsee` JSON-RPC 2.0 client connection over stdio
- Implements the complete ACP session lifecycle: initialize, create, message, destroy
- Streams ACP events (progress, tool calls, results, questions) to a central event bus
- Handles permission requests from agents (approve/deny)
- Manages graceful subprocess shutdown (SIGTERM → 5s wait → SIGKILL)
- Detects agent crashes and triggers stale heartbeat recovery
- Supports per-role session configuration (Builder, Reviewer, Planner, Security consultant)
- Provides a clean public API for the Overlord and Builder workflow to invoke

## Problem Statement
Nexum needs a robust, type-safe client for communicating with ACP-compatible coding agents. Agents are spawned as subprocesses and communicate exclusively via JSON-RPC 2.0 over stdio. The communication pattern involves bidirectional RPC calls (nexum sends requests, agent sends responses and notifications) and continuous event streaming (agent pushes progress, tool calls, results, questions, and permission requests). Without this module, nexum cannot orchestrate agent execution — it cannot spawn agents, send them tasks, receive their progress, handle their permission requests, or cleanly shut them down. This module is the fundamental bridge between nexum's orchestration logic and the agents that do the actual work.

## Solution Approach
Build an `acp` module in `src/acp/` with the following submodules:

1. **`subprocess.rs`** — Agent subprocess management. Spawns agent processes via `tokio::process::Command`, manages stdio pipes, handles graceful shutdown (SIGTERM → SIGKILL fallback), forwards stderr to tracing, and tracks `Child` handles for zombie prevention.

2. **`client.rs`** — JSON-RPC client wrapper. Wraps `jsonrpsee` to provide typed ACP method calls: `initialize()`, `sessions/create()`, `sessions/message()`, `sessions/destroy()`. Handles request/response correlation and error mapping.

3. **`session.rs`** — ACP session lifecycle manager. Encapsulates the full session lifecycle (create → run → interact → complete → destroy). Maintains session state, manages the JSON-RPC client connection, and coordinates with the subprocess manager.

4. **`events.rs`** — Event streaming and types. Defines ACP event types (progress, tool_call, tool_result, question, permission_request, completion). Provides an event stream that broadcasts to the central event bus. Implements event relay for the WebSocket frontend integration.

5. **`permissions.rs`** — Permission request handling. Processes agent permission requests (file write, command execution, network request). Provides approve/deny responses. Integrates with nexum's permission configuration.

6. **`config.rs`** — Session configuration per role. Defines configuration templates per role (Builder, Reviewer, Planner, Security consultant). Maps role to session parameters (tool permissions, timeouts, model selection).

7. **`errors.rs`** — Custom error type `ACPError` using `thiserror` with variants for subprocess failures, JSON-RPC errors, session errors, timeout errors, and permission handling errors.

8. **`tests.rs`** — Unit and integration tests using mock ACP servers.

The module follows these design principles from `design/agent-harness-integration.md` and `design/tech-stack.md`:
- JSON-RPC 2.0 via `jsonrpsee` (mature, handles request/response correlation, batch requests, notifications)
- Subprocess via `tokio::process` (async, stdio pipes)
- ACP-only integration (no adapters in nexum; community-maintained)
- Event streaming to central event bus (for WebSocket frontend relay)
- Permission gates by default (yolo mode is future)
- Graceful shutdown with SIGTERM/SIGKILL fallback
- Crash detection via stale heartbeat recovery

## Relevant Files

### Existing Files
- `design/agent-harness-integration.md` — Primary design doc for ACP integration
- `design/tech-stack.md` — Technology choices (jsonrpsee, tokio::process, tracing)
- `design/agent-roles.md` — Agent role definitions (Builder, Reviewer, Planner, Security consultant)
- `design/permissions.md` — Permission handling requirements
- `design/work-statuses.md` — Task lifecycle including `running` status
- `design/resource-constraints.md` — Concurrency limits affecting agent spawning
- `design/persistence.md` — Status.json heartbeat mechanism for crash detection
- `design/implementation-chunks.md` — Defines this as chunk 6 of the MVP
- `Cargo.toml` — Will contain `jsonrpsee` dependency (added by this chunk)
- `src/acp/mod.rs` — Module stub (created by chunk 001)
- `src/config/` — Configuration system (completed by chunk 002) for agent registration
- `src/persistence/` — Persistence layer (completed by chunk 003) for status.json heartbeat updates

### New Files (if needed)
- `src/acp/mod.rs` — Module root, re-exports public types and functions
- `src/acp/subprocess.rs` — Agent subprocess spawning and lifecycle management
- `src/acp/client.rs` — JSON-RPC client wrapper (jsonrpsee)
- `src/acp/session.rs` — ACP session lifecycle manager
- `src/acp/events.rs` — Event types and streaming
- `src/acp/permissions.rs` — Permission request handling
- `src/acp/config.rs` — Per-role session configuration
- `src/acp/errors.rs` — ACPError enum with thiserror
- `src/acp/tests.rs` — Unit and integration tests

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: acp-core-builder
  - Role: Implement subprocess management, JSON-RPC client wrapper, and error types
  - Agent: builder

- **Builder**
  - Name: acp-session-builder
  - Role: Implement session lifecycle, event streaming, permission handling, and role configuration
  - Agent: builder

- **Builder**
  - Name: acp-tests-builder
  - Role: Write comprehensive unit and integration tests with mock ACP servers
  - Agent: builder

- **Validator**
  - Name: acp-validator
  - Role: Verify subprocess management, JSON-RPC communication, session lifecycle, event streaming, and error handling
  - Agent: validator

- **Documenter**
  - Name: acp-documenter
  - Role: Generate documentation for completed ACP client module
  - Agent: documenter

## Step by Step Tasks

### 1. Implement Agent Subprocess Manager
- **Task ID**: acp-subprocess-manager
- **Depends On**: none
- **Assigned To**: acp-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/subprocess.rs` with:

    **AgentProcess struct**:
    ```rust
    pub struct AgentProcess {
        pub agent_id: String,          // Agent registration name (e.g., "opencode")
        pub child: Option<tokio::process::Child>,
        pub stdin: Option<tokio::process::ChildStdin>,
        pub stdout: Option<tokio::process::ChildStdout>,
        pub stderr_task: Option<tokio::task::JoinHandle<()>>,
        pub spawn_time: Instant,
        pub worktree_path: PathBuf,     // Task's worktree directory
    }
    ```

    **Process spawning**:
    - `pub fn spawn_agent(config: &AgentConfig, worktree_path: &Path) -> Result<AgentProcess>` — Spawn an agent subprocess:
      1. Construct `tokio::process::Command` from agent config (binary path, args)
      2. Set `current_dir` to `worktree_path` (the task's isolated worktree)
      3. Configure stdio: `.stdin(Stdio::piped())`, `.stdout(Stdio::piped())`, `.stderr(Stdio::piped())`
      4. Spawn the process
      5. Extract stdin, stdout, stderr handles
      6. Spawn a background task to forward stderr to `tracing::error!` (or `tracing::info!` for non-error output)
      7. Return `AgentProcess` with all handles
      8. On spawn failure, return `ACPError::SubprocessSpawn` with command details

    **Graceful shutdown**:
    - `pub async fn shutdown(&mut self) -> Result<()>` — Gracefully shut down the agent process:
      1. Send SIGTERM via `child.kill()` (on Unix; use appropriate platform signal)
      2. Wait up to 5 seconds for the process to exit (`tokio::time::timeout(5s, child.wait())`)
      3. If process hasn't exited after 5 seconds, send SIGKILL (force kill)
      4. Reap the child process (`child.wait()`)
      5. Cancel stderr forwarding task
      6. Clear all handles (set to None)
      7. Return success
    - `pub async fn force_kill(&mut self) -> Result<()>` — Force kill immediately (no SIGTERM wait):
      1. Send SIGKILL immediately
      2. Reap the child process
      3. Cancel stderr forwarding task
      4. Clear all handles

    **Process state queries**:
    - `pub fn is_alive(&self) -> bool` — Check if process is still running:
      1. Try to get exit status via `child.try_wait()`
      2. If `Some(exit_status)`, process has exited
      3. If `None`, process is still running
    - `pub fn exit_status(&self) -> Option<ExitStatus>` — Get last known exit status:
      1. Call `child.try_wait()` and return the result

    **AgentConfig struct**:
    ```rust
    pub struct AgentConfig {
        pub name: String,              // Human-readable name (e.g., "OpenCode")
        pub binary: PathBuf,           // Binary path (e.g., "/usr/bin/opencode")
        pub args: Vec<String>,         // Spawn arguments (e.g., ["acp"])
        pub env: HashMap<String, String>, // Environment variables (optional)
    }
    ```

    **Stderr forwarding**:
    - Background task reads from stderr line by line
    - Each line is logged via `tracing::info!(agent_id = ?, line = ?)` or `tracing::error!(...)` based on content
    - Task runs until the process exits or is killed
    - Use `tokio::io::BufReader` for efficient line reading

  - All subprocess operations are async (tokio)
  - SIGTERM → 5s wait → SIGKILL pattern for graceful shutdown
  - stderr is forwarded to tracing (not captured for JSON-RPC)
  - stdin/stdout are reserved exclusively for JSON-RPC communication
  - `Child` handles are tracked to prevent zombie processes

- **Acceptance Criteria**:
  - `spawn_agent()` spawns the agent binary with stdio pipes
  - `spawn_agent()` sets working directory to `worktree_path`
  - `spawn_agent()` spawns a background stderr forwarding task
  - `spawn_agent()` returns `AgentProcess` with all handles populated
  - `shutdown()` sends SIGTERM, waits up to 5 seconds, then SIGKILL
  - `shutdown()` reaps the child process to prevent zombies
  - `shutdown()` cancels the stderr forwarding task
  - `force_kill()` immediately sends SIGKILL
  - `is_alive()` correctly reports process state
  - `exit_status()` returns the last known exit status
  - `AgentConfig` contains binary path, args, env vars
  - Stderr forwarding logs to tracing (not captured for JSON-RPC)
  - All functions return `acp::Result<T>`

### 2. Implement ACP Error Types
- **Task ID**: acp-errors
- **Depends On**: none
- **Assigned To**: acp-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/errors.rs` with `ACPError` enum using `thiserror`:
    - `SubprocessSpawn { agent_id: String, command: String, source: io::Error }` — Failed to spawn agent subprocess
    - `SubprocessCrash { agent_id: String, exit_code: Option<i32> }` — Agent subprocess exited unexpectedly
    - `JsonRpcError { method: String, code: i32, message: String }` — JSON-RPC error from agent
    - `JsonRpcTransport { source: Box<dyn std::error::Error + Send + Sync> }` — JSON-RPC transport error (stdio pipe broken)
    - `SessionNotFound { session_id: String }` — Referenced session does not exist
    - `SessionAlreadyExists { session_id: String }` — Session already exists (unexpected)
    - `SessionNotActive { session_id: String }` — Session is not in active state
    - `SessionTimeout { session_id: String, duration: Duration }` — Session exceeded timeout
    - `InitializeFailed { agent_id: String, reason: String }` — ACP initialize failed (version mismatch, capability mismatch)
    - `PermissionDenied { session_id: String, action: String }` — Permission request denied
    - `PermissionTimeout { session_id: String, request_id: String }` — Permission request timed out (no response)
    - `EventStreamError { source: Box<dyn std::error::Error + Send + Sync> }` — Event stream error
    - `Io(PathBuf, io::Error)` — File system I/O error
    - `ConfigError { field: String, reason: String }` — Configuration error
    - `Timeout { operation: String, duration: Duration }` — Generic operation timeout
  - Implement `std::fmt::Display` (provided by thiserror derive)
  - Implement `From<io::Error>` for ergonomic `?` operator usage
  - Implement `From<jsonrpsee::core::ClientError>` for ergonomic JSON-RPC error handling
  - Define type alias: `pub type Result<T> = std::result::Result<T, ACPError>`
  - Document each error variant with when it occurs and how to handle it

- **Acceptance Criteria**:
  - `ACPError` compiles with `thiserror::Error` derive
  - All error variants include sufficient context for debugging
  - `SubprocessCrash` includes exit code for crash diagnostics
  - `JsonRpcError` includes method name, error code, and message
  - `InitializeFailed` includes agent ID and reason for failure
  - `PermissionDenied` and `PermissionTimeout` include session and request IDs
  - `From<io::Error>` implementation allows `?` operator
  - `From<jsonrpsee::core::ClientError>` allows `?` operator for JSON-RPC errors
  - `Result<T>` type alias is defined and accessible
  - Error messages are descriptive and actionable

### 3. Implement JSON-RPC Client Wrapper
- **Task ID**: acp-jsonrpc-client
- **Depends On**: acp-subprocess-manager, acp-errors
- **Assigned To**: acp-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/client.rs` with:

    **ACPClient struct**:
    ```rust
    pub struct ACPClient {
        inner: jsonrpsee::ws_client::WsClientOrSocket, // Or stdio transport
        agent_id: String,
        protocol_version: String,
        capabilities: ACPCapabilities,
    }
    ```

    **ACP Capabilities struct**:
    ```rust
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct ACPCapabilities {
        pub sessions: bool,           // Supports session management
        pub streaming: bool,          // Supports event streaming
        pub permissions: bool,        // Supports permission requests
        pub tools: Vec<String>,       // Available tools
    }
    ```

    **Client construction**:
    - `pub async fn connect(process: &mut AgentProcess) -> Result<ACPClient>` — Establish JSON-RPC connection:
      1. Create `jsonrpsee` client connected to the subprocess's stdin/stdout pipes
      2. Use `jsonrpsee::client_transport::stdio` transport layer (or equivalent stdio transport)
      3. Call `initialize` method to negotiate protocol version and capabilities
      4. Parse the response to extract protocol version and capabilities
      5. Validate protocol version compatibility (nexum requires ACP version X)
      6. Validate required capabilities (sessions, streaming must be supported)
      7. On failure, return `ACPError::InitializeFailed` with details
      8. Return the `ACPClient`

    **ACP method calls**:
    - `pub async fn sessions_create(&self, params: SessionCreateParams) -> Result<SessionCreateResult>` — Create a new ACP session:
      1. Call JSON-RPC method `sessions/create` with params
      2. Parse response into `SessionCreateResult`
      3. Return session ID
    - `pub async fn sessions_message(&self, session_id: &str, params: SessionMessageParams) -> Result<()>` — Send a message to a session:
      1. Call JSON-RPC method `sessions/message` with session_id and params
      2. Handle response (may be notification, no response expected)
    - `pub async fn sessions_destroy(&self, session_id: &str) -> Result<()>` — Destroy a session:
      1. Call JSON-RPC method `sessions/destroy` with session_id
      2. Handle response
    - `pub async fn permission_respond(&self, request_id: &str, approved: bool) -> Result<()>` — Respond to a permission request:
      1. Call JSON-RPC method `permissions/respond` with request_id and approval
      2. Handle response

    **Request/response types**:
    ```rust
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct SessionCreateParams {
        pub prompt: String,
        pub working_directory: Option<String>,
        pub tool_permissions: Option<Vec<String>>,
        pub timeout: Option<Duration>,
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct SessionCreateResult {
        pub session_id: String,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct SessionMessageParams {
        pub content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub r#type: Option<MessageType>, // "prompt", "follow-up", "feedback"
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum MessageType {
        #[serde(rename = "prompt")]
        Prompt,
        #[serde(rename = "follow-up")]
        FollowUp,
        #[serde(rename = "feedback")]
        Feedback,
    }
    ```

    **Event subscription**:
    - `pub fn subscribe_events(&self) -> tokio::sync::mpsc::UnboundedReceiver<ACPEvent>` — Subscribe to agent events:
      1. Set up a channel for receiving events
      2. Subscribe to JSON-RPC notifications from the agent
      3. Return the receiver
      4. Events flow: agent → JSON-RPC notification → client → channel

  - `jsonrpsee` handles request/response correlation, batch requests, and notifications
  - stdio transport reads from stdout and writes to stdin of the subprocess
  - All method calls return `acp::Result<T>`
  - Event subscription uses `tokio::sync::mpsc` channel for decoupled event delivery
  - Document that `jsonrpsee` is the underlying transport; this wrapper provides typed ACP methods

- **Acceptance Criteria**:
  - `ACPClient` wraps `jsonrpsee` with typed ACP method calls
  - `connect()` establishes JSON-RPC connection over stdio pipes
  - `connect()` calls `initialize` to negotiate protocol version and capabilities
  - `connect()` validates protocol version compatibility
  - `connect()` validates required capabilities (sessions, streaming)
  - `sessions_create()` creates a new ACP session and returns session ID
  - `sessions_message()` sends a message to a running session
  - `sessions_destroy()` destroys a session
  - `permission_respond()` responds to a permission request
  - `subscribe_events()` returns a channel for receiving agent events
  - `SessionCreateParams` includes prompt, working directory, tool permissions, timeout
  - `SessionMessageParams` includes content and message type
  - `ACPEvent` enum covers all ACP event types
  - All method calls return `acp::Result<T>`
  - JSON-RPC errors are mapped to `ACPError::JsonRpcError`

### 4. Implement ACP Event Types and Streaming
- **Task ID**: acp-events
- **Depends On**: acp-errors
- **Assigned To**: acp-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/events.rs` with:

    **ACPEvent enum**:
    ```rust
    #[derive(Debug, Clone, serde::Deserialize)]
    #[serde(tag = "type", rename_all = "kebab-case")]
    pub enum ACPEvent {
        #[serde(rename = "progress")]
        Progress {
            session_id: String,
            message: String,
            percentage: Option<f32>,
            timestamp: DateTime<Utc>,
        },
        #[serde(rename = "tool-call")]
        ToolCall {
            session_id: String,
            tool_name: String,
            arguments: serde_json::Value,
            call_id: String,
            timestamp: DateTime<Utc>,
        },
        #[serde(rename = "tool-result")]
        ToolResult {
            session_id: String,
            call_id: String,
            success: bool,
            output: Option<String>,
            error: Option<String>,
            timestamp: DateTime<Utc>,
        },
        #[serde(rename = "question")]
        Question {
            session_id: String,
            question_id: String,
            content: String,
            options: Option<Vec<String>>,
            timestamp: DateTime<Utc>,
        },
        #[serde(rename = "permission-request")]
        PermissionRequest {
            session_id: String,
            request_id: String,
            action: String,        // "file-write", "command-execution", "network-request"
            details: serde_json::Value,
            timestamp: DateTime<Utc>,
        },
        #[serde(rename = "completion")]
        Completion {
            session_id: String,
            status: CompletionStatus,
            summary: Option<String>,
            artifacts: Option<Vec<String>>,
            timestamp: DateTime<Utc>,
        },
        #[serde(rename = "error")]
        Error {
            session_id: String,
            error_code: String,
            message: String,
            timestamp: DateTime<Utc>,
        },
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum CompletionStatus {
        Success,
        Failed,
        Cancelled,
    }
    ```

    **Event stream**:
    - `pub struct EventStream` — Manages event delivery:
      - `fn new() -> Self` — Create a new event stream
      - `fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ACPEvent>` — Subscribe to events (broadcast channel)
      - `fn publish(&self, event: ACPEvent)` — Publish an event to all subscribers
    - Uses `tokio::sync::broadcast` channel for fan-out to multiple subscribers
    - Broadcast channel capacity: 1024 (configurable)
    - Lagging subscribers receive `broadcast::error::RecvError::Lagged` (document this behavior)

    **Event processing helpers**:
    - `pub fn is_terminal_event(event: &ACPEvent) -> bool` — Check if event signals session completion:
      - Returns true for `Completion` and `Error` events
    - `pub fn requires_response(event: &ACPEvent) -> bool` — Check if event requires nexum response:
      - Returns true for `PermissionRequest` and `Question` events
    - `pub fn session_id(event: &ACPEvent) -> &str` — Extract session ID from any event

    **Event to tracing**:
    - `pub fn log_event(event: &ACPEvent)` — Log event to tracing:
      - Progress events: `tracing::info!`
      - Tool calls: `tracing::debug!`
      - Tool results: `tracing::debug!`
      - Permission requests: `tracing::info!`
      - Questions: `tracing::info!`
      - Completion: `tracing::info!`
      - Errors: `tracing::error!`

  - All events include `session_id` and `timestamp` for correlation
- **Acceptance Criteria**:
  - `ACPEvent` enum covers all ACP event types (progress, tool-call, tool-result, question, permission-request, completion, error)
  - `ACPEvent` deserializes from JSON with serde tag-based enum
  - `CompletionStatus` enum covers success, failed, cancelled
  - `EventStream` uses `tokio::sync::broadcast` for fan-out
  - `EventStream::subscribe()` returns a broadcast receiver
  - `EventStream::publish()` delivers events to all subscribers
  - `is_terminal_event()` correctly identifies completion and error events
  - `requires_response()` correctly identifies permission and question events
  - `session_id()` extracts session ID from any event variant
  - `log_event()` logs events at appropriate tracing levels
  - All event variants include `session_id` and `timestamp`
  - Events deserialize from JSON-RPC notification payloads

### 5. Implement ACP Session Lifecycle Manager
- **Task ID**: acp-session-lifecycle
- **Depends On**: acp-jsonrpc-client, acp-events, acp-subprocess-manager
- **Assigned To**: acp-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/session.rs` with:

    **ACPSession struct**:
    ```rust
    pub struct ACPSession {
        pub session_id: String,
        pub agent_process: AgentProcess,
        pub client: ACPClient,
        pub state: SessionState,
        pub task_id: String,
        pub role: AgentRole,
        pub created_at: DateTime<Utc>,
        pub timeout: Duration,
        pub heartbeat: Instant,     // Last event received time
        pub event_stream_rx: tokio::sync::broadcast::Receiver<ACPEvent>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum SessionState {
        Created,      // Session created, agent working
        Interacting,  // Nexum sent follow-up or responded to permission
        Completing,   // Agent signaled completion
        Destroyed,    // Session destroyed, resources cleaned up
        Error,        // Error state (crash, timeout, etc.)
    }
    ```

    **Session creation**:
    - `pub async fn create(task_id: &str, role: AgentRole, prompt: &str, worktree_path: &Path, agent_config: &AgentConfig, timeout: Duration, event_stream: &EventStream) -> Result<ACPSession>` — Create a new ACP session:
      1. Spawn agent subprocess: `spawn_agent(agent_config, worktree_path)`
      2. Establish JSON-RPC connection: `ACPClient::connect(&mut process)`
      3. Subscribe to events: `client.subscribe_events()`
      4. Call `sessions/create` with prompt and configuration
      5. Parse response to get session ID
      6. Initialize `ACPSession` with all components
      7. Return the session

    **Session interaction**:
    - `pub async fn send_message(&self, content: &str, message_type: MessageType) -> Result<()>` — Send a message to the session:
      1. Validate state is `Created` or `Interacting`
      2. Call `client.sessions_message(session_id, params)`
      3. Update heartbeat
    - `pub async fn respond_to_permission(&self, request_id: &str, approved: bool) -> Result<()>` — Respond to a permission request:
      1. Call `client.permission_respond(request_id, approved)`
      2. Update heartbeat
    - `pub async fn respond_to_question(&self, question_id: &str, answer: &str) -> Result<()>` — Respond to a question:
      1. Call appropriate JSON-RPC method
      2. Update heartbeat

    **Session completion and destruction**:
    - `pub fn check_heartbeat(&mut self, max_stale: Duration) -> bool` — Check if session heartbeat is stale:
      1. Calculate time since last heartbeat
      2. If `elapsed > max_stale`, return true (stale)
      3. Otherwise return false
    - `pub async fn complete(&mut self) -> Result<()>` — Mark session as completing:
      1. Transition state to `Completing`
      2. Log completion
    - `pub async fn destroy(&mut self) -> Result<()>` — Destroy the session:
      1. Call `sessions/destroy` via JSON-RPC
      2. Transition state to `Destroyed`
      3. Shutdown the subprocess: `agent_process.shutdown()`
      4. Clean up resources

    **Event loop integration**:
    - `pub async fn run_event_loop(&mut self, event_stream: &EventStream) -> Result<Option<ACPEvent>>` — Run the session event loop:
      1. Listen on `event_stream_rx` for incoming events
      2. For each event:
        a. Publish to central `event_stream`
        b. Update heartbeat
        c. Log the event
        d. If terminal event (completion/error), return it
        e. If permission request or question, return it (caller handles response)
      3. If subprocess exits unexpectedly, return `ACPError::SubprocessCrash`
      4. If timeout exceeded, return `ACPError::SessionTimeout`

    **Session configuration**:
    - `pub fn apply_role_config(&self, role: AgentRole) -> SessionCreateParams` — Generate session params based on role:
      1. Map role to default tool permissions
      2. Map role to default timeout
      3. Return configured `SessionCreateParams`

  - Session state machine: Created → Interacting → Completing → Destroyed
- **Acceptance Criteria**:
  - `ACPSession` contains all session state (ID, process, client, state, task_id, role, heartbeat)
  - `create()` spawns subprocess, establishes JSON-RPC, creates ACP session
  - `create()` returns fully initialized `ACPSession`
  - `send_message()` sends messages to running session
  - `send_message()` validates session state before sending
  - `respond_to_permission()` responds to agent permission requests
  - `respond_to_question()` responds to agent questions
  - `check_heartbeat()` detects stale sessions
  - `complete()` transitions state to Completing
  - `destroy()` calls ACP destroy, shuts down subprocess
  - `run_event_loop()` processes events, updates heartbeat, detects terminal events
  - `run_event_loop()` returns terminal events and events requiring response
  - `SessionState` enum covers all lifecycle states
  - Heartbeat is updated on every received event
  - Session timeout is enforced via heartbeat checking
  - All functions return `acp::Result<T>`

### 6. Implement Permission Handling
- **Task ID**: acp-permissions
- **Depends On**: acp-events, acp-session-lifecycle
- **Assigned To**: acp-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/permissions.rs` with:

    **Permission types**:
    ```rust
    #[derive(Debug, Clone, serde::Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum PermissionAction {
        FileRead,
        FileWrite { path: String },
        CommandExecution { command: String },
        NetworkRequest { url: String, method: String },
        Other { action: String, details: serde_json::Value },
    }

    #[derive(Debug, Clone)]
    pub struct PermissionRequest {
        pub request_id: String,
        pub session_id: String,
        pub action: PermissionAction,
        pub timestamp: DateTime<Utc>,
    }
    ```

    **Permission handler**:
    - `pub struct PermissionHandler` — Manages permission policy:
      - `fn new(policy: PermissionPolicy) -> Self` — Create with policy
      - `fn evaluate(&self, request: &PermissionRequest) -> PermissionDecision` — Evaluate a permission request:
        1. Check policy for the action type
        2. If action is in `auto_approve` list, return `Approved`
        3. If action is in `auto_deny` list, return `Denied`
        4. Otherwise return `Pending` (requires user/Overlord decision)
    - `pub async fn handle_permission(session: &ACPSession, request: &PermissionRequest, decision: PermissionDecision) -> Result<()>` — Handle a permission request:
      1. If `Approved`, call `session.respond_to_permission(request_id, true)`
      2. If `Denied`, call `session.respond_to_permission(request_id, false)`
      3. If `Pending`, return without responding (caller must decide)

    **Permission policy**:
    ```rust
    #[derive(Debug, Clone)]
    pub struct PermissionPolicy {
        pub auto_approve: Vec<String>,   // Actions auto-approved (e.g., "file-read")
        pub auto_deny: Vec<String>,       // Actions auto-denied (e.g., "network-request")
        pub require_approval: Vec<String>, // Actions requiring approval (default: all others)
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum PermissionDecision {
        Approved,
        Denied,
        Pending,
    }
    ```

    **Default policies per role**:
    - `pub fn default_policy_for_role(role: AgentRole) -> PermissionPolicy` — Generate default permission policy per role:
      - Builder: auto-approve file-read, file-write (in worktree); require approval for command-execution, network-request
      - Reviewer: auto-approve file-read; deny all writes and commands
      - Planner: auto-approve file-read; require approval for writes
      - Security consultant: like reviewer, with additional network-request restrictions

  - Permission handling follows `design/permissions.md` (default: permission gates)
  - Yolo mode (auto-approve all) is future work, not implemented in MVP
  - Policy is configurable per role and per task
  - Auto-approve/deny decisions are made immediately; `Pending` requires external decision

- **Acceptance Criteria**:
  - `PermissionAction` enum covers file-read, file-write, command-execution, network-request, other
  - `PermissionRequest` contains request_id, session_id, action, timestamp
  - `PermissionHandler` evaluates requests against policy
  - `PermissionHandler::evaluate()` returns Approved, Denied, or Pending
  - `handle_permission()` responds to agent based on decision
  - `PermissionPolicy` contains auto_approve, auto_deny, require_approval lists
  - `PermissionDecision` enum covers Approved, Denied, Pending
  - `default_policy_for_role()` generates appropriate policy per role
  - Builder policy auto-approves file operations in worktree
  - Reviewer policy auto-approves reads only
  - Planner policy auto-approves reads, requires approval for writes
  - Security consultant policy is most restrictive
  - All functions return `acp::Result<T>` where applicable

### 7. Implement Per-Role Session Configuration
- **Task ID**: acp-role-config
- **Depends On**: acp-permissions, acp-session-lifecycle
- **Assigned To**: acp-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/config.rs` with:

    **Agent role enum**:
    ```rust
    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum AgentRole {
        Builder,
        Reviewer,
        Planner,
        SecurityConsultant,
    }
    ```

    **Role configuration**:
    ```rust
    #[derive(Debug, Clone)]
    pub struct RoleConfig {
        pub role: AgentRole,
        pub default_timeout: Duration,
        pub tool_permissions: Vec<String>,
        pub permission_policy: PermissionPolicy,
        pub max_tokens: Option<u32>,
        pub model_preference: Option<String>,
    }
    ```

    **Configuration functions**:
    - `pub fn default_role_config(role: AgentRole) -> RoleConfig` — Get default configuration for a role:
      - Builder: 30 min timeout, all tools in worktree, builder permission policy
      - Reviewer: 15 min timeout, read-only tools, reviewer permission policy
      - Planner: 20 min timeout, read + write tools, planner permission policy
      - Security consultant: 20 min timeout, read + scan tools, security permission policy
    - `pub fn merge_with_task_config(role_config: &RoleConfig, task_overrides: &TaskConfigOverrides) -> RoleConfig` — Merge role defaults with task-specific overrides:
      1. Start with role defaults
      2. Override timeout if specified in task config
      3. Override tool permissions if specified
      4. Override model preference if specified
      5. Return merged config
    - `pub fn to_session_params(config: &RoleConfig, prompt: &str, worktree_path: &Path) -> SessionCreateParams` — Convert role config to session creation params:
      1. Map tool permissions to `SessionCreateParams.tool_permissions`
      2. Map timeout to `SessionCreateParams.timeout`
      3. Set working directory to worktree path
      4. Set prompt
      5. Return params

    **Task configuration overrides**:
    ```rust
    #[derive(Debug, Clone, Default)]
    pub struct TaskConfigOverrides {
        pub timeout: Option<Duration>,
        pub tool_permissions: Option<Vec<String>>,
        pub model_preference: Option<String>,
    }
    ```

  - Role configuration drives session behavior (timeout, tools, permissions)
  - Task-specific overrides allow per-task customization
  - Configuration is hierarchical: global → role → task
  - Default values are sensible for each role

- **Acceptance Criteria**:
  - `AgentRole` enum covers Builder, Reviewer, Planner, SecurityConsultant
  - `RoleConfig` contains timeout, tool permissions, permission policy, model preference
  - `default_role_config()` returns appropriate defaults per role
  - Builder default timeout is 30 minutes
  - Reviewer default timeout is 15 minutes
  - Planner default timeout is 20 minutes
  - Security consultant default timeout is 20 minutes
  - `merge_with_task_config()` correctly applies task overrides
  - `to_session_params()` converts role config to session creation params
  - `TaskConfigOverrides` allows per-task customization
  - Configuration hierarchy: global → role → task
  - All functions are pure (no side effects)

### 8. Wire Up Module Exports
- **Task ID**: acp-module-wiring
- **Depends On**: acp-subprocess-manager, acp-errors, acp-jsonrpc-client, acp-events, acp-session-lifecycle, acp-permissions, acp-role-config
- **Assigned To**: acp-core-builder
- **Agent**: builder
- **Actions**:
  - Update `src/acp/mod.rs` to:
    - Declare submodules: `mod errors; mod subprocess; mod client; mod session; mod events; mod permissions; mod config;`
    - Re-export public types:
      - `pub use errors::*;` (ACPError, Result)
      - `pub use subprocess::*;` (AgentProcess, AgentConfig)
      - `pub use client::*;` (ACPClient, ACPCapabilities, SessionCreateParams, SessionCreateResult, SessionMessageParams, MessageType)
      - `pub use events::*;` (ACPEvent, CompletionStatus, EventStream)
      - `pub use session::*;` (ACPSession, SessionState)
      - `pub use permissions::*;` (PermissionAction, PermissionRequest, PermissionHandler, PermissionPolicy, PermissionDecision)
      - `pub use config::*;` (AgentRole, RoleConfig, TaskConfigOverrides)
    - Re-export public functions:
      - Subprocess: `spawn_agent`
      - Client: `connect`
      - Session: `create_session` (alias for `ACPSession::create`)
      - Events: `is_terminal_event`, `requires_response`, `session_id_from_event`, `log_event`
      - Permissions: `handle_permission`, `default_policy_for_role`
      - Config: `default_role_config`, `merge_with_task_config`, `to_session_params`
    - Include module-level documentation referencing `design/agent-harness-integration.md`
  - Add `mod acp;` to `src/main.rs` (if not already present from scaffolding)
  - Add `jsonrpsee` to `Cargo.toml` dependencies:
    ```toml
    [dependencies]
    jsonrpsee = { version = "0.24", features = ["client-transport-stdio"] }
    ```
    - Note: Verify exact feature flag for stdio transport in jsonrpsee documentation

- **Acceptance Criteria**:
  - All public types are accessible as `nexum::acp::ACPError`, `nexum::acp::ACPClient`, etc.
  - All public functions are accessible via the acp module
  - Module compiles without errors
  - Module documentation references design docs
  - Re-exports are organized logically by category
  - `jsonrpsee` dependency is added to `Cargo.toml`
  - `mod acp;` is declared in `src/main.rs`

### 9. Write Tests
- **Task ID**: acp-tests
- **Depends On**: acp-module-wiring
- **Assigned To**: acp-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `src/acp/tests.rs` with comprehensive tests:

    **Test infrastructure**:
    - `struct MockACPServer` — Mock ACP server for testing:
      - Implements a minimal ACP server that responds to JSON-RPC requests
      - Runs in a subprocess (echo/printf based or simple Rust binary)
      - Simulates session creation, messaging, events, and completion
    - `fn create_mock_agent() -> (tempfile::TempDir, PathBuf)` — Create a mock agent binary:
      1. Write a simple script that reads JSON-RPC from stdin and writes responses to stdout
      2. Make it executable
      3. Return path to the script

    **Subprocess tests**:
    - `test_spawn_agent` — Spawn a mock agent and verify subprocess is alive
    - `test_shutdown_graceful` — Test SIGTERM → wait → SIGKILL pattern
    - `test_shutdown_timeout` — Test SIGKILL fallback after 5 seconds
    - `test_stderr_forwarding` — Verify stderr is forwarded to tracing
    - `test_is_alive` — Test process state detection
    - `test_zombie_prevention` — Verify child handles are reaped

    **JSON-RPC client tests**:
    - `test_connect` — Establish JSON-RPC connection with mock agent
    - `test_initialize` — Verify initialize negotiation works
    - `test_sessions_create` — Create a session and verify session ID
    - `test_sessions_message` — Send a message and verify delivery
    - `test_sessions_destroy` — Destroy a session and verify cleanup
    - `test_permission_respond` — Respond to a permission request
    - `test_jsonrpc_error` — Verify JSON-RPC errors are mapped to ACPError
    - `test_transport_error` — Verify stdio pipe break is handled

    **Event tests**:
    - `test_event_deserialization` — Verify ACPEvent deserializes from JSON
    - `test_event_stream_publish_subscribe` — Verify broadcast channel delivery
    - `test_is_terminal_event` — Verify terminal event detection
    - `test_requires_response` — Verify response-required event detection
    - `test_session_id_extraction` — Verify session ID extraction from events

    **Session lifecycle tests**:
    - `test_session_create` — Create a session and verify state
    - `test_session_send_message` — Send message to session
    - `test_session_permission_response` — Respond to permission request
    - `test_session_heartbeat` — Verify heartbeat updates on events
    - `test_session_stale_detection` — Verify stale heartbeat detection
    - `test_session_destroy` — Destroy session and verify cleanup
    - `test_session_timeout` — Verify session timeout enforcement

    **Permission tests**:
    - `test_auto_approve` — Verify auto-approve policy works
    - `test_auto_deny` — Verify auto-deny policy works
    - `test_pending` — Verify pending requires external decision
    - `test_handle_permission_approved` — Verify approved response sent to agent
    - `test_handle_permission_denied` — Verify denied response sent to agent
    - `test_default_policy_builder` — Verify builder default policy
    - `test_default_policy_reviewer` — Verify reviewer default policy

    **Configuration tests**:
    - `test_default_role_config_builder` — Verify builder defaults
    - `test_default_role_config_reviewer` — Verify reviewer defaults
    - `test_merge_with_task_config` — Verify task override merging
    - `test_to_session_params` — Verify params conversion

  - Use `tokio::test` for async tests
  - Mock ACP server simulates realistic agent behavior
  - All tests are self-contained and isolated

- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum acp`
  - Tests cover all modules: subprocess, client, session, events, permissions, config
  - Mock ACP server correctly simulates agent behavior
  - Subprocess tests verify spawn, shutdown, stderr forwarding, zombie prevention
  - Client tests verify all JSON-RPC method calls
  - Event tests verify deserialization, streaming, and helper functions
  - Session tests verify full lifecycle (create, interact, destroy)
  - Permission tests verify policy evaluation and handling
  - Configuration tests verify role defaults and task overrides
  - All tests are async (`#[tokio::test]`)

### 10. Final Validation
- **Task ID**: validate-all
- **Depends On**: acp-tests
- **Assigned To**: acp-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum acp` — all acp tests must pass
  - Run `cargo clippy --package nexum` — no warnings in acp module
  - Verify `AgentProcess` uses `tokio::process::Command`
  - Verify subprocess spawn sets `current_dir` to worktree path
  - Verify SIGTERM → 5s wait → SIGKILL shutdown pattern
  - Verify stderr forwarding to tracing (not JSON-RPC)
  - Verify stdin/stdout reserved for JSON-RPC
  - Verify `ACPClient` uses `jsonrpsee` for JSON-RPC communication
  - Verify `connect()` calls `initialize` for protocol negotiation
  - Verify `initialize` validates protocol version and capabilities
  - Verify `sessions/create`, `sessions/message`, `sessions/destroy` are implemented
  - Verify `permission_respond` is implemented
  - Verify event subscription returns a broadcast receiver
  - Verify `ACPEvent` covers all event types (progress, tool-call, tool-result, question, permission-request, completion, error)
  - Verify `EventStream` uses `tokio::sync::broadcast`
  - Verify `ACPSession` manages full lifecycle (create → run → interact → complete → destroy)
  - Verify heartbeat is updated on every event
  - Verify stale heartbeat detection works
  - Verify session timeout enforcement
  - Verify `PermissionHandler` evaluates requests against policy
  - Verify `default_policy_for_role()` generates correct policies per role
  - Verify `RoleConfig` contains timeout, tool permissions, permission policy
  - Verify `merge_with_task_config()` applies task overrides
  - Verify `to_session_params()` converts config to session params
  - Verify `ACPError` uses `thiserror` with 15+ descriptive error variants
  - Verify `Result<T>` type alias is defined
  - Verify all public types and functions are re-exported from `mod.rs`
  - Verify `jsonrpsee` dependency is in `Cargo.toml`
  - Verify module documentation references `design/agent-harness-integration.md`
  - Verify `Child` handles are tracked and reaped (zombie prevention)
  - Verify crash detection: subprocess exit without ACP destroy triggers error

### 11. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: acp-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the ACP client architecture (subprocess → JSON-RPC → session)
  - Document the session lifecycle (create, run, interact, complete, destroy)
  - Document the event streaming system (types, broadcast channel, relay)
  - Document the permission handling system (policy, evaluation, response)
  - Document the per-role configuration system (defaults, overrides, hierarchy)
  - Document the subprocess management (spawn, shutdown, stderr forwarding)
  - Document the Rust API (public functions and types)
  - Document error handling and common failure modes
  - Document the crash detection and recovery mechanism

## Acceptance Criteria
- `cargo check` succeeds with no errors in the acp module
- `cargo test --package nexum acp` passes all tests
- `cargo clippy --package nexum` produces no warnings in acp module
- `AgentProcess` uses `tokio::process::Command` for subprocess management
- Subprocess spawn sets working directory to task's worktree path
- SIGTERM → 5s wait → SIGKILL graceful shutdown pattern is implemented
- stderr is forwarded to tracing (not captured for JSON-RPC)
- stdin/stdout are reserved for JSON-RPC communication
- `Child` handles are tracked and reaped (zombie prevention)
- `ACPClient` wraps `jsonrpsee` with typed ACP method calls
- `connect()` calls `initialize` for protocol version and capability negotiation
- `sessions/create`, `sessions/message`, `sessions/destroy` are implemented
- `permission_respond` responds to agent permission requests
- Event subscription uses `tokio::sync::broadcast` for fan-out
- `ACPEvent` covers all event types (progress, tool-call, tool-result, question, permission-request, completion, error)
- `ACPSession` manages full lifecycle (create → run → interact → complete → destroy)
- Heartbeat is updated on every received event
- Stale heartbeat detection works (configurable threshold)
- Session timeout enforcement works
- `PermissionHandler` evaluates requests against configurable policy
- `default_policy_for_role()` generates correct policies per role
- `RoleConfig` contains timeout, tool permissions, permission policy, model preference
- `merge_with_task_config()` applies task-specific overrides
- Crash detection: subprocess exit without ACP destroy triggers `ACPError::SubprocessCrash`
- `ACPError` uses `thiserror` with 15+ descriptive error variants
- `Result<T>` type alias is defined for ergonomic error handling
- All public types and functions are re-exported from `mod.rs`
- `jsonrpsee` dependency is in `Cargo.toml`
- Module documentation references `design/agent-harness-integration.md`
- ACP-only integration (no adapters in nexum)

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum acp` — Run acp module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds
- `find src/acp/ -type f` — Verify all module files exist
- `grep -r "tokio::process::Command" src/acp/` — Verify subprocess usage
- `grep -r "jsonrpsee" src/acp/` — Verify jsonrpsee integration
- `grep -r "SIGTERM\|SIGKILL" src/acp/` — Verify graceful shutdown pattern
- `grep -r "broadcast" src/acp/` — Verify broadcast channel for events
- `grep -r "thiserror" src/acp/errors.rs` — Verify thiserror usage
- `grep -r "sessions/create\|sessions/message\|sessions/destroy" src/acp/` — Verify ACP methods
- `grep -r "PermissionRequest\|PermissionHandler" src/acp/` — Verify permission handling
- `grep -r "AgentRole\|RoleConfig" src/acp/` — Verify role configuration

## Notes
- This plan depends on chunk 001 (Project Scaffolding) being completed first. The `src/acp/mod.rs` stub must exist before this plan can be executed.
- This plan depends on chunk 002 (Configuration System) being completed. Agent registration data from config is needed for `AgentConfig`.
- The `jsonrpsee` crate's stdio transport feature flag must be verified. As of jsonrpsee 0.24, the feature is `client-transport-stdio`. If this feature doesn't exist, we may need to use a lower-level approach with manual JSON-RPC framing over tokio pipes.
- The mock ACP server for tests can be implemented as a simple shell script or a minimal Rust binary that reads JSON-RPC requests from stdin and writes responses to stdout.
- ACP events flow: agent subprocess → stdout → jsonrpsee → ACPClient → EventStream (broadcast) → WebSocket frontend (future, chunk 13).
- The event stream uses `tokio::sync::broadcast` because multiple subscribers need to receive events (Overlord for status tracking, WebSocket relay for frontend, logging).
- Permission handling follows `design/permissions.md`: default is permission gates (require approval). Yolo mode (auto-approve all) is future work.
- Crash detection relies on the heartbeat mechanism: if no events flow for a configurable duration, the session is considered stale. The Overlord (chunk 004) detects stale heartbeats via `status.json` and triggers recovery.
- The `initialize` method is critical: it negotiates protocol version and capabilities. If the agent doesn't support required capabilities (sessions, streaming), the session cannot be created.
- Multiple instances of the same agent type can run in parallel (each task gets its own subprocess). No coordination needed between instances.
- Consider adding a `SessionManager` struct that tracks all active sessions, for the Overlord to query session state. This would be useful for the concurrency enforcement in chunk 004.
- The `jsonrpsee` stdio transport may require careful handling of pipe EOF. When the subprocess exits, the pipes close, and jsonrpsee should detect this as a transport error. This is the primary crash detection mechanism.
- For the `run_event_loop()` function, consider whether it should be a separate tokio task or integrated into the Builder workflow. The plan leaves it as a method on `ACPSession` for flexibility.
