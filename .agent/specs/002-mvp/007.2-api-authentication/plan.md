# Plan: 007.2 - API Authentication Middleware

## Task Description
Add a configurable authentication middleware layer to the Nexum REST API. Currently all API endpoints are publicly accessible with no authentication or authorization. The `ApiError::Unauthorized` variant exists in `src/api/errors.rs` but is never used. This plan adds API key-based authentication with a config-driven on/off toggle, enabling production readiness while maintaining the localhost-only MVP default.

## Objective
Create an authentication layer in `src/api/` that:
- Adds authentication configuration fields to the config schema (`~/.config/nexum/config.toml`)
- Provides an API key validation middleware that can be enabled/disabled via config
- Integrates with the existing `ApiError::Unauthorized` error variant
- Guards write operations (POST, PUT, PATCH, DELETE) while optionally leaving read operations (GET) open
- Exposes an authentication status endpoint for clients to check auth requirements
- Maintains backward compatibility: auth is disabled by default (MVP localhost behavior)
- Supports multiple API keys for different clients (e.g., Slack bot, CLI tool, web UI)
- Adds `Authorization: Bearer <api-key>` header-based authentication
- Includes comprehensive tests for both authenticated and unauthenticated flows

## Problem Statement
The Nexum REST API has no authentication mechanism. Every endpoint is publicly accessible, meaning anyone with network access can read and modify plans, tasks, and configuration. The `ApiError::Unauthorized` variant exists but is dead code. While this is acceptable for an MVP running on localhost (per `design/backend-api.md` and `design/tech-stack.md`), it blocks:
- Remote access to the API (e.g., from a CI/CD pipeline or remote dashboard)
- Multi-tenant or multi-user scenarios
- Production deployment behind a reverse proxy
- Third-party integrations over the network (Slack bot, Discord bot)

The design docs explicitly call this out:
> "MVP: localhost-only, no auth required. Future: API key or token-based auth for remote access."

This plan implements that future capability as a self-contained, backward-compatible enhancement.

## Solution Approach
Build an `auth` submodule in `src/api/` with the following components:

1. **`src/api/auth.rs`** — Authentication types, API key storage, and middleware logic
2. **Config schema extension** — New `AuthenticationSettings` struct in `src/config/schema.rs`
3. **Middleware integration** — Wired into the router in `src/api/mod.rs` via Axum's layer system
4. **Endpoint classification** — Write endpoints (POST/PUT/PATCH/DELETE) require auth when enabled; read endpoints (GET) remain open by default
5. **Auth status endpoint** — `GET /api/v1/auth/status` returns current auth configuration (without exposing keys)

The authentication design follows these principles:
- **API key auth**: Simple bearer token in `Authorization: Bearer <key>` header
- **Config-driven**: Enabled/disabled via `~/.config/nexum/config.toml`
- **Multiple keys**: Support multiple named API keys (e.g., `cli`, `slack-bot`, `web-ui`)
- **Selective guarding**: Configurable whether GET endpoints require auth (`authenticate_read: false` default)
- **Non-intrusive**: When disabled, middleware is a no-op (zero overhead)
- **Backward compatible**: Default config has auth disabled, preserving MVP localhost behavior

## Relevant Files

### Existing Files
- `src/api/mod.rs` — Router construction (needs auth middleware layer added)
- `src/api/errors.rs` — `ApiError::Unauthorized` variant exists, needs to be used
- `src/api/middleware.rs` — Existing middleware (request ID); auth middleware added alongside
- `src/api/types.rs` — `AppState` struct (needs auth config field)
- `src/config/schema.rs` — Config schema (needs `AuthenticationSettings` struct)
- `src/api/tests.rs` — Integration tests (need auth test cases added)
- `design/backend-api.md` — Documents "MVP: localhost-only, no auth required"
- `design/tech-stack.md` — Notes "Auth — needed when accessing nexum over network"
- `Cargo.toml` — May need `uuid` crate for key generation

### New Files (if needed)
- `src/api/auth.rs` — Authentication types, middleware, and key validation logic

### Modified Files
- `src/api/mod.rs` — Add auth middleware layer to router
- `src/api/types.rs` — Add `AuthStatusResponse` DTO
- `src/config/schema.rs` — Add `AuthenticationSettings` to `GlobalSettings`
- `src/api/tests.rs` — Add authentication test cases
- `Cargo.toml` — Add `uuid` dependency for API key generation

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: auth-core-builder
  - Role: Implement auth types, config schema extension, and middleware
  - Agent: builder

