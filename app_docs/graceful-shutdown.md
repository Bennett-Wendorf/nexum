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
| `src/main.rs` | Added signal handler setup before task spawn, `JoinHandle` storage, `tokio::select!`, `stop()` + `await` shutdown sequence with timeout |
| `src/overlord/scheduler.rs` | Added responsive shutdown via `tokio::select!` with periodic `running` flag polling during sleep |

### Key Changes in `main.rs`

- **Signal handlers created early** — Signal streams are set up before spawning any tasks, avoiding potential race conditions.
- **JoinHandle stored** — The `tokio::spawn` result is captured in `join_handle` instead of being fire-and-forget.
- **Signal streams** — `tokio::signal::unix::signal(SignalKind::interrupt())` and `signal(SignalKind::terminate())` create async receivers for `SIGINT` and `SIGTERM`.
- **`tokio::select!`** — Blocks until either signal arrives, logging which one triggered shutdown.
- **`scheduler.stop()`** — Calls `running.store(false, SeqCst)` so the scheduler loop exits promptly.
- **Timeout on join handle** — `tokio::time::timeout` wraps the `join_handle.await` with a 5-second deadline, logging a warning if the scheduler doesn't stop in time and logging any task panics.

### Responsive Shutdown in Scheduler

The scheduler uses a responsive shutdown mechanism instead of waiting for the full sleep cycle to elapse. During the sleep interval, a `tokio::select!` concurrently monitors both the sleep timer and a `check_shutdown()` poller that checks the `running` flag every 100ms:

```rust
// src/overlord/scheduler.rs
tokio::select! {
    _ = tokio::time::sleep(self.interval) => {},
    _ = self.check_shutdown() => break,
}
```

This ensures the scheduler exits promptly after `stop()` is called, rather than waiting up to the full interval duration (e.g., 30 seconds by default).

### Relevant Scheduler Code

```rust
// src/overlord/scheduler.rs
pub fn stop(&self) {
    self.running.store(false, Ordering::SeqCst);
}

async fn check_shutdown(&self) {
    loop {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

The scheduler's `start()` loop checks `self.running` each iteration; once `false`, the `tokio::select!` breaks out of the sleep and the loop exits.

### Dependencies

**None added.** Uses only existing crates:

- `tokio::signal::unix` — built into the `tokio` dependency (with `signal` feature)
- `tokio::time::timeout` — built into the `tokio` dependency (with `time` feature)
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
2. Call `stop()` — scheduler's `check_shutdown()` poller detects the flag change within ~100ms
3. Log `Overlord scheduler stopped`
4. Await the join handle with a 5-second timeout (scheduler task completes promptly)
5. Log `Shutdown complete`
6. Exit cleanly (exit code 0)

### Without Graceful Shutdown (Before)

The process would exit immediately on signal, with no `stop()` call and no wait for the scheduler task. The spawned task would be abandoned mid-loop.
