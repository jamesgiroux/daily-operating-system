// Accounts service — extracted from commands.rs
// Business logic for child account creation with collision handling.

use std::collections::HashMap;
use std::path::Path;

use crate::commands::{AccountChildSummary, AccountDetailResult, AccountListItem, MeetingPreview, MeetingSummary, PickerAccount, PrepContext};
use crate::db::ActionDb;
use crate::state::AppState;

/// Create a child account under a parent with collision handling.
///
/// Checks for duplicate names, generates unique IDs, creates DB record,
/// copies parent domains, and optionally writes workspace files.
pub fn create_child_account_record(
    db: &ActionDb,
    workspace: Option<&Path>,
    parent: &crate::db::DbAccount,
    name: &str,
    description: Option<&str>,
    owner_person_id: Option<&str>,
) -> Result<crate::db::DbAccount, String> {
    let children = db
        .get_child_accounts(&parent.id)
        .map_err(|e| e.to_string())?;
    if children.iter().any(|c| c.name.eq_ignore_ascii_case(name)) {
        return Err(format!(
            "A child named '{}' already exists under '{}'",
            name, parent.name
        ));
    }

    let base_slug = crate::util::slugify(name);
    let mut id = format!("{}--{}", parent.id, base_slug);
    let mut suffix = 2usize;
    while db.get_account(&id).map_err(|e| e.to_string())?.is_some() {
        id = format!("{}--{}-{}", parent.id, base_slug, suffix);
        suffix += 1;
    }

    let parent_tracker = parent.tracker_path.clone().unwrap_or_else(|| {
        if parent.is_internal {
            format!("Internal/{}", parent.name)
        } else {
            format!("Accounts/{}", parent.name)
        }
    });
    let tracker_path = format!("{}/{}", parent_tracker, name);
    let now = chrono::Utc::now().to_rfc3339();

    let account = crate::db::DbAccount {
        id,
        name: name.to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: Some(tracker_path),
        parent_id: Some(parent.id.clone()),
        is_internal: parent.is_internal,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;
    db.copy_account_domains(&parent.id, &account.id)
        .map_err(|e| e.to_string())?;

    if let Some(owner_id) = owner_person_id {
        db.link_person_to_entity(owner_id, &account.id, "owner")
            .map_err(|e| e.to_string())?;
    }

    if let Some(ws) = workspace {
        let account_dir = crate::accounts::resolve_account_dir(ws, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, name, "account");

        let mut json = default_account_json(&account);
        if let Some(desc) = description {
            let trimmed = desc.trim();
            if !trimmed.is_empty() {
                json.notes = Some(trimmed.to_string());
            }
        }
        let _ = crate::accounts::write_account_json(ws, &account, Some(&json), db);
        let _ = crate::accounts::write_account_markdown(ws, &account, Some(&json), db);
    }

    Ok(account)
}

/// Build a default AccountJson for a newly created account.
pub fn default_account_json(account: &crate::db::DbAccount) -> crate::accounts::AccountJson {
    crate::accounts::AccountJson {
        version: 1,
        entity_type: "account".to_string(),
        structured: crate::accounts::AccountStructured {
            arr: account.arr,
            health: account.health.clone(),
            lifecycle: account.lifecycle.clone(),
            renewal_date: account.contract_end.clone(),
            nps: account.nps,
            account_team: Vec::new(),
            csm: None,
            champion: None,
        },
        company_overview: None,
        strategic_programs: Vec::new(),
        notes: None,
        custom_sections: Vec::new(),
        parent_id: account.parent_id.clone(),
    }
}

/// Infer which internal account best matches a meeting by title + attendees.
pub fn infer_internal_account_for_meeting(
    db: &ActionDb,
    title: &str,
    attendees_csv: Option<&str>,
) -> Option<crate::db::DbAccount> {
    use std::collections::HashSet;

    let internal_accounts = db.get_internal_accounts().ok()?;
    if internal_accounts.is_empty() {
        return None;
    }
    let root = internal_accounts
        .iter()
        .find(|a| a.parent_id.is_none())
        .cloned();
    let candidates: Vec<crate::db::DbAccount> = internal_accounts
        .iter()
        .filter(|a| a.parent_id.is_some())
        .cloned()
        .collect();
    if candidates.is_empty() {
        return root;
    }

    let title_key = crate::helpers::normalize_key(title);
    let attendee_set: HashSet<String> = attendees_csv
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.contains('@'))
        .collect();

    let mut best: Option<(i32, crate::db::DbAccount)> = None;
    for candidate in candidates {
        let mut score = 0i32;
        let name_key = crate::helpers::normalize_key(&candidate.name);
        if !name_key.is_empty() && title_key.contains(&name_key) {
            score += 2;
        }

        let overlaps = db
            .get_people_for_entity(&candidate.id)
            .unwrap_or_default()
            .iter()
            .filter(|p| attendee_set.contains(&p.email.to_lowercase()))
            .count() as i32;
        score += overlaps * 3;

        match &best {
            None => best = Some((score, candidate)),
            Some((best_score, best_acc)) => {
                if score > *best_score
                    || (score == *best_score
                        && candidate.name.to_lowercase() < best_acc.name.to_lowercase())
                {
                    best = Some((score, candidate));
                }
            }
        }
    }

    match best {
        Some((score, account)) if score > 0 => Some(account),
        _ => root,
    }
}

