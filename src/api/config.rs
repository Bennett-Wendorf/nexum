//! Configuration and agent listing endpoints for the Nexum REST API.
//!
//! This module provides HTTP handlers for reading the current server
//! configuration and listing registered agents:
//!
//! | Method | Path             | Handler            |
//! |--------|------------------|--------------------|
//! | GET    | `/api/config`    | [`get_config`]     |
//! | GET    | `/api/agents`    | [`list_agents`]    |
//!
//! These endpoints expose only non-sensitive configuration data. API keys,
//! OAuth tokens, and other secrets are never returned.

use axum::extract::State;
use axum::Json;

use crate::api::errors::ApiError;
use crate::api::types::*;

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/config — Get current configuration
///
/// Returns the current global server settings and user preferences as a
/// [`ConfigResponse`]. This endpoint intentionally excludes sensitive data
/// such as API keys, OAuth tokens, and the `nexum_config_dir` override.
///
/// Returns **200 OK** with the configuration payload.
pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ConfigResponse>, ApiError> {
    let auth = &state.config.global.authentication;
    let response = ConfigResponse {
        server_host: state.config.global.server_host.clone(),
        server_port: state.config.global.server_port,
        max_parallel: state.config.global.max_parallel,
        default_timeout_seconds: state.config.global.default_timeout_seconds,
        log_level: state.config.global.log_level.clone(),
        yolo_mode: state.config.preferences.yolo_mode,
        auth_enabled: auth.enabled,
        auth_require_read: auth.authenticate_read,
        auth_keys_count: auth.api_keys.len(),
    };

    Ok(Json(response))
}

/// GET /api/agents — List registered agents
///
/// Returns all agent registrations from the configuration file as a list
/// of [`AgentRegistrationResponse`] entries inside an [`AgentsResponse`].
///
/// Only non-internal fields are exposed: `name`, `type`, `spawn_command`,
/// and optional fields (`model`, `tool_permissions`, `timeout_seconds`).
/// The `working_dir` field is omitted as it is internal to the agent
/// harness and maps to task worktrees.
///
/// If no agents are registered, returns `{"agents": []}`.
pub async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<AgentsResponse>, ApiError> {
    let agents: Vec<AgentRegistrationResponse> = state
        .config
        .agents
        .iter()
        .map(|agent| AgentRegistrationResponse {
            name: agent.name.clone(),
            r#type: agent.r#type.clone(),
            spawn_command: agent.spawn_command.clone(),
            model: agent.model.clone(),
            tool_permissions: agent.tool_permissions.clone(),
            timeout_seconds: agent.timeout_seconds,
        })
        .collect();

    Ok(Json(AgentsResponse { agents }))
}
