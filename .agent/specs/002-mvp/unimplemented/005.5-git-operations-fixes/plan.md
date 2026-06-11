# Plan: 005.5 - Fix manual directory removal leaving stale git worktree metadata

## Task Description
Fix the `remove_worktree` and `remove_worktree_force` functions in `src/git/worktree.rs` so that after falling back to manual `fs::remove_dir_all` directory removal, they run `git worktree prune` to clean up stale entries in git's internal worktree database (`.git/worktrees/`).

## Objective
Ensure that when `git worktree remove` (and `git worktree remove --force`) both fail and the code falls back to manual directory removal via `fs::remove_dir_all`, the stale worktree metadata in `.git/worktrees/` is cleaned up by running `git worktree prune`.

## Problem Statement
The `remove_worktree` function (lines 124-165) and `remove_worktree_force` function (lines 180-208) in `src/git/worktree.rs` both follow this pattern:

1. Attempt `git worktree remove` (or `git worktree remove --force`)
2. If that fails, try `git worktree remove --force` (only in `remove_worktree`)
3. If that also fails, fall back to `fs::remove_dir_all` to manually delete the directory

The problem with step 3 is that `fs::remove_dir_all` only removes the worktree directory on disk. It does **not** remove the corresponding entry in git's internal worktree database located at `.git/worktrees/<task_id>/`. This leads to:

