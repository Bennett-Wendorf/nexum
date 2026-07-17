//! Tests for the persistence layer.

use std::fs;
use std::path::PathBuf;

use super::directory::*;
use super::errors::Result;
use super::io::*;
use super::markdown::*;
use super::operations::*;
use super::schema::*;

fn temp_repo() -> (PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    (dir.path().to_path_buf(), dir)
}

// ── Slugify ─────────────────────────────────────────────────────────────────

#[test]
fn test_slugify_basic() {
    assert_eq!(slugify("OAuth2 Flow"), "oauth2-flow");
    assert_eq!(slugify("hello world"), "hello-world");
    assert_eq!(slugify("UPPER CASE"), "upper-case");
}

#[test]
fn test_slugify_special_chars() {
    assert_eq!(slugify("hello!@#world"), "helloworld");
    assert_eq!(slugify("a b_c"), "a-b_c");
}

// ── Path Resolution ─────────────────────────────────────────────────────────

#[test]
fn test_agent_dir() {
    let path = agent_dir(&PathBuf::from("/repo"));
    assert_eq!(path, PathBuf::from("/repo/.agent"));
}

#[test]
fn test_specs_dir() {
    let path = specs_dir(&PathBuf::from("/repo"), "main");
    assert_eq!(path, PathBuf::from("/repo/.agent/specs/main"));
}

#[test]
fn test_state_dir() {
    let path = state_dir(&PathBuf::from("/repo"), "main");
    assert_eq!(path, PathBuf::from("/repo/.agent/state/main"));
}

#[test]
fn test_plan_dir() {
    let path = plan_dir(&PathBuf::from("/repo"), "main", "PLAN-001", "oauth2-flow");
    assert_eq!(
        path,
        PathBuf::from("/repo/.agent/specs/main/PLAN-001-oauth2-flow")
    );
}

#[test]
fn test_task_dir() {
    let path = task_dir(
        &PathBuf::from("/repo"),
        "main",
        "PLAN-001",
        "oauth2-flow",
        "TASK-001",
        "implement-auth",
    );
    assert_eq!(
        path,
        PathBuf::from("/repo/.agent/specs/main/PLAN-001-oauth2-flow/tasks/TASK-001-implement-auth")
    );
}

// ── File I/O ────────────────────────────────────────────────────────────────

#[test]
fn test_read_write_file() -> Result<()> {
    let (repo, _temp) = temp_repo();
    let path = repo.join("test.txt");
    write_file(&path, "hello world")?;
    let content = read_file(&path)?;
    assert_eq!(content, "hello world");
    assert!(file_exists(&path));
    Ok(())
}

#[test]
fn test_atomic_write() -> Result<()> {
    let (repo, _temp) = temp_repo();
    let path = repo.join("atomic.txt");
    fs::create_dir_all(&repo)?;
    atomic_write(&path, "atomic content")?;
    let content = read_file(&path)?;
    assert_eq!(content, "atomic content");
    Ok(())
}

#[test]
fn test_read_json_not_found() {
    let result = read_json::<String>(&PathBuf::from("/nonexistent/file.json"));
    assert!(result.is_err());
}

#[test]
fn test_write_read_json() -> Result<()> {
    let (repo, _temp) = temp_repo();
    let path = repo.join("data.json");
    fs::create_dir_all(&repo)?;
    let data = serde_json::json!({"key": "value"});
    write_json(&path, &data)?;
    let read_back: serde_json::Value = read_json(&path)?;
    assert_eq!(read_back["key"], "value");
    Ok(())
}

// ── Markdown Parsing ────────────────────────────────────────────────────────

#[test]
fn test_render_plan_markdown() {
    let plan = Plan {
        id: "PLAN-001".into(),
        name: "oauth2-flow".into(),
        status: PlanStatus::Planning,
        created: "2024-01-01".into(),
        branch: "main".into(),
        goal: "Implement OAuth2".into(),
        scope: "Auth module".into(),
        background: "Security requirement".into(),
        tasks: vec![TaskReference {
            id: "TASK-001".into(),
            name: "implement-auth".into(),
            completed: false,
        }],
    };
    let md = render_plan_markdown(&plan);
    assert!(md.contains("# Plan: oauth2-flow"));
    assert!(md.contains("PLAN-001"));
    assert!(md.contains("## Goal"));
    assert!(md.contains("Implement OAuth2"));
}

