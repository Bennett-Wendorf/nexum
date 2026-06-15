# Plan: 007.3 - OpenAPI Auth Specification Update

## Task Description
Update the OpenAPI 3.1 specification (`docs/api/openapi.json`) to include authentication documentation. The spec currently has no `securitySchemes` component for Bearer authentication, no `security` requirements on protected endpoints, and no path definition for `/api/v1/auth/status`. The design doc (`design/backend-api.md`) was updated with authentication documentation, but the machine-readable OpenAPI spec was not updated to match.

## Objective
Bring the OpenAPI 3.1 specification into alignment with the authentication design by:
- Adding a `securitySchemes` component defining `bearerAuth`
- Applying `security` requirements to all write endpoints (POST, PUT, PATCH, DELETE)
- Adding the `GET /api/v1/auth/status` path definition
- Adding an `Auth` tag to the tags list
- Adding an `Unauthorized` response to the shared responses
- Adding tests to verify the OpenAPI spec is valid JSON and matches the actual API routes

## Problem Statement
The OpenAPI 3.1 specification at `docs/api/openapi.json` is missing all authentication-related documentation. This means:
- API consumers cannot discover that Bearer token authentication is supported
- Clients cannot determine which endpoints require authentication
- The `/api/v1/auth/status` endpoint (planned in 007.2) is undocumented
- The spec does not match the actual API contract after authentication middleware is added
- Third-party integrations (Slack bot, CLI tool, custom dashboard) lack auth guidance
- The design doc (`design/backend-api.md`) describes authentication but the machine-readable spec does not reflect it

Per the API documentation requirement in `design/backend-api.md`:
> "Documentation lives as a machine-readable OpenAPI 3.1 spec at `docs/api/openapi.json`. Human-readable docs are generated from the spec."

## Solution Approach
Update the OpenAPI spec with the following changes:

1. **Add `securitySchemes` component**: Define a `bearerAuth` scheme using HTTP Bearer authentication with `scheme: "bearer"` and a description explaining the API key format.

2. **Add `security` to write endpoints**: Apply `security: [{bearerAuth: []}]` to all POST, PUT, PATCH, and DELETE operations. GET endpoints remain unauthenticated by default (matching the `authenticate_read: false` default in the auth config).

3. **Add `/auth/status` path**: Define `GET /auth/status` under the `paths` section with the `Auth` tag, returning `AuthStatusResponse` schema.

4. **Add `AuthStatusResponse` schema**: Define the response schema with fields `enabled`, `authenticate_read`, `keys_count`, and `key_names`.

5. **Add `Auth` tag**: Include the `Auth` tag in the `tags` array.

6. **Add `Unauthorized` response**: Add a reusable `Unauthorized` response to the `responses` section for 401 errors.

7. **Add tests**: Create a test that validates the OpenAPI spec is valid JSON and checks for the presence of required authentication elements.

## Relevant Files

### Existing Files
- `docs/api/openapi.json` — The OpenAPI 3.1 specification (primary target for edits)
- `design/backend-api.md` — Design doc describing authentication requirements
- `.agent/specs/002-mvp/unimplemented/007.2-api-authentication/plan.md` — Auth implementation plan (for alignment)
- `src/api/mod.rs` — Router definition (for route matching verification in tests)
- `src/api/errors.rs` — Error types including `ApiError::Unauthorized`
- `src/api/types.rs` — Response types including `AuthStatusResponse` (if added by 007.2)

### New Files (if needed)
- `tests/openapi_spec_test.rs` — Test file for OpenAPI spec validation (or added to existing test module)

### Modified Files
- `docs/api/openapi.json` — Add security schemes, security requirements, auth path, and responses
- `src/api/tests.rs` — Add OpenAPI spec validation test

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: openapi-builder
  - Role: Update the OpenAPI JSON spec with auth components, security requirements, and new paths
  - Agent: builder

- **Builder**
  - Name: openapi-tests-builder
  - Role: Write tests to validate the OpenAPI spec structure and consistency
  - Agent: builder

- **Validator**
  - Name: openapi-validator
  - Role: Verify the OpenAPI spec is valid JSON, matches the actual API routes, and passes all validation checks
  - Agent: validator

## Step by Step Tasks

