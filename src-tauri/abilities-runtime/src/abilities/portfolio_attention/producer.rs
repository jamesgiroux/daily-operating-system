//! `portfolio_attention` producer.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::abilities::get_entity_intelligence::contracts::CursorState;
use crate::abilities::list_pagination::{decode_cursor, encode_cursor, watermark_from_request};
use crate::abilities::portfolio_attention::contracts::{
    AttentionEvidence, AttentionReason, AttentionScore, PortfolioAttentionInput,
    PortfolioAttentionItem, PortfolioAttentionResult, PortfolioAttentionSubject,
    PORTFOLIO_ATTENTION_ABILITY_NAME, PORTFOLIO_ATTENTION_SCHEMA_VERSION,
};
use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::trust::claim_trust_band_from_score;
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, DataSource, EntityId, FieldAttribution, FieldPath,
    GleanDownstream, ProvenanceBuilder, ProvenanceBuilderConfig, SchemaVersion, SourceAttribution,
    SourceIdentifier, SourceName, SubjectAttribution, SubjectRef,
};
use crate::abilities::recommendations::contracts::{
    ClaimId, ScoreSalienceReadRequest, SCORE_SALIENCE_SCHEMA_VERSION,
};
use crate::abilities::trust::types::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::sensitivity::{renderable_claim_text_with_value, RenderActor, RenderSurface};
use crate::services::context::{
    AccountListQuery, AccountListReadError, AccountListSummary, ListOpenLoopsQuery,
    ListOpenLoopsReadError,
};
use crate::types::{subject_ref_from_json, ClaimSubjectRef, IntelligenceClaim};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 25;
const ACCOUNT_CANDIDATE_LIMIT: u32 = 200;
const MAX_EVIDENCE_PER_ITEM: usize = 4;

pub async fn portfolio_attention(
    ctx: &AbilityContext<'_>,
    input: PortfolioAttentionInput,
) -> AbilityResult<PortfolioAttentionResult> {
    validate_schema_version(input.schema_version)?;
    let entity_types = normalize_entity_types(input.entity_types.as_ref())?;
    let page_size = validate_page_size(input.page_size.or(input.limit))?;
    let watermark = watermark_from_request(&request_fingerprint(&entity_types, page_size));
    let offset = match cursor_offset(
        input.cursor.as_ref(),
        &watermark,
        input.schema_version,
        ctx,
    )
    .await?
    {
        CursorOffset::Offset(offset) => offset,
        CursorOffset::Invalidated(result) => return Ok(result),
    };

    let accounts = read_accounts(ctx).await?;
    let open_loop_claims = read_open_loop_claims(ctx).await?;
    let generated_at = ctx.services().clock.now();
    let mut candidates = BTreeMap::<SubjectKey, Candidate>::new();

    for account in accounts {
        if entity_types.contains("account") {
            let key = SubjectKey::new("account", account.account_id.clone());
            candidates
                .entry(key.clone())
                .or_insert_with(|| Candidate::from_account(key, account));
        }
    }

    for claim in open_loop_claims {
        for key in subject_keys_for_claim(&claim) {
            if !entity_types.contains(key.entity_type.as_str()) {
                continue;
            }
            let candidate = candidates
                .entry(key.clone())
                .or_insert_with(|| Candidate::from_subject(key.clone()));
            let salience = score_salience(ctx, &claim).await;
            candidate.add_claim(ctx, claim.clone(), salience)?;
        }
    }

    let mut items = candidates
        .into_values()
        .filter_map(|candidate| candidate.finish(generated_at))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .score
            .total
            .total_cmp(&left.score.total)
            .then_with(|| {
                right
                    .score
                    .open_loops
                    .total_cmp(&left.score.open_loops)
            })
            .then_with(|| {
                right
                    .score
                    .freshness
                    .total_cmp(&left.score.freshness)
            })
            .then_with(|| left.subject.entity_id.cmp(&right.subject.entity_id))
    });
    let total_hint = items.len() as u64;
    let page_start = offset.min(items.len() as u64) as usize;
    let page_end = page_start
        .saturating_add(page_size as usize)
        .min(items.len());
    let page_items = if page_start >= items.len() {
        Vec::new()
    } else {
        items[page_start..page_end].to_vec()
    };
    let consumed = offset.saturating_add(page_items.len() as u64);
    let next_cursor = if consumed < total_hint {
        Some(encode_cursor(consumed, &watermark))
    } else {
        None
    };
    let mut items = page_items;
    for (index, item) in items.iter_mut().enumerate() {
        item.rank = u32::try_from(offset as usize + index + 1).unwrap_or(u32::MAX);
    }

    finalize_output(
        ctx,
        input.schema_version,
        PortfolioAttentionResult {
            schema_version: SchemaVersion(PORTFOLIO_ATTENTION_SCHEMA_VERSION),
            generated_at: generated_at.to_rfc3339(),
            items,
            next_cursor,
            total_hint,
            cursor_state: CursorState::Stable,
        },
    )
}

