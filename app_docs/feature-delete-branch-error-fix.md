# Fix: `delete_branch` Error Swallowing (Spec 005.4)

## Overview

Fixed the `delete_branch` function in `src/git/branch.rs` so that it only treats the "branch not found" case as a no-op, and propagates all other errors (git crashes, repository corruption, permissions errors, timeouts, etc.) to the caller.

## What Was Changed

The `delete_branch` function was rewritten from an indiscriminate error-swallowing pattern to a **check-then-delete** approach that uses the existing `branch_exists` helper to determine whether deletion should be attempted.

### Before (broken)

```rust
pub async fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let output = git(repo_root, &["branch", "-D", branch_name]).await;
    match output {
        Ok(_) => Ok(()),
        Err(_) => {
            // If the branch doesn't exist, treat it as a no-op.
            Ok(())
        }
    }
}
```

The `Err(_)` arm swallowed **every** possible error — git crashes, repo corruption, permissions failures, timeouts — returning `Ok(())` as if deletion succeeded.

### After (fixed)

```rust
pub async fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    if !branch_exists(repo_root, branch_name).await? {
        return Ok(());
    }
    let _output = git(repo_root, &["branch", "-D", branch_name]).await?;
    Ok(())
}
```

Only the "branch doesn't exist" case is a no-op. All other errors propagate via `?`.

## Why It Was Changed

The original implementation had a critical correctness flaw: callers had no way to distinguish between "the branch didn't exist (expected)" and "git crashed / the repo is corrupted / permissions were denied (unexpected)." This meant real failures were silently masked.

Specific error scenarios that were incorrectly swallowed as `Ok(())`:

| Error Type | Old Behavior | New Behavior |
|---|---|---|
| Branch not found | `Ok(())` (correct) | `Ok(())` (correct) |
| Git crash / segfault | `Ok(())` (wrong) | Propagated to caller |
| Repository corruption | `Ok(())` (wrong) | Propagated to caller |
| Permissions error | `Ok(())` (wrong) | Propagated to caller |
| Timeout | `Ok(())` (wrong) | Propagated to caller |

## The Check-Then-Delete Approach

The fix uses `branch_exists` (which runs `git branch --list <name>`) to check existence before attempting deletion. This was chosen over parsing `git branch -D` stderr because:

- **No stderr parsing** — stderr message text varies across git versions and locales, making it fragile.
- **Reuses existing code** — `branch_exists` already handles this check correctly.
- **Semantically clear** — the intent ("delete only if it exists") is obvious.
- **No practical race condition** — Nexum task branches are local-only and managed exclusively by the tool, so no external actor modifies them concurrently.

The performance cost is one additional fast git command (`git branch --list` is a pure filesystem lookup in `.git/refs`), which is negligible compared to the correctness improvement.

## Technical Implementation

### Files Modified

- **`src/git/branch.rs`** (lines 72–81) — Rewrote `delete_branch` with check-then-delete pattern and updated doc comment.

### Files Created

- None.

### Test Added

- **`src/git/tests.rs`** — Added `test_delete_branch_nonexistent_noop` to verify that deleting a non-existent branch returns `Ok(())` (no-op behavior preserved).

## Downstream Callers

Three callers of `delete_branch` were verified to be compatible with the new behavior:

| Caller | File | Error Handling | Impact |
|---|---|---|---|
| `setup_task_workspace` | `src/git/branch.rs:256` | `let _ = ...` (ignores errors) | No change — already discards errors during cleanup |
| `teardown_task_workspace` | `src/git/branch.rs:283` | `?` (propagates errors) | **Now correctly sees real failures** instead of silent masking |
| `cleanup_merged_task` | `src/git/merge.rs:426` | `?` (propagates errors) | **Now correctly sees real failures** instead of silent masking |

No caller code changes were needed — this is a behavioral correction, not a contract change. The function signature and return type are unchanged.

## Validation

The fix was validated with:

- `cargo check` — no compilation errors
- `cargo test` — all tests pass including the new no-op test
- `cargo test test_delete_branch` — existing deletion test passes
- `cargo test test_delete_branch_nonexistent_noop` — new no-op test passes
- `cargo clippy` — no lint warnings
