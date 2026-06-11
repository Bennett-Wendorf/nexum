# Plan: 005.6 - Fix cargo clippy and cargo fmt warnings

## Task Description
Fix all `cargo clippy` warnings and `cargo fmt` formatting differences across the Rust codebase. There are approximately 197 clippy warnings (76 auto-fixable) and formatting issues in 12 files. This plan systematically addresses each category of warning in priority order: automatic fixes first, then manual fixes.

## Objective
Achieve zero `cargo clippy` warnings (excluding `dead_code`) and zero `cargo fmt` formatting differences across the entire codebase.

## Problem Statement
The codebase has accumulated 197 clippy warnings and formatting inconsistencies across multiple files. These fall into several categories:

1. **Redundant closures** (~25 occurrences): `.map_err(|e| ErrorVariant(e))` patterns that should be `.map_err(ErrorVariant)`
2. **Collapsible if statements** (3 occurrences): Nested `if` statements that can be collapsed with `&&`
3. **match_like_matches_macro** (2 occurrences): `match` expressions that should use the `matches!` macro
4. **if_same_then_else** (2 groups): `if`/`else` branches that perform the same action
5. **too_many_arguments** (3 occurrences): Functions with 8+ parameters that should use a config struct
6. **Unused imports** (~16 warnings): Imports that are not used
7. **Dead code** (~80+ warnings): Functions, structs, enums that are never called — **these should NOT be fixed yet** as they are intentionally written ahead of usage
8. **cargo fmt issues**: 12 files with formatting differences (import ordering, method chain formatting, long line breaks, etc.)

These warnings indicate code quality issues that, while not blocking compilation, reduce code clarity and maintainability.

## Solution Approach
Apply fixes in three phases of decreasing effort:

1. **Phase 1: Run `cargo fmt`** — Automatically fixes all formatting issues (zero manual effort)
2. **Phase 2: Run `cargo clippy --fix --allow-dirty`** — Automatically fixes the 76 auto-fixable warnings (redundant closures, unused imports, collapsible ifs)
3. **Phase 3: Manual fixes** — Address the remaining ~30 warnings that require manual intervention (match_like_matches, if_same_then_else, too_many_arguments)

Dead code warnings will be explicitly allowed via `#[allow(dead_code)]` or clippy config, since this code is intentionally written ahead of usage and will be wired up in future plans.

## Relevant Files

### Files with clippy warnings
- `src/overlord/dependency_resolver.rs` — 10 redundant closures, 1 collapsible if
- `src/overlord/heartbeat_monitor.rs` — 5 redundant closures
- `src/overlord/scheduler.rs` — 10 redundant closures, 1 too_many_arguments
- `src/overlord/status_machine.rs` — 2 match_like_matches_macro
- `src/persistence/io.rs` — 1 redundant closure
- `src/persistence/directory.rs` — 1 collapsible if
- `src/persistence/markdown.rs` — 2 if_same_then_else
- `src/persistence/operations.rs` — 2 too_many_arguments
- `src/git/mod.rs` — unused imports
- `src/persistence/mod.rs` — unused imports
- `src/config/mod.rs` — unused imports
- `src/overlord/mod.rs` — unused imports

### Files with fmt issues
- `src/overlord/scheduler.rs` — import ordering, line breaks, method chain formatting
- `src/overlord/heartbeat_monitor.rs` — method chain formatting
- `src/overlord/id_generator.rs` — import ordering, method chain formatting
- `src/overlord/mod.rs` — import ordering
- `src/overlord/status_machine.rs` — import ordering, line width formatting
- `src/overlord/tests.rs` — extensive reformatting of long lines
- `src/persistence/directory.rs` — method chain formatting, blank line
- `src/persistence/io.rs` — method chain formatting
- `src/persistence/markdown.rs` — LazyLock formatting, long line breaks
- `src/persistence/operations.rs` — method chain formatting, function signatures
- `src/persistence/tests.rs` — long line breaks
- `src/config/loader.rs` — minor formatting

### New Files (if needed)
None. All changes are to existing files.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: formatting-builder
  - Role: Apply automatic and manual clippy/fmt fixes across the codebase
  - Agent: builder

- **Validator**
  - Name: formatting-validator
  - Role: Verify zero clippy warnings (excluding dead_code) and zero fmt differences
  - Agent: validator

- **Documenter**
  - Name: formatting-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Run cargo fmt