- **Builder**
  - Name: auth-integration-builder
  - Role: Wire auth into router, add auth status endpoint, update AppState
  - Agent: builder

- **Builder**
  - Name: auth-tests-builder
  - Role: Write authentication integration tests
  - Agent: builder

- **Validator**
  - Name: auth-validator
  - Role: Verify auth middleware correctness, config parsing, and endpoint protection
  - Agent: validator

## Step by Step Tasks

### 1. Add Authentication Configuration to Config Schema
- **Task ID**: auth-config-schema
- **Depends On**: none
- **Assigned To**: auth-core-builder
- **Agent**: builder
- **Actions**:
  - Add `AuthenticationSettings` struct to `src/config/schema.rs`:
    ```rust
    /// Authentication settings for the REST API.
    ///
    /// Controls whether API key authentication is required and which keys
    /// are accepted. When disabled (default), all endpoints are publicly
    /// accessible — suitable for localhost-only MVP deployment.
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct AuthenticationSettings {
        /// Whether authentication is enabled. Defaults to false for MVP.
        #[serde(default)]
        pub enabled: bool,

        /// Whether GET (read) endpoints also require authentication.
        /// Defaults to false: writes require auth, reads do not.
        #[serde(default)]
        pub authenticate_read: bool,

        /// Named API keys. Each key has a name (for logging/identification)
        /// and a secret value. Empty list means no keys configured.
        #[serde(default)]
        pub api_keys: Vec<ApiKeyEntry>,
    }

    /// A single named API key entry.
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct ApiKeyEntry {
        /// Human-readable name for this key (e.g., "cli", "slack-bot")
        pub name: String,

        /// The API key secret value
        pub secret: String,
    }

    impl Default for AuthenticationSettings {
        fn default() -> Self {
            Self {
                enabled: false,
                authenticate_read: false,
                api_keys: Vec::new(),
            }
        }
    }
    ```
  - Add `authentication` field to `GlobalSettings`:
    ```rust
    /// Authentication settings (API keys, enabled/disabled).
    /// Defaults to disabled (no auth) for localhost MVP deployment.
    #[serde(default)]
    pub authentication: AuthenticationSettings,
    ```
  - Update `GlobalSettings::default()` to include `authentication: AuthenticationSettings::default()`
  - Update `ConfigResponse` in `src/api/types.rs` to include auth status (NOT keys):
    ```rust
    /// Whether authentication is enabled on this server
    pub auth_enabled: bool,
    /// Whether read endpoints require authentication
    pub auth_require_read: bool,
    /// Number of configured API keys (for client awareness)
    pub auth_keys_count: usize,
    ```
  - Update `get_config` handler in `src/api/config.rs` to populate these new fields
- **Acceptance Criteria**:
  - `AuthenticationSettings` compiles with serde derive macros
  - `AuthenticationSettings::default()` returns `enabled: false`, `authenticate_read: false`, empty `api_keys`
  - `GlobalSettings` includes `authentication` field with proper serde default
  - `ConfigResponse` includes `auth_enabled`, `auth_require_read`, `auth_keys_count` fields
  - Config TOML example works:
    ```toml
    [global.authentication]
    enabled = true
    authenticate_read = false
    api_keys = [
      { name = "cli", secret = "my-secret-key-1" },
      { name = "slack-bot", secret = "my-secret-key-2" },
    ]
    ```
  - `cargo check` succeeds

