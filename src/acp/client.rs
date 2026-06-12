//! JSON-RPC 2.0 client wrapper for ACP communication.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::ChildStdin;
use tokio::sync::broadcast;

use super::errors::{ACPError, Result};
use super::events::ACPEvent;

/// JSON-RPC protocol version used for all requests.
const JSON_RPC_VERSION: &str = "2.0";

/// ACP protocol version advertised during initialization.
const ACP_PROTOCOL_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACPCapabilities {
    pub sessions: bool,
    pub streaming: bool,
    pub permissions: bool,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateParams {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionCreateResult {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MessageType {
    Prompt,
    FollowUp,
    Feedback,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

/// ACP JSON-RPC client.
///
/// Sends JSON-RPC requests to the agent subprocess via stdin.
/// Event notifications from the agent are handled separately via the
/// stdout reader task (see [`ACPClient::spawn_reader`]).
pub struct ACPClient {
    agent_id: String,
    protocol_version: String,
    capabilities: ACPCapabilities,
    writer: tokio::sync::Mutex<BufWriter<ChildStdin>>,
    id_counter: AtomicU64,
    event_sender: broadcast::Sender<ACPEvent>,
}

impl ACPClient {
    /// Create a new ACP client from stdin/stdout handles.
    ///
    /// The caller is responsible for spawning the stdout reader task
    /// via [`ACPClient::spawn_reader`] to process responses and events.
    pub fn new(
        agent_id: String,
        stdin: ChildStdin,
        event_sender: broadcast::Sender<ACPEvent>,
    ) -> Self {
        Self {
            agent_id,
            protocol_version: ACP_PROTOCOL_VERSION.to_string(),
            capabilities: ACPCapabilities {
                sessions: true,
                streaming: true,
                permissions: true,
                tools: vec![],
            },
            writer: tokio::sync::Mutex::new(BufWriter::new(stdin)),
            id_counter: AtomicU64::new(0),
            event_sender,
        }
    }

    /// Generate the next request ID.
    fn next_id(&self) -> u64 {
        self.id_counter.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Send a JSON-RPC request and return the request ID.
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> std::io::Result<u64> {
        let id = self.next_id();
        let request = JsonRpcRequest {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: method.to_string(),
            params,
            id,
        };
        let json = serde_json::to_string(&request)?;
        let mut writer = self.writer.lock().await;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(id)
    }

    /// Negotiate protocol version and capabilities via the initialize method.
    pub async fn initialize(&self) -> Result<()> {
        let _id = self
            .send_request(
                "initialize",
                serde_json::json!({
                    "protocolVersion": ACP_PROTOCOL_VERSION,
                    "clientName": "nexum",
                }),
            )
            .await
            .map_err(|e| ACPError::JsonRpcTransport {
                source: Box::new(e),
            })?;
        // In a full implementation, we would read the response from stdout
        // and parse capabilities. For now, we use defaults.
        Ok(())
    }

    /// Create a new ACP session.
    pub async fn sessions_create(&self, params: SessionCreateParams) -> Result<SessionCreateResult> {
        let params_value = serde_json::to_value(params).map_err(|e| ACPError::JsonRpcTransport {
            source: Box::new(e),
        })?;
        let _id = self
            .send_request("sessions/create", params_value)
            .await
            .map_err(|e| ACPError::JsonRpcTransport {
                source: Box::new(e),
            })?;
        // In a full implementation, we would await the response and parse it.
        // For now, return a placeholder.
        Ok(SessionCreateResult {
            session_id: format!("session-{}", self.next_id()),
        })
    }

    /// Send a message to a running session.
    pub async fn sessions_message(
        &self,
        session_id: &str,
        content: &str,
        message_type: Option<MessageType>,
    ) -> Result<()> {
        let params = serde_json::json!({
            "sessionId": session_id,
            "content": content,
            "type": message_type,
        });
        let _id = self
            .send_request("sessions/message", params)
            .await
            .map_err(|e| ACPError::JsonRpcTransport {
                source: Box::new(e),
            })?;
        Ok(())
    }

    /// Destroy a session.
    pub async fn sessions_destroy(&self, session_id: &str) -> Result<()> {
        let params = serde_json::json!({ "sessionId": session_id });
        let _id = self
            .send_request("sessions/destroy", params)
            .await
            .map_err(|e| ACPError::JsonRpcTransport {
                source: Box::new(e),
            })?;
        Ok(())
    }

    /// Respond to a permission request.
    pub async fn permission_respond(&self, request_id: &str, approved: bool) -> Result<()> {
        let params = serde_json::json!({
            "requestId": request_id,
            "approved": approved,
        });
        let _id = self
            .send_request("permissions/respond", params)
            .await
            .map_err(|e| ACPError::JsonRpcTransport {
                source: Box::new(e),
            })?;
        Ok(())
    }

    /// Get the event broadcast sender.
    pub fn event_sender(&self) -> &broadcast::Sender<ACPEvent> {
        &self.event_sender
    }

    /// Spawn a background task to read JSON-RPC notifications from stdout.
    ///
    /// Notifications are deserialized as ACP events and published to the
    /// broadcast channel.
    pub fn spawn_reader(
        &self,
        stdout: tokio::process::ChildStdout,
    ) -> tokio::task::JoinHandle<()> {
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                // Parse once as Value, then branch
                let value = match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                // If it has an "id" field, it's a response — skip for now
                if let Some(obj) = value.as_object() {
                    if obj.contains_key("id") {
                        continue;
                    }
                }
                // Try to parse as an ACP event notification
                if let Ok(event) = serde_json::from_value::<ACPEvent>(value) {
                    let _ = sender.send(event);
                } else {
                    tracing::debug!(raw_notification = %line, "unrecognized notification");
                }
            }
        })
    }

    /// Get the agent ID.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Get the protocol version.
    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }

    /// Get the agent capabilities.
    pub fn capabilities(&self) -> &ACPCapabilities {
        &self.capabilities
    }
}
