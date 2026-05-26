//! Surface-safe recommendation render projection.
//!
//! This module turns W2-A `surfacing_decisions` rows into the
//! `list_suggested_next_steps` ability response. The projection reads the
//! denormalized surfacing row for ranking, then rehydrates only the fields the
//! surface needs from the recommendation claim and Shared Receipt DTO.

use std::collections::HashMap;

use abilities_runtime::abilities::claim_receipt as runtime_receipt;
use abilities_runtime::abilities::provenance::subject::SubjectRef;
use abilities_runtime::abilities::provenance::trust::claim_trust_band_from_score;
use abilities_runtime::abilities::recommendations::contracts as runtime;
use abilities_runtime::abilities::registry::ActorKind;
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::sensitivity::{
    renderable_claim_text_with_value, RenderActor, RenderSurface,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension};

use super::contracts as app;
use crate::db::ActionDb;
use crate::services::claim_receipt::contracts as app_receipt;
use crate::services::claim_receipt::privacy::{build_receipt_for_audience, Audience, PrivacyError};
use crate::services::claim_receipt::render::audience_for_surface;
use crate::state::AppState;

const DEFAULT_MAX_ITEMS: u8 = 5;
const MAX_ITEMS_CEILING: u8 = 8;
const DECIDED_ECHO_WINDOW_SECS: i64 = 30;
const SURFACING_FETCH_MULTIPLIER: usize = 4;

#[derive(Debug, Clone)]
pub struct RecommendationRenderPayload {
    pub claim_id: app::ClaimId,
    pub subject: SubjectRef,
    pub headline: String,
    pub field_path: Option<String>,
    pub recommended_action: app::RecommendedAction,
    pub trust_band: TrustBand,
    pub evidence: Vec<app::EvidenceRef>,
    pub feedback_state: app::FeedbackState,
    pub conversion_state: app::ConversionState,
    pub salience: Option<app::SalienceScore>,
}

#[derive(Debug, Clone)]
struct SurfacingRenderRow {
    claim_id: app::ClaimId,
    why_this_now: app::WhyThisNow,
    trigger_refs: Vec<app::TriggerRef>,
    salience_evaluation_id: String,
}

#[derive(Debug, Clone)]
struct ProjectionCandidate {
    row: SurfacingRenderRow,
    payload: RecommendationRenderPayload,
    primary_rationale: Option<app::FactorRationale>,
}

#[derive(Debug, Clone)]
struct ReceiptBatchRow {
    claim_id: String,
    subject: SubjectRef,
    field_path: Option<String>,
}

pub async fn list_suggested_next_steps_projection(
    state: &AppState,
    input: runtime::ListSuggestedNextStepsInput,
    actor: ActorKind,
) -> Result<runtime::ListSuggestedNextStepsResponse> {
    if input.schema_version != runtime::LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION {
        bail!(
            "unsupported schema_version `{}` for `{}`",
            input.schema_version,
            runtime::LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME
        );
    }

    let max_items = usize::from(
        input
            .max_items
            .unwrap_or(DEFAULT_MAX_ITEMS)
            .min(MAX_ITEMS_CEILING),
    );
    let generated_at = Utc::now();
    if max_items == 0 {
        return Ok(runtime::ListSuggestedNextStepsResponse {
            schema_version: runtime::LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION,
            items: Vec::new(),
            generated_at,
        });
    }

    let subject_filter = subject_filter(input.subject.as_ref())?;
    let fetch_limit = (max_items * SURFACING_FETCH_MULTIPLIER).max(max_items);
    let candidates = state
        .db_read(move |db| {
            load_projection_candidates(
                db.conn_ref(),
                subject_filter,
                actor,
                generated_at,
                max_items,
                fetch_limit,
            )
            .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| anyhow!("list_suggested_next_steps surfacing read failed: {error}"))?;

    let claim_ids = candidates
        .iter()
        .map(|candidate| runtime::ClaimId(candidate.row.claim_id.0.clone()))
        .collect::<Vec<_>>();
    let receipts = render_receipts_for_batch(state, &claim_ids, input.surface).await;
    let receipts_by_claim_id = receipts
        .into_iter()
        .filter_map(|receipt| {
            let claim_id = receipt_claim_id(&receipt)?.to_string();
            Some((claim_id, receipt))
        })
        .collect::<HashMap<_, _>>();

    let mut items = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let Some(receipt) = receipts_by_claim_id.get(&candidate.row.claim_id.0).cloned() else {
            continue;
        };
        let why_this_now_surface_text =
            why_this_now_surface_text(&candidate.row, candidate.primary_rationale.as_ref());
        items.push(runtime::SuggestedNextStepItem {
            claim_id: runtime::ClaimId(candidate.row.claim_id.0),
            headline: redact_numeric_text(&candidate.payload.headline),
            why_this_now_surface_text,
            factor_band: factor_band(candidate.row.why_this_now.primary_factor),
            recommended_action: recommended_action_view(&candidate.payload.recommended_action),
            trust_band: candidate.payload.trust_band,
            receipt,
            feedback_state: feedback_state_to_runtime(candidate.payload.feedback_state),
            conversion_state: conversion_state_to_runtime(candidate.payload.conversion_state),
        });
        if items.len() == max_items {
            break;
        }
    }

    Ok(runtime::ListSuggestedNextStepsResponse {
        schema_version: runtime::LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION,
        items,
        generated_at,
    })
}

pub async fn read_recommendation_for_render(
    claim_id: &app::ClaimId,
    actor: ActorKind,
) -> Result<RecommendationRenderPayload> {
    let claim_id = claim_id.clone();
    tokio::task::spawn_blocking(move || {
        let db = ActionDb::open_readonly(std::sync::Arc::new(crate::db::LocalKeychain::new()))
            .context("open recommendation render read connection")?;
        read_recommendation_for_render_from_conn(db.conn_ref(), &claim_id, actor)?
            .ok_or_else(|| anyhow!("recommendation claim `{}` is not visible", claim_id.0))
    })
    .await
    .map_err(|error| anyhow!("recommendation render read task failed: {error}"))?
}

