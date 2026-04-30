//! Intelligence quality feedback service (I529).
//!
//! Records user feedback on intelligence fields and adjusts source weights
//! via the Bayesian signal weight system.
//!
//! Source attribution strategy (coarse, per I529 spec):
//! 1. People: read `enrichment_sources[field]["source"]` for precise field-level attribution
//! 2. All entities: fall back to most recent enrichment signal source for the entity
//! 3. If no source identifiable: record feedback with `source = null` (signal still emitted)

use crate::db::feedback::{CorrectionAction, FeedbackEventInput};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;

/// Submit feedback on an intelligence field for an entity.
///
/// Records the feedback, adjusts source weights (Bayesian alpha/beta),
/// and emits a signal for downstream propagation.
///
/// DOS-209 (W2-A): takes `&ServiceContext` as first parameter and gates
/// on `ctx.check_mutation_allowed()?` per ADR-0104. Errors out of the
/// `WriteBlockedByMode` boundary in non-Live modes.
pub fn submit_intelligence_feedback(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    field: &str,
    feedback_type: &str,
    context: Option<&str>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = match feedback_type {
        "positive" => CorrectionAction::Confirmed,
        "negative" => CorrectionAction::Rejected,
        other => return Err(format!("invalid feedback_type '{other}' (expected positive|negative)")),
    };

    submit_intelligence_correction(
        ctx,
        db,
        SubmitIntelligenceCorrectionInput {
            entity_id,
            entity_type,
            field,
            action,
            corrected_value: None,
            annotation: context,
            item_key: None,
        },
    )
}

/// Resolve the source that produced an intelligence field.
///
/// Strategy:
/// 1. For people: check `enrichment_sources[field]["source"]` (precise attribution)
/// 2. For all entities: check the most recent enrichment-class signal for the entity
///    (covers Glean-sourced intelligence for accounts/projects)
/// 3. Returns None if no source identifiable
fn resolve_intelligence_source(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    field: &str,
) -> Option<String> {
    // 1. Precise: person profile field attribution
    if entity_type == "person" {
        let precise = db
            .get_person(entity_id)
            .ok()
            .flatten()
            .and_then(|person| person.enrichment_sources)
            .and_then(|sources_json| serde_json::from_str::<serde_json::Value>(&sources_json).ok())
            .and_then(|sources| {
                sources[field]
                    .get("source")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            });
        if precise.is_some() {
            return precise;
        }
    }

    // 2. Coarse: most recent enrichment-class signal for this entity.
    // Covers Glean-sourced items on accounts/projects and chapter-level fields on people.
    // Enrichment signals have source in: glean, intel_queue, clay, gravatar, ai
    let coarse = db.conn_ref()
        .query_row(
            "SELECT source FROM signal_events \
             WHERE entity_id = ?1 AND entity_type = ?2 \
             AND signal_type IN ('entity_enriched', 'intelligence_refreshed', 'glean_document', 'enrichment_complete') \
             AND superseded_by IS NULL \
             ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![entity_id, entity_type],
            |row| row.get::<_, String>(0),
        )
        .ok();

    coarse
}

/// DOS-41: Fields whose correction should trigger a background health recalc.
///
/// These mirror the fields `services::accounts::update_account_field` treats
/// as provenance-worthy plus the explicit health scoring surface. When a user
/// corrects any of these on an account, the account's health score is
/// recomputed so the UI reflects the correction immediately without waiting
/// for the next enrichment pass.
fn is_health_affecting_field(field: &str) -> bool {
    matches!(
        field,
        "arr"
            | "lifecycle"
            | "contract_end"
            | "renewal_date"
            | "nps"
            | "health"
            | "health_score"
            | "health_assessment"
            | "risk_level"
    )
}

/// DOS-227 (Codex finding 3): Fields that are stored as top-level account
/// columns (not intelligence-blob fields). A correction on any of these
/// must update the authoritative column so `recompute_entity_health` reads
/// the corrected value; otherwise the recompute runs against the old
/// account row and the new "health score" is identical to the pre-correction
/// score. The feedback event alone is not enough — health scoring reads
/// `DbAccount`, not `entity_feedback_events`.
///
/// `renewal_date` is an alias for `contract_end` on the correction UX side;
/// it is also stored as the `contract_end` column.
fn account_column_for_field(field: &str) -> Option<&'static str> {
    match field {
        "arr" => Some("arr"),
        "lifecycle" => Some("lifecycle"),
        "contract_end" => Some("contract_end"),
        "renewal_date" => Some("contract_end"),
        "nps" => Some("nps"),
        "health" => Some("health"),
        "name" => Some("name"),
        _ => None,
    }
}

