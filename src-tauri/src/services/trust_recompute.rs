use chrono::Utc;
use rusqlite::OptionalExtension;

use crate::db::claims::IntelligenceClaim;
use crate::db::ActionDb;
use crate::services::context::ServiceContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustRecomputeReport {
    pub subject_type: String,
    pub subject_id: String,
    pub claims_seen: usize,
    pub claims_updated: usize,
    pub claims_skipped: usize,
    pub failures_recorded: usize,
}

impl TrustRecomputeReport {
    fn new(subject_type: &str, subject_id: &str) -> Self {
        Self {
            subject_type: normalize_entity_type(subject_type),
            subject_id: subject_id.to_string(),
            claims_seen: 0,
            claims_updated: 0,
            claims_skipped: 0,
            failures_recorded: 0,
        }
    }
}

pub fn recompute_claim_trust_for_subject(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
) -> Result<TrustRecomputeReport, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let subject_type = normalize_entity_type(subject_type);
    let mut report = TrustRecomputeReport::new(&subject_type, subject_id);

    let subject_ref = claim_subject_ref_json_for_entity(&subject_type, subject_id)
        .ok_or_else(|| format!("unsupported claim recompute subject type: {subject_type}"))?;

    let claims = crate::services::claims::load_claims_active(db, &subject_ref, None)
        .map_err(|e| format!("load claims for trust recompute: {e}"))?;
    if claims.is_empty() {
        return Ok(report);
    }
    report.claims_seen = claims.len();
    let mut fatal_errors = Vec::new();

    let account_extraction_context =
        match crate::services::trust_extraction::build_account_extraction_context(
            db,
            &subject_type,
            subject_id,
        ) {
            Ok(context) => context,
            Err(e) => {
                log::warn!(
                    "TrustRecompute: extractor context error on {}:{}: {}",
                    &subject_type,
                    subject_id,
                    e
                );
                for claim in claims {
                    report.failures_recorded += record_trust_recompute_pipeline_failure(
                        ctx,
                        db,
                        &subject_type,
                        subject_id,
                        "extractor_error",
                        Some(&format!("claim_id={} error={e}", claim.id)),
                    );
                }
                return Err(format!(
                    "extractor context error on {subject_type}:{subject_id}: {e}"
                ));
            }
        };

    for claim in claims {
        let subject = match trust_subject_from_claim_json(&claim.subject_ref) {
            Ok(subject) => subject,
            Err(e) => {
                report.claims_skipped += 1;
                log::warn!(
                    "TrustRecompute: skipping claim {} with invalid subject_ref: {}",
                    claim.id,
                    e
                );
                fatal_errors.push(format!("claim_id={} invalid subject_ref: {e}", claim.id));
                continue;
            }
        };

        let extraction_outcome = if let Some(context) = account_extraction_context.as_ref() {
            crate::services::trust_extraction::extract_target_footprint_from_context(
                context, &subject,
            )
        } else {
            match crate::services::trust_extraction::extract_generic_target_footprint(
                db,
                &subject,
                &subject_type,
                subject_id,
            ) {
                Ok(outcome) => outcome,
                Err(e) => {
                    report.claims_skipped += 1;
                    report.failures_recorded += record_trust_recompute_pipeline_failure(
                        ctx,
                        db,
                        &subject_type,
                        subject_id,
                        "extractor_error",
                        Some(&format!("claim_id={} error={e}", claim.id)),
                    );
                    fatal_errors.push(format!("claim_id={} extractor error: {e}", claim.id));
                    continue;
                }
            }
        };

        match extraction_outcome {
            crate::services::trust_extraction::ExtractionOutcome::SkipExtractorMismatch {
                reason,
            } => {
                report.claims_skipped += 1;
                log::debug!(
                    "TrustRecompute: extractor mismatch for claim {} on {}:{} ({:?}); preserving prior trust",
                    claim.id,
                    &subject_type,
                    subject_id,
                    reason
                );
                report.failures_recorded += record_trust_recompute_pipeline_failure(
                    ctx,
                    db,
                    &subject_type,
                    subject_id,
                    "extractor_mismatch",
                    Some(&format!("claim_id={} reason={reason:?}", claim.id)),
                );
            }
            crate::services::trust_extraction::ExtractionOutcome::Ok {
                footprint,
                portfolio_footprints,
            } => {
                let (trust_ctx, indeterminate_reasons) = build_trust_context_for_claim(
                    ctx,
                    db,
                    &subject_type,
                    subject_id,
                    &claim,
                    footprint,
                    portfolio_footprints,
                );
                if !indeterminate_reasons.is_empty() {
                    report.failures_recorded += record_trust_recompute_pipeline_failure(
                        ctx,
                        db,
                        &subject_type,
                        subject_id,
                        "trust_read_state_indeterminate",
                        Some(&format!(
                            "claim_id={} reasons={}",
                            claim.id,
                            indeterminate_reasons.join(",")
                        )),
                    );
                }
                let previous_score = claim.trust_score;
                let previous_band =
                    previous_score.and_then(|score| trust_band_for_score(score, &trust_ctx.config));
                let trust_version = claim.trust_version.unwrap_or(0) + 1;

                let computation = match crate::abilities::trust::compile_trust(&claim, trust_ctx) {
                    Ok(computation) => computation,
                    Err(e) => {
                        report.claims_skipped += 1;
                        log::error!(
                            "TrustRecompute: compile_trust failed for claim {}; preserving prior trust: {}",
                            claim.id,
                            e
                        );
                        report.failures_recorded += record_trust_recompute_pipeline_failure(
                            ctx,
                            db,
                            &subject_type,
                            subject_id,
                            "trust_compile_failed",
                            Some(&format!("claim_id={} error={e}", claim.id)),
                        );
                        fatal_errors
                            .push(format!("claim_id={} trust compile failed: {e}", claim.id));
                        continue;
                    }
                };

                if let Err(e) = crate::services::claims::update_claim_trust(
                    db,
                    &claim.id,
                    computation.score,
                    trust_version,
                    ctx,
                ) {
                    report.claims_skipped += 1;
                    log::error!(
                        "TrustRecompute: update_claim_trust failed for claim {}; preserving prior signals: {}",
                        claim.id,
                        e
                    );
                    report.failures_recorded += record_trust_recompute_pipeline_failure(
                        ctx,
                        db,
                        &subject_type,
                        subject_id,
                        "trust_update_failed",
                        Some(&format!("claim_id={} error={e}", claim.id)),
                    );
                    fatal_errors.push(format!("claim_id={} trust update failed: {e}", claim.id));
                    continue;
                }

                report.claims_updated += 1;
                if previous_band != Some(computation.band) {
                    emit_claim_trust_changed_signal(
                        ctx,
                        db,
                        &subject_type,
                        subject_id,
                        &claim,
                        previous_score,
                        previous_band,
                        computation.score.value(),
                        computation.band,
                        trust_version,
                    );
                }
                emit_confidence_evidence_signals(
                    ctx,
                    db,
                    &subject_type,
                    subject_id,
                    &claim.id,
                    &computation.evidence,
                );
            }
        }
    }

    if !fatal_errors.is_empty() {
        return Err(format!(
            "claim trust recompute failed for {subject_type}:{subject_id}: {}",
            fatal_errors.join("; ")
        ));
    }

    Ok(report)
}

