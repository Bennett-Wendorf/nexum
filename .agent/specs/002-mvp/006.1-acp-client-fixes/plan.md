# Plan: 006.1 - ACP Client Code Review Fixes

## Task Description
Fix four code review findings in the ACP client module that prevent correct JSON-RPC request/response handling, protocol handshake validation, proper timeout management, and reliable event delivery during session creation.

## Objective
Implement a correct request-response mechanism in `ACPClient`, validate the protocol handshake in `initialize()`, fix the event loop timeout in `ACPSession::run_event_loop`, and prevent event loss during `ACPSession::create`.

## Problem Statement

### Finding 11: `sessions_create` fabricates session ID
`ACPClient::sessions_create` sends a JSON-RPC request but returns a fake session ID from an internal counter (`format!("session-{}", self.next_id())`) instead of reading the agent's actual response. The `spawn_reader` task only parses events (notifications without an `"id"` field) and ignores JSON-RPC responses entirely. Downstream callers use the fabricated ID, which will not match the agent's actual session.

### Finding 12: `initialize()` performs no handshake validation
`ACPClient::initialize` sends the protocol handshake request but does not read or validate the agent's response. The protocol version is hardcoded to `"1.0"` and capabilities are hardcoded defaults. Version mismatches and capability incompatibilities go undetected.

### Finding 13: Event loop timeout measures from session creation, not last activity
`ACPSession::run_event_loop` uses `tokio::time::sleep(self.timeout)` which starts a one-shot timer when the event loop begins. The timer does not reset on each event. A session with a 30-minute timeout will be killed after 30 minutes of total runtime regardless of activity, even if the agent is actively producing events.

### Finding 14: Events during `ACPSession::create` are lost
The broadcast channel is created in `ACPSession::create` but no subscriber exists until `run_event_loop` calls `self.client.event_sender().subscribe()` much later. Events emitted by the agent during `initialize()` and `sessions_create()` (lines 92–101) are silently dropped because the broadcast channel has zero subscribers.

## Solution Approach

### Findings 11 & 12: Request-Response Mechanism
Both findings share the same root cause: no request-response correlation. The fix introduces a pending-response tracking system:

1. Add a `HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>` pending-responses map to `ACPClient`
2. `send_request()` creates a oneshot channel, stores the sender keyed by request ID, and returns the receiver
3. `spawn_reader()` is modified to also parse JSON-RPC responses (objects with an `"id"` field) and dispatch them to the matching oneshot sender
4. `initialize()` awaits the response, validates `protocolVersion`, and extracts `capabilities`
5. `sessions_create()` awaits the response and parses `SessionCreateResult` from it

### Finding 13: Resettable Timeout
Replace the one-shot `tokio::time::sleep(self.timeout)` with a `tokio::time::Sleep` that is reset via `.reset(instant)` on each received event. The timeout deadline is calculated from the session's `created_at` time plus `timeout`, ensuring the total session lifetime is bounded while still allowing active sessions to continue.

### Finding 14: MPSC Channel for Client-to-Session Events
Replace the `broadcast` channel between the reader task and the session with an `mpsc` channel. The `mpsc` channel guarantees no message loss (the sender blocks when the buffer is full rather than dropping messages). The session's event loop receives from the `mpsc` channel and then publishes to the central `EventStream` broadcast channel for fan-out to other subscribers.

## Relevant Files

### Files to Modify
- `src/acp/client.rs` — Add pending-response tracking, fix `initialize()` and `sessions_create()`, change event channel from broadcast to mpsc
- `src/acp/session.rs` — Fix `ACPSession::create` to use mpsc, fix `run_event_loop` timeout to be resettable

### Files to Reference (no changes)
- `src/acp/errors.rs` — Error types already cover needed variants (`InitializeFailed`, `JsonRpcError`, `JsonRpcTransport`)
- `src/acp/events.rs` — Event types and `EventStream` (broadcast for central fan-out) remain unchanged
- `src/acp/subprocess.rs` — No changes needed
- `src/acp/mod.rs` — May need updated re-exports if new public types are added

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: acp-client-fixer
  - Role: Implement request-response mechanism, handshake validation, resettable timeout, and mpsc event channel
  - Agent: builder

- **Validator**
  - Name: acp-fixes-validator
  - Role: Verify all four findings are resolved, tests pass, no regressions
  - Agent: validator

