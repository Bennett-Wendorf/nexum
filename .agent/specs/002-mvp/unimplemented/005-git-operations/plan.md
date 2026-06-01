# Plan: 005 - Git Operations

## Task Description
Implement the complete Git operations module for nexum. This module handles all git interactions via subprocess calls to the system git binary: branch creation and deletion, worktree spawn and cleanup, task-branch-to-plan-branch merging, merge conflict detection and handling, and dependency-ordered merge coordination. It provides a typed Rust API built on `tokio::process` for executing git commands, parsing their output, and handling errors — all without using a git library.

## Objective
Create a fully functional `src/git/` module that:
- Wraps `tokio::process::Command` for reliable, async git subprocess execution
- Creates plan branches (e.g., `feature/auth-overhaul`) from a given base branch
- Creates ephemeral task branches (e.g., `task/TASK-001`) for each task
- Spawns isolated git worktrees per task (via `git worktree add`)
- Removes worktrees and deletes task branches after merge completion
- Merges task branches into the plan branch with conflict detection
- Enforces dependency-ordered merge (TASK-003 waits for TASK-001 and TASK-002)
- Never pushes task branches to remote (local-only, ephemeral)
- Provides a clean public API for the Overlord to invoke during task lifecycle management

## Problem Statement
Nexum's branch strategy (defined in `design/persistence.md`) requires sophisticated git operations to enable parallel task execution. Each task needs its own isolated worktree so multiple builder agents can work simultaneously without interfering with each other. When a task completes, its changes must be merged into the plan branch in dependency order — TASK-003 cannot merge until TASK-001 and TASK-002 have been merged. After merge, the task branch and worktree must be cleaned up to avoid resource leaks. All of this must be done via subprocess calls to the system git binary (not a git library), with robust error handling for merge conflicts, missing branches, stale worktrees, and other edge cases. Without this module, the Overlord cannot orchestrate the parallel task execution workflow that is central to nexum's architecture.

## Solution Approach
Build a `git` module in `src/git/` with the following submodules:

1. **`subprocess.rs`** — Git subprocess wrapper. Encapsulates `tokio::process::Command` for executing git commands. Provides `GitCommand` builder pattern for constructing commands, executing them, and parsing output. Handles stdout/stderr capture, exit code validation, and error parsing from git's output. Supports optional timeout, working directory override, and environment variable injection.

2. **`branch.rs`** — Branch operations. Functions for creating branches (`git branch`, `git checkout -b`), deleting branches (`git branch -D`), listing branches (`git branch`), checking out branches (`git checkout`), and determining the current branch. All operations target the repo root path.

3. **`worktree.rs`** — Worktree lifecycle management. Functions for spawning worktrees (`git worktree add <path> <branch>`), removing worktrees (`git worktree remove <path>`), listing worktrees (`git worktree list`), and validating worktree existence. Each worktree is created in a dedicated subdirectory under the repo (e.g., `.worktrees/TASK-001/`) for easy tracking and cleanup.

4. **`merge.rs`** — Merge operations and conflict handling. Functions for merging a task branch into the plan branch (`git merge`), detecting merge conflicts (exit code 1 with conflict markers), and providing conflict resolution strategies. Supports `--no-ff` merges to always create a merge commit for auditability. Dependency-ordered merge coordination: checks that all dependency tasks have been merged before allowing a merge.

5. **`errors.rs`** — Custom error type `GitError` using `thiserror` with variants for subprocess failures, merge conflicts, missing branches, stale worktrees, and git output parse errors.

6. **`tests.rs`** — Comprehensive unit and integration tests using temporary git repositories.

The module follows these design principles from `design/persistence.md` and `design/tech-stack.md`:
- Subprocess calls to system git binary (reference implementation, no library)
- `tokio::process` for async subprocess execution
- Task branches are local-only, never pushed to remote
- Worktree isolation prevents cross-talk between parallel tasks
- Dependency graph dictates merge order
- Cleanup after merge: delete branch, remove worktree

## Relevant Files

### Existing Files
- `design/persistence.md` — Branch strategy section (section "4. Branch strategy")
- `design/tech-stack.md` — Git operations via `tokio::process` subprocess
- `design/work-statuses.md` — Task lifecycle including `merge-queue` and `completed` statuses
- `design/unit-of-work.md` — Dependency chaining rules
- `design/agent-roles.md` — Overlord responsibilities including merge orchestration
- `design/implementation-chunks.md` — Defines this as chunk 5 of the MVP
- `Cargo.toml` — Will contain `tokio` with `process` feature (added by chunk 001)
- `src/git/mod.rs` — Module stub (created by chunk 001)
- `src/persistence/` — Persistence layer module (completed by chunk 003) for reading task dependencies

