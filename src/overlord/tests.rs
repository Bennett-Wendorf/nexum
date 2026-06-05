//! Comprehensive unit tests for the Overlord deterministic core.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::overlord::concurrency_checker::ConcurrencyChecker;
use crate::overlord::dependency_resolver::DependencyResolver;
use crate::overlord::errors::OverlordError;
use crate::overlord::heartbeat_monitor::HeartbeatMonitor;
use crate::overlord::id_generator::{PlanIdGenerator, TaskIdGenerator};
use crate::overlord::scheduler::OverlordScheduler;
use crate::overlord::status_machine::{
    is_concurrency_sensitive, is_plan_concurrency_sensitive, PlanStateMachine, TaskStateMachine,
};
use crate::persistence::PlanStatus;
use crate::persistence::TaskStatusValue;

// ── Test Utilities ──────────────────────────────────────────────────────────

fn create_test_repo() -> tempfile::TempDir {
    tempfile::TempDir::new().expect("Failed to create temp directory")
}

fn setup_test_plan(dir: &Path, branch: &str, plan_id: &str, plan_name: &str) {
    let plan_slug = format!("{}-{}", plan_id, plan_name);
    let plan_dir = dir.join(".agent").join("specs").join(branch).join(&plan_slug);
    std::fs::create_dir_all(plan_dir.join("tasks")).expect("Failed to create plan dir");

    let state_dir = dir.join(".agent").join("state").join(branch).join(&plan_slug);
    std::fs::create_dir_all(&state_dir).expect("Failed to create state dir");

    let exec_state = serde_json::json!({
        "plan_id": plan_id,
        "branch": branch,
        "tasks": [],
        "task_status_map": {}
    });
    std::fs::write(state_dir.join("execution.json"), exec_state.to_string())
        .expect("Failed to write execution.json");

    let plan_md = format!(
        "# Plan: {}\n\n**ID:** {}\n**Status:** draft\n**Created:** 2026-01-01\n**Branch:** {}\n\n## Goal\nTest\n\n## Scope\nTest\n\n## Background\nTest\n\n## Tasks\n"
        , plan_name, plan_id, branch
    );
    std::fs::write(plan_dir.join("plan.md"), plan_md).expect("Failed to write plan.md");
}

fn create_task_status(
    dir: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
    status: TaskStatusValue,
    heartbeat: Option<String>,
    dependencies: Vec<String>,
) {
    let task_slug = format!("{}-{}", task_id, task_name);
    let plan_slug = format!("{}-{}", plan_id, plan_name);
    let task_dir = dir.join(".agent").join("specs").join(branch)
        .join(&plan_slug).join("tasks").join(&task_slug);
    std::fs::create_dir_all(&task_dir).expect("Failed to create task dir");

    let task_md = format!(
        "# {}: {}\n\n**Parent plan:** {}\n**Dependencies:** none\n**Status:** backlog\n\n## Description\nTest\n\n## Acceptance criteria\n- Test\n\n## Files to modify\n- test.rs\n\n## Background\nTest\n\n## Notes\nTest\n"
        , task_id, task_name, plan_id
    );
    std::fs::write(task_dir.join("task.md"), task_md).expect("Failed to write task.md");

    // Serialize status as kebab-case string
    let status_str = match status {
        TaskStatusValue::Backlog => "backlog",
        TaskStatusValue::Queued => "queued",
        TaskStatusValue::Running => "running",
        TaskStatusValue::Reviewing => "reviewing",
        TaskStatusValue::WaitingManualReview => "waiting-manual-review",
        TaskStatusValue::MergeQueue => "merge-queue",
        TaskStatusValue::Abandoned => "abandoned",
        TaskStatusValue::Completed => "completed",
    };

    let status_json = serde_json::json!({
        "id": task_id,
        "status": status_str,
        "agent": null,
        "transitions": [],
        "started_at": null,
        "completed_at": null,
        "attempts": 0,
        "dependencies": dependencies,
        "dependent_tasks": [],
        "heartbeat_at": heartbeat
    });
    std::fs::write(task_dir.join("status.json"), status_json.to_string())
        .expect("Failed to write status.json");
}

// ── Status Machine Tests ────────────────────────────────────────────────────

