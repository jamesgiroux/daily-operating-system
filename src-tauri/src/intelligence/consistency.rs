//! Deterministic consistency checks for intelligence output (I527).
//!
//! This module detects contradictions between AI-generated intelligence text
//! and verifiable local facts (meeting attendance + recent signal activity).
//! It also provides deterministic repair helpers and a compact repair prompt
//! for a single retry when high-severity contradictions remain.

use std::collections::{HashMap, HashSet};

use chrono::{Duration, Local, Utc};
use regex::Regex;

use crate::db::ActionDb;
use crate::intelligence::{
    ConsistencyFinding, ConsistencySeverity, ConsistencyStatus, IntelligenceJson,
};
use crate::util::wrap_user_data;

const ABSENCE_PATTERNS: &[&str] = &[
    "never appeared in a recorded meeting",
    "never appeared in recorded meeting",
    "never appeared in recorded meetings",
    "has never appeared",
    "never attended",
    "has not appeared in any recorded meeting",
];

const NO_PROGRESS_PATTERNS: &[&str] = &[
    "no new progress signals since the prior assessment",
    "no new progress signals since prior assessment",
    "no new progress signals",
    "no progress signals since",
];

const AUTHORITY_UNKNOWN_PATTERNS: &[&str] = &[
    "authority and stance on",
    "actual authority and stance",
    "authority is unknown",
];

#[derive(Debug, Clone, Default)]
pub struct StakeholderFact {
    pub person_id: String,
    pub name: String,
    pub role: Option<String>,
    pub linked_to_entity: bool,
    pub attendance_count: u32,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FactContext {
    pub entity_id: String,
    pub entity_type: String,
    pub entity_name: String,
    /// Names of parent/child entities — mentions of these are NOT cross-entity bleed.
    pub related_entity_names: Vec<String>,
    /// Names of ALL other entities in the portfolio (excluding self and related).
    /// Used for positive identification of cross-entity contamination.
    pub all_other_entity_names: Vec<String>,
    pub stakeholders: Vec<StakeholderFact>,
    pub recent_signal_count_14d: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ConsistencyReport {
    pub findings: Vec<ConsistencyFinding>,
}

impl ConsistencyReport {
    pub fn has_high(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f.severity, ConsistencySeverity::High))
    }
}

pub fn build_fact_context(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
) -> Result<FactContext, String> {
    let mut by_person: HashMap<String, StakeholderFact> = HashMap::new();

    for person in db.get_people_for_entity(entity_id).unwrap_or_default() {
        by_person.insert(
            person.id.clone(),
            StakeholderFact {
                person_id: person.id,
                name: person.name,
                role: person.role,
                linked_to_entity: true,
                attendance_count: 0,
                last_seen: None,
            },
        );
    }

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT p.id, p.name, p.role, COUNT(DISTINCT ma.meeting_id) AS attendance_count, \
                    MAX(mh.start_time) AS last_seen \
             FROM meeting_entities me \
             JOIN meeting_attendees ma ON ma.meeting_id = me.meeting_id \
             JOIN people p ON p.id = ma.person_id \
             JOIN meetings mh ON mh.id = me.meeting_id \
             WHERE me.entity_id = ?1 \
             GROUP BY p.id, p.name, p.role",
        )
        .map_err(|e| format!("Failed to query attendance facts: {e}"))?;

    let rows = stmt
        .query_map([entity_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| format!("Failed to map attendance facts: {e}"))?;

    for row in rows.filter_map(Result::ok) {
        let (person_id, name, role, count, last_seen) = row;
        let entry = by_person
            .entry(person_id.clone())
            .or_insert(StakeholderFact {
                person_id,
                name,
                role: role.clone(),
                linked_to_entity: false,
                attendance_count: 0,
                last_seen: None,
            });
        entry.role = entry.role.clone().or(role);
        entry.attendance_count = count.max(0) as u32;
        entry.last_seen = last_seen;
    }

    let cutoff = (Utc::now() - Duration::days(14)).to_rfc3339();
    let recent_signal_count_14d: u32 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM signal_events \
             WHERE entity_id = ?1 AND entity_type = ?2 \
               AND superseded_by IS NULL \
               AND created_at >= ?3",
            rusqlite::params![entity_id, entity_type, cutoff],
            |row| row.get::<_, i64>(0),
        )
        .map(|v| v.max(0) as u32)
        .unwrap_or(0);

    let mut stakeholders = by_person.into_values().collect::<Vec<_>>();
    stakeholders.sort_by(|a, b| {
        b.attendance_count
            .cmp(&a.attendance_count)
            .then_with(|| b.last_seen.cmp(&a.last_seen))
            .then_with(|| a.name.cmp(&b.name))
    });

    // Resolve entity name and related names (parent/children) for bleed detection.
    let (entity_name, related_entity_names) = resolve_entity_names(db, entity_id, entity_type);

    // Query all other entity names for positive bleed identification.
    let all_other_entity_names = get_all_other_entity_names(db, entity_id, &entity_name, &related_entity_names);

    Ok(FactContext {
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        entity_name,
        related_entity_names,
        all_other_entity_names,
        stakeholders,
        recent_signal_count_14d,
    })
}

