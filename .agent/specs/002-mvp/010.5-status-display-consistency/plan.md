# Plan: 010.5 - Status Display Consistency Audit

## Task Description
Audit the entire codebase for standalone `*_to_string` functions that duplicate `fmt::Display` implementations on enums or structs. Establish a code convention that all status enums and similar typed values must use `fmt::Display` for string conversion. Propose a lint or clippy rule to prevent this anti-pattern from recurring.

## Objective
Eliminate the possibility of redundant standalone `*_to_string` functions by:
1. Verifying no remaining instances exist in the codebase
2. Documenting the convention explicitly
3. Adding an automated check to prevent recurrence

## Problem Statement
A code review of spec 010.4 (migrate-task-status-display) identified a systematic anti-pattern: standalone `*_to_string` functions that duplicate `fmt::Display` implementations on enums. The specific instance was `plan_status_to_string` in `src/overlord/status_machine.rs`, which duplicated the `fmt::Display` impl for `PlanStatus` in `src/persistence/schema.rs`. That instance has been resolved.

However, the broader pattern needs to be captured as an enforceable convention so similar issues do not recur. Without an explicit convention and automated check, developers may unknowingly reintroduce these redundant functions when adding new status enums or typed values.

## Solution Approach

### Phase 1: Codebase Audit
A comprehensive scan of all Rust source files to identify any remaining standalone `*_to_string` functions that duplicate `fmt::Display` implementations. The audit covers:
- Function signatures matching `fn xxx_to_string` patterns
- Functions that take enum/struct references and return `&'static str` or `String`
- Comparison with existing `fmt::Display` implementations

### Phase 2: Convention Documentation
Document the convention in the codebase as a code comment in `src/persistence/schema.rs` (the canonical location for status types) and/or as a note in the project's development guidelines. The convention states:

> **Convention**: All status enums and typed values that require string representation MUST use `fmt::Display` for conversion. Standalone `*_to_string` functions are an anti-pattern and MUST NOT be introduced. Use `value.to_string()` via the `Display` trait instead.

### Phase 3: Automated Prevention
Propose an automated check to prevent this pattern from recurring:
- A custom clippy lint or a grep-based CI check that flags standalone `*_to_string` functions
- A documentation note in `Cargo.toml` `[lints.rust]` section or a `.clippy.toml` file

## Relevant Files

### Files Audited
| File | Status |
|------|--------|
| `src/persistence/schema.rs` | Contains `fmt::Display` for `PlanStatus` (line 72) and `TaskStatusValue` (line 189). No standalone `*_to_string` functions. |
| `src/overlord/status_machine.rs` | Previously contained `plan_status_to_string` (now removed). Uses `.to_string()` via `Display` on lines 63-64, 69-70, 130-131, 136-137. |
| `src/api/errors.rs` | Contains `fmt::Display` for `ApiErrorResponse` (line 158). This is legitimate (not a duplicate). |
| `src/api/execution.rs` | Uses `.to_string()` via `Display` (line 55). |
| `src/api/plans.rs` | Uses `.to_string()` via `Display` (line 48). |
| `src/api/tasks.rs` | Uses `.to_string()` via `Display` (line 55). |
| `src/persistence/operations.rs` | Uses `.to_string()` via `Display` (lines 191-192, 253-254). |
| `src/overlord/scheduler.rs` | Uses `.to_string()` via `Display` (lines 244-245, 300-301). |

### Files to Modify
| File | Changes |
|------|---------|
| `src/persistence/schema.rs` | Add convention doc comment near the top of the module |
| `Cargo.toml` | Add a clippy lint configuration section if not present |

### New Files (if needed)
| File | Purpose |
|------|---------|
| (optional) `.github/workflows/lint-check.yaml` or script in scripts/ | Grep-based CI check for `*_to_string` anti-pattern |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: consistency-audit-builder
  - Role: Perform codebase audit, document convention, implement lint configuration
  - Agent: builder

- **Validator**
  - Name: consistency-audit-validator
  - Role: Verify audit completeness, convention clarity, and lint effectiveness
  - Agent: validator

- **Documenter**
  - Name: consistency-audit-documenter
  - Role: Generate documentation for the convention and lint
  - Agent: documenter

## Step by Step Tasks

### 1. Perform Codebase Audit
- **Task ID**: audit-codebase
- **Depends On**: none
- **Assigned To**: consistency-audit-builder
- **Agent**: builder
- **Actions**:
  - Run `grep -rn 'fn \w*_to_string' src/` to find all standalone `*_to_string` functions
  - Run `grep -rn 'impl fmt::Display' src/` to find all `Display` implementations
  - For each `*_to_string` function found, check if a corresponding `fmt::Display` exists on the same type
  - Document findings in a summary table
  - Verify that no remaining instances of the anti-pattern exist
