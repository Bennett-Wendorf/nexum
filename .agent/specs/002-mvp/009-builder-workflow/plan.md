# Plan: 009 - Builder Workflow

## Task Description
Implement the Builder Workflow — the end-to-end orchestration pipeline that assigns a queued task to a builder agent, spawns an isolated worktree, manages the ACP session lifecycle, executes the task, detects completion, merges the task branch into the plan branch, and cleans up all resources. This is the integration chunk that wires together the persistence layer, git operations, ACP client, and overlord deterministic core into a cohesive, working flow. The Builder Workflow is the core execution engine of nexum: it transforms a `queued` task into a `completed` task through a series of deterministic steps managed by the Overlord.

## Objective
Create a fully functional `src/builder/` module that:
- Implements a `BuilderWorkflowOrchestrator` that ties together persistence, git, ACP client, and overlord
- Dispatches queued tasks to builder agents (one task per agent, fully independent)
- Creates isolated git worktrees per task from the plan branch
- Spawns agent subprocesses and manages ACP session lifecycle (create → run → interact → complete → destroy)
- Streams ACP events to the central event bus for frontend relay
- Monitors task execution via heartbeat updates in `status.json`
- Detects task completion via ACP event stream
- Handles permission requests from agents
- Merges task branches into the plan branch on successful completion
- Cleans up worktrees and task branches after merge
- Handles all error scenarios: agent crash, timeout, merge conflicts, permission denied
- Updates `status.json` state machine transitions at every step
- Provides a clean public API for the REST API and Overlord agent layer (future) to invoke

## Problem Statement
Nexum has all the individual components needed for task execution: persistence layer for state management, git operations for branch/worktree management, ACP client for agent communication, and overlord deterministic core for status machines and concurrency control. However, none of these pieces are wired together into a working end-to-end flow. Without the Builder Workflow, nexum cannot actually execute tasks — it can track their state, manage git branches, and speak ACP, but it cannot orchestrate the full lifecycle from "pick a queued task" to "merge and clean up." This integration gap means the system is a collection of disconnected modules rather than a functional orchestrator.

## Solution Approach
Build a `builder` module in `src/builder/` with the following submodules:

1. **`orchestrator.rs`** — The main `BuilderWorkflowOrchestrator` struct. Coordinates all sub-components (persistence, git, ACP, overlord) to execute the complete task lifecycle. Provides the primary public API: `execute_task(task_context)` which runs the full workflow as an async operation.

2. **`dispatcher.rs`** — Task dispatch logic. Selects queued tasks respecting concurrency limits and dependency constraints. Integrates with the Overlord's concurrency checker and dependency resolver. Manages the pool of active builder sessions.

3. **`worktree_manager.rs`** — Worktree setup and teardown. Creates isolated git worktrees per task, maps worktree paths to task IDs, handles cleanup. Wraps the git module's worktree operations with task-specific context.

4. **`session_manager.rs`** — ACP session lifecycle management. Creates sessions, sends task prompts, streams events, handles permissions, detects completion, destroys sessions. Wraps the ACP client module with workflow-specific behavior.

5. **`completion_handler.rs`** — Completion detection and post-execution logic. Detects completion from ACP events, triggers merge workflow, handles merge conflicts, updates status, cleans up resources.

6. **`heartbeat.rs`** — Heartbeat management. Periodically updates `heartbeat_at` in `status.json` based on ACP progress events. Provides the data the Overlord's heartbeat monitor uses for stale detection.

7. **`merge_coordinator.rs`** — Merge coordination. Merges task branch into plan branch, handles merge conflicts, updates `execution.json` task status map, triggers dependency auto-queue for downstream tasks.

8. **`error_recovery.rs`** — Error handling and recovery. Handles agent crashes (destroy session, re-queue task), timeouts (destroy session, transition to queued), merge conflicts (transition to waiting-manual-review), and permission denied scenarios.

9. **`event_bus.rs`** — Event relay. Collects ACP events from all active sessions and broadcasts them to the central event bus for WebSocket frontend relay.

10. **`errors.rs`** — Custom error type `BuilderError` using `thiserror` with variants for workflow failures, merge conflicts, session errors, and timeout errors.

11. **`tests.rs`** — Comprehensive unit and integration tests using mock agents and temporary git repositories.

The module depends on:
- `persistence` module (chunk 003) for all file I/O and schema types
- `git` module (chunk 005) for branch, worktree, and merge operations
- `acp` module (chunk 006) for agent subprocess and session management
- `overlord` module (chunk 004) for status machines, concurrency checking, and dependency resolution
- `config` module (chunk 002) for agent configuration and timeout settings

## Relevant Files

### Existing Files
- `design/agent-harness-integration.md` — Session lifecycle, subprocess management, event relay
- `design/agent-roles.md` — Builder role definition (single task execution)
- `design/persistence.md` — Status.json schema, branch strategy, heartbeat mechanism, recovery
- `design/work-statuses.md` — Task status machine transitions
- `design/unit-of-work.md` — Dependency chaining rules
- `design/resource-constraints.md` — Concurrency enforcement
- `design/implementation-chunks.md` — Defines this as chunk 9 of the MVP
- `src/persistence/` — Persistence layer module (completed by chunk 003)
- `src/git/` — Git operations module (completed by chunk 005)
- `src/acp/` — ACP client module (completed by chunk 006)
- `src/overlord/` — Overlord deterministic core (completed by chunk 004)
- `src/config/` — Configuration module (completed by chunk 002)

### New Files (if needed)
- `src/builder/mod.rs` — Module root, re-exports public types and functions
- `src/builder/orchestrator.rs` — BuilderWorkflowOrchestrator main coordinator
- `src/builder/dispatcher.rs` — Task dispatch and queue management
- `src/builder/worktree_manager.rs` — Worktree setup and teardown
- `src/builder/session_manager.rs` — ACP session lifecycle management
- `src/builder/completion_handler.rs` — Completion detection and post-execution
- `src/builder/heartbeat.rs` — Heartbeat management
- `src/builder/merge_coordinator.rs` — Merge coordination
- `src/builder/error_recovery.rs` — Error handling and recovery
- `src/builder/event_bus.rs` — Event relay to central bus
- `src/builder/errors.rs` — BuilderError enum with thiserror
- `src/builder/tests.rs` — Unit and integration tests

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: builder-core-builder
  - Role: Implement orchestrator, dispatcher, worktree manager, and error types
  - Agent: builder

- **Builder**
  - Name: builder-session-builder
  - Role: Implement session manager, heartbeat, completion handler, and event bus
  - Agent: builder

