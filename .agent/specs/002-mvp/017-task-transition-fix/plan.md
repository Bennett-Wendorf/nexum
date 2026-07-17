# Plan: 017 - Task Transition Fix

## Task Description
Fix the inconsistency between the `TASK_TRANSITIONS` constant in `src/api/middleware.rs` and the authoritative `TaskStateMachine` in `src/overlord/status_machine.rs`. The middleware transition map allows transitions to `abandoned` from `backlog`, `running`, and `reviewing`, but the status machine does not permit these transitions. Conversely, the status machine allows `running → queued` (re-queue), which the middleware map does not include.

## Objective
Synchronize the `TASK_TRANSITIONS` constant in `src/api/middleware.rs` with the transitions defined by `TaskStateMachine::can_transition()` in `src/overlord/status_machine.rs`, so that middleware validation and status machine validation agree on all allowed transitions.

## Problem Statement
The `TASK_TRANSITIONS` constant (middleware.rs:102-112) defines these transitions to `abandoned`:
- `backlog → abandoned`
- `running → abandoned`
- `reviewing → abandoned`

The authoritative `TaskStateMachine::can_transition()` (status_machine.rs:101-118) only allows ONE transition into `abandoned`:
- `waiting-manual-review → abandoned`

This means requests like `backlog → abandoned` pass middleware validation but are then rejected by the status machine, producing inconsistent error paths. Additionally, the status machine allows `running → queued` (re-queue), which the middleware map omits.

## Solution Approach
1. Update `TASK_TRANSITIONS` to remove invalid transitions to `abandoned` (from `backlog`, `running`, `reviewing`).
2. Add the missing `running → queued` re-queue transition.
3. Keep `waiting-manual-review → abandoned` as-is (this is the only valid transition into `abandoned`).
4. Update the doc comment above `TASK_TRANSITIONS` to accurately reflect the lifecycle.
5. Update the test `test_task_transition_valid` to remove the now-invalid `backlog → abandoned` assertion.
6. Update the test `test_task_transition_invalid` to add assertions for the newly-invalid transitions.
7. Add a new assertion for the `running → queued` re-queue transition.

## Relevant Files

### Existing Files
- **`src/api/middleware.rs`** — Contains `TASK_TRANSITIONS` (line 102-112), its doc comment (line 97-101), and transition validation tests (lines 490-510). This is the primary file to modify.
- **`src/overlord/status_machine.rs`** — Contains the authoritative `TaskStateMachine::can_transition()` (lines 101-119). Read-only reference; no changes needed here.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: middleware-builder
  - Role: Update `TASK_TRANSITIONS`, doc comment, and tests in `src/api/middleware.rs`
  - Agent: builder

- **Validator**
  - Name: middleware-validator
  - Role: Verify implementation meets criteria
  - Agent: validator

- **Documenter**
  - Name: fix-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Update TASK_TRANSITIONS constant
- **Task ID**: update-transition-map
- **Depends On**: none
- **Assigned To**: middleware-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/middleware.rs`, line 102-112, replace `TASK_TRANSITIONS` with the corrected map:
    ```rust
    const TASK_TRANSITIONS: &[(&str, &[&str])] = &[
        ("backlog", &["queued"]),
        ("queued", &["running"]),
        ("running", &["reviewing", "queued"]),
        ("reviewing", &["waiting-manual-review", "merge-queue"]),
        ("waiting-manual-review", &["merge-queue", "abandoned"]),
        ("merge-queue", &["completed"]),
    ];
    ```
  - Changes from current:
    - Remove `"abandoned"` from `backlog` targets
    - Remove `"abandoned"` from `running` targets
    - Add `"queued"` to `running` targets (re-queue)
    - Remove `"abandoned"` from `reviewing` targets
  - Update the doc comment (lines 97-101) to remove "Tasks may be `abandoned` from any non-terminal state" and replace with accurate description: "Tasks may be `abandoned` only from `waiting-manual-review`, and both `abandoned` and `completed` are terminal states."
- **Acceptance Criteria**:
  - `TASK_TRANSITIONS` matches exactly the transitions allowed by `TaskStateMachine::can_transition()`
  - Doc comment accurately describes the lifecycle

### 2. Update transition validation tests
- **Task ID**: update-tests
- **Depends On**: update-transition-map
- **Assigned To**: middleware-builder
- **Agent**: builder
- **Actions**:
  - In `test_task_transition_valid` (line 490-498):
    - Remove line 492: `assert!(validate_status_transition("backlog", "abandoned", "task").is_ok());`
    - Add: `assert!(validate_status_transition("running", "queued", "task").is_ok());` (re-queue)
  - In `test_task_transition_invalid` (line 500-505):
    - Add: `assert!(validate_status_transition("backlog", "abandoned", "task").is_err());`
    - Add: `assert!(validate_status_transition("running", "abandoned", "task").is_err());`
    - Add: `assert!(validate_status_transition("reviewing", "abandoned", "task").is_err());`
- **Acceptance Criteria**:
  - All valid transition assertions test only transitions that `TaskStateMachine::can_transition()` allows
  - All invalid transition assertions cover the previously-overallowed transitions

### 3. Final Validation
- **Task ID**: validate-all
- **Depends On**: update-tests
- **Assigned To**: middleware-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` and verify all tests pass
  - Run `cargo clippy` and verify no warnings
  - Confirm `TASK_TRANSITIONS` transitions match `TaskStateMachine::can_transition()` transitions one-to-one
  - Verify no transitions to `abandoned` exist from `backlog`, `running`, or `reviewing` in the middleware map

### 4. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
1. `TASK_TRANSITIONS` in `src/api/middleware.rs` exactly matches the transitions defined by `TaskStateMachine::can_transition()` in `src/overlord/status_machine.rs`.
2. The only transition into `abandoned` in `TASK_TRANSITIONS` is `waiting-manual-review → abandoned`.
3. The `running → queued` re-queue transition is present in `TASK_TRANSITIONS`.
4. All existing tests pass after the changes.
5. The doc comment for `TASK_TRANSITIONS` accurately describes the allowed transitions.
6. No clippy warnings or compilation errors.

## Validation Commands
- `cargo test` — Run full test suite
- `cargo test middleware::tests::test_task_transition_valid` — Verify valid task transitions
- `cargo test middleware::tests::test_task_transition_invalid` — Verify invalid task transitions
- `cargo clippy -- -D warnings` — Check for lint warnings

## Notes
- The `TaskStateMachine` in `src/overlord/status_machine.rs` is the source of truth. The middleware map is a compile-time lookup used for early validation before the status machine is consulted. They must always agree.
- The `abandoned` state is terminal and has no outgoing transitions (already correctly handled by `TASK_TERMINAL_STATUSES`).
- The `TASK_TERMINAL_STATUSES` constant does not need modification — it correctly lists both `abandoned` and `completed`.
- The `validate_task_status` function does not need modification — it validates that a status string is a recognized state, not whether a transition is valid.
