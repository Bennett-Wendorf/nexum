# Plan: 003 - Persistence Layer

## Task Description
Implement the complete file-based persistence layer for nexum. This module handles all file I/O for the `.agent/` directory structure: reading and writing `plan.md`, `task.md`, `status.json`, and `execution.json` files. It provides atomic file writes to prevent partial reads during concurrent access, typed Rust structs for all file schemas, markdown parsing utilities using `pulldown-cmark`, and directory traversal helpers for navigating the `.agent/specs/` and `.agent/state/` hierarchies.

## Objective
Create a fully functional `src/persistence/` module that:
- Defines typed Rust structs for all persistence file schemas (`Plan`, `Task`, `TaskStatus`, `ExecutionState`)
- Provides atomic file write utilities (write to temp file, then `rename()` into place)
- Implements file read/write operations for `plan.md`, `task.md`, `status.json`, and `execution.json`
- Parses structured data from markdown files using `pulldown-cmark`
- Handles `.agent/` directory creation and traversal
- Manages the `specs/` (committed) vs `state/` (gitignored) split
- Supports the `<id>-<slug>` naming convention for directories and files
- Provides error handling via `thiserror` for all I/O operations

## Problem Statement
Nexum stores all work items (plans and tasks) as files on disk — no database. The `.agent/` directory contains markdown specs (committed to the repo) and JSON runtime state (gitignored). Multiple agents may read and write these files concurrently, so atomic writes are essential to prevent partial reads. Without a persistence layer, the Overlord cannot track task status, the Builder cannot read task definitions, and the Planner cannot generate new task files. The system must be robust against agent crashes (recovery via stale lease detection), support the full status machine for plans and tasks, and provide a clean typed API for all other modules.

## Solution Approach
Build a `persistence` module in `src/persistence/` with the following submodules:

1. **`schema.rs`** — Typed structs for all persistence file formats, deriving `Serialize`, `Deserialize`, `Debug`, and `Clone`. Includes `Plan`, `Task`, `TaskStatus`, `ExecutionState`, `AgentLease`, and `StatusTransition`.

2. **`io.rs`** — Core file I/O utilities: `read_file()`, `write_file()`, `atomic_write()`, `read_json()`, `write_json()`. The `atomic_write()` function writes to a temp file in the same directory, then calls `fs::rename()` for atomic replacement.

3. **`markdown.rs`** — Markdown parsing utilities using `pulldown-cmark`. Extracts structured data from `plan.md` and `task.md` files: headings, key-value metadata fields, section content. Provides `parse_plan_markdown()` and `parse_task_markdown()` functions.

4. **`directory.rs`** — Directory creation and traversal helpers. Functions to create the `.agent/` hierarchy, resolve paths for plans and tasks given a repo root, branch name, and IDs. Includes `ensure_agent_dir()`, `plan_path()`, `task_path()`, `state_path()`, `list_plans()`, `list_tasks()`.

5. **`errors.rs`** — Custom error type `PersistenceError` using `thiserror` with variants for I/O errors, parse errors, and schema validation errors.

6. **`tests.rs`** — Comprehensive unit tests for all modules.

The module follows these design principles from `design/persistence.md`:
- Markdown is source of truth — human-readable, agent-readable
- No database — everything lives as files
- Atomic writes via temp + rename for JSON state files
- `specs/` = committed specifications, `state/` = ephemeral runtime data
- ID scheme: Plans `PLAN-<NNN>` (sequential per branch), Tasks `TASK-<NNN>` (sequential per plan)
- Directory naming: `<id>-<slug>` format (e.g., `PLAN-001-oauth2-flow`)

## Relevant Files

### Existing Files
- `design/persistence.md` — Main persistence specification with directory layout, file formats, status machines, and runtime concerns
- `design/tech-stack.md` — Rust backend dependencies: `serde`, `serde_json`, `pulldown-cmark`, `thiserror`, `anyhow`
- `design/work-statuses.md` — Complete status machine definitions for plans and tasks
- `design/implementation-chunks.md` — Defines this as chunk 3 of the MVP
- `Cargo.toml` — Will contain relevant dependencies (added by chunk 001)
- `src/persistence/mod.rs` — Module stub (created by chunk 001)

