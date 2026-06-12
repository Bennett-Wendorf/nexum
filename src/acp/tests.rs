//! Unit tests for the ACP module.
//!
//! These tests cover event deserialization, permission handling, configuration,
//! error types, and session state logic. Subprocess and JSON-RPC integration
//! tests require real ACP agents and are covered in integration tests.

mod tests {
    use std::collections::HashSet;

    use crate::acp::events::{ACPEvent, CompletionStatus, EventStream};
    use crate::acp::permissions::{
        default_policy_for_role, PermissionAction, PermissionDecision,
        PermissionHandler, PermissionPolicy, PermissionRequest,
    };
    use crate::acp::config::{default_role_config, merge_with_task_config, to_session_params, TaskConfigOverrides};
    use crate::acp::session::AgentRole;
    use crate::acp::errors::{ACPError, Result as ACPResult};
    use crate::acp::events::{is_terminal_event, requires_response, session_id, log_event};

    // ── Event Deserialization Tests ──

    #[test]
    fn test_event_deserialization_progress() {
        let json = r#"{
            "type": "progress",
            "session_id": "sess-1",
            "message": "Working on task",
            "percentage": 50.0,
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::Progress { session_id, message, percentage, .. } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(message, "Working on task");
                assert_eq!(percentage, Some(50.0));
            }
            _ => panic!("Expected Progress event"),
        }
    }

    #[test]
    fn test_event_deserialization_tool_call() {
        let json = r#"{
            "type": "tool-call",
            "session_id": "sess-1",
            "tool_name": "file-read",
            "arguments": {"path": "/tmp/test.txt"},
            "call_id": "call-1",
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::ToolCall { session_id, tool_name, call_id, .. } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(tool_name, "file-read");
                assert_eq!(call_id, "call-1");
            }
            _ => panic!("Expected ToolCall event"),
        }
    }

    #[test]
    fn test_event_deserialization_completion() {
        let json = r#"{
            "type": "completion",
            "session_id": "sess-1",
            "status": "success",
            "summary": "Task completed",
            "artifacts": ["/tmp/output.txt"],
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::Completion { session_id, status, summary, artifacts, .. } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(status, CompletionStatus::Success);
                assert_eq!(summary, Some("Task completed".to_string()));
                assert_eq!(artifacts, Some(vec!["/tmp/output.txt".to_string()]));
            }
            _ => panic!("Expected Completion event"),
        }
    }

    #[test]
    fn test_event_deserialization_permission_request() {
        let json = r#"{
            "type": "permission-request",
            "session_id": "sess-1",
            "request_id": "req-1",
            "action": "file-write",
            "details": {"path": "/tmp/test.txt"},
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::PermissionRequest { session_id, request_id, action, .. } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(request_id, "req-1");
                assert_eq!(action, "file-write");
            }
            _ => panic!("Expected PermissionRequest event"),
        }
    }

    #[test]
    fn test_event_deserialization_error() {
        let json = r#"{
            "type": "error",
            "session_id": "sess-1",
            "error_code": "E001",
            "message": "Something went wrong",
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::Error { session_id, error_code, message, .. } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(error_code, "E001");
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Expected Error event"),
        }
    }

    // ── Event Helper Function Tests ──

    #[test]
    fn test_is_terminal_event() {
        let completion = ACPEvent::Completion {
            session_id: "s1".into(),
            status: CompletionStatus::Success,
            summary: None,
            artifacts: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        let error = ACPEvent::Error {
            session_id: "s1".into(),
            error_code: "E001".into(),
            message: "fail".into(),
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        let progress = ACPEvent::Progress {
            session_id: "s1".into(),
            message: "working".into(),
            percentage: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
        };

        assert!(is_terminal_event(&completion));
        assert!(is_terminal_event(&error));
        assert!(!is_terminal_event(&progress));
    }

    #[test]
    fn test_requires_response() {
        let permission = ACPEvent::PermissionRequest {
            session_id: "s1".into(),
            request_id: "r1".into(),
            action: "file-write".into(),
            details: serde_json::json!({}),
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        let question = ACPEvent::Question {
            session_id: "s1".into(),
            question_id: "q1".into(),
            content: "What should I do?".into(),
            options: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        let progress = ACPEvent::Progress {
            session_id: "s1".into(),
            message: "working".into(),
            percentage: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
        };

        assert!(requires_response(&permission));
        assert!(requires_response(&question));
        assert!(!requires_response(&progress));
    }

    #[test]
    fn test_session_id_extraction() {
        let events: Vec<ACPEvent> = vec![
            ACPEvent::Progress { session_id: "test-session".into(), message: "m".into(), percentage: None, timestamp: "t".into() },
            ACPEvent::ToolCall { session_id: "test-session".into(), tool_name: "t".into(), arguments: serde_json::json!({}), call_id: "c".into(), timestamp: "t".into() },
            ACPEvent::ToolResult { session_id: "test-session".into(), call_id: "c".into(), success: true, output: None, error: None, timestamp: "t".into() },
            ACPEvent::Question { session_id: "test-session".into(), question_id: "q".into(), content: "c".into(), options: None, timestamp: "t".into() },
            ACPEvent::PermissionRequest { session_id: "test-session".into(), request_id: "r".into(), action: "a".into(), details: serde_json::json!({}), timestamp: "t".into() },
            ACPEvent::Completion { session_id: "test-session".into(), status: CompletionStatus::Success, summary: None, artifacts: None, timestamp: "t".into() },
            ACPEvent::Error { session_id: "test-session".into(), error_code: "e".into(), message: "m".into(), timestamp: "t".into() },
        ];
        for event in events {
            assert_eq!(session_id(&event), "test-session");
        }
    }

    #[test]
    fn test_log_event_compiles() {
        // Verify log_event compiles and runs without panicking
        let event = ACPEvent::Progress {
            session_id: "s1".into(),
            message: "test".into(),
            percentage: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        log_event(&event);
    }

    // ── EventStream Tests ──

    #[test]
    fn test_event_stream_publish_subscribe() {
        let stream = EventStream::new(1024);
        let mut rx = stream.subscribe();

        let event = ACPEvent::Progress {
            session_id: "s1".into(),
            message: "test".into(),
            percentage: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        stream.publish(event).expect("should publish");

        let received = rx.try_recv().expect("should receive");
        assert_eq!(session_id(&received), "s1");
    }

    #[test]
    fn test_event_stream_multiple_subscribers() {
        let stream = EventStream::new(1024);
        let mut rx1 = stream.subscribe();
        let mut rx2 = stream.subscribe();

        let event = ACPEvent::Progress {
            session_id: "s1".into(),
            message: "test".into(),
            percentage: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        stream.publish(event).expect("should publish");

        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    // ── Permission Handling Tests ──

    #[test]
    fn test_auto_approve() {
        let policy = PermissionPolicy {
            auto_approve: HashSet::from(["file-read".to_string()]),
            auto_deny: HashSet::new(),
            require_approval: HashSet::new(),
        };
        let handler = PermissionHandler::new(policy);
        let request = PermissionRequest {
            request_id: "r1".into(),
            session_id: "s1".into(),
            action: PermissionAction::FileRead,
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(handler.evaluate(&request), PermissionDecision::Approved);
    }

    #[test]
    fn test_auto_deny() {
        let policy = PermissionPolicy {
            auto_approve: HashSet::new(),
            auto_deny: HashSet::from(["network-request".to_string()]),
            require_approval: HashSet::new(),
        };
        let handler = PermissionHandler::new(policy);
        let request = PermissionRequest {
            request_id: "r1".into(),
            session_id: "s1".into(),
            action: PermissionAction::NetworkRequest { url: "http://example.com".into(), method: "GET".into() },
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(handler.evaluate(&request), PermissionDecision::Denied);
    }

    #[test]
    fn test_pending() {
        let policy = PermissionPolicy {
            auto_approve: HashSet::from(["file-read".to_string()]),
            auto_deny: HashSet::new(),
            require_approval: HashSet::from(["command-execution".to_string()]),
        };
        let handler = PermissionHandler::new(policy);
        let request = PermissionRequest {
            request_id: "r1".into(),
            session_id: "s1".into(),
            action: PermissionAction::CommandExecution { command: "ls".into() },
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(handler.evaluate(&request), PermissionDecision::Pending);
    }

    #[test]
    fn test_default_policy_builder() {
        let policy = default_policy_for_role(AgentRole::Builder);
        assert!(policy.auto_approve.contains("file-read"));
        assert!(policy.auto_approve.contains("file-write"));
        assert!(policy.require_approval.contains("command-execution"));
    }

    #[test]
    fn test_default_policy_reviewer() {
        let policy = default_policy_for_role(AgentRole::Reviewer);
        assert!(policy.auto_approve.contains("file-read"));
        assert!(policy.auto_deny.contains("file-write"));
        assert!(policy.auto_deny.contains("command-execution"));
    }

    #[test]
    fn test_default_policy_planner() {
        let policy = default_policy_for_role(AgentRole::Planner);
        assert!(policy.auto_approve.contains("file-read"));
        assert!(policy.require_approval.contains("file-write"));
    }

    #[test]
    fn test_default_policy_security_consultant() {
        let policy = default_policy_for_role(AgentRole::SecurityConsultant);
        assert!(policy.auto_approve.contains("file-read"));
        assert!(policy.auto_deny.contains("file-write"));
        assert!(policy.auto_deny.contains("network-request"));
    }

    // ── Configuration Tests ──

    #[test]
    fn test_default_role_config_builder() {
        let config = default_role_config(AgentRole::Builder);
        assert_eq!(config.default_timeout.as_secs(), 1800); // 30 minutes
        assert!(config.tool_permissions.contains(&"file-read".to_string()));
        assert!(config.tool_permissions.contains(&"file-write".to_string()));
    }

    #[test]
    fn test_default_role_config_reviewer() {
        let config = default_role_config(AgentRole::Reviewer);
        assert_eq!(config.default_timeout.as_secs(), 900); // 15 minutes
        assert_eq!(config.tool_permissions, vec!["file-read".to_string()]);
    }

    #[test]
    fn test_default_role_config_planner() {
        let config = default_role_config(AgentRole::Planner);
        assert_eq!(config.default_timeout.as_secs(), 1200); // 20 minutes
    }

    #[test]
    fn test_default_role_config_security_consultant() {
        let config = default_role_config(AgentRole::SecurityConsultant);
        assert_eq!(config.default_timeout.as_secs(), 1200); // 20 minutes
    }

    #[test]
    fn test_merge_with_task_config() {
        let role_config = default_role_config(AgentRole::Builder);
        let overrides = TaskConfigOverrides {
            timeout: Some(std::time::Duration::from_secs(600)),
            tool_permissions: Some(vec!["file-read".to_string()]),
            model_preference: Some("gpt-4".to_string()),
        };
        let merged = merge_with_task_config(&role_config, &overrides);
        assert_eq!(merged.default_timeout.as_secs(), 600);
        assert_eq!(merged.tool_permissions, vec!["file-read".to_string()]);
        assert_eq!(merged.model_preference, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_to_session_params() {
        let config = default_role_config(AgentRole::Builder);
        let params = to_session_params(&config, "Build this feature", std::path::Path::new("/tmp/worktree"));
        assert_eq!(params.prompt, "Build this feature");
        assert_eq!(params.working_directory, Some("/tmp/worktree".to_string()));
        assert!(params.tool_permissions.is_some());
        assert_eq!(params.timeout_seconds, Some(1800));
    }

    // ── Error Type Tests ──

    #[test]
    fn test_error_display_subprocess_spawn() {
        let err = ACPError::SubprocessSpawn {
            agent_id: "test-agent".into(),
            command: "opencode acp".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test-agent"));
        assert!(msg.contains("opencode acp"));
    }

    #[test]
    fn test_error_display_session_timeout() {
        let err = ACPError::SessionTimeout {
            session_id: "sess-1".into(),
            duration: std::time::Duration::from_secs(300),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("sess-1"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let acp_err: ACPError = io_err.into();
        match acp_err {
            ACPError::IoBare(ref source) => {
                assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
            }
            _ => panic!("Expected IoBare variant"),
        }
    }

    #[test]
    fn test_acp_result_type() {
        let ok_result: ACPResult<String> = Ok("hello".to_string());
        assert!(ok_result.is_ok());

        let err_result: ACPResult<String> = Err(ACPError::SessionNotFound {
            session_id: "missing".into(),
        });
        assert!(err_result.is_err());
    }

    // ── Event Serialization Tests ──

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = ACPEvent::Progress {
            session_id: "s1".into(),
            message: "test".into(),
            percentage: Some(75.0),
            timestamp: "2025-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        let parsed: ACPEvent = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(session_id(&parsed), "s1");
    }

    #[test]
    fn test_completion_status_serialization() {
        let success = serde_json::to_string(&CompletionStatus::Success).unwrap();
        assert_eq!(success, "\"success\"");

        let failed = serde_json::to_string(&CompletionStatus::Failed).unwrap();
        assert_eq!(failed, "\"failed\"");

        let cancelled = serde_json::to_string(&CompletionStatus::Cancelled).unwrap();
        assert_eq!(cancelled, "\"cancelled\"");
    }

    #[test]
    fn test_tool_result_event_serialization() {
        let json = r#"{
            "type": "tool-result",
            "session_id": "sess-1",
            "call_id": "call-1",
            "success": true,
            "output": "file contents",
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::ToolResult { session_id, call_id, success, output, .. } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(call_id, "call-1");
                assert!(success);
                assert_eq!(output, Some("file contents".to_string()));
            }
            _ => panic!("Expected ToolResult event"),
        }
    }

    #[test]
    fn test_question_event_serialization() {
        let json = r#"{
            "type": "question",
            "session_id": "sess-1",
            "question_id": "q1",
            "content": "What should I do?",
            "options": ["option-a", "option-b"],
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::Question { session_id, question_id, content, options, .. } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(question_id, "q1");
                assert_eq!(content, "What should I do?");
                assert_eq!(options, Some(vec!["option-a".to_string(), "option-b".to_string()]));
            }
            _ => panic!("Expected Question event"),
        }
    }

    // ── PermissionAction serialization tests ──

    #[test]
    fn test_permission_action_file_read() {
        let json = r#"{"file-read": null}"#;
        let action: PermissionAction = serde_json::from_str(json).expect("should deserialize");
        match action {
            PermissionAction::FileRead => {}
            _ => panic!("Expected FileRead"),
        }
    }

    #[test]
    fn test_permission_action_file_write() {
        let json = r#"{"file-write": {"path": "/tmp/test.txt"}}"#;
        let action: PermissionAction = serde_json::from_str(json).expect("should deserialize");
        match action {
            PermissionAction::FileWrite { path } => {
                assert_eq!(path, "/tmp/test.txt");
            }
            _ => panic!("Expected FileWrite"),
        }
    }

    #[test]
    fn test_permission_action_command_execution() {
        let json = r#"{"command-execution": {"command": "ls -la"}}"#;
        let action: PermissionAction = serde_json::from_str(json).expect("should deserialize");
        match action {
            PermissionAction::CommandExecution { command } => {
                assert_eq!(command, "ls -la");
            }
            _ => panic!("Expected CommandExecution"),
        }
    }

    #[test]
    fn test_permission_action_network_request() {
        let json = r#"{"network-request": {"url": "http://example.com", "method": "GET"}}"#;
        let action: PermissionAction = serde_json::from_str(json).expect("should deserialize");
        match action {
            PermissionAction::NetworkRequest { url, method } => {
                assert_eq!(url, "http://example.com");
                assert_eq!(method, "GET");
            }
            _ => panic!("Expected NetworkRequest"),
        }
    }

    // ── Edge cases ──

    #[test]
    fn test_event_with_missing_optional_fields() {
        let json = r#"{
            "type": "progress",
            "session_id": "sess-1",
            "message": "working",
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::Progress { percentage, .. } => {
                assert_eq!(percentage, None);
            }
            _ => panic!("Expected Progress event"),
        }
    }

    #[test]
    fn test_completion_with_no_summary_or_artifacts() {
        let json = r#"{
            "type": "completion",
            "session_id": "sess-1",
            "status": "failed",
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;
        let event: ACPEvent = serde_json::from_str(json).expect("should deserialize");
        match event {
            ACPEvent::Completion { status, summary, artifacts, .. } => {
                assert_eq!(status, CompletionStatus::Failed);
                assert_eq!(summary, None);
                assert_eq!(artifacts, None);
            }
            _ => panic!("Expected Completion event"),
        }
    }

    #[test]
    fn test_permission_handler_other_action() {
        let policy = PermissionPolicy {
            auto_approve: HashSet::from(["custom-action".to_string()]),
            auto_deny: HashSet::new(),
            require_approval: HashSet::new(),
        };
        let handler = PermissionHandler::new(policy);
        let request = PermissionRequest {
            request_id: "r1".into(),
            session_id: "s1".into(),
            action: PermissionAction::Other {
                action: "custom-action".to_string(),
                details: serde_json::json!({"key": "value"}),
            },
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(handler.evaluate(&request), PermissionDecision::Approved);
    }

    #[test]
    fn test_merge_with_empty_overrides() {
        let role_config = default_role_config(AgentRole::Builder);
        let overrides = TaskConfigOverrides::default();
        let merged = merge_with_task_config(&role_config, &overrides);
        assert_eq!(merged.default_timeout, role_config.default_timeout);
        assert_eq!(merged.tool_permissions, role_config.tool_permissions);
        assert_eq!(merged.model_preference, role_config.model_preference);
    }
}
