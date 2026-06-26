# Builder Workflow Module

## Overview

The Builder Workflow module (`src/builder/`) is the core execution engine of nexum. It transforms a `queued` task into a `completed` task through a series of deterministic steps managed by the `WorkflowOrchestrator`. The module wires together the persistence layer, git operations, ACP client, and overlord deterministic core into a cohesive, working flow.

The orchestrator dispatches queued tasks to builder agents, spawns isolated git worktrees, manages the ACP session lifecycle, executes tasks, detects completion, merges task branches into the plan branch, and cleans up all resources.

## What Was Built

The module consists of 11 submodules coordinated by a central orchestrator:

| Module | File | Responsibility |
|--------|------|----------------|
| Orchestrator | `orchestrator.rs` | Central coordinator tying all sub-components together |
| Dispatcher | `dispatcher.rs` | Task selection, concurrency limits, session tracking |
| Worktree Manager | `worktree_manager.rs` | Isolated git worktree creation and cleanup |
| Session Manager | `session_manager.rs` | ACP session lifecycle (create, interact, destroy) |
| Heartbeat Manager | `heartbeat.rs` | Periodic and event-driven heartbeat updates |
| Event Bus | `event_bus.rs` | Event relay from sessions to central broadcast bus |
| Merge Coordinator | `merge_coordinator.rs` | Task branch merge, conflict handling, cleanup |
| Error Recovery | `error_recovery.rs` | Crash, timeout, conflict, and permission recovery |
| Completion Handler | `completion_handler.rs` | Post-completion merge and status transitions |
| Error Types | `errors.rs` | `BuilderError` enum with wrapped dependency errors |
| Tests | `tests.rs` | Unit and integration tests for all submodules |

All sub-components share `Arc` references for concurrent access. The dispatcher is wrapped in a `tokio::sync::Mutex` for `&mut self` session tracking, while the event bus uses interior mutability via `tokio::sync::Mutex` for per-session broadcaster storage.

## Workflow Sequence Diagram

The complete builder workflow proceeds through six phases:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     WorkflowOrchestrator.execute_task()                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  PHASE 1: SETUP                                                         │
│  ┌─────────────────────────────────────────────┐                       │
│  │ worktree_manager.setup_and_commit_agent_dir() │                     │
│  │   1. Create task branch: task/<TASK_ID>      │                       │
│  │   2. Spawn worktree at .worktrees/<ID>-<name>│                       │
│  │   3. Ensure .agent/ directory exists         │                       │
│  └─────────────────────────────────────────────┘                       │
│  → Emit BuilderEvent::TaskStarted                                       │
│                                                                         │
│  PHASE 2: SESSION                                                       │
│  ┌─────────────────────────────────────────────┐                       │
│  │ session_manager.create_session()            │                       │
│  │   1. Generate session ID                    │                       │
│  │   2. Spawn agent subprocess in worktree     │                       │
│  │   3. Initialize event stream                │                       │
│  │   4. Publish initial progress event         │                       │
│  └─────────────────────────────────────────────┘                       │
│  → heartbeat_manager.start_periodic_heartbeat()                        │
│  → event_bus.relay_session_events()                                    │
│  → event_bus.register_session()                                        │
│  → dispatcher.add_session()                                            │
│                                                                         │
│  PHASE 3: EXECUTION                                                     │
│  ┌─────────────────────────────────────────────┐                       │
│  │ session_manager.wait_for_completion(timeout)│                       │
│  │   1. Monitor event stream for terminal event│                       │
│  │   2. Completion → CompletionResult::Completed│                      │
│  │   3. Timeout  → CompletionResult::Timeout   │                       │
│  │   4. Crash    → CompletionResult::Crashed   │                       │
│  └─────────────────────────────────────────────┘                       │
│                                                                         │
│  PHASE 4: COMPLETION / ERROR                                            │
│  ┌─────────────────────────────────────────────┐                       │
│  │ On Completed:                               │                       │
│  │   completion_handler.handle_completion()    │                       │
│  │     → running → reviewing                   │                       │
│  │     → merge task branch (--no-ff)           │                       │
│  │     → on success: reviewing → completed     │                       │
│  │     → on conflict: → waiting-manual-review  │                       │
│  │                                             │                       │
│  │ On Timeout:                                 │                       │
│  │   error_recovery.handle_timeout()           │                       │
│  │     → running → queued                      │                       │
│  │     → cleanup worktree                      │                       │
│  │                                             │                       │
│  │ On Crashed:                                 │                       │
│  │   error_recovery.handle_agent_crash()       │                       │
│  │     → running → queued                      │                       │
│  │     → cleanup worktree                      │                       │
│  └─────────────────────────────────────────────┘                       │
│                                                                         │
│  PHASE 5: CLEANUP (always runs)                                         │
│  ┌─────────────────────────────────────────────┐                       │
│  │ 1. heartbeat_handle.abort()                  │                       │
│  │ 2. session_manager.destroy_session()         │                       │
│  │ 3. event_bus.unregister_session()            │                       │
│  │ 4. dispatcher.remove_session()               │                       │
│  │ 5. worktree_manager.cleanup() (if not done)  │                       │
│  └─────────────────────────────────────────────┘                       │
│  → Emit BuilderEvent::TaskCleanedUp                                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Session Lifecycle