pub async fn render_receipts_for_batch(
    state: &AppState,
    claim_ids: &[runtime::ClaimId],
    surface: runtime_receipt::ClaimReceiptSurfaceContext,
) -> Vec<runtime_receipt::ClaimReceiptSnapshot> {
    if claim_ids.is_empty() {
        return Vec::new();
    }
    let claim_ids = claim_ids.to_vec();
    let app_surface = ability_surface_to_app(surface);
    state
        .db_read(move |db| {
            render_receipts_for_batch_from_conn(db.conn_ref(), &claim_ids, app_surface)
                .map_err(|error| error.to_string())
        })
        .await
        .unwrap_or_default()
}

fn load_projection_candidates(
    conn: &rusqlite::Connection,
    subject_filter: Option<(String, String)>,
    actor: ActorKind,
    now: DateTime<Utc>,
    max_items: usize,
    fetch_limit: usize,
) -> Result<Vec<ProjectionCandidate>> {
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(Vec::new());
    }

    let rows = load_surfacing_rows(conn, subject_filter, fetch_limit)?;
    let mut candidates = Vec::with_capacity(max_items);
    for row in rows {
        let Some(payload) = read_recommendation_for_render_from_conn(conn, &row.claim_id, actor)?
        else {
            continue;
        };
        if !feedback_visible_in_echo_window(&payload.feedback_state, now) {
            continue;
        }
        let primary_rationale = load_primary_rationale(
            conn,
            &row.claim_id,
            row.salience_evaluation_id.as_str(),
            row.why_this_now.primary_factor,
            payload.salience.as_ref(),
        )?;
        candidates.push(ProjectionCandidate {
            row,
            payload,
            primary_rationale,
        });
        if candidates.len() == max_items {
            break;
        }
    }
    Ok(candidates)
}

fn load_surfacing_rows(
    conn: &rusqlite::Connection,
    subject_filter: Option<(String, String)>,
    fetch_limit: usize,
) -> Result<Vec<SurfacingRenderRow>> {
    let limit = i64::try_from(fetch_limit).unwrap_or(i64::MAX);
    let mut rows = Vec::new();
    if let Some((subject_kind, subject_id)) = subject_filter {
        let mut stmt = conn.prepare(
            "SELECT claim_id, why_this_now_json, trigger_refs_json, salience_evaluation_id
               FROM surfacing_decisions
              WHERE subject_kind = ?1
                AND subject_id = ?2
                AND decision_kind = 'render'
                AND surfacing_tier IN ('critical', 'notable', 'background')
              ORDER BY salience_total DESC, created_at DESC
              LIMIT ?3",
        )?;
        let mapped = stmt.query_map(params![subject_kind, subject_id, limit], surfacing_row)?;
        for row in mapped {
            rows.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT claim_id, why_this_now_json, trigger_refs_json, salience_evaluation_id
               FROM surfacing_decisions
              WHERE decision_kind = 'render'
                AND surfacing_tier IN ('critical', 'notable', 'background')
              ORDER BY salience_total DESC, created_at DESC
              LIMIT ?1",
        )?;
        let mapped = stmt.query_map([limit], surfacing_row)?;
        for row in mapped {
            rows.push(row?);
        }
    }
    Ok(rows)
}

fn surfacing_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SurfacingRenderRow> {
    let claim_id = app::ClaimId(row.get::<_, String>(0)?);
    let why_raw: Option<String> = row.get(1)?;
    let trigger_raw: String = row.get(2)?;
    let why_this_now = why_raw
        .as_deref()
        .and_then(|raw| serde_json::from_str::<app::WhyThisNow>(raw).ok())
        .ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                "missing or invalid why_this_now_json".into(),
            )
        })?;
    let trigger_refs =
        serde_json::from_str::<Vec<app::TriggerRef>>(&trigger_raw).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    Ok(SurfacingRenderRow {
        claim_id,
        why_this_now,
        trigger_refs,
        salience_evaluation_id: row.get(3)?,
    })
}

