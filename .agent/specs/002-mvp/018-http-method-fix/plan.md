# Plan: 018 - HTTP Method Fix

## Task Description
Fix HTTP method mismatch in REST API routes where PUT is used for plan and task updates, but the API design specifies PATCH. The handlers already implement partial update semantics (only fields that are `Some` are changed), so the route declarations just need to use the correct HTTP method.

## Objective
Align the REST API route declarations, OpenAPI specification, frontend API client, and tests to use PATCH instead of PUT for plan and task update endpoints, matching the API design specification.

## Problem Statement
The API router in `src/api/mod.rs` uses PUT for plan and task updates:
- Line 53: `.put(plans::update_plan)` should be `.patch(plans::update_plan)`
- Line 68: `.put(tasks::update_task)` should be `.patch(tasks::update_task)`

The handlers themselves do partial updates (only fields that are `Some` are changed), so the semantics are already PATCH. The route just uses the wrong HTTP method.

This mismatch affects:
1. **Backend router** — Wrong HTTP method in route declarations
2. **OpenAPI spec** — Documents `put` instead of `patch`
3. **Frontend API client** — Sends PUT requests instead of PATCH
4. **Tests** — Use PUT method assertions and helpers

## Solution Approach
1. Change `.put()` to `.patch()` in the API router for both update endpoints
2. Update the OpenAPI spec to use `patch` instead of `put` for both endpoints
3. Update the frontend API client to send PATCH requests
4. Update all test helpers and assertions to use PATCH
5. Update documentation comments in module files

## Relevant Files

### Files to Modify
- `src/api/mod.rs` — Change `.put()` to `.patch()` on lines 53 and 68; update doc comments on lines 30 and 32
- `docs/api/openapi.json` — Change `"put"` to `"patch"` for updatePlan (line 173) and updateTask (line 425)
- `web/src/lib/api.ts` — Change `'PUT'` to `'PATCH'` in `updatePlan` (line 80) and `updateTask` (line 127)
- `src/api/tests.rs` — Rename `put_json` helper to `patch_json`, update method string, and update all callers for update_plan/update_task tests
- `src/api/plans.rs` — Update module comment reference (line 10: PUT → PATCH)
- `src/api/tasks.rs` — Update module comment reference (line 10: PUT → PATCH)
- `app_docs/rest-api.md` — Update route documentation (PUT → PATCH for lines 28 and 39)

### No New Files Needed
No new files are required for this fix.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: api-method-builder
  - Role: Fix HTTP method declarations in router, OpenAPI spec, frontend client, and tests
  - Agent: builder

- **Validator**
  - Name: api-method-validator
  - Role: Verify all PUT→PATCH changes are consistent and tests pass
  - Agent: validator

- **Documenter**
  - Name: api-method-documenter
  - Role: Update documentation files to reflect the method change
  - Agent: documenter

## Step by Step Tasks

### 1. Fix API Router Methods
- **Task ID**: fix-router-methods
- **Depends On**: none
- **Assigned To**: api-method-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/mod.rs`, change line 53 from `.put(plans::update_plan)` to `.patch(plans::update_plan)`
  - In `src/api/mod.rs`, change line 68 from `.put(tasks::update_task)` to `.patch(tasks::update_task)`
  - Update doc comment on line 30: `GET/PUT/DELETE` → `GET/PATCH/DELETE` for plan CRUD
  - Update doc comment on line 32: `GET/PUT/DELETE` → `GET/PATCH/DELETE` for task CRUD
  - In `src/api/plans.rs`, update module comment line 10: `PUT` → `PATCH`
  - In `src/api/tasks.rs`, update module comment line 10: `PUT` → `PATCH`
- **Acceptance Criteria**:
  - `cargo check` passes with no errors
  - Router correctly registers PATCH routes for update_plan and update_task
  - All module doc comments reflect PATCH method

### 2. Update OpenAPI Specification
- **Task ID**: update-openapi-spec
- **Depends On**: fix-router-methods
- **Assigned To**: api-method-builder
- **Agent**: builder
- **Actions**:
  - In `docs/api/openapi.json`, change `"put"` to `"patch"` for the `updatePlan` operation at `/plans/{branch}/{plan_id}` (line 173)
  - In `docs/api/openapi.json`, change `"put"` to `"patch"` for the `updateTask` operation at `/plans/{branch}/{plan_id}/tasks/{task_id}` (line 425)
- **Acceptance Criteria**:
  - OpenAPI spec is valid JSON
  - `updatePlan` and `updateTask` operations use `patch` key
  - All other operations remain unchanged

### 3. Update Frontend API Client
- **Task ID**: update-frontend-client
- **Depends On**: fix-router-methods
- **Assigned To**: api-method-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/lib/api.ts`, change `updatePlan` function (line 80): `'PUT'` → `'PATCH'`
  - In `web/src/lib/api.ts`, change `updateTask` function (line 127): `'PUT'` → `'PATCH'`