### New Files (if needed)
- `src/persistence/mod.rs` — Module root, re-exports public types and functions
- `src/persistence/schema.rs` — Typed structs for Plan, Task, TaskStatus, ExecutionState, AgentLease, StatusTransition
- `src/persistence/io.rs` — File I/O utilities: read, write, atomic_write, JSON helpers
- `src/persistence/markdown.rs` — Markdown parsing with pulldown-cmark
- `src/persistence/directory.rs` — Directory creation, path resolution, traversal helpers
- `src/persistence/errors.rs` — PersistenceError enum with thiserror
- `src/persistence/tests.rs` — Unit tests for all modules

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: persistence-builder
  - Role: Implement all persistence layer modules (schema, I/O, markdown, directory, errors)
  - Agent: builder

- **Builder**
  - Name: persistence-tests-builder
  - Role: Write comprehensive unit tests for all persistence modules
  - Agent: builder

- **Validator**
  - Name: persistence-validator
  - Role: Verify all I/O operations, atomic writes, schema correctness, and test coverage
  - Agent: validator

- **Documenter**
  - Name: persistence-documenter
  - Role: Generate documentation for completed persistence layer
  - Agent: documenter

## Step by Step Tasks

### 1. Define Persistence Error Types
- **Task ID**: persistence-errors
- **Depends On**: none
- **Assigned To**: persistence-builder
- **Agent**: builder
- **Actions**:
  - Create `src/persistence/errors.rs` with `PersistenceError` enum using `thiserror`:
    - `Io(PathBuf, io::Error)` — File system I/O error with path context
    - `JsonParse(PathBuf, serde_json::Error)` — JSON deserialization error
    - `MarkdownParse(PathBuf, String)` — Markdown parsing error
    - `DirectoryNotFound(PathBuf)` — Expected directory does not exist
    - `FileNotFound(PathBuf)` — Expected file does not exist
    - `SchemaValidation(String)` — Schema validation failure
    - `PathResolution(String)` — Path resolution error (e.g., invalid branch name)
    - `AtomicWrite(PathBuf, io::Error)` — Atomic write failure (rename error)
  - Implement `std::fmt::Display` (provided by thiserror derive)
  - Implement `From<io::Error>` and `From<serde_json::Error>` for ergonomic `?` operator usage
  - Define type alias: `pub type Result<T> = std::result::Result<T, PersistenceError>`
- **Acceptance Criteria**:
  - `PersistenceError` compiles with `thiserror::Error` derive
  - All error variants include path context where applicable
  - `From` implementations allow `?` operator with `io::Error` and `serde_json::Error`
  - `Result<T>` type alias is defined and accessible