#[test]
fn test_plan_valid_transitions() {
    let m = PlanStateMachine::new();
    assert!(m.can_transition(&PlanStatus::Draft, &PlanStatus::Queued));
    assert!(m.can_transition(&PlanStatus::Queued, &PlanStatus::Planning));
    assert!(m.can_transition(&PlanStatus::Planning, &PlanStatus::Reviewing));
    assert!(m.can_transition(&PlanStatus::Reviewing, &PlanStatus::Approved));
    assert!(m.can_transition(&PlanStatus::Reviewing, &PlanStatus::Rejected));
    assert!(m.can_transition(&PlanStatus::Approved, &PlanStatus::Complete));
    assert!(m.can_transition(&PlanStatus::Approved, &PlanStatus::Rejected));
}

#[test]
fn test_plan_invalid_transitions() {
    let m = PlanStateMachine::new();
    assert!(!m.can_transition(&PlanStatus::Draft, &PlanStatus::Planning));
    assert!(!m.can_transition(&PlanStatus::Draft, &PlanStatus::Complete));
    assert!(!m.can_transition(&PlanStatus::Complete, &PlanStatus::Queued));
    assert!(!m.can_transition(&PlanStatus::Rejected, &PlanStatus::Queued));
}

#[test]
fn test_task_valid_transitions() {
    let m = TaskStateMachine::new();
    assert!(m.can_transition(&TaskStatusValue::Backlog, &TaskStatusValue::Queued));
    assert!(m.can_transition(&TaskStatusValue::Queued, &TaskStatusValue::Running));
    assert!(m.can_transition(&TaskStatusValue::Running, &TaskStatusValue::Reviewing));
    assert!(m.can_transition(&TaskStatusValue::Reviewing, &TaskStatusValue::WaitingManualReview));
    assert!(m.can_transition(&TaskStatusValue::Reviewing, &TaskStatusValue::MergeQueue));
    assert!(m.can_transition(&TaskStatusValue::WaitingManualReview, &TaskStatusValue::MergeQueue));
    assert!(m.can_transition(&TaskStatusValue::WaitingManualReview, &TaskStatusValue::Abandoned));
    assert!(m.can_transition(&TaskStatusValue::MergeQueue, &TaskStatusValue::Completed));
}

#[test]
fn test_task_invalid_transitions() {
    let m = TaskStateMachine::new();
    assert!(!m.can_transition(&TaskStatusValue::Backlog, &TaskStatusValue::Running));
    assert!(!m.can_transition(&TaskStatusValue::Backlog, &TaskStatusValue::Completed));
    assert!(!m.can_transition(&TaskStatusValue::Completed, &TaskStatusValue::Queued));
    assert!(!m.can_transition(&TaskStatusValue::Abandoned, &TaskStatusValue::Queued));
}

#[test]
fn test_plan_terminal_states() {
    assert!(PlanStateMachine::is_terminal(&PlanStatus::Complete));
    assert!(PlanStateMachine::is_terminal(&PlanStatus::Rejected));
    assert!(!PlanStateMachine::is_terminal(&PlanStatus::Draft));
    assert!(!PlanStateMachine::is_terminal(&PlanStatus::Approved));
}

#[test]
fn test_task_terminal_states() {
    assert!(TaskStateMachine::is_terminal(&TaskStatusValue::Completed));
    assert!(TaskStateMachine::is_terminal(&TaskStatusValue::Abandoned));
    assert!(!TaskStateMachine::is_terminal(&TaskStatusValue::Backlog));
    assert!(!TaskStateMachine::is_terminal(&TaskStatusValue::Running));
}

#[test]
fn test_transition_record_creation() {
    let m = PlanStateMachine::new();
    let r = m.transition(&PlanStatus::Draft, &PlanStatus::Queued, "test-actor").expect("ok");
    assert_eq!(r.from, "draft");
    assert_eq!(r.to, "queued");
    assert_eq!(r.by, "test-actor");
    assert!(!r.at.is_empty());
}

#[test]
fn test_invalid_transition_error() {
    let m = PlanStateMachine::new();
    let result = m.transition(&PlanStatus::Complete, &PlanStatus::Queued, "test");
    assert!(matches!(result, Err(OverlordError::InvalidTransition { .. })));
}

#[test]
fn test_concurrency_sensitive_statuses() {
    assert!(is_concurrency_sensitive(&TaskStatusValue::Running));
    assert!(is_concurrency_sensitive(&TaskStatusValue::Reviewing));
    assert!(!is_concurrency_sensitive(&TaskStatusValue::Backlog));
    assert!(is_plan_concurrency_sensitive(&PlanStatus::Planning));
    assert!(is_plan_concurrency_sensitive(&PlanStatus::Reviewing));
    assert!(!is_plan_concurrency_sensitive(&PlanStatus::Draft));
}

