# Work statuses

- Need orchestration levels here. Should be minimum per repo, branch, and plan, but at every level would be ideal
- Can move tasks between certain statuses. For example, if auto-queued accidentally, should be able to move back to backlog.

## Plan-level statuses

These statuses apply to plans (groups of tasks).

- `backlog`
- `queued`
- `planning` (This is part of where concurrency limits are handled. See resource-constraints.md for enforcement details.)
- `reviewing` (Concurrency limit. See resource-constraints.md for enforcement details.)
- `plan-complete` -> transitions to task-level `backlog`

## Task-level statuses

These statuses apply to individual tasks within a plan. Plan-level statuses are defined separately above.

- `backlog`
- `queued`
- `running` (This is part of where concurrency limits are handled. See resource-constraints.md for enforcement details.)
- `reviewing` (Concurrency limit. See resource-constraints.md for enforcement details.)
- `waiting-manual-review`
- `merge-queue`
- `abandoned`
- `completed` (How to clear this out occasionally?)
