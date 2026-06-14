# Plan: 007.1 — Deduplicate Path Resolution Helpers

## Task Description
Extract the duplicated `resolve_plan_path` and `resolve_task_path` helper functions from `plans.rs`, `tasks.rs`, and `execution.rs` into a single shared location in `src/api/middleware.rs`. These functions are copy-pasted verbatim (with minor variable-naming differences) across multiple modules, violating DRY and creating a maintenance hazard.

## Objective
Eliminate all duplicated path resolution helpers by extracting them into `src/api/middleware.rs` and updating each consuming module to import the shared versions. After this change:
- `resolve_plan_path` exists in exactly one place
- `resolve_task_path` exists in exactly one place
- All three modules (`plans.rs`, `tasks.rs`, `execution.rs`) import from `middleware` instead of defining their own copies
- The project compiles and all existing tests pass

## Problem Statement
The code review identified two sets of duplicated functions:

1. **`resolve_plan_path`** — duplicated across 3 modules:
   - `src/api/plans.rs:129-148`
   - `src/api/tasks.rs:178-197`
   - `src/api/execution.rs:51-70`

2. **`resolve_task_path`** — duplicated across 2 modules:
   - `src/api/tasks.rs:85-116`
   - `src/api/execution.rs:109-140`

Each copy performs the same logic: locate a plan or task directory by ID, extract the name from the directory slug, and return the resolved path. The copies differ only in local variable names (`plan_dir` vs `plan_dir_path`) and explicit vs implicit `PathBuf` type annotations. This duplication means any bug fix or logic change must be applied to 2-3 places simultaneously, creating a real maintenance hazard.

## Solution Approach
Extract both helpers into `src/api/middleware.rs`, which is already the home for shared API utilities (validation functions, request ID middleware). Re-export them through `mod.rs` so consuming modules can import via `crate::api::middleware::*` (which is already the pattern used for other middleware exports).

The canonical versions will use:
- The variable naming from `plans.rs` (the original owner of `resolve_plan_path`)
- The variable naming from `tasks.rs` (the original owner of `resolve_task_path`)
- Consistent return types: `Result<(PathBuf, String), ApiError>` for plan, `Result<(PathBuf, String, String), ApiError>` for task

Each consuming module will:
1. Remove its local `fn resolve_plan_path` / `fn resolve_task_path` definition
2. Import the shared version via `use crate::api::middleware::{resolve_plan_path, resolve_task_path};` (as needed)

## Relevant Files

### Files to Modify
- `src/api/middleware.rs` — Add `resolve_plan_path` and `resolve_task_path` as public functions
- `src/api/mod.rs` — Add re-exports for the new helpers in the `pub use middleware::*` glob (automatic)
- `src/api/plans.rs` — Remove local `resolve_plan_path`, import from `middleware`
- `src/api/tasks.rs` — Remove local `resolve_plan_path` and `resolve_task_path`, import from `middleware`
- `src/api/execution.rs` — Remove local `resolve_plan_path` and `resolve_task_path`, import from `middleware`

### Reference Files (read-only)
- `src/api/errors.rs` — `ApiError` type used by both helpers
- `src/persistence/mod.rs` — `find_plan_by_id`, `parse_slug`, `plan_dir`, `list_dir` used by helpers

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: dedup-builder
  - Role: Extract helpers to middleware.rs, update all consuming modules
  - Agent: builder

- **Validator**
  - Name: dedup-validator
  - Role: Verify compilation, test pass, no remaining duplication
  - Agent: validator

## Step by Step Tasks