### New Files (if needed)
- `src/git/mod.rs` — Module root, re-exports public types and functions
- `src/git/subprocess.rs` — Git subprocess wrapper using tokio::process
- `src/git/branch.rs` — Branch creation, deletion, checkout, listing
- `src/git/worktree.rs` — Worktree spawn, removal, listing, validation
- `src/git/merge.rs` — Merge operations, conflict detection, dependency-ordered merge
- `src/git/errors.rs` — GitError enum with thiserror
- `src/git/tests.rs` — Unit and integration tests

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: git-core-builder
  - Role: Implement subprocess wrapper, branch operations, worktree lifecycle, and error types
  - Agent: builder

- **Builder**
  - Name: git-merge-builder
  - Role: Implement merge operations, conflict detection, and dependency-ordered merge coordination
  - Agent: builder

- **Builder**
  - Name: git-tests-builder
  - Role: Write comprehensive unit and integration tests for all git modules
  - Agent: builder

- **Validator**
  - Name: git-validator
  - Role: Verify subprocess execution, branch/worktree lifecycle, merge correctness, and cleanup behavior
  - Agent: validator

- **Documenter**
  - Name: git-documenter
  - Role: Generate documentation for completed git operations module
  - Agent: documenter

## Step by Step Tasks

### 1. Implement Git Subprocess Wrapper
- **Task ID**: git-subprocess-wrapper
- **Depends On**: none
- **Assigned To**: git-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/subprocess.rs` with:

    **GitCommand builder**:
    - `pub struct GitCommand` — Builder for constructing git commands
      - `fn new(repo_root: &Path) -> Self` — Create with repo root (sets cwd)
      - `fn args(mut self, args: &[&str]) -> Self` — Add command arguments
      - `fn arg(mut self, arg: &str) -> Self` — Add single argument
      - `fn timeout(mut self, dur: Duration) -> Self` — Set execution timeout (optional)
      - `fn env(mut self, key: &str, val: &str) -> Self` — Set environment variable
      - `fn execute(self) -> Result<GitOutput>` — Execute the command

    **GitOutput struct**:
    ```rust
    pub struct GitOutput {
        pub stdout: String,
        pub stderr: String,
        pub exit_code: i32,
        pub success: bool,
    }
    ```

    **Execution logic**:
    - Spawn `tokio::process::Command::new("git")`
    - Set `current_dir` to `repo_root`
    - Apply any environment variables
    - Capture stdout and stderr as strings
    - Set exit code validation: `success` = exit_code == 0
    - If timeout is set, use `tokio::time::timeout()` to enforce limit
    - On timeout, return `GitError::Timeout` with command details
    - On non-zero exit, return `GitError::SubprocessFailure` with stdout, stderr, exit_code

    **Helper functions**:
    - `pub fn git(repo_root: &Path, args: &[&str]) -> Result<GitOutput>` — Convenience function for simple git commands
    - `pub fn git_with_output(repo_root: &Path, args: &[&str]) -> Result<GitOutput>` — Same as above, explicit name

  - Default timeout: 30 seconds (configurable via builder)
  - All git commands are executed relative to `repo_root` (the repository root directory)
  - Document that this wrapper is the sole mechanism for git interaction — no library calls

- **Acceptance Criteria**:
  - `GitCommand` builder pattern works correctly with chained method calls
  - `execute()` spawns `tokio::process::Command::new("git")` with correct args and cwd
  - `GitOutput` captures stdout, stderr, exit_code, and success flag
  - Timeout enforcement works via `tokio::time::timeout()`
  - Non-zero exit codes produce `GitError::SubprocessFailure` with full context
  - Timeout produces `GitError::Timeout` with command details
  - `git()` convenience function works for simple invocations
  - All functions return `git::Result<T>`

### 2. Implement Git Error Types
- **Task ID**: git-errors
- **Depends On**: none
- **Assigned To**: git-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/errors.rs` with `GitError` enum using `thiserror`:
    - `SubprocessFailure { command: String, exit_code: i32, stdout: String, stderr: String }` — Git command failed with non-zero exit code
    - `Timeout { command: String, duration: Duration }` — Git command exceeded timeout
    - `MergeConflict { branch: String, plan_branch: String, conflicts: Vec<String> }` — Merge produced conflicts
    - `BranchNotFound { branch: String }` — Referenced branch does not exist
    - `BranchExists { branch: String }` — Branch already exists (unexpected)
    - `WorktreeNotFound { path: PathBuf }` — Expected worktree does not exist
    - `WorktreeExists { path: PathBuf }` — Worktree already exists at path
    - `WorktreeStale { path: PathBuf }` — Worktree is stale (lock file present)
    - `GitNotInstalled` — git binary not found in PATH
    - `ParseError { field: String, raw: String }` — Failed to parse git output (e.g., branch list)
    - `Io(PathBuf, io::Error)` — File system I/O error (for worktree path operations)
    - `DependencyNotMet { task_id: String, unmet_deps: Vec<String> }` — Dependency tasks not yet merged
  - Implement `std::fmt::Display` (provided by thiserror derive)
  - Implement `From<io::Error>` for ergonomic `?` operator usage
  - Define type alias: `pub type Result<T> = std::result::Result<T, GitError>`
  - Document each error variant with when it occurs and how to handle it

