# Plan: 004 - Overlord Deterministic Core

## Task Description
Implement the Overlord Deterministic Core — the rules-based, non-LLM engine that orchestrates plan and task lifecycle management in nexum. This module provides deterministic status machine transitions, sequential ID generation, concurrency limit enforcement, dependency-driven auto-queueing, stale heartbeat detection for orphaned task recovery, and a periodic scheduler loop that ties all behaviors together. It operates entirely on file-based state (reading and writing `status.json`, `execution.json`, and markdown files via the persistence layer) without any AI model involvement.

## Objective
Create a fully functional `src/overlord/` module that:
- Implements the plan status machine with all valid transitions per `design/work-statuses.md`
- Implements the task status machine with all valid transitions per `design/work-statuses.md`
- Generates stable sequential IDs: `PLAN-<NNN>` per branch, `TASK-<NNN>` per plan
- Enforces concurrency limits by checking `execution.json` before dispatching tasks
- Auto-queues tasks when ALL dependencies are completed
- Detects stale heartbeats (>30 min default) and re-queues orphaned tasks
- Provides a scheduler loop that periodically runs all deterministic checks
- Integrates with the persistence layer for all file I/O
- Provides a clean public API for the Overlord agent layer (future) and REST API to invoke

## Problem Statement
Nexum's Overlord role is hybrid: a deterministic core (rules-based) plus an agent layer (LLM-powered, future). The deterministic core is the foundation — it handles all mechanical orchestration that must be predictable, repeatable, and auditable. Without it, there is no status machine enforcement (agents could self-transition arbitrarily), no concurrency control (unbounded parallel agents), no dependency management (tasks could run before their prerequisites finish), and no crash recovery (dead agents leave orphaned tasks in `running` state forever). The deterministic core must work exclusively with file-based state from the persistence layer, using atomic writes for safety during concurrent access.

## Solution Approach
Build an `overlord` module in `src/overlord/` with the following submodules:

1. **`status_machine.rs`** — Typed status machines for plans and tasks. Defines `PlanStateMachine` and `TaskStateMachine` structs that validate transitions against the allowed transition maps from `design/work-statuses.md`. Provides `can_transition(from, to) -> bool` and `transition(current, new_status, by) -> Result<StatusTransition>` methods.

2. **`id_generator.rs`** — Sequential ID generation. `PlanIdGenerator` tracks the next plan ID per branch by scanning existing plan directories. `TaskIdGenerator` tracks the next task ID per plan by scanning existing task directories. IDs are stable — once assigned, they never change.

3. **`concurrency_checker.rs`** — Concurrency enforcement. Reads `execution.json` and the associated task `status.json` files to compute `currently_running` count. Compares against `max_parallel` from the configuration. Provides `can_dispatch(plan_id) -> bool` and `count_running(plan_id) -> u16` methods.

4. **`dependency_resolver.rs`** — Dependency auto-queue logic. For each task in `backlog` status, checks whether ALL dependencies (listed in `task.md`'s `Dependencies` field and `status.json`'s `dependencies` array) are in `completed` status. If so, transitions the task from `backlog` to `queued`. Provides `auto_queue_eligible_tasks(plan_id) -> Vec<String>` method.

5. **`heartbeat_monitor.rs`** — Stale heartbeat detection. Scans all tasks in `running` status, checks `heartbeat_at` in `status.json` against the current time. If elapsed time exceeds the configured threshold (default 30 minutes), transitions the task back to `queued` and increments `attempts`. Provides `detect_stale_heartbeats(branch) -> Vec<String>` method.

6. **`scheduler.rs`** — The main scheduler loop. A periodic async loop that runs all deterministic checks: heartbeat monitoring, dependency auto-queue, and concurrency-based dispatch. Configurable interval (default 30 seconds). Provides `start()` and `stop()` lifecycle methods.

7. **`errors.rs`** — Custom error type `OverlordError` using `thiserror`.

8. **`tests.rs`** — Comprehensive unit tests for all modules.

The module depends on:
- `persistence` module (chunk 003) for file I/O, schema types, and directory helpers
- `config` module (chunk 002) for concurrency limits and heartbeat thresholds

## Relevant Files

### Existing Files
- `design/agent-roles.md` — Overlord role definition with deterministic core responsibilities
- `design/work-statuses.md` — Complete status machine definitions for plans and tasks
- `design/resource-constraints.md` — Concurrency enforcement rules and concurrency-sensitive statuses
- `design/unit-of-work.md` — Dependency chaining and auto-queue rules
- `design/persistence.md` — ID scheme, branch strategy, recovery/heartbeat mechanism, status.json schema
- `design/implementation-chunks.md` — Defines this as chunk 4 of the MVP
- `src/persistence/` — Persistence layer module (completed by chunk 003)
- `src/config/` — Configuration module (completed by chunk 002)

### New Files (if needed)
- `src/overlord/mod.rs` — Module root, re-exports public types and functions
- `src/overlord/status_machine.rs` — Plan and task status machines with transition validation
- `src/overlord/id_generator.rs` — Sequential ID generation for plans and tasks
- `src/overlord/concurrency_checker.rs` — Concurrency limit enforcement
- `src/overlord/dependency_resolver.rs` — Dependency-based auto-queue logic
- `src/overlord/heartbeat_monitor.rs` — Stale heartbeat detection and orphan recovery
- `src/overlord/scheduler.rs` — Periodic scheduler loop
- `src/overlord/errors.rs` — OverlordError enum with thiserror
- `src/overlord/tests.rs` — Unit tests for all modules

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: overlord-core-builder
  - Role: Implement status machines, ID generator, concurrency checker, dependency resolver, heartbeat monitor
  - Agent: builder

