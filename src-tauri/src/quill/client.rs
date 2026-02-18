//! MCP client for communicating with Quill's local server.
//!
//! Uses rmcp's child process transport to spawn a Node.js bridge that
//! proxies JSON-RPC to Quill's local socket.

use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};

/// A meeting as returned by Quill's MCP tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillMeeting {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub participants: Vec<QuillParticipant>,
    #[serde(default)]
    pub has_transcript: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillParticipant {
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Errors from Quill MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum QuillError {
    #[error("Bridge not found at {0}")]
    BridgeNotFound(String),
    #[error("Failed to spawn bridge process: {0}")]
    SpawnFailed(String),
    #[error("MCP connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Meeting not found")]
    MeetingNotFound,
    #[error("Transcript not available")]
    TranscriptNotAvailable,
    #[error("Node.js not found")]
    NodeNotFound,
}

/// MCP client wrapper for Quill's local server.
///
/// Manages a connected rmcp session via child process transport.
/// Each `QuillClient` holds a running MCP service that communicates
/// with the Quill bridge process over stdio.
pub struct QuillClient {
    service: RunningService<RoleClient, ()>,
}

impl QuillClient {
    /// Connect to the Quill MCP bridge by spawning the Node.js process.
    pub async fn connect(bridge_path: &str) -> Result<Self, QuillError> {
        if !std::path::Path::new(bridge_path).exists() {
            return Err(QuillError::BridgeNotFound(bridge_path.to_string()));
        }
        let node_path = crate::util::resolve_node_binary()
            .ok_or(QuillError::NodeNotFound)?;

        let transport = TokioChildProcess::new(
            tokio::process::Command::new(node_path).arg(bridge_path),
        )
        .map_err(|e| QuillError::SpawnFailed(e.to_string()))?;

        let service = ().serve(transport)
            .await
            .map_err(|e| QuillError::ConnectionFailed(e.to_string()))?;

        Ok(Self { service })
    }

    /// Search for meetings matching a time range.
    pub async fn search_meetings(
        &self,
        query: &str,
        after: &str,
        before: &str,
    ) -> Result<Vec<QuillMeeting>, QuillError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "search_meetings".into(),
                arguments: serde_json::json!({
                    "query": query,
                    "after": after,
                    "before": before,
                })
                .as_object()
                .cloned(),
            })
            .await
            .map_err(|e| QuillError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            let msg = result
                .content
                .first()
                .and_then(|c| c.as_text())
                .map(|t| t.text.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(QuillError::ToolCallFailed(msg));
        }

        let text: String = result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect();

        serde_json::from_str(&text)
            .map_err(|e| QuillError::ParseError(e.to_string()))
    }

    /// List recent meetings from Quill by searching a wide time window.
    pub async fn list_meetings(&self) -> Result<Vec<QuillMeeting>, QuillError> {
        let now = chrono::Utc::now();
        let after = (now - chrono::Duration::days(7)).to_rfc3339();
        let before = now.to_rfc3339();
        self.search_meetings("", &after, &before).await
    }

    /// Fetch the transcript for a specific meeting.
    pub async fn get_transcript(&self, meeting_id: &str) -> Result<String, QuillError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "get_transcript".into(),
                arguments: serde_json::json!({ "id": meeting_id })
                    .as_object()
                    .cloned(),
            })
            .await
            .map_err(|e| QuillError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            let msg = result
                .content
                .first()
                .and_then(|c| c.as_text())
                .map(|t| t.text.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(QuillError::ToolCallFailed(msg));
        }

        let text: String = result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect();

        if text.is_empty() {
            return Err(QuillError::TranscriptNotAvailable);
        }

        Ok(text)
    }

    /// Disconnect from the Quill bridge, terminating the child process.
    pub async fn disconnect(self) {
        let _ = self.service.cancel().await;
    }

    /// Check whether a bridge script exists on disk.
    pub fn bridge_exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    /// Verify that Node.js is available (checks PATH and common install locations).
    pub fn node_available() -> bool {
        crate::util::resolve_node_binary().is_some()
    }
}