### 2. Define Schema Structs
- **Task ID**: persistence-schema
- **Depends On**: persistence-errors
- **Assigned To**: persistence-builder
- **Agent**: builder
- **Actions**:
  - Create `src/persistence/schema.rs` with the following typed structs:
    
    **Plan struct** (from `plan.md`):
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Plan {
        pub id: String,           // e.g., "PLAN-001"
        pub name: String,          // e.g., "oauth2-flow"
        pub status: PlanStatus,    // backlog | queued | planning | reviewing | plan-complete
        pub created: String,       // ISO date, e.g., "2026-05-21"
        pub branch: String,        // e.g., "feature/auth-overhaul"
        pub goal: String,          // Goal section content
        pub scope: String,         // Scope section content
        pub background: String,    // Background section content
        pub tasks: Vec<TaskReference>, // List of task references
    }
    
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TaskReference {
        pub id: String,            // e.g., "TASK-001"
        pub name: String,          // e.g., "auth-middleware"
        pub completed: bool,       // Checkbox state from markdown
    }
    ```
    
    **PlanStatus enum** (from persistence.md status machine):
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub enum PlanStatus {
        #[serde(rename = "backlog")]
        Backlog,
        #[serde(rename = "queued")]
        Queued,
        #[serde(rename = "planning")]
        Planning,
        #[serde(rename = "reviewing")]
        Reviewing,
        #[serde(rename = "plan-complete")]
        PlanComplete,
    }
    ```
    
    **Task struct** (from `task.md`):
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Task {
        pub id: String,            // e.g., "TASK-001"
        pub name: String,          // e.g., "auth-middleware"
        pub parent_plan: String,   // e.g., "PLAN-001"
        pub dependencies: Vec<String>, // Task IDs this task depends on
        pub description: String,   // Description section content
        pub acceptance_criteria: Vec<String>, // Bullet points
        pub files_to_modify: Vec<String>, // File paths
        pub background: String,    // Background section content
        pub notes: String,         // Notes section content
    }
    ```
    
    **TaskStatus struct** (from `status.json`):
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TaskStatus {
        pub id: String,            // e.g., "TASK-001"
        pub status: TaskStatusValue,
        pub agent: Option<AgentLease>,
        pub transitions: Vec<StatusTransition>,
        pub started_at: Option<String>,
        pub completed_at: Option<String>,
        pub attempts: u32,
        pub dependencies: Vec<String>,
        pub dependent_tasks: Vec<String>,
        pub heartbeat_at: Option<String>,
    }
    ```
    
    **TaskStatusValue enum** (from work-statuses.md):
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub enum TaskStatusValue {
        #[serde(rename = "backlog")]
        Backlog,
        #[serde(rename = "queued")]
        Queued,
        #[serde(rename = "running")]
        Running,
        #[serde(rename = "reviewing")]
        Reviewing,
        #[serde(rename = "waiting-manual-review")]
        WaitingManualReview,
        #[serde(rename = "merge-queue")]
        MergeQueue,
        #[serde(rename = "abandoned")]
        Abandoned,
        #[serde(rename = "completed")]
        Completed,
    }
    ```
    
    **AgentLease struct**:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AgentLease {
        pub role: String,          // e.g., "builder"
        pub pid: u32,
        pub leased_at: String,     // ISO timestamp
    }
    ```
    
    **StatusTransition struct**:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct StatusTransition {
        pub from: TaskStatusValue,
        pub to: TaskStatusValue,
        pub at: String,            // ISO timestamp
        pub by: String,            // e.g., "overlord"
    }
    ```
    
    **ExecutionState struct** (from `execution.json`):
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ExecutionState {
        pub plan_id: String,       // e.g., "PLAN-001"
        pub branch: String,
        pub tasks: Vec<String>,    // Task IDs
        pub task_status_map: HashMap<String, TaskStatusValue>,
    }
    ```
  - All structs derive `Serialize`, `Deserialize`, `Debug`, `Clone`
  - Use `#[serde(rename = "...")]` for kebab-case status values
  - Document each field with rustdoc comments referencing `design/persistence.md`
- **Acceptance Criteria**:
  - All structs compile with `serde` derive macros
  - `PlanStatus` enum contains exactly 5 variants: backlog, queued, planning, reviewing, plan-complete
  - `TaskStatusValue` enum contains exactly 8 variants: backlog, queued, running, reviewing, waiting-manual-review, merge-queue, abandoned, completed
  - `TaskStatus` matches the `status.json` schema from `design/persistence.md` exactly
  - `ExecutionState` matches the `execution.json` schema from `design/persistence.md` exactly
  - Serde rename attributes map enum variants to kebab-case strings
  - All fields are documented with rustdoc comments

### 3. Implement Atomic File I/O Utilities
- **Task ID**: persistence-io
- **Depends On**: persistence-errors
- **Assigned To**: persistence-builder
- **Agent**: builder
- **Actions**:
  - Create `src/persistence/io.rs` with:
    
    **Basic file operations**:
    - `pub fn read_file(path: &Path) -> Result<String>` — Read file contents as string
    - `pub fn write_file(path: &Path, content: &str) -> Result<()>` — Write string to file (overwrites)
    - `pub fn file_exists(path: &Path) -> bool` — Check if file exists
    - `pub fn directory_exists(path: &Path) -> bool` — Check if directory exists
    
    **Atomic write** (critical for concurrent access):
    - `pub fn atomic_write(path: &Path, content: &str) -> Result<()>` — Write atomically:
      1. Generate temp file path in same directory: `{parent}/.tmp-{random:8}`
      2. Write content to temp file using `fs::write`
      3. Call `fs::rename(temp_path, path)` — atomic on POSIX
      4. If rename fails, clean up temp file and return error
      5. Use `fastrand` or similar for random temp filename
    - Document why atomic write is needed (prevents partial reads during concurrent access)
    
    **JSON helpers**:
    - `pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T>` — Read and deserialize JSON
    - `pub fn write_json<T: Serialize>(path: &Path, data: &T) -> Result<()>` — Serialize and write JSON (non-atomic)
    - `pub fn atomic_write_json<T: Serialize>(path: &Path, data: &T) -> Result<()>` — Serialize and atomically write JSON
    - JSON formatting: pretty-printed with 2-space indent via `serde_json::to_string_pretty`
    
    **Directory operations**:
    - `pub fn create_dir_all(path: &Path) -> Result<()>` — Create directory and all parents
    - `pub fn list_dir(path: &Path) -> Result<Vec<DirEntry>>` — List directory entries
    - `pub fn remove_file(path: &Path) -> Result<()>` — Remove a file
    - `pub fn remove_dir_all(path: &Path) -> Result<()>` — Remove directory recursively
  - All functions return `persistence::Result<T>`
  - Map `io::Error` to `PersistenceError::Io` with path context
  - Map `serde_json::Error` to `PersistenceError::JsonParse` with path context
