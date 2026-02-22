//! I367: Mandatory email enrichment pipeline.
//!
//! Resolves email sender → entity, then runs AI enrichment via PTY
//! to produce contextual_summary, sentiment, and urgency for each email.

use std::path::Path;

use crate::db::emails::EmailEnrichmentUpdate;
use crate::db::types::DbEmail;
use crate::db::ActionDb;
use crate::pty::{ModelTier, PtyManager};
use crate::types::AiModelConfig;

/// Result of enriching a single email.
pub struct EnrichmentResult {
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub contextual_summary: Option<String>,
    pub sentiment: Option<String>,
    pub urgency: Option<String>,
}

/// Convert an `EnrichmentResult` into the DB update struct.
impl EnrichmentResult {
    pub fn as_db_update(&self) -> EmailEnrichmentUpdate<'_> {
        EmailEnrichmentUpdate {
            summary: self.contextual_summary.as_deref(),
            entity_id: self.entity_id.as_deref(),
            entity_type: self.entity_type.as_deref(),
            sentiment: self.sentiment.as_deref(),
            urgency: self.urgency.as_deref(),
        }
    }
}

/// Resolve sender_email → (entity_id, entity_type) via person_emails → people → account_domains.
fn resolve_entity(db: &ActionDb, email: &DbEmail) -> (Option<String>, Option<String>) {
    let sender = match email.sender_email.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => return (None, None),
    };

    // Try person lookup first
    if let Ok(Some(person)) = db.get_person_by_email_or_alias(sender) {
        return (Some(person.id), Some("person".to_string()));
    }

    // Fallback: domain-based account lookup
    let domain = crate::prepare::email_classify::extract_domain(sender);
    if !domain.is_empty() {
        if let Ok(accounts) = db.lookup_account_candidates_by_domain(&domain) {
            if let Some(account) = accounts.first() {
                return (Some(account.id.clone()), Some("account".to_string()));
            }
        }
    }

    (None, None)
}

/// Build the AI enrichment prompt for a single email with relationship context (I369).
fn build_enrichment_prompt(
    db: &ActionDb,
    email: &DbEmail,
    entity_id: Option<&str>,
    entity_type: Option<&str>,
) -> String {
    let sender = email.sender_email.as_deref().unwrap_or("unknown");
    let sender_name = email.sender_name.as_deref().unwrap_or("");
    let subject = email.subject.as_deref().unwrap_or("(no subject)");
    let snippet = email.snippet.as_deref().unwrap_or("");

    // I369: Gather relationship context for the resolved entity
    let relationship_context = build_relationship_context(db, entity_id, entity_type);

    let mut prompt = format!(
        "You are a chief of staff reading an email for your executive. \
         Analyze this email and connect it to what you know about the relationship.\n\n\
         From: {} {}\n\
         Subject: {}\n\
         Preview: {}\n",
        sender, sender_name, subject, snippet
    );

    if !relationship_context.is_empty() {
        prompt.push_str("\n--- Relationship Context ---\n");
        prompt.push_str(&relationship_context);
        prompt.push('\n');
    }

    prompt.push_str(
        "\nReturn ONLY a JSON object with these fields:\n\
         - contextual_summary: string (1-2 sentence chief-of-staff analysis connecting this email to what's known about the relationship. Reference specific meetings or signals when relevant.)\n\
         - sentiment: \"positive\" | \"neutral\" | \"negative\" | \"mixed\"\n\
         - urgency: \"high\" | \"medium\" | \"low\"\n\n\
         Do not include any text outside the JSON object.",
    );

    prompt
}

