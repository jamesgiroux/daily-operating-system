use abilities_runtime::abilities::claims::ClaimType;
use abilities_runtime::abilities::provenance::source::{
    DataSource, DocumentId, MeetingId, SourceAttribution, SourceIdentifier, WorkspaceFileKind,
};
use abilities_runtime::types::{ClaimSensitivity, TemporalScope};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::db::{claim_invalidation::SubjectRef, ActionDb};
use crate::entity::EntityType;
use crate::intelligence::canonicalization::item_hash;
use crate::services::claims::{self, commit_claim, ClaimError, ClaimProposal, CommittedClaim};
use crate::services::context::ServiceContext;
use crate::services::workspace_ingestion::contracts::FileIdentity;
use crate::services::workspace_ingestion::lifecycle::workspace_file_kind_slug;
use crate::services::workspace_ingestion::lifecycle::{LifecycleRepo, LifecycleState};
use crate::services::workspace_ingestion::pipeline::{file_id_from_identity, EntityRef};

static TRANSCRIPT_CLAIM_COMMIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
pub(crate) const TRANSCRIPT_VERIFIED_QUOTE_MAX_CHARS: usize = 320;
pub(crate) const TRANSCRIPT_VERIFIED_QUOTE_MAX_LINES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptClaimProductionMode {
    FreshSingleTranscript,
    ExplicitSingleMeetingReprocess {
        manifest: TranscriptReprocessManifest,
    },
    ProviderBackground,
    BulkBackfill,
}

impl TranscriptClaimProductionMode {
    fn is_enabled(&self) -> bool {
        matches!(
            self,
            Self::FreshSingleTranscript | Self::ExplicitSingleMeetingReprocess { .. }
        )
    }

