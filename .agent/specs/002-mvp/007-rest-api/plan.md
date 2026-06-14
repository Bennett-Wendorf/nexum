# Plan: 007 - REST API

## Task Description
Implement the complete REST API layer for nexum using the Axum web framework. This module provides HTTP endpoints for plan and task CRUD operations, status transitions, execution state queries, and configuration access. The API is the public contract — the bundled Svelte frontend is just one consumer. Any third party (Slack bot, Discord bot, CLI tool, custom dashboard, CI/CD pipeline) must be able to build their own client using only this documented API. See `design/backend-api.md` for the full API contract.

## Objective
Create a fully functional `src/api/` module that:
- Defines an Axum router with all REST endpoints organized by resource
- Implements request/response types for all endpoints using `serde` for JSON serialization
- Provides plan CRUD endpoints (list, get, create, update, delete) with status transition support
- Provides task CRUD endpoints (list by plan, get, create, update, delete) with status transition support
- Provides execution state endpoints (get execution state for a plan, list running tasks)
- Provides configuration endpoints (get current config, list registered agents)
- Implements error handling middleware that converts internal errors to proper HTTP responses
- Validates all request inputs and returns descriptive error messages
- Integrates with the persistence layer for all data operations
- Integrates with the config module for configuration queries
- Supports the complete status machines defined in `design/work-statuses.md`
- Generates an OpenAPI 3.1 spec (`docs/api/openapi.json`) for machine-readable API documentation
- Produces API responses that are never UI-coupled (no HTML, no frontend-specific formatting)

## Problem Statement
Nexum needs a REST API to expose its plan/task management capabilities to any client — the bundled web dashboard, Slack bots, Discord bots, CLI tools, CI/CD pipelines, and any other third-party integration. Without this API, nothing can interact with the system. The API must:
- Follow RESTful conventions with proper HTTP methods and status codes
- Serialize/deserialize data using serde (per tech-stack design)
- Handle errors gracefully using thiserror + anyhow (per tech-stack design)
- Route through Axum's ergonomic routing system
- Integrate with the persistence layer for file-based data access
- Support the full status machine transitions for plans and tasks
- Provide configuration introspection for any client
- Be fully documented in an OpenAPI 3.1 spec for third-party integrations
- Never couple responses to the bundled web UI

## Solution Approach
Build an `api` module in `src/api/` with the following submodules:

1. **`mod.rs`** — Module root, router construction, and middleware setup
2. **`types.rs`** — Request/response DTOs for all endpoints
3. **`plans.rs`** — Plan CRUD + status transition endpoints
4. **`tasks.rs`** — Task CRUD + status transition endpoints
5. **`execution.rs`** — Execution state and running tasks endpoints
6. **`config.rs`** — Configuration and agent listing endpoints
7. **`errors.rs`** — API error type and error response handler
8. **`middleware.rs`** — Error handling middleware, request validation utilities
9. **`openapi.rs`** — OpenAPI 3.1 spec generation for `docs/api/openapi.json`
10. **`tests.rs`** — Integration tests using `axum::test` or `reqwest`

The API follows these design principles:
- **API-first**: Documented before implemented; web UI is just one consumer (see `design/backend-api.md`)
- **RESTful routing**: `/api/v1/plans`, `/api/v1/plans/{plan_id}`, `/api/v1/plans/{plan_id}/tasks`, etc.
- **JSON bodies**: All request/response bodies use `serde_json` serialization, never UI-coupled
- **Error handling**: `thiserror` for typed errors, `anyhow` for internal error chaining
- **Axum extractors**: Use `Path`, `Json`, `State`, `Query` extractors
- **State sharing**: Use Axum's `State` extractor to share `AppState` (repo root, config)
- **Validation**: Validate inputs at the handler level with descriptive error responses
- **Versioned**: URL-based versioning (`/api/v1/...`) for future compatibility

## Relevant Files

### Existing Files
- `design/backend-api.md` — Full API contract: endpoints, schemas, WebSocket events, versioning, third-party integration requirements
- `design/tech-stack.md` — Defines Axum as web framework, serde for serialization, thiserror + anyhow for errors
- `design/persistence.md` — Defines plan/task data models and file structures
- `design/work-statuses.md` — Defines status machines for plans and tasks
- `design/agent-harness-integration.md` — Defines agent registration and configuration structure
- `design/implementation-chunks.md` — Defines this as chunk 7 of the MVP
- `Cargo.toml` — Will contain relevant dependencies (axum, serde, serde_json, thiserror, anyhow)
- `src/api/mod.rs` — Module stub (created by chunk 001)
- `src/persistence/` — Persistence layer module (completed by chunk 003)
- `src/config/` — Configuration module (completed by chunk 002)

### New Files (if needed)
- `src/api/mod.rs` — Module root, router construction with middleware
- `src/api/types.rs` — Request/response DTOs for all endpoints
- `src/api/plans.rs` — Plan CRUD + status transition handlers
- `src/api/tasks.rs` — Task CRUD + status transition handlers
- `src/api/execution.rs` — Execution state handlers
- `src/api/config.rs` — Configuration and agent listing handlers
- `src/api/errors.rs` — ApiError enum and error response handler
- `src/api/middleware.rs` — Error handling middleware and validation utilities
- `src/api/openapi.rs` — OpenAPI 3.1 spec generation
- `src/api/tests.rs` — Integration tests for all endpoints
- `docs/api/openapi.json` — Machine-readable API documentation

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: api-core-builder
  - Role: Implement router setup, error handling, types, and middleware
  - Agent: builder