- **Acceptance Criteria**:
  - `atomic_write()` writes to a temp file in the same directory, then renames
  - `atomic_write()` cleans up temp file on rename failure
  - `read_json()` and `write_json()` handle serialization/deserialization correctly
  - `atomic_write_json()` combines JSON serialization with atomic write
  - All I/O errors include path context in the error message
  - JSON output is pretty-printed with 2-space indentation
  - `file_exists()` and `directory_exists()` return correct boolean values

### 4. Implement Markdown Parsing Utilities
- **Task ID**: persistence-markdown
- **Depends On**: persistence-io, persistence-schema
- **Assigned To**: persistence-builder
- **Agent**: builder
- **Actions**:
  - Create `src/persistence/markdown.rs` with:
    
    **Plan markdown parsing**:
    - `pub fn parse_plan_markdown(content: &str) -> Result<Plan>` — Parse `plan.md` content:
      - Extract `# Plan: <name>` heading for plan name
      - Extract key-value metadata: `**ID:**`, `**Status:**`, `**Created:**`, `**Branch:**`
      - Extract section content: `## Goal`, `## Scope`, `## Background`
      - Extract task list: `- [ ] [TASK-001] description` or `- [x] [TASK-001] description`
      - Use `pulldown-cmark::Parser` to iterate events
      - Build a `Plan` struct from extracted data
    
    **Task markdown parsing**:
    - `pub fn parse_task_markdown(content: &str) -> Result<Task>` — Parse `task.md` content:
      - Extract `# TASK-<NNN>: <name>` heading for task ID and name
      - Extract key-value metadata: `**Parent plan:**`, `**Dependencies:**`, `**Status:**`
      - Extract section content: `## Description`, `## Acceptance criteria`, `## Files to modify`, `## Background`, `## Notes`
      - Parse acceptance criteria as bullet list (lines starting with `- `)
      - Parse files to modify as bullet list
      - Build a `Task` struct from extracted data
    
    **Markdown generation** (for creating new files):
    - `pub fn render_plan_markdown(plan: &Plan) -> String` — Generate `plan.md` content from a `Plan` struct
    - `pub fn render_task_markdown(task: &Task) -> String` — Generate `task.md` content from a `Task` struct
    - Output format matches the templates in `design/persistence.md`
    
    **Helper functions**:
    - `fn extract_metadata(events: &[Event], key: &str) -> Option<String>` — Extract a `**Key:** Value` metadata field
    - `fn extract_section(events: &[Event], heading: &str) -> String` — Extract content under a `## Heading`
    - `fn parse_task_list(events: &[Event]) -> Vec<TaskReference>` — Parse checkbox task list items
  - Use `pulldown-cmark::Parser` and `pulldown-cmark::Event` for parsing
  - Handle edge cases: missing sections, empty content, malformed markdown
  - Document parsing assumptions (e.g., metadata must be in `**Key:** Value` format)
- **Acceptance Criteria**:
  - `parse_plan_markdown()` correctly extracts all fields from a valid `plan.md`
  - `parse_task_markdown()` correctly extracts all fields from a valid `task.md`
  - `render_plan_markdown()` produces markdown matching the `design/persistence.md` template
  - `render_task_markdown()` produces markdown matching the `design/persistence.md` template
  - Missing sections return empty strings (not errors)
  - Malformed markdown returns a `PersistenceError::MarkdownParse` error
  - Round-trip test: parse then render produces equivalent output

