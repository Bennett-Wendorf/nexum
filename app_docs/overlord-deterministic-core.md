# Overlord Deterministic Core

## Overview

The Overlord Deterministic Core is the rules-based, non-LLM engine that orchestrates plan and task lifecycle management in nexum. It provides deterministic status machine transitions, sequential ID generation, concurrency limit enforcement, dependency-driven auto-queueing, stale heartbeat detection for orphaned task recovery, and a periodic scheduler loop that ties all behaviors together.

The module operates entirely on file-based state — reading and writing `status.json`, `execution.json`, and markdown files via the persistence layer — without any AI model involvement. It is designed as the foundation for the future Overlord agent layer (LLM-powered).

## What Was Built

The `src/overlord/` module consists of eight submodules wired together by a central scheduler:

| Submodule | Purpose |
|---|---|
| `status_machine.rs` | Typed state machines for plan and task transitions |
| `id_generator.rs` | Sequential, filesystem-scanned ID generation |
| `concurrency_checker.rs` | Concurrency limit enforcement via `execution.json` |
| `dependency_resolver.rs` | Dependency auto-queue logic with cycle detection |
| `heartbeat_monitor.rs` | Stale heartbeat detection and orphan recovery |
| `scheduler.rs` | Periodic async scheduler loop tying all checks together |
| `errors.rs` | Custom `OverlordError` enum with `thiserror` |
| `tests.rs` | Comprehensive unit tests for all modules |

## Technical Implementation

### Files Created

| File | Description |
|---|---|
| `src/overlord/mod.rs` | Module root; re-exports all public types |
| `src/overlord/status_machine.rs` | `PlanStateMachine`, `TaskStateMachine`, concurrency-sensitive helpers |
| `src/overlord/id_generator.rs` | `PlanIdGenerator`, `TaskIdGenerator`, `slugify()`, `format_dir_name()` |
| `src/overlord/concurrency_checker.rs` | `ConcurrencyChecker`, `DispatchInfo`, `validate_transition_concurrency()` |
| `src/overlord/dependency_resolver.rs` | `DependencyResolver`, auto-queue, dependency graph, cycle detection |
| `src/overlord/heartbeat_monitor.rs` | `HeartbeatMonitor`, `StaleTask`, stale detection and recovery |
| `src/overlord/scheduler.rs` | `OverlordScheduler` — the orchestrating public API |
| `src/overlord/errors.rs` | `OverlordError` enum, `Result<T>` type alias |
| `src/overlord/tests.rs` | 30+ unit and async tests covering all modules |

### Dependencies

- `persistence` module (chunk 003) — all file I/O, schema types, directory helpers
- `config` module (chunk 002) — `get_max_parallel()` concurrency limit
- `thiserror` — error derive macro
- `chrono` — RFC 3339 timestamp handling
- `regex` — ID pattern matching (via `LazyLock`)
- `tokio` — async scheduler loop
- `tempfile` (dev-dependency) — isolated test directories

---

## Status Machines

### Plan Status Machine

The `PlanStateMachine` validates transitions for the plan lifecycle. Plans flow from rough idea through human approval and task execution.

```
draft ──→ queued ──→ planning ──→ reviewing ──┬──→ approved ──→ complete (terminal)
                                                │              └──→ rejected (terminal)
                                                └──→ rejected (terminal)
```

**Valid transitions:**

| From | To |
|---|---|
| `draft` | `queued` |
| `queued` | `planning` |
| `planning` | `reviewing` |
| `reviewing` | `approved` or `rejected` |
| `approved` | `complete` or `rejected` |

**Terminal states:** `complete`, `rejected`

**Concurrency-sensitive states:** `planning`, `reviewing`

### Task Status Machine

The `TaskStateMachine` validates transitions for individual tasks within an approved plan.

```
backlog ──→ queued ──→ running ──→ reviewing ──┬──→ waiting-manual-review ──┬──→ merge-queue ──→ completed (terminal)
                                                 │                              └──→ abandoned (terminal)
                                                 └──→ merge-queue ────────────────────────────────↑
```

**Valid transitions:**

| From | To |
|---|---|
| `backlog` | `queued` |
| `queued` | `running` |
| `running` | `reviewing` |
| `reviewing` | `waiting-manual-review` or `merge-queue` |
| `waiting-manual-review` | `merge-queue` or `abandoned` |
| `merge-queue` | `completed` |

**Terminal states:** `completed`, `abandoned`

**Concurrency-sensitive states:** `running`, `reviewing`

### API

```rust
PlanStateMachine::new()
    .can_transition(&from, &to)          // → bool
    .transition(&from, &to, by: &str)    // → Result<StatusTransitionRecord>

TaskStateMachine::new()
    .can_transition(&from, &to)          // → bool
    .transition(&from, &to, by: &str)    // → Result<StatusTransitionRecord>
```

A successful `transition()` call returns a `StatusTransitionRecord` containing the source status, target status, RFC 3339 timestamp, and actor name. Invalid transitions return `OverlordError::InvalidTransition`.

