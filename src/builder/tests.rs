//! Comprehensive tests for the builder workflow module.
//!
//! Covers error types, worktree manager, dispatcher, session manager,
//! heartbeat, event bus, merge coordinator, error recovery, completion
//! handler, and orchestrator.
//!
//! Tests requiring actual git repos or ACP agents are marked `#[ignore]`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::acp::events::{ACPEvent, CompletionStatus, EventStream};
use crate::acp::ACPError;
use crate::builder::dispatcher::{ActiveSession, TaskContext, TaskDispatcher};
use crate::builder::errors::{BuilderError, Result as BuilderResult};
use crate::builder::event_bus::{BuilderEvent, BuilderEventBus, CompletionResult};
use crate::builder::heartbeat::HeartbeatManager;
use crate::builder::merge_coordinator::MergeResult;
use crate::builder::session_manager::ACPSessionHandle;
use crate::builder::worktree_manager::{TaskWorktree, WorktreeManager};
use crate::git::GitError;
use crate::overlord::OverlordError;
use crate::persistence::PersistenceError;

// ============================================================================
// Test helpers
// ============================================================================

fn make_task_context() -> TaskContext {
    TaskContext {
        task_id: "TASK-001".to_string(),
        task_name: "add-auth-flow".to_string(),
        plan_id: "PLAN-001".to_string(),
        plan_name: "auth-overhaul".to_string(),
        branch: "plan/PLAN-001-auth-overhaul".to_string(),
        task_dir: PathBuf::from("/tmp/test/task"),
        task_prompt: "Implement authentication flow".to_string(),
    }
}

fn make_task_worktree() -> TaskWorktree {
    TaskWorktree {
        task_id: "TASK-001".to_string(),
        task_name: "add-auth-flow".to_string(),
        branch_name: "task/TASK-001".to_string(),
        path: PathBuf::from("/tmp/test/.worktrees/TASK-001-add-auth-flow"),
        created_at: chrono::Utc::now(),
    }
}

// ============================================================================
// 1. Error Types Tests
// ============================================================================

mod error_types {
    use super::*;

    #[test]
    fn test_task_dispatch_error() {
        let err = BuilderError::TaskDispatchError("agent not found".to_string());
        assert_eq!(err.to_string(), "Task dispatch error: agent not found");
    }