- **Acceptance Criteria**:
  - `GitError` compiles with `thiserror::Error` derive
  - All error variants include sufficient context for debugging
  - `MergeConflict` variant includes branch names and conflict file list
  - `DependencyNotMet` variant includes task ID and list of unmet dependencies
  - `From<io::Error>` implementation allows `?` operator
  - `Result<T>` type alias is defined and accessible
  - Error messages are descriptive and actionable

### 3. Implement Branch Operations
- **Task ID**: git-branch-operations
- **Depends On**: git-subprocess-wrapper, git-errors
- **Assigned To**: git-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/branch.rs` with:

    **Branch creation**:
    - `pub fn create_branch(repo_root: &Path, branch_name: &str, from_branch: &str) -> Result<()>` — Create a new branch from an existing branch:
      1. Run `git branch <branch_name> <from_branch>`
      2. Validate success
    - `pub fn create_plan_branch(repo_root: &Path, branch_name: &str, from_branch: &str) -> Result<()>` — Create a plan branch (e.g., `feature/auth-overhaul`):
      1. Call `create_branch(repo_root, branch_name, from_branch)`
      2. Optionally push to remote (controlled by a flag; default: do NOT push for task branches)
    - `pub fn create_task_branch(repo_root: &Path, task_id: &str, from_branch: &str) -> Result<String>` — Create a task branch (e.g., `task/TASK-001`):
      1. Construct branch name: `task/<task_id>` (e.g., `task/TASK-001`)
      2. Call `create_branch(repo_root, branch_name, from_branch)`
      3. Return the branch name

    **Branch deletion**:
    - `pub fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()>` — Delete a branch:
      1. Run `git branch -D <branch_name>` (force delete, regardless of merge status)
      2. Handle case where branch doesn't exist gracefully (no-op)

    **Branch checkout**:
    - `pub fn checkout_branch(repo_root: &Path, branch_name: &str) -> Result<()>` — Checkout a branch:
      1. Run `git checkout <branch_name>`
      2. Validate success

    **Branch listing and queries**:
    - `pub fn list_local_branches(repo_root: &Path) -> Result<Vec<String>>` — List all local branches:
      1. Run `git branch`
      2. Parse output: each line is `<current> branch_name` or `  branch_name`
      3. Strip whitespace and `*` prefix, return list of branch names
    - `pub fn branch_exists(repo_root: &Path, branch_name: &str) -> Result<bool>` — Check if a branch exists:
      1. Run `git branch --list <branch_name>`
      2. If output is non-empty, branch exists
    - `pub fn current_branch(repo_root: &Path) -> Result<String>` — Get current branch name:
      1. Run `git rev-parse --abbrev-ref HEAD`
      2. Parse output (trim whitespace)

    **Branch naming helpers**:
    - `pub fn task_branch_name(task_id: &str) -> String` — Generate task branch name: `task/<task_id>`
      - Example: `task_branch_name("TASK-001")` → `"task/TASK-001"`
    - `pub fn is_task_branch(branch_name: &str) -> bool` — Check if branch name matches task branch pattern
      - Returns true if branch starts with `task/`

  - All functions use the `git()` helper from subprocess module
  - Parse git output carefully (handle whitespace, asterisks, etc.)
  - Task branches are always prefixed with `task/` namespace

- **Acceptance Criteria**:
  - `create_branch()` creates a new branch from the specified parent branch
  - `create_task_branch()` returns branch name in `task/<task_id>` format
  - `delete_branch()` force-deletes a branch (handles unmerged state)
  - `checkout_branch()` checks out the specified branch
  - `list_local_branches()` returns all local branch names without asterisk prefixes
  - `branch_exists()` correctly reports branch existence
  - `current_branch()` returns the current branch name
  - `task_branch_name("TASK-001")` returns `"task/TASK-001"`
  - `is_task_branch("task/TASK-001")` returns true
  - `is_task_branch("feature/auth")` returns false
  - All functions return appropriate errors on failure

### 4. Implement Worktree Lifecycle Management
- **Task ID**: git-worktree-operations
- **Depends On**: git-subprocess-wrapper, git-errors, git-branch-operations
- **Assigned To**: git-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/worktree.rs` with:

    **Worktree path resolution**:
    - `pub fn worktree_path(repo_root: &Path, task_id: &str) -> PathBuf` — Compute worktree path:
      - Returns `<repo_root>/.worktrees/<task_id>/`
      - Example: `worktree_path("/repo", "TASK-001")` → `/repo/.worktrees/TASK-001/`
    - `pub fn worktrees_dir(repo_root: &Path) -> PathBuf` — Returns `<repo_root>/.worktrees/`

    **Worktree spawn**:
    - `pub fn spawn_worktree(repo_root: &Path, task_id: &str, branch_name: &str) -> Result<PathBuf>` — Spawn a new worktree:
      1. Compute worktree path via `worktree_path()`
      2. Ensure parent directory exists (`fs::create_dir_all`)
      3. Run `git worktree add <worktree_path> <branch_name>`
      4. Validate success
      5. Return the worktree path
      6. On failure: clean up any partial directory creation
    - Document that this creates an isolated filesystem copy sharing the same `.git` data

    **Worktree removal**:
    - `pub fn remove_worktree(repo_root: &Path, task_id: &str) -> Result<()>` — Remove a worktree:
      1. Compute worktree path via `worktree_path()`
      2. Run `git worktree remove <worktree_path>`
      3. If worktree is stale (lock file), use `git worktree remove --force <worktree_path>`
      4. Clean up any remaining directory
    - `pub fn remove_worktree_force(repo_root: &Path, task_id: &str) -> Result<()>` — Force remove a stale worktree:
      1. Compute worktree path
      2. Run `git worktree remove --force <worktree_path>`
      3. If that fails, fall back to `fs::remove_dir_all()`
      4. Clean up any remaining directory

    **Worktree listing and queries**:
    - `pub fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeInfo>>` — List all worktrees:
      1. Run `git worktree list --porcelain` or `git worktree list`
      2. Parse output to extract path and branch for each worktree
      3. Return list of `WorktreeInfo` structs
    - `pub fn worktree_exists(repo_root: &Path, task_id: &str) -> Result<bool>` — Check if worktree exists:
      1. Compute worktree path
      2. Check if directory exists AND contains `.git` file/pointer

    **WorktreeInfo struct**:
    ```rust
    pub struct WorktreeInfo {
        pub path: PathBuf,
        pub branch: String,
        pub is_bare: bool,
    }
    ```

  - All git worktree commands use the `git()` helper from subprocess module
  - Worktree paths use the `.worktrees/` directory under repo root
  - Handle stale worktrees (lock files) gracefully with force removal option
  - Document that worktrees share the same git object database (efficient, no duplication)