---

## ID Generation

### Scheme

- **Plan IDs:** `PLAN-<NNN>` — sequential per branch, zero-padded to 3 digits (e.g., `PLAN-001`, `PLAN-042`)
- **Task IDs:** `TASK-<NNN>` — sequential per plan, zero-padded to 3 digits (e.g., `TASK-001`, `TASK-010`)

### Stability Guarantees

IDs are **stable** — they are generated by scanning the filesystem, not by in-memory counters. This means:

1. **Restart-safe:** If the process restarts, it scans existing directories and produces the next sequential ID. No IDs are duplicated or skipped due to process restarts.
2. **Deterministic:** Given the same set of existing directories, the same next ID is always produced.
3. **Gap-tolerant:** If a plan or task directory is deleted, the generator still produces the next-in-sequence number (max + 1), not the gap-filling number. This avoids ID collision risks.

### Implementation

- `PlanIdGenerator::next_plan_id(repo_root, branch)` — scans `.agent/specs/<branch>/` for `PLAN-<NNN>-*` directories
- `TaskIdGenerator::next_task_id(repo_root, branch, plan_id, plan_name)` — scans `<plan>/tasks/` for `TASK-<NNN>-*` directories
- `PlanIdGenerator::parse_plan_id("PLAN-005")` → `5`
- `TaskIdGenerator::parse_task_id("TASK-010")` → `10`
- `slugify("OAuth 2.0 Flow")` → `"oauth-20-flow"`
- `format_dir_name("PLAN-001", "oauth-flow")` → `"PLAN-001-oauth-flow"`

---

## Concurrency Enforcement

### Mechanism

The `ConcurrencyChecker` enforces parallel task limits by:

1. Reading `execution.json` from `.agent/state/<branch>/<plan_id>-<plan_name>/`
2. Counting tasks with `running` status in the `task_status_map`
3. Comparing against `max_parallel` from the global configuration (`config::get_max_parallel()`)

### API

```rust
ConcurrencyChecker::count_running(repo_root, branch, plan_id, plan_name)  // → Result<u16>
ConcurrencyChecker::get_max_parallel(repo_root, branch, plan_id, plan_name) // → Result<u16>
ConcurrencyChecker::can_dispatch(repo_root, branch, plan_id, plan_name)     // → Result<bool>
ConcurrencyChecker::dispatch_info(repo_root, branch, plan_id, plan_name)    // → Result<DispatchInfo>
```

### DispatchInfo

```rust
struct DispatchInfo {
    currently_running: u16,  // tasks currently in "running" status
    max_parallel: u16,       // configured concurrency limit
    can_dispatch: bool,      // true if currently_running < max_parallel
    remaining_slots: u16,    // max_parallel - currently_running (saturating)
}
```

### Concurrency-Gated Transitions

Transitions to `running` or `reviewing` are validated via `validate_transition_concurrency()`. If the concurrency limit is reached, the transition is blocked with `OverlordError::ConcurrencyLimitExceeded`.

Non-concurrency-sensitive transitions (e.g., `backlog` → `queued`) are always allowed.

---

## Dependency Auto-Queue

### Logic

The `DependencyResolver` automatically transitions tasks from `backlog` to `queued` when **all** of their dependencies are in `completed` status:

1. Lists all tasks in the plan
2. For each task in `backlog` status, reads its `status.json`
3. Checks the `dependencies` array — if empty, the task is immediately eligible
4. For each dependency task ID, reads its `status.json` and verifies it is `completed`
5. If ALL dependencies are `completed`, transitions the task to `queued` with actor `"overlord-auto-queue"`

### API

```rust
DependencyResolver::are_all_dependencies_met(repo_root, branch, plan_id, plan_name, task_id, task_name)
    // → Result<bool>

DependencyResolver::auto_queue_eligible_tasks(repo_root, branch, plan_id, plan_name)
    // → Result<Vec<(task_id, task_name)>>

DependencyResolver::auto_queue_tasks(repo_root, branch, plan_id, plan_name)
    // → Result<Vec<String>>  (queued task IDs)

DependencyResolver::resolve_dependent_tasks(repo_root, branch, plan_id, plan_name, completed_task_id)
    // → Result<Vec<String>>  (newly eligible task IDs)

DependencyResolver::build_dependency_graph(repo_root, branch, plan_id, plan_name)
    // → Result<HashMap<task_id, Vec<dep_ids>>>

DependencyResolver::detect_cycles(&graph)
    // → bool  (true if circular dependency detected)
```

### Dependency Graph Utilities

- `build_dependency_graph()` constructs the full adjacency list for a plan
- `detect_cycles()` uses DFS-based cycle detection to validate that no circular dependencies exist (safety check — planners should not create cycles)

---

## Heartbeat Monitoring and Recovery

### Monitoring

The `HeartbeatMonitor` detects stale heartbeats by:

1. Scanning all plans and tasks in a branch
2. For each task in `running` status, reading `heartbeat_at` from `status.json`
3. Parsing the RFC 3339 timestamp and computing elapsed time
4. If elapsed time exceeds the configured threshold (default: **30 minutes**), marking the task as stale

Missing `heartbeat_at` values for running tasks are treated as stale.

### Recovery Process

When a stale task is detected, recovery performs:

1. **Transition:** `running` → `queued` via the `TaskStateMachine`
2. **Increment:** `attempts` counter by 1
3. **Clear:** `agent` lease (set to `None`)
4. **Clear:** `started_at` (set to `None`)
5. **Clear:** `heartbeat_at` (set to `None`)
6. **Record:** Transition with actor `"overlord-heartbeat-recovery"`
7. **Write:** Updated `status.json` via atomic write through the persistence layer

### API

```rust
HeartbeatMonitor::new(stale_threshold_minutes: u64)
    .get_stale_threshold()                          // → Duration
    .is_heartbeat_stale(heartbeat_at: &str)          // → Result<(bool, elapsed_minutes)>
    .detect_stale_tasks(repo_root, branch)           // → Result<Vec<StaleTask>>
    .detect_stale_in_plan(repo_root, branch, plan_id, plan_name)
                                                     // → Result<Vec<StaleTask>>
    .recover_stale_tasks(repo_root, branch)          // → Result<Vec<String>>
    .recover_stale_in_plan(repo_root, branch, plan_id, plan_name)
                                                     // → Result<Vec<String>>
```

### StaleTask

```rust
struct StaleTask {
    task_id: String,
    plan_id: String,
    branch: String,
    plan_name: String,
    task_name: String,
    last_heartbeat: String,
    elapsed_minutes: f64,
}
```

---

## Scheduler Lifecycle and Configuration

### Lifecycle

The `OverlordScheduler` is the central orchestrator. It runs an async loop that executes all deterministic checks at a configurable interval:

```rust
OverlordScheduler::new(repo_root: PathBuf)
    .with_interval(Duration::from_secs(30))
    .with_heartbeat_threshold(30)  // minutes
    .start()  // → async loop
    .stop()   // → graceful shutdown
```

### Tick Order

Each scheduler tick executes checks in this order:

1. **Heartbeat recovery** — scans all branches, recovers stale tasks
2. **Dependency auto-queue** — scans all plans, queues eligible backlog tasks
3. **Dispatch queued tasks** — transitions `queued` → `running` tasks respecting concurrency limits (actor: `"overlord-dispatch"`)

### Configuration Options

| Option | Default | Method |
|---|---|---|
| Tick interval | 30 seconds | `.with_interval(Duration)` |
| Heartbeat stale threshold | 30 minutes | `.with_heartbeat_threshold(u64)` |
| Max parallel tasks | From config | `config::get_max_parallel()` |

### Error Handling

The scheduler handles errors gracefully — each check logs warnings on failure but continues to the next check. The loop never crashes due to individual check failures.

### Startup

The scheduler is initialized and spawned as a background `tokio` task on application startup, with lifecycle events logged via `tracing`.

---

## Public API for External Consumers

The `OverlordScheduler` exposes a clean async API designed for consumption by the REST API layer and the future Overlord agent layer (LLM-powered).

### Task Operations

```rust
scheduler.transition_task_status(branch, plan_id, plan_name, task_id, task_name, new_status, by)
    // → Result<()>
    // Validates transition, checks concurrency for running/reviewing targets

scheduler.generate_task_id(branch, plan_id, plan_name)
    // → Result<String>  (e.g., "TASK-004")
```

### Plan Operations

```rust
scheduler.transition_plan_status(branch, plan_id, plan_name, new_status, by)
    // → Result<()>
    // Validates transition; if approved, moves all tasks to backlog

scheduler.generate_plan_id(branch)
    // → Result<String>  (e.g., "PLAN-003")
```

### Status Queries

```rust
scheduler.get_dispatch_info(branch, plan_id, plan_name)
    // → Result<DispatchInfo>

scheduler.get_stale_tasks(branch)
    // → Result<Vec<StaleTask>>
```

### Lifecycle

```rust
scheduler.start().await   // → Result<()>  (async loop)
scheduler.stop()          // → ()         (graceful shutdown)
scheduler.is_running()    // → bool
```

All public methods return `overlord::Result<T>` and are `async`. The REST API layer maps these to HTTP responses, and the future agent layer invokes them programmatically.

### Error Type

```rust
enum OverlordError {
    InvalidTransition { from, to, entity },
    ConcurrencyLimitExceeded { current, max },
    IdGenerationError(String),
    DependencyError(String),
    HeartbeatError(String),
    PersistenceError(PersistenceError),
    ConfigError(ConfigError),
    SchedulerError(String),
}
```

`PersistenceError` and `ConfigError` are wrapped with `#[from]` for ergonomic `?` operator usage.
