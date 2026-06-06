# Graceful Shutdown

## Overview

The Overlord scheduler runs as a background `tokio::spawn` task inside Nexum's `main()`. Without shutdown coordination, sending `SIGINT` or `SIGTERM` would terminate the process immediately, leaving the scheduler loop unfinished. The graceful shutdown feature ensures the scheduler receives a stop signal and completes its cleanup before the process exits.

## What Was Built

A two-phase shutdown sequence in `src/main.rs`:

1. **Signal listening** — `tokio::select!` waits on both `SIGINT` and `SIGTERM` streams concurrently.
2. **Ordered shutdown** — On signal receipt, `scheduler.stop()` flips the internal `running` flag, then `join_handle.await` waits for the scheduler's async task to finish its current iteration and exit cleanly.

## Technical Implementation

### Files Modified

| File | Change |
|---|---|
| `src/main.rs` | Added `JoinHandle` storage, signal streams, `tokio::select!`, and `stop()` + `await` shutdown sequence |

### Key Changes in `main.rs`

- **JoinHandle stored** — The `tokio::spawn` result is captured in `join_handle` instead of being fire-and-forget.
- **Signal streams** — `tokio::signal::unix::signal(SignalKind::interrupt())` and `signal(SignalKind::terminate())` create async receivers for `SIGINT` and `SIGTERM`.
- **`tokio::select!`** — Blocks until either signal arrives, logging which one triggered shutdown.
- **`scheduler.stop()`** — Calls `running.store(false, SeqCst)` so the scheduler loop exits on its next check.
- **`join_handle.await`** — Waits for the spawned task to finish before returning `Ok(())`.

### Relevant Scheduler Code

```rust
// src/overlord/scheduler.rs
pub fn stop(&self) {
    self.running.store(false, Ordering::SeqCst);
}
```

The scheduler's `start()` loop checks `self.running` each iteration; once `false`, the loop breaks and the task completes.

### Dependencies

**None added.** Uses only existing crates:

- `tokio::signal::unix` — built into the `tokio` dependency (with `signal` feature)
- `anyhow` — already present for error handling

## Usage

### Testing Graceful Shutdown

```bash
# Build
cargo build

# Run in background
cargo run &
PID=$!

# Send SIGINT (Ctrl+C equivalent)
kill -INT $PID

# Or send SIGTERM
kill -TERM $PID
```

### Expected Behavior

The process should:

1. Log `Received SIGINT, shutting down...` or `Received SIGTERM, shutting down...`
2. Call `stop()` — scheduler loop exits on next iteration
3. Log `Overlord scheduler stopped`
4. Await the join handle (scheduler task completes)
5. Log `Shutdown complete`
6. Exit cleanly (exit code 0)

### Without Graceful Shutdown (Before)

The process would exit immediately on signal, with no `stop()` call and no wait for the scheduler task. The spawned task would be abandoned mid-loop.
