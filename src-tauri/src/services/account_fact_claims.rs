//! Account fact promotion into the claim substrate.
//!
//! Account schema columns remain the compatibility projection for existing
//! readers, but claim-backed surfaces read `intelligence_claims`. This service
//! owns the bridge so Glean/MCP/WP/Tauri do not grow parallel account adapters.

use std::collections::HashMap;

use chrono::DateTime;

use crate::db::claims::ClaimSensitivity;
use crate::db::types::{AccountSourceRef, DbAccountSourceRef};
use crate::db::{ActionDb, DbAccount};
use crate::services::claims::{self, ClaimProposal};
use crate::services::context::ServiceContext;
use sha2::{Digest, Sha256};

const CLAIM_TYPE: &str = "account_fact";
const CLAIM_ACTOR: &str = "agent:account_fact_claims";
const USER_ACTOR: &str = "user:account_fact_claims";
const SYSTEM_ACTOR: &str = "system:account_fact_claims";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountFactPromotionReport {
    pub schema_promoted: u32,
    pub schema_skipped_lower_priority: u32,
    pub claims_committed: u32,
    pub claims_already_present: u32,
    pub recompute_jobs_enqueued: u32,
    pub source_ref_errors: Vec<String>,
    pub claim_errors: Vec<String>,
    pub recompute_enqueue_errors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountFactSourcePurgeReport {
    pub source_refs_masked: usize,
    pub claims_withdrawn: usize,
    pub schema_facts_cleared: usize,
    pub recompute_jobs_enqueued: usize,
}

impl AccountFactPromotionReport {
    fn merge(&mut self, outcome: AccountFactPromotionOutcome) {
        match outcome.schema_outcome {
            SchemaPromotionOutcome::Promoted => self.schema_promoted += 1,
            SchemaPromotionOutcome::SkippedLowerPriority => self.schema_skipped_lower_priority += 1,
            SchemaPromotionOutcome::NotWritten => {}
        }
        match outcome.claim_outcome {
            ClaimPromotionOutcome::Committed => self.claims_committed += 1,
            ClaimPromotionOutcome::AlreadyPresent => self.claims_already_present += 1,
            ClaimPromotionOutcome::NotAttempted => {}
        }
        self.source_ref_errors.extend(outcome.source_ref_errors);
        self.claim_errors.extend(outcome.claim_errors);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaPromotionOutcome {
    Promoted,
    SkippedLowerPriority,
    NotWritten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimPromotionOutcome {
    Committed,
    AlreadyPresent,
    NotAttempted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AccountFactPromotionOutcome {
    schema_outcome: SchemaPromotionOutcome,
    claim_outcome: ClaimPromotionOutcome,
    source_ref_errors: Vec<String>,
    claim_errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchemaFactPromotion {
    outcome: SchemaPromotionOutcome,
    reference_id: Option<String>,
}

impl AccountFactPromotionOutcome {
    fn schema_skipped() -> Self {
        Self {
            schema_outcome: SchemaPromotionOutcome::SkippedLowerPriority,
            claim_outcome: ClaimPromotionOutcome::NotAttempted,
            source_ref_errors: Vec::new(),
            claim_errors: Vec::new(),
        }
    }

    fn schema_error(error: String) -> Self {
        Self {
            schema_outcome: SchemaPromotionOutcome::NotWritten,
            claim_outcome: ClaimPromotionOutcome::NotAttempted,
            source_ref_errors: Vec::new(),
            claim_errors: vec![error],
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccountFactInput<'a> {
    pub account_id: &'a str,
    pub schema_field: &'a str,
    pub claim_field: &'a str,
    pub value: &'a str,
    pub source_system: &'a str,
    pub source_kind: &'a str,
    pub source_asof: Option<&'a str>,
    pub observed_at: &'a str,
    pub reference_id: Option<&'a str>,
}

#[derive(Debug, Clone)]
struct ExistingAccountFactInput<'a> {
    account_id: &'a str,
    claim_field: &'a str,
    value: String,
    source_system: String,
    source_kind: String,
    source_asof: Option<String>,
    observed_at: String,
    reference_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct AccountFactSourceInput<'a> {
    source_system: &'a str,
    source_kind: &'a str,
    observed_at: &'a str,
}

/// Promote high-confidence facts from Glean enrichment into account columns and
/// claim-backed facts. This replaces the previous provider-local schema-only
/// promotion path.
pub fn promote_glean_facts_from_intelligence(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    intel: &crate::intelligence::IntelligenceJson,
) -> AccountFactPromotionReport {
    let observed_at = intelligence_observed_at(ctx, intel);
    let mut report = AccountFactPromotionReport::default();
    if let Some(contract) = intel.contract_context.as_ref() {
        if let Some(arr) = contract.current_arr {
            let source = AccountFactSourceInput {
                source_system: "Salesforce",
                source_kind: "fact",
                observed_at: &observed_at,
            };
            report.merge(promote_account_arr_range(
                ctx, db, account_id, arr, arr, source,
            ));
        }
    }

    if let Some(outlook) = intel.agreement_outlook.as_ref() {
        if let Some(confidence) = outlook.confidence.as_deref() {
            let likelihood = match confidence.to_lowercase().as_str() {
                "high" => "0.85",
                "moderate" => "0.55",
                "low" => "0.25",
                _ => confidence,
            };
            report.merge(promote_account_fact(
                ctx,
                db,
                AccountFactInput {
                    account_id,
                    schema_field: "renewal_likelihood",
                    claim_field: "renewal_likelihood",
                    value: likelihood,
                    source_system: "Salesforce",
                    source_kind: "inference",
                    source_asof: None,
                    observed_at: &observed_at,
                    reference_id: None,
                },
            ));
        }
    }

    if let Some(org) = intel.org_health.as_ref() {
        let org_source_asof = non_empty_asof(&org.gathered_at);
        let org_observed_at = org_source_asof.unwrap_or(observed_at.as_str());
        if let Some(tier) = org.support_tier.as_deref() {
            report.merge(promote_account_fact(
                ctx,
                db,
                AccountFactInput {
                    account_id,
                    schema_field: "support_tier",
                    claim_field: "support_tier",
                    value: tier,
                    source_system: "zendesk",
                    source_kind: "fact",
                    source_asof: org_source_asof,
                    observed_at: org_observed_at,
                    reference_id: None,
                },
            ));
        }
        if let Some(likelihood) = org.renewal_likelihood.as_deref() {
            report.merge(promote_account_fact(
                ctx,
                db,
                AccountFactInput {
                    account_id,
                    schema_field: "renewal_likelihood",
                    claim_field: "renewal_likelihood",
                    value: likelihood,
                    source_system: "Salesforce",
                    source_kind: "fact",
                    source_asof: org_source_asof,
                    observed_at: org_observed_at,
                    reference_id: None,
                },
            ));
        }
        if let Some(stage) = org.customer_stage.as_deref() {
            report.merge(promote_account_fact(
                ctx,
                db,
                AccountFactInput {
                    account_id,
                    schema_field: "customer_status",
                    claim_field: "customer_status",
                    value: stage,
                    source_system: "Salesforce",
                    source_kind: "fact",
                    source_asof: org_source_asof,
                    observed_at: org_observed_at,
                    reference_id: None,
                },
            ));
        }
        if let Some(fit) = org.icp_fit.as_deref() {
            let score = score_from_label(fit);
            report.merge(promote_account_fact(
                ctx,
                db,
                AccountFactInput {
                    account_id,
                    schema_field: "icp_fit_score",
                    claim_field: "icp_fit_score",
                    value: &score,
                    source_system: "glean",
                    source_kind: "inference",
                    source_asof: org_source_asof,
                    observed_at: org_observed_at,
                    reference_id: None,
                },
            ));
        }
        if let Some(growth) = org.growth_tier.as_deref() {
            let score = score_from_label(growth);
            report.merge(promote_account_fact(
                ctx,
                db,
                AccountFactInput {
                    account_id,
                    schema_field: "growth_potential_score",
                    claim_field: "growth_potential_score",
                    value: &score,
                    source_system: "glean",
                    source_kind: "inference",
                    source_asof: org_source_asof,
                    observed_at: org_observed_at,
                    reference_id: None,
                },
            ));
        }
    }

    if let Some(classification) = intel.product_classification.as_ref() {
        if !classification.products.is_empty() {
            let count = classification.products.len().to_string();
            report.merge(promote_account_fact(
                ctx,
                db,
                AccountFactInput {
                    account_id,
                    schema_field: "active_subscription_count",
                    claim_field: "active_subscription_count",
                    value: &count,
                    source_system: "Salesforce",
                    source_kind: "fact",
                    source_asof: None,
                    observed_at: &observed_at,
                    reference_id: None,
                },
            ));

            let primary = classification
                .products
                .iter()
                .filter_map(|product| {
                    product
                        .type_
                        .as_ref()
                        .map(|kind| (kind.clone(), product.arr.unwrap_or(0.0)))
                })
                .max_by(|left, right| {
                    left.1
                        .partial_cmp(&right.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(kind, _)| kind);
            if let Some(product) = primary.as_deref() {
                report.merge(promote_account_fact(
                    ctx,
                    db,
                    AccountFactInput {
                        account_id,
                        schema_field: "primary_product",
                        claim_field: "primary_product",
                        value: product,
                        source_system: "Salesforce",
                        source_kind: "fact",
                        source_asof: None,
                        observed_at: &observed_at,
                        reference_id: None,
                    },
                ));
            }
        }
    }

    let claims_committed = report.claims_committed;
    enqueue_recompute_if_needed(
        ctx,
        db,
        &mut report,
        account_id,
        "glean_fact_promotion",
        claims_committed,
    );

    report
}

fn promote_account_fact(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: AccountFactInput<'_>,
) -> AccountFactPromotionOutcome {
    match db.with_transaction(|tx| {
        if has_newer_same_source_account_fact_ref(tx, &input)? {
            return Ok(AccountFactPromotionOutcome::schema_skipped());
        }
        let schema_blocked_by_user_claim = if is_source_less_schema_field(input.schema_field) {
            active_user_account_fact_claim_exists(tx, input.account_id, input.claim_field)?
        } else {
            active_user_claim_conflicts_with_input(
                tx,
                input.account_id,
                input.claim_field,
                input.value,
            )?
        };
        let schema = if schema_blocked_by_user_claim {
            SchemaFactPromotion {
                outcome: SchemaPromotionOutcome::SkippedLowerPriority,
                reference_id: None,
            }
        } else {
            upsert_schema_account_fact_atomic(tx, &input)?
        };
        let claim = ExistingAccountFactInput {
            account_id: input.account_id,
            claim_field: input.claim_field,
            value: input.value.to_string(),
            source_system: input.source_system.to_string(),
            source_kind: input.source_kind.to_string(),
            source_asof: input.source_asof.map(str::to_string),
            observed_at: input.observed_at.to_string(),
            reference_id: schema.reference_id.clone().or_else(|| {
                Some(stable_glean_account_fact_reference_id(
                    input.account_id,
                    input.schema_field,
                    input.source_system,
                    input.source_kind,
                    input.value,
                ))
            }),
        };
        let (claim_outcome, claim_errors) = commit_existing_account_fact_claim(ctx, tx, &claim);
        if !claim_errors.is_empty() {
            return Err(claim_errors.join("; "));
        }
        Ok(AccountFactPromotionOutcome {
            schema_outcome: schema.outcome,
            claim_outcome,
            source_ref_errors: Vec::new(),
            claim_errors: Vec::new(),
        })
    }) {
        Ok(outcome) => outcome,
        Err(error) => AccountFactPromotionOutcome {
            schema_outcome: SchemaPromotionOutcome::NotWritten,
            claim_outcome: ClaimPromotionOutcome::NotAttempted,
            source_ref_errors: Vec::new(),
            claim_errors: vec![format!(
                "{}.{} fact promotion rolled back: {error}",
                input.account_id, input.schema_field
            )],
        },
    }
}

fn upsert_schema_account_fact_atomic(
    db: &ActionDb,
    input: &AccountFactInput<'_>,
) -> Result<SchemaFactPromotion, String> {
    let promoted = db
        .upsert_account_fact(
            input.account_id,
            input.schema_field,
            input.value,
            input.source_system,
            input.observed_at,
        )
        .map_err(|error| {
            format!(
                "{}.{} schema promotion failed: {error}",
                input.account_id, input.schema_field
            )
        })?;
    if promoted {
        let reference_id = input.reference_id.map(str::to_string).unwrap_or_else(|| {
            stable_glean_account_fact_reference_id(
                input.account_id,
                input.schema_field,
                input.source_system,
                input.source_kind,
                input.value,
            )
        });
        db.upsert_account_source_ref(&AccountSourceRef {
            account_id: input.account_id,
            field: input.schema_field,
            source_system: input.source_system,
            source_kind: input.source_kind,
            source_value: Some(input.value),
            observed_at: input.observed_at,
            reference_id: Some(reference_id.as_str()),
        })
        .map_err(|error| {
            format!(
                "{}.{} source ref write failed: {error}",
                input.account_id, input.schema_field
            )
        })?;
        return Ok(SchemaFactPromotion {
            outcome: SchemaPromotionOutcome::Promoted,
            reference_id: Some(reference_id),
        });
    }
    Ok(SchemaFactPromotion {
        outcome: SchemaPromotionOutcome::SkippedLowerPriority,
        reference_id: None,
    })
}

fn promote_schema_only_fact_atomic(
    db: &ActionDb,
    input: AccountFactInput<'_>,
) -> Result<AccountFactPromotionOutcome, String> {
    match upsert_schema_account_fact_atomic(db, &input)?.outcome {
        SchemaPromotionOutcome::Promoted => Ok(AccountFactPromotionOutcome {
            schema_outcome: SchemaPromotionOutcome::Promoted,
            claim_outcome: ClaimPromotionOutcome::NotAttempted,
            source_ref_errors: Vec::new(),
            claim_errors: Vec::new(),
        }),
        SchemaPromotionOutcome::SkippedLowerPriority => {
            Ok(AccountFactPromotionOutcome::schema_skipped())
        }
        SchemaPromotionOutcome::NotWritten => Ok(AccountFactPromotionOutcome {
            schema_outcome: SchemaPromotionOutcome::NotWritten,
            claim_outcome: ClaimPromotionOutcome::NotAttempted,
            source_ref_errors: Vec::new(),
            claim_errors: Vec::new(),
        }),
    }
}

fn promote_account_arr_range(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    low: f64,
    high: f64,
    source: AccountFactSourceInput<'_>,
) -> AccountFactPromotionOutcome {
    let low_value = format!("{low:.0}");
    let high_value = format!("{high:.0}");
    match db.with_transaction(|tx| {
        let claim_value = if (low - high).abs() < f64::EPSILON {
            low_value.clone()
        } else {
            format!("{low_value}-{high_value}")
        };
        let low_input = AccountFactInput {
            account_id,
            schema_field: "arr_range_low",
            claim_field: "arr_range_low",
            value: &low_value,
            source_system: source.source_system,
            source_kind: source.source_kind,
            source_asof: None,
            observed_at: source.observed_at,
            reference_id: None,
        };
        let high_input = AccountFactInput {
            account_id,
            schema_field: "arr_range_high",
            claim_field: "arr_range_high",
            value: &high_value,
            source_system: source.source_system,
            source_kind: source.source_kind,
            source_asof: None,
            observed_at: source.observed_at,
            reference_id: None,
        };
        if has_newer_same_source_account_fact_ref(tx, &low_input)?
            || has_newer_same_source_account_fact_ref(tx, &high_input)?
        {
            return Ok(AccountFactPromotionOutcome::schema_skipped());
        }
        let skip_schema_for_user_claim =
            active_user_account_fact_claim_exists(tx, account_id, "arr")?;
        let low_schema = if skip_schema_for_user_claim {
            SchemaFactPromotion {
                outcome: SchemaPromotionOutcome::SkippedLowerPriority,
                reference_id: None,
            }
        } else {
            upsert_schema_account_fact_atomic(tx, &low_input)?
        };
        let high_schema = if skip_schema_for_user_claim {
            SchemaFactPromotion {
                outcome: SchemaPromotionOutcome::SkippedLowerPriority,
                reference_id: None,
            }
        } else {
            upsert_schema_account_fact_atomic(tx, &high_input)?
        };

        let mut outcome = AccountFactPromotionOutcome {
            schema_outcome: match (low_schema.outcome, high_schema.outcome) {
                (SchemaPromotionOutcome::Promoted, _) | (_, SchemaPromotionOutcome::Promoted) => {
                    SchemaPromotionOutcome::Promoted
                }
                (SchemaPromotionOutcome::SkippedLowerPriority, _)
                | (_, SchemaPromotionOutcome::SkippedLowerPriority) => {
                    SchemaPromotionOutcome::SkippedLowerPriority
                }
                _ => SchemaPromotionOutcome::NotWritten,
            },
            claim_outcome: ClaimPromotionOutcome::NotAttempted,
            source_ref_errors: Vec::new(),
            claim_errors: Vec::new(),
        };

        if matches!(
            outcome.schema_outcome,
            SchemaPromotionOutcome::Promoted | SchemaPromotionOutcome::SkippedLowerPriority
        ) {
            let refs = tx
                .get_account_source_refs(account_id)
                .map_err(|error| format!("{account_id}.arr source ref lookup failed: {error}"))?;
            let refs_by_field = latest_source_refs_by_field(refs);
            let reference_id = source_ref_for_field("arr", &refs_by_field)
                .map(account_source_ref_reference_id)
                .or_else(|| low_schema.reference_id.clone())
                .or_else(|| high_schema.reference_id.clone())
                .or_else(|| {
                    Some(stable_glean_account_fact_reference_id(
                        account_id,
                        "arr",
                        source.source_system,
                        source.source_kind,
                        &claim_value,
                    ))
                });
            let claim = ExistingAccountFactInput {
                account_id,
                claim_field: "arr",
                value: claim_value,
                source_system: source.source_system.to_string(),
                source_kind: source.source_kind.to_string(),
                source_asof: None,
                observed_at: source.observed_at.to_string(),
                reference_id,
            };
            let (claim_outcome, claim_errors) = commit_existing_account_fact_claim(ctx, tx, &claim);
            if !claim_errors.is_empty() {
                return Err(claim_errors.join("; "));
            }
            outcome.claim_outcome = claim_outcome;
        }

        Ok(outcome)
    }) {
        Ok(outcome) => outcome,
        Err(error) => AccountFactPromotionOutcome {
            schema_outcome: SchemaPromotionOutcome::NotWritten,
            claim_outcome: ClaimPromotionOutcome::NotAttempted,
            source_ref_errors: Vec::new(),
            claim_errors: vec![format!(
                "{account_id}.arr fact promotion rolled back: {error}"
            )],
        },
    }
}

/// Backfill claims for account facts that already exist in account schema
/// columns. Idempotent: exact active claim text at the same field path is not
/// committed again.
pub fn backfill_account_fact_claims(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<AccountFactPromotionReport, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let mut report = AccountFactPromotionReport::default();
    for account in db.get_all_accounts().map_err(|e| e.to_string())? {
        let mut account_claims_committed = 0;
        let refs = db
            .get_account_source_refs(&account.id)
            .map_err(|e| e.to_string())?;
        let refs_by_field = latest_source_refs_by_field(refs);
        let provenance = db
            .get_account_field_provenance(&account.id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|row| (row.field.clone(), row))
            .collect::<HashMap<_, _>>();
        let renewal_stage = db
            .get_account_renewal_stage(&account.id)
            .map_err(|e| e.to_string())?;

        for input in existing_account_fact_inputs(
            ctx,
            &account,
            &refs_by_field,
            &provenance,
            renewal_stage.as_deref(),
        ) {
            let (claim_outcome, claim_errors) = commit_existing_account_fact_claim(ctx, db, &input);
            match claim_outcome {
                ClaimPromotionOutcome::Committed => {
                    report.claims_committed += 1;
                    account_claims_committed += 1;
                }
                ClaimPromotionOutcome::AlreadyPresent => report.claims_already_present += 1,
                ClaimPromotionOutcome::NotAttempted => {}
            }
            report.claim_errors.extend(claim_errors);
        }

        let has_unscored = match account_has_unscored_account_fact_claims(db, &account.id) {
            Ok(value) => value,
            Err(error) => {
                report.claim_errors.push(format!(
                    "{} account_fact unscored scan failed: {error}",
                    account.id
                ));
                false
            }
        };
        if account_claims_committed > 0 || has_unscored {
            enqueue_recompute_if_needed(
                ctx,
                db,
                &mut report,
                &account.id,
                "account_fact_backfill",
                account_claims_committed,
            );
        }
    }
    Ok(report)
}

pub fn purge_glean_account_fact_projections_for_source_purge(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<AccountFactSourcePurgeReport, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;

    let refs = load_glean_account_fact_projection_refs(db)?;
    let mut affected_accounts: std::collections::HashSet<String> = refs
        .iter()
        .map(|source_ref| source_ref.account_id.clone())
        .collect();
    affected_accounts.extend(load_glean_account_fact_claim_accounts(db)?);

    let mut schema_facts_cleared = 0usize;
    for source_ref in &refs {
        schema_facts_cleared += clear_glean_schema_projection_if_current(db, source_ref)?;
    }

    let claims_withdrawn = claims::withdraw_glean_account_fact_claims_for_source_purge_in_tx(
        ctx, db,
    )
    .map_err(|e| format!("withdraw Glean account fact claims for source purge failed: {e}"))?;

    let source_refs_masked = db
        .conn_ref()
        .execute(
            "UPDATE account_source_refs
             SET source_system = 'purged:glean',
                 source_kind = 'source_purged',
                 source_value = NULL,
                 source_record_ref = NULL
             WHERE source_kind != 'source_purged'
               AND source_record_ref LIKE 'glean_account_fact:%'",
            [],
        )
        .map_err(|e| format!("mask Glean account source refs failed: {e}"))?;

    let mut recompute_jobs_enqueued = 0usize;
    if claims_withdrawn > 0 || schema_facts_cleared > 0 || source_refs_masked > 0 {
        let mut account_ids = affected_accounts.into_iter().collect::<Vec<_>>();
        account_ids.sort();
        for account_id in account_ids {
            match enqueue_account_fact_claim_recompute_in_tx(
                ctx,
                db,
                &account_id,
                "glean_source_purge",
                0,
            ) {
                Ok(true) => recompute_jobs_enqueued += 1,
                Ok(false) => {}
                Err(error) => {
                    record_recompute_enqueue_failure(
                        ctx,
                        db,
                        &account_id,
                        "source_purge_recompute_enqueue_failed",
                        &error,
                    );
                    return Err(error);
                }
            }
        }
    }

    Ok(AccountFactSourcePurgeReport {
        source_refs_masked,
        claims_withdrawn,
        schema_facts_cleared,
        recompute_jobs_enqueued,
    })
}

#[derive(Debug, Clone)]
struct GleanAccountFactProjectionRef {
    id: String,
    account_id: String,
    field: String,
    source_value: Option<String>,
    observed_at: String,
}

fn load_glean_account_fact_projection_refs(
    db: &ActionDb,
) -> Result<Vec<GleanAccountFactProjectionRef>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, account_id, field, source_value, observed_at
             FROM account_source_refs
             WHERE source_kind != 'source_purged'
               AND source_record_ref LIKE 'glean_account_fact:%'",
        )
        .map_err(|e| format!("prepare Glean source ref projection scan failed: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(GleanAccountFactProjectionRef {
                id: row.get(0)?,
                account_id: row.get(1)?,
                field: row.get(2)?,
                source_value: row.get(3)?,
                observed_at: row.get(4)?,
            })
        })
        .map_err(|e| format!("query Glean source ref projections failed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect Glean source ref projections failed: {e}"))
}

fn load_glean_account_fact_claim_accounts(db: &ActionDb) -> Result<Vec<String>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT DISTINCT json_extract(subject_ref, '$.id')
             FROM intelligence_claims
             WHERE claim_type = 'account_fact'
               AND source_ref LIKE 'glean_account_fact:%'
               AND json_valid(subject_ref) = 1
               AND lower(json_extract(subject_ref, '$.kind')) = 'account'",
        )
        .map_err(|e| format!("prepare Glean account fact claim account scan failed: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query Glean account fact claim accounts failed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect Glean account fact claim accounts failed: {e}"))
}

fn clear_glean_schema_projection_if_current(
    db: &ActionDb,
    source_ref: &GleanAccountFactProjectionRef,
) -> Result<usize, String> {
    let Some(source_value) = source_ref.source_value.as_deref() else {
        return Ok(0);
    };
    if has_newer_non_glean_account_source_ref(db, source_ref)? {
        return Ok(0);
    }
    if is_source_less_schema_field(&source_ref.field)
        && active_user_account_fact_claim_exists(
            db,
            &source_ref.account_id,
            claim_field_for_schema_projection(&source_ref.field),
        )?
    {
        return Ok(0);
    }
    let now = chrono::Utc::now().to_rfc3339();
    let (sql, value) = match source_ref.field.as_str() {
        "arr_range_low" => (
            "UPDATE accounts
             SET arr_range_low = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND arr_range_low IS NOT NULL
               AND printf('%.0f', arr_range_low) = ?3",
            source_value,
        ),
        "arr_range_high" => (
            "UPDATE accounts
             SET arr_range_high = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND arr_range_high IS NOT NULL
               AND printf('%.0f', arr_range_high) = ?3",
            source_value,
        ),
        "renewal_likelihood" => (
            "UPDATE accounts
             SET renewal_likelihood = NULL,
                 renewal_likelihood_source = NULL,
                 renewal_likelihood_updated_at = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND coalesce(renewal_likelihood_source, '') NOT IN ('user', 'user_correction', 'user_edit')
               AND renewal_likelihood IS NOT NULL
               AND ABS(renewal_likelihood - CAST(?3 AS REAL)) < 0.000001",
            source_value,
        ),
        "support_tier" => (
            "UPDATE accounts
             SET support_tier = NULL,
                 support_tier_source = NULL,
                 support_tier_updated_at = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND coalesce(support_tier_source, '') NOT IN ('user', 'user_correction', 'user_edit')
               AND support_tier = ?3",
            source_value,
        ),
        "customer_status" => (
            "UPDATE accounts
             SET customer_status = NULL,
                 customer_status_source = NULL,
                 customer_status_updated_at = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND coalesce(customer_status_source, '') NOT IN ('user', 'user_correction', 'user_edit')
               AND customer_status = ?3",
            source_value,
        ),
        "growth_potential_score" => (
            "UPDATE accounts
             SET growth_potential_score = NULL,
                 growth_potential_score_source = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND coalesce(growth_potential_score_source, '') NOT IN ('user', 'user_correction', 'user_edit')
               AND growth_potential_score IS NOT NULL
               AND ABS(growth_potential_score - CAST(?3 AS REAL)) < 0.000001",
            source_value,
        ),
        "icp_fit_score" => (
            "UPDATE accounts
             SET icp_fit_score = NULL,
                 icp_fit_score_source = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND coalesce(icp_fit_score_source, '') NOT IN ('user', 'user_correction', 'user_edit')
               AND icp_fit_score IS NOT NULL
               AND ABS(icp_fit_score - CAST(?3 AS REAL)) < 0.000001",
            source_value,
        ),
        "active_subscription_count" => (
            "UPDATE accounts
             SET active_subscription_count = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND active_subscription_count IS NOT NULL
               AND CAST(active_subscription_count AS TEXT) = ?3",
            source_value,
        ),
        "primary_product" => (
            "UPDATE accounts
             SET primary_product = NULL,
                 updated_at = ?1
             WHERE id = ?2
               AND primary_product = ?3",
            source_value,
        ),
        _ => return Ok(0),
    };
    db.conn_ref()
        .execute(sql, rusqlite::params![now, source_ref.account_id, value])
        .map_err(|e| {
            format!(
                "{}.{} Glean schema projection clear failed: {e}",
                source_ref.account_id, source_ref.field
            )
        })
}

fn has_newer_non_glean_account_source_ref(
    db: &ActionDb,
    source_ref: &GleanAccountFactProjectionRef,
) -> Result<bool, String> {
    let count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*)
             FROM account_source_refs
             WHERE account_id = ?1
               AND field = ?2
               AND source_kind != 'source_purged'
               AND observed_at > ?3
               AND id != ?4
               AND (source_record_ref IS NULL OR source_record_ref NOT LIKE 'glean_account_fact:%')",
            rusqlite::params![
                source_ref.account_id,
                source_ref.field,
                source_ref.observed_at,
                source_ref.id,
            ],
            |row| row.get(0),
        )
        .map_err(|e| {
            format!(
                "{}.{} newer source ref check failed: {e}",
                source_ref.account_id, source_ref.field
            )
        })?;
    Ok(count > 0)
}

fn enqueue_recompute_if_needed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    report: &mut AccountFactPromotionReport,
    account_id: &str,
    reason: &str,
    claims_committed: u32,
) {
    if claims_committed == 0
        && !account_has_unscored_account_fact_claims(db, account_id).unwrap_or(false)
    {
        return;
    }

    match enqueue_account_fact_claim_recompute(ctx, db, account_id, reason, claims_committed) {
        Ok(()) => report.recompute_jobs_enqueued += 1,
        Err(error) => {
            report.recompute_enqueue_errors.push(format!(
                "{account_id} account_fact recompute enqueue failed: {error}"
            ));
            record_recompute_enqueue_failure(
                ctx,
                db,
                account_id,
                "recompute_enqueue_failed",
                &error,
            );
        }
    }
}

