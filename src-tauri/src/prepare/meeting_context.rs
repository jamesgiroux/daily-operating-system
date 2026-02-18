//! Rich meeting context gathering (ported from ops/meeting_prep.py).
//!
//! Builds context bundles for each meeting that needs prep:
//! - Account dashboard data
//! - Recent meeting history (SQLite)
//! - Recent captures (wins/risks from post-meeting, I33)
//! - Open actions for account
//! - File references (account tracker, summaries, archive)

use std::collections::{HashMap, HashSet};
use std::path::Path;

use regex::Regex;
use serde_json::{json, Value};

/// Build rich context for all meetings that need prep.
///
/// Convenience wrapper over `gather_meeting_context()` for batch use.
pub fn gather_all_meeting_contexts(
    classified: &[Value],
    workspace: &Path,
    db: Option<&crate::db::ActionDb>,
    embedding_model: Option<&crate::embeddings::EmbeddingModel>,
) -> Vec<Value> {
    let mut contexts = Vec::new();
    for meeting in classified {
        let meeting_type = meeting.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if meeting_type == "personal" || meeting_type == "all_hands" {
            continue;
        }
        contexts.push(gather_meeting_context(meeting, workspace, db, embedding_model));
    }
    contexts
}

/// Build rich context for a single meeting prep.
fn gather_meeting_context(
    meeting: &Value,
    workspace: &Path,
    db: Option<&crate::db::ActionDb>,
    embedding_model: Option<&crate::embeddings::EmbeddingModel>,
) -> Value {
    let meeting_type = meeting.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let event_id = meeting.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = meeting
        .get("title")
        .or_else(|| meeting.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let start = meeting.get("start").and_then(|v| v.as_str()).unwrap_or("");

    let description = meeting
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut ctx = json!({
        "event_id": event_id,
        "title": title,
        "start": start,
        "type": meeting_type,
        "refs": {},
    });
    if !description.is_empty() {
        ctx["description"] = json!(description);
    }

    // Skip meetings that don't benefit from prep
    if meeting_type == "personal" || meeting_type == "all_hands" {
        return ctx;
    }

    let accounts_dir = workspace.join("Accounts");
    let archive_dir = workspace.join("_archive");

    match meeting_type {
        "customer" | "qbr" | "training" => {
            if accounts_dir.is_dir() {
                // I305: Use confidence-scored entity resolver, then heuristic fallback.
                let resolver_result = db.and_then(|db| {
                    super::entity_resolver::resolve_account_compat(
                        db,
                        event_id,
                        meeting,
                        &accounts_dir,
                        embedding_model,
                    )
                });
                // Inject resolution metadata for downstream use
                if let Some(ref r) = resolver_result {
                    ctx["resolution_confidence"] = json!(r.confidence);
                    ctx["resolution_source"] = json!(&r.source);
                }
                let matched = resolver_result
                    .map(|r| AccountMatch {
                        name: r.name,
                        relative_path: r.relative_path,
                    })
                    .or_else(|| guess_account_name(meeting, &accounts_dir));
                if let Some(matched) = matched {
                    ctx["account"] = json!(&matched.name);
                    let account_path = accounts_dir.join(&matched.relative_path);

                    // File references
                    if let Some(dashboard) = find_file_in_dir(&account_path, "dashboard.md") {
                        ctx["refs"]["account_dashboard"] = json!(dashboard.to_string_lossy());

                        // Dashboard data extraction (I33)
                        if let Some(data) = parse_dashboard(&dashboard) {
                            ctx["account_data"] = data;
                        }
                    }

                    if let Some(stakeholders) = find_file_in_dir(&account_path, "stakeholders.md") {
                        ctx["refs"]["stakeholder_map"] = json!(stakeholders.to_string_lossy());
                    }

                    if let Some(actions_file) = find_file_in_dir(&account_path, "actions.md") {
                        ctx["refs"]["account_actions"] = json!(actions_file.to_string_lossy());
                    }

                    let recent = find_recent_summaries(&matched.name, &archive_dir, 2);
                    if !recent.is_empty() {
                        ctx["refs"]["meeting_history"] = json!(recent
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect::<Vec<_>>());
                    }

                    // SQLite enrichment
                    if let Some(db) = db {
                        ctx["recent_captures"] = get_captures_for_account(db, &matched.name, 14);
                        ctx["open_actions"] = get_account_actions(db, &matched.name);
                        ctx["meeting_history"] = get_meeting_history(db, &matched.name, 30, 3);
                        if let Ok(Some(acct)) = db.get_account_by_name(&matched.name) {
                            ctx["entity_id"] = json!(acct.id);
                            if let Ok(team) = db.get_account_team(&acct.id) {
                                if !team.is_empty() {
                                    ctx["account_team"] = json!(team
                                        .iter()
                                        .map(|m| {
                                            json!({
                                                "personId": m.person_id,
                                                "name": m.person_name,
                                                "email": m.person_email,
                                                "role": m.role,
                                            })
                                        })
                                        .collect::<Vec<_>>());
                                }
                            }
                        }
                    }

                    // I135: Persistent entity prep from intelligence.json
                    inject_entity_intelligence(&account_path, &mut ctx);
                }
            }
        }

        "external" => {
            let unknown_domains: Vec<String> = meeting
                .get("external_domains")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            if !unknown_domains.is_empty() {
                ctx["unknown_domains"] = json!(unknown_domains);
                for domain in unknown_domains.iter().take(3) {
                    let mentions = search_archive(domain, &archive_dir, 3);
                    if !mentions.is_empty() {
                        ctx["refs"][format!("archive_{}", domain)] = json!(mentions
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect::<Vec<_>>());
                    }
                }
            }
        }

        "internal" | "team_sync" => {
            if let Some(db) = db {
                if let Some(internal_account) = resolve_internal_account_for_meeting(
                    db,
                    event_id,
                    title,
                    meeting.get("attendees"),
                ) {
                    ctx["account"] = json!(internal_account.name.clone());
                    ctx["entity_id"] = json!(internal_account.id.clone());
                    let account_path =
                        crate::accounts::resolve_account_dir(workspace, &internal_account);
                    if let Some(dashboard) = find_file_in_dir(&account_path, "dashboard.md") {
                        ctx["refs"]["account_dashboard"] = json!(dashboard.to_string_lossy());
                    }
                    if let Some(actions_file) = find_file_in_dir(&account_path, "actions.md") {
                        ctx["refs"]["account_actions"] = json!(actions_file.to_string_lossy());
                    }
                    ctx["open_actions"] = get_account_actions(db, &internal_account.id);
                    ctx["meeting_history"] = get_meeting_history(db, &internal_account.id, 30, 3);
                }
            }
            if !title.is_empty() {
                let recent = find_recent_summaries(title, &archive_dir, 1);
                if !recent.is_empty() {
                    ctx["refs"]["last_meeting"] = json!(recent[0].to_string_lossy());
                }
            }

            if let Some(db) = db {
                if !title.is_empty() {
                    ctx["meeting_history"] = get_meeting_history_by_title(db, title, 30, 2);
                    ctx["recent_captures"] = get_captures_by_meeting_title(db, title, 14);
                }
                ctx["open_actions"] = get_all_pending_actions(db, 10);
            }
        }

        "one_on_one" => {
            if let Some(db) = db {
                if let Some(internal_account) = resolve_internal_account_for_meeting(
                    db,
                    event_id,
                    title,
                    meeting.get("attendees"),
                ) {
                    ctx["account"] = json!(internal_account.name.clone());
                    ctx["entity_id"] = json!(internal_account.id.clone());
                    let account_path =
                        crate::accounts::resolve_account_dir(workspace, &internal_account);
                    if let Some(dashboard) = find_file_in_dir(&account_path, "dashboard.md") {
                        ctx["refs"]["account_dashboard"] = json!(dashboard.to_string_lossy());
                    }
                    ctx["open_actions"] = get_account_actions(db, &internal_account.id);
                }
            }
            if !title.is_empty() {
                let recent = find_recent_summaries(title, &archive_dir, 3);
                if !recent.is_empty() {
                    ctx["refs"]["recent_meetings"] = json!(recent
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>());
                }
            }

            if let Some(db) = db {
                if !title.is_empty() {
                    ctx["meeting_history"] = get_meeting_history_by_title(db, title, 60, 3);
                    ctx["recent_captures"] = get_captures_by_meeting_title(db, title, 30);
                }
                ctx["open_actions"] = get_all_pending_actions(db, 10);
            }
        }

        "partnership" => {
            if accounts_dir.is_dir() {
                // I305: Use confidence-scored entity resolver, then heuristic fallback.
                let resolver_result = db.and_then(|db| {
                    super::entity_resolver::resolve_account_compat(
                        db,
                        event_id,
                        meeting,
                        &accounts_dir,
                        embedding_model,
                    )
                });
                if let Some(ref r) = resolver_result {
                    ctx["resolution_confidence"] = json!(r.confidence);
                    ctx["resolution_source"] = json!(&r.source);
                }
                let matched = resolver_result
                    .map(|r| AccountMatch {
                        name: r.name,
                        relative_path: r.relative_path,
                    })
                    .or_else(|| guess_account_name(meeting, &accounts_dir));
                if let Some(matched) = matched {
                    ctx["account"] = json!(&matched.name);
                    let account_path = accounts_dir.join(&matched.relative_path);
                    for fname in &["dashboard.md", "stakeholders.md", "actions.md"] {
                        if let Some(found) = find_file_in_dir(&account_path, fname) {
                            let key = fname.replace(".md", "");
                            ctx["refs"][key] = json!(found.to_string_lossy());
                        }
                    }

                    if let Some(db) = db {
                        ctx["recent_captures"] = get_captures_for_account(db, &matched.name, 14);
                        ctx["open_actions"] = get_account_actions(db, &matched.name);
                        ctx["meeting_history"] = get_meeting_history(db, &matched.name, 30, 3);
                        if let Ok(Some(acct)) = db.get_account_by_name(&matched.name) {
                            ctx["entity_id"] = json!(acct.id);
                            if let Ok(team) = db.get_account_team(&acct.id) {
                                if !team.is_empty() {
                                    ctx["account_team"] = json!(team
                                        .iter()
                                        .map(|m| {
                                            json!({
                                                "personId": m.person_id,
                                                "name": m.person_name,
                                                "email": m.person_email,
                                                "role": m.role,
                                            })
                                        })
                                        .collect::<Vec<_>>());
                                }
                            }
                        }
                    }

                    // I135: Persistent entity prep from intelligence.json
                    inject_entity_intelligence(&account_path, &mut ctx);
                }
            }

            if !title.is_empty() {
                let recent = find_recent_summaries(title, &archive_dir, 2);
                if !recent.is_empty() {
                    ctx["refs"]["recent_meetings"] = json!(recent
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>());
                }

                if let Some(db) = db {
                    if ctx.get("account").is_none()
                        || ctx["account"].as_str().unwrap_or("").is_empty()
                    {
                        ctx["meeting_history"] = get_meeting_history_by_title(db, title, 30, 3);
                        ctx["recent_captures"] = get_captures_by_meeting_title(db, title, 14);
                    }
                }
            }
        }

        _ => {}
    }

    if let Some(db) = db {
        if let Some(entity_id) = ctx
            .get("entity_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            inject_recent_email_signals(db, &entity_id, &mut ctx);
        }
    }

    ctx
}

// ---------------------------------------------------------------------------
// Entity intelligence injection (I135)
// ---------------------------------------------------------------------------

/// Read intelligence.json from an entity directory and inject relevant
/// fields into the meeting context for prep enrichment.
fn inject_entity_intelligence(entity_dir: &Path, ctx: &mut Value) {
    let intel = match crate::entity_intel::read_intelligence_json(entity_dir) {
        Ok(intel) => intel,
        Err(_) => return,
    };

    if let Some(ref assessment) = intel.executive_assessment {
        ctx["executive_assessment"] = json!(assessment);
    }

    if !intel.risks.is_empty() {
        ctx["entity_risks"] = json!(intel
            .risks
            .iter()
            .map(|r| {
                json!({
                    "text": r.text,
                    "urgency": r.urgency,
                    "source": r.source,
                })
            })
            .collect::<Vec<_>>());
    }

    if let Some(ref readiness) = intel.next_meeting_readiness {
        if !readiness.prep_items.is_empty() {
            ctx["entity_readiness"] = json!(&readiness.prep_items);
        }
    }

    if !intel.stakeholder_insights.is_empty() {
        ctx["stakeholder_insights"] = json!(intel
            .stakeholder_insights
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "role": s.role,
                    "assessment": s.assessment,
                    "engagement": s.engagement,
                })
            })
            .collect::<Vec<_>>());
    }
}

