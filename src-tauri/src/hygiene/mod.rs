//! Proactive intelligence maintenance (I145 -- ADR-0058).
//!
//! The hygiene scanner detects data quality gaps across entity types and data
//! sources, then applies mechanical fixes (free, instant) before enqueuing
//! AI-budgeted enrichment for remaining gaps.
//!
//! Background loop: runs 30s after startup, then every 4 hours.

pub(crate) mod detectors;
mod fixers;
mod loop_runner;
mod matcher;
mod narrative;

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::ActionDb;
use crate::types::Config;

// Re-export the full public API so callers can use `crate::hygiene::*` unchanged.
// Some re-exports are not directly referenced but maintain the public API contract.
#[allow(unused_imports)]
pub use detectors::score_name_similarity;
pub use detectors::{
    check_upcoming_meeting_readiness, detect_duplicate_people, DuplicateCandidate,
};
#[allow(unused_imports)]
pub use matcher::{auto_link_people_by_domain, resolve_names_from_emails};
pub use narrative::{
    build_hygiene_narrative, build_intelligence_hygiene_status, HygieneNarrativeView,
    HygieneStatusView, OvernightReport,
};
#[allow(unused_imports)]
pub use narrative::{
    run_overnight_scan, HygieneBudgetView, HygieneFixView, HygieneGapActionView, HygieneGapSummary,
    HygieneGapView,
};

/// How long to wait after startup before the first scan.
const STARTUP_DELAY_SECS: u64 = 30;

/// Interval between scans (4 hours).
const SCAN_INTERVAL_SECS: u64 = 4 * 60 * 60;

/// Maximum number of fix details to store per report.
const MAX_FIX_DETAILS: usize = 20;

/// Public interval getter for UI/command next-scan calculations.
/// Uses config value if provided, otherwise falls back to constant.
pub fn scan_interval_secs(config: Option<&Config>) -> u64 {
    config
        .map(|c| c.hygiene_scan_interval_hours as u64 * 3600)
        .unwrap_or(SCAN_INTERVAL_SECS)
}

/// A single narrative fix description for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneFixDetail {
    pub fix_type: String,
    pub entity_name: Option<String>,
    pub description: String,
}

/// Report from a hygiene scan: gaps detected + mechanical fixes applied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneReport {
    pub unnamed_people: usize,
    pub unknown_relationships: usize,
    pub missing_intelligence: usize,
    pub stale_intelligence: usize,
    pub unsummarized_files: usize,
    pub duplicate_people: usize,
    pub abandoned_quill_syncs: usize,
    /// Meetings with low-confidence entity matches (I305).
    pub low_confidence_entity_matches: usize,
    /// Empty shell accounts (no meetings, no actions, no people after 30d).
    pub empty_shell_accounts: usize,
    /// Entities with quality_score below 0.45 (I406).
    pub low_quality_entities: usize,
    /// Entities blocked by coherence circuit breaker (I410).
    pub coherence_blocked_entities: usize,
    pub fixes: MechanicalFixes,
    pub fix_details: Vec<HygieneFixDetail>,
    pub scanned_at: String,
    pub scan_duration_ms: u64,
}

/// Counts of mechanical fixes applied during a hygiene scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalFixes {
    pub relationships_reclassified: usize,
    pub summaries_extracted: usize,
    pub meeting_counts_updated: usize,
    pub names_resolved: usize,
    pub people_linked_by_domain: usize,
    pub people_deduped_by_alias: usize,
    pub renewals_rolled_over: usize,
    pub ai_enrichments_enqueued: usize,
    pub quill_syncs_retried: usize,
    /// Entity suggestions created from low-confidence matches (I305).
    pub entity_suggestions_created: usize,
    /// Phantom accounts archived (structural folders bootstrapped as accounts).
    pub phantom_accounts_archived: usize,
    /// Orphan internal accounts re-linked to internal root.
    pub orphan_internals_relinked: usize,
    /// Empty shell accounts auto-archived (no activity after 30d).
    pub empty_shells_archived: usize,
    /// High-confidence duplicate people auto-merged.
    pub people_auto_merged: usize,
    /// Names resolved from calendar display names.
    pub names_resolved_calendar: usize,
    /// People linked to accounts via meeting co-attendance.
    pub people_linked_co_attendance: usize,
}

