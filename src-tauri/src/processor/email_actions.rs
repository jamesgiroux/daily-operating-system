//! I321: Email commitment and action extraction from email bodies.
//!
//! Runs high-priority email bodies through Claude (via PTY, Extraction tier)
//! to extract commitments, requests, and deadlines. Creates proposed actions.

use std::path::Path;

use crate::db::ActionDb;
use crate::pty::{ModelTier, PtyManager};
use crate::types::AiModelConfig;

/// A commitment extracted from an email body.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailCommitment {
    pub title: String,
    pub commitment_type: String, // "commitment", "request", "deadline"
    pub due_date: Option<String>,
    pub owner: Option<String>,
}

/// Extract commitments and action items from an email body.
///
/// Runs the body through Claude (Extraction tier) to identify commitments,
/// requests, and deadlines. Each is persisted as a proposed action.
/// The email body is NOT stored — only the extracted structured data persists.
pub fn extract_email_commitments(
    workspace: &Path,
    ai_config: &AiModelConfig,
    body: &str,
    email_id: &str,
    subject: &str,
    from_email: &str,
    db: &ActionDb,
) -> Vec<EmailCommitment> {
    // Truncate body to prevent excessive processing (max 4000 chars)
    let truncated_body = if body.len() > 4000 {
        &body[..4000]
    } else {
        body
    };

    // Build extraction prompt
    let prompt = format!(
        "Extract commitments, action requests, and deadlines from this email.\n\
         From: {from_email}\nSubject: {subject}\n\n\
         Body:\n{truncated_body}\n\n\
         Return ONLY a JSON array of objects with these fields:\n\
         - title: string (concise action description)\n\
         - commitment_type: \"commitment\" | \"request\" | \"deadline\"\n\
         - due_date: ISO 8601 date string or null\n\
         - owner: person name or null\n\n\
         Only include concrete, actionable items. Return [] if none found.\n\
         Do not include any text outside the JSON array."
    );

    // Run through Claude via PTY (Extraction tier, 60s timeout)
    let pty = PtyManager::for_tier(ModelTier::Extraction, ai_config)
        .with_timeout(60)
        .with_nice_priority(10);

    let output = match pty.spawn_claude(workspace, &prompt) {
        Ok(o) => o.stdout,
        Err(e) => {
            log::warn!(
                "email_actions: Claude extraction failed for email {}: {}",
                email_id,
                e
            );
            return Vec::new();
        }
    };

    // Parse response — extract JSON array from output
    let commitments = match parse_commitments_from_output(&output) {
        Some(c) => c,
        None => {
            if !output.trim().is_empty() {
                log::debug!(
                    "email_actions: no parseable commitments from email {} (output: {} bytes)",
                    email_id,
                    output.len()
                );
            }
            return Vec::new();
        }
    };

    // Persist each commitment as a proposed action
    let now = chrono::Utc::now().to_rfc3339();
    for (i, commitment) in commitments.iter().enumerate() {
        let action = crate::db::DbAction {
            id: format!("email-{}-{}", email_id, i),
            title: commitment.title.clone(),
            priority: "P2".to_string(),
            status: "proposed".to_string(),
            created_at: now.clone(),
            due_date: commitment.due_date.clone(),
            completed_at: None,
            account_id: None,
            project_id: None,
            source_type: Some("email".to_string()),
            source_id: Some(email_id.to_string()),
            source_label: Some(subject.to_string()),
            context: Some(format!(
                "From: {} — {}",
                from_email, commitment.commitment_type
            )),
            waiting_on: None,
            updated_at: now.clone(),
            person_id: None,
            account_name: None,
        };

        if let Err(e) = db.upsert_action_if_not_completed(&action) {
            log::warn!(
                "Failed to persist email commitment from {}: {}",
                email_id,
                e
            );
        }
    }

    if !commitments.is_empty() {
        log::info!(
            "extract_email_commitments: {} commitments from email {} ({})",
            commitments.len(),
            email_id,
            subject
        );
    }

    commitments
}

/// Parse commitment JSON from AI output text.
///
/// Tolerates surrounding text by finding the first `[` and last `]` in the output.
fn parse_commitments_from_output(output: &str) -> Option<Vec<EmailCommitment>> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Find the JSON array boundaries
    let start = trimmed.find('[')?;
    let end = trimmed.rfind(']')?;
    if end <= start {
        return None;
    }

    let json_str = &trimmed[start..=end];
    match serde_json::from_str::<Vec<EmailCommitment>>(json_str) {
        Ok(commitments) if !commitments.is_empty() => Some(commitments),
        Ok(_) => None, // Empty array
        Err(e) => {
            log::debug!("email_actions: JSON parse failed: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_commitment_serialization() {
        let commitment = EmailCommitment {
            title: "Send SOW by Friday".to_string(),
            commitment_type: "commitment".to_string(),
            due_date: Some("2026-02-21".to_string()),
            owner: Some("Alice".to_string()),
        };
        let json = serde_json::to_string(&commitment).unwrap();
        assert!(json.contains("Send SOW by Friday"));
        assert!(json.contains("commitment"));
    }

    #[test]
    fn test_parse_commitments_from_clean_json() {
        let output = r#"[{"title":"Send SOW","commitmentType":"commitment","dueDate":"2026-02-21","owner":"Alice"}]"#;
        let result = parse_commitments_from_output(output);
        assert!(result.is_some());
        let commitments = result.unwrap();
        assert_eq!(commitments.len(), 1);
        assert_eq!(commitments[0].title, "Send SOW");
    }

    #[test]
    fn test_parse_commitments_from_wrapped_output() {
        let output = "Here are the commitments:\n[{\"title\":\"Follow up\",\"commitmentType\":\"request\",\"dueDate\":null,\"owner\":null}]\nDone.";
        let result = parse_commitments_from_output(output);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_commitments_empty_array() {
        let output = "[]";
        let result = parse_commitments_from_output(output);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_commitments_no_json() {
        let result = parse_commitments_from_output("No commitments found.");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_commitments_empty_input() {
        let result = parse_commitments_from_output("");
        assert!(result.is_none());
    }
}