- **Builder**
  - Name: builder-merge-builder
  - Role: Implement merge coordinator and error recovery
  - Agent: builder

- **Builder**
  - Name: builder-tests-builder
  - Role: Write comprehensive unit and integration tests with mock agents
  - Agent: builder

- **Validator**
  - Name: builder-validator
  - Role: Verify end-to-end workflow correctness, error handling, and cleanup behavior
  - Agent: validator

- **Documenter**
  - Name: builder-documenter
  - Role: Generate documentation for completed builder workflow
  - Agent: documenter

## Step by Step Tasks

### 1. Define Builder Error Types
- **Task ID**: builder-errors
- **Depends On**: none
- **Assigned To**: builder-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/errors.rs` with `BuilderError` enum using `thiserror`:
    - `TaskDispatchError(String)` — Failed to dispatch a task
    - `WorktreeError(nexum::git::GitError)` — Wrapped git worktree error
    - `SessionError(nexum::acp::ACPError)` — Wrapped ACP session error
    - `MergeError(nexum::git::GitError)` — Wrapped git merge error
    - `MergeConflict { task_id: String, branch: String, conflicts: Vec<String> }` — Merge conflict detected
    - `TimeoutError { task_id: String, elapsed: Duration, limit: Duration }` — Task exceeded timeout
    - `AgentCrash { task_id: String, exit_code: Option<i32> }` — Agent subprocess crashed
    - `PersistenceError(nexum::persistence::PersistenceError)` — Wrapped persistence error
    - `OverlordError(nexum::overlord::OverlordError)` — Wrapped overlord error
    - `PermissionDenied { task_id: String, resource: String }` — Agent permission request denied
    - `WorkflowError(String)` — General workflow error
  - Implement `From` implementations for all wrapped error types: `GitError`, `ACPError`, `PersistenceError`, `OverlordError`
  - Define type alias: `pub type Result<T> = std::result::Result<T, BuilderError>`
- **Acceptance Criteria**:
  - `BuilderError` compiles with `thiserror::Error` derive
  - All error variants include sufficient context for debugging
  - `From` implementations allow `?` operator with git, acp, persistence, and overlord errors
  - `Result<T>` type alias is defined and accessible

### 2. Implement Worktree Manager
- **Task ID**: worktree-manager
- **Depends On**: builder-errors
- **Assigned To**: builder-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/worktree_manager.rs` with:
    
    **Worktree struct**:
    - `pub struct TaskWorktree { task_id: String, task_name: String, branch_name: String, path: PathBuf, created_at: DateTime<Utc> }`
    - `fn branch_name(task_id: &str) -> String` — Returns `task/<TASK_ID>` (e.g., `task/TASK-001`)
    - `fn worktree_path(repo_root: &Path, task_id: &str) -> PathBuf` — Returns `.worktrees/<TASK_ID>-<task_name>/`
    
    **Setup**:
    - `pub struct WorktreeManager { repo_root: PathBuf }`
    - `fn new(repo_root: PathBuf) -> Self` — Initialize
    - `pub async fn setup(&self, task_id: &str, task_name: &str, plan_branch: &str) -> Result<TaskWorktree>` — Create worktree for a task:
      1. Compute branch name: `task/<TASK_ID>`
      2. Compute worktree path: `.worktrees/<TASK_ID>-<task_name>/`
      3. Create task branch from plan branch: `git checkout -b task/TASK-001 feature/auth-overhaul`
      4. Spawn worktree: `git worktree add <path> task/TASK-001`
      5. Return `TaskWorktree` with all metadata
    - `pub async fn setup_and_commit_agent_dir(&self, task_id: &str, task_name: &str, plan_branch: &str) -> Result<TaskWorktree>` — Setup worktree and ensure `.agent/` directory structure exists:
      1. Call `setup()` above
      2. In the worktree, ensure `.agent/specs/<branch>/` directory structure mirrors the repo root
      3. Symlink or copy task files into the worktree's `.agent/` structure
    
    **Teardown**:
    - `pub async fn cleanup(&self, worktree: &TaskWorktree) -> Result<()>` — Remove worktree and delete task branch:
      1. `git worktree remove <path>` — Remove the worktree
      2. `git branch -D task/<TASK_ID>` — Delete the task branch
      3. Remove the worktree directory if `git worktree remove` fails (force cleanup)
    
    **Validation**:
    - `pub async fn exists(&self, task_id: &str) -> bool` — Check if a worktree exists for a task
    - `pub async fn list_active(&self) -> Result<Vec<TaskWorktree>>` — List all active worktrees
  - Use git module's `worktree::add()`, `worktree::remove()`, `branch::create()`, `branch::delete()`
  - Map `GitError` to `BuilderError` via `From` implementation
  - Document the worktree lifecycle referencing `design/persistence.md` branch strategy
- **Acceptance Criteria**:
  - `setup()` creates a task branch from the plan branch
  - `setup()` spawns a worktree at the correct path
  - `setup()` returns a `TaskWorktree` with correct metadata
  - `cleanup()` removes the worktree via `git worktree remove`
  - `cleanup()` deletes the task branch via `git branch -D`
  - `cleanup()` force-removes directory if git command fails
  - `exists()` correctly reports worktree existence
  - `list_active()` returns all active worktrees
  - Handles missing branches gracefully
  - Handles stale worktrees (directory exists but git worktree is gone)

