//! Configuration and agent listing endpoints for the Nexum REST API.
//!
//! This module provides HTTP handlers for reading the current server
//! configuration and listing registered agents:
//!
//! | Method | Path             | Handler            |
//! |--------|------------------|--------------------|
//! | GET    | `/api/config`    | [`get_config`]     |
//! | PATCH  | `/api/config`    | [`patch_config`]   |
//! | GET    | `/api/agents`    | [`list_agents`]    |
//!
//! These endpoints expose only non-sensitive configuration data. API keys,
//! OAuth tokens, and other secrets are never returned.

use axum::extract::State;
use axum::Json;

use crate::api::errors::ApiError;
use crate::api::types::*;
use crate::config;

// ── Helpers ───────────────────────────────────────────────────────────

fn build_config_response(cfg: &config::Config) -> ConfigResponse {
    let auth = &cfg.global.authentication;
    ConfigResponse {
        server_host: cfg.global.server_host.clone(),
        server_port: cfg.global.server_port,
        max_parallel: cfg.global.max_parallel,
        default_timeout_seconds: cfg.global.default_timeout_seconds,
        log_level: cfg.global.log_level.clone(),
        yolo_mode: cfg.preferences.yolo_mode,
        auth_enabled: auth.enabled,
        auth_require_read: auth.authenticate_read,
        auth_keys_count: auth.api_keys.len(),
    }
}

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/config — Get current configuration
///
/// Returns the current global server settings and user preferences as a
/// [`ConfigResponse`]. This endpoint intentionally excludes sensitive data
/// such as API keys, OAuth tokens, and the `nexum_config_dir` override.
///
/// Returns **200 OK** with the configuration payload.
pub async fn get_config(State(state): State<AppState>) -> Result<Json<ConfigResponse>, ApiError> {
    let cfg = state.config.read().await;
    let response = build_config_response(&*cfg);

    Ok(Json(response))
}

/// GET /api/agents — List registered agents
///
/// Returns all agent registrations from the configuration file as a list
/// of [`AgentRegistrationResponse`] entries inside an [`AgentsResponse`].
///
/// Only non-internal fields are exposed: `name`, `type`, `spawn_command`,
/// and optional fields (`tool_permissions`, `timeout_seconds`).
/// The `working_dir` field is omitted as it is internal to the agent
/// harness and maps to task worktrees.
///
/// If no agents are registered, returns `{"agents": []}`.
pub async fn list_agents(State(state): State<AppState>) -> Result<Json<AgentsResponse>, ApiError> {
    let cfg = state.config.read().await;
    let agents: Vec<AgentRegistrationResponse> = cfg
        .agents
        .iter()
        .map(|agent| AgentRegistrationResponse {
            name: agent.name.clone(),
            r#type: agent.r#type.clone(),
            spawn_command: agent.spawn_command.clone(),
            tool_permissions: agent.tool_permissions.clone(),
            timeout_seconds: agent.timeout_seconds,
        })
        .collect();

    Ok(Json(AgentsResponse { agents }))
}

/// PATCH /api/v1/config — Update configuration settings
///
/// Accepts a partial config update (currently only `yolo_mode` is supported).
/// Updates the in-memory config and persists changes to disk.
/// Returns the updated [`ConfigResponse`].
pub async fn patch_config(
    State(state): State<AppState>,
    Json(req): Json<PatchConfigRequest>,
) -> Result<Json<ConfigResponse>, ApiError> {
    if let Some(yolo_mode) = req.yolo_mode {
        // Hold write lock for entire read-modify-write sequence to prevent TOCTOU race
        let config_path = crate::config::loader::config_path()
            .map_err(|e| {
                tracing::error!("Failed to resolve config path: {}", e);
                ApiError::Internal(anyhow::anyhow!("Failed to resolve config path"))
            })?;

        {
            let mut cfg = state.config.write().await;
            cfg.preferences.yolo_mode = yolo_mode;

            // Serialize updated config to TOML
            let toml_string = toml::to_string(&*cfg)
                .map_err(|e| {
                    tracing::error!("Failed to serialize config: {}", e);
                    ApiError::Internal(anyhow::anyhow!("Failed to serialize configuration"))
                })?;

            // Write to disk while still holding the write lock
            tokio::fs::write(&config_path, toml_string).await
                .map_err(|e| {
                    tracing::error!("Failed to write config to disk: {}", e);
                    ApiError::Internal(anyhow::anyhow!("Failed to write config file"))
                })?;
        }
    }

    // Return updated config response
    let cfg = state.config.read().await;
    let response = build_config_response(&*cfg);
    Ok(Json(response))
}