/// Format compact, deterministic attendance lines for prompt grounding.
pub fn format_verified_presence_lines(facts: &FactContext, limit: usize) -> Vec<String> {
    facts
        .stakeholders
        .iter()
        .filter(|s| s.attendance_count > 0)
        .take(limit)
        .map(|s| {
            let last_seen = s
                .last_seen
                .as_deref()
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| {
                    dt.with_timezone(&Local)
                        .format("%Y-%m-%d %-I:%M %p %Z")
                        .to_string()
                })
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                "- {} — appears in {} recorded meetings (last seen {})",
                s.name, s.attendance_count, last_seen
            )
        })
        .collect()
}

/// Resolve entity name and parent/child names for bleed detection.
fn resolve_entity_names(db: &ActionDb, entity_id: &str, entity_type: &str) -> (String, Vec<String>) {
    let mut entity_name = String::new();
    let mut related = Vec::new();

    match entity_type {
        "account" => {
            if let Ok(Some(acct)) = db.get_account(entity_id) {
                entity_name = acct.name;
                // Add parent name if present
                if let Some(pid) = acct.parent_id.as_deref() {
                    if let Ok(Some(parent)) = db.get_account(pid) {
                        related.push(parent.name);
                    }
                }
                // Add child names
                if let Ok(children) = db.get_child_accounts(entity_id) {
                    for child in children {
                        related.push(child.name);
                    }
                }
            }
        }
        "project" => {
            if let Ok(Some(proj)) = db.get_project(entity_id) {
                entity_name = proj.name;
                // Add parent name if present
                if let Some(pid) = proj.parent_id.as_deref() {
                    if let Ok(Some(parent)) = db.get_project(pid) {
                        related.push(parent.name);
                    }
                }
                // Add child names
                if let Ok(children) = db.get_child_projects(entity_id) {
                    for child in children {
                        related.push(child.name);
                    }
                }
            }
        }
        _ => {}
    }

    (entity_name, related)
}

/// Query all account and project names from the DB, excluding the current entity,
/// its related (parent/child) entities, and names <= 3 chars.
fn get_all_other_entity_names(
    db: &ActionDb,
    entity_id: &str,
    entity_name: &str,
    related_names: &[String],
) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();

    // Build set of names to exclude (case-insensitive)
    let mut excluded: HashSet<String> = HashSet::new();
    excluded.insert(entity_name.to_lowercase());
    for r in related_names {
        excluded.insert(r.to_lowercase());
    }

    let query = "SELECT DISTINCT name FROM accounts WHERE id != ?1 AND name IS NOT NULL AND archived = 0 \
                 UNION \
                 SELECT DISTINCT name FROM projects WHERE id != ?1 AND name IS NOT NULL AND archived = 0";
    if let Ok(mut stmt) = db.conn_ref().prepare(query) {
        if let Ok(rows) = stmt.query_map([entity_id], |row| row.get::<_, String>(0)) {
            for row in rows.filter_map(Result::ok) {
                let trimmed = row.trim().to_string();
                // Skip short names (<=3 chars) to avoid false positives on abbreviations
                if trimmed.len() <= 3 {
                    continue;
                }
                // Skip self and related
                if excluded.contains(&trimmed.to_lowercase()) {
                    continue;
                }
                names.push(trimmed);
            }
        }
    }

    names
}

