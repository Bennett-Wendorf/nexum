# Plan: 007.4 - Auth Status Security: Remove Key Names Exposure

## Task Description
A code review identified that the `AuthStatusResponse` struct in the planned `src/api/auth.rs` module includes a `key_names` field that reveals the names of all configured API keys to unauthenticated callers. While the secrets are not exposed, knowing key names helps an attacker narrow down which key identity to target (e.g., if they know "cli" is a key name, they can focus brute-force attacks on that specific identity). This plan removes the `key_names` field from the auth status endpoint response to reduce information leakage.

## Objective
Remove the `key_names` field from `AuthStatusResponse` and the `get_auth_status` handler to prevent exposure of API key identity names through the unauthenticated `/api/v1/auth/status` endpoint. Retain `keys_count` which provides useful awareness without leaking identifying information.

## Problem Statement
The `AuthStatusResponse` struct (defined in plan 007.2) includes a `key_names: Vec<String>` field that lists all configured API key names. The `/api/v1/auth/status` endpoint is intentionally unauthenticated so clients can discover auth requirements before making authenticated requests. This means:

1. **Any network-accessible caller** can enumerate all configured API key names
2. **Attack surface amplification**: Knowing key names lets attackers target specific identities (e.g., "cli" key is likely used more frequently and may have weaker secrets)
3. **Information disclosure**: Key names often reveal system architecture (e.g., "slack-bot", "web-ui", "ci-pipeline" reveal integration points)
4. **No compensating control**: The endpoint is unauthenticated by design, so there's no access control to limit who sees this data

The `keys_count` field remains useful — it tells clients how many keys are configured without revealing their identities.

## Solution Approach
**Recommended: Remove `key_names` entirely from `AuthStatusResponse`.**

### Trade-off Analysis

| Option | Security | Usability | Complexity | Risk |
|--------|----------|-----------|------------|------|
| **(a) Remove `key_names`** | ✅ Maximum | ⚠️ Clients can't identify keys by name | ✅ Simple | ✅ Lowest |
| **(b) Configurable via `expose_key_names`** | ⚠️ Default-off | ✅ Opt-in flexibility | ⚠️ Extra config field | ⚠️ Users may enable it |
| **(c) Docs warning only** | ❌ No change | ✅ No code changes | ✅ None | ❌ Highest |

**Why (a) is recommended:**
- The `keys_count` field already provides the useful "how many keys" information
- Key name enumeration provides minimal utility to legitimate clients (they already know their own key name from config)
- The security benefit of removing the field outweighs the minor usability loss
- Simpler code, fewer config options to document
- Follows the principle of least information disclosure

### What stays, what goes:
- **Removed**: `key_names: Vec<String>` — leaks identifying information
- **Kept**: `enabled: bool` — essential for client auth flow discovery
- **Kept**: `authenticate_read: bool` — essential for client auth flow discovery
- **Kept**: `keys_count: usize` — useful awareness without identity leakage

## Relevant Files

### Existing Files
- `.agent/specs/002-mvp/unimplemented/007.2-api-authentication/plan.md` — Contains the `AuthStatusResponse` definition that needs modification
- `.agent/specs/002-mvp/unimplemented/007.3-openapi-auth/plan.md` — Contains the OpenAPI spec that references `key_names`
- `src/api/types.rs` — If `AuthStatusResponse` is defined here instead of in `auth.rs`
- `docs/api/openapi.json` — OpenAPI spec that may include `key_names` in the schema

### Files to Create/Modify (when 007.2 is executed)
- `src/api/auth.rs` — `AuthStatusResponse` struct and `get_auth_status` handler (to be created by 007.2, modified by this plan)
- `src/api/tests.rs` — Auth status tests that reference `key_names` (to be updated)

### Modified Files
- `.agent/specs/002-mvp/unimplemented/007.2-api-authentication/plan.md` — Update `AuthStatusResponse` definition
- `.agent/specs/002-mvp/unimplemented/007.3-openapi-auth/plan.md` — Update OpenAPI schema definition
- `docs/api/openapi.json` — Update `AuthStatusResponse` schema (remove `key_names`)

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: auth-security-builder
  - Role: Remove `key_names` from spec plans, update code structs and handlers, update tests
  - Agent: builder

- **Validator**
  - Name: auth-security-validator
  - Role: Verify `key_names` is removed from all responses, tests pass, no regressions
  - Agent: validator

- **Documenter**
  - Name: auth-security-documenter
  - Role: Update documentation and OpenAPI spec to reflect the change
  - Agent: documenter

## Step by Step Tasks