#[test]
fn test_plan_markdown_roundtrip() -> Result<()> {
    let (repo, _temp) = temp_repo();
    fs::create_dir_all(&repo)?;

    let plan = Plan {
        id: "PLAN-002".into(),
        name: "roundtrip-test".into(),
        status: PlanStatus::Reviewing,
        created: "2025-06-15".into(),
        branch: "feature-x".into(),
        goal: "Verify round-trip serialization".into(),
        scope: "Persistence layer".into(),
        background: "Testing markdown parse and render".into(),
        tasks: vec![
            TaskReference {
                id: "TASK-010".into(),
                name: "task-alpha".into(),
                completed: false,
            },
            TaskReference {
                id: "TASK-011".into(),
                name: "task-beta".into(),
                completed: true,
            },
        ],
    };

    let md = render_plan_markdown(&plan);
    let path = repo.join("plan.md");
    write_file(&path, &md)?;

    let parsed = parse_plan_markdown(&path)?;

    assert_eq!(parsed.id, plan.id);
    assert_eq!(parsed.name, plan.name);
    assert_eq!(parsed.status, plan.status);
    assert_eq!(parsed.created, plan.created);
    assert_eq!(parsed.branch, plan.branch);
    assert_eq!(parsed.goal, plan.goal);
    assert_eq!(parsed.scope, plan.scope);
    assert_eq!(parsed.background, plan.background);
    assert_eq!(parsed.tasks.len(), plan.tasks.len());
    for (got, want) in parsed.tasks.iter().zip(plan.tasks.iter()) {
        assert_eq!(got.id, want.id);
        assert_eq!(got.name, want.name);
        assert_eq!(got.completed, want.completed);
    }

    Ok(())
}

#[test]
fn test_render_task_markdown() {
    let task = Task {
        id: "TASK-001".into(),
        name: "implement-auth".into(),
        parent_plan: "PLAN-001".into(),
        dependencies: vec!["TASK-000".into()],
        description: "Build auth system".into(),
        acceptance_criteria: vec!["Handles tokens".into()],
        files_to_modify: vec!["src/auth.rs".into()],
        background: "Security requirement".into(),
        notes: "Use oauth2 crate".into(),
        status: Some(TaskStatusValue::Backlog),
    };
    let md = render_task_markdown(&task, task.status.as_ref());
    assert!(md.contains("# TASK-001: implement-auth"));
    assert!(md.contains("## Description"));
    assert!(md.contains("Build auth system"));
    assert!(md.contains("- Handles tokens"));
    assert!(md.contains("- src/auth.rs"));
    assert!(md.contains("**Status:** backlog"));
}

#[test]
fn test_task_markdown_roundtrip() -> Result<()> {
    let (repo, _temp) = temp_repo();
    fs::create_dir_all(&repo)?;

    let task = Task {
        id: "TASK-005".into(),
        name: "roundtrip-task".into(),
        parent_plan: "PLAN-003".into(),
        dependencies: vec!["TASK-001".into(), "TASK-002".into()],
        description: "Verify round-trip serialization for tasks".into(),
        acceptance_criteria: vec![
            "All fields are preserved".into(),
            "Dependencies are comma-separated".into(),
        ],
        files_to_modify: vec!["src/lib.rs".into(), "src/persistence/markdown.rs".into()],
        background: "Testing markdown parse and render for tasks".into(),
        notes: "Ensure emphasis splitting is handled correctly".into(),
        status: Some(TaskStatusValue::Running),
    };

    let md = render_task_markdown(&task, task.status.as_ref());
    let path = repo.join("task.md");
    write_file(&path, &md)?;

    let parsed = parse_task_markdown(&path)?;

    assert_eq!(parsed.id, task.id);
    assert_eq!(parsed.name, task.name);
    assert_eq!(parsed.parent_plan, task.parent_plan);
    assert_eq!(parsed.dependencies, task.dependencies);
    assert_eq!(parsed.description, task.description);
    assert_eq!(parsed.acceptance_criteria, task.acceptance_criteria);
    assert_eq!(parsed.files_to_modify, task.files_to_modify);
    assert_eq!(parsed.background, task.background);
    assert_eq!(parsed.notes, task.notes);
    assert_eq!(parsed.status, task.status);

    Ok(())
}