### 3. Implement Task Dispatcher
- **Task ID**: task-dispatcher
- **Depends On**: builder-errors
- **Assigned To**: builder-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/dispatcher.rs` with:
    
    **Dispatcher struct**:
    - `pub struct TaskDispatcher { repo_root: PathBuf, overlord: &'a OverlordScheduler, concurrency_checker: ConcurrencyChecker }`
    - `fn new(repo_root: PathBuf, overlord: &'a OverlordScheduler) -> Self` — Initialize
    
    **Task selection**:
    - `pub async fn find_next_task(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<Option<TaskContext>>` — Find the next dispatchable task:
      1. Read `execution.json` to get task list and status map
      2. Filter tasks with status `queued`
      3. For each queued task, check `concurrency_checker.can_dispatch()`
      4. If no dispatchable tasks, return `None`
      5. Return `TaskContext` for the first eligible task
    - `pub struct TaskContext { task_id: String, task_name: String, plan_id: String, plan_name: String, branch: String, task_dir: PathBuf, task_prompt: String }`
    
    **Active session tracking**:
    - `pub struct ActiveSession { task_context: TaskContext, worktree: TaskWorktree, session_id: String, pid: Option<u32>, started_at: DateTime<Utc> }`
    - `fn add_session(&self, session: ActiveSession)` — Track an active session
    - `fn remove_session(&self, task_id: &str) -> Option<ActiveSession>` — Remove a completed session
    - `fn get_active_count(&self) -> usize` — Count active sessions
    - `fn get_session(&self, task_id: &str) -> Option<&ActiveSession>` — Get session by task ID
    - `fn list_active(&self) -> Vec<&ActiveSession>` — List all active sessions
    
    **Dispatch**:
    - `pub async fn dispatch_task(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<Option<TaskContext>>` — Dispatch the next available task:
      1. Find next task via `find_next_task()`
      2. If found, transition `queued` → `running` via overlord's `transition_task_status()`
      3. Actor: `"overlord-dispatch"`
      4. Return the task context
  - Use overlord module's `ConcurrencyChecker` and `OverlordScheduler` for status transitions
  - Use persistence module's `read_execution_state()` and `read_task_status()` for state reads
  - Document the dispatch logic referencing `design/resource-constraints.md`
- **Acceptance Criteria**:
  - `find_next_task()` returns `None` when no queued tasks exist
  - `find_next_task()` returns `None` when concurrency limit is reached
  - `find_next_task()` returns the first eligible queued task
  - `dispatch_task()` transitions task from `queued` to `running`
  - `dispatch_task()` records transition with actor `"overlord-dispatch"`
  - Active session tracking correctly adds, removes, and lists sessions
  - `get_active_count()` returns accurate count
  - Handles missing `execution.json` gracefully

### 4. Implement Session Manager
- **Task ID**: session-manager
- **Depends On**: builder-errors, worktree-manager
- **Assigned To**: builder-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/session_manager.rs` with:
    
    **Session manager struct**:
    - `pub struct SessionManager { repo_root: PathBuf, agent_config: AgentConfig }`
    - `fn new(repo_root: PathBuf, agent_config: AgentConfig) -> Self` — Initialize
    
    **Session creation**:
    - `pub async fn create_session(&self, task_context: &TaskContext, worktree: &TaskWorktree) -> Result<ACPSessionHandle>` — Create an ACP session:
      1. Read task prompt from `task.md` (Description, Acceptance criteria, Files to modify, Notes sections)
      2. Format the prompt as a structured message for the agent
      3. Spawn agent subprocess in the worktree directory
      4. Initialize ACP client connection (JSON-RPC over stdio)
      5. Call `sessions/create` with the task prompt
      6. Return `ACPSessionHandle` wrapping the ACP session
    - `pub struct ACPSessionHandle { session_id: String, client: ACPClient, subprocess: Child, event_rx: Receiver<ACPEvent>, started_at: DateTime<Utc> }`
    
    **Event streaming**:
    - `pub fn get_event_stream(&self, handle: &ACPSessionHandle) -> Receiver<ACPEvent>` — Get event receiver for a session
    - `pub async fn send_message(&self, handle: &ACPSessionHandle, message: &str) -> Result<()>` — Send a follow-up message to the agent
    
    **Permission handling**:
    - `pub async fn handle_permission_request(&self, handle: &ACPSessionHandle, permission_id: &str, approve: bool) -> Result<()>` — Respond to a permission request:
      1. If `approve` is true, send approval via ACP
      2. If `approve` is false, send denial via ACP
      3. Record the decision in logs
    
    **Session destruction**:
    - `pub async fn destroy_session(&self, handle: &ACPSessionHandle) -> Result<()>` — Destroy the ACP session:
      1. Call `sessions/destroy` via ACP
      2. Send SIGTERM to subprocess, wait up to 5 seconds
      3. If subprocess hasn't exited, send SIGKILL
      4. Reap the subprocess
      5. Clean up client connection
    
    **Completion detection**:
    - `pub fn is_completed(&self, handle: &ACPSessionHandle) -> bool` — Check if the session has completed
    - `pub async fn wait_for_completion(&self, handle: &ACPSessionHandle, timeout: Duration) -> Result<CompletionResult>` — Wait for completion with timeout:
      1. Monitor event stream for completion event
      2. If timeout elapsed, return `CompletionResult::Timeout`
      3. If agent crashes (subprocess exits), return `CompletionResult::Crashed`
      4. If completion event received, return `CompletionResult::Completed`
    - `pub enum CompletionResult { Completed, Timeout(Duration), Crashed { exit_code: Option<i32> } }`
  - Use ACP client module's subprocess spawning and session lifecycle
  - Use config module's agent configuration for spawn command and working directory
  - Document the session lifecycle referencing `design/agent-harness-integration.md`
- **Acceptance Criteria**:
  - `create_session()` spawns agent subprocess in worktree directory
  - `create_session()` initializes ACP client connection
  - `create_session()` sends task prompt via `sessions/create`
  - `create_session()` returns `ACPSessionHandle` with event stream
  - `send_message()` sends follow-up messages via ACP
  - `handle_permission_request()` responds to agent permission requests
  - `destroy_session()` calls `sessions/destroy`
  - `destroy_session()` sends SIGTERM, waits 5 seconds, then SIGKILL
  - `destroy_session()` reaps the subprocess
  - `wait_for_completion()` returns `Completed` on completion event
  - `wait_for_completion()` returns `Timeout` if timeout elapsed
  - `wait_for_completion()` returns `Crashed` if subprocess exits unexpectedly
  - Event stream correctly relays ACP events

### 5. Implement Heartbeat Manager
- **Task ID**: heartbeat-manager
- **Depends On**: builder-errors
- **Assigned To**: builder-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/heartbeat.rs` with:
    
    **Heartbeat struct**:
    - `pub struct HeartbeatManager { repo_root: PathBuf, interval: Duration }`
    - `fn new(repo_root: PathBuf, interval: Duration) -> Self` — Initialize with configurable interval (default: 5 minutes)
    
    **Heartbeat updates**:
    - `pub async fn update_heartbeat(&self, task_id: &str, task_name: &str, branch: &str, plan_id: &str, plan_name: &str) -> Result<()>` — Update heartbeat timestamp:
      1. Read `status.json` from `.agent/specs/<branch>/<plan_id>-<plan_name>/tasks/<task_id>-<task_name>/`
      2. Set `heartbeat_at` to current UTC timestamp (RFC 3339)
      3. Atomically write via temp + rename (using persistence layer)
    - `pub async fn start_periodic_heartbeat(&self, task_context: &TaskContext) -> Result<JoinHandle<()>>` — Start a background task that periodically updates heartbeat:
      1. Spawn async task
      2. Loop: update heartbeat, sleep for interval
      3. Stop when task context's cancellation token is dropped
    
    **Event-driven heartbeat**:
    - `pub async fn update_on_event(&self, event: &ACPEvent, task_context: &TaskContext) -> Result<()>` — Update heartbeat on ACP progress events:
      1. If event is a progress event (tool_call, tool_result, progress), update heartbeat
      2. If event is a completion event, do not update (task is done)
      3. If event is a question or permission request, update heartbeat
    - `pub fn is_progress_event(event: &ACPEvent) -> bool` — Check if an event should trigger a heartbeat update
  - Use persistence layer's atomic write for `status.json` updates
  - Use `tokio::time::interval` for periodic updates
  - Use `tokio::task::spawn` for background heartbeat task
  - Document the heartbeat mechanism referencing `design/persistence.md` heartbeat section
- **Acceptance Criteria**:
  - `update_heartbeat()` updates `heartbeat_at` in `status.json`
  - `update_heartbeat()` uses atomic write (temp + rename)
  - `start_periodic_heartbeat()` spawns a background task
  - `start_periodic_heartbeat()` updates heartbeat at configured interval
  - `start_periodic_heartbeat()` stops when cancellation token is dropped
  - `update_on_event()` triggers heartbeat for progress events
  - `update_on_event()` does not trigger heartbeat for completion events
  - `is_progress_event()` correctly identifies progress-triggering events
  - Handles missing `status.json` gracefully

### 6. Implement Event Bus Integration
- **Task ID**: event-bus
- **Depends On**: builder-errors
- **Assigned To**: builder-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/event_bus.rs` with:
    
    **Event bus struct**:
    - `pub struct BuilderEventBus { tx: tokio::sync::broadcast::Sender<BuilderEvent>, active_sessions: HashMap<String, tokio::sync::broadcast::Sender<ACPEvent>> }`
    - `fn new(capacity: usize) -> Self` — Initialize with broadcast channel capacity
    - `pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<BuilderEvent>` — Subscribe to events
    
    **Event types**:
    - `pub enum BuilderEvent { TaskStarted(TaskContext), TaskProgress { task_id: String, event: ACPEvent }, TaskCompleted { task_id: String, result: CompletionResult }, TaskFailed { task_id: String, error: String }, TaskMerged { task_id: String }, TaskCleanedUp { task_id: String }, TaskRequeued { task_id: String } }`
    
    **Event relay**:
    - `pub async fn relay_session_events(&self, task_id: &str, mut event_rx: Receiver<ACPEvent>) -> JoinHandle<()>` — Relay ACP events from a session to the central bus:
      1. Spawn async task
      2. Loop: receive ACP events from session
      3. Wrap each event in `BuilderEvent::TaskProgress`
      4. Broadcast to central bus
      5. Stop when session event stream ends
    - `pub fn emit(&self, event: BuilderEvent) -> Result<()>` — Emit a builder event
    - `pub fn register_session(&self, task_id: &str, tx: tokio::sync::broadcast::Sender<ACPEvent>)` — Register a session's event stream
    - `pub fn unregister_session(&self, task_id: &str)` — Unregister a session's event stream
  - Use `tokio::sync::broadcast` for many-to-many event distribution
  - Events flow: ACP session → session event stream → BuilderEventBus → central event bus → WebSocket → frontend
  - Document the event relay referencing `design/agent-harness-integration.md` event relay section
- **Acceptance Criteria**:
  - `BuilderEventBus` creates a broadcast channel with configurable capacity
  - `subscribe()` returns a receiver for builder events
  - `relay_session_events()` spawns a background relay task
  - `relay_session_events()` wraps ACP events in `BuilderEvent::TaskProgress`
  - `relay_session_events()` broadcasts to all subscribers
  - `emit()` broadcasts builder lifecycle events
  - `register_session()` and `unregister_session()` manage session event streams
  - Multiple subscribers can receive events simultaneously
  - Relay task stops when session event stream ends

### 7. Implement Merge Coordinator
- **Task ID**: merge-coordinator
- **Depends On**: builder-errors, worktree-manager
- **Assigned To**: builder-merge-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/merge_coordinator.rs` with:
    
    **Merge coordinator struct**:
    - `pub struct MergeCoordinator { repo_root: PathBuf, overlord: &'a OverlordScheduler }`
    - `fn new(repo_root: PathBuf, overlord: &'a OverlordScheduler) -> Self` — Initialize
    
    **Merge execution**:
    - `pub async fn merge_task_branch(&self, task_context: &TaskContext, worktree: &TaskWorktree) -> Result<MergeResult>` — Merge task branch into plan branch:
      1. Switch to plan branch: `git checkout <plan_branch>`
      2. Merge task branch: `git merge task/<TASK_ID> --no-ff -m "Merge task/<TASK_ID>: <task_name>"`
      3. If merge succeeds (exit code 0), return `MergeResult::Success`
      4. If merge fails with conflicts (exit code 1), return `MergeResult::Conflict { conflicts }`
      5. If merge fails for other reasons, return error
    - `pub enum MergeResult { Success, Conflict { conflicted_files: Vec<String> } }`
    
    **Post-merge**:
    - `pub async fn post_merge_cleanup(&self, task_context: &TaskContext, worktree: &TaskWorktree) -> Result<()>` — Clean up after successful merge:
      1. Delete task branch: `git branch -D task/<TASK_ID>`
      2. Remove worktree: `git worktree remove <path>`
      3. Force remove directory if needed
    - `pub async fn update_execution_state(&self, task_context: &TaskContext, new_status: TaskStatusValue) -> Result<()>` — Update execution.json task status map:
      1. Read `execution.json`
      2. Update `task_status_map` entry for this task
      3. Atomically write via persistence layer
    
    **Conflict handling**:
    - `pub async fn handle_merge_conflict(&self, task_context: &TaskContext) -> Result<()>` — Handle a merge conflict:
      1. Abort the merge: `git merge --abort`
      2. Transition task to `waiting-manual-review` via overlord
      3. Record conflict details in status.json
      4. Actor: `"overlord-merge-conflict"`
    
    **Dependency auto-queue**:
    - `pub async fn trigger_dependency_auto_queue(&self, task_context: &TaskContext) -> Result<Vec<String>>` — After merge, check if dependent tasks are now eligible:
      1. Use overlord's `dependency_resolver.resolve_dependent_tasks()`
      2. Return list of newly eligible task IDs
  - Use git module's `merge::merge()`, `merge::abort()`, `branch::delete()`
  - Use overlord module's `transition_task_status()` for status transitions
  - Use persistence module's atomic write for execution state updates
  - Document the merge process referencing `design/persistence.md` branch strategy
- **Acceptance Criteria**:
  - `merge_task_branch()` merges task branch into plan branch with `--no-ff`
  - `merge_task_branch()` returns `MergeResult::Success` on successful merge
  - `merge_task_branch()` returns `MergeResult::Conflict` on merge conflicts
  - `post_merge_cleanup()` deletes task branch
  - `post_merge_cleanup()` removes worktree
  - `update_execution_state()` updates `task_status_map` in execution.json
  - `handle_merge_conflict()` aborts the merge
  - `handle_merge_conflict()` transitions task to `waiting-manual-review`
  - `handle_merge_conflict()` records conflict with actor `"overlord-merge-conflict"`
  - `trigger_dependency_auto_queue()` identifies newly eligible downstream tasks

### 8. Implement Error Recovery
- **Task ID**: error-recovery
- **Depends On**: builder-errors, session-manager, merge-coordinator
- **Assigned To**: builder-merge-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/error_recovery.rs` with:
    
    **Error recovery struct**:
    - `pub struct ErrorRecovery { repo_root: PathBuf, overlord: &'a OverlordScheduler, worktree_manager: &'a WorktreeManager, session_manager: &'a SessionManager }`
    - `fn new(repo_root: PathBuf, overlord: &'a OverlordScheduler, worktree_manager: &'a WorktreeManager, session_manager: &'a SessionManager) -> Self` — Initialize
    
    **Agent crash recovery**:
    - `pub async fn handle_agent_crash(&self, task_context: &TaskContext, worktree: &TaskWorktree, session: &ACPSessionHandle) -> Result<()>` — Handle agent crash:
      1. Destroy the ACP session: `session_manager.destroy_session()`
      2. Transition task from `running` to `queued` via overlord
      3. Increment `attempts` counter in status.json
      4. Clear agent lease (set `agent` to `None`)
      5. Clear `started_at`
      6. Clean up worktree: `worktree_manager.cleanup()`
      7. Actor: `"overlord-crash-recovery"`
    
    **Timeout handling**:
    - `pub async fn handle_timeout(&self, task_context: &TaskContext, worktree: &TaskWorktree, session: &ACPSessionHandle, elapsed: Duration, limit: Duration) -> Result<()>` — Handle task timeout:
      1. Destroy the ACP session: `session_manager.destroy_session()`
      2. Transition task from `running` to `queued` via overlord
      3. Increment `attempts` counter in status.json
      4. Record timeout details in status.json
      5. Clear agent lease
      6. Clean up worktree
      7. Actor: `"overlord-timeout"`
    
    **Permission denied**:
    - `pub async fn handle_permission_denied(&self, task_context: &TaskContext, session: &ACPSessionHandle, resource: &str) -> Result<()>` — Handle permission denied:
      1. Log the permission denial
      2. Agent decides whether to proceed (may continue or abort)
      3. If agent aborts, treat as crash (use crash recovery flow)
    
    **Merge conflict recovery**:
    - `pub async fn handle_merge_conflict(&self, task_context: &TaskContext, worktree: &TaskWorktree, conflicts: Vec<String>) -> Result<()>` — Handle merge conflict:
      1. Transition task to `waiting-manual-review` via overlord
      2. Record conflict details in status.json
      3. Do NOT clean up worktree (human may need to inspect)
      4. Actor: `"overlord-merge-conflict"`
    
    **General error handler**:
    - `pub async fn handle_workflow_error(&self, task_context: &TaskContext, worktree: &TaskWorktree, session: Option<&ACPSessionHandle>, error: &BuilderError) -> Result<()>` — Generic error handler:
      1. Match error type to specific handler
      2. Log the error with full context
      3. Execute appropriate recovery flow
      4. Emit `BuilderEvent::TaskFailed` to event bus
  - Use overlord module's `transition_task_status()` for all status transitions
  - Use persistence module's atomic write for status.json updates
  - All recovery flows must be safe (no double-free of worktrees, no orphaned sessions)
  - Document error handling referencing `design/agent-harness-integration.md` error handling section
- **Acceptance Criteria**:
  - `handle_agent_crash()` destroys the ACP session
  - `handle_agent_crash()` transitions task from `running` to `queued`
  - `handle_agent_crash()` increments `attempts` counter
  - `handle_agent_crash()` clears agent lease and `started_at`
  - `handle_agent_crash()` cleans up worktree
  - `handle_agent_crash()` records transition with actor `"overlord-crash-recovery"`
  - `handle_timeout()` destroys the ACP session
  - `handle_timeout()` transitions task from `running` to `queued`
  - `handle_timeout()` records timeout details
  - `handle_timeout()` records transition with actor `"overlord-timeout"`
  - `handle_permission_denied()` logs the denial
  - `handle_merge_conflict()` transitions task to `waiting-manual-review`
  - `handle_merge_conflict()` does NOT clean up worktree (preserves for inspection)
  - `handle_merge_conflict()` records transition with actor `"overlord-merge-conflict"`
  - `handle_workflow_error()` dispatches to correct handler based on error type
  - `handle_workflow_error()` emits `BuilderEvent::TaskFailed`

### 9. Implement Completion Handler
- **Task ID**: completion-handler
- **Depends On**: builder-errors, session-manager, merge-coordinator, error-recovery
- **Assigned To**: builder-session-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/completion_handler.rs` with:
    
    **Completion handler struct**:
    - `pub struct CompletionHandler { repo_root: PathBuf, overlord: &'a OverlordScheduler, merge_coordinator: &'a MergeCoordinator, error_recovery: &'a ErrorRecovery }`
    - `fn new(repo_root: PathBuf, overlord: &'a OverlordScheduler, merge_coordinator: &'a MergeCoordinator, error_recovery: &'a ErrorRecovery) -> Self` — Initialize
    
    **Completion processing**:
    - `pub async fn handle_completion(&self, task_context: &TaskContext, worktree: &TaskWorktree, session: &ACPSessionHandle) -> Result<CompletionOutcome>` — Process task completion:
      1. Transition task from `running` to `reviewing` via overlord (or `merge-queue` if auto-merge is enabled)
      2. Record `completed_at` timestamp in status.json
      3. Actor: `"builder-completion"`
      4. Attempt merge via `merge_coordinator.merge_task_branch()`
      5. On merge success:
         - Transition task from `reviewing` (or `merge-queue`) to `completed`
         - Clean up via `merge_coordinator.post_merge_cleanup()`
         - Trigger dependency auto-queue via `merge_coordinator.trigger_dependency_auto_queue()`
         - Return `CompletionOutcome::Merged`
      6. On merge conflict:
         - Handle via `error_recovery.handle_merge_conflict()`
         - Return `CompletionOutcome::Conflict`
    - `pub enum CompletionOutcome { Merged, Conflict }`
    
    **Post-completion**:
    - `pub async fn emit_completion_events(&self, task_context: &TaskContext, outcome: &CompletionOutcome) -> Result<()>` — Emit completion events to event bus
  - Use overlord module's `transition_task_status()` for all status transitions
  - Use merge coordinator's merge and cleanup operations
  - Use error recovery's conflict handling
  - Document the completion flow referencing `design/agent-harness-integration.md` session lifecycle
- **Acceptance Criteria**:
  - `handle_completion()` transitions task from `running` to `reviewing` (or `merge-queue`)
  - `handle_completion()` records `completed_at` timestamp
  - `handle_completion()` records transition with actor `"builder-completion"`
  - `handle_completion()` attempts merge via merge coordinator
  - `handle_completion()` transitions to `completed` on successful merge
  - `handle_completion()` cleans up worktree and branch on successful merge
  - `handle_completion()` triggers dependency auto-queue after merge
  - `handle_completion()` handles merge conflicts via error recovery
  - `handle_completion()` returns `CompletionOutcome::Merged` on success
  - `handle_completion()` returns `CompletionOutcome::Conflict` on merge conflict
  - `emit_completion_events()` broadcasts completion events

### 10. Implement Workflow Orchestrator
- **Task ID**: workflow-orchestrator
- **Depends On**: worktree-manager, task-dispatcher, session-manager, heartbeat-manager, event-bus, merge-coordinator, error-recovery, completion-handler
- **Assigned To**: builder-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/orchestrator.rs` with:
    
    **Orchestrator struct**:
    - `pub struct BuilderWorkflowOrchestrator { repo_root: PathBuf, dispatcher: TaskDispatcher, worktree_manager: WorktreeManager, session_manager: SessionManager, heartbeat_manager: HeartbeatManager, event_bus: BuilderEventBus, merge_coordinator: MergeCoordinator, error_recovery: ErrorRecovery, completion_handler: CompletionHandler }`
    - `fn new(repo_root: PathBuf, overlord: &'a OverlordScheduler, agent_config: AgentConfig) -> Self` — Initialize all sub-components
    
    **Main execution flow**:
    - `pub async fn execute_task(&self, task_context: TaskContext) -> Result<CompletionOutcome>` — Execute the full builder workflow:
      1. **Setup phase**:
         - Create worktree: `worktree_manager.setup()`
         - Emit `BuilderEvent::TaskStarted`
      2. **Session phase**:
         - Create ACP session: `session_manager.create_session()`
         - Start heartbeat: `heartbeat_manager.start_periodic_heartbeat()`
         - Start event relay: `event_bus.relay_session_events()`
      3. **Execution phase**:
         - Wait for completion with timeout: `session_manager.wait_for_completion()`
         - On progress events, update heartbeat: `heartbeat_manager.update_on_event()`
      4. **Completion phase** (on successful completion):
         - Handle completion: `completion_handler.handle_completion()`
         - Destroy session: `session_manager.destroy_session()`
         - Emit `BuilderEvent::TaskCompleted` or `BuilderEvent::TaskMerged`
      5. **Error phase** (on crash, timeout, or other error):
         - Handle error: `error_recovery.handle_workflow_error()`
         - Destroy session if still active: `session_manager.destroy_session()`
         - Clean up worktree if needed
         - Emit `BuilderEvent::TaskFailed`
      6. **Cleanup phase**:
         - Stop heartbeat task
         - Unregister session from event bus
         - Clean up worktree (if not already done)
         - Emit `BuilderEvent::TaskCleanedUp`
    
    **Public API**:
    - `pub async fn dispatch_and_execute(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<Option<CompletionOutcome>>` — Dispatch next task and execute:
      1. `dispatcher.dispatch_task()` to find and claim next task
      2. If task found, `execute_task()` to run the full workflow
      3. Return outcome or `None` if no tasks available
    - `pub async fn execute_all_ready_tasks(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<Vec<CompletionOutcome>>` — Execute all ready tasks up to concurrency limit:
      1. Loop: `dispatch_and_execute()` until no more tasks or concurrency limit reached
      2. Return list of outcomes
    - `pub fn get_active_sessions(&self) -> Vec<&ActiveSession>` — Get list of active sessions
    - `pub fn get_event_bus(&self) -> &BuilderEventBus` — Get event bus reference
  - All sub-components share the same `repo_root` and `overlord` reference
  - The orchestrator is the single entry point for the builder workflow
  - Document the complete workflow referencing `design/agent-harness-integration.md` session lifecycle
- **Acceptance Criteria**:
  - `execute_task()` follows the complete workflow: setup → session → execute → completion → cleanup
  - `execute_task()` creates worktree before spawning agent
  - `execute_task()` starts heartbeat before agent execution
  - `execute_task()` relays events to event bus during execution
  - `execute_task()` handles completion via completion handler
  - `execute_task()` handles errors via error recovery
  - `execute_task()` cleans up all resources (session, worktree, heartbeat)
  - `execute_task()` emits appropriate builder events at each phase
  - `dispatch_and_execute()` claims a task and executes it
  - `dispatch_and_execute()` returns `None` when no tasks available
  - `execute_all_ready_tasks()` executes tasks up to concurrency limit
  - `get_active_sessions()` returns list of active sessions
  - `get_event_bus()` returns event bus reference

### 11. Wire Up Module Exports
- **Task ID**: builder-module-wiring
- **Depends On**: workflow-orchestrator
- **Assigned To**: builder-core-builder
- **Agent**: builder
- **Actions**:
  - Update `src/builder/mod.rs` to:
    - Declare submodules: `mod errors; mod orchestrator; mod dispatcher; mod worktree_manager; mod session_manager; mod completion_handler; mod heartbeat; mod merge_coordinator; mod error_recovery; mod event_bus;`
    - Re-export public types:
      - `pub use errors::*;` (BuilderError, Result)
      - `pub use orchestrator::*;` (BuilderWorkflowOrchestrator)
      - `pub use dispatcher::*;` (TaskDispatcher, TaskContext, ActiveSession)
      - `pub use worktree_manager::*;` (WorktreeManager, TaskWorktree)
      - `pub use session_manager::*;` (SessionManager, ACPSessionHandle, CompletionResult)
      - `pub use completion_handler::*;` (CompletionHandler, CompletionOutcome)
      - `pub use heartbeat::*;` (HeartbeatManager)
      - `pub use merge_coordinator::*;` (MergeCoordinator, MergeResult)
      - `pub use error_recovery::*;` (ErrorRecovery)
      - `pub use event_bus::*;` (BuilderEventBus, BuilderEvent)
    - Include module-level documentation referencing `design/agent-harness-integration.md` and `design/agent-roles.md`
  - Add `mod builder;` to `src/main.rs` (if not already present)
  - Update `src/main.rs` to initialize `BuilderWorkflowOrchestrator` on startup:
    - Create orchestrator with repo root, overlord reference, and agent config
    - Store in application state for REST API access
    - Log orchestrator startup via `tracing`
- **Acceptance Criteria**:
  - All public types are accessible as `nexum::builder::BuilderWorkflowOrchestrator`, etc.
  - All public functions are accessible via the module re-exports
  - Module compiles without errors
  - Module documentation references design docs
  - `src/main.rs` initializes the orchestrator on startup
  - Orchestrator is accessible via REST API (through application state)
  - Orchestrator startup is logged via tracing

### 12. Write Unit and Integration Tests
- **Task ID**: builder-tests
- **Depends On**: builder-module-wiring
- **Assigned To**: builder-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `src/builder/tests.rs` with comprehensive tests:
    
    **Worktree manager tests**:
    - `test_worktree_setup` — Create worktree for a task
    - `test_worktree_cleanup` — Remove worktree and delete branch
    - `test_worktree_exists` — Check worktree existence
    - `test_worktree_cleanup_force` — Force cleanup when git command fails
    - `test_worktree_path` — Verify worktree path format
    
    **Dispatcher tests**:
    - `test_find_next_task_none` — No queued tasks returns None
    - `test_find_next_task_concurrency_limit` — Concurrency limit blocks dispatch
    - `test_find_next_task_eligible` — Eligible task returned
    - `test_dispatch_task_transition` — Task transitions from queued to running
    - `test_active_session_tracking` — Sessions tracked correctly
    
    **Session manager tests** (with mock ACP server):
    - `test_create_session` — Session created with correct prompt
    - `test_send_message` — Follow-up message sent
    - `test_destroy_session` — Session destroyed, subprocess killed
    - `test_wait_for_completion_completed` — Completion detected
    - `test_wait_for_completion_timeout` — Timeout detected
    - `test_wait_for_completion_crashed` — Crash detected
    
    **Heartbeat tests**:
    - `test_update_heartbeat` — Heartbeat timestamp updated
    - `test_periodic_heartbeat` — Periodic updates work
    - `test_event_driven_heartbeat` — Progress events trigger heartbeat
    - `test_completion_no_heartbeat` — Completion events don't trigger heartbeat
    
    **Merge coordinator tests**:
    - `test_merge_success` — Successful merge
    - `test_merge_conflict` — Merge conflict detected
    - `test_post_merge_cleanup` — Branch and worktree cleaned up
    - `test_update_execution_state` — Execution state updated
    - `test_dependency_auto_queue` — Downstream tasks identified
    
    **Error recovery tests**:
    - `test_agent_crash_recovery` — Crash recovery flow
    - `test_timeout_recovery` — Timeout recovery flow
    - `test_merge_conflict_recovery` — Conflict recovery flow
    - `test_workflow_error_dispatch` — Error type routing
    
    **Completion handler tests**:
    - `test_handle_completion_success` — Successful completion and merge
    - `test_handle_completion_conflict` — Completion with merge conflict
    - `test_emit_completion_events` — Events emitted correctly
    
    **Orchestrator integration tests**:
    - `test_execute_task_full_flow` — Complete workflow with mock agent
    - `test_execute_task_crash_recovery` — Workflow with agent crash
    - `test_execute_task_timeout` — Workflow with timeout
    - `test_execute_task_merge_conflict` — Workflow with merge conflict
    - `test_dispatch_and_execute` — Dispatch and execute flow
    - `test_execute_all_ready_tasks` — Multiple task execution
    
    **Test utilities**:
    - `fn create_test_repo() -> tempfile::TempDir` — Create temporary git repo
    - `fn setup_test_task(dir: &Path, task_id: &str, status: TaskStatusValue) -> Result<()>` — Set up test task
    - `fn create_mock_acp_server() -> MockACPserver` — Mock ACP server for testing
    - `fn create_test_execution_state(dir: &Path, tasks: Vec<(String, TaskStatusValue)>) -> Result<()>` — Create test execution state
  - Use `tempfile` crate for isolated test directories
  - Use `tokio::test` for async tests
  - Create mock ACP server that simulates agent behavior (completion, crash, events)
  - Use `git init` in temp directories for git operations testing
- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum builder`
  - Tests cover all modules: worktree, dispatcher, session, heartbeat, merge, error recovery, completion, orchestrator
  - Tests use temporary directories to avoid polluting the real filesystem
  - Async tests use `#[tokio::test]`
  - Mock ACP server simulates completion, crash, and event streaming
  - Integration tests verify end-to-end workflow
  - Error recovery tests verify all error scenarios

### 13. Final Validation
- **Task ID**: validate-all
- **Depends On**: builder-tests
- **Assigned To**: builder-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum builder` — all builder tests must pass
  - Run `cargo clippy --package nexum` — no warnings in builder module
  - Verify `BuilderWorkflowOrchestrator.execute_task()` follows the complete workflow:
    1. Setup: worktree creation
    2. Session: ACP session creation, heartbeat start, event relay
    3. Execution: wait for completion with timeout
    4. Completion: status transition, merge, cleanup
    5. Error: crash recovery, timeout recovery, conflict handling
    6. Cleanup: session destroy, worktree removal, event bus unregister
  - Verify task status transitions follow `design/work-statuses.md`:
    - `queued` → `running` (dispatch)
    - `running` → `reviewing` or `merge-queue` (completion)
    - `reviewing` → `waiting-manual-review` or `merge-queue` (review)
    - `merge-queue` → `completed` (merge success)
    - `running` → `queued` (crash/timeout recovery)
  - Verify worktree lifecycle: create from plan branch, cleanup after merge
  - Verify task branches are `task/<TASK_ID>` format, never pushed to remote
  - Verify ACP session lifecycle: create → run → interact → complete → destroy
  - Verify heartbeat updates `heartbeat_at` in status.json
  - Verify heartbeat interval is configurable (default 5 minutes)
  - Verify event relay broadcasts ACP events to central bus
  - Verify merge uses `--no-ff` for auditability
  - Verify merge conflict transitions task to `waiting-manual-review`
  - Verify merge conflict does NOT clean up worktree (preserves for inspection)
  - Verify crash recovery: destroy session, re-queue task, increment attempts, clear lease, cleanup worktree
  - Verify timeout recovery: destroy session, re-queue task, increment attempts, record timeout
  - Verify dependency auto-queue triggered after successful merge
  - Verify actor labels: `"overlord-dispatch"`, `"builder-completion"`, `"overlord-crash-recovery"`, `"overlord-timeout"`, `"overlord-merge-conflict"`
  - Verify `BuilderError` wraps `GitError`, `ACPError`, `PersistenceError`, `OverlordError`
  - Verify all public types are re-exported from `mod.rs`
  - Verify orchestrator is initialized in `src/main.rs`
  - Verify orchestrator is accessible via REST API application state

### 14. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: builder-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the complete builder workflow with a sequence diagram
  - Document the session lifecycle (create → run → interact → complete → destroy)
  - Document the worktree isolation model
  - Document the heartbeat mechanism and stale detection
  - Document the merge coordination and conflict handling
  - Document all error recovery flows (crash, timeout, conflict, permission)
  - Document the event relay architecture
  - Document the public API for external consumers (REST API, agent layer)

## Acceptance Criteria
- `cargo check` succeeds with no errors in the builder module
- `cargo test --package nexum builder` passes all tests
- `cargo clippy --package nexum` produces no warnings in builder module
- `BuilderWorkflowOrchestrator` coordinates all sub-components (persistence, git, ACP, overlord)
- `execute_task()` follows the complete workflow: setup → session → execute → completion → cleanup
- Task dispatch respects concurrency limits via Overlord's `ConcurrencyChecker`
- Task dispatch respects dependency constraints via Overlord's `DependencyResolver`
- Worktree created from plan branch with `task/<TASK_ID>` branch name
- Worktree path is `.worktrees/<TASK_ID>-<task_name>/`
- ACP session created with task prompt from `task.md`
- Agent subprocess spawned in worktree directory
- Heartbeat updates `heartbeat_at` in `status.json` on progress events
- Heartbeat periodic updates at configurable interval (default 5 minutes)
- ACP events relayed to central event bus via `BuilderEventBus`
- Completion detected via ACP event stream
- On completion: transition `running` → `reviewing` or `merge-queue`
- Merge uses `--no-ff` to always create merge commit
- On merge success: transition to `completed`, cleanup worktree and branch
- On merge conflict: transition to `waiting-manual-review`, preserve worktree
- Dependency auto-queue triggered after successful merge
- Agent crash: destroy session, re-queue task, increment attempts, clear lease, cleanup worktree
- Timeout: destroy session, re-queue task, increment attempts, record timeout
- Permission denied: log denial, agent decides whether to proceed
- Actor labels: `"overlord-dispatch"`, `"builder-completion"`, `"overlord-crash-recovery"`, `"overlord-timeout"`, `"overlord-merge-conflict"`
- `BuilderError` wraps `GitError`, `ACPError`, `PersistenceError`, `OverlordError`
- All public types re-exported from `mod.rs`
- Orchestrator initialized in `src/main.rs`
- Orchestrator accessible via REST API application state
- Builder events emitted at all lifecycle phases: `TaskStarted`, `TaskProgress`, `TaskCompleted`, `TaskFailed`, `TaskMerged`, `TaskCleanedUp`, `TaskRequeued`

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum builder` — Run builder module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds
- `find src/builder/ -type f` — Verify all module files exist
- `grep -r "execute_task" src/builder/` — Verify main execution flow exists
- `grep -r "overlord-dispatch" src/builder/` — Verify dispatch actor label
- `grep -r "builder-completion" src/builder/` — Verify completion actor label
- `grep -r "overlord-crash-recovery" src/builder/` — Verify crash recovery actor label
- `grep -r "overlord-timeout" src/builder/` — Verify timeout actor label
- `grep -r "overlord-merge-conflict" src/builder/` — Verify merge conflict actor label
- `grep -r "BuilderEvent" src/builder/` — Verify builder event types
- `grep -r "heartbeat_at" src/builder/` — Verify heartbeat updates

## Notes
- This plan depends on chunks 002 (Configuration), 003 (Persistence), 004 (Overlord), 005 (Git), and 006 (ACP Client) being completed first. The builder module integrates all of these into a cohesive workflow.
- The builder workflow is the core execution engine of nexum. It's where all previous modules converge into a working system.
- The `BuilderWorkflowOrchestrator` is designed to be the single entry point for task execution. The REST API (chunk 007) will invoke it to start tasks, and the Overlord agent layer (future) will invoke it for dispatch decisions.
- The event bus integration is critical for the WebSocket frontend relay (future chunk 013). The `BuilderEventBus` provides the bridge between ACP events and the frontend.
- The heartbeat mechanism is designed to integrate with the Overlord's stale heartbeat detection (chunk 004). The builder updates `heartbeat_at` in `status.json`, and the overlord's `HeartbeatMonitor` detects stale heartbeats and re-queues orphaned tasks.
- Merge conflicts are handled by transitioning to `waiting-manual-review` and preserving the worktree for human inspection. This allows a human to resolve the conflict manually before re-attempting the merge.
- The timeout mechanism uses the task-specific timeout from the configuration. If not specified, a global default timeout applies.
- Agent crash detection works in two ways: (1) ACP progress events trigger heartbeat updates — if events stop, the overlord detects stale heartbeat; (2) subprocess exit without ACP destroy triggers immediate crash recovery.
- The worktree isolation model ensures complete independence between parallel tasks. Each task gets its own subprocess, worktree, and ACP session. No coordination is needed between instances.
- The `--no-ff` merge flag ensures every task merge creates a merge commit, providing a clear audit trail of which task introduced which changes.
- The builder module should handle the case where a task is cancelled externally (e.g., via REST API). The orchestrator should support graceful cancellation via a cancellation token.
- Consider adding a `BuilderConfig` struct for builder-specific configuration (timeout defaults, heartbeat interval, event bus capacity) that can be set in `~/.config/nexum/config.toml`.