### 2. Implement Authentication Module
- **Task ID**: auth-module
- **Depends On**: auth-config-schema
- **Assigned To**: auth-core-builder
- **Agent**: builder
- **Actions**:
  - Create `src/api/auth.rs` with:
    
    **Auth types**:
    ```rust
    /// Extractor that validates the Authorization header against configured API keys.
    ///
    /// Returns the name of the authenticated key, or `ApiError::Unauthorized` if
    /// authentication is required and the key is missing/invalid.
    #[derive(Debug, Clone)]
    pub struct AuthenticatedKey {
        /// The name of the API key that passed validation
        pub key_name: String,
    }
    ```
    
    **Key validation function**:
    ```rust
    /// Validate an API key against the configured keys.
    ///
    /// Returns the key name if valid, or `None` if no match.
    /// Uses constant-time comparison to prevent timing attacks.
    pub fn validate_api_key(
        provided_key: &str,
        configured_keys: &[config::ApiKeyEntry],
    ) -> Option<String> {
        for entry in configured_keys {
            if constant_time_compare(provided_key, &entry.secret) {
                return Some(entry.name.clone());
            }
        }
        None
    }

    /// Constant-time string comparison to prevent timing attacks.
    fn constant_time_compare(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut result = 0u8;
        for (xa, xb) in a.bytes().zip(b.bytes()) {
            result |= xa ^ xb;
        }
        result == 0
    }
    ```
    
    **Axum middleware handler**:
    ```rust
    /// Authentication middleware handler.
    ///
    /// - If auth is disabled in config, passes through without checking.
    /// - If auth is enabled and `authenticate_read` is false, only checks
    ///   write methods (POST, PUT, PATCH, DELETE).
    /// - If auth is enabled and `authenticate_read` is true, checks all methods.
    /// - Validates `Authorization: Bearer <key>` header against configured keys.
    /// - Returns `ApiError::Unauthorized` if validation fails.
    pub async fn auth_middleware(
        req: Request<axum::body::Body>,
        state: State<AppState>,
        next: Next,
    ) -> Result<impl IntoResponse, ApiError> {
        let auth_config = &state.config.global.authentication;

        // If auth is disabled, pass through
        if !auth_config.enabled {
            return Ok(next.run(req).await);
        }

        // Determine if this request method requires auth
        let method_requires_auth = match req.method().as_str() {
            "GET" => auth_config.authenticate_read,
            _ => true, // POST, PUT, PATCH, DELETE always require auth when enabled
        };

        if !method_requires_auth {
            return Ok(next.run(req).await);
        }

        // Extract and validate API key
        let auth_header = req.headers().get(http::header::AUTHORIZATION);
        match auth_header {
            Some(header) => {
                let header_str = header.to_str().map_err(|_| {
                    ApiError::Unauthorized("Invalid Authorization header encoding".to_string())
                })?;

                // Parse "Bearer <key>" format
                let parts: Vec<&str> = header_str.split_whitespace().collect();
                if parts.len() != 2 || parts[0] != "Bearer" {
                    return Err(ApiError::Unauthorized(
                        "Authorization header must use Bearer format: 'Authorization: Bearer <key>'".to_string(),
                    ));
                }

                let key_name = validate_api_key(parts[1], &auth_config.api_keys)
                    .ok_or_else(|| {
                        ApiError::Unauthorized("Invalid API key".to_string())
                    })?;

                // Log successful authentication
                tracing::info!(key_name = %key_name, method = %req.method(), uri = %req.uri(), "Authenticated request");
                Ok(next.run(req).await)
            }
            None => Err(ApiError::Unauthorized(
                "Missing Authorization header. API key required.".to_string(),
            )),
        }
    }
    ```
    
    **Axum layer factory**:
    ```rust
    /// Returns an Axum layer wrapping [`auth_middleware`].
    pub fn auth_layer() -> impl tower::Layer<axum::routing::MethodRouter> + Clone {
        axum::middleware::from_fn_with_state::<_, axum::body::Body>(auth_middleware)
    }
    ```
    
    **Auth status response type**:
    ```rust
    /// Response for GET /api/v1/auth/status
    ///
    /// Returns authentication configuration without exposing sensitive details.
    /// API key names are intentionally omitted to prevent information leakage.
    /// Use `keys_count` to determine the number of configured keys.
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
    
    **Auth status endpoint handler**:
    ```rust
    /// GET /api/v1/auth/status — Returns current authentication configuration.
    ///
    /// This endpoint is intentionally unauthenticated so clients can
    /// discover auth requirements before making authenticated requests.
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
  - Add `pub mod auth;` to `src/api/mod.rs`
  - Re-export `AuthStatusResponse` from `src/api/mod.rs`
  - Document all public types and functions with rustdoc
- **Acceptance Criteria**:
  - `auth.rs` compiles with no errors
  - `validate_api_key` uses constant-time comparison
  - `auth_middleware` correctly handles all four cases:
    1. Auth disabled → pass through
    2. Auth enabled, GET, authenticate_read=false → pass through
    3. Auth enabled, write method → validate key
    4. Auth enabled, GET, authenticate_read=true → validate key
  - `auth_middleware` returns `ApiError::Unauthorized` for: missing header, bad format, invalid key
  - `get_auth_status` returns auth configuration without exposing secrets or key names
  - `AuthStatusResponse` serializes correctly with serde
  - `Authorization` header parsing handles `Bearer <key>` format
  - Logging includes key name, method, and URI on successful auth

