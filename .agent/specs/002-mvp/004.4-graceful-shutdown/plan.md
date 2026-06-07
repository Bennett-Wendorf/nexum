# Plan: 004.4 - Graceful Shutdown for Overlord Task

## Task Description
Add graceful shutdown support to the Overlord scheduler background task in main.rs.

## Objective
Ensure the Overlord scheduler shuts down cleanly when the process receives SIGINT or SIGTERM.

## Problem Statement
The scheduler is spawned as fire-and-forget tokio::spawn with no JoinHandle stored, no signal handler, and stop() never called. Process exits before scheduler does any work.

## Solution Approach
Store JoinHandle, add signal handler using tokio::signal::unix, call stop() on signal, await handle before exit.

## Relevant Files
- `src/main.rs` — only file modified

## Step by Step Tasks

### 1. Add signal handling and graceful shutdown to main.rs
- Store JoinHandle from tokio::spawn
- Create signal streams for SIGINT and SIGTERM
- Use tokio::select! to wait on either signal
- On signal: call scheduler.stop(), await join_handle
- Return Ok(()) only after shutdown completes

### 2. Verify shutdown behavior
- Build and test SIGINT shutdown
- Test SIGTERM shutdown

### 3. Final validation
- cargo build, cargo clippy, cargo test

## Acceptance Criteria
- JoinHandle stored and awaited
- SIGINT and SIGTERM trigger shutdown
- stop() called before awaiting handle
- Process waits for scheduler to terminate
- No new dependencies
- All tests pass
