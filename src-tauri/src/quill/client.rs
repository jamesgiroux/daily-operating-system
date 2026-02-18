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

        if text.is_empty() {
            return Ok(vec![]);
        }

        // Quill returns XML, not JSON. Try JSON first for forward-compat,
        // then fall back to XML parsing.
        if text.trim_start().starts_with('[') || text.trim_start().starts_with('{') {
            return serde_json::from_str(&text)
                .map_err(|e| QuillError::ParseError(e.to_string()));
        }

        // Parse Quill's XML response:
        // <ToolResponse><results count="N">
        //   <meeting id="..." date="..." participants="..." ...>
        //     <title>...</title><blurb>...</blurb>
        //   </meeting>
        // </results></ToolResponse>
        parse_meetings_xml(&text)
    }

    /// List recent meetings from Quill by searching a wide time window.
    pub async fn list_meetings(&self) -> Result<Vec<QuillMeeting>, QuillError> {
        let now = chrono::Utc::now();
        let after = (now - chrono::Duration::days(7)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let before = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
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

/// Parse Quill's XML response format into QuillMeeting structs.
///
/// Format: `<meeting id="..." date="..." participants="..."><title>...</title></meeting>`
fn parse_meetings_xml(xml: &str) -> Result<Vec<QuillMeeting>, QuillError> {
    let mut meetings = Vec::new();

    // Find each <meeting ...>...</meeting> block
    let mut search_from = 0;
    while let Some(start) = xml[search_from..].find("<meeting ") {
        let abs_start = search_from + start;
        let tag_end = match xml[abs_start..].find('>') {
            Some(pos) => abs_start + pos,
            None => break,
        };

        let attrs = &xml[abs_start..=tag_end];

        let id = extract_attr(attrs, "id").unwrap_or_default();
        let date = extract_attr(attrs, "date");
        let participants_raw = extract_attr(attrs, "participants").unwrap_or_default();

        // Parse participants: "me, Alice, Bob" or "me, and other speakers"
        let participants: Vec<QuillParticipant> = participants_raw
            .split(',')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty() && *p != "me")
            .map(|p| {
                let clean = p.trim_start_matches("and ").trim();
                QuillParticipant {
                    name: Some(clean.to_string()),
                    email: None,
                }
            })
            .collect();

        // Extract <title>...</title>
        let block_end = xml[abs_start..]
            .find("</meeting>")
            .map(|p| abs_start + p)
            .unwrap_or(xml.len());
        let block = &xml[abs_start..block_end];

        let title = extract_element(block, "title").unwrap_or_default();

        meetings.push(QuillMeeting {
            id,
            title,
            start_time: date,
            end_time: None,
            participants,
            has_transcript: true,
        });

        search_from = block_end;
    }

    Ok(meetings)
}

/// Extract an XML attribute value: `attr="value"` → `value`
fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

/// Extract text content of an XML element: `<tag>content</tag>` → `content`
fn extract_element(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].to_string())
}
