//! Action/open-loop promotion into the claim substrate.
//!
//! Actions are not a standalone claim subject yet. Runtime surfaces consume them
//! as `open_loop` / `commitment` evidence attached to the entity they concern.

use std::collections::BTreeSet;

use crate::db::claims::ClaimSensitivity;
use crate::db::{ActionDb, DbAction};
use crate::services::claims::{self, ClaimProposal};
use crate::services::context::ServiceContext;

const CLAIM_ACTOR: &str = "agent:action_claims";
const DATA_SOURCE: &str = "actions";
const CLAIM_TYPE_OPEN_LOOP: &str = "open_loop";
const CLAIM_TYPE_COMMITMENT: &str = "commitment";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionClaimPromotionReport {
    pub total_rows: usize,
    pub rows_examined: usize,
    pub next_offset: usize,
    pub finished: bool,
    pub claims_committed: usize,
    pub claims_already_present: usize,
    pub claims_withdrawn: usize,
    pub rows_skipped: usize,
    pub recompute_jobs_enqueued: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ActionClaimSubject<'a> {
    entity_type: &'static str,
    entity_id: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionClaimWrite {
    Committed,
    AlreadyPresent,
    Withdrawn(usize),
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActionClaimOutcome<'a> {
    subject: Option<ActionClaimSubject<'a>>,
    write: ActionClaimWrite,
}

impl<'a> ActionClaimOutcome<'a> {
    pub(crate) fn changed_subject(&self) -> Option<(&'static str, &'a str)> {
        if matches!(
            self.write,
            ActionClaimWrite::Committed | ActionClaimWrite::Withdrawn(_)
        ) {
            self.subject
                .map(|subject| (subject.entity_type, subject.entity_id))
        } else {
            None
        }
    }
}

/// Backfill open action rows into claim-backed `open_loop` / `commitment`
/// evidence. Idempotent: exact active claims at the same action field path are
/// skipped, while stale action claims are withdrawn when the action is terminal.
pub fn backfill_action_open_loop_claims(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<ActionClaimPromotionReport, String> {
    backfill_action_open_loop_claims_batch(ctx, db, 0, usize::MAX)
}

pub fn backfill_action_open_loop_claims_batch(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    offset: usize,
    limit: usize,
) -> Result<ActionClaimPromotionReport, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let actions = load_action_claim_backfill_rows(db)?;
    let total_rows = actions.len();
    let start = offset.min(total_rows);
    let end = start.saturating_add(limit).min(total_rows);
    let mut report = ActionClaimPromotionReport {
        total_rows,
        rows_examined: end.saturating_sub(start),
        next_offset: end,
        finished: end >= total_rows,
        ..Default::default()
    };
    let mut affected_subjects = BTreeSet::new();
    let _legacy_projection_guard = claims::suppress_legacy_projection_for_current_thread();
    let _canonical_match_guard = claims::suppress_canonical_match_for_current_thread();
    let _shadow_guard = claims::suppress_shadow_canonicalization_for_current_thread();

    for action in &actions[start..end] {
        match sync_action_open_loop_claim(ctx, db, action) {
            Ok(outcome) => {
                match outcome.write {
                    ActionClaimWrite::Committed => report.claims_committed += 1,
                    ActionClaimWrite::AlreadyPresent => report.claims_already_present += 1,
                    ActionClaimWrite::Withdrawn(count) => report.claims_withdrawn += count,
                    ActionClaimWrite::Skipped => report.rows_skipped += 1,
                }
                if matches!(
                    outcome.write,
                    ActionClaimWrite::Committed | ActionClaimWrite::Withdrawn(_)
                ) {
                    if let Some(subject) = outcome.subject {
                        affected_subjects.insert((
                            subject.entity_type.to_string(),
                            subject.entity_id.to_string(),
                        ));
                    }
                }
            }
            Err(error) => report
                .errors
                .push(format!("{} action claim failed: {error}", action.id)),
        }
    }

    for (entity_type, entity_id) in affected_subjects {
        match enqueue_action_claim_recompute(
            ctx,
            db,
            &entity_type,
            &entity_id,
            "action_claim_backfill",
        ) {
            Ok(true) => report.recompute_jobs_enqueued += 1,
            Ok(false) => {}
            Err(error) => report.errors.push(format!(
                "{entity_type}:{entity_id} action recompute enqueue failed: {error}"
            )),
        }
    }

    Ok(report)
}

pub(crate) fn sync_action_open_loop_claim<'a>(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    action: &'a DbAction,
) -> Result<ActionClaimOutcome<'a>, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let Some(subject) = action_claim_subject(action) else {
        return Ok(ActionClaimOutcome {
            subject: None,
            write: ActionClaimWrite::Skipped,
        });
    };

    if !is_open_action(action) {
        let withdrawn =
            withdraw_action_claims_for_action(ctx, db, action, subject, "action_closed")?;
        return Ok(ActionClaimOutcome {
            subject: Some(subject),
            write: if withdrawn == 0 {
                ActionClaimWrite::Skipped
            } else {
                ActionClaimWrite::Withdrawn(withdrawn)
            },
        });
    }

    let claim_type = claim_type_for_action(action, subject);
    let subject_ref = action_subject_ref(subject);
    let field_path = action_field_path(&action.id);
    let text = action.title.trim();
    if text.is_empty() {
        return Ok(ActionClaimOutcome {
            subject: Some(subject),
            write: ActionClaimWrite::Skipped,
        });
    }

    let active_claims = load_active_action_claims(db, &subject_ref, &field_path)?;
    let canonical_text = claims::normalize_claim_text(text);
    if active_claims
        .iter()
        .any(|claim| claim.claim_type == claim_type && claim.text == canonical_text)
    {
        return Ok(ActionClaimOutcome {
            subject: Some(subject),
            write: ActionClaimWrite::AlreadyPresent,
        });
    }

    for claim in active_claims
        .iter()
        .filter(|claim| claim.claim_type != claim_type)
    {
        claims::withdraw_claim(ctx, db, &claim.id, "action_kind_changed")
            .map_err(|error| error.to_string())?;
    }

    let supersedes = active_claims
        .iter()
        .find(|claim| claim.claim_type == claim_type)
        .map(|claim| claim.id.clone());
    let timestamp = action_timestamp(action);
    let source_ref = action_source_ref(action);
    let metadata_json = action_metadata_json(action, claim_type);
    let provenance_json = serde_json::json!({
        "source": {
            "system": "DailyOS",
            "kind": "action",
            "sourceAsOf": timestamp.as_str(),
            "observedAt": timestamp.as_str(),
            "referenceId": source_ref.as_str(),
        },
        "projection": {
            "schema": "actions",
            "field": field_path.as_str(),
        }
    })
    .to_string();

    let proposal = ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref,
        claim_type: claim_type.to_string(),
        field_path: Some(field_path),
        topic_key: Some(action.id.clone()),
        text: text.to_string(),
        actor: CLAIM_ACTOR.to_string(),
        data_source: DATA_SOURCE.to_string(),
        source_ref: Some(source_ref),
        source_asof: Some(timestamp.clone()),
        observed_at: timestamp,
        provenance_json,
        metadata_json: Some(metadata_json),
        thread_id: None,
        temporal_scope: None,
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes,
        tombstone: None,
    };

    claims::commit_claim(ctx, db, proposal).map_err(|error| error.to_string())?;
    Ok(ActionClaimOutcome {
        subject: Some(subject),
        write: ActionClaimWrite::Committed,
    })
}

