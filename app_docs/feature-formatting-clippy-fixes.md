# Feature: Formatting and Clippy Fixes

## Overview

Comprehensive code quality pass across the Rust codebase to eliminate all `cargo clippy` warnings and `cargo fmt` formatting differences. The codebase accumulated ~197 clippy warnings and formatting inconsistencies across multiple files, which were systematically resolved through automated and manual fixes.

## What Was Built

This was a code quality improvement pass with no new functionality. The work addressed eight categories of warnings:

1. **Redundant closures** (~25 occurrences): Converted `.map_err(|e| ErrorVariant(e))` to `.map_err(ErrorVariant)` across 4 files.
2. **Collapsible if statements** (3 occurrences): Merged nested `if` statements with `&&`.
3. **match_like_matches_macro** (2 occurrences): Converted verbose `match` expressions to the `matches!` macro in both `can_transition` methods.
4. **if_same_then_else** (2 groups): Consolidated identical if-else branches using `||` in the markdown parser.
5. **too_many_arguments** (3 occurrences): Refactored functions with 8+ parameters to use params structs.
6. **Unused imports** (~16 warnings): Removed dead imports from mod.rs files.
7. **Dead code** (~80+ warnings): Explicitly allowed via `Cargo.toml` lint configuration (code written ahead of usage).
8. **cargo fmt issues**: Reformatted 25 Rust source files.

## Technical Implementation

### Files Modified

| File | Changes |
|------|---------|
| `Cargo.toml` | Added `dead_code = "allow"` to `[lints.rust]` with TODO comment |
| `src/overlord/status_machine.rs` | Converted `match` to `matches!` in both `can_transition` methods |
| `src/persistence/markdown.rs` | Consolidated identical if-else branches (AcceptanceCriteria/FilesToModify, Background/Notes) |
| `src/persistence/operations.rs` | Created params structs; refactored `update_task_status` and `recover_task_status` |
| `src/overlord/scheduler.rs` | Created `TransitionTaskStatusParams`; refactored `transition_task_status` |
| `src/overlord/dependency_resolver.rs` | Auto-fixed redundant closures, collapsible if |
| `src/overlord/heartbeat_monitor.rs` | Auto-fixed redundant closures |
| `src/persistence/io.rs` | Auto-fixed redundant closure |
| `src/persistence/directory.rs` | Auto-fixed collapsible if |
| `src/git/mod.rs` | Auto-fixed unused imports |
| `src/persistence/mod.rs` | Auto-fixed unused imports |
| `src/config/mod.rs` | Auto-fixed unused imports |
| `src/overlord/mod.rs` | Auto-fixed unused imports |

### New Parameter Structs

Four parameter structs were introduced to replace functions with 8+ positional arguments:

- **`TaskPathParams`** (`src/persistence/operations.rs`): Groups path-related fields (`repo_root`, `branch`, `plan_id`, `plan_name`, `task_id`, `task_name`) used by multiple functions.
- **`UpdateTaskStatusParams`** (`src/persistence/operations.rs`): Wraps `TaskPathParams` + `new_status` + `by` for `update_task_status()`.
- **`RecoverTaskStatusParams`** (`src/persistence/operations.rs`): Wraps `TaskPathParams` + `target_status` + `by` for `recover_task_status()`.
- **`TransitionTaskStatusParams`** (`src/overlord/scheduler.rs`): Groups `branch`, `plan_id`, `plan_name`, `task_id`, `task_name`, `new_status`, `by` for `transition_task_status()`.

### Key API Changes

- `update_task_status(params: &UpdateTaskStatusParams)` — previously 8 positional arguments
- `recover_task_status(params: &RecoverTaskStatusParams)` — previously 8 positional arguments
- `transition_task_status(&self, params: &TransitionTaskStatusParams)` — previously 8 positional arguments
- 10 call sites across 5 files were updated to construct the new params structs

## Usage

### Verification Commands

```bash
# Verify zero clippy warnings
cargo clippy

# Verify zero formatting differences
cargo fmt --check

# Verify all tests pass
cargo test
```

### Specific Warning Checks

```bash
cargo clippy -- -W clippy::redundant_closure
cargo clippy -- -W clippy::match_like_matches_macro
cargo clippy -- -W clippy::if_same_then_else
cargo clippy -- -W clippy::too_many_arguments
```

### Post-MVP: Re-enable dead_code lint

```bash
RUSTFLAGS="-W dead_code" cargo check
```

## Configuration

### Cargo.toml Lint Configuration

```toml
[lints.rust]
# TODO: Remove dead_code = "allow" after MVP is complete.
# This is only allowed during MVP development since many functions
# are written ahead of usage and not yet wired into the application.
dead_code = "allow"
```

The `dead_code` lint is temporarily allowed because ~80 warnings represent code written ahead of usage (git operations, config helpers, overlord utilities) that will be wired up in future development. **Remove this allowance once MVP is complete.**

### Results

- **Zero** clippy warnings (dead_code explicitly allowed)
- **Zero** fmt differences
- **All 124 tests** pass
- **No semantic changes** — purely style and quality improvements