/// Build relationship context string from entity intelligence, meetings, and signals (I369).
fn build_relationship_context(
    db: &ActionDb,
    entity_id: Option<&str>,
    entity_type: Option<&str>,
) -> String {
    let (eid, etype) = match (entity_id, entity_type) {
        (Some(id), Some(t)) => (id, t),
        _ => return String::new(),
    };

    let mut sections = Vec::new();

    // 1. Entity intelligence (executive_assessment from entity_intel table)
    if let Ok(Some(intel)) = db.get_entity_intelligence(eid) {
        if let Some(ref assessment) = intel.executive_assessment {
            if !assessment.is_empty() {
                sections.push(format!("Executive assessment: {}", assessment));
            }
        }
    }

    // 2. Recent meeting history (last 30 days, up to 5)
    let meetings = match etype {
        "account" => db.get_meetings_for_account(eid, 5).unwrap_or_default(),
        "person" => db.get_person_meetings(eid, 5).unwrap_or_default(),
        "project" => db.get_meetings_for_project(eid, 5).unwrap_or_default(),
        _ => Vec::new(),
    };
    if !meetings.is_empty() {
        let meeting_lines: Vec<String> = meetings
            .iter()
            .take(5)
            .map(|m| {
                format!(
                    "- {} | {} | {}",
                    m.start_time,
                    m.title,
                    m.summary.as_deref().unwrap_or("no summary")
                )
            })
            .collect();
        sections.push(format!("Recent meetings:\n{}", meeting_lines.join("\n")));
    }

    // 3. Active signals for entity
    if let Ok(signals) = crate::signals::bus::get_active_signals(db, etype, eid) {
        let signal_lines: Vec<String> = signals
            .iter()
            .take(10)
            .map(|s| {
                let val = s.value.as_deref().unwrap_or("");
                if val.is_empty() {
                    format!("- {} (confidence: {:.1})", s.signal_type, s.confidence)
                } else {
                    format!("- {}: {} (confidence: {:.1})", s.signal_type, val, s.confidence)
                }
            })
            .collect();
        if !signal_lines.is_empty() {
            sections.push(format!("Active signals:\n{}", signal_lines.join("\n")));
        }
    }

    sections.join("\n\n")
}

/// Parse AI enrichment response, extracting JSON fields.
///
/// Tolerates surrounding text by finding the first `{` and last `}`.
fn parse_enrichment_response(output: &str) -> (Option<String>, Option<String>, Option<String>) {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return (None, None, None);
    }

    let start = match trimmed.find('{') {
        Some(i) => i,
        None => return (None, None, None),
    };
    let end = match trimmed.rfind('}') {
        Some(i) => i,
        None => return (None, None, None),
    };
    if end <= start {
        return (None, None, None);
    }

    let json_str = &trimmed[start..=end];
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            log::debug!("email_enrich: JSON parse failed: {e}");
            return (None, None, None);
        }
    };

    let summary = parsed
        .get("contextual_summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let sentiment = parsed
        .get("sentiment")
        .and_then(|v| v.as_str())
        .filter(|s| matches!(*s, "positive" | "neutral" | "negative" | "mixed"))
        .map(|s| s.to_string());
    let urgency = parsed
        .get("urgency")
        .and_then(|v| v.as_str())
        .filter(|s| matches!(*s, "high" | "medium" | "low"))
        .map(|s| s.to_string());

    (summary, sentiment, urgency)
}

