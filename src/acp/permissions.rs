//! Permission request handling for ACP sessions.
//!
//! This module defines the types and logic for evaluating and responding
//! to permission requests made by ACP agents during session execution.

use std::collections::HashSet;

use super::errors::Result;
use super::session::{ACPSession, AgentRole};

/// Actions that an agent may request permission to perform.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionAction {
    FileRead,
    FileWrite { path: String },
    CommandExecution { command: String },
    NetworkRequest { url: String, method: String },
    Other { action: String, details: serde_json::Value },
}

/// A permission request issued by an agent during session execution.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub request_id: String,
    pub session_id: String,
    pub action: PermissionAction,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Policy defining which permission actions are auto-approved, auto-denied,
/// or require explicit approval.
#[derive(Debug, Clone)]
pub struct PermissionPolicy {
    pub auto_approve: HashSet<String>,
    pub auto_deny: HashSet<String>,
    pub require_approval: HashSet<String>,
}

/// The decision outcome for a permission request.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionDecision {
    Approved,
    Denied,
    Pending,
}

/// Evaluates permission requests against a configured policy.
pub struct PermissionHandler {
    policy: PermissionPolicy,
}

impl PermissionHandler {
    /// Create a new permission handler with the given policy.
    pub fn new(policy: PermissionPolicy) -> Self {
        Self { policy }
    }

    /// Evaluate a permission request against the configured policy.
    pub fn evaluate(&self, request: &PermissionRequest) -> PermissionDecision {
        let action_key = match &request.action {
            PermissionAction::FileRead => "file-read",
            PermissionAction::FileWrite { .. } => "file-write",
            PermissionAction::CommandExecution { .. } => "command-execution",
            PermissionAction::NetworkRequest { .. } => "network-request",
            PermissionAction::Other { action, .. } => action.as_str(),
        };

        if self.policy.auto_approve.contains(action_key) {
            PermissionDecision::Approved
        } else if self.policy.auto_deny.contains(action_key) {
            PermissionDecision::Denied
        } else {
            PermissionDecision::Pending
        }
    }
}

/// Handle a permission decision by responding to the agent via the session.
pub async fn handle_permission(
    session: &ACPSession,
    request: &PermissionRequest,
    decision: PermissionDecision,
) -> Result<()> {
    match decision {
        PermissionDecision::Approved => {
            session.respond_to_permission(&request.request_id, true).await
        }
        PermissionDecision::Denied => {
            session.respond_to_permission(&request.request_id, false).await
        }
        PermissionDecision::Pending => {
            // Pending requires external decision, don't respond yet
            Ok(())
        }
    }
}

/// Generate a default permission policy based on the agent's role.
pub fn default_policy_for_role(role: AgentRole) -> PermissionPolicy {
    match role {
        AgentRole::Builder => PermissionPolicy {
            auto_approve: HashSet::from(["file-read".to_string(), "file-write".to_string()]),
            auto_deny: HashSet::new(),
            require_approval: HashSet::from(["command-execution".to_string(), "network-request".to_string()]),
        },
        AgentRole::Reviewer => PermissionPolicy {
            auto_approve: HashSet::from(["file-read".to_string()]),
            auto_deny: HashSet::from(["file-write".to_string(), "command-execution".to_string(), "network-request".to_string()]),
            require_approval: HashSet::new(),
        },
        AgentRole::Planner => PermissionPolicy {
            auto_approve: HashSet::from(["file-read".to_string()]),
            auto_deny: HashSet::new(),
            require_approval: HashSet::from(["file-write".to_string(), "command-execution".to_string(), "network-request".to_string()]),
        },
        AgentRole::SecurityConsultant => PermissionPolicy {
            auto_approve: HashSet::from(["file-read".to_string()]),
            auto_deny: HashSet::from(["file-write".to_string(), "command-execution".to_string(), "network-request".to_string()]),
            require_approval: HashSet::new(),
        },
    }
}
