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
use crate::services::context::ServiceContext;
use crate::signals::propagation::PropagationEngine;
use crate::state::AppState;

pub fn set_account_domains(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    domains: &[String],
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.set_account_domains(account_id, domains)
        .map_err(|e| e.to_string())
}

/// Create a child account under a parent with collision handling.
///
/// Checks for duplicate names, generates unique IDs, creates DB record,
/// copies parent domains, and optionally writes workspace files.
pub fn create_child_account_record(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace: Option<&Path>,
    parent: &crate::db::DbAccount,
    name: &str,
    description: Option<&str>,
    owner_person_id: Option<&str>,
) -> Result<crate::db::DbAccount, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
    let now = ctx.clock.now().to_rfc3339();

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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    auto_completed: &crate::db::success_plans::AutoCompletedMilestones,
    source: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    for milestone in &auto_completed.milestones {
        crate::services::signals::emit_and_propagate(
            ctx,
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
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    transition: &LifecycleTransitionCandidate,
) -> Result<Option<i64>, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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

        let _ = crate::services::intelligence::recompute_entity_health(
            ctx,
            tx,
            account_id,
            "account",
        );
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
            ctx,
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
                    ctx,
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
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
        let _ = apply_lifecycle_transition(ctx, db, engine, account_id, &candidate)?;
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
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
            .map(|date| (date - ctx.clock.now().date_naive()).num_days() <= 150)
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

        ensure_account_lifecycle_state(ctx, db, engine, &account.id)?;
        refreshed += 1;
    }
    Ok(refreshed)
}