fn inject_recent_email_signals(db: &crate::db::ActionDb, entity_id: &str, ctx: &mut Value) {
    if let Ok(signals) = db.list_recent_email_signals_for_entity(entity_id, 8) {
        if !signals.is_empty() {
            ctx["recent_email_signals"] = json!(signals);
        }
    }
}

// ---------------------------------------------------------------------------
// File search helpers
// ---------------------------------------------------------------------------

/// Resolve an account from the database when filesystem matching fails (I168).
///
/// Two-step resolution:
/// 1. Direct: check `meeting_entities` junction for this meeting's primary ID
/// 2. Attendee inference: look up meeting attendees → person → entity links, majority vote
fn resolve_account_from_db(
    db: &crate::db::ActionDb,
    event_id: &str,
    meeting: &Value,
    accounts_dir: &Path,
) -> Option<AccountMatch> {
    let meeting_type = meeting
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let internal_meeting = matches!(
        meeting_type.as_str(),
        "internal" | "team_sync" | "one_on_one"
    );
    let meeting_id = crate::workflow::deliver::meeting_primary_id(Some(event_id), "", "", "");
    let title_lower = meeting
        .get("title")
        .or_else(|| meeting.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let external_domains: Vec<String> = meeting
        .get("external_domains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                .collect()
        })
        .unwrap_or_default();

    // Load meeting row by calendar_event_id when available.
    let meeting_row = if event_id.is_empty() {
        None
    } else {
        db.get_meeting_by_calendar_event_id(event_id).ok().flatten()
    };

    // Step 0: Explicit account assignment on meetings_history is highest-confidence.
    if let Some(ref row) = meeting_row {
        if let Some(ref account_id) = row.account_id {
            if !internal_meeting {
                if let Ok(Some(account)) = db.get_account(account_id) {
                    if account.is_internal {
                        return None;
                    }
                }
            }
            if let Some(matched) = resolve_account_identifier(db, account_id, accounts_dir) {
                return Some(matched);
            }
        }
    }

    // Step 1: Direct meeting_entities junction lookup (meeting id and calendar-backed id).
    let mut direct_candidates: Vec<AccountMatch> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for lookup_id in [
        (!meeting_id.is_empty()).then_some(meeting_id.as_str()),
        meeting_row
            .as_ref()
            .and_then(|m| (!m.id.is_empty() && m.id != meeting_id).then_some(m.id.as_str())),
    ]
    .into_iter()
    .flatten()
    {
        if let Ok(entities) = db.get_meeting_entities(lookup_id) {
            for entity in entities {
                if entity.entity_type != crate::entity::EntityType::Account {
                    continue;
                }
                if !internal_meeting {
                    if let Ok(Some(account)) = db.get_account(&entity.id) {
                        if account.is_internal {
                            continue;
                        }
                    }
                }
                if let Some(matched) = find_account_dir_by_id_hint(&entity.id, accounts_dir)
                    .or_else(|| find_account_dir_by_name(&entity.name, accounts_dir))
                {
                    let key = normalize_account_key(&matched.name);
                    if seen.insert(key) {
                        direct_candidates.push(matched);
                    }
                }
            }
        }
    }

    let attendee_votes = meeting_row
        .as_ref()
        .and_then(|row| row.attendees.as_deref())
        .map(|attendees| build_attendee_account_votes(db, attendees))
        .unwrap_or_default();

    if !direct_candidates.is_empty() {
        if direct_candidates.len() == 1 {
            return direct_candidates.into_iter().next();
        }

        // Multiple linked accounts: use deterministic scoring.
        let mut best: Option<(i32, String, AccountMatch)> = None;
        for candidate in direct_candidates {
            let mut score = 0;
            if matches_meeting(&candidate.name, &title_lower, &external_domains) {
                score += 100;
            }
            if candidate.relative_path.contains('/') {
                score += 5;
            }
            score += attendee_votes
                .get(&normalize_account_key(&candidate.name))
                .copied()
                .unwrap_or(0) as i32
                * 10;

            let tie_name = candidate.name.to_lowercase();
            let should_replace = match &best {
                None => true,
                Some((best_score, best_name, _)) => {
                    score > *best_score || (score == *best_score && tie_name < *best_name)
                }
            };
            if should_replace {
                best = Some((score, tie_name, candidate));
            }
        }

        if let Some((_, _, matched)) = best {
            return Some(matched);
        }
    }

    // Step 2: Attendee inference fallback (majority vote from person↔entity links).
    if let Some((top_key, _)) = attendee_votes.into_iter().max_by_key(|(_, c)| *c) {
        if let Some(matched) = find_account_dir_by_name(&top_key, accounts_dir) {
            return Some(matched);
        }
    }

    None
}