fn enqueue_account_fact_claim_recompute(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    reason: &str,
    _claims_committed: u32,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        enqueue_account_fact_claim_recompute_in_tx(ctx, tx, account_id, reason, 0).map(|_| ())
    })
}

fn enqueue_account_fact_claim_recompute_in_tx(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    reason: &str,
    _claims_committed: u32,
) -> Result<bool, String> {
    let source_claim_version = db
        .current_subject_claim_version("account", account_id)
        .map_err(|error| format!("read account claim version failed: {error}"))?;
    let payload = serde_json::json!({
        "reason": reason,
        "source_claim_version": source_claim_version,
    })
    .to_string();
    let outcome = crate::services::signals::emit_once_for_key(
        ctx,
        db,
        &format!("account_fact_claims:{reason}:{account_id}:{source_claim_version}"),
        "account",
        account_id,
        "account_fact_claims_updated",
        "account_fact_claims",
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
        "account",
        account_id,
    )?;
    Ok(true)
}

fn account_has_unscored_account_fact_claims(
    db: &ActionDb,
    account_id: &str,
) -> Result<bool, crate::db::DbError> {
    db.conn_ref()
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                  FROM intelligence_claims
                 WHERE claim_type = ?1
                   AND claim_state = 'active'
                   AND surfacing_state = 'active'
                   AND trust_score IS NULL
                   AND json_valid(subject_ref) = 1
                   AND lower(json_extract(subject_ref, '$.kind')) = 'account'
                   AND json_extract(subject_ref, '$.id') = ?2
            )",
            rusqlite::params![CLAIM_TYPE, account_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(crate::db::DbError::Sqlite)
}