#[test]
fn test_render_task_markdown_no_status() {
    let task = Task {
        id: "TASK-002".into(),
        name: "no-status-task".into(),
        parent_plan: "PLAN-001".into(),
        dependencies: Vec::new(),
        description: "A task without status".into(),
        acceptance_criteria: Vec::new(),
        files_to_modify: Vec::new(),
        background: String::new(),
        notes: String::new(),
        status: None,
    };
    let md = render_task_markdown(&task, task.status.as_ref());
    assert!(!md.contains("**Status:**"));
}

#[test]
fn test_parse_task_markdown_unknown_status() -> Result<()> {
    let (repo, _temp) = temp_repo();
    fs::create_dir_all(&repo)?;

    // Write a task.md with an unknown status value
    let md = "# TASK-099: unknown-status-task\n\n**Parent plan:** PLAN-001\n**Status:** unknown-status\n\n## Description\n\nA task with an unrecognized status value.\n";
    let path = repo.join("task.md");
    write_file(&path, md)?;

    let parsed = parse_task_markdown(&path)?;

    assert_eq!(parsed.id, "TASK-099");
    assert_eq!(parsed.name, "unknown-status-task");
    // Unknown status should gracefully fall back to None
    assert_eq!(parsed.status, None);

    Ok(())
}

// ── CRUD Operations ─────────────────────────────────────────────────────────

#[test]
fn test_create_and_read_plan() -> Result<()> {
    let (repo, _temp) = temp_repo();
    let plan = Plan {
        id: "PLAN-001".into(),
        name: "test-plan".into(),
        status: PlanStatus::Planning,
        created: "2024-01-01".into(),
        branch: "main".into(),
        goal: "Test goal".into(),
        scope: "Test scope".into(),
        background: "Test background".into(),
        tasks: Vec::new(),
    };
    create_plan(&repo, &plan)?;

    // Verify plan.md exists
    let plan_path = plan_markdown_path(&repo, "main", "PLAN-001", "test-plan");
    assert!(plan_path.exists());

    // Verify execution.json exists
    let exec_path = execution_state_path(&repo, "main", "PLAN-001", "test-plan");
    assert!(exec_path.exists());

    // Read back
    let read_plan = read_plan(&repo, "main", "PLAN-001", "test-plan")?;
    assert_eq!(read_plan.id, "PLAN-001");
    assert_eq!(read_plan.name, "test-plan");
    Ok(())
}

#[test]
fn test_create_and_read_task() -> Result<()> {
    let (repo, _temp) = temp_repo();

    // Create plan first
    let plan = Plan {
        id: "PLAN-001".into(),
        name: "test-plan".into(),
        status: PlanStatus::Planning,
        created: "2024-01-01".into(),
        branch: "main".into(),
        goal: "Test".into(),
        scope: "Test".into(),
        background: "Test".into(),
        tasks: Vec::new(),
    };
    create_plan(&repo, &plan)?;

    // Create task
    let task = Task {
        id: "TASK-001".into(),
        name: "test-task".into(),
        parent_plan: "PLAN-001".into(),
        dependencies: Vec::new(),
        description: "Test task description".into(),
        acceptance_criteria: vec!["Criterion 1".into()],
        files_to_modify: vec!["src/test.rs".into()],
        background: "Test background".into(),
        notes: "Test notes".into(),
        status: None,
    };
    create_task(&repo, "main", "PLAN-001", "test-plan", &task)?;

    // Read task back
    let read_task = read_task(
        &repo,
        "main",
        "PLAN-001",
        "test-plan",
        "TASK-001",
        "test-task",
    )?;
    assert_eq!(read_task.id, "TASK-001");
    assert_eq!(read_task.name, "test-task");

    // Read task status
    let status = read_task_status(
        &repo,
        "main",
        "PLAN-001",
        "test-plan",
        "TASK-001",
        "test-task",
    )?;
    assert!(matches!(status.status, TaskStatusValue::Backlog));
    Ok(())
}