/// Run a full hygiene scan: detect gaps, apply mechanical fixes, return report.
/// If `budget` is provided, enqueue AI enrichment for remaining gaps.
///
/// Each phase is wrapped in error isolation: a failure in one phase logs
/// the error and continues to the next phase rather than aborting the scan.
pub fn run_hygiene_scan(
    db: &ActionDb,
    config: &Config,
    workspace: &Path,
    budget: Option<&crate::state::HygieneBudget>,
    queue: Option<&crate::intel_queue::IntelligenceQueue>,
    _first_run: bool,
    embedding_model: Option<&crate::embeddings::EmbeddingModel>,
) -> HygieneReport {
    let scan_start = std::time::Instant::now();
    let mut report = HygieneReport {
        scanned_at: Utc::now().to_rfc3339(),
        ..Default::default()
    };

    // --- Gap detection ---
    report.unnamed_people = db.get_unnamed_people().map(|v| v.len()).unwrap_or(0);
    report.unknown_relationships = db
        .get_unknown_relationship_people()
        .map(|v| v.len())
        .unwrap_or(0);
    report.missing_intelligence = db
        .get_entities_without_intelligence()
        .map(|v| v.len())
        .unwrap_or(0);
    report.stale_intelligence = db
        .get_stale_entity_intelligence(14)
        .map(|v| v.len())
        .unwrap_or(0);
    report.unsummarized_files = db
        .get_unsummarized_content_files()
        .map(|v| v.len())
        .unwrap_or(0);
    report.duplicate_people = detectors::detect_duplicate_people(db)
        .map(|v| v.len())
        .unwrap_or(0);
    report.abandoned_quill_syncs = db.count_quill_syncs_by_state("abandoned").unwrap_or(0);
    report.empty_shell_accounts = detectors::count_empty_shell_accounts(db);

    // --- Phase 1: Mechanical fixes (free, instant) ---
    let user_domains = config.resolved_user_domains();
    let mut all_details: Vec<HygieneFixDetail> = Vec::new();

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (count, details) = fixers::fix_unknown_relationships(db, &user_domains);
        report.fixes.relationships_reclassified = count;
        all_details.extend(details);

        let (count, details) = fixers::backfill_file_summaries(db);
        report.fixes.summaries_extracted = count;
        all_details.extend(details);

        let (count, details) = fixers::fix_meeting_counts(db);
        report.fixes.meeting_counts_updated = count;
        all_details.extend(details);

        let (count, details) = fixers::fix_renewal_rollovers(db);
        report.fixes.renewals_rolled_over = count;
        all_details.extend(details);

        let (count, details) = fixers::retry_abandoned_quill_syncs(db);
        report.fixes.quill_syncs_retried = count;
        all_details.extend(details);
    })) {
        Ok(()) => {}
        Err(e) => {
            log::error!("Hygiene phase 1 (mechanical fixes) panicked: {:?}", e);
        }
    }

    // --- Phase 1b: Account cleanup (free, instant) ---
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (count, details) = fixers::archive_phantom_accounts(db);
        report.fixes.phantom_accounts_archived = count;
        all_details.extend(details);

        let (count, details) = fixers::relink_orphan_internal_accounts(db);
        report.fixes.orphan_internals_relinked = count;
        all_details.extend(details);

        let (count, details) = fixers::archive_empty_shell_accounts(db);
        report.fixes.empty_shells_archived = count;
        all_details.extend(details);
    })) {
        Ok(()) => {}
        Err(e) => {
            log::error!("Hygiene phase 1b (account cleanup) panicked: {:?}", e);
        }
    }

    // --- Phase 2: Email name resolution + domain linking (free) ---
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (count, details) = matcher::resolve_names_from_emails(db, workspace);
        report.fixes.names_resolved = count;
        all_details.extend(details);

        let (count, details) = matcher::auto_link_people_by_domain(db);
        report.fixes.people_linked_by_domain = count;
        all_details.extend(details);

        let (count, details) = matcher::dedup_people_by_domain_alias(db, &user_domains);
        report.fixes.people_deduped_by_alias = count;
        all_details.extend(details);
    })) {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Hygiene phase 2 (name resolution + linking) panicked: {:?}",
                e
            );
        }
    }

    // --- Phase 2b: Low-confidence entity match detection (I305) ---
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let accounts_dir = workspace.join("Accounts");
        let (count, details) =
            detectors::detect_low_confidence_matches(db, &accounts_dir, embedding_model);
        report.fixes.entity_suggestions_created = count;
        report.low_confidence_entity_matches = count;
        all_details.extend(details);
    })) {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Hygiene phase 2b (low-confidence matches) panicked: {:?}",
                e
            );
        }
    }

    // --- Phase 2c: Attendee group pattern mining (I307) ---
    match crate::signals::patterns::mine_attendee_patterns(db) {
        Ok(count) if count > 0 => {
            log::info!("Hygiene: mined {} attendee group pattern updates", count);
        }
        Err(e) => {
            log::warn!("Hygiene: attendee pattern mining failed: {}", e);
        }
        _ => {}
    }

    // --- Phase 2d: Self-healing intelligence (I342) ---
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (count, details) = matcher::fix_auto_merge_duplicates(db);
        report.fixes.people_auto_merged = count;
        all_details.extend(details);

        let (count, details) = matcher::resolve_names_from_calendar(db);
        report.fixes.names_resolved_calendar = count;
        all_details.extend(details);

        let (count, details) = matcher::fix_co_attendance_links(db);
        report.fixes.people_linked_co_attendance = count;
        all_details.extend(details);
    })) {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Hygiene phase 2d (self-healing intelligence) panicked: {:?}",
                e
            );
        }
    }

    // --- Phase 2e: Email cadence monitoring (I319) ---
    let cadence_anomalies = crate::signals::cadence::compute_and_emit_cadence_anomalies(db);
    if !cadence_anomalies.is_empty() {
        log::info!(
            "Hygiene: {} email cadence anomalies detected",
            cadence_anomalies.len()
        );
    }

    // --- Phase 3: AI-budgeted gap filling (self-healing portfolio evaluation) ---
    if let (Some(budget), Some(queue)) = (budget, queue) {
        let (risk_gap_count, details) = matcher::enqueue_glean_risk_gap_fills(db, budget, queue);
        all_details.extend(details);
        report.fixes.ai_enrichments_enqueued = risk_gap_count
            + crate::self_healing::evaluate_portfolio(db, budget, queue, embedding_model);
    }

    // Truncate details to max and store on report
    all_details.truncate(MAX_FIX_DETAILS);
    report.fix_details = all_details;

    // --- Re-count gaps after fixes so UI shows remaining problems, not stale pre-fix counts ---
    report.unnamed_people = db.get_unnamed_people().map(|v| v.len()).unwrap_or(0);
    report.unknown_relationships = db
        .get_unknown_relationship_people()
        .map(|v| v.len())
        .unwrap_or(0);
    report.unsummarized_files = db
        .get_unsummarized_content_files()
        .map(|v| v.len())
        .unwrap_or(0);
    report.duplicate_people = detectors::detect_duplicate_people(db)
        .map(|v| v.len())
        .unwrap_or(0);
    report.abandoned_quill_syncs = db.count_quill_syncs_by_state("abandoned").unwrap_or(0);
    report.empty_shell_accounts = detectors::count_empty_shell_accounts(db);
    report.low_quality_entities = crate::self_healing::quality::get_low_quality_count(db);
    report.coherence_blocked_entities =
        crate::self_healing::quality::get_coherence_blocked_count(db);

    report.scan_duration_ms = scan_start.elapsed().as_millis() as u64;
    report
}