pub fn enqueue_action_claim_recompute(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    reason: &str,
) -> Result<bool, String> {
    let source_claim_version = db
        .current_subject_claim_version(entity_type, entity_id)
        .map_err(|error| format!("read action subject claim version failed: {error}"))?;
    let payload = serde_json::json!({
        "reason": reason,
        "source_claim_version": source_claim_version,
    })
    .to_string();
    let outcome = crate::services::signals::emit_once_for_key(
        ctx,
        db,
        &format!("action_claims:{reason}:{entity_type}:{entity_id}:{source_claim_version}"),
        entity_type,
        entity_id,
        "action_claims_updated",
        "action_claims",
        Some(&payload),
        0.8,
    )
    .map_err(|error| format!("signal emit failed: {error}"))?;
    if outcome.coalesced {
        return Ok(false);
    }

    crate::services::invalidation_jobs::enqueue_signal_claim_recompute_in_tx(
        db,
        &outcome.id,
        entity_type,
        entity_id,
    )?;
    Ok(true)
}

fn load_action_claim_backfill_rows(db: &ActionDb) -> Result<Vec<DbAction>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    COALESCE(actions.action_kind, 'task') AS action_kind,
                    actions.commitment_id, actions.owner_raw, actions.owner_entity_id,
                    actions.owner_confidence, actions.owner_source, actions.trust_score,
                    actions.trust_band,
                    (SELECT COUNT(DISTINCT acs.source_key)
                       FROM action_commitment_sources acs
                      WHERE acs.action_id = actions.id
                         OR acs.commitment_id IN (
                            SELECT b.commitment_id
                              FROM ai_commitment_bridge b
                             WHERE b.action_id = actions.id
                         )) AS commitment_source_count,
                    actions.needs_decision, actions.decision_owner, actions.decision_stakes,
                    all_links.linear_identifier, all_links.linear_url
               FROM actions
               LEFT JOIN accounts acc ON actions.account_id = acc.id
               LEFT JOIN action_linear_links all_links ON actions.id = all_links.action_id
              WHERE COALESCE(actions.is_demo, 0) = 0",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(DbAction {
                id: row.get(0)?,
                title: row.get(1)?,
                priority: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                due_date: row.get(5)?,
                completed_at: row.get(6)?,
                account_id: row.get(7)?,
                project_id: row.get(8)?,
                source_type: row.get(9)?,
                source_id: row.get(10)?,
                source_label: row.get(11)?,
                context: row.get(12)?,
                waiting_on: row.get(13)?,
                updated_at: row.get(14)?,
                person_id: row.get(15)?,
                account_name: row.get(16)?,
                action_kind: row.get(17)?,
                commitment_id: row.get(18)?,
                owner_raw: row.get(19)?,
                owner_entity_id: row.get(20)?,
                owner_confidence: row.get(21)?,
                owner_source: row.get(22)?,
                trust_score: row.get(23)?,
                trust_band: row.get(24)?,
                commitment_source_count: row.get(25)?,
                needs_decision: row.get::<_, i32>(26)? != 0,
                decision_owner: row.get(27)?,
                decision_stakes: row.get(28)?,
                linear_identifier: row.get(29)?,
                linear_url: row.get(30)?,
                next_meeting_title: None,
                next_meeting_start: None,
            })
        })
        .map_err(|error| error.to_string())?;

    let mut actions = Vec::new();
    for row in rows {
        actions.push(row.map_err(|error| error.to_string())?);
    }
    Ok(actions)
}

