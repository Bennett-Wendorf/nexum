# Unit of work

## Definition
## Storage
- Markdown as source of truth on work for max interoperability
- Directory structure to handle orchestration levels?
    - Should there also be metadata files for this?
- Additional json data for handling control-plane pieces (agent leases on tasks, queues, statuses, etc.
## Creation
## Dependency chaining
- Need to be able to auto-queue tasks when *ALL* dependencies are completed
## Execution plan
- The dependency structure determined by the planner during task generation
- Defines which tasks can run in parallel and which must wait for predecessors
- Goal: minimize conflicts where parallel work overwrites or blocks each other
- Materialized as the `dependencies` field in each task's `task.md` (see persistence.md)
- Enforced by the Overlord during task dispatch (see agent-roles.md)