- **Acceptance Criteria**:
  - `worktree_path()` returns correct `<repo_root>/.worktrees/<task_id>/` path
  - `spawn_worktree()` creates a new worktree with the specified branch
  - `spawn_worktree()` creates parent directories if needed
  - `spawn_worktree()` cleans up partial state on failure
  - `remove_worktree()` removes the worktree via git worktree remove
  - `remove_worktree()` handles stale worktrees by falling back to force removal
  - `remove_worktree_force()` force-removes stale worktrees
  - `list_worktrees()` returns all active worktrees with path and branch info
  - `worktree_exists()` correctly reports worktree existence
  - `WorktreeInfo` struct contains path, branch, and is_bare fields
  - All functions return appropriate errors on failure

### 5. Implement Merge Operations
- **Task ID**: git-merge-operations
- **Depends On**: git-subprocess-wrapper, git-errors, git-branch-operations
- **Assigned To**: git-merge-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/merge.rs` with:

    **Basic merge**:
    - `pub fn merge_branch(repo_root: &Path, source_branch: &str, target_branch: &str) -> Result<GitOutput>` — Merge source into target:
      1. Checkout target branch: `checkout_branch(repo_root, target_branch)`
      2. Run `git merge --no-ff <source_branch>`
        - `--no-ff` forces a merge commit for auditability
      3. If exit code is 0, merge succeeded — return output
      4. If exit code is 1, check for merge conflicts:
        a. Run `git diff --name-only --diff-filter=U` to list conflicted files
        b. If output is non-empty, return `GitError::MergeConflict` with branch names and conflict file list
        c. If output is empty, return `GitError::SubprocessFailure` with full context
      5. After successful merge, reset to original branch (leave repo in clean state)

    **Merge with abort capability**:
    - `pub fn abort_merge(repo_root: &Path) -> Result<()>` — Abort an in-progress merge:
      1. Run `git merge --abort`
      2. Handle case where no merge is in progress (no-op)

    **Conflict detection**:
    - `pub fn has_merge_conflicts(repo_root: &Path) -> Result<bool>` — Check if current branch has merge conflicts:
      1. Run `git diff --name-only --diff-filter=U`
      2. Return true if output is non-empty
    - `pub fn list_conflicted_files(repo_root: &Path) -> Result<Vec<String>>` — List files with merge conflicts:
      1. Run `git diff --name-only --diff-filter=U`
      2. Parse output into list of file paths

    **Merge status**:
    - `pub fn is_merging(repo_root: &Path) -> Result<bool>` — Check if a merge is in progress:
      1. Check for `.git/MERGE_HEAD` file existence
      2. Return true if file exists

    **Dependency-ordered merge coordination**:
    - `pub fn merge_task_branch(repo_root: &Path, task_id: &str, plan_branch: &str, merged_tasks: &[String]) -> Result<()>` — Merge a task branch into the plan branch with dependency checking:
      1. Validate the task branch exists: `branch_exists(repo_root, &task_branch_name(task_id))`
      2. Accept `merged_tasks` list (task IDs already merged into plan branch)
      3. Caller is responsible for dependency checking (Overlord provides merged_tasks list)
      4. Checkout plan branch
      5. Run `git merge --no-ff task/<task_id>`
      6. On merge success, the function completes
      7. On merge conflict, return `GitError::MergeConflict`
    - Document that the Overlord determines merge order via the dependency graph; this function only executes the merge

  - All merge operations leave the repo in a clean state (checkout back to original branch)
  - `--no-ff` flag ensures merge commits are always created for audit trail
  - Merge conflicts are detected via `git diff --name-only --diff-filter=U`
  - Document the merge workflow: checkout target → merge → handle conflicts → cleanup

- **Acceptance Criteria**:
  - `merge_branch()` merges source branch into target branch with `--no-ff`
  - `merge_branch()` detects merge conflicts and returns `GitError::MergeConflict`
  - `merge_branch()` includes conflicted file list in the error
  - `abort_merge()` aborts an in-progress merge
  - `has_merge_conflicts()` correctly detects conflict state
  - `list_conflicted_files()` returns list of conflicted file paths
  - `is_merging()` detects active merge state via MERGE_HEAD file
  - `merge_task_branch()` merges task branch into plan branch
  - `merge_task_branch()` accepts merged_tasks list for context
  - All merge operations leave the repo in a clean state after execution
  - Merge commits are created (not fast-forwarded) for auditability

### 6. Implement Dependency-Ordered Merge Coordination
- **Task ID**: git-dependency-merge
- **Depends On**: git-merge-operations, git-branch-operations
- **Assigned To**: git-merge-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/merge.rs` (extend existing file) with:

    **Merge orchestration types**:
    ```rust
    pub struct MergePlan {
        pub repo_root: PathBuf,
        pub plan_branch: String,
        pub pending_tasks: Vec<String>,    // Task IDs waiting to merge
        pub merged_tasks: Vec<String>,     // Task IDs already merged
        pub dependencies: HashMap<String, Vec<String>>, // task_id -> [dependency task IDs]
    }
    ```

    **Merge orchestration functions**:
    - `pub fn determine_merge_order(plan: &MergePlan) -> Result<Vec<String>>` — Compute dependency-ordered merge sequence:
      1. Build a dependency graph from `plan.dependencies`
      2. Filter to only pending tasks (not yet merged)
      3. Topological sort respecting dependency order
      4. Return ordered list of task IDs to merge
      5. If circular dependency detected, return error
    - `pub fn next_mergeable_tasks(plan: &MergePlan) -> Result<Vec<String>>` — Determine which tasks can merge next:
      1. For each pending task, check if ALL dependencies are in `merged_tasks`
      2. Return list of tasks whose dependencies are satisfied
      3. These tasks can be merged in parallel
    - `pub fn execute_merge_sequence(plan: &mut MergePlan) -> Result<Vec<String>>` — Execute merges in dependency order:
      1. Call `next_mergeable_tasks()` to find tasks ready to merge
      2. For each ready task:
        a. Call `merge_task_branch()` to merge into plan branch
        b. On success, move task ID from `pending_tasks` to `merged_tasks`
        c. On merge conflict, return error with task ID and conflict details
      3. Repeat until no more tasks are mergeable or all are merged
      4. Return list of successfully merged task IDs
    - `pub fn cleanup_merged_task(repo_root: &Path, task_id: &str) -> Result<()>` — Cleanup after successful merge:
      1. Delete task branch: `delete_branch(repo_root, &task_branch_name(task_id))`
      2. Remove worktree: `remove_worktree(repo_root, task_id)`
      3. On worktree removal failure, try force removal: `remove_worktree_force()`

  - Dependency graph uses a simple adjacency list (HashMap<String, Vec<String>>)
  - Topological sort uses Kahn's algorithm (BFS-based) for O(V + E) complexity
  - `next_mergeable_tasks()` enables parallel merge of independent tasks
  - `cleanup_merged_task()` is called after each successful merge to free resources
  - Document that this module provides the coordination logic; the Overlord calls these functions