fn normalize_entity_type(entity_type: &str) -> String {
    entity_type
        .trim()
        .trim_end_matches('s')
        .to_ascii_lowercase()
}

fn claim_subject_ref_json_for_entity(entity_type: &str, entity_id: &str) -> Option<String> {
    let kind = match normalize_entity_type(entity_type).as_str() {
        "account" => "account",
        "meeting" => "meeting",
        "person" => "person",
        "project" => "project",
        _ => return None,
    };
    Some(serde_json::json!({ "kind": kind, "id": entity_id }).to_string())
}

fn trust_subject_from_claim_json(
    subject_ref: &str,
) -> Result<crate::abilities::provenance::SubjectRef, String> {
    let value: serde_json::Value =
        serde_json::from_str(subject_ref).map_err(|e| format!("not JSON: {e}"))?;

    if let Some(id) = value.get("account").and_then(|v| v.as_str()) {
        return Ok(crate::abilities::provenance::SubjectRef::Account(
            id.to_string(),
        ));
    }
    if let Some(id) = value.get("project").and_then(|v| v.as_str()) {
        return Ok(crate::abilities::provenance::SubjectRef::Project(
            id.to_string(),
        ));
    }
    if let Some(id) = value.get("person").and_then(|v| v.as_str()) {
        return Ok(crate::abilities::provenance::SubjectRef::Person(
            id.to_string(),
        ));
    }
    if let Some(id) = value.get("meeting").and_then(|v| v.as_str()) {
        return Ok(crate::abilities::provenance::SubjectRef::Meeting(
            id.to_string(),
        ));
    }
    if value.get("global").is_some() {
        return Ok(crate::abilities::provenance::SubjectRef::Global);
    }

    let kind = value
        .get("kind")
        .or_else(|| value.get("type"))
        .or_else(|| value.get("entity_type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing kind/type".to_string())?
        .to_ascii_lowercase();
    let id = value
        .get("id")
        .or_else(|| value.get("entity_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    match kind.as_str() {
        "account" | "accounts" => Ok(crate::abilities::provenance::SubjectRef::Account(id)),
        "project" | "projects" => Ok(crate::abilities::provenance::SubjectRef::Project(id)),
        "person" | "people" => Ok(crate::abilities::provenance::SubjectRef::Person(id)),
        "meeting" | "meetings" => Ok(crate::abilities::provenance::SubjectRef::Meeting(id)),
        "user" | "users" => Ok(crate::abilities::provenance::SubjectRef::User(id)),
        "global" => Ok(crate::abilities::provenance::SubjectRef::Global),
        other => Err(format!("unsupported subject kind/type '{other}'")),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TrustInput<T> {
    value: T,
    indeterminate_reason: Option<&'static str>,
}

impl<T> TrustInput<T> {
    pub(crate) fn ok(value: T) -> Self {
        Self {
            value,
            indeterminate_reason: None,
        }
    }

    pub(crate) fn indeterminate(value: T, reason: &'static str) -> Self {
        Self {
            value,
            indeterminate_reason: Some(reason),
        }
    }

    pub(crate) fn into_parts(self) -> (T, Option<&'static str>) {
        (self.value, self.indeterminate_reason)
    }
}

fn collect_trust_input<T>(input: TrustInput<T>, reasons: &mut Vec<&'static str>) -> T {
    let (value, reason) = input.into_parts();
    if let Some(r) = reason {
        reasons.push(r);
    }
    value
}

fn build_trust_context_for_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
    claim: &IntelligenceClaim,
    footprint: crate::abilities::trust::TargetFootprint,
    portfolio_footprints: Vec<crate::abilities::trust::EntityFootprint>,
) -> (crate::abilities::trust::TrustContext, Vec<&'static str>) {
    let mut indeterminate_reasons: Vec<&'static str> = Vec::new();

    let feedback_signal = collect_trust_input(
        trust_feedback_signal_for_claim(db, &claim.id),
        &mut indeterminate_reasons,
    );
    let corroborators = collect_trust_input(
        source_reliability_corroborators_for_claim(db, &claim.id),
        &mut indeterminate_reasons,
    );
    let contradiction_count = collect_trust_input(
        contradiction_count_for_claim(db, &claim.id),
        &mut indeterminate_reasons,
    );
    let corroboration_strength = collect_trust_input(
        corroboration_strength_for_claim(db, &claim.id),
        &mut indeterminate_reasons,
    );
    let source_reliability = collect_trust_input(
        source_reliability_for_claim(db, subject_type, claim),
        &mut indeterminate_reasons,
    );
    let now = ctx.clock.now();
    let freshness = collect_trust_input(
        freshness_context_for_claim(db, now, claim),
        &mut indeterminate_reasons,
    );
    let source_lifecycle = collect_trust_input(
        source_lifecycle_for_claim(claim),
        &mut indeterminate_reasons,
    );
    let internal_consistency = collect_trust_input(
        internal_consistency_for_claim(claim),
        &mut indeterminate_reasons,
    );
    let read_state_indeterminate = !indeterminate_reasons.is_empty();
    if read_state_indeterminate {
        log::warn!(
            "TrustRecompute: claim {} reads indeterminate, reasons: {}",
            claim.id,
            indeterminate_reasons.join(",")
        );
    }
    let trust_ctx = crate::abilities::trust::TrustContext {
        now,
        renewal_context: renewal_context_for_claim(now, db, subject_type, subject_id),
        config: crate::abilities::trust::TrustConfig::default(),
        factor_inputs: crate::abilities::trust::TrustFactorInputs {
            source_reliability,
            source_reliability_corroborators: corroborators,
            freshness,
            corroboration_strength,
            contradiction_count,
            user_feedback: feedback_signal,
            subject_fit_confidence: subject_fit_confidence_for_feedback(feedback_signal),
            internal_consistency,
            source_lifecycle,
            linear_issue_state: crate::abilities::trust::LinearIssueStateContext::default(),
            read_state_indeterminate,
        },
        cross_entity: crate::abilities::trust::CrossEntityCoherenceInput {
            claim_text: claim.text.clone(),
            target_footprint: footprint,
            portfolio_footprints,
            cross_entity_context_expected: cross_entity_context_expected(claim),
        },
        target_surface: None,
    };
    (trust_ctx, indeterminate_reasons)
}

fn renewal_context_for_claim(
    now: chrono::DateTime<Utc>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
) -> Option<crate::abilities::trust::RenewalContext> {
    if normalize_entity_type(subject_type) != "account" {
        return None;
    }

    let account = db.get_account(subject_id).ok().flatten()?;
    let contract_end = account.contract_end.as_deref()?;
    let renewal_date = chrono::NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
    let renewal_at = renewal_date.and_hms_opt(0, 0, 0)?.and_utc();
    let days_to_renewal = renewal_date
        .signed_duration_since(now.date_naive())
        .num_days();

    Some(crate::abilities::trust::RenewalContext {
        renewal_at: Some(renewal_at),
        days_to_renewal: Some(days_to_renewal),
    })
}

pub(crate) fn source_reliability_for_claim(
    db: &ActionDb,
    subject_type: &str,
    claim: &IntelligenceClaim,
) -> TrustInput<f64> {
    match claim_projection_signature_signal(db, &claim.id) {
        Ok(Some(signal))
            if signal
                == crate::services::projection_signing::PROJECTION_SIGNATURE_INVALID_SIGNAL =>
        {
            return TrustInput::indeterminate(0.2, "projection_signature_invalid");
        }
        Ok(Some(signal))
            if signal
                == crate::services::projection_signing::PROJECTION_SIGNATURE_RETIRED_KEY_SIGNAL =>
        {
            return TrustInput::indeterminate(0.6, "projection_signature_retired_key");
        }
        Ok(Some(_)) | Ok(None) => {}
        Err(e) => {
            log::warn!(
                "TrustRecompute: failed to read projection signature signal for {}: {e}",
                claim.id
            );
            return TrustInput::indeterminate(1.0, "projection_signature_signal_read_failed");
        }
    }

    match db.get_signal_weight(&claim.data_source, subject_type, "enrichment_quality") {
        Ok(Some((alpha, beta, _))) => {
            let denom = alpha + beta;
            if !alpha.is_finite()
                || !beta.is_finite()
                || alpha < 0.0
                || beta < 0.0
                || !denom.is_finite()
                || denom <= 0.0
            {
                log::warn!(
                    "TrustRecompute: malformed signal_weights row for source={} entity_type={} on {}: alpha={alpha} beta={beta}",
                    claim.data_source,
                    subject_type,
                    claim.id
                );
                return TrustInput::indeterminate(1.0, "source_reliability_malformed_components");
            }
            let value = alpha / denom;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                log::warn!(
                    "TrustRecompute: out-of-range source_reliability computed for {} (alpha={alpha} beta={beta} value={value})",
                    claim.id
                );
                return TrustInput::indeterminate(1.0, "source_reliability_out_of_range");
            }
            TrustInput::ok(value)
        }
        Ok(None) => TrustInput::ok(1.0),
        Err(e) => {
            log::warn!(
                "TrustRecompute: failed to read signal_weights for source={} entity_type={} on {}: {e}",
                claim.data_source,
                subject_type,
                claim.id
            );
            TrustInput::indeterminate(1.0, "source_reliability_read_failed")
        }
    }
}

fn claim_projection_signature_signal(
    db: &ActionDb,
    claim_id: &str,
) -> Result<Option<String>, rusqlite::Error> {
    db.conn_ref()
        .query_row(
            "SELECT signal_type
               FROM signal_events
              WHERE entity_type = ?2
                AND entity_id = ?1
                AND signal_type IN (?3, ?4)
              ORDER BY created_at DESC
              LIMIT 1",
            rusqlite::params![
                claim_id,
                "claim",
                crate::services::projection_signing::PROJECTION_SIGNATURE_INVALID_SIGNAL,
                crate::services::projection_signing::PROJECTION_SIGNATURE_RETIRED_KEY_SIGNAL,
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()
}

pub(crate) fn source_lifecycle_for_claim(
    claim: &IntelligenceClaim,
) -> TrustInput<crate::abilities::trust::SourceLifecycleState> {
    let mut indeterminate: Option<&'static str> = None;
    let lifecycle_field = match claim.metadata_json.as_deref() {
        None => None,
        Some(raw) => match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(value) => value
                .get("source_lifecycle_state")
                .or_else(|| value.get("source_lifecycle"))
                .or_else(|| value.get("lifecycle_state"))
                .cloned(),
            Err(e) => {
                log::warn!(
                    "TrustRecompute: malformed metadata_json on lifecycle read for {}: {e}",
                    claim.id
                );
                indeterminate = Some("source_lifecycle_metadata_malformed");
                None
            }
        },
    };

    let normalized_state = match &lifecycle_field {
        None => None,
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => {
            let normalized = s.trim().to_ascii_lowercase().replace('-', "_");
            if normalized.is_empty() {
                log::warn!(
                    "TrustRecompute: empty/whitespace lifecycle field for {}",
                    claim.id
                );
                indeterminate = Some("source_lifecycle_empty_field");
                None
            } else {
                Some(normalized)
            }
        }
        Some(other) => {
            log::warn!(
                "TrustRecompute: non-string lifecycle field for {}: {other}",
                claim.id
            );
            indeterminate = Some("source_lifecycle_non_string_field");
            None
        }
    };

    let lifecycle = match normalized_state.as_deref() {
        Some("withdrawn") => crate::abilities::trust::SourceLifecycleState::Withdrawn,
        Some("dismissed") | Some("user_dismissed") => {
            crate::abilities::trust::SourceLifecycleState::Dismissed
        }
        Some("active") => crate::abilities::trust::SourceLifecycleState::Active,
        Some(unknown) => {
            log::warn!(
                "TrustRecompute: unknown lifecycle value '{unknown}' for {}; routing through indeterminate gate",
                claim.id
            );
            indeterminate = Some("source_lifecycle_unknown_value");
            crate::abilities::trust::SourceLifecycleState::Active
        }
        None if matches!(&claim.claim_state, crate::db::claims::ClaimState::Withdrawn) => {
            crate::abilities::trust::SourceLifecycleState::Withdrawn
        }
        None => crate::abilities::trust::SourceLifecycleState::Active,
    };
    match indeterminate {
        Some(reason) => TrustInput::indeterminate(lifecycle, reason),
        None => TrustInput::ok(lifecycle),
    }
}

pub(crate) fn freshness_context_for_claim(
    db: &ActionDb,
    now: chrono::DateTime<Utc>,
    claim: &IntelligenceClaim,
) -> TrustInput<crate::abilities::trust::FreshnessContext> {
    let mut indeterminate: Option<&'static str> = None;
    let mut freshest_source_asof = None;

    if let Some(source_asof) = claim.source_asof.as_deref() {
        match chrono::DateTime::parse_from_rfc3339(source_asof) {
            Ok(parsed) => {
                freshest_source_asof = Some(parsed.with_timezone(&Utc));
            }
            Err(e) => {
                log::warn!(
                    "TrustRecompute: malformed source_asof on {}: {e}; falling back to observed_at",
                    claim.id
                );
                indeterminate = Some("freshness_source_asof_malformed");
            }
        }
    }

    match freshest_corroboration_source_asof(db, &claim.id) {
        TrustInput {
            value: Some(corroboration_source_asof),
            indeterminate_reason,
        } => {
            if freshest_source_asof
                .map(|current| corroboration_source_asof > current)
                .unwrap_or(true)
            {
                freshest_source_asof = Some(corroboration_source_asof);
            }
            if indeterminate_reason.is_some() {
                indeterminate = indeterminate_reason;
            }
        }
        TrustInput {
            value: None,
            indeterminate_reason: Some(reason),
        } => indeterminate = Some(reason),
        TrustInput {
            value: None,
            indeterminate_reason: None,
        } => {}
    }

    if let Some(source_asof) = freshest_source_asof {
        let value = crate::abilities::trust::FreshnessContext {
            timestamp_known: true,
            age_days: age_days(now, source_asof),
        };
        return match indeterminate {
            Some(reason) => TrustInput::indeterminate(value, reason),
            None => TrustInput::ok(value),
        };
    }

    for (label, fallback) in [
        ("observed_at", &claim.observed_at),
        ("created_at", &claim.created_at),
    ] {
        match chrono::DateTime::parse_from_rfc3339(fallback) {
            Ok(parsed) => {
                let value = crate::abilities::trust::FreshnessContext {
                    timestamp_known: false,
                    age_days: age_days(now, parsed.with_timezone(&Utc)),
                };
                return match indeterminate {
                    Some(reason) => TrustInput::indeterminate(value, reason),
                    None => TrustInput::ok(value),
                };
            }
            Err(e) => {
                log::warn!("TrustRecompute: malformed {label} on {}: {e}", claim.id);
                indeterminate = Some("freshness_fallback_malformed");
            }
        }
    }

    let value = crate::abilities::trust::FreshnessContext {
        timestamp_known: false,
        age_days: 0.0,
    };
    match indeterminate {
        Some(reason) => TrustInput::indeterminate(value, reason),
        None => TrustInput::ok(value),
    }
}

fn freshest_corroboration_source_asof(
    db: &ActionDb,
    claim_id: &str,
) -> TrustInput<Option<chrono::DateTime<Utc>>> {
    let mut stmt = match db
        .conn_ref()
        .prepare("SELECT source_asof FROM claim_corroborations WHERE claim_id = ?1")
    {
        Ok(stmt) => stmt,
        Err(e) => {
            log::warn!(
                "TrustRecompute: failed to prepare corroboration freshness read for {claim_id}: {e}"
            );
            return TrustInput::indeterminate(None, "freshness_corroboration_prepare_failed");
        }
    };
    let rows = match stmt.query_map(rusqlite::params![claim_id], |row| {
        row.get::<_, Option<String>>(0)
    }) {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!(
                "TrustRecompute: failed to read corroboration freshness for {claim_id}: {e}"
            );
            return TrustInput::indeterminate(None, "freshness_corroboration_query_failed");
        }
    };

    let mut freshest = None;
    let mut indeterminate = None;
    for row in rows {
        match row {
            Ok(Some(source_asof)) => match chrono::DateTime::parse_from_rfc3339(&source_asof) {
                Ok(parsed) => {
                    let parsed = parsed.with_timezone(&Utc);
                    if freshest.map(|current| parsed > current).unwrap_or(true) {
                        freshest = Some(parsed);
                    }
                }
                Err(e) => {
                    log::warn!(
                        "TrustRecompute: malformed corroboration source_asof on {claim_id}: {e}"
                    );
                    indeterminate = Some("freshness_corroboration_source_asof_malformed");
                }
            },
            Ok(None) => {}
            Err(e) => {
                log::warn!("TrustRecompute: malformed corroboration row for {claim_id}: {e}");
                indeterminate = Some("freshness_corroboration_row_decode_failed");
            }
        }
    }

    match indeterminate {
        Some(reason) => TrustInput::indeterminate(freshest, reason),
        None => TrustInput::ok(freshest),
    }
}

fn age_days(now: chrono::DateTime<Utc>, source_time: chrono::DateTime<Utc>) -> f64 {
    (now - source_time).num_seconds() as f64 / 86_400.0
}

pub(crate) fn corroboration_strength_for_claim(db: &ActionDb, claim_id: &str) -> TrustInput<f64> {
    let mut stmt = match db
        .conn_ref()
        .prepare("SELECT strength FROM claim_corroborations WHERE claim_id = ?1")
    {
        Ok(stmt) => stmt,
        Err(e) => {
            log::warn!("TrustRecompute: failed to prepare corroboration read for {claim_id}: {e}");
            return TrustInput::indeterminate(0.0, "corroboration_strength_prepare_failed");
        }
    };
    let strengths = match stmt.query_map(rusqlite::params![claim_id], |row| row.get::<_, f64>(0)) {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!("TrustRecompute: failed to read corroborations for {claim_id}: {e}");
            return TrustInput::indeterminate(0.0, "corroboration_strength_query_failed");
        }
    };

    let mut any = false;
    let mut had_error = false;
    let mut had_out_of_range = false;
    let mut miss_probability = 1.0;
    for strength in strengths {
        match strength {
            Ok(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {
                any = true;
                miss_probability *= 1.0 - value;
            }
            Ok(value) => {
                log::warn!(
                    "TrustRecompute: corroboration strength {value} out of [0.0, 1.0] for {claim_id}; clamping into noisy-OR but marking read indeterminate"
                );
                let clamped = value.clamp(0.0, 1.0);
                if clamped.is_finite() {
                    any = true;
                    miss_probability *= 1.0 - clamped;
                }
                had_out_of_range = true;
            }
            Err(e) => {
                log::warn!("TrustRecompute: malformed corroboration for {claim_id}: {e}");
                had_error = true;
            }
        }
    }

    let value = if any { 1.0 - miss_probability } else { 0.0 };
    if had_error {
        TrustInput::indeterminate(value, "corroboration_strength_row_decode_failed")
    } else if had_out_of_range {
        TrustInput::indeterminate(value, "corroboration_strength_out_of_range")
    } else {
        TrustInput::ok(value)
    }
}

pub(crate) fn source_reliability_corroborators_for_claim(
    db: &ActionDb,
    claim_id: &str,
) -> TrustInput<Vec<crate::abilities::trust::CorroboratorWeight>> {
    let mut out = Vec::new();
    let mut indeterminate: Option<&'static str> = None;

    match db
        .conn_ref()
        .prepare("SELECT strength FROM claim_corroborations WHERE claim_id = ?1")
    {
        Ok(mut stmt) => {
            match stmt.query_map(rusqlite::params![claim_id], |row| row.get::<_, f64>(0)) {
                Ok(rows) => {
                    for row in rows {
                        match row {
                            Ok(strength)
                                if strength.is_finite() && (0.0..=1.0).contains(&strength) =>
                            {
                                out.push(crate::abilities::trust::CorroboratorWeight {
                                    evidence_weight: strength,
                                    confirms: true,
                                });
                            }
                            Ok(strength) if strength.is_finite() => {
                                log::warn!(
                                    "TrustRecompute: corroborator strength {strength} out of [0.0, 1.0] for {claim_id}; clamping but marking read indeterminate"
                                );
                                out.push(crate::abilities::trust::CorroboratorWeight {
                                    evidence_weight: strength.clamp(0.0, 1.0),
                                    confirms: true,
                                });
                                indeterminate = Some("corroborators_strength_out_of_range");
                            }
                            Ok(other) => {
                                log::warn!(
                                    "TrustRecompute: discarding non-finite corroboration strength {other} for {claim_id}"
                                );
                                indeterminate = Some("corroborators_row_decode_failed");
                            }
                            Err(e) => {
                                log::warn!(
                                    "TrustRecompute: malformed corroboration row for {claim_id}: {e}"
                                );
                                indeterminate = Some("corroborators_row_decode_failed");
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "TrustRecompute: failed to query corroborations for {claim_id}: {e}"
                    );
                    indeterminate = Some("corroborators_query_failed");
                }
            }
        }
        Err(e) => {
            log::warn!("TrustRecompute: failed to prepare corroboration query for {claim_id}: {e}");
            indeterminate = Some("corroborators_prepare_failed");
        }
    }

    match db.conn_ref().query_row(
        "SELECT COUNT(*) FROM claim_contradictions \
         WHERE (primary_claim_id = ?1 OR contradicting_claim_id = ?1) \
           AND reconciled_at IS NULL \
           AND winner_claim_id IS NULL \
           AND merged_claim_id IS NULL",
        rusqlite::params![claim_id],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(count) => {
            for _ in 0..count.max(0) {
                out.push(crate::abilities::trust::CorroboratorWeight {
                    evidence_weight: 1.0,
                    confirms: false,
                });
            }
        }
        Err(e) => {
            log::warn!("TrustRecompute: failed to count contradictions for {claim_id}: {e}");
            indeterminate = Some("corroborators_contradiction_count_failed");
        }
    }

    match indeterminate {
        Some(reason) => TrustInput::indeterminate(out, reason),
        None => TrustInput::ok(out),
    }
}

pub(crate) fn internal_consistency_for_claim(claim: &IntelligenceClaim) -> TrustInput<f64> {
    let raw = match claim.metadata_json.as_deref() {
        Some(s) => s,
        None => return TrustInput::ok(1.0),
    };
    const MAX_METADATA_BYTES: usize = 64 * 1024;
    if raw.len() > MAX_METADATA_BYTES {
        log::warn!(
            "TrustRecompute: metadata_json oversized ({} bytes) for {}, defaulting internal_consistency to 1.0",
            raw.len(),
            claim.id
        );
        return TrustInput::indeterminate(1.0, "internal_consistency_metadata_oversized");
    }
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "TrustRecompute: malformed metadata_json for {}: {e}; defaulting internal_consistency to 1.0",
                claim.id
            );
            return TrustInput::indeterminate(1.0, "internal_consistency_metadata_malformed");
        }
    };
    let hint = value.get("internal_consistency");
    match hint.and_then(|v| v.as_f64()) {
        Some(v) if v.is_finite() && (0.0..=1.0).contains(&v) => TrustInput::ok(v),
        Some(v) => {
            log::warn!(
                "TrustRecompute: internal_consistency hint {v} out of range for {}; defaulting to 1.0",
                claim.id
            );
            TrustInput::indeterminate(1.0, "internal_consistency_hint_out_of_range")
        }
        None if hint.is_some() => {
            log::warn!(
                "TrustRecompute: internal_consistency hint not a number for {}; defaulting to 1.0",
                claim.id
            );
            TrustInput::indeterminate(1.0, "internal_consistency_hint_not_numeric")
        }
        None => TrustInput::ok(1.0),
    }
}

pub(crate) fn contradiction_count_for_claim(db: &ActionDb, claim_id: &str) -> TrustInput<u32> {
    match db.conn_ref().query_row(
        "SELECT COUNT(*) FROM claim_contradictions
         WHERE (primary_claim_id = ?1 OR contradicting_claim_id = ?1)
           AND reconciled_at IS NULL
           AND winner_claim_id IS NULL
           AND merged_claim_id IS NULL",
        rusqlite::params![claim_id],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(count) => TrustInput::ok(count.max(0) as u32),
        Err(e) => {
            log::warn!("TrustRecompute: failed to count contradictions for {claim_id}: {e}");
            TrustInput::indeterminate(0, "contradiction_count_query_failed")
        }
    }
}

pub(crate) fn trust_feedback_signal_for_claim(
    db: &ActionDb,
    claim_id: &str,
) -> TrustInput<crate::abilities::trust::UserFeedbackSignal> {
    let mut stmt = match db.conn_ref().prepare(
        "SELECT feedback_type FROM claim_feedback
         WHERE claim_id = ?1
         ORDER BY submitted_at DESC, rowid DESC",
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            log::warn!("TrustRecompute: failed to prepare feedback read for {claim_id}: {e}");
            return TrustInput::indeterminate(
                crate::abilities::trust::UserFeedbackSignal::None,
                "feedback_prepare_failed",
            );
        }
    };
    let rows = match stmt.query_map(rusqlite::params![claim_id], |row| row.get::<_, String>(0)) {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!("TrustRecompute: failed to read feedback for {claim_id}: {e}");
            return TrustInput::indeterminate(
                crate::abilities::trust::UserFeedbackSignal::None,
                "feedback_query_failed",
            );
        }
    };

    let mut had_row_error = false;
    for row in rows {
        match row {
            Ok(value) => {
                let signal = match value.as_str() {
                    "confirm_current" => {
                        Some(crate::abilities::trust::UserFeedbackSignal::Confirmed)
                    }
                    "mark_false" => Some(crate::abilities::trust::UserFeedbackSignal::Retracted),
                    "wrong_subject" => {
                        Some(crate::abilities::trust::UserFeedbackSignal::WrongSubject)
                    }
                    "mark_outdated" | "wrong_source" | "needs_nuance" => {
                        Some(crate::abilities::trust::UserFeedbackSignal::Corrected)
                    }
                    _ => None,
                };
                if let Some(s) = signal {
                    return if had_row_error {
                        TrustInput::indeterminate(s, "feedback_row_decode_failed")
                    } else {
                        TrustInput::ok(s)
                    };
                }
            }
            Err(e) => {
                log::warn!("TrustRecompute: malformed claim_feedback row for {claim_id}: {e}");
                had_row_error = true;
            }
        }
    }

    if had_row_error {
        TrustInput::indeterminate(
            crate::abilities::trust::UserFeedbackSignal::None,
            "feedback_row_decode_failed",
        )
    } else {
        TrustInput::ok(crate::abilities::trust::UserFeedbackSignal::None)
    }
}