fn read_recommendation_for_render_from_conn(
    conn: &rusqlite::Connection,
    claim_id: &app::ClaimId,
    actor: ActorKind,
) -> Result<Option<RecommendationRenderPayload>> {
    let row = conn
        .query_row(
            "SELECT id, subject_ref, text, field_path, metadata_json, trust_score,
                    claim_state, surfacing_state
               FROM intelligence_claims
              WHERE id = ?1 AND claim_type = 'recommendation'",
            [&claim_id.0],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<f64>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?;

    let Some((
        id,
        subject_ref,
        text,
        field_path,
        metadata_json,
        trust_score,
        claim_state,
        surfacing_state,
    )) = row
    else {
        return Ok(None);
    };

    if !claim_visible_to_actor(actor, &claim_state, &surfacing_state) {
        return Ok(None);
    }

    let metadata_raw = metadata_json
        .as_deref()
        .ok_or_else(|| anyhow!("recommendation claim `{id}` missing metadata_json"))?;
    let metadata = serde_json::from_str::<app::RecommendationMetadataEnvelope>(metadata_raw)
        .with_context(|| {
            format!("recommendation claim `{id}` metadata_json did not match schema")
        })?;
    let subject = subject_ref_from_storage(&subject_ref)?;

    Ok(Some(RecommendationRenderPayload {
        claim_id: app::ClaimId(id),
        subject,
        headline: text,
        field_path,
        recommended_action: metadata.recommendation.recommended_action,
        trust_band: claim_trust_band_from_score(trust_score),
        evidence: metadata.recommendation.evidence,
        feedback_state: metadata.recommendation.feedback_state,
        conversion_state: metadata.recommendation.conversion_state,
        salience: Some(metadata.recommendation.salience),
    }))
}

fn load_primary_rationale(
    conn: &rusqlite::Connection,
    claim_id: &app::ClaimId,
    evaluation_id: &str,
    primary_factor: app::SalienceFactorKind,
    metadata_salience: Option<&app::SalienceScore>,
) -> Result<Option<app::FactorRationale>> {
    if table_exists(conn, "salience_factors")? {
        let raw = conn
            .query_row(
                "SELECT rationale_json
                   FROM salience_factors
                  WHERE claim_id = ?1
                    AND evaluation_id = ?2
                    AND factor_kind = ?3
                  LIMIT 1",
                params![
                    &claim_id.0,
                    evaluation_id,
                    salience_factor_kind_storage(primary_factor)
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(raw) = raw {
            return serde_json::from_str::<app::FactorRationale>(&raw)
                .map(Some)
                .map_err(Into::into);
        }
    }

    Ok(metadata_salience.and_then(|score| {
        score
            .factors
            .iter()
            .find(|factor| factor.kind == primary_factor)
            .map(|factor| factor.rationale.clone())
    }))
}

fn feedback_visible_in_echo_window(
    feedback_state: &app::FeedbackState,
    now: DateTime<Utc>,
) -> bool {
    match feedback_state {
        app::FeedbackState::Pending => true,
        app::FeedbackState::Decided(decision) => {
            now.signed_duration_since(feedback_decided_at(decision))
                <= Duration::seconds(DECIDED_ECHO_WINDOW_SECS)
        }
    }
}

fn feedback_decided_at(decision: &app::RecommendationFeedbackDecision) -> DateTime<Utc> {
    match decision {
        app::RecommendationFeedbackDecision::Accept { at }
        | app::RecommendationFeedbackDecision::Dismiss { at, .. }
        | app::RecommendationFeedbackDecision::NotUseful { at }
        | app::RecommendationFeedbackDecision::TooNoisy { at }
        | app::RecommendationFeedbackDecision::Convert { at, .. } => *at,
    }
}

fn render_receipts_for_batch_from_conn(
    conn: &rusqlite::Connection,
    claim_ids: &[runtime::ClaimId],
    surface: app_receipt::SurfaceContext,
) -> Result<Vec<runtime_receipt::ClaimReceiptSnapshot>> {
    let batch_rows = load_receipt_batch_rows(conn, claim_ids)?;
    let row_by_claim_id = batch_rows
        .into_iter()
        .map(|row| (row.claim_id.clone(), row))
        .collect::<HashMap<_, _>>();
    let audience = audience_for_surface(surface);
    let mut receipts = Vec::with_capacity(claim_ids.len());

    for claim_id in claim_ids {
        let Some(row) = row_by_claim_id.get(&claim_id.0) else {
            continue;
        };
        let target = app_receipt::ReceiptTarget::Claim {
            claim_id: row.claim_id.clone(),
            subject: row.subject.clone(),
            field_path: row.field_path.clone(),
        };
        let mut receipt = match build_receipt_for_audience(&target, audience, conn) {
            Ok(receipt) => receipt,
            Err(PrivacyError::ClaimNotFound(_))
            | Err(PrivacyError::NonDisclosureAudience)
            | Err(PrivacyError::ComposedClaimDropped)
            | Err(PrivacyError::SurfaceDrop) => continue,
            Err(PrivacyError::Storage(error)) => return Err(error.into()),
            Err(PrivacyError::InvalidMetadata(message)) => {
                bail!("invalid receipt metadata: {message}")
            }
        };
        receipt.surface_context = surface;
        if matches!(audience, Audience::UserTauri) {
            if let Some(rendered_text) =
                render_text_for_user_tauri_surface_from_conn(conn, &row.claim_id, surface)?
            {
                receipt.rendered_text = Some(rendered_text);
            }
        }
        receipts.push(app_receipt_to_ability(receipt));
    }

    Ok(receipts)
}

fn load_receipt_batch_rows(
    conn: &rusqlite::Connection,
    claim_ids: &[runtime::ClaimId],
) -> Result<Vec<ReceiptBatchRow>> {
    if claim_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat_n("?", claim_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT id, subject_ref, field_path
           FROM intelligence_claims
          WHERE id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(claim_ids.iter().map(|claim_id| claim_id.0.as_str()));
    let mapped = stmt.query_map(params, |row| {
        let subject_raw: String = row.get(1)?;
        let subject = subject_ref_from_storage(&subject_raw).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, error.into())
        })?;
        Ok(ReceiptBatchRow {
            claim_id: row.get(0)?,
            subject,
            field_path: row.get(2)?,
        })
    })?;
    let mut rows = Vec::new();
    for row in mapped {
        rows.push(row?);
    }
    Ok(rows)
}

fn render_text_for_user_tauri_surface_from_conn(
    conn: &rusqlite::Connection,
    claim_id: &str,
    surface: app_receipt::SurfaceContext,
) -> Result<Option<abilities_runtime::sensitivity::RenderableClaimText>> {
    let claim = crate::services::claims::load_claim_by_id(conn, claim_id)?
        .ok_or_else(|| anyhow!("claim `{claim_id}` not found for receipt render text"))?;
    let actor = RenderActor {
        actor: "user".to_string(),
        user_id: None,
    };
    Ok(renderable_claim_text_with_value(
        &claim,
        &claim.text,
        render_surface_for(surface),
        &actor,
    ))
}

fn why_this_now_surface_text(
    row: &SurfacingRenderRow,
    rationale: Option<&app::FactorRationale>,
) -> String {
    let rationale = rationale
        .map(surface_rationale_summary)
        .unwrap_or_else(|| redact_numeric_text(&row.why_this_now.text));
    format!(
        "Salience driven by {}: {}. Triggers: {}.",
        factor_label(row.why_this_now.primary_factor),
        rationale,
        trigger_summary(if row.why_this_now.triggers.is_empty() {
            &row.trigger_refs
        } else {
            &row.why_this_now.triggers
        })
    )
}

fn surface_rationale_summary(rationale: &app::FactorRationale) -> String {
    match rationale {
        app::FactorRationale::Importance {
            trust_band,
            source_authority,
        } => format!(
            "trusted source strength is {} with {} trust",
            qualitative_band(*source_authority),
            trust_band_label(*trust_band)
        ),
        app::FactorRationale::Novelty {
            vector_distance, ..
        } => format!(
            "new compared with nearby memory at {} distance",
            qualitative_band(*vector_distance)
        ),
        app::FactorRationale::Urgency { decay_factor, .. } => {
            format!(
                "deadline or time pressure is {}",
                qualitative_band(*decay_factor)
            )
        }
        app::FactorRationale::Timing {
            signal_age_secs, ..
        } => format!("signal timing is {}", recency_band(*signal_age_secs)),
        app::FactorRationale::UserFit {
            feedback_history_score,
        } => format!(
            "prior feedback fit is {}",
            qualitative_band(*feedback_history_score)
        ),
        app::FactorRationale::Freshness { decay_factor } => {
            format!("source timing is {}", qualitative_band(*decay_factor))
        }
        app::FactorRationale::Trust { trust_band } => {
            format!("trust band is {}", trust_band_label(*trust_band))
        }
        app::FactorRationale::Corroboration { .. } => "supporting evidence is present".to_string(),
        app::FactorRationale::Contradiction { .. } => {
            "contradiction state needs review".to_string()
        }
        app::FactorRationale::OpenLoopRelevance { has_action, .. } => {
            if *has_action {
                "related open loop has an action".to_string()
            } else {
                "related open loop context is present".to_string()
            }
        }
    }
}

fn recommended_action_view(action: &app::RecommendedAction) -> runtime::RecommendedActionView {
    match action {
        app::RecommendedAction::ScheduleMeeting { when_window, .. } => {
            runtime::RecommendedActionView::ScheduleMeeting {
                entity_label: "Entity".to_string(),
                when_window: redact_numeric_text(when_window),
            }
        }
        app::RecommendedAction::SendMessage { channel, .. } => {
            runtime::RecommendedActionView::SendMessage {
                entity_label: "Entity".to_string(),
                channel: redact_numeric_text(channel),
            }
        }
        app::RecommendedAction::ReviewClaim { .. } => runtime::RecommendedActionView::ReviewClaim {
            claim_label: "Claim".to_string(),
        },
        app::RecommendedAction::UpdateRecord { field_path, .. } => {
            runtime::RecommendedActionView::UpdateRecord {
                entity_label: "Entity".to_string(),
                field_label: field_label(field_path),
            }
        }
        app::RecommendedAction::InvestigateChange { change_summary, .. } => {
            runtime::RecommendedActionView::InvestigateChange {
                entity_label: "Entity".to_string(),
                change_label: redact_numeric_text(change_summary),
            }
        }
        app::RecommendedAction::Custom { action_kind, .. } => {
            runtime::RecommendedActionView::Custom {
                action_label: action_label(action_kind),
            }
        }
    }
}

fn factor_band(kind: app::SalienceFactorKind) -> runtime::PrimaryFactorBand {
    match kind {
        app::SalienceFactorKind::Urgency | app::SalienceFactorKind::Timing => {
            runtime::PrimaryFactorBand::TimeSensitive
        }
        app::SalienceFactorKind::Novelty | app::SalienceFactorKind::Freshness => {
            runtime::PrimaryFactorBand::NewInformation
        }
        app::SalienceFactorKind::OpenLoopRelevance => runtime::PrimaryFactorBand::OpenLoopRelated,
        app::SalienceFactorKind::Trust
        | app::SalienceFactorKind::Corroboration
        | app::SalienceFactorKind::Contradiction => runtime::PrimaryFactorBand::TrustChange,
        app::SalienceFactorKind::Importance | app::SalienceFactorKind::UserFit => {
            runtime::PrimaryFactorBand::Other
        }
    }
}

fn redact_numeric_text(text: &str) -> String {
    let mut redacted = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            while matches!(chars.peek(), Some(next) if next.is_ascii_digit() || *next == '.') {
                chars.next();
            }
            redacted.push_str("value");
        } else {
            redacted.push(ch);
        }
    }
    redacted
}

