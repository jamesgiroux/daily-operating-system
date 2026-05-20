use chrono::{DateTime, Utc};
use rusqlite::params;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::claim_receipt::contracts::ClaimReceipt;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueueTargetKind {
    Claim,
    Proposal,
    Candidate,
}

impl QueueTargetKind {
    pub fn as_storage_str(self) -> &'static str {
        match self {
            QueueTargetKind::Claim => "claim",
            QueueTargetKind::Proposal => "proposal",
            QueueTargetKind::Candidate => "candidate",
        }
    }

    pub fn from_storage_str(value: &str) -> Option<Self> {
        match value {
            "claim" => Some(QueueTargetKind::Claim),
            "proposal" => Some(QueueTargetKind::Proposal),
            "candidate" => Some(QueueTargetKind::Candidate),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub kind: QueueTargetKind,
    pub target_id: String,
    pub receipt: Option<ClaimReceipt>,
    pub deferred_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeferralRow {
    pub id: String,
    pub kind: QueueTargetKind,
    pub target_id: String,
    pub surface: Option<String>,
    pub reason: Option<String>,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub actor: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("unknown target_kind value in storage: {0}")]
    UnknownTargetKind(String),
    #[error("invalid timestamp in storage: {0}")]
    BadTimestamp(String),
    #[error("storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

pub async fn defer_target(
    state: &AppState,
    kind: QueueTargetKind,
    target_id: &str,
    surface: Option<&str>,
    reason: Option<&str>,
    snoozed_until: Option<DateTime<Utc>>,
    actor: &str,
) -> Result<DeferralRow, QueueError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let row = DeferralRow {
        id: id.clone(),
        kind,
        target_id: target_id.to_string(),
        surface: surface.map(str::to_string),
        reason: reason.map(str::to_string),
        snoozed_until,
        resolved_at: None,
        actor: actor.to_string(),
        created_at: now,
        updated_at: now,
    };

    let insert_row = row.clone();
    state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "INSERT INTO claim_review_deferrals (
                        id, target_kind, target_id, surface, reason, snoozed_until,
                        created_at, updated_at, resolved_at, actor
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9)",
                    params![
                        insert_row.id,
                        insert_row.kind.as_storage_str(),
                        insert_row.target_id,
                        insert_row.surface,
                        insert_row.reason,
                        insert_row.snoozed_until.map(|ts| ts.to_rfc3339()),
                        insert_row.created_at.to_rfc3339(),
                        insert_row.updated_at.to_rfc3339(),
                        insert_row.actor,
                    ],
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|message| QueueError::Storage(anyhow::anyhow!(message)))?;

    Ok(row)
}

pub async fn resolve_deferral(
    state: &AppState,
    kind: QueueTargetKind,
    target_id: &str,
) -> Result<usize, QueueError> {
    let kind_str = kind.as_storage_str();
    let target_id = target_id.to_string();
    let now = Utc::now().to_rfc3339();

    state
        .db_write(move |db| {
            let n = db
                .conn_ref()
                .execute(
                    "UPDATE claim_review_deferrals
                       SET resolved_at = ?1, updated_at = ?1
                     WHERE target_kind = ?2 AND target_id = ?3 AND resolved_at IS NULL",
                    params![now, kind_str, target_id],
                )
                .map_err(|e| e.to_string())?;
            Ok(n)
        })
        .await
        .map_err(|message| QueueError::Storage(anyhow::anyhow!(message)))
}

pub async fn list_active_deferrals(state: &AppState) -> Result<Vec<DeferralRow>, QueueError> {
    let rows = state
        .db_read(move |db| {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT id, target_kind, target_id, surface, reason, snoozed_until,
                            resolved_at, actor, created_at, updated_at
                       FROM claim_review_deferrals
                      WHERE resolved_at IS NULL
                      ORDER BY created_at DESC",
                )
                .map_err(|e| e.to_string())?;
            let collected = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(collected)
        })
        .await
        .map_err(|message| QueueError::Storage(anyhow::anyhow!(message)))?;

    rows.into_iter()
        .map(
            |(
                id,
                kind_str,
                target_id,
                surface,
                reason,
                snoozed,
                resolved,
                actor,
                created,
                updated,
            )| {
                let kind = QueueTargetKind::from_storage_str(&kind_str)
                    .ok_or_else(|| QueueError::UnknownTargetKind(kind_str.clone()))?;
                Ok(DeferralRow {
                    id,
                    kind,
                    target_id,
                    surface,
                    reason,
                    snoozed_until: parse_optional_ts(snoozed.as_deref())?,
                    resolved_at: parse_optional_ts(resolved.as_deref())?,
                    actor,
                    created_at: parse_required_ts(&created)?,
                    updated_at: parse_required_ts(&updated)?,
                })
            },
        )
        .collect()
}

fn parse_required_ts(value: &str) -> Result<DateTime<Utc>, QueueError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| QueueError::BadTimestamp(value.to_string()))
}

fn parse_optional_ts(value: Option<&str>) -> Result<Option<DateTime<Utc>>, QueueError> {
    match value {
        None => Ok(None),
        Some(s) => parse_required_ts(s).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_state() -> (AppState, tempfile::TempDir) {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db_path = tempdir.path().join("claim-review-queue.db");
        let db_service = crate::db_service::DbService::open_at_unencrypted(db_path)
            .await
            .expect("open test db service");
        (AppState::test_with_db_service(db_service), tempdir)
    }

    #[tokio::test]
    async fn defer_then_list_then_resolve_roundtrip() {
        let (state, _tempdir) = test_state().await;

        let row = defer_target(
            &state,
            QueueTargetKind::Claim,
            "claim-1",
            Some("actions_work"),
            Some("waiting on stakeholder"),
            None,
            "user:test",
        )
        .await
        .expect("defer");
        assert_eq!(row.kind, QueueTargetKind::Claim);
        assert_eq!(row.target_id, "claim-1");
        assert!(row.resolved_at.is_none());

        let active = list_active_deferrals(&state).await.expect("list active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, row.id);
        assert_eq!(active[0].target_id, "claim-1");

        let resolved = resolve_deferral(&state, QueueTargetKind::Claim, "claim-1")
            .await
            .expect("resolve");
        assert_eq!(resolved, 1);

        let after = list_active_deferrals(&state).await.expect("list after");
        assert!(after.is_empty());
    }

    #[tokio::test]
    async fn resolve_with_no_match_returns_zero() {
        let (state, _tempdir) = test_state().await;
        let n = resolve_deferral(&state, QueueTargetKind::Proposal, "missing")
            .await
            .expect("resolve");
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn candidate_target_kind_round_trips() {
        let (state, _tempdir) = test_state().await;
        let row = defer_target(
            &state,
            QueueTargetKind::Candidate,
            "cand-1",
            None,
            None,
            None,
            "agent:salience",
        )
        .await
        .expect("defer candidate");
        assert_eq!(row.kind, QueueTargetKind::Candidate);

        let active = list_active_deferrals(&state).await.expect("list");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].kind, QueueTargetKind::Candidate);
    }

    #[test]
    fn storage_str_round_trips_all_variants() {
        for k in [
            QueueTargetKind::Claim,
            QueueTargetKind::Proposal,
            QueueTargetKind::Candidate,
        ] {
            assert_eq!(
                QueueTargetKind::from_storage_str(k.as_storage_str()),
                Some(k)
            );
        }
        assert_eq!(QueueTargetKind::from_storage_str("nonsense"), None);
    }
}
