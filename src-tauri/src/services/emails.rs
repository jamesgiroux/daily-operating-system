// Emails service — extracted from commands.rs
// Business logic for email enrichment and retrieval.

use std::collections::{HashMap, HashSet};
use tauri::Emitter;

use crate::state::AppState;
use crate::types::{
    EmailBriefingData, EmailBriefingStats, EmailSignal, EnrichedEmail, EntityEmailThread,
};

/// Get enriched email data for the emails page.
///
/// Tries to load emails from the DB first (I368). If the DB has active emails,
/// uses those. Otherwise falls back to JSON loading for first-run compatibility.
pub async fn get_emails_enriched(state: &AppState) -> Result<EmailBriefingData, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Config lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // DB is the source of truth for active emails (I448: resolved_at IS NULL filtering)
    let db_emails: Vec<crate::db::DbEmail> = state
        .db_read(|db| db.get_all_active_emails().map_err(|e| e.to_string()))
        .await
        .unwrap_or_default();
    // Gmail inbox is thread-based in most clients. Collapse to one row per thread
    // so counts and listing align with what users see in Gmail.
    let thread_emails = collapse_to_latest_thread_emails(&db_emails);

    let emails = if !thread_emails.is_empty() {
        // Batch-resolve entity names from IDs
        let entity_ids: HashSet<String> = thread_emails
            .iter()
            .filter_map(|e| e.entity_id.clone())
            .collect();
        // Build email context map outside DB closure for the person account lookup
        let email_context_map: HashMap<String, String> = entity_ids
            .iter()
            .map(|eid| {
                let context: String = thread_emails
                    .iter()
                    .filter(|e| e.entity_id.as_deref() == Some(eid.as_str()))
                    .filter_map(|e| e.contextual_summary.as_deref().or(e.subject.as_deref()))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                (eid.clone(), context)
            })
            .collect();

        let entity_names: HashMap<String, String> = state
            .db_read(move |db| {
                let mut map = HashMap::new();
                for (eid, email_context) in &email_context_map {
                    // Look up entity name, and for persons also find linked account
                    if let Ok(Some(p)) = db.get_person(eid) {
                        let account_name = best_account_for_person(db, eid, email_context);
                        let display = account_name.unwrap_or(p.name);
                        map.insert(eid.clone(), display);
                    } else if let Ok(Some(a)) = db.get_account(eid) {
                        map.insert(eid.clone(), a.name);
                    } else if let Ok(Some(p)) = db.get_project(eid) {
                        map.insert(eid.clone(), p.name);
                    }
                }
                Ok(map)
            })
            .await
            .unwrap_or_default();

        thread_emails
            .iter()
            .map(|dbe| {
                let entity_name = dbe
                    .entity_id
                    .as_ref()
                    .and_then(|eid| entity_names.get(eid).cloned());
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
        Vec::new()
    };

    // I395: Sort by relevance score (highest first, nulls last)
    let mut emails = emails;
    emails.sort_by(|a, b| {
        let sa = a.relevance_score.unwrap_or(-1.0);
        let sb = b.relevance_score.unwrap_or(-1.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // I368 AC3: Write emails.json from DB so it stays current even without a Gmail fetch
    if !thread_emails.is_empty() {
        let json_path = today_dir.join("data").join("emails.json");
        if let Ok(json) = serde_json::to_string_pretty(&emails) {
            let _ = std::fs::create_dir_all(today_dir.join("data"));
            let _ = std::fs::write(&json_path, json);
        }
    }

    // I513: Build replies_needed from DB instead of directive file.
    let replies_needed: Vec<crate::json_loader::DirectiveReplyNeeded> = state
        .db_read(|db| {
            let now = chrono::Utc::now();
            Ok(db
                .get_threads_awaiting_reply()
                .unwrap_or_default()
                .into_iter()
                .map(|(thread_id, subject, from, date)| {
                    let wait_duration =
                        crate::prepare::orchestrate::compute_wait_duration_public(&date, &now);
                    crate::json_loader::DirectiveReplyNeeded {
                        thread_id,
                        subject,
                        from,
                        date: Some(date),
                        wait_duration: Some(wait_duration),
                    }
                })
                .collect())
        })
        .await
        .unwrap_or_default();

    // Collect email IDs for batch signal lookup
    let email_ids: Vec<String> = emails.iter().map(|e| e.id.clone()).collect();

    // Batch-query signals from DB
    let email_ids_clone = email_ids.clone();
    let db_signals = state
        .db_read(move |db| {
            db.list_email_signals_by_email_ids(&email_ids_clone)
                .map_err(|e| e.to_string())
        })
        .await
        .unwrap_or_default();

    let has_enrichment = !db_signals.is_empty() || emails.iter().any(|e| e.summary.is_some());

    // Index signals by email_id
    let mut signals_by_email: HashMap<String, Vec<EmailSignal>> = HashMap::new();
    for sig in &db_signals {
        signals_by_email
            .entry(sig.email_id.clone())
            .or_default()
            .push(EmailSignal {
                id: Some(sig.id),
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

    // Capture entity IDs before the loop consumes emails (used for narrative below)
    let email_entity_ids: HashSet<String> =
        emails.iter().filter_map(|e| e.entity_id.clone()).collect();

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
            id: Some(sig.id),
            signal_type: sig.signal_type.clone(),
            signal_text: sig.signal_text.clone(),
            confidence: sig.confidence,
            sentiment: sig.sentiment.clone(),
            urgency: sig.urgency.clone(),
            detected_at: Some(sig.detected_at.clone()),
        });
        entry.2.insert(sig.email_id.clone());
    }

    // Batch-resolve entity names from DB
    let entity_lookup_keys: Vec<(String, String)> = entity_map
        .keys()
        .map(|eid| {
            let etype = entity_map
                .get(eid)
                .map(|(et, _, _)| et.clone())
                .unwrap_or_default();
            (eid.clone(), etype)
        })
        .collect();

    let resolved_names: HashMap<String, String> = state
        .db_read(move |db| {
            let mut map = HashMap::new();
            for (eid, etype) in &entity_lookup_keys {
                let name = if etype == "account" {
                    db.get_account(eid).ok().flatten().map(|a| a.name)
                } else {
                    db.get_project(eid).ok().flatten().map(|p| p.name)
                };
                if let Some(n) = name {
                    map.insert(eid.clone(), n);
                }
            }
            Ok(map)
        })
        .await
        .unwrap_or_default();

    let entity_threads: Vec<EntityEmailThread> = entity_map
        .into_iter()
        .map(|(entity_id, (entity_type, signals, email_set))| {
            let entity_name = resolved_names
                .get(&entity_id)
                .cloned()
                .unwrap_or_else(|| entity_id.clone());

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

    // I448: Build narrative dynamically from real counts, not the stale directive.
    // Count how many emails are linked to entities that have meetings today.
    let meeting_linked = if email_entity_ids.is_empty() {
        0usize
    } else {
        let ids: Vec<String> = email_entity_ids.into_iter().collect();
        let email_tz: chrono_tz::Tz = state
            .config
            .read()
            .ok()
            .and_then(|c| c.as_ref().map(|c| c.schedules.today.timezone.clone()))
            .and_then(|t| t.parse().ok())
            .unwrap_or(chrono_tz::America::New_York);
        let tf_em = crate::helpers::today_meeting_filter(&email_tz);
        let em_start = tf_em.date;
        let em_end = tf_em.next_date;
        state
            .db_read(move |db| {
                let count = ids
                    .iter()
                    .filter(|eid| {
                        db.conn_ref()
                            .query_row(
                                "SELECT COUNT(*) FROM meeting_entities me
                         JOIN meetings mh ON me.meeting_id = mh.id
                         WHERE me.entity_id = ?1 AND mh.start_time >= ?2 AND mh.start_time < ?3",
                                rusqlite::params![eid, em_start, em_end],
                                |row| row.get::<_, i64>(0),
                            )
                            .unwrap_or(0)
                            > 0
                    })
                    .count();
                Ok::<usize, String>(count)
            })
            .await
            .unwrap_or(0)
    };

    let email_narrative: Option<String> = if total == 0 {
        None
    } else if meeting_linked > 0 {
        Some(format!(
            "{} threads in your inbox, {} linked to today's meetings.",
            total, meeting_linked
        ))
    } else {
        Some(format!("{} threads in your inbox.", total))
    };

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

fn collapse_to_latest_thread_emails(db_emails: &[crate::db::DbEmail]) -> Vec<crate::db::DbEmail> {
    let mut seen_threads: HashSet<String> = HashSet::new();
    let mut collapsed = Vec::new();

    for email in db_emails {
        let thread_key = email
            .thread_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .unwrap_or(&email.email_id)
            .to_string();
        if seen_threads.insert(thread_key) {
            collapsed.push(email.clone());
        }
    }

    collapsed
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
         JOIN account_stakeholders as_ ON a.id = as_.account_id
         WHERE as_.person_id = ?1",
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

// ── I451: Email mutation handlers extracted from commands.rs ──────────

/// Get emails linked to a specific entity for entity detail pages (I368 AC5).
/// Queries by entity_id directly, OR by sender domain for accounts without direct entity links.
pub fn get_entity_emails(
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
) -> Result<Vec<crate::db::DbEmail>, String> {
    // Try direct entity_id match first
    let direct = db
        .get_emails_for_entity(entity_id)
        .map_err(|e| e.to_string())?;
    if !direct.is_empty() {
        return Ok(direct);
    }

    // For person entities, also check emails sent by this person's email addresses
    if entity_type == "person" {
        if let Ok(Some(person)) = db.get_person(entity_id) {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                            priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                            last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                            contextual_summary, sentiment, urgency, user_is_last_sender,
                            last_sender_email, message_count, created_at, updated_at,
                            relevance_score, score_reason
                     FROM emails WHERE sender_email = ?1 ORDER BY received_at DESC",
                )
                .map_err(|e| format!("query error: {e}"))?;
            let rows = stmt
                .query_map([&person.email], |row| {
                    Ok(crate::db::DbEmail {
                        email_id: row.get(0)?,
                        thread_id: row.get(1)?,
                        sender_email: row.get(2)?,
                        sender_name: row.get(3)?,
                        subject: row.get(4)?,
                        snippet: row.get(5)?,
                        priority: row.get(6)?,
                        is_unread: row.get::<_, i32>(7)? != 0,
                        received_at: row.get(8)?,
                        enrichment_state: row.get(9)?,
                        enrichment_attempts: row.get(10)?,
                        last_enrichment_at: row.get(11)?,
                        last_seen_at: row.get(12)?,
                        resolved_at: row.get(13)?,
                        entity_id: row.get(14)?,
                        entity_type: row.get(15)?,
                        contextual_summary: row.get(16)?,
                        sentiment: row.get(17)?,
                        urgency: row.get(18)?,
                        user_is_last_sender: row.get::<_, i32>(19)? != 0,
                        last_sender_email: row.get(20)?,
                        message_count: row.get(21)?,
                        created_at: row.get(22)?,
                        updated_at: row.get(23)?,
                        relevance_score: row.get(24).ok(),
                        score_reason: row.get(25).ok(),
                    })
                })
                .map_err(|e| format!("query error: {e}"))?;
            let by_sender: Vec<_> = rows.flatten().collect();
            if !by_sender.is_empty() {
                return Ok(by_sender);
            }
        }
    }

    // For account entities, check emails from people linked to this account
    if entity_type == "account" {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT DISTINCT pe.email FROM account_stakeholders as_
                 JOIN person_emails pe ON as_.person_id = pe.person_id
                 WHERE as_.account_id = ?1",
            )
            .map_err(|e| format!("query error: {e}"))?;
        let emails_list: Vec<String> = stmt
            .query_map([entity_id], |row| row.get(0))
            .map_err(|e| format!("query error: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        if !emails_list.is_empty() {
            let placeholders: Vec<String> = (0..emails_list.len())
                .map(|i| format!("?{}", i + 1))
                .collect();
            let sql = format!(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason
                 FROM emails WHERE sender_email IN ({}) ORDER BY received_at DESC",
                placeholders.join(",")
            );
            let mut stmt = db
                .conn_ref()
                .prepare(&sql)
                .map_err(|e| format!("query error: {e}"))?;
            let params: Vec<&dyn rusqlite::types::ToSql> = emails_list
                .iter()
                .map(|s| s as &dyn rusqlite::types::ToSql)
                .collect();
            let rows = stmt
                .query_map(params.as_slice(), |row| {
                    Ok(crate::db::DbEmail {
                        email_id: row.get(0)?,
                        thread_id: row.get(1)?,
                        sender_email: row.get(2)?,
                        sender_name: row.get(3)?,
                        subject: row.get(4)?,
                        snippet: row.get(5)?,
                        priority: row.get(6)?,
                        is_unread: row.get::<_, i32>(7)? != 0,
                        received_at: row.get(8)?,
                        enrichment_state: row.get(9)?,
                        enrichment_attempts: row.get(10)?,
                        last_enrichment_at: row.get(11)?,
                        last_seen_at: row.get(12)?,
                        resolved_at: row.get(13)?,
                        entity_id: row.get(14)?,
                        entity_type: row.get(15)?,
                        contextual_summary: row.get(16)?,
                        sentiment: row.get(17)?,
                        urgency: row.get(18)?,
                        user_is_last_sender: row.get::<_, i32>(19)? != 0,
                        last_sender_email: row.get(20)?,
                        message_count: row.get(21)?,
                        created_at: row.get(22)?,
                        updated_at: row.get(23)?,
                        relevance_score: row.get(24).ok(),
                        score_reason: row.get(25).ok(),
                    })
                })
                .map_err(|e| format!("query error: {e}"))?;
            let results: Vec<_> = rows.flatten().collect();
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    Ok(Vec::new())
}

/// Update the entity assignment for an email with signal emission.
pub fn update_email_entity(
    db: &crate::db::ActionDb,
    email_id: &str,
    entity_id: Option<&str>,
    entity_type: Option<&str>,
) -> Result<(), String> {
    db.update_email_entity(email_id, entity_id, entity_type)?;

    let etype = entity_type.unwrap_or("email");
    let eid = entity_id.unwrap_or(email_id);
    let _ = crate::services::signals::emit(
        db,
        etype,
        eid,
        "email_entity_reassigned",
        "user_correction",
        Some(&format!("{{\"email_id\":\"{}\"}}", email_id)),
        0.9,
    );

    Ok(())
}

/// Dismiss a single email signal by ID with relevance learning signal.
pub fn dismiss_email_signal(db: &crate::db::ActionDb, signal_id: i64) -> Result<(), String> {
    let context = db
        .dismiss_email_signal(signal_id)
        .map_err(|e| e.to_string())?;

    if let Some((entity_id, entity_type, signal_type, email_id)) = context {
        let _ = crate::services::signals::emit(
            db,
            &entity_type,
            &entity_id,
            "email_signal_dismissed",
            "user_correction",
            Some(&format!(
                "{{\"signal_id\":{},\"signal_type\":\"{}\",\"email_id\":\"{}\"}}",
                signal_id, signal_type, email_id
            )),
            0.3,
        );
    }

    Ok(())
}

/// Dismiss an email item (commitment, question, etc.) with signal emission.
pub fn dismiss_email_item(
    db: &crate::db::ActionDb,
    item_type: &str,
    email_id: &str,
    item_text: &str,
    sender_domain: Option<&str>,
    email_type: Option<&str>,
    entity_id: Option<&str>,
) -> Result<(), String> {
    db.dismiss_email_item(
        item_type,
        email_id,
        item_text,
        sender_domain,
        email_type,
        entity_id,
    )
    .map_err(|e| e.to_string())?;

    let etype = entity_id.map(|_| "account").unwrap_or("email");
    let eid = entity_id.unwrap_or(email_id);
    let _ = crate::services::signals::emit(
        db,
        etype,
        eid,
        "email_item_dismissed",
        item_type,
        Some(&format!(
            "{{\"email_id\":\"{}\",\"item_type\":\"{}\"}}",
            email_id, item_type
        )),
        0.3,
    );

    Ok(())
}

struct InboxPresenceReconcileResult {
    changed: bool,
    reappeared_or_new_count: usize,
}

fn reconcile_inbox_presence_from_ids(
    db: &crate::db::ActionDb,
    inbox_ids: &HashSet<String>,
) -> Result<InboxPresenceReconcileResult, String> {
    let active_db_emails = db.get_all_active_emails().map_err(|e| e.to_string())?;
    let db_ids: HashSet<String> = active_db_emails
        .iter()
        .map(|e| e.email_id.clone())
        .collect();

    let vanished: Vec<String> = db_ids.difference(inbox_ids).cloned().collect();
    let reappeared_or_new: Vec<String> = inbox_ids.difference(&db_ids).cloned().collect();

    let mut changed = false;

    if !vanished.is_empty() {
        let resolved = db.mark_emails_resolved(&vanished)?;
        let deactivated = db
            .deactivate_signals_for_emails(&vanished)
            .map_err(|e| e.to_string())?;
        if resolved > 0 || deactivated > 0 {
            changed = true;
            log::info!(
                "Email inbox reconcile: resolved {} vanished emails, deactivated {} signals",
                resolved,
                deactivated
            );
        }
    }

    if !reappeared_or_new.is_empty() {
        let reopened = db.unmark_resolved(&reappeared_or_new)?;
        if reopened > 0 {
            changed = true;
            log::info!(
                "Email inbox reconcile: unmarked {} reappeared emails",
                reopened
            );
        }
    }

    Ok(InboxPresenceReconcileResult {
        changed,
        reappeared_or_new_count: reappeared_or_new.len(),
    })
}

/// Fast inbox-presence sync for the /emails page.
///
/// Reconciles local active emails against current Gmail inbox IDs without
/// triggering enrichment PTY work. This keeps archived emails from lingering.
pub async fn sync_email_inbox_presence(
    state: &std::sync::Arc<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    let access_token = crate::google_api::get_valid_access_token()
        .await
        .map_err(|e| format!("Gmail auth failed: {}", e))?;
    let inbox_ids = crate::google_api::gmail::fetch_inbox_message_ids(&access_token, 100)
        .await
        .map_err(|e| format!("Gmail inbox sync failed: {}", e))?;

    let result = state
        .db_write(move |db| reconcile_inbox_presence_from_ids(db, &inbox_ids))
        .await?;

    if result.changed {
        let _ = app_handle.emit("emails-updated", ());
    }

    // If Gmail has IDs we don't have active locally, wake the poller to ingest
    // those messages and classify/enrich in the normal pipeline.
    if result.reappeared_or_new_count > 0 {
        state.integrations.email_poller_wake.notify_one();
    }

    Ok(result.changed)
}

/// Archive low-priority emails in Gmail and remove from local data (I144).
pub async fn archive_low_priority_emails(state: &AppState) -> Result<usize, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let emails_path = workspace.join("_today").join("data").join("emails.json");

    let content = std::fs::read_to_string(&emails_path)
        .map_err(|e| format!("Failed to read emails.json: {}", e))?;
    let mut data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse emails.json: {}", e))?;

    let low_emails = data["lowPriority"].as_array().cloned().unwrap_or_default();
    let ids: Vec<String> = low_emails
        .iter()
        .filter_map(|e| e["id"].as_str().map(String::from))
        .collect();

    if ids.is_empty() {
        return Ok(0);
    }

    let access_token = crate::google_api::get_valid_access_token()
        .await
        .map_err(|e| format!("Gmail auth failed: {}", e))?;

    let archived = crate::google_api::gmail::archive_emails(&access_token, &ids)
        .await
        .map_err(|e| format!("Gmail archive failed: {}", e))?;

    data["lowPriority"] = serde_json::json!([]);
    if let Some(stats) = data.get_mut("stats") {
        let high = stats["highCount"].as_u64().unwrap_or(0);
        let medium = stats["mediumCount"].as_u64().unwrap_or(0);
        stats["lowCount"] = serde_json::json!(0);
        stats["total"] = serde_json::json!(high + medium);
    }

    crate::util::atomic_write_str(
        &emails_path,
        &serde_json::to_string_pretty(&data)
            .map_err(|e| format!("Failed to serialize emails: {}", e))?,
    )
    .map_err(|e| format!("Failed to write emails.json: {}", e))?;

    log::info!("Archived {} low-priority emails in Gmail", archived);
    Ok(archived)
}

/// Refresh emails independently without re-running the full /today pipeline (I20).
pub async fn refresh_emails(
    state: &std::sync::Arc<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state_clone = state.clone();
    let workspace_path = config.workspace_path.clone();

    tauri::async_runtime::spawn(async move {
        let workspace = std::path::Path::new(&workspace_path);
        let executor = crate::executor::Executor::new(state_clone, app_handle);
        executor.execute_email_refresh(workspace).await
    })
    .await
    .map_err(|e| format!("Email refresh task failed: {}", e))?
    .map(|_| "Email refresh complete".to_string())
}