### 5. Implement Directory Helpers
- **Task ID**: persistence-directory
- **Depends On**: persistence-io
- **Assigned To**: persistence-builder
- **Agent**: builder
- **Actions**:
  - Create `src/persistence/directory.rs` with:
    
    **Path resolution** (given repo root, branch, plan/task IDs):
    - `pub fn agent_dir(repo_root: &Path) -> PathBuf` — Returns `<repo_root>/.agent/`
    - `pub fn specs_dir(repo_root: &Path, branch: &str) -> PathBuf` — Returns `<repo_root>/.agent/specs/<branch>/`
    - `pub fn state_dir(repo_root: &Path, branch: &str) -> PathBuf` — Returns `<repo_root>/.agent/state/<branch>/`
    - `pub fn plan_dir(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> PathBuf` — Returns `<specs>/<branch>/<plan_id>-<plan_name>/`
    - `pub fn task_dir(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> PathBuf` — Returns `<plan_dir>/tasks/<task_id>-<task_name>/`
    - `pub fn plan_markdown_path(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> PathBuf` — Returns `<plan_dir>/plan.md`
    - `pub fn task_markdown_path(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> PathBuf` — Returns `<task_dir>/task.md`
    - `pub fn task_status_path(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> PathBuf` — Returns `<task_dir>/status.json`
    - `pub fn execution_state_path(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> PathBuf` — Returns `<state>/<branch>/<plan_id>-<plan_name>/execution.json`
    - `pub fn task_log_dir(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> PathBuf` — Returns `<state>/<branch>/<plan_id>-<plan_name>/logs/<task_id>-<task_name>/`
    
    **Directory creation**:
    - `pub fn ensure_agent_dir(repo_root: &Path) -> Result<()>` — Create `.agent/`, `.agent/specs/`, `.agent/state/` if missing
    - `pub fn ensure_plan_dir(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<()>` — Create plan directory in both specs/ and state/
    - `pub fn ensure_task_dir(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> Result<()>` — Create task directory under plan
    
    **Directory traversal**:
    - `pub fn list_branches(repo_root: &Path) -> Result<Vec<String>>` — List branch directories under `specs/`
    - `pub fn list_plans(repo_root: &Path, branch: &str) -> Result<Vec<String>>` — List plan directories under `specs/<branch>/`
    - `pub fn list_tasks(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<Vec<String>>` — List task directories under `<plan>/tasks/`
    - `pub fn find_plan_by_id(repo_root: &Path, branch: &str, plan_id: &str) -> Result<PathBuf>` — Find plan directory by ID prefix
    
    **Slug generation**:
    - `pub fn slugify(text: &str) -> String` — Convert text to kebab-case slug (lowercase, spaces to hyphens, remove special chars)
  - All path functions use `PathBuf::join()` for cross-platform safety
  - Validate branch names (no path traversal, no special characters)
  - Document the directory hierarchy with a diagram comment
- **Acceptance Criteria**:
  - `agent_dir()` returns correct `.agent/` path
  - `plan_dir()` returns `<specs>/<branch>/<id>-<name>/`
  - `task_dir()` returns `<plan>/tasks/<id>-<name>/`
  - `ensure_agent_dir()` creates all required directories
  - `ensure_plan_dir()` creates directories in both specs/ and state/
  - `list_branches()`, `list_plans()`, `list_tasks()` return correct directory names
  - `slugify()` converts "OAuth 2.0 Flow" to "oauth-20-flow" or similar kebab-case
  - Path functions handle edge cases (empty strings, special characters)

