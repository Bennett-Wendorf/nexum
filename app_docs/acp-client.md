# ACP Client Module

## Overview

The ACP (Agent Communication Protocol) client module provides a complete client for communicating with ACP-compatible coding agents via **JSON-RPC 2.0 over stdio**. It is the fundamental bridge between nexum's orchestration logic (the Overlord and Builder workflow) and the external agents that perform actual coding work.

The module handles:
- **Subprocess spawning** — launching agent processes with piped stdio
- **JSON-RPC communication** — sending typed requests and receiving responses/notifications over stdin/stdout
- **Session lifecycle management** — create, run, interact, complete, destroy
- **Event streaming** — real-time progress, tool calls, results, questions, and permission requests
- **Permission handling** — policy-based approve/deny/pending decisions per agent role
- **Crash detection** — heartbeat monitoring and stale session recovery
- **Per-role configuration** — role-specific timeouts, tool permissions, and permission policies

See `design/agent-harness-integration.md` for the full design specification.

---

## Architecture

The ACP client follows a three-layer pipeline:

```
┌──────────────────────────────────────────────────────────────┐
│                    nexum (Overlord/Builder)                  │
└────────────────────────┬─────────────────────────────────────┘
                         │
          ┌───────────────┼───────────────┐
          │               │               │
   ┌──────▼──────┐ ┌─────▼──────┐ ┌──────▼──────┐
   │  subprocess │ │   client   │ │  session    │
   │  (spawn,    │ │ (JSON-RPC  │ │ (lifecycle, │
   │   shutdown) │ │  requests) │ │  event loop)│
   └──────┬──────┘ └─────┬──────┘ └──────┬──────┘
          │               │               │
          └───────┬───────┴───────┬───────┘
                  │               │
          ┌───────▼───────┐ ┌─────▼──────┐
          │   events.rs   │ │permissions │
          │ (broadcast    │ │ (policy,   │
          │  channel)     │ │  approve)  │
          └───────┬───────┘ └─────┬──────┘
                  │               │
          ┌───────▼────────────────▼───────┐
          │         config.rs              │
          │  (role defaults, overrides)    │
          └────────────────────────────────┘
```

Data flows:
1. **nexum** → calls `ACPSession::create()` → spawns subprocess
2. **subprocess** → pipes stdin/stdout → **ACPClient** (JSON-RPC)
3. **ACPClient** → sends `sessions/create` → agent creates session
4. **Agent** → writes events to stdout → **reader task** → **EventStream** (broadcast)
5. **EventStream** → delivers to subscribers (Overlord, WebSocket frontend, logging)
6. **Permission requests** → evaluated by **PermissionHandler** → response sent via client

---

## Module Structure

| File | Purpose |
|------|---------|
| `src/acp/mod.rs` | Module root; re-exports all public types and functions |
| `src/acp/subprocess.rs` | Agent subprocess spawning, stdio pipes, graceful shutdown, stderr forwarding |
| `src/acp/client.rs` | JSON-RPC client wrapper; typed ACP method calls over stdin/stdout |
| `src/acp/session.rs` | Session lifecycle manager; create → run → interact → complete → destroy |
| `src/acp/events.rs` | Event types (`ACPEvent`), broadcast channel (`EventStream`), helper functions |
| `src/acp/permissions.rs` | Permission request handling; policy evaluation, approve/deny/pending |
| `src/acp/config.rs` | Per-role session configuration; defaults, task overrides, hierarchy |
| `src/acp/errors.rs` | `ACPError` enum (15 variants) via `thiserror`; `Result<T>` type alias |
| `src/acp/tests.rs` | Unit tests for events, permissions, config, errors, serialization |

---

## Session Lifecycle

The `ACPSession` manages the complete lifecycle of an agent interaction:

### States

```
Created → Interacting → Completing → Destroyed
   │
   └──→ Error  (crash, timeout)
```

| State | Description |
|-------|-------------|
| `Created` | Session created; agent is working on the task |
| `Interacting` | Nexum sent a follow-up message or responded to a permission request |
| `Completing` | Agent signaled task completion |
| `Destroyed` | Session destroyed; all resources cleaned up |
| `Error` | Error state (subprocess crash, timeout, etc.) |

### Creation

