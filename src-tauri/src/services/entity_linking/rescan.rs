//! Tier 4 — graph-drift re-evaluation for weak primaries.
//!
//! When the entity graph changes (account_domains, account_stakeholders,
//! accounts.name/keywords, entity_keywords, projects), `entity_graph_version`
//! bumps via existing triggers (migrations 113 and 117). This module consumes
//! the bump: any owner whose latest primary was elected via a weak rule
//! (P5 title evidence or P11 fallback) and whose stored `graph_version` is
//! now stale is eligible for re-evaluation.
//!
//! The rescan is intentionally narrow:
//! - LIMIT 50 owners per pass (cheap, avoids long tx in the writer pool).
//! - Only emails are re-evaluated here — meetings are left to the existing
//!   calendar poller / resolver-sweep cadence since rebuilding a full
//!   `CalendarEvent` from persisted state is lossy (e.g., RSVPs, organizer
//!   metadata that the evaluate path reads from the Google payload).
//! - User overrides (`source='user'` / `source='user_dismissed'`) are
//!   never touched — the SELECT filter only matches `rule:P5` / `rule:P11`.

use std::sync::Arc;

use crate::state::AppState;

use super::types::{OwnerType, Trigger};

const PASS_LIMIT: i64 = 50;

/// Re-evaluate owners whose current primary was elected via a weak rule and
/// whose graph_version has since drifted. Returns the number of owners the
/// rescan successfully re-evaluated.
pub async fn rescan_stale_weak_primaries(
    ctx: &crate::services::context::ServiceContext<'_>,
    state: Arc<AppState>,
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let stale = state
        .db_read(|db| {
            let current: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT version FROM entity_graph_version WHERE id = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| format!("read entity_graph_version: {e}"))?;

            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT owner_type, owner_id FROM linked_entities_raw \
                     WHERE role = 'primary' \
                       AND source IN ('rule:P5', 'rule:P11') \
                       AND graph_version < ?1 \
                     ORDER BY graph_version ASC \
                     LIMIT ?2",
                )
                .map_err(|e| format!("prepare rescan: {e}"))?;
            let rows: Vec<(String, String)> = stmt
                .query_map(rusqlite::params![current, PASS_LIMIT], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| format!("rescan query: {e}"))?
                .filter_map(Result::ok)
                .collect();
            Ok::<_, String>(rows)
        })
        .await?;

    if stale.is_empty() {
        return Ok(0);
    }
    log::info!(
        "entity_linking rescan: re-evaluating {} stale weak primaries",
        stale.len()
    );

    let mut attempts = 0usize;
    for (owner_type_str, owner_id) in stale {
        let owner_type = match OwnerType::try_from(owner_type_str.as_str()) {
            Ok(t) => t,
            Err(_) => continue,
        };

        match owner_type {
            OwnerType::Email => match rescan_email(state.clone(), &owner_id).await {
                Ok(()) => attempts += 1,
                Err(e) => log::warn!(
                    "entity_linking rescan: email {owner_id} re-eval failed: {e}"
                ),
            },
            OwnerType::Meeting | OwnerType::EmailThread => {
                // Meetings: the calendar adapter needs a live CalendarEvent
                // payload (RSVPs, organiser metadata) that we don't persist
                // in a round-trippable form. The periodic calendar poller
                // / resolver-sweep path re-evaluates meetings with fresh
                // payloads, so the rescan intentionally skips here.
                //
                // EmailThread: downstream of its member emails; those get
                // re-evaluated here and propagate via thread inheritance.
                continue;
            }
        }
    }

    // Record that a sweep happened so the one-shot upgrade sweep doesn't
    // re-fire. Periodic rescans overwrite the timestamp harmlessly.
    let now = ctx.clock.now().to_rfc3339();
    if let Err(err) = state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "UPDATE entity_graph_version SET last_migration_sweep_at = ?1 WHERE id = 1",
                    rusqlite::params![now],
                )
                .map(|_| ())
                .map_err(|e| format!("update last_migration_sweep_at: {e}"))
        })
        .await
    {
        log::warn!("entity linking rescan marker update failed: {}", err);
    }

    Ok(attempts)
}

async fn rescan_email(state: Arc<AppState>, email_id: &str) -> Result<(), String> {
    let owner_id = email_id.to_string();
    let email_opt = state
        .db_read(move |db| db.get_email_by_id_for_linking(&owner_id))
        .await?;
    let email = match email_opt {
        Some(e) => e,
        None => return Err(format!("email {email_id} not found for rescan")),
    };
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let svc_ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
    super::email_adapter::evaluate_email(&svc_ctx, state, &email, Trigger::EntityGraphChange)
        .await
        .map(|_| ())
}

/// Run the one-time post-upgrade sweep if it hasn't been run yet.
///
/// Gated by `entity_graph_version.last_migration_sweep_at`: if NULL, we run
/// the sweep and set it to now. Subsequent boots skip.
pub async fn run_one_shot_upgrade_sweep(state: Arc<AppState>) -> Result<usize, String> {
    let already_done: bool = state
        .db_read(|db| {
            let val: Option<String> = db
                .conn_ref()
                .query_row(
                    "SELECT last_migration_sweep_at FROM entity_graph_version WHERE id = 1",
                    [],
                    |row| row.get(0),
                )
                .ok()
                .flatten();
            Ok::<_, String>(val.is_some())
        })
        .await
        .unwrap_or(true);

    if already_done {
        return Ok(0);
    }

    log::info!("entity_linking: running one-shot post-upgrade weak-primary sweep");
    let ctx = state.live_service_context();
    rescan_stale_weak_primaries(&ctx, state.clone()).await
}

