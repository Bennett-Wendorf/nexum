# Resource constraints

## Parallel agent limits
- Should be able to limit parallel agents
    - Local LLMs have major compute restraints
    - Paid services would eat tokens very quickly without throttling
    - Too much going on can be stressful for the user
- **Enforcement**: The Overlord deterministic core enforces concurrency limits by checking `execution.json`'s `concurrency.currently_running` against `concurrency.max_parallel` before transitioning a task from `queued` to `running`.
- **Configurable per plan**: `max_parallel` is set in `execution.json` at plan level (see persistence.md).
- **Concurrency-sensitive statuses**: The following statuses are gated by concurrency limits (see work-statuses.md):
    - `planning` (plan-level) — limited during pre-planning phase
    - `reviewing` (plan-level) — limited during plan review
    - `reviewing` (task-level) — limited to prevent review bottlenecks
    - `running` (task-level) — primary concurrency gate during execution
- **Who enforces**: The Overlord deterministic core checks limits before task dispatch. Agents cannot self-transition to `running` without approval.

## Context limits
- Standard for auto-compaction to deal with context limits?