```rust
let session = ACPSession::create(
    task_id,              // e.g., "task-001"
    AgentRole::Builder,   // role determines permissions/timeout
    prompt,               // task description for the agent
    worktree_path,        // isolated worktree directory
    agent_config,         // binary path, args, env vars
    timeout,              // maximum session duration
    event_stream,         // central broadcast channel
).await?;
```

Creation performs these steps:
1. Spawns the agent subprocess via `spawn_agent()`
2. Creates a `broadcast` channel and constructs `ACPClient`
3. Spawns a stdout reader task for event notifications
4. Sends `initialize` JSON-RPC request (protocol handshake)
5. Calls `sessions/create` with prompt and configuration
6. Returns a fully initialized `ACPSession`

### Interaction

```rust
// Send a follow-up message
session.send_message("Please also handle edge cases", MessageType::FollowUp).await?;

// Respond to a permission request
session.respond_to_permission("req-123", true).await?;

// Respond to a question
session.respond_to_question("q-456", "Use the existing function.").await?;
```

### Event Loop

```rust
match session.run_event_loop(&event_stream).await? {
    Some(ACPEvent::Completion { .. }) => { /* task done */ }
    Some(ACPEvent::PermissionRequest { .. }) => { /* handle permission */ }
    Some(ACPEvent::Question { .. }) => { /* answer question */ }
    None => { /* unexpected */ }
}
```

The event loop:
1. Subscribes to the client's broadcast channel
2. For each event: publishes to central `EventStream`, updates heartbeat, logs via tracing
3. Returns on terminal events (`Completion`, `Error`) or response-required events (`PermissionRequest`, `Question`)
4. Detects crashes via broadcast channel closure
5. Enforces session timeout via `tokio::select!`

### Destruction

```rust
session.destroy().await?;
```

Destruction performs:
1. Sends `sessions/destroy` via JSON-RPC
2. Transitions state to `Destroyed`
3. Shuts down the subprocess (SIGKILL with 5s wait)
4. Aborts the stdout reader task

---

## Event Streaming

### Event Types

All events carry `session_id` and an ISO 8601 `timestamp` for correlation.

| Event | Description |
|-------|-------------|
| `Progress` | Periodic progress update with optional percentage |
| `ToolCall` | Agent invoked a tool (name, arguments, call ID) |
| `ToolResult` | Result from a tool invocation (success, output, error) |
| `Question` | Agent asks a question requiring user input |
| `PermissionRequest` | Agent requests permission for an action |
| `Completion` | Task completed (success, failed, or cancelled) |
| `Error` | Error occurred during agent execution |

### EventStream (Broadcast Channel)

```rust
let stream = EventStream::with_default_capacity(); // capacity: 1024

// Subscribe (multiple subscribers supported)
let mut rx = stream.subscribe();

// Publish events to all subscribers
stream.publish(event)?;
```

The `EventStream` uses `tokio::sync::broadcast` for fan-out delivery to multiple subscribers (Overlord for status tracking, WebSocket relay for the frontend, logging).

**Lagging subscribers**: If a subscriber falls behind the channel buffer, it receives `RecvError::Lagged(n)`. Subscribers should decide whether to skip missed events or disconnect.

### Helper Functions

```rust
is_terminal_event(&event)   // true for Completion and Error
requires_response(&event)   // true for PermissionRequest and Question
session_id(&event)          // extracts session_id from any event variant
log_event(&event)           // logs at appropriate tracing level
```

### Event Creation Helpers

```rust
progress_event(session_id, message, percentage)
tool_call_event(session_id, tool_name, arguments, call_id)
tool_result_event(session_id, call_id, success, output, error)
question_event(session_id, question_id, content, options)
permission_request_event(session_id, request_id, action, details)
completion_event(session_id, status, summary, artifacts)
error_event(session_id, error_code, message)
```

### Tracing Levels

| Event | Level |
|-------|-------|
| Progress | `info` |
| ToolCall | `debug` |
| ToolResult | `debug` |
| Question | `info` |
| PermissionRequest | `info` |
| Completion | `info` |
| Error | `error` |

---

## Permission Handling

### Permission Actions

```rust
enum PermissionAction {
    FileRead,
    FileWrite { path: String },
    CommandExecution { command: String },
    NetworkRequest { url: String, method: String },
    Other { action: String, details: serde_json::Value },
}
```

