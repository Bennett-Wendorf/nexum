# Plan: 002.7 - Remove Model Field from AgentRegistration

## Task Description
Remove the `model: Option<String>` field from the `AgentRegistration` struct in nexum's configuration schema. This field stores a hardcoded model name (e.g., "llama3.1") which is fragile because different agents have different model names. The ACP (Agent Client Protocol) spec handles model selection through session config options: the agent advertises available models during session setup, and the client picks one.

## Objective
Remove the `model` field from the configuration schema, API response types, tests, documentation, and all downstream references so that model selection is handled entirely by the ACP protocol rather than nexum's config.

## Problem Statement
The `model` field in `AgentRegistration` hardcodes a model name per agent registration. This is fragile because:
- Different ACP agents expose different model names
- Model names change over time (e.g., "llama3.1" → "llama3.2")
- The ACP spec already provides a proper mechanism for model selection via session config options
- Hardcoding model names in nexum's config creates unnecessary coupling between nexum and agent-specific details

## Solution Approach
Remove the `model` field from all layers of the codebase:
1. Remove from the schema struct (`AgentRegistration`)
2. Remove from the API response type (`AgentRegistrationResponse`)
3. Remove from the API handler mapping logic
4. Remove from the default config template
5. Remove from the example config
6. Update all test fixtures and assertions
7. Update the OpenAPI spec
8. Verify with `cargo test` and `cargo clippy`

## Relevant Files

### Existing Files to Modify
- `src/config/schema.rs` — Remove `model` field from `AgentRegistration` struct
- `src/config/loader.rs` — Remove `# model = ...` comment from default config template
- `src/config/tests.rs` — Remove all `model` field references in test fixtures and assertions
- `src/api/types.rs` — Remove `model` field from `AgentRegistrationResponse` struct
- `src/api/config.rs` — Remove `model` mapping in `list_agents` handler and update doc comment
- `examples/config.toml` — Remove `model = ...` lines from agent entries
- `docs/api/openapi.json` — Remove `model` property from `AgentRegistrationResponse` schema

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: schema-refactor-builder
  - Role: Remove the model field from schema, API types, handlers, tests, and docs
  - Agent: builder

- **Validator**
  - Name: schema-refactor-validator
  - Role: Verify compilation, tests, and clippy pass after the refactor
  - Agent: validator

- **Documenter**
  - Name: schema-refactor-documenter
  - Role: Update changelog and document the breaking config change
  - Agent: documenter

## Step by Step Tasks

### 1. Remove model field from AgentRegistration schema
- **Task ID**: remove-schema-model-field
- **Depends On**: none
- **Assigned To**: schema-refactor-builder
- **Agent**: builder
- **Actions**:
  - In `src/config/schema.rs`, remove lines 96-98 from `AgentRegistration`:
    ```rust
    /// Model selection (optional, agent-specific)
    #[serde(default)]
    pub model: Option<String>,
    ```
  - Verify the struct still compiles with the remaining fields
- **Acceptance Criteria**:
  - `AgentRegistration` no longer has a `model` field
  - `cargo check` succeeds

### 2. Update default config template
- **Task ID**: update-default-template
- **Depends On**: remove-schema-model-field
- **Assigned To**: schema-refactor-builder
- **Agent**: builder
- **Actions**:
  - In `src/config/loader.rs`, remove line 53 from `DEFAULT_CONFIG_TEMPLATE`:
    ```
    # model = "llama3.1"
    ```
  - The template should still be valid TOML parseable by the updated schema
- **Acceptance Criteria**:
  - Default template no longer references `model`
  - Template is still valid TOML

### 3. Update example configuration
- **Task ID**: update-example-config
- **Depends On**: remove-schema-model-field
- **Assigned To**: schema-refactor-builder
- **Agent**: builder
- **Actions**:
  - In `examples/config.toml`, remove the following lines:
    - Line 47: `model = "llama3.1"` (opencode-builder agent)
    - Line 56: `model = "llama3.1"` (opencode-planner agent)
    - Line 71: `model = "claude-sonnet-4-20250514"` (claude-assistant agent)
- **Acceptance Criteria**:
  - Example config no longer has `model` fields
  - Example config is still valid TOML

