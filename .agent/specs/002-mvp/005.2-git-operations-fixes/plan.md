# Plan: 005.2 - Fix execute_merge_sequence state corruption on partial failure

## Task Description
Fix the `execute_merge_sequence` function in `src/git/merge.rs` so that it does not corrupt the `MergePlan` state when a merge fails midway through a batch. Currently, the function mutates `plan.pending_tasks` and `plan.merged_tasks` incrementally inside the merge loop. If a merge fails after some tasks have already been merged, the plan struct has been partially updated — tasks are moved from pending to merged even though the overall batch operation failed and returned an error.

## Objective
Ensure that `execute_merge_sequence` only updates `plan.pending_tasks` and `plan.merged_tasks` after all merges in a batch complete successfully. If any merge in the batch fails, the plan state must remain unchanged from before the batch started.

## Problem Statement
The `execute_merge_sequence` function (lines 390-410) iterates over the ready-to-merge tasks and, for each one:
1. Calls `merge_task_branch()` to perform the actual git merge
2. Immediately moves the task from `plan.pending_tasks` to `plan.merged_tasks`

If `merge_task_branch()` fails on the Nth task in a batch of M tasks (where N < M), the first N-1 tasks have already been moved from pending to merged in the plan struct. The function then returns an error, leaving the `MergePlan` in an inconsistent state:
- `pending_tasks` is missing tasks that were not actually merged successfully
- `merged_tasks` contains tasks whose merges may have partially failed
- The caller has no way to recover the original plan state

This is especially problematic because the caller may retry the operation with the corrupted plan, leading to tasks being skipped or double-merged.

## Solution Approach
Accumulate merge results into a local list without mutating the plan struct until the entire batch succeeds. The fix involves:

1. Inside the `loop`, collect all task IDs from `next_mergeable_tasks()` into a local `ready` vector
2. Run all merges using a local `let mut batch_merged = Vec::new()` accumulator
3. Only after the `for` loop completes without errors, update `plan.pending_tasks` and `plan.merged_tasks` by moving the batch results
4. If any merge fails, the `?` operator propagates the error and the plan remains unmodified

The key change is moving the `plan.pending_tasks.retain(...)` and `plan.merged_tasks.push(...)` calls from inside the `for` loop to after it.

## Relevant Files
- `src/git/merge.rs` — Contains `execute_merge_sequence` (lines 390-410) that needs the fix
- `src/git/tests.rs` — Contains existing git tests; new regression test to be added here
- `src/git/merge.rs` — `MergePlan` struct (lines 284-295) — no changes needed, but referenced by the fix

### New Files (if needed)
None. All changes are to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: merge-sequence-fix-builder
  - Role: Implement the atomic batch merge fix in `execute_merge_sequence` and add a regression test
  - Agent: builder

- **Validator**
  - Name: merge-sequence-fix-validator
  - Role: Verify the fix works correctly and all tests pass
  - Agent: validator

- **Documenter**
  - Name: merge-sequence-fix-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Refactor execute_merge_sequence for atomic batch updates