fn field_label(field_path: &str) -> String {
    let label = field_path
        .rsplit(['.', '['])
        .next()
        .unwrap_or("field")
        .trim_matches(']')
        .replace(['_', '-'], " ");
    let label = redact_numeric_text(&label);
    if label.trim().is_empty() {
        "Record field".to_string()
    } else {
        label
    }
}

fn action_label(action_kind: &str) -> String {
    let label = redact_numeric_text(&action_kind.replace(['_', '-'], " "));
    if label.trim().is_empty() {
        "Custom action".to_string()
    } else {
        label
    }
}

fn qualitative_band(value: f64) -> &'static str {
    if value >= 0.75 {
        "high"
    } else if value >= 0.4 {
        "moderate"
    } else {
        "low"
    }
}

fn recency_band(signal_age_secs: i64) -> &'static str {
    if signal_age_secs <= 86_400 {
        "high"
    } else if signal_age_secs <= 604_800 {
        "moderate"
    } else {
        "low"
    }
}

fn trust_band_label(trust_band: TrustBand) -> &'static str {
    match trust_band {
        TrustBand::LikelyCurrent => "likely current",
        TrustBand::UseWithCaution => "use with caution",
        TrustBand::NeedsVerification => "needs verification",
        TrustBand::Unscored => "unscored",
    }
}

fn trigger_summary(triggers: &[app::TriggerRef]) -> String {
    let mut sources = triggers
        .iter()
        .map(|trigger| trigger.source.as_str())
        .collect::<Vec<_>>();
    sources.sort_unstable();
    sources.dedup();
    if sources.is_empty() {
        "scheduled_scan".to_string()
    } else {
        redact_numeric_text(&sources.into_iter().take(3).collect::<Vec<_>>().join(", "))
    }
}

fn factor_label(kind: app::SalienceFactorKind) -> &'static str {
    match kind {
        app::SalienceFactorKind::Importance => "importance",
        app::SalienceFactorKind::Novelty => "novelty",
        app::SalienceFactorKind::Urgency => "urgency",
        app::SalienceFactorKind::Timing => "timing",
        app::SalienceFactorKind::UserFit => "user fit",
        app::SalienceFactorKind::Freshness => "freshness",
        app::SalienceFactorKind::Trust => "trust",
        app::SalienceFactorKind::Corroboration => "corroboration",
        app::SalienceFactorKind::Contradiction => "contradiction",
        app::SalienceFactorKind::OpenLoopRelevance => "open loop relevance",
    }
}

fn claim_visible_to_actor(actor: ActorKind, claim_state: &str, surfacing_state: &str) -> bool {
    match actor {
        ActorKind::System => matches!(claim_state, "active" | "dormant"),
        ActorKind::User | ActorKind::SurfaceClient => {
            claim_state == "active" && surfacing_state == "active"
        }
        ActorKind::Agent | ActorKind::Admin | ActorKind::McpClient => false,
    }
}

