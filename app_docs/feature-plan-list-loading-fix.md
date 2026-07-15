# Fix PlanList Loading Bug

## Overview

Fixed a runtime crash in the PlanList page caused by plans with empty string IDs (`id: ""`) being returned from the API. The duplicate keys triggered Svelte's `each_key_duplicate` error, which prevented the `finally` block from executing and left the page permanently frozen on the loading spinner.

## What Was Built

A two-part fix addressing the bug at both the data source (backend) and the rendering layer (frontend), plus 22 code review improvements applied across five files during two rounds of review, followed by four additional fixes in a final review round.

### The Bug

- The `{#each planList as plan (plan.id)}` block in `PlanList.svelte` used `plan.id` as the keyed-`each` key.
- The `list_plans` API handler extracted `plan_id` from the directory slug (e.g., `PLAN-001`) but then called `read_plan()`, which parsed the markdown file and extracted the ID from `**ID:**` metadata.
- Plan markdown files created before the metadata format was established had no `**ID:**` field, so `read_plan()` returned an empty string for `id`.
- All plans with empty IDs shared the same key (`""`), triggering Svelte's `each_key_duplicate` error.
- The error was thrown synchronously during rendering, preventing the `finally` block from executing, leaving `loading` stuck at `true`.

### The Fix

**Backend (primary):** In `list_plans`, the plan ID extracted from the directory slug is now assigned back to `plan.id` before the response is serialized. This ensures the API always returns the correct ID regardless of markdown file contents.

**Frontend (defensive):** The original plan included a `crypto.randomUUID()` fallback for plans with empty IDs. However, since the backend fix guarantees correct IDs are always returned, the frontend fallback was not needed. The `{#each}` key remains `plan.id`, which is now always valid.

## Technical Implementation

### Files Modified

| File | Changes |
|------|---------|
| `src/api/plans.rs` | Backend ID override, removed duplicated helper functions, improved error handling, added race-condition guard, added input validation |
| `src/api/tasks.rs` | Removed duplicated helper functions, adopted `Display`/`FromStr` traits for `TaskStatusValue` |
| `src/persistence/schema.rs` | Added `Display` and `FromStr` trait implementations for `PlanStatus` and `TaskStatusValue` |
| `web/src/pages/PlanList.svelte` | Refactored loading function, cleaned up reactivity |

### Key Changes in `src/api/plans.rs`

- **Line 32** — Added `PLAN_ID_MUTEX` (`OnceLock<Mutex<()>>`) to guard plan ID generation against race conditions.
- **Lines 41–61** — Replaced `plan_status_display` helper with `PlanStatus::Display` trait (`plan.status.to_string()`).
- **Lines 63–97** — Removed `parse_plan_status_from_string` helper; replaced with `PlanStatus::FromStr` trait throughout.
- **Lines 77–84** — Replaced `unwrap_or_default()` with `match` + `tracing::warn!` + `continue` in `generate_plan_id` for proper error handling.
- **Line 141** — Added `tracing::warn!` (replacing `debug_assert_eq!`) to detect ID mismatches in all environments.
- **Line 142** — Added `plan.id = plan_id.clone();` to override the ID with the slug-extracted value (the primary fix).
- **Line 144** — `total += 1` now counts all readable plans before status filtering (unfiltered count).
- **Lines 201–205** — Added validation that `goal` is non-empty in `create_plan`.
- **Line 212** — Added `_guard` naming convention for mutex guard binding via `PLAN_ID_MUTEX`.
- **Lines 263–267** — Added validation that `goal` is non-empty in `update_plan`.
- **Lines 336–345** — Updated `transition_plan_status` to use `.to_string()` and `.as_str()` consistently, and `FromStr` trait for parsing.

### Key Changes in `src/api/tasks.rs`

- **Line 53** — Replaced `task_status_to_string(&status.status)` with `status.status.to_string()` in `task_to_response`.
- **Lines 82–101** — Removed `parse_task_status_value` helper function (replaced with `TaskStatusValue::FromStr` trait).
- **Line 168** — Updated `list_tasks` status filter to use `status.status.to_string()`.
- **Lines 465–471** — Updated `transition_task_status` to use `.to_string()`, `.as_str()`, and `FromStr` trait.
- **Line 557** — Updated `claim_task` status message to use `.to_string()`.

### Key Changes in `src/persistence/schema.rs`

- Added `use std::fmt;` and `use std::str::FromStr;` imports.
- Added `impl fmt::Display for PlanStatus` — converts enum variants to kebab-case strings.
- Added `impl FromStr for PlanStatus` — parses kebab-case strings back to enum variants.
- Added `impl fmt::Display for TaskStatusValue` — converts enum variants to kebab-case strings.
- Added `impl FromStr for TaskStatusValue` — parses kebab-case strings back to enum variants.

### Key Changes in `web/src/pages/PlanList.svelte`

- **Line 13** — Removed `$state` wrapper from `errorCleanup` (unnecessary reactivity for a function reference).
- **Lines 15–27** — Refactored `onMount` callback into a named `loadPlans()` function for cleaner structure.
- **Line 21** — `totalPlans` now derives from the API's `response.total` field via `$derived(totalFromApi)`.
- **Line 29** — `onMount(() => loadPlans())` invokes the named function.