/// DOS-41 Codex follow-up: the DB layer writes numeric account columns via
/// `CAST(?1 AS REAL)` / `CAST(?1 AS INTEGER)`. SQLite's CAST silently
/// converts unparseable strings to `0` — which would overwrite real ARR /
/// NPS commercial data with zero whenever a correction payload is
/// malformed. Validate parse-ability in Rust before handing the value to
/// the DB so malformed corrections are rejected with a clear error
/// instead of corrupting state.
///
/// `arr` is written via `CAST(?1 AS REAL)` — validate as f64.
/// `nps` is written via `CAST(?1 AS INTEGER)` — validate as i64.
/// `lifecycle`, `contract_end`, `name`, `health` are TEXT-bound and pass
///  through (`health` is stored as the raw string value — see
///  `update_account_field` in db/accounts.rs).
fn validate_numeric_corrected_value(column: &str, value: &str) -> Result<(), String> {
    match column {
        "arr" => value.trim().parse::<f64>().map(|_| ()).map_err(|_| {
            format!("corrected_value for accounts.arr must be numeric, got '{value}'")
        }),
        "nps" => value.trim().parse::<i64>().map(|_| ()).map_err(|_| {
            format!("corrected_value for accounts.nps must be an integer, got '{value}'")
        }),
        _ => Ok(()),
    }
}

pub struct SubmitIntelligenceCorrectionInput<'a> {
    pub entity_id: &'a str,
    pub entity_type: &'a str,
    pub field: &'a str,
    pub action: CorrectionAction,
    pub corrected_value: Option<&'a str>,
    pub annotation: Option<&'a str>,
    pub item_key: Option<&'a str>,
}

