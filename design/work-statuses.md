# Work statuses

- Need orchestration levels here. Should be minimum per repo, branch, and plan, but at every level would be ideal
- Can move tasks between certain statuses. For example, if auto-queued accidentally, should be able to move back to backlog.

## Plan statuses (pre-planning)

These statuses track a plan from rough idea through human approval.

- `draft` — rough idea, user-created, sparse content
- `queued` — waiting for planner agent
- `planning` — planner is working on it (concurrency limit applies here. See resource-constraints.md for enforcement details.)
- `reviewing` — plan ready, waiting for human approval (concurrency limit. See resource-constraints.md for enforcement details.)

## Plan statuses (post-planning)

After human approval, the plan tracks its tasks through execution.

- `approved` — human approved, tasks flow to task backlog
- `complete` — all tasks done
- `rejected` — human rejected (terminal; user can create a new draft if wanted)

## Task statuses (post-planning)

These statuses apply to individual tasks within an approved plan.

- `backlog` — approved plan's tasks start here
- `queued` — ready for builder agent
- `running` — builder is working on it (concurrency limit applies here. See resource-constraints.md for enforcement details.)
- `reviewing` — task done, waiting for reviewer (concurrency limit. See resource-constraints.md for enforcement details.)
- `waiting-manual-review` — reviewer flagged for human attention
- `merge-queue` — approved, waiting for overlord to merge into plan branch
- `abandoned` — task dropped, won't be retried
- `completed` — merged, task branch and worktree cleaned up
