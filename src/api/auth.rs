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
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use tower::Layer;

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

use axum::http::Request as HttpRequest;
use axum::http::StatusCode;
use axum::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Service, ServiceExt};

fn unauthorized_response(msg: &str) -> Response {
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": msg}))).into_response()
}

/// A clonable tower service that wraps another service with auth checking.
#[derive(Clone)]
pub struct AuthCloneService<S> {
    inner: Arc<S>,
    auth_config: Arc<config::AuthenticationSettings>,
}

impl<S> Service<HttpRequest<Body>> for AuthCloneService<S>
where
    S: Service<HttpRequest<Body>, Response = Response> + Clone + Send + Sync + 'static,
    S::Error: IntoResponse,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HttpRequest<Body>) -> Self::Future {
        let auth_config = self.auth_config.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            if !auth_config.enabled {
                let s = (*inner).clone();
                return Ok(s.oneshot(req).await.unwrap_or_else(|e| e.into_response()));
            }

            let method_requires_auth = match req.method().as_str() {
                "GET" => auth_config.authenticate_read,
                _ => true,
            };

            if !method_requires_auth {
                let s = (*inner).clone();
                return Ok(s.oneshot(req).await.unwrap_or_else(|e| e.into_response()));
            }

            match req.headers().get(axum::http::header::AUTHORIZATION) {
                Some(header) => {
                    match header.to_str() {
                        Ok(header_str) => {
                            let parts: Vec<&str> = header_str.split_whitespace().collect();
                            if parts.len() != 2 || parts[0] != "Bearer" {
                                return Ok(unauthorized_response(
                                    "Authorization header must use Bearer format: 'Authorization: Bearer <key>'",
                                ));
                            }
                            if validate_api_key(parts[1], &auth_config.api_keys).is_none() {
                                return Ok(unauthorized_response("Invalid API key"));
                            }
                            tracing::info!(method = %req.method(), uri = %req.uri(), "Authenticated request");
                            let s = (*inner).clone();
                            Ok(s.oneshot(req).await.unwrap_or_else(|e| e.into_response()))
                        }
                        Err(_) => Ok(unauthorized_response("Invalid Authorization header encoding")),
                    }
                }
                None => Ok(unauthorized_response("Missing Authorization header. API key required.")),
            }
        })
    }
}

/// Tower middleware layer for authentication.
#[derive(Clone)]
pub struct AuthLayer {
    auth_config: Arc<config::AuthenticationSettings>,
}

impl<S> Layer<S> for AuthLayer
where
    S: Clone + Send + 'static,
{
    type Service = AuthCloneService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthCloneService {
            inner: Arc::new(inner),
            auth_config: self.auth_config.clone(),
        }
    }
}

/// Returns an Axum layer wrapping the auth middleware.
///
/// Takes [`AppState`] to extract the authentication configuration,
/// then creates a tower middleware layer with the config captured.
pub fn auth_layer(state: AppState) -> AuthLayer {
    AuthLayer {
        auth_config: Arc::new(state.config.global.authentication.clone()),
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