### 4. Remove model from API response types
- **Task ID**: remove-api-model-field
- **Depends On**: remove-schema-model-field
- **Assigned To**: schema-refactor-builder
- **Agent**: builder
- **Actions**:
  - In `src/api/types.rs`, remove lines 312-313 from `AgentRegistrationResponse`:
    ```rust
    /// Optional LLM model to use for the agent.
    pub model: Option<String>,
    ```
  - In `src/api/config.rs`, remove line 70 from the `list_agents` handler:
    ```rust
    model: agent.model.clone(),
    ```
  - In `src/api/config.rs`, update the doc comment on line 54 to remove `model` from the list of optional fields:
    - Change: `/// and optional fields (`model`, `tool_permissions`, `timeout_seconds`).`
    - To: `/// and optional fields (`tool_permissions`, `timeout_seconds`).`
- **Acceptance Criteria**:
  - `AgentRegistrationResponse` no longer has a `model` field
  - `list_agents` handler no longer maps `model`
  - Doc comment is updated

### 5. Update unit tests
- **Task ID**: update-tests
- **Depends On**: remove-schema-model-field, remove-api-model-field
- **Assigned To**: schema-refactor-builder
- **Agent**: builder
- **Actions**:
  - In `src/config/tests.rs`, update all test fixtures:
    - `test_agent_registration_parse` (line 50): Remove `model = "llama3.1"` from the TOML string; remove assertion on line 60
    - `test_validate_duplicate_agent_names` (line 152): Remove `model: None,` from agent fixture
    - `test_validate_empty_spawn_command` (line 176): Remove `model: None,` from agent fixture
    - `test_validate_whitespace_spawn_command` (line 198): Remove `model: None,` from agent fixture
    - `test_validate_valid_config` (line 242): Remove `model: None,` from agent fixture
    - `make_test_config` helper (lines 313, 322, 331): Remove `model: Some("llama3.1".to_string()),` and `model: None,` from all three agent registrations
  - After changes, all test `AgentRegistration` constructions should no longer include a `model` field
- **Acceptance Criteria**:
  - No test references `model` field
  - All tests compile and pass

### 6. Update OpenAPI specification
- **Task ID**: update-openapi-spec
- **Depends On**: remove-api-model-field
- **Assigned To**: schema-refactor-builder
- **Agent**: builder
- **Actions**:
  - In `docs/api/openapi.json`, remove the `model` property block from `AgentRegistrationResponse` schema (lines 1345-1348):
    ```json
    "model": {
      "type": ["string", "null"],
      "description": "Optional LLM model to use for the agent."
    },
    ```
  - Ensure the remaining properties (tool_permissions, timeout_seconds) maintain proper JSON formatting
- **Acceptance Criteria**:
  - OpenAPI spec no longer documents the `model` property
  - JSON remains valid

### 7. Final Validation
- **Task ID**: validate-all
- **Depends On**: update-tests, update-openapi-spec
- **Assigned To**: schema-refactor-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed with no errors
  - Run `cargo test` — all tests must pass
  - Run `cargo clippy --package nexum` — no warnings related to the removed field
  - Verify `AgentRegistration` struct has no `model` field
  - Verify `AgentRegistrationResponse` struct has no `model` field
  - Verify `DEFAULT_CONFIG_TEMPLATE` does not reference `model`
  - Verify `examples/config.toml` does not contain `model = ` lines
  - Verify `docs/api/openapi.json` does not contain `model` in AgentRegistrationResponse
  - Verify no remaining references to `agent.model` or `.model` in the Rust source code

### 8. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: schema-refactor-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation changes
  - Document the breaking config change (users with `model = ...` in their config.toml should remove it)
  - Note that model selection is now handled by the ACP protocol during session setup

## Acceptance Criteria
- `cargo check` succeeds with no errors
- `cargo test` passes all tests
- `cargo clippy --package nexum` produces no warnings
- `AgentRegistration` struct no longer has a `model` field
- `AgentRegistrationResponse` struct no longer has a `model` field
- Default config template does not reference `model`
- Example config does not contain `model` fields
- OpenAPI spec does not document the `model` property
- No remaining references to `agent.model` in the Rust source code
- The `list_agents` API handler no longer maps the `model` field

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo test` — Run full test suite
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `grep -r "\.model" src/` — Verify no remaining references to `.model` in source code
- `grep -r "model" examples/config.toml` — Verify no model fields in example config
- `grep -r "model" docs/api/openapi.json` — Verify no model in OpenAPI spec

## Notes
- This is a **breaking change** for users who have `model = ...` in their `~/.config/nexum/config.toml`. However, since the `model` field was `#[serde(default)]`, serde will silently ignore the field on deserialization — existing configs will still load without error. The field will simply be ignored.
- No migration is needed since serde's default behavior handles unknown fields gracefully.
- The ACP protocol handles model selection through session config options, which is the correct and more flexible approach.
- This change is scoped strictly to removing the `model` field. No other fields or functionality are affected.