/// DOS-41: Submit a consolidated intelligence correction.
///
/// Persists the correction into `entity_feedback_events` with one of five
/// actions (`confirmed` | `rejected` | `annotated` | `corrected` | `dismissed`), then runs the
/// downstream side effects appropriate to the action:
///
/// - `confirmed` → rewards the attributed source (Bayesian alpha++) and emits
///   an `intelligence_confirmed` signal.
/// - `rejected` → penalizes the source (Bayesian beta++) and emits
///   `intelligence_rejected`, but does not suppress the content.
/// - `annotated` → stores the user-authored note in `reason`; next
///   intelligence prompt will thread this as user context. Emits
///   `intelligence_annotated`.
/// - `corrected` → captures `previous_value` + `corrected_value`, penalizes
///   the source (Bayesian beta++), emits `intelligence_corrected`, and — if
///   the field is health-affecting on an account — triggers a background
///   health recalc so the UI reflects the correction immediately.
/// - `dismissed` → records that the claim is wrong, penalizes the source,
///   creates a suppression tombstone keyed by `field` + `item_key`, and emits
///   `intelligence_dismissed`.
///
/// All correction paths write through `record_feedback_event`, keeping
/// `entity_feedback_events` as the single source of truth for correction
/// history.
pub fn submit_intelligence_correction(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: SubmitIntelligenceCorrectionInput<'_>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let SubmitIntelligenceCorrectionInput {
        entity_id,
        entity_type,
        field,
        action,
        corrected_value,
        annotation,
        item_key,
    } = input;
    // Authoritative backend validation. The Tauri IPC boundary is
    // reachable by any caller, not just the useIntelligenceCorrection
    // hook — enforce action-specific payload shape here so invalid
    // submissions cannot write rows, penalize Bayesian weights, or
    // emit correction signals through a direct IPC call.
    if entity_id.trim().is_empty() {
        return Err("entity_id is required".to_string());
    }
    if entity_type.trim().is_empty() {
        return Err("entity_type is required".to_string());
    }
    if field.trim().is_empty() {
        return Err("field is required".to_string());
    }
    match action {
        CorrectionAction::Corrected => {
            let ok = corrected_value
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            if !ok {
                return Err(
                    "corrected action requires a non-empty corrected_value".to_string(),
                );
            }
        }
        CorrectionAction::Annotated => {
            let ok = annotation.map(|v| !v.trim().is_empty()).unwrap_or(false);
            if !ok {
                return Err(
                    "annotated action requires a non-empty annotation".to_string(),
                );
            }
        }
        CorrectionAction::Confirmed | CorrectionAction::Rejected | CorrectionAction::Dismissed => {
            // Confirmed carries neither corrected_value nor annotation.
        }
    }

    // Resolve the source that produced this intelligence so we can attribute
    // Bayesian weight adjustments + tag the feedback row.
    let prior_source = resolve_intelligence_source(db, entity_id, entity_type, field);

    // Capture the previous value only when we are correcting. We read it from
    // the most recent feedback event for this field (falls back to None), so
    // successive corrections chain: v1 → v2 captures v1, v2 → v3 captures v2.
    let previous_value = if action == CorrectionAction::Corrected {
        latest_corrected_value(db, entity_id, field).or_else(|| {
            // No prior correction — on accounts, fall back to the stored
            // column value if the field is a known account column. This is
            // best-effort; None is acceptable when the field lives inside
            // an intelligence JSON blob.
            read_account_field_snapshot(db, entity_id, entity_type, field)
        })
    } else {
        None
    };

    // Write the feedback event. `reason` carries the user's annotation for
    // annotated + corrected actions so it can be threaded into the next
    // intelligence prompt and displayed in correction history.
    db.record_feedback_event(&FeedbackEventInput {
        entity_id,
        entity_type,
        field_key: field,
        item_key,
        feedback_type: action.as_str(),
        source_system: None, // source_system — intelligence correction UX is app-driven
        source_kind: prior_source.as_deref(),
        previous_value: previous_value.as_deref(),
        corrected_value,
        reason: annotation,
    })
    .map_err(|e| format!("record_feedback_event: {e}"))?;

    // Bayesian source-weight update. `confirmed` rewards, `rejected` /
    // `corrected` / `dismissed` penalize, `annotated` is neutral (user added context but didn't
    // reject the AI output).
    if let Some(ref source) = prior_source {
        let field_category = field_to_signal_category(field);
        match action {
            CorrectionAction::Confirmed => {
                let _ = db.upsert_signal_weight(source, entity_type, &field_category, 1.0, 0.0);
            }
            CorrectionAction::Rejected | CorrectionAction::Corrected | CorrectionAction::Dismissed => {
                let _ = db.upsert_signal_weight(source, entity_type, &field_category, 0.0, 1.0);
            }
            CorrectionAction::Annotated => {}
        }
    }

    // Emit a signal for downstream propagation (intel prompts, suppression,
    // health context, etc.). Keep confidence consistent with the existing
    // feedback path (0.8).
    let signal_type = match action {
        CorrectionAction::Confirmed => "intelligence_confirmed",
        CorrectionAction::Rejected => "intelligence_rejected",
        CorrectionAction::Annotated => "intelligence_annotated",
        CorrectionAction::Corrected => "intelligence_corrected",
        CorrectionAction::Dismissed => "intelligence_dismissed",
    };
    let value_json = serde_json::json!({
        "field": field,
        "action": action.as_str(),
        "source": prior_source,
        "previous_value": previous_value,
        "corrected_value": corrected_value,
        "annotation": annotation,
        "item_key": item_key,
    })
    .to_string();
    let _ = crate::services::signals::emit(
        db,
        entity_type,
        entity_id,
        signal_type,
        "user_feedback",
        Some(&value_json),
        0.8,
    );

    // Self-healing tie-in: a correction is a strong negative signal for the
    // attributed enrichment source on Clay-enrichable account fields. Reuses
    // the same pattern as `services::accounts::update_account_field`.
    if action == CorrectionAction::Corrected
        && entity_type == "account"
        && matches!(field, "lifecycle" | "arr" | "health" | "nps")
    {
        if let Some(ref source) = prior_source {
            crate::self_healing::feedback::record_enrichment_correction(
                db, entity_id, "account", source,
            );
        }
    }

    // DOS-227 (Codex finding 3): write the corrected value to the
    // authoritative account column BEFORE recompute_entity_health runs.
    // recompute_entity_health reads `DbAccount` as-is; without this
    // column update the "new" health score is computed from the OLD
    // ARR / lifecycle / contract_end / nps / health value. The feedback
    // event alone doesn't flow into scoring — it's provenance, not state.
    //
    // Intelligence-blob fields (state_of_play, watch_list, risks, plan,
    // health_score, health_assessment, risk_level, …) are NOT account
    // columns; those are handled by the existing recompute path which
    // re-derives them from signals. We only patch the column for fields
    // that have a dedicated column on `accounts`.
    if action == CorrectionAction::Corrected && entity_type == "account" {
        if let (Some(column), Some(value)) =
            (account_column_for_field(field), corrected_value)
        {
            // Normalize lifecycle for parity with services::accounts::update_account_field.
            // Mirrors the whitelist + normalization used by the direct-edit
            // path; the semantic correctness of a user "correction" should
            // match a user "edit" on the same field.
            let normalized = if column == "lifecycle" {
                crate::services::accounts::normalized_lifecycle(value)
            } else {
                value.to_string()
            };

            // DOS-41 Codex follow-up: `db.update_account_field` writes
            // numeric columns via `CAST(?1 AS REAL)` / `CAST(?1 AS INTEGER)`
            // — SQLite silently coerces non-numeric strings to 0, so a
            // malformed "not-a-number" correction on ARR or NPS would wipe
            // real commercial data to 0 and then recompute health from the
            // corrupted value. Validate numerics in Rust BEFORE the DB
            // call. The IPC-boundary empty-string rejection above is not
            // sufficient — "abc" passes that check.
            validate_numeric_corrected_value(column, &normalized)?;

            db.update_account_field(entity_id, column, &normalized)
                .map_err(|e| {
                    format!(
                        "Failed to apply corrected value to accounts.{column} for {entity_id}: {e}"
                    )
                })?;
            log::info!(
                "DOS-227: applied correction to accounts.{column} for {entity_id} (feedback field = {field})"
            );
        }
    }

    // Health recalc — only on a corrected action against a health-affecting
    // field on an account. Route through the full pipeline so
    // entity_assessment.health_json + entity_quality are updated and signals
    // propagate; otherwise the next detail read would still see stale health.
    //
    // DOS-227: because the block above updated the authoritative column
    // first, this recompute now reads the corrected value and the
    // resulting health score reflects the user's correction.
    if action == CorrectionAction::Corrected
        && entity_type == "account"
        && is_health_affecting_field(field)
    {
        if let Err(e) =
            crate::services::intelligence::recompute_entity_health(ctx, db, entity_id, "account")
        {
            log::warn!(
                "recompute_entity_health failed after correction of {field} on {entity_id}: {e}"
            );
        }
    }

    if action == CorrectionAction::Dismissed {
        db.create_suppression_tombstone(
            entity_id,
            field,
            item_key,
            None,
            prior_source.as_deref(),
            None,
        )
        .map_err(|e| format!("create_suppression_tombstone: {e}"))?;
    }

    Ok(())
}