fn record_recompute_enqueue_failure(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    error_type: &str,
    error: &str,
) {
    if let Err(record_error) = crate::services::mutations::record_pipeline_failure(
        ctx,
        db,
        "account_fact_claims",
        Some(account_id),
        Some("account"),
        error_type,
        Some(error),
        0,
    ) {
        log::warn!(
            "account_fact_claims: failed to record recompute enqueue failure for {account_id}: {record_error}"
        );
    }
}

fn commit_existing_account_fact_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &ExistingAccountFactInput<'_>,
) -> (ClaimPromotionOutcome, Vec<String>) {
    if input.value.trim().is_empty() {
        return (ClaimPromotionOutcome::NotAttempted, Vec::new());
    }
    let subject_ref = serde_json::json!({
        "kind": "account",
        "id": input.account_id,
    })
    .to_string();
    let field_path = format!("account.{}", input.claim_field);
    let text = account_fact_text(input.claim_field, &input.value);
    let canonical_text = claims::normalize_claim_text(&text);

    let supersedes = match claims::load_claims_active(db, &subject_ref, Some(CLAIM_TYPE)) {
        Ok(claims) => {
            if claims.iter().any(|claim| {
                claim.field_path.as_deref() == Some(field_path.as_str())
                    && claim.text == canonical_text
                    && account_fact_claim_provenance_matches(claim, input)
            }) {
                return (ClaimPromotionOutcome::AlreadyPresent, Vec::new());
            }
            claims
                .iter()
                .find(|claim| {
                    claim.field_path.as_deref() == Some(field_path.as_str())
                        && can_supersede_existing_account_fact_claim(claim, input, &canonical_text)
                })
                .map(|claim| claim.id.clone())
        }
        Err(error) => {
            return (
                ClaimPromotionOutcome::NotAttempted,
                vec![format!(
                    "{}.{} claim preflight failed: {error}",
                    input.account_id, input.claim_field
                )],
            );
        }
    };

    let actor = actor_for_source(&input.source_system);
    let provenance_json = serde_json::json!({
        "source": {
            "system": input.source_system,
            "kind": input.source_kind,
            "sourceAsOf": input.source_asof,
            "observedAt": input.observed_at,
            "referenceId": input.reference_id,
        },
        "projection": {
            "schema": "accounts",
            "field": input.claim_field,
        }
    })
    .to_string();
    let metadata_json = serde_json::json!({
        "account_fact": {
            "field": input.claim_field,
            "raw_value": input.value,
            "source_system": input.source_system,
            "source_kind": input.source_kind,
        }
    })
    .to_string();

    let proposal = ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref,
        claim_type: CLAIM_TYPE.to_string(),
        field_path: Some(field_path),
        topic_key: Some(input.claim_field.to_string()),
        text,
        actor: actor.to_string(),
        data_source: input.source_system.clone(),
        source_ref: input.reference_id.clone(),
        source_asof: input.source_asof.clone(),
        observed_at: input.observed_at.clone(),
        provenance_json,
        metadata_json: Some(metadata_json),
        thread_id: None,
        temporal_scope: None,
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes,
        tombstone: None,
    };

    match claims::commit_claim(ctx, db, proposal) {
        Ok(_) => (ClaimPromotionOutcome::Committed, Vec::new()),
        Err(error) => (
            ClaimPromotionOutcome::NotAttempted,
            vec![format!(
                "{}.{} claim commit failed: {error}",
                input.account_id, input.claim_field
            )],
        ),
    }
}

