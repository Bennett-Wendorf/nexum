# OpenAPI Auth Specification Update (Spec 007.3)

## Overview

Updated the OpenAPI 3.1 specification (`docs/api/openapi.json`) to document the authentication contract of the Nexum REST API. The spec previously had no security schemes, no security requirements on endpoints, and no path definition for the auth status endpoint. This update brings the machine-readable spec into alignment with the authentication design described in `design/backend-api.md`.

## What Was Built

The OpenAPI 3.1 spec was extended with authentication documentation:

- **Security schemes**: A `bearerAuth` scheme was added defining HTTP Bearer token authentication with API keys.
- **Security requirements**: All 9 write endpoints (POST, PATCH, DELETE) now declare `security: [{bearerAuth: []}]` and include a `401` response.
- **Auth status endpoint**: `GET /auth/status` was added as a new path, returning the current authentication configuration.
- **AuthStatusResponse schema**: A new schema defines the auth status response structure.
- **Auth tag**: A new `Auth` tag groups authentication-related operations.
- **Unauthorized response**: A reusable `401` response component was added for auth failures.
- **Info description**: The API description was updated to mention Bearer token authentication.
- **Validation tests**: 7 new tests were added to `src/api/tests.rs` to verify spec correctness.

## Technical Implementation

### Files Modified

| File | Changes |
|------|---------|
| `docs/api/openapi.json` | Added security schemes, security requirements, auth path, schema, tag, and response |
| `src/api/tests.rs` | Added 7 OpenAPI spec validation tests |

### Key Changes in `docs/api/openapi.json`

**Security Schemes** (`components.securitySchemes`):
```json
"securitySchemes": {
  "bearerAuth": {
    "type": "http",
    "scheme": "bearer",
    "bearerFormat": "api-key",
    "description": "API key authentication. Include the API key in the Authorization header..."
  }
}
```

**Auth Status Schema** (`components.schemas.AuthStatusResponse`):
```json
"AuthStatusResponse": {
  "type": "object",
  "required": ["enabled", "authenticate_read", "keys_count"],
  "properties": {
    "enabled": { "type": "boolean", "description": "Whether authentication is enabled on this server." },
    "authenticate_read": { "type": "boolean", "description": "Whether read (GET) endpoints also require authentication." },
    "keys_count": { "type": "integer", "format": "uint", "description": "Number of configured API keys. Key names are not exposed for security." }
  }
}
```

**Auth Status Path** (`paths./auth/status`):
- `GET /auth/status` — operationId: `getAuthStatus`, tagged with `Auth`, returns `AuthStatusResponse`
- Intentionally unauthenticated so clients can discover auth requirements before authenticating

**Write Endpoints with Security Requirements** (9 total):
1. `POST /plans` — `createPlan`
2. `PATCH /plans/{branch}/{plan_id}` — `updatePlan`
3. `DELETE /plans/{branch}/{plan_id}` — `deletePlan`
4. `PATCH /plans/{branch}/{plan_id}/status` — `transitionPlanStatus`
5. `POST /plans/{branch}/{plan_id}/tasks` — `createTask`
6. `PATCH /plans/{branch}/{plan_id}/tasks/{task_id}` — `updateTask`
7. `DELETE /plans/{branch}/{plan_id}/tasks/{task_id}` — `deleteTask`
8. `PATCH /plans/{branch}/{plan_id}/tasks/{task_id}/status` — `transitionTaskStatus`
9. `POST /plans/{branch}/{plan_id}/tasks/{task_id}/claim` — `claimTask`

Each write endpoint has:
- `"security": [{"bearerAuth": []}]`
- `"401": { "$ref": "#/components/responses/Unauthorized" }`

**Unauthorized Response** (`components.responses.Unauthorized`):
```json
"Unauthorized": {
  "description": "Authentication required or invalid credentials",
  "content": {
    "application/json": {
      "schema": { "$ref": "#/components/schemas/ApiErrorResponse" },
      "example": { "error": "Unauthorized", "message": "Missing Authorization header. API key required.", "status": 401 }
    }
  }
}
```

### New Tests in `src/api/tests.rs`

| Test | Purpose |
|------|---------|
| `test_openapi_json_valid` | Confirms spec is valid JSON with version 3.1.0 |
| `test_openapi_security_schemes` | Confirms `bearerAuth` exists with `type: "http"` and `scheme: "bearer"` |
| `test_openapi_auth_status_path` | Confirms `/auth/status` GET path exists with `Auth` tag |
| `test_openapi_write_endpoints_have_security` | Confirms all 9 write endpoints have `bearerAuth` security and `401` responses |
| `test_openapi_get_endpoints_no_security` | Confirms 10 GET endpoints (including `/health`, `/auth/status`) lack security requirements |
| `test_openapi_auth_tag` | Confirms `Auth` tag exists in `components.tags` |
| `test_openapi_unauthorized_response` | Confirms `Unauthorized` response exists in `components.responses` |

> **Note:** 6 of the 7 tests share a `load_openapi_spec()` helper function that loads and parses `docs/api/openapi.json` into a `serde_json::Value`, avoiding repetitive file-IO code across tests.

## Authentication Security Model

### How It Works

- **Bearer token authentication** via API key: `Authorization: Bearer <api-key>`
- **Write endpoints** (POST, PUT, PATCH, DELETE) require authentication by default
- **Read endpoints** (GET) do NOT require authentication by default (`authenticate_read: false`)
- **`/health`** and **`/auth/status`** are always unauthenticated (discoverability)
- If `authenticate_read` is set to `true` in config, GET endpoints also require auth (runtime behavior, not reflected in spec)

### Auth Status Endpoint

Call `GET /api/v1/auth/status` to discover authentication requirements before making authenticated requests:

```bash
curl http://localhost:3000/api/v1/auth/status
```

Response:
```json
{
  "enabled": true,
  "authenticate_read": false,
  "keys_count": 2
}
```

### Making Authenticated Requests

```bash
curl -H "Authorization: Bearer your-api-key" http://localhost:3000/api/v1/plans
```

## Usage

### Validating the Spec

**Quick JSON validation:**
```bash
python3 -c "import json; json.load(open('docs/api/openapi.json'))"
```

**Verify security schemes exist:**
```bash
python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'securitySchemes' in d['components']"
```

**Verify auth path exists:**
```bash
python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert '/auth/status' in d['paths']"
```

**Verify Auth tag exists:**
```bash
python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'Auth' in [t['name'] for t in d['components']['tags']]"
```

**Verify Unauthorized response exists:**
```bash
python3 -c "import json; d=json.load(open('docs/api/openapi.json')); assert 'Unauthorized' in d['components']['responses']"
```

**Run Rust tests:**
```bash
cargo test --package nexum api::tests::test_openapi
cargo test --package nexum api
cargo check
```

## Configuration

Authentication is controlled via the `authentication` section in the Nexum config:

| Setting | Default | Description |
|---------|---------|-------------|
| `enabled` | `false` | Whether authentication is active |
| `authenticate_read` | `false` | Whether GET endpoints also require auth |
| `api_keys` | `[]` | List of `{ name, secret }` API key entries |

The OpenAPI spec documents the **default behavior** (`authenticate_read: false`). If `authenticate_read` is enabled at runtime, GET endpoints also require authentication — this runtime override is not reflected in the static spec.