- **Documenter**
  - Name: acp-fixes-documenter
  - Role: Document the fixes and updated architecture
  - Agent: documenter

## Step by Step Tasks

### 1. Implement Request-Response Mechanism in ACPClient
- **Task ID**: acp-request-response
- **Depends On**: none
- **Assigned To**: acp-client-fixer
- **Agent**: builder
- **Actions**:
  - In `src/acp/client.rs`, add a pending-responses map to `ACPClient`:
    ```rust
    pub struct ACPClient {
        agent_id: String,
        protocol_version: String,
        capabilities: ACPCapabilities,
        writer: tokio::sync::Mutex<BufWriter<ChildStdin>>,
        id_counter: AtomicU64,
        // NEW: pending response tracking
        pending_responses: tokio::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>,
        // CHANGED: mpsc instead of broadcast for client-to-session delivery
        event_sender: tokio::sync::mpsc::Sender<ACPEvent>,
        event_receiver: tokio::sync::mpsc::Receiver<ACPEvent>,
    }
    ```
  - Update `ACPClient::new()` to initialize the pending-responses map and create the mpsc channel:
    ```rust
    pub fn new(agent_id: String, stdin: ChildStdin) -> Self {
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1024);
        Self {
            agent_id,
            protocol_version: String::new(),
            capabilities: ACPCapabilities {
                sessions: false,
                streaming: false,
                permissions: false,
                tools: vec![],
            },
            writer: tokio::sync::Mutex::new(BufWriter::new(stdin)),
            id_counter: AtomicU64::new(0),
            pending_responses: tokio::sync::Mutex::new(HashMap::new()),
            event_sender,
            event_receiver,
        }
    }
    ```
    Note: `protocol_version` starts empty and `capabilities` starts with all `false` — these are populated by `initialize()`.

  - Modify `send_request()` to create a oneshot channel, register the sender, and return the receiver:
    ```rust
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> std::io::Result<tokio::sync::oneshot::Receiver<serde_json::Value>> {
        let id = self.next_id();
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_responses.lock().await;
            pending.insert(id, tx);
        }
        // ... write JSON-RPC request to stdin (same as before) ...
        Ok(rx)
    }
    ```

  - Modify `spawn_reader()` to handle both events (notifications) and responses (requests with `"id"` field):
    ```rust
    pub fn spawn_reader(
        &self,
        stdout: tokio::process::ChildStdout,
    ) -> tokio::task::JoinHandle<()> {
        let sender = self.event_sender.clone();
        let pending_responses = self.pending_responses.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(obj) = value.as_object() {
                        if let Some(id_value) = obj.get("id") {
                            // This is a response — dispatch to pending request
                            if let Some(id) = id_value.as_u64() {
                                let tx = {
                                    let mut pending = pending_responses.lock().await;
                                    pending.remove(&id)
                                };
                                if let Some(tx) = tx {
                                    let _ = tx.send(value);
                                }
                            }
                            return; // response dispatched, continue loop
                        }
                        // No "id" field — it's a notification/event
                        if let Ok(event) = serde_json::from_str::<ACPEvent>(&line) {
                            let _ = sender.send(event).await;
                        } else {
                            tracing::debug!(raw_notification = %line, "unrecognized notification");
                        }
                    }
                }
            }
        })
    }
    ```

  - Remove the `event_sender` broadcast channel from `ACPClient::new()` parameters and from `ACPSession::create()`.

  - Add a method to get the event receiver:
    ```rust
    pub fn event_receiver(&self) -> &tokio::sync::mpsc::Receiver<ACPEvent> {
        &self.event_receiver
    }
    ```
    Note: Since `mpsc::Receiver` can only be moved once, we'll need to restructure slightly — the session takes ownership of the receiver. See Task 3.

- **Acceptance Criteria**:
  - `ACPClient` has a `pending_responses` HashMap for request-response correlation
  - `send_request()` returns a `oneshot::Receiver<serde_json::Value>` instead of just the request ID
  - `spawn_reader()` dispatches responses (with `"id"` field) to the matching oneshot sender
  - `spawn_reader()` sends events (without `"id"` field) to the mpsc channel
  - The `ACPClient` no longer uses `broadcast::Sender<ACPEvent>` — uses `mpsc::Sender<ACPEvent>` instead
  - The `event_sender` parameter is removed from `ACPClient::new()`
  - The mpsc channel is created internally in `ACPClient::new()`