fn subject_filter(subject: Option<&SubjectRef>) -> Result<Option<(String, String)>> {
    Ok(match subject {
        None | Some(SubjectRef::Global) | Some(SubjectRef::Unknown) => None,
        Some(SubjectRef::Account(id)) => Some(("account".to_string(), non_empty_id(id)?)),
        Some(SubjectRef::Project(id)) => Some(("project".to_string(), non_empty_id(id)?)),
        Some(SubjectRef::Person(id)) => Some(("person".to_string(), non_empty_id(id)?)),
        Some(SubjectRef::Meeting(id)) => Some(("meeting".to_string(), non_empty_id(id)?)),
        Some(SubjectRef::User(id)) => Some(("user".to_string(), non_empty_id(id)?)),
        Some(SubjectRef::Multi(_)) => {
            bail!("list_suggested_next_steps does not support multi-subject filters")
        }
    })
}

fn non_empty_id(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("subject id must not be empty");
    }
    Ok(trimmed.to_string())
}

fn subject_ref_from_storage(raw: &str) -> Result<SubjectRef> {
    let value = serde_json::from_str::<serde_json::Value>(raw)
        .with_context(|| format!("invalid subject_ref JSON: {raw}"))?;
    let kind = value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("subject_ref missing kind"))?;
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok(match kind {
        "account" => SubjectRef::Account(id),
        "project" => SubjectRef::Project(id),
        "person" => SubjectRef::Person(id),
        "meeting" => SubjectRef::Meeting(id),
        "user" => SubjectRef::User(id),
        "global" => SubjectRef::Global,
        _ => SubjectRef::Unknown,
    })
}

fn salience_factor_kind_storage(kind: app::SalienceFactorKind) -> &'static str {
    match kind {
        app::SalienceFactorKind::Importance => "importance",
        app::SalienceFactorKind::Novelty => "novelty",
        app::SalienceFactorKind::Urgency => "urgency",
        app::SalienceFactorKind::Timing => "timing",
        app::SalienceFactorKind::UserFit => "user_fit",
        app::SalienceFactorKind::Freshness => "freshness",
        app::SalienceFactorKind::Trust => "trust",
        app::SalienceFactorKind::Corroboration => "corroboration",
        app::SalienceFactorKind::Contradiction => "contradiction",
        app::SalienceFactorKind::OpenLoopRelevance => "open_loop_relevance",
    }
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0).map(|count| count > 0),
    )
    .map_err(Into::into)
}

fn ability_surface_to_app(
    surface: runtime_receipt::ClaimReceiptSurfaceContext,
) -> app_receipt::SurfaceContext {
    match surface {
        runtime_receipt::ClaimReceiptSurfaceContext::ActionsWork => {
            app_receipt::SurfaceContext::ActionsWork
        }
        runtime_receipt::ClaimReceiptSurfaceContext::EntityDetail => {
            app_receipt::SurfaceContext::EntityDetail
        }
        runtime_receipt::ClaimReceiptSurfaceContext::DailyBriefing => {
            app_receipt::SurfaceContext::DailyBriefing
        }
        runtime_receipt::ClaimReceiptSurfaceContext::MeetingDetail => {
            app_receipt::SurfaceContext::MeetingDetail
        }
        runtime_receipt::ClaimReceiptSurfaceContext::Mcp => app_receipt::SurfaceContext::Mcp,
    }
}

fn app_surface_to_ability(
    surface: app_receipt::SurfaceContext,
) -> runtime_receipt::ClaimReceiptSurfaceContext {
    match surface {
        app_receipt::SurfaceContext::ActionsWork => {
            runtime_receipt::ClaimReceiptSurfaceContext::ActionsWork
        }
        app_receipt::SurfaceContext::EntityDetail => {
            runtime_receipt::ClaimReceiptSurfaceContext::EntityDetail
        }
        app_receipt::SurfaceContext::DailyBriefing => {
            runtime_receipt::ClaimReceiptSurfaceContext::DailyBriefing
        }
        app_receipt::SurfaceContext::MeetingDetail => {
            runtime_receipt::ClaimReceiptSurfaceContext::MeetingDetail
        }
        app_receipt::SurfaceContext::Mcp => runtime_receipt::ClaimReceiptSurfaceContext::Mcp,
    }
}

fn render_surface_for(surface: app_receipt::SurfaceContext) -> RenderSurface {
    match surface {
        app_receipt::SurfaceContext::ActionsWork => RenderSurface::Action,
        app_receipt::SurfaceContext::EntityDetail => RenderSurface::TauriEntityDetail,
        app_receipt::SurfaceContext::DailyBriefing => RenderSurface::TauriBriefingPrep,
        app_receipt::SurfaceContext::MeetingDetail => RenderSurface::TauriMeetingDetail,
        app_receipt::SurfaceContext::Mcp => RenderSurface::McpTool,
    }
}

fn app_receipt_to_ability(
    receipt: app_receipt::ClaimReceipt,
) -> runtime_receipt::ClaimReceiptSnapshot {
    runtime_receipt::ClaimReceiptSnapshot {
        target: app_target_to_ability(&receipt.target),
        surface_context: app_surface_to_ability(receipt.surface_context),
        rendered_text: receipt.rendered_text,
        trust: runtime_receipt::ClaimReceiptTrust {
            band: receipt.trust.band,
            source_asof: receipt.trust.source_asof,
            freshness: match receipt.trust.freshness {
                app_receipt::Freshness::Current => runtime_receipt::ClaimReceiptFreshness::Current,
                app_receipt::Freshness::Aging => runtime_receipt::ClaimReceiptFreshness::Aging,
                app_receipt::Freshness::Stale => runtime_receipt::ClaimReceiptFreshness::Stale,
                app_receipt::Freshness::Unknown => runtime_receipt::ClaimReceiptFreshness::Unknown,
            },
            caveat: receipt.trust.caveat,
            rationale: receipt.trust.rationale,
        },
        lifecycle: runtime_receipt::ClaimReceiptLifecycle {
            claim_state: receipt.lifecycle.claim_state,
            surfacing_state: receipt.lifecycle.surfacing_state,
            verification_state: receipt.lifecycle.verification_state,
            updated_at: receipt.lifecycle.updated_at,
        },
        provenance: runtime_receipt::ClaimReceiptProvenance {
            sources: receipt
                .provenance
                .sources
                .into_iter()
                .map(|source| runtime_receipt::ClaimReceiptProvenanceSource {
                    label: source.label,
                    source_type: source.source_type,
                    as_of: source.as_of,
                    href: source.href,
                    redacted: source.redacted,
                })
                .collect(),
            field_path: receipt.provenance.field_path,
            evidence_summary: receipt.provenance.evidence_summary,
            redaction: match receipt.provenance.redaction {
                app_receipt::RedactionLevel::None => {
                    runtime_receipt::ClaimReceiptRedactionLevel::None
                }
                app_receipt::RedactionLevel::Partial => {
                    runtime_receipt::ClaimReceiptRedactionLevel::Partial
                }
                app_receipt::RedactionLevel::Full => {
                    runtime_receipt::ClaimReceiptRedactionLevel::Full
                }
            },
        },
        actions: receipt
            .actions
            .into_iter()
            .map(|action| runtime_receipt::ClaimReceiptAction {
                action: action.action,
                label: action.label,
                disabled_reason: action.disabled_reason,
            })
            .collect(),
    }
}

