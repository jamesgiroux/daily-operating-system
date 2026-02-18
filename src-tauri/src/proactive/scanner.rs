//! Background proactive scanner (I260).
//!
//! Piggybacks on the hygiene scanner's timing — runs after each hygiene scan
//! completes, and also before `prepare_today()` runs.

use crate::state::AppState;

use super::engine::{self, DetectorContext};

/// Run a proactive scan using the default engine.
///
/// Called from hygiene loop and pre-briefing hook. Acquires DB lock internally.
pub fn run_proactive_scan(state: &AppState) -> Result<usize, String> {
    let db_guard = state.db.lock().map_err(|e| format!("DB lock: {}", e))?;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (profile, user_domains) = {
        let config_guard = state.config.read().ok();
        let config = config_guard.as_ref().and_then(|g| g.as_ref());
        let profile = config
            .map(|c| c.profile.clone())
            .unwrap_or_else(|| "general".to_string());
        let domains = config.map(|c| c.resolved_user_domains()).unwrap_or_default();
        (profile, domains)
    };

    let today = chrono::Local::now().date_naive();
    let ctx = DetectorContext {
        today,
        user_domains,
        profile,
    };

    let engine = engine::default_engine();
    engine.run_scan(db, &ctx)
}
