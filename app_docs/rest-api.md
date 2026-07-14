# REST API Module

## Overview

The Nexum REST API provides a versioned HTTP interface (`/api/v1/`) for managing plans, tasks, execution state, and server configuration. Built on Axum, it exposes CRUD operations for plans and tasks, status transition endpoints, agent claiming, execution inspection, and config/agent listing. All endpoints return JSON and share a consistent error response format.

The API is designed as the public contract for Nexum — any third-party client (web UI, CLI, Slack bot, CI/CD pipeline) can interact with the system exclusively through these endpoints.

---

## Route Table

All routes are prefixed with `/api/v1/`.

### Health

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| `GET` | `/api/v1/health` | `health_check` | Returns `{"status": "ok"}` |

### Plans

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| `GET` | `/api/v1/plans` | `list_plans` | List all plans (filterable by `branch`, `status`) |
| `POST` | `/api/v1/plans` | `create_plan` | Create a new plan (201 Created) |
| `GET` | `/api/v1/plans/{branch}/{plan_id}` | `get_plan` | Get a specific plan |
| `PUT` | `/api/v1/plans/{branch}/{plan_id}` | `update_plan` | Partially update a plan |
| `DELETE` | `/api/v1/plans/{branch}/{plan_id}` | `delete_plan` | Delete a plan and all associated data (204 No Content) |
| `PATCH` | `/api/v1/plans/{branch}/{plan_id}/status` | `transition_plan_status` | Transition plan status |

### Tasks

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| `GET` | `/api/v1/plans/{branch}/{plan_id}/tasks` | `list_tasks` | List tasks for a plan (filterable by `status`) |
| `POST` | `/api/v1/plans/{branch}/{plan_id}/tasks` | `create_task` | Create a new task within a plan (201 Created) |
| `GET` | `/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}` | `get_task` | Get a specific task |
| `PUT` | `/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}` | `update_task` | Partially update a task |
| `DELETE` | `/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}` | `delete_task` | Delete a task (204 No Content) |
| `PATCH` | `/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/status` | `transition_task_status` | Transition task status |
| `POST` | `/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/claim` | `claim_task` | Agent claims (leases) a queued task |

### Execution

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| `GET` | `/api/v1/plans/{branch}/{plan_id}/execution` | `get_execution_state` | Get execution state for a plan |
| `GET` | `/api/v1/running` | `list_running_tasks` | List all tasks currently running across all branches |

### Config

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| `GET` | `/api/v1/config` | `get_config` | Get current server configuration |
| `GET` | `/api/v1/agents` | `list_agents` | List registered agents |

---

## Status Machines and Valid Transitions

### Plan Statuses

Plans progress through two phases: **pre-planning** (idea → approval) and **post-planning** (approval → completion).

#### Pre-planning transitions

```
draft → queued → planning → reviewing → approved | queued
```

| Current | Allowed Transitions |
|---------|---------------------|
| `draft` | `queued` |
| `queued` | `planning` |
| `planning` | `reviewing` |
| `reviewing` | `approved`, `queued` |

#### Post-planning transitions

```
approved → complete | rejected
```

| Current | Allowed Transitions |
|---------|---------------------|
| `approved` | `complete`, `rejected` |

#### Terminal statuses

`complete` and `rejected` are terminal — no further transitions are permitted.

### Task Statuses

Tasks follow a lifecycle from backlog through execution to completion or abandonment.

```
backlog → queued → running → reviewing → waiting-manual-review | merge-queue → completed
```

Tasks may be `abandoned` from any non-terminal state.

| Current | Allowed Transitions |
|---------|---------------------|
| `backlog` | `queued`, `abandoned` |
| `queued` | `running` |
| `running` | `reviewing`, `abandoned` |
| `reviewing` | `waiting-manual-review`, `merge-queue`, `abandoned` |
| `waiting-manual-review` | `merge-queue`, `abandoned` |
| `merge-queue` | `completed` |

#### Terminal statuses

`abandoned` and `completed` are terminal — no further transitions are permitted.

---

## Error Response Format

All errors return a JSON body with the `ApiErrorResponse` schema:

```json
{
  "error": "string",
  "message": "string",
  "status": 400
}
```

| Field | Type | Description |
|-------|------|-------------|
| `error` | `string` | Short error type name (e.g. `"NotFound"`, `"Validation"`, `"Conflict"`) |
| `message` | `string` | Full human-readable message |
| `status` | `integer` | HTTP status code |

### Error Variants and Status Codes

| Variant | Status Code | Description |
|---------|-------------|-------------|
| `NotFound` | 404 | Resource does not exist |
| `BadRequest` | 400 | Malformed or invalid request |
| `Conflict` | 409 | Concurrency conflict (TOCTOU) |
| `Internal` | 500 | Unexpected server error |
| `Unauthorized` | 401 | Missing or invalid authentication |
| `MethodNotAllowed` | 405 | HTTP method not permitted |
| `Validation` | 422 | Request failed schema validation |

