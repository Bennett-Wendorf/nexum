# Plan: 021 - Config Patch Data-Loss Fix

## Task Description
Fix a data-loss bug in the `PATCH /api/config` endpoint where the handler serializes the entire in-memory `Config` struct to TOML and overwrites the config file, destroying any fields present on disk but not in the in-memory struct — notably the `agents` array, which can result in complete loss of agent registrations.

## Objective
Replace the full-struct serialization approach with a TOML-aware patch mechanism that mutates only the changed fields in the existing file, preserving all other content including comments, formatting, unknown fields, and the `agents` array.

## Problem Statement

The `patch_config` handler in `src/api/config.rs` (lines 88–123) performs a destructive full read-modify-write:

1. Updates one field in-memory (`cfg.preferences.yolo_mode = yolo_mode`)
2. Serializes the entire `Config` struct: `toml::to_string(&*cfg)`
3. Overwrites `~/.config/nexum/config.toml` with the serialized output

The root cause is that the in-memory `Config` struct may not contain all fields that exist on disk:
- The `config` crate (used to load config at startup) merges multiple sources (TOML file + environment variables). The in-memory state may contain env-var overrides that aren't in the file.
- Fields in the file that aren't in the `Config` struct schema (e.g., legacy `model = ...` fields, user-added custom fields) are silently dropped.
- If agents weren't loaded properly at startup, the `agents` array would be empty in memory, and writing it back would wipe all agent registrations.

**Impact**: Calling `PATCH /api/config` can permanently destroy the user's config file, including all agent registrations.

## Solution Approach

### Approach Evaluation

Three approaches were considered:

| Approach | Description | Pros | Cons |
|----------|-------------|------|------|
| **A — TOML-aware patch** | Parse file as `toml_edit::DocumentMut`, mutate only changed fields, write back | Preserves comments, formatting, unknown fields; safest; most maintainable | Requires `toml_edit` dependency |
| **B — Full reload** | Re-read file from disk, merge in-memory changes, write | No new deps; simpler | Still loses unknown fields; loses comments/formatting; bakes env-var overrides into file |
| **C — Explicit mapping** | Manually construct only changed fields and merge | Targeted | Complex, fragile, requires manual field mapping for each new field |

**Recommendation: Approach A** — TOML-aware patch using `toml_edit`.

Rationale:
1. **Safest**: Preserves ALL data on disk, including unknown fields and comments
2. **`toml_edit` is already in the dependency tree** (transitive dep of `toml` 0.8 via `config` crate) — adding it as a direct dep is straightforward
3. **Most maintainable**: When new config fields are added, the patch logic doesn't need updating
4. **Most user-friendly**: Preserves their formatting and comments
5. **Doesn't bake env-var overrides into the file**: Only changes what the user explicitly requested

### Implementation Details

The fix replaces the `toml::to_string(&*cfg)` serialization with a `toml_edit::DocumentMut` approach:

1. Read the existing config file as a string
2. Parse it as `toml_edit::DocumentMut`
3. Navigate to the target field (e.g., `preferences.yolo_mode`) and mutate it in the document
4. Serialize the document back to string and write to disk
5. Update the in-memory config struct for immediate consistency

## Relevant Files

### Files to Modify
- `src/api/config.rs` — Replace the serialization logic in `patch_config` (lines 104–112) with TOML-aware patching
- `Cargo.toml` — Add `toml_edit = "0.22"` dependency
- `src/api/tests.rs` — Add a regression test that verifies agents are preserved when patching `yolo_mode`
- `src/config/loader.rs` — Potentially add a `config_path_override` mechanism for testability

### No New Files Needed
No new files are required for this fix.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: config-patch-builder
  - Role: Implement TOML-aware patch logic and add dependency
  - Agent: builder

- **Validator**
  - Name: config-patch-validator
  - Role: Verify agents are preserved, all tests pass, no regressions
  - Agent: validator

- **Documenter**
  - Name: config-patch-documenter
  - Role: Update documentation for the config system
  - Agent: documenter

## Step by Step Tasks

### 1. Add `toml_edit` Dependency
- **Task ID**: add-toml-edit-dep
- **Depends On**: none
- **Assigned To**: config-patch-builder
- **Agent**: builder
- **Actions**:
  - Add `toml_edit = "0.22"` to `[dependencies]` in `Cargo.toml` (use the same major version as the transitive `toml_edit` already in `Cargo.lock`)
  - Run `cargo check` to verify the dependency resolves correctly
- **Acceptance Criteria**:
  - `toml_edit` is listed in `Cargo.toml` under `[dependencies]`
  - `cargo check` passes with no errors
  - `Cargo.lock` is updated

