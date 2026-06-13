# ACP Client Fixes

## Overview

This document describes the fixes applied to the ACP (Agent Communication Protocol) client module to resolve four code review findings that prevented correct JSON-RPC request/response handling, protocol handshake validation, proper timeout management, and reliable event delivery during session creation. The fixes introduce a proper request-response correlation mechanism, validate the protocol handshake, implement a resettable idle timeout, and replace the event delivery channel to prevent message loss.

## What Was Built

The ACP client was restructured to implement a complete JSON-RPC 2.0 request-response cycle:

- **Request-response mechanism**: `ACPClient` now tracks pending requests in a `HashMap<String, oneshot::Sender>` keyed by JSON-RPC request ID. The `spawn_reader` task dispatches incoming responses to the matching oneshot channel, enabling `initialize()` and `sessions_create()` to await actual agent responses.

- **Handshake validation**: `initialize()` now sends the protocol handshake, awaits the response with a 10-second timeout, validates the agent's `protocolVersion` matches `"1.0"`, and extracts the agent's `capabilities`.

- **Resettable idle timeout**: `ACPSession::run_event_loop()` uses a pinned `tokio::time::Sleep` that resets on each received event, implementing true idle-timeout semantics instead of a fixed-duration timer.

- **Loss-free event delivery**: The client-to-session event channel was changed from `broadcast` to `mpsc` with a 1024-buffer. This buffers all events emitted during `initialize()` and `sessions_create()` so none are lost before the event loop starts consuming.

Additionally, several code review improvements were applied: fire-and-forget memory leak prevention, JSON-RPC error response handling, safe error handling replacing panics, `OnceLock` replacing `RwLock`, named constants replacing magic numbers, relaxed atomic ordering, and per-request string allocation elimination.

## Technical Implementation

### Files Modified

| File | Description |
|------|-------------|
| `src/acp/client.rs` | Request-response mechanism, handshake validation, mpsc channel, notification separation |
| `src/acp/session.rs` | Resettable idle timeout, mpsc receiver consumption, event loop restructuring |
| `src/acp/mod.rs` | Re-exports restored for public API stability |

### Request-Response Mechanism (`client.rs`)

The `ACPClient` struct contains:

```rust
pub struct ACPClient {
    agent_id: String,
    protocol_version: OnceLock<String>,
    capabilities: OnceLock<ACPCapabilities>,
    writer: tokio::sync::Mutex<BufWriter<ChildStdin>>,
    id_counter: AtomicU64,
    pending_responses: Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<Value>>>>,
    event_sender: tokio::sync::mpsc::Sender<ACPEvent>,
    event_receiver: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<ACPEvent>>>,
}
```

**Key design decisions:**

- **`pending_responses`** — A `HashMap<String, oneshot::Sender<serde_json::Value>>` wrapped in `Arc<tokio::sync::Mutex<>>`. The `Arc` allows cloning for the reader task. Request IDs are stored as `String` keys to support both numeric and string IDs per RFC 7464.

- **`send_request()`** — Creates a oneshot channel, registers the sender in the pending-responses map keyed by the serialized request ID, writes the JSON-RPC request to stdin, and returns the receiver. The caller `await`s the receiver (with timeout) to get the response.

- **`send_notification()`** — A separate method for fire-and-forget messages (`sessions/message`, `sessions/destroy`, `permissions/respond`). It does not register a pending response entry, preventing memory leaks from accumulated oneshot senders.

- **`spawn_reader()`** — Reads lines from stdout, parses JSON, and dispatches:
  - Objects with an `"id"` field → response: look up the ID in `pending_responses`, send the value through the oneshot channel, remove the entry.
  - Objects without an `"id"` field → event: deserialize as `ACPEvent` and send through the mpsc channel.
  - Supports both numeric and string JSON-RPC IDs per RFC 7464.

### Handshake Validation (`client.rs`)

`initialize()` performs the following steps:

1. Sends the handshake request via `send_request("initialize", ...)`.
2. Awaits the oneshot response with `INIT_TIMEOUT` (10 seconds).
3. Checks for a JSON-RPC error response (`error.code` field).
4. Validates `result.protocolVersion` equals `ACP_PROTOCOL_VERSION` (`"1.0"`).
5. Extracts `result.capabilities` and stores in the `OnceLock<ACPCapabilities>`.
6. Stores the protocol version in `OnceLock<String>`.

Error conditions return specific `ACPError` variants:
- `ACPError::Timeout` — handshake timed out after 10 seconds.
- `ACPError::InitializeFailed` — version mismatch, missing fields, or agent error.

### Session Creation (`client.rs`)

`sessions_create()` replaces the fabricated session ID with actual response parsing:

1. Sends `sessions/create` request via `send_request()`.
2. Awaits the oneshot response with `SESSION_CREATE_TIMEOUT` (30 seconds).
3. Checks for JSON-RPC error responses.
4. Parses `result` as `SessionCreateResult`, extracting the real `session_id`.
5. Returns the parsed result.

### Resettable Timeout (`session.rs`)

`run_event_loop()` uses a pinned, resettable `tokio::time::Sleep`:

```rust
let mut timeout_sleep: Pin<Box<tokio::time::Sleep>> =
    Box::pin(tokio::time::sleep(self.timeout));

loop {
    tokio::select! {
        result = rx.recv() => {
            // ... process event ...
            // Reset idle timeout on each event
            let wake_at = tokio::time::Instant::now() + self.timeout;
            timeout_sleep.as_mut().reset(wake_at);
        }
        _ = &mut timeout_sleep => {
            return Err(ACPError::SessionTimeout { ... });
        }
    }
}
```