enum CursorOffset {
    Offset(u64),
    Invalidated(crate::abilities::provenance::AbilityOutput<PortfolioAttentionResult>),
}

async fn cursor_offset(
    cursor: Option<&crate::abilities::get_entity_intelligence::contracts::Cursor>,
    watermark: &str,
    schema_version: u32,
    ctx: &AbilityContext<'_>,
) -> Result<CursorOffset, AbilityError> {
    let Some(cursor) = cursor else {
        return Ok(CursorOffset::Offset(0));
    };
    let Some(payload) = decode_cursor(cursor) else {
        let result = invalidated_result(
            ctx,
            schema_version,
            "cursor token malformed",
            ctx.services().clock.now(),
        )?;
        return Ok(CursorOffset::Invalidated(result));
    };
    if payload.watermark != watermark {
        let result = invalidated_result(
            ctx,
            schema_version,
            "filter or page_size changed since cursor was issued",
            ctx.services().clock.now(),
        )?;
        return Ok(CursorOffset::Invalidated(result));
    }
    Ok(CursorOffset::Offset(payload.offset))
}

fn invalidated_result(
    ctx: &AbilityContext<'_>,
    schema_version: u32,
    reason: impl Into<String>,
    generated_at: DateTime<Utc>,
) -> AbilityResult<PortfolioAttentionResult> {
    finalize_output(
        ctx,
        schema_version,
        PortfolioAttentionResult {
            schema_version: SchemaVersion(PORTFOLIO_ATTENTION_SCHEMA_VERSION),
            generated_at: generated_at.to_rfc3339(),
            items: Vec::new(),
            next_cursor: None,
            total_hint: 0,
            cursor_state: CursorState::Invalidated {
                reason: reason.into(),
                restart_required: true,
            },
        },
    )
}

async fn read_accounts(ctx: &AbilityContext<'_>) -> Result<Vec<AccountListSummary>, AbilityError> {
    let snapshot = ctx
        .services()
        .read_list_accounts(AccountListQuery {
            status: None,
            health_band: None,
            name_contains: None,
            offset: 0,
            page_size: ACCOUNT_CANDIDATE_LIMIT,
        })
        .await
        .map_err(account_read_error)?;
    Ok(snapshot.items)
}

async fn read_open_loop_claims(
    ctx: &AbilityContext<'_>,
) -> Result<Vec<IntelligenceClaim>, AbilityError> {
    let snapshot = ctx
        .services()
        .read_list_open_loops(ListOpenLoopsQuery {
            entity_type: None,
            entity_id: None,
            surface: ctx.entity_context_claim_surface(),
        })
        .await
        .map_err(open_loop_read_error)?;
    Ok(snapshot.claims)
}

async fn score_salience(ctx: &AbilityContext<'_>, claim: &IntelligenceClaim) -> Option<f64> {
    ctx.services()
        .score_salience(ScoreSalienceReadRequest {
            schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
            claim_id: ClaimId(claim.id.clone()),
            actor: ctx.actor.kind(),
        })
        .await
        .ok()
        .map(|response| response.salience.total.clamp(0.0, 1.0))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SubjectKey {
    entity_type: String,
    entity_id: String,
}

impl SubjectKey {
    fn new(entity_type: impl Into<String>, entity_id: impl Into<String>) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
        }
    }
}

struct Candidate {
    key: SubjectKey,
    label: Option<String>,
    health_band: Option<TrustBand>,
    last_touchpoint_at: Option<String>,
    evidence: Vec<CandidateEvidence>,
}

impl Candidate {
    fn from_account(key: SubjectKey, account: AccountListSummary) -> Self {
        Self {
            key,
            label: Some(account.name),
            health_band: Some(account.health_band),
            last_touchpoint_at: account.last_touchpoint_at,
            evidence: Vec::new(),
        }
    }

    fn from_subject(key: SubjectKey) -> Self {
        Self {
            key,
            label: None,
            health_band: None,
            last_touchpoint_at: None,
            evidence: Vec::new(),
        }
    }