- **Builder**
  - Name: api-endpoints-builder
  - Role: Implement all endpoint handlers (plans, tasks, execution, config)
  - Agent: builder

- **Builder**
  - Name: api-tests-builder
  - Role: Write integration tests for all endpoints
  - Agent: builder

- **Validator**
  - Name: api-validator
  - Role: Verify endpoint correctness, error handling, and integration with persistence/config
  - Agent: validator

- **Documenter**
  - Name: api-documenter
  - Role: Generate API documentation for completed REST API
  - Agent: documenter

## Step by Step Tasks

### 1. Define API Error Types and Error Response Handler
- **Task ID**: api-errors
- **Depends On**: none
- **Assigned To**: api-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/errors.rs` with:
    
    **ApiError enum** using `thiserror`:
    ```rust
    #[derive(Debug, thiserror::Error)]
    pub enum ApiError {
        #[error("Not found: {0}")]
        NotFound(&'static str),
        
        #[error("Invalid request: {0}")]
        BadRequest(String),
        
        #[error("Conflict: {0}")]
        Conflict(String),
        
        #[error("Internal error: {0}")]
        Internal(anyhow::Error),
        
        #[error("Unauthorized: {0}")]
        Unauthorized(String),
        
        #[error("Method not allowed: {0}")]
        MethodNotAllowed(String),
        
        #[error("Validation error: {0}")]
        Validation(String),
    }
    ```
    
    **HTTP status code mapping** via `IntoResponse` impl:
    ```rust
    impl IntoResponse for ApiError {
        fn into_response(self) -> Response {
            match &self {
                ApiError::NotFound(_) => (StatusCode::NOT_FOUND, Json(ApiErrorResponse::from(&self))).into_response(),
                ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, Json(ApiErrorResponse::from(&self))).into_response(),
                ApiError::Conflict(msg) => (StatusCode::CONFLICT, Json(ApiErrorResponse::from(&self))).into_response(),
                ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiErrorResponse::from(&self))).into_response(),
                ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, Json(ApiErrorResponse::from(&self))).into_response(),
                ApiError::MethodNotAllowed(msg) => (StatusCode::METHOD_NOT_ALLOWED, Json(ApiErrorResponse::from(&self))).into_response(),
                ApiError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, Json(ApiErrorResponse::from(&self))).into_response(),
            }
        }
    }
    ```
    
    **ApiErrorResponse struct**:
    ```rust
    #[derive(Serialize, Debug)]
    pub struct ApiErrorResponse {
        pub error: String,
        pub message: String,
        pub status: u16,
    }
    ```
    
    **From implementations** for ergonomic `?` operator:
    - `impl From<persistence::PersistenceError> for ApiError` — Map persistence errors to appropriate API errors
    - `impl From<config::ConfigError> for ApiError` — Map config errors to API errors
    - `impl From<serde_json::Error> for ApiError` — Map JSON errors to BadRequest
    
    **Error conversion logic**:
    - `PersistenceError::FileNotFound` → `ApiError::NotFound`
    - `PersistenceError::DirectoryNotFound` → `ApiError::NotFound`
    - `PersistenceError::Io` → `ApiError::Internal`
    - `PersistenceError::JsonParse` → `ApiError::BadRequest`
    - `PersistenceError::SchemaValidation` → `ApiError::Validation`
  - Implement `std::fmt::Display` for `ApiErrorResponse`
  - Document each error variant with examples of when it occurs
- **Acceptance Criteria**:
  - `ApiError` compiles with `thiserror::Error` derive
  - `IntoResponse` implementation maps each variant to correct HTTP status code
  - `ApiErrorResponse` serializes to JSON with `error`, `message`, and `status` fields
  - `From` implementations allow `?` operator with persistence and config errors
  - NotFound → 404, BadRequest → 400, Conflict → 409, Internal → 500, Validation → 422

### 2. Define Request/Response DTOs
- **Task ID**: api-types
- **Depends On**: api-errors
- **Assigned To**: api-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/types.rs` with all request/response DTOs:
    
    **Plan DTOs**:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct PlanResponse {
        pub id: String,
        pub name: String,
        pub status: String,
        pub created: String,
        pub branch: String,
        pub goal: String,
        pub scope: String,
        pub background: String,
        pub tasks: Vec<TaskReferenceResponse>,
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TaskReferenceResponse {
        pub id: String,
        pub name: String,
        pub completed: bool,
    }
    
    #[derive(Serialize, Deserialize, Debug)]
    pub struct CreatePlanRequest {
        pub name: String,
        pub branch: String,
        pub goal: String,
        pub scope: Option<String>,
        pub background: Option<String>,
    }
    
    #[derive(Serialize, Deserialize, Debug)]
    pub struct UpdatePlanRequest {
        pub name: Option<String>,
        pub goal: Option<String>,
        pub scope: Option<String>,
        pub background: Option<String>,
    }
    
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TransitionPlanStatusRequest {
        pub status: String,
        pub by: Option<String>,
    }
    ```
    
    **Task DTOs**:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TaskResponse {
        pub id: String,
        pub name: String,
        pub parent_plan: String,
        pub dependencies: Vec<String>,
        pub description: String,
        pub acceptance_criteria: Vec<String>,
        pub files_to_modify: Vec<String>,
        pub background: String,
        pub notes: String,
        pub status: TaskStatusResponse,
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TaskStatusResponse {
        pub id: String,
        pub status: String,
        pub agent: Option<AgentLeaseResponse>,
        pub transitions: Vec<StatusTransitionResponse>,
        pub started_at: Option<String>,
        pub completed_at: Option<String>,
        pub attempts: u32,
        pub dependencies: Vec<String>,
        pub dependent_tasks: Vec<String>,
        pub heartbeat_at: Option<String>,
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AgentLeaseResponse {
        pub role: String,
        pub pid: u32,
        pub leased_at: String,
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct StatusTransitionResponse {
        pub from: String,
        pub to: String,
        pub at: String,
        pub by: String,
    }
    
    #[derive(Serialize, Deserialize, Debug)]
    pub struct CreateTaskRequest {
        pub name: String,
        pub parent_plan: String,
        pub dependencies: Vec<String>,
        pub description: String,
        pub acceptance_criteria: Vec<String>,
        pub files_to_modify: Vec<String>,
        pub background: Option<String>,
        pub notes: Option<String>,
    }
    
    #[derive(Serialize, Deserialize, Debug)]
    pub struct UpdateTaskRequest {
        pub name: Option<String>,
        pub description: Option<String>,
        pub acceptance_criteria: Option<Vec<String>>,
        pub files_to_modify: Option<Vec<String>>,
        pub background: Option<String>,
        pub notes: Option<String>,
    }
    
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TransitionTaskStatusRequest {
        pub status: String,
        pub by: Option<String>,
    }
    ```
    
    **Execution state DTOs**:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ExecutionStateResponse {
        pub plan_id: String,
        pub branch: String,
        pub tasks: Vec<String>,
        pub task_status_map: HashMap<String, String>,
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct RunningTaskResponse {
        pub task_id: String,
        pub task_name: String,
        pub plan_id: String,
        pub branch: String,
        pub agent: Option<AgentLeaseResponse>,
        pub started_at: Option<String>,
        pub heartbeat_at: Option<String>,
    }
    ```
    
    **Config DTOs**:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ConfigResponse {
        pub server_host: String,
        pub server_port: u16,
        pub max_parallel: u16,
        pub default_timeout_seconds: u64,
        pub log_level: String,
        pub yolo_mode: bool,
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AgentRegistrationResponse {
        pub name: String,
        pub r#type: String,
        pub spawn_command: String,
        pub model: Option<String>,
        pub tool_permissions: Option<Vec<String>>,
        pub timeout_seconds: Option<u64>,
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AgentsResponse {
        pub agents: Vec<AgentRegistrationResponse>,
    }
    ```
    
    **List response wrapper**:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ListResponse<T> {
        pub items: Vec<T>,
        pub total: usize,
    }
    ```
    
    **AppState** (shared application state):
    ```rust
    #[derive(Clone)]
    pub struct AppState {
        pub repo_root: PathBuf,
        pub config: config::Config,
    }
    ```
    
    **Query parameter types**:
    ```rust
    #[derive(Deserialize, Debug)]
    pub struct PlanListQuery {
        pub branch: Option<String>,
        pub status: Option<String>,
    }
    
    #[derive(Deserialize, Debug)]
    pub struct TaskListQuery {
        pub status: Option<String>,
    }
    ```
  - All response DTOs derive `Serialize`, `Deserialize`, `Debug`, `Clone`
  - All request DTOs derive `Serialize`, `Deserialize`, `Debug`
  - Document each field with rustdoc comments
  - Use `HashMap` from `std::collections` for `task_status_map`
- **Acceptance Criteria**:
  - All DTOs compile with serde derive macros
  - `PlanResponse` includes all fields from persistence::Plan
  - `TaskResponse` includes all fields from persistence::Task plus status info
  - `TaskStatusResponse` matches the status.json schema from design/persistence.md
  - `ExecutionStateResponse` matches the execution.json schema from design/persistence.md
  - `ConfigResponse` includes all global settings fields
  - `AgentRegistrationResponse` includes all agent registration fields
  - `AppState` is Clone for Axum State extractor compatibility
  - `ListResponse<T>` is generic for reusable list response format
  - Request DTOs use `Option<T>` for optional fields

### 3. Implement Error Handling Middleware
- **Task ID**: api-middleware
- **Depends On**: api-errors
- **Assigned To**: api-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/middleware.rs` with:
    
    **Request ID layer**:
    - Add a middleware layer that generates a unique request ID for each request
    - Include request ID in response headers (`X-Request-ID`)
    - Include request ID in error responses for debugging
    
    **Error handling layer**:
    - Use `tower::Layer` or Axum's built-in error handling
    - Catch all `Result<_, ApiError>` returns from handlers
    - Convert to proper HTTP responses via `IntoResponse`
    
    **Request validation utilities**:
    - `pub fn validate_status_transition(current: &str, requested: &str, entity_type: &str) -> Result<(), ApiError>` — Validates a status transition is allowed per the status machines in `design/work-statuses.md`
    - `pub fn validate_plan_status(status: &str) -> Result<(), ApiError>` — Validates a plan status value is valid
    - `pub fn validate_task_status(status: &str) -> Result<(), ApiError>` — Validates a task status value is valid
    - `pub fn validate_branch_name(name: &str) -> Result<(), ApiError>` — Validates branch name (no path traversal, alphanumeric + hyphens + slashes)
    - `pub fn validate_slug(name: &str) -> Result<(), ApiError>` — Validates slug format (kebab-case, alphanumeric + hyphens)
    
    **Status transition maps** (from `design/work-statuses.md`):
    ```rust
    // Plan transitions (pre-planning)
    const PLAN_TRANSITIONS_PRE: &[(&str, &[&str])] = &[
        ("draft", &["queued"]),
        ("queued", &["planning"]),
        ("planning", &["reviewing"]),
        ("reviewing", &["approved", "queued"]),
    ];
    
    // Plan transitions (post-planning)
    const PLAN_TRANSITIONS_POST: &[(&str, &[&str])] = &[
        ("approved", &["complete", "rejected"]),
        // "complete" and "rejected" are terminal
    ];
    
    // Task transitions
    const TASK_TRANSITIONS: &[(&str, &[&str])] = &[
        ("backlog", &["queued", "abandoned"]),
        ("queued", &["running"]),
        ("running", &["reviewing", "abandoned"]),
        ("reviewing", &["waiting-manual-review", "merge-queue", "abandoned"]),
        ("waiting-manual-review", &["merge-queue", "abandoned"]),
        ("merge-queue", &["completed"]),
        // "abandoned" and "completed" are terminal
    ];
    ```
  - Document validation rules and their rationale
- **Acceptance Criteria**:
  - Request ID is generated and included in response headers
  - `validate_status_transition()` correctly validates all allowed transitions
  - `validate_plan_status()` accepts all 7 plan statuses (draft, queued, planning, reviewing, approved, complete, rejected)
  - `validate_task_status()` accepts all 8 task statuses (backlog, queued, running, reviewing, waiting-manual-review, merge-queue, abandoned, completed)
  - Invalid transitions return `ApiError::Validation` with descriptive message
  - Branch name validation rejects path traversal attempts
  - Terminal statuses (complete, rejected, abandoned, completed) reject all transitions

### 4. Implement Plan CRUD Endpoints
- **Task ID**: api-plans
- **Depends On**: api-types, api-middleware
- **Assigned To**: api-endpoints-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/plans.rs` with the following handlers:
    
    **GET /api/plans** — List all plans:
    ```rust
    pub async fn list_plans(
        State(state): State<AppState>,
        Query(query): Query<PlanListQuery>,
    ) -> Result<Json<ListResponse<PlanResponse>>, ApiError>
    ```
    - List plans from `.agent/specs/` directory
    - Filter by branch if `branch` query param provided
    - Filter by status if `status` query param provided
    - Parse each `plan.md` to extract metadata
    - Return `ListResponse<PlanResponse>` with items and total count
    
    **GET /api/plans/:branch/:plan_id** — Get a specific plan:
    ```rust
    pub async fn get_plan(
        State(state): State<AppState>,
        Path((branch, plan_id)): Path<(String, String)>,
    ) -> Result<Json<PlanResponse>, ApiError>
    ```
    - Resolve plan directory using persistence layer helpers
    - Read and parse `plan.md`
    - Return `PlanResponse`
    - Return 404 if plan not found
    
    **POST /api/plans** — Create a new plan:
    ```rust
    pub async fn create_plan(
        State(state): State<AppState>,
        Json(req): Json<CreatePlanRequest>,
    ) -> Result<Json<PlanResponse>, ApiError>
    ```
    - Validate request fields (name non-empty, branch valid)
    - Generate plan ID using persistence layer (or overlord if available)
    - Create plan directory structure
    - Write `plan.md` with initial content
    - Create initial `execution.json`
    - Return created `PlanResponse` with 201 status
    
    **PUT /api/plans/:branch/:plan_id** — Update a plan:
    ```rust
    pub async fn update_plan(
        State(state): State<AppState>,
        Path((branch, plan_id)): Path<(String, String)>,
        Json(req): Json<UpdatePlanRequest>,
    ) -> Result<Json<PlanResponse>, ApiError>
    ```
    - Read existing plan
    - Apply partial update from request fields
    - Write updated `plan.md`
    - Return updated `PlanResponse`
    
    **DELETE /api/plans/:branch/:plan_id** — Delete a plan:
    ```rust
    pub async fn delete_plan(
        State(state): State<AppState>,
        Path((branch, plan_id)): Path<(String, String)>,
    ) -> Result<StatusCode, ApiError>
    ```
    - Verify plan exists
    - Remove plan directory from both specs/ and state/
    - Return 204 No Content
    
    **PATCH /api/plans/:branch/:plan_id/status** — Transition plan status:
    ```rust
    pub async fn transition_plan_status(
        State(state): State<AppState>,
        Path((branch, plan_id)): Path<(String, String)>,
        Json(req): Json<TransitionPlanStatusRequest>,
    ) -> Result<Json<PlanResponse>, ApiError>
    ```
    - Read current plan status
    - Validate transition using `validate_status_transition()`
    - Update plan status in `plan.md`
    - Return updated `PlanResponse`
  - All handlers use `State<AppState>` for accessing repo root and config
  - All handlers return `Result<..., ApiError>` for automatic error response conversion
  - Map persistence errors to API errors using the `From` implementations
- **Acceptance Criteria**:
  - `list_plans` returns all plans, supports branch and status filtering
  - `get_plan` returns a single plan or 404
  - `create_plan` creates plan.md and execution.json, returns 201
  - `update_plan` applies partial updates to existing plan
  - `delete_plan` removes plan directories, returns 204
  - `transition_plan_status` validates transition, updates plan.md
  - All handlers use correct HTTP methods (GET, POST, PUT, DELETE, PATCH)
  - Invalid plan IDs return 404
  - Invalid status transitions return 422 with descriptive message

### 5. Implement Task CRUD Endpoints
- **Task ID**: api-tasks
- **Depends On**: api-types, api-middleware
- **Assigned To**: api-endpoints-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/tasks.rs` with the following handlers:
    
    **GET /api/plans/:branch/:plan_id/tasks** — List tasks for a plan:
    ```rust
    pub async fn list_tasks(
        State(state): State<AppState>,
        Path((branch, plan_id)): Path<(String, String)>,
        Query(query): Query<TaskListQuery>,
    ) -> Result<Json<ListResponse<TaskResponse>>, ApiError>
    ```
    - Verify plan exists
    - List tasks from `<plan>/tasks/` directory
    - Filter by status if `status` query param provided
    - Parse each `task.md` and `status.json` to build response
    - Return `ListResponse<TaskResponse>` with items and total count
    
    **GET /api/plans/:branch/:plan_id/tasks/:task_id** — Get a specific task:
    ```rust
    pub async fn get_task(
        State(state): State<AppState>,
        Path((branch, plan_id, task_id)): Path<(String, String, String)>,
    ) -> Result<Json<TaskResponse>, ApiError>
    ```
    - Resolve task directory
    - Read `task.md` and `status.json`
    - Combine into `TaskResponse`
    - Return 404 if task not found
    
    **POST /api/plans/:branch/:plan_id/tasks** — Create a new task:
    ```rust
    pub async fn create_task(
        State(state): State<AppState>,
        Path((branch, plan_id)): Path<(String, String)>,
        Json(req): Json<CreateTaskRequest>,
    ) -> Result<Json<TaskResponse>, ApiError>
    ```
    - Verify plan exists
    - Validate request fields (name non-empty, valid dependencies)
    - Generate task ID
    - Create task directory
    - Write `task.md` with initial content
    - Create initial `status.json` with status=backlog
    - Update plan's task list
    - Update execution state
    - Return created `TaskResponse` with 201 status
    
    **PUT /api/plans/:branch/:plan_id/tasks/:task_id** — Update a task:
    ```rust
    pub async fn update_task(
        State(state): State<AppState>,
        Path((branch, plan_id, task_id)): Path<(String, String, String)>,
        Json(req): Json<UpdateTaskRequest>,
    ) -> Result<Json<TaskResponse>, ApiError>
    ```
    - Read existing task
    - Apply partial update to `task.md`
    - Return updated `TaskResponse`
    
    **DELETE /api/plans/:branch/:plan_id/tasks/:task_id** — Delete a task:
    ```rust
    pub async fn delete_task(
        State(state): State<AppState>,
        Path((branch, plan_id, task_id)): Path<(String, String, String)>,
    ) -> Result<StatusCode, ApiError>
    ```
    - Verify task exists
    - Remove task directory
    - Update plan's task list
    - Update execution state
    - Return 204 No Content
    
    **PATCH /api/plans/:branch/:plan_id/tasks/:task_id/status** — Transition task status:
    ```rust
    pub async fn transition_task_status(
        State(state): State<AppState>,
        Path((branch, plan_id, task_id)): Path<(String, String, String)>,
        Json(req): Json<TransitionTaskStatusRequest>,
    ) -> Result<Json<TaskResponse>, ApiError>
    ```
    - Read current task status from `status.json`
    - Validate transition using `validate_status_transition()`
    - Update `status.json` with new status, transition record
    - Update execution state task_status_map
    - Return updated `TaskResponse`
    
    **POST /api/plans/:branch/:plan_id/tasks/:task_id/claim** — Claim a task (optimistic locking):
    ```rust
    pub async fn claim_task(
        State(state): State<AppState>,
        Path((branch, plan_id, task_id)): Path<(String, String, String)>,
        Json(req): Json<ClaimTaskRequest>,
    ) -> Result<Json<TaskResponse>, ApiError>
    ```
    - Read current `status.json`
    - Verify current status is "queued"
    - Atomically write new status "running" with agent lease
    - If another agent claimed it first, return 409 Conflict
    - Return updated `TaskResponse`
    
    **ClaimTaskRequest**:
    ```rust
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ClaimTaskRequest {
        pub agent_role: String,
        pub agent_pid: u32,
    }
    ```
  - All handlers use correct HTTP methods
  - Task claim uses atomic write pattern from persistence layer
  - Status transitions record `from`, `to`, `at`, and `by` fields
- **Acceptance Criteria**:
  - `list_tasks` returns all tasks for a plan, supports status filtering
  - `get_task` returns task with status info or 404
  - `create_task` creates task.md and status.json, updates plan and execution state
  - `update_task` applies partial updates to existing task
  - `delete_task` removes task, updates plan and execution state
  - `transition_task_status` validates transition, updates status.json with transition record
  - `claim_task` implements optimistic locking with atomic write
  - Invalid task IDs return 404
  - Invalid status transitions return 422
  - Task claim on non-queued task returns 409 Conflict
  - All handlers return correct HTTP status codes

### 6. Implement Execution State Endpoints
- **Task ID**: api-execution
- **Depends On**: api-types
- **Assigned To**: api-endpoints-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/execution.rs` with the following handlers:
    
    **GET /api/plans/:branch/:plan_id/execution** — Get execution state for a plan:
    ```rust
    pub async fn get_execution_state(
        State(state): State<AppState>,
        Path((branch, plan_id)): Path<(String, String)>,
    ) -> Result<Json<ExecutionStateResponse>, ApiError>
    ```
    - Read `execution.json` from `.agent/state/<branch>/<plan>/`
    - Return `ExecutionStateResponse`
    - Return 404 if execution state not found (plan not yet executing)
    
    **GET /api/running** — List all currently running tasks across all plans:
    ```rust
    pub async fn list_running_tasks(
        State(state): State<AppState>,
    ) -> Result<Json<Vec<RunningTaskResponse>>, ApiError>
    ```
    - Scan all branches under `.agent/state/`
    - For each plan, read `execution.json`
    - For each task in `running` status, read `status.json`
    - Collect into `Vec<RunningTaskResponse>`
    - Return all running tasks
    
    **GET /api/health** — Health check endpoint:
    ```rust
    pub async fn health_check() -> impl IntoResponse
    ```
    - Return `{"status": "ok"}` with 200 status
    - Used for liveness probes and basic connectivity checks
  - Document the execution state response format
- **Acceptance Criteria**:
  - `get_execution_state` returns execution.json data or 404
  - `list_running_tasks` scans all branches and returns running tasks
  - `health_check` returns 200 with `{"status": "ok"}`
  - Running tasks include agent lease info and timestamps
  - Empty running task list returns `[]` (not error)

### 7. Implement Configuration Endpoints
- **Task ID**: api-config
- **Depends On**: api-types
- **Assigned To**: api-endpoints-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/config.rs` with the following handlers:
    
    **GET /api/config** — Get current configuration:
    ```rust
    pub async fn get_config(
        State(state): State<AppState>,
    ) -> Result<Json<ConfigResponse>, ApiError>
    ```
    - Read global settings from `state.config`
    - Map to `ConfigResponse`
    - Return current configuration
    
    **GET /api/agents** — List registered agents:
    ```rust
    pub async fn list_agents(
        State(state): State<AppState>,
    ) -> Result<Json<AgentsResponse>, ApiError>
    ```
    - Read agent registrations from `state.config`
    - Map to `Vec<AgentRegistrationResponse>`
    - Return `AgentsResponse` with agents list
  - Configuration endpoint does NOT expose sensitive data (API keys, OAuth tokens)
  - Agent listing only exposes name, type, spawn_command, and optional fields from config
- **Acceptance Criteria**:
  - `get_config` returns current configuration or 500 on error
  - `list_agents` returns all registered agents from config
  - Agent responses include name, type, spawn_command, and optional fields
  - No sensitive data (API keys, credentials) is exposed
  - Empty agent list returns `{"agents": []}` (not error)

### 8. Wire Up Router and Module Exports
- **Task ID**: api-router
- **Depends On**: api-plans, api-tasks, api-execution, api-config
- **Assigned To**: api-core-builder
- **Agent**: builder
- **Actions**:
  - Update `src/api/mod.rs` to:
    
    **Declare submodules**:
    ```rust
    mod errors;
    mod types;
    mod plans;
    mod tasks;
    mod execution;
    mod config;
    mod middleware;
    
    #[cfg(test)]
    mod tests;
    ```
    
    **Re-export public types**:
    ```rust
    pub use errors::*;
    pub use types::*;
    pub use middleware::*;
    ```
    
    **Router construction function**:
    ```rust
    pub fn create_router(state: AppState) -> Router {
        Router::new()
            // Health check
            .route("/api/v1/health", get(execution::health_check))
            
            // Plan endpoints
            .route("/api/v1/plans", get(plans::list_plans).post(plans::create_plan))
            .route("/api/v1/plans/{branch}/{plan_id}", get(plans::get_plan).put(plans::update_plan).delete(plans::delete_plan))
            .route("/api/v1/plans/{branch}/{plan_id}/status", patch(plans::transition_plan_status))
            
            // Task endpoints
            .route("/api/v1/plans/{branch}/{plan_id}/tasks", get(tasks::list_tasks).post(tasks::create_task))
            .route("/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}", get(tasks::get_task).put(tasks::update_task).delete(tasks::delete_task))
            .route("/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/status", patch(tasks::transition_task_status))
            .route("/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/claim", post(tasks::claim_task))
            
            // Execution endpoints
            .route("/api/v1/plans/{branch}/{plan_id}/execution", get(execution::get_execution_state))
            .route("/api/v1/running", get(execution::list_running_tasks))
            
            // Config endpoints
            .route("/api/v1/config", get(config::get_config))
            .route("/api/v1/agents", get(config::list_agents))
            
            // Middleware
            .layer(middleware::request_id_layer())
            .with_state(state)
    }
    ```
    
    **Integration with main.rs**:
    - Update `src/main.rs` to call `api::create_router(state)` and build the Axum app
    - Include static file serving for the Svelte frontend (via `tower_http::services::ServeDir`)
    - Set up tracing subscriber for logging
  - Document the complete route table
- **Acceptance Criteria**:
  - Router includes all endpoint routes with correct HTTP methods
  - Router uses `State<AppState>` for all handlers
  - Router includes request ID middleware layer
  - Route paths follow RESTful conventions
  - `create_router()` returns a configured `Router` ready to be served
  - Static file serving is configured for Svelte build output
  - Tracing subscriber is configured for structured logging
  - All modules are properly declared and re-exported

### 9. Write Integration Tests
- **Task ID**: api-tests
- **Depends On**: api-router
- **Assigned To**: api-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/tests.rs` with comprehensive integration tests:
    
    **Test setup**:
    - `fn test_app() -> (TestClient, TempDir)` — Create a test Axum app with a temporary directory as repo root
    - `fn setup_test_repo(dir: &Path)` — Create minimal `.agent/` structure for testing
    - Use `axum::testing::TestClient` and `tempfile::TempDir`
    
    **Plan endpoint tests**:
    - `test_list_plans_empty` — Returns empty list when no plans exist
    - `test_list_plans_with_filter` — Filter by branch and status
    - `test_create_plan` — POST creates plan, returns 201
    - `test_create_plan_invalid_name` — Empty name returns 400
    - `test_get_plan` — Returns plan by ID
    - `test_get_plan_not_found` — Returns 404 for missing plan
    - `test_update_plan` — PUT applies partial update
    - `test_delete_plan` — DELETE removes plan, returns 204
    - `test_transition_plan_status_valid` — Valid transition succeeds
    - `test_transition_plan_status_invalid` — Invalid transition returns 422
    - `test_transition_plan_terminal` — Terminal status rejects transitions
    
    **Task endpoint tests**:
    - `test_list_tasks` — Returns tasks for a plan
    - `test_list_tasks_status_filter` — Filter by status
    - `test_create_task` — POST creates task, returns 201
    - `test_get_task` — Returns task with status info
    - `test_get_task_not_found` — Returns 404
    - `test_update_task` — PUT applies partial update
    - `test_delete_task` — DELETE removes task, returns 204
    - `test_transition_task_status_valid` — Valid transition succeeds
    - `test_transition_task_status_invalid` — Invalid transition returns 422
    - `test_claim_task` — POST claim transitions queued→running
    - `test_claim_task_already_running` — Already running returns 409
    - `test_claim_task_not_queued` — Non-queued returns 409
    
    **Execution endpoint tests**:
    - `test_get_execution_state` — Returns execution state
    - `test_get_execution_state_not_found` — Returns 404 for non-executing plan
    - `test_list_running_tasks` — Returns running tasks
    - `test_health_check` — Returns 200 with status ok
    
    **Config endpoint tests**:
    - `test_get_config` — Returns current configuration
    - `test_list_agents` — Returns registered agents
    
    **Error handling tests**:
    - `test_bad_request_format` — Malformed JSON returns 400
    - `test_validation_error_format` — Validation errors return 422 with proper body
    - `test_not_found_format` — 404 responses include proper error body
    - `test_request_id_header` — Response includes X-Request-ID header
  - Use `tokio::test` for async tests
  - Use `axum::body::Body` and `http::Request` for test construction
  - Use `serde_json::json!` for building request bodies
- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum api`
  - Tests cover all endpoint handlers
  - Tests cover both success and error paths
  - Tests verify correct HTTP status codes
  - Tests verify JSON response body structure
  - Tests verify error response format
  - Tests use temporary directories for isolation
  - Status transition validation is tested for both valid and invalid transitions
  - Task claim optimistic locking is tested

### 10. Final Validation
- **Task ID**: validate-all
- **Depends On**: api-tests
- **Assigned To**: api-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum api` — all API tests must pass
  - Run `cargo clippy --package nexum` — no warnings in api module
  - Verify all endpoint routes are registered in the router
  - Verify all handlers use correct HTTP methods
  - Verify error responses follow the `ApiErrorResponse` format
  - Verify `ApiError::IntoResponse` maps to correct HTTP status codes
  - Verify request validation catches: invalid status transitions, empty names, invalid branch names
  - Verify plan CRUD operations work correctly (list, get, create, update, delete, transition)
  - Verify task CRUD operations work correctly (list, get, create, update, delete, transition, claim)
  - Verify execution state endpoints return correct data
  - Verify config endpoints return current configuration
  - Verify health check returns 200
  - Verify status transition validation matches `design/work-statuses.md` exactly
  - Verify plan status machine: draft→queued→planning→reviewing→approved→complete/rejected
  - Verify task status machine: backlog→queued→running→reviewing→waiting-manual-review→merge-queue→completed
  - Verify terminal statuses reject all transitions
  - Verify task claim implements optimistic locking (queued→running)
  - Verify request ID is included in response headers
  - Verify all DTOs serialize/deserialize correctly
  - Verify persistence errors are mapped to appropriate API errors

### 11. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: api-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the complete REST API route table with methods, paths, request/response formats
  - Document the status machines and valid transitions
  - Document error response format
  - Document request validation rules
  - Document the AppState and how it's shared via Axum State extractor
  - Generate OpenAPI 3.1 spec at `docs/api/openapi.json` covering all endpoints, schemas, and error responses
  - Verify the OpenAPI spec validates against the OpenAPI 3.1 schema

## Acceptance Criteria
- `cargo check` succeeds with no errors in the api module
- `cargo test --package nexum api` passes all tests
- `cargo clippy --package nexum` produces no warnings in api module
- All REST endpoints are registered and accessible:
  - Plans: GET/POST /api/v1/plans, GET/PUT/DELETE /api/v1/plans/{branch}/{plan_id}, PATCH /api/v1/plans/{branch}/{plan_id}/status
  - Tasks: GET/POST /api/v1/plans/{branch}/{plan_id}/tasks, GET/PUT/DELETE /api/v1/plans/{branch}/{plan_id}/tasks/{task_id}, PATCH /api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/status, POST /api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/claim
  - Execution: GET /api/v1/plans/{branch}/{plan_id}/execution, GET /api/v1/running
  - Config: GET /api/v1/config, GET /api/v1/agents
  - Health: GET /api/v1/health
- All handlers return proper HTTP status codes:
  - 200 for successful GET/PUT/PATCH
  - 201 for successful POST (create)
  - 204 for successful DELETE
  - 400 for malformed requests
  - 404 for not found resources
  - 409 for conflicts (task claim)
  - 422 for validation errors
  - 500 for internal errors
- Error responses follow `ApiErrorResponse` format with `error`, `message`, and `status` fields
- Request ID is included in all response headers (`X-Request-ID`)
- Status transition validation matches `design/work-statuses.md` exactly:
  - Plan statuses: draft, queued, planning, reviewing, approved, complete, rejected
  - Task statuses: backlog, queued, running, reviewing, waiting-manual-review, merge-queue, abandoned, completed
- Terminal statuses (complete, rejected, abandoned, completed) reject all transitions
- Task claim implements optimistic locking with atomic write
- All DTOs serialize/deserialize correctly with serde
- Persistence errors are mapped to appropriate API errors
- Config errors are mapped to appropriate API errors
- Branch name validation rejects path traversal attempts
- `AppState` is shared via Axum `State` extractor
- Router construction includes all routes and middleware
- Integration tests cover all endpoints, success paths, and error paths
- OpenAPI 3.1 spec exists at `docs/api/openapi.json` and covers all endpoints, schemas, and error responses
- API responses are never UI-coupled (no HTML, no frontend-specific formatting)

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum api` — Run API module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds
- `find src/api/ -type f` — Verify all module files exist
- `grep -r "api/v1/plans" src/api/` — Verify plan routes are registered
- `grep -r "api/v1/tasks" src/api/` — Verify task routes are registered
- `grep -r "ApiError" src/api/` — Verify error handling is used throughout
- `test -f docs/api/openapi.json` — Verify OpenAPI spec exists

## Notes
- This plan depends on chunks 001 (Project Scaffolding), 002 (Configuration System), and 003 (Persistence Layer) being completed first. The `src/api/mod.rs` stub, persistence layer types and functions, and config module must exist before this plan can be executed.
- **API-first principle**: The API is the public contract. The bundled web UI is just one consumer. All design decisions must support third-party clients (Slack, Discord, CLI, CI/CD). See `design/backend-api.md`.
- All routes use `/api/v1/` prefix for URL-based versioning.
- The `tower` crate is needed for middleware layers (request ID). It's a dependency of Axum but may need explicit import.
- The `axum::extract::Query` extractor automatically deserializes query parameters from the URL.
- The `axum::extract::Path` extractor automatically deserializes path parameters.
- The `axum::extract::Json` extractor automatically deserializes JSON request bodies.
- The `axum::extract::State` extractor provides access to shared application state.
- For integration tests, `axum::testing::TestClient` allows sending HTTP requests to the router without starting a server.
- The `tempfile` crate should be added as a dev-dependency for testing.
- The `serde_json::json!` macro is useful for building JSON bodies in tests.
- Consider adding CORS middleware in a future enhancement if the frontend and backend are served from different origins. For MVP, they're served from the same origin (Axum serves static files).
- Authentication is not implemented in MVP (localhost-only per tech-stack design). Future auth support should use the Axum session/cookie system or JWT.
- The `ClaimTaskRequest` includes `agent_pid` which will be the subprocess PID of the spawned agent. This is used for stale heartbeat detection by the Overlord.
- The PATCH method is used for status transitions because it represents a partial update (only the status field changes).
- The router uses Axum's `.route()` method for registering handlers. Each route can have multiple HTTP methods chained (e.g., `.route("/api/v1/plans", get(...).post(...))`).
- The OpenAPI 3.1 spec (`docs/api/openapi.json`) must be generated from the implementation. Consider using `utoipa` for derive-macro-based spec generation, or generate the spec manually.
- WebSocket endpoints (`/api/v1/ws/events`, `/api/v1/ws/agent/{agent_id}`) are deferred to chunk 013 (WebSocket Event Streaming) but must be documented in the OpenAPI spec.