### 3. Wire Authentication into Router and AppState
- **Task ID**: auth-integration
- **Depends On**: auth-module
- **Assigned To**: auth-integration-builder
- **Agent**: builder
- **Actions**:
  - Update `src/api/mod.rs` router construction:
    ```rust
    pub fn create_router(state: AppState) -> Router {
        Router::new()
            // Auth status (unauthenticated — always accessible)
            .route("/api/v1/auth/status", get(auth::get_auth_status))

            // Health check
            .route("/api/v1/health", get(execution::health_check))

            // ... all existing routes ...

            // Middleware layers (order matters: auth before request_id)
            .layer(auth::auth_layer())
            .layer(axum::middleware::from_fn(middleware::request_id_middleware))
            .with_state(state)
    }
    ```
  - Update `src/api/types.rs` `ConfigResponse`:
    ```rust
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ConfigResponse {
        pub server_host: String,
        pub server_port: u16,
        pub max_parallel: u16,
        pub default_timeout_seconds: u64,
        pub log_level: String,
        pub yolo_mode: bool,
        // New fields:
        pub auth_enabled: bool,
        pub auth_require_read: bool,
        pub auth_keys_count: usize,
    }
    ```
  - Update `src/api/config.rs` `get_config` handler to populate new fields:
    ```rust
    pub async fn get_config(
        State(state): State<AppState>,
    ) -> Result<Json<ConfigResponse>, ApiError> {
        let auth = &state.config.global.authentication;
        Ok(Json(ConfigResponse {
            server_host: state.config.global.server_host.clone(),
            server_port: state.config.global.server_port,
            max_parallel: state.config.global.max_parallel,
            default_timeout_seconds: state.config.global.default_timeout_seconds,
            log_level: state.config.global.log_level.clone(),
            yolo_mode: state.config.preferences.yolo_mode,
            auth_enabled: auth.enabled,
            auth_require_read: auth.authenticate_read,
            auth_keys_count: auth.api_keys.len(),
        }))
    }
    ```
  - Add `uuid` to `Cargo.toml` dependencies for API key generation utility (optional helper)
- **Acceptance Criteria**:
  - Router includes `/api/v1/auth/status` route
  - Auth middleware layer is applied before request ID middleware
  - `ConfigResponse` includes auth-related fields
  - `get_config` populates auth fields from config
  - `cargo check` succeeds
  - Health check endpoint remains accessible without auth

### 4. Write Authentication Integration Tests
- **Task ID**: auth-tests
- **Depends On**: auth-integration
- **Assigned To**: auth-tests-builder
- **Agent**: builder
- **Actions**:
  - Add test helper function to `src/api/tests.rs`:
    ```rust
    /// Build a test app with authentication enabled.
    fn test_app_with_auth() -> (Router, TempDir) {
        let temp_dir = TempDir::with_prefix("nexum-test-auth").unwrap();
        let specs_dir = temp_dir.path().join(".agent/specs");
        let state_dir = temp_dir.path().join(".agent/state");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::create_dir_all(&state_dir).unwrap();

        let config = config::Config {
            agents: Vec::new(),
            global: config::GlobalSettings {
                server_host: "127.0.0.1".to_string(),
                server_port: 3000,
                max_parallel: 4,
                default_timeout_seconds: 3600,
                log_level: "info".to_string(),
                nexum_config_dir: None,
                authentication: config::AuthenticationSettings {
                    enabled: true,
                    authenticate_read: false,
                    api_keys: vec![
                        config::ApiKeyEntry { name: "test-cli".to_string(), secret: "test-key-123".to_string() },
                    ],
                },
            },
            preferences: config::Preferences::default(),
        };

        let state = AppState { repo_root: temp_dir.path().to_path_buf(), config };
        (create_router(state), temp_dir)
    }

    /// Helper: send a GET request with an Authorization header.
    async fn get_authed(app: &Router, uri: &str, api_key: &str) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
        let req = Request::builder()
            .uri(uri)
            .header(http::header::AUTHORIZATION, format!("Bearer {}", api_key))
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
        (status, body, headers)
    }

    /// Helper: send a POST request with an Authorization header.
    async fn post_authed(app: &Router, uri: &str, api_key: &str, body: &serde_json::Value) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
        let req = Request::builder()
            .uri(uri)
            .method("POST")
            .header(http::header::AUTHORIZATION, format!("Bearer {}", api_key))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(body).unwrap()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
        (status, body, headers)
    }
    ```
    
    **Auth disabled tests** (use existing `test_app()`):
    - `test_auth_disabled_all_endpoints_open` — All endpoints accessible without auth header
    - `test_auth_disabled_get_auth_status` — Returns `enabled: false`
    
    **Auth enabled tests** (use `test_app_with_auth()`):
    - `test_auth_enabled_get_allowed_without_auth` — GET endpoints work without auth (authenticate_read=false)
    - `test_auth_enabled_post_requires_auth` — POST returns 401 without auth header
    - `test_auth_enabled_post_requires_bearer_format` — POST returns 401 with non-Bearer format
    - `test_auth_enabled_post_invalid_key` — POST returns 401 with wrong key
    - `test_auth_enabled_post_valid_key` — POST succeeds with correct key
    - `test_auth_enabled_put_requires_auth` — PUT returns 401 without auth
    - `test_auth_enabled_patch_requires_auth` — PATCH returns 401 without auth
    - `test_auth_enabled_delete_requires_auth` — DELETE returns 401 without auth
    - `test_auth_enabled_health_check_no_auth` — Health check accessible without auth
    - `test_auth_enabled_auth_status_no_auth` — Auth status endpoint accessible without auth
    - `test_auth_enabled_error_response_format` — 401 responses include proper `ApiErrorResponse` body
    
    **Auth with read protection tests**:
    - `test_auth_read_protected_get_requires_auth` — With `authenticate_read=true`, GET returns 401 without auth
    - `test_auth_read_protected_get_with_auth` — With `authenticate_read=true`, GET succeeds with valid key
    
    **Key validation tests**:
    - `test_constant_time_compare_equal` — Equal strings return true
    - `test_constant_time_compare_different` — Different strings return false
    - `test_constant_time_compare_different_length` — Different length strings return false
    - `test_validate_api_key_match` — Valid key returns key name
    - `test_validate_api_key_no_match` — Invalid key returns None
    - `test_validate_api_key_multiple_keys` — Correctly identifies among multiple keys