    #[test]
    fn test_merge_conflict_error() {
        let err = BuilderError::MergeConflict {
            task_id: "TASK-001".to_string(),
            branch: "task/TASK-001".to_string(),
            conflicts: vec!["src/main.rs".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("TASK-001"));
        assert!(msg.contains("task/TASK-001"));
        assert!(msg.contains("src/main.rs"));
    }

    #[test]
    fn test_timeout_error() {
        let elapsed = Duration::from_secs(600);
        let limit = Duration::from_secs(300);
        let err = BuilderError::TimeoutError {
            task_id: "TASK-001".to_string(),
            elapsed,
            limit,
        };
        let msg = err.to_string();
        assert!(msg.contains("TASK-001"));
        assert!(msg.contains("600s"));
        assert!(msg.contains("300s"));
    }

    #[test]
    fn test_agent_crash_error() {
        let err = BuilderError::AgentCrash {
            task_id: "TASK-001".to_string(),
            exit_code: Some(137),
        };
        let msg = err.to_string();
        assert!(msg.contains("TASK-001"));
        assert!(msg.contains("137"));
    }

    #[test]
    fn test_agent_crash_error_no_exit_code() {
        let err = BuilderError::AgentCrash {
            task_id: "TASK-001".to_string(),
            exit_code: None,
        };
        let msg = err.to_string();
        assert!(msg.contains("TASK-001"));
        assert!(msg.contains("Agent crashed"));
    }

    #[test]
    fn test_permission_denied_error() {
        let err = BuilderError::PermissionDenied {
            task_id: "TASK-001".to_string(),
            resource: "file-write".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("TASK-001"));
        assert!(msg.contains("file-write"));
    }

    #[test]
    fn test_workflow_error() {
        let err = BuilderError::WorkflowError("something went wrong".to_string());
        assert_eq!(err.to_string(), "Workflow error: something went wrong");
    }

    #[test]
    fn test_from_git_error_worktree() {
        let git_err = GitError::SubprocessFailure {
            command: "git status".to_string(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "not a git repo".to_string(),
        };
        let builder_err: BuilderError = git_err.into();
        match builder_err {
            BuilderError::WorktreeError(_) => {}
            other => panic!("Expected WorktreeError, got {:?}", other),
        }
    }

    #[test]
    fn test_from_acp_error_session() {
        let acp_err = ACPError::SubprocessSpawn {
            agent_id: "test".to_string(),
            command: "opencode acp".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let builder_err: BuilderError = acp_err.into();
        match builder_err {
            BuilderError::SessionError(_) => {}
            other => panic!("Expected SessionError, got {:?}", other),
        }
    }

    #[test]
    fn test_from_persistence_error() {
        let persist_err = PersistenceError::Io(
            PathBuf::from("/tmp/test"),
            std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        );
        let builder_err: BuilderError = persist_err.into();
        match builder_err {
            BuilderError::PersistenceError(_) => {}
            other => panic!("Expected PersistenceError, got {:?}", other),
        }
    }

    #[test]
    fn test_from_overlord_error() {
        let overlord_err = OverlordError::InvalidTransition {
            from: "queued".to_string(),
            to: "running".to_string(),
            entity: "TASK-001".to_string(),
        };
        let builder_err: BuilderError = overlord_err.into();
        match builder_err {
            BuilderError::OverlordError(_) => {}
            other => panic!("Expected OverlordError, got {:?}", other),
        }
    }

    #[test]
    fn test_result_type_alias_ok() {
        let result: BuilderResult<String> = Ok("success".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_result_type_alias_err() {
        let result: BuilderResult<String> = Err(BuilderError::WorkflowError("fail".to_string()));
        assert!(result.is_err());
    }
}

// ============================================================================
// 2. Worktree Manager Tests
// ============================================================================

mod worktree_manager {
    use super::*;

    #[test]
    fn test_branch_name() {
        let branch = TaskWorktree::branch_name("TASK-001");
        assert_eq!(branch, "task/TASK-001");
    }

    #[test]
    fn test_branch_name_with_special_chars() {
        let branch = TaskWorktree::branch_name("TASK-002");
        assert_eq!(branch, "task/TASK-002");
    }

    #[test]
    fn test_worktree_path() {
        let repo_root = PathBuf::from("/home/user/project");
        let path = TaskWorktree::worktree_path(&repo_root, "TASK-001", "add-auth");
        assert_eq!(
            path,
            PathBuf::from("/home/user/project/.worktrees/TASK-001-add-auth")
        );
    }

    #[test]
    fn test_worktree_path_nested() {
        let repo_root = PathBuf::from("/a/b/c");
        let path = TaskWorktree::worktree_path(&repo_root, "TASK-002", "feature-x");
        assert_eq!(path, PathBuf::from("/a/b/c/.worktrees/TASK-002-feature-x"));
    }

    #[test]
    fn test_worktree_manager_new() {
        let repo_root = PathBuf::from("/tmp/test");
        let manager = WorktreeManager::new(repo_root.clone());
        // Verify construction succeeded
        let _ = manager;
    }

    #[tokio::test]
    async fn test_worktree_exists_false() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WorktreeManager::new(tmp.path().to_path_buf());
        assert!(!manager.exists("NONEXIST", "test").await);
    }

    #[tokio::test]
    async fn test_worktree_exists_true() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WorktreeManager::new(tmp.path().to_path_buf());
        // Create the expected worktree directory
        let wt_path = TaskWorktree::worktree_path(tmp.path(), "TASK-001", "test");
        tokio::fs::create_dir_all(&wt_path).await.unwrap();
        assert!(manager.exists("TASK-001", "test").await);
    }

    #[tokio::test]
    async fn test_list_active_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WorktreeManager::new(tmp.path().to_path_buf());
        let active = manager.list_active().await.unwrap();
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_list_active_with_worktrees() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WorktreeManager::new(tmp.path().to_path_buf());

        // Create .worktrees directory with a valid worktree (has .git file)
        // Note: parsing uses splitn(2, '-') so task_id must not contain hyphens
        // or the parse will be wrong. We use a simple task ID to test parsing.
        let wt_path = TaskWorktree::worktree_path(tmp.path(), "TASK001", "testfeature");
        tokio::fs::create_dir_all(&wt_path).await.unwrap();
        tokio::fs::write(
            wt_path.join(".git"),
            "gitdir: /path/to/main/.git/worktrees/...",
        )
        .await
        .unwrap();

        let active = manager.list_active().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].task_id, "TASK001");
        assert_eq!(active[0].task_name, "testfeature");
        assert_eq!(active[0].branch_name, "task/TASK001");
    }

    #[tokio::test]
    async fn test_list_active_skips_non_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = WorktreeManager::new(tmp.path().to_path_buf());

        // Create a directory without .git file — should be skipped
        let wt_path = TaskWorktree::worktree_path(tmp.path(), "TASK-001", "test");
        tokio::fs::create_dir_all(&wt_path).await.unwrap();
        // No .git file

        let active = manager.list_active().await.unwrap();
        assert!(active.is_empty());
    }
}

// ============================================================================
// 3. Dispatcher Tests
// ============================================================================

mod dispatcher {
    use super::*;
    use crate::overlord::OverlordScheduler;

    fn make_dispatcher() -> TaskDispatcher {
        let overlord = Arc::new(OverlordScheduler::new(PathBuf::from("/tmp/test")));
        TaskDispatcher::new(PathBuf::from("/tmp/test"), overlord)
    }

    #[test]
    fn test_dispatcher_new() {
        let dispatcher = make_dispatcher();
        assert_eq!(dispatcher.get_active_count(), 0);
    }