### Permission Policy

```rust
struct PermissionPolicy {
    auto_approve: Vec<String>,   // Actions auto-approved
    auto_deny: Vec<String>,      // Actions auto-denied
    require_approval: Vec<String>, // Actions requiring explicit approval
}
```

### Policy Evaluation

```rust
let handler = PermissionHandler::new(policy);
let decision = handler.evaluate(&request); // Approved | Denied | Pending
```

Evaluation logic:
1. If action is in `auto_approve` → `Approved`
2. If action is in `auto_deny` → `Denied`
3. Otherwise → `Pending` (requires external decision)

### Handling Decisions

```rust
handle_permission(&session, &request, decision).await?;
```

- `Approved` → sends `permissions/respond` with `approved: true`
- `Denied` → sends `permissions/respond` with `approved: false`
- `Pending` → no response sent (caller must decide externally)

---

## Role Configuration

### Agent Roles

| Role | Description |
|------|-------------|
| `Builder` | Implements features; full file and command access |
| `Reviewer` | Reviews code; read-only access |
| `Planner` | Plans tasks; read + write with approval for writes |
| `SecurityConsultant` | Security analysis; read-only with scan tools |

### Default Timeouts

| Role | Timeout |
|------|---------|
| Builder | 30 minutes (1800s) |
| Reviewer | 15 minutes (900s) |
| Planner | 20 minutes (1200s) |
| SecurityConsultant | 20 minutes (1200s) |

### Default Tool Permissions

| Role | Tools |
|------|-------|
| Builder | file-read, file-write, command-execution, network-request |
| Reviewer | file-read |
| Planner | file-read, file-write |
| SecurityConsultant | file-read, security-scan |

### Default Permission Policies

| Role | Auto-Approve | Auto-Deny | Require Approval |
|------|-------------|-----------|------------------|
| Builder | file-read, file-write | — | command-execution, network-request |
| Reviewer | file-read | file-write, command-execution, network-request | — |
| Planner | file-read | — | file-write, command-execution, network-request |
| SecurityConsultant | file-read | file-write, command-execution, network-request | — |

### Configuration Hierarchy

Configuration flows from global → role → task:

```rust
// 1. Get role defaults
let role_config = default_role_config(AgentRole::Builder);

// 2. Apply task-specific overrides
let overrides = TaskConfigOverrides {
    timeout: Some(Duration::from_secs(600)),
    tool_permissions: Some(vec!["file-read".to_string()]),
    model_preference: Some("gpt-4".to_string()),
};
let merged = merge_with_task_config(&role_config, &overrides);

// 3. Convert to session creation parameters
let params = to_session_params(&merged, prompt, worktree_path);
```

---

## Subprocess Management

### AgentConfig

```rust
struct AgentConfig {
    name: String,              // e.g., "OpenCode"
    binary: PathBuf,           // e.g., "/usr/bin/opencode"
    args: Vec<String>,         // e.g., ["acp"]
    env: HashMap<String, String>,
}
```

### Spawning

```rust
let process = spawn_agent("opencode", &config, worktree_path)?;
```

Spawn performs:
1. Constructs `tokio::process::Command` from config
2. Sets `current_dir` to `worktree_path` (isolated worktree)
3. Configures piped stdio (stdin, stdout, stderr)
4. Applies environment variables
5. Spawns the process
6. Extracts stdin/stdout/stderr handles
7. Spawns a background task to forward stderr to `tracing::info!`
8. Returns `AgentProcess` with all handles

### Graceful Shutdown

```rust
process.shutdown().await?;
```

Shutdown sequence:
1. Sends termination signal via `child.kill()` (SIGKILL on Unix)
2. Waits up to 5 seconds for the process to exit
3. Aborts the stderr forwarding task
4. Clears all handles (stdin, stdout, child)

### Force Kill

```rust
process.force_kill().await?;
```

Immediate SIGKILL without the 5-second grace period.

### Process State

```rust
process.is_alive()       // true if process is still running
process.exit_status()    // Some(ExitStatus) if exited, None if running
```

### Resource Safety

`AgentProcess` implements `Drop` to clean up handles if dropped without explicit shutdown. The `Child` handle is released, and the OS cleans up the zombie process.