### 1. Extract Helpers into `src/api/middleware.rs`
- **Task ID**: extract-helpers
- **Depends On**: none
- **Assigned To**: dedup-builder
- **Agent**: builder
- **Actions**:
  - Add the following two public functions to `src/api/middleware.rs`, placed after the validation functions section (after `validate_slug`) and before the `#[cfg(test)]` module:

    **`resolve_plan_path`** (canonical version from `plans.rs`):
    ```rust
    /// Locate a plan directory and extract the plan name from its slug.
    ///
    /// Searches for a plan directory whose slug starts with `<plan_id>-`
    /// under `.agent/specs/<branch>/`. Returns the resolved path and plan
    /// name, or a 404 error if the plan does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::NotFound`] if the plan cannot be located or
    /// its directory slug cannot be parsed.
    pub fn resolve_plan_path(
        repo_root: &std::path::Path,
        branch: &str,
        plan_id: &str,
    ) -> std::result::Result<(std::path::PathBuf, String), ApiError> {
        use crate::persistence::{find_plan_by_id, parse_slug};

        let plan_dir = find_plan_by_id(repo_root, branch, plan_id)
            .map_err(|_| ApiError::NotFound("plan not found"))?;

        let slug = plan_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ApiError::NotFound("plan not found"))?;

        let plan_name = match parse_slug(slug) {
            (Some(_), Some(name)) => name.to_string(),
            _ => return Err(ApiError::NotFound("plan not found")),
        };

        Ok((plan_dir, plan_name))
    }
    ```

    **`resolve_task_path`** (canonical version from `tasks.rs`):
    ```rust
    /// Locate a task directory within a plan and extract its ID and name.
    ///
    /// Searches all subdirectories of the plan's `tasks/` directory for a
    /// slug that starts with `<task_id>-`. Returns the resolved path, task
    /// ID, and task name, or a 404 error if the task does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::NotFound`] if the task cannot be located or
    /// its directory slug cannot be parsed.
    pub fn resolve_task_path(
        repo_root: &std::path::Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
        task_id: &str,
    ) -> std::result::Result<(std::path::PathBuf, String, String), ApiError> {
        use crate::persistence::{plan_dir, parse_slug};

        let tasks_dir = plan_dir(repo_root, branch, plan_id, plan_name).join("tasks");

        if !tasks_dir.exists() {
            return Err(ApiError::NotFound("task not found"));
        }

        let entries = crate::persistence::list_dir(&tasks_dir)
            .map_err(|_| ApiError::NotFound("task not found"))?;

        for entry in entries {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&format!("{}-", task_id)) {
                        let (parsed_id, parsed_name) = match parse_slug(name) {
                            (Some(id), Some(n)) => (id.to_string(), n.to_string()),
                            _ => continue,
                        };
                        return Ok((entry.path(), parsed_id, parsed_name));
                    }
                }
            }
        }

        Err(ApiError::NotFound("task not found"))
    }
    ```

  - Add required imports at the top of `middleware.rs`:
    - `use std::path::PathBuf;` (if not already present)
    - The functions use `crate::persistence::*` via inline `use` statements to avoid polluting the module-level imports
  - Ensure both functions are `pub` so they are exported via `pub use middleware::*` in `mod.rs`
  - Add rustdoc comments to both functions (as shown above)
- **Acceptance Criteria**:
  - Both functions compile without errors
  - Both functions are `pub` and exported via `mod.rs`'s `pub use middleware::*`
  - Function signatures match the original copies exactly (same parameter types, same return types)
  - Rustdoc comments are present and accurate
  - No new clippy warnings introduced

### 2. Update `src/api/plans.rs`
- **Task ID**: update-plans
- **Depends On**: extract-helpers
- **Assigned To**: dedup-builder
- **Agent**: builder
- **Actions**:
  - Remove the local `fn resolve_plan_path` definition (lines 129-148)
  - Add `resolve_plan_path` to the existing middleware import:
    - Change `use crate::api::middleware::{validate_branch_name, validate_status_transition};`
    - To `use crate::api::middleware::{resolve_plan_path, validate_branch_name, validate_status_transition};`
  - Verify all call sites still work (they should, since the function signature is identical)
  - Remove the `use std::path::PathBuf;` import if it was only needed for the removed function (check if it's still used elsewhere)
- **Acceptance Criteria**:
  - `plans.rs` compiles without errors
  - No local `resolve_plan_path` function remains
  - All handlers (`get_plan`, `update_plan`, `delete_plan`, `transition_plan_status`) still call `resolve_plan_path` correctly
  - No clippy warnings

### 3. Update `src/api/tasks.rs`
- **Task ID**: update-tasks
- **Depends On**: extract-helpers
- **Assigned To**: dedup-builder
- **Agent**: builder
- **Actions**:
  - Remove the local `fn resolve_plan_path` definition (lines 178-197)
  - Remove the local `fn resolve_task_path` definition (lines 85-116)
  - Add both helpers to the existing middleware import:
    - Change `use crate::api::middleware::validate_status_transition;`
    - To `use crate::api::middleware::{resolve_plan_path, resolve_task_path, validate_status_transition};`
  - Verify all call sites still work:
    - `resolve_plan_path` is called by: `list_tasks`, `get_task`, `create_task`, `update_task`, `delete_task`, `transition_task_status`, `claim_task`
    - `resolve_task_path` is called by: `get_task`, `update_task`, `delete_task`, `transition_task_status`, `claim_task`
  - Remove the `use std::path::PathBuf;` import if it was only needed for the removed functions
- **Acceptance Criteria**:
  - `tasks.rs` compiles without errors
  - No local `resolve_plan_path` or `resolve_task_path` functions remain
  - All handlers still call the shared helpers correctly
  - No clippy warnings

### 4. Update `src/api/execution.rs`
- **Task ID**: update-execution
- **Depends On**: extract-helpers
- **Assigned To**: dedup-builder
- **Agent**: builder
- **Actions**:
  - Remove the local `fn resolve_plan_path` definition (lines 51-70)
  - Remove the local `fn resolve_task_path` definition (lines 109-140)
  - Add both helpers to the imports:
    - Add `use crate::api::middleware::{resolve_plan_path, resolve_task_path};` (no existing middleware import to extend)
  - Verify all call sites still work:
    - `resolve_plan_path` is called by: `get_execution_state`
    - `resolve_task_path` is called by: `list_running_tasks`
  - Remove any unused imports (e.g., `use std::path::PathBuf;` if no longer needed)
- **Acceptance Criteria**:
  - `execution.rs` compiles without errors
  - No local `resolve_plan_path` or `resolve_task_path` functions remain
  - All handlers still call the shared helpers correctly
  - No clippy warnings

### 5. Final Validation
- **Task ID**: validate-all
- **Depends On**: update-plans, update-tasks, update-execution
- **Assigned To**: dedup-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum` — all tests must pass
  - Run `cargo clippy --package nexum` — no warnings in api module
  - Verify `resolve_plan_path` appears in exactly one file: `src/api/middleware.rs`
  - Verify `resolve_task_path` appears in exactly one file: `src/api/middleware.rs`
  - Verify no `fn resolve_plan_path` or `fn resolve_task_path` definitions remain in `plans.rs`, `tasks.rs`, or `execution.rs`
  - Verify `mod.rs` re-exports both functions via `pub use middleware::*`
  - Verify each consuming module imports from `crate::api::middleware`
  - Count total lines removed vs added (expect net reduction of ~80-90 lines of duplicated code)