fn intelligence_observed_at(
    ctx: &ServiceContext<'_>,
    intel: &crate::intelligence::IntelligenceJson,
) -> String {
    non_empty_asof(&intel.enriched_at)
        .map(str::to_string)
        .unwrap_or_else(|| ctx.clock.now().to_rfc3339())
}

fn active_user_claim_conflicts_with_input(
    db: &ActionDb,
    account_id: &str,
    claim_field: &str,
    value: &str,
) -> Result<bool, String> {
    let subject_ref = serde_json::json!({
        "kind": "account",
        "id": account_id,
    })
    .to_string();
    let field_path = format!("account.{claim_field}");
    let canonical_text = claims::normalize_claim_text(&account_fact_text(claim_field, value));
    let claims =
        claims::load_claims_active(db, &subject_ref, Some(CLAIM_TYPE)).map_err(|error| {
            format!("{account_id}.{claim_field} user-claim conflict scan failed: {error}")
        })?;
    Ok(claims.iter().any(|claim| {
        claim.field_path.as_deref() == Some(field_path.as_str())
            && is_user_authored_account_fact_claim(claim)
            && claim.text != canonical_text
    }))
}

fn active_user_account_fact_claim_exists(
    db: &ActionDb,
    account_id: &str,
    claim_field: &str,
) -> Result<bool, String> {
    let subject_ref = serde_json::json!({
        "kind": "account",
        "id": account_id,
    })
    .to_string();
    let field_path = format!("account.{claim_field}");
    let claims =
        claims::load_claims_active(db, &subject_ref, Some(CLAIM_TYPE)).map_err(|error| {
            format!("{account_id}.{claim_field} user-claim ownership scan failed: {error}")
        })?;
    Ok(claims.iter().any(|claim| {
        claim.field_path.as_deref() == Some(field_path.as_str())
            && is_user_authored_account_fact_claim(claim)
    }))
}