### 2. Fix `initialize()` to Validate Handshake Response
- **Task ID**: acp-initialize-validation
- **Depends On**: acp-request-response
- **Assigned To**: acp-client-fixer
- **Agent**: builder
- **Actions**:
  - Rewrite `initialize()` in `src/acp/client.rs` to await the response and validate:
    ```rust
    pub async fn initialize(&self) -> Result<()> {
        let rx = self.send_request(
            "initialize",
            serde_json::json!({
                "protocolVersion": "1.0",
                "clientName": "nexum",
            }),
        ).await.map_err(|e| ACPError::JsonRpcTransport {
            source: Box::new(e),
        })?;

        // Await the response
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            rx,
        ).await {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => {
                return Err(ACPError::InitializeFailed {
                    agent_id: self.agent_id.clone(),
                    reason: "response channel closed".to_string(),
                });
            }
            Err(_) => {
                return Err(ACPError::Timeout {
                    operation: "initialize".to_string(),
                    duration: std::time::Duration::from_secs(10),
                });
            }
        };

        // Parse the response
        let result = response.get("result")
            .ok_or_else(|| ACPError::InitializeFailed {
                agent_id: self.agent_id.clone(),
                reason: "response missing 'result' field".to_string(),
            })?;

        // Validate protocol version
        let agent_version = result.get("protocolVersion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ACPError::InitializeFailed {
                agent_id: self.agent_id.clone(),
                reason: "response missing 'protocolVersion'".to_string(),
            })?;

        if agent_version != "1.0" {
            return Err(ACPError::InitializeFailed {
                agent_id: self.agent_id.clone(),
                reason: format!(
                    "protocol version mismatch: nexum requires 1.0, agent reports {}",
                    agent_version
                ),
            });
        }

        // Extract capabilities
        let caps_obj = result.get("capabilities")
            .and_then(|c| c.as_object())
            .ok_or_else(|| ACPError::InitializeFailed {
                agent_id: self.agent_id.clone(),
                reason: "response missing 'capabilities'".to_string(),
            })?;

        // Note: we need to update the struct fields. Since &self doesn't allow mutation,
        // we'll use interior mutability (RwLock) for protocol_version and capabilities.
        // See note below.

        Ok(())
    }
    ```

  - Add interior mutability for `protocol_version` and `capabilities` since `initialize()` takes `&self`:
    ```rust
    pub struct ACPClient {
        // ... existing fields ...
        protocol_version: std::sync::RwLock<String>,
        capabilities: std::sync::RwLock<ACPCapabilities>,
    }
    ```
    Update `new()`, `initialize()`, `protocol_version()`, and `capabilities()` accordingly.

  - Update `protocol_version()` and `capabilities()` to read from the RwLock:
    ```rust
    pub fn protocol_version(&self) -> String {
        self.protocol_version.read().unwrap().clone()
    }
    pub fn capabilities(&self) -> ACPCapabilities {
        self.capabilities.read().unwrap().clone()
    }
    ```

- **Acceptance Criteria**:
  - `initialize()` sends the handshake request and awaits the response via oneshot channel
  - `initialize()` applies a 10-second timeout to the handshake
  - `initialize()` validates the `protocolVersion` field matches `"1.0"`
  - `initialize()` extracts and stores the agent's `capabilities` from the response
  - `initialize()` returns `ACPError::InitializeFailed` on version mismatch
  - `initialize()` returns `ACPError::InitializeFailed` on missing fields
  - `initialize()` returns `ACPError::Timeout` on handshake timeout
  - `protocol_version` and `capabilities` use interior mutability (`RwLock`) for `&self` access
  - `protocol_version()` and `capabilities()` return the values set by `initialize()`
  - Capabilities start as all `false` until `initialize()` populates them