- **Acceptance Criteria**:
  - Audit confirms zero remaining standalone `*_to_string` functions that duplicate `fmt::Display`
  - Audit report documents all `fmt::Display` implementations found (3 expected: `PlanStatus`, `TaskStatusValue`, `ApiErrorResponse`)
  - Audit report confirms all call sites use `.to_string()` via `Display` trait

### 2. Document Convention in Source Code
- **Task ID**: document-convention
- **Depends On**: audit-codebase
- **Assigned To**: consistency-audit-builder
- **Agent**: builder
- **Actions**:
  - Add a module-level doc comment to `src/persistence/schema.rs` stating the Display convention, e.g.:
    ```rust
    /// # String Conversion Convention
    ///
    /// All status enums and typed values in this crate use `fmt::Display` for
    /// string conversion. Do NOT create standalone `*_to_string` functions —
    /// they duplicate the `Display` implementation and create maintenance burden.
    /// Use `value.to_string()` via the `Display` trait instead.
    ```
  - Place the comment near the top of the module (after the existing module doc comment)
  - Ensure the convention is clear, actionable, and references the `Display` trait
- **Acceptance Criteria**:
  - Convention doc comment is present in `src/persistence/schema.rs`
  - Comment clearly states the rule and the preferred approach
  - `cargo doc` builds without warnings

### 3. Add Clippy Lint Configuration
- **Task ID**: add-clippy-lint
- **Depends On**: document-convention
- **Assigned To**: consistency-audit-builder
- **Agent: builder
- **Actions**:
  - Add a `[lints.clippy]` section to `Cargo.toml` (or create `.clippy.toml` if needed)
  - Note: There is no built-in clippy lint for this specific pattern, so the approach is:
    - Add a comment in `[lints.rust]` or `[lints.clippy]` section referencing the convention
    - Create a grep-based check script for CI that flags `*_to_string` functions
  - Create a script `scripts/check-display-consistency.sh` that:
    - Runs `grep -rn 'fn \w*_to_string' src/` 
    - For each match, checks if a corresponding `fmt::Display` exists
    - Exits with non-zero status if violations are found
    - Prints helpful error messages explaining the convention
  - Make the script executable
- **Acceptance Criteria**:
  - Convention is documented in `Cargo.toml` lint section or a dedicated file
  - Grep-based check script exists and is executable
  - Script correctly identifies the anti-pattern and reports violations
  - Script passes on the current clean codebase

### 4. Validate Audit and Convention
- **Task ID**: validate-all
- **Depends On**: add-clippy-lint
- **Assigned To**: consistency-audit-validator
- **Agent: validator
- **Checks**:
  - Run `cargo check` — must compile with no errors
  - Run `cargo test` — all tests must pass
  - Run `grep -rn 'fn \w*_to_string' src/` — must return zero results
  - Run `grep -rn 'impl fmt::Display' src/` — must find exactly 3 implementations
  - Run `scripts/check-display-consistency.sh` — must pass (exit 0)
  - Verify convention doc comment is present and clear in `schema.rs`
  - Verify `cargo doc` builds without warnings

### 5. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: consistency-audit-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- Codebase audit confirms zero remaining standalone `*_to_string` functions that duplicate `fmt::Display`
- Convention is documented in `src/persistence/schema.rs` module doc comment
- All 3 `fmt::Display` implementations (`PlanStatus`, `TaskStatusValue`, `ApiErrorResponse`) are accounted for
- A grep-based check script exists to detect the anti-pattern
- `cargo check` passes with no errors
- `cargo test` passes with no failures
- `cargo doc` builds without warnings
- `grep -rn 'fn \w*_to_string' src/` returns zero results

## Validation Commands
- `cargo check` — Verify the project compiles
- `cargo test` — Run the full test suite
- `cargo doc` — Verify documentation builds cleanly
- `grep -rn 'fn \w*_to_string' src/` — Confirm no remaining `*_to_string` functions
- `grep -rn 'impl fmt::Display' src/` — Verify all Display implementations are accounted for
- `bash scripts/check-display-consistency.sh` — Run the anti-pattern detection script

## Notes
- The `ApiErrorResponse` `fmt::Display` implementation in `src/api/errors.rs` is NOT a duplicate — it serves a legitimate purpose for logging/debugging the error response structure. It does not correspond to a standalone `*_to_string` function.
- The `FromStr` implementations for `PlanStatus` and `TaskStatusValue` are the inverse of `Display` and are NOT part of this anti-pattern — they serve a different purpose (parsing strings into enum values).
- This plan is primarily about establishing convention and prevention, not fixing existing code (which was already fixed by 010.4).
- The grep-based script approach is preferred over a custom clippy lint because the pattern is simple and the script can provide contextual error messages. A custom clippy lint would require significant infrastructure investment for a narrow pattern.
- If the pattern reoccurs frequently, consider upgrading to a proper clippy plugin or a custom macro-gate that requires `Display` implementations for any new enum.