/// Resolve an account identifier that may be an entity/account ID or name.
fn resolve_account_identifier(
    db: &crate::db::ActionDb,
    account_ref: &str,
    accounts_dir: &Path,
) -> Option<AccountMatch> {
    let trimmed = account_ref.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Prefer path resolution from slug-like IDs (e.g., "salesforce--digital-marketing-technology")
    // to disambiguate duplicate names across top-level and BU child folders.
    if let Some(matched) = find_account_dir_by_id_hint(trimmed, accounts_dir) {
        return Some(matched);
    }

    if let Ok(Some(entity)) = db.get_entity(trimmed) {
        if entity.entity_type == crate::entity::EntityType::Account {
            if let Some(matched) = find_account_dir_by_name(&entity.name, accounts_dir) {
                return Some(matched);
            }
        }
    }

    if let Ok(Some(account)) = db.get_account(trimmed) {
        if let Some(matched) = find_account_dir_by_name(&account.name, accounts_dir) {
            return Some(matched);
        }
    }

    find_account_dir_by_name(trimmed, accounts_dir)
}

/// Try resolving an account from an ID hint that encodes parent/child slugs.
///
/// Example: `salesforce--digital-marketing-technology` -> `Salesforce/Digital-Marketing-Technology`.
fn find_account_dir_by_id_hint(account_ref: &str, accounts_dir: &Path) -> Option<AccountMatch> {
    let (parent_hint, child_hint) = account_ref.split_once("--")?;
    let parent_key = normalize_account_key(parent_hint);
    let child_key = normalize_account_key(child_hint);
    if parent_key.is_empty() || child_key.is_empty() {
        return None;
    }

    let entries = std::fs::read_dir(accounts_dir).ok()?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }
        let parent_name = entry.file_name().to_string_lossy().to_string();
        if normalize_account_key(&parent_name) != parent_key {
            continue;
        }
        if let Ok(children) = std::fs::read_dir(entry.path()) {
            for child in children.flatten() {
                if !child.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    continue;
                }
                let child_name = child.file_name().to_string_lossy().to_string();
                if normalize_account_key(&child_name) == child_key {
                    return Some(AccountMatch {
                        name: child_name.clone(),
                        relative_path: format!("{}/{}", parent_name, child_name),
                    });
                }
            }
        }
    }
    None
}

