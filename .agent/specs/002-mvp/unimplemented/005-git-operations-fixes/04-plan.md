# Plan: 005.4 - Fix delete_branch swallowing all errors indiscriminately

## Task Description
Fix the `delete_branch` function in `src/git/branch.rs` so that it only treats the "branch not found" case as a no-op, and propagates all other errors (git crashes, repo corruption, permissions errors, etc.) to the caller instead of silently returning `Ok(())`.

## Objective
Replace the indiscriminate error-swallowing pattern in `delete_branch` with a check-then-delete approach that uses the existing `branch_exists` function to determine whether deletion should be attempted. This ensures only the "branch doesn't exist" case is treated as a no-op, while all other failure modes propagate correctly.

## Problem Statement
The `delete_branch` function (lines 65-76 in `src/git/branch.rs`) currently uses this pattern:

```rust
pub async fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let output = git(repo_root, &["branch", "-D", branch_name]).await;
    match output {
        Ok(_) => Ok(()),
        Err(_) => {
            // If the branch doesn't exist, treat it as a no-op.
            // `git branch -D` returns exit code 1 for non-existent branches,
            // so a failure here is acceptable when the branch is absent.
            Ok(())
        }
    }
}
```

The `Err(_)` arm swallows **every** possible error:
1. **Git crash / segfault** — Returns `Ok(())` as if deletion succeeded
2. **Repository corruption** — Returns `Ok(())` despite the repo being broken
3. **Permissions error** — Returns `Ok(())` despite the operation failing
4. **Timeout** — Returns `Ok(())` despite git not completing
5. **Branch not found** — This is the only case that should be a no-op

This means callers like `teardown_task_workspace` and `cleanup_merged_task` have no way to know if the branch deletion actually failed due to a real problem.

## Solution Approach
Use the existing `branch_exists` function (which runs `git branch --list <name>`) to check if the branch exists before attempting deletion. This approach:

1. **Is safe** — No stderr parsing needed (which is fragile and git-version dependent)
2. **Reuses existing code** — `branch_exists` already handles this check correctly
3. **Is semantically clear** — The intent is obvious: "delete only if it exists"
4. **Avoids the race condition concern** — In the Nexum workflow, branches are local task branches managed by the tool itself; they are not concurrently modified by external actors

The new implementation:

```rust
pub async fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    // Check existence first — only attempt deletion if the branch exists.
    // This avoids swallowing real errors (git crash, permissions, corruption)
    // that would occur if we blindly ran `git branch -D` and ignored all failures.
    if !branch_exists(repo_root, branch_name).await? {
        return Ok(());
    }
    let _output = git(repo_root, &["branch", "-D", branch_name]).await?;
    Ok(())
}
```

## Relevant Files
- `src/git/branch.rs` — Primary fix: rewrite `delete_branch` (lines 65-76)

### New Files (if needed)
None. All changes are to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: branch-fix-builder
  - Role: Rewrite `delete_branch` to use check-then-delete pattern
  - Agent: builder

- **Validator**
  - Name: branch-fix-validator
  - Role: Verify implementation meets criteria, run tests
  - Agent: validator

- **Documenter**
  - Name: branch-fix-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Rewrite delete_branch to use check-then-delete pattern
- **Task ID**: rewrite-delete-branch
- **Depends On**: none
- **Assigned To**: branch-fix-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/branch.rs`
  - Replace the `delete_branch` function (lines 65-76) with:
    ```rust
    /// Delete a branch (force delete).
    ///
    /// Runs `git branch -D <branch_name>` to force-delete the branch regardless
    /// of its merge status. If the branch does not exist, this is a no-op.
    ///
    /// Other errors (git crash, permissions issues, repo corruption) are
    /// propagated to the caller.
    ///
    /// # Arguments
    ///
    /// * `repo_root` - The root directory of the git repository.
    /// * `branch_name` - The name of the branch to delete.
    pub async fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
        // Check existence first — only attempt deletion if the branch exists.
        // This avoids swallowing real errors (git crash, permissions, corruption)
        // that would occur if we blindly ran `git branch -D` and ignored all failures.
        if !branch_exists(repo_root, branch_name).await? {
            return Ok(());
        }
        let _output = git(repo_root, &["branch", "-D", branch_name]).await?;
        Ok(())
    }
    ```
  - Update the doc comment to clarify that non-"not-found" errors are propagated
- **Acceptance Criteria**:
  - The function compiles without errors
  - The doc comment accurately describes the new behavior (only "not found" is a no-op)
  - The implementation uses `branch_exists` for the existence check
  - All other errors from `git branch -D` propagate via `?`

### 2. Add regression test for error propagation
- **Task ID**: add-error-propagation-test
- **Depends On**: rewrite-delete-branch
- **Assigned To**: branch-fix-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/tests.rs`
  - Add a new test after `test_delete_branch` that verifies `delete_branch` propagates errors for non-"not-found" failures. Since we can't easily simulate a git crash in a unit test, the test should verify:
    1. Deleting a non-existent branch returns `Ok(())` (no-op behavior preserved)
    2. The existing `test_delete_branch` test still passes (deleting an existing branch still works)
  - Add test:
    ```rust
    #[tokio::test]
    async fn test_delete_branch_nonexistent_is_noop() {
        let (_dir, repo) = create_test_repo();
        // Deleting a branch that doesn't exist should be a no-op, not an error
        delete_branch(&repo, "does-not-exist").await.unwrap();
    }
    ```
