// Emails service — extracted from commands.rs
// Business logic for email enrichment and retrieval.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use tauri::Emitter;

use crate::state::AppState;
use crate::types::{
    EmailBriefingData, EmailBriefingStats, EmailSignal, EnrichedEmail, EntityEmailThread,
    GoneQuietAccount, LinkedMeeting, ReplyDebtItem, TrackedEmailCommitment,
};

/// Get enriched email data for the emails page.
///
/// Tries to load emails from the DB first (I368). If the DB has active emails,
/// uses those. Otherwise falls back to JSON loading for first-run compatibility.
pub async fn get_emails_enriched(state: &AppState) -> Result<EmailBriefingData, String> {
    let config = state
        .config
        .read()
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
                    is_unread: dbe.is_unread,
                    avatar_url: None,
                    summary: dbe.contextual_summary.clone(),
                    recommended_action: None,
                    conversation_arc: None,
                    email_type: None,
                    commitments: dbe
                        .commitments
                        .as_ref()
                        .and_then(|c| serde_json::from_str::<Vec<String>>(c).ok())
                        .unwrap_or_default(),
                    questions: dbe
                        .questions
                        .as_ref()
                        .and_then(|q| serde_json::from_str::<Vec<String>>(q).ok())
                        .unwrap_or_default(),
                    sentiment: dbe.sentiment.clone(),
                    urgency: dbe.urgency.clone(),
                    entity_id: dbe.entity_id.clone(),
                    entity_type: dbe.entity_type.clone(),
                    entity_name,
                    relevance_score: dbe.relevance_score,
                    score_reason: dbe.score_reason.clone(),
                    pinned_at: dbe.pinned_at.clone(),
                    tracked_commitments: Vec::new(),
                    meeting_linked: None,
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // I395: Sort by relevance score (highest first, nulls last)
    let mut emails = emails;
    emails.sort_by(compare_email_rank);

    // I368 AC3: Write emails.json from DB so it stays current even without a Gmail fetch
    if !thread_emails.is_empty() {
        let json_path = today_dir.join("data").join("emails.json");
        if let Ok(json) = serde_json::to_string_pretty(&emails) {
            let _ = std::fs::create_dir_all(today_dir.join("data"));
            let _ = std::fs::write(&json_path, json);
        }
    }

    // I577: Build reply debt from active emails where user hasn't replied yet
    // and the email is linked to a tracked entity.
    let email_entity_names: HashMap<String, String> = emails
        .iter()
        .filter_map(|em| em.entity_name.as_ref().map(|n| (em.id.clone(), n.clone())))
        .collect();
    let now_utc = chrono::Utc::now();
    let reply_debt: Vec<ReplyDebtItem> = thread_emails
        .iter()
        .filter(|dbe| {
            !dbe.user_is_last_sender
                && dbe.resolved_at.is_none()
                && dbe.entity_id.is_some()
                && dbe.enrichment_state == "enriched" // only show enriched emails
                && dbe.contextual_summary.is_some() // must have AI context
        })
        .filter_map(|dbe| {
            let received_at = dbe.received_at.as_deref()?;
            let received_dt =
                chrono::NaiveDateTime::parse_from_str(received_at, "%Y-%m-%dT%H:%M:%S%.fZ")
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(received_at, "%Y-%m-%d %H:%M:%S")
                    })
                    .or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339(received_at).map(|dt| dt.naive_utc())
                    })
                    .ok()?;
            let age_hours = (now_utc.naive_utc() - received_dt).num_minutes() as f64 / 60.0;
            let age_bucket = if age_hours < 24.0 {
                "today"
            } else if age_hours < 48.0 {
                "1-2 days"
            } else if age_hours < 120.0 {
                "3-5 days"
            } else {
                "overdue"
            };

            Some(ReplyDebtItem {
                email_id: dbe.email_id.clone(),
                sender_name: dbe.sender_name.clone().unwrap_or_default(),
                sender_email: dbe.sender_email.clone().unwrap_or_default(),
                subject: dbe.subject.clone().unwrap_or_default(),
                summary: dbe.contextual_summary.clone(),
                entity_id: dbe.entity_id.clone(),
                entity_name: email_entity_names.get(&dbe.email_id).cloned(),
                entity_type: dbe.entity_type.clone(),
                age_hours,
                age_bucket: age_bucket.to_string(),
                urgency: dbe.urgency.clone(),
                sentiment: dbe.sentiment.clone(),
            })
        })
        .collect();

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

    let tracked_commitments_by_email: HashMap<String, Vec<TrackedEmailCommitment>> = state
        .db_read({
            let email_ids = email_ids.clone();
            move |db| {
                let actions = db
                    .get_actions_by_source_type_and_ids("email_commitment", &email_ids)
                    .map_err(|e| e.to_string())?;
                let mut tracked: HashMap<String, Vec<TrackedEmailCommitment>> = HashMap::new();
                for action in actions {
                    let Some(source_id) = action.source_id.clone() else {
                        continue;
                    };
                    let (commitment_text, owner) =
                        parse_email_commitment_context(action.context.as_deref());
                    tracked
                        .entry(source_id)
                        .or_default()
                        .push(TrackedEmailCommitment {
                            action_id: action.id.clone(),
                            commitment_text: commitment_text
                                .unwrap_or_else(|| action.title.clone()),
                            action_title: action.title.clone(),
                            due_date: action.due_date.clone(),
                            owner,
                        });
                }
                Ok(tracked)
            }
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

    for mut email in emails {
        email.tracked_commitments = tracked_commitments_by_email
            .get(&email.id)
            .cloned()
            .unwrap_or_default();
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
            .as_ref()
            .map(|c| c.schedules.today.timezone.clone())
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

    // ── I581: Gone-quiet accounts from entity_email_cadence ──────────────
    let gone_quiet: Vec<GoneQuietAccount> = state
        .db_read(detect_gone_quiet_accounts)
        .await
        .unwrap_or_default();

    // Gap 3: Emit email_cadence_drop signals for detected gone-quiet accounts,
    // with 7-day deduplication so the callout system can surface them.
    if !gone_quiet.is_empty() {
        let gq_accounts = gone_quiet.clone();
        let _ = state
            .db_read(move |db| {
                let engine = crate::signals::propagation::PropagationEngine::default();
                for acct in &gq_accounts {
                    // Dedup: skip if a recent email_cadence_drop signal already exists
                    let recent_exists: bool = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*) FROM signal_events
                             WHERE entity_id = ?1
                               AND signal_type = 'email_cadence_drop'
                               AND created_at > datetime('now', '-7 days')",
                            rusqlite::params![acct.entity_id],
                            |row| row.get::<_, i64>(0).map(|c| c > 0),
                        )
                        .unwrap_or(false);
                    if recent_exists {
                        continue;
                    }
                    let value_json = format!(
                        "{{\"normal_interval_days\":{:.1},\"days_since_last\":{:.0}}}",
                        acct.normal_interval_days, acct.days_since_last_email
                    );
                    let _ = crate::signals::bus::emit_signal_and_propagate(
                        db,
                        &engine,
                        &acct.entity_type,
                        &acct.entity_id,
                        "email_cadence_drop",
                        "system",
                        Some(&value_json),
                        0.6,
                    );
                }
                Ok::<(), String>(())
            })
            .await;
    }

    // ── I582: Link emails to upcoming meetings via pre_meeting_context bridge ──
    let all_sender_emails: HashSet<String> = high
        .iter()
        .chain(medium.iter())
        .chain(low.iter())
        .map(|ee| ee.email.sender_email.clone())
        .filter(|s| !s.is_empty())
        .collect();

    if !all_sender_emails.is_empty() {
        let sender_list: Vec<String> = all_sender_emails.into_iter().collect();
        let meeting_links: HashMap<String, LinkedMeeting> = state
            .db_read(move |db| load_pre_meeting_links(db, &sender_list))
            .await
            .unwrap_or_default();

        // Apply meeting links to enriched emails
        for enriched in high
            .iter_mut()
            .chain(medium.iter_mut())
            .chain(low.iter_mut())
        {
            let sender_lower = enriched.email.sender_email.to_lowercase();
            if let Some(link) = meeting_links.get(&sender_lower) {
                enriched.email.meeting_linked = Some(link.clone());
            }
        }
    }

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
        reply_debt,
        gone_quiet,
    })
}

