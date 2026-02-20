//! Proactive detection engine (I260).
//!
//! Manages a registry of pattern detectors, runs them against the database,
//! deduplicates insights by fingerprint, and emits signals into the signal bus.

use chrono::NaiveDate;
use rusqlite::params;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::ActionDb;
use crate::signals::bus;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A raw insight detected by a pattern detector.
#[derive(Debug, Clone)]
pub struct RawInsight {
    pub detector_name: String,
    pub fingerprint: String,
    pub entity_type: String,
    pub entity_id: String,
    pub signal_type: String,
    pub headline: String,
    pub detail: String,
    pub confidence: f64,
    pub context_json: Option<String>,
}

/// Profile tag indicating a detector runs for all profiles.
pub const PROFILE_ALL: &str = "all";

/// Context passed to each detector.
pub struct DetectorContext {
    pub today: NaiveDate,
    pub user_domains: Vec<String>,
    pub profile: String,
}

/// Function signature for a pattern detector.
pub type DetectorFn = fn(&ActionDb, &DetectorContext) -> Vec<RawInsight>;

/// A registered pattern detector with profile tags.
pub struct DetectorEntry {
    pub name: String,
    pub profiles: Vec<String>,
    pub detector: DetectorFn,
}

/// The proactive detection engine.
#[derive(Default)]
pub struct ProactiveEngine {
    detectors: Vec<DetectorEntry>,
}

impl ProactiveEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a detector with the profiles it applies to.
    pub fn register(&mut self, name: &str, profiles: &[&str], detector: DetectorFn) {
        self.detectors.push(DetectorEntry {
            name: name.to_string(),
            profiles: profiles.iter().map(|s| s.to_string()).collect(),
            detector,
        });
    }

    /// Run detectors matching the context profile, dedup, emit signals.
    /// Returns the count of new insights emitted.
    pub fn run_scan(&self, db: &ActionDb, ctx: &DetectorContext) -> Result<usize, String> {
        let mut total_new = 0usize;

        for entry in &self.detectors {
            // Filter by profile
            if !entry.profiles.iter().any(|p| p == PROFILE_ALL || p == &ctx.profile) {
                continue;
            }

            let insights = (entry.detector)(db, ctx);
            let insight_count = insights.len();

            for insight in insights {
                // Check dedup: skip if fingerprint was emitted within 7 days
                if is_recently_emitted(db, &insight.fingerprint) {
                    continue;
                }

                // Emit signal via the bus
                let signal_id = bus::emit_signal(
                    db,
                    &insight.entity_type,
                    &insight.entity_id,
                    &insight.signal_type,
                    "proactive",
                    insight.context_json.as_deref(),
                    insight.confidence,
                )
                .map_err(|e| format!("Failed to emit proactive signal: {}", e))?;

                // Insert into proactive_insights
                let insight_id = format!("pi-{}", Uuid::new_v4());
                db.conn_ref()
                    .execute(
                        "INSERT OR IGNORE INTO proactive_insights
                            (id, detector_name, fingerprint, signal_id, entity_type, entity_id, headline, detail, expires_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now', '+7 days'))",
                        params![
                            insight_id,
                            insight.detector_name,
                            insight.fingerprint,
                            signal_id,
                            insight.entity_type,
                            insight.entity_id,
                            insight.headline,
                            insight.detail,
                        ],
                    )
                    .map_err(|e| format!("Failed to insert proactive insight: {}", e))?;

                total_new += 1;
            }

            // Update scan state
            db.conn_ref()
                .execute(
                    "INSERT INTO proactive_scan_state (detector_name, last_run_at, last_insight_count)
                     VALUES (?1, datetime('now'), ?2)
                     ON CONFLICT(detector_name) DO UPDATE SET
                         last_run_at = datetime('now'),
                         last_insight_count = excluded.last_insight_count",
                    params![entry.name, insight_count as i32],
                )
                .map_err(|e| format!("Failed to update scan state: {}", e))?;
        }

        Ok(total_new)
    }
}

/// Build a default engine with all 8 detectors registered.
pub fn default_engine() -> ProactiveEngine {
    use super::detectors;

    let mut engine = ProactiveEngine::new();

    engine.register(
        "detect_renewal_gap",
        &["cs", "executive"],
        detectors::detect_renewal_gap,
    );
    engine.register(
        "detect_relationship_drift",
        &[PROFILE_ALL],
        detectors::detect_relationship_drift,
    );
    engine.register(
        "detect_email_volume_spike",
        &[PROFILE_ALL],
        detectors::detect_email_volume_spike,
    );
    engine.register(
        "detect_meeting_load_forecast",
        &[PROFILE_ALL],
        detectors::detect_meeting_load_forecast,
    );
    engine.register(
        "detect_stale_champion",
        &["cs", "executive"],
        detectors::detect_stale_champion,
    );
    engine.register(
        "detect_action_cluster",
        &[PROFILE_ALL],
        detectors::detect_action_cluster,
    );
    engine.register(
        "detect_prep_coverage_gap",
        &[PROFILE_ALL],
        detectors::detect_prep_coverage_gap,
    );
    engine.register(
        "detect_no_contact_accounts",
        &[PROFILE_ALL],
        detectors::detect_no_contact_accounts,
    );
    engine.register(
        "detect_renewal_proximity",
        &["cs", "sales", "partnerships", "executive"],
        detectors::detect_renewal_proximity,
    );

    engine
}