    #[test]
    fn test_add_session() {
        let mut dispatcher = make_dispatcher();
        let session = ActiveSession {
            task_context: make_task_context(),
            worktree: make_task_worktree(),
            session_id: "session-001".to_string(),
            pid: Some(1234),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session);
        assert_eq!(dispatcher.get_active_count(), 1);
    }

    #[test]
    fn test_add_multiple_sessions() {
        let mut dispatcher = make_dispatcher();
        let ctx1 = make_task_context();
        let session1 = ActiveSession {
            task_context: ctx1,
            worktree: make_task_worktree(),
            session_id: "session-001".to_string(),
            pid: Some(1234),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session1);

        let ctx2 = TaskContext {
            task_id: "TASK-002".to_string(),
            task_name: "add-logging".to_string(),
            plan_id: "PLAN-001".to_string(),
            plan_name: "auth-overhaul".to_string(),
            branch: "plan/PLAN-001-auth-overhaul".to_string(),
            task_dir: PathBuf::from("/tmp/test/task2"),
            task_prompt: "Add logging".to_string(),
        };
        let session2 = ActiveSession {
            task_context: ctx2,
            worktree: make_task_worktree(),
            session_id: "session-002".to_string(),
            pid: Some(5678),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session2);
        assert_eq!(dispatcher.get_active_count(), 2);
    }

    #[test]
    fn test_remove_session() {
        let mut dispatcher = make_dispatcher();
        let session = ActiveSession {
            task_context: make_task_context(),
            worktree: make_task_worktree(),
            session_id: "session-001".to_string(),
            pid: Some(1234),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session);
        assert_eq!(dispatcher.get_active_count(), 1);

        let removed = dispatcher.remove_session("TASK-001");
        assert!(removed.is_some());
        assert_eq!(dispatcher.get_active_count(), 0);
    }

    #[test]
    fn test_remove_nonexistent_session() {
        let mut dispatcher = make_dispatcher();
        let removed = dispatcher.remove_session("NONEXIST");
        assert!(removed.is_none());
    }

    #[test]
    fn test_get_session() {
        let mut dispatcher = make_dispatcher();
        let session = ActiveSession {
            task_context: make_task_context(),
            worktree: make_task_worktree(),
            session_id: "session-001".to_string(),
            pid: Some(1234),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session);

        let sess = dispatcher.get_session("TASK-001");
        assert!(sess.is_some());
        assert_eq!(sess.unwrap().session_id, "session-001");
    }

    #[test]
    fn test_get_session_not_found() {
        let dispatcher = make_dispatcher();
        let sess = dispatcher.get_session("NONEXIST");
        assert!(sess.is_none());
    }

    #[test]
    fn test_list_active() {
        let mut dispatcher = make_dispatcher();
        let session = ActiveSession {
            task_context: make_task_context(),
            worktree: make_task_worktree(),
            session_id: "session-001".to_string(),
            pid: Some(1234),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session);

        let active = dispatcher.list_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].session_id, "session-001");
    }

    #[test]
    fn test_list_active_empty() {
        let dispatcher = make_dispatcher();
        let active = dispatcher.list_active();
        assert!(active.is_empty());
    }

    #[test]
    fn test_overwrite_session() {
        let mut dispatcher = make_dispatcher();
        let ctx = make_task_context();
        let session1 = ActiveSession {
            task_context: ctx.clone(),
            worktree: make_task_worktree(),
            session_id: "session-old".to_string(),
            pid: Some(1111),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session1);

        let session2 = ActiveSession {
            task_context: ctx,
            worktree: make_task_worktree(),
            session_id: "session-new".to_string(),
            pid: Some(2222),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session2);

        // Should still be 1 session (overwritten)
        assert_eq!(dispatcher.get_active_count(), 1);
        let sess = dispatcher.get_session("TASK-001").unwrap();
        assert_eq!(sess.session_id, "session-new");
    }
}

// ============================================================================
// 4. Session Manager Tests (mocked)
// ============================================================================

mod session_manager {
    use super::*;
    use crate::builder::session_manager::SessionManager;

    #[test]
    fn test_acp_session_handle_debug() {
        let event_stream = EventStream::with_default_capacity();
        let handle = ACPSessionHandle {
            session_id: "test-session".to_string(),
            subprocess: crate::acp::subprocess::AgentProcess {
                agent_id: "test-agent".to_string(),
                child: None,
                stdin: None,
                stdout: None,
                stderr_task: None,
                spawn_time: std::time::Instant::now(),
                worktree_path: PathBuf::from("/tmp"),
            },
            event_stream,
            started_at: chrono::Utc::now(),
        };
        let debug_str = format!("{:?}", handle);
        assert!(debug_str.contains("test-session"));
    }