pub fn compare_email_rank(a: &crate::types::Email, b: &crate::types::Email) -> Ordering {
    match (a.pinned_at.is_some(), b.pinned_at.is_some()) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }

    let sa = a.relevance_score.unwrap_or(-1.0);
    let sb = b.relevance_score.unwrap_or(-1.0);
    sb.partial_cmp(&sa).unwrap_or(Ordering::Equal)
}

fn build_email_commitment_context(owner: Option<&str>, original_commitment: &str) -> String {
    let mut lines = vec![format!(
        "Original commitment: {}",
        original_commitment.trim()
    )];
    if let Some(owner) = owner.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Owner: {}", owner.trim()));
    }
    lines.join("\n")
}

fn parse_email_commitment_context(context: Option<&str>) -> (Option<String>, Option<String>) {
    let mut commitment_text = None;
    let mut owner = None;

    if let Some(context) = context {
        for line in context.lines() {
            if let Some(value) = line.strip_prefix("Original commitment:") {
                let value = value.trim();
                if !value.is_empty() {
                    commitment_text = Some(value.to_string());
                }
            } else if let Some(value) = line.strip_prefix("Owner:") {
                let value = value.trim();
                if !value.is_empty() {
                    owner = Some(value.to_string());
                }
            }
        }
    }

    (commitment_text, owner)
}