- **Acceptance Criteria**:
  - All auth tests pass with `cargo test --package nexum api::tests::auth`
  - Auth disabled: all endpoints accessible without auth header
  - Auth enabled: write endpoints return 401 without valid auth
  - Auth enabled: GET endpoints accessible without auth (when `authenticate_read=false`)
  - Auth enabled: all endpoints require auth (when `authenticate_read=true`)
  - 401 responses include `ApiErrorResponse` with `error: "Unauthorized"`
  - Auth status endpoint always accessible without auth
  - Health check endpoint always accessible without auth
  - Constant-time comparison prevents timing attacks
  - Multiple API keys are validated correctly

### 5. Final Validation
- **Task ID**: validate-auth
- **Depends On**: auth-tests
- **Assigned To**: auth-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test --package nexum api` — all API tests must pass (including new auth tests)
  - Run `cargo clippy --package nexum` — no warnings in auth module
  - Verify `AuthenticationSettings` compiles with serde derive macros
  - Verify `AuthenticationSettings::default()` returns disabled auth
  - Verify `GlobalSettings` includes `authentication` field
  - Verify `ConfigResponse` includes auth fields
  - Verify auth middleware is wired into router before request ID middleware
  - Verify `/api/v1/auth/status` route exists and is unauthenticated
  - Verify health check remains unauthenticated
  - Verify GET endpoints are open when `authenticate_read=false`
  - Verify GET endpoints are protected when `authenticate_read=true`
  - Verify POST/PUT/PATCH/DELETE require auth when `enabled=true`
  - Verify `Authorization: Bearer <key>` format is enforced
  - Verify invalid keys return 401 with descriptive message
  - Verify missing auth header returns 401 with descriptive message
  - Verify constant-time comparison is used for key validation
  - Verify auth status endpoint does NOT expose API key secrets
  - Verify `ApiError::Unauthorized` is now used (no longer dead code)
  - Verify existing tests still pass (backward compatibility)
  - Verify config TOML parsing works with new `authentication` section
  - Verify `get_config` response includes auth fields

### 6. Documentation
- **Task ID**: auth-docs
- **Depends On**: validate-auth
- **Assigned To**: auth-core-builder
- **Agent**: builder
- **Actions**:
  - Update `design/backend-api.md` Authentication section:
    ```markdown
    ## Authentication

    Nexum supports optional API key authentication. When enabled, write
    operations (POST, PUT, PATCH, DELETE) require a valid API key. Read
    operations (GET) are open by default but can be protected via config.

    ### Configuration

    In `~/.config/nexum/config.toml`:
    ```toml
    [global.authentication]
    enabled = true
    authenticate_read = false
    api_keys = [
      { name = "cli", secret = "your-api-key-here" },
      { name = "slack-bot", secret = "another-api-key" },
    ]
    ```

    ### Usage

    Include the API key in the `Authorization` header:
    ```
    Authorization: Bearer your-api-key-here
    ```

    ### Endpoints

    - `GET /api/v1/auth/status` — Check auth requirements (unauthenticated)
    - All write endpoints — Require auth when `enabled=true`
    - All read endpoints — Require auth when `enabled=true` AND `authenticate_read=true`

    ### Default (MVP)

    Authentication is disabled by default. All endpoints are publicly accessible
    on localhost. Enable authentication for remote or production deployments.
    ```
  - Add rustdoc comments to all public types and functions in `auth.rs`
  - Document the `AuthenticationSettings` struct in `config/schema.rs`

## Acceptance Criteria
- `cargo check` succeeds with no errors in the auth module
- `cargo test --package nexum api` passes all tests (existing + new auth tests)
- `cargo clippy --package nexum` produces no warnings in auth module
- `AuthenticationSettings` is properly integrated into `GlobalSettings`
- `AuthenticationSettings::default()` returns auth disabled (backward compatible)
- Auth middleware is wired into the router before request ID middleware
- `/api/v1/auth/status` endpoint returns auth configuration without exposing secrets
- Auth disabled (default): all endpoints accessible without authentication
- Auth enabled:
  - GET endpoints accessible without auth when `authenticate_read=false`
  - GET endpoints require auth when `authenticate_read=true`
  - POST/PUT/PATCH/DELETE always require auth
- `Authorization: Bearer <key>` header format is enforced
- Invalid or missing API keys return 401 with `ApiErrorResponse` body
- `ApiError::Unauthorized` is now actively used (no longer dead code)
- Constant-time comparison prevents timing attacks on key validation
- Health check (`/api/v1/health`) remains accessible without auth
- `ConfigResponse` includes `auth_enabled`, `auth_require_read`, `auth_keys_count`
- Config TOML authentication section parses correctly
- Multiple API keys are supported and correctly validated
- Existing tests continue to pass (backward compatibility maintained)
- `design/backend-api.md` Authentication section is updated

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test --package nexum api` — Run all API module tests
- `cargo test --package nexum auth` — Run auth-specific tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds
- `grep -r "AuthenticationSettings" src/config/` — Verify config schema extension
- `grep -r "auth_middleware" src/api/` — Verify middleware is wired into router
- `grep -r "AuthStatusResponse" src/api/` — Verify auth status endpoint exists
- `grep -r "ApiError::Unauthorized" src/api/` — Verify Unauthorized error is actively used
- `grep -r "authenticate_read" src/` — Verify read auth config is propagated