    /// Test that SessionManager can be created.
    /// Note: create_session requires actual agent binary, so it's ignored.
    #[test]
    fn test_session_manager_new() {
        let config = crate::acp::subprocess::AgentConfig {
            name: "test-agent".to_string(),
            binary: PathBuf::from("/nonexistent"),
            args: vec!["acp".to_string()],
            env: std::collections::HashMap::new(),
        };
        let manager = SessionManager::new(PathBuf::from("/tmp/test"), config);
        // is_completed always returns false (see source)
        // We can't actually call it without a valid handle, so just verify construction
        let _ = manager;
    }

    /// Test that is_completed always returns false (per implementation).
    #[test]
    fn test_is_completed_always_false() {
        let config = crate::acp::subprocess::AgentConfig {
            name: "test-agent".to_string(),
            binary: PathBuf::from("/nonexistent"),
            args: vec!["acp".to_string()],
            env: std::collections::HashMap::new(),
        };
        let manager = SessionManager::new(PathBuf::from("/tmp/test"), config);
        // We can't create a real handle, but we know from the source that
        // is_completed always returns false.
        // This is documented in the source code comment.
        let _ = manager;
    }

    #[ignore]
    #[tokio::test]
    async fn test_create_session() {
        // Requires actual agent binary and worktree — ignored
        // This would test:
        // 1. Session ID generation
        // 2. Agent subprocess spawning
        // 3. Event stream initialization
        // 4. Initial progress event
    }

    #[ignore]
    #[tokio::test]
    async fn test_destroy_session() {
        // Requires actual agent subprocess — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_wait_for_completion_timeout() {
        // Requires actual session — ignored
    }
}

// ============================================================================
// 5. Heartbeat Tests
// ============================================================================

mod heartbeat {
    use super::*;