### 3. Fix `sessions_create()` to Read Agent Response
- **Task ID**: acp-sessions-create-response
- **Depends On**: acp-request-response
- **Assigned To**: acp-client-fixer
- **Agent**: builder
- **Actions**:
  - Rewrite `sessions_create()` in `src/acp/client.rs` to await the response:
    ```rust
    pub async fn sessions_create(&self, params: SessionCreateParams) -> Result<SessionCreateResult> {
        let params_value = serde_json::to_value(params).map_err(|e| ACPError::JsonRpcTransport {
            source: Box::new(e),
        })?;

        let rx = self.send_request("sessions/create", params_value)
            .await
            .map_err(|e| ACPError::JsonRpcTransport {
                source: Box::new(e),
            })?;

        // Await the response with timeout
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            rx,
        ).await {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => {
                return Err(ACPError::JsonRpcError {
                    method: "sessions/create".to_string(),
                    code: -32603,
                    message: "response channel closed".to_string(),
                });
            }
            Err(_) => {
                return Err(ACPError::Timeout {
                    operation: "sessions/create".to_string(),
                    duration: std::time::Duration::from_secs(30),
                });
            }
        };

        // Check for JSON-RPC error response
        if let Some(code) = response.get("error").and_then(|e| e.get("code")).and_then(|c| c.as_i64()) {
            let message = response.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(ACPError::JsonRpcError {
                method: "sessions/create".to_string(),
                code: code as i32,
                message,
            });
        }

        // Parse the result
        let result = response.get("result")
            .ok_or_else(|| ACPError::JsonRpcError {
                method: "sessions/create".to_string(),
                code: -32603,
                message: "response missing 'result' field".to_string(),
            })?;

        let session_create_result: SessionCreateResult =
            serde_json::from_value(result.clone())
            .map_err(|e| ACPError::JsonRpcError {
                method: "sessions/create".to_string(),
                code: -32603,
                message: format!("failed to parse session create result: {}", e),
            })?;

        Ok(session_create_result)
    }
    ```

  - The fabricated `format!("session-{}", self.next_id())` is completely removed.

- **Acceptance Criteria**:
  - `sessions_create()` sends the request and awaits the response via oneshot channel
  - `sessions_create()` applies a 30-second timeout to the request
  - `sessions_create()` parses the response into `SessionCreateResult`
  - `sessions_create()` returns the actual `session_id` from the agent's response
  - `sessions_create()` detects and returns `ACPError::JsonRpcError` on error responses
  - `sessions_create()` detects and returns `ACPError::Timeout` on timeout
  - The fabricated session ID is completely removed
  - All other methods (`sessions_message`, `sessions_destroy`, `permission_respond`) remain fire-and-forget since they don't require parsed responses (they return `Result<()>`)

### 4. Fix Event Loop Timeout to Be Resettable
- **Task ID**: acp-resettable-timeout
- **Depends On**: none
- **Assigned To**: acp-client-fixer
- **Agent**: builder
- **Actions**:
  - In `src/acp/session.rs`, modify `run_event_loop()` to use a resettable timeout:
    ```rust
    pub async fn run_event_loop(&mut self, event_stream: &EventStream) -> Result<Option<ACPEvent>> {
        // Take ownership of the mpsc receiver
        let mut rx = {
            // We need to get the receiver from the client.
            // Since mpsc::Receiver can only be moved once, we restructure ACPClient
            // to allow the session to take the receiver.
            // For now, assume we have a method like `take_event_receiver()`.
            // See note in Task 1 about this.
        };

        // Create a timeout deadline from session creation time
        let deadline = self.created_at + chrono::Duration::from_std(self.timeout).unwrap();
        let mut timeout_sleep = tokio::time::sleep(Duration::from_secs(0)); // immediately ready

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
                // Check for absolute timeout (from session creation)
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
    ```

  - Wait — the finding says the timeout should reset on each event (activity-based), not be absolute from creation. Let me re-read the finding:

    > Event loop timeout measures from session creation, not last activity.
    > `tokio::time::sleep(self.timeout)` starts timer when event loop begins, not on last event.
    > Suggestion: Use resettable timer that resets on each event.

    The current code uses `tokio::time::sleep(self.timeout)` which is a one-shot sleep starting from when the event loop begins. The fix should reset the sleep on each event to implement an **idle timeout** (timeout from last activity).

  - Revised approach — use `tokio::time::Sleep::reset()` to implement idle timeout:
    ```rust
    pub async fn run_event_loop(&mut self, event_stream: &EventStream) -> Result<Option<ACPEvent>> {
        let mut rx = self.client.take_event_receiver();
        let mut timeout_sleep = tokio::time::sleep(self.timeout);

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Some(event) => {
                            event_stream.publish(event.clone())?;
                            self.heartbeat = Instant::now();
                            log_event(&event);

                            // Reset the idle timeout on each event
                            let now = Instant::now();
                            let wake_at = now + self.timeout;
                            timeout_sleep.reset(wake_at);

                            if is_terminal_event(&event) {
                                return Ok(Some(event));
                            }
                            if requires_response(&event) {
                                return Ok(Some(event));
                            }
                        }
                        None => {
                            return Err(ACPError::SubprocessCrash {
                                agent_id: self.agent_process.agent_id.clone(),
                                exit_code: None,
                            });
                        }
                    }
                }
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
    ```

  - Add `take_event_receiver()` method to `ACPClient`:
    ```rust
    /// Take ownership of the event receiver. Can only be called once.
    pub fn take_event_receiver(&self) -> tokio::sync::mpsc::Receiver<ACPEvent> {
        // Since the receiver is stored in the struct and can only be moved once,
        // we use a OnceLock or Option pattern.
        // Alternative: store the receiver in an Option<Receiver> and take it.
        self.event_receiver.take()
    }
    ```
    This requires changing `event_receiver` to `Option<mpsc::Receiver<ACPEvent>>`.

