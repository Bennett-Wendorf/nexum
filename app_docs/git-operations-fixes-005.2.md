# Git Operations Fixes (005.2)

## Overview

This fix addresses two critical issues in the git merge subsystem: (1) merge conflict paths that left repositories in a dirty, conflicted state, and (2) state corruption in `execute_merge_sequence` when a batch of merges partially failed.

## What Was Built

Three areas were fixed:

- **`merge_branch` conflict cleanup** — On merge conflict, the function now aborts the in-progress merge and restores the original branch before returning the error.
- **`merge_task_branch` conflict cleanup** — Same cleanup applied to the task-branch merge variant.
- **Atomic batch updates in `execute_merge_sequence`** — Plan state (the `pending_tasks` and `merged_tasks` lists) is now only updated after *all* merges in a batch succeed. If any merge fails, the plan state is left untouched for that batch, allowing safe retry.

## Problem

### Conflicted Repository State

When `git merge --no-ff` produced conflicts (exit code 1), both `merge_branch` and `merge_task_branch` detected the conflicts and returned a `MergeConflict` error. However, they did **not**:

1. Abort the in-progress merge (`git merge --abort`).
2. Restore the original branch (`git checkout <original>`).

This left the repository checked out to the target/plan branch with unresolved conflicts, potentially breaking subsequent git operations.

### State Corruption on Partial Batch Failure

`execute_merge_sequence` processes merges in dependency-ordered batches. The original implementation updated plan state incrementally — moving each task from `pending_tasks` to `merged_tasks` immediately after its individual merge succeeded. If a later merge in the same batch failed (e.g., due to a conflict), the plan state was already partially updated, creating an inconsistent view: some tasks appeared merged when they were not, and the failed task remained in `pending_tasks` alongside tasks that had already been processed.

## Solution

### Conflict Cleanup

In both `merge_branch` and `merge_task_branch`, the conflict detection path now includes two cleanup steps before returning the error:

```rust
let _ = abort_merge(repo_root).await;
let _ = checkout_branch(repo_root, &original_branch).await;
```

Both calls use `let _ = ...` to ignore errors — if cleanup fails, the primary `MergeConflict` error is still returned. This matches the existing error-handling pattern in the `Err(e)` fallback paths of these functions.

### Atomic Batch Updates

`execute_merge_sequence` now uses a `batch_merged` accumulator:

1. All mergeable tasks for the current batch are collected.
2. Each task is merged sequentially, and successful merges are recorded in `batch_merged`.
3. If **any** merge fails, the `?` operator propagates the error and `batch_merged` is discarded — plan state is **not** updated.
4. If **all** merges succeed, tasks in `batch_merged` are moved from `pending_tasks` to `merged_tasks`.

This ensures plan state is only committed after a full batch succeeds.

## Impact

- Repositories are no longer left in conflicted states after merge failures.
- The original branch is always restored after a merge attempt, regardless of outcome.
- Plan state remains consistent even when a batch partially fails — callers can safely retry.
- All existing tests continue to pass; four new regression tests verify the fixes.

## Tests

### `test_merge_branch_conflict_cleanup`

Creates a test repo with two branches containing conflicting changes to the same file. Calls `merge_branch` and expects a `MergeConflict` error. Verifies:
- The repository is **not** in a merging state (`is_merging` returns `false`).
- The current branch is restored to the original branch.

### `test_merge_task_branch_conflict_cleanup`

Creates a plan branch and a task branch with conflicting changes. Calls `merge_task_branch` and expects a `MergeConflict` error. Verifies:
- The repository is **not** in a merging state.
- The current branch is restored to the original branch.

### `test_execute_merge_sequence_partial_failure_preserves_state`

Creates three task branches: TASK-001 and TASK-003 (no dependencies, batch 1) and TASK-002 (depends on TASK-001, batch 2, with a conflict). Executes `execute_merge_sequence`. Verifies:
- Batch 1 succeeds: TASK-001 and TASK-003 are in `merged_tasks`.
- Batch 2 fails: TASK-002 is **not** in `merged_tasks` and remains in `pending_tasks`.

### `test_execute_merge_sequence_successful_batch`

Creates three independent task branches (no dependencies). Executes `execute_merge_sequence`. Verifies:
- All three tasks merge successfully in a single batch.
- `merged_tasks` contains all three tasks; `pending_tasks` is empty.

## Files Changed

| File | Changes |
|------|---------|
| `src/git/merge.rs` | Added `abort_merge` + `checkout_branch` cleanup in `merge_branch` conflict path (line 108-110). Added same cleanup in `merge_task_branch` conflict path (line 249-251). Refactored `execute_merge_sequence` to use `batch_merged` accumulator for atomic batch state updates (line 389-400). |
| `src/git/tests.rs` | Added four regression tests: `test_merge_branch_conflict_cleanup`, `test_merge_task_branch_conflict_cleanup`, `test_execute_merge_sequence_partial_failure_preserves_state`, `test_execute_merge_sequence_successful_batch`. |