/// Compute attendee-based account votes keyed by normalized account name.
fn build_attendee_account_votes(
    db: &crate::db::ActionDb,
    attendees_csv: &str,
) -> HashMap<String, usize> {
    let emails: Vec<&str> = attendees_csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let mut votes: HashMap<String, usize> = HashMap::new();

    for email in emails {
        if let Ok(Some(person)) = db.get_person_by_email_or_alias(email) {
            if let Ok(entities) = db.get_entities_for_person(&person.id) {
                for entity in entities {
                    if entity.entity_type == crate::entity::EntityType::Account {
                        let key = normalize_account_key(&entity.name);
                        if !key.is_empty() {
                            *votes.entry(key).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    votes
}

/// Normalize account-like labels for fuzzy matching.
///
/// Examples:
/// - "Digital-Marketing-Technology" -> "digitalmarketingtechnology"
/// - "Digital Marketing Technology" -> "digitalmarketingtechnology"
fn normalize_account_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn attendee_emails_from_value(attendees: Option<&Value>) -> HashSet<String> {
    attendees
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_lowercase())
                .filter(|s| s.contains('@'))
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

fn resolve_internal_account_for_meeting(
    db: &crate::db::ActionDb,
    event_id: &str,
    title: &str,
    attendees: Option<&Value>,
) -> Option<crate::db::DbAccount> {
    if !event_id.is_empty() {
        if let Ok(Some(row)) = db.get_meeting_by_calendar_event_id(event_id) {
            if let Some(ref account_id) = row.account_id {
                if let Ok(Some(account)) = db.get_account(account_id) {
                    if account.is_internal && !account.archived {
                        return Some(account);
                    }
                }
            }
            if let Ok(entities) = db.get_meeting_entities(&row.id) {
                for entity in entities {
                    if entity.entity_type != crate::entity::EntityType::Account {
                        continue;
                    }
                    if let Ok(Some(account)) = db.get_account(&entity.id) {
                        if account.is_internal && !account.archived {
                            return Some(account);
                        }
                    }
                }
            }
        }
    }

    let internal_accounts = db.get_internal_accounts().ok()?;
    if internal_accounts.is_empty() {
        return None;
    }
    let root = internal_accounts
        .iter()
        .find(|a| a.parent_id.is_none())
        .cloned();
    let candidate_accounts: Vec<crate::db::DbAccount> = internal_accounts
        .iter()
        .filter(|a| a.parent_id.is_some())
        .cloned()
        .collect();
    if candidate_accounts.is_empty() {
        return root;
    }

    let title_key = normalize_account_key(title);
    let attendee_emails = attendee_emails_from_value(attendees);

    let mut best: Option<(i32, crate::db::DbAccount)> = None;
    for account in candidate_accounts {
        let mut score = 0i32;
        let account_key = normalize_account_key(&account.name);
        if !account_key.is_empty() && title_key.contains(&account_key) {
            score += 2;
        }

        let overlap = db
            .get_people_for_entity(&account.id)
            .unwrap_or_default()
            .iter()
            .filter(|p| attendee_emails.contains(&p.email.to_lowercase()))
            .count() as i32;
        score += overlap * 3;

        match &best {
            None => best = Some((score, account)),
            Some((best_score, best_account)) => {
                if score > *best_score
                    || (score == *best_score
                        && account.name.to_lowercase() < best_account.name.to_lowercase())
                {
                    best = Some((score, account));
                }
            }
        }
    }

    match best {
        Some((score, account)) if score > 0 => Some(account),
        _ => root,
    }
}

/// Find an account directory by name (exact, case-insensitive match).
/// Checks both top-level and child BU directories.
fn find_account_dir_by_name(name: &str, accounts_dir: &Path) -> Option<AccountMatch> {
    let target_key = normalize_account_key(name);
    let mut matches: Vec<AccountMatch> = Vec::new();

    let entries = std::fs::read_dir(accounts_dir).ok()?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }
        let parent_name = entry.file_name().to_string_lossy().to_string();
        if let Ok(children) = std::fs::read_dir(entry.path()) {
            for child in children.flatten() {
                let child_name = child.file_name().to_string_lossy().to_string();
                if child.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                    && normalize_account_key(&child_name) == target_key
                {
                    matches.push(AccountMatch {
                        name: child_name.clone(),
                        relative_path: format!("{}/{}", parent_name, child_name),
                    });
                }
            }
        }
    }

    let entries = std::fs::read_dir(accounts_dir).ok()?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if normalize_account_key(&dir_name) == target_key {
            matches.push(AccountMatch {
                name: dir_name.clone(),
                relative_path: dir_name,
            });
        }
    }

    if matches.is_empty() {
        return None;
    }
    if matches.len() == 1 {
        return matches.into_iter().next();
    }

    // Guardrail: if multiple folders match the same normalized name, choose
    // deterministically with a BU/context bias and log the ambiguity.
    let mut best_idx = 0usize;
    let mut best_score = i32::MIN;
    for (idx, m) in matches.iter().enumerate() {
        let path = accounts_dir.join(&m.relative_path);
        let mut score = 0;
        if m.relative_path.contains('/') {
            score += 10; // BU child paths are usually more specific.
        }
        if path.join("intelligence.json").is_file() {
            score += 5;
        }
        if path.join("dashboard.md").is_file() {
            score += 3;
        }
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }

    let chosen = matches.remove(best_idx);
    let all_paths = matches
        .iter()
        .map(|m| m.relative_path.clone())
        .chain(std::iter::once(chosen.relative_path.clone()))
        .collect::<Vec<_>>();
    log::warn!(
        "account resolution ambiguity for '{}': matches={:?}; chosen='{}'",
        name,
        all_paths,
        chosen.relative_path
    );
    Some(chosen)
}

/// Result of matching a meeting to an account directory.
pub(crate) struct AccountMatch {
    /// Display name (e.g., "Consumer-Brands" for a child, "Cox" for a parent).
    pub(crate) name: String,
    /// Relative path from Accounts/ dir (e.g., "Cox/Consumer-Brands" for a child, "Cox" for flat).
    pub(crate) relative_path: String,
}

/// Try to match a meeting to a known account directory.
///
/// Performs a two-level scan: first checks top-level account directories,
/// then checks child BU subdirectories within each parent (using `is_bu_directory`).
/// Child matches are preferred over parent matches when both exist, since child BU
/// meetings should reference the BU-specific context files.
fn guess_account_name(meeting: &Value, accounts_dir: &Path) -> Option<AccountMatch> {
    if !accounts_dir.is_dir() {
        return None;
    }

    let title_lower = meeting
        .get("title")
        .or_else(|| meeting.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let external_domains: Vec<String> = meeting
        .get("external_domains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                .collect()
        })
        .unwrap_or_default();

    let top_level_dirs: Vec<String> = std::fs::read_dir(accounts_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    // Check child BU directories first (more specific match wins)
    for parent_name in &top_level_dirs {
        let parent_path = accounts_dir.join(parent_name);
        if let Ok(children) = std::fs::read_dir(&parent_path) {
            for entry in children.filter_map(|e| e.ok()) {
                let child_name = entry.file_name().to_string_lossy().to_string();
                if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    continue;
                }
                if !crate::accounts::is_bu_directory(&child_name) {
                    continue;
                }
                if matches_meeting(&child_name, &title_lower, &external_domains) {
                    return Some(AccountMatch {
                        name: child_name.clone(),
                        relative_path: format!("{parent_name}/{child_name}"),
                    });
                }
            }
        }
    }

    // Fall back to top-level account match
    for name in &top_level_dirs {
        if matches_meeting(name, &title_lower, &external_domains) {
            return Some(AccountMatch {
                name: name.clone(),
                relative_path: name.clone(),
            });
        }
    }

    None
}

/// Check if an account/BU name matches a meeting by title or external domain.
fn matches_meeting(name: &str, title_lower: &str, external_domains: &[String]) -> bool {
    let name_key = normalize_account_key(name);
    let title_key = normalize_account_key(title_lower);
    if !name_key.is_empty() && title_key.contains(&name_key) {
        return true;
    }
    for domain in external_domains {
        let domain_base = domain.split('.').next().unwrap_or("");
        let domain_key = normalize_account_key(domain_base);
        if !domain_key.is_empty() && (domain_key == name_key || name_key.contains(&domain_key)) {
            return true;
        }
    }
    false
}

/// Find a file by name in a directory (case-insensitive).
fn find_file_in_dir(directory: &Path, filename: &str) -> Option<std::path::PathBuf> {
    if !directory.is_dir() {
        return None;
    }

    let exact = directory.join(filename);
    if exact.exists() {
        return Some(exact);
    }

    let target_lower = filename.to_lowercase();
    if let Ok(entries) = std::fs::read_dir(directory) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && entry.file_name().to_string_lossy().to_lowercase() == target_lower
            {
                return Some(entry.path());
            }
        }
    }

    // Search one level of subdirectories
    if let Ok(entries) = std::fs::read_dir(directory) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                && !name_str.starts_with('.')
                && !name_str.starts_with('_')
            {
                if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                    for sub in sub_entries.flatten() {
                        if sub.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                            && sub
                                .file_name()
                                .to_string_lossy()
                                .to_lowercase()
                                .ends_with(&target_lower)
                        {
                            return Some(sub.path());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Find recent meeting summaries mentioning a search term in the archive.
fn find_recent_summaries(
    search_term: &str,
    archive_dir: &Path,
    limit: usize,
) -> Vec<std::path::PathBuf> {
    if !archive_dir.is_dir() {
        return Vec::new();
    }

    let search_lower = search_term.to_lowercase();
    let search_slug: String = search_lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    let mut matches: Vec<(std::time::SystemTime, std::path::PathBuf)> = Vec::new();

    let mut date_dirs: Vec<_> = std::fs::read_dir(archive_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    date_dirs.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
    date_dirs.truncate(30);

    for date_dir in date_dirs {
        if let Ok(files) = std::fs::read_dir(date_dir.path()) {
            for f in files.flatten() {
                if !f.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    continue;
                }
                let fname = f.file_name();
                let fname_str = fname.to_string_lossy();
                if !fname_str.ends_with(".md") {
                    continue;
                }
                let fname_lower = fname_str.to_lowercase();
                if fname_lower.contains(&search_lower) || fname_lower.contains(&search_slug) {
                    if let Ok(meta) = f.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            matches.push((mtime, f.path()));
                        }
                    }
                }
            }
        }
    }

    matches.sort_by(|a, b| b.0.cmp(&a.0));
    matches.into_iter().take(limit).map(|(_, p)| p).collect()
}

/// Search recent archive files for content matching a query.
fn search_archive(query: &str, archive_dir: &Path, max_results: usize) -> Vec<std::path::PathBuf> {
    if !archive_dir.is_dir() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    let mut date_dirs: Vec<_> = std::fs::read_dir(archive_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    date_dirs.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
    date_dirs.truncate(14);

    for date_dir in date_dirs {
        if matches.len() >= max_results {
            break;
        }
        if let Ok(files) = std::fs::read_dir(date_dir.path()) {
            for f in files.flatten() {
                if matches.len() >= max_results {
                    break;
                }
                let fname = f.file_name();
                if !fname.to_string_lossy().ends_with(".md") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(f.path()) {
                    if content.to_lowercase().contains(&query_lower) {
                        matches.push(f.path());
                    }
                }
            }
        }
    }

    matches
}

// ---------------------------------------------------------------------------
// Dashboard parsing (I33)
// ---------------------------------------------------------------------------

/// Best-effort extraction of Quick View data from account dashboard markdown.
fn parse_dashboard(dashboard_path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(dashboard_path).ok()?;
    let mut data = serde_json::Map::new();

    let patterns = [
        (
            r"(?i)(?:ARR|Annual Revenue|MRR)\s*[:\|]\s*\$?([\d,\.]+[KMB]?)",
            "arr",
        ),
        (r"(?i)(?:Health\s*(?:Score)?)\s*[:\|]\s*(\w+)", "health"),
        (
            r"(?i)(?:Renewal\s*(?:Date)?)\s*[:\|]\s*([\d\-/]+)",
            "renewal",
        ),
        (
            r"(?i)(?:Lifecycle|Stage)\s*[:\|]\s*(.+?)(?:\n|\|)",
            "lifecycle",
        ),
        (
            r"(?i)(?:CSM|Account Manager)\s*[:\|]\s*(.+?)(?:\n|\|)",
            "csm",
        ),
    ];

    for (pattern, key) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(&content) {
                if let Some(m) = caps.get(1) {
                    data.insert(key.to_string(), json!(m.as_str().trim()));
                }
            }
        }
    }

    // Extract Recent Wins section
    let wins = extract_section_items(&content, "Recent Wins");
    if !wins.is_empty() {
        data.insert(
            "recent_wins".to_string(),
            json!(wins.into_iter().take(5).collect::<Vec<_>>()),
        );
    }

    // Extract Current Risks section
    let risks = extract_section_items(&content, "Current Risks");
    if !risks.is_empty() {
        data.insert(
            "current_risks".to_string(),
            json!(risks.into_iter().take(5).collect::<Vec<_>>()),
        );
    }

    if data.is_empty() {
        None
    } else {
        Some(Value::Object(data))
    }
}

/// Extract bullet items from a markdown section.
fn extract_section_items(content: &str, section_name: &str) -> Vec<String> {
    let pattern = format!(
        r"(?i)#+\s*{}.*?\n([\s\S]*?)(?:\n#|\z)",
        regex::escape(section_name)
    );
    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let caps = match re.captures(content) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let section_text = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    section_text
        .lines()
        .filter_map(|line| {
            let stripped = line.trim();
            if stripped.starts_with("- ")
                || stripped.starts_with("* ")
                || stripped.starts_with("• ")
            {
                let item = stripped.trim_start_matches(['-', '*', '•', ' ']).trim();
                if !item.is_empty() {
                    Some(item.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// SQLite query helpers
// ---------------------------------------------------------------------------

fn get_captures_for_account(db: &crate::db::ActionDb, account_id: &str, days_back: i64) -> Value {
    let result: Vec<Value> = (|| {
        let conn = db.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, meeting_id, meeting_title, capture_type, content, captured_at
             FROM captures
             WHERE account_id = ?1
               AND captured_at >= date('now', ?2)
             ORDER BY captured_at DESC",
            )
            .ok()?;
        let rows = stmt
            .query_map(
                rusqlite::params![account_id, format!("-{} days", days_back)],
                |row: &rusqlite::Row| {
                    Ok(json!({
                        "id": row.get::<_, Option<String>>(0)?,
                        "meeting_title": row.get::<_, Option<String>>(2)?,
                        "type": row.get::<_, Option<String>>(3)?,
                        "content": row.get::<_, Option<String>>(4)?,
                        "captured_at": row.get::<_, Option<String>>(5)?,
                    }))
                },
            )
            .ok()?;
        Some(rows.flatten().collect())
    })()
    .unwrap_or_default();
    json!(result)
}

fn get_account_actions(db: &crate::db::ActionDb, account_id: &str) -> Value {
    let result: Vec<Value> = (|| {
        let conn = db.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, priority, status, due_date
             FROM actions
             WHERE account_id = ?1
               AND status IN ('pending', 'waiting')
             ORDER BY priority, due_date",
            )
            .ok()?;
        let rows = stmt
            .query_map([account_id], |row: &rusqlite::Row| {
                Ok(json!({
                    "id": row.get::<_, Option<String>>(0)?,
                    "title": row.get::<_, Option<String>>(1)?,
                    "priority": row.get::<_, Option<String>>(2)?,
                    "status": row.get::<_, Option<String>>(3)?,
                    "due_date": row.get::<_, Option<String>>(4)?,
                }))
            })
            .ok()?;
        Some(rows.flatten().collect())
    })()
    .unwrap_or_default();
    json!(result)
}

fn get_meeting_history(
    db: &crate::db::ActionDb,
    account_id: &str,
    lookback_days: i64,
    limit: usize,
) -> Value {
    let result: Vec<Value> = (|| {
        let conn = db.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, meeting_type, start_time, summary
             FROM meetings_history
             WHERE account_id = ?1
               AND start_time >= date('now', ?2)
             ORDER BY start_time DESC
             LIMIT ?3",
            )
            .ok()?;
        let rows = stmt
            .query_map(
                rusqlite::params![account_id, format!("-{} days", lookback_days), limit as i64],
                |row: &rusqlite::Row| {
                    Ok(json!({
                        "id": row.get::<_, Option<String>>(0)?,
                        "title": row.get::<_, Option<String>>(1)?,
                        "type": row.get::<_, Option<String>>(2)?,
                        "start_time": row.get::<_, Option<String>>(3)?,
                        "summary": row.get::<_, Option<String>>(4)?,
                    }))
                },
            )
            .ok()?;
        Some(rows.flatten().collect())
    })()
    .unwrap_or_default();
    json!(result)
}

fn get_meeting_history_by_title(
    db: &crate::db::ActionDb,
    title: &str,
    lookback_days: i64,
    limit: usize,
) -> Value {
    let result: Vec<Value> = (|| {
        let conn = db.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, meeting_type, start_time, summary
             FROM meetings_history
             WHERE LOWER(title) = LOWER(?1)
               AND start_time >= date('now', ?2)
             ORDER BY start_time DESC
             LIMIT ?3",
            )
            .ok()?;
        let rows = stmt
            .query_map(
                rusqlite::params![title, format!("-{} days", lookback_days), limit as i64],
                |row: &rusqlite::Row| {
                    Ok(json!({
                        "id": row.get::<_, Option<String>>(0)?,
                        "title": row.get::<_, Option<String>>(1)?,
                        "type": row.get::<_, Option<String>>(2)?,
                        "start_time": row.get::<_, Option<String>>(3)?,
                        "summary": row.get::<_, Option<String>>(4)?,
                    }))
                },
            )
            .ok()?;
        Some(rows.flatten().collect())
    })()
    .unwrap_or_default();
    json!(result)
}

fn get_captures_by_meeting_title(db: &crate::db::ActionDb, title: &str, days_back: i64) -> Value {
    let result: Vec<Value> = (|| {
        let conn = db.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, meeting_id, meeting_title, capture_type, content, captured_at
             FROM captures
             WHERE LOWER(meeting_title) = LOWER(?1)
               AND captured_at >= date('now', ?2)
             ORDER BY captured_at DESC",
            )
            .ok()?;
        let rows = stmt
            .query_map(
                rusqlite::params![title, format!("-{} days", days_back)],
                |row: &rusqlite::Row| {
                    Ok(json!({
                        "id": row.get::<_, Option<String>>(0)?,
                        "meeting_title": row.get::<_, Option<String>>(2)?,
                        "type": row.get::<_, Option<String>>(3)?,
                        "content": row.get::<_, Option<String>>(4)?,
                        "captured_at": row.get::<_, Option<String>>(5)?,
                    }))
                },
            )
            .ok()?;
        Some(rows.flatten().collect())
    })()
    .unwrap_or_default();
    json!(result)
}

fn get_all_pending_actions(db: &crate::db::ActionDb, limit: usize) -> Value {
    let result: Vec<Value> = (|| {
        let conn = db.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, priority, status, due_date
             FROM actions
             WHERE status IN ('pending', 'waiting')
             ORDER BY priority, due_date
             LIMIT ?1",
            )
            .ok()?;
        let rows = stmt
            .query_map([limit as i64], |row: &rusqlite::Row| {
                Ok(json!({
                    "id": row.get::<_, Option<String>>(0)?,
                    "title": row.get::<_, Option<String>>(1)?,
                    "priority": row.get::<_, Option<String>>(2)?,
                    "status": row.get::<_, Option<String>>(3)?,
                    "due_date": row.get::<_, Option<String>>(4)?,
                }))
            })
            .ok()?;
        Some(rows.flatten().collect())
    })()
    .unwrap_or_default();
    json!(result)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_account_name_by_title() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Acme")).unwrap();

        let meeting = json!({
            "title": "Acme QBR",
            "external_domains": [],
        });
        let matched = guess_account_name(&meeting, dir.path()).unwrap();
        assert_eq!(matched.name, "Acme");
        assert_eq!(matched.relative_path, "Acme");
    }

    #[test]
    fn test_guess_account_name_by_domain() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("BigCorp")).unwrap();

        let meeting = json!({
            "title": "Weekly Sync",
            "external_domains": ["bigcorp.com"],
        });
        let matched = guess_account_name(&meeting, dir.path()).unwrap();
        assert_eq!(matched.name, "BigCorp");
        assert_eq!(matched.relative_path, "BigCorp");
    }

    #[test]
    fn test_guess_account_name_no_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Acme")).unwrap();

        let meeting = json!({
            "title": "Random Meeting",
            "external_domains": ["other.com"],
        });
        assert!(guess_account_name(&meeting, dir.path()).is_none());
    }

    #[test]
    fn test_guess_account_name_child_bu() {
        let dir = tempfile::tempdir().unwrap();
        // Parent with numbered internal dir + BU child
        std::fs::create_dir_all(dir.path().join("Cox/01-Customer-Information")).unwrap();
        std::fs::create_dir_all(dir.path().join("Cox/Consumer-Brands")).unwrap();

        // Match by domain (most common for BU meetings)
        let meeting = json!({
            "title": "Weekly Sync",
            "external_domains": ["consumer-brands.cox.com"],
        });
        let matched = guess_account_name(&meeting, dir.path()).unwrap();
        assert_eq!(matched.name, "Consumer-Brands");
        assert_eq!(matched.relative_path, "Cox/Consumer-Brands");
    }

    #[test]
    fn test_guess_account_name_child_by_title() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Cox/Enterprise")).unwrap();

        // Title contains exact dir name
        let meeting = json!({
            "title": "Enterprise QBR",
            "external_domains": [],
        });
        let matched = guess_account_name(&meeting, dir.path()).unwrap();
        assert_eq!(matched.name, "Enterprise");
        assert_eq!(matched.relative_path, "Cox/Enterprise");
    }

    #[test]
    fn test_guess_account_name_child_by_title_normalized() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Salesforce/Digital-Marketing-Technology"))
            .unwrap();

        // Title uses spaces while directory uses hyphens.
        let meeting = json!({
            "title": "Digital Marketing Technology Weekly",
            "external_domains": [],
        });
        let matched = guess_account_name(&meeting, dir.path()).unwrap();
        assert_eq!(matched.name, "Digital-Marketing-Technology");
        assert_eq!(
            matched.relative_path,
            "Salesforce/Digital-Marketing-Technology"
        );
    }

    #[test]
    fn test_guess_account_name_child_by_domain() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Salesforce/Engineering")).unwrap();

        let meeting = json!({
            "title": "Weekly Sync",
            "external_domains": ["engineering.salesforce.com"],
        });
        let matched = guess_account_name(&meeting, dir.path()).unwrap();
        assert_eq!(matched.name, "Engineering");
        assert_eq!(matched.relative_path, "Salesforce/Engineering");
    }

    #[test]
    fn test_find_account_dir_by_name_normalized() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Salesforce/Digital-Marketing-Technology"))
            .unwrap();

        let matched = find_account_dir_by_name("Digital Marketing Technology", dir.path())
            .expect("should match normalized BU name");

        assert_eq!(matched.name, "Digital-Marketing-Technology");
        assert_eq!(
            matched.relative_path,
            "Salesforce/Digital-Marketing-Technology"
        );
    }

    #[test]
    fn test_find_account_dir_by_name_prefers_child_over_top_level_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Digital-marketing-technology")).unwrap();
        std::fs::create_dir_all(dir.path().join("Salesforce/Digital-Marketing-Technology"))
            .unwrap();

        let matched = find_account_dir_by_name("Digital-Marketing-Technology", dir.path())
            .expect("should match child BU dir first");

        assert_eq!(matched.name, "Digital-Marketing-Technology");
        assert_eq!(
            matched.relative_path,
            "Salesforce/Digital-Marketing-Technology"
        );
    }

    #[test]
    fn test_find_account_dir_by_id_hint_prefers_parent_child_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Digital-marketing-technology")).unwrap();
        std::fs::create_dir_all(dir.path().join("Salesforce/Digital-Marketing-Technology"))
            .unwrap();

        let matched =
            find_account_dir_by_id_hint("salesforce--digital-marketing-technology", dir.path())
                .expect("should resolve parent/child from id hint");

        assert_eq!(matched.name, "Digital-Marketing-Technology");
        assert_eq!(
            matched.relative_path,
            "Salesforce/Digital-Marketing-Technology"
        );
    }

    #[test]
    fn test_guess_account_name_skips_numbered_dirs() {
        let dir = tempfile::tempdir().unwrap();
        // Only numbered internal dirs, no BU children
        std::fs::create_dir_all(dir.path().join("Acme/01-Customer-Information")).unwrap();
        std::fs::create_dir_all(dir.path().join("Acme/02-Meetings")).unwrap();

        let meeting = json!({
            "title": "Customer Information Review",
            "external_domains": [],
        });
        // Should NOT match the numbered internal dir
        assert!(guess_account_name(&meeting, dir.path()).is_none());
    }

    #[test]
    fn test_resolve_account_from_db_prefers_meeting_account_id() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(
            dir.path()
                .join("Accounts/Salesforce/Digital-Marketing-Technology"),
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("Accounts/Slack")).unwrap();

        let db = crate::db::ActionDb::open_at(dir.path().join("test.db")).expect("open test db");
        let now = chrono::Utc::now().to_rfc3339();

        db.upsert_entity(&crate::entity::DbEntity {
            id: "dmt-entity".to_string(),
            name: "Digital Marketing Technology".to_string(),
            entity_type: crate::entity::EntityType::Account,
            tracker_path: None,
            updated_at: now.clone(),
        })
        .expect("upsert dmt entity");

        db.upsert_entity(&crate::entity::DbEntity {
            id: "slack-entity".to_string(),
            name: "Slack".to_string(),
            entity_type: crate::entity::EntityType::Account,
            tracker_path: None,
            updated_at: now.clone(),
        })
        .expect("upsert slack entity");

        db.upsert_meeting(&crate::db::DbMeeting {
            id: "evt-1".to_string(),
            title: "Weekly Sync".to_string(),
            meeting_type: "customer".to_string(),
            start_time: "2026-02-12T10:00:00Z".to_string(),
            end_time: None,
            account_id: Some("dmt-entity".to_string()),
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now,
            calendar_event_id: Some("evt-1".to_string()),
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
        })
        .expect("upsert meeting");

        // Stale direct link that should not win over explicit meeting.account_id.
        db.link_meeting_entity("evt-1", "slack-entity", "account")
            .expect("link stale account");

        let meeting = json!({
            "id": "evt-1",
            "title": "Weekly Sync",
            "external_domains": ["slack.com"],
        });

        let matched = resolve_account_from_db(&db, "evt-1", &meeting, &dir.path().join("Accounts"))
            .expect("should resolve account");

        assert_eq!(matched.name, "Digital-Marketing-Technology");
        assert_eq!(
            matched.relative_path,
            "Salesforce/Digital-Marketing-Technology"
        );
    }

    #[test]
    fn test_find_file_in_dir_exact() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("dashboard.md"), "test").unwrap();
        assert!(find_file_in_dir(dir.path(), "dashboard.md").is_some());
    }

    #[test]
    fn test_find_file_in_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_file_in_dir(dir.path(), "nonexistent.md").is_none());
    }

    #[test]
    fn test_extract_section_items() {
        let content = "# Recent Wins\n- Won deal\n- Expanded usage\n\n# Other\nStuff\n";
        let items = extract_section_items(content, "Recent Wins");
        assert_eq!(items, vec!["Won deal", "Expanded usage"]);
    }

    #[test]
    fn test_extract_section_items_empty() {
        let content = "# Nothing Here\nNo bullets\n";
        let items = extract_section_items(content, "Recent Wins");
        assert!(items.is_empty());
    }

    #[test]
    fn test_gather_all_skips_personal() {
        let dir = tempfile::tempdir().unwrap();
        let classified = vec![
            json!({"id": "1", "type": "personal", "title": "Lunch"}),
            json!({"id": "2", "type": "customer", "title": "Acme Call", "start": "2026-02-08T10:00:00"}),
        ];
        let contexts = gather_all_meeting_contexts(&classified, dir.path(), None, None);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0]["event_id"], "2");
    }
}