// ── ID Generator Tests ──────────────────────────────────────────────────────

#[test]
fn test_next_plan_id_empty() {
    let dir = create_test_repo();
    let id = PlanIdGenerator::next_plan_id(dir.path(), "main").expect("ok");
    assert_eq!(id, "PLAN-001");
}

#[test]
fn test_next_plan_id_incremental() {
    let dir = create_test_repo();
    let specs = dir.path().join(".agent/specs/main");
    std::fs::create_dir_all(specs.join("PLAN-001-test")).expect("ok");
    std::fs::create_dir_all(specs.join("PLAN-002-test")).expect("ok");
    let id = PlanIdGenerator::next_plan_id(dir.path(), "main").expect("ok");
    assert_eq!(id, "PLAN-003");
}

#[test]
fn test_next_task_id_empty() {
    let dir = create_test_repo();
    setup_test_plan(dir.path(), "main", "PLAN-001", "test-plan");
    let id = TaskIdGenerator::next_task_id(dir.path(), "main", "PLAN-001", "test-plan").expect("ok");
    assert_eq!(id, "TASK-001");
}

#[test]
fn test_next_task_id_incremental() {
    let dir = create_test_repo();
    setup_test_plan(dir.path(), "main", "PLAN-001", "test-plan");
    let tasks = dir.path().join(".agent/specs/main/PLAN-001-test-plan/tasks");
    std::fs::create_dir_all(tasks.join("TASK-001-a")).expect("ok");
    std::fs::create_dir_all(tasks.join("TASK-002-b")).expect("ok");
    std::fs::create_dir_all(tasks.join("TASK-003-c")).expect("ok");
    let id = TaskIdGenerator::next_task_id(dir.path(), "main", "PLAN-001", "test-plan").expect("ok");
    assert_eq!(id, "TASK-004");
}

#[test]
fn test_parse_plan_id() {
    assert_eq!(PlanIdGenerator::parse_plan_id("PLAN-005").expect("ok"), 5);
    assert_eq!(PlanIdGenerator::parse_plan_id("PLAN-010").expect("ok"), 10);
}

#[test]
fn test_parse_task_id() {
    assert_eq!(TaskIdGenerator::parse_task_id("TASK-010").expect("ok"), 10);
    assert_eq!(TaskIdGenerator::parse_task_id("TASK-042").expect("ok"), 42);
}

#[test]
fn test_parse_invalid_id() {
    assert!(PlanIdGenerator::parse_plan_id("INVALID").is_err());
    assert!(TaskIdGenerator::parse_task_id("INVALID").is_err());
}

#[test]
fn test_slugify() {
    let slug = crate::overlord::id_generator::slugify("OAuth 2.0 Flow");
    assert_eq!(slug, "oauth-20-flow");
}

#[test]
fn test_format_dir_name() {
    let name = crate::overlord::id_generator::format_dir_name("PLAN-001", "oauth-flow");
    assert_eq!(name, "PLAN-001-oauth-flow");
}

// ── Concurrency Checker Tests ───────────────────────────────────────────────

#[test]
fn test_count_running_zero() {
    let dir = create_test_repo();
    setup_test_plan(dir.path(), "main", "PLAN-001", "test-plan");
    let count = ConcurrencyChecker::count_running(dir.path(), "main", "PLAN-001", "test-plan").expect("ok");
    assert_eq!(count, 0);
}

#[test]
fn test_is_concurrency_sensitive() {
    assert!(is_concurrency_sensitive(&TaskStatusValue::Running));
    assert!(is_concurrency_sensitive(&TaskStatusValue::Reviewing));
    assert!(!is_concurrency_sensitive(&TaskStatusValue::Backlog));
    assert!(!is_concurrency_sensitive(&TaskStatusValue::Completed));
}

// ── Dependency Resolver Tests ───────────────────────────────────────────────

#[test]
fn test_no_dependencies_met() {
    let dir = create_test_repo();
    setup_test_plan(dir.path(), "main", "PLAN-001", "test-plan");
    create_task_status(dir.path(), "main", "PLAN-001", "test-plan", "TASK-001", "task-one",
        TaskStatusValue::Backlog, None, vec![]);
    let met = DependencyResolver::are_all_dependencies_met(
        dir.path(), "main", "PLAN-001", "test-plan", "TASK-001", "task-one"
    ).expect("ok");
    assert!(met);
}

