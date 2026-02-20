//! Gmail API v1 — fetch unread emails.
//!
//! Replaces ops/email_fetch.py:_fetch_unread_emails().
//! Fetches message list (is:unread newer_than:1d), then metadata
//! for each message (From, Subject, Date, List-Unsubscribe, Precedence).

use serde::Deserialize;

use super::{send_with_retry, GoogleApiError, RetryPolicy};

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
    /// Optional email body text (only fetched when emailBodyAccess is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
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
    let resp = send_with_retry(
        client
            .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
            .bearer_auth(access_token)
            .query(&[
                ("q", "is:unread newer_than:1d"),
                ("maxResults", &max_results.to_string()),
            ]),
        &RetryPolicy::default(),
    )
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

    let resp = send_with_retry(
        client.get(&url).bearer_auth(access_token).query(&[
            ("format", "metadata"),
            ("metadataHeaders", "From"),
            ("metadataHeaders", "Subject"),
            ("metadataHeaders", "Date"),
            ("metadataHeaders", "List-Unsubscribe"),
            ("metadataHeaders", "Precedence"),
        ]),
        &RetryPolicy::default(),
    )
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
        body: None,
    })
}

// ============================================================================
// Full message body fetch (I321)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FullMessageDetail {
    #[serde(default)]
    payload: Option<FullPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FullPayload {
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    body: Option<PayloadBody>,
    #[serde(default)]
    parts: Vec<FullPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayloadBody {
    #[serde(default)]
    data: Option<String>,
}

/// Fetch the full message body for a single email.
///
/// Requests `format=full`, walks MIME parts to find `text/plain` (preferred)
/// or `text/html`, and decodes the URL-safe base64 body data.
/// Returns `Ok(None)` if no text body is found (e.g., attachment-only messages).
pub async fn fetch_message_body(
    access_token: &str,
    message_id: &str,
) -> Result<Option<String>, GoogleApiError> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
        message_id
    );

    let resp = send_with_retry(
        client
            .get(&url)
            .bearer_auth(access_token)
            .query(&[("format", "full")]),
        &RetryPolicy::default(),
    )
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

    let detail: FullMessageDetail = resp.json().await?;

    let payload = match detail.payload {
        Some(p) => p,
        None => return Ok(None),
    };

    // Try text/plain first, then text/html
    if let Some(text) = extract_body_text(&payload, "text/plain") {
        return Ok(Some(text));
    }
    if let Some(text) = extract_body_text(&payload, "text/html") {
        return Ok(Some(text));
    }

    Ok(None)
}

/// Recursively walk MIME parts to find body data matching the target MIME type.
fn extract_body_text(payload: &FullPayload, target_mime: &str) -> Option<String> {
    // Check this node
    if payload.mime_type == target_mime {
        if let Some(ref body) = payload.body {
            if let Some(ref data) = body.data {
                return decode_url_safe_base64(data);
            }
        }
    }
    // Recurse into child parts
    for part in &payload.parts {
        if let Some(text) = extract_body_text(part, target_mime) {
            return Some(text);
        }
    }
    None
}

/// Decode URL-safe base64 (no padding) as used by Gmail API.
fn decode_url_safe_base64(data: &str) -> Option<String> {
    use base64::Engine;
    match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data) {
        Ok(bytes) => String::from_utf8(bytes).ok(),
        Err(_) => None,
    }
}

// ============================================================================
// Archive (remove INBOX label)
// ============================================================================

/// Archive emails by removing the INBOX label via Gmail batchModify.
///
/// This is a soft operation — emails remain in All Mail and are fully searchable.
/// Gmail's batchModify accepts up to 1000 IDs per call.
pub async fn archive_emails(
    access_token: &str,
    message_ids: &[String],
) -> Result<usize, GoogleApiError> {
    if message_ids.is_empty() {
        return Ok(0);
    }

    let client = reqwest::Client::new();

    // Gmail batchModify supports up to 1000 IDs per request
    for chunk in message_ids.chunks(1000) {
        let body = serde_json::json!({
            "ids": chunk,
            "removeLabelIds": ["INBOX"]
        });

        let resp = send_with_retry(
            client
                .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/batchModify")
                .bearer_auth(access_token)
                .json(&body),
            &RetryPolicy::default(),
        )
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
    }

    Ok(message_ids.len())
}