The timeout measures **idle time** (time since the last event), not total runtime. Actively producing sessions continue indefinitely; inactive sessions are killed after `self.timeout` of no events.

### MPSC Event Channel (`client.rs`, `session.rs`)

The client-to-session event delivery uses `tokio::sync::mpsc::channel(1024)`:

- Created internally in `ACPClient::new()`.
- The `spawn_reader` task writes events to the mpsc sender.
- `ACPSession::run_event_loop()` takes ownership of the receiver via `client.take_event_receiver()`.
- The receiver is stored in `std::sync::Mutex<Option<mpsc::Receiver<>>>`, allowing `take_event_receiver()` to work with `&self` and be called exactly once.
- Events emitted during `initialize()` and `sessions_create()` are buffered in the 1024-slot channel. When `run_event_loop()` takes the receiver, all buffered events are consumed first.

The central `EventStream` (broadcast channel) remains unchanged for fan-out to external subscribers. `run_event_loop()` publishes each event to `EventStream` after receiving it from the mpsc channel.

### Session Creation Flow (`session.rs`)

`ACPSession::create()` follows this sequence:

1. Spawn agent subprocess via `spawn_agent()`.
2. Create `ACPClient` (mpsc channel created internally).
3. Spawn stdout reader task via `client.spawn_reader()`.
4. Run protocol handshake via `client.initialize()`.
5. Create ACP session via `client.sessions_create()`.
6. Return `ACPSession` with the real session ID from the agent's response.

Events emitted during steps 4–5 are buffered in the mpsc channel and consumed by `run_event_loop()`.

### Code Review Fixes Applied

| Fix | Before | After |
|-----|--------|-------|
| Fire-and-forget leak | `send_request()` used for all methods, accumulating oneshot senders | Separate `send_notification()` for fire-and-forget methods |
| JSON-RPC error check | `initialize()` did not check for error responses | Both `initialize()` and `sessions_create()` check `error.code` |
| Panic-prone unwrap | `unwrap()`/`expect()` calls throughout | Safe error handling with `ok_or_else()`, `unwrap_or()`, `map_err()` |
| RwLock → OnceLock | `RwLock<String>` and `RwLock<ACPCapabilities>` | `OnceLock<String>` and `OnceLock<ACPCapabilities>` (write-once, read-many) |
| mod.rs re-exports | Missing or broken re-exports | Restored full public API re-exports |
| Magic numbers | `10`, `30`, `"2.0"`, `"1.0"` as literals | Named constants: `INIT_TIMEOUT`, `SESSION_CREATE_TIMEOUT`, `JSON_RPC_VERSION`, `ACP_PROTOCOL_VERSION` |
| Atomic ordering | `SeqCst` for `id_counter` | `Relaxed` (sufficient for monotonically increasing IDs) |
| String allocations | Per-request string allocation for JSON serialization | Single `serde_json::to_string()` call per request |
| JSON-RPC string ID | Only supported numeric IDs | Supports both `u64` and `String` IDs per RFC 7464 |

## Usage

### Creating and Using a Session

```rust
use nexum::acp::{ACPSession, AgentRole, EventStream};
use nexum::acp::subprocess::AgentConfig;

let event_stream = EventStream::with_default_capacity();
let agent_config = AgentConfig { /* ... */ };

let mut session = ACPSession::create(
    "task-123",
    AgentRole::Builder,
    "Implement the feature",
    &worktree_path,
    &agent_config,
    std::time::Duration::from_secs(1800),
    &event_stream,
).await?;

// Run the event loop (blocks until terminal event, response-required event, or timeout)
let maybe_event = session.run_event_loop(&event_stream).await?;

// Send follow-up messages
session.send_message("Please also handle edge cases", MessageType::FollowUp).await?;

// Respond to permission requests
session.respond_to_permission("req-42", true).await?;

// Clean up
session.destroy().await?;
```

### Querying Client State

```rust
// After initialize() completes:
let version = session.client.protocol_version(); // e.g., "1.0"
let caps = session.client.capabilities();         // ACPCapabilities

// Check if session is stale
if session.check_heartbeat(Duration::from_secs(60)) {
    // Session has been idle for over 60 seconds
}
```

## Configuration

### Named Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `JSON_RPC_VERSION` | `"2.0"` | JSON-RPC protocol version in all requests |
| `ACP_PROTOCOL_VERSION` | `"1.0"` | ACP protocol version for handshake |
| `INIT_TIMEOUT` | 10 seconds | Timeout for the `initialize` handshake |
| `SESSION_CREATE_TIMEOUT` | 30 seconds | Timeout for the `sessions/create` request |

### Channel Capacity

The mpsc event channel between the reader task and the session has a capacity of **1024** events. This buffers all events emitted during the `initialize()` and `sessions_create()` calls. If the agent produces events faster than the channel can drain, the sender blocks (backpressure on stdout). This is acceptable for the MVP.

### Interior Mutability

- `protocol_version` and `capabilities` use `OnceLock` — written once by `initialize()`, read many times.
- `event_receiver` uses `std::sync::Mutex<Option<Receiver>>` — allows `take_event_receiver()` to work with `&self` and be called exactly once.
- `pending_responses` uses `Arc<tokio::sync::Mutex<HashMap>>` — shared between the main task and the reader task.