The ACP session follows a five-stage lifecycle managed by `SessionManager`:

### 1. Create
- `session_manager.create_session(task_context, worktree_path)`
- Generates session ID: `session-<TASK_ID>-<timestamp_millis>`
- Spawns agent subprocess via `spawn_agent()` in the worktree directory
- Initializes `EventStream` with default broadcast capacity
- Publishes initial `ACPEvent::Progress` event ("Task started")
- Returns `ACPSessionHandle` containing session_id, subprocess, event_stream, and started_at

### 2. Run
- Agent subprocess executes within the isolated worktree
- ACP event stream produces `Progress`, `ToolCall`, `ToolResult`, `Question`, and `PermissionRequest` events
- Heartbeat manager updates `heartbeat_at` in `status.json` on each progress event
- Event bus relays all ACP events to the central broadcast channel

### 3. Interact
- `session_manager.send_message(handle, message)` — sends follow-up messages as Progress events
- `session_manager.handle_permission_request(handle, permission_id, approve)` — responds to agent permission requests
- Permission decisions are recorded as Progress events; actual JSON-RPC response handled at lower ACP layer

### 4. Complete
- `session_manager.wait_for_completion(handle, timeout)` monitors the event stream
- Detects three terminal states:
  - **Completed**: `ACPEvent::Completion` with `CompletionStatus::Success`
  - **Timeout**: `tokio::time::timeout` expires before completion event
  - **Crashed**: `ACPEvent::Error`, `CompletionStatus::Failed/Cancelled`, or stream closed
- The deprecated `is_completed()` method always returns `false`; use `wait_for_completion()` instead

### 5. Destroy
- `session_manager.destroy_session(handle)` consumes the handle
- Calls `AgentProcess::shutdown()` which sends SIGKILL and waits up to 5 seconds
- Resources (stdin/stdout pipes, stderr task) are cleaned up by `AgentProcess::shutdown`

## Worktree Isolation Model

Each task gets a completely isolated git worktree:

### Branch Naming
- Task branches follow the pattern `task/<TASK_ID>` (e.g., `task/TASK-001`)
- Generated via `TaskWorktree::branch_name(task_id)` which delegates to `git::task_branch_name()`

### Path Convention
- Worktrees live at `.worktrees/<TASK_ID>-<task_name>/` relative to repo root
- Computed via `TaskWorktree::worktree_path(repo_root, task_id, task_name)`
- Example: `/repo/.worktrees/TASK-001-add-auth-flow/`

### Setup Process (`worktree_manager.setup()`)
1. Create parent directory `.worktrees/` if needed
2. Create task branch from plan branch: `git checkout -b task/<TASK_ID> <plan_branch>`
3. Spawn worktree: `git worktree add <path> task/<TASK_ID>`
4. If branch or worktree already exists, reuse it (warns via tracing)
5. On failure, cleans up partial state (deletes branch)

### Setup with Agent Directory (`setup_and_commit_agent_dir()`)
- Calls `setup()` then ensures `.agent/` directory exists in the worktree

### Cleanup Process (`worktree_manager.cleanup()`)
1. Try `git worktree remove <path>`
2. If that fails, try `git worktree remove --force <path>`
3. If that fails, fall back to `tokio::fs::remove_dir_all()`
4. Delete task branch: `git branch -D task/<TASK_ID>`
5. Force-remove directory if still present
6. Run `git worktree prune` to clean stale metadata

### Validation
- `exists(task_id, task_name)` — checks if worktree directory exists
- `list_active()` — scans `.worktrees/` directory, validates `.git` file presence, parses directory names