// ============================================================================
// Frequent correspondents (onboarding teammate suggestions)
// ============================================================================

/// A frequent email correspondent within the user's domain.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequentCorrespondent {
    pub name: String,
    pub email: String,
    pub message_count: usize,
}

/// Fetch the user's most frequent same-domain correspondents from sent mail.
///
/// Scans `in:sent newer_than:90d` messages, extracts To/Cc headers,
/// filters to the user's domain, and returns the top N by frequency.
pub async fn fetch_frequent_correspondents(
    access_token: &str,
    user_email: &str,
    limit: usize,
) -> Result<Vec<FrequentCorrespondent>, GoogleApiError> {
    let user_domain = user_email
        .split('@')
        .nth(1)
        .unwrap_or("")
        .to_lowercase();
    let user_email_lower = user_email.to_lowercase();

    let client = reqwest::Client::new();
    let mut counts: std::collections::HashMap<String, (String, usize)> =
        std::collections::HashMap::new();
    let mut page_token: Option<String> = None;

    for _ in 0..3 {
        let mut query_params: Vec<(&str, String)> = vec![
            ("q", "in:sent newer_than:90d".to_string()),
            ("maxResults", "100".to_string()),
        ];
        if let Some(ref token) = page_token {
            query_params.push(("pageToken", token.clone()));
        }

        let resp = send_with_retry(
            client
                .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
                .bearer_auth(access_token)
                .query(&query_params),
            &RetryPolicy::default(),
        )
        .await?;

        if !resp.status().is_success() {
            break;
        }
        let list: MessageListResponse = resp.json().await?;

        // Fetch metadata for each message
        for msg in &list.messages {
            let detail_url = format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
                msg.id
            );
            let detail_resp = match send_with_retry(
                client
                    .get(&detail_url)
                    .bearer_auth(access_token)
                    .query(&[
                        ("format", "metadata"),
                        ("metadataHeaders", "To"),
                        ("metadataHeaders", "Cc"),
                    ]),
                &RetryPolicy::default(),
            )
            .await
            {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let detail: MessageDetail = match detail_resp.json().await {
                Ok(d) => d,
                Err(_) => continue,
            };

            if let Some(payload) = &detail.payload {
                for header in &payload.headers {
                    if header.name.eq_ignore_ascii_case("To")
                        || header.name.eq_ignore_ascii_case("Cc")
                    {
                        for addr in parse_email_addresses(&header.value) {
                            let email_lower = addr.1.to_lowercase();
                            if email_lower == user_email_lower {
                                continue;
                            }
                            if let Some(domain) = email_lower.split('@').nth(1) {
                                if domain == user_domain {
                                    let entry = counts
                                        .entry(email_lower.clone())
                                        .or_insert_with(|| (addr.0.clone(), 0));
                                    entry.1 += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        page_token = list.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    let mut results: Vec<FrequentCorrespondent> = counts
        .into_iter()
        .map(|(email, (name, count))| FrequentCorrespondent {
            name: if name.is_empty() {
                email.clone()
            } else {
                name
            },
            email,
            message_count: count,
        })
        .collect();
    results.sort_by(|a, b| b.message_count.cmp(&a.message_count));
    results.truncate(limit);

    Ok(results)
}

/// Parse email addresses from a header value like `"Alice" <alice@co.com>, Bob <bob@co.com>`.
fn parse_email_addresses(header: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for part in header.split(',') {
        let trimmed = part.trim();
        if let Some(lt) = trimmed.find('<') {
            if let Some(gt) = trimmed.find('>') {
                let email = trimmed[lt + 1..gt].trim().to_string();
                let name = trimmed[..lt].trim().trim_matches('"').trim().to_string();
                results.push((name, email));
            }
        } else if trimmed.contains('@') {
            results.push((String::new(), trimmed.to_string()));
        }
    }
    results
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
            body: None,
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