- **Builder**
  - Name: overlord-scheduler-builder
  - Role: Implement the scheduler loop and wire up all submodules
  - Agent: builder

- **Builder**
  - Name: overlord-tests-builder
  - Role: Write comprehensive unit tests for all overlord modules
  - Agent: builder

- **Validator**
  - Name: overlord-validator
  - Role: Verify status machine correctness, ID generation, concurrency enforcement, dependency logic, and heartbeat detection
  - Agent: validator

- **Documenter**
  - Name: overlord-documenter
  - Role: Generate documentation for completed overlord deterministic core
  - Agent: documenter

## Step by Step Tasks

### 1. Define Overlord Error Types
- **Task ID**: overlord-errors
- **Depends On**: none
- **Assigned To**: overlord-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/errors.rs` with `OverlordError` enum using `thiserror`:
    - `InvalidTransition { from: String, to: String, entity: String }` — Attempted invalid status transition
    - `ConcurrencyLimitExceeded { current: u16, max: u16 }` — Cannot dispatch, at concurrency limit
    - `IdGenerationError(String)` — Failed to generate a stable ID
    - `DependencyError(String)` — Dependency resolution failure
    - `HeartbeatError(String)` — Heartbeat detection failure
    - `PersistenceError(nexum::persistence::PersistenceError)` — Wrapped persistence error
    - `ConfigError(nexum::config::ConfigError)` — Wrapped config error
    - `SchedulerError(String)` — Scheduler lifecycle error
  - Implement `From<nexum::persistence::PersistenceError>` and `From<nexum::config::ConfigError>` for ergonomic `?` operator usage
  - Define type alias: `pub type Result<T> = std::result::Result<T, OverlordError>`
- **Acceptance Criteria**:
  - `OverlordError` compiles with `thiserror::Error` derive
  - All error variants include sufficient context for debugging
  - `From` implementations allow `?` operator with persistence and config errors
  - `Result<T>` type alias is defined and accessible

### 2. Implement Plan Status Machine
- **Task ID**: plan-status-machine
- **Depends On**: overlord-errors
- **Assigned To**: overlord-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/status_machine.rs` with:
    
    **Plan status machine**:
    - Define `PlanStateMachine` struct with a `transitions: HashMap<PlanStatus, Vec<PlanStatus>>` field
    - Valid transitions per `design/work-statuses.md`:
      - `draft` → `queued`
      - `queued` → `planning`
      - `planning` → `reviewing`
      - `reviewing` → `approved` OR `rejected`
      - `approved` → `complete` OR `rejected`
      - `complete` — terminal, no outgoing transitions
      - `rejected` — terminal, no outgoing transitions
    - `impl PlanStateMachine`:
      - `fn new() -> Self` — Initialize with the transition map above
      - `fn can_transition(&self, from: PlanStatus, to: PlanStatus) -> bool` — Check if transition is valid
      - `fn transition(from: PlanStatus, to: PlanStatus, by: &str) -> Result<StatusTransition>` — Create a transition record:
        - Validate the transition is allowed
        - Return `OverlordError::InvalidTransition` if not
        - Return `StatusTransition { from, to, at: chrono::Utc::now().to_rfc3339(), by: by.to_string() }`
      - `fn is_terminal(status: PlanStatus) -> bool` — Returns true for `complete` and `rejected`
      - `fn all_transitions(&self) -> &HashMap<PlanStatus, Vec<PlanStatus>>` — Expose transition map for testing
    
    **Task status machine**:
    - Define `TaskStateMachine` struct with a `transitions: HashMap<TaskStatusValue, Vec<TaskStatusValue>>` field
    - Valid transitions per `design/work-statuses.md`:
      - `backlog` → `queued`
      - `queued` → `running`
      - `running` → `reviewing`
      - `reviewing` → `waiting-manual-review` OR `merge-queue`
      - `waiting-manual-review` → `merge-queue` OR `abandoned`
      - `merge-queue` → `completed`
      - `abandoned` — terminal, no outgoing transitions
      - `completed` — terminal, no outgoing transitions
    - `impl TaskStateMachine`:
      - `fn new() -> Self` — Initialize with the transition map above
      - `fn can_transition(&self, from: TaskStatusValue, to: TaskStatusValue) -> bool` — Check if transition is valid
      - `fn transition(from: TaskStatusValue, to: TaskStatusValue, by: &str) -> Result<StatusTransition>` — Create a transition record:
        - Validate the transition is allowed
        - Return `OverlordError::InvalidTransition` if not
        - Return `StatusTransition { from, to, at: chrono::Utc::now().to_rfc3339(), by: by.to_string() }`
      - `fn is_terminal(status: TaskStatusValue) -> bool` — Returns true for `completed` and `abandoned`
      - `fn all_transitions(&self) -> &HashMap<TaskStatusValue, Vec<TaskStatusValue>>` — Expose transition map for testing
    
    **Concurrency-sensitive status check**:
    - `fn is_concurrency_sensitive(status: TaskStatusValue) -> bool` — Returns true for `running` and `reviewing` (task-level)
    - `fn is_plan_concurrency_sensitive(status: PlanStatus) -> bool` — Returns true for `planning` and `reviewing` (plan-level)
  - Import types from `persistence::schema`: `PlanStatus`, `TaskStatusValue`, `StatusTransition`
  - Use `chrono` for timestamp generation
  - Document each transition with rustdoc comments referencing `design/work-statuses.md`
