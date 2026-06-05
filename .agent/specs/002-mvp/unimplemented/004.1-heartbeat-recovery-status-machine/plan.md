# Plan: 004.1 - Heartbeat Recovery Status Machine Validation

## Task Description
Fix heartbeat recovery to validate `running -> queued` transition through TaskStateMachine instead of bypassing it.

## Objective
Ensure all status transitions flow through the state machine, including recovery transitions.

## Problem Statement
`recover_single_task` in heartbeat_monitor.rs directly writes status.json via atomic_write_json without going through the status machine. The `running -> queued` transition is not in the TaskStateMachine, so recovery bypasses all transition validation.

## Solution Approach
Extend TaskStateMachine with a `running -> queued` transition for recovery scenarios. Create a new persistence function `recover_task_status` that handles recovery-specific field modifications. Refactor heartbeat_monitor to validate via state machine before delegating to persistence layer.

## Relevant Files
- `src/overlord/status_machine.rs` — add transition
- `src/persistence/operations.rs` — add `recover_task_status`
- `src/overlord/heartbeat_monitor.rs` — refactor `recover_single_task`
- `src/overlord/tests.rs` — add recovery tests

## Step by Step Tasks

### 1. Add running -> queued transition to TaskStateMachine
- Add `TaskStatusValue::Running -> [TaskStatusValue::Queued]` to transition map
- Update tests to cover this transition

### 2. Create recover_task_status in persistence layer
- New function that: reads status, clears agent/started_at/heartbeat_at, increments attempts, records transition, writes atomically
- Includes TOCTOU mitigation like existing update_task_status

### 3. Refactor recover_single_task in heartbeat_monitor
- Validate transition via TaskStateMachine
- Call recover_task_status from persistence layer
- Remove direct atomic_write_json call

### 4. Add tests
- Test recovery transition validates
- Test recover_task_status clears fields correctly
- Test recovery increments attempts

### 5. Final validation
- cargo check, cargo test, cargo clippy

## Acceptance Criteria
- running -> queued is a valid transition in TaskStateMachine
- recover_single_task validates via state machine
- recover_task_status handles all recovery-specific field modifications
- All tests pass