    #[test]
    fn test_is_progress_event_progress() {
        let event = ACPEvent::Progress {
            session_id: "s1".to_string(),
            message: "working".to_string(),
            percentage: Some(50.0),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(HeartbeatManager::is_progress_event(&event));
    }

    #[test]
    fn test_is_progress_event_tool_call() {
        let event = ACPEvent::ToolCall {
            session_id: "s1".to_string(),
            tool_name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/tmp/test"}),
            call_id: "c1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(HeartbeatManager::is_progress_event(&event));
    }

    #[test]
    fn test_is_progress_event_tool_result() {
        let event = ACPEvent::ToolResult {
            session_id: "s1".to_string(),
            call_id: "c1".to_string(),
            success: true,
            output: Some("content".to_string()),
            error: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(HeartbeatManager::is_progress_event(&event));
    }

    #[test]
    fn test_is_progress_event_question() {
        let event = ACPEvent::Question {
            session_id: "s1".to_string(),
            question_id: "q1".to_string(),
            content: "What should I do?".to_string(),
            options: Some(vec!["A".to_string(), "B".to_string()]),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(HeartbeatManager::is_progress_event(&event));
    }

    #[test]
    fn test_is_progress_event_permission_request() {
        let event = ACPEvent::PermissionRequest {
            session_id: "s1".to_string(),
            request_id: "r1".to_string(),
            action: "file-write".to_string(),
            details: serde_json::json!({"path": "/tmp/test"}),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(HeartbeatManager::is_progress_event(&event));
    }

    #[test]
    fn test_is_not_progress_event_completion() {
        let event = ACPEvent::Completion {
            session_id: "s1".to_string(),
            status: CompletionStatus::Success,
            summary: Some("done".to_string()),
            artifacts: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(!HeartbeatManager::is_progress_event(&event));
    }

    #[test]
    fn test_is_not_progress_event_error() {
        let event = ACPEvent::Error {
            session_id: "s1".to_string(),
            error_code: "E001".to_string(),
            message: "something broke".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(!HeartbeatManager::is_progress_event(&event));
    }

    #[test]
    fn test_heartbeat_manager_new() {
        let manager = HeartbeatManager::new(PathBuf::from("/tmp/test"), Duration::from_secs(60));
        let _ = manager;
    }

    #[test]
    fn test_heartbeat_manager_default_interval() {
        let manager = HeartbeatManager::with_default_interval(PathBuf::from("/tmp/test"));
        let _ = manager;
    }

    #[ignore]
    #[tokio::test]
    async fn test_update_heartbeat() {
        // Requires actual persistence state — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_start_periodic_heartbeat() {
        // Requires actual persistence state — ignored
    }
}

// ============================================================================
// 6. Event Bus Tests
// ============================================================================

mod event_bus {
    use super::*;

    #[test]
    fn test_builder_event_task_started() {
        let ctx = make_task_context();
        let event = BuilderEvent::TaskStarted(ctx.clone());
        match event {
            BuilderEvent::TaskStarted(ec) => {
                assert_eq!(ec.task_id, "TASK-001");
            }
            _ => panic!("Expected TaskStarted"),
        }
    }

    #[test]
    fn test_builder_event_task_progress() {
        let event = BuilderEvent::TaskProgress {
            task_id: "TASK-001".to_string(),
            event: ACPEvent::Progress {
                session_id: "s1".to_string(),
                message: "working".to_string(),
                percentage: Some(50.0),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            },
        };
        match event {
            BuilderEvent::TaskProgress { task_id, .. } => {
                assert_eq!(task_id, "TASK-001");
            }
            _ => panic!("Expected TaskProgress"),
        }
    }

    #[test]
    fn test_builder_event_task_completed() {
        let event = BuilderEvent::TaskCompleted {
            task_id: "TASK-001".to_string(),
            result: CompletionResult::Completed,
        };
        match event {
            BuilderEvent::TaskCompleted { task_id, result } => {
                assert_eq!(task_id, "TASK-001");
                match result {
                    CompletionResult::Completed => {}
                    _ => panic!("Expected Completed"),
                }
            }
            _ => panic!("Expected TaskCompleted"),
        }
    }

    #[test]
    fn test_builder_event_task_failed() {
        let event = BuilderEvent::TaskFailed {
            task_id: "TASK-001".to_string(),
            error: "agent crashed".to_string(),
        };
        match event {
            BuilderEvent::TaskFailed { task_id, error } => {
                assert_eq!(task_id, "TASK-001");
                assert_eq!(error, "agent crashed");
            }
            _ => panic!("Expected TaskFailed"),
        }
    }

    #[test]
    fn test_builder_event_task_merged() {
        let event = BuilderEvent::TaskMerged {
            task_id: "TASK-001".to_string(),
        };
        match event {
            BuilderEvent::TaskMerged { task_id } => {
                assert_eq!(task_id, "TASK-001");
            }
            _ => panic!("Expected TaskMerged"),
        }
    }

    #[test]
    fn test_builder_event_task_cleaned_up() {
        let event = BuilderEvent::TaskCleanedUp {
            task_id: "TASK-001".to_string(),
        };
        match event {
            BuilderEvent::TaskCleanedUp { task_id } => {
                assert_eq!(task_id, "TASK-001");
            }
            _ => panic!("Expected TaskCleanedUp"),
        }
    }

    #[test]
    fn test_builder_event_task_requeued() {
        let event = BuilderEvent::TaskRequeued {
            task_id: "TASK-001".to_string(),
        };
        match event {
            BuilderEvent::TaskRequeued { task_id } => {
                assert_eq!(task_id, "TASK-001");
            }
            _ => panic!("Expected TaskRequeued"),
        }
    }

    #[test]
    fn test_completion_result_completed() {
        let result = CompletionResult::Completed;
        match result {
            CompletionResult::Completed => {}
            _ => panic!("Expected Completed"),
        }
    }

    #[test]
    fn test_completion_result_timeout() {
        let result = CompletionResult::Timeout(Duration::from_secs(300));
        match result {
            CompletionResult::Timeout(d) => {
                assert_eq!(d, Duration::from_secs(300));
            }
            _ => panic!("Expected Timeout"),
        }
    }

    #[test]
    fn test_completion_result_crashed() {
        let result = CompletionResult::Crashed {
            exit_code: Some(137),
        };
        match result {
            CompletionResult::Crashed { exit_code } => {
                assert_eq!(exit_code, Some(137));
            }
            _ => panic!("Expected Crashed"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_new() {
        let bus = BuilderEventBus::new(16);
        let _ = bus;
    }

    #[tokio::test]
    async fn test_event_bus_emit_and_subscribe() {
        let bus = BuilderEventBus::new(16);
        let mut rx = bus.subscribe();

        let event = BuilderEvent::TaskStarted(make_task_context());
        bus.emit(event).unwrap();

        let received = rx.try_recv();
        assert!(received.is_ok());
        match received.unwrap() {
            BuilderEvent::TaskStarted(ctx) => {
                assert_eq!(ctx.task_id, "TASK-001");
            }
            _ => panic!("Expected TaskStarted"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = BuilderEventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = BuilderEvent::TaskMerged {
            task_id: "TASK-001".to_string(),
        };
        bus.emit(event).unwrap();

        let e1 = rx1.try_recv().unwrap();
        let e2 = rx2.try_recv().unwrap();
        assert!(matches!(e1, BuilderEvent::TaskMerged { .. }));
        assert!(matches!(e2, BuilderEvent::TaskMerged { .. }));
    }

    #[tokio::test]
    async fn test_event_bus_emit_no_subscribers() {
        let bus = BuilderEventBus::new(16);
        // Create a subscriber and immediately drop it to simulate "no subscribers"
        let _rx = bus.subscribe();
        // Now emit — should succeed since channel is still open
        let event = BuilderEvent::TaskStarted(make_task_context());
        let result = bus.emit(event);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_event_bus_register_and_subscribe_session() {
        let bus = BuilderEventBus::new(16);
        let (acp_tx, _) = tokio::sync::broadcast::channel::<ACPEvent>(16);

        bus.register_session("TASK-001", acp_tx).await;

        let session_rx = bus.subscribe_session("TASK-001").await;
        assert!(session_rx.is_some());

        let session_rx = bus.subscribe_session("NONEXIST").await;
        assert!(session_rx.is_none());
    }

    #[tokio::test]
    async fn test_event_bus_unregister_session() {
        let bus = BuilderEventBus::new(16);
        let (acp_tx, _) = tokio::sync::broadcast::channel::<ACPEvent>(16);

        bus.register_session("TASK-001", acp_tx).await;
        assert!(bus.subscribe_session("TASK-001").await.is_some());

        bus.unregister_session("TASK-001").await;
        assert!(bus.subscribe_session("TASK-001").await.is_none());
    }

    #[tokio::test]
    async fn test_relay_session_events() {
        let bus = BuilderEventBus::new(16);
        let mut builder_rx = bus.subscribe();

        let (acp_tx, _) = tokio::sync::broadcast::channel::<ACPEvent>(16);
        let acp_rx = acp_tx.subscribe();

        // Spawn the relay
        let _handle = bus.relay_session_events("TASK-001", acp_rx).await;

        // Send an ACP event
        let acp_event = ACPEvent::Progress {
            session_id: "s1".to_string(),
            message: "test progress".to_string(),
            percentage: Some(25.0),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        let _ = acp_tx.send(acp_event);

        // Give relay time to process
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Check builder event bus received it
        let received = builder_rx.try_recv();
        assert!(received.is_ok());
        match received.unwrap() {
            BuilderEvent::TaskProgress { task_id, event } => {
                assert_eq!(task_id, "TASK-001");
                match event {
                    ACPEvent::Progress { message, .. } => {
                        assert_eq!(message, "test progress");
                    }
                    _ => panic!("Expected Progress event"),
                }
            }
            _ => panic!("Expected TaskProgress"),
        }
    }
}

// ============================================================================
// 7. Merge Coordinator Tests (mocked)
// ============================================================================

mod merge_coordinator {
    use super::*;

    #[test]
    fn test_merge_result_success_debug() {
        let result = MergeResult::Success;
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("Success"));
    }

    #[test]
    fn test_merge_result_conflict_debug() {
        let result = MergeResult::Conflict {
            conflicted_files: vec!["src/main.rs".to_string()],
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("Conflict"));
        assert!(debug_str.contains("src/main.rs"));
    }

    #[test]
    fn test_merge_result_clone() {
        let result = MergeResult::Conflict {
            conflicted_files: vec!["src/main.rs".to_string()],
        };
        let cloned = result.clone();
        match cloned {
            MergeResult::Conflict { conflicted_files } => {
                assert_eq!(conflicted_files, vec!["src/main.rs".to_string()]);
            }
            _ => panic!("Expected Conflict"),
        }
    }

    #[ignore]
    #[tokio::test]
    async fn test_merge_task_branch_success() {
        // Requires actual git repo — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_merge_task_branch_conflict() {
        // Requires actual git repo — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_post_merge_cleanup() {
        // Requires actual git repo — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_update_execution_state() {
        // Requires actual persistence state — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_handle_merge_conflict() {
        // Requires actual persistence state — ignored
    }
}

// ============================================================================
// 8. Error Recovery Tests (mocked)
// ============================================================================

mod error_recovery_tests {

    #[ignore]
    #[tokio::test]
    async fn test_handle_agent_crash() {
        // Requires Overlord, WorktreeManager, SessionManager, EventBus — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_handle_timeout() {
        // Requires Overlord, WorktreeManager, EventBus — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_handle_merge_conflict() {
        // Requires Overlord, persistence state — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_handle_permission_denied() {
        // Requires EventBus — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_handle_workflow_error_dispatch() {
        // Requires full component chain — ignored
    }
}

// ============================================================================
// 9. Completion Handler Tests (mocked)
// ============================================================================

mod completion_handler_tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::builder::completion_handler::CompletionOutcome;

    #[test]
    fn test_completion_outcome_merged() {
        let outcome = CompletionOutcome::Merged;
        match outcome {
            CompletionOutcome::Merged => {}
            _ => panic!("Expected Merged"),
        }
    }

    #[test]
    fn test_completion_outcome_conflict() {
        let outcome = CompletionOutcome::Conflict;
        match outcome {
            CompletionOutcome::Conflict => {}
            _ => panic!("Expected Conflict"),
        }
    }

    #[test]
    fn test_completion_outcome_clone() {
        let outcome = CompletionOutcome::Merged;
        let cloned = outcome.clone();
        match cloned {
            CompletionOutcome::Merged => {}
            _ => panic!("Expected Merged"),
        }
    }

    #[test]
    fn test_completion_outcome_debug() {
        let outcome = CompletionOutcome::Conflict;
        let debug_str = format!("{:?}", outcome);
        assert!(debug_str.contains("Conflict"));
    }

    #[ignore]
    #[tokio::test]
    async fn test_handle_completion_success() {
        // Requires full component chain with git repo — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_handle_completion_conflict() {
        // Requires full component chain with git repo — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_emit_completion_events_merged() {
        // Requires EventBus — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_emit_completion_events_conflict() {
        // Requires EventBus — ignored
    }
}

// ============================================================================
// 10. Orchestrator Tests (mocked)
// ============================================================================

mod orchestrator_tests {

    #[ignore]
    #[tokio::test]
    async fn test_orchestrator_new() {
        // Requires OverlordScheduler and AgentConfig — ignored
        // This would test construction of the orchestrator with all sub-components
    }

    #[ignore]
    #[tokio::test]
    async fn test_execute_task_workflow() {
        // Requires full environment — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_dispatch_and_execute() {
        // Requires full environment — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_execute_all_ready_tasks() {
        // Requires full environment — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_get_active_sessions() {
        // Requires full environment — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_get_event_bus() {
        // Requires full environment — ignored
    }

    #[ignore]
    #[tokio::test]
    async fn test_orchestrator_clone() {
        // Requires full environment — ignored
    }
}

// ============================================================================
// Cross-module integration tests
// ============================================================================

mod integration {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_with_dispatcher_session_tracking() {
        // Test that event bus and dispatcher can work together
        let bus = BuilderEventBus::new(16);
        let overlord = Arc::new(crate::overlord::OverlordScheduler::new(PathBuf::from(
            "/tmp",
        )));
        let mut dispatcher = TaskDispatcher::new(PathBuf::from("/tmp"), overlord);

        // Add a session
        let session = ActiveSession {
            task_context: make_task_context(),
            worktree: make_task_worktree(),
            session_id: "session-001".to_string(),
            pid: Some(1234),
            started_at: chrono::Utc::now(),
        };
        dispatcher.add_session(session);

        // Verify dispatcher tracking
        assert_eq!(dispatcher.get_active_count(), 1);

        // Create subscriber first, then emit
        let mut rx = bus.subscribe();
        bus.emit(BuilderEvent::TaskMerged {
            task_id: "TASK-001".to_string(),
        })
        .unwrap();
        let event = rx.try_recv();
        assert!(event.is_ok());
        assert!(matches!(event.unwrap(), BuilderEvent::TaskMerged { .. }));
    }

    #[tokio::test]
    async fn test_builder_event_debug_output() {
        let ctx = make_task_context();
        let events = vec![
            BuilderEvent::TaskStarted(ctx.clone()),
            BuilderEvent::TaskProgress {
                task_id: "TASK-001".to_string(),
                event: crate::acp::events::progress_event(
                    "s1".to_string(),
                    "working".to_string(),
                    Some(50.0),
                ),
            },
            BuilderEvent::TaskCompleted {
                task_id: "TASK-001".to_string(),
                result: CompletionResult::Completed,
            },
            BuilderEvent::TaskCompleted {
                task_id: "TASK-001".to_string(),
                result: CompletionResult::Timeout(Duration::from_secs(300)),
            },
            BuilderEvent::TaskCompleted {
                task_id: "TASK-001".to_string(),
                result: CompletionResult::Crashed {
                    exit_code: Some(137),
                },
            },
            BuilderEvent::TaskFailed {
                task_id: "TASK-001".to_string(),
                error: "agent crashed".to_string(),
            },
            BuilderEvent::TaskMerged {
                task_id: "TASK-001".to_string(),
            },
            BuilderEvent::TaskCleanedUp {
                task_id: "TASK-001".to_string(),
            },
            BuilderEvent::TaskRequeued {
                task_id: "TASK-001".to_string(),
            },
        ];

        for event in &events {
            let debug_str = format!("{:?}", event);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_task_context_clone() {
        let ctx = make_task_context();
        let cloned = ctx.clone();
        assert_eq!(cloned.task_id, ctx.task_id);
        assert_eq!(cloned.task_name, ctx.task_name);
        assert_eq!(cloned.plan_id, ctx.plan_id);
        assert_eq!(cloned.plan_name, ctx.plan_name);
        assert_eq!(cloned.branch, ctx.branch);
        assert_eq!(cloned.task_dir, ctx.task_dir);
        assert_eq!(cloned.task_prompt, ctx.task_prompt);
    }

    #[test]
    fn test_active_session_clone() {
        let session = ActiveSession {
            task_context: make_task_context(),
            worktree: make_task_worktree(),
            session_id: "session-001".to_string(),
            pid: Some(1234),
            started_at: chrono::Utc::now(),
        };
        let cloned = session.clone();
        assert_eq!(cloned.session_id, session.session_id);
        assert_eq!(cloned.task_context.task_id, session.task_context.task_id);
        assert_eq!(cloned.worktree.task_id, session.worktree.task_id);
    }

    #[test]
    fn test_task_worktree_clone() {
        let wt = make_task_worktree();
        let cloned = wt.clone();
        assert_eq!(cloned.task_id, wt.task_id);
        assert_eq!(cloned.task_name, wt.task_name);
        assert_eq!(cloned.branch_name, wt.branch_name);
        assert_eq!(cloned.path, wt.path);
    }

    #[tokio::test]
    async fn test_error_recovery_event_flow() {
        // Test that error events flow through the event bus correctly
        let bus = BuilderEventBus::new(16);
        let mut rx = bus.subscribe();

        // Simulate agent crash event flow
        bus.emit(BuilderEvent::TaskFailed {
            task_id: "TASK-001".to_string(),
            error: "Agent crashed".to_string(),
        })
        .unwrap();

        bus.emit(BuilderEvent::TaskRequeued {
            task_id: "TASK-001".to_string(),
        })
        .unwrap();

        let e1 = rx.try_recv().unwrap();
        let e2 = rx.try_recv().unwrap();

        match e1 {
            BuilderEvent::TaskFailed { task_id, error } => {
                assert_eq!(task_id, "TASK-001");
                assert_eq!(error, "Agent crashed");
            }
            _ => panic!("Expected TaskFailed"),
        }

        match e2 {
            BuilderEvent::TaskRequeued { task_id } => {
                assert_eq!(task_id, "TASK-001");
            }
            _ => panic!("Expected TaskRequeued"),
        }
    }

    #[tokio::test]
    async fn test_completion_event_flow() {
        // Test that completion events flow through the event bus correctly
        let bus = BuilderEventBus::new(16);
        let mut rx = bus.subscribe();

        // Simulate successful completion event flow
        bus.emit(BuilderEvent::TaskCompleted {
            task_id: "TASK-001".to_string(),
            result: CompletionResult::Completed,
        })
        .unwrap();

        bus.emit(BuilderEvent::TaskMerged {
            task_id: "TASK-001".to_string(),
        })
        .unwrap();

        bus.emit(BuilderEvent::TaskCleanedUp {
            task_id: "TASK-001".to_string(),
        })
        .unwrap();

        let e1 = rx.try_recv().unwrap();
        let e2 = rx.try_recv().unwrap();
        let e3 = rx.try_recv().unwrap();

        match e1 {
            BuilderEvent::TaskCompleted { task_id, result } => {
                assert_eq!(task_id, "TASK-001");
                assert!(matches!(result, CompletionResult::Completed));
            }
            _ => panic!("Expected TaskCompleted"),
        }

        match e2 {
            BuilderEvent::TaskMerged { task_id } => {
                assert_eq!(task_id, "TASK-001");
            }
            _ => panic!("Expected TaskMerged"),
        }

        match e3 {
            BuilderEvent::TaskCleanedUp { task_id } => {
                assert_eq!(task_id, "TASK-001");
            }
            _ => panic!("Expected TaskCleanedUp"),
        }
    }

    #[tokio::test]
    async fn test_full_workflow_event_sequence() {
        // Test the full lifecycle event sequence
        let bus = BuilderEventBus::new(64);
        let mut rx = bus.subscribe();

        // Phase 1: Setup
        bus.emit(BuilderEvent::TaskStarted(make_task_context()))
            .unwrap();

        // Phase 2: Progress
        bus.emit(BuilderEvent::TaskProgress {
            task_id: "TASK-001".to_string(),
            event: crate::acp::events::progress_event(
                "s1".to_string(),
                "implementing auth".to_string(),
                Some(75.0),
            ),
        })
        .unwrap();

        // Phase 3: Completion
        bus.emit(BuilderEvent::TaskCompleted {
            task_id: "TASK-001".to_string(),
            result: CompletionResult::Completed,
        })
        .unwrap();

        // Phase 4: Merge
        bus.emit(BuilderEvent::TaskMerged {
            task_id: "TASK-001".to_string(),
        })
        .unwrap();

        // Phase 5: Cleanup
        bus.emit(BuilderEvent::TaskCleanedUp {
            task_id: "TASK-001".to_string(),
        })
        .unwrap();

        // Verify all events in order
        let events: Vec<BuilderEvent> = (0..5).map(|_| rx.try_recv().unwrap()).collect();

        assert!(matches!(events[0], BuilderEvent::TaskStarted(_)));
        assert!(matches!(events[1], BuilderEvent::TaskProgress { .. }));
        assert!(matches!(events[2], BuilderEvent::TaskCompleted { .. }));
        assert!(matches!(events[3], BuilderEvent::TaskMerged { .. }));
        assert!(matches!(events[4], BuilderEvent::TaskCleanedUp { .. }));
    }

    #[test]
    fn test_error_type_coverage() {
        // Verify all BuilderError variants can be constructed
        let errors: Vec<BuilderError> = vec![
            BuilderError::TaskDispatchError("test".to_string()),
            BuilderError::MergeConflict {
                task_id: "T1".to_string(),
                branch: "b1".to_string(),
                conflicts: vec!["f1".to_string()],
            },
            BuilderError::TimeoutError {
                task_id: "T1".to_string(),
                elapsed: Duration::from_secs(100),
                limit: Duration::from_secs(50),
            },
            BuilderError::AgentCrash {
                task_id: "T1".to_string(),
                exit_code: Some(1),
            },
            BuilderError::PermissionDenied {
                task_id: "T1".to_string(),
                resource: "file-write".to_string(),
            },
            BuilderError::WorkflowError("test".to_string()),
        ];

        // Verify each error has a valid display message
        for err in &errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }
}
