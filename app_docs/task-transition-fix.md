# Task Transition Fix (002-mvp/017)

## Overview

Synchronized the `TASK_TRANSITIONS` constant in `src/api/middleware.rs` with the authoritative `TaskStateMachine` in `src/overlord/status_machine.rs`, eliminating inconsistent validation between the API middleware layer and the overlord state machine.

## What Was Built

The middleware transition map previously allowed invalid transitions into `abandoned` and was missing a valid re-queue transition. The fix corrected the constant so middleware validation and status machine validation agree on every allowed and disallowed transition.

## The Inconsistency

The `TASK_TRANSITIONS` constant in `src/api/middleware.rs` defined three transitions into `abandoned` that the `TaskStateMachine::can_transition()` method did **not** permit:

| Invalid transition | Middleware | Status machine |
|---|---|---|
| `backlog → abandoned` | ✅ allowed | ❌ rejected |
| `running → abandoned` | ✅ allowed | ❌ rejected |
| `reviewing → abandoned` | ✅ allowed | ❌ rejected |

The status machine only allows one transition into `abandoned`:

- `waiting-manual-review → abandoned`

Requests using the invalid transitions would pass middleware validation but be rejected by the status machine, producing confusing error paths.

Additionally, the status machine allows `running → queued` (re-queue), which the middleware map did **not** include — meaning legitimate re-queue requests were incorrectly rejected at the middleware layer.

## What Was Changed

### `src/api/middleware.rs`

1. **`TASK_TRANSITIONS` constant** (lines 104–111) — replaced with the corrected map:

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

   - Removed `"abandoned"` from `backlog`, `running`, and `reviewing` targets.
   - Added `"queued"` to `running` targets (re-queue).

2. **Doc comment** (lines 96–103) — updated to state that tasks may be abandoned only from `waiting-manual-review`, and that both `abandoned` and `completed` are terminal states.

3. **Tests** (lines 490–508):
   - `test_task_transition_valid`: replaced the `backlog → abandoned` assertion with `running → queued` (re-queue).
   - `test_task_transition_invalid`: added assertions confirming `backlog → abandoned`, `running → abandoned`, and `reviewing → abandoned` are now correctly rejected.

### `src/overlord/status_machine.rs`

No changes — this file is the source of truth and was already correct.

## Resulting State

The middleware `TASK_TRANSITIONS` map now matches the `TaskStateMachine::can_transition()` transitions one-to-one:

| From | Allowed targets |
|---|---|
| `backlog` | `queued` |
| `queued` | `running` |
| `running` | `reviewing`, `queued` |
| `reviewing` | `waiting-manual-review`, `merge-queue` |
| `waiting-manual-review` | `merge-queue`, `abandoned` |
| `merge-queue` | `completed` |

Both `abandoned` and `completed` are terminal states with no outgoing transitions.

## Validation

- `cargo test` passes — all tests including the updated transition assertions succeed.
- `cargo clippy -- -D warnings` produces no warnings.
- Every transition in `TASK_TRANSITIONS` has a corresponding match arm in `TaskStateMachine::can_transition()`, and vice versa.