    fn add_claim(
        &mut self,
        ctx: &AbilityContext<'_>,
        claim: IntelligenceClaim,
        salience: Option<f64>,
    ) -> Result<(), AbilityError> {
        let Some(renderable) = renderable_claim_text_with_value(
            &claim,
            &claim.text,
            render_surface_for_context(ctx),
            &render_actor_for_context(ctx),
        ) else {
            return Ok(());
        };
        if renderable.text.trim().is_empty() {
            return Ok(());
        }

        self.evidence.push(CandidateEvidence {
            text: renderable.text,
            claim,
            salience,
        });
        Ok(())
    }

    fn finish(mut self, now: DateTime<Utc>) -> Option<PortfolioAttentionItem> {
        self.evidence.sort_by(|left, right| {
            right
                .salience
                .unwrap_or(0.0)
                .total_cmp(&left.salience.unwrap_or(0.0))
                .then_with(|| {
                    right
                        .evidence_at(now)
                        .cmp(&left.evidence_at(now))
                })
                .then_with(|| left.claim.id.cmp(&right.claim.id))
        });

        let open_loop_count = self.evidence.len();
        let risk = risk_score(self.health_band);
        let open_loops = (open_loop_count as f64 / 3.0).clamp(0.0, 1.0);
        let freshness = freshness_score(&self, now);
        let trust = trust_score(&self);
        let salience = average_salience(&self.evidence);

        if open_loop_count == 0 && risk < 0.5 && freshness < 0.6 {
            return None;
        }

        let total = weighted_total(risk, open_loops, freshness, trust, salience);
        let mut reasons = Vec::new();
        if open_loop_count > 0 {
            reasons.push(AttentionReason {
                kind: "open_loop".to_string(),
                summary: format!("{open_loop_count} active open loop(s) need follow-up"),
                weight: open_loops,
            });
        }
        if let Some(health_band) = self.health_band {
            if !matches!(health_band, TrustBand::LikelyCurrent) {
                reasons.push(AttentionReason {
                    kind: "account_health".to_string(),
                    summary: format!("Account health/trust band is {health_band:?}"),
                    weight: risk,
                });
            }
        }
        if touchpoint_is_stale(self.last_touchpoint_at.as_deref(), now) {
            reasons.push(AttentionReason {
                kind: "stale_touchpoint".to_string(),
                summary: "No recent touchpoint is available in the account index".to_string(),
                weight: freshness,
            });
        }
        if salience.unwrap_or(0.0) >= 0.65 {
            reasons.push(AttentionReason {
                kind: "salience".to_string(),
                summary: "Claim salience is high for the current context".to_string(),
                weight: salience.unwrap_or(0.0),
            });
        }

        let trust_band = trust_band_for_candidate(&self);
        let evidence = self
            .evidence
            .into_iter()
            .take(MAX_EVIDENCE_PER_ITEM)
            .map(|entry| AttentionEvidence {
                text: entry.text,
                claim_type: entry.claim.claim_type.clone(),
                source_type: entry.claim.data_source.clone(),
                source_asof: entry.claim.source_asof.clone(),
                observed_at: entry.claim.observed_at.clone(),
                trust_band: claim_trust_band_from_score(entry.claim.trust_score),
                salience: entry.salience,
            })
            .collect::<Vec<_>>();

        Some(PortfolioAttentionItem {
            rank: 0,
            subject: PortfolioAttentionSubject {
                entity_type: self.key.entity_type,
                entity_id: self.key.entity_id,
                label: self.label,
            },
            score: AttentionScore {
                total,
                risk,
                open_loops,
                freshness,
                trust,
                salience,
            },
            trust_band,
            reasons,
            evidence,
        })
    }
}

struct CandidateEvidence {
    text: String,
    claim: IntelligenceClaim,
    salience: Option<f64>,
}

impl CandidateEvidence {
    fn evidence_at(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        parsed_claim_timestamp(&self.claim, now).0
    }
}

fn subject_keys_for_claim(claim: &IntelligenceClaim) -> Vec<SubjectKey> {
    let value = match serde_json::from_str::<serde_json::Value>(&claim.subject_ref) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let parsed = match subject_ref_from_json(&value) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };
    let mut keys = Vec::new();
    collect_subject_keys(&parsed, &mut keys);
    keys
}

fn collect_subject_keys(subject: &ClaimSubjectRef, keys: &mut Vec<SubjectKey>) {
    match subject {
        ClaimSubjectRef::Account { id } => keys.push(SubjectKey::new("account", id.clone())),
        ClaimSubjectRef::Project { id } => keys.push(SubjectKey::new("project", id.clone())),
        ClaimSubjectRef::Person { id } => keys.push(SubjectKey::new("person", id.clone())),
        ClaimSubjectRef::Meeting { id } => keys.push(SubjectKey::new("meeting", id.clone())),
        ClaimSubjectRef::Multi(subjects) => {
            for subject in subjects {
                collect_subject_keys(subject, keys);
            }
        }
        ClaimSubjectRef::Email { .. } | ClaimSubjectRef::Global => {}
    }
}