### 1. Update Plan 007.2 AuthStatusResponse Definition
- **Task ID**: update-plan-0072
- **Depends On**: none
- **Assigned To**: auth-security-builder
- **Agent**: builder
- **Actions**:
  - In `.agent/specs/002-mvp/unimplemented/007.2-api-authentication/plan.md`:
    - Remove `key_names: Vec<String>` from the `AuthStatusResponse` struct definition
    - Update the struct comment from "List of key names (NOT secrets) for client identification" to removed
    - Update the `get_auth_status` handler code example to remove the `key_names` field assignment
    - Update the acceptance criteria to remove references to `key_names`
    - Add a security note in the plan's Notes section explaining why `key_names` was removed
  - Updated struct should look like:
    ```rust
    /// Response for GET /api/v1/auth/status
    #[derive(Serialize, Debug, Clone)]
    pub struct AuthStatusResponse {
        /// Whether authentication is enabled
        pub enabled: bool,
        /// Whether read endpoints require authentication
        pub authenticate_read: bool,
        /// Number of configured API keys
        pub keys_count: usize,
    }
    ```
  - Updated handler should look like:
    ```rust
    pub async fn get_auth_status(
        State(state): State<AppState>,
    ) -> Json<AuthStatusResponse> {
        let auth = &state.config.global.authentication;
        Json(AuthStatusResponse {
            enabled: auth.enabled,
            authenticate_read: auth.authenticate_read,
            keys_count: auth.api_keys.len(),
        })
    }
    ```
- **Acceptance Criteria**:
  - `AuthStatusResponse` struct in plan 007.2 has no `key_names` field
  - `get_auth_status` handler in plan 007.2 does not populate `key_names`
  - Plan notes section includes security rationale for the removal
  - All other fields (`enabled`, `authenticate_read`, `keys_count`) remain unchanged

### 2. Update Plan 007.3 OpenAPI Schema Definition
- **Task ID**: update-plan-0073
- **Depends On**: none
- **Assigned To**: auth-security-builder
- **Agent**: builder
- **Actions**:
  - In `.agent/specs/002-mvp/unimplemented/007.3-openapi-auth/plan.md`:
    - Remove `key_names` from the `AuthStatusResponse` schema in the OpenAPI spec example
    - Update the `required` array to remove `key_names`
    - Update the schema description to note that key names are not exposed
  - Updated schema should look like:
    ```json
    "AuthStatusResponse": {
      "type": "object",
      "required": ["enabled", "authenticate_read", "keys_count"],
      "properties": {
        "enabled": {
          "type": "boolean",
          "description": "Whether authentication is enabled on this server."
        },
        "authenticate_read": {
          "type": "boolean",
          "description": "Whether read (GET) endpoints also require authentication."
        },
        "keys_count": {
          "type": "integer",
          "format": "uint",
          "description": "Number of configured API keys."
        }
      }
    }
    ```
- **Acceptance Criteria**:
  - `AuthStatusResponse` schema in plan 007.3 has no `key_names` property
  - `required` array does not include `key_names`
  - Schema has exactly 3 properties: `enabled`, `authenticate_read`, `keys_count`

### 3. Update Auth Status Tests
- **Task ID**: update-auth-tests
- **Depends On**: update-plan-0072
- **Assigned To**: auth-security-builder
- **Agent**: builder
- **Actions**:
  - In plan 007.2's test section (Task 4):
    - Remove any test assertions that check for `key_names` in the auth status response
    - Add a new test: `test_auth_status_no_key_names` that verifies the response does NOT contain a `key_names` field:
      ```rust
      /// Verify auth status response does not expose API key names.
      #[tok::test]
      async fn test_auth_status_no_key_names() {
          let (app, _temp_dir) = test_app_with_auth();
          let req = Request::builder()
              .uri("/api/v1/auth/status")
              .body(Body::empty())
              .unwrap();
          let res = app.clone().oneshot(req).await.unwrap();
          let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
          let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
          assert!(body.get("key_names").is_none(), "Auth status should not expose key_names");
          assert!(body.get("keys_count").is_some(), "Auth status should include keys_count");
          assert_eq!(body["keys_count"], 1); // matches test config with 1 key
      }
      ```
    - Update `test_auth_enabled_auth_status_no_auth` to verify response structure without `key_names`
- **Acceptance Criteria**:
  - No test asserts on `key_names` field existence or values
  - `test_auth_status_no_key_names` test verifies `key_names` is absent from response
  - Test verifies `keys_count` is still present and correct
  - All auth status tests pass without `key_names` references

### 4. Update OpenAPI Spec File
- **Task ID**: update-openapi-spec
- **Depends On**: update-plan-0073
- **Assigned To**: auth-security-builder
- **Agent: builder
- **Actions**:
  - In `docs/api/openapi.json`:
    - Remove `key_names` from the `AuthStatusResponse` schema's `properties`
    - Remove `key_names` from the `AuthStatusResponse` schema's `required` array
    - Update the schema description to note key names are not exposed for security
  - Verify JSON remains valid after changes
- **Acceptance Criteria**:
  - `AuthStatusResponse` schema has no `key_names` property
  - `required` array has exactly 3 items: `enabled`, `authenticate_read`, `keys_count`
  - JSON is valid (passes `python3 -c "import json; json.load(open('docs/api/openapi.json'))"`)
  - No other schemas are affected

