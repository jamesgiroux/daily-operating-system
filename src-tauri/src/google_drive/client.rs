//! Google Drive API client for file operations and change tracking.

use crate::google_api::get_valid_access_token;

/// Represents a file in Google Drive.
#[derive(Debug, Clone)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub web_view_link: Option<String>,
}

/// Represents a change in Google Drive.
#[derive(Debug, Clone)]
pub struct DriveChange {
    pub file_id: String,
    pub removed: bool,
    pub file: Option<DriveFile>,
}

/// Get changes from Google Drive since the given page token.
pub async fn get_changes(page_token: &str) -> Result<(Vec<DriveChange>, String), String> {
    let token = get_valid_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

    let url = if page_token.is_empty() {
        "https://www.googleapis.com/drive/v3/changes?pageSize=100&fields=*".to_string()
    } else {
        format!(
            "https://www.googleapis.com/drive/v3/changes?pageSize=100&pageToken={}&fields=*",
            page_token
        )
    };

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch changes: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Drive API error: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse changes response: {}", e))?;

    let changes = body
        .get("changes")
        .and_then(|c| c.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|change| {
            let file_id = change.get("fileId")?.as_str()?;
            let removed = change
                .get("removed")
                .and_then(|r| r.as_bool())
                .unwrap_or(false);
            let file = if !removed {
                change.get("file").and_then(|f| {
                    Some(DriveFile {
                        id: f.get("id")?.as_str()?.to_string(),
                        name: f.get("name")?.as_str()?.to_string(),
                        mime_type: f.get("mimeType")?.as_str()?.to_string(),
                        web_view_link: f
                            .get("webViewLink")
                            .and_then(|w| w.as_str())
                            .map(String::from),
                    })
                })
            } else {
                None
            };

            Some(DriveChange {
                file_id: file_id.to_string(),
                removed,
                file,
            })
        })
        .collect();

    // nextPageToken = more pages of changes to fetch
    // newStartPageToken = no more changes; use this token for the next poll
    let next_token = body
        .get("nextPageToken")
        .or_else(|| body.get("newStartPageToken"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    Ok((changes, next_token))
}

/// Get a start page token for the Changes API.
///
/// This token represents "now" — subsequent `get_changes()` calls with this
/// token will return only changes that happen after this point.
pub async fn get_start_page_token() -> Result<String, String> {
    let token = get_valid_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

    let client = reqwest::Client::new();
    let response = client
        .get("https://www.googleapis.com/drive/v3/changes/startPageToken")
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Failed to get start page token: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Drive API error getting start token: {}",
            response.status()
        ));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse start token response: {}", e))?;

    body.get("startPageToken")
        .and_then(|t| t.as_str())
        .map(String::from)
        .ok_or_else(|| "No startPageToken in response".to_string())
}

/// Download a file from Google Drive and convert to markdown.
pub async fn download_file_as_markdown(file_id: &str) -> Result<String, String> {
    let token = get_valid_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

    // Get file metadata to determine how to export
    let metadata_url = format!(
        "https://www.googleapis.com/drive/v3/files/{}?fields=mimeType,name",
        file_id
    );
    let client = reqwest::Client::new();
    let metadata_response = client
        .get(&metadata_url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch file metadata: {}", e))?;

    let metadata: serde_json::Value = metadata_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse metadata: {}", e))?;

    let mime_type = metadata
        .get("mimeType")
        .and_then(|m| m.as_str())
        .unwrap_or("application/octet-stream");

    let name = metadata
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("document");

    // For Google Docs, export as markdown
    if mime_type.contains("document") {
        let export_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}/export?mimeType=text/markdown",
            file_id
        );
        let content = client
            .get(&export_url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Failed to export document: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read export content: {}", e))?;

        Ok(content)
    } else if mime_type.contains("spreadsheet") {
        // For sheets, export as CSV and wrap in markdown code block
        let export_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}/export?mimeType=text/csv",
            file_id
        );
        let content = client
            .get(&export_url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Failed to export sheet: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read export content: {}", e))?;

        Ok(format!("```csv\n{}\n```", content))
    } else if mime_type.contains("presentation") {
        // For slides, export as plain text
        let export_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}/export?mimeType=text/plain",
            file_id
        );
        let content = client
            .get(&export_url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Failed to export presentation: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read export content: {}", e))?;

        Ok(format!("# {}\n\n{}", name, content))
    } else {
        // For files we can't export, just add metadata
        Ok(format!(
            "# {}\n\n*File type: {}*\n\n(Binary or unsupported format)",
            name, mime_type
        ))
    }
}