fn weighted_total(
    risk: f64,
    open_loops: f64,
    freshness: f64,
    trust: f64,
    salience: Option<f64>,
) -> f64 {
    let salience = salience.unwrap_or(0.0);
    ((risk * 0.25) + (open_loops * 0.30) + (freshness * 0.20) + (trust * 0.10) + (salience * 0.15))
        .clamp(0.0, 1.0)
}

fn risk_score(health_band: Option<TrustBand>) -> f64 {
    match health_band.unwrap_or(TrustBand::Unscored) {
        TrustBand::NeedsVerification => 1.0,
        TrustBand::UseWithCaution => 0.65,
        TrustBand::Unscored => 0.35,
        TrustBand::LikelyCurrent => 0.15,
    }
}

fn freshness_score(candidate: &Candidate, now: DateTime<Utc>) -> f64 {
    let latest_evidence = candidate
        .evidence
        .iter()
        .map(|entry| entry.evidence_at(now))
        .max()
        .map(|timestamp| recency_score(timestamp, now))
        .unwrap_or(0.0);
    let stale_touchpoint = touchpoint_staleness_score(candidate.last_touchpoint_at.as_deref(), now);
    latest_evidence.max(stale_touchpoint)
}

fn recency_score(timestamp: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
    let age_days = (now - timestamp).num_days();
    if age_days <= 7 {
        1.0
    } else if age_days <= 30 {
        0.8
    } else if age_days <= 90 {
        0.5
    } else {
        0.25
    }
}

fn touchpoint_staleness_score(value: Option<&str>, now: DateTime<Utc>) -> f64 {
    let Some(value) = value else {
        return 0.35;
    };
    let Some(timestamp) = parse_optional_timestamp(Some(value), now) else {
        return 0.35;
    };
    let age_days = (now - timestamp).num_days();
    if age_days >= 90 {
        1.0
    } else if age_days >= 45 {
        0.7
    } else {
        0.0
    }
}

fn touchpoint_is_stale(value: Option<&str>, now: DateTime<Utc>) -> bool {
    touchpoint_staleness_score(value, now) >= 0.7
}

fn trust_score(candidate: &Candidate) -> f64 {
    let mut values = candidate
        .evidence
        .iter()
        .filter_map(|entry| entry.claim.trust_score)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    if values.is_empty() {
        return match candidate.health_band.unwrap_or(TrustBand::Unscored) {
            TrustBand::LikelyCurrent => 0.9,
            TrustBand::UseWithCaution => 0.65,
            TrustBand::NeedsVerification => 0.35,
            TrustBand::Unscored => 0.45,
        };
    }
    values.sort_by(f64::total_cmp);
    values.iter().sum::<f64>() / values.len() as f64
}

fn trust_band_for_candidate(candidate: &Candidate) -> TrustBand {
    let mut bands = candidate
        .evidence
        .iter()
        .map(|entry| claim_trust_band_from_score(entry.claim.trust_score))
        .collect::<Vec<_>>();
    if let Some(health_band) = candidate.health_band {
        bands.push(health_band);
    }
    if bands.iter().any(|band| *band == TrustBand::NeedsVerification) {
        TrustBand::NeedsVerification
    } else if bands.iter().any(|band| *band == TrustBand::UseWithCaution) {
        TrustBand::UseWithCaution
    } else if bands.iter().any(|band| *band == TrustBand::LikelyCurrent) {
        TrustBand::LikelyCurrent
    } else {
        TrustBand::Unscored
    }
}

fn average_salience(evidence: &[CandidateEvidence]) -> Option<f64> {
    let values = evidence
        .iter()
        .filter_map(|entry| entry.salience)
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some((values.iter().sum::<f64>() / values.len() as f64).clamp(0.0, 1.0))
    }
}