#[test]
fn test_dependency_graph() {
    let dir = create_test_repo();
    setup_test_plan(dir.path(), "main", "PLAN-001", "test-plan");
    create_task_status(dir.path(), "main", "PLAN-001", "test-plan", "TASK-001", "task-one",
        TaskStatusValue::Backlog, None, vec![]);
    create_task_status(dir.path(), "main", "PLAN-001", "test-plan", "TASK-002", "task-two",
        TaskStatusValue::Backlog, None, vec!["TASK-001".to_string()]);
    let graph = DependencyResolver::build_dependency_graph(
        dir.path(), "main", "PLAN-001", "test-plan"
    ).expect("ok");
    assert_eq!(graph.len(), 2);
    assert!(graph.get("TASK-001").unwrap().is_empty());
    assert_eq!(graph.get("TASK-002").unwrap(), &vec!["TASK-001".to_string()]);
}

#[test]
fn test_detect_cycles_no_cycle() {
    let mut graph = HashMap::new();
    graph.insert("A".to_string(), vec!["B".to_string()]);
    graph.insert("B".to_string(), vec!["C".to_string()]);
    graph.insert("C".to_string(), vec![]);
    assert!(!DependencyResolver::detect_cycles(&graph));
}

#[test]
fn test_detect_cycles_with_cycle() {
    let mut graph = HashMap::new();
    graph.insert("A".to_string(), vec!["B".to_string()]);
    graph.insert("B".to_string(), vec!["C".to_string()]);
    graph.insert("C".to_string(), vec!["A".to_string()]);
    assert!(DependencyResolver::detect_cycles(&graph));
}

// ── Heartbeat Monitor Tests ─────────────────────────────────────────────────

#[test]
fn test_fresh_heartbeat_not_stale() {
    let monitor = HeartbeatMonitor::new(30);
    let now = chrono::Utc::now().to_rfc3339();
    let (stale, _elapsed) = monitor.is_heartbeat_stale(&now).expect("ok");
    assert!(!stale);
}

#[test]
fn test_old_heartbeat_is_stale() {
    let monitor = HeartbeatMonitor::new(30);
    let old = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let (stale, _elapsed) = monitor.is_heartbeat_stale(&old).expect("ok");
    assert!(stale);
}

#[test]
fn test_stale_threshold_default() {
    let monitor = HeartbeatMonitor::new(30);
    assert_eq!(monitor.get_stale_threshold(), Duration::from_secs(30 * 60));
}

#[test]
fn test_detect_stale_tasks_empty() {
    let dir = create_test_repo();
    std::fs::create_dir_all(dir.path().join(".agent/specs/main")).expect("ok");
    let monitor = HeartbeatMonitor::new(30);
    let stale = monitor.detect_stale_tasks(dir.path(), "main").expect("ok");
    assert!(stale.is_empty());
}

// ── Scheduler Tests ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_scheduler_start_stop() {
    let dir = create_test_repo();
    let scheduler = std::sync::Arc::new(OverlordScheduler::new(dir.path().to_path_buf()));

    assert!(!scheduler.is_running());

    let handle = tokio::spawn({
        let s = scheduler.clone();
        async move { let _ = s.start().await; }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(scheduler.is_running());

    scheduler.stop();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!scheduler.is_running());

    let _ = handle.await;
}

#[tokio::test]
async fn test_scheduler_generate_plan_id() {
    let dir = create_test_repo();
    let scheduler = OverlordScheduler::new(dir.path().to_path_buf());
    let id = scheduler.generate_plan_id("main").await.expect("ok");
    assert_eq!(id, "PLAN-001");
}

#[tokio::test]
async fn test_scheduler_generate_task_id() {
    let dir = create_test_repo();
    setup_test_plan(dir.path(), "main", "PLAN-001", "test-plan");
    let scheduler = OverlordScheduler::new(dir.path().to_path_buf());
    let id = scheduler.generate_task_id("main", "PLAN-001", "test-plan").await.expect("ok");
    assert_eq!(id, "TASK-001");
}

#[tokio::test]
async fn test_transition_task_status_invalid() {
    let dir = create_test_repo();
    setup_test_plan(dir.path(), "main", "PLAN-001", "test-plan");
    create_task_status(dir.path(), "main", "PLAN-001", "test-plan", "TASK-001", "task-one",
        TaskStatusValue::Backlog, None, vec![]);
    let scheduler = OverlordScheduler::new(dir.path().to_path_buf());
    let result = scheduler.transition_task_status(
        "main", "PLAN-001", "test-plan", "TASK-001", "task-one",
        TaskStatusValue::Running, "test-agent"
    ).await;
    assert!(matches!(result, Err(OverlordError::InvalidTransition { .. })));
}