/// Get full detail for an account by ID.
///
/// Loads account from DB, reads dashboard.json + intelligence.json,
/// fetches actions, meetings, people, team, signals, captures, and email signals.
pub fn get_account_detail(
    account_id: &str,
    state: &AppState,
) -> Result<AccountDetailResult, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    // Read narrative fields from dashboard.json + intelligence.json if they exist
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let (overview, programs, notes, intelligence) = if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
        let json_path = account_dir.join("dashboard.json");
        let (ov, prg, nt) = if json_path.exists() {
            match crate::accounts::read_account_json(&json_path) {
                Ok(result) => (
                    result.json.company_overview,
                    result.json.strategic_programs,
                    result.json.notes,
                ),
                Err(_) => (None, Vec::new(), None),
            }
        } else {
            (None, Vec::new(), None)
        };
        // Read intelligence.json (ADR-0057), migrate from CompanyOverview if needed
        let intel = crate::intelligence::read_intelligence_json(&account_dir)
            .ok()
            .or_else(|| {
                // Auto-migrate from legacy CompanyOverview on first access
                ov.as_ref().and_then(|overview| {
                    crate::intelligence::migrate_company_overview_to_intelligence(
                        workspace, &account, overview,
                    )
                })
            });
        (ov, prg, nt, intel)
    } else {
        (None, Vec::new(), None, None)
    };
    drop(config); // Release config lock before more DB queries

    let open_actions = db
        .get_account_actions(account_id)
        .map_err(|e| e.to_string())?;

    let upcoming_meetings: Vec<MeetingSummary> = db
        .get_upcoming_meetings_for_account(account_id, 5)
        .unwrap_or_default()
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
            meeting_type: m.meeting_type,
        })
        .collect();

    let recent_meetings: Vec<MeetingPreview> = db
        .get_meetings_for_account_with_prep(account_id, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| {
            let prep_context = m
                .prep_context_json
                .as_ref()
                .and_then(|json_str| serde_json::from_str::<PrepContext>(json_str).ok());
            MeetingPreview {
                id: m.id,
                title: m.title,
                start_time: m.start_time,
                meeting_type: m.meeting_type,
                prep_context,
            }
        })
        .collect();

    let linked_people = db.get_people_for_entity(account_id).unwrap_or_default();
    let account_team = db.get_account_team(account_id).unwrap_or_default();
    let account_team_import_notes = db
        .get_account_team_import_notes(account_id)
        .unwrap_or_default();

    let signals = db.get_stakeholder_signals(account_id).ok();

    let recent_captures = db
        .get_captures_for_account(account_id, 90)
        .unwrap_or_default();
    let recent_email_signals = db
        .list_recent_email_signals_for_entity(account_id, 12)
        .unwrap_or_default();

    // I114: Resolve parent name for child accounts, children for parent accounts
    let parent_name = account
        .parent_id
        .as_ref()
        .and_then(|pid| db.get_account(pid).ok().flatten().map(|a| a.name));

    let child_accounts = db.get_child_accounts(&account.id).unwrap_or_default();
    let parent_aggregate = if !child_accounts.is_empty() {
        db.get_parent_aggregate(&account.id).ok()
    } else {
        None
    };
    let children: Vec<AccountChildSummary> = child_accounts
        .iter()
        .map(|child| {
            let open_action_count = db
                .get_account_actions(&child.id)
                .map(|a| a.len())
                .unwrap_or(0);
            AccountChildSummary {
                id: child.id.clone(),
                name: child.name.clone(),
                health: child.health.clone(),
                arr: child.arr,
                open_action_count,
            }
        })
        .collect();

    Ok(AccountDetailResult {
        id: account.id,
        name: account.name,
        lifecycle: account.lifecycle,
        arr: account.arr,
        health: account.health,
        nps: account.nps,
        renewal_date: account.contract_end,
        contract_start: account.contract_start,
        company_overview: overview,
        strategic_programs: programs,
        notes,
        open_actions,
        upcoming_meetings,
        recent_meetings,
        linked_people,
        account_team,
        account_team_import_notes,
        signals,
        recent_captures,
        recent_email_signals,
        parent_id: account.parent_id,
        parent_name,
        children,
        parent_aggregate,
        is_internal: account.is_internal,
        archived: account.archived,
        intelligence,
    })
}