fn validate_schema_version(schema_version: u32) -> Result<(), AbilityError> {
    if schema_version == PORTFOLIO_ATTENTION_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{PORTFOLIO_ATTENTION_ABILITY_NAME}`"
        )))
    }
}

fn validate_page_size(page_size: Option<u32>) -> Result<u32, AbilityError> {
    let page_size = page_size.unwrap_or(DEFAULT_LIMIT);
    if page_size == 0 {
        Ok(DEFAULT_LIMIT)
    } else if page_size > MAX_LIMIT {
        Err(validation_error(format!(
            "page_size `{page_size}` exceeds MAX_LIMIT `{MAX_LIMIT}`"
        )))
    } else {
        Ok(page_size)
    }
}

fn normalize_entity_types(values: Option<&Vec<String>>) -> Result<BTreeSet<String>, AbilityError> {
    let Some(values) = values else {
        return Ok(BTreeSet::from(["account".to_string()]));
    };
    let mut normalized = BTreeSet::new();
    for value in values {
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Err(validation_error("entity_types cannot contain empty values"));
        }
        match value.as_str() {
            "account" | "project" | "person" | "meeting" => {
                normalized.insert(value);
            }
            other => {
                return Err(validation_error(format!(
                    "unsupported portfolio attention entity_type `{other}`"
                )));
            }
        }
    }
    if normalized.is_empty() {
        return Err(validation_error("entity_types must include at least one value"));
    }
    Ok(normalized)
}

fn request_fingerprint(entity_types: &BTreeSet<String>, page_size: u32) -> String {
    json!({
        "entity_types": entity_types,
        "page_size": page_size,
        "ability": PORTFOLIO_ATTENTION_ABILITY_NAME,
    })
    .to_string()
}

fn finalize_output(
    ctx: &AbilityContext<'_>,
    schema_version: u32,
    output: PortfolioAttentionResult,
) -> AbilityResult<PortfolioAttentionResult> {
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, schema_version));
    let subject_ref = if output.items.is_empty() {
        SubjectRef::Global
    } else {
        SubjectRef::Multi(
            output
                .items
                .iter()
                .map(|item| subject_ref_for_item(&item.subject))
                .collect(),
        )
    };
    let subject = SubjectAttribution::direct_confident(subject_ref);
    builder.set_subject(subject.clone());
    builder
        .attribute(
            FieldPath::new("/schemaVersion").map_err(field_error)?,
            FieldAttribution::constant(subject.clone()),
        )
        .map_err(provenance_error)?;
    builder
        .attribute(
            FieldPath::new("/generatedAt").map_err(field_error)?,
            FieldAttribution::constant(subject.clone()),
        )
        .map_err(provenance_error)?;
    builder
        .attribute(
            FieldPath::new("/nextCursor").map_err(field_error)?,
            FieldAttribution::constant(subject.clone()),
        )
        .map_err(provenance_error)?;
    builder
        .attribute(
            FieldPath::new("/totalHint").map_err(field_error)?,
            FieldAttribution::constant(subject.clone()),
        )
        .map_err(provenance_error)?;
    builder
        .attribute(
            FieldPath::new("/cursorState").map_err(field_error)?,
            FieldAttribution::constant(subject.clone()),
        )
        .map_err(provenance_error)?;

    if output.items.is_empty() {
        builder
            .attribute(
                FieldPath::new("/items").map_err(field_error)?,
                FieldAttribution::constant(subject),
            )
            .map_err(provenance_error)?;
    } else {
        for (index, item) in output.items.iter().enumerate() {
            let item_subject = subject_ref_for_item(&item.subject);
            let subject_attr = SubjectAttribution::direct_confident(item_subject);
            if let Some(evidence) = item.evidence.first() {
                let source_index = builder.add_source(source_for_evidence(ctx, evidence)?);
                builder.set_source_trust_band(source_index, evidence.trust_band);
                builder
                    .attribute_subtree(
                        FieldPath::new(format!("/items/{index}")).map_err(field_error)?,
                        FieldAttribution::direct(subject_attr, source_index),
                    )
                    .map_err(provenance_error)?;
            } else {
                builder
                    .attribute_subtree(
                        FieldPath::new(format!("/items/{index}")).map_err(field_error)?,
                        FieldAttribution::constant(subject_attr),
                    )
                    .map_err(provenance_error)?;
            }
        }
    }

    builder.finalize(output).map_err(provenance_error)
}

fn subject_ref_for_item(subject: &PortfolioAttentionSubject) -> SubjectRef {
    match subject.entity_type.as_str() {
        "account" => SubjectRef::Account(subject.entity_id.clone()),
        "project" => SubjectRef::Project(subject.entity_id.clone()),
        "person" => SubjectRef::Person(subject.entity_id.clone()),
        "meeting" => SubjectRef::Meeting(subject.entity_id.clone()),
        _ => SubjectRef::Unknown,
    }
}

fn source_for_evidence(
    ctx: &AbilityContext<'_>,
    evidence: &AttentionEvidence,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let observed_at = parse_optional_timestamp(Some(&evidence.observed_at), now).unwrap_or(now);
    let source_asof = parse_optional_timestamp(evidence.source_asof.as_deref(), now);
    SourceAttribution::new(
        data_source_for_claim(&evidence.source_type),
        vec![SourceIdentifier::Entity {
            entity_id: EntityId::new(evidence.claim_type.clone()),
            field: Some(evidence.claim_type.clone()),
        }],
        observed_at,
        source_asof,
        1.0,
        None,
    )
    .map_err(|error| validation_error(format!("invalid source attribution: {error}")))
}

fn data_source_for_claim(value: &str) -> DataSource {
    match value.trim().to_ascii_lowercase().as_str() {
        "user" | "human" | "manual" => DataSource::User,
        "google" => DataSource::Google,
        "glean" => DataSource::Glean {
            downstream: GleanDownstream::Documents,
        },
        "glean_salesforce" | "salesforce" | "sfdc" => DataSource::Glean {
            downstream: GleanDownstream::Salesforce,
        },
        "glean_zendesk" | "zendesk" => DataSource::Glean {
            downstream: GleanDownstream::Zendesk,
        },
        "glean_gong" | "gong" => DataSource::Glean {
            downstream: GleanDownstream::Gong,
        },
        "glean_slack" | "slack" => DataSource::Glean {
            downstream: GleanDownstream::Slack,
        },
        "glean_p2" | "p2" => DataSource::Glean {
            downstream: GleanDownstream::P2,
        },
        "ai" | "agent" => DataSource::Ai,
        "local_enrichment" => DataSource::LocalEnrichment,
        "legacy_unattributed" => DataSource::LegacyUnattributed,
        other => DataSource::Other(SourceName::new(other)),
    }
}

fn parsed_claim_timestamp(
    claim: &IntelligenceClaim,
    now: DateTime<Utc>,
) -> (DateTime<Utc>, Option<DateTime<Utc>>) {
    let source_asof = parse_optional_timestamp(claim.source_asof.as_deref(), now);
    for candidate in [claim.observed_at.as_str(), claim.created_at.as_str()] {
        if let Some(parsed) = parse_optional_timestamp(Some(candidate), now) {
            return (parsed, source_asof);
        }
    }
    (now, source_asof)
}

fn parse_optional_timestamp(candidate: Option<&str>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match parse_source_timestamp(candidate, now, None) {
        SourceTimestampStatus::Accepted(parsed)
        | SourceTimestampStatus::Implausible { parsed, .. } => Some(parsed),
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    }
}

fn render_surface_for_context(ctx: &AbilityContext<'_>) -> RenderSurface {
    if ctx.entity_context_claim_surface() == crate::sensitivity::ClaimDismissalSurface::McpTool {
        RenderSurface::McpTool
    } else {
        RenderSurface::TauriEntityDetail
    }
}

fn render_actor_for_context(ctx: &AbilityContext<'_>) -> RenderActor {
    match &ctx.actor {
        Actor::User | Actor::SurfaceClient { .. } => RenderActor::user("user", None::<String>),
        Actor::McpClient { .. } | Actor::Agent => RenderActor::agent("agent:portfolio_attention"),
        Actor::System => RenderActor::agent("system:portfolio_attention"),
        Actor::Admin => RenderActor::user("admin", None::<String>),
    }
}

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config =
        ProvenanceBuilderConfig::new(PORTFOLIO_ATTENTION_ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".to_string(),
            version: "unknown".to_string(),
        },
        Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::System {
            component: "surface_client".to_string(),
        },
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
            name: "mcp".to_string(),
            version: "unknown".to_string(),
        },
    }
}

fn account_read_error(error: AccountListReadError) -> AbilityError {
    match error {
        AccountListReadError::ReadFailed(message) => {
            hard_error("portfolio_attention_account_read_failed", message)
        }
    }
}

fn open_loop_read_error(error: ListOpenLoopsReadError) -> AbilityError {
    match error {
        ListOpenLoopsReadError::SubjectNotOwned {
            entity_type,
            entity_id,
        } => AbilityError {
            kind: AbilityErrorKind::SubjectNotOwned,
            message: format!("subject is not owned by this workspace: {entity_type}:{entity_id}"),
        },
        ListOpenLoopsReadError::ReadFailed(message) => {
            hard_error("portfolio_attention_open_loop_read_failed", message)
        }
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn hard_error(code: impl Into<String>, message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError(code.into()),
        message: message.into(),
    }
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("invalid field path: {error}"))
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("provenance construction failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;
    use crate::abilities::registry::{AbilityRegistry, McpClientId, OpaqueConversationHandle};
    use crate::abilities::recommendations::contracts::{
        SaliencePersistence, SalienceScore, ScoreSalienceResponse,
    };
    use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
        ProviderError, ProviderKind,
    };
    use crate::sensitivity::ClaimDismissalSurface;
    use crate::services::context::{
        AccountListReadFuture, AccountListReadHandle, AccountListSnapshot, FixedClock,
        ListOpenLoopsReadFuture, ListOpenLoopsReadHandle, ListOpenLoopsSnapshot, SalienceReadFuture,
        SalienceReadHandle, SeedableRng, ServiceContext,
    };
    use crate::types::{ClaimSensitivity, ClaimState, SurfacingState, TemporalScope};

    struct StaticProvider;

    #[async_trait]
    impl IntelligenceProvider for StaticProvider {
        async fn complete(
            &self,
            _prompt: PromptInput,
            _tier: ModelTier,
        ) -> Result<Completion, ProviderError> {
            Ok(Completion {
                text: String::new(),
                fingerprint_metadata: FingerprintMetadata {
                    provider: ProviderKind::Other("test"),
                    model: ModelName::new("unused"),
                    temperature: 0.0,
                    top_p: None,
                    seed: None,
                    tokens_input: None,
                    tokens_output: None,
                    provider_completion_id: None,
                },
            })
        }

        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::Other("test")
        }

        fn current_model(&self, _tier: ModelTier) -> ModelName {
            ModelName::new("unused")
        }
    }

    struct AccountsReader(Vec<AccountListSummary>);

    impl AccountListReadHandle for AccountsReader {
        fn read_accounts<'a>(&'a self, _query: AccountListQuery) -> AccountListReadFuture<'a> {
            Box::pin(async move {
                Ok(AccountListSnapshot {
                    items: self.0.clone(),
                    total_after_filter: self.0.len() as u64,
                    data_shifted_advisory: None,
                })
            })
        }
    }

    struct OpenLoopsReader(Vec<IntelligenceClaim>);

    impl ListOpenLoopsReadHandle for OpenLoopsReader {
        fn read_open_loops<'a>(
            &'a self,
            _query: ListOpenLoopsQuery,
        ) -> ListOpenLoopsReadFuture<'a> {
            Box::pin(async move {
                Ok(ListOpenLoopsSnapshot {
                    claims: self.0.clone(),
                })
            })
        }
    }

    struct SalienceReader(Mutex<BTreeMap<String, f64>>);

    impl SalienceReadHandle for SalienceReader {
        fn score_salience<'a>(
            &'a self,
            request: ScoreSalienceReadRequest,
        ) -> SalienceReadFuture<'a> {
            Box::pin(async move {
                let total = self
                    .0
                    .lock()
                    .expect("salience lock")
                    .get(&request.claim_id.0)
                    .copied()
                    .unwrap_or(0.0);
                Ok(ScoreSalienceResponse {
                    schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                    claim_id: request.claim_id,
                    computed_at: Utc
                        .with_ymd_and_hms(2026, 5, 24, 12, 0, 0)
                        .unwrap()
                        .to_rfc3339(),
                    persistence: SaliencePersistence::Preview,
                    salience: SalienceScore {
                        total,
                        factors: Vec::new(),
                    },
                })
            })
        }
    }

    async fn invoke(
        accounts: Vec<AccountListSummary>,
        claims: Vec<IntelligenceClaim>,
        salience: BTreeMap<String, f64>,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(11);
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_actor("test")
            .with_account_list_reader(Arc::new(AccountsReader(accounts)))
            .with_list_open_loops_reader(Arc::new(OpenLoopsReader(claims)))
            .with_salience_reader(Arc::new(SalienceReader(Mutex::new(salience))));
        let provider = StaticProvider;
        let ctx = crate::abilities::AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::McpClient {
                client_id: McpClientId::new("test-mcp"),
                conversation_handle: Some(OpaqueConversationHandle::new("conversation")),
            },
            None,
            ClaimDismissalSurface::McpTool,
        );
        registry
            .invoke_by_name_json(&ctx, PORTFOLIO_ATTENTION_ABILITY_NAME, input)
            .await
    }

    fn account(id: &str, name: &str, health_band: TrustBand) -> AccountListSummary {
        AccountListSummary {
            account_id: id.to_string(),
            name: name.to_string(),
            status: "active".to_string(),
            health_band,
            last_touchpoint_at: Some("2026-05-20T12:00:00Z".to_string()),
            open_loops_count: 0,
        }
    }

    fn open_loop_claim(id: &str, account_id: &str, text: &str, source_asof: &str) -> IntelligenceClaim {
        IntelligenceClaim {
            id: id.to_string(),
            claim_version: 1,
            subject_ref: json!({ "kind": "account", "id": account_id }).to_string(),
            claim_type: "open_loop".to_string(),
            field_path: Some("open_loop".to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key: format!("dedup:{id}"),
            item_hash: None,
            actor: "system".to_string(),
            data_source: "local_enrichment".to_string(),
            source_ref: None,
            source_asof: Some(source_asof.to_string()),
            observed_at: source_asof.to_string(),
            created_at: source_asof.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: Some(0.88),
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: crate::sensitivity::ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    #[tokio::test]
    async fn ranks_claim_backed_attention_above_healthy_empty_accounts() {
        let accounts = vec![
            account("acct-stable", "Stable Account", TrustBand::LikelyCurrent),
            account("acct-risk", "Risk Account", TrustBand::UseWithCaution),
        ];
        let claims = vec![open_loop_claim(
            "claim-1",
            "acct-risk",
            "Follow up on unresolved launch commitment.",
            "2026-05-23T12:00:00Z",
        )];
        let salience = BTreeMap::from([("claim-1".to_string(), 0.9)]);

        let output = invoke(
            accounts,
            claims,
            salience,
            json!({ "schemaVersion": 1, "limit": 10 }),
        )
        .await
        .expect("portfolio attention output");

        let data = &output["data"];
        assert_eq!(data["items"].as_array().expect("items").len(), 1);
        assert_eq!(data["items"][0]["subject"]["entityId"], "acct-risk");
        assert_eq!(data["items"][0]["subject"]["label"], "Risk Account");
        assert_eq!(
            data["items"][0]["evidence"][0]["text"],
            "Follow up on unresolved launch commitment."
        );
        assert_eq!(data["items"][0]["evidence"][0]["salience"], 0.9);
    }

    #[tokio::test]
    async fn supports_non_account_subjects_when_requested() {
        let mut claim = open_loop_claim(
            "claim-project",
            "acct-ignored",
            "Project milestone needs owner confirmation.",
            "2026-05-22T12:00:00Z",
        );
        claim.subject_ref = json!({ "kind": "project", "id": "project-1" }).to_string();

        let output = invoke(
            Vec::new(),
            vec![claim],
            BTreeMap::new(),
            json!({ "schemaVersion": 1, "entityTypes": ["project"], "limit": 5 }),
        )
        .await
        .expect("portfolio attention output");

        let data = &output["data"];
        assert_eq!(data["items"][0]["subject"]["entityType"], "project");
        assert_eq!(data["items"][0]["subject"]["entityId"], "project-1");
    }

    #[tokio::test]
    async fn paginates_ranked_attention_with_opaque_cursor() {
        let accounts = vec![
            account("acct-one", "Account One", TrustBand::UseWithCaution),
            account("acct-two", "Account Two", TrustBand::UseWithCaution),
            account("acct-three", "Account Three", TrustBand::UseWithCaution),
        ];
        let claims = vec![
            open_loop_claim(
                "claim-one",
                "acct-one",
                "First ranked follow-up.",
                "2026-05-23T12:00:00Z",
            ),
            open_loop_claim(
                "claim-two",
                "acct-two",
                "Second ranked follow-up.",
                "2026-05-22T12:00:00Z",
            ),
            open_loop_claim(
                "claim-three",
                "acct-three",
                "Third ranked follow-up.",
                "2026-05-21T12:00:00Z",
            ),
        ];
        let salience = BTreeMap::from([
            ("claim-one".to_string(), 0.9),
            ("claim-two".to_string(), 0.8),
            ("claim-three".to_string(), 0.7),
        ]);

        let first = invoke(
            accounts.clone(),
            claims.clone(),
            salience.clone(),
            json!({ "schemaVersion": 1, "pageSize": 1 }),
        )
        .await
        .expect("first page");
        let first_data = &first["data"];
        assert_eq!(first_data["items"].as_array().expect("items").len(), 1);
        assert_eq!(first_data["items"][0]["rank"], 1);
        assert_eq!(first_data["items"][0]["subject"]["entityId"], "acct-one");
        let cursor = first_data["nextCursor"].as_str().expect("next cursor");

        let second = invoke(
            accounts,
            claims,
            salience,
            json!({ "schemaVersion": 1, "pageSize": 1, "cursor": cursor }),
        )
        .await
        .expect("second page");
        let second_data = &second["data"];
        assert_eq!(second_data["items"].as_array().expect("items").len(), 1);
        assert_eq!(second_data["items"][0]["rank"], 2);
        assert_eq!(second_data["items"][0]["subject"]["entityId"], "acct-two");
        assert_eq!(second_data["cursorState"]["kind"], "stable");
    }
}