/// Check if a text field mentions a known OTHER entity name from the portfolio.
/// Uses positive identification: only flags when a specific known account/project
/// name appears in text belonging to a different entity.
fn check_text_entity_bleed(
    intel: &IntelligenceJson,
    facts: &FactContext,
    findings: &mut Vec<ConsistencyFinding>,
    seen: &mut HashSet<String>,
) {
    if facts.entity_name.is_empty() || facts.all_other_entity_names.is_empty() {
        return;
    }

    // Collect all text fields worth checking
    let mut fields: Vec<(String, String)> = Vec::new();

    if let Some(text) = intel.executive_assessment.as_ref() {
        fields.push(("executiveAssessment".to_string(), text.clone()));
    }
    if let Some(ref ctx) = intel.company_context {
        if let Some(desc) = ctx.description.as_ref() {
            fields.push(("companyContext.description".to_string(), desc.clone()));
        }
        if let Some(extra) = ctx.additional_context.as_ref() {
            fields.push((
                "companyContext.additionalContext".to_string(),
                extra.clone(),
            ));
        }
    }
    if let Some(ref health) = intel.health {
        if let Some(ref narrative) = health.narrative {
            fields.push(("health.narrative".to_string(), narrative.clone()));
        }
    }
    if let Some(ref metrics) = intel.success_metrics {
        for (idx, metric) in metrics.iter().enumerate() {
            fields.push((format!("successMetrics[{idx}].name"), metric.name.clone()));
        }
    }

    // Build word-boundary regex patterns for each known other entity name.
    // We pre-compile them once and reuse across all fields.
    let entity_patterns: Vec<(String, Regex)> = facts
        .all_other_entity_names
        .iter()
        .filter_map(|name| {
            let escaped = regex::escape(name);
            // Word-boundary-aware: entity name must appear as a whole word/phrase
            Regex::new(&format!(r"(?i)\b{}\b", escaped))
                .ok()
                .map(|re| (name.clone(), re))
        })
        .collect();

    for (field_path, text) in &fields {
        // Check if any known other entity name appears in the text
        for (foreign_name, pattern) in &entity_patterns {
            if pattern.is_match(text) {
                push_unique(
                    findings,
                    seen,
                    ConsistencyFinding {
                        code: "CROSS_ENTITY_CONTENT_BLEED".to_string(),
                        severity: ConsistencySeverity::High,
                        field_path: field_path.clone(),
                        claim_text: if text.len() > 200 {
                            format!("{}...", &text[..200])
                        } else {
                            text.clone()
                        },
                        evidence_text: format!(
                            "Text for entity '{}' mentions another entity '{}' from the portfolio.",
                            facts.entity_name, foreign_name
                        ),
                        auto_fixed: false,
                    },
                );
                // One finding per field is enough — break after first match
                break;
            }
        }
    }
}