- **Acceptance Criteria**:
  - `run_event_loop()` uses `tokio::time::Sleep::reset()` to reset the timeout on each event
  - The timeout measures idle time (time since last event), not total runtime
  - Actively producing sessions are not killed prematurely
  - Inactive sessions are still killed after `self.timeout` of no events
  - The `tokio::time::sleep(self.timeout)` one-shot pattern is completely replaced
  - The periodic 5-second alive check is removed (crash detection happens via mpsc channel close)

### 5. Fix Event Loss During Session Creation (MPSC Channel)
- **Task ID**: acp-mpsc-event-channel
- **Depends On**: acp-request-response (Task 1 already changes broadcast to mpsc)
- **Assigned To**: acp-client-fixer
- **Agent**: builder
- **Actions**:
  - In `src/acp/session.rs`, modify `ACPSession::create()` to use the mpsc-based event channel:
    ```rust
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

        // 2. Create client (no longer needs event_sender parameter)
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

        // 6. Drain any events that arrived during initialize/sessions_create
        // (they are in the mpsc buffer, will be consumed by run_event_loop)

        // 7. Return the session
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
    ```

  - The key insight: since the mpsc channel is created inside `ACPClient::new()` and the reader task starts writing to it immediately, events during `initialize()` and `sessions_create()` are buffered in the mpsc channel (up to 1024 events). When `run_event_loop()` takes the receiver, it will process all buffered events.

  - Update `ACPSession` struct to remove the `event_stream_rx` field (no longer needed — events come from the mpsc channel inside the client):
    ```rust
    pub struct ACPSession {
        pub session_id: String,
        pub agent_process: AgentProcess,
        pub client: ACPClient,
        pub state: SessionState,
        pub task_id: String,
        pub role: AgentRole,
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub timeout: Duration,
        pub heartbeat: Instant,
        reader_handle: Option<tokio::task::JoinHandle<()>>,
        // REMOVED: event_stream_rx (no longer needed)
    }
    ```

  - Update `run_event_loop()` to take the receiver from the client:
    ```rust
    pub async fn run_event_loop(&mut self, event_stream: &EventStream) -> Result<Option<ACPEvent>> {
        let mut rx = self.client.take_event_receiver();
        // ... rest of the method as described in Task 4
    }
    ```

  - The `_event_stream` parameter in `ACPSession::create()` can remain for now (it's used in `run_event_loop`), but the broadcast channel creation code (`tokio::sync::broadcast::channel(1024)`) is removed from `create()`.

- **Acceptance Criteria**:
  - `ACPSession::create()` no longer creates a broadcast channel
  - `ACPClient::new()` no longer takes an `event_sender` parameter
  - The mpsc channel is created internally in `ACPClient::new()`
  - Events during `initialize()` and `sessions_create()` are buffered in the mpsc channel
  - `run_event_loop()` takes the mpsc receiver from the client
  - The `event_stream_rx` field is removed from `ACPSession`
  - No events are lost during session creation
  - The mpsc channel capacity is 1024 (same as the old broadcast channel)

### 6. Update ACPClient API for Interior Mutability and Receiver Take
- **Task ID**: acp-client-api-updates
- **Depends On**: acp-request-response, acp-initialize-validation
- **Assigned To**: acp-client-fixer
- **Agent**: builder
- **Actions**:
  - Consolidate all `ACPClient` struct and method changes:
    - `protocol_version: std::sync::RwLock<String>` (interior mutability for `initialize()`)
    - `capabilities: std::sync::RwLock<ACPCapabilities>` (interior mutability for `initialize()`)
    - `event_receiver: Option<tokio::sync::mpsc::Receiver<ACPEvent>>` (takeable once)
    - `pending_responses: tokio::sync::Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>`
    - Remove `event_sender: broadcast::Sender<ACPEvent>` → replace with `event_sender: tokio::sync::mpsc::Sender<ACPEvent>`

  - Add `take_event_receiver()` method:
    ```rust
    pub fn take_event_receiver(&self) -> Option<tokio::sync::mpsc::Receiver<ACPEvent>> {
        self.event_receiver.take()
    }
    ```

  - Update `protocol_version()` to return `String` (clone from RwLock) or `&str` with lifetime tied to RwLock guard. Since `&str` with RwLock guard is awkward, return `String`:
    ```rust
    pub fn protocol_version(&self) -> String {
        self.protocol_version.read().unwrap().clone()
    }
    ```

  - Update `capabilities()` similarly:
    ```rust
    pub fn capabilities(&self) -> ACPCapabilities {
        self.capabilities.read().unwrap().clone()
    }
    ```

  - Remove the `event_sender()` method (no longer needed — the sender is internal).

  - Update imports in `client.rs`:
    - Add: `tokio::sync::oneshot`, `tokio::sync::mpsc`, `std::collections::HashMap`, `std::sync::RwLock`
    - Remove: `tokio::sync::broadcast`

  - Update `src/acp/mod.rs` re-exports if any public API changed.

- **Acceptance Criteria**:
  - `ACPClient` struct compiles with all new fields
  - `protocol_version` and `capabilities` use `RwLock` for interior mutability
  - `event_receiver` is `Option<mpsc::Receiver<>>` and can be taken once
  - `take_event_receiver()` returns the receiver or `None` if already taken
  - `protocol_version()` returns the current protocol version as `String`
  - `capabilities()` returns the current capabilities as `ACPCapabilities`
  - `event_sender()` method is removed
  - `event_sender` parameter is removed from `ACPClient::new()`
  - All imports are correct (no unused import warnings)
  - `mod.rs` re-exports are still valid

### 7. Compile and Test
- **Task ID**: acp-fixes-compile-test
- **Depends On**: acp-request-response, acp-initialize-validation, acp-sessions-create-response, acp-resettable-timeout, acp-mpsc-event-channel, acp-client-api-updates
- **Assigned To**: acp-client-fixer
- **Agent**: builder
- **Actions**:
  - Run `cargo check` to verify compilation
  - Fix any compilation errors
  - Run `cargo clippy --package nexum` to check for lint warnings
  - Run `cargo test --package nexum acp` to run existing tests
  - Fix any test failures caused by API changes (e.g., removed `event_sender` parameter)
  - Add new tests:
    - `test_initialize_validates_version` — Verify `initialize()` validates protocol version
    - `test_initialize_rejects_mismatch` — Verify `initialize()` rejects version mismatch
    - `test_sessions_create_reads_response` — Verify `sessions_create()` returns the agent's session ID
    - `test_resettable_timeout` — Verify timeout resets on each event
    - `test_events_buffered_during_create` — Verify events during create are not lost

- **Acceptance Criteria**:
  - `cargo check` succeeds with no errors
  - `cargo clippy --package nexum` produces no warnings in the acp module
  - `cargo test --package nexum acp` passes all tests
  - New tests cover all four findings

### 8. Final Validation
- **Task ID**: validate-all
- **Depends On**: acp-fixes-compile-test
- **Assigned To**: acp-fixes-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum acp` — all acp tests must pass
  - Run `cargo clippy --package nexum` — no warnings in acp module

  **Finding 11 verification**:
  - `sessions_create()` no longer contains `format!("session-{}", ...)` pattern
  - `sessions_create()` contains `send_request(...).await` followed by response parsing
  - `sessions_create()` parses `SessionCreateResult` from the JSON-RPC response
  - `sessions_create()` returns the `session_id` from the parsed response

  **Finding 12 verification**:
  - `initialize()` no longer contains "For now, we use defaults" comment
  - `initialize()` contains `send_request(...).await` followed by response parsing
  - `initialize()` validates `protocolVersion` field
  - `initialize()` extracts and stores `capabilities` from the response
  - `initialize()` returns `ACPError::InitializeFailed` on version mismatch
  - `initialize()` returns `ACPError::Timeout` on handshake timeout

  **Finding 13 verification**:
  - `run_event_loop()` no longer contains `tokio::time::sleep(self.timeout)` one-shot pattern
  - `run_event_loop()` contains `timeout_sleep.reset(...)` call inside the event branch
  - The timeout resets on each received event (idle timeout semantics)

  **Finding 14 verification**:
  - `ACPSession::create()` no longer creates a `broadcast::channel`
  - `ACPClient::new()` no longer takes `event_sender` parameter
  - `ACPClient` uses `mpsc::Sender<ACPEvent>` instead of `broadcast::Sender<ACPEvent>`
  - `run_event_loop()` takes the mpsc receiver from the client
  - The `event_stream_rx` field is removed from `ACPSession`

  **Regression checks**:
  - `sessions_message()`, `sessions_destroy()`, `permission_respond()` still compile and work
  - `spawn_reader()` still handles events (notifications) correctly
  - `EventStream` broadcast channel (central fan-out) is still used in `run_event_loop`
  - `ACPSession` struct fields are correct
  - `mod.rs` re-exports are valid

### 9. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: acp-fixes-documenter
- **Agent**: documenter
- **Actions**:
  - Document the request-response mechanism in `ACPClient` (pending responses map, oneshot channels)
  - Document the handshake validation in `initialize()` (version check, capability extraction)
  - Document the resettable timeout in `run_event_loop()` (idle timeout semantics)
  - Document the mpsc event channel (client-to-session delivery without loss)
  - Update any existing documentation that references the old patterns

## Acceptance Criteria
- `cargo check` succeeds with no errors in the acp module
- `cargo test --package nexum acp` passes all tests
- `cargo clippy --package nexum` produces no warnings in acp module
- `sessions_create()` returns the actual session ID from the agent's JSON-RPC response
- `initialize()` validates protocol version and extracts agent capabilities
- `initialize()` returns `ACPError::InitializeFailed` on version mismatch
- `initialize()` returns `ACPError::Timeout` on handshake timeout
- `run_event_loop()` uses a resettable idle timeout (resets on each event)
- Events during `ACPSession::create` are not lost (buffered in mpsc channel)
- `ACPClient` uses `mpsc` for client-to-session event delivery
- `ACPClient` uses `HashMap<u64, oneshot::Sender>` for request-response correlation
- `protocol_version` and `capabilities` use interior mutability (`RwLock`)
- No regression in `sessions_message`, `sessions_destroy`, `permission_respond`
- No regression in `EventStream` central broadcast fan-out

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum acp` — Run acp module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `grep -n "session-{}" src/acp/client.rs` — Should return nothing (no fabricated IDs)
- `grep -n "send_request" src/acp/client.rs` — Should show oneshot pattern in initialize and sessions_create
- `grep -n "reset" src/acp/session.rs` — Should show timeout reset in run_event_loop
- `grep -n "broadcast" src/acp/client.rs` — Should return nothing (replaced with mpsc)
- `grep -n "mpsc" src/acp/client.rs` — Should show mpsc usage
- `grep -n "oneshot" src/acp/client.rs` — Should show oneshot usage for responses
- `grep -n "pending_responses" src/acp/client.rs` — Should show pending response tracking
- `grep -n "RwLock" src/acp/client.rs` — Should show interior mutability for protocol_version and capabilities

## Notes
- The `spawn_reader` task must handle both responses (with `"id"` field) and events (without `"id"` field). The dispatch logic must not hold locks longer than necessary to avoid blocking other requests.
- The oneshot channel approach means each pending response sender is consumed when the response arrives. If a request times out before the response arrives, the sender is dropped and the response is silently discarded (which is correct — the caller already timed out).
- The `mpsc` channel capacity of 1024 should be sufficient for event buffering during the brief initialization window. If the agent produces events faster than the channel can drain, the sender will block, which may cause backpressure on the agent's stdout pipe. This is acceptable for the MVP.
- The `RwLock` for `protocol_version` and `capabilities` is a pragmatic solution given `initialize()` takes `&self`. An alternative would be to use `&mut self` for `initialize()`, but that would require restructuring the session creation flow.
- The `take_event_receiver()` pattern means the receiver can only be consumed once. This matches the usage pattern: `run_event_loop()` is called once per session lifecycle.
- The `sessions_message`, `sessions_destroy`, and `permission_respond` methods are fire-and-forget notifications in the ACP protocol. They do not need to await responses. The current implementation is correct for these methods — they only need to verify the request was sent successfully.