---

## Request Validation Rules

### Branch Name Validation

- Must be non-empty.
- Must not contain `..` (path traversal protection).
- Must not start or end with `/`.
- Only alphanumeric characters, hyphens (`-`), underscores (`_`), and forward slashes (`/`) are allowed.

### Status Transitions

- The target status must be a valid status for the entity type (plan or task).
- The transition from current status to target status must be in the allowed transition map.
- Terminal statuses reject all transitions.

### Plan and Task Names

- Must be non-empty (whitespace-only names are rejected).

### Task Claiming

- The task must be in `queued` status to be claimed.
- The request must include `agent_role` (string) and `agent_pid` (integer).

---

## AppState and Axum State Extractor

Shared application state is carried by the `AppState` struct:

```rust
pub struct AppState {
    pub repo_root: PathBuf,   // Absolute path to the Nexum repository root
    pub config: Config,        // Loaded server configuration
}
```

The `AppState` is cloned and attached to the Axum router via `.with_state(state)`. Each route handler receives it through Axum's `State` extractor:

```rust
pub async fn list_plans(
    State(state): State<AppState>,
    Query(query): Query<PlanListQuery>,
) -> Result<Json<ListResponse<PlanResponse>>, ApiError> { ... }
```

This provides every handler access to the repository root path and the loaded configuration without passing them as arguments.

---

## Integration with Persistence Layer

The API delegates all data operations to the `persistence` module:

- **Plan operations**: `read_plan`, `create_plan`, `update_plan`, `list_plans`, `find_plan_by_id`
- **Task operations**: `read_task`, `create_task`, `list_tasks`, `read_task_status`, `update_task_status`
- **Execution state**: `read_execution_state`, `update_execution_state`, `add_task_to_execution`
- **File I/O**: `write_file`, `atomic_write_json`, `list_dir`

Error conversion is handled automatically via `From` implementations:
- `PersistenceError` → `ApiError` (maps file-not-found to 404, IO errors to 500, etc.)
- `ConfigError` → `ApiError::Internal`
- `serde_json::Error` → `ApiError::BadRequest`

This allows the `?` operator to be used freely in route handlers.

---

## Integration with Config Module

The `config` module provides server settings and agent registrations. The API exposes:

- **`GET /api/v1/config`**: Returns non-sensitive global settings (host, port, max_parallel, timeout, log level, yolo mode). API keys and OAuth tokens are excluded.
- **`GET /api/v1/agents`**: Returns all registered agents with name, type, spawn command, tool permissions, and timeout. The internal `working_dir` field is omitted.

---

## Middleware

### Request ID Middleware

Every response includes an `X-Request-ID` header in the format `req-{timestamp_ms}-{counter}`, providing unique traceability for each request. The middleware is applied as a layer on the router.

---

## Request/Response Schemas

### CreatePlanRequest

```json
{
  "name": "string",           // required
  "branch": "string",         // required
  "goal": "string",           // required
  "scope": "string",          // optional
  "background": "string"      // optional
}
```

### UpdatePlanRequest

```json
{
  "name": "string",           // optional
  "goal": "string",           // optional
  "scope": "string",          // optional
  "background": "string"      // optional
}
```

### TransitionPlanStatusRequest

```json
{
  "status": "string",         // required (target status)
  "by": "string"              // optional (actor identifier)
}
```

### CreateTaskRequest

```json
{
  "name": "string",                    // required
  "parent_plan": "string",             // required
  "dependencies": ["string"],          // required
  "description": "string",             // required
  "acceptance_criteria": ["string"],   // required
  "files_to_modify": ["string"],       // required
  "background": "string",              // optional
  "notes": "string"                    // optional
}
```

### UpdateTaskRequest

```json
{
  "name": "string",                    // optional
  "description": "string",             // optional
  "acceptance_criteria": ["string"],   // optional
  "files_to_modify": ["string"],       // optional
  "background": "string",              // optional
  "notes": "string"                    // optional
}
```

### TransitionTaskStatusRequest

```json
{
  "status": "string",         // required (target status)
  "by": "string"              // optional (actor identifier)
}
```

### ClaimTaskRequest

```json
{
  "agent_role": "string",     // required
  "agent_pid": 0              // required (integer)
}
```

### ListResponse (generic wrapper)

```json
{
  "items": [],                // array of items
  "total": 0                  // total count
}
```

---

## Query Parameters

### Plan List (`GET /api/v1/plans`)

| Parameter | Type | Description |
|-----------|------|-------------|
| `branch` | `string` (optional) | Filter by git branch name |
| `status` | `string` (optional) | Filter by plan status |

### Task List (`GET /api/v1/plans/{branch}/{plan_id}/tasks`)

| Parameter | Type | Description |
|-----------|------|-------------|
| `status` | `string` (optional) | Filter by task status |
