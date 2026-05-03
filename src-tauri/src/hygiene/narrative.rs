//! Report building, narrative construction, and UI view models for hygiene.

use chrono::Utc;
use serde::Serialize;
use std::path::Path;

use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::Config;

use super::{HygieneFixDetail, HygieneReport};

/// Default overnight AI budget (higher than daytime).
const OVERNIGHT_AI_BUDGET: u32 = 20;

// =============================================================================
// View Model Types
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneFixView {
    pub key: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneGapActionView {
    pub kind: String,
    pub label: String,
    pub route: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneGapView {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub impact: String,
    pub description: String,
    pub action: HygieneGapActionView,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneBudgetView {
    pub used_today: u32,
    pub daily_limit: u32,
    pub queued_for_next_budget: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneStatusView {
    pub status: String,
    pub status_label: String,
    pub last_scan_time: Option<String>,
    pub next_scan_time: Option<String>,
    pub total_gaps: usize,
    pub total_fixes: usize,
    pub is_running: bool,
    pub fixes: Vec<HygieneFixView>,
    pub fix_details: Vec<HygieneFixDetail>,
    pub gaps: Vec<HygieneGapView>,
    pub budget: HygieneBudgetView,
    pub scan_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneNarrativeView {
    pub narrative: String,
    pub remaining_gaps: Vec<HygieneGapSummary>,
    pub last_scan_time: Option<String>,
    pub total_fixes: usize,
    pub total_remaining_gaps: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneGapSummary {
    pub label: String,
    pub count: usize,
    pub severity: String, // "critical" | "medium" | "low"
}

/// Overnight maintenance report.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OvernightReport {
    pub ran_at: String,
    pub entities_refreshed: usize,
    pub names_resolved: usize,
    pub summaries_extracted: usize,
    pub relationships_reclassified: usize,
}

// =============================================================================
// Narrative Building
// =============================================================================

/// Join items into prose: "a, b, and c"
fn join_prose_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

pub fn build_hygiene_narrative(report: &HygieneReport) -> Option<HygieneNarrativeView> {
    // Build fix descriptions
    let mut fix_parts: Vec<String> = Vec::new();
    let fixes = &report.fixes;
    if fixes.names_resolved > 0 {
        fix_parts.push(format!(
            "resolved {} unnamed {}",
            fixes.names_resolved,
            if fixes.names_resolved == 1 {
                "person"
            } else {
                "people"
            }
        ));
    }
    if fixes.relationships_reclassified > 0 {
        fix_parts.push(format!(
            "reclassified {} {}",
            fixes.relationships_reclassified,
            if fixes.relationships_reclassified == 1 {
                "relationship"
            } else {
                "relationships"
            }
        ));
    }
    if fixes.summaries_extracted > 0 {
        fix_parts.push(format!(
            "extracted {} {}",
            fixes.summaries_extracted,
            if fixes.summaries_extracted == 1 {
                "summary"
            } else {
                "summaries"
            }
        ));
    }
    if fixes.meeting_counts_updated > 0 {
        fix_parts.push(format!(
            "updated {} meeting {}",
            fixes.meeting_counts_updated,
            if fixes.meeting_counts_updated == 1 {
                "count"
            } else {
                "counts"
            }
        ));
    }
    if fixes.people_linked_by_domain > 0 {
        fix_parts.push(format!(
            "linked {} {} by domain",
            fixes.people_linked_by_domain,
            if fixes.people_linked_by_domain == 1 {
                "person"
            } else {
                "people"
            }
        ));
    }
    if fixes.renewals_rolled_over > 0 {
        fix_parts.push(format!(
            "rolled over {} {}",
            fixes.renewals_rolled_over,
            if fixes.renewals_rolled_over == 1 {
                "renewal"
            } else {
                "renewals"
            }
        ));
    }
    if fixes.ai_enrichments_enqueued > 0 {
        fix_parts.push(format!(
            "queued {} intelligence {}",
            fixes.ai_enrichments_enqueued,
            if fixes.ai_enrichments_enqueued == 1 {
                "refresh"
            } else {
                "refreshes"
            }
        ));
    }
    if fixes.phantom_accounts_archived > 0 {
        fix_parts.push(format!(
            "archived {} phantom {}",
            fixes.phantom_accounts_archived,
            if fixes.phantom_accounts_archived == 1 {
                "account"
            } else {
                "accounts"
            }
        ));
    }
    if fixes.orphan_internals_relinked > 0 {
        fix_parts.push(format!(
            "re-linked {} orphan internal {}",
            fixes.orphan_internals_relinked,
            if fixes.orphan_internals_relinked == 1 {
                "account"
            } else {
                "accounts"
            }
        ));
    }
    if fixes.empty_shells_archived > 0 {
        fix_parts.push(format!(
            "archived {} empty shell {}",
            fixes.empty_shells_archived,
            if fixes.empty_shells_archived == 1 {
                "account"
            } else {
                "accounts"
            }
        ));
    }
    if fixes.people_auto_merged > 0 {
        fix_parts.push(format!(
            "merged {} duplicate {}",
            fixes.people_auto_merged,
            if fixes.people_auto_merged == 1 {
                "person"
            } else {
                "people"
            }
        ));
    }
    if fixes.names_resolved_calendar > 0 {
        fix_parts.push(format!(
            "resolved {} {} from calendar",
            fixes.names_resolved_calendar,
            if fixes.names_resolved_calendar == 1 {
                "name"
            } else {
                "names"
            }
        ));
    }
    if fixes.people_linked_co_attendance > 0 {
        fix_parts.push(format!(
            "linked {} {} by co-attendance",
            fixes.people_linked_co_attendance,
            if fixes.people_linked_co_attendance == 1 {
                "person"
            } else {
                "people"
            }
        ));
    }

    // Build gap summaries
    let mut remaining_gaps: Vec<HygieneGapSummary> = Vec::new();
    let gap_rows: Vec<(&str, usize, &str)> = vec![
        ("unnamed people", report.unnamed_people, "critical"),
        ("duplicate people", report.duplicate_people, "critical"),
        (
            "unknown relationships",
            report.unknown_relationships,
            "medium",
        ),
        (
            "missing intelligence",
            report.missing_intelligence,
            "medium",
        ),
        ("stale intelligence", report.stale_intelligence, "low"),
        ("unsummarized files", report.unsummarized_files, "medium"),
        ("empty shell accounts", report.empty_shell_accounts, "low"),
    ];
    for (label, count, severity) in &gap_rows {
        if *count > 0 {
            remaining_gaps.push(HygieneGapSummary {
                label: format!("{} {}", count, label),
                count: *count,
                severity: severity.to_string(),
            });
        }
    }

    let total_fix_count = fixes.names_resolved
        + fixes.relationships_reclassified
        + fixes.summaries_extracted
        + fixes.meeting_counts_updated
        + fixes.people_linked_by_domain
        + fixes.renewals_rolled_over
        + fixes.ai_enrichments_enqueued
        + fixes.phantom_accounts_archived
        + fixes.orphan_internals_relinked
        + fixes.empty_shells_archived
        + fixes.people_auto_merged
        + fixes.names_resolved_calendar
        + fixes.people_linked_co_attendance;
    let total_remaining_gaps: usize = remaining_gaps.iter().map(|g| g.count).sum();

    // Return None when nothing happened
    if total_fix_count == 0 && total_remaining_gaps == 0 {
        return None;
    }

    // Build narrative prose
    let mut narrative = String::new();
    if !fix_parts.is_empty() {
        let capitalized = {
            let joined = join_prose_list(&fix_parts);
            let mut chars = joined.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        };
        narrative.push_str(&format!("{}.", capitalized));
    }
    if total_remaining_gaps > 0 {
        if !narrative.is_empty() {
            narrative.push(' ');
        }
        narrative.push_str(&format!(
            "{} {} remaining.",
            total_remaining_gaps,
            if total_remaining_gaps == 1 {
                "gap"
            } else {
                "gaps"
            }
        ));
    } else if !narrative.is_empty() {
        narrative.push_str(" All clear.");
    }

    Some(HygieneNarrativeView {
        narrative,
        remaining_gaps,
        last_scan_time: Some(report.scanned_at.clone()),
        total_fixes: total_fix_count,
        total_remaining_gaps,
    })
}

pub fn hygiene_gap_action(key: &str) -> HygieneGapActionView {
    match key {
        "unnamed_people" => HygieneGapActionView {
            kind: "navigate".to_string(),
            label: "View People".to_string(),
            route: Some("/people?hygiene=unnamed".to_string()),
        },
        "unknown_relationships" => HygieneGapActionView {
            kind: "navigate".to_string(),
            label: "Review Relationships".to_string(),
            route: Some("/people?relationship=unknown".to_string()),
        },
        "duplicate_people" => HygieneGapActionView {
            kind: "navigate".to_string(),
            label: "Review Duplicates".to_string(),
            route: Some("/people?hygiene=duplicates".to_string()),
        },
        _ => HygieneGapActionView {
            kind: "run_scan_now".to_string(),
            label: "Run Hygiene Scan Now".to_string(),
            route: None,
        },
    }
}

/// Build the hygiene status view model from app state and an optional report.
pub fn build_intelligence_hygiene_status(
    state: &AppState,
    report: Option<&HygieneReport>,
) -> HygieneStatusView {
    let unnamed_people = report.map(|r| r.unnamed_people).unwrap_or(0);
    let unknown_relationships = report.map(|r| r.unknown_relationships).unwrap_or(0);
    let missing_intelligence = report.map(|r| r.missing_intelligence).unwrap_or(0);
    let stale_intelligence = report.map(|r| r.stale_intelligence).unwrap_or(0);
    let unsummarized_files = report.map(|r| r.unsummarized_files).unwrap_or(0);
    let duplicate_people = report.map(|r| r.duplicate_people).unwrap_or(0);

    let fixes = report
        .map(|r| {
            vec![
                HygieneFixView {
                    key: "relationships_reclassified".to_string(),
                    label: "Relationships reclassified".to_string(),
                    count: r.fixes.relationships_reclassified,
                },
                HygieneFixView {
                    key: "summaries_extracted".to_string(),
                    label: "Summaries extracted".to_string(),
                    count: r.fixes.summaries_extracted,
                },
                HygieneFixView {
                    key: "meeting_counts_updated".to_string(),
                    label: "Meeting counts updated".to_string(),
                    count: r.fixes.meeting_counts_updated,
                },
                HygieneFixView {
                    key: "names_resolved".to_string(),
                    label: "Names resolved".to_string(),
                    count: r.fixes.names_resolved,
                },
                HygieneFixView {
                    key: "people_linked_by_domain".to_string(),
                    label: "People linked by domain".to_string(),
                    count: r.fixes.people_linked_by_domain,
                },
                HygieneFixView {
                    key: "renewals_rolled_over".to_string(),
                    label: "Renewals rolled over".to_string(),
                    count: r.fixes.renewals_rolled_over,
                },
                HygieneFixView {
                    key: "ai_enrichments_enqueued".to_string(),
                    label: "AI enrichments enqueued".to_string(),
                    count: r.fixes.ai_enrichments_enqueued,
                },
                HygieneFixView {
                    key: "people_auto_merged".to_string(),
                    label: "Duplicates auto-merged".to_string(),
                    count: r.fixes.people_auto_merged,
                },
                HygieneFixView {
                    key: "names_resolved_calendar".to_string(),
                    label: "Names resolved from calendar".to_string(),
                    count: r.fixes.names_resolved_calendar,
                },
                HygieneFixView {
                    key: "people_linked_co_attendance".to_string(),
                    label: "People linked by co-attendance".to_string(),
                    count: r.fixes.people_linked_co_attendance,
                },
            ]
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|fix| fix.count > 0)
        .collect::<Vec<_>>();

    let mut gaps = Vec::new();
    let gap_rows = vec![
        (
            "unnamed_people",
            "Unnamed people",
            unnamed_people,
            "critical",
            "Missing names make prep less personal.",
        ),
        (
            "unknown_relationships",
            "Unknown relationships",
            unknown_relationships,
            "medium",
            "Unknown relationships reduce meeting classification accuracy.",
        ),
        (
            "duplicate_people",
            "Duplicate people",
            duplicate_people,
            "critical",
            "Duplicate records fragment context and meeting history.",
        ),
        (
            "missing_intelligence",
            "Missing intelligence",
            missing_intelligence,
            "medium",
            "Entities without intelligence produce sparse prep.",
        ),
        (
            "stale_intelligence",
            "Stale intelligence",
            stale_intelligence,
            "low",
            "Older intelligence can miss recent customer signals.",
        ),
        (
            "unsummarized_files",
            "Unsummarized files",
            unsummarized_files,
            "medium",
            "Summaries speed up context retrieval during prep.",
        ),
    ];

    for (key, label, count, impact, description) in gap_rows {
        if count == 0 {
            continue;
        }
        gaps.push(HygieneGapView {
            key: key.to_string(),
            label: label.to_string(),
            count,
            impact: impact.to_string(),
            description: description.to_string(),
            action: hygiene_gap_action(key),
        });
    }

    let total_gaps = unnamed_people
        + unknown_relationships
        + missing_intelligence
        + stale_intelligence
        + unsummarized_files
        + duplicate_people;

    let total_fixes = report
        .map(|r| {
            r.fixes.relationships_reclassified
                + r.fixes.summaries_extracted
                + r.fixes.meeting_counts_updated
                + r.fixes.names_resolved
                + r.fixes.people_linked_by_domain
                + r.fixes.renewals_rolled_over
                + r.fixes.ai_enrichments_enqueued
        })
        .unwrap_or(0);

    let is_running = state
        .hygiene
        .scan_running
        .load(std::sync::atomic::Ordering::Acquire);

    let (status, status_label) = if is_running {
        ("running".to_string(), "Running".to_string())
    } else if total_gaps == 0 {
        ("healthy".to_string(), "Healthy".to_string())
    } else {
        ("needs_attention".to_string(), "Needs Attention".to_string())
    };

    let queued_for_next_budget = report
        .map(|r| {
            (r.missing_intelligence + r.stale_intelligence)
                .saturating_sub(r.fixes.ai_enrichments_enqueued)
        })
        .unwrap_or(0);

    let last_scan_time = state
        .hygiene
        .last_scan_at
        .lock()
        .clone()
        .or_else(|| report.map(|r| r.scanned_at.clone()));
    let next_scan_time = state
        .hygiene
        .next_scan_at
        .lock()
        .clone();

    let fix_details = report.map(|r| r.fix_details.clone()).unwrap_or_default();

    HygieneStatusView {
        status,
        status_label,
        last_scan_time,
        next_scan_time,
        total_gaps,
        total_fixes,
        is_running,
        fixes,
        fix_details,
        gaps,
        // budget view now reflects token budget, not call count.
        budget: {
            let (used_today, daily_limit) = if let Ok(db) = crate::db::ActionDb::open() {
                let budget = crate::pty::read_configured_daily_budget(&db);
                let usage = crate::pty::DailyTokenUsage::load(&db);
                (usage.tokens_used, budget)
            } else {
                (0, crate::pty::DEFAULT_DAILY_AI_TOKEN_BUDGET)
            };
            HygieneBudgetView {
                used_today,
                daily_limit,
                queued_for_next_budget,
            }
        },
        scan_duration_ms: report.map(|r| r.scan_duration_ms),
    }
}

/// Run an expanded overnight scan with higher AI budget.
pub fn run_overnight_scan(
    db: &ActionDb,
    config: &Config,
    workspace: &Path,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> OvernightReport {
    // Call-count budget replaced by token budget enforced at PTY time.
    // Use an unlimited enqueue budget so overnight doesn't self-throttle; the
    // token budget gate handles actual enforcement.
    let _ = OVERNIGHT_AI_BUDGET; // kept to avoid unused-const warning
    let overnight_budget = crate::state::HygieneBudget::unlimited();

    let report = super::run_hygiene_scan(
        db,
        config,
        workspace,
        Some(&overnight_budget),
        Some(queue),
        false,
        None,
    );

    OvernightReport {
        ran_at: Utc::now().to_rfc3339(),
        entities_refreshed: report.fixes.ai_enrichments_enqueued,
        names_resolved: report.fixes.names_resolved,
        summaries_extracted: report.fixes.summaries_extracted,
        relationships_reclassified: report.fixes.relationships_reclassified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::hygiene::tests_common::default_test_config;

    // --- Hygiene Narrative tests  ---

    #[test]
    fn test_join_prose_list_empty() {
        assert_eq!(join_prose_list(&[]), "");
    }

    #[test]
    fn test_join_prose_list_one() {
        assert_eq!(join_prose_list(&["a".to_string()]), "a");
    }

    #[test]
    fn test_join_prose_list_two() {
        assert_eq!(
            join_prose_list(&["a".to_string(), "b".to_string()]),
            "a and b"
        );
    }

    #[test]
    fn test_join_prose_list_three() {
        assert_eq!(
            join_prose_list(&["a".to_string(), "b".to_string(), "c".to_string()]),
            "a, b, and c"
        );
    }

    #[test]
    fn test_build_narrative_empty_report() {
        let report = HygieneReport::default();
        assert!(build_hygiene_narrative(&report).is_none());
    }

    #[test]
    fn test_build_narrative_only_fixes() {
        let mut report = HygieneReport::default();
        report.fixes.names_resolved = 3;
        report.scanned_at = "2026-01-15T10:00:00Z".to_string();
        let view = build_hygiene_narrative(&report).unwrap();
        assert!(view.narrative.contains("Resolved 3 unnamed people"));
        assert!(view.narrative.contains("All clear."));
        assert_eq!(view.total_fixes, 3);
        assert_eq!(view.total_remaining_gaps, 0);
        assert!(view.remaining_gaps.is_empty());
    }

    #[test]
    fn test_build_narrative_only_gaps() {
        let report = HygieneReport {
            unnamed_people: 4,
            scanned_at: "2026-01-15T10:00:00Z".to_string(),
            ..Default::default()
        };
        let view = build_hygiene_narrative(&report).unwrap();
        assert!(view.narrative.contains("4 gaps remaining"));
        assert_eq!(view.total_fixes, 0);
        assert_eq!(view.total_remaining_gaps, 4);
        assert_eq!(view.remaining_gaps.len(), 1);
    }

    #[test]
    fn test_build_narrative_fixes_and_gaps() {
        let mut report = HygieneReport::default();
        report.fixes.relationships_reclassified = 2;
        report.missing_intelligence = 3;
        report.scanned_at = "2026-01-15T10:00:00Z".to_string();
        let view = build_hygiene_narrative(&report).unwrap();
        assert!(view.narrative.contains("Reclassified 2 relationships"));
        assert!(view.narrative.contains("3 gaps remaining"));
        assert_eq!(view.total_fixes, 2);
        assert_eq!(view.total_remaining_gaps, 3);
    }

    #[test]
    fn test_overnight_scan_returns_report_without_writing_filesystem_artifact() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();
        let workspace = tempfile::tempdir().unwrap();

        let config = crate::types::Config {
            workspace_path: workspace.path().to_string_lossy().to_string(),
            user_domain: Some("myco.com".to_string()),
            ..default_test_config()
        };

        let report = run_overnight_scan(&db, &config, workspace.path(), &queue);

        // Report should have a timestamp
        assert!(!report.ran_at.is_empty());
        let maint_path = workspace
            .path()
            .join("_today")
            .join("data")
            .join("maintenance.json");
        assert!(!maint_path.exists());
    }

    #[test]
    fn test_overnight_budget_uses_unlimited_enqueue() {
        // Overnight scan uses unlimited enqueue budget;
        // token enforcement happens at PTY call time.
        let unlimited = crate::state::HygieneBudget::unlimited();
        assert_eq!(unlimited.daily_limit, u32::MAX);
        assert!(unlimited.try_consume()); // Always permits
    }
}