/// Update a single structured field on an account.
/// Writes to SQLite, emits signal, then regenerates dashboard.json + dashboard.md.
pub fn update_account_field(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    field: &str,
    value: &str,
) -> Result<(), String> {
    db.update_account_field(account_id, field, value)
        .map_err(|e| e.to_string())?;

    // Emit field update signal (I308)
    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", account_id, "field_updated", "user_edit",
        Some(&format!("{{\"field\":\"{}\",\"value\":\"{}\"}}", field, value.replace('"', "\\\""))), 0.8);

    // Regenerate workspace files
    if let Ok(Some(account)) = db.get_account(account_id) {
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let json_path =
                crate::accounts::resolve_account_dir(workspace, &account).join("dashboard.json");
            let existing = if json_path.exists() {
                crate::accounts::read_account_json(&json_path)
                    .ok()
                    .map(|r| r.json)
            } else {
                None
            };
            let _ = crate::accounts::write_account_json(workspace, &account, existing.as_ref(), db);
            let _ =
                crate::accounts::write_account_markdown(workspace, &account, existing.as_ref(), db);
        }
    }

    Ok(())
}

/// Update account notes (narrative field — JSON only, not SQLite).
/// Writes dashboard.json + regenerates dashboard.md.
pub fn update_account_notes(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    notes: &str,
) -> Result<(), String> {
    let account = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let json_path =
        crate::accounts::resolve_account_dir(workspace, &account).join("dashboard.json");
    let mut existing = if json_path.exists() {
        crate::accounts::read_account_json(&json_path)
            .map(|r| r.json)
            .unwrap_or_else(|_| default_account_json(&account))
    } else {
        default_account_json(&account)
    };

    existing.notes = if notes.is_empty() { None } else { Some(notes.to_string()) };

    let _ = crate::accounts::write_account_json(workspace, &account, Some(&existing), db);
    let _ = crate::accounts::write_account_markdown(workspace, &account, Some(&existing), db);

    // Emit field update signal (I377)
    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", account_id, "field_updated", "user_edit",
        Some(&format!("{{\"field\":\"notes\",\"value\":\"{}\"}}", notes.chars().take(100).collect::<String>().replace('"', "\\\""))), 0.8);

    Ok(())
}

/// Update account strategic programs (narrative field — JSON only).
/// Writes dashboard.json + regenerates dashboard.md.
pub fn update_account_programs(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    programs_json: &str,
) -> Result<(), String> {
    let account = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let programs: Vec<crate::accounts::StrategicProgram> = serde_json::from_str(programs_json)
        .map_err(|e| format!("Invalid programs JSON: {}", e))?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let json_path =
        crate::accounts::resolve_account_dir(workspace, &account).join("dashboard.json");
    let mut existing = if json_path.exists() {
        crate::accounts::read_account_json(&json_path)
            .map(|r| r.json)
            .unwrap_or_else(|_| default_account_json(&account))
    } else {
        default_account_json(&account)
    };

    existing.strategic_programs = programs;

    let _ = crate::accounts::write_account_json(workspace, &account, Some(&existing), db);
    let _ = crate::accounts::write_account_markdown(workspace, &account, Some(&existing), db);

    // Emit field update signal (I377)
    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", account_id, "field_updated", "user_edit",
        Some("{\"field\":\"strategic_programs\"}"), 0.8);

    Ok(())
}