- **Acceptance Criteria**:
  - `PlanStateMachine` has exactly the transitions defined in work-statuses.md
  - `TaskStateMachine` has exactly the transitions defined in work-statuses.md
  - `can_transition()` returns true only for valid transitions
  - `transition()` returns `OverlordError::InvalidTransition` for invalid transitions
  - `transition()` creates a valid `StatusTransition` with timestamp and actor
  - `is_terminal()` correctly identifies terminal states
  - `is_concurrency_sensitive()` correctly identifies concurrency-gated statuses
  - Both state machines compile and all methods are accessible

### 3. Implement ID Generator
- **Task ID**: id-generator
- **Depends On**: overlord-errors
- **Assigned To**: overlord-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/id_generator.rs` with:
    
    **Plan ID Generator**:
    - `pub struct PlanIdGenerator` — Tracks next plan ID per branch
    - `fn new() -> Self` — Initialize empty state
    - `fn next_plan_id(repo_root: &Path, branch: &str) -> Result<String>` — Generate next plan ID:
      1. Scan `.agent/specs/<branch>/` for existing plan directories matching `PLAN-<NNN>-*` pattern
      2. Extract all NNN values, find the maximum
      3. Next ID = max + 1, zero-padded to 3 digits
      4. Return `PLAN-<NNN>` (e.g., `PLAN-001`)
      5. If no existing plans, return `PLAN-001`
    - `fn parse_plan_id(id: &str) -> Result<u32>` — Parse `PLAN-<NNN>` to extract the numeric portion
    
    **Task ID Generator**:
    - `pub struct TaskIdGenerator` — Tracks next task ID per plan
    - `fn new() -> Self` — Initialize empty state
    - `fn next_task_id(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<String>` — Generate next task ID:
      1. Scan `.agent/specs/<branch>/<plan_id>-<plan_name>/tasks/` for existing task directories matching `TASK-<NNN>-*` pattern
      2. Extract all NNN values, find the maximum
      3. Next ID = max + 1, zero-padded to 3 digits
      4. Return `TASK-<NNN>` (e.g., `TASK-001`)
      5. If no existing tasks, return `TASK-001`
    - `fn parse_task_id(id: &str) -> Result<u32>` — Parse `TASK-<NNN>` to extract the numeric portion
    
    **Slug generation**:
    - `pub fn slugify(text: &str) -> String` — Convert text to kebab-case slug (reuse or reference persistence layer's slugify)
    - `pub fn format_dir_name(id: &str, slug: &str) -> String` — Returns `<id>-<slug>` (e.g., `PLAN-001-oauth2-flow`)
  - Use regex `^PLAN-(\d{3})$` and `^TASK-(\d{3})$` for parsing
  - Return `OverlordError::IdGenerationError` on parse failures
  - Handle edge cases: no existing plans/tasks, malformed directory names, special characters in slugs
- **Acceptance Criteria**:
  - `next_plan_id()` returns `PLAN-001` for a branch with no existing plans
  - `next_plan_id()` returns `PLAN-003` for a branch with `PLAN-001-*` and `PLAN-002-*` directories
  - `next_task_id()` returns `TASK-001` for a plan with no existing tasks
  - `next_task_id()` returns `TASK-004` for a plan with 3 existing tasks
  - `parse_plan_id("PLAN-005")` returns `5`
  - `parse_task_id("TASK-010")` returns `10`
  - `slugify("OAuth 2.0 Flow")` produces a valid kebab-case slug
  - `format_dir_name("PLAN-001", "oauth-flow")` returns `PLAN-001-oauth-flow`
  - ID generation handles missing directories gracefully

### 4. Implement Concurrency Checker
- **Task ID**: concurrency-checker
- **Depends On**: overlord-errors
- **Assigned To**: overlord-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/concurrency_checker.rs` with:
    
    **Core concurrency logic**:
    - `pub struct ConcurrencyChecker` — Main concurrency enforcement struct
    - `fn new() -> Self` — Initialize
    - `fn count_running(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<u16>` — Count currently running tasks:
      1. Read `execution.json` from `.agent/state/<branch>/<plan_id>-<plan_name>/`
      2. Iterate `task_status_map` entries
      3. Count tasks with status `running`
      4. Also check individual `status.json` files for accuracy (in case execution.json is stale)
      5. Return the count
    - `fn get_max_parallel(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<u16>` — Get concurrency limit:
      1. First check `execution.json` for a `concurrency.max_parallel` field (plan-level override)
      2. Fall back to `config::get_max_parallel()` from global config
      3. Return the limit
    - `fn can_dispatch(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<bool>` — Check if a new task can be dispatched:
      1. `currently_running = count_running(...)`
      2. `max_parallel = get_max_parallel(...)`
      3. Return `currently_running < max_parallel`
    - `fn dispatch_info(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<DispatchInfo>` — Return structured dispatch information:
      - `pub struct DispatchInfo { currently_running: u16, max_parallel: u16, can_dispatch: bool, remaining_slots: u16 }`
    
    **Concurrency-sensitive status enforcement**:
    - `fn is_concurrency_gated(status: TaskStatusValue) -> bool` — Returns true for `running` and `reviewing`
    - `fn validate_transition_concurrency(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, to_status: TaskStatusValue) -> Result<()>` — Validate a transition doesn't violate concurrency:
      - If `to_status` is `running` or `reviewing`, check `can_dispatch()`
      - Return `OverlordError::ConcurrencyLimitExceeded` if limit reached
  - Use persistence layer for all file reads: `read_execution_state()`, `read_task_status()`
  - Map `PersistenceError` to `OverlordError` via `From` implementation
- **Acceptance Criteria**:
  - `count_running()` correctly counts tasks with `running` status
  - `get_max_parallel()` uses plan-level override if present, falls back to config
  - `can_dispatch()` returns true when `currently_running < max_parallel`
  - `can_dispatch()` returns false when `currently_running >= max_parallel`
  - `dispatch_info()` returns correct structured information
  - `validate_transition_concurrency()` blocks `running` transition when at limit
  - `validate_transition_concurrency()` allows non-concurrency-sensitive transitions (e.g., `backlog` → `queued`)
  - Handles missing `execution.json` gracefully (returns 0 running)

### 5. Implement Dependency Resolver
- **Task ID**: dependency-resolver
- **Depends On**: overlord-errors, plan-status-machine
- **Assigned To**: overlord-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/dependency_resolver.rs` with:
    
    **Core dependency logic**:
    - `pub struct DependencyResolver` — Main dependency resolution struct
    - `fn new() -> Self` — Initialize with a `TaskStateMachine` instance
    - `fn are_all_dependencies_met(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> Result<bool>` — Check if all dependencies are completed:
      1. Read `status.json` for the task
      2. Get `dependencies` list (task IDs this task depends on)
      3. If empty, return `true` (no dependencies)
      4. For each dependency task ID, read its `status.json`
      5. If ALL dependencies have status `completed`, return `true`
      6. If ANY dependency is not `completed`, return `false`
    - `fn auto_queue_eligible_tasks(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<Vec<String>>` — Find tasks eligible for auto-queue:
      1. List all task directories under the plan
      2. For each task, read `status.json`
      3. If status is `backlog` AND `are_all_dependencies_met()` returns true, add to eligible list
      4. Return list of eligible task IDs
    - `fn auto_queue_tasks(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<Vec<String>>` — Perform auto-queue transitions:
      1. Get eligible tasks via `auto_queue_eligible_tasks()`
      2. For each eligible task:
        a. Validate transition `backlog` → `queued` via `TaskStateMachine`
        b. Use persistence layer's `update_task_status()` to perform the transition
        c. Actor: `"overlord-auto-queue"`
      3. Return list of queued task IDs
    - `fn resolve_dependent_tasks(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, completed_task_id: &str) -> Result<Vec<String>>` — When a task completes, find tasks that depended on it and check if they're now eligible:
      1. Find all tasks that list `completed_task_id` in their `dependencies`
      2. For each, check if ALL their dependencies are now completed
      3. Return list of newly eligible task IDs
    
    **Dependency graph utilities**:
    - `fn build_dependency_graph(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<HashMap<String, Vec<String>>>` — Build the full dependency graph for a plan
    - `fn detect_cycles(graph: &HashMap<String, Vec<String>>) -> bool` — Detect circular dependencies (should never happen, but validate)
  - Use persistence layer for all file reads and writes
  - Document the auto-queue behavior referencing `design/unit-of-work.md`
- **Acceptance Criteria**:
  - `are_all_dependencies_met()` returns true for tasks with no dependencies
  - `are_all_dependencies_met()` returns true when all dependencies are `completed`
  - `are_all_dependencies_met()` returns false when any dependency is not `completed`
  - `auto_queue_eligible_tasks()` returns correct list of backlog tasks with met dependencies
  - `auto_queue_tasks()` transitions eligible tasks from `backlog` to `queued`
  - `auto_queue_tasks()` records transitions with actor `"overlord-auto-queue"`
  - `resolve_dependent_tasks()` correctly identifies tasks unblocked by a completion
  - `build_dependency_graph()` returns correct adjacency list
  - `detect_cycles()` correctly identifies circular dependencies

### 6. Implement Heartbeat Monitor
- **Task ID**: heartbeat-monitor
- **Depends On**: overlord-errors, plan-status-machine
- **Assigned To**: overlord-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/heartbeat_monitor.rs` with:
    
    **Core heartbeat logic**:
    - `pub struct HeartbeatMonitor` — Stale heartbeat detection struct
    - `fn new(stale_threshold_minutes: u64) -> Self` — Initialize with configurable stale threshold (default: 30 minutes)
    - `fn get_stale_threshold(&self) -> Duration` — Returns the stale threshold as a `Duration`
    - `fn is_heartbeat_stale(heartbeat_at: &str) -> Result<bool>` — Check if a heartbeat timestamp is stale:
      1. Parse `heartbeat_at` as RFC 3339 timestamp
      2. Compute elapsed time since heartbeat
      3. Return `elapsed > stale_threshold`
    - `fn detect_stale_tasks(repo_root: &Path, branch: &str) -> Result<Vec<StaleTask>>` — Scan all plans and tasks for stale heartbeats:
      1. List all branches under `.agent/state/`
      2. For each branch, list all plan directories
      3. For each plan, list all task directories
      4. For each task, read `status.json`
      5. If status is `running` AND `heartbeat_at` is stale, record as stale
      6. Return list of `StaleTask` structs
    - `pub struct StaleTask { task_id: String, plan_id: String, branch: String, plan_name: String, task_name: String, last_heartbeat: String, elapsed_minutes: f64 }`
    - `fn recover_stale_tasks(repo_root: &Path, branch: &str) -> Result<Vec<String>>` — Re-queue stale tasks:
      1. Detect stale tasks via `detect_stale_tasks()`
      2. For each stale task:
        a. Validate transition `running` → `queued` via `TaskStateMachine`
        b. Read current `status.json`
        c. Increment `attempts` counter
        d. Clear `agent` lease (set to `None`)
        e. Clear `started_at` (set to `None`)
        f. Use persistence layer's `update_task_status()` to perform the transition
        g. Actor: `"overlord-heartbeat-recovery"`
      3. Return list of recovered task IDs
    
    **Branch-level scan**:
    - `fn detect_stale_in_plan(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<Vec<StaleTask>>` — Scan a single plan for stale tasks
    - `fn recover_stale_in_plan(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<Vec<String>>` — Recover stale tasks in a single plan
  - Use `chrono` for timestamp parsing and elapsed time calculation
  - Default stale threshold: 30 minutes (configurable via constructor)
  - Document the recovery process referencing `design/persistence.md` recovery section
- **Acceptance Criteria**:
  - `is_heartbeat_stale()` returns true when elapsed time exceeds threshold
  - `is_heartbeat_stale()` returns false when elapsed time is within threshold
  - `detect_stale_tasks()` finds all tasks with stale heartbeats in `running` status
  - `detect_stale_tasks()` ignores tasks not in `running` status
  - `recover_stale_tasks()` transitions stale tasks from `running` to `queued`
  - `recover_stale_tasks()` increments `attempts` counter
  - `recover_stale_tasks()` clears `agent` lease and `started_at`
  - `recover_stale_tasks()` records transitions with actor `"overlord-heartbeat-recovery"`
  - Handles missing `heartbeat_at` field (treats as stale if in `running` status)
  - Handles malformed timestamps gracefully

### 7. Implement Overlord Scheduler
- **Task ID**: overlord-scheduler
- **Depends On**: plan-status-machine, id-generator, concurrency-checker, dependency-resolver, heartbeat-monitor
- **Assigned To**: overlord-scheduler-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/scheduler.rs` with:
    
    **Scheduler struct**:
    - `pub struct OverlordScheduler` — Main scheduler with all sub-components:
      - `status_machine: TaskStateMachine`
      - `plan_status_machine: PlanStateMachine`
      - `id_generator_plan: PlanIdGenerator`
      - `id_generator_task: TaskIdGenerator`
      - `concurrency_checker: ConcurrencyChecker`
      - `dependency_resolver: DependencyResolver`
      - `heartbeat_monitor: HeartbeatMonitor`
      - `repo_root: PathBuf`
      - `interval: Duration` (default: 30 seconds)
      - `running: AtomicBool`
    - `fn new(repo_root: PathBuf) -> Self` — Initialize all sub-components with defaults
    - `fn with_interval(mut self, interval: Duration) -> Self` — Builder pattern for interval
    - `fn with_heartbeat_threshold(mut self, minutes: u64) -> Self` — Builder pattern for heartbeat threshold
    
    **Lifecycle methods**:
    - `pub async fn start(&self) -> Result<()>` — Start the scheduler loop:
      1. Set `running` to true
      2. Enter async loop:
        a. `self.tick().await` — Run one iteration of all checks
        b. `tokio::time::sleep(self.interval).await`
      3. Loop until `running` is false
    - `pub fn stop(&self)` — Set `running` to false
    - `pub fn is_running(&self) -> bool` — Check scheduler state
    
    **Tick (one iteration)**:
    - `async fn tick(&self) -> Result<()>` — Run all deterministic checks:
      1. `self.heartbeat_monitor.detect_and_recover_all(self.repo_root).await` — Recover stale heartbeats
      2. `self.dependency_resolver.auto_queue_all(self.repo_root).await` — Auto-queue tasks with met dependencies
      3. `self.try_dispatch_queued_tasks().await` — Dispatch queued tasks respecting concurrency
    - `async fn try_dispatch_queued_tasks(&self) -> Result<()>` — Dispatch queued tasks:
      1. Find all tasks in `queued` status across all branches/plans
      2. For each, check `concurrency_checker.can_dispatch()`
      3. If dispatchable, transition `queued` → `running`
      4. Stop when concurrency limit is reached
      5. Actor: `"overlord-dispatch"`
    
    **Public API methods** (for REST API and agent layer to invoke):
    - `pub async fn transition_task_status(&self, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str, new_status: TaskStatusValue, by: &str) -> Result<()>` — Transition a task status:
      1. Read current status from `status.json`
      2. Validate transition via `TaskStateMachine`
      3. Check concurrency if target is `running` or `reviewing`
      4. Use persistence layer's `update_task_status()` to perform transition
    - `pub async fn transition_plan_status(&self, branch: &str, plan_id: &str, plan_name: &str, new_status: PlanStatus, by: &str) -> Result<()>` — Transition a plan status:
      1. Read current status from `plan.md` metadata
      2. Validate transition via `PlanStateMachine`
      3. Check concurrency if target is `planning` or `reviewing`
      4. Update plan.md metadata
      5. If transitioning to `approved`, move all tasks to `backlog`
    - `pub async fn generate_plan_id(&self, branch: &str) -> Result<String>` — Generate next plan ID
    - `pub async fn generate_task_id(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<String>` — Generate next task ID
    - `pub async fn get_dispatch_info(&self, branch: &str, plan_id: &str, plan_name: &str) -> Result<DispatchInfo>` — Get concurrency status
    - `pub async fn get_stale_tasks(&self, branch: &str) -> Result<Vec<StaleTask>>` — Get stale task list
  - Use `tokio::time::sleep` for interval-based scheduling
  - Use `std::sync::atomic::AtomicBool` for running state
  - All public methods return `overlord::Result<T>`
- **Acceptance Criteria**:
  - `start()` enters an async loop that runs `tick()` at the configured interval
  - `stop()` cleanly terminates the scheduler loop
  - `tick()` runs heartbeat recovery, auto-queue, and dispatch in order
  - `try_dispatch_queued_tasks()` respects concurrency limits
  - `transition_task_status()` validates transitions and checks concurrency
  - `transition_plan_status()` validates transitions and handles `approved` → task backlog
  - `generate_plan_id()` and `generate_task_id()` delegate to ID generator
  - `get_dispatch_info()` returns correct concurrency status
  - Scheduler handles errors gracefully (logs and continues, doesn't crash)
  - All public methods are async and return `Result<T>`

### 8. Wire Up Module Exports
- **Task ID**: overlord-module-wiring
- **Depends On**: overlord-scheduler
- **Assigned To**: overlord-scheduler-builder
- **Agent**: builder
- **Actions**:
  - Update `src/overlord/mod.rs` to:
    - Declare submodules: `mod errors; mod status_machine; mod id_generator; mod concurrency_checker; mod dependency_resolver; mod heartbeat_monitor; mod scheduler;`
    - Re-export public types:
      - `pub use errors::*;` (OverlordError, Result)
      - `pub use status_machine::*;` (PlanStateMachine, TaskStateMachine, StatusTransition)
      - `pub use id_generator::*;` (PlanIdGenerator, TaskIdGenerator)
      - `pub use concurrency_checker::*;` (ConcurrencyChecker, DispatchInfo)
      - `pub use dependency_resolver::*;` (DependencyResolver)
      - `pub use heartbeat_monitor::*;` (HeartbeatMonitor, StaleTask)
      - `pub use scheduler::*;` (OverlordScheduler)
    - Include module-level documentation referencing `design/agent-roles.md` deterministic core section
  - Add `mod overlord;` to `src/main.rs` (if not already present)
  - Update `src/main.rs` to initialize `OverlordScheduler` on startup:
    - Create scheduler with repo root path
    - Spawn as background task: `tokio::spawn(scheduler.start())`
    - Log scheduler startup via `tracing`
- **Acceptance Criteria**:
  - All public types are accessible as `nexum::overlord::OverlordScheduler`, etc.
  - All public functions are accessible via the module re-exports
  - Module compiles without errors
  - Module documentation references design docs
  - `src/main.rs` initializes and spawns the scheduler on startup
  - Scheduler startup is logged via tracing

### 9. Write Unit Tests
- **Task ID**: overlord-tests
- **Depends On**: overlord-module-wiring
- **Assigned To**: overlord-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `src/overlord/tests.rs` with comprehensive tests:
    
    **Status machine tests**:
    - `test_plan_valid_transitions` — Verify all valid plan transitions return true
    - `test_plan_invalid_transitions` — Verify invalid plan transitions return false
    - `test_task_valid_transitions` — Verify all valid task transitions return true
    - `test_task_invalid_transitions` — Verify invalid task transitions return false
    - `test_plan_terminal_states` — Verify `complete` and `rejected` are terminal
    - `test_task_terminal_states` — Verify `completed` and `abandoned` are terminal
    - `test_transition_record_creation` — Verify transition records have correct fields
    - `test_concurrency_sensitive_statuses` — Verify correct statuses are flagged
    
    **ID generator tests**:
    - `test_next_plan_id_empty` — First plan ID is `PLAN-001`
    - `test_next_plan_id_incremental` — Subsequent IDs increment correctly
    - `test_next_task_id_empty` — First task ID is `TASK-001`
    - `test_next_task_id_incremental` — Subsequent IDs increment correctly
    - `test_parse_plan_id` — Parse valid plan IDs
    - `test_parse_task_id` — Parse valid task IDs
    - `test_parse_invalid_id` — Handle malformed IDs
    - `test_slugify` — Verify kebab-case slug generation
    - `test_format_dir_name` — Verify directory name format
    
    **Concurrency checker tests**:
    - `test_count_running_zero` — No running tasks returns 0
    - `test_count_running_multiple` — Multiple running tasks counted correctly
    - `test_can_dispatch_within_limit` — Dispatch allowed when under limit
    - `test_can_dispatch_at_limit` — Dispatch blocked when at limit
    - `test_dispatch_info` — Verify structured dispatch info
    - `test_validate_transition_concurrency_allowed` — Non-gated transitions allowed
    - `test_validate_transition_concurrency_blocked` — Gated transitions blocked at limit
    - `test_plan_level_override` — Plan-level max_parallel overrides global config
    
    **Dependency resolver tests**:
    - `test_no_dependencies_met` — Task with no dependencies is eligible
    - `test_all_dependencies_met` — Task with all deps completed is eligible
    - `test_some_dependencies_unmet` — Task with unmet deps is not eligible
    - `test_auto_queue_eligible` — Correct tasks identified for auto-queue
    - `test_auto_queue_transition` — Tasks transitioned from backlog to queued
    - `test_resolve_dependent_tasks` — Downstream tasks identified after completion
    - `test_dependency_graph` — Correct graph construction
    - `test_detect_cycles` — Circular dependencies detected
    
    **Heartbeat monitor tests**:
    - `test_fresh_heartbeat_not_stale` — Recent heartbeat is not stale
    - `test_old_heartbeat_is_stale` — Old heartbeat is detected as stale
    - `test_detect_stale_tasks` — Stale tasks correctly identified
    - `test_detect_stale_ignores_non_running` — Non-running tasks ignored
    - `test_recover_stale_task` — Stale task re-queued correctly
    - `test_recover_increments_attempts` — Attempts counter incremented
    - `test_recover_clears_lease` — Agent lease cleared on recovery
    - `test_missing_heartbeat_treated_as_stale` — Missing heartbeat triggers recovery
    
    **Scheduler tests**:
    - `test_scheduler_start_stop` — Lifecycle works correctly
    - `test_scheduler_tick` — One tick runs all checks
    - `test_transition_task_status_valid` — Valid transition succeeds
    - `test_transition_task_status_invalid` — Invalid transition returns error
    - `test_transition_plan_status_valid` — Valid plan transition succeeds
    - `test_transition_plan_approved_queues_tasks` — Approved plan moves tasks to backlog
    - `test_generate_ids` — ID generation via scheduler API
    - `test_dispatch_info_via_scheduler` — Dispatch info accessible via scheduler
    
    **Test utilities**:
    - `fn create_test_repo() -> tempfile::TempDir` — Create temporary repo structure
    - `fn setup_test_plan(dir: &Path, branch: &str, plan_id: &str) -> Result<()>` — Set up a test plan with tasks
    - `fn create_task_status(dir: &Path, task_id: &str, status: TaskStatusValue, heartbeat: Option<String>) -> Result<()>` — Create a task status.json
    - `fn create_execution_state(dir: &Path, plan_id: &str, tasks: Vec<(String, TaskStatusValue)>) -> Result<()>` — Create execution.json
  - Use `tempfile` crate for isolated test directories
  - Use `tokio::test` for async tests
  - Use `chrono` for controlled timestamps in heartbeat tests
- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum overlord`
  - Tests cover all modules: status machines, ID generator, concurrency, dependencies, heartbeat, scheduler
  - Tests use temporary directories to avoid polluting the real filesystem
  - Async tests use `#[tokio::test]`
  - Status machine tests cover all valid and invalid transitions
  - Concurrency tests cover boundary conditions (at limit, over limit)
  - Dependency tests cover various dependency graph configurations
  - Heartbeat tests use controlled timestamps for reliable stale detection
  - Scheduler tests verify lifecycle and public API methods

### 10. Final Validation
- **Task ID**: validate-all
- **Depends On**: overlord-tests
- **Assigned To**: overlord-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum overlord` — all overlord tests must pass
  - Run `cargo clippy --package nexum` — no warnings in overlord module
  - Verify `PlanStateMachine` transitions match `design/work-statuses.md` exactly:
    - Pre-planning: `draft` → `queued` → `planning` → `reviewing`
    - Post-planning: `reviewing` → `approved`/`rejected`, `approved` → `complete`/`rejected`
  - Verify `TaskStateMachine` transitions match `design/work-statuses.md` exactly:
    - `backlog` → `queued` → `running` → `reviewing` → `waiting-manual-review`/`merge-queue` → `completed`/`abandoned`
  - Verify `waiting-manual-review` → `merge-queue` transition exists
  - Verify `merge-queue` → `completed` transition exists
  - Verify terminal states: plan (`complete`, `rejected`), task (`completed`, `abandoned`)
  - Verify ID generation produces `PLAN-<NNN>` and `TASK-<NNN>` format
  - Verify IDs are sequential and zero-padded to 3 digits
  - Verify concurrency checker counts `running` status tasks
  - Verify concurrency checker respects plan-level `max_parallel` override
  - Verify concurrency checker falls back to global config
  - Verify dependency resolver auto-queues only when ALL dependencies are `completed`
  - Verify dependency resolver records transitions with actor `"overlord-auto-queue"`
  - Verify heartbeat monitor detects stale heartbeats > configured threshold
  - Verify heartbeat monitor default threshold is 30 minutes
  - Verify heartbeat recovery transitions `running` → `queued`
  - Verify heartbeat recovery increments `attempts` counter
  - Verify heartbeat recovery clears agent lease
  - Verify heartbeat recovery records transitions with actor `"overlord-heartbeat-recovery"`
  - Verify scheduler runs all checks in each tick
  - Verify scheduler dispatch respects concurrency limits
  - Verify scheduler public API methods work correctly
  - Verify `OverlordError` wraps `PersistenceError` and `ConfigError`
  - Verify all public types are re-exported from `mod.rs`
  - Verify scheduler is initialized and spawned in `src/main.rs`

### 11. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: overlord-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the status machines with transition diagrams
  - Document the ID generation scheme and stability guarantees
  - Document the concurrency enforcement mechanism
  - Document the dependency auto-queue logic
  - Document the heartbeat monitoring and recovery process
  - Document the scheduler lifecycle and configuration options
  - Document the public API for external consumers (REST API, agent layer)

## Acceptance Criteria
- `cargo check` succeeds with no errors in the overlord module
- `cargo test --package nexum overlord` passes all tests
- `cargo clippy --package nexum` produces no warnings in overlord module
- `PlanStateMachine` implements all transitions from `design/work-statuses.md` for plans
- `TaskStateMachine` implements all transitions from `design/work-statuses.md` for tasks
- Plan terminal states: `complete`, `rejected`
- Task terminal states: `completed`, `abandoned`
- `can_transition()` returns true only for defined transitions
- `transition()` returns `OverlordError::InvalidTransition` for invalid transitions
- `transition()` creates `StatusTransition` records with timestamp and actor
- ID generator produces `PLAN-<NNN>` (sequential per branch) and `TASK-<NNN>` (sequential per plan)
- IDs are zero-padded to 3 digits (e.g., `PLAN-001`, `TASK-042`)
- IDs are stable — scanning filesystem, not in-memory counters
- Concurrency checker counts `running` tasks from `execution.json` and individual `status.json` files
- Concurrency checker respects plan-level `max_parallel` override from `execution.json`
- Concurrency checker falls back to global config `max_parallel`
- `can_dispatch()` returns false when `currently_running >= max_parallel`
- Dependency resolver auto-queues tasks when ALL dependencies are `completed`
- Dependency resolver transitions `backlog` → `queued` with actor `"overlord-auto-queue"`
- Dependency resolver handles tasks with no dependencies (always eligible)
- Heartbeat monitor detects stale heartbeats > configurable threshold (default 30 min)
- Heartbeat recovery transitions `running` → `queued` for stale tasks
- Heartbeat recovery increments `attempts` counter
- Heartbeat recovery clears agent lease (`agent: None`)
- Heartbeat recovery clears `started_at`
- Heartbeat recovery records transitions with actor `"overlord-heartbeat-recovery"`
- Scheduler runs heartbeat recovery, auto-queue, and dispatch in each tick
- Scheduler dispatch transitions `queued` → `running` with actor `"overlord-dispatch"`
- Scheduler respects concurrency limits during dispatch
- Scheduler public API provides `transition_task_status()`, `transition_plan_status()`, `generate_plan_id()`, `generate_task_id()`, `get_dispatch_info()`, `get_stale_tasks()`
- `OverlordError` wraps `PersistenceError` and `ConfigError`
- All public types and functions are re-exported from `mod.rs`
- Scheduler is initialized and spawned as background task in `src/main.rs`
- Scheduler startup is logged via `tracing`

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum overlord` — Run overlord module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds
- `find src/overlord/ -type f` — Verify all module files exist
- `grep -r "can_transition" src/overlord/` — Verify transition validation is used
- `grep -r "atomic_write" src/overlord/` — Verify atomic writes are used for status updates (via persistence layer)
- `grep -r "overlord-auto-queue" src/overlord/` — Verify auto-queue actor label
- `grep -r "overlord-heartbeat-recovery" src/overlord/` — Verify heartbeat recovery actor label
- `grep -r "overlord-dispatch" src/overlord/` — Verify dispatch actor label

## Notes
- This plan depends on chunk 003 (Persistence Layer) being completed first. The persistence module provides all file I/O operations, schema types, and directory helpers that the overlord module uses extensively.
- This plan depends on chunk 002 (Configuration System) being completed first. The config module provides `get_max_parallel()` and other global settings.
- The `chrono` crate should be used for all timestamp operations. Ensure it's in `Cargo.toml` (likely already added by the persistence layer chunk).
- The `tokio` crate is needed for the async scheduler loop. It's already in the dependency list from chunk 001.
- The `regex` crate may be needed for ID parsing. Consider adding it to `Cargo.toml` if not already present.
- The `tempfile` crate should be added as a dev-dependency for testing.
- The `fastrand` crate may be needed if the persistence layer uses it for atomic write temp filenames.
- The scheduler's `tick()` method should handle errors gracefully — log the error and continue to the next check, rather than crashing the entire scheduler loop.
- The `transition_plan_status()` method, when transitioning to `approved`, should iterate all tasks in the plan and transition them from their current status to `backlog`. This is the mechanism described in `design/work-statuses.md`: "After human approval, the plan tracks its tasks through execution" — tasks start in `backlog`.
- The `execution.json` file should be updated whenever task statuses change to maintain the `task_status_map`. The persistence layer's `update_task_status()` should handle this, or the overlord module should do it as a post-transition step.
- Consider adding a `RepoContext` struct that holds the repo root path, to avoid passing it to every function. This would be a nice ergonomic improvement but is not required for MVP.
- The heartbeat threshold should be configurable per-plan in future iterations. For MVP, a global default of 30 minutes is sufficient.
- The `detect_cycles()` function in the dependency resolver is a safety check. In normal operation, the planner should not create circular dependencies, but the overlord should validate this to prevent infinite loops.
- The overlord module is designed to be the foundation for the future Overlord agent layer (LLM-powered). The public API methods (`transition_task_status()`, `generate_plan_id()`, etc.) are the interface the agent layer will use.