/// Best-effort: read the latest `corrected_value` recorded for a field so
/// successive corrections can chain their previous_value.
fn latest_corrected_value(db: &ActionDb, entity_id: &str, field: &str) -> Option<String> {
    db.conn_ref()
        .query_row(
            "SELECT corrected_value FROM entity_feedback_events \
             WHERE entity_id = ?1 AND field_key = ?2 \
             AND feedback_type = 'corrected' AND corrected_value IS NOT NULL \
             ORDER BY created_at DESC, id DESC LIMIT 1",
            rusqlite::params![entity_id, field],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
}

/// Best-effort snapshot of an account column value for previous_value capture.
/// Returns None for fields not stored as top-level columns (intelligence JSON
/// fields, etc.) — that's acceptable; the feedback row simply records None.
fn read_account_field_snapshot(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    field: &str,
) -> Option<String> {
    if entity_type != "account" {
        return None;
    }
    let account = db.get_account(entity_id).ok().flatten()?;
    match field {
        "name" => Some(account.name),
        "lifecycle" => account.lifecycle,
        "arr" => account.arr.map(|v| v.to_string()),
        "health" => account.health,
        "nps" => account.nps.map(|v| v.to_string()),
        "contract_end" => account.contract_end,
        "renewal_date" => account.contract_end.clone(),
        _ => None,
    }
}

/// Map an intelligence field name to a signal weight category.
fn field_to_signal_category(field: &str) -> String {
    match field {
        // Person profile fields
        "role" | "organization" | "linkedin_url" | "name" | "title" => {
            "profile_enrichment".to_string()
        }
        // Health scoring
        "health_score" | "health_assessment" | "risk_level" => "health_scoring".to_string(),
        // Relationship intelligence
        "relationship_strength" | "sentiment" | "engagement" => "relationship_intel".to_string(),
        // Chapter-level intelligence sections
        "state_of_play" | "watch_list" | "risks" | "plan" => "intelligence_assessment".to_string(),
        // Fallback
        _ => format!("intelligence_{field}"),
    }
}

#[cfg(test)]
mod correction_tests {
    //! DOS-41: Unit tests for `submit_intelligence_correction`.
    //!
    //! Each correction action goes through a distinct downstream path; the
    //! tests here exercise the contract.

    use super::*;
    use crate::db::{DbAccount, test_utils::test_db};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    /// DOS-209 test scaffold: returns a `Live` `ServiceContext` with
    /// deterministic clock + RNG so test mutators pass `check_mutation_allowed`
    /// AND get reproducible time/random values.
    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_account(db: &ActionDb, id: &str) {
        let account = DbAccount {
            id: id.to_string(),
            name: format!("Test {}", id),
            lifecycle: Some("renewing".to_string()),
            arr: Some(100_000.0),
            health: Some("green".to_string()),
            contract_end: Some("2026-12-31".to_string()),
            nps: Some(50),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
            ..Default::default()
        };
        db.upsert_account(&account).expect("seed account");
    }

    #[allow(clippy::too_many_arguments)]
    fn submit_intelligence_correction(
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        field: &str,
        action: CorrectionAction,
        corrected_value: Option<&str>,
        annotation: Option<&str>,
        item_key: Option<&str>,
    ) -> Result<(), String> {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        super::submit_intelligence_correction(
            &ctx,
            db,
            SubmitIntelligenceCorrectionInput {
                entity_id,
                entity_type,
                field,
                action,
                corrected_value,
                annotation,
                item_key,
            },
        )
    }

    fn submit_intelligence_feedback(
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        field: &str,
        feedback_type: &str,
        context: Option<&str>,
    ) -> Result<(), String> {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        super::submit_intelligence_feedback(&ctx, db, entity_id, entity_type, field, feedback_type, context)
    }

    /// Read all feedback rows for an entity; newest first.
    fn feedback_rows(db: &ActionDb, entity_id: &str) -> Vec<(String, Option<String>, Option<String>, Option<String>)> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT feedback_type, previous_value, corrected_value, reason \
                 FROM entity_feedback_events WHERE entity_id = ?1 \
                 ORDER BY created_at DESC, id DESC",
            )
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap();
        rows.map(|r| r.unwrap()).collect()
    }

    #[test]
    fn confirmed_persists_action_and_emits_signal() {
        let db = test_db();
        seed_account(&db, "acct-1");

        submit_intelligence_correction(
            &db,
            "acct-1",
            "account",
            "state_of_play",
            CorrectionAction::Confirmed,
            None,
            None,
            None,
        )
        .expect("confirmed submission");

        let rows = feedback_rows(&db, "acct-1");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "confirmed");
        assert!(rows[0].1.is_none(), "no previous_value on confirmed");
        assert!(rows[0].2.is_none(), "no corrected_value on confirmed");

        // Signal must have been emitted.
        let sig_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events \
                 WHERE entity_id = 'acct-1' AND signal_type = 'intelligence_confirmed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sig_count, 1);
    }

    #[test]
    fn legacy_negative_feedback_flows_through_rejected_event() {
        let db = test_db();
        seed_account(&db, "acct-legacy");

        submit_intelligence_feedback(
            &db,
            "acct-legacy",
            "account",
            "overall_assessment",
            "negative",
            None,
        )
        .expect("legacy negative feedback submission");

        let rows = feedback_rows(&db, "acct-legacy");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "rejected");

        let compat = db
            .get_entity_feedback("acct-legacy", "account")
            .expect("compatibility feedback rows");
        assert_eq!(compat.len(), 1);
        assert_eq!(compat[0].field, "overall_assessment");
        assert_eq!(compat[0].feedback_type, "negative");
    }

    #[test]
    fn annotated_stores_reason_without_previous_or_corrected_value() {
        let db = test_db();
        seed_account(&db, "acct-2");

        submit_intelligence_correction(
            &db,
            "acct-2",
            "account",
            "state_of_play",
            CorrectionAction::Annotated,
            None,
            Some("This is a crucial nuance the model missed."),
            None,
        )
        .expect("annotated submission");

        let rows = feedback_rows(&db, "acct-2");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "annotated");
        assert!(rows[0].1.is_none());
        assert!(rows[0].2.is_none());
        assert_eq!(
            rows[0].3.as_deref(),
            Some("This is a crucial nuance the model missed.")
        );

        let sig_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events \
                 WHERE entity_id = 'acct-2' AND signal_type = 'intelligence_annotated'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sig_count, 1);
    }

    #[test]
    fn corrected_captures_previous_and_corrected_values() {
        let db = test_db();
        seed_account(&db, "acct-3");

        // health is a health-affecting field → also exercises the recalc
        // branch. compute_account_health is stateless + cheap.
        submit_intelligence_correction(
            &db,
            "acct-3",
            "account",
            "health",
            CorrectionAction::Corrected,
            Some("yellow"),
            Some("Stakeholder just flagged concerns — no longer green."),
            None,
        )
        .expect("corrected submission");

        let rows = feedback_rows(&db, "acct-3");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "corrected");
        // Previous value falls back to stored account column value.
        assert_eq!(rows[0].1.as_deref(), Some("green"));
        assert_eq!(rows[0].2.as_deref(), Some("yellow"));
        assert_eq!(
            rows[0].3.as_deref(),
            Some("Stakeholder just flagged concerns — no longer green.")
        );

        let sig_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events \
                 WHERE entity_id = 'acct-3' AND signal_type = 'intelligence_corrected'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sig_count, 1);
    }

    #[test]
    fn dismissed_creates_feedback_row_and_suppression_tombstone() {
        let db = test_db();
        seed_account(&db, "acct-dismissed");

        submit_intelligence_correction(
            &db,
            "acct-dismissed",
            "account",
            "triage:local-risk-0",
            CorrectionAction::Dismissed,
            None,
            None,
            Some("Champion has gone dark"),
        )
        .expect("dismissed submission");

        let rows = feedback_rows(&db, "acct-dismissed");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "dismissed");

        let tombstone_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM suppression_tombstones \
                 WHERE entity_id = 'acct-dismissed' \
                 AND field_key = 'triage:local-risk-0' \
                 AND item_key = 'Champion has gone dark'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tombstone_count, 1);
    }

    #[test]
    fn successive_corrections_chain_previous_values() {
        let db = test_db();
        seed_account(&db, "acct-4");

        // First correction: green → yellow
        submit_intelligence_correction(
            &db, "acct-4", "account", "health",
            CorrectionAction::Corrected, Some("yellow"), None, None,
        )
        .unwrap();

        // Second correction: yellow → red. Previous should be "yellow"
        // (pulled from the prior correction row), not "green".
        submit_intelligence_correction(
            &db, "acct-4", "account", "health",
            CorrectionAction::Corrected, Some("red"), None, None,
        )
        .unwrap();

        let rows = feedback_rows(&db, "acct-4");
        assert_eq!(rows.len(), 2, "both corrections persisted");
        // rows are DESC by id — newest first
        assert_eq!(rows[0].1.as_deref(), Some("yellow"),
            "second correction's previous_value chains from first");
        assert_eq!(rows[0].2.as_deref(), Some("red"));
        assert_eq!(rows[1].1.as_deref(), Some("green"),
            "first correction's previous_value came from account column");
        assert_eq!(rows[1].2.as_deref(), Some("yellow"));
    }

    #[test]
    fn correction_action_parse_rejects_invalid() {
        assert!(CorrectionAction::parse("confirmed").is_ok());
        assert!(CorrectionAction::parse("rejected").is_ok());
        assert!(CorrectionAction::parse("annotated").is_ok());
        assert!(CorrectionAction::parse("corrected").is_ok());
        assert!(CorrectionAction::parse("dismissed").is_ok());
        assert!(CorrectionAction::parse("").is_err());
        assert!(CorrectionAction::parse("CONFIRMED").is_err());
        assert!(CorrectionAction::parse("nope").is_err());
    }

    #[test]
    fn is_health_affecting_field_covers_known_surface() {
        assert!(is_health_affecting_field("arr"));
        assert!(is_health_affecting_field("lifecycle"));
        assert!(is_health_affecting_field("health"));
        assert!(is_health_affecting_field("renewal_date"));
        assert!(!is_health_affecting_field("state_of_play"));
        assert!(!is_health_affecting_field("watch_list"));
    }

    /// DOS-41 backend hardening: invalid IPC payloads must be rejected
    /// before any DB write, signal emission, or Bayesian weight update.
    /// The hook has client-side guards but those are not the boundary.
    #[test]
    fn corrected_without_corrected_value_is_rejected() {
        let db = test_db();
        seed_account(&db, "acct-1");

        let err = submit_intelligence_correction(
            &db,
            "acct-1",
            "account",
            "state_of_play",
            CorrectionAction::Corrected,
            None,
            None,
            None,
        )
        .expect_err("corrected requires corrected_value");
        assert!(err.contains("corrected_value"), "err: {err}");
        assert!(
            feedback_rows(&db, "acct-1").is_empty(),
            "validation rejection must not persist any feedback rows",
        );
    }

    #[test]
    fn corrected_with_whitespace_corrected_value_is_rejected() {
        let db = test_db();
        seed_account(&db, "acct-1");

        let err = submit_intelligence_correction(
            &db,
            "acct-1",
            "account",
            "state_of_play",
            CorrectionAction::Corrected,
            Some("   "),
            None,
            None,
        )
        .expect_err("whitespace corrected_value is not valid");
        assert!(err.contains("corrected_value"), "err: {err}");
        assert!(feedback_rows(&db, "acct-1").is_empty());
    }

    #[test]
    fn annotated_without_annotation_is_rejected() {
        let db = test_db();
        seed_account(&db, "acct-1");

        let err = submit_intelligence_correction(
            &db,
            "acct-1",
            "account",
            "state_of_play",
            CorrectionAction::Annotated,
            None,
            None,
            None,
        )
        .expect_err("annotated requires annotation");
        assert!(err.contains("annotation"), "err: {err}");
        assert!(feedback_rows(&db, "acct-1").is_empty());
    }

    #[test]
    fn empty_required_fields_are_rejected() {
        let db = test_db();
        for (eid, etype, field) in &[
            ("", "account", "state_of_play"),
            ("acct-1", "", "state_of_play"),
            ("acct-1", "account", ""),
        ] {
            let err = submit_intelligence_correction(
                &db,
                eid,
                etype,
                field,
                CorrectionAction::Confirmed,
                None,
                None,
                None,
            )
            .expect_err("required fields must be non-empty");
            assert!(!err.is_empty());
        }
    }

    /// DOS-227: when a health-affecting field (arr, lifecycle, health,
    /// contract_end, renewal_date) is corrected, the feedback service must
    /// drive the full health pipeline — not just compute and discard. That
    /// means entity_assessment.health_json and entity_quality.health_score
    /// both reflect the recomputed state on the next read.
    #[test]
    fn corrected_health_affecting_field_updates_persisted_health_state() {
        use crate::intelligence::io::IntelligenceJson;

        let db = test_db();
        seed_account(&db, "acct-dos227");

        // Seed minimal intelligence so recompute_entity_health has something
        // to hydrate from (it early-returns without a row). Mirrors the
        // recompute_entity_health test fixture in services::intelligence.
        let intel = IntelligenceJson {
            entity_id: "acct-dos227".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-04-18T00:00:00Z".to_string(),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&intel)
            .expect("seed intelligence");

        // Precondition: no entity_quality row yet.
        let pre_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM entity_quality WHERE entity_id = 'acct-dos227'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pre_count, 0, "no entity_quality row before correction");

        // Correct a health-affecting field (arr).
        submit_intelligence_correction(
            &db,
            "acct-dos227",
            "account",
            "arr",
            CorrectionAction::Corrected,
            Some("250000"),
            None,
            None,
        )
        .expect("corrected submission");

        // Post: entity_quality.health_score populated by the full pipeline.
        let score: Option<f64> = db
            .conn_ref()
            .query_row(
                "SELECT health_score FROM entity_quality WHERE entity_id = 'acct-dos227'",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        assert!(
            score.is_some(),
            "entity_quality.health_score must be written after correction"
        );

        // Post: entity_assessment.health_json reflects the recomputed health.
        let refreshed = db
            .get_entity_intelligence("acct-dos227")
            .expect("read intelligence")
            .expect("intelligence row exists");
        assert!(
            refreshed.health.is_some(),
            "IntelligenceJson.health must be populated (surfaces via health_json)"
        );
    }

    /// DOS-227 (Codex finding 3): the regression that matters.
    ///
    /// The pre-fix code recorded a correction of `arr` only in
    /// `entity_feedback_events`, then called `recompute_entity_health`.
    /// `recompute_entity_health` reads `DbAccount` as-is. The corrected ARR
    /// never flowed into the account row, so the new health score was
    /// computed from the OLD ARR and was effectively identical to the
    /// pre-correction score. The existing DOS-227 test above only asserts
    /// "a score exists," not "the corrected value changed the score" —
    /// which is how the bug survived review.
    ///
    /// This test pins the actual contract: correcting `arr` upward must
    /// (1) update `accounts.arr` and (2) produce a strictly higher health
    /// score on the re-read.
    #[test]
    fn corrected_arr_updates_account_column_and_shifts_health_score() {
        use crate::intelligence::io::IntelligenceJson;

        let db = test_db();
        let account_id = "acct-dos227-arr";

        // Seed a small-ARR account. Lifecycle=renewing + small ARR lands
        // in the "low value at renewal" scoring band; bumping ARR 2.5x
        // should measurably move the score (health_scoring is
        // deterministic on DbAccount inputs).
        let account = DbAccount {
            id: account_id.to_string(),
            name: format!("Test {}", account_id),
            lifecycle: Some("renewing".to_string()),
            arr: Some(100_000.0),
            health: Some("green".to_string()),
            contract_end: Some("2026-12-31".to_string()),
            nps: Some(50),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
            ..Default::default()
        };
        db.upsert_account(&account).expect("seed account");
        let intel = IntelligenceJson {
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-04-18T00:00:00Z".to_string(),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&intel)
            .expect("seed intelligence");

        // Capture the pre-correction health score by running the same
        // pipeline with the old ARR. We do this directly rather than via
        // a sentinel correction to avoid polluting the feedback log.
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        crate::services::intelligence::recompute_entity_health(&ctx, &db, account_id, "account")
            .expect("pre recompute");
        let score_before: f64 = db
            .conn_ref()
            .query_row(
                "SELECT health_score FROM entity_quality WHERE entity_id = ?1",
                rusqlite::params![account_id],
                |row| row.get(0),
            )
            .expect("pre score");

        // Correct ARR upward.
        submit_intelligence_correction(
            &db,
            account_id,
            "account",
            "arr",
            CorrectionAction::Corrected,
            Some("250000"),
            None,
            None,
        )
        .expect("corrected submission");

        // (1) The authoritative account column reflects the correction.
        let stored = db
            .get_account(account_id)
            .expect("read account")
            .expect("account exists");
        assert!(
            (stored.arr.unwrap_or_default() - 250_000.0).abs() < 1e-6,
            "accounts.arr must reflect the correction (got {:?})",
            stored.arr
        );

        // (2) The recomputed health score reflects the new ARR, not the old.
        // For a renewing account, more ARR means more at stake — the scoring
        // function weights this and the resulting score changes.
        let score_after: f64 = db
            .conn_ref()
            .query_row(
                "SELECT health_score FROM entity_quality WHERE entity_id = ?1",
                rusqlite::params![account_id],
                |row| row.get(0),
            )
            .expect("post score");
        assert!(
            (score_after - score_before).abs() > f64::EPSILON,
            "health_score must change when ARR is corrected 100k -> 250k; \
             pre-fix it stayed identical because recompute read the old \
             account row. before={score_before}, after={score_after}"
        );
    }

    /// DOS-227: lifecycle is stored as an account column too — verify the
    /// column update path covers it (and normalizes, matching the
    /// services::accounts::update_account_field contract).
    #[test]
    fn corrected_lifecycle_updates_account_column() {
        let db = test_db();
        seed_account(&db, "acct-dos227-lc");

        submit_intelligence_correction(
            &db,
            "acct-dos227-lc",
            "account",
            "lifecycle",
            CorrectionAction::Corrected,
            Some("at-risk"),
            None,
            None,
        )
        .expect("corrected submission");

        let stored = db
            .get_account("acct-dos227-lc")
            .expect("read")
            .expect("exists");
        // Normalization produces the same token that the direct-edit path
        // would write — the shape the rest of the codebase reads.
        let expected = crate::services::accounts::normalized_lifecycle("at-risk");
        assert_eq!(
            stored.lifecycle.as_deref(),
            Some(expected.as_str()),
            "lifecycle column must reflect the corrected (and normalized) value"
        );
    }

    /// DOS-227: intelligence-blob fields must NOT touch account columns —
    /// they don't have one. Guards against the column-update branch
    /// accidentally mapping blob fields to columns.
    #[test]
    fn corrected_state_of_play_does_not_mutate_account_columns() {
        let db = test_db();
        seed_account(&db, "acct-dos227-sop");

        let before = db
            .get_account("acct-dos227-sop")
            .expect("read")
            .expect("exists");

        submit_intelligence_correction(
            &db,
            "acct-dos227-sop",
            "account",
            "state_of_play",
            CorrectionAction::Corrected,
            Some("new narrative"),
            None,
            None,
        )
        .expect("corrected submission");

        let after = db
            .get_account("acct-dos227-sop")
            .expect("read")
            .expect("exists");
        assert_eq!(before.arr, after.arr);
        assert_eq!(before.lifecycle, after.lifecycle);
        assert_eq!(before.health, after.health);
        assert_eq!(before.contract_end, after.contract_end);
        assert_eq!(before.nps, after.nps);
    }

    /// DOS-227: non-health-affecting fields must NOT trigger a health
    /// recompute. Guards against regression where the branch broadens and
    /// cascades unnecessary work / signals on every correction.
    #[test]
    fn corrected_non_health_affecting_field_does_not_touch_entity_quality() {
        let db = test_db();
        seed_account(&db, "acct-dos227-neg");

        submit_intelligence_correction(
            &db,
            "acct-dos227-neg",
            "account",
            "state_of_play",
            CorrectionAction::Corrected,
            Some("new narrative"),
            None,
            None,
        )
        .expect("corrected submission on non-health field");

        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM entity_quality WHERE entity_id = 'acct-dos227-neg'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 0,
            "non-health-affecting correction must not write entity_quality"
        );
    }

    // -----------------------------------------------------------------
    // DOS-41 Codex follow-up: `db.update_account_field` uses
    // `CAST(?1 AS REAL)` / `CAST(?1 AS INTEGER)` for `arr` / `nps`. SQLite
    // silently coerces non-numeric strings to 0, so a malformed
    // correction would wipe real ARR / NPS commercial data and then
    // recompute health from the zeroed value. The service layer now
    // rejects non-numeric corrected_values before the DB call.
    // -----------------------------------------------------------------

    /// Seed account ARR, submit a non-numeric ARR correction, assert the
    /// call errors AND the stored ARR is untouched. This is the crucial
    /// guarantee: rejection must happen BEFORE any mutation.
    #[test]
    fn corrected_arr_with_non_numeric_value_is_rejected_and_does_not_mutate_account() {
        let db = test_db();
        seed_account(&db, "acct-arr-bad");

        // Sanity check: pre-correction ARR is 100_000.
        let before = db
            .get_account("acct-arr-bad")
            .expect("read")
            .expect("account exists");
        assert_eq!(before.arr, Some(100_000.0));

        let err = submit_intelligence_correction(
            &db,
            "acct-arr-bad",
            "account",
            "arr",
            CorrectionAction::Corrected,
            Some("not-a-number"),
            None,
            None,
        )
        .expect_err("non-numeric ARR correction must be rejected");
        assert!(
            err.contains("accounts.arr") && err.contains("numeric"),
            "error must cite the column + numeric constraint, got: {err}"
        );

        // Stored ARR must be unchanged — no silent CAST-to-0.
        let after = db
            .get_account("acct-arr-bad")
            .expect("read")
            .expect("account still exists");
        assert_eq!(
            after.arr,
            Some(100_000.0),
            "DOS-41: malformed ARR correction must not mutate accounts.arr"
        );
    }

    #[test]
    fn corrected_nps_with_non_numeric_value_is_rejected() {
        let db = test_db();
        seed_account(&db, "acct-nps-bad");

        let before = db
            .get_account("acct-nps-bad")
            .expect("read")
            .expect("account exists");
        assert_eq!(before.nps, Some(50));

        let err = submit_intelligence_correction(
            &db,
            "acct-nps-bad",
            "account",
            "nps",
            CorrectionAction::Corrected,
            Some("excellent"),
            None,
            None,
        )
        .expect_err("non-integer NPS correction must be rejected");
        assert!(
            err.contains("accounts.nps") && err.contains("integer"),
            "error must cite the column + integer constraint, got: {err}"
        );

        let after = db
            .get_account("acct-nps-bad")
            .expect("read")
            .expect("account still exists");
        assert_eq!(
            after.nps,
            Some(50),
            "DOS-41: malformed NPS correction must not mutate accounts.nps"
        );
    }

    /// Valid numeric corrections must still flow through.
    #[test]
    fn corrected_arr_with_valid_numeric_value_is_applied() {
        let db = test_db();
        seed_account(&db, "acct-arr-ok");

        submit_intelligence_correction(
            &db,
            "acct-arr-ok",
            "account",
            "arr",
            CorrectionAction::Corrected,
            Some("250000"),
            None,
            None,
        )
        .expect("numeric ARR correction must succeed");

        let after = db
            .get_account("acct-arr-ok")
            .expect("read")
            .expect("account exists");
        assert_eq!(
            after.arr,
            Some(250_000.0),
            "valid ARR correction must land on accounts.arr"
        );
    }
}