- **Acceptance Criteria**:
  - `MergePlan` struct contains all necessary fields for merge coordination
  - `determine_merge_order()` returns topologically sorted task IDs
  - `determine_merge_order()` detects circular dependencies and returns error
  - `next_mergeable_tasks()` returns tasks whose dependencies are all satisfied
  - `next_mergeable_tasks()` returns empty list when no tasks are ready
  - `execute_merge_sequence()` merges tasks in dependency order
  - `execute_merge_sequence()` handles merge conflicts by returning error
  - `execute_merge_sequence()` updates merged_tasks list after each merge
  - `cleanup_merged_task()` deletes task branch and removes worktree
  - `cleanup_merged_task()` falls back to force worktree removal if needed
  - Parallel merge of independent tasks is supported via `next_mergeable_tasks()`

### 7. Implement Plan Branch Creation Workflow
- **Task ID**: git-plan-branch-workflow
- **Depends On**: git-branch-operations, git-worktree-operations
- **Assigned To**: git-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/branch.rs` (extend existing file) with:

    **Plan branch creation**:
    - `pub fn create_plan_branch(repo_root: &Path, plan_branch: &str, from_branch: &str, push_to_remote: bool) -> Result<()>` — Create a new plan branch:
      1. Check if branch already exists: `branch_exists(repo_root, plan_branch)`
      2. If exists, return `GitError::BranchExists`
      3. Create branch: `create_branch(repo_root, plan_branch, from_branch)`
      4. If `push_to_remote` is true, run `git push -u origin <plan_branch>`
      5. If push fails, return error (but branch exists locally)

    **Task workspace setup**:
    - `pub fn setup_task_workspace(repo_root: &Path, task_id: &str, plan_branch: &str) -> Result<PathBuf>` — Set up complete task workspace:
      1. Create task branch: `create_task_branch(repo_root, task_id, plan_branch)` → returns branch name
      2. Spawn worktree: `spawn_worktree(repo_root, task_id, branch_name)` → returns worktree path
      3. Return worktree path
      4. On any failure, clean up partial state (delete branch if worktree failed, etc.)

    **Task workspace teardown**:
    - `pub fn teardown_task_workspace(repo_root: &Path, task_id: &str) -> Result<()>` — Tear down task workspace:
      1. Remove worktree: `remove_worktree(repo_root, task_id)`
      2. On failure, try force removal: `remove_worktree_force(repo_root, task_id)`
      3. Delete task branch: `delete_branch(repo_root, &task_branch_name(task_id))`
      4. On branch delete failure (branch already gone), continue (no-op)

  - `setup_task_workspace()` is the primary entry point for creating a task execution environment
  - `teardown_task_workspace()` is the primary entry point for cleanup after task completion
  - Both functions handle partial failure gracefully (clean up what was created)
  - Document the complete lifecycle: setup → execute → merge → teardown

- **Acceptance Criteria**:
  - `create_plan_branch()` creates a new plan branch from the specified base
  - `create_plan_branch()` optionally pushes to remote when flag is true
  - `create_plan_branch()` returns error if branch already exists
  - `setup_task_workspace()` creates both task branch and worktree
  - `setup_task_workspace()` returns the worktree path
  - `setup_task_workspace()` cleans up partial state on failure
  - `teardown_task_workspace()` removes worktree and deletes branch
  - `teardown_task_workspace()` falls back to force removal for stale worktrees
  - `teardown_task_workspace()` handles missing branch gracefully (no-op)

### 8. Wire Up Module Exports
- **Task ID**: git-module-wiring
- **Depends On**: git-subprocess-wrapper, git-errors, git-branch-operations, git-worktree-operations, git-merge-operations, git-dependency-merge, git-plan-branch-workflow
- **Assigned To**: git-core-builder
- **Agent**: builder
- **Actions**:
  - Update `src/git/mod.rs` to:
    - Declare submodules: `mod errors; mod subprocess; mod branch; mod worktree; mod merge;`
    - Re-export public types:
      - `pub use errors::*;` (GitError, Result)
      - `pub use subprocess::*;` (GitCommand, GitOutput)
      - `pub use worktree::*;` (WorktreeInfo)
      - `pub use merge::*;` (MergePlan)
    - Re-export public functions organized by category:
      - Branch operations: `create_branch`, `delete_branch`, `checkout_branch`, `list_local_branches`, `branch_exists`, `current_branch`, `task_branch_name`, `is_task_branch`, `create_plan_branch`, `create_task_branch`
      - Worktree operations: `spawn_worktree`, `remove_worktree`, `remove_worktree_force`, `list_worktrees`, `worktree_exists`, `worktree_path`, `worktrees_dir`
      - Merge operations: `merge_branch`, `merge_task_branch`, `abort_merge`, `has_merge_conflicts`, `list_conflicted_files`, `is_merging`
      - Coordination: `determine_merge_order`, `next_mergeable_tasks`, `execute_merge_sequence`, `cleanup_merged_task`
      - Workspace: `setup_task_workspace`, `teardown_task_workspace`
      - Subprocess: `git`, `git_with_output`
    - Include module-level documentation referencing `design/persistence.md` branch strategy section
  - Add `mod git;` to `src/main.rs` (if not already present from scaffolding)

- **Acceptance Criteria**:
  - All public types are accessible as `nexum::git::GitError`, `nexum::git::GitOutput`, etc.
  - All public functions are accessible via the git module
  - Module compiles without errors
  - Module documentation references design docs
  - Re-exports are organized logically by category

### 9. Write Tests
- **Task ID**: git-tests
- **Depends On**: git-module-wiring
- **Assigned To**: git-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `src/git/tests.rs` with comprehensive tests:

    **Test infrastructure**:
    - `fn create_test_repo() -> (tempfile::TempDir, PathBuf)` — Create a temporary git repository:
      1. Create temp directory
      2. Run `git init` in the directory
      3. Configure `git config user.email` and `git config user.name`
      4. Create an initial commit on `main` branch
      5. Return (temp_dir, repo_path)

    **Subprocess tests**:
    - `test_git_command_success` — Execute a simple git command and verify output
    - `test_git_command_failure` — Execute an invalid git command and verify error
    - `test_git_command_timeout` — Execute a command with timeout and verify timeout error
    - `test_git_convenience_function` — Test `git()` helper function

    **Branch tests**:
    - `test_create_branch` — Create a branch and verify it exists
    - `test_create_task_branch` — Create a task branch and verify naming
    - `test_delete_branch` — Create then delete a branch, verify it's gone
    - `test_branch_exists` — Test branch existence check for existing and non-existing branches
    - `test_list_local_branches` — Create branches and verify listing
    - `test_current_branch` — Checkout a branch and verify current_branch() returns it
    - `test_task_branch_name` — Verify task branch name generation
    - `test_is_task_branch` — Verify task branch detection

    **Worktree tests**:
    - `test_spawn_worktree` — Spawn a worktree and verify it exists
    - `test_worktree_path` — Verify worktree path computation
    - `test_remove_worktree` — Spawn then remove a worktree, verify cleanup
    - `test_worktree_exists` — Test worktree existence check
    - `test_list_worktrees` — Spawn worktrees and verify listing
    - `test_remove_stale_worktree` — Create a stale worktree (lock file) and verify force removal works

    **Merge tests**:
    - `test_merge_branch_success` — Merge a branch with no conflicts
    - `test_merge_branch_conflict` — Create conflicting changes and verify conflict detection
    - `test_abort_merge` — Start a merge, then abort it
    - `test_has_merge_conflicts` — Verify conflict detection
    - `test_list_conflicted_files` — Verify conflicted file listing
    - `test_is_merging` — Verify merge state detection

    **Coordination tests**:
    - `test_determine_merge_order` — Verify topological sort with simple dependency chain
    - `test_determine_merge_order_parallel` — Verify parallel tasks are correctly identified
    - `test_next_mergeable_tasks` — Verify dependency-satisfied task identification
    - `test_next_mergeable_tasks_blocked` — Verify blocked tasks are not returned
    - `test_cleanup_merged_task` — Verify branch deletion and worktree removal

    **Workspace tests**:
    - `test_setup_task_workspace` — Verify complete workspace setup
    - `test_teardown_task_workspace` — Verify complete workspace teardown
    - `test_setup_then_teardown` — Full lifecycle: setup → teardown

  - Use `tempfile` crate for isolated test repositories
  - Use `tokio::test` for async tests
  - All tests should be self-contained and not depend on external state
  - Test git commands that produce expected output patterns

- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum git`
  - Tests cover all modules: subprocess, branch, worktree, merge, coordination
  - Tests use temporary git repositories (no side effects)
  - Subprocess tests verify success, failure, and timeout scenarios
  - Branch tests verify creation, deletion, listing, and existence checks
  - Worktree tests verify spawn, removal, listing, and stale worktree handling
  - Merge tests verify success, conflict detection, and abort capability
  - Coordination tests verify dependency ordering and mergeable task identification
  - Workspace tests verify complete setup and teardown lifecycle
  - All tests are async (`#[tokio::test]`)

