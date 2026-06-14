//! Authentication middleware for the Nexum REST API.
//!
//! Provides API key-based authentication via `Authorization: Bearer <key>` headers.
//! When disabled (default), all endpoints are publicly accessible.
//! When enabled, write operations (POST, PUT, PATCH, DELETE) require a valid API key.
//! Read operations (GET) are open by default but can be protected via config.
//!
//! # Design
//! - **API key auth**: Simple bearer token in `Authorization: Bearer <key>` header
//! - **Config-driven**: Enabled/disabled via `~/.config/nexum/config.toml`
//! - **Multiple keys**: Support multiple named API keys (e.g., `cli`, `slack-bot`, `web-ui`)
//! - **Selective guarding**: Configurable whether GET endpoints require auth
//! - **Non-intrusive**: When disabled, middleware is a no-op (zero overhead)
//! - **Constant-time comparison**: Prevents timing attacks on key validation

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use tower::Layer;

use crate::api::errors::ApiError;
use crate::api::types::AppState;
use crate::config;

// ── Auth Types ───────────────────────────────────────────────────────

/// Extractor that validates the Authorization header against configured API keys.
///
/// Returns the name of the authenticated key, or `ApiError::Unauthorized` if
/// authentication is required and the key is missing/invalid.
#[derive(Debug, Clone)]
pub struct AuthenticatedKey {
    /// The name of the API key that passed validation
    pub key_name: String,
}

/// Response for GET /api/v1/auth/status
#[derive(Serialize, Debug, Clone)]
pub struct AuthStatusResponse {
    /// Whether authentication is enabled
    pub enabled: bool,
    /// Whether read endpoints require authentication
    pub authenticate_read: bool,
    /// Number of configured API keys
    pub keys_count: usize,
    /// List of key names (NOT secrets) for client identification
    pub key_names: Vec<String>,
}

// ── Key Validation ───────────────────────────────────────────────────

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

// ── Middleware ───────────────────────────────────────────────────────

/// Authentication middleware handler.
///
/// - If auth is disabled in config, passes through without checking.
/// - If auth is enabled and `authenticate_read` is false, only checks
///   write methods (POST, PUT, PATCH, DELETE).
/// - If auth is enabled and `authenticate_read` is true, checks all methods.
/// - Validates `Authorization: Bearer <key>` header against configured keys.
/// - Returns `ApiError::Unauthorized` if validation fails.
pub async fn auth_middleware(
    req: Request<Body>,
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
    let auth_header = req.headers().get(axum::http::header::AUTHORIZATION);
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

/// Returns an Axum layer wrapping [`auth_middleware`].
pub fn auth_layer() -> impl Layer<axum::routing::MethodRouter> + Clone {
    axum::middleware::from_fn::<_, Body>(auth_middleware)
}

// ── Auth Status Endpoint ─────────────────────────────────────────────

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
        key_names: auth.api_keys.iter().map(|k| k.name.clone()).collect(),
    })
}

// ── Unit Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare_equal() {
        assert!(constant_time_compare("abc", "abc"));
    }

    #[test]
    fn test_constant_time_compare_different() {
        assert!(!constant_time_compare("abc", "abd"));
    }

    #[test]
    fn test_constant_time_compare_different_length() {
        assert!(!constant_time_compare("abc", "abcd"));
    }

    #[test]
    fn test_validate_api_key_match() {
        let keys = vec![
            config::ApiKeyEntry { name: "cli".to_string(), secret: "key-123".to_string() },
            config::ApiKeyEntry { name: "bot".to_string(), secret: "key-456".to_string() },
        ];
        let result = validate_api_key("key-123", &keys);
        assert_eq!(result, Some("cli".to_string()));
    }

    #[test]
    fn test_validate_api_key_no_match() {
        let keys = vec![
            config::ApiKeyEntry { name: "cli".to_string(), secret: "key-123".to_string() },
        ];
        let result = validate_api_key("wrong-key", &keys);
        assert_eq!(result, None);
    }

    #[test]
    fn test_validate_api_key_multiple_keys() {
        let keys = vec![
            config::ApiKeyEntry { name: "first".to_string(), secret: "aaa".to_string() },
            config::ApiKeyEntry { name: "second".to_string(), secret: "bbb".to_string() },
            config::ApiKeyEntry { name: "third".to_string(), secret: "ccc".to_string() },
        ];
        assert_eq!(validate_api_key("bbb", &keys), Some("second".to_string()));
        assert_eq!(validate_api_key("ccc", &keys), Some("third".to_string()));
        assert_eq!(validate_api_key("ddd", &keys), None);
    }
}