### 1. Add Security Schemes Component
- **Task ID**: add-security-schemes
- **Depends On**: none
- **Assigned To**: openapi-builder
- **Agent**: builder
- **Actions**:
  - Add `securitySchemes` to the `components` section of `docs/api/openapi.json`:
    ```json
    "securitySchemes": {
      "bearerAuth": {
        "type": "http",
        "scheme": "bearer",
        "bearerFormat": "api-key",
        "description": "API key authentication. Include the API key in the Authorization header: 'Authorization: Bearer <api-key>'. Authentication is enabled via config and protects write endpoints by default."
      }
    }
    ```
  - Place `securitySchemes` alongside the existing `parameters`, `schemas`, `responses`, and `tags` objects within `components`.
- **Acceptance Criteria**:
  - `securitySchemes` object exists under `components`
  - `bearerAuth` scheme uses `type: "http"` with `scheme: "bearer"`
  - `bearerFormat` is set to `"api-key"`
  - Description explains the Bearer token format and config-driven nature
  - JSON remains valid after the change

### 2. Add Auth Status Response Schema
- **Task ID**: add-auth-status-schema
- **Depends On**: add-security-schemes
- **Assigned To**: openapi-builder
- **Agent**: builder
- **Actions**:
  - Add `AuthStatusResponse` to the `schemas` section:
    ```json
    "AuthStatusResponse": {
      "type": "object",
      "required": ["enabled", "authenticate_read", "keys_count", "key_names"],
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
        },
        "key_names": {
          "type": "array",
          "items": {
            "type": "string"
          },
          "description": "List of API key names (secrets are never exposed)."
        }
      }
    }
    ```
- **Acceptance Criteria**:
  - `AuthStatusResponse` schema exists under `components/schemas`
  - All four required fields are present: `enabled`, `authenticate_read`, `keys_count`, `key_names`
  - Types match the Rust implementation in plan 007.2
  - JSON remains valid

### 3. Add Auth Status Endpoint Path
- **Task ID**: add-auth-status-path
- **Depends On**: add-auth-status-schema
- **Assigned To**: openapi-builder
- **Agent**: builder
- **Actions**:
  - Add `/auth/status` path to the `paths` section:
    ```json
    "/auth/status": {
      "get": {
        "operationId": "getAuthStatus",
        "summary": "Get authentication status",
        "description": "Returns the current authentication configuration including whether auth is enabled, whether read endpoints require auth, and the number of configured API keys. API key secrets are never exposed. This endpoint is intentionally unauthenticated so clients can discover auth requirements before making authenticated requests.",
        "tags": ["Auth"],
        "responses": {
          "200": {
            "description": "Authentication status",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/AuthStatusResponse"
                }
              }
            }
          }
        }
      }
    }
    ```
  - Place this path alongside the other paths (e.g., after `/health` or before `/plans`).
- **Acceptance Criteria**:
  - `/auth/status` path exists in `paths`
  - Operation `GET /auth/status` has `operationId: "getAuthStatus"`
  - Uses `Auth` tag
  - Response references `AuthStatusResponse` schema
  - Description explains the endpoint is unauthenticated
  - JSON remains valid

### 4. Add Auth Tag
- **Task ID**: add-auth-tag
- **Depends On**: none
- **Assigned To**: openapi-builder
- **Agent**: builder
- **Actions**:
  - Add `Auth` tag to the `tags` array in `components`:
    ```json
    {
      "name": "Auth",
      "description": "Authentication status and configuration"
    }
    ```
  - Place it alongside the existing `Plans`, `Tasks`, `Execution`, and `Config` tags.
- **Acceptance Criteria**:
  - `Auth` tag exists in the `tags` array
  - Description is meaningful
  - JSON remains valid

### 5. Add Unauthorized Response
- **Task ID**: add-unauthorized-response
- **Depends On**: none
- **Assigned To**: openapi-builder
- **Agent**: builder
- **Actions**:
  - Add `Unauthorized` response to the `responses` section:
    ```json
    "Unauthorized": {
      "description": "Authentication required or invalid credentials",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/ApiErrorResponse"
          },
          "example": {
            "error": "Unauthorized",
            "message": "Missing Authorization header. API key required.",
            "status": 401
          }
        }
      }
    }
    ```
- **Acceptance Criteria**:
  - `Unauthorized` response exists under `components/responses`
  - References `ApiErrorResponse` schema
  - Example shows 401 status code
  - JSON remains valid