fn app_target_to_ability(
    target: &app_receipt::ReceiptTarget,
) -> runtime_receipt::ClaimReceiptTarget {
    match target {
        app_receipt::ReceiptTarget::Claim {
            claim_id,
            subject,
            field_path,
        } => runtime_receipt::ClaimReceiptTarget::Claim {
            claim_id: claim_id.clone(),
            subject: subject.clone(),
            field_path: field_path.clone(),
        },
        app_receipt::ReceiptTarget::Proposal {
            proposal_id,
            subject,
            field_path,
        } => runtime_receipt::ClaimReceiptTarget::Proposal {
            proposal_id: proposal_id.clone(),
            subject: subject.clone(),
            field_path: field_path.clone(),
        },
        app_receipt::ReceiptTarget::WorkItem {
            action_id,
            backing_claim_id,
            subject,
        } => runtime_receipt::ClaimReceiptTarget::WorkItem {
            action_id: action_id.clone(),
            backing_claim_id: backing_claim_id.clone(),
            subject: subject.clone(),
        },
    }
}

fn receipt_claim_id(receipt: &runtime_receipt::ClaimReceiptSnapshot) -> Option<&str> {
    match &receipt.target {
        runtime_receipt::ClaimReceiptTarget::Claim { claim_id, .. } => Some(claim_id.as_str()),
        runtime_receipt::ClaimReceiptTarget::Proposal { .. }
        | runtime_receipt::ClaimReceiptTarget::WorkItem { .. } => None,
    }
}

fn feedback_state_to_runtime(state: app::FeedbackState) -> runtime::FeedbackState {
    match state {
        app::FeedbackState::Pending => runtime::FeedbackState::Pending,
        app::FeedbackState::Decided(decision) => {
            runtime::FeedbackState::Decided(feedback_decision_to_runtime(decision))
        }
    }
}

fn feedback_decision_to_runtime(
    decision: app::RecommendationFeedbackDecision,
) -> runtime::RecommendationFeedbackDecision {
    match decision {
        app::RecommendationFeedbackDecision::Accept { at } => {
            runtime::RecommendationFeedbackDecision::Accept { at }
        }
        app::RecommendationFeedbackDecision::Dismiss { at, reason } => {
            runtime::RecommendationFeedbackDecision::Dismiss {
                at,
                reason: dismiss_reason_to_runtime(reason),
            }
        }
        app::RecommendationFeedbackDecision::NotUseful { at } => {
            runtime::RecommendationFeedbackDecision::NotUseful { at }
        }
        app::RecommendationFeedbackDecision::TooNoisy { at } => {
            runtime::RecommendationFeedbackDecision::TooNoisy { at }
        }
        app::RecommendationFeedbackDecision::Convert { at, into } => {
            runtime::RecommendationFeedbackDecision::Convert {
                at,
                into: conversion_target_to_runtime(into),
            }
        }
    }
}

fn dismiss_reason_to_runtime(reason: app::DismissReason) -> runtime::DismissReason {
    match reason {
        app::DismissReason::NotRelevant => runtime::DismissReason::NotRelevant,
        app::DismissReason::AlreadyKnew => runtime::DismissReason::AlreadyKnew,
        app::DismissReason::WrongSubject => runtime::DismissReason::WrongSubject,
        app::DismissReason::Other(note) => runtime::DismissReason::Other(
            runtime::BoundedNote::try_from(note.into_inner())
                .expect("app bounded note fits runtime"),
        ),
    }
}

fn conversion_target_to_runtime(target: app::ConversionTarget) -> runtime::ConversionTarget {
    match target {
        app::ConversionTarget::Action(action_id) => runtime::ConversionTarget::Action(action_id),
        app::ConversionTarget::ClaimCorrection(claim_id) => {
            runtime::ConversionTarget::ClaimCorrection(runtime::ClaimId(claim_id.0))
        }
        app::ConversionTarget::ReviewQueue(queue_item_id) => {
            runtime::ConversionTarget::ReviewQueue(queue_item_id)
        }
    }
}

fn conversion_state_to_runtime(state: app::ConversionState) -> runtime::ConversionState {
    match state {
        app::ConversionState::NotConverted => runtime::ConversionState::NotConverted,
        app::ConversionState::ConvertedToAction { action_id } => {
            runtime::ConversionState::ConvertedToAction { action_id }
        }
        app::ConversionState::ConvertedToClaimCorrection { claim_id } => {
            runtime::ConversionState::ConvertedToClaimCorrection {
                claim_id: runtime::ClaimId(claim_id.0),
            }
        }
        app::ConversionState::ConvertedToReviewQueue { queue_item_id } => {
            runtime::ConversionState::ConvertedToReviewQueue { queue_item_id }
        }
    }
}

#[cfg(test)]
mod list_suggested_next_steps_tests {
    use super::*;

    use abilities_runtime::abilities::claim_receipt::ClaimReceiptRedactionLevel;
    use chrono::TimeZone;
    use rusqlite::params;
    use std::collections::HashSet;

    use crate::services::recommendations::contracts::{
        EvidenceRef, FactorRationale, RecommendationDraft, RecommendationFeedbackDecision,
        RecommendationMetadataEnvelope, RecommendationMetadataPayload, RecommendedAction,
        SalienceFactor, SalienceScore, TriggerKind, TriggerRef,
        RECOMMENDATION_METADATA_SCHEMA_VERSION,
    };
    use crate::services::recommendations::recommendation::metadata_envelope;

