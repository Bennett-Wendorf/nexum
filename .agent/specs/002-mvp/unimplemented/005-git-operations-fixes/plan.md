# Plan: 005.1 - Fix merge conflict cleanup

## Task Description
Fix the `merge_branch` and `merge_task_branch` functions in `src/git/merge.rs` so that when a merge conflict is detected, the repository state is properly restored (merge aborted and original branch checked out) before returning the error. Currently, both functions return a `MergeConflict` error but leave the repository in a conflicted state with unresolved conflicts.

## Objective
Ensure that merge conflict detection paths in `merge_branch` and `merge_task_branch` properly clean up the repository state by aborting the in-progress merge and restoring the original branch before returning the `MergeConflict` error.

## Problem Statement
When `git merge --no-ff` produces conflicts (exit code 1), both `merge_branch` and `merge_task_branch` detect the conflicts and return a `GitError::MergeConflict` error. However, they do NOT:
1. Abort the in-progress merge (via `abort_merge`)
2. Restore the original branch (via `checkout_branch`)

This leaves the repository checked out to the target/plan branch with unresolved conflicts, potentially causing issues for subsequent git operations.

## Solution Approach
In both functions, on the conflict path (the `if !conflicted_files.is_empty()` branch), add cleanup steps before returning the error:
1. Call `abort_merge(repo_root).await` to abort the in-progress merge
2. Call `checkout_branch(repo_root, &original_branch).await` to restore the original branch

Both cleanup calls use `let _ = ...` to ignore errors — if cleanup fails, the primary `MergeConflict` error is still returned to the caller. This matches the pattern already used in the other error paths of these same functions.

## Relevant Files
- `src/git/merge.rs` — Contains `merge_branch` (lines 37-103) and `merge_task_branch` (lines 204-276) that need fixes
- `src/git/tests.rs` — Contains existing merge tests; new tests to be added here
- `src/git/branch.rs` — Contains `checkout_branch` used for restoration
- `src/git/mod.rs` — Module structure (no changes needed)

### New Files (if needed)
None. All changes are to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: merge-fix-builder
  - Role: Implement the merge conflict cleanup fix in both functions and add regression tests
  - Agent: builder

- **Validator**
  - Name: merge-fix-validator
  - Role: Verify the fix works correctly and tests pass
  - Agent: validator

- **Documenter**
  - Name: merge-fix-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Fix merge_branch conflict cleanup
- **Task ID**: fix-merge-branch-cleanup
- **Depends On**: none
- **Assigned To**: merge-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/git/merge.rs`, locate the conflict path in `merge_branch` (lines 80-86, the `if !conflicted_files.is_empty()` block)
  - Before the `Err(GitError::MergeConflict {...})` return, add two cleanup lines:
    ```rust
    let _ = abort_merge(repo_root).await;
    let _ = checkout_branch(repo_root, &original_branch).await;
    ```
  - Update the comment from `// Step 5a: Conflicts detected` to `// Step 5a: Conflicts detected — abort merge and restore original branch`
- **Acceptance Criteria**:
  - The conflict path now calls `abort_merge` and `checkout_branch` before returning the error
  - The code compiles without errors

### 2. Fix merge_task_branch conflict cleanup
- **Task ID**: fix-merge-task-branch-cleanup
- **Depends On**: none
- **Assigned To**: merge-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/git/merge.rs`, locate the conflict path in `merge_task_branch` (lines 256-261, the `if !conflicted_files.is_empty()` block)
  - Before the `Err(GitError::MergeConflict {...})` return, add two cleanup lines:
    ```rust
    let _ = abort_merge(repo_root).await;
    let _ = checkout_branch(repo_root, &original_branch).await;
    ```
  - Add a comment: `// Conflicts detected — abort merge and restore original branch`
- **Acceptance Criteria**:
  - The conflict path now calls `abort_merge` and `checkout_branch` before returning the error
  - The code compiles without errors

### 3. Add regression tests for conflict cleanup
- **Task ID**: add-conflict-cleanup-tests
- **Depends On**: fix-merge-branch-cleanup, fix-merge-task-branch-cleanup
- **Assigned To**: merge-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/git/tests.rs`, add a test `test_merge_branch_conflict_cleanup` that:
    - Creates a test repo with two branches that have conflicting changes to the same file
    - Calls `merge_branch` and expects a `MergeConflict` error
    - Verifies the repo is NOT in a merging state (`is_merging` returns false)
    - Verifies the current branch is restored to the original branch
  - In `src/git/tests.rs`, add a test `test_merge_task_branch_conflict_cleanup` that:
    - Creates a test repo with a plan branch and a task branch with conflicting changes
    - Calls `merge_task_branch` and expects a `MergeConflict` error
    - Verifies the repo is NOT in a merging state
    - Verifies the current branch is restored
- **Acceptance Criteria**:
  - Both new tests pass
  - Tests verify both merge abort and branch restoration on conflict

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: fix-merge-branch-cleanup, fix-merge-task-branch-cleanup, add-conflict-cleanup-tests
- **Assigned To**: merge-fix-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all tests pass including new conflict cleanup tests
  - Run `cargo clippy` — no warnings
  - Verify `merge_branch` conflict path calls `abort_merge` and `checkout_branch`
  - Verify `merge_task_branch` conflict path calls `abort_merge` and `checkout_branch`
  - Verify no other error paths were inadvertently modified

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: merge-fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
1. `merge_branch` aborts the merge and restores the original branch on conflict
2. `merge_task_branch` aborts the merge and restores the original branch on conflict
3. All existing tests continue to pass
4. New regression tests verify cleanup behavior on merge conflicts
5. Code compiles cleanly with no clippy warnings
6. The non-conflict error path (Step 5b in merge_branch) remains unchanged — it does NOT need cleanup since there's no merge in progress when there are no conflicts

## Validation Commands
- `cargo test` — Run full test suite
- `cargo test git::tests::tests::test_merge_branch_conflict_cleanup` — Run specific conflict cleanup test for merge_branch
- `cargo test git::tests::tests::test_merge_task_branch_conflict_cleanup` — Run specific conflict cleanup test for merge_task_branch
- `cargo clippy -- -D warnings` — Check for clippy warnings
- `cargo fmt -- --check` — Check code formatting

## Notes
- The `abort_merge` function already handles the case where no merge is in progress (returns `Ok(())` as a no-op), so calling it is safe even if the merge state is unclear.
- The cleanup uses `let _ = ...` to ignore errors, consistent with the existing error-handling pattern in the `Err(e)` fallback paths of both functions (lines 98-101 and 271-274).
- The non-conflict failure path (Step 5b) does NOT need cleanup because `git merge` with exit code 1 and no conflicts means the merge did not start (no MERGE_HEAD file), so there's nothing to abort.
- The `execute_merge_sequence` function calls `merge_task_branch` and propagates errors, so fixing `merge_task_branch` automatically fixes the sequence-level behavior.