/// Create a new account. Creates SQLite record + workspace files.
/// If `parent_id` is provided, creates a child (BU) account under that parent.
pub fn create_account(
    db: &ActionDb,
    state: &AppState,
    name: &str,
    parent_id: Option<&str>,
) -> Result<String, String> {
    let name = crate::util::validate_entity_name(name)?.to_string();

    let (id, tracker_path, is_internal) = if let Some(pid) = parent_id {
        let parent = db
            .get_account(pid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Parent account not found: {}", pid))?;
        let child_id = format!("{}--{}", pid, crate::util::slugify(&name));
        let parent_dir = parent
            .tracker_path
            .unwrap_or_else(|| format!("Accounts/{}", parent.name));
        let tp = format!("{}/{}", parent_dir, name);
        (child_id, tp, parent.is_internal)
    } else {
        let id = crate::util::slugify(&name);
        (id, format!("Accounts/{}", name), false)
    };

    let now = chrono::Utc::now().to_rfc3339();

    let account = crate::db::DbAccount {
        id: id.clone(),
        name: name.clone(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: Some(tracker_path),
        parent_id: parent_id.map(|s| s.to_string()),
        is_internal,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;
    if let Some(pid) = parent_id {
        let _ = db.copy_account_domains(pid, &account.id);
    }

    // Create workspace files + directory template (ADR-0059)
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, &name, "account");
        let _ = crate::accounts::write_account_json(workspace, &account, None, db);
        let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);
    }

    Ok(id)
}

/// Archive or unarchive an account with signal emission. Cascades to children when archiving.
pub fn archive_account(db: &ActionDb, state: &crate::state::AppState, id: &str, archived: bool) -> Result<(), String> {
    db.archive_account(id, archived)
        .map_err(|e| e.to_string())?;

    let signal_type = if archived { "entity_archived" } else { "entity_unarchived" };
    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", id, signal_type, "user_action", None, 0.9);

    Ok(())
}

/// Merge source account into target account with signal emission.
pub fn merge_accounts(db: &ActionDb, state: &crate::state::AppState, from_id: &str, into_id: &str) -> Result<crate::db::MergeResult, String> {
    let result = db.merge_accounts(from_id, into_id)
        .map_err(|e| e.to_string())?;

    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", into_id, "entity_merged", "user_action",
        Some(&format!("{{\"merged_from\":\"{}\"}}", from_id)), 0.9);

    Ok(result)
}

/// Restore an archived account with optional child restoration.
pub fn restore_account(db: &ActionDb, account_id: &str, restore_children: bool) -> Result<usize, String> {
    db.restore_account(account_id, restore_children)
        .map_err(|e| e.to_string())
}

/// Add a person-role pair to an account team with signal emission.
pub fn add_account_team_member(
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    let role = role.trim().to_lowercase();
    if role.is_empty() {
        return Err("Role is required".to_string());
    }
    db.add_account_team_member(account_id, person_id, &role)
        .map_err(|e| e.to_string())?;

    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", account_id, "team_member_added", "user_action",
        Some(&format!("{{\"person_id\":\"{}\",\"role\":\"{}\"}}", person_id, role)), 0.8);

    Ok(())
}

/// Remove a person-role pair from an account team with signal emission.
pub fn remove_account_team_member(
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    db.remove_account_team_member(account_id, person_id, role)
        .map_err(|e| e.to_string())?;

    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", account_id, "team_member_removed", "user_action",
        Some(&format!("{{\"person_id\":\"{}\",\"role\":\"{}\"}}", person_id, role)), 0.7);

    Ok(())
}

/// Record an account lifecycle event with signal emission.
pub fn record_account_event(
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    event_type: &str,
    event_date: &str,
    arr_impact: Option<f64>,
    notes: Option<&str>,
) -> Result<i64, String> {
    let event_id = db.record_account_event(account_id, event_type, event_date, arr_impact, notes)
        .map_err(|e| e.to_string())?;

    let _ = crate::signals::bus::emit_signal_and_propagate(db, &state.signal_engine, "account", account_id, "account_event_recorded", "user_action",
        Some(&format!("{{\"event_type\":\"{}\",\"event_date\":\"{}\"}}", event_type, event_date)), 0.8);

    Ok(event_id)
}

