# Work statuses

- Need orchestration levels here. Should be minimum per repo, branch, and plan, but at every level would be ideal
- Can move tasks between certain statuses. For example, if auto-queued accidentally, should be able to move back to backlog.

## Plan-level statuses

These statuses apply to plans (groups of tasks).

- Backlog
- Queued
- Planning (This is part of where concurrency limits are handled)
- Reviewing (Concurrency limit)
- Plan complete -> post-planning backlog

## Task-level statuses

These statuses apply to individual tasks within a plan. Plan-level statuses are defined separately above.

- Backlog
- Queued
- Running (This is part of where concurrency limits are handled)
- Reviewing (Concurrency limit)
- Waiting manual review
- Merge-queue
- Abandoned
- Completed (How to clear this out occasionally?)
