//! Gmail API v1 — fetch unread emails.
//!
//! Replaces ops/email_fetch.py:_fetch_unread_emails().
//! Fetches message list (is:unread newer_than:1d), then metadata
//! for each message (From, Subject, Date, List-Unsubscribe, Precedence).

use serde::Deserialize;

use super::GoogleApiError;

// ============================================================================
// API response types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageListResponse {
    #[serde(default)]
    messages: Vec<MessageStub>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageStub {
    id: String,
    #[serde(default)]
    thread_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageDetail {
    #[serde(default)]
    id: String,
    #[serde(default)]
    thread_id: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    payload: Option<MessagePayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagePayload {
    #[serde(default)]
    headers: Vec<Header>,
}

#[derive(Debug, Deserialize)]
struct Header {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
}

// ============================================================================
// Public types
// ============================================================================

/// A raw email with metadata extracted from Gmail API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RawEmail {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub subject: String,
    pub snippet: String,
    pub date: String,
    pub list_unsubscribe: String,
    pub precedence: String,
}

// ============================================================================
// Gmail API
// ============================================================================

/// Fetch unread emails from the last 24 hours.
///
/// Mirrors Python's `_fetch_unread_emails()`: lists messages matching
/// "is:unread newer_than:1d", then fetches metadata headers for each.
/// Individual message fetch failures are silently skipped.
pub async fn fetch_unread_emails(
    access_token: &str,
    max_results: u32,
) -> Result<Vec<RawEmail>, GoogleApiError> {
    let client = reqwest::Client::new();

    // Step 1: List unread messages
    let resp = client
        .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
        .bearer_auth(access_token)
        .query(&[
            ("q", "is:unread newer_than:1d"),
            ("maxResults", &max_results.to_string()),
        ])
        .send()
        .await?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(GoogleApiError::AuthExpired);
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(GoogleApiError::ApiError {
            status: status.as_u16(),
            message: body,
        });
    }

    let list: MessageListResponse = resp.json().await?;

    if list.messages.is_empty() {
        return Ok(Vec::new());
    }

    // Step 2: Fetch metadata for each message
    let mut emails = Vec::with_capacity(list.messages.len());

    for stub in &list.messages {
        match fetch_message_metadata(&client, access_token, &stub.id).await {
            Ok(email) => emails.push(email),
            Err(e) => {
                log::debug!("Skipping message {}: {}", stub.id, e);
                continue;
            }
        }
    }

    Ok(emails)
}

/// Fetch metadata headers for a single message.
async fn fetch_message_metadata(
    client: &reqwest::Client,
    access_token: &str,
    message_id: &str,
) -> Result<RawEmail, GoogleApiError> {
    let url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
        message_id
    );

    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .query(&[
            ("format", "metadata"),
            ("metadataHeaders", "From"),
            ("metadataHeaders", "Subject"),
            ("metadataHeaders", "Date"),
            ("metadataHeaders", "List-Unsubscribe"),
            ("metadataHeaders", "Precedence"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(GoogleApiError::ApiError {
            status: 0,
            message: body,
        });
    }

    let detail: MessageDetail = resp.json().await?;

    let headers = detail
        .payload
        .as_ref()
        .map(|p| &p.headers[..])
        .unwrap_or(&[]);

    let get_header = |name: &str| -> String {
        headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.clone())
            .unwrap_or_default()
    };

    Ok(RawEmail {
        id: detail.id,
        thread_id: detail.thread_id,
        from: get_header("From"),
        subject: get_header("Subject"),
        snippet: detail.snippet,
        date: get_header("Date"),
        list_unsubscribe: get_header("List-Unsubscribe"),
        precedence: get_header("Precedence"),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_list_deserialization() {
        let json = r#"{
            "messages": [
                {"id": "msg1", "threadId": "thread1"},
                {"id": "msg2", "threadId": "thread2"}
            ],
            "nextPageToken": "token123"
        }"#;

        let resp: MessageListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.messages.len(), 2);
        assert_eq!(resp.messages[0].id, "msg1");
        assert_eq!(resp.messages[0].thread_id, "thread1");
        assert_eq!(resp.next_page_token.as_deref(), Some("token123"));
    }

    #[test]
    fn test_message_list_empty() {
        let json = r#"{"resultSizeEstimate": 0}"#;
        let resp: MessageListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.messages.is_empty());
        assert!(resp.next_page_token.is_none());
    }

    #[test]
    fn test_message_detail_deserialization() {
        let json = r#"{
            "id": "msg123",
            "threadId": "thread456",
            "snippet": "Hey, just checking in...",
            "payload": {
                "headers": [
                    {"name": "From", "value": "Jane Doe <jane@customer.com>"},
                    {"name": "Subject", "value": "Re: Project Update"},
                    {"name": "Date", "value": "Sat, 8 Feb 2026 09:30:00 -0500"},
                    {"name": "List-Unsubscribe", "value": ""},
                    {"name": "Precedence", "value": ""}
                ]
            }
        }"#;

        let detail: MessageDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.id, "msg123");
        assert_eq!(detail.thread_id, "thread456");
        assert_eq!(detail.snippet, "Hey, just checking in...");

        let headers = detail.payload.unwrap().headers;
        assert_eq!(headers.len(), 5);

        let from = headers.iter().find(|h| h.name == "From").unwrap();
        assert_eq!(from.value, "Jane Doe <jane@customer.com>");
    }

    #[test]
    fn test_message_detail_no_payload() {
        let json = r#"{"id": "msg789", "threadId": "t1", "snippet": ""}"#;
        let detail: MessageDetail = serde_json::from_str(json).unwrap();
        assert!(detail.payload.is_none());
    }

    #[test]
    fn test_raw_email_serialization() {
        let email = RawEmail {
            id: "msg1".to_string(),
            thread_id: "t1".to_string(),
            from: "Jane <jane@example.com>".to_string(),
            subject: "Hello".to_string(),
            snippet: "Hi there".to_string(),
            date: "2026-02-08".to_string(),
            list_unsubscribe: "".to_string(),
            precedence: "".to_string(),
        };

        let json = serde_json::to_value(&email).unwrap();
        assert_eq!(json["from"], "Jane <jane@example.com>");
        assert_eq!(json["subject"], "Hello");
    }

    #[test]
    fn test_newsletter_headers() {
        let json = r#"{
            "id": "newsletter1",
            "threadId": "t2",
            "snippet": "This week in tech...",
            "payload": {
                "headers": [
                    {"name": "From", "value": "noreply@newsletter.example.com"},
                    {"name": "Subject", "value": "Weekly Digest"},
                    {"name": "Date", "value": "Sat, 8 Feb 2026 06:00:00 +0000"},
                    {"name": "List-Unsubscribe", "value": "<https://example.com/unsub>"},
                    {"name": "Precedence", "value": "bulk"}
                ]
            }
        }"#;

        let detail: MessageDetail = serde_json::from_str(json).unwrap();
        let headers = detail.payload.unwrap().headers;

        let unsub = headers
            .iter()
            .find(|h| h.name == "List-Unsubscribe")
            .unwrap();
        assert!(!unsub.value.is_empty());

        let prec = headers.iter().find(|h| h.name == "Precedence").unwrap();
        assert_eq!(prec.value, "bulk");
    }
}