pub fn check_consistency(intel: &IntelligenceJson, facts: &FactContext) -> ConsistencyReport {
    let mut findings: Vec<ConsistencyFinding> = Vec::new();
    let mut seen = HashSet::new();

    let mut fields: Vec<(String, String)> = Vec::new();
    if let Some(text) = intel.executive_assessment.as_ref() {
        fields.push(("executiveAssessment".to_string(), text.clone()));
    }
    for (idx, risk) in intel.risks.iter().enumerate() {
        fields.push((format!("risks[{idx}].text"), risk.text.clone()));
    }
    for (idx, stakeholder) in intel.stakeholder_insights.iter().enumerate() {
        if let Some(text) = stakeholder.assessment.as_ref() {
            fields.push((
                format!("stakeholderInsights[{idx}].assessment"),
                text.clone(),
            ));
        }
    }

    for (field_path, text) in &fields {
        let lower = text.to_lowercase();
        if contains_any(&lower, NO_PROGRESS_PATTERNS) && facts.recent_signal_count_14d >= 2 {
            push_unique(
                &mut findings,
                &mut seen,
                ConsistencyFinding {
                    code: "NO_PROGRESS_CONTRADICTION".to_string(),
                    severity: ConsistencySeverity::High,
                    field_path: field_path.clone(),
                    claim_text: text.clone(),
                    evidence_text: format!(
                        "{} active signals found in the last 14 days for entity {}.",
                        facts.recent_signal_count_14d, facts.entity_id
                    ),
                    auto_fixed: false,
                },
            );
        }

        for person in facts.stakeholders.iter().filter(|p| p.attendance_count > 0) {
            if mentions_person(&lower, &person.name) && contains_any(&lower, ABSENCE_PATTERNS) {
                push_unique(
                    &mut findings,
                    &mut seen,
                    ConsistencyFinding {
                        code: "ABSENCE_CONTRADICTION".to_string(),
                        severity: ConsistencySeverity::High,
                        field_path: field_path.clone(),
                        claim_text: text.clone(),
                        evidence_text: format!(
                            "{} appears in {} recorded meetings{}.",
                            person.name,
                            person.attendance_count,
                            person
                                .last_seen
                                .as_ref()
                                .map(|ts| format!(" (last seen {})", ts))
                                .unwrap_or_default()
                        ),
                        auto_fixed: false,
                    },
                );
            }

            if mentions_person(&lower, &person.name)
                && contains_any(&lower, AUTHORITY_UNKNOWN_PATTERNS)
                && lower.contains("unknown")
                && (person.role.as_deref().is_some_and(|r| !r.trim().is_empty())
                    || person.attendance_count > 0)
            {
                push_unique(
                    &mut findings,
                    &mut seen,
                    ConsistencyFinding {
                        code: "AUTHORITY_UNKNOWN_CONTRADICTION".to_string(),
                        severity: ConsistencySeverity::Medium,
                        field_path: field_path.clone(),
                        claim_text: text.clone(),
                        evidence_text: format!(
                            "{} has role '{}' and {} recorded meeting(s).",
                            person.name,
                            person.role.as_deref().unwrap_or("unknown"),
                            person.attendance_count
                        ),
                        auto_fixed: false,
                    },
                );
            }
        }
    }

    let linked_names: HashSet<String> = facts
        .stakeholders
        .iter()
        .filter(|p| p.linked_to_entity)
        .map(|p| normalize_name(&p.name))
        .collect();
    let attendance_by_name: HashMap<String, u32> = facts
        .stakeholders
        .iter()
        .map(|p| (normalize_name(&p.name), p.attendance_count))
        .collect();

    for (idx, stakeholder) in intel.stakeholder_insights.iter().enumerate() {
        let key = normalize_name(&stakeholder.name);
        let linked = linked_names.contains(&key);
        let attendance = attendance_by_name.get(&key).copied().unwrap_or(0);
        if !linked && attendance == 0 {
            push_unique(
                &mut findings,
                &mut seen,
                ConsistencyFinding {
                    code: "CROSS_ENTITY_BLEED_SUSPECT".to_string(),
                    severity: ConsistencySeverity::Medium,
                    field_path: format!("stakeholderInsights[{idx}].name"),
                    claim_text: stakeholder.name.clone(),
                    evidence_text: format!(
                        "{} is not linked to entity {} and has no recorded attendance in linked meetings.",
                        stakeholder.name, facts.entity_id
                    ),
                    auto_fixed: false,
                },
            );
        }
    }

    // DOS-83: Check text fields for cross-entity content contamination.
    check_text_entity_bleed(intel, facts, &mut findings, &mut seen);

    ConsistencyReport { findings }
}

pub fn apply_deterministic_repairs(
    intel: &IntelligenceJson,
    report: &ConsistencyReport,
    _facts: &FactContext,
) -> IntelligenceJson {
    let mut out = intel.clone();

    for finding in &report.findings {
        match finding.code.as_str() {
            "ABSENCE_CONTRADICTION" => {
                apply_text_repair(&mut out, &finding.field_path, repair_absence_claim);
            }
            "NO_PROGRESS_CONTRADICTION" => {
                apply_text_repair(&mut out, &finding.field_path, repair_no_progress_claim);
            }
            "AUTHORITY_UNKNOWN_CONTRADICTION" => {
                apply_text_repair(
                    &mut out,
                    &finding.field_path,
                    repair_authority_unknown_claim,
                );
            }
            "CROSS_ENTITY_BLEED_SUSPECT" => {
                apply_bleed_repair(&mut out, &finding.field_path);
            }
            _ => {}
        }
    }

    out
}

pub fn merge_fixed_flags(
    original: &ConsistencyReport,
    unresolved: &ConsistencyReport,
) -> Vec<ConsistencyFinding> {
    let unresolved_keys: HashSet<String> = unresolved
        .findings
        .iter()
        .map(finding_key)
        .collect::<HashSet<_>>();

    original
        .findings
        .iter()
        .cloned()
        .map(|mut f| {
            if !unresolved_keys.contains(&finding_key(&f)) {
                f.auto_fixed = true;
            }
            f
        })
        .collect()
}

