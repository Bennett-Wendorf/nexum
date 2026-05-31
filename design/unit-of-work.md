# Unit of work

## Definition
## Storage
- Markdown as source of truth on work for max interoperability
- Two-file model per task: `task.md` (spec) + `status.json` (runtime state)
- No additional metadata files — the two-file split covers all needs
## Creation
## Dependency chaining
- Overlord auto-queues tasks when *ALL* dependencies are completed
- Configurable: users can disable auto-queue for manual control

## Execution plan
- The dependency structure determined by the planner during task generation
- Defines which tasks can run in parallel and which must wait for predecessors
- Goal: minimize conflicts where parallel work overwrites or blocks each other
- Materialized as the `dependencies` field in each task's `task.md` (see persistence.md)
- Enforced by the Overlord during task dispatch (see agent-roles.md)