---

## Public API

### Re-exported Types (via `acp::`)

| Type | From | Description |
|------|------|-------------|
| `ACPError` | errors | Error enum (15 variants) |
| `Result<T>` | errors | Type alias `Result<T, ACPError>` |
| `AgentConfig` | subprocess | Agent spawn configuration |
| `AgentProcess` | subprocess | Running agent subprocess |
| `ACPClient` | client | JSON-RPC client |
| `ACPCapabilities` | client | Agent capability flags |
| `SessionCreateParams` | client | Session creation parameters |
| `SessionCreateResult` | client | Session creation result |
| `MessageType` | client | Prompt / FollowUp / Feedback |
| `ACPEvent` | events | Event enum (7 variants) |
| `CompletionStatus` | events | Success / Failed / Cancelled |
| `EventStream` | events | Broadcast event channel |
| `ACPSession` | session | Session lifecycle manager |
| `SessionState` | session | Created / Interacting / Completing / Destroyed / Error |
| `AgentRole` | session | Builder / Reviewer / Planner / SecurityConsultant |
| `PermissionAction` | permissions | FileRead / FileWrite / CommandExecution / NetworkRequest / Other |
| `PermissionRequest` | permissions | Permission request struct |
| `PermissionHandler` | permissions | Policy evaluator |
| `PermissionPolicy` | permissions | Auto-approve/deny/approval lists |
| `PermissionDecision` | permissions | Approved / Denied / Pending |
| `RoleConfig` | config | Per-role configuration |
| `TaskConfigOverrides` | config | Task-level overrides |

### Re-exported Functions

| Function | From | Description |
|----------|------|-------------|
| `spawn_agent()` | subprocess | Spawn an agent subprocess |
| `is_terminal_event()` | events | Check if event ends a session |
| `requires_response()` | events | Check if event needs a response |
| `session_id()` | events | Extract session ID from event |
| `log_event()` | events | Log event at appropriate tracing level |
| `handle_permission()` | permissions | Respond to permission request |
| `default_policy_for_role()` | permissions | Generate default policy per role |
| `default_role_config()` | config | Generate default config per role |
| `merge_with_task_config()` | config | Merge role config with task overrides |
| `to_session_params()` | config | Convert config to session params |

### Usage Example

```rust
use nexum::acp::*;

// Configure the agent
let agent_config = AgentConfig {
    name: "opencode".to_string(),
    binary: "/usr/bin/opencode".into(),
    args: vec!["acp".to_string()],
    env: HashMap::new(),
};

// Set up event stream
let event_stream = EventStream::with_default_capacity();

// Create a session
let mut session = ACPSession::create(
    "task-001",
    AgentRole::Builder,
    "Implement the login feature",
    &worktree_path,
    &agent_config,
    Duration::from_secs(1800),
    &event_stream,
).await?;

// Run the event loop
loop {
    match session.run_event_loop(&event_stream).await? {
        Some(ACPEvent::Completion { status, .. }) => {
            println!("Task completed: {:?}", status);
            break;
        }
        Some(ACPEvent::PermissionRequest { request_id, action, .. }) => {
            // Evaluate against policy
            let policy = default_policy_for_role(AgentRole::Builder);
            let handler = PermissionHandler::new(policy);
            let request = PermissionRequest {
                request_id,
                session_id: session.session_id.clone(),
                action: /* parse from event */,
                timestamp: chrono::Utc::now(),
            };
            let decision = handler.evaluate(&request);
            handle_permission(&session, &request, decision).await?;
        }
        Some(event) => {
            log_event(&event);
        }
        None => break,
    }
}

// Clean up
session.destroy().await?;
```

---

## Error Handling

### Error Types

The `ACPError` enum (via `thiserror`) defines 15 error variants:

| Variant | When It Occurs | Handling |
|---------|---------------|----------|
| `SubprocessSpawn` | OS refuses to create child process | Verify binary path and permissions |
| `SubprocessCrash` | Agent exits unexpectedly | Clean up session; consider restart |
| `JsonRpcError` | Agent returns JSON-RPC error | Inspect code/message; retry if appropriate |
| `JsonRpcTransport` | Stdio pipe broken | Tear down session; restart agent |
| `SessionNotFound` | Unknown session ID | Create new session or verify ID |
| `SessionAlreadyExists` | Duplicate session ID | Generate new unique ID |
| `SessionNotActive` | Operation on inactive session | Wait or create new session |
| `SessionTimeout` | Session exceeded timeout | Create new session and retry |
| `InitializeFailed` | Protocol version/capability mismatch | Verify agent compatibility |
| `PermissionDenied` | Permission explicitly denied | Use alternative approach |
| `PermissionTimeout` | No permission response in time | Treat as denial (fail-safe) |
| `EventStreamError` | Broadcast channel error | Reconnect or close |
| `Io(path, source)` | File system I/O error | Check permissions, disk space |
| `ConfigError` | Missing/malformed config | Fix configuration |
| `Timeout` | Generic operation timeout | Retry with longer timeout |

### Conversions

- `From<std::io::Error>` — allows `?` operator on I/O operations
- `Result<T>` type alias for ergonomic error handling

---

## Crash Detection

### Heartbeat Mechanism

Each session tracks a `heartbeat` (`Instant`) that is updated every time an event is received from the agent. A stale heartbeat indicates the agent may have crashed.

```rust
let is_stale = session.check_heartbeat(Duration::from_secs(30));
```

### Crash Detection in Event Loop

The `run_event_loop()` detects crashes through two mechanisms:

1. **Broadcast channel closure**: If the agent subprocess dies, the stdout reader task terminates, closing the broadcast channel. The event loop receives `RecvError::Closed` and returns `ACPError::SubprocessCrash`.

2. **Timeout enforcement**: The event loop uses `tokio::select!` with a timeout arm. If no events arrive within the session's configured timeout, it returns `ACPError::SessionTimeout`.

### Recovery Strategy

When a crash is detected:
1. The session state transitions to `Error`
2. The caller should clean up the session via `destroy()`
3. Optionally, a new session can be created to retry the task

---

## Testing

The test suite (`src/acp/tests.rs`) covers the following areas:

### Event Tests (12 tests)
- Deserialization of all event types (Progress, ToolCall, Completion, PermissionRequest, Error)
- `is_terminal_event()` detection
- `requires_response()` detection
- `session_id()` extraction from all variants
- `log_event()` compilation
- EventStream publish/subscribe with single and multiple subscribers
- Serialization roundtrip for events and CompletionStatus
- ToolResult and Question event deserialization
- Edge cases: missing optional fields, empty completion

### Permission Tests (7 tests)
- Auto-approve policy evaluation
- Auto-deny policy evaluation
- Pending (require approval) evaluation
- Default policies for Builder, Reviewer, Planner, SecurityConsultant
- Custom action handling via `Other` variant

### Configuration Tests (6 tests)
- Default role config for Builder (30min timeout, full tools)
- Default role config for Reviewer (15min timeout, read-only)
- Default role config for Planner (20min timeout)
- Default role config for SecurityConsultant (20min timeout)
- Task config override merging
- Conversion to `SessionCreateParams`
- Empty overrides (no-op merge)

### Error Tests (4 tests)
- `SubprocessSpawn` error display message
- `SessionTimeout` error display message
- `From<io::Error>` conversion
- `Result<T>` type alias

### Running Tests

```bash
cargo test --package nexum acp
```

### Test Coverage Summary

| Module | Tests | Coverage |
|--------|-------|----------|
| Events | 12 | Deserialization, helpers, broadcast, serialization |
| Permissions | 7 | Policy evaluation, default policies, custom actions |
| Configuration | 6 | Role defaults, overrides, session params |
| Errors | 4 | Display, conversions, result type |
| **Total** | **29** | Unit tests (subprocess/JSON-RPC require integration tests) |

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime, process management, I/O, broadcast channels |
| `serde` + `serde_json` | JSON serialization/deserialization for events and requests |
| `thiserror` | Error derive macro for `ACPError` |
| `chrono` | Timestamp handling (RFC 3339 / ISO 8601) |
| `tracing` | Structured logging for events and stderr forwarding |

Note: The current implementation uses manual JSON-RPC framing over stdio pipes rather than the `jsonrpsee` crate. The `ACPClient` writes JSON-RPC requests directly to stdin and reads responses from stdout via a background reader task.