pub use loop_runner::run_hygiene_loop;

// =============================================================================
// Shared test utilities
// =============================================================================

#[cfg(test)]
pub(crate) mod tests_common {
    use chrono::Utc;

    use crate::db::ActionDb;
    use crate::types::Config;

    pub fn seed_person(db: &ActionDb, id: &str, email: &str, name: &str, relationship: &str) {
        let now = Utc::now().to_rfc3339();
        let person = crate::db::DbPerson {
            id: id.to_string(),
            email: email.to_string(),
            name: name.to_string(),
            organization: None,
            role: None,
            relationship: relationship.to_string(),
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
        db.upsert_person(&person).expect("upsert person");
    }

    pub fn seed_account(db: &ActionDb, id: &str, name: &str) {
        let now = Utc::now().to_rfc3339();
        let account = crate::db::DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            account_type: crate::db::AccountType::Customer,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        };
        db.upsert_account(&account).expect("upsert account");
    }

    pub fn seed_account_with_renewal(
        db: &ActionDb,
        id: &str,
        name: &str,
        contract_end: &str,
        arr: Option<f64>,
    ) {
        let now = Utc::now().to_rfc3339();
        let account = crate::db::DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            lifecycle: None,
            arr,
            health: None,
            contract_start: None,
            contract_end: Some(contract_end.to_string()),
            nps: None,
            tracker_path: None,
            parent_id: None,
            account_type: crate::db::AccountType::Customer,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        };
        db.upsert_account(&account).expect("upsert account");
    }

    pub fn seed_entity(db: &ActionDb, id: &str, name: &str, entity_type: &str) {
        let now = Utc::now().to_rfc3339();
        db.conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entities (id, name, entity_type, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, name, entity_type, now],
            )
            .unwrap();
    }

    pub fn seed_upcoming_meeting(db: &ActionDb, meeting_id: &str, hours_from_now: i64) {
        let start = Utc::now() + chrono::Duration::hours(hours_from_now);
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, 'Test Meeting', 'customer', ?2, ?2)",
                rusqlite::params![meeting_id, start.to_rfc3339()],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
                rusqlite::params![meeting_id],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES (?1)",
                rusqlite::params![meeting_id],
            )
            .unwrap();
    }

    pub fn link_meeting_entity(db: &ActionDb, meeting_id: &str, entity_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES (?1, ?2, 'account')",
                rusqlite::params![meeting_id, entity_id],
            )
            .unwrap();
    }

    pub fn seed_entity_intelligence(db: &ActionDb, entity_id: &str, enriched_at: &str) {
        db.conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entity_assessment (entity_id, entity_type, enriched_at)
                 VALUES (?1, 'account', ?2)",
                rusqlite::params![entity_id, enriched_at],
            )
            .unwrap();
    }

    pub fn default_test_config() -> Config {
        Config {
            workspace_path: String::new(),
            schedules: crate::types::Schedules::default(),
            profile: "customer-success".to_string(),
            profile_config: None,
            entity_mode: "account".to_string(),
            google: crate::types::GoogleConfig::default(),
            post_meeting_capture: crate::types::PostMeetingCaptureConfig::default(),
            quill: crate::quill::QuillConfig::default(),
            granola: crate::granola::GranolaConfig::default(),
            gravatar: crate::gravatar::GravatarConfig::default(),
            clay: crate::clay::ClayConfig::default(),
            linear: crate::linear::LinearConfig::default(),
            drive: crate::types::DriveConfig::default(),
            features: std::collections::HashMap::new(),
            user_domain: None,
            user_domains: None,
            user_name: None,
            user_company: None,
            user_title: None,
            user_focus: None,
            internal_team_setup_completed: false,
            internal_team_setup_version: 0,
            internal_org_account_id: None,
            developer_mode: false,
            personality: "professional".to_string(),
            ai_models: crate::types::AiModelConfig::default(),
            ai_model_routing_version: crate::types::AI_MODEL_ROUTING_VERSION,
            embeddings: crate::types::EmbeddingConfig::default(),
            role: "customer-success".to_string(),
            custom_preset_path: None,
            app_lock_timeout_minutes: Some(15),
            icloud_warning_dismissed: None,
            hygiene_scan_interval_hours: 4,
            hygiene_ai_budget: 10,
            hygiene_pre_meeting_hours: 12,
            email_enrichment_timeout_seconds: 90,
            notifications: crate::types::NotificationConfig::default(),
            text_scale_percent: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tests_common::default_test_config;

    #[test]
    fn test_scan_interval_secs_default() {
        assert_eq!(scan_interval_secs(None), SCAN_INTERVAL_SECS);
    }

    #[test]
    fn test_scan_interval_secs_from_config() {
        let config = default_test_config();
        let result = scan_interval_secs(Some(&config));
        assert_eq!(result, config.hygiene_scan_interval_hours as u64 * 3600);
    }
}
