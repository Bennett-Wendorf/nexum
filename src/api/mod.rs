//! REST API module for Nexum.
//!
//! This module defines the HTTP routes, request/response types, error
//! handling, and authentication middleware for the Nexum REST API.

pub mod auth;
pub mod config;
pub mod errors;
pub mod execution;
pub mod middleware;
pub mod plans;
pub mod tasks;
pub mod types;

#[cfg(test)]
mod tests;

use axum::{
    routing::{get, patch, post},
    Router,
};

pub use types::*;

/// Construct the API router with all routes and middleware.
///
/// The router includes:
/// - Health check: GET /api/v1/health
/// - Plan CRUD: GET/POST /api/v1/plans, GET/PATCH/DELETE /api/v1/plans/{branch}/{plan_id}
/// - Plan status: PATCH /api/v1/plans/{branch}/{plan_id}/status
/// - Task CRUD: GET/POST /api/v1/plans/{branch}/{plan_id}/tasks, GET/PATCH/DELETE /api/v1/plans/{branch}/{plan_id}/tasks/{task_id}
/// - Task status: PATCH /api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/status
/// - Task claim: POST /api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/claim
/// - Execution: GET /api/v1/plans/{branch}/{plan_id}/execution, GET /api/v1/running
/// - Config: GET /api/v1/config, GET /api/v1/agents
/// - Auth: GET /api/v1/auth/status (unauthenticated)
/// - Authentication middleware guards write endpoints when enabled
///
/// All routes use `/api/v1/` prefix for URL-based versioning.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/api/v1/health", get(execution::health_check))
        // Plan endpoints
        .route(
            "/api/v1/plans",
            get(plans::list_plans).post(plans::create_plan),
        )
        .route(
            "/api/v1/plans/{branch}/{plan_id}",
            get(plans::get_plan)
                .patch(plans::update_plan)
                .delete(plans::delete_plan),
        )
        .route(
            "/api/v1/plans/{branch}/{plan_id}/status",
            patch(plans::transition_plan_status),
        )
        // Task endpoints
        .route(
            "/api/v1/plans/{branch}/{plan_id}/tasks",
            get(tasks::list_tasks).post(tasks::create_task),
        )
        .route(
            "/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}",
            get(tasks::get_task)
                .patch(tasks::update_task)
                .delete(tasks::delete_task),
        )
        .route(
            "/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/status",
            patch(tasks::transition_task_status),
        )
        .route(
            "/api/v1/plans/{branch}/{plan_id}/tasks/{task_id}/claim",
            post(tasks::claim_task),
        )
        // Execution endpoints
        .route(
            "/api/v1/plans/{branch}/{plan_id}/execution",
            get(execution::get_execution_state),
        )
        .route("/api/v1/running", get(execution::list_running_tasks))
        // Config endpoints
        .route("/api/v1/config", get(config::get_config).patch(config::patch_config))
        .route("/api/v1/agents", get(config::list_agents))
        // Auth endpoints
        .route("/api/v1/auth/status", get(auth::get_auth_status))
        .with_state(state.clone())
        // Middleware layers (order matters: auth before request_id)
        .layer(axum::middleware::from_fn_with_state(state, auth::auth_middleware_with_state))
        .layer(axum::middleware::from_fn(middleware::request_id_middleware))
}