fn parse_email_datetime(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::DateTime::parse_from_rfc2822(value).map(|dt| dt.with_timezone(&chrono::Utc))
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").map(|dt| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
            })
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").map(|dt| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
            })
        })
        .ok()
}

pub fn detect_gone_quiet_accounts(
    db: &crate::db::ActionDb,
) -> Result<Vec<GoneQuietAccount>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT entity_id, AVG(message_count) AS avg_count, SUM(message_count) AS total_count
             FROM entity_email_cadence
             WHERE entity_type = 'account'
             GROUP BY entity_id
             HAVING total_count >= 3 AND avg_count > 0
             ORDER BY avg_count DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, f64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let now = chrono::Utc::now();
    let mut alerts = Vec::new();
    for (entity_id, avg_count, historical_email_count) in rows {
        let Some(account) = db.get_account(&entity_id).map_err(|e| e.to_string())? else {
            continue;
        };

        let last_email: Option<(Option<String>, Option<String>, Option<String>)> = db
            .conn_ref()
            .query_row(
                "SELECT received_at, sender_name, sender_email
                 FROM emails
                 WHERE entity_id = ?1 AND entity_type = 'account'
                 ORDER BY datetime(received_at) DESC, datetime(created_at) DESC
                 LIMIT 1",
                rusqlite::params![entity_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();
        let Some((last_email_date, sender_name, sender_email)) = last_email else {
            continue;
        };
        let Some(last_email_iso) = last_email_date else {
            continue;
        };
        let Some(last_email_at) = parse_email_datetime(&last_email_iso) else {
            continue;
        };

        let days_since_last_email = (now - last_email_at).num_days();
        let normal_interval_days = (7.0 / avg_count).max(1.0);
        if (days_since_last_email as f64) <= normal_interval_days * 2.0 {
            continue;
        }

        let dismissed_after_last_email: bool = db
            .conn_ref()
            .query_row(
                "SELECT created_at
                 FROM signal_events
                 WHERE entity_type = 'account'
                   AND entity_id = ?1
                   AND signal_type = 'email_cadence_drop_dismissed'
                   AND superseded_by IS NULL
                 ORDER BY created_at DESC
                 LIMIT 1",
                rusqlite::params![entity_id],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|created_at| parse_email_datetime(&created_at))
            .is_some_and(|dismissed_at| dismissed_at >= last_email_at);
        if dismissed_after_last_email {
            continue;
        }

        alerts.push(GoneQuietAccount {
            entity_id,
            entity_name: account.name,
            entity_type: "account".to_string(),
            normal_interval_days,
            days_since_last_email,
            last_email_date: Some(last_email_iso),
            last_email_sender: sender_name.or(sender_email),
            historical_email_count,
        });
    }

    alerts.sort_by(|a, b| {
        let ar = a.days_since_last_email as f64 / a.normal_interval_days.max(1.0);
        let br = b.days_since_last_email as f64 / b.normal_interval_days.max(1.0);
        br.partial_cmp(&ar).unwrap_or(Ordering::Equal)
    });
    Ok(alerts)
}

fn load_pre_meeting_links(
    db: &crate::db::ActionDb,
    sender_list: &[String],
) -> Result<HashMap<String, LinkedMeeting>, String> {
    let sender_set: HashSet<String> = sender_list.iter().map(|s| s.to_lowercase()).collect();
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT se.value, m.id, m.title, m.start_time
             FROM signal_events se
             JOIN meetings m ON m.id = se.entity_id
             WHERE se.entity_type = 'meeting'
               AND se.signal_type = 'pre_meeting_context'
               AND se.superseded_by IS NULL
               AND m.start_time >= datetime('now')
               AND m.start_time <= datetime('now', '+48 hours')
             ORDER BY m.start_time ASC, se.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for row in rows.flatten() {
        let (value_json, meeting_id, title, start_time) = row;
        let Some(value_json) = value_json else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&value_json) else {
            continue;
        };
        let Some(sender_email) = value.get("sender_email").and_then(|v| v.as_str()) else {
            continue;
        };
        let sender_lower = sender_email.to_lowercase();
        if !sender_set.contains(&sender_lower) || map.contains_key(&sender_lower) {
            continue;
        }
        map.insert(
            sender_lower,
            LinkedMeeting {
                meeting_id,
                title,
                start_time,
            },
        );
    }
    Ok(map)
}