- **Task ID**: refactor-atomic-batch-merge
- **Depends On**: none
- **Assigned To**: merge-sequence-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/git/merge.rs`, locate `execute_merge_sequence` (lines 390-410)
  - Replace the current `for` loop body so that it only accumulates successfully merged task IDs into a local `batch_merged` vector:
    ```rust
    for task_id in &ready {
        merge_task_branch(&plan.repo_root, task_id, &plan.plan_branch, &plan.merged_tasks).await?;
        batch_merged.push(task_id.clone());
    }
    ```
  - After the `for` loop (but still inside the `loop` block), apply the accumulated results to the plan:
    ```rust
    // Batch succeeded — update plan state atomically
    for task_id in &batch_merged {
        plan.pending_tasks.retain(|t| t != task_id);
        plan.merged_tasks.push(task_id.clone());
        merged.push(task_id.clone());
    }
    ```
  - Declare `let mut batch_merged: Vec<String> = Vec::new();` before the `for` loop
  - Update the function's doc comment to mention the atomic batch behavior:
    ```
    /// Merges are applied atomically per batch — plan state is only updated
    /// after all merges in a batch succeed. On partial failure, the plan
    /// remains unmodified so the caller can retry.
    ```
- **Acceptance Criteria**:
  - The `for` loop no longer mutates `plan.pending_tasks` or `plan.merged_tasks`
  - Plan state is updated only after the `for` loop completes without errors
  - The function signature and return type remain unchanged
  - The code compiles without errors

### 2. Add regression test for partial failure state preservation
- **Task ID**: add-partial-failure-test
- **Depends On**: refactor-atomic-batch-merge
- **Assigned To**: merge-sequence-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/git/tests.rs`, add a test `test_execute_merge_sequence_partial_failure_preserves_state` that:
    - Creates a test repo with a plan branch and multiple task branches
    - Constructs a `MergePlan` with 3 pending tasks (e.g., TASK-001, TASK-002, TASK-003) with TASK-002 depending on TASK-001
    - Arranges for the second batch to fail (e.g., TASK-002's branch has conflicting content with the plan branch)
    - Calls `execute_merge_sequence` and expects an error
    - Verifies that `plan.pending_tasks` still contains all originally pending tasks (none were removed)
    - Verifies that `plan.merged_tasks` is still empty (no tasks were added)
  - This test proves that on partial failure, the plan state is fully preserved
- **Acceptance Criteria**:
  - The new test passes
  - The test verifies both `pending_tasks` and `merged_tasks` are unmodified after a partial failure
  - The test uses the existing `create_test_repo` infrastructure

### 3. Add regression test for successful batch completion
- **Task ID**: add-successful-batch-test
- **Depends On**: refactor-atomic-batch-merge
- **Assigned To**: merge-sequence-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/git/tests.rs`, add a test `test_execute_merge_sequence_successful_batch` that:
    - Creates a test repo with a plan branch and multiple non-conflicting task branches
    - Constructs a `MergePlan` with several pending tasks (no dependencies, so all merge in one batch)
    - Calls `execute_merge_sequence` and expects success
    - Verifies that all tasks were moved from `pending_tasks` to `merged_tasks`
    - Verifies the returned vector contains all merged task IDs
- **Acceptance Criteria**:
  - The new test passes
  - The test verifies correct plan state update after a fully successful batch
  - The test confirms the function still works correctly for the happy path

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: refactor-atomic-batch-merge, add-partial-failure-test, add-successful-batch-test
- **Assigned To**: merge-sequence-fix-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all tests pass including new regression tests
  - Run `cargo clippy` — no warnings
  - Verify `execute_merge_sequence` does not mutate `plan` inside the `for` loop
  - Verify plan state is updated only after the `for` loop completes
  - Verify the doc comment mentions atomic batch behavior
  - Verify no other functions in `merge.rs` were inadvertently modified

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: merge-sequence-fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
1. `execute_merge_sequence` does not mutate `plan.pending_tasks` or `plan.merged_tasks` inside the per-task merge loop
2. Plan state is updated atomically only after all merges in a batch succeed
3. On partial batch failure, the `MergePlan` state is fully preserved (unchanged from before the batch started)
4. All existing tests continue to pass
5. New regression tests verify both failure-state preservation and successful-batch behavior
6. Code compiles cleanly with no clippy warnings
7. The function signature and return type remain unchanged (backward compatible)

## Validation Commands
- `cargo test` — Run full test suite
- `cargo test git::tests::tests::test_execute_merge_sequence_partial_failure_preserves_state` — Run partial failure regression test
- `cargo test git::tests::tests::test_execute_merge_sequence_successful_batch` — Run successful batch regression test
- `cargo clippy -- -D warnings` — Check for clippy warnings
- `cargo fmt -- --check` — Check code formatting

## Notes
- This fix is independent of the `005.1` plan (merge conflict cleanup fix). Both fixes should be applied to `src/git/merge.rs`, but they address different functions and issues.
- The `merge_task_branch` function called within the loop may still leave the repository in a conflicted state on failure (addressed by plan 005.1). This plan focuses solely on the `MergePlan` struct state corruption issue.
- The atomic batch approach means that if a batch has multiple tasks and one fails, none of the tasks in that batch are recorded as merged in the plan. The caller can then retry the entire batch.
- The `merged` return vector (local to the function) is still updated per-task inside the loop in the original code. In the fix, it should also be updated only after the batch succeeds, to maintain consistency between the return value and the plan state.
