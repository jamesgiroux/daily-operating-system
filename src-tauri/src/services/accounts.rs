// Accounts service — extracted from commands.rs
// Business logic for child account creation with collision handling.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{Datelike, NaiveDate, Utc};
use serde_json::Value;

use crate::commands::{
    AccountChildSummary, AccountDetailResult, AccountListItem, MeetingPreview, MeetingSummary,
    PickerAccount, PrepContext,
};
use crate::db::ActionDb;
use crate::signals::propagation::PropagationEngine;
use crate::state::AppState;

pub fn set_account_domains(
    db: &ActionDb,
    account_id: &str,
    domains: &[String],
) -> Result<(), String> {
    db.set_account_domains(account_id, domains)
        .map_err(|e| e.to_string())
}

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
        if parent.account_type.is_internal() {
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
        tracker_path: Some(tracker_path),
        parent_id: Some(parent.id.clone()),
        account_type: parent.account_type.clone(),
        updated_at: now,
        ..Default::default()
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;
    db.copy_account_domains(&parent.id, &account.id)
        .map_err(|e| e.to_string())?;

    if let Some(owner_id) = owner_person_id {
        db.link_person_to_entity(owner_id, &account.id, "owner")
            .map_err(|e| e.to_string())?;
    }

    if let Some(desc) = description {
        let trimmed = desc.trim();
        if !trimmed.is_empty() {
            let _ = db.update_account_field(&account.id, "notes", trimmed);
        }
    }

    let account = db
        .get_account(&account.id)
        .map_err(|e| e.to_string())?
        .unwrap_or(account);

    if let Some(ws) = workspace {
        let account_dir = crate::accounts::resolve_account_dir(ws, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, name, "account");

        let _ = crate::accounts::write_account_json(ws, &account, None, db);
        let _ = crate::accounts::write_account_markdown(ws, &account, None, db);
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
        company_overview: account
            .company_overview
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok()),
        strategic_programs: account
            .strategic_programs
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        notes: account.notes.clone(),
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

#[derive(Debug, Clone)]
pub struct LifecycleTransitionCandidate {
    pub new_lifecycle: String,
    pub renewal_stage: Option<String>,
    pub source: String,
    pub confidence: f64,
    pub evidence: Option<String>,
    pub completion_trigger: Option<String>,
}

fn parse_iso_date(value: &str) -> Option<NaiveDate> {
    let trimmed = value.trim();
    let date_str = trimmed.get(0..10).unwrap_or(trimmed);
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

pub fn normalized_lifecycle(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "renewal" => "renewing".to_string(),
        "at-risk" => "at_risk".to_string(),
        other => other.to_string(),
    }
}

fn infer_renewal_stage(contract_end: Option<&str>) -> Option<String> {
    let contract_end = contract_end.and_then(parse_iso_date)?;
    let days_until = (contract_end - Utc::now().date_naive()).num_days();
    let stage = match days_until {
        i64::MIN..=-1 => "processed",
        0..=30 => "contract_sent",
        31..=60 => "negotiating",
        61..=120 => "approaching",
        _ => return None, // >120 days out — not yet in renewal stage
    };
    Some(stage.to_string())
}

fn add_one_year(date: NaiveDate) -> Option<NaiveDate> {
    let next_year = date.year() + 1;
    NaiveDate::from_ymd_opt(next_year, date.month(), date.day()).or_else(|| {
        if date.month() == 2 && date.day() == 29 {
            NaiveDate::from_ymd_opt(next_year, 2, 28)
        } else {
            None
        }
    })
}

fn maybe_roll_contract_end(
    previous_contract_end: Option<&str>,
    previous_lifecycle: Option<&str>,
    new_lifecycle: &str,
    completion_trigger: Option<&str>,
) -> Option<String> {
    if new_lifecycle != "active"
        || completion_trigger != Some("contract_signed")
        || !matches!(
            previous_lifecycle.unwrap_or_default(),
            "renewing" | "at_risk"
        )
    {
        return previous_contract_end.map(str::to_string);
    }

    previous_contract_end
        .and_then(parse_iso_date)
        .and_then(add_one_year)
        .map(|date| date.format("%Y-%m-%d").to_string())
        .or_else(|| previous_contract_end.map(str::to_string))
}

fn account_field_conflict_feedback_key(field: &str, suggested_value: &str) -> String {
    format!("account_field_conflict:{field}:{suggested_value}")
}

fn account_field_signal_category(field: &str) -> String {
    match field {
        "arr" => "account_arr_conflict".to_string(),
        "lifecycle" => "lifecycle_transition".to_string(),
        "contract_end" => "renewal_date_conflict".to_string(),
        "nps" => "nps_conflict".to_string(),
        _ => format!("account_field_{field}"),
    }
}

fn current_health_score(db: &ActionDb, account_id: &str) -> Option<f64> {
    db.conn_ref()
        .query_row(
            "SELECT health_score FROM entity_quality WHERE entity_id = ?1 AND entity_type = 'account'",
            rusqlite::params![account_id],
            |row| row.get(0),
        )
        .ok()
}

fn extract_json_conflict_value(payload: &Value, field: &str) -> Option<String> {
    let keys: &[&str] = match field {
        "arr" => &["currentArr", "current_arr", "arr"],
        "lifecycle" => &["customerStage", "customer_stage", "lifecycle"],
        "contract_end" => &["renewalDate", "renewal_date", "contractEnd", "contract_end"],
        "nps" => &["nps", "NPS"],
        _ => &[],
    };

    for key in keys {
        let Some(value) = payload.get(*key) else {
            continue;
        };
        let suggestion = match value {
            Value::Null => None,
            Value::Number(number) => Some(number.to_string()),
            Value::String(string) if !string.trim().is_empty() => Some(string.trim().to_string()),
            _ => Some(value.to_string()),
        };
        if suggestion.is_some() {
            return suggestion;
        }
    }

    None
}

fn field_matches_current_value(
    account: &crate::db::DbAccount,
    field: &str,
    suggested_value: &str,
) -> bool {
    match field {
        "arr" => account
            .arr
            .zip(suggested_value.parse::<f64>().ok())
            .map(|(current, suggested)| (current - suggested).abs() < 1.0)
            .unwrap_or(false),
        "lifecycle" => account
            .lifecycle
            .as_deref()
            .map(normalized_lifecycle)
            .is_some_and(|current| current == normalized_lifecycle(suggested_value)),
        "contract_end" => account
            .contract_end
            .as_deref()
            .zip(parse_iso_date(suggested_value))
            .map(|(current, suggested)| parse_iso_date(current) == Some(suggested))
            .unwrap_or(false),
        "nps" => account
            .nps
            .zip(suggested_value.parse::<i32>().ok())
            .map(|(current, suggested)| current == suggested)
            .unwrap_or(false),
        _ => false,
    }
}

fn build_account_field_conflicts(
    db: &ActionDb,
    account: &crate::db::DbAccount,
    intelligence: Option<&crate::intelligence::IntelligenceJson>,
) -> Vec<crate::types::AccountFieldConflictSuggestion> {
    let mut conflicts: HashMap<String, crate::types::AccountFieldConflictSuggestion> =
        HashMap::new();
    let dismissed_conflicts: HashSet<String> = db
        .get_entity_feedback(&account.id, "account")
        .unwrap_or_default()
        .into_iter()
        .filter(|row| {
            row.feedback_type == "negative" && row.field.starts_with("account_field_conflict:")
        })
        .map(|row| row.field)
        .collect();

    for signal in
        crate::services::signals::get_for_entity(db, "account", &account.id).unwrap_or_default()
    {
        let Some(raw_value) = signal.value.as_deref() else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<Value>(raw_value) else {
            continue;
        };

        for field in ["arr", "lifecycle", "contract_end", "nps"] {
            let Some(suggested_value) = extract_json_conflict_value(&payload, field) else {
                continue;
            };
            let feedback_key = account_field_conflict_feedback_key(field, &suggested_value);
            if dismissed_conflicts.contains(&feedback_key) {
                continue;
            }
            if field_matches_current_value(account, field, &suggested_value) {
                continue;
            }

            conflicts.entry(field.to_string()).or_insert_with(|| {
                crate::types::AccountFieldConflictSuggestion {
                    field: field.to_string(),
                    source: signal.source.clone(),
                    suggested_value,
                    signal_id: signal.id.clone(),
                    confidence: signal.confidence,
                    detected_at: Some(signal.created_at.clone()),
                }
            });
        }
    }

    if let Some(intelligence) = intelligence {
        if let Some(contract) = intelligence.contract_context.as_ref() {
            if let Some(current_arr) = contract.current_arr {
                let suggested = format!("{current_arr:.0}");
                if !field_matches_current_value(account, "arr", &suggested) {
                    let feedback_key = account_field_conflict_feedback_key("arr", &suggested);
                    if !dismissed_conflicts.contains(&feedback_key) {
                        conflicts.entry("arr".to_string()).or_insert_with(|| {
                            crate::types::AccountFieldConflictSuggestion {
                                field: "arr".to_string(),
                                source: "intelligence_contract_context".to_string(),
                                suggested_value: suggested,
                                signal_id: "intelligence-contract-context-arr".to_string(),
                                confidence: 0.6,
                                detected_at: None,
                            }
                        });
                    }
                }
            }
            if let Some(renewal_date) = contract.renewal_date.as_ref() {
                if !field_matches_current_value(account, "contract_end", renewal_date) {
                    let feedback_key =
                        account_field_conflict_feedback_key("contract_end", renewal_date);
                    if !dismissed_conflicts.contains(&feedback_key) {
                        conflicts
                            .entry("contract_end".to_string())
                            .or_insert_with(|| crate::types::AccountFieldConflictSuggestion {
                                field: "contract_end".to_string(),
                                source: "intelligence_contract_context".to_string(),
                                suggested_value: renewal_date.clone(),
                                signal_id: "intelligence-contract-context-renewal-date".to_string(),
                                confidence: 0.6,
                                detected_at: None,
                            });
                    }
                }
            }
        }
        if let Some(org_health) = intelligence.org_health.as_ref() {
            if let Some(stage) = org_health.customer_stage.as_ref() {
                if !field_matches_current_value(account, "lifecycle", stage) {
                    let suggested_lifecycle = normalized_lifecycle(stage);
                    let feedback_key =
                        account_field_conflict_feedback_key("lifecycle", &suggested_lifecycle);
                    if !dismissed_conflicts.contains(&feedback_key) {
                        conflicts.entry("lifecycle".to_string()).or_insert_with(|| {
                            crate::types::AccountFieldConflictSuggestion {
                                field: "lifecycle".to_string(),
                                source: if org_health.source.is_empty() {
                                    "glean_crm".to_string()
                                } else {
                                    org_health.source.clone()
                                },
                                suggested_value: suggested_lifecycle.clone(),
                                signal_id: "intelligence-org-health-lifecycle".to_string(),
                                confidence: 0.7,
                                detected_at: if org_health.gathered_at.is_empty() {
                                    None
                                } else {
                                    Some(org_health.gathered_at.clone())
                                },
                            }
                        });
                    }
                }
            }
        }
        if let Some(nps_csat) = intelligence.nps_csat.as_ref() {
            if let Some(nps) = nps_csat.nps {
                let suggested = nps.to_string();
                if !field_matches_current_value(account, "nps", &suggested) {
                    let feedback_key = account_field_conflict_feedback_key("nps", &suggested);
                    if !dismissed_conflicts.contains(&feedback_key) {
                        conflicts.entry("nps".to_string()).or_insert_with(|| {
                            crate::types::AccountFieldConflictSuggestion {
                                field: "nps".to_string(),
                                source: nps_csat
                                    .source
                                    .clone()
                                    .unwrap_or_else(|| "survey_tool".to_string()),
                                suggested_value: suggested,
                                signal_id: "intelligence-nps".to_string(),
                                confidence: 0.65,
                                detected_at: nps_csat.survey_date.clone(),
                            }
                        });
                    }
                }
            }
        }
    }

    conflicts.into_values().collect()
}

fn build_account_products(
    db: &ActionDb,
    account_id: &str,
    intelligence: Option<&crate::intelligence::IntelligenceJson>,
) -> Vec<crate::db::DbAccountProduct> {
    let stored = db.get_account_products(account_id).unwrap_or_default();
    if !stored.is_empty() {
        return stored;
    }

    intelligence
        .and_then(|item| item.product_adoption.as_ref())
        .map(|adoption| {
            adoption
                .feature_adoption
                .iter()
                .enumerate()
                .map(|(index, feature)| crate::db::DbAccountProduct {
                    id: -((index as i64) + 1),
                    account_id: account_id.to_string(),
                    name: feature.clone(),
                    category: Some("adopted_feature".to_string()),
                    status: "active".to_string(),
                    arr_portion: None,
                    source: adoption
                        .source
                        .clone()
                        .unwrap_or_else(|| "ai_inference".to_string()),
                    confidence: 0.55,
                    notes: adoption
                        .trend
                        .as_ref()
                        .map(|trend| format!("Observed in product adoption ({trend})")),
                    product_type: None,
                    tier: None,
                    billing_terms: None,
                    arr: None,
                    last_verified_at: None,
                    data_source: None,
                    created_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn evaluate_lifecycle_transition_candidate(
    db: &ActionDb,
    account: &crate::db::DbAccount,
) -> Option<LifecycleTransitionCandidate> {
    let active_signals =
        crate::services::signals::get_for_entity(db, "account", &account.id).unwrap_or_default();
    let current_lifecycle = account
        .lifecycle
        .as_deref()
        .map(normalized_lifecycle)
        .unwrap_or_default();
    let days_until_renewal = account
        .contract_end
        .as_deref()
        .and_then(parse_iso_date)
        .map(|date| (date - Utc::now().date_naive()).num_days());

    let renewal_signals: Vec<_> = active_signals
        .iter()
        .filter(|signal| {
            matches!(
                signal.signal_type.as_str(),
                "renewal_proximity" | "proactive_renewal_gap" | "renewal_data_updated"
            )
        })
        .collect();
    let risk_signals: Vec<_> = active_signals
        .iter()
        .filter(|signal| {
            matches!(
                signal.signal_type.as_str(),
                "renewal_at_risk" | "renewal_risk_escalation"
            )
        })
        .collect();
    let onboarding_completion = active_signals.iter().find(|signal| {
        matches!(
            signal.signal_type.as_str(),
            "go_live" | "onboarding_complete"
        )
    });
    let contract_signed = active_signals
        .iter()
        .find(|signal| signal.signal_type == "contract_signed");

    if let Some(signal) = contract_signed {
        if matches!(current_lifecycle.as_str(), "renewing" | "at_risk") {
            return Some(LifecycleTransitionCandidate {
                new_lifecycle: "active".to_string(),
                renewal_stage: None,
                source: signal.source.clone(),
                confidence: signal.confidence.max(0.9),
                evidence: Some("Contract-signed signal detected.".to_string()),
                completion_trigger: Some("contract_signed".to_string()),
            });
        }
    }

    if let Some(signal) = onboarding_completion {
        if matches!(
            current_lifecycle.as_str(),
            "" | "onboarding" | "adoption" | "ramping"
        ) {
            return Some(LifecycleTransitionCandidate {
                new_lifecycle: "active".to_string(),
                renewal_stage: None,
                source: signal.source.clone(),
                confidence: signal.confidence.max(0.82),
                evidence: Some(format!(
                    "{} indicates onboarding completion.",
                    signal.signal_type.replace('_', " ")
                )),
                completion_trigger: Some(signal.signal_type.clone()),
            });
        }
    }

    let renewal_confidence = renewal_signals
        .iter()
        .map(|signal| signal.confidence)
        .fold(0.0f64, f64::max);
    if !matches!(current_lifecycle.as_str(), "renewing" | "churned") {
        let proximity_confidence = match days_until_renewal {
            Some(days) if days <= 30 => 0.9,
            Some(days) if days <= 60 => 0.8,
            Some(days) if days <= 120 => 0.72,
            _ => 0.0,
        };
        let confidence = renewal_confidence.max(proximity_confidence);
        if confidence >= 0.72 {
            let signal_sources = renewal_signals
                .iter()
                .map(|signal| signal.signal_type.replace('_', " "))
                .collect::<Vec<_>>();
            let evidence = if signal_sources.is_empty() {
                days_until_renewal.map(|days| format!("Contract end is {days} days away."))
            } else {
                Some(format!("Signals: {}.", signal_sources.join(", ")))
            };
            return Some(LifecycleTransitionCandidate {
                new_lifecycle: "renewing".to_string(),
                renewal_stage: infer_renewal_stage(account.contract_end.as_deref()),
                source: renewal_signals
                    .first()
                    .map(|signal| signal.source.clone())
                    .unwrap_or_else(|| "proactive".to_string()),
                confidence,
                evidence,
                completion_trigger: Some("renewal".to_string()),
            });
        }
    }

    let compound_risk =
        risk_signals.len() >= 2 || (!risk_signals.is_empty() && !renewal_signals.is_empty());
    if compound_risk && current_lifecycle != "at_risk" {
        let confidence = risk_signals
            .iter()
            .map(|signal| signal.confidence)
            .fold(0.0f64, f64::max)
            .max(0.83);
        let evidence = Some(format!(
            "Compound renewal risk detected from {}.",
            risk_signals
                .iter()
                .map(|signal| signal.signal_type.replace('_', " "))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        return Some(LifecycleTransitionCandidate {
            new_lifecycle: "at_risk".to_string(),
            renewal_stage: infer_renewal_stage(account.contract_end.as_deref()),
            source: risk_signals
                .first()
                .map(|signal| signal.source.clone())
                .unwrap_or_else(|| "proactive".to_string()),
            confidence,
            evidence,
            completion_trigger: None,
        });
    }

    None
}

fn emit_auto_completed_success_plan_signals(
    db: &ActionDb,
    engine: &PropagationEngine,
    auto_completed: &crate::db::success_plans::AutoCompletedMilestones,
    source: &str,
) -> Result<(), String> {
    for milestone in &auto_completed.milestones {
        crate::services::signals::emit_and_propagate(
            db,
            engine,
            "account",
            &milestone.account_id,
            "milestone_completed",
            source,
            Some(&format!(
                "{{\"milestone_id\":\"{}\",\"objective_id\":\"{}\",\"completion_trigger\":\"{}\"}}",
                milestone.id,
                milestone.objective_id,
                milestone.completion_trigger.clone().unwrap_or_default()
            )),
            0.9,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
    }

    for objective in &auto_completed.objectives {
        crate::services::signals::emit_and_propagate(
            db,
            engine,
            "account",
            &objective.account_id,
            "objective_completed",
            source,
            Some(&format!("{{\"objective_id\":\"{}\"}}", objective.id)),
            0.95,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
    }

    Ok(())
}

pub fn apply_lifecycle_transition(
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    transition: &LifecycleTransitionCandidate,
) -> Result<Option<i64>, String> {
    db.with_transaction(|tx| {
        let account = tx
            .get_account(account_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Account not found: {account_id}"))?;
        let previous_lifecycle = account
            .lifecycle
            .as_deref()
            .map(normalized_lifecycle);
        let next_lifecycle = normalized_lifecycle(&transition.new_lifecycle);
        let previous_contract_end = account.contract_end.clone();
        let next_contract_end = maybe_roll_contract_end(
            previous_contract_end.as_deref(),
            previous_lifecycle.as_deref(),
            &next_lifecycle,
            transition.completion_trigger.as_deref(),
        );
        let previous_stage = tx
            .get_account_renewal_stage(account_id)
            .map_err(|e| e.to_string())?;
        let next_stage = transition.renewal_stage.as_deref().map(str::to_string);

        if previous_lifecycle.as_deref() == Some(next_lifecycle.as_str())
            && previous_stage == next_stage
        {
            return Ok(None);
        }

        let health_before = current_health_score(tx, account_id);

        tx.update_account_field(account_id, "lifecycle", &next_lifecycle)
            .map_err(|e| e.to_string())?;
        tx.set_account_field_provenance(account_id, "lifecycle", &transition.source, None)
            .map_err(|e| e.to_string())?;
        tx.set_account_renewal_stage(account_id, transition.renewal_stage.as_deref())
            .map_err(|e| e.to_string())?;
        if next_contract_end != previous_contract_end {
            if let Some(contract_end) = next_contract_end.as_deref() {
                tx.update_account_field(account_id, "contract_end", contract_end)
                    .map_err(|e| e.to_string())?;
                tx.set_account_field_provenance(account_id, "contract_end", &transition.source, None)
                    .map_err(|e| e.to_string())?;
            }
        }

        let _ = crate::services::intelligence::recompute_entity_health(tx, account_id, "account");
        let health_after = current_health_score(tx, account_id);

        let change_id = if previous_lifecycle.as_deref() != Some(next_lifecycle.as_str()) {
            Some(
                tx.insert_lifecycle_change(
                    account_id,
                    previous_lifecycle.as_deref(),
                    &next_lifecycle,
                    previous_stage.as_deref(),
                    transition.renewal_stage.as_deref(),
                    previous_contract_end.as_deref(),
                    next_contract_end.as_deref(),
                    &transition.source,
                    transition.confidence,
                    transition.evidence.as_deref(),
                    health_before,
                    health_after,
                )
                .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };

        let payload = serde_json::json!({
            "previous_lifecycle": previous_lifecycle,
            "new_lifecycle": next_lifecycle,
            "previous_stage": previous_stage,
            "new_stage": next_stage,
            "confidence": transition.confidence,
            "evidence": transition.evidence,
        })
        .to_string();
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "account",
            account_id,
            "lifecycle_transition",
            &transition.source,
            Some(&payload),
            transition.confidence,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;

        if let Some(trigger) = transition.completion_trigger.as_deref() {
            if transition.confidence >= 0.8 {
                let auto_completed = tx
                    .complete_milestones_for_completion_trigger(
                        account_id,
                        trigger,
                        Some("lifecycle_transition"),
                    )
                    .map_err(|e| e.to_string())?;
                emit_auto_completed_success_plan_signals(
                    tx,
                    engine,
                    &auto_completed,
                    "lifecycle_transition",
                )?;
            } else {
                // I628 AC3: Note potential milestone matches in timeline (sub-0.8 confidence)
                let matching = tx
                    .find_milestones_for_trigger(account_id, trigger)
                    .unwrap_or_default();
                for (milestone_id, milestone_title) in &matching {
                    log::info!(
                        "Noting milestone match '{}' for {} at confidence {:.2} (below 0.8 threshold)",
                        milestone_title, account_id, transition.confidence
                    );
                    let _ = crate::services::signals::emit_and_propagate(
                        tx,
                        engine,
                        "account",
                        account_id,
                        "milestone_match_noted",
                        "lifecycle_transition",
                        Some(&serde_json::json!({
                            "milestone_id": milestone_id,
                            "milestone_title": milestone_title,
                            "confidence": transition.confidence,
                            "trigger": trigger,
                        }).to_string()),
                        transition.confidence,
                    );
                }
            }
        }

        Ok(change_id)
    })
}

pub fn ensure_account_lifecycle_state(
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
) -> Result<(), String> {
    let Some(account) = db.get_account(account_id).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    if !matches!(
        account.account_type,
        crate::db::AccountType::Customer | crate::db::AccountType::Partner
    ) {
        return Ok(());
    }

    if let Some(candidate) = evaluate_lifecycle_transition_candidate(db, &account) {
        let _ = apply_lifecycle_transition(db, engine, account_id, &candidate)?;
        return Ok(());
    }

    if normalized_lifecycle(account.lifecycle.as_deref().unwrap_or_default()) == "renewing" {
        let current_stage = db
            .get_account_renewal_stage(account_id)
            .map_err(|e| e.to_string())?;
        let inferred_stage = infer_renewal_stage(account.contract_end.as_deref());
        if current_stage != inferred_stage {
            db.set_account_renewal_stage(account_id, inferred_stage.as_deref())
                .map_err(|e| e.to_string())?;
            let payload = format!(
                "{{\"stage\":\"{}\"}}",
                inferred_stage.as_deref().unwrap_or("")
            );
            let _ = crate::services::signals::emit_and_propagate(
                db,
                engine,
                "account",
                account_id,
                "renewal_stage_updated",
                "system",
                Some(&payload),
                0.7,
            );
        }
    }

    Ok(())
}

pub fn refresh_lifecycle_states_for_dashboard(
    db: &ActionDb,
    engine: &PropagationEngine,
) -> Result<usize, String> {
    let mut refreshed = 0usize;
    for account in db.get_all_accounts().map_err(|e| e.to_string())? {
        if !matches!(
            account.account_type,
            crate::db::AccountType::Customer | crate::db::AccountType::Partner
        ) {
            continue;
        }

        let near_renewal = account
            .contract_end
            .as_deref()
            .and_then(parse_iso_date)
            .map(|date| (date - Utc::now().date_naive()).num_days() <= 150)
            .unwrap_or(false);
        let has_signals = crate::services::signals::get_for_entity(db, "account", &account.id)
            .map(|signals| {
                signals.iter().any(|signal| {
                    matches!(
                        signal.signal_type.as_str(),
                        "renewal_proximity"
                            | "proactive_renewal_gap"
                            | "renewal_data_updated"
                            | "renewal_at_risk"
                            | "renewal_risk_escalation"
                            | "contract_signed"
                    )
                })
            })
            .unwrap_or(false);
        if !near_renewal && !has_signals {
            continue;
        }

        ensure_account_lifecycle_state(db, engine, &account.id)?;
        refreshed += 1;
    }
    Ok(refreshed)
}

pub fn confirm_lifecycle_change(
    db: &ActionDb,
    engine: &PropagationEngine,
    change_id: i64,
) -> Result<(), String> {
    let change = db
        .get_lifecycle_change(change_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Lifecycle change not found: {change_id}"))?;
    db.set_lifecycle_change_response(change_id, "confirmed", None)
        .map_err(|e| e.to_string())?;
    let _ = db.upsert_signal_weight(&change.source, "account", "lifecycle_transition", 1.0, 0.0);
    crate::services::signals::emit_and_propagate(
        db,
        engine,
        "account",
        &change.account_id,
        "lifecycle_change_confirmed",
        "user_feedback",
        Some(&format!("{{\"change_id\":{change_id}}}")),
        0.95,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;
    Ok(())
}

pub fn correct_account_product(
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    product_id: i64,
    name: &str,
    status: Option<&str>,
    source_to_penalize: &str,
) -> Result<(), String> {
    db.update_account_product(product_id, name, status, None, "user_correction", 1.0)
        .map_err(|e| e.to_string())?;
    let _ = db.upsert_signal_weight(source_to_penalize, "account", "product_adoption", 0.0, 1.0);
    crate::services::signals::emit_and_propagate(
        db,
        engine,
        "account",
        account_id,
        "product_data_updated",
        "user_correction",
        Some(&format!(
            "{{\"product_id\":{product_id},\"name\":\"{name}\"}}"
        )),
        1.0,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;
    Ok(())
}

pub fn correct_lifecycle_change(
    db: &ActionDb,
    engine: &PropagationEngine,
    change_id: i64,
    corrected_lifecycle: &str,
    corrected_stage: Option<&str>,
    notes: Option<&str>,
) -> Result<(), String> {
    let change = db
        .get_lifecycle_change(change_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Lifecycle change not found: {change_id}"))?;
    let transition = LifecycleTransitionCandidate {
        new_lifecycle: corrected_lifecycle.to_string(),
        renewal_stage: corrected_stage.map(str::to_string),
        source: "user_correction".to_string(),
        confidence: 1.0,
        evidence: notes.map(str::to_string),
        completion_trigger: None,
    };
    apply_lifecycle_transition(db, engine, &change.account_id, &transition)?;
    db.set_lifecycle_change_response(change_id, "corrected", notes)
        .map_err(|e| e.to_string())?;
    let _ = db.upsert_signal_weight(&change.source, "account", "lifecycle_transition", 0.0, 1.0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn accept_account_field_conflict(
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    field: &str,
    suggested_value: &str,
    source: &str,
    signal_id: Option<&str>,
) -> Result<(), String> {
    let next_value = if field == "lifecycle" {
        normalized_lifecycle(suggested_value)
    } else {
        suggested_value.to_string()
    };
    update_account_field(db, state, account_id, field, &next_value)?;

    if matches!(field, "arr" | "lifecycle" | "contract_end" | "nps") {
        db.set_account_field_provenance(account_id, field, source, None)
            .map_err(|e| e.to_string())?;
    }

    if let Some(sig_id) = signal_id {
        let feedback_id = uuid::Uuid::new_v4().to_string();
        let feedback_key = account_field_conflict_feedback_key(field, suggested_value);
        let context = serde_json::json!({
            "source": source,
            "signal_id": sig_id,
            "suggested_value": suggested_value,
        })
        .to_string();
        db.insert_intelligence_feedback(
            &crate::db::intelligence_feedback::FeedbackInput {
                id: &feedback_id,
                entity_id: account_id,
                entity_type: "account",
                field: &feedback_key,
                feedback_type: "positive",
                previous_value: None,
                context: Some(&context),
            },
        )?;

        let accepted_signal_id =
            format!("account-field-conflict-accepted-{}", uuid::Uuid::new_v4());
        let _ = crate::signals::bus::supersede_signal(db, sig_id, &accepted_signal_id);
    }

    // I645: Record feedback event for accepted field conflict.
    let _ = db.record_feedback_event(
        account_id,
        "account",
        field,
        signal_id,
        "accept",
        Some(source),
        Some("field_conflict"),
        None,
        Some(suggested_value),
        None,
    );

    let _ = db.upsert_signal_weight(
        source,
        "account",
        &account_field_signal_category(field),
        1.0,
        0.0,
    );

    let payload = serde_json::json!({
        "field": field,
        "source": source,
        "signal_id": signal_id,
        "suggested_value": suggested_value,
    })
    .to_string();
    crate::services::signals::emit_propagate_and_evaluate(
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_conflict_accepted",
        "user_feedback",
        Some(&payload),
        0.95,
        &state.intel_queue,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    Ok(())
}

pub fn dismiss_account_field_conflict(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    field: &str,
    signal_id: &str,
    source: &str,
    suggested_value: Option<&str>,
) -> Result<(), String> {
    let feedback_id = uuid::Uuid::new_v4().to_string();
    let feedback_key = account_field_conflict_feedback_key(field, suggested_value.unwrap_or(""));
    let context = serde_json::json!({
        "source": source,
        "signal_id": signal_id,
        "suggested_value": suggested_value,
    })
    .to_string();
    db.insert_intelligence_feedback(
        &crate::db::intelligence_feedback::FeedbackInput {
            id: &feedback_id,
            entity_id: account_id,
            entity_type: "account",
            field: &feedback_key,
            feedback_type: "negative",
            previous_value: None,
            context: Some(&context),
        },
    )?;

    // I645: Record feedback event + suppression tombstone for rejected field conflict.
    let _ = db.record_feedback_event(
        account_id,
        "account",
        field,
        Some(signal_id),
        "reject",
        Some(source),
        Some("field_conflict"),
        None,
        suggested_value,
        None,
    );
    let _ = db.create_suppression_tombstone(
        account_id,
        field,
        Some(signal_id),
        None,
        Some(source),
        None,
    );

    let _ = db.upsert_signal_weight(
        source,
        "account",
        &account_field_signal_category(field),
        0.0,
        1.0,
    );

    let dismissed_signal_id = format!("account-field-conflict-dismissed-{}", uuid::Uuid::new_v4());
    let _ = crate::signals::bus::supersede_signal(db, signal_id, &dismissed_signal_id);
    let payload = serde_json::json!({
        "field": field,
        "source": source,
        "signal_id": signal_id,
        "suggested_value": suggested_value,
    })
    .to_string();
    crate::services::signals::emit_propagate_and_evaluate(
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_conflict_dismissed",
        "user_feedback",
        Some(&payload),
        0.95,
        &state.intel_queue,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    Ok(())
}

/// Get full detail for an account by ID.
///
/// I644: All data from DB — no filesystem reads on the detail page path.
/// Fetches actions, meetings, people, team, signals, captures, and email signals.
pub async fn get_account_detail(
    account_id: &str,
    state: &AppState,
) -> Result<AccountDetailResult, String> {
    let _config = state.config.read().clone();
    let engine = std::sync::Arc::clone(&state.signals.engine);

    let lifecycle_account_id = account_id.to_string();
    let _ = state
        .db_write(move |db| ensure_account_lifecycle_state(db, &engine, &lifecycle_account_id))
        .await;

    let account_id = account_id.to_string();
    state
        .db_read(move |db| build_account_detail_result(db, &account_id))
        .await
}

/// DOS-229: Synchronous assembly of `AccountDetailResult` against a given DB
/// connection. Extracted so write commands can read back post-write state
/// from the SAME writer connection (avoiding SQLite WAL reader-snapshot lag
/// that makes a follow-up `db_read` return stale rows until app reload).
pub fn build_account_detail_result(
    db: &ActionDb,
    account_id: &str,
) -> Result<AccountDetailResult, String> {
    // Shadow with owned String so the rest of the original closure body —
    // which takes `&account_id` referring to a `String` — still borrows as
    // `&String` (coerces to `&str`). Keeps the diff minimal on extract.
    let account_id: String = account_id.to_string();
            let account = db
                .get_account(&account_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Account not found: {}", account_id))?;

            // I644: Read narrative fields from DB columns (promoted from dashboard.json).
            let overview: Option<crate::accounts::CompanyOverview> = account
                .company_overview
                .as_ref()
                .and_then(|json| serde_json::from_str(json).ok());
            let programs: Vec<crate::accounts::StrategicProgram> = account
                .strategic_programs
                .as_ref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            let notes = account.notes.clone();
            // I644: Intelligence from DB only — no filesystem fallback.
            let mut intelligence = db.get_entity_intelligence(&account_id).ok().flatten();

            // I645: Filter stale items from active display using relevance windows.
            if let Some(ref mut intel) = intelligence {
                intel.risks.retain(|risk| {
                    let sourced = risk.item_source.as_ref().map(|s| s.sourced_at.as_str());
                    match sourced {
                        Some(ts) => crate::intelligence::timeliness::is_within_relevance_window(
                            "active_blocker",
                            ts,
                        ),
                        None => true,
                    }
                });
                intel.recent_wins.retain(|win| {
                    let sourced = win.item_source.as_ref().map(|s| s.sourced_at.as_str());
                    match sourced {
                        Some(ts) => crate::intelligence::timeliness::is_within_relevance_window(
                            "call_theme",
                            ts,
                        ),
                        None => true,
                    }
                });
            }

            let open_actions = db
                .get_account_actions(&account_id)
                .map_err(|e| e.to_string())?;

            let upcoming_meetings: Vec<MeetingSummary> = db
                .get_upcoming_meetings_for_account(&account_id, 5)
                .unwrap_or_default()
                .into_iter()
                .map(|m| MeetingSummary {
                    id: m.id,
                    title: m.title,
                    start_time: m.start_time,
                    meeting_type: m.meeting_type,
                })
                .collect();

            // DOS-233 Codex fix: totals are COUNT(*) without a LIMIT so
            // active accounts don't stall at 10 meetings / transcripts in
            // the About-this-dossier chapter.
            let meeting_total_count = db
                .get_total_meeting_count_for_account(&account_id)
                .unwrap_or(0);
            let transcript_total_count = db
                .get_total_transcript_count_for_account(&account_id)
                .unwrap_or(0);

            let recent_meetings: Vec<MeetingPreview> =
                db.get_meetings_for_account_with_prep(&account_id, 10)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|m| {
                        let prep_context = m.prep_context_json.as_ref().and_then(|json_str| {
                            serde_json::from_str::<PrepContext>(json_str).ok()
                        });
                        MeetingPreview {
                            id: m.id,
                            title: m.title,
                            start_time: m.start_time,
                            meeting_type: m.meeting_type,
                            prep_context,
                        }
                    })
                    .collect();

            let linked_people = db.get_people_for_entity(&account_id).unwrap_or_default();
            let account_team = db
                .get_account_team_internal(&account_id)
                .unwrap_or_default();
            let account_team_import_notes = db
                .get_account_team_import_notes(&account_id)
                .unwrap_or_default();

            let signals = db.get_stakeholder_signals(&account_id).ok();

            let recent_captures = db
                .get_captures_for_account(&account_id, 90)
                .unwrap_or_default();
            // DOS-156: Use direct-only query for The Record display (precision over recall).
            // Propagated person→account signals caused 14.6x fan-out noise.
            let recent_email_signals = db
                .list_direct_email_signals_for_entity(&account_id, 12)
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
            let objectives = db
                .get_account_objectives(&account.id)
                .map_err(|e: crate::db::DbError| e.to_string())?;
            let renewal_stage = db
                .get_account_renewal_stage(&account.id)
                .map_err(|e| e.to_string())?;
            let lifecycle_changes = db
                .get_account_lifecycle_changes(&account.id, 12)
                .map_err(|e| e.to_string())?;
            let field_provenance = db
                .get_account_field_provenance(&account.id)
                .map_err(|e| e.to_string())?;
            let field_conflicts =
                build_account_field_conflicts(db, &account, intelligence.as_ref());
            let products = build_account_products(db, &account.id, intelligence.as_ref());
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
                        account_type: child.account_type.as_db_str().to_string(),
                    }
                })
                .collect();

            // I628 AC5: auto-completed milestones for timeline display (last 90 days)
            let auto_completed_milestones = db
                .get_auto_completed_milestones(&account.id, 90)
                .unwrap_or_default();

            // I649: Technical footprint
            let technical_footprint = db
                .get_account_technical_footprint(&account.id)
                .unwrap_or(None);

            // DB-first stakeholder read model: all stakeholders with provenance
            let stakeholders_full = db
                .get_account_stakeholders_full(&account.id)
                .unwrap_or_default();

            // I644: Source references for promoted account facts
            let source_refs = db.get_account_source_refs(&account.id).unwrap_or_default();

            // DOS-228 Fix 3: current risk-briefing job status for UI progress
            // and retry affordance.
            let risk_briefing_job = db.get_risk_briefing_job(&account.id).ok().flatten();

            // DOS-27: Sentiment journal + sparkline (last 90 days).
            let sentiment_history = db
                .get_sentiment_history(&account.id, 90)
                .unwrap_or_default();
            let sentiment_note = db
                .get_latest_sentiment_note(&account.id)
                .ok()
                .flatten()
                .map(|(note, _)| note);
            let health_sparkline = db
                .get_health_score_sparkline(&account.id, 90)
                .unwrap_or_default();

            // DOS-15: Glean leading-signal enrichment bundle — nullable.
            let glean_signals: Option<
                crate::intelligence::glean_leading_signals::HealthOutlookSignals,
            > = db
                .conn_ref()
                .query_row(
                    "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                    rusqlite::params![&account.id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten()
                .and_then(|json| serde_json::from_str(&json).ok());

            Ok(AccountDetailResult {
                id: account.id,
                name: account.name,
                lifecycle: account.lifecycle,
                arr: account.arr,
                health: account.health,
                nps: account.nps,
                renewal_date: account.contract_end,
                renewal_stage,
                commercial_stage: account.commercial_stage,
                contract_start: account.contract_start,
                company_overview: overview,
                strategic_programs: programs,
                notes,
                open_actions,
                upcoming_meetings,
                recent_meetings,
                meeting_total_count,
                transcript_total_count,
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
                account_type: account.account_type.clone(),
                archived: account.archived,
                objectives,
                lifecycle_changes,
                products,
                field_provenance,
                field_conflicts,
                intelligence,
                auto_completed_milestones,
                technical_footprint,
                stakeholders_full,
                source_refs,
                user_health_sentiment: account.user_health_sentiment,
                sentiment_set_at: account.sentiment_set_at,
                sentiment_note,
                sentiment_history,
                health_sparkline,
                glean_signals,
                risk_briefing_job,
            })
}

/// Update a single structured field on an account.
/// Writes to SQLite, emits signal, then regenerates dashboard.json + dashboard.md.
pub fn update_account_field(
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    field: &str,
    value: &str,
) -> Result<(), String> {
    let normalized_value = if field == "lifecycle" {
        normalized_lifecycle(value)
    } else {
        value.to_string()
    };

    db.update_account_field(account_id, field, &normalized_value)
        .map_err(|e| e.to_string())?;

    if matches!(field, "arr" | "lifecycle" | "contract_end" | "nps") {
        db.set_account_field_provenance(account_id, field, "user_edit", None)
            .map_err(|e| e.to_string())?;
    }

    if field == "lifecycle" {
        let next_stage = if normalized_value == "renewing" {
            db.get_account(account_id)
                .map_err(|e| e.to_string())?
                .and_then(|account| infer_renewal_stage(account.contract_end.as_deref()))
        } else {
            None
        };
        db.set_account_renewal_stage(account_id, next_stage.as_deref())
            .map_err(|e| e.to_string())?;
    } else if field == "contract_end" {
        let next_stage = db
            .get_account(account_id)
            .map_err(|e| e.to_string())?
            .and_then(|account| {
                if normalized_lifecycle(account.lifecycle.as_deref().unwrap_or_default())
                    == "renewing"
                {
                    infer_renewal_stage(account.contract_end.as_deref())
                } else {
                    None
                }
            });
        db.set_account_renewal_stage(account_id, next_stage.as_deref())
            .map_err(|e| e.to_string())?;
    }

    // Emit field update signal + self-healing evaluation (I308, I410)
    crate::services::signals::emit_propagate_and_evaluate(
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_updated",
        "user_edit",
        Some(&format!(
            "{{\"field\":\"{}\",\"value\":\"{}\"}}",
            field,
            normalized_value.replace('"', "\\\"")
        )),
        0.8,
        &state.intel_queue,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    // Self-healing: record user correction for Clay-enrichable fields (I409)
    if matches!(field, "lifecycle" | "arr" | "health" | "nps") {
        crate::self_healing::feedback::record_enrichment_correction(
            db, account_id, "account", "clay",
        );
    }

    // DOS-228 Fix 2: Health-relevant field edits schedule a DEBOUNCED recompute
    // on the backend. 10 rapid edits within the debounce window coalesce into
    // exactly one recompute, reflecting the last committed state. This replaces
    // the previous synchronous recompute, which fired once per edit and could
    // not be trusted (AI agents, chat, and automation bypass the UI debounce).
    if is_health_relevant_field(field) {
        crate::services::health_debouncer::schedule_recompute(state, account_id);
    }

    // Regenerate workspace files
    if let Ok(Some(account)) = db.get_account(account_id) {
        let config = state.config.read();
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);

            // DOS-44: When the name changes, rename the workspace directory to match.
            // The old directory path is resolved from the account's current tracker_path
            // or the old name; the new path uses the updated name.
            if field == "name" {
                let old_dir = crate::accounts::resolve_account_dir(workspace, &account);
                let new_dir_name =
                    crate::processor::transcript::sanitize_account_dir(&normalized_value);
                let new_dir = if let Some(parent) = old_dir.parent() {
                    parent.join(&new_dir_name)
                } else {
                    workspace.join("Accounts").join(&new_dir_name)
                };

                if old_dir.exists() && old_dir != new_dir && !new_dir.exists() {
                    if let Err(e) = std::fs::rename(&old_dir, &new_dir) {
                        log::warn!(
                            "DOS-44: Failed to rename account dir '{}' → '{}': {}",
                            old_dir.display(),
                            new_dir.display(),
                            e
                        );
                    } else {
                        log::info!(
                            "DOS-44: Renamed account dir '{}' → '{}'",
                            old_dir.display(),
                            new_dir.display()
                        );
                        // Update tracker_path in DB
                        let new_tracker = format!(
                            "Accounts/{}",
                            new_dir_name
                        );
                        let _ = db.update_account_field(
                            account_id,
                            "tracker_path",
                            &new_tracker,
                        );
                    }
                }
            }

            // Re-fetch account after potential tracker_path update
            let account = db
                .get_account(account_id)
                .ok()
                .flatten()
                .unwrap_or(account);
            let _ = crate::accounts::write_account_json(workspace, &account, None, db);
            let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);
        }
    }

    Ok(())
}

/// DOS-27: Sentiment values that represent elevated risk.
/// Transitioning INTO one of these from a non-risk value triggers
/// background risk-briefing generation.
const RISK_SENTIMENTS: &[&str] = &["at_risk", "critical"];

/// DOS-110 / DOS-27: Set the user's manual health sentiment on an account.
/// Writes the current sentiment + timestamp, appends a journal entry (value +
/// optional note + computed band snapshot), emits a `field_updated` signal,
/// and — on transition into at_risk/critical — enqueues a background risk
/// briefing generation.
pub fn set_user_health_sentiment(
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    sentiment: &str,
    note: Option<&str>,
) -> Result<AccountDetailResult, String> {
    const VALID_SENTIMENTS: &[&str] =
        &["strong", "on_track", "concerning", "at_risk", "critical"];
    if !VALID_SENTIMENTS.contains(&sentiment) {
        return Err(format!(
            "Invalid sentiment '{}'. Must be one of: {}",
            sentiment,
            VALID_SENTIMENTS.join(", ")
        ));
    }

    let previous = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .and_then(|a| a.user_health_sentiment);

    let now = Utc::now().to_rfc3339();
    db.update_account_field(account_id, "user_health_sentiment", sentiment)
        .map_err(|e| e.to_string())?;
    db.update_account_field(account_id, "sentiment_set_at", &now)
        .map_err(|e| e.to_string())?;

    // Snapshot computed health at set-time for divergence analysis.
    let (computed_band, computed_score) = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .map(|acct| {
            let health = crate::intelligence::health_scoring::compute_account_health(
                db, &acct, None,
            );
            (Some(health.band), Some(health.score))
        })
        .unwrap_or((None, None));

    db.insert_sentiment_journal_entry(
        account_id,
        sentiment,
        note,
        computed_band.as_deref(),
        computed_score,
    )
    .map_err(|e| e.to_string())?;

    // Emit field_updated signal with high confidence (user-initiated)
    crate::services::signals::emit_propagate_and_evaluate(
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_updated",
        "user_edit",
        Some(&format!(
            "{{\"field\":\"user_health_sentiment\",\"value\":\"{}\"}}",
            sentiment
        )),
        0.95,
        &state.intel_queue,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    // DOS-27: Transition INTO at_risk/critical enqueues background risk briefing.
    // Not every save — only the transition. Silent on failure.
    let transitioning_into_risk = RISK_SENTIMENTS.contains(&sentiment)
        && !previous
            .as_deref()
            .map(|p| RISK_SENTIMENTS.contains(&p))
            .unwrap_or(false);

    if transitioning_into_risk {
        enqueue_risk_briefing(state, account_id.to_string());
    }

    // DOS-229: Read back the updated detail on the SAME writer connection so
    // the frontend sees post-write state immediately. A follow-up `db_read`
    // hits a different pool connection whose WAL snapshot can lag.
    build_account_detail_result(db, account_id)
}

/// DOS-27 / DOS-228 Fix 3: Spawn a background risk-briefing generation task.
/// Runs on the Tauri async runtime. Job lifecycle is persisted to
/// `risk_briefing_jobs` so the UI can render progress and offer a retry on
/// failure instead of the old log-only failure mode.
pub(crate) fn enqueue_risk_briefing(state: &std::sync::Arc<AppState>, account_id: String) {
    let state_clone = state.clone();
    let handle = state.app_handle();

    // DOS-228: record enqueue BEFORE spawning so the UI sees the status even
    // if the spawned task is slow to start. Write through db_write for
    // serialization; ignore errors (best-effort status tracking, never
    // blocks the user's sentiment save).
    let enqueue_id = account_id.clone();
    let enqueue_state = state.clone();
    tauri::async_runtime::spawn(async move {
        let id_for_write = enqueue_id.clone();
        if let Err(e) = enqueue_state
            .db_write(move |db| {
                db.upsert_risk_briefing_job_enqueued(&id_for_write)
                    .map_err(|e| e.to_string())
            })
            .await
        {
            log::warn!(
                "DOS-228: failed to persist risk briefing 'enqueued' status for {}: {}",
                enqueue_id,
                e
            );
        }
    });

    tauri::async_runtime::spawn(async move {
        log::info!(
            "DOS-27: enqueueing background risk briefing for account {}",
            account_id
        );

        // Transition enqueued → running.
        let running_id = account_id.clone();
        let running_state = state_clone.clone();
        let _ = running_state
            .db_write(move |db| {
                db.mark_risk_briefing_job_running(&running_id)
                    .map_err(|e| e.to_string())
            })
            .await;

        let outcome = crate::services::intelligence::generate_risk_briefing(
            &state_clone,
            &account_id,
            handle,
        )
        .await;

        // Terminal transition: record complete or failed. The error message
        // is persisted so the UI can explain WHY a retry is offered.
        let terminal_id = account_id.clone();
        let terminal_state = state_clone.clone();
        let terminal_outcome = outcome
            .as_ref()
            .map(|_| ())
            .map_err(|e| e.clone());
        let _ = terminal_state
            .db_write(move |db| match &terminal_outcome {
                Ok(()) => db
                    .mark_risk_briefing_job_complete(&terminal_id)
                    .map_err(|e| e.to_string()),
                Err(msg) => db
                    .mark_risk_briefing_job_failed(&terminal_id, msg)
                    .map_err(|e| e.to_string()),
            })
            .await;

        match outcome {
            Ok(_) => log::info!(
                "DOS-27: risk briefing generated for account {}",
                account_id
            ),
            Err(e) => log::warn!(
                "DOS-27: risk briefing generation failed for account {}: {}",
                account_id,
                e
            ),
        }
    });
}

/// DOS-228 Fix 3: Re-enqueue a risk briefing generation for an account.
///
/// Intended for UI "retry" buttons that surface after a failed job. Only the
/// latest attempt is retained in `risk_briefing_jobs`; calling this on an
/// account with a `complete` job is also allowed — it overwrites the row and
/// regenerates the briefing (useful when underlying intel has changed).
///
/// Returns immediately; the regeneration runs on the async runtime and
/// status transitions are visible via `get_account_detail`.
pub fn retry_risk_briefing(
    state: &std::sync::Arc<AppState>,
    account_id: &str,
) -> Result<(), String> {
    enqueue_risk_briefing(state, account_id.to_string());
    Ok(())
}

/// DOS-27: Field edits to health-relevant columns trigger a synchronous
/// health recompute so the UI reflects the new band immediately.
/// Rapid-fire edits are naturally coalesced: the frontend debounces at 2s
/// before calling the backend command, so we don't duplicate the debounce here.
pub fn is_health_relevant_field(field: &str) -> bool {
    matches!(
        field,
        "health" | "lifecycle" | "contract_end" | "arr" | "nps"
    )
}

/// Update account notes (narrative field).
/// Writes to SQLite, then regenerates dashboard.json + dashboard.md.
pub fn update_account_notes(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    notes: &str,
) -> Result<(), String> {
    db.update_account_field(account_id, "notes", notes)
        .map_err(|e| e.to_string())?;

    let account = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let config = state.config.read();
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let _ = crate::accounts::write_account_json(workspace, &account, None, db);
    let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);

    // Emit field update signal (I377)
    crate::services::signals::emit_and_propagate(
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_updated",
        "user_edit",
        Some(&format!(
            "{{\"field\":\"notes\",\"value\":\"{}\"}}",
            notes
                .chars()
                .take(100)
                .collect::<String>()
                .replace('"', "\\\"")
        )),
        0.8,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    Ok(())
}

/// Update account strategic programs (narrative field).
/// Validates JSON, writes to SQLite, then regenerates dashboard.json + dashboard.md.
pub fn update_account_programs(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    programs_json: &str,
) -> Result<(), String> {
    let _: Vec<crate::accounts::StrategicProgram> =
        serde_json::from_str(programs_json).map_err(|e| format!("Invalid programs JSON: {}", e))?;

    db.update_account_field(account_id, "strategic_programs", programs_json)
        .map_err(|e| e.to_string())?;

    let account = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let config = state.config.read();
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let _ = crate::accounts::write_account_json(workspace, &account, None, db);
    let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);

    // Emit field update signal (I377)
    crate::services::signals::emit_and_propagate(
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_updated",
        "user_edit",
        Some("{\"field\":\"strategic_programs\"}"),
        0.8,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    Ok(())
}

/// Create a new account. Creates SQLite record + workspace files.
/// If `parent_id` is provided, creates a child (BU) account under that parent.
pub fn create_account(
    db: &ActionDb,
    state: &AppState,
    name: &str,
    parent_id: Option<&str>,
    explicit_type: Option<crate::db::AccountType>,
) -> Result<String, String> {
    let name = crate::util::validate_entity_name(name)?.to_string();

    let (id, tracker_path, account_type) = if let Some(pid) = parent_id {
        let parent = db
            .get_account(pid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Parent account not found: {}", pid))?;
        let child_id = format!("{}--{}", pid, crate::util::slugify(&name));
        let parent_dir = parent
            .tracker_path
            .unwrap_or_else(|| format!("Accounts/{}", parent.name));
        let tp = format!("{}/{}", parent_dir, name);
        // Explicit type overrides parent inheritance
        let at = explicit_type.unwrap_or_else(|| parent.account_type.clone());
        (child_id, tp, at)
    } else {
        let id = crate::util::slugify(&name);
        let at = explicit_type.unwrap_or(crate::db::AccountType::Customer);
        (id, format!("Accounts/{}", name), at)
    };

    let now = chrono::Utc::now().to_rfc3339();

    let account = crate::db::DbAccount {
        id: id.clone(),
        name: name.clone(),
        tracker_path: Some(tracker_path),
        parent_id: parent_id.map(|s| s.to_string()),
        account_type,
        updated_at: now,
        ..Default::default()
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;
    if let Some(pid) = parent_id {
        let _ = db.copy_account_domains(pid, &account.id);
    }

    // Create workspace files + directory template (ADR-0059)
    let config = state.config.read();
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, &name, "account");
        let _ = crate::accounts::write_account_json(workspace, &account, None, db);
        let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);
    }

    // Self-healing: initialize quality row for new entity (I406)
    crate::self_healing::quality::ensure_quality_row(db, &id, "account");

    Ok(id)
}

/// Archive or unarchive an account with signal emission. Cascades to children when archiving.
pub fn archive_account(
    db: &ActionDb,
    state: &crate::state::AppState,
    id: &str,
    archived: bool,
) -> Result<(), String> {
    let signal_type = if archived {
        "entity_archived"
    } else {
        "entity_unarchived"
    };
    db.with_transaction(|tx| {
        tx.archive_account(id, archived)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            &state.signals.engine,
            "account",
            id,
            signal_type,
            "user_action",
            None,
            0.9,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Merge source account into target account with signal emission.
pub fn merge_accounts(
    db: &ActionDb,
    state: &crate::state::AppState,
    from_id: &str,
    into_id: &str,
) -> Result<crate::db::MergeResult, String> {
    db.with_transaction(|tx| {
        let result = tx
            .merge_accounts(from_id, into_id)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            &state.signals.engine,
            "account",
            into_id,
            "entity_merged",
            "user_action",
            Some(&format!("{{\"merged_from\":\"{}\"}}", from_id)),
            0.9,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(result)
    })
}

/// Restore an archived account with optional child restoration.
pub fn restore_account(
    db: &ActionDb,
    account_id: &str,
    restore_children: bool,
) -> Result<usize, String> {
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
    db.with_transaction(|tx| {
        tx.add_account_team_member(account_id, person_id, &role)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            &state.signals.engine,
            "account",
            account_id,
            "team_member_added",
            "user_action",
            Some(&format!(
                "{{\"person_id\":\"{}\",\"role\":\"{}\"}}",
                person_id, role
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Replace all roles for a team member (single-select role change).
pub fn set_team_member_role(
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    person_id: &str,
    new_role: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.set_team_member_role(account_id, person_id, new_role)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            &state.signals.engine,
            "account",
            account_id,
            "team_member_role_changed",
            "user_action",
            Some(&format!(
                "{{\"person_id\":\"{}\",\"role\":\"{}\"}}",
                person_id, new_role
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Remove a person-role pair from an account team with signal emission.
pub fn remove_account_team_member(
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.remove_account_team_member(account_id, person_id, role)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            &state.signals.engine,
            "account",
            account_id,
            "team_member_removed",
            "user_action",
            Some(&format!(
                "{{\"person_id\":\"{}\",\"role\":\"{}\"}}",
                person_id, role
            )),
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
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
    db.with_transaction(|tx| {
        let event_id = tx
            .record_account_event(account_id, event_type, event_date, arr_impact, notes)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            &state.signals.engine,
            "account",
            account_id,
            "account_event_recorded",
            "user_action",
            Some(&format!(
                "{{\"event_type\":\"{}\",\"event_date\":\"{}\"}}",
                event_type, event_date
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        let auto_completed = tx
            .complete_milestones_for_account_event(account_id, event_type)
            .map_err(|e| e.to_string())?;
        for completed in auto_completed.milestones {
            crate::services::signals::emit_and_propagate(
                tx,
                &state.signals.engine,
                "account",
                account_id,
                "milestone_completed",
                "lifecycle_event",
                Some(&format!(
                    "{{\"milestone_id\":\"{}\",\"objective_id\":\"{}\",\"event_type\":\"{}\"}}",
                    completed.id, completed.objective_id, event_type
                )),
                0.9,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }
        for completed in auto_completed.objectives {
            crate::services::signals::emit_and_propagate(
                tx,
                &state.signals.engine,
                "account",
                account_id,
                "objective_completed",
                "lifecycle_event",
                Some(&format!(
                    "{{\"objective_id\":\"{}\",\"event_type\":\"{}\"}}",
                    completed.id, event_type
                )),
                0.95,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }
        Ok(event_id)
    })
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
            tracker_path: Some(format!("Accounts/{}", name)),
            updated_at: now,
            ..Default::default()
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
/// For parent accounts, rolls up child ARR into `arr` when the parent's own ARR is unset.
pub fn account_to_list_item(
    a: &crate::db::DbAccount,
    db: &ActionDb,
    child_count: usize,
) -> AccountListItem {
    let open_action_count = db
        .get_account_actions(&a.id)
        .map(|actions| actions.len())
        .unwrap_or(0);

    // Roll up child ARR for parent accounts with no direct ARR
    let arr = if a.arr.is_some() {
        a.arr
    } else if child_count > 0 {
        db.get_parent_aggregate(&a.id)
            .ok()
            .and_then(|agg| agg.total_arr)
    } else {
        None
    };

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

    AccountListItem {
        id: a.id.clone(),
        name: a.name.clone(),
        lifecycle: a.lifecycle.clone(),
        arr,
        health: a.health.clone(),
        nps: a.nps,
        renewal_date: a.contract_end.clone(),
        open_action_count,
        days_since_last_meeting,
        parent_id: a.parent_id.clone(),
        parent_name: None,
        child_count,
        is_parent: child_count > 0,
        account_type: a.account_type.clone(),
        archived: a.archived,
        user_health_sentiment: a.user_health_sentiment.clone(),
        sentiment_set_at: a.sentiment_set_at.clone(),
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
                account_type: a.account_type.clone(),
            }
        })
        .collect();

    Ok(items)
}

// ── I452: Account mutation handlers extracted from commands.rs ──────────

/// Create the internal organization (root account + initial team + colleagues).
///
/// Wraps all DB writes in a transaction. Filesystem writes are best-effort after commit.
pub async fn create_internal_organization(
    state: &AppState,
    company_name: &str,
    domains: &[String],
    team_name: &str,
    colleagues: &[crate::commands::TeamColleagueInput],
    existing_person_ids: &[String],
) -> Result<crate::commands::CreateInternalOrganizationResult, String> {
    let company_name = crate::util::validate_entity_name(company_name)?.to_string();
    let team_name = crate::util::validate_entity_name(team_name)?.to_string();
    let domains = crate::helpers::normalize_domains(domains);
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("Config not loaded")?;

    let colleagues = colleagues.to_vec();
    let existing_person_ids = existing_person_ids.to_vec();
    let domains_clone = domains.clone();
    let company_name_clone = company_name.clone();
    let team_name_clone = team_name.clone();

    let (root_account, initial_team, created_people, updated_people) = state
        .db_write(move |db| {
            let workspace = std::path::Path::new(&workspace_path);

            let (root_account, initial_team, created_people, updated_people) = db
                .with_transaction(|db| {
                    if db
                        .get_internal_root_account()
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        return Err("Internal organization already exists".to_string());
                    }

                    let mut root_id =
                        format!("internal-{}", crate::util::slugify(&company_name_clone));
                    let mut suffix = 2usize;
                    while db
                        .get_account(&root_id)
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        root_id = format!(
                            "internal-{}-{}",
                            crate::util::slugify(&company_name_clone),
                            suffix
                        );
                        suffix += 1;
                    }

                    let now = chrono::Utc::now().to_rfc3339();
                    let root_account = crate::db::DbAccount {
                        id: root_id.clone(),
                        name: company_name_clone.clone(),
                        lifecycle: Some("active".to_string()),
                        health: Some("green".to_string()),
                        tracker_path: Some(format!("Internal/{}", company_name_clone)),
                        account_type: crate::db::AccountType::Internal,
                        updated_at: now,
                        ..Default::default()
                    };
                    db.upsert_account(&root_account)
                        .map_err(|e| e.to_string())?;
                    db.set_account_domains(&root_account.id, &domains_clone)
                        .map_err(|e| e.to_string())?;

                    let initial_team = create_child_account_record(
                        db,
                        None,
                        &root_account,
                        &team_name_clone,
                        None,
                        None,
                    )?;
                    db.copy_account_domains(&root_account.id, &initial_team.id)
                        .map_err(|e| e.to_string())?;

                    let mut created_people: Vec<crate::db::DbPerson> = Vec::new();
                    for colleague in &colleagues {
                        let email = match crate::util::validate_email(&colleague.email) {
                            Ok(e) => e,
                            Err(_) => continue,
                        };
                        let person_id = crate::util::slugify(&email);
                        let now = chrono::Utc::now().to_rfc3339();
                        let person = crate::db::DbPerson {
                            id: person_id.clone(),
                            email: email.clone(),
                            name: colleague.name.trim().to_string(),
                            organization: Some(company_name_clone.clone()),
                            role: colleague.title.clone(),
                            relationship: "internal".to_string(),
                            notes: None,
                            tracker_path: None,
                            last_seen: None,
                            first_seen: Some(now.clone()),
                            meeting_count: 0,
                            updated_at: now,
                            archived: false,
                            linkedin_url: None,
                            twitter_handle: None,
                            phone: None,
                            photo_url: None,
                            bio: None,
                            title_history: None,
                            company_industry: None,
                            company_size: None,
                            company_hq: None,
                            last_enriched_at: None,
                            enrichment_sources: None,
                        };
                        db.upsert_person(&person).map_err(|e| e.to_string())?;
                        db.link_person_to_entity(&person_id, &root_account.id, "member")
                            .map_err(|e| e.to_string())?;
                        db.link_person_to_entity(&person_id, &initial_team.id, "member")
                            .map_err(|e| e.to_string())?;
                        created_people.push(person);
                    }

                    let mut updated_people: Vec<crate::db::DbPerson> = Vec::new();
                    for person_id in &existing_person_ids {
                        if let Ok(Some(mut person)) = db.get_person(person_id) {
                            if person.relationship != "internal" {
                                person.relationship = "internal".to_string();
                                person.organization = Some(company_name_clone.clone());
                                db.upsert_person(&person).map_err(|e| e.to_string())?;
                                updated_people.push(person);
                            }
                            db.link_person_to_entity(person_id, &root_account.id, "member")
                                .map_err(|e| e.to_string())?;
                            db.link_person_to_entity(person_id, &initial_team.id, "member")
                                .map_err(|e| e.to_string())?;
                        }
                    }

                    Ok((root_account, initial_team, created_people, updated_people))
                })?;

            // Filesystem writes (best-effort, outside transaction)
            let root_dir = crate::accounts::resolve_account_dir(workspace, &root_account);
            let _ = std::fs::create_dir_all(&root_dir);
            let _ =
                crate::util::bootstrap_entity_directory(&root_dir, &company_name_clone, "account");
            let _ = crate::accounts::write_account_json(workspace, &root_account, None, db);
            let _ = crate::accounts::write_account_markdown(workspace, &root_account, None, db);

            let team_dir = crate::accounts::resolve_account_dir(workspace, &initial_team);
            let _ = std::fs::create_dir_all(&team_dir);
            let _ = crate::util::bootstrap_entity_directory(&team_dir, &team_name_clone, "account");
            let _ = crate::accounts::write_account_json(workspace, &initial_team, None, db);
            let _ = crate::accounts::write_account_markdown(workspace, &initial_team, None, db);

            for person in &created_people {
                let _ = crate::people::write_person_json(workspace, person, db);
                let _ = crate::people::write_person_markdown(workspace, person, db);
            }
            for person in &updated_people {
                let _ = crate::people::write_person_json(workspace, person, db);
                let _ = crate::people::write_person_markdown(workspace, person, db);
            }

            Ok((root_account, initial_team, created_people, updated_people))
        })
        .await?;

    // Suppress unused warnings — these were used inside the closure for filesystem writes
    let _ = &created_people;
    let _ = &updated_people;

    crate::state::create_or_update_config(state, |config| {
        config.internal_team_setup_completed = true;
        config.internal_team_setup_version = 1;
        config.internal_org_account_id = Some(root_account.id.clone());
        if config.user_company.is_none() {
            config.user_company = Some(company_name.clone());
        }
        if !domains.is_empty() {
            config.user_domain = domains.first().cloned();
            config.user_domains = Some(domains.clone());
        }
    })?;

    Ok(crate::commands::CreateInternalOrganizationResult {
        root_account_id: root_account.id,
        initial_team_id: initial_team.id,
    })
}

/// Create a child account under a parent with intel queue enqueue.
pub async fn create_child_account_cmd(
    state: &AppState,
    parent_id: &str,
    name: &str,
    description: Option<&str>,
    owner_person_id: Option<&str>,
) -> Result<crate::commands::CreateChildAccountResult, String> {
    let name = crate::util::validate_entity_name(name)?.to_string();
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone());

    let parent_id = parent_id.to_string();
    let description = description.map(|s| s.to_string());
    let owner_person_id = owner_person_id.map(|s| s.to_string());

    let child_id = state
        .db_write(move |db| {
            let workspace = workspace_path.as_deref().map(std::path::Path::new);
            let parent = db
                .get_account(&parent_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Parent account not found: {}", parent_id))?;
            let child = create_child_account_record(
                db,
                workspace,
                &parent,
                &name,
                description.as_deref(),
                owner_person_id.as_deref(),
            )?;
            Ok(child.id)
        })
        .await?;

    state
        .intel_queue
        .enqueue(crate::intel_queue::IntelRequest::new(
            child_id.clone(),
            "account".to_string(),
            crate::intel_queue::IntelPriority::ContentChange,
        ));
    state.integrations.intel_queue_wake.notify_one();

    Ok(crate::commands::CreateChildAccountResult { id: child_id })
}

/// Backfill internal meeting → account associations for meetings missing entity links.
pub fn backfill_internal_meeting_associations(db: &ActionDb) -> Result<usize, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT m.id, m.title, m.attendees
             FROM meetings m
             LEFT JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_type = 'account'
             WHERE m.meeting_type IN ('internal', 'team_sync', 'one_on_one')
               AND me.meeting_id IS NULL",
        )
        .map_err(|e| e.to_string())?;
    let meetings: Vec<(String, String, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut updated = 0usize;
    for (meeting_id, title, attendees) in meetings {
        let Some(account) = infer_internal_account_for_meeting(db, &title, attendees.as_deref())
        else {
            continue;
        };
        let _ = db.link_meeting_entity(&meeting_id, &account.id, "account");
        let _ = db.cascade_meeting_entity_to_people(&meeting_id, Some(&account.id), None);
        updated += 1;
    }

    Ok(updated)
}

// =============================================================================
// I652 Phase 2: Person-first stakeholder mutations
// =============================================================================

/// Update engagement level for a stakeholder with signal emission.
pub fn update_stakeholder_engagement(
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    person_id: &str,
    engagement: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE account_stakeholders
                 SET engagement = ?1, data_source_engagement = 'user'
                 WHERE account_id = ?2 AND person_id = ?3",
                rusqlite::params![engagement, account_id, person_id],
            )
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "account",
            account_id,
            "stakeholder_engagement_updated",
            "user_action",
            Some(&format!(
                "{{\"person_id\":\"{}\",\"engagement\":\"{}\"}}",
                person_id, engagement
            )),
            1.0,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Update assessment text for a stakeholder with signal emission.
pub fn update_stakeholder_assessment(
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    person_id: &str,
    assessment: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE account_stakeholders
                 SET assessment = ?1, data_source_assessment = 'user'
                 WHERE account_id = ?2 AND person_id = ?3",
                rusqlite::params![assessment, account_id, person_id],
            )
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "account",
            account_id,
            "stakeholder_assessment_updated",
            "user_action",
            Some(&format!("{{\"person_id\":\"{}\"}}", person_id)),
            1.0,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Add a role to a stakeholder (multi-role — doesn't replace existing roles).
pub fn add_stakeholder_role(
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    let role = role.trim().to_lowercase();
    if role.is_empty() {
        return Err("Role is required".to_string());
    }
    db.with_transaction(|tx| {
        let now = chrono::Utc::now().to_rfc3339();
        tx.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source, created_at)
                 VALUES (?1, ?2, ?3, 'user', ?4)
                 ON CONFLICT(account_id, person_id, role) DO UPDATE SET
                    data_source = 'user'",
                rusqlite::params![account_id, person_id, role, now],
            )
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "account",
            account_id,
            "stakeholder_role_added",
            "user_action",
            Some(&format!(
                "{{\"person_id\":\"{}\",\"role\":\"{}\"}}",
                person_id, role
            )),
            0.9,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Remove a specific role from a stakeholder.
pub fn remove_stakeholder_role(
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "DELETE FROM account_stakeholder_roles
                 WHERE account_id = ?1 AND person_id = ?2 AND role = ?3",
                rusqlite::params![account_id, person_id, role],
            )
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "account",
            account_id,
            "stakeholder_role_removed",
            "user_action",
            Some(&format!(
                "{{\"person_id\":\"{}\",\"role\":\"{}\"}}",
                person_id, role
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Accept a stakeholder suggestion: create person if needed, add to account.
pub fn accept_stakeholder_suggestion(
    db: &ActionDb,
    state: &crate::state::AppState,
    suggestion_id: i64,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        let suggestion = tx
            .get_stakeholder_suggestion(suggestion_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Suggestion not found: {suggestion_id}"))?;

        if suggestion.status != "pending" {
            return Err(format!(
                "Suggestion {} is already {}",
                suggestion_id, suggestion.status
            ));
        }

        // Resolve person_id: use existing, or find/create from email
        let person_id = if let Some(pid) = suggestion.person_id.as_deref() {
            pid.to_string()
        } else if let Some(email) = suggestion.suggested_email.as_deref() {
            let name = suggestion
                .suggested_name
                .as_deref()
                .unwrap_or_else(|| email.split('@').next().unwrap_or("Unknown"));
            let config = state
                .config
                .read()
                .clone()
                .ok_or("Config not loaded")?;
            let user_domains = config.resolved_user_domains();
            let resolution =
                tx.find_or_create_person(Some(email), name, None, "external", &user_domains)
                    .map_err(|e| e.to_string())?;
            match resolution {
                crate::db::people::PersonResolution::FoundByEmail(p)
                | crate::db::people::PersonResolution::Created(p) => p.id,
                crate::db::people::PersonResolution::FoundByName { person, .. } => person.id,
            }
        } else if let Some(name) = suggestion.suggested_name.as_deref() {
            // Name-only suggestion (no email): find by name or create with synthetic email
            let config = state
                .config
                .read()
                .clone()
                .ok_or("Config not loaded")?;
            let user_domains = config.resolved_user_domains();
            let resolution =
                tx.find_or_create_person(None, name, None, "external", &user_domains)
                    .map_err(|e| e.to_string())?;
            match resolution {
                crate::db::people::PersonResolution::FoundByEmail(p)
                | crate::db::people::PersonResolution::Created(p) => p.id,
                crate::db::people::PersonResolution::FoundByName { person, .. } => person.id,
            }
        } else {
            return Err("Cannot accept suggestion: no person_id, email, or name".to_string());
        };

        // Ensure stakeholder link exists
        let now = chrono::Utc::now().to_rfc3339();
        tx.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source, created_at)
                 VALUES (?1, ?2, 'user', ?3)
                 ON CONFLICT(account_id, person_id) DO UPDATE SET data_source = 'user'",
                rusqlite::params![suggestion.account_id, person_id, now],
            )
            .map_err(|e| e.to_string())?;

        // Add suggested role if present
        if let Some(role) = suggestion.suggested_role.as_deref() {
            let role = role.trim().to_lowercase();
            if !role.is_empty() {
                tx.conn_ref()
                    .execute(
                        "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source, created_at)
                         VALUES (?1, ?2, ?3, 'user', ?4)
                         ON CONFLICT(account_id, person_id, role) DO UPDATE SET data_source = 'user'",
                        rusqlite::params![suggestion.account_id, person_id, role, now],
                    )
                    .map_err(|e| e.to_string())?;
            }
        }

        // Set engagement if suggested
        if let Some(engagement) = suggestion.suggested_engagement.as_deref() {
            tx.conn_ref()
                .execute(
                    "UPDATE account_stakeholders
                     SET engagement = ?1, data_source_engagement = 'user'
                     WHERE account_id = ?2 AND person_id = ?3",
                    rusqlite::params![engagement, suggestion.account_id, person_id],
                )
                .map_err(|e| e.to_string())?;
        }

        // Mark suggestion as accepted
        tx.conn_ref()
            .execute(
                "UPDATE stakeholder_suggestions
                 SET status = 'accepted', resolved_at = datetime('now')
                 WHERE id = ?1",
                rusqlite::params![suggestion_id],
            )
            .map_err(|e| e.to_string())?;

        crate::services::signals::emit_and_propagate(
            tx,
            &state.signals.engine,
            "account",
            &suggestion.account_id,
            "stakeholder_suggestion_accepted",
            "user_action",
            Some(&format!(
                "{{\"person_id\":\"{}\",\"source\":\"{}\"}}",
                person_id, suggestion.source
            )),
            1.0,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Dismiss a stakeholder suggestion.
pub fn dismiss_stakeholder_suggestion(
    db: &ActionDb,
    engine: &PropagationEngine,
    suggestion_id: i64,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        let suggestion = tx
            .get_stakeholder_suggestion(suggestion_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Suggestion not found: {suggestion_id}"))?;

        tx.conn_ref()
            .execute(
                "UPDATE stakeholder_suggestions
                 SET status = 'dismissed', resolved_at = datetime('now')
                 WHERE id = ?1",
                rusqlite::params![suggestion_id],
            )
            .map_err(|e| e.to_string())?;

        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "account",
            &suggestion.account_id,
            "stakeholder_suggestion_dismissed",
            "user_action",
            Some(&format!("{{\"suggestion_id\":{}}}", suggestion_id)),
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::signals::propagation::PropagationEngine;
    use rusqlite::params;

    fn make_account(id: &str, name: &str) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some(format!("Accounts/{name}")),
            parent_id: None,
            account_type: AccountType::Customer,
            updated_at: chrono::Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        }
    }

    fn signal_count(db: &crate::db::ActionDb, entity_id: &str, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND signal_type = ?2",
                params![entity_id, signal_type],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    /// Test account creation at the DB level (create_account needs AppState for workspace files,
    /// so we test the underlying upsert + signal emission pattern directly).
    #[test]
    fn test_create_account_db_level() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-new", "New Corp");

        // Use the mutations service which wraps upsert + signal
        crate::services::mutations::upsert_account(&db, &engine, &account).expect("upsert_account");

        let name: String = db
            .conn_ref()
            .query_row(
                "SELECT name FROM accounts WHERE id = 'acc-new'",
                [],
                |row| row.get(0),
            )
            .expect("query account name");
        assert_eq!(name, "New Corp");
    }

    /// Test archive + restore toggle at DB level with signal verification.
    #[test]
    fn test_archive_restore_account() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-ar", "Archive Me");
        db.upsert_account(&account).unwrap();

        // Archive via DB transaction (mirrors archive_account service without AppState)
        db.with_transaction(|tx| {
            tx.archive_account("acc-ar", true)
                .map_err(|e| e.to_string())?;
            crate::services::signals::emit_and_propagate(
                tx,
                &engine,
                "account",
                "acc-ar",
                "entity_archived",
                "user_action",
                None,
                0.9,
            )
            .map_err(|e| format!("{e}"))?;
            Ok(())
        })
        .expect("archive");

        let archived: bool = db
            .conn_ref()
            .query_row(
                "SELECT archived FROM accounts WHERE id = 'acc-ar'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(archived, "Account should be archived");
        assert!(
            signal_count(&db, "acc-ar", "entity_archived") > 0,
            "Expected entity_archived signal"
        );

        // Restore
        super::restore_account(&db, "acc-ar", false).expect("restore");
        let archived_after: bool = db
            .conn_ref()
            .query_row(
                "SELECT archived FROM accounts WHERE id = 'acc-ar'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!archived_after, "Account should be restored");
    }

    /// Test team member add/remove at DB level with signal emission.
    #[test]
    fn test_add_remove_team_member() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-tm", "Team Corp");
        db.upsert_account(&account).unwrap();

        // Seed a person
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p-tm', 'tm@x.com', 'TeamPerson', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();

        // Add team member via DB transaction (mirrors add_account_team_member without AppState)
        db.with_transaction(|tx| {
            tx.add_account_team_member("acc-tm", "p-tm", "csm")
                .map_err(|e| e.to_string())?;
            crate::services::signals::emit_and_propagate(
                tx,
                &engine,
                "account",
                "acc-tm",
                "team_member_added",
                "user_action",
                None,
                0.8,
            )
            .map_err(|e| format!("{e}"))?;
            Ok(())
        })
        .expect("add team member");

        let member_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE account_id = 'acc-tm'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(member_count, 1, "Should have 1 team member");
        assert!(
            signal_count(&db, "acc-tm", "team_member_added") > 0,
            "Expected team_member_added signal"
        );
        let source: String = db
            .conn_ref()
            .query_row(
                "SELECT COALESCE(data_source, '') FROM account_stakeholders
                 WHERE account_id = 'acc-tm' AND person_id = 'p-tm'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "user", "Team member link should be user-owned");

        // Remove
        db.with_transaction(|tx| {
            tx.remove_account_team_member("acc-tm", "p-tm", "csm")
                .map_err(|e| e.to_string())?;
            crate::services::signals::emit_and_propagate(
                tx,
                &engine,
                "account",
                "acc-tm",
                "team_member_removed",
                "user_action",
                None,
                0.7,
            )
            .map_err(|e| format!("{e}"))?;
            Ok(())
        })
        .expect("remove team member");

        let member_count_after: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE account_id = 'acc-tm'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(member_count_after, 0, "Team member should be removed");
        assert!(
            signal_count(&db, "acc-tm", "team_member_removed") > 0,
            "Expected team_member_removed signal"
        );
    }

    /// Test account event recording at DB level with signal emission.
    #[test]
    fn test_record_account_event() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-ev", "Event Corp");
        db.upsert_account(&account).unwrap();

        db.with_transaction(|tx| {
            tx.record_account_event(
                "acc-ev",
                "renewal",
                "2026-06-15",
                Some(50000.0),
                Some("Renewed for 1 year"),
            )
            .map_err(|e| e.to_string())?;
            crate::services::signals::emit_and_propagate(
                tx,
                &engine,
                "account",
                "acc-ev",
                "account_event_recorded",
                "user_action",
                Some(r#"{"event_type":"renewal","event_date":"2026-06-15"}"#),
                0.8,
            )
            .map_err(|e| format!("{e}"))?;
            Ok(())
        })
        .expect("record event");

        let event_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_events WHERE account_id = 'acc-ev'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1, "Should have 1 account event");
        assert!(
            signal_count(&db, "acc-ev", "account_event_recorded") > 0,
            "Expected account_event_recorded signal"
        );
    }

    /// Test set_account_domains.
    #[test]
    fn test_set_account_domains() {
        let db = test_db();
        let account = make_account("acc-dom", "Domain Corp");
        db.upsert_account(&account).unwrap();

        let domains = vec!["example.com".to_string(), "example.org".to_string()];
        super::set_account_domains(&db, "acc-dom", &domains).expect("set_account_domains");

        let domain_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_domains WHERE account_id = 'acc-dom'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(domain_count, 2, "Should have 2 domains");
    }

    /// Test bulk account creation with duplicate handling.
    #[test]
    fn test_bulk_create_accounts() {
        let db = test_db();
        let tmp_dir = tempfile::tempdir().expect("tmp workspace");
        let workspace = tmp_dir.path();

        let names = vec![
            "Alpha Corp".to_string(),
            "Beta Inc".to_string(),
            "Alpha Corp".to_string(), // duplicate
        ];

        let created =
            super::bulk_create_accounts(&db, workspace, &names).expect("bulk_create_accounts");

        // First call: 2 unique accounts created, duplicate skipped
        assert_eq!(created.len(), 2, "Should create 2 unique accounts");

        let total: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 2, "DB should have 2 accounts");

        // Second call with same names: all skipped as duplicates
        let created_again = super::bulk_create_accounts(&db, workspace, &names)
            .expect("bulk_create_accounts second run");
        assert_eq!(created_again.len(), 0, "Duplicates should be skipped");
    }

    // =========================================================================
    // Lifecycle engine (v1.1.0)
    // =========================================================================

    #[test]
    fn test_infer_renewal_stage() {
        use chrono::{NaiveDate, Utc};

        let today = Utc::now().date_naive();
        let fmt = |d: NaiveDate| d.format("%Y-%m-%d").to_string();

        // >120 days → None (not yet renewing)
        let far_future = fmt(today + chrono::Duration::days(200));
        assert_eq!(super::infer_renewal_stage(Some(&far_future)), None);

        // 61-120 days → approaching
        let approaching = fmt(today + chrono::Duration::days(90));
        assert_eq!(
            super::infer_renewal_stage(Some(&approaching)),
            Some("approaching".to_string())
        );

        // 31-60 days → negotiating
        let negotiating = fmt(today + chrono::Duration::days(45));
        assert_eq!(
            super::infer_renewal_stage(Some(&negotiating)),
            Some("negotiating".to_string())
        );

        // 0-30 days → contract_sent
        let soon = fmt(today + chrono::Duration::days(15));
        assert_eq!(
            super::infer_renewal_stage(Some(&soon)),
            Some("contract_sent".to_string())
        );

        // Past → processed
        let past = fmt(today - chrono::Duration::days(5));
        assert_eq!(
            super::infer_renewal_stage(Some(&past)),
            Some("processed".to_string())
        );

        // None → None
        assert_eq!(super::infer_renewal_stage(None), None);
    }

    #[test]
    fn test_ensure_lifecycle_state_emits_renewal_stage_signal() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let today = chrono::Utc::now().date_naive();
        let contract_end = (today + chrono::Duration::days(90))
            .format("%Y-%m-%d")
            .to_string();

        let mut account = make_account("acc-renew", "Renewing Corp");
        account.lifecycle = Some("renewing".to_string());
        account.contract_end = Some(contract_end);
        db.upsert_account(&account).expect("upsert");

        super::ensure_account_lifecycle_state(&db, &engine, "acc-renew").expect("ensure_lifecycle");

        // Should have emitted renewal_stage_updated signal
        assert_eq!(signal_count(&db, "acc-renew", "renewal_stage_updated"), 1);

        // Stage should be set to "approaching" (61-120 days)
        let stage = db.get_account_renewal_stage("acc-renew").unwrap();
        assert_eq!(stage, Some("approaching".to_string()));
    }

    #[test]
    fn test_confirm_lifecycle_change_positive_weight() {
        let db = test_db();
        let engine = PropagationEngine::default();

        let mut account = make_account("acc-confirm", "Confirm Corp");
        account.lifecycle = Some("renewing".to_string());
        db.upsert_account(&account).expect("upsert");

        // Insert a pending lifecycle change
        db.conn_ref()
            .execute(
                "INSERT INTO lifecycle_changes (account_id, previous_lifecycle, new_lifecycle, source, confidence, evidence, user_response, created_at)
                 VALUES ('acc-confirm', 'renewing', 'active', 'email_signal', 0.85, 'Order form signed', 'pending', datetime('now'))",
                [],
            )
            .expect("insert lifecycle_changes");
        let change_id: i64 = db
            .conn_ref()
            .query_row(
                "SELECT id FROM lifecycle_changes WHERE account_id = 'acc-confirm'",
                [],
                |row| row.get(0),
            )
            .expect("get change_id");

        super::confirm_lifecycle_change(&db, &engine, change_id).expect("confirm");

        // Signal weight alpha should increase for email_signal source (positive feedback)
        let (alpha, beta): (f64, f64) = db
            .conn_ref()
            .query_row(
                "SELECT alpha, beta FROM signal_weights WHERE source = 'email_signal' AND signal_type = 'lifecycle_transition'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0.0, 0.0));
        assert!(
            alpha > 1.0,
            "Alpha should increase on confirm (was {})",
            alpha
        );
        assert!(
            (beta - 1.0).abs() < f64::EPSILON,
            "Beta should stay at base (was {})",
            beta
        );
    }

    #[test]
    fn test_correct_lifecycle_change_negative_weight() {
        let db = test_db();
        let engine = PropagationEngine::default();

        let mut account = make_account("acc-correct", "Correct Corp");
        account.lifecycle = Some("renewing".to_string());
        db.upsert_account(&account).expect("upsert");

        // Insert a pending lifecycle change
        db.conn_ref()
            .execute(
                "INSERT INTO lifecycle_changes (account_id, previous_lifecycle, new_lifecycle, source, confidence, evidence, user_response, created_at)
                 VALUES ('acc-correct', 'renewing', 'active', 'calendar_pattern', 0.75, 'No meetings 30d', 'pending', datetime('now'))",
                [],
            )
            .expect("insert lifecycle_changes");
        let change_id: i64 = db
            .conn_ref()
            .query_row(
                "SELECT id FROM lifecycle_changes WHERE account_id = 'acc-correct'",
                [],
                |row| row.get(0),
            )
            .expect("get change_id");

        super::correct_lifecycle_change(
            &db,
            &engine,
            change_id,
            "at_risk",
            None,
            Some("Actually at risk"),
        )
        .expect("correct");

        // Signal weight beta should increase for calendar_pattern source (negative feedback)
        let (alpha, beta): (f64, f64) = db
            .conn_ref()
            .query_row(
                "SELECT alpha, beta FROM signal_weights WHERE source = 'calendar_pattern' AND signal_type = 'lifecycle_transition'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0.0, 0.0));
        assert!(
            (alpha - 1.0).abs() < f64::EPSILON,
            "Alpha should stay at base (was {})",
            alpha
        );
        assert!(beta > 1.0, "Beta should increase on correct (was {})", beta);
    }

    /// DOS-229: `build_account_detail_result` — called on the writer
    /// connection after a mutation — must reflect the just-written state.
    /// Regression guard for the SQLite WAL reader-snapshot lag that caused
    /// save→refresh UI staleness on account detail pages.
    #[test]
    fn test_build_account_detail_reflects_writer_updates() {
        let db = test_db();
        let account = make_account("acc-dos229", "Post-Write Corp");
        db.upsert_account(&account).unwrap();

        // Baseline: no sentiment set.
        let before = super::build_account_detail_result(&db, "acc-dos229")
            .expect("baseline build");
        assert!(before.user_health_sentiment.is_none());
        assert!(before.sentiment_set_at.is_none());

        // Simulate the writer-side mutation that `set_user_health_sentiment`
        // performs (field update only — signal emission needs AppState).
        db.update_account_field("acc-dos229", "user_health_sentiment", "at_risk")
            .expect("write sentiment");
        let ts = chrono::Utc::now().to_rfc3339();
        db.update_account_field("acc-dos229", "sentiment_set_at", &ts)
            .expect("write sentiment_set_at");

        // Re-assembling the detail on the SAME connection must reflect the
        // write — this is the invariant DOS-229 depends on.
        let after = super::build_account_detail_result(&db, "acc-dos229")
            .expect("post-write build");
        assert_eq!(after.user_health_sentiment.as_deref(), Some("at_risk"));
        assert_eq!(after.sentiment_set_at.as_deref(), Some(ts.as_str()));
    }
}
