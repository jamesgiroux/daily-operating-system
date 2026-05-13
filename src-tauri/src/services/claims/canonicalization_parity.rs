//! ADR-0131 Phase B calibration corpus runner.
//!
//! The runner is read-only against claim state: it loads labeled pair files,
//! evaluates the v2 comparator through the same shadow-mode path used by
//! Phase A, compares that result to expected corpus labels, and writes
//! report artifacts next to the corpus.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use abilities_runtime::predicates::registry::PredicateRef;
use abilities_runtime::structured_claim::{
    CanonicalStatus, ClaimStatus as StructuredClaimStatus, EntityRef, ObjectValue, Polarity,
    QualifierSet, Sentiment, StructuredClaim,
};
use serde::{Deserialize, Serialize};

use crate::db::claims::{ClaimSensitivity, ClaimState, SurfacingState};
use crate::services::comparator_thresholds::COMPARATOR_THRESHOLD_VERSION;

use super::{
    canonical_match_config, canonical_match_v2, CanonicalDecisionKind, CanonicalMatchInput,
    CanonicalizationMode, ThresholdBand,
};

pub const PARITY_REPORT_MARKDOWN: &str = "parity-report.md";
pub const PARITY_REPORT_JSON: &str = ".parity-report.json";

const REPORT_SCHEMA_VERSION: &str = "canonicalization-parity-report:v2";
const SHADOW_MODE_LABEL: &str = "shadow";