/// Build a compact repair prompt for a single retry pass.
pub fn build_repair_prompt(
    intel: &IntelligenceJson,
    report: &ConsistencyReport,
    facts: &FactContext,
) -> String {
    let intel_json = serde_json::to_string_pretty(intel).unwrap_or_else(|_| "{}".to_string());
    let findings = report
        .findings
        .iter()
        .map(|f| {
            format!(
                "- [{}] {} @ {} | claim: {} | evidence: {}",
                match f.severity {
                    ConsistencySeverity::High => "high",
                    ConsistencySeverity::Medium => "medium",
                    ConsistencySeverity::Low => "low",
                },
                f.code,
                f.field_path,
                f.claim_text,
                f.evidence_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let verified_presence = format_verified_presence_lines(facts, 10).join("\n");

    format!(
        "You are repairing a previously generated intelligence JSON object.\n\
         Task: correct contradictory statements using deterministic evidence below.\n\
         Keep structure and tone intact. Change only contradictory claims.\n\
         Return ONLY valid JSON (no markdown, no commentary).\n\n\
         ## Deterministic Contradictions\n\
         {}\n\n\
         ## Verified Stakeholder Meeting Presence\n\
         {}\n\n\
         ## Current Intelligence JSON\n\
         {}\n",
        wrap_user_data(&findings),
        wrap_user_data(&verified_presence),
        wrap_user_data(&intel_json),
    )
}

pub fn status_from_reports(
    initial: &ConsistencyReport,
    unresolved: &ConsistencyReport,
) -> ConsistencyStatus {
    if initial.findings.is_empty() {
        return ConsistencyStatus::Ok;
    }
    if unresolved.findings.is_empty() {
        return ConsistencyStatus::Corrected;
    }
    ConsistencyStatus::Flagged
}

fn finding_key(f: &ConsistencyFinding) -> String {
    format!("{}|{}|{}", f.code, f.field_path, f.claim_text)
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn mentions_person(text_lower: &str, name: &str) -> bool {
    let norm = normalize_name(name);
    if norm.is_empty() {
        return false;
    }
    if text_lower.contains(&norm) {
        return true;
    }
    if let Some(first) = norm.split_whitespace().next() {
        if first.len() >= 4 && text_lower.contains(first) {
            return true;
        }
    }
    false
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

fn push_unique(
    findings: &mut Vec<ConsistencyFinding>,
    seen: &mut HashSet<String>,
    finding: ConsistencyFinding,
) {
    let key = finding_key(&finding);
    if seen.insert(key) {
        findings.push(finding);
    }
}

fn apply_text_repair<F>(intel: &mut IntelligenceJson, field_path: &str, f: F)
where
    F: Fn(&str) -> String,
{
    if field_path == "executiveAssessment" {
        if let Some(text) = intel.executive_assessment.as_mut() {
            *text = f(text);
        }
        return;
    }

    if let Some(idx) = parse_index(field_path, "risks[", "].text") {
        if let Some(risk) = intel.risks.get_mut(idx) {
            risk.text = f(&risk.text);
        }
        return;
    }

    if let Some(idx) = parse_index(field_path, "stakeholderInsights[", "].assessment") {
        if let Some(stakeholder) = intel.stakeholder_insights.get_mut(idx) {
            if let Some(assessment) = stakeholder.assessment.as_mut() {
                *assessment = f(assessment);
            }
        }
    }
}

fn apply_bleed_repair(intel: &mut IntelligenceJson, field_path: &str) {
    if let Some(idx) = parse_index(field_path, "stakeholderInsights[", "].name") {
        if let Some(stakeholder) = intel.stakeholder_insights.get_mut(idx) {
            stakeholder.assessment = Some(
                "Role and influence need direct verification in a customer-facing meeting."
                    .to_string(),
            );
            stakeholder.engagement = Some("unknown".to_string());
        }
    }
}

fn parse_index(field_path: &str, prefix: &str, suffix: &str) -> Option<usize> {
    let start = field_path.strip_prefix(prefix)?;
    let idx_str = start.strip_suffix(suffix)?;
    idx_str.parse::<usize>().ok()
}

fn repair_absence_claim(text: &str) -> String {
    let mut out = text.to_string();
    for (pattern, replacement) in [
        (
            r"(?i)has never appeared in a recorded meeting",
            "has appeared in recorded meetings",
        ),
        (
            r"(?i)never appeared in recorded meetings?",
            "appeared in recorded meetings",
        ),
        (r"(?i)has never appeared", "has appeared"),
        (r"(?i)never attended", "has attended"),
    ] {
        if let Ok(re) = Regex::new(pattern) {
            out = re.replace_all(&out, replacement).to_string();
        }
    }
    out
}

fn repair_no_progress_claim(text: &str) -> String {
    let mut out = text.to_string();
    for (pattern, replacement) in [
        (
            r"(?i)no new progress signals since (the )?prior assessment",
            "recent progress signals are present since the prior assessment",
        ),
        (
            r"(?i)no new progress signals",
            "recent progress signals are present",
        ),
    ] {
        if let Ok(re) = Regex::new(pattern) {
            out = re.replace_all(&out, replacement).to_string();
        }
    }
    out
}

fn repair_authority_unknown_claim(text: &str) -> String {
    if let Ok(re) = Regex::new(r"(?i)actual authority and stance .* unknown") {
        let replaced = re
            .replace_all(
                text,
                "authority and stance require direct confirmation in the next meeting",
            )
            .to_string();
        if replaced != text {
            return replaced;
        }
    }

    if let Ok(re) = Regex::new(r"(?i)authority .* unknown") {
        return re
            .replace_all(
                text,
                "authority should be validated explicitly in the next meeting",
            )
            .to_string();
    }

    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::{IntelRisk, StakeholderInsight};

    fn fact_context_with_matt() -> FactContext {
        FactContext {
            entity_id: "janus-henderson".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Meridian Asset".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec![],
            stakeholders: vec![StakeholderFact {
                person_id: "p1".to_string(),
                name: "Matt Wickham".to_string(),
                role: Some("Director".to_string()),
                linked_to_entity: true,
                attendance_count: 2,
                last_seen: Some("2026-03-01T15:00:00Z".to_string()),
            }],
            recent_signal_count_14d: 3,
        }
    }

    #[test]
    fn detects_absence_contradiction() {
        let facts = fact_context_with_matt();
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Matt Wickham has never appeared in a recorded meeting.".to_string(),
            ),
            ..Default::default()
        };

        let report = check_consistency(&intel, &facts);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "ABSENCE_CONTRADICTION"));
    }

    #[test]
    fn detects_no_progress_contradiction() {
        let facts = fact_context_with_matt();
        let intel = IntelligenceJson {
            risks: vec![IntelRisk {
                text: "No new progress signals since the prior assessment.".to_string(),
                source: None,
                urgency: "watch".to_string(),
                item_source: None,
                discrepancy: None,
            }],
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "NO_PROGRESS_CONTRADICTION"));
    }

    #[test]
    fn deterministic_repair_rewrites_absence_claim() {
        let facts = fact_context_with_matt();
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Matt Wickham has never appeared in a recorded meeting.".to_string(),
            ),
            ..Default::default()
        };
        let initial = check_consistency(&intel, &facts);
        let repaired = apply_deterministic_repairs(&intel, &initial, &facts);
        assert!(repaired
            .executive_assessment
            .as_deref()
            .is_some_and(|t| t.contains("has appeared in recorded meetings")));
    }

    #[test]
    fn marks_auto_fixed_findings() {
        let initial = ConsistencyReport {
            findings: vec![ConsistencyFinding {
                code: "ABSENCE_CONTRADICTION".to_string(),
                severity: ConsistencySeverity::High,
                field_path: "executiveAssessment".to_string(),
                claim_text: "never appeared".to_string(),
                evidence_text: "attended 2 meetings".to_string(),
                auto_fixed: false,
            }],
        };
        let unresolved = ConsistencyReport { findings: vec![] };
        let merged = merge_fixed_flags(&initial, &unresolved);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].auto_fixed);
    }

    #[test]
    fn detects_bleed_for_unlinked_stakeholder() {
        let facts = FactContext {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Acme Corp".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec![],
            stakeholders: vec![StakeholderFact {
                person_id: "p1".to_string(),
                name: "Alice".to_string(),
                role: Some("VP".to_string()),
                linked_to_entity: true,
                attendance_count: 1,
                last_seen: None,
            }],
            recent_signal_count_14d: 0,
        };
        let intel = IntelligenceJson {
            stakeholder_insights: vec![StakeholderInsight {
                name: "Unrelated Person".to_string(),
                role: Some("Manager".to_string()),
                assessment: Some("High influence".to_string()),
                engagement: Some("high".to_string()),
                source: None,
                person_id: None,
                suggested_person_id: None,
                item_source: None,
                discrepancy: None,
            }],
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "CROSS_ENTITY_BLEED_SUSPECT"));
    }

    #[test]
    fn detects_content_bleed_when_known_other_entity_appears() {
        let facts = FactContext {
            entity_id: "globex".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Globex Holdings".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec!["Clevertap".to_string(), "Acme Industries".to_string()],
            stakeholders: vec![],
            recent_signal_count_14d: 0,
        };
        // Assessment mentions Clevertap — a known other entity
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Clevertap is showing strong growth in their mobile analytics platform. \
                 The team has expanded significantly this quarter."
                    .to_string(),
            ),
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "CROSS_ENTITY_CONTENT_BLEED"),
            "Should detect content bleed when text mentions a known other entity"
        );
    }

    #[test]
    fn no_false_positive_for_correct_entity() {
        let facts = FactContext {
            entity_id: "globex".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Globex Holdings".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec!["Clevertap".to_string()],
            stakeholders: vec![],
            recent_signal_count_14d: 0,
        };
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Globex Holdings continues to demonstrate strong engagement with the platform."
                    .to_string(),
            ),
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.code == "CROSS_ENTITY_CONTENT_BLEED"),
            "Should not flag content that correctly references the entity"
        );
    }

    #[test]
    fn no_false_positive_for_parent_child_reference() {
        // "Salesforce" is in related_entity_names, NOT in all_other_entity_names
        let facts = FactContext {
            entity_id: "salesforce-bu".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Salesforce BU".to_string(),
            related_entity_names: vec!["Salesforce".to_string()],
            all_other_entity_names: vec!["Clevertap".to_string()],
            stakeholders: vec![],
            recent_signal_count_14d: 0,
        };
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Salesforce continues to invest in this business unit's growth."
                    .to_string(),
            ),
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.code == "CROSS_ENTITY_CONTENT_BLEED"),
            "Should not flag parent entity references as bleed"
        );
    }

    #[test]
    fn no_false_positive_for_common_business_phrases() {
        let facts = FactContext {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Acme Corp".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec!["Clevertap".to_string(), "Globex Industries".to_string()],
            stakeholders: vec![],
            recent_signal_count_14d: 0,
        };
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Customer Success team shows Executive alignment on the Enterprise platform. \
                 Revenue growth is strong with quarterly milestones on track."
                    .to_string(),
            ),
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.code == "CROSS_ENTITY_CONTENT_BLEED"),
            "Should not flag common business phrases as bleed"
        );
    }

    #[test]
    fn short_entity_names_are_skipped() {
        // "IBM" is only 3 chars and should be filtered out by get_all_other_entity_names.
        // Simulate that by NOT including it in all_other_entity_names.
        let facts = FactContext {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Acme Corp".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec![], // Short names excluded during population
            stakeholders: vec![],
            recent_signal_count_14d: 0,
        };
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "IBM has been a strong partner in the enterprise space.".to_string(),
            ),
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.code == "CROSS_ENTITY_CONTENT_BLEED"),
            "Short entity names should not trigger bleed detection"
        );
    }

    #[test]
    fn bleed_detection_is_case_insensitive() {
        let facts = FactContext {
            entity_id: "globex".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Globex Holdings".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec!["Acme Industries".to_string()],
            stakeholders: vec![],
            recent_signal_count_14d: 0,
        };
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "acme industries has been growing steadily this quarter.".to_string(),
            ),
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "CROSS_ENTITY_CONTENT_BLEED"),
            "Bleed detection should be case-insensitive"
        );
    }

    #[test]
    fn no_bleed_when_no_other_entities_exist() {
        let facts = FactContext {
            entity_id: "only-one".to_string(),
            entity_type: "account".to_string(),
            entity_name: "Only One Corp".to_string(),
            related_entity_names: vec![],
            all_other_entity_names: vec![], // No other entities in portfolio
            stakeholders: vec![],
            recent_signal_count_14d: 0,
        };
        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Random Company is doing great things with their platform.".to_string(),
            ),
            ..Default::default()
        };
        let report = check_consistency(&intel, &facts);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.code == "CROSS_ENTITY_CONTENT_BLEED"),
            "No bleed when there are no other entities to match against"
        );
    }
}