/// Bulk-create accounts from a list of names. Returns created account IDs.
pub fn bulk_create_accounts(
    db: &ActionDb,
    workspace: &Path,
    names: &[String],
) -> Result<Vec<String>, String> {
    let mut created_ids = Vec::with_capacity(names.len());

    for raw_name in names {
        let name = crate::util::validate_entity_name(raw_name)?;
        let id = crate::util::slugify(name);

        // Skip duplicates
        if let Ok(Some(_)) = db.get_account(&id) {
            continue;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let account = crate::db::DbAccount {
            id: id.clone(),
            name: name.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some(format!("Accounts/{}", name)),
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        db.upsert_account(&account).map_err(|e| e.to_string())?;

        let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, name, "account");
        let _ = crate::accounts::write_account_json(workspace, &account, None, db);
        let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);

        created_ids.push(id);
    }

    Ok(created_ids)
}

/// Convert a DbAccount to an AccountListItem with computed signals.
pub fn account_to_list_item(
    a: &crate::db::DbAccount,
    db: &ActionDb,
    child_count: usize,
) -> AccountListItem {
    let open_action_count = db
        .get_account_actions(&a.id)
        .map(|actions| actions.len())
        .unwrap_or(0);

    let signals = db.get_stakeholder_signals(&a.id).ok();
    let days_since_last_meeting = signals.as_ref().and_then(|s| {
        s.last_meeting.as_ref().and_then(|lm| {
            chrono::DateTime::parse_from_rfc3339(lm)
                .or_else(|_| {
                    chrono::DateTime::parse_from_rfc3339(&format!(
                        "{}+00:00",
                        lm.trim_end_matches('Z')
                    ))
                })
                .ok()
                .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days())
        })
    });

    let team_summary = db.get_account_team(&a.id).ok().and_then(|members| {
        if members.is_empty() {
            None
        } else {
            let labels: Vec<String> = members
                .iter()
                .take(2)
                .map(|m| format!("{} ({})", m.person_name, m.role.to_uppercase()))
                .collect();
            let suffix = if members.len() > 2 {
                format!(" +{}", members.len() - 2)
            } else {
                String::new()
            };
            Some(format!("Team: {}{}", labels.join(", "), suffix))
        }
    });

    AccountListItem {
        id: a.id.clone(),
        name: a.name.clone(),
        lifecycle: a.lifecycle.clone(),
        arr: a.arr,
        health: a.health.clone(),
        nps: a.nps,
        team_summary,
        renewal_date: a.contract_end.clone(),
        open_action_count,
        days_since_last_meeting,
        parent_id: a.parent_id.clone(),
        parent_name: None,
        child_count,
        is_parent: child_count > 0,
        is_internal: a.is_internal,
        archived: a.archived,
    }
}

/// Get top-level accounts list with computed fields.
pub fn get_accounts_list(db: &ActionDb) -> Result<Vec<AccountListItem>, String> {
    let accounts = db.get_top_level_accounts().map_err(|e| e.to_string())?;

    let items: Vec<AccountListItem> = accounts
        .into_iter()
        .map(|a| {
            let child_count = db.get_child_accounts(&a.id).map(|c| c.len()).unwrap_or(0);
            account_to_list_item(&a, db, child_count)
        })
        .collect();

    Ok(items)
}

/// Get child accounts for a parent with computed fields.
pub fn get_child_accounts_list(
    db: &ActionDb,
    parent_id: &str,
) -> Result<Vec<AccountListItem>, String> {
    let children = db
        .get_child_accounts(parent_id)
        .map_err(|e| e.to_string())?;

    let parent_name = db.get_account(parent_id).ok().flatten().map(|a| a.name);

    let items: Vec<AccountListItem> = children
        .into_iter()
        .map(|a| {
            let grandchild_count = db.get_child_accounts(&a.id).map(|c| c.len()).unwrap_or(0);
            let mut item = account_to_list_item(&a, db, grandchild_count);
            item.parent_name = parent_name.clone();
            item
        })
        .collect();

    Ok(items)
}

/// Lightweight list of ALL accounts (parents + children) for entity pickers.
pub fn get_accounts_for_picker(db: &ActionDb) -> Result<Vec<PickerAccount>, String> {
    let all = db.get_all_accounts().map_err(|e| e.to_string())?;

    let parent_names: HashMap<String, String> = all
        .iter()
        .filter(|a| a.parent_id.is_none())
        .map(|a| (a.id.clone(), a.name.clone()))
        .collect();

    let items: Vec<PickerAccount> = all
        .into_iter()
        .map(|a| {
            let parent_name = a
                .parent_id
                .as_ref()
                .and_then(|pid| parent_names.get(pid).cloned());
            PickerAccount {
                id: a.id,
                name: a.name,
                parent_name,
                is_internal: a.is_internal,
            }
        })
        .collect();

    Ok(items)
}