## Heartbeat Mechanism and Stale Detection

The heartbeat system ensures the Overlord can detect stale or orphaned tasks.

### Periodic Heartbeat
- `heartbeat_manager.start_periodic_heartbeat(task_context)` spawns a background `JoinHandle`
- Uses `tokio::time::interval` with configurable duration (default: 5 minutes / 300 seconds)
- Each tick reads `status.json`, updates `heartbeat_at` to current UTC RFC 3339 timestamp, and writes atomically
- Stops when the `JoinHandle` is aborted (done in cleanup phase)
- Uses `MissedTickBehavior::Skip` to avoid burst updates

### Event-Driven Heartbeat
- `heartbeat_manager.update_on_event(event, task_context)` triggers heartbeat on specific ACP events
- Progress events that trigger heartbeat: `Progress`, `ToolCall`, `ToolResult`, `Question`, `PermissionRequest`
- Events that do NOT trigger heartbeat: `Completion`, `Error`
- Determined by `HeartbeatManager::is_progress_event(event)`

### Atomic Writes
- All heartbeat updates use `persistence::atomic_write_json()` (temp file + rename)
- Prevents corruption if process dies mid-write

## Merge Coordination and Conflict Handling

### Merge Process (`merge_coordinator.merge_task_branch()`)
1. Acquires `merge_lock` (`tokio::sync::Mutex`) to serialize concurrent merges into the same plan branch
2. Checks out plan branch: `git checkout <plan_branch>`
3. Merges task branch: `git merge task/<TASK_ID> --no-ff -m "Merge task/<TASK_ID>: <task_name>"`
4. On success (exit code 0): returns `MergeResult::Success`
5. On conflict (exit code 1): lists conflicted files, aborts merge, returns `MergeResult::Conflict { conflicted_files }`
6. On other error: aborts merge, returns error

### Post-Merge Cleanup (`merge_coordinator.post_merge_cleanup()`)
1. Deletes task branch: `git branch -D task/<TASK_ID>`
2. Removes worktree: `git worktree remove <path>` (with force and manual fallback)
3. Runs `git worktree prune`

### Conflict Handling (`merge_coordinator.handle_merge_conflict()`)
1. Transitions task to `waiting-manual-review` via Overlord
2. Actor: `"overlord-merge-conflict"`
3. Worktree is NOT cleaned up (preserved for human inspection)
4. Conflict details recorded in `status.json` transitions

### Dependency Auto-Queue (`merge_coordinator.trigger_dependency_auto_queue()`)
- After successful merge, calls `DependencyResolver::auto_queue_tasks()`
- Identifies downstream tasks whose dependencies are now satisfied
- Newly eligible tasks are automatically transitioned to `queued`

## Error Recovery Flows

### Agent Crash Recovery (`error_recovery.handle_agent_crash()`)
1. Transitions task from `running` → `queued` via Overlord
2. Actor: `"overlord-crash-recovery"`
3. Cleans up worktree (branch deletion + directory removal)
4. Emits `BuilderEvent::TaskFailed` with "Agent crashed" message
5. Emits `BuilderEvent::TaskRequeued`

### Timeout Recovery (`error_recovery.handle_timeout()`)
1. Transitions task from `running` → `queued` via Overlord
2. Actor: `"overlord-timeout"`
3. Cleans up worktree
4. Emits `BuilderEvent::TaskFailed` with elapsed/limit details
5. Emits `BuilderEvent::TaskRequeued`

### Merge Conflict Recovery (`error_recovery.handle_merge_conflict()`)
1. Transitions task to `waiting-manual-review` via Overlord
2. Actor: `"overlord-merge-conflict"`
3. Records conflict details in `status.json` transitions
4. Does NOT clean up worktree (preserved for inspection)
5. Emits `BuilderEvent::TaskFailed` with conflict details

### Permission Denied (`error_recovery.handle_permission_denied()`)
1. Logs the denial via tracing
2. Agent decides whether to proceed (may continue or abort)
3. If agent aborts, the crash recovery flow handles it

### Generic Error Handler (`error_recovery.handle_workflow_error()`)
- Pattern matches `BuilderError` variants to dispatch to specific handlers
- Unhandled errors: transitions to `queued` with actor `"overlord-error-recovery"`, cleans up worktree if still present
- Always emits `BuilderEvent::TaskFailed`

## Event Relay Architecture

### Event Flow
```
ACP Session → EventStream → BuilderEventBus → Central Event Bus → WebSocket → Frontend
```