/// Collect pending emails from DB (short lock), enrich via AI (no lock),
/// then persist results (short lock per email).
///
/// This two-phase approach avoids holding the DB mutex during PTY calls
/// which can take 60s each.
pub fn enrich_pending_emails_two_phase(
    state: &crate::state::AppState,
    workspace: &Path,
    ai_config: &AiModelConfig,
    limit: usize,
) -> usize {
    // Phase 1: Get pending emails + resolve entities (short DB lock)
    let pending: Vec<(DbEmail, Option<String>, Option<String>)> = {
        let guard = state.db.lock().ok();
        let db = match guard.as_ref().and_then(|g| g.as_ref()) {
            Some(db) => db,
            None => return 0,
        };
        let emails = match db.get_pending_enrichment(limit) {
            Ok(e) => e,
            Err(e) => {
                log::warn!("email_enrich: failed to get pending emails: {e}");
                return 0;
            }
        };
        emails
            .into_iter()
            .map(|email| {
                let (eid, etype) = resolve_entity(db, &email);
                (email, eid, etype)
            })
            .collect()
    }; // DB lock released here

    if pending.is_empty() {
        return 0;
    }

    log::info!("email_enrich: {} emails pending enrichment", pending.len());
    let mut enriched_count = 0usize;

    // Phase 2: AI enrichment via PTY (no DB lock held)
    for (email, entity_id, entity_type) in &pending {
        // Build context prompt — needs DB for relationship context
        let prompt = {
            let guard = state.db.lock().ok();
            let db = match guard.as_ref().and_then(|g| g.as_ref()) {
                Some(db) => db,
                None => continue,
            };
            build_enrichment_prompt(db, email, entity_id.as_deref(), entity_type.as_deref())
        }; // DB lock released before PTY call

        let pty = PtyManager::for_tier(ModelTier::Extraction, ai_config)
            .with_timeout(60)
            .with_nice_priority(10);

        let ai_result = match pty.spawn_claude(workspace, &prompt) {
            Ok(output) => {
                let (summary, sentiment, urgency) = parse_enrichment_response(&output.stdout);
                Ok(EnrichmentResult {
                    entity_id: entity_id.clone(),
                    entity_type: entity_type.clone(),
                    contextual_summary: summary,
                    sentiment,
                    urgency,
                })
            }
            Err(e) => Err(format!("AI enrichment failed for {}: {e}", email.email_id)),
        };

        // Phase 3: Persist result (short DB lock per email)
        let guard = state.db.lock().ok();
        if let Some(db) = guard.as_ref().and_then(|g| g.as_ref()) {
            match ai_result {
                Ok(result) => {
                    let update = result.as_db_update();
                    if let Err(e) =
                        db.set_enrichment_state(&email.email_id, "enriched", update)
                    {
                        log::warn!(
                            "email_enrich: failed to persist enrichment for {}: {e}",
                            email.email_id
                        );
                    } else {
                        enriched_count += 1;
                    }
                }
                Err(e) => {
                    log::warn!("email_enrich: {e}");
                    let empty = EmailEnrichmentUpdate {
                        summary: None,
                        entity_id: None,
                        entity_type: None,
                        sentiment: None,
                        urgency: None,
                    };
                    let _ = db.set_enrichment_state(&email.email_id, "failed", empty);
                }
            }
        }
    }

    if enriched_count > 0 {
        log::info!(
            "email_enrich: enriched {}/{} emails",
            enriched_count,
            pending.len()
        );
    }
    enriched_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enrichment_clean_json() {
        let output = r#"{"contextual_summary":"Important renewal discussion","sentiment":"positive","urgency":"high"}"#;
        let (summary, sentiment, urgency) = parse_enrichment_response(output);
        assert_eq!(summary.as_deref(), Some("Important renewal discussion"));
        assert_eq!(sentiment.as_deref(), Some("positive"));
        assert_eq!(urgency.as_deref(), Some("high"));
    }

    #[test]
    fn test_parse_enrichment_wrapped_output() {
        let output = "Here is my analysis:\n{\"contextual_summary\":\"FYI email\",\"sentiment\":\"neutral\",\"urgency\":\"low\"}\nDone.";
        let (summary, sentiment, urgency) = parse_enrichment_response(output);
        assert_eq!(summary.as_deref(), Some("FYI email"));
        assert_eq!(sentiment.as_deref(), Some("neutral"));
        assert_eq!(urgency.as_deref(), Some("low"));
    }

    #[test]
    fn test_parse_enrichment_invalid_sentiment() {
        let output = r#"{"contextual_summary":"Test","sentiment":"angry","urgency":"high"}"#;
        let (summary, sentiment, urgency) = parse_enrichment_response(output);
        assert_eq!(summary.as_deref(), Some("Test"));
        assert!(sentiment.is_none()); // "angry" is not a valid sentiment
        assert_eq!(urgency.as_deref(), Some("high"));
    }

    #[test]
    fn test_parse_enrichment_empty() {
        let (s, se, u) = parse_enrichment_response("");
        assert!(s.is_none());
        assert!(se.is_none());
        assert!(u.is_none());
    }

    #[test]
    fn test_parse_enrichment_no_json() {
        let (s, se, u) = parse_enrichment_response("No JSON here");
        assert!(s.is_none());
        assert!(se.is_none());
        assert!(u.is_none());
    }
}