- **Task ID**: run-cargo-fmt
- **Depends On**: none
- **Assigned To**: formatting-builder
- **Agent**: builder
- **Actions**:
  - Run `cargo fmt` to automatically fix all formatting issues
  - Review the diff to ensure formatting changes are reasonable (no semantic changes)
  - Files expected to be reformatted:
    - `src/overlord/scheduler.rs`
    - `src/overlord/heartbeat_monitor.rs`
    - `src/overlord/id_generator.rs`
    - `src/overlord/mod.rs`
    - `src/overlord/status_machine.rs`
    - `src/overlord/tests.rs`
    - `src/persistence/directory.rs`
    - `src/persistence/io.rs`
    - `src/persistence/markdown.rs`
    - `src/persistence/operations.rs`
    - `src/persistence/tests.rs`
    - `src/config/loader.rs`
- **Acceptance Criteria**:
  - `cargo fmt --check` produces no output (all files are formatted)
  - Code compiles successfully after formatting
  - No semantic changes were introduced by formatting

### 2. Run cargo clippy auto-fix
- **Task ID**: run-clippy-fix
- **Depends On**: run-cargo-fmt
- **Assigned To**: formatting-builder
- **Agent**: builder
- **Actions**:
  - Run `cargo clippy --fix --allow-dirty --allow-staged` to automatically fix the 76 auto-fixable warnings
  - This will fix:
    - All redundant closures (`.map_err(|e| ErrorVariant(e))` → `.map_err(ErrorVariant)`)
    - All unused imports
    - All collapsible if statements
  - Review the diff to ensure auto-fixes are correct
  - Run `cargo fmt` again in case clippy's fixes introduced formatting differences
- **Acceptance Criteria**:
  - All auto-fixable clippy warnings are resolved (~76 warnings)
  - Code compiles successfully after fixes
  - `cargo fmt --check` still passes (no new formatting issues)

### 3. Fix match_like_matches_macro warnings
- **Task ID**: fix-matches-macro
- **Depends On**: run-clippy-fix
- **Assigned To**: formatting-builder
- **Agent**: builder
- **Actions**:
  - Open `src/overlord/status_machine.rs`
  - Line 39: Convert `PlanStateMachine::can_transition` match expression to `matches!` macro:
    ```rust
    // Before:
    match (self.current, next) {
        (From, To) => true,
        (_, _) => false,
    }
    // After:
    matches!((self.current, next), (From, To))
    ```
    Replace `match` with `matches!` macro. The pattern will depend on the actual transition rules defined in the enum.
  - Line 96: Convert `TaskStateMachine::can_transition` match expression to `matches!` macro using the same pattern
- **Acceptance Criteria**:
  - Both `can_transition` methods use `matches!` macro instead of `match`
  - Function behavior is unchanged (same boolean output for all inputs)
  - Code compiles without errors
  - `cargo clippy` reports no match_like_matches_macro warnings

### 4. Fix if_same_then_else warnings
- **Task ID**: fix-same-then-else
- **Depends On**: run-clippy-fix
- **Assigned To**: formatting-builder
- **Agent: builder
- **Actions**:
  - Open `src/persistence/markdown.rs`
  - Line 225: The `AcceptanceCriteria` and `FilesToModify` branches perform the same action. Consolidate:
    ```rust
    // Before:
    if condition1 { do_something() } else if condition2 { do_something() } else { other() }
    // After:
    if condition1 || condition2 { do_something() } else { other() }
    ```
  - Line 229: The `Background` and `Notes` branches perform the same action. Apply the same consolidation pattern
  - Verify the exact branch bodies are identical before merging conditions
- **Acceptance Criteria**:
  - Both if_same_then_else warnings are resolved
  - Function behavior is unchanged
  - Code compiles without errors
  - `cargo clippy` reports no if_same_then_else warnings

### 5. Fix too_many_arguments warnings
- **Task ID**: fix-too-many-args
- **Depends On**: run-clippy-fix
- **Assigned To**: formatting-builder
- **Agent**: builder
- **Actions**:
  - Open `src/persistence/operations.rs`
  - Line 148: `update_task_status` has 8 arguments. Create a config struct:
    ```rust
    pub struct UpdateTaskStatusParams {
        pub task_id: String,
        pub project_slug: String,
        pub new_status: TaskStatus,
        pub // ... remaining fields
    }
    ```
    Change signature to `fn update_task_status(params: UpdateTaskStatusParams) -> Result<..., ...>`
    Update all call sites to construct the struct instead of passing positional args.
  - Line 214: `recover_task_status` has 8 arguments. Apply the same pattern with a `RecoverTaskStatusParams` struct (or reuse `UpdateTaskStatusParams` if the fields are identical)
  - Open `src/overlord/scheduler.rs`
  - Line 209: `transition_task_status` has 8 arguments. Apply the same pattern with an appropriate params struct
  - Review all call sites and update them to use the new struct-based signatures
- **Acceptance Criteria**:
  - All three functions use a params struct instead of 8+ positional arguments
  - All call sites are updated to construct the params struct
  - Code compiles without errors
  - `cargo clippy` reports no too_many_arguments warnings
  - Parameter structs have appropriate visibility and documentation

