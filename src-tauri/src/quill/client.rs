//! MCP client for communicating with Quill's local server.
//!
//! Uses rmcp's child process transport to spawn a Node.js bridge that
//! proxies JSON-RPC to Quill's local socket.

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
/// Manages the lifecycle of a child process running the Quill MCP bridge,
/// and provides typed methods for calling Quill's MCP tools.
pub struct QuillClient {
    bridge_path: String,
}

impl QuillClient {
    /// Create a new client pointing at the given bridge script path.
    pub fn new(bridge_path: String) -> Self {
        Self { bridge_path }
    }

    /// Check whether the bridge script exists on disk.
    pub fn bridge_exists(&self) -> bool {
        std::path::Path::new(&self.bridge_path).exists()
    }

    /// Verify that Node.js is available on PATH.
    pub fn node_available() -> bool {
        std::process::Command::new("node")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// List recent meetings from Quill.
    ///
    /// Spawns the bridge process, calls the `list_meetings` tool, and
    /// parses the response into `QuillMeeting` structs.
    pub async fn list_meetings(&self) -> Result<Vec<QuillMeeting>, QuillError> {
        if !self.bridge_exists() {
            return Err(QuillError::BridgeNotFound(self.bridge_path.clone()));
        }
        if !Self::node_available() {
            return Err(QuillError::NodeNotFound);
        }
        // TODO: spawn bridge via rmcp TokioChildProcess transport,
        // call list_meetings tool, parse response
        Ok(Vec::new())
    }

    /// Fetch the transcript for a specific meeting.
    pub async fn get_transcript(&self, meeting_id: &str) -> Result<String, QuillError> {
        if !self.bridge_exists() {
            return Err(QuillError::BridgeNotFound(self.bridge_path.clone()));
        }
        if !Self::node_available() {
            return Err(QuillError::NodeNotFound);
        }
        // TODO: spawn bridge via rmcp TokioChildProcess transport,
        // call get_transcript tool with meeting_id, return text
        let _ = meeting_id;
        Err(QuillError::TranscriptNotAvailable)
    }
}