### BuilderEventBus
- Uses `tokio::sync::broadcast` for many-to-many event distribution
- Configurable channel capacity
- Per-session broadcasters stored in a `Mutex<HashMap<String, broadcast::Sender<ACPEvent>>>`

### Event Types (`BuilderEvent`)
| Variant | Description |
|---------|-------------|
| `TaskStarted(TaskContext)` | Task began execution |
| `TaskProgress { task_id, event }` | ACP progress event relayed from session |
| `TaskCompleted { task_id, result }` | Task finished (Completed/Timeout/Crashed) |
| `TaskFailed { task_id, error }` | Task encountered an error |
| `TaskMerged { task_id }` | Task branch merged into plan branch |
| `TaskCleanedUp { task_id }` | Worktree and resources cleaned up |
| `TaskRequeued { task_id }` | Task re-queued after crash/timeout recovery |

### Session Event Relay (`event_bus.relay_session_events()`)
- Spawns background task that receives from session's ACP event stream
- Wraps each `ACPEvent` in `BuilderEvent::TaskProgress`
- Broadcasts to all central bus subscribers
- Stops when session event stream closes
- Handles lagged events with a warning (skips missed events)

### Per-Session Subscription (`event_bus.subscribe_session()`)
- Allows external consumers to subscribe to a specific task's ACP events
- Registered via `event_bus.register_session(task_id, broadcaster)`
- Unregistered via `event_bus.unregister_session(task_id)`

## Public API

### WorkflowOrchestrator

The primary entry point for external consumers (REST API, agent layer).

```rust
pub struct WorkflowOrchestrator { /* internal fields */ }
```

#### Construction
```rust
WorkflowOrchestrator::new(
    repo_root: PathBuf,
    overlord: Arc<OverlordScheduler>,
    agent_config: AgentConfig,
    concurrency_limit: usize,
    task_timeout: Duration,
    event_bus_capacity: usize,
) -> Self
```

#### Task Execution
```rust
/// Execute a single task through the complete workflow.
pub async fn execute_task(&self, task_context: TaskContext) -> Result<()>

/// Claim a task from the dispatcher and execute it.
pub async fn dispatch_and_execute(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<()>

/// Execute all ready tasks up to the concurrency limit.
pub async fn execute_all_ready_tasks(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<()>
```

#### Status and Monitoring
```rust
/// Get a list of currently active sessions.
pub async fn get_active_sessions(&self) -> Vec<ActiveSession>

/// Get a reference to the event bus for subscribing to builder events.
pub fn get_event_bus(&self) -> &Arc<BuilderEventBus>
```

#### Clone Support
`WorkflowOrchestrator` implements `Clone` by cloning all `Arc`-held components, enabling use in spawned tasks.

### BuilderEventBus

```rust
/// Subscribe to builder lifecycle events.
pub fn subscribe(&self) -> broadcast::Receiver<BuilderEvent>

/// Emit a builder lifecycle event.
pub fn emit(&self, event: BuilderEvent) -> Result<()>

/// Subscribe to a specific session's ACP events.
pub async fn subscribe_session(&self, task_id: &str) -> Option<broadcast::Receiver<ACPEvent>>
```

### TaskDispatcher

```rust
/// Find the next dispatchable task (queued + concurrency eligible).
pub async fn find_next_task(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<Option<TaskContext>>

/// Dispatch next task (find + transition queued → running).
pub async fn dispatch_task(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<Option<TaskContext>>
```

### Re-exported Types

All public types are accessible via `nexum::builder::*`:
- `BuilderError`, `Result`
- `WorkflowOrchestrator`
- `TaskDispatcher`, `TaskContext`, `ActiveSession`
- `WorktreeManager`, `TaskWorktree`
- `SessionManager`, `ACPSessionHandle`
- `CompletionHandler`, `CompletionOutcome`
- `HeartbeatManager`
- `MergeCoordinator`, `MergeResult`
- `ErrorRecovery`
- `BuilderEventBus`, `BuilderEvent`, `CompletionResult`

## Configuration and Tuning

### Orchestrator Parameters

| Parameter | Type | Description | Default/Recommendation |
|-----------|------|-------------|----------------------|
| `concurrency_limit` | `usize` | Maximum concurrent task executions | Based on available CPU cores / agent capacity |
| `task_timeout` | `Duration` | Maximum wall-clock time per task | 10-30 minutes depending on task complexity |
| `event_bus_capacity` | `usize` | Broadcast channel buffer size | 16-64 (higher if many subscribers) |