    fn as_slug(&self) -> &'static str {
        match self {
            Self::FreshSingleTranscript => "fresh_single_transcript",
            Self::ExplicitSingleMeetingReprocess { .. } => "explicit_single_meeting_reprocess",
            Self::ProviderBackground => "provider_background",
            Self::BulkBackfill => "bulk_backfill",
        }
    }

    fn manifest(&self) -> Option<&TranscriptReprocessManifest> {
        match self {
            Self::ExplicitSingleMeetingReprocess { manifest } => Some(manifest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptReprocessManifest {
    pub manifest_id: String,
    pub reason: String,
    pub prior_claims_preserved: bool,
    pub tombstones_preserved: bool,
    pub feedback_preserved: bool,
    pub omitted_prior_claim_ids: Vec<String>,
}

impl TranscriptReprocessManifest {
    fn is_complete(&self) -> bool {
        !self.manifest_id.trim().is_empty()
            && self.prior_claims_preserved
            && self.tombstones_preserved
            && self.feedback_preserved
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptClaimSubject {
    Account { id: String },
    Project { id: String },
    Person { id: String },
    Meeting { id: String },
}

impl TranscriptClaimSubject {
    fn to_subject_ref(&self) -> SubjectRef {
        match self {
            Self::Account { id } => SubjectRef::Account { id: id.clone() },
            Self::Project { id } => SubjectRef::Project { id: id.clone() },
            Self::Person { id } => SubjectRef::Person { id: id.clone() },
            Self::Meeting { id } => SubjectRef::Meeting { id: id.clone() },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptClaimKind {
    Win,
    Risk,
    Decision,
    Commitment,
    OpenLoop,
    Topic,
}

impl TranscriptClaimKind {
    fn claim_type(self) -> ClaimType {
        match self {
            Self::Win => ClaimType::EntityWin,
            Self::Risk => ClaimType::EntityRisk,
            Self::Decision => ClaimType::MeetingEventNote,
            Self::Commitment => ClaimType::Commitment,
            Self::OpenLoop => ClaimType::OpenLoop,
            Self::Topic => ClaimType::MeetingTopic,
        }
    }

    fn default_field_path(self) -> &'static str {
        match self {
            Self::Win => "transcript.wins",
            Self::Risk => "transcript.risks",
            Self::Decision => "transcript.decisions",
            Self::Commitment => "transcript.commitments",
            Self::OpenLoop => "transcript.open_loops",
            Self::Topic => "transcript.topics",
        }
    }

    fn as_slug(self) -> &'static str {
        match self {
            Self::Win => "win",
            Self::Risk => "risk",
            Self::Decision => "decision",
            Self::Commitment => "commitment",
            Self::OpenLoop => "open_loop",
            Self::Topic => "topic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedTranscriptQuote {
    pub text: String,
    pub start_char: usize,
    pub end_char: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptClaimItem {
    pub kind: TranscriptClaimKind,
    pub subject: TranscriptClaimSubject,
    pub text: String,
    pub field_path: Option<String>,
    pub topic_key: Option<String>,
    pub verified_quote: Option<VerifiedTranscriptQuote>,
    pub source_item_ref: Option<String>,
    pub sensitivity: ClaimSensitivity,
}

#[derive(Debug, Clone)]
pub struct TranscriptClaimBatch {
    pub meeting_id: String,
    pub workspace_file_id: String,
    pub workspace_file_kind: WorkspaceFileKind,
    pub source_content_hash: String,
    pub source_asof: DateTime<Utc>,
    pub observed_at: DateTime<Utc>,
    pub production_mode: TranscriptClaimProductionMode,
    pub items: Vec<TranscriptClaimItem>,
}

pub struct TranscriptWorkspaceSourceInput<'a> {
    pub workspace_root: &'a Path,
    pub file_path: &'a Path,
    pub source_kind: WorkspaceFileKind,
    pub source_asof: DateTime<Utc>,
    pub content: &'a str,
    pub entity: Option<TranscriptClaimSubject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptWorkspaceSource {
    pub file_id: String,
    pub source_kind: WorkspaceFileKind,
    pub source_asof: DateTime<Utc>,
    pub content_sha256: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TranscriptClaimCommitReport {
    pub attempted: usize,
    pub inserted: usize,
    pub reinforced: usize,
    pub skipped_duplicates: usize,
    pub skipped_disabled_mode: usize,
    pub committed_claim_ids: Vec<String>,
    pub duplicate_claim_ids: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TranscriptClaimError {
    #[error("missing transcript source identity: {0}")]
    MissingSourceIdentity(&'static str),
    #[error("explicit single-meeting reprocess requires a complete manifest")]
    IncompleteReprocessManifest,
    #[error("invalid transcript source_ref")]
    InvalidSourceRef,
    #[error("serialize transcript claim JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database read failed: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("source registration failed: {0}")]
    SourceRegistration(String),
    #[error("source attribution failed: {0}")]
    SourceAttribution(String),
    #[error("service mode rejected transcript claim write: {0}")]
    Mode(String),
    #[error("claim commit failed: {0}")]
    Claim(#[from] ClaimError),
    #[error("transcript claim unexpectedly reinforced existing claim: {0}")]
    UnexpectedReinforcement(String),
}

pub fn ensure_transcript_workspace_source(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: TranscriptWorkspaceSourceInput<'_>,
) -> Result<TranscriptWorkspaceSource, TranscriptClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|error| TranscriptClaimError::Mode(error.to_string()))?;
    let canonical_path = input
        .file_path
        .canonicalize()
        .map_err(|error| TranscriptClaimError::SourceRegistration(error.to_string()))?;
    let canonical_workspace_root = input
        .workspace_root
        .canonicalize()
        .map_err(|error| TranscriptClaimError::SourceRegistration(error.to_string()))?;
    let metadata = std::fs::metadata(&canonical_path)
        .map_err(|error| TranscriptClaimError::SourceRegistration(error.to_string()))?;
    let identity = FileIdentity {
        canonical_path,
        device: metadata.dev(),
        inode: metadata.ino(),
    };
    let file_id = file_id_from_identity(&identity, &canonical_workspace_root)
        .map_err(|error| TranscriptClaimError::SourceRegistration(format!("{error:?}")))?;
    let content_sha256 = content_sha256(input.content);
    let entity = input
        .entity
        .as_ref()
        .and_then(transcript_subject_to_entity_ref);

    LifecycleRepo::insert_pending(
        db.conn_ref(),
        &file_id,
        &identity,
        &input.source_kind,
        input.source_asof,
        entity.as_ref(),
    )
    .map_err(|error| TranscriptClaimError::SourceRegistration(error.to_string()))?;
    LifecycleRepo::update_content_sha256(db.conn_ref(), &file_id, &content_sha256)
        .map_err(|error| TranscriptClaimError::SourceRegistration(error.to_string()))?;
    LifecycleRepo::transition(
        db.conn_ref(),
        &file_id,
        LifecycleState::Pending,
        LifecycleState::Ingested,
    )
    .map_err(|error| TranscriptClaimError::SourceRegistration(error.to_string()))?;

    Ok(TranscriptWorkspaceSource {
        file_id,
        source_kind: input.source_kind,
        source_asof: input.source_asof,
        content_sha256,
    })
}

pub fn commit_transcript_claims(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    batch: TranscriptClaimBatch,
) -> Result<TranscriptClaimCommitReport, TranscriptClaimError> {
    validate_batch_identity(&batch)?;

    let mut report = TranscriptClaimCommitReport {
        attempted: batch.items.len(),
        ..TranscriptClaimCommitReport::default()
    };

    if !batch.production_mode.is_enabled() {
        report.skipped_disabled_mode = batch.items.len();
        report.warnings.push(format!(
            "transcript claim production disabled for mode {}",
            batch.production_mode.as_slug()
        ));
        return Ok(report);
    }

    if let Some(manifest) = batch.production_mode.manifest() {
        if !manifest.is_complete() {
            return Err(TranscriptClaimError::IncompleteReprocessManifest);
        }
    }

    let source_ref = workspace_source_ref(&batch.workspace_file_id)?;
    let workspace_kind = workspace_file_kind_slug(&batch.workspace_file_kind);

    for (index, item) in batch.items.iter().enumerate() {
        let text = item.text.trim();
        if text.is_empty() {
            report
                .warnings
                .push(format!("item {index} skipped because text is empty"));
            continue;
        }

        let proposal = transcript_claim_proposal(&batch, item, index, &source_ref, workspace_kind)?;
        let data_source = format!("workspace_file:{workspace_kind}");

        let _guard = TRANSCRIPT_CLAIM_COMMIT_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("transcript claim commit lock poisoned");
        if let Some(existing_id) = duplicate_transcript_claim_id(
            db,
            item,
            proposal.field_path.as_deref(),
            text,
            &source_ref,
            &data_source,
        )? {
            report.skipped_duplicates += 1;
            report.duplicate_claim_ids.push(existing_id);
            continue;
        }

        let _canonical_guard = claims::suppress_canonical_match_for_current_thread();
        match commit_claim(ctx, db, proposal)? {
            CommittedClaim::Inserted { claim } => {
                report.inserted += 1;
                report.committed_claim_ids.push(claim.id);
            }
            CommittedClaim::Reinforced { claim, .. } => {
                return Err(TranscriptClaimError::UnexpectedReinforcement(claim.id));
            }
            CommittedClaim::Forked { new_claim_id, .. } => {
                report.inserted += 1;
                report.committed_claim_ids.push(new_claim_id);
            }
            CommittedClaim::Tombstoned { claim } => {
                report.committed_claim_ids.push(claim.id);
            }
        }
    }

    Ok(report)
}

fn validate_batch_identity(batch: &TranscriptClaimBatch) -> Result<(), TranscriptClaimError> {
    if batch.meeting_id.trim().is_empty() {
        return Err(TranscriptClaimError::MissingSourceIdentity("meeting_id"));
    }
    if batch.workspace_file_id.trim().is_empty() {
        return Err(TranscriptClaimError::MissingSourceIdentity(
            "workspace_file_id",
        ));
    }
    workspace_source_ref(&batch.workspace_file_id)?;
    Ok(())
}

fn workspace_source_ref(file_id: &str) -> Result<String, TranscriptClaimError> {
    let file_id = file_id.trim();
    if file_id.is_empty()
        || file_id.contains('/')
        || file_id.contains('\\')
        || file_id.contains(':')
    {
        return Err(TranscriptClaimError::InvalidSourceRef);
    }
    Ok(format!("workspace_file:{file_id}"))
}

fn transcript_claim_proposal(
    batch: &TranscriptClaimBatch,
    item: &TranscriptClaimItem,
    index: usize,
    source_ref: &str,
    workspace_kind: &str,
) -> Result<ClaimProposal, TranscriptClaimError> {
    let subject = item.subject.to_subject_ref();
    let subject_ref = claims::canonical_subject_ref(&subject)?;
    let claim_type = item.kind.claim_type();
    let field_path = item
        .field_path
        .clone()
        .unwrap_or_else(|| item.kind.default_field_path().to_string());
    let source_attribution = SourceAttribution::new(
        DataSource::WorkspaceFile {
            kind: batch.workspace_file_kind.clone(),
        },
        vec![
            SourceIdentifier::Document {
                document_id: DocumentId::new(batch.workspace_file_id.clone()),
                chunk_id: None,
            },
            SourceIdentifier::Meeting {
                meeting_id: MeetingId::new(batch.meeting_id.clone()),
            },
        ],
        batch.observed_at,
        Some(batch.source_asof),
        0.5,
        None,
    )
    .map_err(|error| TranscriptClaimError::SourceAttribution(error.to_string()))?;
    let provenance_json = serde_json::to_string(&source_attribution)?;
    let metadata_json = transcript_claim_metadata(batch, item, index, source_ref, workspace_kind)?;

    Ok(ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref,
        claim_type: claim_type.as_str().to_string(),
        field_path: Some(field_path),
        topic_key: item.topic_key.clone(),
        text: item.text.trim().to_string(),
        actor: "agent:transcript_claims".to_string(),
        data_source: format!("workspace_file:{workspace_kind}"),
        source_ref: Some(source_ref.to_string()),
        source_asof: Some(batch.source_asof.to_rfc3339()),
        observed_at: batch.observed_at.to_rfc3339(),
        provenance_json,
        metadata_json: Some(metadata_json),
        thread_id: None,
        temporal_scope: Some(TemporalScope::PointInTime),
        sensitivity: Some(item.sensitivity.clone()),
        supersedes: None,
        tombstone: None,
    })
}

fn transcript_claim_metadata(
    batch: &TranscriptClaimBatch,
    item: &TranscriptClaimItem,
    index: usize,
    source_ref: &str,
    workspace_kind: &str,
) -> Result<String, TranscriptClaimError> {
    let quote_payload = item
        .verified_quote
        .as_ref()
        .filter(|quote| transcript_quote_can_persist_metadata(item, quote))
        .map(|quote| {
            serde_json::json!({
                "text": quote.text,
                "start_char": quote.start_char,
                "end_char": quote.end_char,
                "verification": "exact_match",
                "redaction_policy": "sensitivity_ceiling"
            })
        });
    let quote_verified = quote_payload.is_some();
    Ok(serde_json::json!({
        "schema_version": 1,
        "producer": "transcript_claims",
        "meeting_id": batch.meeting_id,
        "workspace_file_id": batch.workspace_file_id,
        "workspace_file_kind": workspace_kind,
        "source_content_sha256": batch.source_content_hash,
        "source_ref": source_ref,
        "production_mode": batch.production_mode.as_slug(),
        "reprocess_manifest_id": batch.production_mode.manifest().map(|m| m.manifest_id.as_str()),
        "claim_kind": item.kind.as_slug(),
        "source_item_index": index,
        "source_item_ref": item.source_item_ref,
        "quote": quote_payload,
        "quote_verified": quote_verified
    })
    .to_string())
}

pub(crate) fn transcript_quote_within_bounds(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= TRANSCRIPT_VERIFIED_QUOTE_MAX_CHARS
        && trimmed.lines().count() <= TRANSCRIPT_VERIFIED_QUOTE_MAX_LINES
}

fn transcript_quote_can_persist_metadata(
    item: &TranscriptClaimItem,
    quote: &VerifiedTranscriptQuote,
) -> bool {
    matches!(item.sensitivity, ClaimSensitivity::Internal)
        && quote.start_char < quote.end_char
        && transcript_quote_within_bounds(&quote.text)
}

fn content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn transcript_subject_to_entity_ref(subject: &TranscriptClaimSubject) -> Option<EntityRef> {
    let (entity_type, entity_id) = match subject {
        TranscriptClaimSubject::Account { id } => (EntityType::Account, id.as_str()),
        TranscriptClaimSubject::Project { id } => (EntityType::Project, id.as_str()),
        TranscriptClaimSubject::Person { id } => (EntityType::Person, id.as_str()),
        TranscriptClaimSubject::Meeting { .. } => return None,
    };
    Some(EntityRef {
        entity_type,
        entity_id: abilities_runtime::abilities::provenance::source::EntityId::new(entity_id),
        entity_name: None,
    })
}

fn duplicate_transcript_claim_id(
    db: &ActionDb,
    item: &TranscriptClaimItem,
    field_path: Option<&str>,
    text: &str,
    source_ref: &str,
    data_source: &str,
) -> Result<Option<String>, rusqlite::Error> {
    let canonical_text = claims::normalize_claim_text(text);
    let claim_type = item.kind.claim_type();
    let field_path = field_path.unwrap_or("");
    let subject_ref = claims::canonical_subject_ref(&item.subject.to_subject_ref())
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let hash = item_hash(
        claims::item_kind_for_claim_type(claim_type.as_str()),
        &canonical_text,
    );
    let dedup_key =
        claims::compute_dedup_key(&hash, &subject_ref, claim_type.as_str(), Some(field_path));
    db.conn_ref()
        .query_row(
            "SELECT id
               FROM intelligence_claims
              WHERE dedup_key = ?1
                AND (
                    (claim_state = 'active' AND surfacing_state = 'active')
                    OR (
                        source_ref = ?2
                        AND data_source = ?3
                        AND claim_state IN ('active', 'dormant', 'tombstoned', 'withdrawn')
                    )
                )
              ORDER BY CASE
                    WHEN claim_state = 'active' AND surfacing_state = 'active' THEN 0
                    ELSE 1
                END,
                created_at DESC
              LIMIT 1",
            params![dedup_key, source_ref, data_source],
            |row| row.get(0),
        )
        .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::services::claims::{record_claim_feedback, ClaimFeedbackInput};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use abilities_runtime::abilities::feedback::FeedbackAction;
    use chrono::TimeZone;

    fn ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn account(id: &str) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: "Example Account".to_string(),
            account_type: AccountType::Customer,
            updated_at: "2026-06-07T00:00:00Z".to_string(),
            ..Default::default()
        }
    }

    fn batch(item: TranscriptClaimItem) -> TranscriptClaimBatch {
        TranscriptClaimBatch {
            meeting_id: "meeting-transcript-1".to_string(),
            workspace_file_id: "wf_transcript_1".to_string(),
            workspace_file_kind: WorkspaceFileKind::GranolaTranscript,
            source_content_hash: "content-sha".to_string(),
            source_asof: Utc.with_ymd_and_hms(2026, 6, 7, 14, 0, 0).unwrap(),
            observed_at: Utc.with_ymd_and_hms(2026, 6, 7, 14, 5, 0).unwrap(),
            production_mode: TranscriptClaimProductionMode::FreshSingleTranscript,
            items: vec![item],
        }
    }

    fn win_item() -> TranscriptClaimItem {
        TranscriptClaimItem {
            kind: TranscriptClaimKind::Win,
            subject: TranscriptClaimSubject::Account {
                id: "acct-transcript".to_string(),
            },
            text: "Expansion pilot was approved".to_string(),
            field_path: None,
            topic_key: None,
            verified_quote: Some(VerifiedTranscriptQuote {
                text: "We are approving the expansion pilot".to_string(),
                start_char: 42,
                end_char: 81,
            }),
            source_item_ref: Some("phase2.win.0".to_string()),
            sensitivity: ClaimSensitivity::Internal,
        }
    }

    #[test]
    fn commits_transcript_claim_with_workspace_source_and_point_in_time_scope() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(11);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);

        let report = commit_transcript_claims(&ctx, &db, batch(win_item())).unwrap();
        assert_eq!(report.inserted, 1);
        assert_eq!(report.skipped_duplicates, 0);

        let (claim_type, field_path, source_ref, data_source, source_asof, temporal_scope, metadata_json, provenance_json): (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT claim_type, field_path, source_ref, data_source, source_asof, temporal_scope, metadata_json, provenance_json
                   FROM intelligence_claims
                  WHERE id = ?1",
                params![report.committed_claim_ids[0]],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(claim_type, "entity_win");
        assert_eq!(field_path, "transcript.wins");
        assert_eq!(source_ref, "workspace_file:wf_transcript_1");
        assert_eq!(data_source, "workspace_file:granola_transcript");
        assert_eq!(source_asof, "2026-06-07T14:00:00+00:00");
        assert_eq!(temporal_scope, "point_in_time");

        let metadata: serde_json::Value = serde_json::from_str(&metadata_json).unwrap();
        assert_eq!(metadata["producer"], "transcript_claims");
        assert_eq!(metadata["quote"]["verification"], "exact_match");
        assert_eq!(metadata["quote"]["redaction_policy"], "sensitivity_ceiling");

        let provenance: serde_json::Value = serde_json::from_str(&provenance_json).unwrap();
        assert_eq!(
            provenance["data_source"]["workspace_file"]["kind"],
            "granola_transcript"
        );
        assert_eq!(provenance["source_asof"], "2026-06-07T14:00:00Z");
    }

    #[test]
    fn skips_transcript_duplicate_before_commit_claim_reinforces() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(12);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);

        let first = commit_transcript_claims(&ctx, &db, batch(win_item())).unwrap();
        assert_eq!(first.inserted, 1);
        let second = commit_transcript_claims(&ctx, &db, batch(win_item())).unwrap();
        assert_eq!(second.inserted, 0);
        assert_eq!(second.skipped_duplicates, 1);
        assert_eq!(second.duplicate_claim_ids, first.committed_claim_ids);

        let claim_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM intelligence_claims WHERE claim_type = 'entity_win'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let corroboration_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM claim_corroborations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(claim_count, 1);
        assert_eq!(corroboration_count, 0);
    }

    #[test]
    fn skips_matching_claim_from_different_transcript_before_reinforcement() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(16);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);

        let first = commit_transcript_claims(&ctx, &db, batch(win_item())).unwrap();
        assert_eq!(first.inserted, 1);

        let mut second_batch = batch(win_item());
        second_batch.workspace_file_id = "wf_transcript_2".to_string();
        let second = commit_transcript_claims(&ctx, &db, second_batch).unwrap();

        assert_eq!(second.inserted, 0);
        assert_eq!(second.reinforced, 0);
        assert_eq!(second.skipped_duplicates, 1);
        assert_eq!(second.duplicate_claim_ids, first.committed_claim_ids);

        let claim_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM intelligence_claims WHERE claim_type = 'entity_win'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let corroboration_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM claim_corroborations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(claim_count, 1);
        assert_eq!(corroboration_count, 0);
    }

    #[test]
    fn distinct_transcripts_with_distinct_claims_keep_distinct_source_refs() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(17);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);

        let first = commit_transcript_claims(&ctx, &db, batch(win_item())).unwrap();
        assert_eq!(first.inserted, 1);

        let mut second_item = win_item();
        second_item.text = "Implementation sponsor confirmed rollout timing".to_string();
        second_item.verified_quote = Some(VerifiedTranscriptQuote {
            text: "We can start rollout next month".to_string(),
            start_char: 8,
            end_char: 39,
        });
        let mut second_batch = batch(second_item);
        second_batch.workspace_file_id = "wf_transcript_2".to_string();
        let second = commit_transcript_claims(&ctx, &db, second_batch).unwrap();
        assert_eq!(second.inserted, 1);
        assert_eq!(second.reinforced, 0);

        let source_refs = db
            .conn_ref()
            .prepare(
                "SELECT source_ref
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_win'
                  ORDER BY source_ref",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            source_refs,
            vec![
                "workspace_file:wf_transcript_1".to_string(),
                "workspace_file:wf_transcript_2".to_string()
            ]
        );
    }

    #[test]
    fn transcript_backed_claim_accepts_standard_claim_feedback() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(18);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);

        let report = commit_transcript_claims(&ctx, &db, batch(win_item())).unwrap();
        let claim_id = report.committed_claim_ids[0].clone();
        let feedback_ctx = ctx.with_actor("user:test");
        let outcome = record_claim_feedback(
            &feedback_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::CannotVerify,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: None,
            },
        )
        .unwrap();

        assert_eq!(outcome.claim_id, claim_id);
        assert_eq!(outcome.action, FeedbackAction::CannotVerify);
        let (feedback_count, job_count, verification_state): (i64, i64, String) = db
            .conn_ref()
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM claim_feedback WHERE claim_id = ?1),
                    (SELECT COUNT(*) FROM claim_feedback_propagation_jobs jobs
                       JOIN claim_feedback feedback ON feedback.id = jobs.feedback_id
                      WHERE feedback.claim_id = ?1),
                    (SELECT verification_state FROM intelligence_claims WHERE id = ?1)",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(feedback_count, 1);
        assert!(job_count > 0);
        assert_eq!(verification_state, "contested");
    }

    #[test]
    fn disabled_modes_do_not_write_claims() {
        for production_mode in [
            TranscriptClaimProductionMode::ProviderBackground,
            TranscriptClaimProductionMode::BulkBackfill,
        ] {
            let db = test_db();
            db.upsert_account(&account("acct-transcript")).unwrap();
            let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
            let rng = SeedableRng::new(13);
            let ext = ExternalClients::default();
            let ctx = ctx(&clock, &rng, &ext);
            let mut input = batch(win_item());
            input.production_mode = production_mode;

            let report = commit_transcript_claims(&ctx, &db, input).unwrap();
            assert_eq!(report.inserted, 0);
            assert_eq!(report.skipped_disabled_mode, 1);

            let claim_count: i64 = db
                .conn_ref()
                .query_row("SELECT COUNT(*) FROM intelligence_claims", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(claim_count, 0);
        }
    }

    #[test]
    fn explicit_reprocess_requires_preservation_manifest() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(14);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let mut input = batch(win_item());
        input.production_mode = TranscriptClaimProductionMode::ExplicitSingleMeetingReprocess {
            manifest: TranscriptReprocessManifest {
                manifest_id: "manifest-1".to_string(),
                reason: "manual retry".to_string(),
                prior_claims_preserved: true,
                tombstones_preserved: false,
                feedback_preserved: true,
                omitted_prior_claim_ids: Vec::new(),
            },
        };

        let err = commit_transcript_claims(&ctx, &db, input).unwrap_err();
        assert!(matches!(
            err,
            TranscriptClaimError::IncompleteReprocessManifest
        ));
    }

    #[test]
    fn explicit_reprocess_skips_duplicate_and_preserves_feedback() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(19);
        let ext = ExternalClients::default();
        let service_ctx = ctx(&clock, &rng, &ext);

        let first = commit_transcript_claims(&service_ctx, &db, batch(win_item())).unwrap();
        let claim_id = first.committed_claim_ids[0].clone();
        let feedback_ctx = ctx(&clock, &rng, &ext).with_actor("user:test");
        record_claim_feedback(
            &feedback_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::CannotVerify,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: None,
            },
        )
        .unwrap();

        let mut input = batch(win_item());
        input.production_mode = TranscriptClaimProductionMode::ExplicitSingleMeetingReprocess {
            manifest: TranscriptReprocessManifest {
                manifest_id: "manifest-complete".to_string(),
                reason: "manual retry".to_string(),
                prior_claims_preserved: true,
                tombstones_preserved: true,
                feedback_preserved: true,
                omitted_prior_claim_ids: Vec::new(),
            },
        };

        let second = commit_transcript_claims(&service_ctx, &db, input).unwrap();
        assert_eq!(second.inserted, 0);
        assert_eq!(second.reinforced, 0);
        assert_eq!(second.skipped_duplicates, 1);
        assert_eq!(second.duplicate_claim_ids, vec![claim_id.clone()]);

        let (claim_count, feedback_count, corroboration_count): (i64, i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM intelligence_claims WHERE claim_type = 'entity_win'),
                    (SELECT COUNT(*) FROM claim_feedback WHERE claim_id = ?1),
                    (SELECT COUNT(*) FROM claim_corroborations)",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(claim_count, 1);
        assert_eq!(feedback_count, 1);
        assert_eq!(corroboration_count, 0);
    }

    #[test]
    fn explicit_reprocess_does_not_resurrect_marked_false_claim() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(20);
        let ext = ExternalClients::default();
        let service_ctx = ctx(&clock, &rng, &ext);

        let first = commit_transcript_claims(&service_ctx, &db, batch(win_item())).unwrap();
        let claim_id = first.committed_claim_ids[0].clone();
        let feedback_ctx = ctx(&clock, &rng, &ext).with_actor("user:test");
        record_claim_feedback(
            &feedback_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::MarkFalse,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: None,
            },
        )
        .unwrap();

        let mut input = batch(win_item());
        input.production_mode = TranscriptClaimProductionMode::ExplicitSingleMeetingReprocess {
            manifest: TranscriptReprocessManifest {
                manifest_id: "manifest-mark-false".to_string(),
                reason: "manual retry".to_string(),
                prior_claims_preserved: true,
                tombstones_preserved: true,
                feedback_preserved: true,
                omitted_prior_claim_ids: Vec::new(),
            },
        };

        let second = commit_transcript_claims(&service_ctx, &db, input).unwrap();
        assert_eq!(second.inserted, 0);
        assert_eq!(second.reinforced, 0);
        assert_eq!(second.skipped_duplicates, 1);
        assert_eq!(second.duplicate_claim_ids, vec![claim_id.clone()]);

        let (claim_count, feedback_count, claim_state, surfacing_state): (
            i64,
            i64,
            String,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM intelligence_claims WHERE claim_type = 'entity_win'),
                    (SELECT COUNT(*) FROM claim_feedback WHERE claim_id = ?1),
                    (SELECT claim_state FROM intelligence_claims WHERE id = ?1),
                    (SELECT surfacing_state FROM intelligence_claims WHERE id = ?1)",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(claim_count, 1);
        assert_eq!(feedback_count, 1);
        assert_eq!(claim_state, "withdrawn");
        assert_eq!(surfacing_state, "dormant");
    }

    #[test]
    fn quote_metadata_requires_internal_bounded_quote() {
        let db = test_db();
        db.upsert_account(&account("acct-transcript")).unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(21);
        let ext = ExternalClients::default();
        let service_ctx = ctx(&clock, &rng, &ext);

        let mut oversized = win_item();
        oversized.text = "Large excerpt should not be quote metadata".to_string();
        oversized.verified_quote = Some(VerifiedTranscriptQuote {
            text: "a".repeat(TRANSCRIPT_VERIFIED_QUOTE_MAX_CHARS + 1),
            start_char: 0,
            end_char: TRANSCRIPT_VERIFIED_QUOTE_MAX_CHARS + 1,
        });
        let oversized_report =
            commit_transcript_claims(&service_ctx, &db, batch(oversized)).unwrap();
        let oversized_metadata: String = db
            .conn_ref()
            .query_row(
                "SELECT metadata_json FROM intelligence_claims WHERE id = ?1",
                params![oversized_report.committed_claim_ids[0]],
                |row| row.get(0),
            )
            .unwrap();
        let oversized_json: serde_json::Value = serde_json::from_str(&oversized_metadata).unwrap();
        assert_eq!(oversized_json["quote_verified"], false);
        assert!(oversized_json["quote"].is_null());

        let mut confidential = win_item();
        confidential.text = "Confidential quote should not be quote metadata".to_string();
        confidential.sensitivity = ClaimSensitivity::Confidential;
        confidential.verified_quote = Some(VerifiedTranscriptQuote {
            text: "Small exact quote".to_string(),
            start_char: 12,
            end_char: 29,
        });
        let mut confidential_batch = batch(confidential);
        confidential_batch.workspace_file_id = "wf_transcript_confidential".to_string();
        let confidential_report =
            commit_transcript_claims(&service_ctx, &db, confidential_batch).unwrap();
        let confidential_metadata: String = db
            .conn_ref()
            .query_row(
                "SELECT metadata_json FROM intelligence_claims WHERE id = ?1",
                params![confidential_report.committed_claim_ids[0]],
                |row| row.get(0),
            )
            .unwrap();
        let confidential_json: serde_json::Value =
            serde_json::from_str(&confidential_metadata).unwrap();
        assert_eq!(confidential_json["quote_verified"], false);
        assert!(confidential_json["quote"].is_null());
    }

    #[test]
    fn registers_transcript_workspace_source_lifecycle() {
        let db = test_db();
        let workspace = tempfile::tempdir().unwrap();
        let transcript_path = workspace.path().join("meeting.md");
        std::fs::write(&transcript_path, "Customer quote: expansion approved").unwrap();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 14, 10, 0).unwrap());
        let rng = SeedableRng::new(15);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);

        let source = ensure_transcript_workspace_source(
            &ctx,
            &db,
            TranscriptWorkspaceSourceInput {
                workspace_root: workspace.path(),
                file_path: &transcript_path,
                source_kind: WorkspaceFileKind::GenericTranscript,
                source_asof: Utc.with_ymd_and_hms(2026, 6, 7, 14, 0, 0).unwrap(),
                content: "Customer quote: expansion approved",
                entity: Some(TranscriptClaimSubject::Account {
                    id: "acct-transcript".to_string(),
                }),
            },
        )
        .unwrap();

        let (source_type, lifecycle_state, source_asof, content_sha256): (
            String,
            String,
            String,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT source_type, lifecycle_state, source_asof, content_sha256
                   FROM workspace_file_lifecycle
                  WHERE file_id = ?1",
                params![source.file_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(source_type, "generic_transcript");
        assert_eq!(lifecycle_state, "ingested");
        assert_eq!(source_asof, "2026-06-07T14:00:00.000Z");
        assert_eq!(content_sha256, source.content_sha256);
    }
}
