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
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: Some(tracker_path),
        parent_id: Some(parent.id.clone()),
        account_type: parent.account_type.clone(),
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

fn normalized_lifecycle(value: &str) -> String {
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

fn account_field_conflict_feedback_key(field: &str, signal_id: &str) -> String {
    format!("account_field_conflict:{field}:{signal_id}")
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
            let feedback_key = account_field_conflict_feedback_key(field, &signal.id);
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
                    let feedback_key = account_field_conflict_feedback_key(
                        "arr",
                        "intelligence-contract-context-arr",
                    );
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
                    let feedback_key = account_field_conflict_feedback_key(
                        "contract_end",
                        "intelligence-contract-context-renewal-date",
                    );
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
                    let feedback_key = account_field_conflict_feedback_key(
                        "lifecycle",
                        "intelligence-org-health-lifecycle",
                    );
                    if !dismissed_conflicts.contains(&feedback_key) {
                        conflicts.entry("lifecycle".to_string()).or_insert_with(|| {
                            crate::types::AccountFieldConflictSuggestion {
                                field: "lifecycle".to_string(),
                                source: if org_health.source.is_empty() {
                                    "glean_crm".to_string()
                                } else {
                                    org_health.source.clone()
                                },
                                suggested_value: normalized_lifecycle(stage),
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
                    let feedback_key =
                        account_field_conflict_feedback_key("nps", "intelligence-nps");
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
                log::info!(
                    "Lifecycle transition for {} skipped milestone auto-complete at confidence {:.2}",
                    account_id,
                    transition.confidence
                );
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
    state: &AppState,
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

    if let Some(signal_id) = signal_id {
        let feedback_id = uuid::Uuid::new_v4().to_string();
        let feedback_key = account_field_conflict_feedback_key(field, signal_id);
        let context = serde_json::json!({
            "source": source,
            "signal_id": signal_id,
            "suggested_value": suggested_value,
        })
        .to_string();
        db.insert_intelligence_feedback(
            &feedback_id,
            account_id,
            "account",
            &feedback_key,
            "positive",
            None,
            Some(&context),
        )?;

        let accepted_signal_id =
            format!("account-field-conflict-accepted-{}", uuid::Uuid::new_v4());
        let _ = crate::signals::bus::supersede_signal(db, signal_id, &accepted_signal_id);
    }

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
    let feedback_key = account_field_conflict_feedback_key(field, signal_id);
    let context = serde_json::json!({
        "source": source,
        "signal_id": signal_id,
        "suggested_value": suggested_value,
    })
    .to_string();
    db.insert_intelligence_feedback(
        &feedback_id,
        account_id,
        "account",
        &feedback_key,
        "negative",
        None,
        Some(&context),
    )?;
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
/// Loads account from DB, reads dashboard.json + intelligence.json,
/// fetches actions, meetings, people, team, signals, captures, and email signals.
pub async fn get_account_detail(
    account_id: &str,
    state: &AppState,
) -> Result<AccountDetailResult, String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?.clone();
    let engine = std::sync::Arc::clone(&state.signals.engine);

    let lifecycle_account_id = account_id.to_string();
    let _ = state
        .db_write(move |db| ensure_account_lifecycle_state(db, &engine, &lifecycle_account_id))
        .await;

    let account_id = account_id.to_string();
    state
        .db_read(move |db| {
            let account = db
                .get_account(&account_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Account not found: {}", account_id))?;

            // Read narrative fields from dashboard.json + intelligence.json if they exist
            let (overview, programs, notes, intelligence) = if let Some(ref config) = config {
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
                // Read intelligence from DB (I513), fall back to legacy migration
                let intel = db
                    .get_entity_intelligence(&account_id)
                    .ok()
                    .flatten()
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
            let recent_email_signals = db
                .list_recent_email_signals_for_entity(&account_id, 12)
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

            Ok(AccountDetailResult {
                id: account.id,
                name: account.name,
                lifecycle: account.lifecycle,
                arr: account.arr,
                health: account.health,
                nps: account.nps,
                renewal_date: account.contract_end,
                renewal_stage,
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
                account_type: account.account_type.clone(),
                archived: account.archived,
                objectives,
                lifecycle_changes,
                products,
                field_provenance,
                field_conflicts,
                intelligence,
            })
        })
        .await
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

    existing.notes = if notes.is_empty() {
        None
    } else {
        Some(notes.to_string())
    };

    let _ = crate::accounts::write_account_json(workspace, &account, Some(&existing), db);
    let _ = crate::accounts::write_account_markdown(workspace, &account, Some(&existing), db);

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

    let programs: Vec<crate::accounts::StrategicProgram> =
        serde_json::from_str(programs_json).map_err(|e| format!("Invalid programs JSON: {}", e))?;

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
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: Some(tracker_path),
        parent_id: parent_id.map(|s| s.to_string()),
        account_type,
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
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some(format!("Accounts/{}", name)),
            parent_id: None,
            account_type: crate::db::AccountType::Customer,
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
        .map_err(|_| "Lock poisoned")?
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
                        arr: None,
                        health: Some("green".to_string()),
                        contract_start: None,
                        contract_end: None,
                        nps: None,
                        tracker_path: Some(format!("Internal/{}", company_name_clone)),
                        parent_id: None,
                        account_type: crate::db::AccountType::Internal,
                        updated_at: now,
                        archived: false,
                        keywords: None,
                        keywords_extracted_at: None,
                        metadata: None,
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
        .map_err(|_| "Lock poisoned")?
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
}