### 6. Apply Security Requirements to Write Endpoints
- **Task ID**: apply-security-requirements
- **Depends On**: add-security-schemes
- **Assigned To**: openapi-builder
- **Agent**: builder
- **Actions**:
  - Add `"security": [{"bearerAuth": []}]` to each write endpoint operation.
  
  **Write endpoints requiring security** (9 endpoints):
  1. `POST /plans` — `createPlan`
  2. `PUT /plans/{branch}/{plan_id}` — `updatePlan`
  3. `DELETE /plans/{branch}/{plan_id}` — `deletePlan`
  4. `PATCH /plans/{branch}/{plan_id}/status` — `transitionPlanStatus`
  5. `POST /plans/{branch}/{plan_id}/tasks` — `createTask`
  6. `PUT /plans/{branch}/{plan_id}/tasks/{task_id}` — `updateTask`
  7. `DELETE /plans/{branch}/{plan_id}/tasks/{task_id}` — `deleteTask`
  8. `PATCH /plans/{branch}/{plan_id}/tasks/{task_id}/status` — `transitionTaskStatus`
  9. `POST /plans/{branch}/{plan_id}/tasks/{task_id}/claim` — `claimTask`

  For each endpoint, add the `security` field and a `401` response:
  ```json
  "security": [{"bearerAuth": []}],
  "responses": {
    ...existing responses...,
    "401": {
      "$ref": "#/components/responses/Unauthorized"
    }
  }
  ```

  **GET endpoints** do NOT receive security requirements (matching the default `authenticate_read: false` config).
- **Acceptance Criteria**:
  - All 9 write endpoints have `security: [{"bearerAuth": []}]`
  - All 9 write endpoints have `401` response referencing `#/components/responses/Unauthorized`
  - GET endpoints do NOT have security requirements
  - `/auth/status` does NOT have security requirements (unauthenticated endpoint)
  - `/health` does NOT have security requirements (health check is always open)
  - JSON remains valid

