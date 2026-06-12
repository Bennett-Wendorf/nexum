//! Per-role session configuration.
//!
//! This module defines configuration types for ACP agent sessions based on
//! their assigned role, including default timeouts, tool permissions, and
//! permission policies.

use super::client::SessionCreateParams;

/// Configuration for a specific agent role.
#[derive(Debug, Clone)]
pub struct RoleConfig {
    pub role: super::session::AgentRole,
    pub default_timeout: std::time::Duration,
    pub tool_permissions: Vec<String>,
    pub permission_policy: super::permissions::PermissionPolicy,
    pub max_tokens: Option<u32>,
    pub model_preference: Option<String>,
}

/// Task-level overrides that can be merged into a `RoleConfig`.
#[derive(Debug, Clone, Default)]
pub struct TaskConfigOverrides {
    pub timeout: Option<std::time::Duration>,
    pub tool_permissions: Option<Vec<String>>,
    pub model_preference: Option<String>,
}

/// Generate a default `RoleConfig` for the given agent role.
pub fn default_role_config(role: super::session::AgentRole) -> RoleConfig {
    let permission_policy = super::permissions::default_policy_for_role(role.clone());

    match role {
        super::session::AgentRole::Builder => RoleConfig {
            role: super::session::AgentRole::Builder,
            default_timeout: std::time::Duration::from_secs(1800), // 30 minutes
            tool_permissions: vec!["file-read".into(), "file-write".into(), "command-execution".into(), "network-request".into()],
            permission_policy,
            max_tokens: None,
            model_preference: None,
        },
        super::session::AgentRole::Reviewer => RoleConfig {
            role: super::session::AgentRole::Reviewer,
            default_timeout: std::time::Duration::from_secs(900), // 15 minutes
            tool_permissions: vec!["file-read".into()],
            permission_policy,
            max_tokens: None,
            model_preference: None,
        },
        super::session::AgentRole::Planner => RoleConfig {
            role: super::session::AgentRole::Planner,
            default_timeout: std::time::Duration::from_secs(1200), // 20 minutes
            tool_permissions: vec!["file-read".into(), "file-write".into()],
            permission_policy,
            max_tokens: None,
            model_preference: None,
        },
        super::session::AgentRole::SecurityConsultant => RoleConfig {
            role: super::session::AgentRole::SecurityConsultant,
            default_timeout: std::time::Duration::from_secs(1200), // 20 minutes
            tool_permissions: vec!["file-read".into(), "security-scan".into()],
            permission_policy,
            max_tokens: None,
            model_preference: None,
        },
    }
}

/// Merge task-level overrides into a base `RoleConfig`.
pub fn merge_with_task_config(
    role_config: &RoleConfig,
    task_overrides: &TaskConfigOverrides,
) -> RoleConfig {
    let mut config = role_config.clone();

    if let Some(timeout) = task_overrides.timeout {
        config.default_timeout = timeout;
    }

    if let Some(permissions) = &task_overrides.tool_permissions {
        config.tool_permissions = permissions.clone();
    }

    if let Some(model) = &task_overrides.model_preference {
        config.model_preference = Some(model.clone());
    }

    config
}

/// Convert a `RoleConfig` into `SessionCreateParams` for the ACP client.
pub fn to_session_params(
    config: &RoleConfig,
    prompt: &str,
    worktree_path: &std::path::Path,
) -> SessionCreateParams {
    SessionCreateParams {
        prompt: prompt.to_string(),
        working_directory: Some(worktree_path.to_string_lossy().to_string()),
        tool_permissions: Some(config.tool_permissions.clone()),
        timeout_seconds: Some(config.default_timeout.as_secs()),
    }
}
