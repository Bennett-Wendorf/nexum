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
//! - **CORS-safe**: OPTIONS requests pass through without authentication check

use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderValue;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::api::types::AppState;
use crate::config;

// ── Auth Types ───────────────────────────────────────────────────────

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
/// Compares against ALL keys to prevent position-based timing leaks.
pub fn validate_api_key(
    provided_key: &str,
    configured_keys: &[config::ApiKeyEntry],
) -> Option<String> {
    let mut matched_name: Option<String> = None;
    for entry in configured_keys {
        if constant_time_compare(provided_key, &entry.secret) {
            matched_name = Some(entry.name.clone());
        }
    }
    matched_name
}

/// Constant-time string comparison to prevent timing attacks.
///
/// This implementation does NOT short-circuit on length mismatch.
/// It always iterates over the maximum length of both strings,
/// padding the shorter one with null bytes, to prevent length leaks.
fn constant_time_compare(a: &str, b: &str) -> bool {
    let bytes_a = a.as_bytes();
    let bytes_b = b.as_bytes();
    let max_len = bytes_a.len().max(bytes_b.len());
    let mut result = 0u8;
    // Length mismatch contributes to result (prevents short-circuit)
    result |= bytes_a.len() as u8 ^ bytes_b.len() as u8;
    for i in 0..max_len {
        let ca = bytes_a.get(i).copied().unwrap_or(0);
        let cb = bytes_b.get(i).copied().unwrap_or(0);
        result |= ca ^ cb;
    }
    result == 0
}

// ── Middleware ───────────────────────────────────────────────────────

/// Create an unauthorized response with proper format and WWW-Authenticate header.
fn unauthorized_response(msg: &str) -> Response {
    let error = ApiError::Unauthorized(msg.to_string());
    let mut resp = error.into_response();
    resp.headers_mut().insert(
        axum::http::header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Bearer"),
    );
    resp
}

/// Authentication middleware handler (2-parameter form for axum `from_fn`).
///
/// The auth config is captured via `Arc` in the closure returned by
/// [`create_auth_middleware`]. This avoids the 3-parameter `State`
/// extractor pattern which axum 0.8 cannot infer for middleware layers.
///
/// - If auth is disabled in config, passes through without checking.
/// - OPTIONS requests always pass through (CORS preflight).
/// - If auth is enabled and `authenticate_read` is false, only checks
///   write methods (POST, PUT, PATCH, DELETE).
/// - If auth is enabled and `authenticate_read` is true, checks all methods.
/// - Validates `Authorization: Bearer <key>` header against configured keys.
/// - Returns 401 with `ApiErrorResponse` format and `WWW-Authenticate: Bearer` header.
pub async fn auth_middleware(
    req: Request<Body>,
    next: Next,
    auth_config: Arc<config::AuthenticationSettings>,
) -> Response {
    // If auth is disabled, pass through
    if !auth_config.enabled {
        return next.run(req).await;
    }

    // OPTIONS always passes through (CORS preflight)
    if req.method().as_str() == "OPTIONS" {
        return next.run(req).await;
    }

    // Determine if this request method requires auth
    let method_requires_auth = match req.method().as_str() {
        "GET" => auth_config.authenticate_read,
        _ => true, // POST, PUT, PATCH, DELETE always require auth when enabled
    };

    if !method_requires_auth {
        return next.run(req).await;
    }

    // Extract and validate API key
    let auth_header = req.headers().get(axum::http::header::AUTHORIZATION);
    match auth_header {
        Some(header) => {
            match header.to_str() {
                Ok(header_str) => {
                    // Parse "Bearer <key>" format
                    let parts: Vec<&str> = header_str.split_whitespace().collect();
                    if parts.len() != 2 || parts[0] != "Bearer" {
                        return unauthorized_response(
                            "Authorization header must use Bearer format: 'Authorization: Bearer <key>'",
                        );
                    }

                    let key_name = match validate_api_key(parts[1], &auth_config.api_keys) {
                        Some(name) => name,
                        None => return unauthorized_response("Invalid API key"),
                    };

                    // Log successful authentication
                    tracing::info!(key_name = %key_name, method = %req.method(), uri = %req.uri(), "Authenticated request");
                    next.run(req).await
                }
                Err(_) => {
                    return unauthorized_response("Invalid Authorization header encoding");
                }
            }
        }
        None => unauthorized_response("Missing Authorization header. API key required."),
    }
}

/// Creates a 2-parameter middleware closure that captures the auth config.
///
/// Returns a `Clone` + `Send` + `Sync` function suitable for use with
/// `axum::middleware::from_fn`.
pub fn create_auth_middleware(
    auth_config: Arc<config::AuthenticationSettings>,
) -> impl Fn(Request<Body>, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone + Send + Sync + 'static {
    move |req: Request<Body>, next: Next| {
        let auth_config = auth_config.clone();
        Box::pin(async move { auth_middleware(req, next, auth_config).await })
    }
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
        // This now does NOT short-circuit on length mismatch
        assert!(!constant_time_compare("abc", "abcd"));
    }

    #[test]
    fn test_constant_time_compare_empty() {
        assert!(constant_time_compare("", ""));
        assert!(!constant_time_compare("", "a"));
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
