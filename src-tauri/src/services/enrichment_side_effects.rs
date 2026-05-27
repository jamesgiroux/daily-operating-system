//! Service-owned side effects for finalized enrichment output.
//!
//! The queue is an orchestrator. Durable projections derived from enrichment
//! output live behind this service so mutation gates, idempotency, and signal
//! emission stay centralized.

use sha2::{Digest, Sha256};

use crate::db::ActionDb;
use crate::intelligence::io::IntelligenceJson;
use crate::services::context::ServiceContext;
use crate::signals::propagation::PropagationEngine;

#[derive(Debug, Clone, Copy)]
pub struct EnrichmentSideEffectSource<'a> {
    pub commitment_source_label: &'a str,
    pub signal_source: &'a str,
    pub product_source: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichmentSideEffectProducer {
    Glean,
    Pty,
}

pub fn sync_account_enrichment_side_effects_for_producer(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    intel: &IntelligenceJson,
    producer: EnrichmentSideEffectProducer,
) -> Result<(), String> {
    let (commitment_source_label, signal_source, product_source) = match producer {
        EnrichmentSideEffectProducer::Glean => {
            (format!("glean_enrichment:{account_id}"), "glean", "glean")
        }
        EnrichmentSideEffectProducer::Pty => (
            format!("pty_enrichment:{account_id}"),
            "ai_enrichment",
            "ai_inference",
        ),
    };

    sync_account_enrichment_side_effects(
        ctx,
        db,
        engine,
        account_id,
        intel,
        EnrichmentSideEffectSource {
            commitment_source_label: &commitment_source_label,
            signal_source,
            product_source,
        },
    )
}

pub fn sync_account_enrichment_side_effects(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    intel: &IntelligenceJson,
    source: EnrichmentSideEffectSource<'_>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    sync_commitments(ctx, db, engine, account_id, intel, source)?;
    sync_products(ctx, db, engine, account_id, intel, source)?;
    Ok(())
}

fn sync_commitments(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    intel: &IntelligenceJson,
    source: EnrichmentSideEffectSource<'_>,
) -> Result<(), String> {
    let now = ctx.clock.now().to_rfc3339();
    let mut commitment_count = 0usize;
    let mut evidence_parts = Vec::new();

    if let Some(commitments) = intel.open_commitments.as_ref() {
        for commitment in commitments {
            let owner = commitment.owner.as_deref().unwrap_or("joint");
            let title = commitment.description.trim();
            if title.is_empty() {
                continue;
            }
            let target_date = commitment.due_date.as_deref();
            let id = stable_id(
                "captured-commitment",
                &[
                    account_id,
                    title,
                    owner,
                    target_date.unwrap_or(""),
                    source.commitment_source_label,
                ],
            );
            db.conn_ref()
                .execute(
                    "INSERT OR IGNORE INTO captured_commitments
                        (id, account_id, meeting_id, title, owner, target_date, confidence, source, consumed, created_at)
                     VALUES (?1, ?2, NULL, ?3, ?4, ?5, 'medium', ?6, 0, ?7)",
                    rusqlite::params![
                        id,
                        account_id,
                        title,
                        owner,
                        target_date,
                        source.commitment_source_label,
                        now,
                    ],
                )
                .map_err(|e| format!("insert enrichment commitment failed: {e}"))?;
            commitment_count += 1;
            evidence_parts.push(format!("{title}|{owner}|{}", target_date.unwrap_or("")));
        }
    }

    if let Some(signals) = intel.success_plan_signals.as_ref() {
        for objective in &signals.stated_objectives {
            let owner = objective.owner.as_deref().unwrap_or("joint");
            let title = objective.objective.trim();
            if title.is_empty() {
                continue;
            }
            let target_date = objective.target_date.as_deref();
            let id = stable_id(
                "captured-commitment",
                &[
                    account_id,
                    title,
                    owner,
                    target_date.unwrap_or(""),
                    source.commitment_source_label,
                ],
            );
            db.conn_ref()
                .execute(
                    "INSERT OR IGNORE INTO captured_commitments
                        (id, account_id, meeting_id, title, owner, target_date, confidence, source, consumed, created_at)
                     VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
                    rusqlite::params![
                        id,
                        account_id,
                        title,
                        owner,
                        target_date,
                        objective.confidence,
                        source.commitment_source_label,
                        now,
                    ],
                )
                .map_err(|e| format!("insert enrichment objective failed: {e}"))?;
            commitment_count += 1;
            evidence_parts.push(format!("{title}|{owner}|{}", target_date.unwrap_or("")));
        }
    }

    if commitment_count > 0 {
        evidence_parts.sort();
        let evidence_key = stable_id(
            "commitment-evidence",
            &evidence_parts
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );
        let value = serde_json::json!({
            "count": commitment_count,
            "source": source.commitment_source_label,
        })
        .to_string();
        crate::services::signals::emit_once_and_propagate(
            ctx,
            db,
            engine,
            &stable_id(
                "sig-enrichment-commitment",
                &[account_id, source.signal_source, &evidence_key],
            ),
            "account",
            account_id,
            "commitment_captured",
            source.signal_source,
            Some(&value),
            0.7,
        )
        .map(|_| ())
        .map_err(|e| format!("emit enrichment commitment signal failed: {e}"))?;
    }

    Ok(())
}

fn sync_products(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    account_id: &str,
    intel: &IntelligenceJson,
    source: EnrichmentSideEffectSource<'_>,
) -> Result<(), String> {
    let Some(adoption) = intel.product_adoption.as_ref() else {
        return Ok(());
    };

    let mut upserted = 0usize;
    let mut evidence_parts = Vec::new();

    for feature in &adoption.feature_adoption {
        let (name, adoption_pct) = if let Some(colon_pos) = feature.find(':') {
            let raw_name = feature[..colon_pos].trim();
            let pct_str = feature[colon_pos + 1..].trim().trim_end_matches('%');
            let pct = pct_str.parse::<f64>().ok().map(|v| v / 100.0);
            (raw_name.to_string(), pct)
        } else {
            (feature.trim().to_string(), None)
        };

        if name.is_empty() {
            continue;
        }

        db.upsert_account_product(
            account_id,
            &name,
            None,
            "active",
            adoption_pct,
            source.product_source,
            0.55,
            None,
        )
        .map_err(|e| format!("upsert enrichment account product failed: {e}"))?;
        upserted += 1;
        evidence_parts.push(format!("{name}|{}", adoption_pct.unwrap_or(-1.0)));
    }

    if upserted > 0 {
        evidence_parts.sort();
        let evidence_key = stable_id(
            "product-evidence",
            &evidence_parts
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );
        crate::services::signals::emit_once_and_propagate(
            ctx,
            db,
            engine,
            &stable_id(
                "sig-enrichment-product",
                &[account_id, source.product_source, &evidence_key],
            ),
            "account",
            account_id,
            "product_data_updated",
            source.product_source,
            Some(&format!("{{\"count\":{upserted}}}")),
            0.55,
        )
        .map(|_| ())
        .map_err(|e| format!("emit enrichment product signal failed: {e}"))?;
    }

    Ok(())
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{prefix}-{}", hex::encode(hasher.finalize()))
}