- **Acceptance Criteria**:
  - New test compiles and passes
  - Test explicitly covers the "branch not found → no-op" path
  - Existing `test_delete_branch` test still passes

### 3. Verify downstream callers still work correctly
- **Task ID**: verify-callers
- **Depends On**: add-error-propagation-test
- **Assigned To**: branch-fix-builder
- **Agent**: builder
- **Actions**:
  - Review all callers of `delete_branch` to confirm the new behavior is compatible:
    - `src/git/branch.rs:236` — `setup_task_workspace` cleanup: uses `let _ = ...` (ignores errors anyway, compatible)
    - `src/git/branch.rs:262` — `teardown_task_workspace`: uses `?` (now correctly propagates real errors)
    - `src/git/merge.rs:418` — `cleanup_merged_task`: uses `?` (now correctly propagates real errors)
  - Confirm no callers need adjustment since the contract became "only swallow 'not found'" which is what all callers expected
- **Acceptance Criteria**:
  - All callers are verified to be compatible with the new behavior
  - No caller changes needed (the fix is a behavioral correction, not a contract change)

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: verify-callers
- **Assigned To**: branch-fix-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — confirm no compilation errors
  - Run `cargo test` — confirm all tests pass including the new no-op test
  - Run `cargo test test_delete_branch` — verify existing deletion test still works
  - Run `cargo test test_delete_branch_nonexistent_is_noop` — verify new no-op test
  - Verify `delete_branch` no longer has an `Err(_)` arm that swallows all errors
  - Verify the function uses `branch_exists` for the existence check
  - Verify `teardown_task_workspace` and `cleanup_merged_task` still compile and work correctly
  - Run `cargo clippy` — check for lint warnings

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: branch-fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `delete_branch` uses `branch_exists` to check existence before attempting deletion
- Only the "branch not found" case returns `Ok(())` as a no-op
- All other errors (git crash, permissions, timeout, corruption) propagate to the caller via `?`
- The doc comment accurately describes the new error propagation behavior
- A test covers the "non-existent branch → no-op" path
- All existing tests continue to pass
- All downstream callers (`setup_task_workspace`, `teardown_task_workspace`, `cleanup_merged_task`) remain functional
- `cargo check`, `cargo test`, and `cargo clippy` succeed with no warnings

## Validation Commands
- `cargo check` — Verify compilation
- `cargo test` — Run full test suite
- `cargo test test_delete_branch` — Run the existing deletion test
- `cargo test test_delete_branch_nonexistent_is_noop` — Run the new no-op test
- `cargo clippy` — Check for lint warnings

## Notes
- **Why not parse stderr?**: The code review suggested parsing stderr for the "not found" message. This is fragile — the exact message text varies across git versions and locales. The check-then-delete approach is cleaner and reuses existing code.
- **Race condition?**: In Nexum's workflow, task branches are local-only (never pushed to remote) and managed exclusively by the tool. No external actor modifies these branches concurrently, so the TOCTOU window between `branch_exists` and `git branch -D` is not a practical concern.
- **Performance**: The check-then-delete approach runs two git commands instead of one when the branch exists. However, `git branch --list` is extremely fast (pure filesystem lookup in the `.git/refs` directory), and this trade-off is worth the correctness improvement.
- **Backward compatibility**: This is a behavioral correction, not a breaking API change. The function signature and return type are unchanged. Callers that relied on the old behavior (where all errors were swallowed) were already getting incorrect results — this fix makes them correctly see the error.