### Key Changes in `web/src/components/Layout.svelte`

- **Line 22** — `agentsByType` converted to `$derived.by()` for proper reactive grouping of agents by type.

## Code Review Improvements

### First Round (11 items)

| # | Fix | File(s) |
|---|-----|---------|
| 1 | Removed `plan_status_display` helper — replaced with `PlanStatus::Display` trait | `plans.rs`, `schema.rs` |
| 2 | Removed `parse_plan_status_from_string` helper — replaced with `PlanStatus::FromStr` trait | `plans.rs`, `schema.rs` |
| 3 | Moved trait implementations to schema module — `Display`/`FromStr` live alongside type definitions | `schema.rs` |
| 4 | Proper error handling in `generate_plan_id` — replaced `unwrap_or_default()` with `match` + `tracing::warn!` + `continue` | `plans.rs` |
| 5 | Fixed `total` counting in `list_plans` — counter increments for all readable plans (unfiltered count) | `plans.rs` |
| 6 | Added `goal` validation in `create_plan` — rejects requests with empty goal fields | `plans.rs` |
| 7 | Added mutex guard for ID generation — `PLAN_ID_MUTEX` prevents concurrent plan ID generation races | `plans.rs` |
| 8 | Added `goal` validation in `update_plan` — rejects updates with empty goal fields | `plans.rs` |
| 9 | Refactored `onMount` to named `loadPlans()` function | `PlanList.svelte` |
| 10 | Removed `$state` from `errorCleanup` — function reference does not need reactive tracking | `PlanList.svelte` |
| 11 | Added `debug_assert_eq!` for ID mismatch detection in `list_plans` | `plans.rs` |

### Second Round (11 items)

| # | Fix | File(s) |
|---|-----|---------|
| 1 | Removed `parse_task_status_value` helper — replaced with `TaskStatusValue::FromStr` trait | `tasks.rs`, `schema.rs` |
| 2 | Added `Display` trait for `TaskStatusValue` — converts enum variants to kebab-case strings | `schema.rs` |
| 3 | Added `FromStr` trait for `TaskStatusValue` — parses kebab-case strings back to enum variants | `schema.rs` |
| 4 | Updated `task_to_response` to use `status.to_string()` instead of `task_status_to_string()` | `tasks.rs` |
| 5 | Updated `list_tasks` status filter to use `status.to_string()` | `tasks.rs` |
| 6 | Updated `transition_task_status` to use `.to_string()` and `.as_str()` consistently | `tasks.rs` |
| 7 | Updated `transition_task_status` to use `FromStr` trait for parsing target status | `tasks.rs` |
| 8 | Updated `claim_task` status message to use `.to_string()` | `tasks.rs` |
| 9 | Updated `transition_plan_status` to use `.to_string()` and `.as_str()` consistently | `plans.rs` |
| 10 | Updated `transition_plan_status` to use `FromStr` trait for parsing target status | `plans.rs` |
| 11 | Updated `validate_status_transition` calls to pass `.as_str()` references instead of owned strings | `plans.rs`, `tasks.rs` |

### Final Round (4 items)

| # | Fix | File(s) |
|---|-----|---------|
| 1 | `_guard` naming convention — mutex guard binding uses `_` prefix per Rust convention for unused bindings | `plans.rs` |
| 2 | `tracing::warn!` replacing `debug_assert_eq!` — ID mismatch detection now logs in all environments, not just debug builds | `plans.rs` |
| 3 | `totalPlans` now uses API `response.total` — frontend total derives directly from backend unfiltered count | `PlanList.svelte` |
| 4 | `agentsByType` converted to `$derived.by()` — proper reactive grouping of agents by type in Layout | `Layout.svelte` |

## Planned Follow-up Specs

The following specs have been created to address remaining issues identified during the final review round:

### 010.1: TOCTOU Race in `create_task`

Per-plan locking infrastructure to prevent time-of-check-to-time-of-use races when creating tasks within a plan.

### 010.2: TOCTOU Race in `delete_task`

Execution state atomicity — ensures task deletion and execution state updates occur atomically to prevent inconsistent state.

### 010.3: TOCTOU Race in `transition_task_status`

Task status map consistency — prevents races between status transitions and concurrent reads of the task status map.

### 010.4: Migrate `task_status_to_string` to `Display` trait

Consolidate remaining `task_status_to_string` usages in persistence modules to use the `Display` trait implementation already defined in `schema.rs`.

## Usage

No changes to end-user workflow or API contracts are required. The fix operates transparently:

- The PlanList page now renders all plans correctly, including those with missing ID metadata in their markdown files.
- PlanCard hrefs (`/plans/{branch}/{plan_id}`) now use the correct plan ID.
- The `loading` state properly transitions to `false` after the API call completes.

## Configuration

No configuration changes required.

## Verification

Run the following commands to verify the fix:

```bash
# Backend
cargo test          # All tests pass
cargo clippy -- -D warnings  # No lint warnings

# Frontend
cd web && npm run check   # Svelte type checking passes
cd web && npm run build   # Build succeeds without errors
```

To manually verify:
1. Start the server and navigate to the Plans page.
2. Confirm the page renders all plans without freezing on the loading spinner.
3. Confirm PlanCard links use valid plan IDs (non-empty strings).