## Acceptance Criteria
- `cargo check` succeeds with no errors
- `cargo build` succeeds
- `cargo test --package nexum` passes all tests
- `cargo clippy --package nexum` produces no warnings in the api module
- `grep -rn "fn resolve_plan_path" src/api/` returns exactly one match in `middleware.rs`
- `grep -rn "fn resolve_task_path" src/api/` returns exactly one match in `middleware.rs`
- No `resolve_plan_path` or `resolve_task_path` definitions remain in `plans.rs`, `tasks.rs`, or `execution.rs`
- All three modules import the helpers from `crate::api::middleware`
- `mod.rs` re-exports both functions via `pub use middleware::*`
- Net reduction of duplicated code (approximately 80-90 lines removed)
- No behavioral changes to any API endpoint

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build the project
- `cargo test --package nexum` — Run all tests
- `cargo clippy --package nexum` — Check for lint warnings
- `grep -rn "fn resolve_plan_path" src/api/` — Verify exactly one definition (in middleware.rs)
- `grep -rn "fn resolve_task_path" src/api/` — Verify exactly one definition (in middleware.rs)
- `grep -rn "resolve_plan_path" src/api/plans.rs src/api/tasks.rs src/api/execution.rs` — Verify all three modules use the shared version (call sites only, no definitions)
- `grep -rn "resolve_task_path" src/api/tasks.rs src/api/execution.rs` — Verify both modules use the shared version (call sites only, no definitions)
- `wc -l src/api/plans.rs src/api/tasks.rs src/api/execution.rs` — Verify net line count reduction

## Notes
- This is a pure refactoring task — no new behavior, no new endpoints, no API contract changes.
- The canonical function implementations are chosen from the original owner modules: `plans.rs` for `resolve_plan_path` and `tasks.rs` for `resolve_task_path`.
- `middleware.rs` is the chosen home because it already contains shared API utilities (validation functions, request ID middleware) and is already re-exported via `pub use middleware::*` in `mod.rs`.
- The `use crate::persistence::*` imports inside the helper functions use inline `use` statements to avoid polluting the middleware module's top-level imports with persistence types that are otherwise unused there.
- All three consuming modules already import from `crate::api::middleware`, so the import pattern is established.
- This task is a sub-task of chunk 007 (REST API) and should be executed as 007.1 after the initial REST API implementation is complete.