fn is_source_less_schema_field(field: &str) -> bool {
    matches!(
        field,
        "arr_range_low"
            | "arr_range_high"
            | "renewal_model"
            | "renewal_pricing_method"
            | "active_subscription_count"
            | "primary_product"
            | "commercial_stage"
            | "renewal_stage"
    )
}

fn claim_field_for_schema_projection(field: &str) -> &str {
    match field {
        "arr_range_low" | "arr_range_high" => "arr",
        other => other,
    }
}

fn can_supersede_existing_account_fact_claim(
    claim: &crate::db::claims::IntelligenceClaim,
    input: &ExistingAccountFactInput<'_>,
    canonical_text: &str,
) -> bool {
    if claim.text == canonical_text || is_user_authored_account_fact_claim(claim) {
        return false;
    }

    claim
        .data_source
        .eq_ignore_ascii_case(input.source_system.as_str())
        && account_fact_claim_source_kind(claim).as_deref() == Some(input.source_kind.as_str())
        && !timestamp_after(
            claim.source_asof.as_deref().unwrap_or(&claim.observed_at),
            input
                .source_asof
                .as_deref()
                .unwrap_or(input.observed_at.as_str()),
        )
}

fn has_newer_same_source_account_fact_ref(
    db: &ActionDb,
    input: &AccountFactInput<'_>,
) -> Result<bool, String> {
    let incoming_observed_at = input.observed_at;
    let refs = db
        .get_account_source_refs(input.account_id)
        .map_err(|error| {
            format!(
                "{}.{} source freshness scan failed: {error}",
                input.account_id, input.schema_field
            )
        })?;
    Ok(refs.iter().any(|source_ref| {
        source_ref.field == input.schema_field
            && source_ref
                .source_system
                .eq_ignore_ascii_case(input.source_system)
            && source_ref
                .source_kind
                .eq_ignore_ascii_case(input.source_kind)
            && timestamp_after(source_ref.observed_at.as_str(), incoming_observed_at)
    }))
}

fn timestamp_after(left: &str, right: &str) -> bool {
    match (
        DateTime::parse_from_rfc3339(left),
        DateTime::parse_from_rfc3339(right),
    ) {
        (Ok(left), Ok(right)) => left > right,
        _ => left > right,
    }
}

pub(crate) fn has_active_account_fact_tombstone(
    db: &ActionDb,
    account_id: &str,
    claim_field: &str,
) -> Result<bool, String> {
    let field_path = format!("account.{claim_field}");
    db.conn_ref()
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                  FROM intelligence_claims ic
                 WHERE ic.claim_state = 'tombstoned'
                   AND ic.claim_type = ?1
                   AND ic.field_path = ?2
                   AND json_valid(ic.subject_ref) = 1
                   AND lower(json_extract(ic.subject_ref, '$.kind')) = 'account'
                   AND json_extract(ic.subject_ref, '$.id') = ?3
                   AND (ic.expires_at IS NULL OR ic.expires_at > datetime('now'))
            )",
            rusqlite::params![CLAIM_TYPE, field_path, account_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| {
            format!("{account_id}.{claim_field} active tombstone scan failed: {error}")
        })
}

fn is_user_authored_account_fact_claim(claim: &crate::db::claims::IntelligenceClaim) -> bool {
    claim.actor.trim().to_ascii_lowercase().starts_with("user:")
        || claim
            .data_source
            .trim()
            .to_ascii_lowercase()
            .starts_with("user")
}

fn account_fact_claim_provenance_matches(
    claim: &crate::db::claims::IntelligenceClaim,
    input: &ExistingAccountFactInput<'_>,
) -> bool {
    claim.data_source == input.source_system
        && claim.source_ref.as_deref() == input.reference_id.as_deref()
        && input
            .source_asof
            .as_deref()
            .map(|source_asof| claim.source_asof.as_deref() == Some(source_asof))
            .unwrap_or(true)
        && account_fact_claim_source_kind(claim).as_deref() == Some(input.source_kind.as_str())
}

fn account_fact_claim_source_kind(claim: &crate::db::claims::IntelligenceClaim) -> Option<String> {
    claim
        .metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| {
            value
                .get("account_fact")
                .and_then(|fact| fact.get("source_kind"))
                .and_then(|kind| kind.as_str())
                .map(str::to_string)
        })
        .or_else(|| {
            serde_json::from_str::<serde_json::Value>(&claim.provenance_json)
                .ok()
                .and_then(|value| {
                    value
                        .get("source")
                        .and_then(|source| source.get("kind"))
                        .and_then(|kind| kind.as_str())
                        .map(str::to_string)
                })
        })
}

fn latest_source_refs_by_field(
    refs: Vec<DbAccountSourceRef>,
) -> HashMap<String, DbAccountSourceRef> {
    let mut by_field = HashMap::new();
    for source_ref in refs {
        by_field
            .entry(source_ref.field.clone())
            .or_insert(source_ref);
    }
    by_field
}

fn existing_account_fact_inputs<'a>(
    ctx: &ServiceContext<'_>,
    account: &'a DbAccount,
    refs_by_field: &HashMap<String, DbAccountSourceRef>,
    provenance: &HashMap<String, crate::db::types::DbAccountFieldProvenance>,
    renewal_stage: Option<&'a str>,
) -> Vec<ExistingAccountFactInput<'a>> {
    let mut facts = Vec::new();

    if account.arr_range_low.is_none() && account.arr_range_high.is_none() {
        push_number_fact(
            &mut facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            "arr",
            account.arr,
        );
    }
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "lifecycle",
        account.lifecycle.as_deref(),
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "contract_start",
        account.contract_start.as_deref(),
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "contract_end",
        account.contract_end.as_deref(),
    );
    if let Some(nps) = account.nps {
        push_value_fact(
            &mut facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            "nps",
            nps.to_string(),
        );
    }
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "commercial_stage",
        account.commercial_stage.as_deref(),
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "renewal_stage",
        renewal_stage,
    );

    match (account.arr_range_low, account.arr_range_high) {
        (Some(low), Some(high)) if (low - high).abs() < f64::EPSILON => push_value_fact(
            &mut facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            "arr",
            format!("{low:.0}"),
        ),
        (Some(low), Some(high)) => push_value_fact(
            &mut facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            "arr",
            format!("{low:.0}-{high:.0}"),
        ),
        (Some(low), None) => push_value_fact(
            &mut facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            "arr_range_low",
            format!("{low:.0}"),
        ),
        (None, Some(high)) => push_value_fact(
            &mut facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            "arr_range_high",
            format!("{high:.0}"),
        ),
        (None, None) => {}
    }

    push_number_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "renewal_likelihood",
        account.renewal_likelihood,
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "renewal_model",
        account.renewal_model.as_deref(),
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "renewal_pricing_method",
        account.renewal_pricing_method.as_deref(),
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "support_tier",
        account.support_tier.as_deref(),
    );
    if let Some(count) = account.active_subscription_count {
        push_value_fact(
            &mut facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            "active_subscription_count",
            count.to_string(),
        );
    }
    push_number_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "growth_potential_score",
        account.growth_potential_score,
    );
    push_number_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "icp_fit_score",
        account.icp_fit_score,
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "primary_product",
        account.primary_product.as_deref(),
    );
    push_text_fact(
        &mut facts,
        ctx,
        account,
        refs_by_field,
        provenance,
        "customer_status",
        account.customer_status.as_deref(),
    );

    facts
}

