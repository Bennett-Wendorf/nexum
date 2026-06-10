# Git Operations Fixes (005.2)

## Overview

This fix addresses state corruption in `execute_merge_sequence` when a batch of merges partially fails.

## What Was Built

One fix was delivered:

- **Atomic batch updates in `execute_merge_sequence`** — Plan state (the `pending_tasks` and `merged_tasks` lists) is now only updated after *all* merges in a batch succeed. If any merge fails, the plan state is left untouched for that batch, allowing safe retry.

## Problem

### State Corruption on Partial Batch Failure

`execute_merge_sequence` processes merges in dependency-ordered batches. The original implementation updated plan state incrementally — moving each task from `pending_tasks` to `merged_tasks` immediately after its individual merge succeeded. If a later merge in the same batch failed (e.g., due to a conflict), the plan state was already partially updated, creating an inconsistent view: some tasks appeared merged when they were not, and the failed task remained in `pending_tasks` alongside tasks that had already been processed.

## Solution

### Atomic Batch Updates

`execute_merge_sequence` now uses a `batch_merged` accumulator:

1. All mergeable tasks for the current batch are collected.
2. Each task is merged sequentially, and successful merges are recorded in `batch_merged`.
3. If **any** merge fails, the `?` operator propagates the error and `batch_merged` is discarded — plan state is **not** updated.
4. If **all** merges succeed, tasks in `batch_merged` are moved from `pending_tasks` to `merged_tasks`.

This ensures plan state is only committed after a full batch succeeds.

## Impact

- Plan state remains consistent even when a batch partially fails — callers can safely retry.
- On failure within a batch, no tasks from that batch are recorded as merged.
- Tasks from previously successful batches remain correctly recorded.
- All existing tests continue to pass; two new regression tests verify the fix.

## Tests

### `test_execute_merge_sequence_partial_failure_preserves_state`

Creates three task branches: TASK-001 and TASK-003 (no dependencies, batch 1) and TASK-002 (depends on TASK-001, batch 2, with a conflict). Executes `execute_merge_sequence`. Verifies:
- Batch 1 succeeds: TASK-001 and TASK-003 are in `merged_tasks`.
- Batch 2 fails: TASK-002 is **not** in `merged_tasks` and remains in `pending_tasks`.
- Repository is not in a merging state after the failure.
- Current branch is restored after the failure.

### `test_execute_merge_sequence_successful_batch`

Creates three independent task branches (no dependencies). Executes `execute_merge_sequence`. Verifies:
- All three tasks merge successfully in a single batch.
- `merged_tasks` contains all three tasks; `pending_tasks` is empty.
- Repository is not in a merging state after execution.
- Current branch is restored after execution.

## Files Changed

| File | Changes |
|------|---------|
| `src/git/merge.rs` | Refactored `execute_merge_sequence` to use `batch_merged` accumulator for atomic batch state updates. Plan state (`pending_tasks`, `merged_tasks`) is now updated only after all merges in a batch succeed. |
| `src/git/tests.rs` | Added two regression tests: `test_execute_merge_sequence_partial_failure_preserves_state` and `test_execute_merge_sequence_successful_batch`. |