#[cfg(test)]
mod tests {
    use crate::db::test_utils::test_db;

    /// The rescan SELECT must return only rows where the primary is weak
    /// (rule:P5 / rule:P11) AND the stored graph_version is behind the
    /// current version. User overrides never match because source='user'
    /// is outside the IN (...) list.
    #[test]
    fn rescan_stale_weak_primaries_reevaluates_p5_rows() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('acc-x', 'X', '2026-04-20', 0)",
                [],
            )
            .expect("seed account");
        // Bump graph version to simulate drift.
        db.conn_ref()
            .execute(
                "UPDATE entity_graph_version SET version = 5 WHERE id = 1",
                [],
            )
            .expect("bump version");

        // Weak primary (P5) with stale graph_version=1 < 5 → should match.
        db.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw (owner_type, owner_id, entity_id, entity_type, role, source, rule_id, graph_version, created_at) \
                 VALUES ('email', 'em-weak', 'acc-x', 'account', 'primary', 'rule:P5', 'P5', 1, '2026-04-20')",
                [],
            ).expect("seed weak");

        // Strong primary (P4a) at same stale version → must NOT match.
        db.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw (owner_type, owner_id, entity_id, entity_type, role, source, rule_id, graph_version, created_at) \
                 VALUES ('email', 'em-strong', 'acc-x', 'account', 'primary', 'rule:P4a', 'P4a', 1, '2026-04-20')",
                [],
            ).expect("seed strong");

        // Weak primary but up-to-date → must NOT match.
        db.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw (owner_type, owner_id, entity_id, entity_type, role, source, rule_id, graph_version, created_at) \
                 VALUES ('email', 'em-fresh', 'acc-x', 'account', 'primary', 'rule:P5', 'P5', 5, '2026-04-20')",
                [],
            ).expect("seed fresh");

        let current: i64 = db
            .conn_ref()
            .query_row("SELECT version FROM entity_graph_version WHERE id = 1", [], |r| r.get(0))
            .expect("read version");
        let mut stmt = db.conn_ref().prepare(
            "SELECT owner_id FROM linked_entities_raw \
             WHERE role = 'primary' \
               AND source IN ('rule:P5', 'rule:P11') \
               AND graph_version < ?1",
        ).expect("prepare");
        let stale: Vec<String> = stmt
            .query_map(rusqlite::params![current], |r| r.get::<_, String>(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(stale, vec!["em-weak".to_string()]);
    }

    /// The SELECT must never surface user-override rows (`source='user'`
    /// or `source='user_dismissed'`), because those represent explicit user
    /// choices that a re-evaluation must not stomp on.
    #[test]
    fn rescan_stale_weak_primaries_skips_user_override() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('acc-x', 'X', '2026-04-20', 0)",
                [],
            )
            .expect("seed");
        db.conn_ref()
            .execute(
                "UPDATE entity_graph_version SET version = 5 WHERE id = 1",
                [],
            )
            .expect("bump");
        db.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw (owner_type, owner_id, entity_id, entity_type, role, source, rule_id, graph_version, created_at) \
                 VALUES ('email', 'em-user', 'acc-x', 'account', 'primary', 'user', 'P1', 1, '2026-04-20')",
                [],
            ).expect("seed user override");

        let current: i64 = db
            .conn_ref()
            .query_row("SELECT version FROM entity_graph_version WHERE id = 1", [], |r| r.get(0))
            .expect("version");
        let mut stmt = db.conn_ref().prepare(
            "SELECT owner_id FROM linked_entities_raw \
             WHERE role = 'primary' \
               AND source IN ('rule:P5', 'rule:P11') \
               AND graph_version < ?1",
        ).expect("prepare");
        let stale: Vec<String> = stmt
            .query_map(rusqlite::params![current], |r| r.get::<_, String>(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect();
        assert!(stale.is_empty(), "user override must not be rescanned");
    }

    /// Tier 4 (b) domain hierarchy: a subsidiary domain lookup surfaces both
    /// the subsidiary and the parent as candidates so P9 can present the picker.
    #[test]
    fn domain_hierarchy_subsidiary_lookup_returns_parent() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('parent', 'Parent', '2026-04-20', 0)",
                [],
            ).expect("parent");
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, parent_id, updated_at, archived) VALUES ('sub', 'Sub', 'parent', '2026-04-20', 0)",
                [],
            ).expect("sub");
        db.conn_ref()
            .execute(
                "INSERT INTO account_domains (account_id, domain) VALUES ('sub', 'sub.example')",
                [],
            ).expect("domain");

        let candidates = db
            .lookup_account_candidates_by_domain("sub.example")
            .expect("lookup");
        let ids: Vec<String> = candidates.iter().map(|a| a.id.clone()).collect();
        assert!(ids.contains(&"sub".to_string()));
        assert!(
            ids.contains(&"parent".to_string()),
            "parent account should be surfaced alongside subsidiary"
        );
    }
}