### Heartbeat Configuration

| Parameter | Type | Description | Default |
|-----------|------|-------------|---------|
| `interval` | `Duration` | Periodic heartbeat interval | 5 minutes (300 seconds) |

Configure via `HeartbeatManager::new(repo_root, interval)` or use `HeartbeatManager::with_default_interval(repo_root)`.

### Agent Configuration

Passed through `AgentConfig` to `SessionManager`:
- `binary`: Path to agent executable
- `args`: Command-line arguments
- `env`: Environment variables
- `name`: Agent identifier

### Tuning Guidelines

1. **Concurrency limit**: Set based on the number of available builder agents and system resources. Too high leads to resource contention; too low wastes capacity.

2. **Task timeout**: Set generously enough for the longest expected task but short enough to detect truly stuck agents. Consider task type (code generation vs. testing) when setting per-plan timeouts.

3. **Heartbeat interval**: 5 minutes is the default. Shorter intervals (1-2 min) provide faster stale detection but increase disk I/O. Longer intervals reduce I/O but delay detection of orphaned tasks.

4. **Event bus capacity**: 16 is sufficient for a few subscribers. If the frontend has many concurrent WebSocket connections, increase to 64 or higher to avoid lagged events.

5. **Merge lock**: The `MergeCoordinator` uses a `tokio::sync::Mutex` to serialize merges. This prevents concurrent git state corruption. No tuning needed, but be aware it creates a bottleneck if many tasks complete simultaneously.

## Technical Implementation

### Files Created

| File | Lines | Description |
|------|-------|-------------|
| `src/builder/mod.rs` | 44 | Module root with re-exports |
| `src/builder/errors.rs` | 83 | `BuilderError` enum with thiserror |
| `src/builder/orchestrator.rs` | 429 | Central workflow coordinator |
| `src/builder/dispatcher.rs` | 294 | Task dispatch and session tracking |
| `src/builder/worktree_manager.rs` | 313 | Worktree lifecycle management |
| `src/builder/session_manager.rs` | 247 | ACP session lifecycle |
| `src/builder/heartbeat.rs` | 175 | Heartbeat updates |
| `src/builder/event_bus.rs` | 160 | Event broadcast relay |
| `src/builder/merge_coordinator.rs` | 252 | Merge and conflict handling |
| `src/builder/error_recovery.rs` | 315 | Error recovery flows |
| `src/builder/completion_handler.rs` | 191 | Post-completion processing |
| `src/builder/tests.rs` | 1445 | Comprehensive test suite |

### Dependencies

The builder module depends on:
- `persistence` module — `TaskStatus`, `atomic_write_json`, `read_json`, `read_execution_state`, `update_execution_state`, `TaskStatusValue`
- `git` module — `create_task_branch`, `delete_branch`, `checkout_branch`, `git` (subprocess wrapper), `task_branch_name`, `list_conflicted_files`, `abort_merge`
- `acp` module — `AgentConfig`, `spawn_agent`, `AgentProcess`, `ACPEvent`, `CompletionStatus`, `EventStream`, `progress_event`, `log_event`
- `overlord` module — `OverlordScheduler`, `ConcurrencyChecker`, `DependencyResolver`, `TransitionTaskStatusParams`, `OverlordError`
- `config` module — Agent configuration

External crates used: `thiserror`, `tokio`, `chrono`, `serde_json`, `tempfile` (tests), `tracing`

### Key Design Decisions

1. **Arc-based sharing**: All sub-components are wrapped in `Arc` for shared ownership across concurrent task executions. The dispatcher uses `tokio::sync::Mutex` for mutable session tracking.

2. **Merge serialization**: A `tokio::sync::Mutex` in `MergeCoordinator` prevents concurrent git state corruption when multiple tasks merge into the same plan branch.

3. **Guaranteed cleanup**: The orchestrator's cleanup phase always runs regardless of success or failure, preventing orphaned worktrees and sessions.

4. **Worktree preservation on conflict**: Merge conflicts preserve the worktree for human inspection rather than cleaning it up immediately.

5. **Event-driven + periodic heartbeats**: Combines event-triggered updates (low latency on active tasks) with periodic updates (catches inactive tasks that stop producing events).

6. **Reuse on re-dispatch**: If a task branch or worktree already exists (from a previous failed attempt), the setup phase reuses them rather than failing.
