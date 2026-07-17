# Fix: Plan Status Transition Map Inconsistency

## Overview

Fixed an inconsistency between the middleware-level plan transition validation (`PLAN_TRANSITIONS` constant in `src/api/middleware.rs`) and the authoritative status machine (`PlanStateMachine` in `src/overlord/status_machine.rs`). The two layers disagreed on valid transitions from the `reviewing` status, causing requests to pass middleware validation only to be rejected downstream with a confusing 422 error.

## What Was Fixed

The `PLAN_TRANSITIONS` constant incorrectly allowed `reviewing → queued` as a valid plan status transition. The authoritative `PlanStateMachine::can_transition()` only permits `reviewing → approved` or `reviewing → rejected`. This mismatch created a validation gap:

1. A PATCH request to transition a plan from `reviewing` to `queued` passed the middleware's `validate_status_transition()` check.
2. The request then reached the overlord layer where `PlanStateMachine::transition()` rejected it with `OverlordError::InvalidTransition`.
3. The client received a 422 error with an unhelpful message, because the API never validated this transition upfront.

## How It Was Fixed

The `PLAN_TRANSITIONS` constant was updated to match the authoritative transitions defined in `PlanStateMachine::can_transition()`:

| Before (middleware) | After (corrected) |
|---|---|
| `reviewing → approved, queued` | `reviewing → approved, rejected` |

The doc comment on line 82 was also updated to reflect the corrected transition flow: `reviewing → approved|rejected`.

## Why It Matters

The middleware transition map serves as a pre-validation layer. It must never allow transitions that the status machine would reject. When these layers disagree, clients receive late-stage 422 errors with unclear messages instead of early, descriptive validation errors. Synchronizing both layers ensures consistent validation and better error reporting.

## Files Changed

### `src/api/middleware.rs`

- **Line 82** — Updated doc comment from `reviewing → approved|queued` to `reviewing → approved|rejected`.
- **Line 87** — Changed `("reviewing", &["approved", "queued"])` to `("reviewing", &["approved", "rejected"])`.
- **Line 477** (`test_plan_transition_valid`) — Replaced `reviewing → queued` valid assertion with `reviewing → rejected` valid assertion.
- **Line 487** (`test_plan_transition_invalid`) — Added `reviewing → queued` as an invalid transition test case.

## Test Updates

- `test_plan_transition_valid` now asserts that `reviewing → rejected` is a valid transition (replacing the incorrect `reviewing → queued` assertion).
- `test_plan_transition_invalid` now asserts that `reviewing → queued` is an invalid transition.

## Validation

All tests pass (`cargo test`) and no clippy warnings are introduced (`cargo clippy -- -D warnings`).
