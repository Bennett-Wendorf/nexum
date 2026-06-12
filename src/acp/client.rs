//! JSON-RPC 2.0 client wrapper for ACP communication.
//!
//! This module provides a client for sending JSON-RPC requests to ACP agent
//! subprocesses and receiving correlated responses via oneshot channels.
//! Event notifications are delivered through an internal mpsc channel.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::ChildStdin;

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
/// Responses are correlated via pending-responses map using oneshot channels.
/// Event notifications from the agent are delivered through an mpsc channel
/// that the session takes ownership of via `take_event_receiver()`.
pub struct ACPClient {
    agent_id: String,
    /// Protocol version, populated by `initialize()` using interior mutability.
    protocol_version: std::sync::RwLock<String>,
    /// Agent capabilities, populated by `initialize()` using interior mutability.
    capabilities: std::sync::RwLock<ACPCapabilities>,
    writer: tokio::sync::Mutex<BufWriter<ChildStdin>>,
    id_counter: AtomicU64,
    /// Pending response tracking: maps request ID to oneshot sender.
    /// Wrapped in Arc so it can be cloned for the reader task.
    pending_responses:
        Arc<tokio::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>>,
    /// mpsc sender for event notifications (client-to-session delivery).
    event_sender: tokio::sync::mpsc::Sender<ACPEvent>,
    /// mpsc receiver for event notifications. Can be taken once by the session.
    /// Uses Mutex for interior mutability so `take_event_receiver` works with `&self`.
    event_receiver: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<ACPEvent>>>,
}

