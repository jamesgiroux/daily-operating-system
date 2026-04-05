//! Background proactive scanner (I260).
//!
//! Piggybacks on the hygiene scanner's timing — runs after each hygiene scan
//! completes, and also before `prepare_today()` runs.

use crate::db::ActionDb;
use crate::state::AppState;

use super::engine::{self, DetectorContext};

/// Run a proactive scan using the default engine.
///
/// Called from hygiene loop and pre-briefing hook.
pub fn run_proactive_scan(state: &AppState) -> Result<usize, String> {
    let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

    let (profile, user_domains) = {
        let config_guard = state.config.read();
        let config = config_guard.as_ref();
        let profile = config
            .map(|c| c.profile.clone())
            .unwrap_or_else(|| "general".to_string());
        let domains = config
            .map(|c| c.resolved_user_domains())
            .unwrap_or_default();
        (profile, domains)
    };

    let today = chrono::Local::now().date_naive();
    let ctx = DetectorContext {
        today,
        user_domains,
        profile,
    };

    let engine = engine::default_engine();
    engine.run_scan(&db, &ctx)
}