fn push_text_fact<'a>(
    facts: &mut Vec<ExistingAccountFactInput<'a>>,
    ctx: &ServiceContext<'_>,
    account: &'a DbAccount,
    refs_by_field: &HashMap<String, DbAccountSourceRef>,
    provenance: &HashMap<String, crate::db::types::DbAccountFieldProvenance>,
    field: &'a str,
    value: Option<&str>,
) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        push_value_fact(
            facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            field,
            value.to_string(),
        );
    }
}

fn push_number_fact<'a>(
    facts: &mut Vec<ExistingAccountFactInput<'a>>,
    ctx: &ServiceContext<'_>,
    account: &'a DbAccount,
    refs_by_field: &HashMap<String, DbAccountSourceRef>,
    provenance: &HashMap<String, crate::db::types::DbAccountFieldProvenance>,
    field: &'a str,
    value: Option<f64>,
) {
    if let Some(value) = value {
        push_value_fact(
            facts,
            ctx,
            account,
            refs_by_field,
            provenance,
            field,
            format!("{value:.4}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string(),
        );
    }
}

fn push_value_fact<'a>(
    facts: &mut Vec<ExistingAccountFactInput<'a>>,
    ctx: &ServiceContext<'_>,
    account: &'a DbAccount,
    refs_by_field: &HashMap<String, DbAccountSourceRef>,
    provenance: &HashMap<String, crate::db::types::DbAccountFieldProvenance>,
    field: &'a str,
    value: String,
) {
    if let Some(input) = existing_input(ctx, account, refs_by_field, provenance, field, value) {
        facts.push(input);
    }
}

fn existing_input<'a>(
    ctx: &ServiceContext<'_>,
    account: &'a DbAccount,
    refs_by_field: &HashMap<String, DbAccountSourceRef>,
    provenance: &HashMap<String, crate::db::types::DbAccountFieldProvenance>,
    field: &'a str,
    value: String,
) -> Option<ExistingAccountFactInput<'a>> {
    let source_ref = source_ref_for_field(field, refs_by_field);
    let provenance = provenance.get(field);
    let source_system = provenance
        .map(|row| row.source.clone())
        .or_else(|| source_ref.map(|row| row.source_system.clone()))
        .unwrap_or_else(|| "account_schema".to_string());
    let source_kind = source_ref
        .map(|row| row.source_kind.clone())
        .unwrap_or_else(|| {
            if provenance.is_some() {
                "fact".to_string()
            } else {
                "schema_snapshot".to_string()
            }
        });
    let reference_id = source_ref.map(account_source_ref_reference_id).or_else(|| {
        Some(stable_account_fact_reference_id(
            account.id.as_str(),
            field,
            &source_system,
            &source_kind,
            &value,
            None,
        ))
    });
    Some(ExistingAccountFactInput {
        account_id: account.id.as_str(),
        claim_field: field,
        value,
        source_system,
        source_kind,
        source_asof: None,
        observed_at: provenance
            .and_then(|row| row.updated_at.clone())
            .or_else(|| source_ref.map(|row| row.observed_at.clone()))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                if account.updated_at.trim().is_empty() {
                    ctx.clock.now().to_rfc3339()
                } else {
                    account.updated_at.clone()
                }
            }),
        reference_id,
    })
}

fn source_ref_for_field<'a>(
    field: &str,
    refs_by_field: &'a HashMap<String, DbAccountSourceRef>,
) -> Option<&'a DbAccountSourceRef> {
    refs_by_field.get(field).or_else(|| {
        if field == "arr" {
            refs_by_field
                .get("arr_range_low")
                .or_else(|| refs_by_field.get("arr_range_high"))
        } else {
            None
        }
    })
}

fn account_source_ref_reference_id(source_ref: &DbAccountSourceRef) -> String {
    stable_account_fact_reference_id(
        &source_ref.account_id,
        &source_ref.field,
        &source_ref.source_system,
        &source_ref.source_kind,
        source_ref.source_value.as_deref().unwrap_or(""),
        source_ref.source_record_ref.as_deref(),
    )
}

fn stable_account_fact_reference_id(
    account_id: &str,
    field: &str,
    source_system: &str,
    source_kind: &str,
    value: &str,
    source_record_ref: Option<&str>,
) -> String {
    if let Some(source_record_ref) = source_record_ref.filter(|value| !value.trim().is_empty()) {
        return source_record_ref.to_string();
    }

    let mut hasher = Sha256::new();
    for component in [account_id, field, source_system, source_kind, value] {
        hasher.update((component.len() as u64).to_be_bytes());
        hasher.update(component.as_bytes());
    }
    let digest = hasher.finalize();
    format!("account_fact:{}", hex::encode(&digest[..16]))
}

fn stable_glean_account_fact_reference_id(
    account_id: &str,
    field: &str,
    source_system: &str,
    source_kind: &str,
    value: &str,
) -> String {
    let mut hasher = Sha256::new();
    for component in [account_id, field, source_system, source_kind, value] {
        hasher.update((component.len() as u64).to_be_bytes());
        hasher.update(component.as_bytes());
    }
    let digest = hasher.finalize();
    format!("glean_account_fact:{}", hex::encode(&digest[..16]))
}

fn account_fact_text(field: &str, raw_value: &str) -> String {
    match field {
        "arr" => format!("ARR: {}", display_number_or_range(raw_value)),
        "arr_range_low" => format!("ARR lower bound: {}", display_number(raw_value)),
        "arr_range_high" => format!("ARR upper bound: {}", display_number(raw_value)),
        "renewal_likelihood" => format!("Renewal likelihood: {}", display_percent(raw_value)),
        "growth_potential_score" => {
            format!("Growth potential score: {}", display_score(raw_value))
        }
        "icp_fit_score" => format!("ICP fit score: {}", display_score(raw_value)),
        "active_subscription_count" => {
            format!("Active subscriptions: {}", raw_value.trim())
        }
        other => format!("{}: {}", label_for_field(other), raw_value.trim()),
    }
}

fn non_empty_asof(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn label_for_field(field: &str) -> &'static str {
    match field {
        "lifecycle" => "Lifecycle",
        "health" => "Health",
        "contract_start" => "Contract start",
        "contract_end" => "Contract end",
        "commercial_stage" => "Commercial stage",
        "renewal_stage" => "Renewal stage",
        "renewal_model" => "Renewal model",
        "renewal_pricing_method" => "Renewal pricing method",
        "support_tier" => "Support tier",
        "primary_product" => "Primary product",
        "customer_status" => "Customer status",
        "nps" => "NPS",
        _ => "Account fact",
    }
}

fn display_number_or_range(raw: &str) -> String {
    if let Some((low, high)) = raw.split_once('-') {
        return format!("{}-{}", display_number(low), display_number(high));
    }
    display_number(raw)
}

fn display_number(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.parse::<f64>() {
        Ok(value) if value.is_finite() => add_commas(value.round() as i64),
        _ => trimmed.to_string(),
    }
}