pub fn collapse_to_latest_thread_emails(
    db_emails: &[crate::db::DbEmail],
) -> Vec<crate::db::DbEmail> {
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
                            last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                            contextual_summary, sentiment, urgency, user_is_last_sender,
                            last_sender_email, message_count, created_at, updated_at,
                            relevance_score, score_reason,
                            pinned_at, commitments, questions
                     FROM emails WHERE sender_email = ?1 AND resolved_at IS NULL ORDER BY received_at DESC",
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
                        enriched_at: row.get(12).ok(),
                        last_seen_at: row.get(13)?,
                        resolved_at: row.get(14)?,
                        entity_id: row.get(15)?,
                        entity_type: row.get(16)?,
                        contextual_summary: row.get(17)?,
                        sentiment: row.get(18)?,
                        urgency: row.get(19)?,
                        user_is_last_sender: row.get::<_, i32>(20)? != 0,
                        last_sender_email: row.get(21)?,
                        message_count: row.get(22)?,
                        created_at: row.get(23)?,
                        updated_at: row.get(24)?,
                        relevance_score: row.get(25).ok(),
                        score_reason: row.get(26).ok(),
                        pinned_at: row.get(27).ok(),
                        commitments: row.get(28).ok(),
                        questions: row.get(29).ok(),
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
                        last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason,
                            pinned_at, commitments, questions
                 FROM emails WHERE sender_email IN ({}) AND resolved_at IS NULL ORDER BY received_at DESC",
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
                        enriched_at: row.get(12).ok(),
                        last_seen_at: row.get(13)?,
                        resolved_at: row.get(14)?,
                        entity_id: row.get(15)?,
                        entity_type: row.get(16)?,
                        contextual_summary: row.get(17)?,
                        sentiment: row.get(18)?,
                        urgency: row.get(19)?,
                        user_is_last_sender: row.get::<_, i32>(20)? != 0,
                        last_sender_email: row.get(21)?,
                        message_count: row.get(22)?,
                        created_at: row.get(23)?,
                        updated_at: row.get(24)?,
                        relevance_score: row.get(25).ok(),
                        score_reason: row.get(26).ok(),
                        pinned_at: row.get(27).ok(),
                        commitments: row.get(28).ok(),
                        questions: row.get(29).ok(),
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

/// Mark an email as replied to (I577 reply debt).
/// Sets `user_is_last_sender` and emits a `reply_debt_cleared` signal via the bus
/// with propagation so downstream effects (health scoring, prep invalidation) fire.
pub fn mark_reply_sent(db: &crate::db::ActionDb, email_id: &str) -> Result<(), String> {
    let entity_info = db.mark_reply_sent(email_id)?;

    // Emit engagement signal if the email is linked to an entity
    if let Some((entity_id, entity_type)) = entity_info {
        let engine = crate::signals::propagation::PropagationEngine::default();
        let _ = crate::signals::bus::emit_signal_and_propagate(
            db,
            &engine,
            &entity_type,
            &entity_id,
            "reply_debt_cleared",
            "user_action",
            Some(&format!("{{\"email_id\":\"{}\"}}", email_id)),
            0.8,
        );
    }

    Ok(())
}

// ── I579: Per-email triage actions ────────────────────────────────────

/// Archive an email: set resolved_at locally AND archive in Gmail.
/// Signal emission for Intelligence Loop compliance.
pub async fn archive_email(state: &AppState, email_id: &str) -> Result<String, String> {
    let eid = email_id.to_string();
    let thread_email_ids = state
        .db_read({
            let eid = eid.clone();
            move |db| db.get_thread_email_ids(&eid).map_err(|e| e.to_string())
        })
        .await?;
    state
        .db_write(move |db| {
            db.archive_email(&eid)?;

            let engine = crate::signals::propagation::PropagationEngine::default();
            let (entity_type, entity_id) = email_entity_context(db, &eid);
            let _ = crate::services::signals::emit_and_propagate(
                db,
                &engine,
                &entity_type,
                &entity_id,
                "email_archived",
                "user_action",
                Some(&format!("{{\"email_id\":\"{}\"}}", eid)),
                0.5,
            );
            Ok(())
        })
        .await?;

    // Also archive in Gmail (remove INBOX label) so sync doesn't bring it back
    if let Ok(token) = crate::google_api::get_valid_access_token().await {
        if let Err(e) = crate::google_api::gmail::archive_emails(&token, &thread_email_ids).await {
            log::warn!("Gmail archive failed for {email_id}: {e:?} — archived locally only");
        }
    }

    Ok(email_id.to_string())
}

/// Unarchive an email: clear resolved_at locally AND move back to Gmail inbox.
pub async fn unarchive_email(state: &AppState, email_id: &str) -> Result<(), String> {
    let eid = email_id.to_string();
    let thread_email_ids = state
        .db_read({
            let eid = eid.clone();
            move |db| db.get_thread_email_ids(&eid).map_err(|e| e.to_string())
        })
        .await?;
    state.db_write(move |db| db.unarchive_email(&eid)).await?;

    // Move the same thread back to INBOX in Gmail
    if let Ok(token) = crate::google_api::get_valid_access_token().await {
        if let Err(e) = unarchive_emails_in_gmail(&token, &thread_email_ids).await {
            log::warn!("Gmail unarchive failed for {email_id}: {e} — unarchived locally only");
        }
    }

    Ok(())
}

async fn unarchive_emails_in_gmail(
    access_token: &str,
    message_ids: &[String],
) -> Result<(), String> {
    if message_ids.is_empty() {
        return Ok(());
    }

    let client = reqwest::Client::new();
    for chunk in message_ids.chunks(1000) {
        let body = serde_json::json!({
            "ids": chunk,
            "addLabelIds": ["INBOX"]
        });
        let response = client
            .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/batchModify")
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("{} {}", status, body));
        }
    }

    Ok(())
}

