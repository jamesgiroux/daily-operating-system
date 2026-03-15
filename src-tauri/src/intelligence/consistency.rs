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

    Ok(FactContext {
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
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
            entity_id: "assetco".to_string(),
            entity_type: "account".to_string(),
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
}