### 6. Temporarily allow dead_code warnings during MVP
- **Task ID**: allow-dead-code
- **Depends On**: run-clippy-fix
- **Assigned To**: formatting-builder
- **Agent**: builder
- **Actions**:
  - Open `Cargo.toml`
  - Add `dead_code = "allow"` under `[lints.rust]` (note: `dead_code` is a rustc lint, not a clippy lint):
    ```toml
    [lints.rust]
    dead_code = "allow"
    ```
  - Add a comment explaining this is temporary for MVP development and should be removed (or changed back to `"warn"`) once the initial build is complete and all scaffolding code is wired up:
    ```toml
    # TODO: Remove dead_code = "allow" after MVP is complete.
    # This is only allowed during MVP development since many functions
    # are written ahead of usage and not yet wired into the application.
    dead_code = "allow"
    ```
  - The ~80 dead_code warnings represent code written ahead of usage (git operations, config helpers, overlord utilities) that will be wired up in future plans
- **Acceptance Criteria**:
  - `cargo clippy` produces no dead_code warnings
  - Dead code lint is explicitly allowed in `[lints.rust]` with a TODO comment for post-MVP removal
  - No functional code was removed

### 7. Final clippy verification
- **Task ID**: verify-clippy-clean
- **Depends On**: fix-matches-macro, fix-same-then-else, fix-too-many-args, allow-dead-code
- **Assigned To**: formatting-builder
- **Agent**: builder
- **Actions**:
  - Run `cargo clippy` and verify zero warnings (excluding dead_code which is allowed)
  - If any warnings remain, fix them iteratively
  - Run `cargo fmt` one final time to ensure no formatting regressions
- **Acceptance Criteria**:
  - `cargo clippy` produces zero warnings
  - `cargo fmt --check` produces no output

### 8. Final Validation
- **Task ID**: validate-all
- **Depends On**: verify-clippy-clean
- **Assigned To**: formatting-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — confirm no compilation errors
  - Run `cargo clippy` — confirm zero warnings
  - Run `cargo fmt --check` — confirm zero formatting differences
  - Run `cargo test` — confirm all tests still pass
  - Verify `src/overlord/dependency_resolver.rs` has no redundant closures
  - Verify `src/overlord/heartbeat_monitor.rs` has no redundant closures
  - Verify `src/overlord/scheduler.rs` has no redundant closures and no too_many_arguments
  - Verify `src/overlord/status_machine.rs` uses `matches!` macro in both `can_transition` methods
  - Verify `src/persistence/markdown.rs` has no if_same_then_else warnings
  - Verify `src/persistence/operations.rs` uses params structs for multi-arg functions
  - Verify no unused imports remain in mod.rs files
  - Verify dead_code is allowed in clippy configuration

### 9. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: formatting-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `cargo clippy` produces zero warnings (dead_code explicitly allowed)
- `cargo fmt --check` produces no output (all files properly formatted)
- All 197 clippy warnings are resolved (or allowed)
- All 12 files with formatting issues are properly formatted
- All existing tests continue to pass
- No semantic changes were introduced (only style/quality improvements)
- Code compiles without errors

## Validation Commands
- `cargo check` — Verify compilation
- `cargo clippy` — Verify zero clippy warnings
- `cargo fmt --check` — Verify zero formatting differences
- `cargo test` — Verify all tests pass
- `cargo clippy -- -W clippy::redundant_closure` — Verify no redundant closures remain
- `cargo clippy -- -W clippy::match_like_matches_macro` — Verify matches! macro is used
- `cargo clippy -- -W clippy::if_same_then_else` — Verify no same-then-else branches
- `cargo clippy -- -W clippy::too_many_arguments` — Verify no functions with 7+ args
- `RUSTFLAGS="-W dead_code" cargo check` — Post-MVP: verify no dead code remains after re-enabling the lint

## Notes
- **Phase order matters**: Run `cargo fmt` first, then `cargo clippy --fix`, then `cargo fmt` again. Clippy's auto-fixes can introduce formatting differences that need a second fmt pass.
- **Dead code temporarily allowed**: The ~80 dead_code warnings represent code written ahead of usage during MVP development. `dead_code` is allowed in `[lints.rust]` with a TODO comment — remove this allowance once MVP is complete and all scaffolding code is wired up, to keep the codebase clean going forward.
- **too_many_arguments refactoring**: When creating params structs, consider whether the structs should be public (if used across modules) or private (if only used within the same module). Prefer private structs to minimize API surface.
- **Backward compatibility**: All changes are purely stylistic/quality improvements. No function signatures change from the caller's perspective (params structs are internal refactoring). No breaking changes are introduced.
- **Testing**: Since no logic is changing, the full test suite should pass without modification. If any tests fail, it indicates an unintended semantic change that needs investigation.