### 2. Implement TOML-Aware Patch in `patch_config`
- **Task ID**: implement-toml-patch
- **Depends On**: add-toml-edit-dep
- **Assigned To**: config-patch-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/config.rs`, add `use toml_edit;` import
  - Replace the serialization block (lines 104–112) with the following logic:
    1. Read the existing config file content: `tokio::fs::read_to_string(&config_path).await`
    2. Parse as `toml_edit::DocumentMut`: `existing.parse().map_err(...)`
    3. Navigate to the target field and mutate:
       ```rust
       doc.as_table_mut()
           .entry("preferences")
           .or_insert_with(toml_edit::Item::Table)
           .as_table_mut()
           .expect("preferences should be a table")
           .insert("yolo_mode", toml_edit::value(yolo_mode));
       ```
    4. Write back: `tokio::fs::write(&config_path, doc.to_string()).await`
  - Keep the in-memory update (`cfg.preferences.yolo_mode = yolo_mode`) for immediate consistency
  - Update the handler's doc comment to reflect the new approach
  - Ensure error handling uses `ApiError::Internal` with descriptive messages
- **Acceptance Criteria**:
  - `cargo check` passes
  - The `patch_config` handler reads the file, mutates only the target field in the TOML document, and writes it back
  - In-memory config is still updated for immediate consistency
  - Error handling covers: file read failure, TOML parse failure, file write failure
  - Comments and formatting in the config file are preserved after a patch

### 3. Make Config Path Testable
- **Task ID**: testable-config-path
- **Depends On**: add-toml-edit-dep
- **Assigned To**: config-patch-builder
- **Agent**: builder
- **Actions**:
  - In `src/config/loader.rs`, modify `config_path()` to check for a `NEXUM_CONFIG_FILE` environment variable that overrides the full config file path
  - If `NEXUM_CONFIG_FILE` is set, return that path directly
  - Otherwise, use the existing XDG-based resolution logic
  - This allows tests to set a temp directory path for the config file
- **Acceptance Criteria**:
  - `config_path()` returns the path from `NEXUM_CONFIG_FILE` env var when set
  - Falls back to existing XDG resolution when env var is not set
  - `cargo check` passes

### 4. Add Regression Test for Agent Preservation
- **Task ID**: add-regression-test
- **Depends On**: implement-toml-patch, testable-config-path
- **Assigned To**: config-patch-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/tests.rs`, add a new test `test_patch_config_preserves_agents`:
    1. Set up a temp directory with a config file that contains agents
    2. Set `NEXUM_CONFIG_FILE` env var to point to that config file
    3. Create a test app with the config loaded
    4. Call `PATCH /api/v1/config` with `{"yolo_mode": true}`
    5. Read the config file from disk and verify:
       - `yolo_mode` is `true`
       - The `agents` array is still present with all original agents
       - Comments in the file are preserved
    6. Clean up the env var
  - Also add `test_patch_config_preserves_comments` that verifies comment preservation
- **Acceptance Criteria**:
  - `test_patch_config_preserves_agents` passes
  - `test_patch_config_preserves_comments` passes
  - Both tests use isolated temp directories (no interference with real config)
  - `cargo test` passes for all tests

### 5. Update Existing Tests to Use Isolated Config
- **Task ID**: isolate-existing-tests
- **Depends On**: testable-config-path
- **Assigned To**: config-patch-builder
- **Agent**: builder
- **Actions**:
  - Review existing `test_patch_config_*` tests in `src/api/tests.rs`
  - Update them to use `NEXUM_CONFIG_FILE` pointing to a temp config file
  - Ensure the tests don't write to the user's real config directory
  - Verify all existing patch config tests still pass with the new approach
- **Acceptance Criteria**:
  - All existing `test_patch_config_*` tests pass
  - Tests use isolated temp directories for config files
  - No test writes to `~/.config/nexum/config.toml`

### 6. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-regression-test, isolate-existing-tests
- **Assigned To**: config-patch-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` to verify compilation
  - Run `cargo test` to verify all tests pass
  - Run `cargo test --lib config` to verify config-specific tests pass
  - Run `cargo test --lib api::tests` to verify API tests pass
  - Manually verify: create a config file with agents, call the patch endpoint, verify agents are preserved
  - Verify `toml_edit` dependency is in `Cargo.toml`
  - Grep for any remaining `toml::to_string` calls in the config patch handler

### 7. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: config-patch-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Update `app_docs/configuration-system.md` to document the TOML-aware patch approach
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `PATCH /api/config` preserves all fields in the config file that are not being modified
- Agent registrations are not lost when patching `yolo_mode`
- Comments and formatting in the config file are preserved after a patch
- Unknown fields (not in the `Config` schema) are preserved
- All existing tests pass
- New regression test `test_patch_config_preserves_agents` passes
- `toml_edit` dependency is added to `Cargo.toml`
- Config path is testable via `NEXUM_CONFIG_FILE` environment variable
- Tests use isolated temp directories (no writes to real config)

## Validation Commands
- `cargo check` — Verify compilation
- `cargo test` — Run all tests
- `cargo test test_patch_config_preserves_agents` — Run the regression test specifically
- `cargo test test_patch_config_` — Run all patch config tests
- `grep -n 'toml::to_string' src/api/config.rs` — Verify no `toml::to_string` remains in the patch handler
- `grep -n 'toml_edit' Cargo.toml` — Verify dependency is added

## Notes
- **Data loss risk**: This bug can cause permanent loss of agent registrations. The fix is critical and should be tested thoroughly.
- **Env-var overrides**: The `config` crate merges environment variables into the loaded config. The TOML-aware patch approach correctly avoids baking env-var overrides into the file — only the explicitly changed field is written.
- **Future extensibility**: When new patchable fields are added (e.g., `server_port`, `log_level`), the same `toml_edit` approach can be extended by adding more field mutations to the document.
- **`toml_edit` version**: Use `0.22` to match the transitive dependency already in `Cargo.lock` (from `toml` 0.8).
- **Thread safety**: The existing write lock (`state.config.write().await`) is maintained to prevent TOCTOU races during the read-modify-write sequence.
- **Backward compatibility**: This change is fully backward compatible — users who have unknown fields or custom comments in their config will now have them preserved instead of lost.
