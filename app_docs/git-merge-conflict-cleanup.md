# Git Merge Conflict Cleanup

## Overview

Fixed `merge_branch` and `merge_task_branch` in `src/git/merge.rs` so that when a merge conflict is detected, the repository state is properly restored before returning the error. Previously, both functions returned a `MergeConflict` error but left the repository in a conflicted state with unresolved conflicts.

## What Was Built

Both merge functions now abort the in-progress merge and restore the original branch whenever conflicts are detected. The cleanup uses `let _ = ...` to ignore errors — if cleanup fails, the primary `MergeConflict` error is still returned to the caller. This matches the existing error-handling pattern in the other error paths of these functions.

## Behavior on Conflict Detection

### `merge_branch`

When `git merge --no-ff` produces conflicts (exit code 1):

1. Records the current branch before checkout.
2. Checks out the target branch and runs the merge.
3. On conflict detection (exit code 1 with conflicted files):
   - Calls `abort_merge(repo_root)` to cancel the in-progress merge.
   - Calls `checkout_branch(repo_root, &original_branch)` to restore the original branch.
   - Returns `Err(GitError::MergeConflict { ... })`.

### `merge_task_branch`

Same cleanup pattern as `merge_branch`:

1. Validates the task branch exists and records the current branch.
2. Checks out the plan branch and merges the task branch.
3. On conflict detection:
   - Calls `abort_merge(repo_root)`.
   - Calls `checkout_branch(repo_root, &original_branch)`.
   - Returns `Err(GitError::MergeConflict { ... })`.

In both cases, cleanup errors are ignored (`let _ = ...`) so the caller always receives the meaningful `MergeConflict` error.

## Technical Implementation

### Files Modified

- **`src/git/merge.rs`** — Added cleanup calls in the conflict paths of `merge_branch` (lines 100–103) and `merge_task_branch` (lines 246–249).

### Key Changes

In `merge_branch`, the conflict path (previously lines 80–86) now includes:

```rust
// Conflicts detected — abort merge and restore original branch
let _ = abort_merge(repo_root).await;
let _ = checkout_branch(repo_root, &original_branch).await;
Err(conflict_error)
```

The same pattern was applied to `merge_task_branch`.

### Dependencies

No new dependencies were added. The fix uses existing `abort_merge` and `checkout_branch` functions.

## Regression Tests

Two new tests were added to `src/git/tests.rs`:

### `test_merge_branch_conflict_cleanup`

- Creates a test repo with two branches that have conflicting changes to `README.md`.
- Calls `merge_branch` and expects a `MergeConflict` error.
- Verifies the repo is **not** in a merging state (`is_merging` returns `false`).
- Verifies the current branch is restored to the original branch.

### `test_merge_task_branch_conflict_cleanup`

- Creates a test repo with a plan branch and a task branch with conflicting changes.
- Calls `merge_task_branch` and expects a `MergeConflict` error.
- Verifies the repo is **not** in a merging state.
- Verifies the current branch is restored to the original branch.

## Validation

```bash
cargo test git::tests::tests::test_merge_branch_conflict_cleanup
cargo test git::tests::tests::test_merge_task_branch_conflict_cleanup
cargo clippy -- -D warnings
```

## Notes

- `abort_merge` is safe to call even when no merge is in progress — it returns `Ok(())` as a no-op.
- The non-conflict failure path (merge failed with exit code 1 but no conflicts) does **not** need cleanup, since no `MERGE_HEAD` file exists in that case.
- `execute_merge_sequence` calls `merge_task_branch` and propagates errors, so fixing `merge_task_branch` automatically fixes the sequence-level behavior.
