# Plan: 005.3 - Fix circular dependency error type in determine_merge_order

## Task Description
Fix the `determine_merge_order` function in `src/git/merge.rs` so that it returns a semantically correct error type when a circular dependency is detected in the task dependency graph. Currently, it returns `GitError::SubprocessFailure` with fabricated fields (exit_code: -1, empty stdout, fake stderr message), which is misleading since no subprocess was involved.

## Objective
Add a dedicated `GitError::CircularDependency` variant and use it in `determine_merge_order` when Kahn's algorithm detects a cycle in the dependency graph. This ensures callers can correctly distinguish between actual git subprocess failures and logical dependency errors.

## Problem Statement
The `determine_merge_order` function (lines 305-361 in `src/git/merge.rs`) implements Kahn's algorithm for topological sorting of pending tasks. When the algorithm detects a cycle (i.e., `result.len() != pending.len()`), it returns:

```rust
Err(GitError::SubprocessFailure {
    command: "determine_merge_order".to_string(),
    exit_code: -1,
    stdout: String::new(),
    stderr: "Circular dependency detected in merge order".to_string(),
})
```

This is semantically wrong for several reasons:
1. **No subprocess was invoked** — the cycle detection is a pure Rust computation on the dependency graph
2. **Fabricated fields** — `exit_code: -1` is not a valid git exit code; stdout and stderr are empty/fake
3. **Misleading to callers** — any code matching on `SubprocessFailure` will misinterpret this as a git command failure, potentially triggering retries or inappropriate error messages
4. **Lost diagnostic information** — the `SubprocessFailure` variant carries no information about which tasks form the cycle

## Solution Approach
1. Add a new `CircularDependency { tasks: Vec<String> }` variant to `GitError` in `src/git/errors.rs`
2. Update `determine_merge_order` to return `GitError::CircularDependency` with the tasks that could not be placed in the topological order (i.e., those still in the cycle)
3. Update the doc comment on `determine_merge_order` to reflect the new error type
4. Add a unit test for circular dependency detection

### Identifying the cycled tasks
The tasks involved in the cycle are those present in `pending` but absent from `result`. These can be computed by iterating over `pending` and filtering out tasks that appear in `result`.

## Relevant Files
- `src/git/errors.rs` — Add new `CircularDependency` variant to `GitError` enum
- `src/git/merge.rs` — Update `determine_merge_order` to return the new error type; update doc comment
- `src/git/tests.rs` — Add test for circular dependency detection

### New Files (if needed)
None. All changes are to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: git-error-builder
  - Role: Implement the new error variant and update the merge function
  - Agent: builder

- **Validator**
  - Name: git-error-validator
  - Role: Verify implementation meets criteria, run tests
  - Agent: validator

- **Documenter**
  - Name: git-error-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Add CircularDependency variant to GitError
- **Task ID**: add-circular-dependency-variant
- **Depends On**: none
- **Assigned To**: git-error-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/errors.rs`
  - Add a new variant after `DependencyNotMet` (line ~92):
    ```rust
    /// Circular dependency detected in the task dependency graph.
    #[error("circular dependency among tasks: {tasks:?}")]
    CircularDependency {
        tasks: Vec<String>,
    },
    ```
  - Place it between `DependencyNotMet` and `SpawnFailed` to keep dependency-related errors grouped together
- **Acceptance Criteria**:
  - The `CircularDependency` variant compiles without errors
  - The `#[error(...)]` message clearly indicates the cycled tasks
  - The variant is placed logically among other dependency-related errors

### 2. Update determine_merge_order to use CircularDependency
- **Task ID**: update-determine-merge-order
- **Depends On**: add-circular-dependency-variant
- **Assigned To**: git-error-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/merge.rs`
  - Replace lines 351-358 (the `if result.len() != pending.len()` block) with:
    ```rust
    if result.len() != pending.len() {
        let cycled: Vec<String> = plan.pending_tasks
            .iter()
            .filter(|t| !result.contains(t.as_str()))
            .cloned()
            .collect();
        return Err(GitError::CircularDependency { tasks: cycled });
    }
    ```
  - Update the doc comment on line 304 from:
    `/// Returns \`GitError::SubprocessFailure\` if a circular dependency is detected.`
    to:
    `/// Returns \`GitError::CircularDependency\` if a circular dependency is detected.`
- **Acceptance Criteria**:
  - The function returns `GitError::CircularDependency` with the correct set of cycled tasks
  - The doc comment accurately describes the error type
  - No other `SubprocessFailure` usages in the file are affected

### 3. Add unit test for circular dependency detection
- **Task ID**: add-circular-dependency-test
- **Depends On**: update-determine-merge-order
- **Assigned To**: git-error-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/tests.rs`
  - Add a new test function after `test_determine_merge_order`:
    ```rust
    #[test]
    fn test_determine_merge_order_circular_dependency() {
        let mut deps = std::collections::HashMap::new();
        deps.insert("TASK-001".to_string(), vec!["TASK-002".to_string()]);
        deps.insert("TASK-002".to_string(), vec!["TASK-001".to_string()]);

        let plan = MergePlan {
            repo_root: PathBuf::from("/tmp"),
            plan_branch: "feature/test".to_string(),
            pending_tasks: vec![
                "TASK-001".to_string(),
                "TASK-002".to_string(),
            ],
            merged_tasks: vec![],
            dependencies: deps,
        };

        let result = determine_merge_order(&plan);
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::CircularDependency { tasks } => {
                assert!(tasks.contains(&"TASK-001".to_string()));
                assert!(tasks.contains(&"TASK-002".to_string()));
            }
            other => panic!("Expected CircularDependency, got: {:?}", other),
        }
    }
    ```
  - Ensure `GitError` is imported in the test module (it already is via the wildcard import on line 12)
- **Acceptance Criteria**:
  - Test compiles and passes
  - Test verifies both the error type AND the cycled task IDs
  - Test is placed logically alongside `test_determine_merge_order`

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-circular-dependency-test
- **Assigned To**: git-error-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — confirm no compilation errors
  - Run `cargo test` — confirm all tests pass including the new circular dependency test
  - Verify `GitError::SubprocessFailure` is no longer used in `determine_merge_order`
  - Verify no other callers of `determine_merge_order` break due to the new error variant
  - Verify doc comments are accurate

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: git-error-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `GitError::CircularDependency` variant exists with `tasks: Vec<String>` field
- `determine_merge_order` returns `CircularDependency` (not `SubprocessFailure`) on cycle detection
- The `tasks` field contains the correct set of tasks involved in the cycle
- Doc comment on `determine_merge_order` references the correct error type
- A unit test covers circular dependency detection with explicit error-type assertion
- All existing tests continue to pass
- `cargo check` and `cargo test` succeed with no warnings

## Validation Commands
- `cargo check` — Verify compilation
- `cargo test` — Run full test suite
- `cargo test test_determine_merge_order_circular_dependency` — Run the specific new test
- `cargo clippy` — Check for lint warnings

## Notes
- No breaking changes for callers: adding a new enum variant is backward compatible in Rust since pattern matches using `_` or non-exhaustive matching still compile. Callers that match on `SubprocessFailure` specifically were not handling the circular dependency case correctly anyway (since the fabricated fields were misleading).
- The `CircularDependency` variant is placed between `DependencyNotMet` and `SpawnFailed` to keep dependency-related errors grouped together in the enum.
- The cycled tasks are identified as those in `plan.pending_tasks` that are not present in the topological sort `result` — these are precisely the tasks trapped in cycles.