pub const GATE_METRIC_KEYS: [&str; 11] = [
    "true_merge_precision",
    "true_merge_recall",
    "true_fork_recall",
    "contradiction_detection",
    "false_merge_rate",
    "ambiguous_rate_per_bucket",
    "tombstone_bypass_rate",
    "cross_tier_merge_rate",
    "cross_account_merge_rate",
    "cross_workspace_merge_rate",
    "legacy_unmigrated_merge_rate",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CorpusBucket {
    PositiveParaphrases,
    HardNegatives,
    Contradictions,
    AsymmetricQualifiers,
    LowTrustDuplicates,
    TombstoneShadowed,
    CrossTier,
    CrossWorkspace,
    LegacyUnmigrated,
}

impl CorpusBucket {
    const ALL: [CorpusBucket; 9] = [
        CorpusBucket::PositiveParaphrases,
        CorpusBucket::HardNegatives,
        CorpusBucket::Contradictions,
        CorpusBucket::AsymmetricQualifiers,
        CorpusBucket::LowTrustDuplicates,
        CorpusBucket::TombstoneShadowed,
        CorpusBucket::CrossTier,
        CorpusBucket::CrossWorkspace,
        CorpusBucket::LegacyUnmigrated,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::PositiveParaphrases => "positive_paraphrases",
            Self::HardNegatives => "hard_negatives",
            Self::Contradictions => "contradictions",
            Self::AsymmetricQualifiers => "asymmetric_qualifiers",
            Self::LowTrustDuplicates => "low_trust_duplicates",
            Self::TombstoneShadowed => "tombstone_shadowed",
            Self::CrossTier => "cross_tier",
            Self::CrossWorkspace => "cross_workspace",
            Self::LegacyUnmigrated => "legacy_unmigrated",
        }
    }

    fn target_share(self) -> &'static str {
        match self {
            Self::PositiveParaphrases => "37.04%",
            Self::HardNegatives => "27.78%",
            Self::Contradictions => "13.89%",
            Self::AsymmetricQualifiers => "9.26%",
            Self::LowTrustDuplicates => "4.63%",
            Self::TombstoneShadowed => "1.85%",
            Self::CrossTier => "1.85%",
            Self::CrossWorkspace => "1.85%",
            Self::LegacyUnmigrated => "1.85%",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionLabel {
    Merge,
    Fork,
    Contradict,
    Ambiguous,
}

impl DecisionLabel {
    fn from_v2(decision: CanonicalDecisionKind) -> Self {
        match decision {
            CanonicalDecisionKind::Merge => Self::Merge,
            CanonicalDecisionKind::Fork | CanonicalDecisionKind::ForkFiltered => Self::Fork,
            CanonicalDecisionKind::ForkContradiction => Self::Contradict,
            CanonicalDecisionKind::ForkAmbiguous => Self::Ambiguous,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Fork => "fork",
            Self::Contradict => "contradict",
            Self::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairSource {
    Production,
    Synthetic,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanonicalizationPair {
    pub pair_id: String,
    pub bucket: CorpusBucket,
    pub expected_decision: DecisionLabel,
    pub source: PairSource,
    pub claim_a: CorpusClaim,
    pub claim_b: CorpusClaim,
    pub rationale: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorpusClaim {
    pub text: String,
    pub sensitivity: ClaimSensitivity,
    pub subject_ref: EntityRef,
    pub predicate: PredicateRef,
    pub polarity: Polarity,
    pub object: ObjectValue,
    pub qualifiers: QualifierSet,
    pub status: StructuredClaimStatus,
    pub sentiment: Option<Sentiment>,
    #[serde(default)]
    pub claim_type: Option<String>,
    #[serde(default)]
    pub field_path: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub tier_key: Option<String>,
    #[serde(default)]
    pub claim_state: Option<ClaimState>,
    #[serde(default)]
    pub surfacing_state: Option<SurfacingState>,
    #[serde(default)]
    pub canonical_status: Option<CanonicalStatus>,
    #[serde(default)]
    pub non_semantic_mergeable: Option<bool>,
    #[serde(default)]
    pub tombstone_shadowed: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParityReport {
    pub schema_version: String,
    pub corpus_dir: String,
    pub canonicalization_mode: String,
    pub comparator_threshold_version: String,
    pub pair_count: usize,
    pub bucket_counts: BTreeMap<String, usize>,
    pub metrics: BTreeMap<String, GateMetric>,
    pub buckets: BTreeMap<String, BucketBreakdown>,
    pub pairs: Vec<PairEvaluation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GateMetric {
    pub label: String,
    pub numerator: usize,
    pub denominator: usize,
    pub value: f64,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_bucket: Option<BTreeMap<String, RateSummary>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RateSummary {
    pub numerator: usize,
    pub denominator: usize,
    pub value: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BucketBreakdown {
    pub total: usize,
    pub target_share: String,
    pub expected_decisions: BTreeMap<String, usize>,
    pub v2_decisions: BTreeMap<String, usize>,
    pub expected_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairEvaluation {
    pub pair_id: String,
    pub bucket: String,
    pub source: String,
    pub source_path: String,
    pub expected_decision: DecisionLabel,
    pub v2_decision: DecisionLabel,
    pub matches_expected: bool,
    pub v2_reason: String,
    pub v2_reason_secondary: Vec<String>,
    pub v2_threshold_band: Option<String>,
    pub v2_canonicalization_mode: String,
    pub v2_embedding_model_version: String,
    pub v2_field_scores: serde_json::Value,
    pub tombstone_scoped: bool,
    pub cross_tier: bool,
    pub cross_account: bool,
    pub cross_workspace: bool,
    pub legacy_unmigrated: bool,
    pub rationale: String,
}

pub fn default_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("suites/E/canonicalization-thresholds")
}

pub fn generate_default_parity_report_files() -> Result<ParityReport, String> {
    generate_parity_report_files(default_corpus_dir())
}

pub fn generate_parity_report_files(corpus_dir: impl AsRef<Path>) -> Result<ParityReport, String> {
    let corpus_dir = corpus_dir.as_ref();
    let report = build_parity_report(corpus_dir)?;
    let json_path = corpus_dir.join(PARITY_REPORT_JSON);
    let markdown_path = corpus_dir.join(PARITY_REPORT_MARKDOWN);

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize parity report JSON: {error}"))?;
    write_file(&json_path, format!("{json}\n"))?;
    write_file(
        &markdown_path,
        with_single_trailing_newline(render_markdown(&report)),
    )?;

    Ok(report)
}

pub fn build_parity_report(corpus_dir: impl AsRef<Path>) -> Result<ParityReport, String> {
    let corpus_dir = corpus_dir.as_ref();
    let pairs = read_corpus_pairs(corpus_dir)?;
    let mut evaluations = Vec::with_capacity(pairs.len());

    for (pair, path) in pairs {
        evaluations.push(evaluate_pair(pair, &path));
    }

    let metrics = compute_metrics(&evaluations);
    let mut bucket_counts = BTreeMap::new();
    let mut buckets = BTreeMap::new();

    for bucket in CorpusBucket::ALL {
        buckets.insert(
            bucket.as_str().to_string(),
            BucketBreakdown {
                target_share: bucket.target_share().to_string(),
                ..BucketBreakdown::default()
            },
        );
        bucket_counts.insert(bucket.as_str().to_string(), 0);
    }

    for evaluation in &evaluations {
        *bucket_counts.entry(evaluation.bucket.clone()).or_insert(0) += 1;
        let bucket = buckets.entry(evaluation.bucket.clone()).or_default();
        bucket.total += 1;
        increment(
            &mut bucket.expected_decisions,
            evaluation.expected_decision.as_str(),
        );
        increment(&mut bucket.v2_decisions, evaluation.v2_decision.as_str());
        if !evaluation.matches_expected {
            bucket.expected_mismatches += 1;
        }
    }

    Ok(ParityReport {
        schema_version: REPORT_SCHEMA_VERSION.to_string(),
        corpus_dir: corpus_dir.display().to_string(),
        canonicalization_mode: SHADOW_MODE_LABEL.to_string(),
        comparator_threshold_version: COMPARATOR_THRESHOLD_VERSION.to_string(),
        pair_count: evaluations.len(),
        bucket_counts,
        metrics,
        buckets,
        pairs: evaluations,
    })
}

fn read_corpus_pairs(corpus_dir: &Path) -> Result<Vec<(CanonicalizationPair, PathBuf)>, String> {
    let mut pairs = Vec::new();
    let mut seen_pair_ids = BTreeSet::new();

    for bucket in CorpusBucket::ALL {
        let bucket_dir = corpus_dir.join(bucket.as_str());
        if !bucket_dir.is_dir() {
            return Err(format!(
                "missing canonicalization corpus bucket directory: {}",
                bucket_dir.display()
            ));
        }

        let mut files = fs::read_dir(&bucket_dir)
            .map_err(|error| format!("read bucket {}: {error}", bucket_dir.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("read bucket entry {}: {error}", bucket_dir.display()))?;
        files.sort();

        for path in files {
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("read corpus pair {}: {error}", path.display()))?;
            let pair = serde_json::from_str::<CanonicalizationPair>(&content)
                .map_err(|error| format!("parse corpus pair {}: {error}", path.display()))?;
            if pair.bucket != bucket {
                return Err(format!(
                    "corpus pair {} declares bucket {:?}, expected {:?}",
                    path.display(),
                    pair.bucket,
                    bucket
                ));
            }
            if !seen_pair_ids.insert(pair.pair_id.clone()) {
                return Err(format!(
                    "duplicate canonicalization pair_id {}",
                    pair.pair_id
                ));
            }
            pairs.push((pair, path));
        }
    }

    Ok(pairs)
}

fn evaluate_pair(pair: CanonicalizationPair, path: &Path) -> PairEvaluation {
    let input_a = match_input(&pair.pair_id, "a", &pair.claim_a);
    let input_b = match_input(&pair.pair_id, "b", &pair.claim_b);
    let config = canonical_match_config(&input_a, &input_b);
    let outcome = canonical_match_v2(&input_a, &input_b, &config);
    let v2_decision = DecisionLabel::from_v2(outcome.decision);
    PairEvaluation {
        pair_id: pair.pair_id,
        bucket: pair.bucket.as_str().to_string(),
        source: source_label(pair.source).to_string(),
        source_path: path.display().to_string(),
        expected_decision: pair.expected_decision,
        v2_decision,
        matches_expected: v2_decision == pair.expected_decision,
        v2_reason: outcome.reason,
        v2_reason_secondary: outcome.reason_secondary,
        v2_threshold_band: outcome.threshold_band.map(threshold_band_label),
        v2_canonicalization_mode: canonicalization_mode_label(config.mode).to_string(),
        v2_embedding_model_version: config.embedding_model_version,
        v2_field_scores: outcome.field_scores,
        tombstone_scoped: tombstone_scoped(&pair.claim_a) || tombstone_scoped(&pair.claim_b),
        cross_tier: tier_key(&pair.claim_a) != tier_key(&pair.claim_b),
        cross_account: account_scope(&pair.claim_a) != account_scope(&pair.claim_b),
        cross_workspace: pair.claim_a.workspace_id != pair.claim_b.workspace_id,
        legacy_unmigrated: legacy_unmigrated(&pair.claim_a) || legacy_unmigrated(&pair.claim_b),
        rationale: pair.rationale,
    }
}

fn match_input(pair_id: &str, side: &str, claim: &CorpusClaim) -> CanonicalMatchInput {
    CanonicalMatchInput {
        claim_id: format!("{pair_id}-{side}"),
        claim_type: claim
            .claim_type
            .clone()
            .unwrap_or_else(|| "canonicalization_fixture".to_string()),
        field_path: claim.field_path.clone(),
        text: claim.text.clone(),
        item_hash: None,
        canonical_subject_kind: claim.subject_ref.kind.clone(),
        canonical_subject_id: claim.subject_ref.id.clone(),
        account_id: claim.account_id.clone().or_else(|| {
            (claim.subject_ref.kind == "account").then(|| claim.subject_ref.id.clone())
        }),
        workspace_id: claim.workspace_id.clone(),
        tier_key: tier_key(claim),
        claim_state: claim.claim_state.clone().unwrap_or(ClaimState::Active),
        surfacing_state: claim
            .surfacing_state
            .clone()
            .unwrap_or(SurfacingState::Active),
        canonical_status: claim
            .canonical_status
            .clone()
            .unwrap_or(CanonicalStatus::Live),
        non_semantic_mergeable: claim.non_semantic_mergeable.unwrap_or(false),
        tombstone_shadowed: claim.tombstone_shadowed.unwrap_or(false),
        structured: StructuredClaim {
            subject_ref: claim.subject_ref.clone(),
            predicate: claim.predicate.clone(),
            polarity: claim.polarity,
            object: claim.object.clone(),
            qualifiers: claim.qualifiers.clone(),
            status: claim.status.clone(),
            sentiment: claim.sentiment.clone(),
        },
        structural_field_content_hash: None,
        backfill_epoch: 1,
    }
}

fn compute_metrics(evaluations: &[PairEvaluation]) -> BTreeMap<String, GateMetric> {
    let mut metrics = BTreeMap::new();

    insert_metric(
        &mut metrics,
        "true_merge_precision",
        "True-merge precision",
        count_where(evaluations, |pair| {
            pair.v2_decision == DecisionLabel::Merge
                && pair.expected_decision == DecisionLabel::Merge
        }),
        count_where(evaluations, |pair| pair.v2_decision == DecisionLabel::Merge),
        ">= 0.98 on should_merge corpus",
    );
    insert_metric(
        &mut metrics,
        "true_merge_recall",
        "True-merge recall",
        count_where(evaluations, |pair| {
            pair.expected_decision == DecisionLabel::Merge
                && pair.v2_decision == DecisionLabel::Merge
        }),
        count_where(evaluations, |pair| {
            pair.expected_decision == DecisionLabel::Merge
        }),
        ">= 0.95 on should_merge corpus",
    );
    insert_metric(
        &mut metrics,
        "true_fork_recall",
        "True-fork recall",
        count_where(evaluations, |pair| {
            pair.expected_decision == DecisionLabel::Fork && pair.v2_decision == DecisionLabel::Fork
        }),
        count_where(evaluations, |pair| {
            pair.expected_decision == DecisionLabel::Fork
        }),
        ">= 0.95 on should_fork corpus",
    );
    insert_metric(
        &mut metrics,
        "contradiction_detection",
        "Contradiction detection",
        count_where(evaluations, |pair| {
            pair.expected_decision == DecisionLabel::Contradict
                && pair.v2_decision == DecisionLabel::Contradict
        }),
        count_where(evaluations, |pair| {
            pair.expected_decision == DecisionLabel::Contradict
        }),
        ">= 0.97 on should_contradict corpus",
    );
    insert_metric(
        &mut metrics,
        "false_merge_rate",
        "False-merge rate",
        count_where(evaluations, |pair| {
            pair.v2_decision == DecisionLabel::Merge
                && pair.expected_decision != DecisionLabel::Merge
        }),
        evaluations.len(),
        "<= 0.005; false merges are double-weighted at gate review",
    );

    let ambiguous_numerator = count_where(evaluations, |pair| {
        pair.v2_decision == DecisionLabel::Ambiguous
    });
    metrics.insert(
        "ambiguous_rate_per_bucket".to_string(),
        GateMetric {
            label: "Ambiguous-rate per bucket".to_string(),
            numerator: ambiguous_numerator,
            denominator: evaluations.len(),
            value: rate(ambiguous_numerator, evaluations.len()),
            target: "<= 0.05 per label bucket".to_string(),
            per_bucket: Some(ambiguous_rate_per_bucket(evaluations)),
        },
    );

    insert_scoped_merge_metric(
        &mut metrics,
        evaluations,
        "tombstone_bypass_rate",
        "Tombstone-bypass rate",
        |pair| pair.tombstone_scoped,
        "= 0",
    );
    insert_scoped_merge_metric(
        &mut metrics,
        evaluations,
        "cross_tier_merge_rate",
        "Cross-tier merge rate",
        |pair| pair.cross_tier,
        "= 0",
    );
    insert_scoped_merge_metric(
        &mut metrics,
        evaluations,
        "cross_account_merge_rate",
        "Cross-account merge rate",
        |pair| pair.cross_account,
        "= 0",
    );
    insert_scoped_merge_metric(
        &mut metrics,
        evaluations,
        "cross_workspace_merge_rate",
        "Cross-workspace merge rate",
        |pair| pair.cross_workspace,
        "= 0",
    );
    insert_scoped_merge_metric(
        &mut metrics,
        evaluations,
        "legacy_unmigrated_merge_rate",
        "Legacy-unmigrated merge rate",
        |pair| pair.legacy_unmigrated,
        "= 0",
    );

    metrics
}

fn ambiguous_rate_per_bucket(evaluations: &[PairEvaluation]) -> BTreeMap<String, RateSummary> {
    let mut summaries = BTreeMap::new();
    for bucket in CorpusBucket::ALL {
        let denominator = count_where(evaluations, |pair| pair.bucket == bucket.as_str());
        let numerator = count_where(evaluations, |pair| {
            pair.bucket == bucket.as_str() && pair.v2_decision == DecisionLabel::Ambiguous
        });
        summaries.insert(
            bucket.as_str().to_string(),
            RateSummary {
                numerator,
                denominator,
                value: rate(numerator, denominator),
            },
        );
    }
    summaries
}

fn insert_scoped_merge_metric(
    metrics: &mut BTreeMap<String, GateMetric>,
    evaluations: &[PairEvaluation],
    key: &str,
    label: &str,
    scope: impl Fn(&PairEvaluation) -> bool,
    target: &str,
) {
    let denominator = evaluations.iter().filter(|pair| scope(pair)).count();
    let numerator = evaluations
        .iter()
        .filter(|pair| scope(pair) && pair.v2_decision == DecisionLabel::Merge)
        .count();
    insert_metric(metrics, key, label, numerator, denominator, target);
}

fn insert_metric(
    metrics: &mut BTreeMap<String, GateMetric>,
    key: &str,
    label: &str,
    numerator: usize,
    denominator: usize,
    target: &str,
) {
    metrics.insert(
        key.to_string(),
        GateMetric {
            label: label.to_string(),
            numerator,
            denominator,
            value: rate(numerator, denominator),
            target: target.to_string(),
            per_bucket: None,
        },
    );
}

fn count_where(
    evaluations: &[PairEvaluation],
    predicate: impl Fn(&PairEvaluation) -> bool,
) -> usize {
    evaluations.iter().filter(|pair| predicate(pair)).count()
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn increment(counts: &mut BTreeMap<String, usize>, key: &str) {
    *counts.entry(key.to_string()).or_insert(0) += 1;
}

fn tier_key(claim: &CorpusClaim) -> String {
    claim
        .tier_key
        .clone()
        .unwrap_or_else(|| default_tier_key(&claim.sensitivity).to_string())
}

fn default_tier_key(sensitivity: &ClaimSensitivity) -> &'static str {
    match sensitivity {
        ClaimSensitivity::Public => "state:public",
        ClaimSensitivity::Internal => "state:internal",
        ClaimSensitivity::Confidential => "state:confidential",
        ClaimSensitivity::UserOnly => "state:user_only",
    }
}

fn account_scope(claim: &CorpusClaim) -> String {
    claim
        .account_id
        .clone()
        .unwrap_or_else(|| format!("{}:{}", claim.subject_ref.kind, claim.subject_ref.id))
}

fn tombstone_scoped(claim: &CorpusClaim) -> bool {
    matches!(
        claim.claim_state,
        Some(ClaimState::Tombstoned | ClaimState::Withdrawn)
    ) || claim.tombstone_shadowed.unwrap_or(false)
}

fn legacy_unmigrated(claim: &CorpusClaim) -> bool {
    matches!(
        claim.canonical_status,
        Some(CanonicalStatus::LegacyUnmigrated)
    ) || claim.non_semantic_mergeable.unwrap_or(false)
}

fn threshold_band_label(band: ThresholdBand) -> String {
    match band {
        ThresholdBand::High => "high",
        ThresholdBand::Ambiguous => "ambiguous",
        ThresholdBand::Low => "low",
    }
    .to_string()
}

fn canonicalization_mode_label(mode: CanonicalizationMode) -> &'static str {
    match mode {
        CanonicalizationMode::Full => "full",
        CanonicalizationMode::Deterministic => "deterministic",
        CanonicalizationMode::HashFallback => "hash_fallback",
    }
}

fn source_label(source: PairSource) -> &'static str {
    match source {
        PairSource::Production => "production",
        PairSource::Synthetic => "synthetic",
    }
}

fn render_markdown(report: &ParityReport) -> String {
    let mut markdown = String::new();
    writeln!(markdown, "# Canonicalization Parity Report").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- Schema: `{}`", report.schema_version).unwrap();
    writeln!(markdown, "- Mode: `{}`", report.canonicalization_mode).unwrap();
    writeln!(
        markdown,
        "- Comparator thresholds: `{}`",
        report.comparator_threshold_version
    )
    .unwrap();
    writeln!(markdown, "- Corpus: `{}`", report.corpus_dir).unwrap();
    writeln!(markdown, "- Pair count: {}", report.pair_count).unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "## Bucket Composition").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "| Bucket | Pairs | Target |").unwrap();
    writeln!(markdown, "|---|---:|---:|").unwrap();
    for bucket in CorpusBucket::ALL {
        let key = bucket.as_str();
        let count = report.bucket_counts.get(key).copied().unwrap_or(0);
        writeln!(
            markdown,
            "| `{key}` | {count} | {} |",
            bucket.target_share()
        )
        .unwrap();
    }
    writeln!(markdown).unwrap();
    writeln!(markdown, "## Gate Metrics").unwrap();
    writeln!(markdown).unwrap();
    writeln!(
        markdown,
        "| Metric | Value | Numerator | Denominator | Target |"
    )
    .unwrap();
    writeln!(markdown, "|---|---:|---:|---:|---|").unwrap();
    for key in GATE_METRIC_KEYS {
        if let Some(metric) = report.metrics.get(key) {
            writeln!(
                markdown,
                "| `{key}` | {:.4} | {} | {} | {} |",
                metric.value, metric.numerator, metric.denominator, metric.target
            )
            .unwrap();
        }
    }
    writeln!(markdown).unwrap();
    writeln!(markdown, "## Per-Bucket Expected vs V2").unwrap();
    writeln!(markdown).unwrap();
    for (bucket, breakdown) in &report.buckets {
        writeln!(
            markdown,
            "### `{bucket}` ({}, target {})",
            breakdown.total, breakdown.target_share
        )
        .unwrap();
        writeln!(
            markdown,
            "- Expected mismatches: {}",
            breakdown.expected_mismatches
        )
        .unwrap();
        writeln!(markdown, "- Expected decisions:").unwrap();
        for (decision, count) in &breakdown.expected_decisions {
            writeln!(markdown, "  - `{decision}`: {count}").unwrap();
        }
        writeln!(markdown, "- V2 decisions:").unwrap();
        for (decision, count) in &breakdown.v2_decisions {
            writeln!(markdown, "  - `{decision}`: {count}").unwrap();
        }
        writeln!(markdown).unwrap();
    }
    markdown
}

fn write_file(path: &Path, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|error| format!("write {}: {error}", path.display()))
}

fn with_single_trailing_newline(content: String) -> String {
    format!("{}\n", content.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "regenerates on-disk canonicalization parity report artifacts"]
    fn stub_corpus_generates_well_formed_parity_report() {
        let report = generate_default_parity_report_files()
            .expect("stub canonicalization parity corpus should generate reports");

        assert!(report.pair_count >= 10);
        for bucket in CorpusBucket::ALL {
            let count = report
                .bucket_counts
                .get(bucket.as_str())
                .copied()
                .unwrap_or(0);
            assert!(
                count >= 2,
                "stub bucket {} should include at least two pairs",
                bucket.as_str()
            );
        }
        for key in GATE_METRIC_KEYS {
            let metric = report
                .metrics
                .get(key)
                .unwrap_or_else(|| panic!("missing gate metric {key}"));
            assert!(metric.value.is_finite(), "metric {key} must be populated");
        }

        let corpus_dir = default_corpus_dir();
        let markdown = fs::read_to_string(corpus_dir.join(PARITY_REPORT_MARKDOWN))
            .expect("markdown report should be written");
        assert!(markdown.contains("# Canonicalization Parity Report"));
        assert!(markdown.contains("`true_merge_precision`"));

        let json = fs::read_to_string(corpus_dir.join(PARITY_REPORT_JSON))
            .expect("JSON report should be written");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("JSON report should be well formed");
        assert_eq!(
            parsed["schema_version"],
            serde_json::Value::String(REPORT_SCHEMA_VERSION.to_string())
        );
        assert_eq!(
            parsed["metrics"].as_object().map(|metrics| metrics.len()),
            Some(GATE_METRIC_KEYS.len())
        );
    }
}