fn display_percent(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.parse::<f64>() {
        Ok(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {
            format!("{:.0}%", value * 100.0)
        }
        Ok(value) if value.is_finite() => format!("{:.0}%", value),
        _ => trimmed.to_string(),
    }
}

fn display_score(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.parse::<f64>() {
        Ok(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {
            format!("{:.0}/100", value * 100.0)
        }
        Ok(value) if value.is_finite() => format!("{:.0}/100", value),
        _ => trimmed.to_string(),
    }
}

fn add_commas(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let mut formatted = out.chars().rev().collect::<String>();
    if negative {
        formatted.insert(0, '-');
    }
    formatted
}

fn score_from_label(value: &str) -> String {
    match value.to_lowercase().as_str() {
        "strong" | "high" => "0.85".to_string(),
        "moderate" | "medium" => "0.55".to_string(),
        "weak" | "low" => "0.25".to_string(),
        _ => value.to_string(),
    }
}

fn actor_for_source(source: &str) -> &'static str {
    let normalized = source.trim().to_ascii_lowercase();
    if normalized.starts_with("user") {
        USER_ACTOR
    } else if normalized.starts_with("system") || normalized == "account_schema" {
        SYSTEM_ACTOR
    } else {
        CLAIM_ACTOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::services::claims::{
        load_claims_active, load_entity_context_claims_active_for_surface,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    fn test_ctx<'a>(
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
            updated_at: "2026-05-20T00:00:00Z".to_string(),
            ..Default::default()
        }
    }

    fn claim_recompute_job_count(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM invalidation_jobs WHERE job_kind = 'claim_recompute'",
                [],
                |row| row.get(0),
            )
            .expect("count claim recompute jobs")
    }

    #[test]
    fn promote_account_fact_writes_schema_source_ref_and_claim() {
        let db = test_db();
        db.upsert_account(&account("acct-fact")).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let outcome = promote_account_fact(
            &ctx,
            &db,
            AccountFactInput {
                account_id: "acct-fact",
                schema_field: "renewal_likelihood",
                claim_field: "renewal_likelihood",
                value: "0.85",
                source_system: "glean",
                source_kind: "inference",
                source_asof: Some("2026-05-20T12:00:00Z"),
                observed_at: "2026-05-22T12:00:00Z",
                reference_id: None,
            },
        );

        assert_eq!(outcome.schema_outcome, SchemaPromotionOutcome::Promoted);
        assert_eq!(outcome.claim_outcome, ClaimPromotionOutcome::Committed);
        let stored = db.get_account("acct-fact").unwrap().unwrap();
        assert_eq!(stored.renewal_likelihood, Some(0.85));
        assert_eq!(stored.renewal_likelihood_source.as_deref(), Some("glean"));
        let source_refs = db.get_account_source_refs("acct-fact").unwrap();
        assert_eq!(source_refs.len(), 1);
        let source_ref_id = stable_glean_account_fact_reference_id(
            "acct-fact",
            "renewal_likelihood",
            "glean",
            "inference",
            "0.85",
        );
        let source_asof: String = db
            .conn_ref()
            .query_row(
                "SELECT source_asof FROM intelligence_claims WHERE claim_type = 'account_fact'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_asof, "2026-05-20T12:00:00Z");

        let subject_ref = serde_json::json!({"kind": "account", "id": "acct-fact"}).to_string();
        let active_claims = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        let field_claims: Vec<_> = active_claims
            .iter()
            .filter(|claim| claim.field_path.as_deref() == Some("account.renewal_likelihood"))
            .collect();
        assert_eq!(field_claims.len(), 1);
        assert_eq!(field_claims[0].text, "renewal likelihood: 85%");
        assert!(field_claims[0].trust_score.is_none());
        assert_eq!(
            field_claims[0].source_asof.as_deref(),
            Some("2026-05-20T12:00:00Z")
        );
        assert_eq!(
            field_claims[0].source_ref.as_deref(),
            Some(source_ref_id.as_str())
        );

        let backfill = backfill_account_fact_claims(&ctx, &db).unwrap();
        assert_eq!(backfill.claims_committed, 0);
        let active_claims_after = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        let field_claims_after: Vec<_> = active_claims_after
            .iter()
            .filter(|claim| claim.field_path.as_deref() == Some("account.renewal_likelihood"))
            .collect();
        assert_eq!(field_claims_after.len(), 1);
        assert_eq!(
            field_claims_after[0].source_ref.as_deref(),
            Some(source_ref_id.as_str())
        );

        let repeated = promote_account_fact(
            &ctx,
            &db,
            AccountFactInput {
                account_id: "acct-fact",
                schema_field: "renewal_likelihood",
                claim_field: "renewal_likelihood",
                value: "0.85",
                source_system: "glean",
                source_kind: "inference",
                source_asof: Some("2026-05-20T12:00:00Z"),
                observed_at: "2026-05-22T12:00:00Z",
                reference_id: None,
            },
        );
        assert_eq!(
            repeated.claim_outcome,
            ClaimPromotionOutcome::AlreadyPresent
        );
        let active_claims_after_repeat =
            load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        assert_eq!(
            active_claims_after_repeat
                .iter()
                .filter(|claim| claim.field_path.as_deref() == Some("account.renewal_likelihood"))
                .count(),
            1
        );

        let claims = load_entity_context_claims_active_for_surface(
            &db,
            "account",
            "acct-fact",
            1,
            "mcp_tool",
        )
        .unwrap();
        assert!(claims.iter().any(|claim| {
            claim.claim_type == CLAIM_TYPE
                && claim.field_path.as_deref() == Some("account.renewal_likelihood")
                && claim.text == "renewal likelihood: 85%"
        }));
    }

    #[test]
    fn promote_glean_facts_carries_org_gathered_at_into_claim_source_asof() {
        let db = test_db();
        db.upsert_account(&account("acct-org-asof")).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(17);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = crate::intelligence::IntelligenceJson {
            org_health: Some(crate::intelligence::io::OrgHealthData {
                renewal_likelihood: Some("0.80".to_string()),
                growth_tier: Some("high".to_string()),
                customer_stage: Some("active".to_string()),
                support_tier: Some("premium".to_string()),
                gathered_at: "2026-05-19T09:30:00Z".to_string(),
                source: "glean".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let report = promote_glean_facts_from_intelligence(&ctx, &db, "acct-org-asof", &intel);

        assert_eq!(report.claim_errors, Vec::<String>::new());
        assert_eq!(report.claims_committed, 4);
        let subject_ref = serde_json::json!({"kind": "account", "id": "acct-org-asof"}).to_string();
        let active_claims = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        for field in [
            "account.support_tier",
            "account.renewal_likelihood",
            "account.customer_status",
            "account.growth_potential_score",
        ] {
            let claim = active_claims
                .iter()
                .find(|claim| claim.field_path.as_deref() == Some(field))
                .unwrap_or_else(|| panic!("missing promoted claim for {field}"));
            assert_eq!(
                claim.source_asof.as_deref(),
                Some("2026-05-19T09:30:00Z"),
                "{field} should use orgHealth.gatheredAt as source_asof"
            );
        }
        for field in ["account.renewal_likelihood", "account.customer_status"] {
            let claim = active_claims
                .iter()
                .find(|claim| claim.field_path.as_deref() == Some(field))
                .unwrap_or_else(|| panic!("missing promoted claim for {field}"));
            assert_eq!(
                claim.data_source, "Salesforce",
                "{field} should preserve CRM system-of-record provenance"
            );
        }
    }

    #[test]
    fn promote_glean_product_classification_keeps_system_of_record_source() {
        let db = test_db();
        db.upsert_account(&account("acct-product-source")).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(18);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = crate::intelligence::IntelligenceJson {
            product_classification: Some(crate::intelligence::io::ProductClassification {
                products: vec![
                    crate::intelligence::io::ProductInfo {
                        type_: Some("cms".to_string()),
                        arr: Some(120_000.0),
                        ..Default::default()
                    },
                    crate::intelligence::io::ProductInfo {
                        type_: Some("analytics".to_string()),
                        arr: Some(50_000.0),
                        ..Default::default()
                    },
                ],
            }),
            ..Default::default()
        };

        let report =
            promote_glean_facts_from_intelligence(&ctx, &db, "acct-product-source", &intel);

        assert_eq!(report.claim_errors, Vec::<String>::new());
        assert_eq!(report.claims_committed, 2);
        let subject_ref =
            serde_json::json!({"kind": "account", "id": "acct-product-source"}).to_string();
        let active_claims = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        for field in [
            "account.active_subscription_count",
            "account.primary_product",
        ] {
            let claim = active_claims
                .iter()
                .find(|claim| claim.field_path.as_deref() == Some(field))
                .unwrap_or_else(|| panic!("missing promoted claim for {field}"));
            assert_eq!(claim.data_source, "Salesforce");
        }
        let source_refs = db.get_account_source_refs("acct-product-source").unwrap();
        for field in ["active_subscription_count", "primary_product"] {
            let source_ref = source_refs
                .iter()
                .find(|source_ref| source_ref.field == field)
                .unwrap_or_else(|| panic!("missing source ref for {field}"));
            assert_eq!(source_ref.source_system, "Salesforce");
        }
    }

    #[test]
    fn promote_glean_contract_arr_keeps_system_of_record_source() {
        let db = test_db();
        db.upsert_account(&account("acct-contract-source")).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(19);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = crate::intelligence::IntelligenceJson {
            contract_context: Some(crate::intelligence::io::ContractContext {
                current_arr: Some(125_000.0),
                ..Default::default()
            }),
            ..Default::default()
        };

        let report =
            promote_glean_facts_from_intelligence(&ctx, &db, "acct-contract-source", &intel);

        assert_eq!(report.claim_errors, Vec::<String>::new());
        assert_eq!(report.claims_committed, 1);
        let subject_ref =
            serde_json::json!({"kind": "account", "id": "acct-contract-source"}).to_string();
        let active_claims = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        let arr_claim = active_claims
            .iter()
            .find(|claim| claim.field_path.as_deref() == Some("account.arr"))
            .expect("missing promoted ARR claim");
        assert_eq!(arr_claim.data_source, "Salesforce");

        let source_refs = db.get_account_source_refs("acct-contract-source").unwrap();
        for field in ["arr_range_low", "arr_range_high"] {
            let source_ref = source_refs
                .iter()
                .find(|source_ref| source_ref.field == field)
                .unwrap_or_else(|| panic!("missing source ref for {field}"));
            assert_eq!(source_ref.source_system, "Salesforce");
        }
    }

    #[test]
    fn backfill_account_fact_claims_is_idempotent() {
        let db = test_db();
        let mut seeded = account("acct-backfill");
        seeded.arr_range_low = Some(125_000.0);
        seeded.arr_range_high = Some(125_000.0);
        seeded.customer_status = Some("active".to_string());
        seeded.customer_status_source = Some("glean".to_string());
        seeded.customer_status_updated_at = Some("2026-05-21T09:00:00Z".to_string());
        db.upsert_account(&seeded).unwrap();
        db.upsert_account_fact(
            "acct-backfill",
            "arr_range_low",
            "125000",
            "glean",
            "2026-05-21T09:00:00Z",
        )
        .unwrap();
        db.upsert_account_fact(
            "acct-backfill",
            "arr_range_high",
            "125000",
            "glean",
            "2026-05-21T09:00:00Z",
        )
        .unwrap();
        db.upsert_account_fact(
            "acct-backfill",
            "customer_status",
            "active",
            "glean",
            "2026-05-21T09:00:00Z",
        )
        .unwrap();
        for (field, value) in [
            ("arr_range_low", "125000"),
            ("arr_range_high", "125000"),
            ("customer_status", "active"),
        ] {
            db.upsert_account_source_ref(&AccountSourceRef {
                account_id: "acct-backfill",
                field,
                source_system: "glean",
                source_kind: "fact",
                source_value: Some(value),
                observed_at: "2026-05-21T09:00:00Z",
                reference_id: None,
            })
            .unwrap();
        }

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(8);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = backfill_account_fact_claims(&ctx, &db).unwrap();
        assert_eq!(first.claim_errors, Vec::<String>::new());
        assert_eq!(first.claims_committed, 2);
        assert_eq!(first.recompute_jobs_enqueued, 1);
        assert_eq!(claim_recompute_job_count(&db), 1);

        let second = backfill_account_fact_claims(&ctx, &db).unwrap();
        assert_eq!(second.claims_committed, 0);
        assert!(second.claims_already_present >= 2);
        assert_eq!(
            claim_recompute_job_count(&db),
            1,
            "unscored recovery enqueue should coalesce by subject"
        );

        let claims = load_entity_context_claims_active_for_surface(
            &db,
            "account",
            "acct-backfill",
            1,
            "mcp_tool",
        )
        .unwrap();
        assert!(claims.iter().any(|claim| claim.text == "arr: 125,000"));
        assert!(claims
            .iter()
            .any(|claim| claim.text == "customer status: active"));
    }

    #[test]
    fn backfill_account_fact_claims_promotes_source_less_schema_facts() {
        let db = test_db();
        db.upsert_account(&account("acct-schema-fallback")).unwrap();
        db.conn_ref()
            .execute(
                "UPDATE accounts
                    SET arr_range_low = 125000,
                        arr_range_high = 125000,
                        customer_status = 'active',
                        customer_status_source = NULL,
                        customer_status_updated_at = NULL
                  WHERE id = 'acct-schema-fallback'",
                [],
            )
            .expect("seed source-less schema facts");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(11);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let report = backfill_account_fact_claims(&ctx, &db).unwrap();

        assert_eq!(report.claim_errors, Vec::<String>::new());
        assert_eq!(report.claims_committed, 2);
        let subject_ref =
            serde_json::json!({"kind": "account", "id": "acct-schema-fallback"}).to_string();
        let active_claims = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        let arr_claim = active_claims
            .iter()
            .find(|claim| claim.field_path.as_deref() == Some("account.arr"))
            .expect("missing ARR claim");
        assert_eq!(arr_claim.data_source, "account_schema");
        assert_eq!(arr_claim.text, "arr: 125,000");
        assert_eq!(arr_claim.observed_at, "2026-05-20T00:00:00Z");
        assert!(arr_claim
            .source_ref
            .as_deref()
            .is_some_and(|reference| { reference.starts_with("account_fact:") }));
        assert_eq!(
            account_fact_claim_source_kind(arr_claim).as_deref(),
            Some("schema_snapshot")
        );

        let customer_status_claim = active_claims
            .iter()
            .find(|claim| claim.field_path.as_deref() == Some("account.customer_status"))
            .expect("missing customer status claim");
        assert_eq!(customer_status_claim.data_source, "account_schema");
        assert_eq!(customer_status_claim.text, "customer status: active");
    }

    #[test]
    fn changed_account_fact_supersedes_prior_active_claim() {
        let db = test_db();
        db.upsert_account(&account("acct-supersede")).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(9);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = promote_account_fact(
            &ctx,
            &db,
            AccountFactInput {
                account_id: "acct-supersede",
                schema_field: "renewal_likelihood",
                claim_field: "renewal_likelihood",
                value: "0.55",
                source_system: "glean",
                source_kind: "inference",
                source_asof: Some("2026-05-20T12:00:00Z"),
                observed_at: "2026-05-22T12:00:00Z",
                reference_id: Some("src-1"),
            },
        );
        assert_eq!(first.claim_outcome, ClaimPromotionOutcome::Committed);

        let second = promote_account_fact(
            &ctx,
            &db,
            AccountFactInput {
                account_id: "acct-supersede",
                schema_field: "renewal_likelihood",
                claim_field: "renewal_likelihood",
                value: "0.85",
                source_system: "glean",
                source_kind: "inference",
                source_asof: Some("2026-05-21T12:00:00Z"),
                observed_at: "2026-05-22T12:00:00Z",
                reference_id: Some("src-2"),
            },
        );
        assert_eq!(second.claim_outcome, ClaimPromotionOutcome::Committed);

        let subject_ref =
            serde_json::json!({"kind": "account", "id": "acct-supersede"}).to_string();
        let active_claims = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        let field_claims: Vec<_> = active_claims
            .iter()
            .filter(|claim| claim.field_path.as_deref() == Some("account.renewal_likelihood"))
            .collect();

        assert_eq!(field_claims.len(), 1);
        assert_eq!(field_claims[0].text, "renewal likelihood: 85%");
        assert_eq!(field_claims[0].source_ref.as_deref(), Some("src-2"));
    }

    #[test]
    fn same_text_account_fact_corroborates_when_provenance_changes() {
        let db = test_db();
        db.upsert_account(&account("acct-same-text-source"))
            .unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(10);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = promote_account_fact(
            &ctx,
            &db,
            AccountFactInput {
                account_id: "acct-same-text-source",
                schema_field: "renewal_likelihood",
                claim_field: "renewal_likelihood",
                value: "0.85",
                source_system: "ai",
                source_kind: "inference",
                source_asof: Some("2026-05-20T12:00:00Z"),
                observed_at: "2026-05-22T12:00:00Z",
                reference_id: Some("src-ai"),
            },
        );
        assert_eq!(first.claim_outcome, ClaimPromotionOutcome::Committed);

        let second = promote_account_fact(
            &ctx,
            &db,
            AccountFactInput {
                account_id: "acct-same-text-source",
                schema_field: "renewal_likelihood",
                claim_field: "renewal_likelihood",
                value: "0.85",
                source_system: "glean",
                source_kind: "fact",
                source_asof: Some("2026-05-21T12:00:00Z"),
                observed_at: "2026-05-22T12:00:00Z",
                reference_id: Some("src-glean"),
            },
        );
        assert_eq!(second.claim_outcome, ClaimPromotionOutcome::Committed);

        let subject_ref =
            serde_json::json!({"kind": "account", "id": "acct-same-text-source"}).to_string();
        let active_claims = load_claims_active(&db, &subject_ref, Some(CLAIM_TYPE)).unwrap();
        let field_claims: Vec<_> = active_claims
            .iter()
            .filter(|claim| claim.field_path.as_deref() == Some("account.renewal_likelihood"))
            .collect();

        assert_eq!(field_claims.len(), 1);
        assert_eq!(field_claims[0].text, "renewal likelihood: 85%");
        assert_eq!(field_claims[0].data_source, "ai");
        assert_eq!(field_claims[0].source_ref.as_deref(), Some("src-ai"));
        assert_eq!(
            field_claims[0].source_asof.as_deref(),
            Some("2026-05-20T12:00:00Z")
        );
        assert_eq!(
            account_fact_claim_source_kind(field_claims[0]).as_deref(),
            Some("inference")
        );
        let corroborating_sources: Vec<String> = db
            .conn_ref()
            .prepare(
                "SELECT data_source
                 FROM claim_corroborations
                 WHERE claim_id = ?1
                 ORDER BY data_source",
            )
            .expect("prepare corroboration query")
            .query_map([field_claims[0].id.as_str()], |row| row.get::<_, String>(0))
            .expect("query corroborations")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect corroborations");
        assert_eq!(
            corroborating_sources,
            vec!["glean".to_string()],
            "same text from another source should corroborate instead of replacing the claim"
        );
    }
}