### 7. Write OpenAPI Spec Validation Tests
- **Task ID**: openapi-spec-tests
- **Depends On**: apply-security-requirements
- **Assigned To**: openapi-tests-builder
- **Agent**: builder
- **Actions**:
  - Add a test module to `src/api/tests.rs` (or create a new file if appropriate) that validates the OpenAPI spec:
    
    **Test 1: Spec is valid JSON**
    ```rust
    /// Verify that docs/api/openapi.json is valid JSON.
    #[test]
    fn test_openapi_json_valid() {
        let spec_path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/api/openapi.json");
        let content = std::fs::read_to_string(spec_path).expect("Failed to read openapi.json");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("openapi.json is not valid JSON");
        assert_eq!(parsed["openapi"], "3.1.0", "OpenAPI version should be 3.1.0");
    }
    ```
    
    **Test 2: Security schemes exist**
    ```rust
    /// Verify that securitySchemes.bearerAuth exists in the spec.
    #[test]
    fn test_openapi_security_schemes() {
        let spec = load_openapi_spec();
        let schemes = &spec["components"]["securitySchemes"];
        assert!(schemes.is_object(), "securitySchemes should be an object");
        assert!(schemes["bearerAuth"].is_object(), "bearerAuth scheme should exist");
        assert_eq!(schemes["bearerAuth"]["type"], "http");
        assert_eq!(schemes["bearerAuth"]["scheme"], "bearer");
    }
    ```
    
    **Test 3: Auth status path exists**
    ```rust
    /// Verify that /auth/status path exists in the spec.
    #[test]
    fn test_openapi_auth_status_path() {
        let spec = load_openapi_spec();
        let auth_path = &spec["paths"]["/auth/status"]["get"];
        assert!(auth_path.is_object(), "/auth/status GET path should exist");
        assert_eq!(auth_path["operationId"], "getAuthStatus");
        assert!(auth_path["tags"].as_array().unwrap().iter().any(|t| t == "Auth"));
    }
    ```
    
    **Test 4: Write endpoints have security requirements**
    ```rust
    /// Verify that all write endpoints have security requirements.
    #[test]
    fn test_openapi_write_endpoints_have_security() {
        let spec = load_openapi_spec();
        let write_operations = [
            ("POST /plans", &spec["paths"]["/plans"]["post"]),
            ("PUT /plans/{branch}/{plan_id}", &spec["paths"]["/plans/{branch}/{plan_id}"]["put"]),
            ("DELETE /plans/{branch}/{plan_id}", &spec["paths"]["/plans/{branch}/{plan_id}"]["delete"]),
            ("PATCH /plans/{branch}/{plan_id}/status", &spec["paths"]["/plans/{branch}/{plan_id}/status"]["patch"]),
            ("POST /plans/{branch}/{plan_id}/tasks", &spec["paths"]["/plans/{branch}/{plan_id}/tasks"]["post"]),
            ("PUT /plans/{branch}/{plan_id}/tasks/{task_id}", &spec["paths"]["/plans/{branch}/{plan_id}/tasks/{task_id}"]["put"]),
            ("DELETE /plans/{branch}/{plan_id}/tasks/{task_id}", &spec["paths"]["/plans/{branch}/{plan_id}/tasks/{task_id}"]["delete"]),
            ("PATCH /plans/{branch}/{plan_id}/tasks/{task_id}/status", &spec["paths"]["/plans/{branch}/{plan_id}/tasks/{task_id}/status"]["patch"]),
            ("POST /plans/{branch}/{plan_id}/tasks/{task_id}/claim", &spec["paths"]["/plans/{branch}/{plan_id}/tasks/{task_id}/claim"]["post"]),
        ];
        for (name, op) in &write_operations {
            let security = op["security"].as_array()
                .unwrap_or_else(|| panic!("{} should have security array", name));
            assert!(!security.is_empty(), "{} should have security requirements", name);
            assert!(security[0].get("bearerAuth").is_some(), "{} should reference bearerAuth", name);
            // Also verify 401 response exists
            assert!(op["responses"]["401"].is_object(), "{} should have 401 response", name);
        }
    }
    ```
    
    **Test 5: GET endpoints do NOT have security requirements**
    ```rust
    /// Verify that GET endpoints do not have security requirements (default: authenticate_read=false).
    #[test]
    fn test_openapi_get_endpoints_no_security() {
        let spec = load_openapi_spec();
        let get_operations = [
            ("GET /health", &spec["paths"]["/health"]["get"]),
            ("GET /plans", &spec["paths"]["/plans"]["get"]),
            ("GET /plans/{branch}/{plan_id}", &spec["paths"]["/plans/{branch}/{plan_id}"]["get"]),
            ("GET /plans/{branch}/{plan_id}/tasks", &spec["paths"]["/plans/{branch}/{plan_id}/tasks"]["get"]),
            ("GET /plans/{branch}/{plan_id}/tasks/{task_id}", &spec["paths"]["/plans/{branch}/{plan_id}/tasks/{task_id}"]["get"]),
            ("GET /plans/{branch}/{plan_id}/execution", &spec["paths"]["/plans/{branch}/{plan_id}/execution"]["get"]),
            ("GET /running", &spec["paths"]["/running"]["get"]),
            ("GET /config", &spec["paths"]["/config"]["get"]),
            ("GET /agents", &spec["paths"]["/agents"]["get"]),
            ("GET /auth/status", &spec["paths"]["/auth/status"]["get"]),
        ];
        for (name, op) in &get_operations {
            assert!(op.get("security").is_none(), "{} should NOT have security requirements", name);
        }
    }
    ```
    
    **Test 6: Auth tag exists**
    ```rust
    /// Verify that the Auth tag exists in the spec.
    #[test]
    fn test_openapi_auth_tag() {
        let spec = load_openapi_spec();
        let tags = spec["components"]["tags"].as_array().expect("tags should be an array");
        let has_auth_tag = tags.iter().any(|t| t["name"] == "Auth");
        assert!(has_auth_tag, "Auth tag should exist in the spec");
    }
    ```
    
    **Test 7: Unauthorized response exists**
    ```rust
    /// Verify that the Unauthorized response exists in the spec.
    #[test]
    fn test_openapi_unauthorized_response() {
        let spec = load_openapi_spec();
        let unauthorized = &spec["components"]["responses"]["Unauthorized"];
        assert!(unauthorized.is_object(), "Unauthorized response should exist");
        assert_eq!(unauthorized["description"], "Authentication required or invalid credentials");
    }
    ```
    
    **Helper function**:
    ```rust
    fn load_openapi_spec() -> serde_json::Value {
        let spec_path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/api/openapi.json");
        let content = std::fs::read_to_string(spec_path).expect("Failed to read openapi.json");
        serde_json::from_str(&content).expect("openapi.json should be valid JSON")
    }
    ```
- **Acceptance Criteria**:
  - All 7 tests compile and pass
  - `test_openapi_json_valid` confirms spec is valid JSON with version 3.1.0
  - `test_openapi_security_schemes` confirms bearerAuth exists
  - `test_openapi_auth_status_path` confirms /auth/status path exists
  - `test_openapi_write_endpoints_have_security` confirms all 9 write endpoints have security
  - `test_openapi_get_endpoints_no_security` confirms GET endpoints lack security
  - `test_openapi_auth_tag` confirms Auth tag exists
  - `test_openapi_unauthorized_response` confirms Unauthorized response exists
  - Tests use `env!("CARGO_MANIFEST_DIR")` for path resolution