### 10. Final Validation
- **Task ID**: validate-all
- **Depends On**: git-tests
- **Assigned To**: git-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum git` — all git tests must pass
  - Run `cargo clippy --package nexum` — no warnings in git module
  - Verify `GitCommand` uses `tokio::process::Command::new("git")`
  - Verify git commands are executed with correct cwd (repo_root)
  - Verify timeout enforcement via `tokio::time::timeout()`
  - Verify task branch naming convention: `task/<TASK_ID>`
  - Verify worktree paths use `.worktrees/<TASK_ID>/` under repo root
  - Verify merge uses `--no-ff` flag for audit trail
  - Verify merge conflict detection via `git diff --name-only --diff-filter=U`
  - Verify merge abort capability exists
  - Verify dependency-ordered merge uses topological sort
  - Verify `setup_task_workspace()` creates both branch and worktree
  - Verify `teardown_task_workspace()` removes worktree and deletes branch
  - Verify `cleanup_merged_task()` is called after successful merge
  - Verify task branches are NEVER pushed to remote (only plan branches may be)
  - Verify `GitError` uses `thiserror` with descriptive error variants
  - Verify `Result<T>` type alias is defined
  - Verify all public types and functions are re-exported from `mod.rs`
  - Verify module documentation references `design/persistence.md` branch strategy

### 11. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: git-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the git subprocess wrapper architecture
  - Document the branch strategy (plan branches, task branches, worktree isolation)
  - Document the merge workflow (merge, conflict detection, dependency ordering)
  - Document the worktree lifecycle (spawn, execute, merge, cleanup)
  - Document the Rust API (public functions and types)
  - Document error handling and common failure modes
  - Document the dependency-ordered merge algorithm

## Acceptance Criteria
- `cargo check` succeeds with no errors in the git module
- `cargo test --package nexum git` passes all tests
- `cargo clippy --package nexum` produces no warnings in git module
- `GitCommand` builder pattern works with chained method calls
- `GitCommand::execute()` uses `tokio::process::Command::new("git")`
- Git commands execute with correct working directory (repo_root)
- Timeout enforcement works via `tokio::time::timeout()`
- Task branch naming follows `task/<TASK_ID>` convention
- Worktree paths use `.worktrees/<TASK_ID>/` under repo root
- Worktree isolation: each task gets its own isolated filesystem
- Merge uses `--no-ff` flag to create merge commits for audit trail
- Merge conflict detection via `git diff --name-only --diff-filter=U`
- Merge abort capability (`git merge --abort`) exists
- Dependency-ordered merge uses topological sort (Kahn's algorithm)
- `setup_task_workspace()` atomically creates branch + worktree
- `teardown_task_workspace()` cleans up worktree + branch
- `cleanup_merged_task()` deletes branch and removes worktree after merge
- Task branches are NEVER pushed to remote (local-only, ephemeral)
- Plan branches may optionally be pushed to remote
- `GitError` uses `thiserror` with 11+ descriptive error variants
- `Result<T>` type alias is defined for ergonomic error handling
- All public types and functions are re-exported from `mod.rs`
- Module documentation references `design/persistence.md` branch strategy section
- Stale worktree handling: force removal fallback for locked worktrees
- Merge coordination supports parallel merge of independent tasks

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum git` — Run git module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds
- `find src/git/ -type f` — Verify all module files exist
- `grep -r "tokio::process::Command" src/git/` — Verify subprocess usage
- `grep -r "git worktree add" src/git/` — Verify worktree spawn command
- `grep -r "git merge --no-ff" src/git/` — Verify merge --no-ff flag
- `grep -r "git diff --name-only --diff-filter=U" src/git/` — Verify conflict detection
- `grep -r "task/" src/git/` — Verify task branch naming convention