fn is_open_action(action: &DbAction) -> bool {
    matches!(
        action.status.as_str(),
        crate::action_status::BACKLOG
            | crate::action_status::UNSTARTED
            | crate::action_status::STARTED
    )
}

fn action_claim_subject(action: &DbAction) -> Option<ActionClaimSubject<'_>> {
    if let Some(account_id) = action
        .account_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        return Some(ActionClaimSubject {
            entity_type: "account",
            entity_id: account_id,
        });
    }
    if let Some(project_id) = action
        .project_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        return Some(ActionClaimSubject {
            entity_type: "project",
            entity_id: project_id,
        });
    }
    if let Some(person_id) = action
        .person_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        return Some(ActionClaimSubject {
            entity_type: "person",
            entity_id: person_id,
        });
    }
    match (action.source_type.as_deref(), action.source_id.as_deref()) {
        (Some("transcript" | "post_meeting"), Some(meeting_id)) if !meeting_id.is_empty() => {
            Some(ActionClaimSubject {
                entity_type: "meeting",
                entity_id: meeting_id,
            })
        }
        _ => None,
    }
}

fn claim_type_for_action(action: &DbAction, subject: ActionClaimSubject<'_>) -> &'static str {
    if subject.entity_type == "account"
        && action.action_kind == crate::action_status::KIND_COMMITMENT
    {
        CLAIM_TYPE_COMMITMENT
    } else {
        CLAIM_TYPE_OPEN_LOOP
    }
}

fn action_subject_ref(subject: ActionClaimSubject<'_>) -> String {
    serde_json::json!({
        "kind": subject.entity_type,
        "id": subject.entity_id,
    })
    .to_string()
}

fn action_field_path(action_id: &str) -> String {
    format!("actions.{action_id}")
}

fn action_timestamp(action: &DbAction) -> String {
    if action.updated_at.trim().is_empty() {
        action.created_at.clone()
    } else {
        action.updated_at.clone()
    }
}

fn action_source_ref(action: &DbAction) -> String {
    format!("action:{}", action.id)
}

fn action_metadata_json(action: &DbAction, claim_type: &str) -> String {
    serde_json::json!({
        "loop_kind": action.action_kind.as_str(),
        "status": action.status.as_str(),
        "priority": action.priority,
        "owner": action.owner_raw.as_deref().or(action.waiting_on.as_deref()),
        "owner_entity_id": action.owner_entity_id.as_deref(),
        "owner_confidence": action.owner_confidence,
        "owner_source": action.owner_source.as_deref(),
        "due_date": action.due_date.as_deref(),
        "completed_at": action.completed_at.as_deref(),
        "source_type": action.source_type.as_deref(),
        "source_id": action.source_id.as_deref(),
        "source_label": action.source_label.as_deref(),
        "account_id": action.account_id.as_deref(),
        "project_id": action.project_id.as_deref(),
        "person_id": action.person_id.as_deref(),
        "action_id": action.id.as_str(),
        "claim_type": claim_type,
    })
    .to_string()
}

fn load_active_action_claims(
    db: &ActionDb,
    subject_ref: &str,
    field_path: &str,
) -> Result<Vec<abilities_runtime::types::IntelligenceClaim>, String> {
    let mut claims = Vec::new();
    for claim_type in [CLAIM_TYPE_OPEN_LOOP, CLAIM_TYPE_COMMITMENT] {
        claims.extend(
            claims::load_claims_active(db, subject_ref, Some(claim_type))
                .map_err(|error| error.to_string())?
                .into_iter()
                .filter(|claim| claim.field_path.as_deref() == Some(field_path)),
        );
    }
    Ok(claims)
}

fn withdraw_action_claims_for_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    action: &DbAction,
    subject: ActionClaimSubject<'_>,
    reason: &str,
) -> Result<usize, String> {
    let subject_ref = action_subject_ref(subject);
    let field_path = action_field_path(&action.id);
    let claims = load_active_action_claims(db, &subject_ref, &field_path)?;
    let mut withdrawn = 0usize;
    for claim in claims {
        claims::withdraw_claim(ctx, db, &claim.id, reason).map_err(|error| error.to_string())?;
        withdrawn += 1;
    }
    Ok(withdrawn)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::db::{AccountType, DbAccount};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    fn test_db() -> ActionDb {
        ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests())
    }

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_account(db: &ActionDb, id: &str) {
        db.upsert_account(&DbAccount {
            id: id.to_string(),
            name: "Example Account".to_string(),
            account_type: AccountType::Customer,
            updated_at: "2026-05-20T00:00:00Z".to_string(),
            ..Default::default()
        })
        .expect("seed account");
    }

    fn action(id: &str, account_id: &str, status: &str) -> DbAction {
        DbAction {
            id: id.to_string(),
            title: "Send the follow-up plan".to_string(),
            priority: crate::action_status::PRIORITY_DEFAULT,
            status: status.to_string(),
            created_at: "2026-05-20T00:00:00Z".to_string(),
            due_date: Some("2026-05-28".to_string()),
            completed_at: None,
            account_id: Some(account_id.to_string()),
            project_id: None,
            source_type: Some("user_manual".to_string()),
            source_id: None,
            source_label: Some("manual".to_string()),
            action_kind: crate::action_status::KIND_TASK.to_string(),
            commitment_id: None,
            owner_raw: Some("Alex".to_string()),
            owner_entity_id: None,
            owner_confidence: None,
            owner_source: None,
            trust_score: None,
            trust_band: None,
            commitment_source_count: None,
            context: None,
            waiting_on: None,
            updated_at: "2026-05-21T00:00:00Z".to_string(),
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
            needs_decision: false,
            decision_owner: None,
            decision_stakes: None,
            linear_identifier: None,
            linear_url: None,
        }
    }

    #[test]
    fn backfill_action_open_loops_commits_claim_backed_evidence() {
        let db = test_db();
        seed_account(&db, "acct-actions");
        db.upsert_action(&action(
            "action-claim-backfill",
            "acct-actions",
            crate::action_status::UNSTARTED,
        ))
        .expect("seed action");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let report = backfill_action_open_loop_claims(&ctx, &db).expect("backfill actions");

        assert_eq!(report.errors, Vec::<String>::new());
        assert_eq!(report.claims_committed, 1);
        let subject_ref = serde_json::json!({"kind": "account", "id": "acct-actions"}).to_string();
        let claims = claims::load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE_OPEN_LOOP))
            .expect("active open loop claims");
        assert!(claims.iter().any(|claim| {
            claim.field_path.as_deref() == Some("actions.action-claim-backfill")
                && claim.text == "send the follow-up plan"
        }));
    }

    #[test]
    fn terminal_action_withdraws_open_loop_claim() {
        let db = test_db();
        seed_account(&db, "acct-actions");
        let mut row = action(
            "action-claim-terminal",
            "acct-actions",
            crate::action_status::UNSTARTED,
        );
        db.upsert_action(&row).expect("seed action");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(43);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        sync_action_open_loop_claim(&ctx, &db, &row).expect("commit action claim");

        row.status = crate::action_status::COMPLETED.to_string();
        row.updated_at = "2026-05-23T00:00:00Z".to_string();
        sync_action_open_loop_claim(&ctx, &db, &row).expect("withdraw action claim");

        let subject_ref = serde_json::json!({"kind": "account", "id": "acct-actions"}).to_string();
        let claims = claims::load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE_OPEN_LOOP))
            .expect("active open loop claims");
        assert!(
            claims
                .iter()
                .all(|claim| claim.field_path.as_deref() != Some("actions.action-claim-terminal")),
            "terminal actions should not remain active open-loop evidence"
        );
    }
}