### 8. Final Validation
- **Task ID**: validate-openapi
- **Depends On**: openapi-spec-tests
- **Assigned To**: openapi-validator
- **Agent**: validator
- **Checks**:
  - Run `python3 -c "import json; json.load(open('docs/api/openapi.json'))"` — valid JSON
  - Run `cargo test --package nexum api::tests::test_openapi` — all OpenAPI tests pass
  - Run `cargo test --package nexum api` — all API tests pass (including existing tests)
  - Verify `securitySchemes.bearerAuth` exists with correct type/scheme
  - Verify all 9 write endpoints have `security: [{"bearerAuth": []}]`
  - Verify all 9 write endpoints have `401` response
  - Verify all GET endpoints do NOT have `security` field
  - Verify `/auth/status` path exists with correct schema reference
  - Verify `Auth` tag exists in tags array
  - Verify `Unauthorized` response exists in responses section
  - Verify `AuthStatusResponse` schema exists in schemas section
  - Verify spec version is `3.1.0`
  - Verify no broken `$ref` references
  - Verify JSON formatting is consistent (2-space indentation)
  - Verify the spec matches the actual routes in `src/api/mod.rs`

### 9. Documentation
- **Task ID**: openapi-docs
- **Depends On**: validate-openapi
- **Assigned To**: openapi-builder
- **Agent**: builder
- **Actions**:
  - Verify `design/backend-api.md` Authentication section is consistent with the updated OpenAPI spec
  - Ensure the spec description mentions authentication:
    - Update the `info.description` to mention Bearer token authentication
    - Add a note about the `/auth/status` endpoint
  - Verify the `info.description` field mentions authentication support

## Acceptance Criteria
- `docs/api/openapi.json` is valid JSON
- OpenAPI version is `3.1.0`
- `components.securitySchemes.bearerAuth` exists with `type: "http"`, `scheme: "bearer"`
- `AuthStatusResponse` schema exists in `components.schemas`
- `/auth/status` path exists with GET operation and `Auth` tag
- `Auth` tag exists in `components.tags`
- `Unauthorized` response exists in `components.responses`
- All 9 write endpoints (POST, PUT, PATCH, DELETE) have `security: [{"bearerAuth": []}]`
- All 9 write endpoints have `401` response referencing `#/components/responses/Unauthorized`
- GET endpoints do NOT have `security` requirements
- `/auth/status` does NOT have `security` requirements
- `/health` does NOT have `security` requirements
- All tests in the validation test suite pass
- `cargo test --package nexum api` passes all tests
- JSON formatting uses consistent 2-space indentation
- No broken `$ref` references

## Validation Commands
- `python3 -c "import json; json.load(open('docs/api/openapi.json'))"` — Verify valid JSON
- `python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'securitySchemes' in d['components']"` — Verify security schemes
- `python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert '/auth/status' in d['paths']"` — Verify auth path
- `python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'Auth' in [t['name'] for t in d['components']['tags']]"` — Verify Auth tag
- `python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'Unauthorized' in d['components']['responses']"` — Verify Unauthorized response
- `cargo test --package nexum api::tests::test_openapi` — Run OpenAPI spec tests
- `cargo test --package nexum api` — Run all API tests
- `cargo check` — Verify project compiles

## Notes
- This plan is a documentation-only update to the OpenAPI spec. It does not modify any Rust source code (except adding tests).
- **Dependency on 007.2**: This plan should be executed after or alongside 007.2 (API Authentication Middleware). The OpenAPI spec changes describe the authentication contract that 007.2 implements. If 007.2 is not yet implemented, the spec still correctly documents the intended API contract.
- **Security requirements scope**: Only write endpoints (POST, PUT, PATCH, DELETE) have security requirements. This matches the default `authenticate_read: false` configuration. If `authenticate_read` is set to `true` in config, GET endpoints also require auth, but the OpenAPI spec documents the default behavior.
- **Auth status endpoint**: The `/auth/status` endpoint is intentionally unauthenticated in the spec (no `security` field) because it must be discoverable before clients can authenticate.
- **Format consistency**: The existing spec uses 2-space indentation. All additions should follow this convention.
- **$ref references**: All `$ref` references must point to existing components. The `AuthStatusResponse` schema must exist before any path references it.
- **OpenAPI 3.1 features**: The spec uses OpenAPI 3.1.0 (not 3.0.x). Be aware that 3.1 uses JSON Schema 2020-12 for schemas.