## Notes
- This plan depends on chunk 001 (Project Scaffolding) being completed first. The `src/git/mod.rs` stub and `Cargo.toml` with `tokio` (process feature) must exist before this plan can be executed.
- The `tempfile` crate should be added as a dev-dependency for testing (if not already present from chunk 003).
- Git subprocess commands should always be executed with the repository root as the working directory. This is critical for correct git behavior.
- The `.worktrees/` directory should be added to `.gitignore` (alongside `.agent/state/`).
- The `git` binary must be available in the system PATH. The `GitError::GitNotInstalled` variant handles the case where it's not found.
- For the topological sort in `determine_merge_order()`, use Kahn's algorithm (BFS-based) which is O(V + E) and detects cycles.
- Merge conflicts are an expected part of parallel development. The module detects them but does NOT auto-resolve them — conflict resolution is the Overlord's responsibility (potentially with LLM assistance in the future).
- The `--no-ff` flag on merges ensures every merge creates a commit, providing a clear audit trail of which task contributed which changes.
- Task branches are ephemeral by design — they exist only during task execution and are deleted after merge. This keeps the branch namespace clean.
- Consider adding a `GitContext` struct that holds the repo root path, to avoid passing it to every function. This would be a nice ergonomic improvement but is not required for MVP.
- The `spawn_worktree()` function should handle the case where the worktree directory already exists (e.g., if a previous run was interrupted). It should clean up and retry.
- For `remove_worktree_force()`, the fallback to `fs::remove_dir_all()` is important because git worktree can leave stale directories if the worktree process was killed.
- The dependency graph for merge ordering is provided by the Overlord (which reads it from `task.md` dependencies and `status.json`). This git module only executes the merge operations; it does not manage the dependency graph itself.
