# Plan: 016 - Fix plan status transition map inconsistency

## Task Description
Fix an inconsistency between the `PLAN_TRANSITIONS` constant in `src/api/middleware.rs` and the authoritative `PlanStateMachine` in `src/overlord/status_machine.rs`. The middleware transition map incorrectly allows `reviewing → queued` as a valid plan status transition, while the status machine only permits `reviewing → approved` or `reviewing → rejected`. This causes PATCH requests that pass middleware validation to be rejected downstream by the status machine with a confusing 422 error.

## Objective
Synchronize `PLAN_TRANSITIONS` in `src/api/middleware.rs` with `PlanStateMachine::can_transition()` so that both layers agree on valid plan status transitions, eliminating the validation gap.

## Problem Statement
The `PLAN_TRANSITIONS` constant at `src/api/middleware.rs:83-89` defines `"reviewing" → ["approved", "queued"]` as valid. However, `PlanStateMachine::can_transition()` at `src/overlord/status_machine.rs:38-52` only allows `"reviewing" → "approved"` or `"reviewing" → "rejected"`. This mismatch means:

1. A PATCH request to transition a plan from `reviewing` to `queued` passes the middleware's `validate_status_transition()` check.
2. The request then reaches the overlord layer where `PlanStateMachine::transition()` rejects it with `OverlordError::InvalidTransition`.
3. The client receives a 422 error with an unhelpful message, because the API never validated this transition upfront.

## Solution Approach
Update `PLAN_TRANSITIONS` in `src/api/middleware.rs` to match the authoritative transitions defined in `PlanStateMachine::can_transition()`:

| Current (middleware) | Correct (status machine) |
|---|---|
| `reviewing → approved, queued` | `reviewing → approved, rejected` |

Additionally:
- Fix the doc comment on line 82 that references the incorrect transition.
- Update the test at line 477 that asserts `reviewing → queued` is valid.
- Add a negative test case confirming `reviewing → queued` is invalid.

## Relevant Files

### Existing Files
- **`src/api/middleware.rs`** — Contains `PLAN_TRANSITIONS` (line 83-89), its doc comment (line 82), and the test suite (lines 411-545). This is the primary file to modify.
- **`src/overlord/status_machine.rs`** — Contains the authoritative `PlanStateMachine::can_transition()` (lines 38-52). Read-only reference; not to be modified.
- **`design/work-statuses.md`** — Design document describing plan status lifecycle. Read-only reference.

### New Files (if needed)
None.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: middleware-builder
  - Role: Update `PLAN_TRANSITIONS`, doc comment, and tests in `middleware.rs`
  - Agent: builder

- **Validator**
  - Name: middleware-validator
  - Role: Verify implementation meets criteria, run tests
  - Agent: validator

- **Documenter**
  - Name: fix-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Fix PLAN_TRANSITIONS constant
- **Task ID**: fix-transition-map
- **Depends On**: none
- **Assigned To**: middleware-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/middleware.rs`, line 87: Change `("reviewing", &["approved", "queued"])` to `("reviewing", &["approved", "rejected"])`.
  - In `src/api/middleware.rs`, line 82: Update the doc comment from `reviewing → approved|queued` to `reviewing → approved|rejected`.
- **Acceptance Criteria**:
  - `PLAN_TRANSITIONS` matches the transitions in `PlanStateMachine::can_transition()` exactly.
  - The doc comment accurately describes the transition flow.

### 2. Update unit tests
- **Task ID**: update-tests
- **Depends On**: fix-transition-map
- **Assigned To**: middleware-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/middleware.rs`, line 477: Replace `assert!(validate_status_transition("reviewing", "queued", "plan").is_ok());` with `assert!(validate_status_transition("reviewing", "rejected", "plan").is_ok());` to test the correct valid transition.
  - In `src/api/middleware.rs`, `test_plan_transition_invalid` (line 483-487): Add `assert!(validate_status_transition("reviewing", "queued", "plan").is_err());` to confirm the previously-invalid transition is now properly rejected.
- **Acceptance Criteria**:
  - `test_plan_transition_valid` tests all valid transitions including `reviewing → rejected`.
  - `test_plan_transition_invalid` includes `reviewing → queued` as an invalid transition.
  - All tests pass.

### 3. Final Validation
- **Task ID**: validate-all
- **Depends On**: fix-transition-map, update-tests
- **Assigned To**: middleware-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` to verify all tests pass.
  - Run `cargo clippy` to check for lint issues.
  - Verify `PLAN_TRANSITIONS` entries match `PlanStateMachine::can_transition()` transitions one-to-one.
  - Confirm no other files reference the `reviewing → queued` plan transition.

### 4. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `PLAN_TRANSITIONS` in `src/api/middleware.rs` exactly matches the transitions allowed by `PlanStateMachine::can_transition()`.
- The doc comment on line 82 accurately reflects the corrected transition flow.
- Unit test `test_plan_transition_valid` asserts `reviewing → rejected` is valid (replacing the old `reviewing → queued` assertion).
- Unit test `test_plan_transition_invalid` asserts `reviewing → queued` is invalid.
- All existing tests continue to pass (`cargo test`).
- No clippy warnings are introduced (`cargo clippy`).

## Validation Commands
- `cargo test` — Run full test suite
- `cargo test middleware::tests::test_plan_transition_valid` — Run specific plan transition test
- `cargo test middleware::tests::test_plan_transition_invalid` — Run specific invalid transition test
- `cargo clippy -- -D warnings` — Check for lint issues

## Notes
- The `PlanStateMachine` in `src/overlord/status_machine.rs` is the authoritative source for valid transitions. The middleware transition map is a pre-validation layer and must never allow transitions that the status machine would reject.
- This is a straightforward fix with no architectural changes. The scope is limited to one file (`src/api/middleware.rs`) and its inline tests.
- The task transition map (`TASK_TRANSITIONS`) was not found to have similar inconsistencies and does not need changes.
