# Plan: 010 - Fix PlanList Loading Bug

## Task Description
Fix the `each_key_duplicate` runtime error in `PlanList.svelte` that occurs when the API returns plans with empty string IDs (`id: ""`). This error prevents the `finally` block from executing, leaving `loading` stuck at `true` and the page frozen on the loading spinner indefinitely.

## Objective
Restore the PlanList page to a functional state where it correctly renders all plans from the API, regardless of whether the plan markdown files contain ID metadata.

## Problem Statement
The `{#each planList as plan (plan.id)}` block in `PlanList.svelte` (line 89) uses `plan.id` as the Svelte keyed-`each` key. When the API returns plans where `plan.id` is `""` (empty string), all items share the same key, triggering Svelte's `each_key_duplicate` error. This error is thrown synchronously during rendering, which:

1. Prevents the `finally` block in `onMount` from executing
2. Leaves `loading = true` permanently
3. Freezes the page on the loading state

**Root cause**: The `list_plans` handler in `src/api/plans.rs` extracts `plan_id` from the directory slug (e.g., `PLAN-001` from `PLAN-001-design-consistency-fixes`), but then calls `read_plan()` which parses the markdown file. The markdown parser (`parse_plan_markdown`) extracts the ID from `**ID:**` metadata in the file. Plan markdown files that lack this metadata (e.g., plans created before the metadata format was established) return an empty string for `id`. The `plan_to_response()` function then uses this empty `plan.id` in the API response.

## Solution Approach
Two-part fix:

1. **Backend fix (primary)**: In `list_plans`, override `plan.id` with the slug-extracted `plan_id` before calling `plan_to_response()`. This ensures the API always returns the correct ID regardless of markdown file contents.

2. **Frontend fix (defensive)**: In `PlanList.svelte`, change the `{#each}` key from `plan.id` to use the loop index as a fallback when `plan.id` is empty. This provides a safety net against future regressions.

## Relevant Files

### Files to Modify
| File | Purpose |
|------|---------|
| `src/api/plans.rs` | Backend: Override `plan.id` with slug-extracted ID in `list_plans` |
| `web/src/pages/PlanList.svelte` | Frontend: Add defensive index fallback in `{#each}` key |

### Reference Files
| File | Purpose |
|------|---------|
| `src/persistence/markdown.rs` | Understand how `parse_plan_markdown` extracts ID from markdown |
| `src/persistence/directory.rs` | Understand `parse_slug` function that extracts plan_id from directory name |
| `web/src/components/PlanCard.svelte` | Uses `plan.id` in href — also benefits from backend fix |
| `web/src/lib/types.ts` | Plan interface definition |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: plan-list-builder
  - Role: Apply backend and frontend fixes for the PlanList loading bug
  - Agent: builder

- **Validator**
  - Name: plan-list-validator
  - Role: Verify the fix resolves the each_key_duplicate error and page renders correctly
  - Agent: validator

## Step by Step Tasks

### 1. Fix Backend: Override plan.id in list_plans
- **Task ID**: fix-backend-plan-id
- **Depends On**: none
- **Assigned To**: plan-list-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/plans.rs`, in the `list_plans` handler (line ~165-178), after `read_plan()` returns the plan, set `plan.id = plan_id.clone()` before calling `plan_to_response(&plan)`.
  - Specifically, add `plan.id = plan_id.clone();` between the `read_plan` call and the `plan_to_response` call.
  - The change should be in the loop body, approximately:
    ```rust
    let plan = read_plan(&state.repo_root, branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;
    plan.id = plan_id.clone(); // Ensure ID from slug is used
    // ... status filtering ...
    items.push(plan_to_response(&plan));
    ```
- **Acceptance Criteria**:
  - `cargo check` passes without errors
  - The `list_plans` handler returns correct plan IDs in the response
  - Existing tests pass (`cargo test`)

### 2. Fix Frontend: Defensive each key fallback
- **Task ID**: fix-frontend-each-key
- **Depends On**: none
- **Assigned To**: plan-list-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/pages/PlanList.svelte`, line 89, change the `{#each}` key expression to use index as fallback:
    ```svelte
    {#each planList as plan, index (plan.id || index)}
    ```
  - This ensures unique keys even if `plan.id` is empty or falsy.
- **Acceptance Criteria**:
  - The `{#each}` block renders without `each_key_duplicate` errors
  - Svelte compiler produces no warnings

### 3. Final Validation
- **Task ID**: validate-all
- **Depends On**: fix-backend-plan-id, fix-frontend-each-key
- **Assigned To**: plan-list-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo test` — all tests pass
  - Run `cd web && npm run check` — type checking passes
  - Run `cd web && npm run build` — build succeeds
  - Verify the `list_plans` response includes non-empty `id` for all plans
  - Verify PlanList page renders correctly with plans that have empty IDs in markdown

## Acceptance Criteria
- The `each_key_duplicate` error no longer occurs in PlanList.svelte
- The PlanList page renders all plans correctly, including those with missing ID metadata in their markdown files
- The `loading` state properly transitions to `false` after the API call completes
- PlanCard hrefs use the correct plan ID (not empty string)
- All existing tests pass
- No new compiler warnings or lint errors are introduced

## Validation Commands
- `cargo test` — Run Rust test suite
- `cd web && npm run check` — Run Svelte type checking
- `cd web && npm run build` — Build the frontend (verifies no runtime compilation errors)
- `cargo clippy -- -D warnings` — Run Rust linting (if configured)

## Notes
- The backend fix is the primary solution because it addresses the root cause: the API returning incorrect data. The frontend fix is a defensive measure only.
- The `plan.id` field is also used in `PlanCard.svelte` for the href URL (`/plans/${plan.branch}/${plan.id}`), so the backend fix benefits multiple components.
- No test file changes are expected — the existing `list_plans` test coverage should continue to pass since we're fixing the returned ID value to be correct.
- Plan markdown files created through `create_plan` already contain the `**ID: PLAN-XXX**` metadata (written by `render_plan_markdown`). This bug only affects pre-existing plan files that were created before the metadata format was established.
