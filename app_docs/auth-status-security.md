# Auth Status Security: Remove Key Names Exposure

## Overview

The `/api/v1/auth/status` endpoint is intentionally unauthenticated so clients can discover authentication requirements before making authenticated requests. A security review identified that the original `AuthStatusResponse` struct included a `key_names` field that revealed the names of all configured API keys to any network-accessible caller. This feature removes that field to prevent information leakage.

## What Was Changed and Why

### Security Rationale

API key names are identifying information. While not secrets themselves, they reveal the identities of configured API consumers. An attacker who can enumerate key names can:

- **Focus brute-force attacks** on specific key identities (e.g., "cli" keys are often reused across environments)
- **Correlate API key usage with system architecture** (e.g., a "slack-bot" key reveals a Slack integration)
- **Perform targeted credential stuffing** against known key identities

Since the auth status endpoint is unauthenticated by design, there is no access control to limit who sees this data, making the removal of `key_names` a necessary security hardening step.

### What Was Removed

- **`key_names: Vec<String>`** — The field that listed all configured API key names. This leaked identifying information to any caller.

### What Remains

The `AuthStatusResponse` struct retains three fields that provide useful client-facing information without leaking identities:

| Field | Type | Purpose |
|-------|------|---------|
| `enabled` | `bool` | Whether authentication is enabled on the server |
| `authenticate_read` | `bool` | Whether read (GET) endpoints also require authentication |
| `keys_count` | `usize` | Number of configured API keys (numeric, no identity leakage) |

## Technical Implementation

### Files Modified

| File | Change |
|------|--------|
| `src/api/auth.rs` | Removed `key_names` from `AuthStatusResponse` struct and `get_auth_status` handler. Added rustdoc comment explaining the omission. |
| `docs/api/openapi.json` | Removed `key_names` property from `AuthStatusResponse` schema. Updated `required` array to `["enabled", "authenticate_read", "keys_count"]`. |
| `src/api/tests.rs` | Added `test_auth_status_no_key_names` regression test. Added OpenAPI schema validation asserting `key_names` is absent. |
| `.agent/specs/002-mvp/007.2-api-authentication/plan.md` | Updated `AuthStatusResponse` struct definition and handler code. Added security note to plan. |
| `.agent/specs/002-mvp/unimplemented/007.3-openapi-auth/plan.md` | Updated `AuthStatusResponse` schema definition to omit `key_names`. |

### Key Functions and Types

**`AuthStatusResponse`** (in `src/api/auth.rs`) — Updated struct:
```rust
/// Response for GET /api/v1/auth/status
///
/// Returns authentication configuration without exposing sensitive details.
/// API key names are intentionally omitted to prevent information leakage.
/// Use `keys_count` to determine the number of configured keys.
#[derive(Serialize, Debug, Clone)]
pub struct AuthStatusResponse {
    pub enabled: bool,
    pub authenticate_read: bool,
    pub keys_count: usize,
}
```

**`get_auth_status`** (in `src/api/auth.rs`) — Handler returns only the three safe fields:
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

**`test_auth_status_no_key_names`** (in `src/api/tests.rs`) — Regression test verifying `key_names` is never serialized:
```rust
async fn test_auth_status_no_key_names() {
    // ... asserts body.get("key_names").is_none()
    // ... asserts body.get("keys_count").is_some()
}
```

### Dependencies Added

None. This change is purely a removal of an existing field.

## Usage

### Current API Response

The `/api/v1/auth/status` endpoint returns:
```json
{
  "enabled": true,
  "authenticate_read": false,
  "keys_count": 3
}
```

### How Clients Should Adapt

If your client previously parsed the `key_names` field from the auth status response, update it to use `keys_count` instead:

| Before (removed) | After (replacement) |
|---|---|
| `response.key_names` → `["cli", "bot"]` | `response.keys_count` → `2` |
| `response.key_names.len()` | `response.keys_count` |
| `response.key_names.contains("cli")` | Not available — clients should know their own key name from local config |

Clients that need to identify their own key should store the key name in their local configuration rather than querying the server. The server's auth status endpoint is a discovery mechanism, not an identity registry.

## Configuration

No new configuration options were introduced. The behavior is fixed — key names are never exposed through the auth status endpoint. This is a security decision, not a configurable option.

The configurable alternative (`expose_key_names: bool`) was considered and rejected because:
- It adds config complexity for minimal benefit
- Most users would not know to disable it, leaving them exposed
- The utility of key names in the auth status response is questionable (clients already know their own key name from config)

## Security Implications

### Improvements
- **Reduced information disclosure**: Attackers can no longer enumerate configured API key identities through the public auth status endpoint
- **Principle of least information**: The endpoint now exposes only what clients need to discover auth requirements
- **No regression risk**: The removal does not affect authentication functionality — only what is visible to unauthenticated callers

### Considerations
- **Breaking change**: Clients that parse `key_names` from the response will need to be updated. Since the API is in MVP/development stage, this is acceptable.
- **`keys_count` remains**: The numeric count is safe (it reveals no identities) and provides useful awareness for clients.
