# Git Circular Dependency Error

## Overview

The `determine_merge_order` function in `src/git/merge.rs` previously returned a semantically incorrect `GitError::SubprocessFailure` error when Kahn's algorithm detected a cycle in the task dependency graph. Since no git subprocess was involved in the cycle detection (it is a pure Rust computation), the fabricated error fields (`exit_code: -1`, empty stdout, fake stderr) were misleading to callers and could trigger inappropriate error-handling behavior such as retries.

This fix introduces a dedicated `GitError::CircularDependency` variant that accurately represents the error condition and carries the list of tasks involved in the cycle.

## What Was Built

Three changes were made across the git module:

1. **New error variant** — `GitError::CircularDependency { tasks: Vec<String> }` was added to the `GitError` enum, placed between `DependencyNotMet` and `SpawnFailed` to keep dependency-related errors grouped together.

2. **Updated error handling** — `determine_merge_order` now returns `CircularDependency` (with the actual cycled task IDs) instead of the fabricated `SubprocessFailure`. The cycled tasks are identified as those in `plan.pending_tasks` that do not appear in the topological sort result.

3. **Test coverage** — Two unit tests were added: a 2-node cycle test (`TASK-001 <-> TASK-002`) and a 3-node cycle test (`TASK-A -> TASK-B -> TASK-C -> TASK-A`), both asserting the correct error type and the presence of all cycled task IDs.

## Technical Implementation

### Files Modified

| File | Change |
|------|--------|
| `src/git/errors.rs` | Added `CircularDependency { tasks: Vec<String> }` variant to `GitError` enum (lines 79–81) |
| `src/git/merge.rs` | Updated `determine_merge_order` to return `CircularDependency` with cycled tasks (lines 334–342); updated doc comment (line 282) |
| `src/git/tests.rs` | Added `test_determine_merge_order_circular_dependency` (lines 232–258) and `test_determine_merge_order_circular_dependency_three_node` (lines 260–290) |

### Key Changes

#### `GitError::CircularDependency` (errors.rs, lines 79–81)

```rust
/// Circular dependency detected in the merge task dependency graph.
#[error("circular dependency detected among tasks: {tasks:?}")]
CircularDependency { tasks: Vec<String> },
```

The `#[error(...)]` attribute produces a clear diagnostic message listing the cycled task IDs.

#### `determine_merge_order` (merge.rs, lines 334–342)

The cycle detection block now computes the unresolved tasks and returns the proper error variant:

```rust
if result.len() != pending.len() {
    let unresolved: Vec<String> = plan
        .pending_tasks
        .iter()
        .filter(|t| !result.contains(*t))
        .cloned()
        .collect();
    return Err(GitError::CircularDependency { tasks: unresolved });
}
```

The doc comment was updated from referencing `SubprocessFailure` to `CircularDependency`.

### Dependencies

No new dependencies were added. The change uses only existing types (`GitError`, `Vec<String>`).

## Usage

Callers of `determine_merge_order` can now pattern-match on the specific error variant:

```rust
match determine_merge_order(&plan) {
    Ok(order) => { /* merge tasks in order */ }
    Err(GitError::CircularDependency { tasks }) => {
        eprintln!("Cannot merge: circular dependency among tasks: {:?}", tasks);
    }
    Err(e) => { /* handle other errors */ }
}
```

This allows callers to distinguish cycle detection failures from git subprocess failures and respond appropriately (e.g., reporting the specific cycled tasks rather than retrying).

## Test Coverage

Two tests verify circular dependency detection:

| Test | Description |
|------|-------------|
| `test_determine_merge_order_circular_dependency` | 2-node cycle: `TASK-001` depends on `TASK-002` and vice versa. Verifies both task IDs appear in the error's `tasks` field. |
| `test_determine_merge_order_circular_dependency_three_node` | 3-node cycle: `TASK-A -> TASK-C -> TASK-B -> TASK-A`. Verifies all three task IDs appear in the error's `tasks` field. |

Both tests assert `result.is_err()` and then match on `GitError::CircularDependency { tasks }`, confirming the correct error type and task membership.

## Configuration

No configuration changes are required. This is an internal error-type correction with no user-facing configuration impact.
