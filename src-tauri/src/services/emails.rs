// Emails service — extracted from commands.rs
// Business logic for email enrichment and retrieval.

use std::collections::{HashMap, HashSet};

use crate::state::AppState;
use crate::types::{
    EmailBriefingData, EmailBriefingStats, EmailSignal, EnrichedEmail, EntityEmailThread,
};

/// Get enriched email data for the emails page.
///
/// Tries to load emails from the DB first (I368). If the DB has active emails,
/// uses those. Otherwise falls back to JSON loading for first-run compatibility.
pub fn get_emails_enriched(state: &AppState) -> Result<EmailBriefingData, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Config lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // Try DB first (I368), fall back to JSON if empty
    let db_emails: Vec<crate::db::DbEmail> =
        match state.with_db_try_read(|db| db.get_all_active_emails()) {
            crate::state::DbTryRead::Ok(Ok(rows)) if !rows.is_empty() => rows,
            _ => Vec::new(),
        };

    let emails = if !db_emails.is_empty() {
        // Batch-resolve entity names from IDs
        let entity_ids: HashSet<String> = db_emails
            .iter()
            .filter_map(|e| e.entity_id.clone())
            .collect();
        let entity_names: HashMap<String, String> =
            match state.with_db_try_read(|db| -> HashMap<String, String> {
                let mut map = HashMap::new();
                for eid in &entity_ids {
                    // Look up entity name, and for persons also find linked account
                    if let Ok(Some(p)) = db.get_person(eid) {
                        // Find the linked account most relevant to this email's context.
                        // Person may be linked to many accounts — pick the one whose name
                        // or keywords best match the email subject/summary.
                        let email_context: String = db_emails.iter()
                            .filter(|e| e.entity_id.as_deref() == Some(eid.as_str()))
                            .filter_map(|e| e.contextual_summary.as_deref()
                                .or(e.subject.as_deref()))
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_lowercase();
                        let account_name = best_account_for_person(db, eid, &email_context);
                        let display = account_name.unwrap_or(p.name);
                        map.insert(eid.clone(), display);
                    } else if let Ok(Some(a)) = db.get_account(eid) {
                        map.insert(eid.clone(), a.name);
                    } else if let Ok(Some(p)) = db.get_project(eid) {
                        map.insert(eid.clone(), p.name);
                    }
                }
                map
            }) {
                crate::state::DbTryRead::Ok(names) => names,
                _ => HashMap::new(),
            };

        db_emails
            .iter()
            .map(|dbe| {
                let entity_name = dbe.entity_id.as_ref().and_then(|eid| entity_names.get(eid).cloned());
                crate::types::Email {
                    id: dbe.email_id.clone(),
                    sender: dbe.sender_name.clone().unwrap_or_default(),
                    sender_email: dbe.sender_email.clone().unwrap_or_default(),
                    subject: dbe.subject.clone().unwrap_or_default(),
                    snippet: dbe.snippet.clone(),
                    priority: match dbe.priority.as_deref() {
                        Some("high") => crate::types::EmailPriority::High,
                        Some("low") => crate::types::EmailPriority::Low,
                        _ => crate::types::EmailPriority::Medium,
                    },
                    avatar_url: None,
                    summary: dbe.contextual_summary.clone(),
                    recommended_action: None,
                    conversation_arc: None,
                    email_type: None,
                    commitments: Vec::new(),
                    questions: Vec::new(),
                    sentiment: dbe.sentiment.clone(),
                    urgency: dbe.urgency.clone(),
                    entity_id: dbe.entity_id.clone(),
                    entity_type: dbe.entity_type.clone(),
                    entity_name,
                    relevance_score: dbe.relevance_score,
                    score_reason: dbe.score_reason.clone(),
                }
            })
            .collect()
    } else {
        crate::json_loader::load_emails_json(&today_dir).unwrap_or_default()
    };

    // I395: Sort by relevance score (highest first, nulls last)
    let mut emails = emails;
    emails.sort_by(|a, b| {
        let sa = a.relevance_score.unwrap_or(-1.0);
        let sb = b.relevance_score.unwrap_or(-1.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // I368 AC3: Write emails.json from DB so it stays current even without a Gmail fetch
    if !db_emails.is_empty() {
        let json_path = today_dir.join("data").join("emails.json");
        if let Ok(json) = serde_json::to_string_pretty(&emails) {
            let _ = std::fs::create_dir_all(today_dir.join("data"));
            let _ = std::fs::write(&json_path, json);
        }
    }

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

/// Find the most relevant account linked to a person, given email context.
///
/// When a person is linked to multiple accounts, simple `LIMIT 1` grabs a random one.
/// This function scores each linked account by whether its name or keywords appear
/// in the email's subject/summary context. Falls back to the first account if no match.
pub(crate) fn best_account_for_person(
    db: &crate::db::ActionDb,
    person_id: &str,
    email_context_lower: &str,
) -> Option<String> {
    let mut stmt = match db.conn_ref().prepare(
        "SELECT a.id, a.name, a.keywords FROM accounts a
         JOIN entity_people ep ON a.id = ep.entity_id
         WHERE ep.person_id = ?1"
    ) {
        Ok(s) => s,
        Err(_) => return None,
    };

    let accounts: Vec<(String, String, Option<String>)> = stmt
        .query_map(rusqlite::params![person_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect();

    if accounts.is_empty() {
        return None;
    }
    if accounts.len() == 1 || email_context_lower.is_empty() {
        return Some(accounts[0].1.clone());
    }

    // Score each account by name/keyword match in the email context
    let mut best_name = &accounts[0].1;
    let mut best_score = 0i32;
    for (_id, name, keywords) in &accounts {
        let mut score = 0i32;
        if email_context_lower.contains(&name.to_lowercase()) {
            score += 10;
        }
        // Check keywords JSON array: ["keyword1", "keyword2", ...]
        if let Some(kw_json) = keywords {
            if let Ok(kws) = serde_json::from_str::<Vec<String>>(kw_json) {
                for kw in &kws {
                    if email_context_lower.contains(&kw.to_lowercase()) {
                        score += 5;
                    }
                }
            }
        }
        if score > best_score {
            best_score = score;
            best_name = name;
        }
    }

    Some(best_name.clone())
}
