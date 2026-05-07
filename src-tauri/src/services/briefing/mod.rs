// Daily Briefing per-section composers (W2). Each module produces its slice
// of the BriefingViewModel; the orchestrator (`briefing_view_model::compose`,
// W2b) composes them via `tokio::try_join!`.
//
// Trust source — every composer's module-level doc-comment must declare its
// upstream source, today's state, default behavior on missing data, and the
// ticket that unblocks live data. See `.docs/plans/daily-briefing-redesign-
// waves.md` (W2a merge gate).

pub mod email_signals;
pub mod lead;
pub mod moving;
pub mod predictions;
pub mod schedule;
pub mod watch;
