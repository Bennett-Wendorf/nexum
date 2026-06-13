//! API error handling types for the Nexum REST API.
//!
//! This module defines [`ApiError`], the top-level error type used across
//! all API routes, together with [`ApiErrorResponse`] which is the JSON
//! structure returned to clients on failure.
//!
//! # Error mapping
//!
//! Each [`ApiError`] variant maps to a specific HTTP status code via the
//! [`axum::response::IntoResponse`] implementation:
//!
//! | Variant             | Status |
//! |---------------------|--------|
//! | `NotFound`          | 404    |
//! | `BadRequest`        | 400    |
//! | `Conflict`          | 409    |
//! | `Internal`          | 500    |
//! | `Unauthorized`      | 401    |
//! | `MethodNotAllowed`  | 405    |
//! | `Validation`        | 422    |
//!
//! # Integration
//!
//! The module provides `From` conversions from lower-layer error types so
//! that the `?` operator can be used freely in route handlers:
//!
//! - [`persistence::PersistenceError`] → [`ApiError`]
//! - [`config::loader::ConfigError`] → [`ApiError`]
//! - [`serde_json::Error`] → [`ApiError`]

use std::fmt;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::config;
use crate::persistence;

/// Top-level API error type.
///
/// Each variant corresponds to a distinct HTTP status code and carries
/// a human-readable message. Use this type as the return error in all
/// API route handlers; the [`IntoResponse`] implementation will
/// automatically convert it into a JSON error response.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Resource not found — HTTP 404.
    #[error("Not found: {0}")]
    NotFound(&'static str),

    /// Malformed or invalid request — HTTP 400.
    #[error("Invalid request: {0}")]
    BadRequest(String),

    /// Resource conflict (e.g. concurrency) — HTTP 409.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Unexpected internal error — HTTP 500.
    #[error("Internal error: {0}")]
    Internal(anyhow::Error),

    /// Missing or invalid authentication — HTTP 401.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// HTTP method not permitted on this route — HTTP 405.
    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    /// Request body failed schema validation — HTTP 422.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// JSON error body returned to the client.
///
/// Serialized as:
/// ```json
/// {
///   "error": "NotFound",
///   "message": "Not found: agent 'foo'",
///   "status": 404
/// }
/// ```
#[derive(Serialize, Debug)]
pub struct ApiErrorResponse {
    /// Short error type name (e.g. `"NotFound"`).
    pub error: String,
    /// Full human-readable message.
    pub message: String,
    /// HTTP status code.
    pub status: u16,
}

// ── IntoResponse ──────────────────────────────────────────────────────

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, Json(ApiErrorResponse::from(&self))),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, Json(ApiErrorResponse::from(&self))),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, Json(ApiErrorResponse::from(&self))),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiErrorResponse::from(&self))),
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, Json(ApiErrorResponse::from(&self))),
            ApiError::MethodNotAllowed(_) => (StatusCode::METHOD_NOT_ALLOWED, Json(ApiErrorResponse::from(&self))),
            ApiError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, Json(ApiErrorResponse::from(&self))),
        };
        (status, body).into_response()
    }
}

// ── From<&ApiError> for ApiErrorResponse ─────────────────────────────

impl From<&ApiError> for ApiErrorResponse {
    fn from(err: &ApiError) -> Self {
        let (error_type, message, status) = match err {
            ApiError::NotFound(msg) => ("NotFound".to_string(), format!("Not found: {msg}"), 404),
            ApiError::BadRequest(msg) => ("BadRequest".to_string(), msg.clone(), 400),
            ApiError::Conflict(msg) => ("Conflict".to_string(), msg.clone(), 409),
            ApiError::Internal(err) => ("Internal".to_string(), err.to_string(), 500),
            ApiError::Unauthorized(msg) => ("Unauthorized".to_string(), msg.clone(), 401),
            ApiError::MethodNotAllowed(msg) => ("MethodNotAllowed".to_string(), msg.clone(), 405),
            ApiError::Validation(msg) => ("Validation".to_string(), msg.clone(), 422),
        };
        Self {
            error: error_type,
            message,
            status,
        }
    }
}

// ── Display for ApiErrorResponse ──────────────────────────────────────

impl fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} (status {})", self.error, self.message, self.status)
    }
}

// ── From<persistence::PersistenceError> ──────────────────────────────

/// Maps persistence-layer errors into API errors so that the `?` operator
/// works directly in route handlers.
impl From<persistence::PersistenceError> for ApiError {
    fn from(err: persistence::PersistenceError) -> Self {
        match err {
            persistence::PersistenceError::FileNotFound(_)
            | persistence::PersistenceError::DirectoryNotFound(_) => {
                ApiError::NotFound("resource not found")
            }
            persistence::PersistenceError::Io(_, _)
            | persistence::PersistenceError::IoBare(_)
            | persistence::PersistenceError::AtomicWrite(_, _)
            | persistence::PersistenceError::JsonSerialize(_, _) => {
                ApiError::Internal(anyhow::anyhow!(err))
            }
            persistence::PersistenceError::JsonParse(_, _)
            | persistence::PersistenceError::JsonParseBare(_)
            | persistence::PersistenceError::MarkdownParse(_, _)
            | persistence::PersistenceError::PathResolution(_) => {
                ApiError::BadRequest(err.to_string())
            }
            persistence::PersistenceError::SchemaValidation(msg) => {
                ApiError::Validation(msg)
            }
            persistence::PersistenceError::ConcurrencyConflict(_) => {
                ApiError::Conflict("file was modified by another process".to_string())
            }
        }
    }
}

// ── From<config::loader::ConfigError> ────────────────────────────────

/// Maps configuration errors into API errors. Config loading failures are
/// treated as internal errors since they indicate a server-side issue.
impl From<config::loader::ConfigError> for ApiError {
    fn from(err: config::loader::ConfigError) -> Self {
        ApiError::Internal(anyhow::anyhow!(err))
    }
}

// ── From<serde_json::Error> ──────────────────────────────────────────

/// Maps JSON serialization/deserialization errors into API errors.
/// These are treated as bad requests since they typically stem from
/// malformed client input.
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::BadRequest(err.to_string())
    }
}