/// Check if this fingerprint was emitted in the last 7 days.
fn is_recently_emitted(db: &ActionDb, fingerprint: &str) -> bool {
    db.conn_ref()
        .query_row(
            "SELECT 1 FROM proactive_insights
             WHERE fingerprint = ?1
               AND created_at >= datetime('now', '-7 days')",
            params![fingerprint],
            |_| Ok(()),
        )
        .is_ok()
}

/// Compute a dedup fingerprint from key components.
pub fn fingerprint(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"|");
    }
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_fingerprint_deterministic() {
        let fp1 = fingerprint(&["account", "a1", "renewal_gap"]);
        let fp2 = fingerprint(&["account", "a1", "renewal_gap"]);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_differs() {
        let fp1 = fingerprint(&["account", "a1", "renewal_gap"]);
        let fp2 = fingerprint(&["account", "a2", "renewal_gap"]);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_empty_engine_runs_clean() {
        let db = test_db();
        let engine = ProactiveEngine::new();
        let ctx = DetectorContext {
            today: NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            user_domains: vec!["example.com".to_string()],
            profile: "general".to_string(),
        };
        let count = engine.run_scan(&db, &ctx).expect("scan");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_dedup_prevents_re_emission() {
        let db = test_db();

        // Manually insert a recent insight with a known fingerprint
        let fp = fingerprint(&["account", "a1", "test_dedup"]);
        db.conn_ref()
            .execute(
                "INSERT INTO proactive_insights (id, detector_name, fingerprint, entity_type, entity_id, headline)
                 VALUES ('pi-test', 'test', ?1, 'account', 'a1', 'Test')",
                params![fp],
            )
            .unwrap();

        assert!(is_recently_emitted(&db, &fp));
    }

    #[test]
    fn test_run_scan_with_mock_detector() {
        let db = test_db();

        // Create an account for the detector to find
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'TestCo', '2026-01-01')",
                [],
            )
            .unwrap();

        fn mock_detector(_db: &ActionDb, _ctx: &DetectorContext) -> Vec<RawInsight> {
            vec![RawInsight {
                detector_name: "mock".to_string(),
                fingerprint: fingerprint(&["mock", "a1"]),
                entity_type: "account".to_string(),
                entity_id: "a1".to_string(),
                signal_type: "proactive_test".to_string(),
                headline: "Test insight".to_string(),
                detail: "Detail".to_string(),
                confidence: 0.80,
                context_json: Some(r#"{"test": true}"#.to_string()),
            }]
        }

        let mut engine = ProactiveEngine::new();
        engine.register("mock", &[PROFILE_ALL], mock_detector);

        let ctx = DetectorContext {
            today: NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            user_domains: vec![],
            profile: "general".to_string(),
        };

        let count = engine.run_scan(&db, &ctx).expect("scan");
        assert_eq!(count, 1);

        // Second run should dedup
        let count2 = engine.run_scan(&db, &ctx).expect("scan2");
        assert_eq!(count2, 0, "should be deduplicated on second run");
    }

    #[test]
    fn test_profile_filtering() {
        let db = test_db();

        fn cs_only_detector(_db: &ActionDb, _ctx: &DetectorContext) -> Vec<RawInsight> {
            vec![RawInsight {
                detector_name: "cs_only".to_string(),
                fingerprint: fingerprint(&["cs_only", "a1"]),
                entity_type: "account".to_string(),
                entity_id: "a1".to_string(),
                signal_type: "proactive_test".to_string(),
                headline: "CS insight".to_string(),
                detail: "Detail".to_string(),
                confidence: 0.80,
                context_json: None,
            }]
        }

        let mut engine = ProactiveEngine::new();
        engine.register("cs_only", &["cs", "executive"], cs_only_detector);

        // Profile "general" should not run the cs_only detector
        let ctx = DetectorContext {
            today: NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            user_domains: vec![],
            profile: "general".to_string(),
        };
        let count = engine.run_scan(&db, &ctx).expect("scan");
        assert_eq!(count, 0, "general profile should skip cs-only detector");

        // Profile "cs" should run it
        let ctx_cs = DetectorContext {
            today: NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            user_domains: vec![],
            profile: "cs".to_string(),
        };
        let count_cs = engine.run_scan(&db, &ctx_cs).expect("scan_cs");
        assert_eq!(count_cs, 1, "cs profile should run cs-only detector");
    }
}