## Notes
- This plan is a self-contained enhancement to chunk 007 (REST API). It does not require any other chunks to be completed first, but does require the existing `src/api/` module from chunk 007.
- **Backward compatibility**: Auth is disabled by default. Existing localhost-only deployments are unaffected.
- **Security**: API key comparison uses constant-time comparison to prevent timing attacks. API keys are never exposed in responses or logs.
- **Key generation**: Consider adding a CLI command or helper function to generate secure random API keys (e.g., using `uuid::Uuid::new_v4().to_string()` or `fastrand`). This is out of scope for this plan but noted for future enhancement.
- **Key rotation**: No key rotation mechanism is implemented. Users should regenerate keys in the config file. Consider adding key expiration in a future enhancement.
- **Scalability**: For production deployments with many API keys, consider moving key validation to a more efficient data structure (e.g., HashMap lookup). Current linear scan is fine for typical use cases (< 10 keys).
- **Bearer token format**: The middleware strictly requires `Bearer <key>` format. Other auth schemes (Basic, Digest) are not supported.
- **Auth status endpoint**: Intentionally unauthenticated so clients can discover auth requirements before making authenticated requests. This is a standard pattern for API discovery.
- **Middleware order**: Auth middleware runs before request ID middleware. This ensures authenticated requests get a request ID, and unauthenticated 401 responses also get a request ID.
- **Logging**: Successful authentication is logged at INFO level with key name, method, and URI. Failed authentication is logged at WARN level. Both exclude the actual key secret.
- **Security fix (007.4)**: The `key_names` field was removed from `AuthStatusResponse` to prevent information leakage. API key names are considered identifying information that could help attackers target specific key identities. The `keys_count` field remains as it provides useful awareness without revealing identities.