fn subject_fit_confidence_for_feedback(
    feedback: crate::abilities::trust::UserFeedbackSignal,
) -> f64 {
    match feedback {
        crate::abilities::trust::UserFeedbackSignal::WrongSubject => 0.05,
        _ => 1.0,
    }
}

pub(crate) fn cross_entity_context_expected(claim: &IntelligenceClaim) -> bool {
    if claim
        .field_path
        .as_deref()
        .is_some_and(|path| path.contains("peer_benchmark"))
    {
        return true;
    }

    claim
        .metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| {
            value
                .get("cross_entity_context_expected")
                .and_then(|flag| flag.as_bool())
        })
        .unwrap_or(false)
}

fn trust_band_for_score(
    score: f64,
    config: &crate::abilities::trust::TrustConfig,
) -> Option<crate::abilities::trust::TrustBand> {
    if !score.is_finite() {
        return None;
    }
    if score >= config.likely_current_min {
        Some(crate::abilities::trust::TrustBand::LikelyCurrent)
    } else if score >= config.use_with_caution_min {
        Some(crate::abilities::trust::TrustBand::UseWithCaution)
    } else {
        Some(crate::abilities::trust::TrustBand::NeedsVerification)
    }
}

fn trust_band_label(band: crate::abilities::trust::TrustBand) -> &'static str {
    match band {
        crate::abilities::trust::TrustBand::LikelyCurrent => "likely_current",
        crate::abilities::trust::TrustBand::UseWithCaution => "use_with_caution",
        crate::abilities::trust::TrustBand::NeedsVerification => "needs_verification",
        crate::abilities::trust::TrustBand::Unscored => "unscored",
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_claim_trust_changed_signal(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
    claim: &IntelligenceClaim,
    previous_score: Option<f64>,
    previous_band: Option<crate::abilities::trust::TrustBand>,
    score: f64,
    band: crate::abilities::trust::TrustBand,
    trust_version: i64,
) {
    let payload = serde_json::json!({
        "claim_id": &claim.id,
        "from_score": previous_score,
        "to_score": score,
        "from_band": previous_band.map(trust_band_label),
        "to_band": trust_band_label(band),
        "trust_version": trust_version,
    })
    .to_string();

    if let Err(e) = crate::services::signals::emit_without_meeting_refresh(
        ctx,
        db,
        subject_type,
        subject_id,
        "ClaimTrustChanged",
        "trust_recompute",
        Some(&payload),
        1.0,
    ) {
        log::warn!(
            "TrustRecompute: failed to emit ClaimTrustChanged for claim {}: {}",
            claim.id,
            e
        );
    }
}

fn emit_confidence_evidence_signals(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
    claim_id: &str,
    evidence: &crate::abilities::trust::ConfidenceEvidence,
) {
    for factor in &evidence.factor_breakdown {
        let payload = serde_json::json!({
            "claim_id": claim_id,
            "score": evidence.score,
            "band_label": &evidence.band_label,
            "factor": factor,
            "caveats": &evidence.caveats,
        })
        .to_string();
        if let Err(e) = crate::services::signals::emit_without_meeting_refresh(
            ctx,
            db,
            subject_type,
            subject_id,
            "ConfidenceEvidence",
            "trust_recompute",
            Some(&payload),
            evidence.score,
        ) {
            log::warn!(
                "TrustRecompute: failed to emit ConfidenceEvidence for claim {claim_id}: {e}"
            );
        }
    }
}

fn record_trust_recompute_pipeline_failure(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
    error_type: &str,
    error_message: Option<&str>,
) -> usize {
    if let Err(e) = crate::services::mutations::record_pipeline_failure(
        ctx,
        db,
        "trust_recompute",
        Some(subject_id),
        Some(subject_type),
        error_type,
        error_message,
        0,
    ) {
        log::warn!(
            "TrustRecompute: failed to record pipeline failure for {}:{}: {}",
            subject_type,
            subject_id,
            e
        );
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    use super::*;
    use crate::db::claims::{ClaimSensitivity, TemporalScope};
    use crate::db::test_utils::test_db;
    use crate::intelligence::{IntelRisk, IntelligenceJson, ItemSource};
    use crate::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use crate::signals::propagation::PropagationEngine;

    const TS: &str = "2026-05-04T12:00:00+00:00";

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::new_live(clock, rng, ext)
    }

    fn fixed_ctx() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap()),
            SeedableRng::new(7),
            ExternalClients::default(),
        )
    }

    fn inserted_claim_id(result: CommittedClaim) -> String {
        match result {
            CommittedClaim::Inserted { claim } | CommittedClaim::Tombstoned { claim } => claim.id,
            other => panic!("expected inserted claim, got {other:?}"),
        }
    }

    fn seed_account(db: &ActionDb, account_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params![account_id, format!("Account {account_id}"), TS],
            )
            .expect("insert account");
    }

    fn seed_person(db: &ActionDb, person_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at)
                 VALUES (?1, 'person@example.com', 'Fixture Person', ?2)",
                params![person_id, TS],
            )
            .expect("insert person");
    }

    fn claim_proposal(
        entity_type: &str,
        entity_id: &str,
        claim_type: &str,
        text: &str,
    ) -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: serde_json::json!({ "kind": entity_type, "id": entity_id }).to_string(),
            claim_type: claim_type.to_string(),
            field_path: None,
            topic_key: None,
            text: text.to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test_source".to_string(),
            source_ref: None,
            source_asof: Some(TS.to_string()),
            observed_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        }
    }

    fn read_trust_columns(db: &ActionDb, claim_id: &str) -> (Option<f64>, Option<i64>) {
        db.conn_ref()
            .query_row(
                "SELECT trust_score, trust_version
                 FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read trust columns")
    }

    fn signal_count(db: &ActionDb, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE signal_type = ?1",
                params![signal_type],
                |row| row.get(0),
            )
            .expect("count signals")
    }

    fn invalidation_job_count(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM invalidation_jobs WHERE job_kind = 'claim_recompute'",
                [],
                |row| row.get(0),
            )
            .expect("count invalidation jobs")
    }

    #[test]
    fn generation_commit_enqueues_trust_recompute() {
        let db = test_db();
        let account_id = "acct-generated-trust-recompute";
        seed_account(&db, account_id);
        let (clock, rng, ext) = fixed_ctx();
        let ctx = test_ctx(&clock, &rng, &ext);
        let engine = PropagationEngine::default();
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: TS.to_string(),
            risks: vec![IntelRisk {
                render_policy: None,
                claim_id: None,
                text: "Renewal owner has not approved the deployment plan.".to_string(),
                item_source: Some(ItemSource {
                    source: "glean_crm".to_string(),
                    confidence: 0.9,
                    sourced_at: TS.to_string(),
                    reference: Some("CRM opportunity fixture".to_string()),
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        crate::services::intelligence::upsert_assessment_from_enrichment(
            &ctx, &db, &engine, "account", account_id, &intel,
        )
        .expect("commit generated intelligence");

        assert_eq!(
            invalidation_job_count(&db),
            1,
            "generated intelligence commits must enqueue trust recompute"
        );
    }

    #[test]
    fn freshness_context_uses_newer_corroboration_source_asof() {
        let db = test_db();
        let account_id = "acct-corroboration-freshness";
        seed_account(&db, account_id);
        let (clock, rng, ext) = fixed_ctx();
        let ctx = test_ctx(&clock, &rng, &ext);
        let text = "Renewal owner has not approved the deployment plan.";

        let mut first = claim_proposal("account", account_id, "risk", text);
        first.data_source = "glean_crm".to_string();
        first.source_asof = Some("2026-05-01T12:00:00+00:00".to_string());
        first.observed_at = "2026-05-01T12:00:00+00:00".to_string();
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, first).expect("commit claim"));

        let mut second = claim_proposal("account", account_id, "risk", text);
        second.data_source = "glean_crm".to_string();
        second.source_asof = Some("2026-05-02T12:00:00+00:00".to_string());
        second.observed_at = "2026-05-02T12:00:00+00:00".to_string();
        match commit_claim(&ctx, &db, second).expect("reinforce claim") {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, claim_id),
            other => panic!("same claim should reinforce, got {other:?}"),
        }

        let mut third = claim_proposal("account", account_id, "risk", text);
        third.data_source = "glean_crm".to_string();
        third.source_asof = Some("2026-05-03T12:00:00+00:00".to_string());
        third.observed_at = "2026-05-03T12:00:00+00:00".to_string();
        match commit_claim(&ctx, &db, third).expect("reinforce claim again") {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, claim_id),
            other => panic!("same claim should reinforce again, got {other:?}"),
        }

        let subject_ref = serde_json::json!({ "kind": "account", "id": account_id }).to_string();
        let active = crate::services::claims::load_claims_active(&db, &subject_ref, Some("risk"))
            .expect("load active claim");
        assert_eq!(active.len(), 1);
        assert_eq!(
            active[0].source_asof.as_deref(),
            Some("2026-05-01T12:00:00+00:00"),
            "the canonical claim row stays immutable"
        );

        let corroboration: (Option<String>, i64) = db
            .conn_ref()
            .query_row(
                "SELECT source_asof, reinforcement_count
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'glean_crm'",
                params![&claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read corroboration");
        assert_eq!(
            corroboration.0.as_deref(),
            Some("2026-05-03T12:00:00+00:00")
        );
        assert_eq!(corroboration.1, 2);

        let (freshness, reason) = freshness_context_for_claim(
            &db,
            Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
            &active[0],
        )
        .into_parts();
        assert_eq!(reason, None);
        assert!(freshness.timestamp_known);
        assert_eq!(freshness.age_days, 1.0);
    }

    #[test]
    fn recompute_scores_person_claim_with_generic_footprint() {
        let db = test_db();
        let person_id = "person-trust";
        seed_person(&db, person_id);
        let (clock, rng, ext) = fixed_ctx();
        let ctx = test_ctx(&clock, &rng, &ext);
        let claim_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                claim_proposal(
                    "person",
                    person_id,
                    "stakeholder_engagement",
                    "The stakeholder is actively engaged.",
                ),
            )
            .expect("commit person claim"),
        );

        let report =
            recompute_claim_trust_for_subject(&ctx, &db, "Person", person_id).expect("recompute");

        assert_eq!(report.claims_seen, 1);
        assert_eq!(report.subject_type, "person");
        assert_eq!(report.claims_updated, 1);
        let (score, version) = read_trust_columns(&db, &claim_id);
        assert!(score.is_some());
        assert_eq!(version, Some(1));
    }

    #[test]
    fn recompute_emits_change_signal_without_self_enqueueing_recompute() {
        let db = test_db();
        let account_id = "acct-unscored-signal";
        seed_account(&db, account_id);
        let (clock, rng, ext) = fixed_ctx();
        let ctx = test_ctx(&clock, &rng, &ext);
        let claim_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                claim_proposal(
                    "account",
                    account_id,
                    "risk",
                    "The account has a current renewal risk.",
                ),
            )
            .expect("commit account claim"),
        );

        recompute_claim_trust_for_subject(&ctx, &db, "account", account_id).expect("recompute");

        let (score, version) = read_trust_columns(&db, &claim_id);
        assert!(score.is_some());
        assert_eq!(version, Some(1));
        assert_eq!(signal_count(&db, "ClaimTrustChanged"), 1);
        assert_eq!(invalidation_job_count(&db), 0);
    }
}