/// Toggle pin on an email. Returns the new pinned state.
pub fn pin_email(
    db: &crate::db::ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    email_id: &str,
) -> Result<bool, String> {
    let now_pinned = db.toggle_pin_email(email_id)?;
    if now_pinned {
        let (entity_type, entity_id) = email_entity_context(db, email_id);
        let _ = crate::services::signals::emit_and_propagate(
            db,
            engine,
            &entity_type,
            &entity_id,
            "email_pinned",
            "email_triage",
            Some(&format!(r#"{{"email_id":"{}"}}"#, email_id)),
            0.65,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(now_pinned)
}

// ── I580: Commitment -> Action promotion ──────────────────────────────

/// Parameters for promoting an email commitment to a tracked action.
#[derive(Debug)]
pub struct PromoteCommitmentParams<'a> {
    pub email_id: &'a str,
    pub commitment_text: &'a str,
    pub action_title: Option<&'a str>,
    pub entity_id: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub owner: Option<&'a str>,
    pub due_date: Option<&'a str>,
}

/// Promote an email commitment to a tracked action.
pub fn promote_commitment_to_action(
    db: &crate::db::ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    params: &PromoteCommitmentParams<'_>,
) -> Result<String, String> {
    let email_id = params.email_id;
    let commitment_text = params.commitment_text;
    let action_title = params.action_title;
    let entity_id = params.entity_id;
    let entity_type = params.entity_type;
    let now = chrono::Utc::now().to_rfc3339();
    let action_id = uuid::Uuid::new_v4().to_string();
    let trimmed_title = action_title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(commitment_text);
    let owner = params
        .owner
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .or_else(|| {
            db.conn_ref()
                .query_row(
                    "SELECT sender_name FROM emails WHERE email_id = ?1",
                    rusqlite::params![email_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });

    let (account_id, project_id) = match entity_type {
        Some("account") => (entity_id.map(|s| s.to_string()), None),
        Some("project") => (None, entity_id.map(|s| s.to_string())),
        _ => (None, None),
    };

    let action = crate::db::DbAction {
        id: action_id.clone(),
        title: trimmed_title.to_string(),
        priority: crate::action_status::PRIORITY_MEDIUM,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date: params
            .due_date
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|s| s.to_string()),
        completed_at: None,
        account_id,
        project_id,
        source_type: Some("email_commitment".to_string()),
        source_id: Some(email_id.to_string()),
        source_label: Some("Email commitment".to_string()),
        context: Some(build_email_commitment_context(
            owner.as_deref(),
            commitment_text,
        )),
        waiting_on: None,
        updated_at: now,
        person_id: None,
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    };

    db.upsert_action(&action).map_err(|e| e.to_string())?;

    let sig_entity_type = entity_type.unwrap_or("email");
    let sig_entity_id = entity_id.unwrap_or(email_id);
    let _ = crate::services::signals::emit_and_propagate(
        db,
        engine,
        sig_entity_type,
        sig_entity_id,
        "action_promoted_from_email",
        "email_commitment",
        Some(&format!(
            "{{\"action_id\":\"{}\",\"email_id\":\"{}\"}}",
            action_id, email_id
        )),
        0.7,
    );

    Ok(action_id)
}

/// Helper: resolve entity type and ID from an email for signal context.
fn email_entity_context(db: &crate::db::ActionDb, email_id: &str) -> (String, String) {
    let result: Option<(Option<String>, Option<String>)> = db
        .conn_ref()
        .query_row(
            "SELECT entity_type, entity_id FROM emails WHERE email_id = ?1",
            rusqlite::params![email_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    match result {
        Some((Some(etype), Some(eid))) => (etype, eid),
        _ => ("email".to_string(), email_id.to_string()),
    }
}

/// Dismiss a gone-quiet cadence alert for an account (I581).
pub fn dismiss_gone_quiet(
    db: &crate::db::ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    entity_id: &str,
) -> Result<(), String> {
    let _ = crate::services::signals::emit_and_propagate(
        db,
        engine,
        "account",
        entity_id,
        "email_cadence_drop_dismissed",
        "user_correction",
        Some(&format!(
            "{{\"entity_id\":\"{}\",\"action\":\"dismissed_gone_quiet\"}}",
            entity_id
        )),
        0.3,
    )
    .map_err(|e| e.to_string())?;

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

    // Only treat as "reappeared" if the email is truly new (not in our DB at all).
    // Emails that are in Gmail but resolved locally (user-archived via I579) should
    // NOT be unmarked — the user explicitly archived them. We query ALL known email
    // IDs (including resolved) to distinguish "genuinely new" from "user-archived but
    // still in Gmail inbox".
    let all_known_ids: HashSet<String> = db
        .conn_ref()
        .prepare("SELECT email_id FROM emails")
        .and_then(|mut stmt| {
            let ids = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(ids)
        })
        .unwrap_or_default();

    // Only IDs that Gmail has AND we've never seen before are truly new
    let genuinely_new: Vec<String> = inbox_ids
        .iter()
        .filter(|id| !all_known_ids.contains(*id))
        .cloned()
        .collect();

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

    // Don't unmark_resolved for known emails — they were user-archived.
    // Only wake poller for genuinely new messages we haven't seen.

    Ok(InboxPresenceReconcileResult {
        changed,
        reappeared_or_new_count: genuinely_new.len(),
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

    // Also mark as resolved in DB so they don't reappear on any page
    let ids_clone = ids.clone();
    let _ = state
        .db_write(move |db| db.mark_emails_resolved(&ids_clone))
        .await;

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

// ══════════════════════════════════════════════════════════════════════════════
// I652 Phase 3: EmailSnapshot batch helper
// ══════════════════════════════════════════════════════════════════════════════

/// Snapshot of email content for deduplication and change detection (I652).
/// Stores the state of an email when it was last enriched.
/// Used to determine if content has changed since last enrichment (e.g., new reply in thread).
/// Gate 0 compares current email content with this prior snapshot.
#[derive(Clone, Debug)]
pub struct EmailSnapshot {
    /// Optional snippet text from email body (used for content change detection)
    pub snippet: Option<String>,
    /// Subject line (used for content change detection)
    pub subject: Option<String>,
    /// Received date (for reference and context)
    pub received_at: chrono::DateTime<chrono::Utc>,
}

/// Load email snapshots in batch for content change detection (I652 Gate 0).
///
/// Prevents N+1 query pattern by loading all snapshots in a single SQL query.
/// Returns a HashMap mapping email_id to EmailSnapshot for content-change detection.
///
/// # Graceful handling
/// - Empty email_ids → empty HashMap
/// - Email not found → silently skipped (not an error)
/// - Partial matches → only present emails in result
///
/// # Arguments
/// * `db` - Database reference
/// * `_account_id` - Account ID (for potential future filtering; currently unused)
/// * `email_ids` - List of email IDs to snapshot
///
/// # Returns
/// HashMap<email_id, EmailSnapshot> for matched emails only
pub fn get_email_snapshots_for_content_check(
    db: &crate::db::ActionDb,
    _account_id: &str,
    email_ids: &[String],
) -> Result<HashMap<String, EmailSnapshot>, String> {
    if email_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Build placeholders for IN clause: ?, ?, ...
    let placeholders: Vec<String> = (1..=email_ids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT email_id, subject, snippet, received_at FROM emails WHERE email_id IN ({})",
        placeholders.join(", ")
    );

    let mut stmt = db
        .conn_ref()
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare snapshot query: {e}"))?;

    // Build parameter references for the IN clause
    let param_values: Vec<&dyn rusqlite::types::ToSql> = email_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_values.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| format!("Failed to query snapshots: {e}"))?;

    let mut snapshots = HashMap::new();
    for row in rows {
        let (email_id, subject, snippet, received_at_str) =
            row.map_err(|e| format!("Failed to read snapshot row: {e}"))?;

        // Parse received_at timestamp
        let received_at = if let Some(ref date_str) = received_at_str {
            // Try RFC3339, then other formats
            chrono::DateTime::parse_from_rfc3339(date_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .or_else(|_| {
                    chrono::DateTime::parse_from_rfc2822(date_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                })
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").map(|dt| {
                        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                    })
                })
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S").map(|dt| {
                        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                    })
                })
                .unwrap_or_else(|_| chrono::Utc::now())
        } else {
            chrono::Utc::now()
        };

        snapshots.insert(
            email_id,
            EmailSnapshot {
                snippet,
                subject,
                received_at,
            },
        );
    }

    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    // Helper to create a test database connection with sample emails table
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

        // Create emails table with minimal required columns for snapshots
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS emails (
                email_id TEXT PRIMARY KEY,
                subject TEXT,
                snippet TEXT,
                received_at TEXT
            )",
        )
        .expect("Failed to create test table");

        conn
    }

    #[test]
    fn test_empty_email_ids() {
        let _conn = setup_test_db();
        let db = crate::db::ActionDb::from_conn(&_conn);
        let result = get_email_snapshots_for_content_check(db, "account_123", &[]);
        assert!(result.is_ok());
        let snapshots = result.unwrap();
        assert!(snapshots.is_empty());
    }

    #[test]
    fn test_single_email_snapshot() {
        let conn = setup_test_db();

        // Insert a test email with received_at timestamp
        conn.execute(
            "INSERT INTO emails (email_id, subject, snippet, received_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![
                "email_1",
                "Test Subject",
                "Test snippet content",
                "2024-01-15T10:30:00Z"
            ],
        )
        .expect("Failed to insert test email");

        let db = crate::db::ActionDb::from_conn(&conn);
        let result =
            get_email_snapshots_for_content_check(db, "account_123", &["email_1".to_string()]);
        assert!(result.is_ok());
        let snapshots = result.unwrap();

        assert_eq!(snapshots.len(), 1);
        assert!(snapshots.contains_key("email_1"));

        let snapshot = &snapshots["email_1"];
        assert_eq!(snapshot.subject, Some("Test Subject".to_string()));
        assert_eq!(snapshot.snippet, Some("Test snippet content".to_string()));
    }

    #[test]
    fn test_multiple_emails_snapshot() {
        let conn = setup_test_db();

        // Insert multiple test emails
        for i in 1..=3 {
            conn.execute(
                "INSERT INTO emails (email_id, subject, snippet, received_at) VALUES (?, ?, ?, ?)",
                rusqlite::params![
                    format!("email_{i}"),
                    format!("Subject {i}"),
                    format!("Snippet {i}"),
                    "2024-01-15T10:30:00Z",
                ],
            )
            .expect("Failed to insert test email");
        }

        let email_ids: Vec<String> = vec![
            "email_1".to_string(),
            "email_2".to_string(),
            "email_3".to_string(),
        ];
        let db = crate::db::ActionDb::from_conn(&conn);
        let result = get_email_snapshots_for_content_check(db, "account_123", &email_ids);
        assert!(result.is_ok());
        let snapshots = result.unwrap();

        assert_eq!(snapshots.len(), 3);
        for i in 1..=3 {
            let key = format!("email_{i}");
            assert!(snapshots.contains_key(&key));
            let snapshot = &snapshots[&key];
            assert_eq!(snapshot.subject, Some(format!("Subject {i}")));
            assert_eq!(snapshot.snippet, Some(format!("Snippet {i}")));
        }
    }

    #[test]
    fn test_email_not_found() {
        let conn = setup_test_db();

        // Insert only one email
        conn.execute(
            "INSERT INTO emails (email_id, subject, snippet, received_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![
                "email_1",
                "Test Subject",
                "Test snippet",
                "2024-01-15T10:30:00Z"
            ],
        )
        .expect("Failed to insert test email");

        // Request two emails, only one exists
        let email_ids = vec!["email_1".to_string(), "email_missing".to_string()];
        let db = crate::db::ActionDb::from_conn(&conn);
        let result = get_email_snapshots_for_content_check(db, "account_123", &email_ids);
        assert!(result.is_ok());
        let snapshots = result.unwrap();

        // Only the found email should be in the result
        assert_eq!(snapshots.len(), 1);
        assert!(snapshots.contains_key("email_1"));
        assert!(!snapshots.contains_key("email_missing"));
    }

    #[test]
    fn test_order_independence() {
        let conn = setup_test_db();

        // Insert multiple emails
        for i in 1..=3 {
            conn.execute(
                "INSERT INTO emails (email_id, subject, snippet, received_at) VALUES (?, ?, ?, ?)",
                rusqlite::params![
                    format!("email_{i}"),
                    format!("Subject {i}"),
                    format!("Snippet {i}"),
                    "2024-01-15T10:30:00Z",
                ],
            )
            .expect("Failed to insert test email");
        }

        // Request emails in different order (HashMap doesn't guarantee order)
        let email_ids = vec![
            "email_3".to_string(),
            "email_1".to_string(),
            "email_2".to_string(),
        ];
        let db = crate::db::ActionDb::from_conn(&conn);
        let result = get_email_snapshots_for_content_check(db, "account_123", &email_ids);
        assert!(result.is_ok());
        let snapshots = result.unwrap();

        // All should be present regardless of request order
        assert_eq!(snapshots.len(), 3);
        assert!(snapshots.contains_key("email_1"));
        assert!(snapshots.contains_key("email_2"));
        assert!(snapshots.contains_key("email_3"));

        // Verify content correctness
        assert_eq!(snapshots["email_1"].subject, Some("Subject 1".to_string()));
        assert_eq!(snapshots["email_2"].subject, Some("Subject 2".to_string()));
        assert_eq!(snapshots["email_3"].subject, Some("Subject 3".to_string()));
    }

    #[test]
    fn test_null_fields_default_to_none() {
        let conn = setup_test_db();

        // Insert email with NULL subject and snippet
        conn.execute(
            "INSERT INTO emails (email_id, subject, snippet, received_at) VALUES (?, NULL, NULL, ?)",
            rusqlite::params!["email_1", "2024-01-15T10:30:00Z"],
        )
        .expect("Failed to insert test email with NULLs");

        let db = crate::db::ActionDb::from_conn(&conn);
        let result =
            get_email_snapshots_for_content_check(db, "account_123", &["email_1".to_string()]);
        assert!(result.is_ok());
        let snapshots = result.unwrap();

        assert_eq!(snapshots.len(), 1);
        let snapshot = &snapshots["email_1"];
        assert_eq!(snapshot.subject, None);
        assert_eq!(snapshot.snippet, None);
    }
}
