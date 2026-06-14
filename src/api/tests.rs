//! Integration tests for the Nexum REST API.
//!
//! Uses `tower::Service` (via `ServiceExt::oneshot`) and `tempfile::TempDir`
//! for isolated testing of all API endpoints including plan CRUD, task CRUD,
//! execution state, configuration, and error handling.

use axum::{body::Body, http::{self, Request, StatusCode}, Router};
use http_body_util::BodyExt;
use serde_json::json;
use tempfile::TempDir;
use tower::ServiceExt;

use crate::api::{create_router, AppState};
use crate::config;
use crate::config::AuthenticationSettings;
use crate::config::ApiKeyEntry;

// ── Test Setup ──────────────────────────────────────────────────────────────

/// Build a test-ready Axum router backed by a temporary directory with a
/// minimal `.agent/` structure.
///
/// Returns the [`Router`] for sending requests and the [`TempDir`] handle
/// (which keeps the directory alive until dropped).
fn test_app() -> (Router, TempDir) {
    let temp_dir = TempDir::with_prefix("nexum-test").unwrap();

    // Set up minimal .agent directory structure
    let specs_dir = temp_dir.path().join(".agent/specs");
    let state_dir = temp_dir.path().join(".agent/state");
    std::fs::create_dir_all(&specs_dir).unwrap();
    std::fs::create_dir_all(&state_dir).unwrap();

    // Create a minimal config with defaults
    let config = config::Config {
        agents: Vec::new(),
        global: config::GlobalSettings::default(),
        preferences: config::Preferences::default(),
    };

    let state = AppState {
        repo_root: temp_dir.path().to_path_buf(),
        config,
    };

    (create_router(state), temp_dir)
}

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
            authentication: AuthenticationSettings {
                enabled: true,
                authenticate_read: false,
                api_keys: vec![
                    ApiKeyEntry { name: "test-cli".to_string(), secret: "test-key-123".to_string() },
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
    let body: serde_json::Value = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body_bytes).to_string()}))
    };
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
    let body: serde_json::Value = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body_bytes).to_string()}))
    };
    (status, body, headers)
}

/// Helper: send a GET request and return the response status code, JSON body,
/// and headers.
async fn get(app: &Router, uri: &str) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body_bytes).to_string()}))
    };
    (status, body, headers)
}

/// Helper: send a POST request with a JSON body and return the response.
async fn post_json(app: &Router, uri: &str, body: &serde_json::Value) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let req = Request::builder()
        .uri(uri)
        .method("POST")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body_bytes).to_string()}))
    };
    (status, body, headers)
}

/// Helper: send a PATCH request with a JSON body and return the response.
async fn patch_json(app: &Router, uri: &str, body: &serde_json::Value) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let req = Request::builder()
        .uri(uri)
        .method("PATCH")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body_bytes).to_string()}))
    };
    (status, body, headers)
}

/// Helper: send a PUT request with a JSON body and return the response.
async fn put_json(app: &Router, uri: &str, body: &serde_json::Value) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let req = Request::builder()
        .uri(uri)
        .method("PUT")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body_bytes).to_string()}))
    };
    (status, body, headers)
}

/// Helper: send a DELETE request and return the response status and headers.
async fn delete(app: &Router, uri: &str) -> (StatusCode, axum::http::HeaderMap) {
    let req = Request::builder()
        .uri(uri)
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    // Consume the body
    let _body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, headers)
}

/// Helper: create a plan via the API and return its JSON response body.
async fn create_test_plan(app: &Router, name: &str, branch: &str) -> serde_json::Value {
    let (status, body, _) = post_json(app, "/api/v1/plans", &json!({
        "name": name,
        "branch": branch,
        "goal": "Test goal",
        "scope": "Test scope",
    })).await;
    assert_eq!(status, StatusCode::CREATED, "expected 201 Created when creating plan '{}'", name);
    body
}

// ==========================================================================
// Plan Endpoint Tests
// ==========================================================================

/// Verify that listing plans returns an empty list when no plans exist.
#[tokio::test]
async fn test_list_plans_empty() {
    let (app, _dir) = test_app();
    let (status, body, _) = get(&app, "/api/v1/plans").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"], json!([]));
    assert_eq!(body["total"], 0);
}