    async fn test_state() -> (AppState, tempfile::TempDir) {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db_path = tempdir.path().join("suggested-next-steps.db");
        let db_service = crate::db_service::DbService::open_at_unencrypted(db_path)
            .await
            .expect("open test db service");
        (AppState::test_with_db_service(db_service), tempdir)
    }

    async fn seed_recommendation(
        state: &AppState,
        claim_id: &str,
        subject_id: &str,
        action: RecommendedAction,
        feedback_state: app::FeedbackState,
        created_at: DateTime<Utc>,
    ) {
        let claim_id = claim_id.to_string();
        let subject_id = subject_id.to_string();
        let metadata = recommendation_metadata(&claim_id, action, feedback_state);
        let metadata_json = serde_json::to_string(&metadata).expect("metadata serializes");
        state
            .db_write(move |db| {
                db.conn_ref()
                    .execute(
                        "INSERT INTO intelligence_claims /* dos7-allowed: suggested next steps unit test seed */ (
                            id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
                            item_hash, actor, data_source, source_ref, source_asof, observed_at,
                            created_at, provenance_json, metadata_json, claim_state, surfacing_state,
                            demotion_reason, reactivated_at, retraction_reason, expires_at,
                            superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
                            temporal_scope, sensitivity, verification_state, verification_reason,
                            needs_user_decision_at, claim_version, canonical_status,
                            non_semantic_mergeable
                        ) VALUES (
                            ?1, ?2, 'recommendation', 'recommendation.reviewClaim', ?1,
                            'Review the account before the next customer conversation.',
                            ?3, ?4, 'agent:test', 'recommendation', 'run:test', ?5, ?5,
                            ?5, '{\"sources\":[]}', ?6, 'active', 'active',
                            NULL, NULL, NULL, NULL, NULL, 0.82, ?5, 1, NULL, ?7,
                            ?8, 'active', NULL, NULL, 1, 'live', 0
                        )",
                        params![
                            claim_id,
                            format!(r#"{{"kind":"account","id":"{subject_id}"}}"#),
                            format!("dedup-{claim_id}"),
                            format!("hash-{claim_id}"),
                            created_at.to_rfc3339(),
                            metadata_json,
                            "state",
                            "internal",
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .await
            .expect("seed recommendation");
    }

    async fn seed_surfacing_decision(
        state: &AppState,
        claim_id: &str,
        subject_id: &str,
        salience_total: f64,
        created_at: DateTime<Utc>,
        primary_factor: app::SalienceFactorKind,
    ) {
        let claim_id = claim_id.to_string();
        let subject_id = subject_id.to_string();
        let why_this_now = app::WhyThisNow {
            primary_factor,
            text: "Salience driven by urgency: decay_factor 0.92 and signal_age_secs 42."
                .to_string(),
            triggers: vec![TriggerRef {
                trigger_kind: TriggerKind::SignalArrival,
                at: created_at,
                source: "signal_event".to_string(),
            }],
        };
        let trigger_refs_json = serde_json::to_string(&why_this_now.triggers).unwrap();
        let why_this_now_json = serde_json::to_string(&why_this_now).unwrap();
        state
            .db_write(move |db| {
                db.conn_ref()
                    .execute(
                        "INSERT INTO surfacing_decisions (
                            id, idempotency_key, policy_version, claim_id, decision_kind,
                            surfacing_tier, defer_reason, defer_until, suppress_reason, budget_key,
                            actor_kind, local_day, claim_type, sensitivity, surface_class,
                            render_surface, subject_kind, subject_id, action_signature,
                            salience_total, salience_evaluation_id, why_this_now_json,
                            trigger_refs_json, evidence_signature, source_asof,
                            source_signal_id, created_at
                        ) VALUES (
                            ?1, ?2, 'recommendation_surfacing_v1', ?3, 'render',
                            'notable', NULL, NULL, NULL, 'test-budget',
                            'system', '2026-05-26', 'recommendation', 'internal',
                            'primary', 'briefing', 'account', ?4, 'reviewClaim',
                            ?5, 'salience-eval-test', ?6, ?7, NULL, NULL, NULL, ?8
                        )",
                        params![
                            format!("surfacing-{claim_id}"),
                            format!("surfacing-key-{claim_id}"),
                            claim_id,
                            subject_id,
                            salience_total,
                            why_this_now_json,
                            trigger_refs_json,
                            created_at.to_rfc3339(),
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .await
            .expect("seed surfacing decision");
    }

    fn recommendation_metadata(
        claim_id: &str,
        action: RecommendedAction,
        feedback_state: app::FeedbackState,
    ) -> RecommendationMetadataEnvelope {
        let draft = RecommendationDraft {
            subject: SubjectRef::Account("acct-1".to_string()),
            recommended_action: action,
            evidence: vec![EvidenceRef {
                source: format!("claim:{claim_id}"),
                chunk: None,
            }],
            provenance_json: r#"{"sources":[]}"#.to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some(Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap()),
            observed_at: Utc.with_ymd_and_hms(2026, 5, 26, 10, 0, 0).unwrap(),
            text: "Review the account before the next customer conversation.".to_string(),
            salience: SalienceScore {
                total: 0.91,
                factors: vec![SalienceFactor {
                    kind: app::SalienceFactorKind::Urgency,
                    value: Some(0.91),
                    weight: 0.15,
                    rationale: FactorRationale::Urgency {
                        deadline: Some(Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap()),
                        decay_factor: 0.91,
                    },
                }],
            },
        };
        let mut envelope = metadata_envelope(&draft);
        envelope.recommendation = RecommendationMetadataPayload {
            schema_version: RECOMMENDATION_METADATA_SCHEMA_VERSION,
            recommended_action: envelope.recommendation.recommended_action,
            evidence: envelope.recommendation.evidence,
            salience: envelope.recommendation.salience,
            feedback_state,
            conversion_state: app::ConversionState::NotConverted,
        };
        envelope
    }

    #[test]
    fn factor_band_collapse_table_is_deterministic_per_kind() {
        use app::SalienceFactorKind::*;
        let cases = [
            (Urgency, runtime::PrimaryFactorBand::TimeSensitive),
            (Timing, runtime::PrimaryFactorBand::TimeSensitive),
            (Novelty, runtime::PrimaryFactorBand::NewInformation),
            (Freshness, runtime::PrimaryFactorBand::NewInformation),
            (
                OpenLoopRelevance,
                runtime::PrimaryFactorBand::OpenLoopRelated,
            ),
            (Trust, runtime::PrimaryFactorBand::TrustChange),
            (Corroboration, runtime::PrimaryFactorBand::TrustChange),
            (Contradiction, runtime::PrimaryFactorBand::TrustChange),
            (Importance, runtime::PrimaryFactorBand::Other),
            (UserFit, runtime::PrimaryFactorBand::Other),
        ];

        for (kind, expected) in cases {
            assert_eq!(factor_band(kind), expected);
        }
    }

    #[test]
    fn numeric_redactor_strips_factor_rationale_numeric_leaves() {
        let now = Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap();
        let rationales = vec![
            FactorRationale::Importance {
                trust_band: TrustBand::LikelyCurrent,
                source_authority: 0.83,
            },
            FactorRationale::Novelty {
                vector_distance: 0.72,
                neighbor_count: 12,
            },
            FactorRationale::Urgency {
                deadline: Some(now),
                decay_factor: 0.92,
            },
            FactorRationale::Timing {
                signal_age_secs: 42,
                calendar_proximity_secs: Some(3600),
            },
            FactorRationale::UserFit {
                feedback_history_score: 0.61,
            },
            FactorRationale::Freshness { decay_factor: 0.44 },
            FactorRationale::Trust {
                trust_band: TrustBand::UseWithCaution,
            },
            FactorRationale::Corroboration {
                corroboration_count: 4,
            },
            FactorRationale::Contradiction {
                contradiction_count: 3,
            },
            FactorRationale::OpenLoopRelevance {
                open_loop_count: 7,
                has_action: true,
            },
        ];
        let forbidden = [
            "0.83",
            "0.72",
            "12",
            "0.92",
            "42",
            "3600",
            "0.61",
            "0.44",
            "4",
            "3",
            "7",
            "source_authority",
            "vector_distance",
            "decay_factor",
            "signal_age_secs",
            "calendar_proximity_secs",
            "feedback_history_score",
            "corroboration_count",
            "contradiction_count",
            "open_loop_count",
        ];

        for rationale in rationales {
            let rendered = surface_rationale_summary(&rationale);
            for token in forbidden {
                assert!(
                    !rendered.contains(token),
                    "redacted rationale `{rendered}` leaked `{token}`"
                );
            }
        }
    }

    #[tokio::test]
    async fn surface_context_propagation_uses_surface_client_receipt_tier() {
        let (state, _tempdir) = test_state().await;
        let now = Utc::now();
        seed_recommendation(
            &state,
            "claim-receipt-surface",
            "acct-1",
            RecommendedAction::ReviewClaim {
                claim_id: app::ClaimId("claim-source".to_string()),
                reason: "verify".to_string(),
            },
            app::FeedbackState::Pending,
            now,
        )
        .await;
        seed_surfacing_decision(
            &state,
            "claim-receipt-surface",
            "acct-1",
            0.9,
            now,
            app::SalienceFactorKind::Urgency,
        )
        .await;

        let response = list_suggested_next_steps_projection(
            &state,
            runtime::ListSuggestedNextStepsInput {
                schema_version: runtime::LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION,
                subject: Some(SubjectRef::Account("acct-1".to_string())),
                surface: runtime_receipt::ClaimReceiptSurfaceContext::EntityDetail,
                max_items: Some(5),
            },
            ActorKind::SurfaceClient,
        )
        .await
        .expect("projection succeeds");

        assert_eq!(response.items.len(), 1);
        assert_eq!(
            response.items[0].receipt.surface_context,
            runtime_receipt::ClaimReceiptSurfaceContext::EntityDetail
        );
        assert_eq!(
            response.items[0].receipt.provenance.redaction,
            ClaimReceiptRedactionLevel::None
        );
    }

    #[tokio::test]
    async fn thirty_second_echo_window_filters_old_decided_rows() {
        let (state, _tempdir) = test_state().await;
        let now = Utc::now();
        let old_decided_at = now - Duration::seconds(31);
        let recent_decided_at = now - Duration::seconds(29);
        seed_recommendation(
            &state,
            "claim-old-decided",
            "acct-echo",
            RecommendedAction::ReviewClaim {
                claim_id: app::ClaimId("claim-old-source".to_string()),
                reason: "verify".to_string(),
            },
            app::FeedbackState::Decided(RecommendationFeedbackDecision::Dismiss {
                at: old_decided_at,
                reason: app::DismissReason::NotRelevant,
            }),
            now,
        )
        .await;
        seed_surfacing_decision(
            &state,
            "claim-old-decided",
            "acct-echo",
            0.99,
            now,
            app::SalienceFactorKind::Urgency,
        )
        .await;
        seed_recommendation(
            &state,
            "claim-recent-decided",
            "acct-echo",
            RecommendedAction::ReviewClaim {
                claim_id: app::ClaimId("claim-recent-source".to_string()),
                reason: "verify".to_string(),
            },
            app::FeedbackState::Decided(RecommendationFeedbackDecision::Dismiss {
                at: recent_decided_at,
                reason: app::DismissReason::NotRelevant,
            }),
            now,
        )
        .await;
        seed_surfacing_decision(
            &state,
            "claim-recent-decided",
            "acct-echo",
            0.8,
            now,
            app::SalienceFactorKind::Urgency,
        )
        .await;

        let response = list_suggested_next_steps_projection(
            &state,
            runtime::ListSuggestedNextStepsInput {
                schema_version: runtime::LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION,
                subject: Some(SubjectRef::Account("acct-echo".to_string())),
                surface: runtime_receipt::ClaimReceiptSurfaceContext::EntityDetail,
                max_items: Some(8),
            },
            ActorKind::SurfaceClient,
        )
        .await
        .expect("projection succeeds");

        let ids = response
            .items
            .iter()
            .map(|item| item.claim_id.0.as_str())
            .collect::<HashSet<_>>();
        assert!(!ids.contains("claim-old-decided"));
        assert!(ids.contains("claim-recent-decided"));
    }
}