#[test]
fn test_update_task_status() -> Result<()> {
    let (repo, _temp) = temp_repo();

    let plan = Plan {
        id: "PLAN-001".into(),
        name: "test-plan".into(),
        status: PlanStatus::Planning,
        created: "2024-01-01".into(),
        branch: "main".into(),
        goal: "Test".into(),
        scope: "Test".into(),
        background: "Test".into(),
        tasks: Vec::new(),
    };
    create_plan(&repo, &plan)?;

    let task = Task {
        id: "TASK-001".into(),
        name: "test-task".into(),
        parent_plan: "PLAN-001".into(),
        dependencies: Vec::new(),
        description: "Test".into(),
        acceptance_criteria: Vec::new(),
        files_to_modify: Vec::new(),
        background: "".into(),
        notes: "".into(),
        status: None,
    };
    create_task(&repo, "main", "PLAN-001", "test-plan", &task)?;

    // Transition to Running
    let status = update_task_status(&UpdateTaskStatusParams {
        path: TaskPathParams {
            repo_root: repo.clone(),
            branch: "main".to_string(),
            plan_id: "PLAN-001".to_string(),
            plan_name: "test-plan".to_string(),
            task_id: "TASK-001".to_string(),
            task_name: "test-task".to_string(),
        },
        new_status: TaskStatusValue::Running,
        by: "builder".to_string(),
    })?;
    assert!(matches!(status.status, TaskStatusValue::Running));
    assert!(status.started_at.is_some());
    assert_eq!(status.attempts, 1);
    assert_eq!(status.transitions.len(), 1);

    // Transition to Completed
    let status = update_task_status(&UpdateTaskStatusParams {
        path: TaskPathParams {
            repo_root: repo.clone(),
            branch: "main".to_string(),
            plan_id: "PLAN-001".to_string(),
            plan_name: "test-plan".to_string(),
            task_id: "TASK-001".to_string(),
            task_name: "test-task".to_string(),
        },
        new_status: TaskStatusValue::Completed,
        by: "reviewer".to_string(),
    })?;
    assert!(matches!(status.status, TaskStatusValue::Completed));
    assert!(status.completed_at.is_some());
    assert_eq!(status.transitions.len(), 2);
    Ok(())
}

#[test]
fn test_add_task_to_execution() -> Result<()> {
    let (repo, _temp) = temp_repo();

    let plan = Plan {
        id: "PLAN-001".into(),
        name: "test-plan".into(),
        status: PlanStatus::Planning,
        created: "2024-01-01".into(),
        branch: "main".into(),
        goal: "Test".into(),
        scope: "Test".into(),
        background: "Test".into(),
        tasks: Vec::new(),
    };
    create_plan(&repo, &plan)?;

    add_task_to_execution(
        &repo,
        "main",
        "PLAN-001",
        "test-plan",
        "TASK-001",
        TaskStatusValue::Backlog,
    )?;

    let state = read_execution_state(&repo, "main", "PLAN-001", "test-plan")?;
    assert_eq!(state.tasks, vec!["TASK-001"]);
    assert_eq!(
        *state.task_status_map.get("TASK-001").unwrap(),
        TaskStatusValue::Backlog
    );
    Ok(())
}

// ── Directory Traversal ─────────────────────────────────────────────────────

#[test]
fn test_list_branches() -> Result<()> {
    let (repo, _temp) = temp_repo();
    let plan = Plan {
        id: "PLAN-001".into(),
        name: "test-plan".into(),
        status: PlanStatus::Planning,
        created: "2024-01-01".into(),
        branch: "feature-branch".into(),
        goal: "Test".into(),
        scope: "Test".into(),
        background: "Test".into(),
        tasks: Vec::new(),
    };
    create_plan(&repo, &plan)?;

    let branches = list_branches(&repo)?;
    assert!(branches.contains(&"feature-branch".to_string()));
    Ok(())
}

#[test]
fn test_find_plan_by_id() -> Result<()> {
    let (repo, _temp) = temp_repo();
    let plan = Plan {
        id: "PLAN-001".into(),
        name: "test-plan".into(),
        status: PlanStatus::Planning,
        created: "2024-01-01".into(),
        branch: "main".into(),
        goal: "Test".into(),
        scope: "Test".into(),
        background: "Test".into(),
        tasks: Vec::new(),
    };
    create_plan(&repo, &plan)?;

    let found = find_plan_by_id(&repo, "main", "PLAN-001")?;
    assert!(found.exists());
    assert!(found.to_string_lossy().contains("PLAN-001-test-plan"));
    Ok(())
}