/// Verify that creating a plan returns 201 with the correct fields.
#[tokio::test]
async fn test_create_plan() {
    let (app, _dir) = test_app();
    let (status, body, _) = post_json(&app, "/api/v1/plans", &json!({
        "name": "test-plan",
        "branch": "main",
        "goal": "Test goal",
        "scope": "Test scope",
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "test-plan");
    assert_eq!(body["status"], "draft");
    assert_eq!(body["branch"], "main");
}

/// Verify that creating a plan with an empty name returns 422.
#[tokio::test]
async fn test_create_plan_invalid_name() {
    let (app, _dir) = test_app();
    let (status, _, _) = post_json(&app, "/api/v1/plans", &json!({
        "name": "",
        "branch": "main",
        "goal": "Test goal",
    })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Verify that getting a non-existent plan returns 404.
#[tokio::test]
async fn test_get_plan_not_found() {
    let (app, _dir) = test_app();
    let (status, _, _) = get(&app, "/api/v1/plans/main/PLAN-999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Verify that a valid plan status transition (draft → queued) returns 200.
#[tokio::test]
async fn test_transition_plan_status_valid() {
    let (app, _dir) = test_app();
    // Create a plan
    let plan = create_test_plan(&app, "transition-test", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Transition draft -> queued
    let (status, body, _) = patch_json(&app, &format!("/api/v1/plans/main/{}/status", plan_id), &json!({
        "status": "queued",
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "queued");
}

/// Verify that an invalid plan status transition (draft → complete) returns 422.
#[tokio::test]
async fn test_transition_plan_status_invalid() {
    let (app, _dir) = test_app();
    // Create a plan
    let plan = create_test_plan(&app, "invalid-transition", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Try invalid transition: draft -> complete (should fail)
    let (status, _, _) = patch_json(&app, &format!("/api/v1/plans/main/{}/status", plan_id), &json!({
        "status": "complete",
    })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Verify that creating multiple plans assigns sequential IDs.
#[tokio::test]
async fn test_create_multiple_plans() {
    let (app, _dir) = test_app();

    let plan1 = create_test_plan(&app, "first-plan", "main").await;
    let plan2 = create_test_plan(&app, "second-plan", "main").await;

    let id1 = plan1["id"].as_str().unwrap();
    let id2 = plan2["id"].as_str().unwrap();

    // IDs should be different
    assert_ne!(id1, id2);

    // Verify listing returns both plans
    let (status, body, _) = get(&app, "/api/v1/plans").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
}

/// Verify that retrieving a plan by ID returns its data.
#[tokio::test]
async fn test_get_plan() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "get-test", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    let (status, body, _) = get(&app, &format!("/api/v1/plans/main/{}", plan_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], plan_id);
    assert_eq!(body["name"], "get-test");
}

/// Verify that updating a plan's name returns the updated data.
#[tokio::test]
async fn test_update_plan() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "original-name", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    let (status, body, _) = put_json(&app, &format!("/api/v1/plans/main/{}", plan_id), &json!({
        "name": "updated-name",
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "updated-name");
}

/// Verify that deleting a plan returns 204.
#[tokio::test]
async fn test_delete_plan() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "delete-me", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    let (status, _) = delete(&app, &format!("/api/v1/plans/main/{}", plan_id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify the plan is gone
    let (status, _, _) = get(&app, &format!("/api/v1/plans/main/{}", plan_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ==========================================================================
// Task Endpoint Tests
// ==========================================================================

/// Verify that listing tasks for a plan with no tasks returns an empty list.
#[tokio::test]
async fn test_list_tasks_empty() {
    let (app, _dir) = test_app();
    // Create a plan first
    let plan = create_test_plan(&app, "task-test-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    let (status, body, _) = get(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"], json!([]));
    assert_eq!(body["total"], 0);
}

/// Verify that creating a task within a plan returns 201 with correct fields.
#[tokio::test]
async fn test_create_task() {
    let (app, _dir) = test_app();
    // Create plan first
    let plan = create_test_plan(&app, "task-create-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create task
    let (status, body, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "test-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Test description",
        "acceptance_criteria": ["Criterion 1"],
        "files_to_modify": ["src/test.rs"],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "test-task");
    assert_eq!(body["parent_plan"], plan_id);
    assert_eq!(body["dependencies"], json!([]));
}

/// Verify that creating a task for a non-existent plan returns 404.
#[tokio::test]
async fn test_create_task_plan_not_found() {
    let (app, _dir) = test_app();
    let (status, _, _) = post_json(&app, "/api/v1/plans/main/PLAN-999/tasks", &json!({
        "name": "orphan-task",
        "parent_plan": "PLAN-999",
        "dependencies": [],
        "description": "Test",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Verify that listing tasks returns them after creation.
#[tokio::test]
async fn test_list_tasks_after_creation() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "list-tasks-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task
    let (status, _, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "listed-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Test",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::CREATED);

    // List tasks
    let (status, body, _) = get(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["name"], "listed-task");
}

/// Verify that getting a specific task by ID returns its data.
#[tokio::test]
async fn test_get_task() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "get-task-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "specific-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Get me",
        "acceptance_criteria": ["Done"],
        "files_to_modify": ["src/lib.rs"],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();

    // Get the task
    let (status, body, _) = get(&app, &format!("/api/v1/plans/main/{}/tasks/{}", plan_id, task_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], task_id);
    assert_eq!(body["name"], "specific-task");
}

/// Verify that getting a non-existent task returns 404.
#[tokio::test]
async fn test_get_task_not_found() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "not-found-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    let (status, _, _) = get(&app, &format!("/api/v1/plans/main/{}/tasks/TASK-999", plan_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Verify that updating a task returns the updated data.
#[tokio::test]
async fn test_update_task() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "update-task-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "original-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Original",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();

    // Update the task
    let (status, body, _) = put_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}", plan_id, task_id), &json!({
        "name": "updated-task",
        "description": "Updated description",
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "updated-task");
    assert_eq!(body["description"], "Updated description");
}

/// Verify that deleting a task returns 204.
#[tokio::test]
async fn test_delete_task() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "delete-task-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "to-delete",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Delete me",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();

    // Delete the task
    let (status, _) = delete(&app, &format!("/api/v1/plans/main/{}/tasks/{}", plan_id, task_id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify the task is gone
    let (status, _, _) = get(&app, &format!("/api/v1/plans/main/{}/tasks/{}", plan_id, task_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Verify that transitioning a task status (backlog → queued) returns 200.
#[tokio::test]
async fn test_transition_task_status_valid() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "task-transition-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "transition-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Test",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();

    // Transition backlog -> queued
    let (status, body, _) = patch_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}/status", plan_id, task_id), &json!({
        "status": "queued",
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"]["status"], "queued");
}

/// Verify that an invalid task status transition (backlog → running) returns 422.
#[tokio::test]
async fn test_transition_task_status_invalid() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "invalid-task-transition", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "invalid-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Test",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();

    // Try invalid transition: backlog -> running (should fail)
    let (status, _, _) = patch_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}/status", plan_id, task_id), &json!({
        "status": "running",
    })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Verify that claiming a queued task transitions it to running with agent lease.
#[tokio::test]
async fn test_claim_task() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "claim-task-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "claimable-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Claim me",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();

    // Transition backlog -> queued first
    let (status, _, _) = patch_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}/status", plan_id, task_id), &json!({
        "status": "queued",
    })).await;
    assert_eq!(status, StatusCode::OK);

    // Claim the task
    let (status, body, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}/claim", plan_id, task_id), &json!({
        "agent_role": "builder",
        "agent_pid": 12345,
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"]["status"], "running");
    assert!(body["status"]["agent"].is_object());
    assert_eq!(body["status"]["agent"]["role"], "builder");
    assert_eq!(body["status"]["agent"]["pid"], 12345);
}

/// Verify that claiming a non-queued task returns an error.
#[tokio::test]
async fn test_claim_task_not_queued() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "non-queued-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // Create a task (starts in backlog)
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "backlog-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Test",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();

    // Try to claim while in backlog (should fail)
    let (status, _, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}/claim", plan_id, task_id), &json!({
        "agent_role": "builder",
        "agent_pid": 12345,
    })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ==========================================================================
// Execution Endpoint Tests
// ==========================================================================

/// Verify that the health check returns 200 with `{"status": "ok"}`.
#[tokio::test]
async fn test_health_check() {
    let (app, _dir) = test_app();
    let (status, body, _) = get(&app, "/api/v1/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

/// Verify that listing running tasks returns an empty array when none are running.
#[tokio::test]
async fn test_list_running_tasks() {
    let (app, _dir) = test_app();
    let (status, body, _) = get(&app, "/api/v1/running").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 0);
}

/// Verify that getting execution state for a plan returns its state.
#[tokio::test]
async fn test_get_execution_state() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "exec-state-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    let (status, body, _) = get(&app, &format!("/api/v1/plans/main/{}/execution", plan_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["plan_id"], plan_id);
    assert_eq!(body["branch"], "main");
}

/// Verify that getting execution state for a non-existent plan returns 404.
#[tokio::test]
async fn test_get_execution_state_not_found() {
    let (app, _dir) = test_app();
    let (status, _, _) = get(&app, "/api/v1/plans/main/PLAN-999/execution").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ==========================================================================
// Config Endpoint Tests
// ==========================================================================

/// Verify that getting config returns server settings with default values.
#[tokio::test]
async fn test_get_config() {
    let (app, _dir) = test_app();
    let (status, body, _) = get(&app, "/api/v1/config").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["server_host"].is_string());
    assert!(body["server_port"].is_number());
    assert_eq!(body["server_host"], "127.0.0.1");
    assert_eq!(body["server_port"], 3000);
    assert_eq!(body["max_parallel"], 4);
    assert_eq!(body["default_timeout_seconds"], 3600);
    assert_eq!(body["log_level"], "info");
    assert_eq!(body["yolo_mode"], false);
}

/// Verify that listing agents returns an empty list when none are registered.
#[tokio::test]
async fn test_list_agents() {
    let (app, _dir) = test_app();
    let (status, body, _) = get(&app, "/api/v1/agents").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["agents"].is_array());
    assert_eq!(body["agents"].as_array().unwrap().len(), 0);
}

// ==========================================================================
// Error Handling Tests
// ==========================================================================

/// Verify that sending malformed JSON returns 400.
#[tokio::test]
async fn test_bad_request_format() {
    let (app, _dir) = test_app();
    let req = Request::builder()
        .uri("/api/v1/plans")
        .method("POST")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from("not valid json"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

/// Verify that every response includes an x-request-id header.
#[tokio::test]
async fn test_request_id_header() {
    let (app, _dir) = test_app();
    let (_, _, headers) = get(&app, "/api/v1/health").await;
    let request_id = headers.get("x-request-id");
    assert!(request_id.is_some());
    let id_str = request_id.unwrap().to_str().unwrap();
    assert!(id_str.starts_with("req-"));
}

/// Verify that every response includes an x-request-id header, even on errors.
#[tokio::test]
async fn test_request_id_header_on_error() {
    let (app, _dir) = test_app();
    let (status, _, headers) = get(&app, "/api/v1/plans/main/PLAN-999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let request_id = headers.get("x-request-id");
    assert!(request_id.is_some());
}

/// Verify that creating a plan with an invalid branch name returns 422.
#[tokio::test]
async fn test_create_plan_invalid_branch() {
    let (app, _dir) = test_app();
    let (status, _, _) = post_json(&app, "/api/v1/plans", &json!({
        "name": "bad-branch",
        "branch": "../traversal",
        "goal": "Test",
    })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Verify that error responses include error type and message fields.
#[tokio::test]
async fn test_error_response_structure() {
    let (app, _dir) = test_app();
    let (status, body, _) = get(&app, "/api/v1/plans/main/PLAN-999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].is_string());
    assert!(body["message"].is_string());
    assert!(body["status"].is_number());
    assert_eq!(body["error"], "NotFound");
}

/// Verify that creating a plan with an empty branch name returns 422.
#[tokio::test]
async fn test_create_plan_empty_branch() {
    let (app, _dir) = test_app();
    let (status, _, _) = post_json(&app, "/api/v1/plans", &json!({
        "name": "test",
        "branch": "",
        "goal": "Test",
    })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Verify that creating a task with an empty name returns 422.
#[tokio::test]
async fn test_create_task_empty_name() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "empty-task-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    let (status, _, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Test",
        "acceptance_criteria": [],
        "files_to_modify": [],
    })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Verify that transition on a non-existent plan returns 404.
#[tokio::test]
async fn test_transition_nonexistent_plan() {
    let (app, _dir) = test_app();
    let (status, _, _) = patch_json(&app, "/api/v1/plans/main/PLAN-999/status", &json!({
        "status": "queued",
    })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Verify the full plan lifecycle: create → transition → update → delete.
#[tokio::test]
async fn test_full_plan_lifecycle() {
    let (app, _dir) = test_app();

    // 1. Create
    let plan = create_test_plan(&app, "lifecycle-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();
    assert_eq!(plan["status"], "draft");

    // 2. Transition draft -> queued
    let (status, body, _) = patch_json(&app, &format!("/api/v1/plans/main/{}/status", plan_id), &json!({
        "status": "queued",
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "queued");

    // 3. Update name
    let (status, body, _) = put_json(&app, &format!("/api/v1/plans/main/{}", plan_id), &json!({
        "name": "renamed-plan",
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "renamed-plan");

    // 4. Delete
    let (status, _) = delete(&app, &format!("/api/v1/plans/main/{}", plan_id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 5. Verify deletion
    let (status, _, _) = get(&app, &format!("/api/v1/plans/main/{}", plan_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Verify the full task lifecycle: create → transition → claim → delete.
#[tokio::test]
async fn test_full_task_lifecycle() {
    let (app, _dir) = test_app();
    let plan = create_test_plan(&app, "task-lifecycle-plan", "main").await;
    let plan_id = plan["id"].as_str().unwrap();

    // 1. Create task
    let (status, task, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks", plan_id), &json!({
        "name": "lifecycle-task",
        "parent_plan": plan_id,
        "dependencies": [],
        "description": "Full lifecycle test",
        "acceptance_criteria": ["Works"],
        "files_to_modify": ["src/test.rs"],
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = task["id"].as_str().unwrap();
    assert_eq!(task["status"]["status"], "backlog");

    // 2. Transition backlog -> queued
    let (status, _, _) = patch_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}/status", plan_id, task_id), &json!({
        "status": "queued",
    })).await;
    assert_eq!(status, StatusCode::OK);

    // 3. Claim the task
    let (status, body, _) = post_json(&app, &format!("/api/v1/plans/main/{}/tasks/{}/claim", plan_id, task_id), &json!({
        "agent_role": "builder",
        "agent_pid": 99999,
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"]["status"], "running");

    // 4. Delete the task
    let (status, _) = delete(&app, &format!("/api/v1/plans/main/{}/tasks/{}", plan_id, task_id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// Verify that plans on different branches are isolated.
#[tokio::test]
async fn test_branch_isolation() {
    let (app, _dir) = test_app();

    // Create plan on "main" branch
    let plan_main = create_test_plan(&app, "main-plan", "main").await;
    // Create plan on "feature" branch
    let plan_feature = create_test_plan(&app, "feature-plan", "feature").await;

    // Filter by "main" branch
    let (status, body, _) = get(&app, "/api/v1/plans?branch=main").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["id"], plan_main["id"]);

    // Filter by "feature" branch
    let (status, body, _) = get(&app, "/api/v1/plans?branch=feature").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["id"], plan_feature["id"]);
}

/// Verify that creating a plan with optional fields omitted still works.
#[tokio::test]
async fn test_create_plan_with_minimal_fields() {
    let (app, _dir) = test_app();
    let (status, body, _) = post_json(&app, "/api/v1/plans", &json!({
        "name": "minimal-plan",
        "branch": "main",
        "goal": "Minimal goal",
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "minimal-plan");
    assert_eq!(body["scope"], ""); // defaults to empty string
    assert_eq!(body["background"], ""); // defaults to empty string
}

// ==========================================================================
// Authentication Tests
// ==========================================================================

/// Verify that with auth disabled (default), all endpoints are accessible without auth header.
#[tokio::test]
async fn test_auth_disabled_all_endpoints_open() {
    let (app, _dir) = test_app();
    // Health check should work
    let (status, _, _) = get(&app, "/api/v1/health").await;
    assert_eq!(status, StatusCode::OK);
    // Config should work
    let (status, _, _) = get(&app, "/api/v1/config").await;
    assert_eq!(status, StatusCode::OK);
    // Plans should work
    let (status, _, _) = get(&app, "/api/v1/plans").await;
    assert_eq!(status, StatusCode::OK);
}

/// Verify that auth status endpoint reports enabled: false when auth is disabled.
#[tokio::test]
async fn test_auth_disabled_get_auth_status() {
    let (app, _dir) = test_app();
    let (status, body, _) = get(&app, "/api/v1/auth/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], false);
    assert_eq!(body["authenticate_read"], false);
    assert_eq!(body["keys_count"], 0);
}

/// Verify that with auth enabled, GET endpoints work without auth (authenticate_read=false).
#[tokio::test]
async fn test_auth_enabled_get_allowed_without_auth() {
    let (app, _dir) = test_app_with_auth();
    let (status, _, _) = get(&app, "/api/v1/health").await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = get(&app, "/api/v1/plans").await;
    assert_eq!(status, StatusCode::OK);
}

/// Verify that POST returns 401 without auth header when auth is enabled.
#[tokio::test]
async fn test_auth_enabled_post_requires_auth() {
    let (app, _dir) = test_app_with_auth();
    let (status, body, _) = post_json(&app, "/api/v1/plans", &json!({
        "name": "test",
        "branch": "main",
        "goal": "Test",
    })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].is_string());
}

/// Verify that POST returns 401 with non-Bearer format.
#[tokio::test]
async fn test_auth_enabled_post_requires_bearer_format() {
    let (app, _dir) = test_app_with_auth();
    let req = Request::builder()
        .uri("/api/v1/plans")
        .method("POST")
        .header(http::header::AUTHORIZATION, "Basic dXNlcjpwYXNz")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json!({"name":"test","branch":"main","goal":"Test"})).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Verify that POST returns 401 with wrong key.
#[tokio::test]
async fn test_auth_enabled_post_invalid_key() {
    let (app, _dir) = test_app_with_auth();
    let (status, body, _) = post_authed(&app, "/api/v1/plans", "wrong-key", &json!({
        "name": "test",
        "branch": "main",
        "goal": "Test",
    })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].is_string());
}

/// Verify that POST succeeds with correct key.
#[tokio::test]
async fn test_auth_enabled_post_valid_key() {
    let (app, _dir) = test_app_with_auth();
    let (status, body, _) = post_authed(&app, "/api/v1/plans", "test-key-123", &json!({
        "name": "auth-test-plan",
        "branch": "main",
        "goal": "Test goal",
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "auth-test-plan");
}

/// Verify that PUT returns 401 without auth.
#[tokio::test]
async fn test_auth_enabled_put_requires_auth() {
    let (app, _dir) = test_app_with_auth();
    let req = Request::builder()
        .uri("/api/v1/plans/main/PLAN-999")
        .method("PUT")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json!({"name": "updated"})).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Verify that PATCH returns 401 without auth.
#[tokio::test]
async fn test_auth_enabled_patch_requires_auth() {
    let (app, _dir) = test_app_with_auth();
    let req = Request::builder()
        .uri("/api/v1/plans/main/PLAN-999/status")
        .method("PATCH")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json!({"status": "queued"})).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Verify that DELETE returns 401 without auth.
#[tokio::test]
async fn test_auth_enabled_delete_requires_auth() {
    let (app, _dir) = test_app_with_auth();
    let req = Request::builder()
        .uri("/api/v1/plans/main/PLAN-999")
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Verify that health check remains accessible without auth.
#[tokio::test]
async fn test_auth_enabled_health_check_no_auth() {
    let (app, _dir) = test_app_with_auth();
    let (status, body, _) = get(&app, "/api/v1/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

/// Verify that auth status endpoint is accessible without auth.
#[tokio::test]
async fn test_auth_enabled_auth_status_no_auth() {
    let (app, _dir) = test_app_with_auth();
    let (status, body, _) = get(&app, "/api/v1/auth/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], true);
    assert_eq!(body["authenticate_read"], false);
    assert_eq!(body["keys_count"], 1);
    assert_eq!(body["key_names"][0], "test-cli");
}

/// Verify that 401 responses include an error field with a descriptive message.
#[tokio::test]
async fn test_auth_enabled_error_response_format() {
    let (app, _dir) = test_app_with_auth();
    let (status, body, _) = post_json(&app, "/api/v1/plans", &json!({
        "name": "test",
        "branch": "main",
        "goal": "Test",
    })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].is_string());
}

/// Build a test app with read authentication enabled.
fn test_app_with_read_auth() -> (Router, TempDir) {
    let temp_dir = TempDir::with_prefix("nexum-test-read-auth").unwrap();
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
            authentication: AuthenticationSettings {
                enabled: true,
                authenticate_read: true,
                api_keys: vec![
                    ApiKeyEntry { name: "test-cli".to_string(), secret: "test-key-123".to_string() },
                ],
            },
        },
        preferences: config::Preferences::default(),
    };

    let state = AppState { repo_root: temp_dir.path().to_path_buf(), config };
    (create_router(state), temp_dir)
}

/// Verify that with authenticate_read=true, GET returns 401 without auth.
#[tokio::test]
async fn test_auth_read_protected_get_requires_auth() {
    let (app, _dir) = test_app_with_read_auth();
    let (status, body, _) = get(&app, "/api/v1/plans").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].is_string());
}

/// Verify that with authenticate_read=true, GET succeeds with valid key.
#[tokio::test]
async fn test_auth_read_protected_get_with_auth() {
    let (app, _dir) = test_app_with_read_auth();
    let (status, body, _) = get_authed(&app, "/api/v1/plans", "test-key-123").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 0);
}

/// Verify that config endpoint includes auth fields.
#[tokio::test]
async fn test_config_includes_auth_fields() {
    let (app, _dir) = test_app_with_auth();
    let (status, body, _) = get_authed(&app, "/api/v1/config", "test-key-123").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["auth_enabled"], true);
    assert_eq!(body["auth_require_read"], false);
    assert_eq!(body["auth_keys_count"], 1);
}

/// Verify that creating a plan with auth works through the full lifecycle.
#[tokio::test]
async fn test_auth_enabled_plan_lifecycle() {
    let (app, _dir) = test_app_with_auth();

    // Create plan with auth
    let (status, body, _) = post_authed(&app, "/api/v1/plans", "test-key-123", &json!({
        "name": "auth-lifecycle-plan",
        "branch": "main",
        "goal": "Test lifecycle with auth",
    })).await;
    assert_eq!(status, StatusCode::CREATED);
    let plan_id = body["id"].as_str().unwrap();

    // GET plan without auth should work (authenticate_read=false)
    let (status, body, _) = get(&app, &format!("/api/v1/plans/main/{}", plan_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "auth-lifecycle-plan");

    // Update plan requires auth
    let put_req = Request::builder()
        .uri(&format!("/api/v1/plans/main/{}", plan_id))
        .method("PUT")
        .header(http::header::AUTHORIZATION, "Bearer test-key-123")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json!({"name": "updated-auth-plan"})).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(put_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Delete plan requires auth
    let del_req = Request::builder()
        .uri(&format!("/api/v1/plans/main/{}", plan_id))
        .method("DELETE")
        .header(http::header::AUTHORIZATION, "Bearer test-key-123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

/// Verify that Bearer token with extra whitespace is handled.
#[tokio::test]
async fn test_auth_bearer_extra_whitespace() {
    let (app, _dir) = test_app_with_auth();
    // Extra space between Bearer and key — split_whitespace handles this
    let req = Request::builder()
        .uri("/api/v1/plans")
        .method("POST")
        .header(http::header::AUTHORIZATION, "Bearer  test-key-123")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json!({"name":"test","branch":"main","goal":"Test"})).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    // split_whitespace collapses multiple spaces, so "Bearer  key" → ["Bearer", "key"]
    // This should succeed since the key is valid
    assert_eq!(res.status(), StatusCode::CREATED);
}

/// Verify that empty Bearer token returns 401.
#[tokio::test]
async fn test_auth_bearer_empty_token() {
    let (app, _dir) = test_app_with_auth();
    let req = Request::builder()
        .uri("/api/v1/plans")
        .method("POST")
        .header(http::header::AUTHORIZATION, "Bearer ")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json!({"name":"test","branch":"main","goal":"Test"})).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Verify that OPTIONS request passes through without auth.
#[tokio::test]
async fn test_auth_options_preflight() {
    let (app, _dir) = test_app_with_auth();
    let req = Request::builder()
        .uri("/api/v1/plans")
        .method("OPTIONS")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    // OPTIONS should pass through (not 401)
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Verify that WWW-Authenticate header is present on 401 responses.
#[tokio::test]
async fn test_auth_www_authenticate_header() {
    let (app, _dir) = test_app_with_auth();
    let req = Request::builder()
        .uri("/api/v1/plans")
        .method("POST")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json!({"name":"test","branch":"main","goal":"Test"})).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let www_auth = res.headers().get(axum::http::header::WWW_AUTHENTICATE);
    assert!(www_auth.is_some());
    assert_eq!(www_auth.unwrap().to_str().unwrap(), "Bearer");
}