- **Acceptance Criteria**:
  - `updatePlan` sends PATCH requests
  - `updateTask` sends PATCH requests
  - SvelteKit build passes

### 4. Update Test Helpers and Tests
- **Task ID**: update-tests
- **Depends On**: fix-router-methods
- **Assigned To**: api-method-builder
- **Agent**: builder
- **Actions**:
  - Rename `put_json` helper to `patch_json` in `src/api/tests.rs` (line 223)
  - Change method string from `"PUT"` to `"PATCH"` in the renamed helper (line 230)
  - Update `test_update_plan` to use `patch_json` instead of `put_json` (line 426)
  - Update `test_update_task` to use `patch_json` instead of `put_json` (line 624)
  - Update `test_auth_enabled_put_requires_auth` to use `"PATCH"` method (line 1387) and rename test to `test_auth_enabled_patch_update_requires_auth`
  - Update `test_auth_lifecycle_with_read` to use `"PATCH"` for plan update (line 1577)
  - Update `test_openapi_write_endpoints_have_security` to reference `"patch"` instead of `"put"` for plan and task update endpoints (lines 1732-1734, 1748-1750)
  - Verify no other uses of `put_json` remain (the helper is only used for update operations)
- **Acceptance Criteria**:
  - All tests compile and pass
  - No references to PUT remain for update_plan or update_task routes
  - Auth middleware test correctly tests PATCH method

### 5. Update Documentation
- **Task ID**: update-documentation
- **Depends On**: update-openapi-spec
- **Assigned To**: api-method-documenter
- **Agent**: documenter
- **Actions**:
  - In `app_docs/rest-api.md`, change line 28: `PUT` → `PATCH` for plan update endpoint
  - In `app_docs/rest-api.md`, change line 39: `PUT` → `PATCH` for task update endpoint
  - In `app_docs/openapi-auth-spec.md`, update references to PUT for updatePlan and updateTask (lines 62, 66)
- **Acceptance Criteria**:
  - Documentation accurately reflects PATCH method for update endpoints
  - No stale PUT references remain in documentation

### 6. Final Validation
- **Task ID**: validate-all
- **Depends On**: update-tests, update-documentation
- **Assigned To**: api-method-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` to verify compilation
  - Run `cargo test` to verify all tests pass
  - Verify no remaining PUT references for update operations in codebase (grep for `.put(` in api module, `'PUT'` in frontend api.ts, `"put"` in openapi.json for update operations)
  - Verify OpenAPI spec is valid JSON
  - Verify frontend build passes (`cd web && npx svelte-check`)

### 7. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: api-method-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- API router uses PATCH for plan and task update routes
- OpenAPI spec documents PATCH for updatePlan and updateTask operations
- Frontend API client sends PATCH requests for update operations
- All tests pass with PATCH method assertions
- Documentation comments reflect PATCH method
- No remaining PUT references for update_plan or update_task endpoints

## Validation Commands
- `cargo check` — Verify compilation
- `cargo test` — Run all tests
- `cd web && npx svelte-check` — Verify frontend type checking
- `grep -rn '\.put(' src/api/` — Verify no PUT routes remain for updates
- `grep -rn "'PUT'" web/src/lib/api.ts` — Verify no PUT calls remain in frontend
- `python3 -c "import json; json.load(open('docs/api/openapi.json'))"` — Verify OpenAPI JSON validity

## Notes
- This is a straightforward fix with no behavioral changes — the handler logic already implements partial updates (PATCH semantics)
- The auth middleware treats PATCH the same as PUT (both are write operations requiring authentication), so no auth changes are needed
- The `put_json` test helper is only used for update_plan and update_task tests, so renaming it to `patch_json` is appropriate
- Be careful to only change PUT→PATCH for update operations; PUT is not used elsewhere in the API router
- The OpenAPI spec test `test_openapi_write_endpoints_have_security` validates that write endpoints have security requirements; after this fix, it should reference `patch` instead of `put` for the two update endpoints