pub fn confirm_lifecycle_change(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    change_id: i64,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let change = db
        .get_lifecycle_change(change_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Lifecycle change not found: {change_id}"))?;
    db.set_lifecycle_change_response(change_id, "confirmed", None)
        .map_err(|e| e.to_string())?;
    let _ = db.upsert_signal_weight(&change.source, "account", "lifecycle_transition", 1.0, 0.0);
    crate::services::signals::emit_and_propagate(
            ctx,
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

// DOS-209: ServiceContext+ adds 1 arg; refactor to request struct deferred to W3.
#[allow(clippy::too_many_arguments)]
pub fn correct_account_product(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    product_id: i64,
    name: &str,
    status: Option<&str>,
    source_to_penalize: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.update_account_product(product_id, name, status, None, "user_correction", 1.0)
        .map_err(|e| e.to_string())?;
    let _ = db.upsert_signal_weight(source_to_penalize, "account", "product_adoption", 0.0, 1.0);
    crate::services::signals::emit_and_propagate(
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    change_id: i64,
    corrected_lifecycle: &str,
    corrected_stage: Option<&str>,
    notes: Option<&str>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
    apply_lifecycle_transition(ctx, db, engine, &change.account_id, &transition)?;
    db.set_lifecycle_change_response(change_id, "corrected", notes)
        .map_err(|e| e.to_string())?;
    let _ = db.upsert_signal_weight(&change.source, "account", "lifecycle_transition", 0.0, 1.0);
    Ok(())
}

// DOS-209: ServiceContext+ adds 1 arg; refactor to request struct deferred to W3.
#[allow(clippy::too_many_arguments)]
pub fn accept_account_field_conflict(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    field: &str,
    suggested_value: &str,
    source: &str,
    signal_id: Option<&str>,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let next_value = if field == "lifecycle" {
        normalized_lifecycle(suggested_value)
    } else {
        suggested_value.to_string()
    };

    // `update_account_field_inner` runs its own internal DB writes plus
    // post-commit side effects (emit_propagate_and_evaluate, self-healing
    // feedback, health debounce, workspace file regen). We do NOT pull it
    // inside the transaction below — it manages its own atomicity.
    update_account_field_inner(ctx, db, state, account_id, field, &next_value)?;

    // DOS-309: pull the conflict-resolution writes into one transaction so
    // a failure on any single write rolls back the whole conflict-resolution
    // sequence. Side-effect emission (emit_propagate_and_evaluate, which
    // enqueues cross-entity intel work via engine.propagate) runs AFTER
    // the transaction commits — a downstream propagation failure cannot
    // roll back the user's accept intent.
    let accepted_signal_id_holder: Option<String> = signal_id
        .map(|_| format!("account-field-conflict-accepted-{}", uuid::Uuid::new_v4()));

    db.with_transaction(|tx| -> Result<(), String> {
        if matches!(field, "arr" | "lifecycle" | "contract_end" | "nps") {
            tx.set_account_field_provenance(account_id, field, source, None)
                .map_err(|e| format!("set_account_field_provenance: {e}"))?;
        }

        if let (Some(sig_id), Some(accepted_id)) = (signal_id, accepted_signal_id_holder.as_deref()) {
            crate::signals::bus::supersede_signal(tx, sig_id, accepted_id)
                .map_err(|e| format!("supersede_signal: {e}"))?;
        }

        tx.record_feedback_event(&crate::db::feedback::FeedbackEventInput {
            entity_id: account_id,
            entity_type: "account",
            field_key: field,
            item_key: signal_id,
            feedback_type: "accept",
            source_system: Some(source),
            source_kind: Some("field_conflict"),
            previous_value: None,
            corrected_value: Some(suggested_value),
            reason: None,
        })
        .map_err(|e| format!("record_feedback_event: {e}"))?;

        tx.upsert_signal_weight(
            source,
            "account",
            &account_field_signal_category(field),
            1.0,
            0.0,
        )
        .map_err(|e| format!("upsert_signal_weight: {e}"))?;
        Ok(())
    })?;

    // Post-commit side effects. emit_propagate_and_evaluate calls
    // engine.propagate which can enqueue cross-entity intel work via
    // state.intel_queue. Running it after commit means a downstream
    // failure cannot roll back the conflict resolution. DB is canonical;
    // emission failure logs.
    let payload = serde_json::json!({
        "field": field,
        "source": source,
        "signal_id": signal_id,
        "suggested_value": suggested_value,
    })
    .to_string();
    if let Err(e) = crate::services::signals::emit_propagate_and_evaluate(
            ctx,
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_conflict_accepted",
        "user_feedback",
        Some(&payload),
        0.95,
        &state.intel_queue,
    ) {
        log::warn!(
            "post-commit signal emission failed; \
             repair_target=signals_engine \
             account={account_id} field={field}: {e}"
        );
    }

    // DOS-229 Wave 0e Fix 5: return post-write detail so the frontend can
    // setDetail(result) without a second fetch.
    build_account_detail_result(db, account_id)
}

// DOS-209: ServiceContext+ adds 1 arg; refactor to request struct deferred to W3.
#[allow(clippy::too_many_arguments)]
pub fn dismiss_account_field_conflict(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    field: &str,
    signal_id: &str,
    source: &str,
    suggested_value: Option<&str>,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // DOS-309: pull conflict-resolution writes into one transaction; emit
    // signal-bus side effects after commit. See accept_account_field_conflict
    // for the architectural rationale.
    let dismissed_signal_id = format!("account-field-conflict-dismissed-{}", uuid::Uuid::new_v4());

    db.with_transaction(|tx| -> Result<(), String> {
        tx.record_feedback_event(&crate::db::feedback::FeedbackEventInput {
            entity_id: account_id,
            entity_type: "account",
            field_key: field,
            item_key: Some(signal_id),
            feedback_type: "reject",
            source_system: Some(source),
            source_kind: Some("field_conflict"),
            previous_value: None,
            corrected_value: suggested_value,
            reason: None,
        })
        .map_err(|e| format!("record_feedback_event: {e}"))?;

        let signal_id_str = signal_id;
        tx.create_suppression_tombstone(
            account_id,
            field,
            Some(signal_id_str),
            crate::intelligence::canonicalization::maybe_item_hash_for_field(
                field,
                Some(signal_id_str),
            )
            .as_deref(),
            Some(source),
            None,
        )
        .map_err(|e| format!("create_suppression_tombstone: {e}"))?;

        // DOS-7 D4-1a: shadow-write tombstone claim. Subject is the account;
        // field carries the field key; item_text is the signal_id (opaque
        // structural identifier — kept consistent with backfill m1 metadata).
        let observed_at = ctx.clock.now().to_rfc3339();
        crate::services::claims::shadow_write_tombstone_claim(
            tx,
            crate::services::claims::ShadowTombstoneClaim {
                subject_kind: "Account",
                subject_id: account_id,
                claim_type: "account_field_correction",
                field_path: Some(field),
                text: signal_id_str,
                actor: "user",
                source_scope: Some(source),
                observed_at: &observed_at,
            },
        );

        tx.upsert_signal_weight(
            source,
            "account",
            &account_field_signal_category(field),
            0.0,
            1.0,
        )
        .map_err(|e| format!("upsert_signal_weight: {e}"))?;

        crate::signals::bus::supersede_signal(tx, signal_id, &dismissed_signal_id)
            .map_err(|e| format!("supersede_signal: {e}"))?;
        Ok(())
    })?;

    // Post-commit side effects. Emission failure logs; DB is canonical.
    let payload = serde_json::json!({
        "field": field,
        "source": source,
        "signal_id": signal_id,
        "suggested_value": suggested_value,
    })
    .to_string();
    if let Err(e) = crate::services::signals::emit_propagate_and_evaluate(
            ctx,
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_conflict_dismissed",
        "user_feedback",
        Some(&payload),
        0.95,
        &state.intel_queue,
    ) {
        log::warn!(
            "post-commit signal emission failed; \
             repair_target=signals_engine \
             account={account_id} field={field}: {e}"
        );
    }

    build_account_detail_result(db, account_id)
}

/// Get full detail for an account by ID.
///
/// I644: All data from DB — no filesystem reads on the detail page path.
/// Fetches actions, meetings, people, team, signals, captures, and email signals.
pub async fn get_account_detail(
    ctx: &ServiceContext<'_>,
    account_id: &str,
    state: &std::sync::Arc<AppState>,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let _config = state.config.read().clone();
    let engine = std::sync::Arc::clone(&state.signals.engine);
    let state_for_ctx = state.clone();

    let lifecycle_account_id = account_id.to_string();
    let _ = state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            ensure_account_lifecycle_state(&ctx, db, &engine, &lifecycle_account_id)
        })
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    field: &str,
    value: &str,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    update_account_field_inner(ctx, db, state, account_id, field, value)?;
    // DOS-229 generalization (Wave 0e Fix 5): return the fresh detail
    // assembled on the SAME writer connection so the frontend hook can
    // setDetail(result) without a follow-up silentRefresh (which hits a
    // different pool reader whose WAL snapshot may lag).
    build_account_detail_result(db, account_id)
}

fn update_account_field_inner(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    field: &str,
    value: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
            ctx,
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
    //
    // DOS-228 Wave 0f: persist the durable `health_recompute_pending` marker
    // SYNCHRONOUSLY on the same writer connection that just committed the
    // field edit, BEFORE scheduling the in-memory debounce. The debouncer
    // only owns timing/coalescing — it must never own marker durability, or
    // a crash during the 2s sleep window silently loses the committed edit's
    // health recompute. Marker-write failure is propagated as Err so the
    // mutation fails loudly rather than swallowing durability loss.
    if is_health_relevant_field(field) {
        db.mark_health_recompute_pending(account_id)
            .map_err(|e| format!("failed to persist health_recompute_pending marker: {e}"))?;
        crate::services::health_debouncer::schedule_recompute(ctx, state, account_id);
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

/// DOS-231 Codex fix: persist a single gap-row field on `account_technical_footprint`.
///
/// Intelligence Loop 5Q:
///   1. Emits a `field_updated` signal with source `user_edit` and
///      confidence 0.9 — same propagation path as `update_account_field`.
///   2. Technical-footprint fields already feed the health scoring behavioral
///      layer (see `intelligence::health_scoring`), so a user edit
///      materially improves the dimension's signal quality.
///   3. Is included in `build_intelligence_context` via the existing
///      `DbAccountTechnicalFootprint` block (read by prompts.rs / meeting_context.rs).
///   4. No new briefing callout type required — the existing
///      `field_updated` signal covers it.
///   5. User corrections are already recorded against the "glean" enrichment
///      source when applicable via the self-healing feedback system; here we
///      stamp `source = 'user_edit'` on the row.
pub fn update_technical_footprint_field(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    field: &str,
    value: &str,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.update_technical_footprint_field(account_id, field, value)
        .map_err(|e| e.to_string())?;

    // Emit field-update signal + self-healing evaluation so the rest of
    // the Intelligence Loop picks up the user correction.
    crate::services::signals::emit_propagate_and_evaluate(
            ctx,
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_updated",
        "user_edit",
        Some(&format!(
            "{{\"table\":\"account_technical_footprint\",\"field\":\"{}\",\"value\":\"{}\"}}",
            field,
            value.replace('"', "\\\"")
        )),
        0.9,
        &state.intel_queue,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    // Return the updated detail on the SAME writer connection so the
    // frontend sees the persisted value immediately (DOS-229 pattern).
    build_account_detail_result(db, account_id)
}

/// DOS-110 / DOS-27: Set the user's manual health sentiment on an account.
/// Writes the current sentiment + timestamp, appends a journal entry (value +
/// optional note + computed band snapshot), emits a `field_updated` signal,
/// and — on transition into at_risk/critical — enqueues a background risk
/// briefing generation.
pub fn set_user_health_sentiment(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    sentiment: &str,
    note: Option<&str>,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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

    let now = ctx.clock.now().to_rfc3339();
    db.update_account_field(account_id, "user_health_sentiment", sentiment)
        .map_err(|e| e.to_string())?;
    db.update_account_field(account_id, "sentiment_set_at", &now)
        .map_err(|e| e.to_string())?;

    // Snapshot computed health at set-time for divergence analysis.
    let (computed_band, computed_score) = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .map(|acct| {
            let preset_guard = state.active_preset.read();
            let health = crate::intelligence::health_scoring::compute_account_health_with_preset(
                db,
                &acct,
                None,
                preset_guard.as_ref(),
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
            ctx,
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

    // DOS-228 Wave 0e Fix 1: persist the `enqueued` row on THIS writer
    // connection BEFORE we build the result. Previously the row was written
    // by a spawned `db_write` task, which cannot execute while the current
    // writer closure is still running — so the first AccountDetailResult
    // the user saw had a missing/stale `risk_briefing_job`. Enqueue on the
    // same connection, then kick off the async lifecycle runner with the
    // attempt_id we just stamped.
    if transitioning_into_risk {
        let attempt_id = uuid::Uuid::new_v4().to_string();
        db.upsert_risk_briefing_job_enqueued(account_id, &attempt_id)
            .map_err(|e| format!("persist risk briefing enqueue: {e}"))?;
        spawn_risk_briefing_lifecycle(ctx, state, account_id.to_string(), attempt_id);
    }

    // DOS-229: Read back the updated detail on the SAME writer connection so
    // the frontend sees post-write state immediately. A follow-up `db_read`
    // hits a different pool connection whose WAL snapshot can lag.
    build_account_detail_result(db, account_id)
}

/// DOS-269: "Add more detail" — update the note on the newest sentiment
/// history row whose sentiment matches the account's current value. No new
/// history entry is appended. Falls back to `insert_sentiment_journal_entry`
/// when there is no matching row yet (first-ever note for this value).
/// Emits the same `field_updated` signal as `set_user_health_sentiment` so
/// the Intelligence Loop sees that the user touched this surface.
pub fn update_latest_sentiment_note(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    note: Option<&str>,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let current_sentiment: Option<String> = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .and_then(|a| a.user_health_sentiment);
    let sentiment = current_sentiment.ok_or_else(|| {
        "Cannot update sentiment note before a sentiment value is set".to_string()
    })?;

    let updated = db
        .update_latest_sentiment_note(account_id, note)
        .map_err(|e| e.to_string())?;

    if !updated {
        // No prior history row for this sentiment — insert a fresh one so
        // the note has somewhere to live. Computed-band snapshot mirrors
        // `set_user_health_sentiment` for divergence parity.
        let (computed_band, computed_score) = db
            .get_account(account_id)
            .map_err(|e| e.to_string())?
            .map(|acct| {
                let preset_guard = state.active_preset.read();
                let health = crate::intelligence::health_scoring::compute_account_health_with_preset(
                    db,
                    &acct,
                    None,
                    preset_guard.as_ref(),
                );
                (Some(health.band), Some(health.score))
            })
            .unwrap_or((None, None));
        db.insert_sentiment_journal_entry(
            account_id,
            &sentiment,
            note,
            computed_band.as_deref(),
            computed_score,
        )
        .map_err(|e| e.to_string())?;
    }

    // Emit annotation-level signal (user augmented their journal entry).
    crate::services::signals::emit_propagate_and_evaluate(
            ctx,
        db,
        &state.signals.engine,
        "account",
        account_id,
        "field_updated",
        "user_edit",
        Some(&format!(
            "{{\"field\":\"sentiment_note\",\"value\":\"{}\"}}",
            sentiment
        )),
        0.8,
        &state.intel_queue,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;

    build_account_detail_result(db, account_id)
}

/// DOS-269: Triage snooze / resolve persistence. Serializable row for the
/// `list_triage_snoozes` command.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageSnoozeRow {
    pub triage_key: String,
    pub snoozed_until: Option<String>,
    pub resolved_at: Option<String>,
}

/// DOS-7 L2 cycle-1 fix #5: map a lowercase entity_type column value
/// (e.g. "account", "person") to the PascalCase `subject_kind` field
/// the claims substrate uses (e.g. "Account", "Person"). Unknown
/// values are passed through unchanged.
fn capitalize_entity_kind(kind: &str) -> String {
    let mut chars = kind.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// DOS-269: Snooze a triage card for N days. `days` must be positive; the
/// frontend default is 14.
pub fn snooze_triage_item(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    triage_key: &str,
    days: i64,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let days = days.max(1);
    let until = ctx.clock.now() + chrono::Duration::days(days);
    db.snooze_triage_item(entity_type, entity_id, triage_key, &until.to_rfc3339())
        .map_err(|e| e.to_string())?;

    // DOS-7 L2 cycle-1 fix #5: shadow-write the snooze as an
    // intelligence_claims tombstone (claim_type='triage_snooze',
    // retraction_reason='system_snooze' via shadow_write helper's
    // default user_removal mapping) so PRE-GATE shadows the snoozed
    // card across enrichment passes.
    let now = ctx.clock.now().to_rfc3339();
    let kind = capitalize_entity_kind(entity_type);
    crate::services::claims::shadow_write_tombstone_claim(
        db,
        crate::services::claims::ShadowTombstoneClaim {
            subject_kind: &kind,
            subject_id: entity_id,
            claim_type: "triage_snooze",
            field_path: Some(entity_type),
            text: triage_key,
            actor: "user",
            source_scope: None,
            observed_at: &now,
        },
    );
    Ok(())
}

/// DOS-269: Mark a triage card resolved. Permanent for that card id.
/// Emits a low-weight field_updated signal so the Intelligence Loop
/// records that the user acted on this card (parity with DOS-41 confirm).
pub fn resolve_triage_item(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    entity_type: &str,
    entity_id: &str,
    triage_key: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.resolve_triage_item(entity_type, entity_id, triage_key)
        .map_err(|e| e.to_string())?;

    // DOS-7 L2 cycle-1 fix #5: shadow-write the resolve as an
    // intelligence_claims tombstone so PRE-GATE shadows the resolved
    // card across subsequent enrichment passes.
    let now = ctx.clock.now().to_rfc3339();
    let kind = capitalize_entity_kind(entity_type);
    crate::services::claims::shadow_write_tombstone_claim(
        db,
        crate::services::claims::ShadowTombstoneClaim {
            subject_kind: &kind,
            subject_id: entity_id,
            claim_type: "triage_snooze",
            field_path: Some(entity_type),
            text: triage_key,
            actor: "user",
            source_scope: None,
            observed_at: &now,
        },
    );

    // Best-effort signal emit — triage resolution is user-intent evidence
    // the card was accurate + actioned. Failure should not rollback.
    let _ = crate::services::signals::emit_propagate_and_evaluate(
            ctx,
        db,
        &state.signals.engine,
        entity_type,
        entity_id,
        "field_updated",
        "user_edit",
        Some(&format!(
            "{{\"field\":\"triage_resolved\",\"triage_key\":\"{}\"}}",
            triage_key
        )),
        0.8,
        &state.intel_queue,
    );
    Ok(())
}

/// DOS-269: Return all snooze/resolve rows for an entity. Rendering-time
/// filter decides whether a snooze is still active.
pub fn list_triage_snoozes(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<TriageSnoozeRow>, String> {
    let rows = db
        .list_triage_snoozes(entity_type, entity_id)
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(triage_key, snoozed_until, resolved_at)| TriageSnoozeRow {
            triage_key,
            snoozed_until,
            resolved_at,
        })
        .collect())
}

/// DOS-228 Wave 0e Fix 2: Spawn the SINGLE ordered lifecycle task for a
/// risk briefing attempt.
///
/// The caller is responsible for having written the `enqueued` row with
/// `attempt_id` on the originating writer connection; this task only runs
/// the enqueued → running → complete|failed transitions, each gated by a
/// compare-and-set against `attempt_id`. If a superseding retry overwrites
/// the row's attempt_id between two of our transitions, our CAS fails and
/// we bail out cleanly — preventing the "two racing retries corrupt each
/// other" last-write-wins bug that the prior two-spawn design had.
fn spawn_risk_briefing_lifecycle(
    ctx: &ServiceContext<'_>,
    state: &std::sync::Arc<AppState>,
    account_id: String,
    attempt_id: String,
) {
    if let Err(e) = ctx.check_mutation_allowed() {
        log::warn!("risk briefing lifecycle spawn blocked by execution mode: {e}");
        return;
    }
    let state_clone = state.clone();
    let handle = state.app_handle();

    tauri::async_runtime::spawn(async move {
        log::info!(
            "DOS-27: risk briefing lifecycle {} for account {}",
            attempt_id,
            account_id
        );

        // Transition enqueued → running (CAS).
        let running_id = account_id.clone();
        let running_attempt = attempt_id.clone();
        let running_state = state_clone.clone();
        let owns_attempt = running_state
            .db_write(move |db| {
                db.mark_risk_briefing_job_running(&running_id, &running_attempt)
                    .map_err(|e| e.to_string())
            })
            .await
            .unwrap_or(false);

        if !owns_attempt {
            log::info!(
                "DOS-228: risk briefing attempt {} for {} superseded before running; exiting",
                attempt_id,
                account_id
            );
            return;
        }

        let outcome = crate::services::intelligence::generate_risk_briefing(
            &state_clone,
            &account_id,
            handle,
        )
        .await;

        // Terminal transition (CAS). A superseding retry that landed while
        // generation was running will own the next attempt; our update
        // affects zero rows and we log+exit without clobbering.
        let terminal_id = account_id.clone();
        let terminal_attempt = attempt_id.clone();
        let terminal_state = state_clone.clone();
        let terminal_outcome = outcome.as_ref().map(|_| ()).map_err(|e| e.clone());
        let still_current = terminal_state
            .db_write(move |db| match &terminal_outcome {
                Ok(()) => db
                    .mark_risk_briefing_job_complete(&terminal_id, &terminal_attempt)
                    .map_err(|e| e.to_string()),
                Err(msg) => db
                    .mark_risk_briefing_job_failed(&terminal_id, &terminal_attempt, msg)
                    .map_err(|e| e.to_string()),
            })
            .await
            .unwrap_or(false);

        if !still_current {
            log::info!(
                "DOS-228: risk briefing attempt {} for {} superseded before terminal write; outcome discarded",
                attempt_id,
                account_id
            );
            return;
        }

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

/// DOS-228 Wave 0e Fix 2: Re-enqueue a risk briefing generation.
///
/// Behavior is now guarded:
/// 1. The account must exist (rejects dangling IDs that would otherwise
///    spawn a useless lifecycle task).
/// 2. If the current job is `running`, the retry is COALESCED into the
///    existing attempt — we return Ok without spawning a duplicate runner.
///    This closes the "tap retry twice fast → two runners corrupt each
///    other" hole.
/// 3. Otherwise (failed / complete / no prior row / enqueued-but-not-yet-
///    started) we stamp a fresh attempt_id and spawn a new lifecycle. The
///    CAS machinery ensures any stale runner from a prior attempt can't
///    clobber the new one's state.
pub async fn retry_risk_briefing(
    ctx: &ServiceContext<'_>,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let id_for_read = account_id.to_string();
    let (exists, current_status) = state
        .db_read(move |db| {
            let exists = db
                .get_account(&id_for_read)
                .map_err(|e| e.to_string())?
                .is_some();
            let status = db
                .get_risk_briefing_job_status(&id_for_read)
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((exists, status))
        })
        .await?;

    if !exists {
        return Err(format!("Account not found: {}", account_id));
    }

    // Coalesce: don't start a second runner while one is actively generating.
    if current_status.as_deref() == Some("running") {
        log::info!(
            "DOS-228: retry_risk_briefing for {} coalesced into running job",
            account_id
        );
        return Ok(());
    }

    let attempt_id = uuid::Uuid::new_v4().to_string();
    let enqueue_id = account_id.to_string();
    let enqueue_attempt = attempt_id.clone();
    state
        .db_write(move |db| {
            db.upsert_risk_briefing_job_enqueued(&enqueue_id, &enqueue_attempt)
                .map_err(|e| e.to_string())
        })
        .await?;

    spawn_risk_briefing_lifecycle(ctx, state, account_id.to_string(), attempt_id);
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    notes: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.update_account_field(account_id, "notes", notes)
        .map_err(|e| e.to_string())?;

    let account = db
        .get_account(account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let config = state.config.read();
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    crate::accounts::write_account_json(workspace, &account, None, db)
        .map_err(|e| format!("failed to write account dashboard.json: {e}"))?;
    crate::accounts::write_account_markdown(workspace, &account, None, db)
        .map_err(|e| format!("failed to write account dashboard.md: {e}"))?;

    // Emit field update signal (I377)
    crate::services::signals::emit_and_propagate(
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    programs_json: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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

    crate::accounts::write_account_json(workspace, &account, None, db)
        .map_err(|e| format!("failed to write account dashboard.json: {e}"))?;
    crate::accounts::write_account_markdown(workspace, &account, None, db)
        .map_err(|e| format!("failed to write account dashboard.md: {e}"))?;

    // Emit field update signal (I377)
    crate::services::signals::emit_and_propagate(
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    name: &str,
    parent_id: Option<&str>,
    explicit_type: Option<crate::db::AccountType>,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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

    let now = ctx.clock.now().to_rfc3339();

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
///
/// DOS-286: when archiving, drops any pending intel queue entries for this
/// account and its cascaded children so already-queued enrichments don't run
/// against a now-archived target.
pub fn archive_account(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &crate::state::AppState,
    id: &str,
    archived: bool,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let signal_type = if archived {
        "entity_archived"
    } else {
        "entity_unarchived"
    };

    // DOS-286: collect child IDs before archiving so we can drop their queued
    // enrichments too (cascade archives children in the same transaction).
    let child_ids: Vec<String> = if archived {
        db.conn_ref()
            .prepare("SELECT id FROM accounts WHERE parent_id = ?1")
            .ok()
            .and_then(|mut stmt| {
                stmt.query_map(rusqlite::params![id], |row| row.get::<_, String>(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    db.with_transaction(|tx| {
        tx.archive_account(id, archived)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
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
    })?;

    // DOS-286: drop any in-flight enrichments for the archived account and its children.
    if archived {
        state.intel_queue.remove_by_entity_id(id);
        for child_id in &child_ids {
            state.intel_queue.remove_by_entity_id(child_id);
        }
    }

    Ok(())
}

/// Merge source account into target account with signal emission.
pub fn merge_accounts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &crate::state::AppState,
    from_id: &str,
    into_id: &str,
) -> Result<crate::db::MergeResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        let result = tx
            .merge_accounts(from_id, into_id)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    restore_children: bool,
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.restore_account(account_id, restore_children)
        .map_err(|e| e.to_string())
}

/// Add a person-role pair to an account team with signal emission.
pub fn add_account_team_member(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let role = role.trim().to_lowercase();
    if role.is_empty() {
        return Err("Role is required".to_string());
    }
    db.with_transaction(|tx| {
        tx.add_account_team_member(account_id, person_id, &role)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    person_id: &str,
    new_role: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let observed_at = ctx.clock.now().to_rfc3339();
    db.with_transaction(|tx| {
        // DOS-7 L2 cycle-2 fix #4: capture the user-owned roles being
        // dismissed BEFORE the swap. The original D4-1b shadow write
        // tombstoned `new_role` — the role the user just pinned —
        // instead of the roles being retired. That's exactly inverted:
        // the AI re-surfacing should be blocked for the dismissed
        // roles, not for the freshly-selected one.
        let normalized_new_role = new_role.trim().to_lowercase();
        let dismissed_roles: Vec<String> = {
            let mut stmt = tx
                .conn_ref()
                .prepare(
                    "SELECT role FROM account_stakeholder_roles \
                     WHERE account_id = ?1 AND person_id = ?2 \
                       AND data_source = 'user' \
                       AND dismissed_at IS NULL",
                )
                .map_err(|e| format!("read active user roles: {e}"))?;
            let rows = stmt
                .query_map(rusqlite::params![account_id, person_id], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| format!("query active user roles: {e}"))?;
            rows.filter_map(|r| r.ok())
                // Don't tombstone the role the user is re-pinning;
                // the swap reactivates it via ON CONFLICT.
                .filter(|r| r != &normalized_new_role)
                .collect()
        };

        tx.set_team_member_role(account_id, person_id, new_role)
            .map_err(|e| e.to_string())?;

        // DOS-7 L2 cycle-2 fix #4: shadow-write tombstones for each
        // role that was actually retired. PRE-GATE then blocks the AI
        // from re-surfacing those specific roles on the next pass.
        for old_role in &dismissed_roles {
            let _ = crate::services::claims::shadow_write_tombstone_claim(
                tx,
                crate::services::claims::ShadowTombstoneClaim {
                    subject_kind: "Person",
                    subject_id: person_id,
                    claim_type: "stakeholder_role",
                    field_path: None,
                    text: old_role,
                    actor: "user",
                    source_scope: Some("team_member_role_change"),
                    observed_at: &observed_at,
                },
            );
        }

        crate::services::signals::emit_and_propagate(
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        tx.remove_account_team_member(account_id, person_id, role)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
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
// DOS-209: ServiceContext+ adds 1 arg; refactor to request struct deferred to W3.
#[allow(clippy::too_many_arguments)]
pub fn record_account_event(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &crate::state::AppState,
    account_id: &str,
    event_type: &str,
    event_date: &str,
    arr_impact: Option<f64>,
    notes: Option<&str>,
) -> Result<i64, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        let event_id = tx
            .record_account_event(account_id, event_type, event_date, arr_impact, notes)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
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
            ctx,
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
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace: &Path,
    names: &[String],
) -> Result<Vec<String>, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let mut created_ids = Vec::with_capacity(names.len());

    for raw_name in names {
        let name = crate::util::validate_entity_name(raw_name)?;
        let id = crate::util::slugify(name);

        // Skip duplicates
        if let Ok(Some(_)) = db.get_account(&id) {
            continue;
        }

        let now = ctx.clock.now().to_rfc3339();
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
    ctx: &ServiceContext<'_>,
    state: &std::sync::Arc<AppState>,
    company_name: &str,
    domains: &[String],
    team_name: &str,
    colleagues: &[crate::commands::TeamColleagueInput],
    existing_person_ids: &[String],
) -> Result<crate::commands::CreateInternalOrganizationResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
    let state_for_ctx = state.clone();

    let (root_account, initial_team, created_people, updated_people) = state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
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

                    let now = ctx.clock.now().to_rfc3339();
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
                        &ctx,
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
    ctx: &ServiceContext<'_>,
    state: &std::sync::Arc<AppState>,
    parent_id: &str,
    name: &str,
    description: Option<&str>,
    owner_person_id: Option<&str>,
) -> Result<crate::commands::CreateChildAccountResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let name = crate::util::validate_entity_name(name)?.to_string();
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone());

    let parent_id = parent_id.to_string();
    let description = description.map(|s| s.to_string());
    let owner_person_id = owner_person_id.map(|s| s.to_string());
    let state_for_ctx = state.clone();

    let child_id = state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let workspace = workspace_path.as_deref().map(std::path::Path::new);
            let parent = db
                .get_account(&parent_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Parent account not found: {}", parent_id))?;
            let child = create_child_account_record(
                &ctx,
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

    let _ = state        .intel_queue
        .enqueue(crate::intel_queue::IntelRequest::new(
            child_id.clone(),
            "account".to_string(),
            crate::intel_queue::IntelPriority::ContentChange,
        ));
    state.integrations.intel_queue_wake.notify_one();

    Ok(crate::commands::CreateChildAccountResult { id: child_id })
}

/// Backfill internal meeting → account associations for meetings missing entity links.
pub fn backfill_internal_meeting_associations(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    engagement: &str,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    update_stakeholder_engagement_inner(ctx, db, state, account_id, person_id, engagement)?;
    build_account_detail_result(db, account_id)
}

fn update_stakeholder_engagement_inner(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    engagement: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
            ctx,
            tx,
            &state.signals.engine,
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
        // DOS-228 Wave 0g: stakeholder engagement feeds the `stakeholder_coverage`
        // and `key_advocate_health` health dimensions. Persist the durable marker
        // co-committed with the mutation so a crash between here and the debounce
        // flush leaves a trail for startup drain. See `update_account_field_inner`.
        tx.mark_health_recompute_pending(account_id)
            .map_err(|e| format!("failed to persist health_recompute_pending marker: {e}"))?;
        Ok(())
    })?;
    crate::services::health_debouncer::schedule_recompute(ctx, state, account_id);
    Ok(())
}

/// Update assessment text for a stakeholder with signal emission.
pub fn update_stakeholder_assessment(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    assessment: &str,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    update_stakeholder_assessment_inner(ctx, db, state, account_id, person_id, assessment)?;
    build_account_detail_result(db, account_id)
}

fn update_stakeholder_assessment_inner(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    assessment: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
            ctx,
            tx,
            &state.signals.engine,
            "account",
            account_id,
            "stakeholder_assessment_updated",
            "user_action",
            Some(&format!("{{\"person_id\":\"{}\"}}", person_id)),
            1.0,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        // DOS-228 Wave 0g: assessment edits change key_advocate_health inputs —
        // co-commit the marker with the mutation, then debounce a recompute.
        tx.mark_health_recompute_pending(account_id)
            .map_err(|e| format!("failed to persist health_recompute_pending marker: {e}"))?;
        Ok(())
    })?;
    crate::services::health_debouncer::schedule_recompute(ctx, state, account_id);
    Ok(())
}

/// Add a role to a stakeholder (multi-role — doesn't replace existing roles).
pub fn add_stakeholder_role(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    add_stakeholder_role_inner(ctx, db, state, account_id, person_id, role)?;
    build_account_detail_result(db, account_id)
}

fn add_stakeholder_role_inner(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let role = role.trim().to_lowercase();
    if role.is_empty() {
        return Err("Role is required".to_string());
    }
    db.with_transaction(|tx| {
        let now = ctx.clock.now().to_rfc3339();
        // Re-adding a role must clear any prior soft-delete tombstone.
        // Without the `dismissed_at = NULL` in the ON CONFLICT clause,
        // a user who dismisses then re-adds would see the row written
        // but still filtered out of reads (because dismissed_at stays
        // set) — effectively a silent failure.
        tx.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source, created_at)
                 VALUES (?1, ?2, ?3, 'user', ?4)
                 ON CONFLICT(account_id, person_id, role) DO UPDATE SET
                    data_source = 'user',
                    dismissed_at = NULL",
                rusqlite::params![account_id, person_id, role, now],
            )
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
            tx,
            &state.signals.engine,
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
        // DOS-228 Wave 0g: key_advocate_health reads account_stakeholder_roles
        // directly. Adding a role (especially 'champion') is load-bearing for
        // the health dimension — co-commit the pending marker.
        tx.mark_health_recompute_pending(account_id)
            .map_err(|e| format!("failed to persist health_recompute_pending marker: {e}"))?;
        Ok(())
    })?;
    crate::services::health_debouncer::schedule_recompute(ctx, state, account_id);
    Ok(())
}

/// Remove a specific role from a stakeholder.
pub fn remove_stakeholder_role(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<AccountDetailResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    remove_stakeholder_role_inner(ctx, db, state, account_id, person_id, role)?;
    build_account_detail_result(db, account_id)
}

fn remove_stakeholder_role_inner(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    person_id: &str,
    role: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        // Soft-delete: tombstone the row with `dismissed_at = now` and
        // flip provenance to 'user'. The hard-DELETE version let the
        // next enrichment cycle re-add the same role via intel_queue's
        // INSERT ON CONFLICT path — user dismissal was forgotten every
        // time AI re-surfaced the role. With the tombstone, intel_queue's
        // existence check finds data_source='user' and skips; reads
        // filter dismissed_at IS NULL so the UI doesn't show the row.
        tx.conn_ref()
            .execute(
                "UPDATE account_stakeholder_roles
                 SET dismissed_at = datetime('now'), data_source = 'user'
                 WHERE account_id = ?1 AND person_id = ?2 AND role = ?3",
                rusqlite::params![account_id, person_id, role],
            )
            .map_err(|e| e.to_string())?;
        // L2 cycle-21 fix: shadow-write the m2 stakeholder_role
        // tombstone so commit_claim PRE-GATE blocks the AI from
        // re-surfacing the dismissed role on the next enrichment.
        // Cycle-1 fix #5 audit missed this third remove path
        // (set_team_member_role at line ~2596 covers the swap case;
        // soft-delete at db/accounts.rs covers the multi-role
        // case; this single-role remove was uncovered).
        let observed_at = ctx.clock.now().to_rfc3339();
        let _ = crate::services::claims::shadow_write_tombstone_claim(
            tx,
            crate::services::claims::ShadowTombstoneClaim {
                subject_kind: "Person",
                subject_id: person_id,
                claim_type: "stakeholder_role",
                field_path: None,
                text: role,
                actor: "user",
                source_scope: Some("stakeholder_role_removed"),
                observed_at: &observed_at,
            },
        );
        crate::services::signals::emit_and_propagate(
            ctx,
            tx,
            &state.signals.engine,
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
        // DOS-228 Wave 0g: removing a role (e.g. champion) downgrades
        // key_advocate_health and stakeholder_coverage — co-commit the marker.
        tx.mark_health_recompute_pending(account_id)
            .map_err(|e| format!("failed to persist health_recompute_pending marker: {e}"))?;
        Ok(())
    })?;
    crate::services::health_debouncer::schedule_recompute(ctx, state, account_id);
    Ok(())
}

/// Accept a stakeholder suggestion: create person if needed, add to account.
pub fn accept_stakeholder_suggestion(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &std::sync::Arc<AppState>,
    suggestion_id: i64,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let account_id = db.with_transaction(|tx| {
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
        let now = ctx.clock.now().to_rfc3339();
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
            ctx,
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
        // DOS-228 Wave 0g: accepting a suggestion inserts stakeholder rows,
        // optional roles, and optional engagement — all health-scoring inputs.
        // Co-commit the marker on the same writer before returning.
        tx.mark_health_recompute_pending(&suggestion.account_id)
            .map_err(|e| format!("failed to persist health_recompute_pending marker: {e}"))?;
        Ok(suggestion.account_id.clone())
    })?;
    crate::services::health_debouncer::schedule_recompute(ctx, state, &account_id);
    Ok(())
}

/// Dismiss a stakeholder suggestion.
pub fn dismiss_stakeholder_suggestion(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    suggestion_id: i64,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
            ctx,
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
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use crate::signals::propagation::PropagationEngine;
    use crate::state::AppState;
    use chrono::TimeZone;
    use rusqlite::params;
    use serde_json::json;
    use std::path::Path;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

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

    fn test_config(workspace_path: &Path) -> crate::types::Config {
        serde_json::from_value(json!({
            "workspacePath": workspace_path.to_string_lossy(),
        }))
        .expect("minimal config should deserialize with defaults")
    }

    fn test_state_with_workspace(workspace_path: &Path) -> AppState {
        let state = AppState::new();
        *state.config.write() = Some(test_config(workspace_path));
        state
    }

    #[test]
    fn test_update_account_notes_surfaces_dashboard_write_failure() {
        let db = test_db();
        let account = make_account("acc-notes-write-failure", "Notes Write Failure Corp");
        db.upsert_account(&account).expect("upsert account");

        let temp = tempfile::tempdir().expect("tempdir");
        let workspace_file = temp.path().join("workspace-file");
        std::fs::write(&workspace_file, "not a directory").expect("workspace marker file");
        let state = test_state_with_workspace(&workspace_file);

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let err = super::update_account_notes(
            &ctx,
            &db,
            &state,
            "acc-notes-write-failure",
            "Notes that cannot be mirrored to disk",
        )
        .expect_err("dashboard write failure should surface to caller");

        assert!(
            err.contains("failed to write account dashboard.json"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_update_account_programs_surfaces_dashboard_write_failure() {
        let db = test_db();
        let account = make_account("acc-programs-write-failure", "Programs Write Failure Corp");
        db.upsert_account(&account).expect("upsert account");

        let temp = tempfile::tempdir().expect("tempdir");
        let workspace_file = temp.path().join("workspace-file");
        std::fs::write(&workspace_file, "not a directory").expect("workspace marker file");
        let state = test_state_with_workspace(&workspace_file);

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let err = super::update_account_programs(
            &ctx,
            &db,
            &state,
            "acc-programs-write-failure",
            r#"[{"name":"Migration","status":"active"}]"#,
        )
        .expect_err("dashboard write failure should surface to caller");

        assert!(
            err.contains("failed to write account dashboard.json"),
            "unexpected error: {err}"
        );
    }

    /// Test account creation at the DB level (create_account needs AppState for workspace files,
    /// so we test the underlying upsert + signal emission pattern directly).
    #[test]
    fn test_create_account_db_level() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-new", "New Corp");

        // DOS-209: ServiceContext required for mutations service.
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        // Use the mutations service which wraps upsert + signal
        crate::services::mutations::upsert_account(&ctx, &db, &engine, &account)
            .expect("upsert_account");

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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        // Archive via DB transaction (mirrors archive_account service without AppState)
        db.with_transaction(|tx| {
            tx.archive_account("acc-ar", true)
                .map_err(|e| e.to_string())?;
            crate::services::signals::emit_and_propagate(
                &ctx,
                tx,
                &engine,
                "account",
                "acc-ar",
                "entity_archived",
                "user_action",
                None,
                0.9,
            )
            .map_err(|e| e.to_string())?;
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
        super::restore_account(&ctx, &db, "acc-ar", false).expect("restore");
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

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
                &ctx,
                tx,
                &engine,
                "account",
                "acc-tm",
                "team_member_added",
                "user_action",
                None,
                0.8,
            )
            .map_err(|e| e.to_string())?;
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
                &ctx,
                tx,
                &engine,
                "account",
                "acc-tm",
                "team_member_removed",
                "user_action",
                None,
                0.7,
            )
            .map_err(|e| e.to_string())?;
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

    /// Regression test for the "AI overwrites my pinned champion" bug.
    ///
    /// Before the fix, `set_team_member_role` deleted EVERY role row for the
    /// (account, person) pair regardless of provenance. A user dropdown
    /// swap from Champion → Economic wiped out the AI-surfaced Technical
    /// role alongside the user's Champion pin. The next enrichment
    /// re-inserted Champion with `data_source='ai'`, silently erasing the
    /// human's original intent.
    ///
    /// Post-fix behavior: set_team_member_role only touches user-owned
    /// rows. AI-surfaced rows survive. A subsequent role pin can promote
    /// an AI row to user ownership via the ON CONFLICT clause.
    #[test]
    fn test_set_team_member_role_preserves_ai_owned_roles() {
        let db = test_db();
        let account = make_account("acc-rp", "RolePreserve Corp");
        db.upsert_account(&account).unwrap();

        // Seed a person and a stakeholder link
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p-rp', 'rp@x.com', 'RoleP', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source) VALUES ('acc-rp', 'p-rp', 'user')",
                [],
            )
            .unwrap();

        // Seed two existing roles: one user-pinned (champion), one
        // AI-surfaced (technical). This is the Chris-on-Globex shape.
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES ('acc-rp', 'p-rp', 'champion', 'user')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES ('acc-rp', 'p-rp', 'technical', 'ai')",
                [],
            )
            .unwrap();

        // User swaps their pinned role from Champion → Economic.
        db.set_team_member_role("acc-rp", "p-rp", "economic")
            .expect("set_team_member_role should succeed");

        // AI-surfaced 'technical' row MUST survive the user action.
        let technical_survived: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholder_roles
                 WHERE account_id = 'acc-rp' AND person_id = 'p-rp'
                   AND role = 'technical' AND data_source = 'ai'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            technical_survived, 1,
            "AI-owned 'technical' role should survive a user role swap"
        );

        // DOS-7 D4-1b: the old user-pinned 'champion' row is now SOFT-deleted
        // (dismissed_at populated) instead of hard-deleted. Reads filter
        // `WHERE dismissed_at IS NULL` so it stays invisible to enrichment + UI;
        // the row preservation is what lets the claim layer reason about
        // the user's retraction in the audit trail.
        let champion_active: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholder_roles
                 WHERE account_id = 'acc-rp' AND person_id = 'p-rp' AND role = 'champion'
                   AND dismissed_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            champion_active, 0,
            "Old user-pinned 'champion' should be soft-cleared (no active row)"
        );

        let champion_dismissed: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholder_roles
                 WHERE account_id = 'acc-rp' AND person_id = 'p-rp' AND role = 'champion'
                   AND dismissed_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            champion_dismissed, 1,
            "Old user-pinned 'champion' row should be preserved with dismissed_at set"
        );

        let economic: (i64, String) = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*), COALESCE(MAX(data_source), '')
                 FROM account_stakeholder_roles
                 WHERE account_id = 'acc-rp' AND person_id = 'p-rp' AND role = 'economic'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(economic.0, 1, "New 'economic' role should be present");
        assert_eq!(economic.1, "user", "New 'economic' role should be user-owned");
    }

    /// L2 cycle-2 fix #4: the service-level set_team_member_role
    /// shadow-write must tombstone the role being DISMISSED (e.g.
    /// 'champion'), not the role being PINNED (e.g. 'economic').
    /// The original D4-1b implementation tombstoned `new_role` —
    /// exactly inverted — which would have blocked the AI from
    /// re-surfacing the freshly-selected role on the next pass while
    /// leaving the actually-dismissed role unprotected.
    #[test]
    fn set_team_member_role_tombstones_dismissed_role_not_new_role() {
        let db = test_db();
        let account = make_account("acc-rt", "RoleTombstone Corp");
        db.upsert_account(&account).unwrap();

        // Seed person + active user-owned 'champion' role.
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) \
                 VALUES ('p-rt', 'rt@example.com', 'RoleT', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source) \
                 VALUES ('acc-rt', 'p-rt', 'user')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles \
                 (account_id, person_id, role, data_source) \
                 VALUES ('acc-rt', 'p-rt', 'champion', 'user')",
                [],
            )
            .unwrap();

        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_state_with_workspace(temp.path());
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        // Swap champion → economic via the service wrapper (the path
        // that does shadow-writes; the prior test went straight to db).
        super::set_team_member_role(&ctx, &db, &state, "acc-rt", "p-rt", "economic")
            .expect("service-level role swap");

        // Tombstone claim for OLD role 'champion' must exist.
        let champion_tombstone: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned' \
                   AND claim_type = 'stakeholder_role' \
                   AND lower(json_extract(subject_ref, '$.kind')) = 'person' \
                   AND json_extract(subject_ref, '$.id') = 'p-rt' \
                   AND text = 'champion'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            champion_tombstone, 1,
            "shadow-write must tombstone the dismissed 'champion' role"
        );

        // Tombstone claim for NEW role 'economic' must NOT exist —
        // the user just selected it.
        let economic_tombstone: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned' \
                   AND claim_type = 'stakeholder_role' \
                   AND lower(json_extract(subject_ref, '$.kind')) = 'person' \
                   AND json_extract(subject_ref, '$.id') = 'p-rt' \
                   AND text = 'economic'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            economic_tombstone, 0,
            "shadow-write must NOT tombstone the freshly-selected 'economic' role"
        );
    }

    /// When the user re-pins a role that the AI had previously surfaced,
    /// `set_team_member_role` should flip provenance to 'user' via the
    /// ON CONFLICT promotion clause rather than error or leave it as 'ai'.
    /// Regression test for the "I removed 'associated' but it came back on
    /// reload" bug. The old service-layer remove path ran a hard DELETE;
    /// next enrichment cycle the intel_queue check saw "no row exists"
    /// and re-INSERTed the role with data_source='ai'. User dismissal was
    /// forgotten every time AI re-surfaced the role.
    ///
    /// Post-fix: remove soft-deletes via UPDATE SET dismissed_at = now
    /// AND data_source = 'user'. Reads filter dismissed_at IS NULL so the
    /// UI hides the row; intel_queue's existence check sees either
    /// data_source='user' or a populated dismissed_at and skips re-insert.
    ///
    /// This test exercises the DB-level contract directly (without
    /// AppState/signals plumbing) so it focuses on the data-integrity
    /// invariant.
    #[test]
    fn test_stakeholder_role_dismissal_tombstone_and_reactivation() {
        let db = test_db();
        let account = make_account("acc-dsm", "Dismissal Corp");
        db.upsert_account(&account).unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p-dsm', 'dsm@x.com', 'DismissP', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source) VALUES ('acc-dsm', 'p-dsm', 'user')",
                [],
            )
            .unwrap();

        // Seed a user-pinned role (what `add_stakeholder_role` writes).
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES ('acc-dsm', 'p-dsm', 'associated', 'user')",
                [],
            )
            .unwrap();

        let visible_after_add: Vec<String> = db
            .get_stakeholder_roles("acc-dsm", "p-dsm")
            .unwrap()
            .into_iter()
            .map(|r| r.role)
            .collect();
        assert_eq!(visible_after_add, vec!["associated"]);

        // Simulate the soft-delete write path (UPDATE the row to
        // tombstone it rather than DELETE). dos7-allowed: this is
        // a #[cfg(test)] fixture that exercises the DB-layer
        // soft-delete shape directly; the production path
        // (remove_stakeholder_role_inner) writes the m2 shadow
        // tombstone alongside the same UPDATE.
        db.conn_ref()
            .execute(
                "UPDATE account_stakeholder_roles
                 SET dismissed_at = datetime('now'), data_source = 'user'
                 WHERE account_id = 'acc-dsm' AND person_id = 'p-dsm' AND role = 'associated'",
                [],
            )
            .unwrap();

        // Filtered reads don't see the tombstone.
        let visible_after_remove = db.get_stakeholder_roles("acc-dsm", "p-dsm").unwrap();
        assert!(
            visible_after_remove.is_empty(),
            "soft-deleted role should be hidden from read; got {:?}",
            visible_after_remove,
        );

        // Underlying row still exists as a tombstone.
        let tombstone_exists: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholder_roles
                 WHERE account_id = 'acc-dsm' AND person_id = 'p-dsm'
                   AND role = 'associated' AND dismissed_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tombstone_exists, 1, "soft-delete should leave a tombstone row");

        // Simulate the user re-adding the role (`add_stakeholder_role`'s
        // INSERT ON CONFLICT path). dismissed_at MUST clear on reactivate.
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES ('acc-dsm', 'p-dsm', 'associated', 'user')
                 ON CONFLICT(account_id, person_id, role) DO UPDATE SET
                    data_source = 'user',
                    dismissed_at = NULL",
                [],
            )
            .unwrap();
        let visible_after_readd: Vec<String> = db
            .get_stakeholder_roles("acc-dsm", "p-dsm")
            .unwrap()
            .into_iter()
            .map(|r| r.role)
            .collect();
        assert_eq!(visible_after_readd, vec!["associated"]);
        let dismissed_cleared: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT dismissed_at FROM account_stakeholder_roles
                 WHERE account_id = 'acc-dsm' AND person_id = 'p-dsm' AND role = 'associated'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(dismissed_cleared.is_none(), "re-adding should clear dismissed_at");
    }

    #[test]
    fn test_set_team_member_role_promotes_ai_row_to_user() {
        let db = test_db();
        let account = make_account("acc-pr", "Promote Corp");
        db.upsert_account(&account).unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p-pr', 'pr@x.com', 'PromoteP', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source) VALUES ('acc-pr', 'p-pr', 'ai')",
                [],
            )
            .unwrap();
        // AI had surfaced Chris as Champion; user now explicitly pins Champion.
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES ('acc-pr', 'p-pr', 'champion', 'ai')",
                [],
            )
            .unwrap();

        db.set_team_member_role("acc-pr", "p-pr", "champion")
            .expect("set_team_member_role should succeed");

        let champion: (i64, String) = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*), COALESCE(MAX(data_source), '')
                 FROM account_stakeholder_roles
                 WHERE account_id = 'acc-pr' AND person_id = 'p-pr' AND role = 'champion'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(champion.0, 1, "Champion row should still exist");
        assert_eq!(
            champion.1, "user",
            "Re-pinning an AI-surfaced role should promote data_source to 'user'",
        );
    }

    /// Test account event recording at DB level with signal emission.
    #[test]
    fn test_record_account_event() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-ev", "Event Corp");
        db.upsert_account(&account).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

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
                &ctx,
                tx,
                &engine,
                "account",
                "acc-ev",
                "account_event_recorded",
                "user_action",
                Some(r#"{"event_type":"renewal","event_date":"2026-06-15"}"#),
                0.8,
            )
            .map_err(|e| e.to_string())?;
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        super::set_account_domains(&ctx, &db, "acc-dom", &domains).expect("set_account_domains");

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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let created =
            super::bulk_create_accounts(&ctx, &db, workspace, &names).expect("bulk_create_accounts");

        // First call: 2 unique accounts created, duplicate skipped
        assert_eq!(created.len(), 2, "Should create 2 unique accounts");

        let total: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 2, "DB should have 2 accounts");

        // Second call with same names: all skipped as duplicates
        let created_again = super::bulk_create_accounts(&ctx, &db, workspace, &names)
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

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        super::ensure_account_lifecycle_state(&ctx, &db, &engine, "acc-renew")
            .expect("ensure_lifecycle");

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

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        super::confirm_lifecycle_change(&ctx, &db, &engine, change_id).expect("confirm");

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

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        super::correct_lifecycle_change(
            &ctx,
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

    /// DOS-228 Wave 0e Fix 1: the `risk_briefing_job` row must be visible
    /// on the FIRST result returned from the sentiment save. Previously the
    /// row was written by a separately spawned task and the first response
    /// had `risk_briefing_job: None`. This test simulates the writer-side
    /// sequence: persist the enqueued row on the same connection, THEN
    /// build the result — which must include the enqueued status.
    #[test]
    fn test_build_account_detail_includes_enqueued_risk_briefing_job() {
        let db = test_db();
        let account = make_account("acc-w0eb-1", "Sentiment Co");
        db.upsert_account(&account).unwrap();

        // Simulate the order production code now enforces:
        db.upsert_risk_briefing_job_enqueued("acc-w0eb-1", "attempt-xyz")
            .expect("enqueue on writer");
        let result = super::build_account_detail_result(&db, "acc-w0eb-1")
            .expect("build");

        let job = result
            .risk_briefing_job
            .as_ref()
            .expect("risk_briefing_job must be present on first response");
        assert_eq!(job.status, "enqueued");
        assert!(job.error_message.is_none());
    }
}