- `git worktree list` continues to show the removed worktree as existing
- Git may refuse to create a new worktree at the same path because it thinks the old one still exists
- Stale metadata accumulates in `.git/worktrees/` over time
- The `worktree_exists` function may return inconsistent results (it checks for the directory, but git's database disagrees)

## Solution Approach
After a successful `fs::remove_dir_all` call in the fallback path, run `git worktree prune` to clean up stale entries. The prune command is safe to run even when the git commands already cleaned things up (it's a no-op in that case), so we only need to add it to the fallback path where manual removal was used.

The fix applies to three locations:

1. **`remove_worktree` fallback** (line 156-160): After `fs::remove_dir_all`, run `git worktree prune`
2. **`remove_worktree_force` fallback** (line 201-204): After `fs::remove_dir_all`, run `git worktree prune`

The prune operation itself uses `let _ = ...` to ignore errors, since:
- The directory is already removed (the critical cleanup is done)
- Prune failure would indicate a deeper git issue that doesn't block the caller
- The caller has already committed to removing the worktree

## Relevant Files
- `src/git/worktree.rs` — Primary fix: add `git worktree prune` after fallback removal (3 locations)
- `src/git/tests.rs` — Add regression test for the prune-after-fallback behavior

### New Files (if needed)
None. All changes are to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: worktree-prune-builder
  - Role: Add `git worktree prune` after manual directory removal fallbacks
  - Agent: builder

- **Validator**
  - Name: worktree-prune-validator
  - Role: Verify implementation meets criteria, run tests
  - Agent: validator

- **Documenter**
  - Name: worktree-prune-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Add prune call to `remove_worktree` fallback path
- **Task ID**: add-prune-to-remove-worktree
- **Depends On**: none
- **Assigned To**: worktree-prune-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/worktree.rs`
  - In `remove_worktree`, after the `fs::remove_dir_all` call at line 156-159 (inside the innermost `Err(_)` arm), add:
    ```rust
    // Force removal also failed — fall back to manual cleanup
    fs::remove_dir_all(&wt_path).await.map_err(|e| GitError::Io {
        path: wt_path.clone(),
        source: e,
    })?;
    // Prune stale worktree metadata from git's internal database.
    // fs::remove_dir_all removes the directory but leaves entries in
    // .git/worktrees/, causing git worktree list to show stale entries.
    let _ = git(repo_root, &["worktree", "prune"]).await;
    Ok(())
    ```
  - Update the module-level doc comment or function doc comment to note that stale metadata is cleaned up
- **Acceptance Criteria**:
  - The code compiles without errors
  - `git worktree prune` is called after `fs::remove_dir_all` in the fallback path
  - Prune errors are ignored with `let _ = ...` (non-critical cleanup)
  - A code comment explains why prune is needed

### 2. Add prune call to `remove_worktree_force` fallback path
- **Task ID**: add-prune-to-remove-worktree-force
- **Depends On**: none
- **Assigned To**: worktree-prune-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/worktree.rs`
  - In `remove_worktree_force`, after the `fs::remove_dir_all` call at line 201-204 (inside the `Err(_)` arm), add:
    ```rust
    // Git force removal failed — fall back to manual directory removal
    fs::remove_dir_all(&wt_path).await.map_err(|e| GitError::Io {
        path: wt_path.clone(),
        source: e,
    })?;
    // Prune stale worktree metadata from git's internal database.
    // fs::remove_dir_all removes the directory but leaves entries in
    // .git/worktrees/, causing git worktree list to show stale entries.
    let _ = git(repo_root, &["worktree", "prune"]).await;
    Ok(())
    ```
- **Acceptance Criteria**:
  - The code compiles without errors
  - `git worktree prune` is called after `fs::remove_dir_all` in the fallback path
  - Prune errors are ignored with `let _ = ...` (non-critical cleanup)
  - A code comment explains why prune is needed

### 3. Add regression test for prune-after-fallback behavior
- **Task ID**: add-prune-regression-test
- **Depends On**: add-prune-to-remove-worktree, add-prune-to-remove-worktree-force
- **Assigned To**: worktree-prune-builder
- **Agent**: builder
- **Actions**:
  - Open `src/git/tests.rs`
  - Add a test that verifies `git worktree prune` is called when the fallback path is triggered. The test strategy:
    1. Create a test repo and spawn a worktree
    2. Manually corrupt the worktree to force the fallback path (e.g., remove the `.git` file inside the worktree so `git worktree remove` fails)
    3. Call `remove_worktree` and verify it succeeds
    4. Run `git worktree list` and verify the removed worktree no longer appears
  - Add test:
    ```rust
    #[tokio::test]
    async fn test_remove_worktree_fallback_prunes_metadata() {
        let (_dir, repo) = create_test_repo();
        create_task_branch(&repo, "TASK-PRUNE", "main").await.unwrap();
        spawn_worktree(&repo, "TASK-PRUNE", "task/TASK-PRUNE").await.unwrap();

        // Corrupt the worktree by removing its .git pointer file
        // This forces the fallback to fs::remove_dir_all
        let wt_path = worktree_path(&repo, "TASK-PRUNE");
        std::fs::remove_file(wt_path.join(".git")).unwrap();

        // Removal should still succeed via fallback
        remove_worktree(&repo, "TASK-PRUNE").await.unwrap();

        // Verify the worktree directory is gone
        assert!(!worktree_exists(&repo, "TASK-PRUNE").await.unwrap());

        // Verify git worktree list no longer shows the stale entry
        let output = std::process::Command::new("git")
            .current_dir(&repo)
            .args(["worktree", "list"])
            .output()
            .unwrap();
        let list_output = String::from_utf8_lossy(&output.stdout);
        assert!(!list_output.contains("TASK-PRUNE"),
            "Stale worktree metadata should be pruned after manual removal");
    }
    ```
- **Acceptance Criteria**:
  - New test compiles and passes
  - Test verifies that after fallback removal, `git worktree list` no longer shows the removed worktree
  - Test covers the critical path: corrupt worktree → fallback removal → prune → clean metadata

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-prune-regression-test
- **Assigned To**: worktree-prune-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — confirm no compilation errors
  - Run `cargo test` — confirm all tests pass including the new prune regression test
  - Run `cargo test test_remove_worktree_fallback_prunes_metadata` — verify the new test passes
  - Verify `remove_worktree` calls `git worktree prune` after `fs::remove_dir_all` in fallback
  - Verify `remove_worktree_force` calls `git worktree prune` after `fs::remove_dir_all` in fallback
  - Verify prune errors are ignored with `let _ = ...`
  - Verify both code comments explain why prune is needed
  - Run `cargo clippy` — check for lint warnings
  - Run `cargo test test_remove_worktree` — verify existing removal test still passes
  - Run `cargo test test_teardown_task_workspace` — verify downstream teardown still works

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: worktree-prune-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `remove_worktree` calls `git worktree prune` after `fs::remove_dir_all` in its fallback path
- `remove_worktree_force` calls `git worktree prune` after `fs::remove_dir_all` in its fallback path
- Prune errors are non-critical (ignored with `let _ = ...`) since the directory is already removed
- Code comments explain why prune is needed (stale `.git/worktrees/` metadata)
- A regression test verifies that `git worktree list` no longer shows a removed worktree after fallback removal
- All existing tests continue to pass
- `cargo check`, `cargo test`, and `cargo clippy` succeed with no warnings

## Validation Commands
- `cargo check` — Verify compilation
- `cargo test` — Run full test suite
- `cargo test test_remove_worktree_fallback_prunes_metadata` — Run the new prune regression test
- `cargo test test_remove_worktree` — Run the existing removal test
- `cargo test test_teardown_task_workspace` — Verify downstream teardown still works
- `cargo clippy` — Check for lint warnings

## Notes
- **Why only in the fallback path?**: When `git worktree remove` (or `--force`) succeeds, git already handles cleaning up `.git/worktrees/` metadata. The prune is only needed when we bypass git and use `fs::remove_dir_all` directly.
- **Why ignore prune errors?**: The directory is already removed — the critical cleanup is done. If prune fails, it indicates a deeper git issue (corrupted `.git/worktrees/` directory, permissions problem, etc.) that doesn't block the caller's workflow. The stale metadata will be cleaned up by the next `git worktree prune` run or when the repo is garbage collected.
- **Performance**: `git worktree prune` is fast — it only reads `.git/worktrees/` and removes entries whose worktree directories no longer exist. The overhead is negligible compared to the `fs::remove_dir_all` call that preceded it.
- **Backward compatibility**: This is a behavioral correction, not a breaking API change. The function signatures and return types are unchanged. The prune call is an internal implementation detail that improves correctness.