impl ACPClient {
    /// Create a new ACP client from stdin handle.
    ///
    /// The mpsc event channel is created internally with capacity 1024.
    /// The caller is responsible for spawning the stdout reader task
    /// via [`ACPClient::spawn_reader`] to process responses and events.
    pub fn new(agent_id: String, stdin: ChildStdin) -> Self {
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1024);
        Self {
            agent_id,
            protocol_version: std::sync::RwLock::new(String::new()),
            capabilities: std::sync::RwLock::new(ACPCapabilities {
                sessions: false,
                streaming: false,
                permissions: false,
                tools: vec![],
            }),
            writer: tokio::sync::Mutex::new(BufWriter::new(stdin)),
            id_counter: AtomicU64::new(0),
            pending_responses: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            event_sender,
            event_receiver: std::sync::Mutex::new(Some(event_receiver)),
        }
    }

    /// Generate the next request ID.
    fn next_id(&self) -> u64 {
        self.id_counter.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Send a JSON-RPC request and return a oneshot receiver for the response.
    ///
    /// The request ID is registered in the pending-responses map. When the
    /// reader task receives a matching response, it dispatches the result
    /// through the oneshot channel.
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> std::io::Result<tokio::sync::oneshot::Receiver<serde_json::Value>> {
        let id = self.next_id();
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_responses.lock().await;
            pending.insert(id, tx);
        }
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
        Ok(rx)
    }

    /// Negotiate protocol version and capabilities via the initialize method.
    ///
    /// Sends the handshake request, awaits the response with a 10-second
    /// timeout, validates the protocol version, and extracts capabilities.
    pub async fn initialize(&self) -> Result<()> {
        let rx = self
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

        // Await the response with timeout
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            rx,
        )
        .await
        {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => {
                return Err(ACPError::InitializeFailed {
                    agent_id: self.agent_id.clone(),
                    reason: "response channel closed".to_string(),
                });
            }
            Err(_) => {
                return Err(ACPError::Timeout {
                    operation: "initialize".to_string(),
                    duration: std::time::Duration::from_secs(10),
                });
            }
        };

        // Parse the result field
        let result = response.get("result").ok_or_else(|| ACPError::InitializeFailed {
            agent_id: self.agent_id.clone(),
            reason: "response missing 'result' field".to_string(),
        })?;

        // Validate protocol version
        let agent_version = result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ACPError::InitializeFailed {
                agent_id: self.agent_id.clone(),
                reason: "response missing 'protocolVersion'".to_string(),
            })?;

        if agent_version != ACP_PROTOCOL_VERSION {
            return Err(ACPError::InitializeFailed {
                agent_id: self.agent_id.clone(),
                reason: format!(
                    "protocol version mismatch: nexum requires {}, agent reports {}",
                    ACP_PROTOCOL_VERSION, agent_version
                ),
            });
        }

        // Extract and store capabilities
        let caps_obj = result.get("capabilities").and_then(|c| c.as_object());

        // Parse capabilities from the response object
        if let Some(caps) = caps_obj {
            let sessions = caps.get("sessions").and_then(|v| v.as_bool()).unwrap_or(false);
            let streaming = caps.get("streaming").and_then(|v| v.as_bool()).unwrap_or(false);
            let permissions = caps.get("permissions").and_then(|v| v.as_bool()).unwrap_or(false);
            let tools = caps
                .get("tools")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let mut caps_lock = self.capabilities.write().unwrap();
            *caps_lock = ACPCapabilities {
                sessions,
                streaming,
                permissions,
                tools,
            };
        }

        // Store protocol version
        let mut version_lock = self.protocol_version.write().unwrap();
        *version_lock = agent_version.to_string();

        Ok(())
    }

    /// Create a new ACP session by awaiting the agent's JSON-RPC response.
    ///
    /// Returns the actual `SessionCreateResult` parsed from the agent's response,
    /// including the real session ID assigned by the agent.
    pub async fn sessions_create(&self, params: SessionCreateParams) -> Result<SessionCreateResult> {
        let params_value = serde_json::to_value(params).map_err(|e| ACPError::JsonRpcTransport {
            source: Box::new(e),
        })?;

        let rx = self
            .send_request("sessions/create", params_value)
            .await
            .map_err(|e| ACPError::JsonRpcTransport {
                source: Box::new(e),
            })?;

        // Await the response with timeout
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            rx,
        )
        .await
        {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => {
                return Err(ACPError::JsonRpcError {
                    method: "sessions/create".to_string(),
                    code: -32603,
                    message: "response channel closed".to_string(),
                });
            }
            Err(_) => {
                return Err(ACPError::Timeout {
                    operation: "sessions/create".to_string(),
                    duration: std::time::Duration::from_secs(30),
                });
            }
        };

        // Check for JSON-RPC error response
        if let Some(code) = response
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64())
        {
            let message = response
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(ACPError::JsonRpcError {
                method: "sessions/create".to_string(),
                code: code as i32,
                message,
            });
        }

        // Parse the result
        let result = response.get("result").ok_or_else(|| ACPError::JsonRpcError {
            method: "sessions/create".to_string(),
            code: -32603,
            message: "response missing 'result' field".to_string(),
        })?;

        let session_create_result: SessionCreateResult =
            serde_json::from_value(result.clone()).map_err(|e| ACPError::JsonRpcError {
                method: "sessions/create".to_string(),
                code: -32603,
                message: format!("failed to parse session create result: {}", e),
            })?;

        Ok(session_create_result)
    }

    /// Send a message to a running session (fire-and-forget).
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
        // Fire-and-forget: drop the receiver, response will be cleaned up by reader task
        Ok(())
    }

    /// Destroy a session (fire-and-forget).
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

    /// Respond to a permission request (fire-and-forget).
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

    /// Take ownership of the event receiver. Can only be called once.
    ///
    /// Returns `None` if the receiver has already been taken.
    pub fn take_event_receiver(&self) -> Option<tokio::sync::mpsc::Receiver<ACPEvent>> {
        self.event_receiver.lock().unwrap().take()
    }

    /// Spawn a background task to read JSON-RPC output from stdout.
    ///
    /// Handles both responses (objects with an `"id"` field) and events
    /// (notifications without an `"id"` field). Responses are dispatched
    /// to the matching oneshot sender in the pending-responses map.
    /// Events are sent to the mpsc channel.
    pub fn spawn_reader(
        &self,
        stdout: tokio::process::ChildStdout,
    ) -> tokio::task::JoinHandle<()> {
        let sender = self.event_sender.clone();
        let pending_responses = self.pending_responses.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(obj) = value.as_object() {
                        if let Some(id_value) = obj.get("id") {
                            // This is a response — dispatch to pending request
                            if let Some(id) = id_value.as_u64() {
                                let tx = {
                                    let mut pending = pending_responses.lock().await;
                                    pending.remove(&id)
                                };
                                if let Some(tx) = tx {
                                    let _ = tx.send(value);
                                }
                            }
                            continue;
                        }
                    }
                    // No "id" field — it's a notification/event
                    if let Ok(event) = serde_json::from_value::<ACPEvent>(value) {
                        let _ = sender.send(event).await;
                    } else {
                        tracing::debug!(raw_notification = %line, "unrecognized notification");
                    }
                }
            }
        })
    }

    /// Get the agent ID.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Get the protocol version (populated by `initialize()`).
    pub fn protocol_version(&self) -> String {
        self.protocol_version.read().unwrap().clone()
    }

    /// Get the agent capabilities (populated by `initialize()`).
    pub fn capabilities(&self) -> ACPCapabilities {
        self.capabilities.read().unwrap().clone()
    }
}
