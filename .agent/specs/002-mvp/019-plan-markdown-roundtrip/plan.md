# Plan: 019 - Fix plan.md markdown render/parse round-trip inconsistency

## Task Description
Fix the format inconsistency between `render_plan_markdown` and `parse_plan_markdown` in `src/persistence/markdown.rs`. The renderer outputs metadata with the value inside the bold markers (e.g., `**ID: PLAN-001**`), but the design spec and parser expect the value outside the bold markers (e.g., `**ID:** PLAN-001`).

## Objective
Align `render_plan_markdown` output with the documented format in `design/persistence.md` so that rendered markdown matches the expected template and round-trips correctly through parse → render → parse.

## Problem Statement
`render_plan_markdown` (lines 326-364) outputs metadata as:
- `**ID: PLAN-001**`
- `**Status: planning**`
- `**Created: 2024-01-01**`
- `**Branch: main**`

The design template (`design/persistence.md:111-114`) specifies the format as:
- `**ID:** PLAN-001`
- `**Status:** draft`
- `**Created:** 2026-05-21`
- `**Branch:** feature/auth-overhaul`

While `extract_metadata_value` currently handles both formats (since it strips `**` before matching), the rendered output does not conform to the documented spec. This creates a round-trip inconsistency where `render(parse(x))` ≠ the documented format, and future parser changes could break silently.

## Solution Approach
Update the four `format!` calls in `render_plan_markdown` to move the value outside the bold markers:
- `**ID: {}**` → `**ID:** {}`
- `**Status: {}**` → `**Status:** {}`
- `**Created: {}**` → `**Created:** {}`
- `**Branch: {}**` → `**Branch:** {}`

Add a round-trip test that verifies `parse(render(plan))` produces a plan with identical field values.

## Relevant Files
- `src/persistence/markdown.rs` — Lines 333-339: the `render_plan_markdown` function containing the four format strings to fix
- `src/persistence/tests.rs` — Line 123: existing `test_render_plan_markdown` test; needs a new round-trip test added
- `design/persistence.md` — Lines 108-129: the documented template format (reference only)

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: markdown-fix-builder
  - Role: Update render format strings and add round-trip test
  - Agent: builder

- **Validator**
  - Name: markdown-fix-validator
  - Role: Verify format matches design spec and round-trip test passes
  - Agent: validator

- **Documenter**
  - Name: markdown-fix-documenter
  - Role: Generate documentation for completed work
  - Agent: documenter

## Step by Step Tasks

### 1. Fix render_plan_markdown format strings
- **Task ID**: fix-render-format
- **Depends On**: none
- **Assigned To**: markdown-fix-builder
- **Agent**: builder
- **Actions**:
  - Open `src/persistence/markdown.rs`
  - On line 333, change `format!("**ID: {}**\n", plan.id)` to `format!("**ID:** {}\n", plan.id)`
  - On lines 334-337, change `format!("**Status: {}**\n", ...)` to `format!("**Status:** {}\n", ...)`
  - On line 338, change `format!("**Created: {}**\n", plan.created)` to `format!("**Created:** {}\n", plan.created)`
  - On line 339, change `format!("**Branch: {}**\n", plan.branch)` to `format!("**Branch:** {}\n", plan.branch)`
  - Verify the rendered output now matches the template in `design/persistence.md`
- **Acceptance Criteria**:
  - `render_plan_markdown` output matches the format in `design/persistence.md` lines 111-114
  - Each metadata line follows the pattern `**Label:** value` (bold wraps only the label+colon, value is outside bold)

### 2. Add round-trip test
- **Task ID**: add-roundtrip-test
- **Depends On**: fix-render-format
- **Assigned To**: markdown-fix-builder
- **Agent**: builder
- **Actions**:
  - In `src/persistence/tests.rs`, add a new test `test_plan_markdown_roundtrip`
  - Create a `Plan` instance with known field values
  - Call `render_plan_markdown(&plan)` to get markdown string
  - Write the markdown to a temp file and call `parse_plan_markdown(&path)` to parse it back
  - Assert that all fields (id, name, status, created, branch, goal, scope, background, tasks) match the original
- **Acceptance Criteria**:
  - Round-trip test passes: `parse(render(plan))` produces a plan with identical field values
  - Test compiles without errors

### 3. Final Validation
- **Task ID**: validate-all
- **Depends On**: fix-render-format, add-roundtrip-test
- **Assigned To**: markdown-fix-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all tests pass
  - Run `cargo clippy` — no warnings
  - Verify rendered plan markdown matches `design/persistence.md` template format
  - Verify the round-trip test exercises all metadata fields

### 4. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: markdown-fix-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `render_plan_markdown` outputs metadata in the format `**Label:** value` (matching `design/persistence.md`)
- All four metadata lines (ID, Status, Created, Branch) are updated
- A round-trip test is added that verifies `parse(render(plan))` preserves all field values
- All existing tests continue to pass
- `cargo clippy` reports no warnings

## Validation Commands
- `cargo test` — Run full test suite including the new round-trip test
- `cargo clippy` — Check for lint warnings
- `cargo test test_plan_markdown_roundtrip` — Run the specific round-trip test

## Notes
- The `render_task_markdown` function (lines 367-399) has the same format inconsistency (`**Parent plan: {}**`, `**Dependencies: {}**`). This is out of scope for this plan but should be tracked as a follow-up.
- The `extract_metadata_value` function strips `**` before matching, so both old and new formats parse correctly. The fix is about conformance to the documented spec, not about fixing a parsing bug.
- The existing `test_render_plan_markdown` test does not assert on exact format — it only checks substring presence. The new round-trip test provides stronger guarantees.