### 6. Implement High-Level Plan/Task Operations
- **Task ID**: persistence-operations
- **Depends On**: persistence-io, persistence-markdown, persistence-directory, persistence-schema
- **Assigned To**: persistence-builder
- **Agent**: builder
- **Actions**:
  - Create `src/persistence/operations.rs` with high-level CRUD operations:
    
    **Plan operations**:
    - `pub fn create_plan(repo_root: &Path, plan: &Plan) -> Result<()>` — Create a new plan:
      1. Ensure plan directory exists (both specs/ and state/)
      2. Render `plan.md` using `render_plan_markdown()`
      3. Write `plan.md` to specs/ (non-atomic, markdown is committed)
      4. Create initial `execution.json` in state/ with empty task list
      5. Write `execution.json` atomically
    - `pub fn read_plan(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<Plan>` — Read and parse `plan.md`
    - `pub fn update_plan(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, plan: &Plan) -> Result<()>` — Update plan:
      1. Render updated `plan.md`
      2. Write to specs/ (non-atomic)
    
    **Task operations**:
    - `pub fn create_task(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task: &Task) -> Result<()>` — Create a new task:
      1. Ensure task directory exists
      2. Render `task.md` using `render_task_markdown()`
      3. Write `task.md` to specs/ (non-atomic)
      4. Create initial `status.json` with status=backlog, no agent, no transitions
      5. Write `status.json` atomically
    - `pub fn read_task(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> Result<Task>` — Read and parse `task.md`
    - `pub fn read_task_status(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str) -> Result<TaskStatus>` — Read and parse `status.json`
    - `pub fn update_task_status(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, task_name: &str, new_status: TaskStatusValue, by: &str) -> Result<TaskStatus>` — Update task status:
      1. Read current `status.json`
      2. Record transition (from current status to new status, with timestamp)
      3. Update status field
      4. Set `started_at` if transitioning to `running`
      5. Set `completed_at` if transitioning to `completed`
      6. Write updated `status.json` atomically
      7. Return updated `TaskStatus`
    
    **Execution state operations**:
    - `pub fn read_execution_state(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<ExecutionState>` — Read `execution.json`
    - `pub fn update_execution_state(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, state: &ExecutionState) -> Result<()>` — Write `execution.json` atomically
    - `pub fn add_task_to_execution(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str, task_id: &str, status: TaskStatusValue) -> Result<()>` — Add task to execution state:
      1. Read current execution state
      2. Add task ID to tasks list
      3. Add entry to task_status_map
      4. Write atomically
  - All update operations use atomic writes for JSON files
  - Timestamps use `chrono::Utc::now().to_rfc3339()` or `time::OffsetDateTime::now_utc()`
  - Document the optimistic locking pattern for task claiming
- **Acceptance Criteria**:
  - `create_plan()` creates plan.md in specs/ and execution.json in state/
  - `create_task()` creates task.md in specs/ and status.json atomically
  - `update_task_status()` records transitions with timestamps and actor
  - `update_task_status()` uses atomic write for status.json
  - `add_task_to_execution()` updates both tasks list and status map
  - All operations return appropriate errors on failure
  - Transition recording includes `from`, `to`, `at`, and `by` fields

### 7. Wire Up Module Exports
- **Task ID**: persistence-module-wiring
- **Depends On**: persistence-operations
- **Assigned To**: persistence-builder
- **Agent**: builder
- **Actions**:
  - Update `src/persistence/mod.rs` to:
    - Declare submodules: `mod errors; mod schema; mod io; mod markdown; mod directory; mod operations;`
    - Re-export public types:
      - `pub use errors::*;` (PersistenceError, Result)
      - `pub use schema::*;` (Plan, Task, TaskStatus, ExecutionState, PlanStatus, TaskStatusValue, AgentLease, StatusTransition, TaskReference)
    - Re-export public functions:
      - `pub use io::*;`
      - `pub use markdown::*;`
      - `pub use directory::*;`
      - `pub use operations::*;`
    - Include module-level documentation referencing `design/persistence.md`
  - Add `mod persistence;` to `src/main.rs` (if not already present from scaffolding)
- **Acceptance Criteria**:
  - All public types are accessible as `nexum::persistence::Plan`, etc.
  - All public functions are accessible as `nexum::persistence::read_plan()`, etc.
  - Module compiles without errors
  - Module documentation references design docs