### 5. Final Validation
- **Task ID**: validate-auth-security
- **Depends On**: update-auth-tests, update-openapi-spec
- **Assigned To**: auth-security-validator
- **Agent**: validator
- **Checks**:
  - Verify `AuthStatusResponse` in plan 007.2 has no `key_names` field
  - Verify `AuthStatusResponse` schema in plan 007.3 has no `key_names` property
  - Verify `docs/api/openapi.json` `AuthStatusResponse` schema has no `key_names`
  - Verify no Rust code references `key_names` in the auth module context:
    ```bash
    grep -r "key_names" src/api/  # Should return nothing
    ```
  - Verify `keys_count` field still exists in all relevant locations
  - Verify plan 007.2 notes section includes security rationale
  - Verify `test_auth_status_no_key_names` test exists in plan 007.2 test section
  - Verify OpenAPI spec is valid JSON
  - Verify no broken `$ref` references in OpenAPI spec
  - Verify all other `AuthStatusResponse` fields (`enabled`, `authenticate_read`, `keys_count`) are intact

### 6. Documentation
- **Task ID**: auth-security-docs
- **Depends On**: validate-auth-security
- **Assigned To**: auth-security-documenter
- **Agent**: documenter
- **Actions**:
  - Update `design/backend-api.md` Authentication section to note:
    ```markdown
    ### Security Considerations

    The `/api/v1/auth/status` endpoint exposes only `enabled`, `authenticate_read`,
    and `keys_count`. API key names are intentionally NOT exposed to prevent
    information leakage that could help attackers target specific key identities.
    Use `keys_count` to determine how many keys are configured.
    ```
  - Add rustdoc comment to `AuthStatusResponse` in the plan:
    ```rust
    /// Response for GET /api/v1/auth/status
    ///
    /// Returns authentication configuration without exposing sensitive details.
    /// API key names are intentionally omitted to prevent information leakage.
    /// Use `keys_count` to determine the number of configured keys.
    #[derive(Serialize, Debug, Clone)]
    pub struct AuthStatusResponse { ... }
    ```
  - Update this plan's notes section with the security rationale

## Acceptance Criteria
- `AuthStatusResponse` struct has no `key_names` field in plan 007.2
- `AuthStatusResponse` handler does not populate `key_names` in plan 007.2
- `AuthStatusResponse` OpenAPI schema has no `key_names` property in plan 007.3
- `docs/api/openapi.json` `AuthStatusResponse` schema has no `key_names` property
- `keys_count` field remains present and functional in all locations
- `enabled`, `authenticate_read`, `keys_count` fields are all present and correct
- No Rust source code references `key_names` in auth context
- `test_auth_status_no_key_names` test verifies absence of `key_names`
- All existing auth status tests pass without `key_names` references
- OpenAPI spec is valid JSON with no broken references
- `design/backend-api.md` documents the security rationale
- Plan 007.2 notes section includes security rationale for the removal
- `cargo check` succeeds (when code is implemented)

## Validation Commands
- `grep -r "key_names" src/api/` — Should return nothing (no key_names references)
- `grep -r "keys_count" src/api/` — Should find references (keys_count still exists)
- `python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'key_names' not in d['components']['schemas']['AuthStatusResponse']['properties']"` — Verify no key_names in spec
- `python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'keys_count' in d['components']['schemas']['AuthStatusResponse']['properties']"` — Verify keys_count exists
- `grep -r "key_names" .agent/specs/` — Should find only in plan files referencing removal rationale, not in code examples
- `cargo check` — Verify Rust project compiles (when code is implemented)
- `cargo test --package nexum api` — Run all API tests (when code is implemented)

## Notes
- **Dependency on 007.2**: This plan modifies the spec defined in 007.2. If 007.2 has already been executed, the changes in this plan need to be applied as a patch to the existing `src/api/auth.rs` file. If 007.2 has NOT been executed yet, this plan's changes should be incorporated into the 007.2 plan before execution.
- **Execution order**: This plan (007.4) should be executed BEFORE 007.2 if possible, so the corrected spec is used during implementation. If 007.2 is already implemented, execute this plan as a fix/patch.
- **Security rationale**: API key names are considered identifying information. While not secrets themselves, they reveal the identities of configured API consumers. An attacker who knows key names can:
  - Focus brute-force attacks on specific key identities (e.g., "cli" keys are often reused)
  - Correlate API key usage with system architecture (e.g., "slack-bot" reveals Slack integration)
  - Perform targeted credential stuffing against known key identities
- **Why `keys_count` remains**: The count provides useful awareness for clients (e.g., "how many keys are configured?") without revealing any identifying information. It's a numeric value that doesn't leak system architecture.
- **Configurable alternative considered and rejected**: Making `key_names` exposure configurable (`expose_key_names: bool`) was considered but rejected because:
  - It adds config complexity for minimal benefit
  - Most users won't know to disable it, leaving them exposed
  - The utility of key names in the auth status response is questionable (clients already know their own key name from their config)
  - Simpler to remove entirely than to add a config toggle
- **Backward compatibility**: If this plan is applied after 007.2 is already deployed, it's a breaking API change (removing a response field). Clients that parse `key_names` will need to be updated. However, since the API is in MVP/development stage, this is acceptable. The deprecation path would be: add `#[serde(skip_serializing_if = "...")]` or a deprecation warning header, then remove in a future version.
