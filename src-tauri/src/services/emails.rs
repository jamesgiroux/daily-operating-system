// Emails service — extracted from commands.rs
// Business logic for email enrichment and retrieval.

use std::collections::{HashMap, HashSet};

use crate::state::AppState;
use crate::types::{
    EmailBriefingData, EmailBriefingStats, EmailSignal, EnrichedEmail, EntityEmailThread,
};

/// Get enriched email data for the emails page.
///
/// Loads emails from JSON, enriches with DB signals, builds entity threads,
/// and returns the full EmailBriefingData for the frontend.
pub fn get_emails_enriched(state: &AppState) -> Result<EmailBriefingData, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Config lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    let emails = crate::json_loader::load_emails_json(&today_dir).unwrap_or_default();

    // Load email narrative + replies_needed from directive (I355)
    let (email_narrative, replies_needed) = crate::json_loader::load_directive(&today_dir)
        .map(|d| (d.emails.narrative, d.emails.replies_needed))
        .unwrap_or_default();

    // Collect email IDs for batch signal lookup
    let email_ids: Vec<String> = emails.iter().map(|e| e.id.clone()).collect();

    // Batch-query signals from DB
    let db_signals = match state.with_db_try_read(|db| db.list_email_signals_by_email_ids(&email_ids)) {
        crate::state::DbTryRead::Ok(Ok(sigs)) => sigs,
        _ => Vec::new(),
    };

    let has_enrichment = !db_signals.is_empty()
        || emails.iter().any(|e| e.summary.is_some());

    // Index signals by email_id
    let mut signals_by_email: HashMap<String, Vec<EmailSignal>> = HashMap::new();
    for sig in &db_signals {
        signals_by_email
            .entry(sig.email_id.clone())
            .or_default()
            .push(EmailSignal {
                signal_type: sig.signal_type.clone(),
                signal_text: sig.signal_text.clone(),
                confidence: sig.confidence,
                sentiment: sig.sentiment.clone(),
                urgency: sig.urgency.clone(),
                detected_at: Some(sig.detected_at.clone()),
            });
    }

    // Build enriched emails by priority
    let mut high = Vec::new();
    let mut medium = Vec::new();
    let mut low = Vec::new();
    let mut needs_action = 0usize;

    for email in emails {
        let sigs = signals_by_email.remove(&email.id).unwrap_or_default();
        if email.recommended_action.is_some() {
            needs_action += 1;
        }
        let enriched = EnrichedEmail {
            email: email.clone(),
            signals: sigs,
        };
        match email.priority {
            crate::types::EmailPriority::High => high.push(enriched),
            crate::types::EmailPriority::Medium => medium.push(enriched),
            crate::types::EmailPriority::Low => low.push(enriched),
        }
    }

    // Build entity threads from signals
    let mut entity_map: HashMap<String, (String, Vec<EmailSignal>, HashSet<String>)> =
        HashMap::new();
    for sig in &db_signals {
        let entry = entity_map
            .entry(sig.entity_id.clone())
            .or_insert_with(|| (sig.entity_type.clone(), Vec::new(), HashSet::new()));
        entry.1.push(EmailSignal {
            signal_type: sig.signal_type.clone(),
            signal_text: sig.signal_text.clone(),
            confidence: sig.confidence,
            sentiment: sig.sentiment.clone(),
            urgency: sig.urgency.clone(),
            detected_at: Some(sig.detected_at.clone()),
        });
        entry.2.insert(sig.email_id.clone());
    }

    // Resolve entity names from DB
    let entity_threads: Vec<EntityEmailThread> = entity_map
        .into_iter()
        .map(|(entity_id, (entity_type, signals, email_set))| {
            let entity_name: String = {
                let eid = entity_id.clone();
                let etype = entity_type.clone();
                match state.with_db_try_read(|db| -> Result<String, crate::db::DbError> {
                    if &etype == "account" {
                        Ok(db.get_account(&eid)?.map(|a| a.name).unwrap_or_else(|| eid.clone()))
                    } else {
                        Ok(db.get_project(&eid)?.map(|p| p.name).unwrap_or_else(|| eid.clone()))
                    }
                }) {
                    crate::state::DbTryRead::Ok(Ok(name)) => name,
                    _ => entity_id.clone(),
                }
            };

            // Build editorial signal summary as a prose sentence
            let signal_summary =
                crate::services::entities::build_entity_signal_prose(&signals, email_set.len());

            EntityEmailThread {
                entity_id,
                entity_name,
                entity_type,
                email_count: email_set.len(),
                signal_summary,
                signals,
            }
        })
        .collect();

    let total = high.len() + medium.len() + low.len();
    Ok(EmailBriefingData {
        stats: EmailBriefingStats {
            total,
            high_count: high.len(),
            medium_count: medium.len(),
            low_count: low.len(),
            needs_action,
        },
        high_priority: high,
        medium_priority: medium,
        low_priority: low,
        entity_threads,
        has_enrichment,
        email_narrative,
        replies_needed,
    })
}