### 8. Write Unit Tests
- **Task ID**: persistence-tests
- **Depends On**: persistence-module-wiring
- **Assigned To**: persistence-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `src/persistence/tests.rs` with comprehensive tests:
    
    **Schema tests**:
    - `test_plan_serde_roundtrip` — Serialize and deserialize a Plan
    - `test_task_serde_roundtrip` — Serialize and deserialize a Task
    - `test_task_status_serde_roundtrip` — Serialize and deserialize TaskStatus with agent lease
    - `test_execution_state_serde_roundtrip` — Serialize and deserialize ExecutionState
    - `test_plan_status_enum_rename` — Verify PlanStatus serializes to kebab-case strings
    - `test_task_status_value_enum_rename` — Verify TaskStatusValue serializes to kebab-case strings
    
    **I/O tests**:
    - `test_read_write_file` — Write and read back a file
    - `test_atomic_write` — Verify atomic write produces correct content
    - `test_atomic_write_temp_cleanup` — Verify temp file is cleaned up on failure
    - `test_read_write_json` — Write and read back a JSON file
    - `test_atomic_write_json` — Atomic JSON write and read
    - `test_file_exists` — Verify file_exists() with existing and non-existing files
    - `test_read_nonexistent_file` — Verify error on reading missing file
    
    **Markdown tests**:
    - `test_parse_plan_markdown` — Parse a sample plan.md
    - `test_parse_task_markdown` — Parse a sample task.md
    - `test_render_plan_markdown` — Render a Plan to markdown and verify structure
    - `test_render_task_markdown` — Render a Task to markdown and verify structure
    - `test_roundtrip_plan` — Parse then render, verify equivalent output
    - `test_roundtrip_task` — Parse then render, verify equivalent output
    - `test_parse_missing_sections` — Handle markdown with missing sections
    
    **Directory tests**:
    - `test_agent_dir` — Verify .agent/ path resolution
    - `test_plan_dir` — Verify plan directory path
    - `test_task_dir` — Verify task directory path
    - `test_ensure_agent_dir` — Verify directory creation
    - `test_ensure_plan_dir` — Verify plan directory creation in both specs/ and state/
    - `test_list_branches` — List branch directories
    - `test_list_plans` — List plan directories
    - `test_list_tasks` — List task directories
    - `test_slugify` — Verify kebab-case slug generation
    
    **Operations tests**:
    - `test_create_plan` — Create a plan and verify files exist
    - `test_read_plan` — Read a plan back
    - `test_create_task` — Create a task and verify files exist
    - `test_read_task` — Read a task back
    - `test_update_task_status` — Update task status and verify transition recording
    - `test_add_task_to_execution` — Add task to execution state
    - `test_task_status_running_sets_started_at` — Verify started_at is set on running transition
    - `test_task_status_completed_sets_completed_at` — Verify completed_at is set on completed transition
    
    **Test utilities**:
    - `fn create_test_repo() -> tempfile::TempDir` — Create a temporary directory simulating a repo root
    - `fn setup_agent_structure(dir: &Path) -> Result<()>` — Set up .agent/ directory structure in temp dir
  - Use `tempfile` crate for isolated test directories
  - Use sample markdown content matching the templates in `design/persistence.md`
- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum persistence`
  - Tests cover all modules: schema, I/O, markdown, directory, operations
  - Tests use temporary directories to avoid polluting the real filesystem
  - Round-trip tests verify parse → render equivalence
  - Atomic write tests verify temp file cleanup
  - Operation tests verify file creation and content correctness

### 9. Final Validation
- **Task ID**: validate-all
- **Depends On**: persistence-tests
- **Assigned To**: persistence-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum persistence` — all persistence tests must pass
  - Run `cargo clippy --package nexum` — no warnings in persistence module
  - Verify `atomic_write()` uses temp file + rename pattern
  - Verify `status.json` files are written atomically
  - Verify `plan.md` and `task.md` files are written non-atomically (they're committed specs)
  - Verify `execution.json` is written atomically
  - Verify all schema structs match `design/persistence.md` file format specifications
  - Verify `PlanStatus` enum has exactly 5 variants matching persistence.md
  - Verify `TaskStatusValue` enum has exactly 8 variants matching work-statuses.md
  - Verify directory paths follow the `.agent/specs/<branch>/<plan>/tasks/<task>/` structure
  - Verify directory paths follow the `.agent/state/<branch>/<plan>/` structure
  - Verify markdown parsing handles all sections from the templates
  - Verify slugify produces valid kebab-case strings
  - Verify all error types include path context
  - Verify `thiserror` is used for PersistenceError
  - Verify `serde` rename attributes produce correct kebab-case JSON keys
  - Verify `status.json` is excluded from git (matches .gitignore pattern)
  - Verify `.agent/state/` is excluded from git (matches .gitignore pattern)

### 10. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: persistence-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the file schemas (plan.md, task.md, status.json, execution.json)
  - Document the directory structure and naming conventions
  - Document the atomic write mechanism and why it's used
  - Document the Rust API (public functions and types)
  - Document the markdown parsing assumptions and limitations

## Acceptance Criteria
- `cargo check` succeeds with no errors in the persistence module
- `cargo test --package nexum persistence` passes all tests
- `cargo clippy --package nexum` produces no warnings in persistence module
- All schema structs compile with serde derive macros and match `design/persistence.md` specifications
- `PlanStatus` enum: 5 variants (backlog, queued, planning, reviewing, plan-complete)
- `TaskStatusValue` enum: 8 variants (backlog, queued, running, reviewing, waiting-manual-review, merge-queue, abandoned, completed)
- `atomic_write()` uses temp file + `fs::rename()` pattern
- `atomic_write()` cleans up temp file on failure
- JSON files (`status.json`, `execution.json`) are written atomically
- Markdown files (`plan.md`, `task.md`) are written non-atomically
- `TaskStatus` struct matches `status.json` schema exactly (id, status, agent, transitions, started_at, completed_at, attempts, dependencies, dependent_tasks, heartbeat_at)
- `ExecutionState` struct matches `execution.json` schema exactly (plan_id, branch, tasks, task_status_map)
- Directory structure follows `.agent/specs/<branch>/<plan>/tasks/<task>/` and `.agent/state/<branch>/<plan>/`
- Directory naming uses `<id>-<slug>` format (e.g., `PLAN-001-oauth2-flow`)
- Markdown parsing extracts all fields from `plan.md` and `task.md` templates
- Markdown rendering produces output matching the design doc templates
- Round-trip parsing (parse → render) produces equivalent output
- `PersistenceError` uses `thiserror` with path context for I/O errors
- `Result<T>` type alias is defined for ergonomic error handling
- All public types and functions are re-exported from `mod.rs`
- `slugify()` produces valid kebab-case strings
- Unit tests cover all modules with temporary directory isolation
- Transition recording includes from, to, timestamp, and actor fields
- `started_at` is set when transitioning to `running`
- `completed_at` is set when transitioning to `completed`

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum persistence` — Run persistence module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds
- `find src/persistence/ -type f` — Verify all module files exist
- `grep -r "atomic_write" src/persistence/` — Verify atomic write is used for JSON files
- `grep -r "fs::rename" src/persistence/` — Verify atomic rename pattern is used

## Notes
- This plan depends on chunk 001 (Project Scaffolding) being completed first. The `src/persistence/mod.rs` stub and `Cargo.toml` with `serde`, `serde_json`, `pulldown-cmark`, `thiserror`, `anyhow` dependencies must exist before this plan can be executed.
- The `tempfile` crate should be added as a dev-dependency for testing.
- The `chrono` or `time` crate should be added for timestamp generation in `StatusTransition` records. Prefer `chrono` since it's more commonly used in the Rust ecosystem.
- The `fastrand` crate may be needed for generating random temp filenames in `atomic_write()`. Alternatively, use `std::process::id()` combined with a counter.
- The `HashMap` in `ExecutionState` requires `use std::collections::HashMap`.
- The `pulldown-cmark` API is event-based (iterator of `Event`s). Parsing requires collecting events and then processing them to extract structured data. Consider caching the parsed events for efficiency.
- The `status.json` file is NOT committed to git (per `.gitignore` pattern `.agent/specs/**/tasks/**/status.json`). Only `task.md` is committed.
- The entire `.agent/state/` directory is gitignored.
- The `execution.json` file should include a `heartbeat_at` field in future iterations for stale lease detection by the Overlord. The current schema includes it in `TaskStatus` but not in `ExecutionState` — this is a design decision that can be revisited.
- Consider adding a `RepoContext` struct that holds the repo root path, to avoid passing it to every function. This would be a nice ergonomic improvement but is not required for MVP.
- The `slugify()` function should handle Unicode characters gracefully. Consider using `unicode-segmentation` or a simple approach of removing non-alphanumeric characters (except hyphens).
- For the `update_task_status()` function, consider adding a `lease_agent()` variant that specifically handles the optimistic locking pattern described in `design/persistence.md` (read status, check it's "queued", atomically write "running" with agent lease).
