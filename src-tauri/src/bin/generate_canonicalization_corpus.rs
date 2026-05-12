//! Generator for the ADR-0131 Phase B calibration corpus.
//!
//! Emits 500 labeled-pair JSON files across the 5 buckets at the wave-plan
//! composition: positive_paraphrases (200), hard_negatives (150),
//! contradictions (75), asymmetric_qualifiers (50), low_trust_duplicates (25).
//!
//! Pairs are constructed synthetically — there is no production drain at this
//! stage of the project. Construction is adversarial: each pair is designed
//! to probe a specific edge of `canonical_match_v2`'s decision boundary.
//!
//! Deterministic: re-running produces byte-identical files.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use abilities_runtime::predicates::registry::PredicateRef;
use abilities_runtime::structured_claim::{
    EntityRef, LiteralKind, ObjectValue, Polarity, QualifierSet, RegionCode, ScopeMarker, Sentiment,
    StructuredClaim, TemporalQualifier,
};
use serde::Serialize;
use serde_json::{json, Value};

const CORPUS_ROOT: &str = "src-tauri/suites/E/canonicalization-thresholds";

const POSITIVE_COUNT: usize = 200;
const HARD_NEGATIVE_COUNT: usize = 150;
const CONTRADICTION_COUNT: usize = 75;
const ASYMMETRIC_QUALIFIER_COUNT: usize = 50;
const LOW_TRUST_COUNT: usize = 25;

fn main() {
    let root = repo_relative(CORPUS_ROOT);
    purge_generated_pairs(&root);

    let mut written = 0;
    written += emit_positive_paraphrases(&root);
    written += emit_hard_negatives(&root);
    written += emit_contradictions(&root);
    written += emit_asymmetric_qualifiers(&root);
    written += emit_low_trust_duplicates(&root);

    println!("wrote {written} labeled pairs to {}", root.display());
}

fn repo_relative(rel: &str) -> PathBuf {
    let cwd = std::env::current_dir().expect("cwd");
    // When the binary is invoked from the repo root via `cargo run`, the cwd
    // is `src-tauri/`. Strip that if present so the path resolves regardless
    // of where the binary was launched from.
    if cwd.ends_with("src-tauri") {
        cwd.parent().unwrap().join(rel)
    } else {
        cwd.join(rel)
    }
}

fn purge_generated_pairs(root: &Path) {
    for bucket in [
        "positive_paraphrases",
        "hard_negatives",
        "contradictions",
        "asymmetric_qualifiers",
        "low_trust_duplicates",
    ] {
        let dir = root.join(bucket);
        if !dir.exists() {
            fs::create_dir_all(&dir).expect("create bucket dir");
            continue;
        }
        for entry in fs::read_dir(&dir).expect("read bucket dir") {
            let path = entry.expect("read entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                fs::remove_file(&path).expect("remove existing pair");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct PredicateTemplate {
    predicate: PredicateRef,
    claim_type: &'static str,
    field_path: &'static str,
    object_variants: &'static [ObjectShape],
    paraphrase_pairs: &'static [(&'static str, &'static str)],
    affirm_negate_pairs: &'static [(&'static str, &'static str)],
}

#[derive(Clone)]
enum ObjectShape {
    LiteralEnum(&'static str),
    LiteralText(&'static str),
    LiteralMoney(&'static str),
    LiteralDate(&'static str),
    LiteralPercentage(&'static str),
    FreeText(&'static str),
    Resolved(&'static str, &'static str),
}

fn templates() -> Vec<PredicateTemplate> {
    vec![
        PredicateTemplate {
            predicate: PredicateRef::ContractApprovalStatus,
            claim_type: "risk",
            field_path: "approval.status",
            object_variants: &[
                ObjectShape::LiteralEnum("pending_finance_approval"),
                ObjectShape::LiteralEnum("approved"),
                ObjectShape::LiteralEnum("blocked_legal"),
                ObjectShape::LiteralEnum("pending_security_review"),
                ObjectShape::LiteralEnum("revoked"),
            ],
            paraphrase_pairs: &[
                (
                    "Phase 2 budget approval is pending with finance.",
                    "Finance still needs to approve the Phase 2 budget.",
                ),
                (
                    "The MSA is awaiting legal sign-off.",
                    "Legal has not yet signed the MSA.",
                ),
                (
                    "Approval was granted by procurement on May 1.",
                    "Procurement approved the request on the first of May.",
                ),
                (
                    "Security review is the current blocker.",
                    "Security review remains outstanding.",
                ),
                (
                    "The contract has been rescinded.",
                    "We have revoked the contract.",
                ),
            ],
            affirm_negate_pairs: &[
                (
                    "The contract has been approved.",
                    "The contract has not been approved.",
                ),
                (
                    "Legal review is complete.",
                    "Legal review is not complete.",
                ),
            ],
        },
        PredicateTemplate {
            predicate: PredicateRef::ProductUsageTrend,
            claim_type: "engagement",
            field_path: "usage.trend",
            object_variants: &[
                ObjectShape::LiteralEnum("declining"),
                ObjectShape::LiteralEnum("flat"),
                ObjectShape::LiteralEnum("rising"),
                ObjectShape::LiteralEnum("seasonal_spike"),
            ],
            paraphrase_pairs: &[
                (
                    "Weekly active users have dropped 18 percent over the last quarter.",
                    "Weekly actives are down roughly a fifth quarter over quarter.",
                ),
                (
                    "Engagement is flat with no movement either direction.",
                    "Usage has plateaued — no growth, no decline.",
                ),
                (
                    "Adoption climbed sharply after the March launch.",
                    "Usage went up significantly following the March release.",
                ),
                (
                    "Pre-renewal activity spiked as expected.",
                    "We saw the usual surge ahead of renewal.",
                ),
                (
                    "Daily logins have dwindled to a handful.",
                    "Only a handful of users log in each day now.",
                ),
            ],
            affirm_negate_pairs: &[
                (
                    "Usage is trending up this quarter.",
                    "Usage is not trending up this quarter.",
                ),
                (
                    "Adoption stalled after the May rollout.",
                    "Adoption did not stall after the May rollout.",
                ),
            ],
        },
        PredicateTemplate {
            predicate: PredicateRef::AccountRenewalRisk,
            claim_type: "risk",
            field_path: "renewal.risk",
            object_variants: &[
                ObjectShape::LiteralEnum("high"),
                ObjectShape::LiteralEnum("medium"),
                ObjectShape::LiteralEnum("low"),
                ObjectShape::LiteralEnum("at_risk"),
            ],
            paraphrase_pairs: &[
                (
                    "Renewal probability looks weak this cycle.",
                    "Renewal looks unlikely this cycle.",
                ),
                (
                    "We expect a clean renewal in Q3.",
                    "Q3 renewal should land without trouble.",
                ),
                (
                    "Renewal risk is elevated due to leadership changes.",
                    "Leadership churn has raised our renewal risk.",
                ),
                (
                    "The renewal conversation is back on track.",
                    "Renewal momentum has recovered.",
                ),
                (
                    "Renewal is in jeopardy after the latest incident.",
                    "Following the incident, renewal is at significant risk.",
                ),
            ],
            affirm_negate_pairs: &[
                (
                    "Renewal is at risk this cycle.",
                    "Renewal is not at risk this cycle.",
                ),
                (
                    "Q3 renewal is at risk.",
                    "Q3 renewal is no longer at risk.",
                ),
            ],
        },
        PredicateTemplate {
            predicate: PredicateRef::StakeholderRole,
            claim_type: "stakeholder_role",
            field_path: "role.classification",
            object_variants: &[
                ObjectShape::LiteralEnum("champion"),
                ObjectShape::LiteralEnum("blocker"),
                ObjectShape::LiteralEnum("decision_maker"),
                ObjectShape::LiteralEnum("influencer"),
                ObjectShape::LiteralEnum("end_user"),
            ],
            paraphrase_pairs: &[
                (
                    "She is acting as the executive champion for the rollout.",
                    "She is the exec sponsor advocating for the rollout.",
                ),
                (
                    "He has been the primary blocker on the procurement path.",
                    "Procurement has stalled because of his pushback.",
                ),
                (
                    "Decision authority sits with the new CFO.",
                    "The new CFO holds final sign-off authority.",
                ),
                (
                    "She influences the buying committee but does not own it.",
                    "She sways the committee without holding the decision.",
                ),
                (
                    "He uses the product daily as an end user.",
                    "Daily product usage from him as an end user is consistent.",
                ),
            ],
            affirm_negate_pairs: &[
                (
                    "He is the champion for the integration project.",
                    "He is not the champion for the integration project.",
                ),
                (
                    "She is the decision maker on procurement.",
                    "She is not the decision maker on procurement.",
                ),
            ],
        },
        PredicateTemplate {
            predicate: PredicateRef::RiskStatus,
            claim_type: "risk",
            field_path: "risk.status",
            object_variants: &[
                ObjectShape::LiteralEnum("open"),
                ObjectShape::LiteralEnum("mitigated"),
                ObjectShape::LiteralEnum("escalated"),
                ObjectShape::LiteralEnum("closed"),
            ],
            paraphrase_pairs: &[
                (
                    "Migration risk is still open and unresolved.",
                    "The migration risk remains open.",
                ),
                (
                    "The compliance gap has been closed.",
                    "Compliance no longer flags this account.",
                ),
                (
                    "We escalated the data quality issue to engineering.",
                    "Data quality concerns went to engineering for escalation.",
                ),
                (
                    "Mitigation is in place for the SSO outage.",
                    "The SSO outage has compensating controls in place.",
                ),
                (
                    "Performance regressions are tracked as open.",
                    "Open status on the performance regression issue.",
                ),
            ],
            affirm_negate_pairs: &[
                (
                    "The risk is open.",
                    "The risk is not open.",
                ),
                (
                    "The compliance risk is mitigated.",
                    "The compliance risk is not mitigated.",
                ),
            ],
        },
        PredicateTemplate {
            predicate: PredicateRef::TopicMentioned,
            claim_type: "engagement_topic",
            field_path: "topic.mentioned",
            object_variants: &[
                ObjectShape::FreeText("competitive_displacement"),
                ObjectShape::FreeText("pricing_negotiation"),
                ObjectShape::FreeText("support_quality"),
                ObjectShape::FreeText("integration_timeline"),
                ObjectShape::FreeText("security_audit"),
            ],
            paraphrase_pairs: &[
                (
                    "The team raised competitive displacement risk on the call.",
                    "On the call, the team flagged risk of being displaced by a competitor.",
                ),
                (
                    "Pricing came up multiple times during the QBR.",
                    "Multiple pricing references showed up in the QBR.",
                ),
                (
                    "Support quality concerns dominated the meeting.",
                    "The meeting was dominated by complaints about support.",
                ),
                (
                    "Integration timeline was the main topic of discussion.",
                    "Most of the discussion centered on the integration timeline.",
                ),
                (
                    "Security audit findings were mentioned in passing.",
                    "Brief mention of security audit findings on the call.",
                ),
            ],
            affirm_negate_pairs: &[
                (
                    "Pricing was discussed during the meeting.",
                    "Pricing was not discussed during the meeting.",
                ),
                (
                    "Competitive displacement came up.",
                    "Competitive displacement did not come up.",
                ),
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// Bucket emitters
// ---------------------------------------------------------------------------

fn emit_positive_paraphrases(root: &Path) -> usize {
    let bucket_dir = root.join("positive_paraphrases");
    let templates = templates();
    let mut written = 0;
    for (idx, ord) in (0..POSITIVE_COUNT).map(|i| (i, i + 1)) {
        let template = &templates[idx % templates.len()];
        let object_idx = (idx / templates.len()) % template.object_variants.len();
        let paraphrase_idx = (idx / templates.len()) % template.paraphrase_pairs.len();
        let (text_a, text_b) = template.paraphrase_pairs[paraphrase_idx];
        let subject = subject_for_index("account", idx);
        let object = object_value(&template.object_variants[object_idx]);
        let qualifiers = qualifier_set_default(idx);
        let pair_id = format!("positive_paraphrase_{ord:03}");
        write_pair(
            &bucket_dir,
            &pair_id,
            "positive_paraphrases",
            "merge",
            &claim(
                text_a,
                &subject,
                template,
                Polarity::Affirm,
                object.clone(),
                qualifiers.clone(),
            ),
            &claim(
                text_b,
                &subject,
                template,
                Polarity::Affirm,
                object,
                qualifiers,
            ),
            "Same subject, predicate, polarity, object, qualifiers, and status; text varies only by paraphrase.",
        );
        written += 1;
    }
    written
}

fn emit_hard_negatives(root: &Path) -> usize {
    let bucket_dir = root.join("hard_negatives");
    let templates = templates();
    let mut written = 0;
    for (idx, ord) in (0..HARD_NEGATIVE_COUNT).map(|i| (i, i + 1)) {
        let template = &templates[idx % templates.len()];
        let variant = idx % 4;
        let pair_id = format!("hard_negative_{ord:03}");
        let subject_a = subject_for_index("account", idx);
        let qualifiers = qualifier_set_default(idx);

        let (claim_a, claim_b, rationale) = match variant {
            0 => {
                // Same predicate, different object value (different vendor/enum).
                let obj_a = object_value(&template.object_variants[0]);
                let obj_b = object_value(
                    &template.object_variants
                        [(1 % template.object_variants.len()).max(0)],
                );
                let text_a = template.paraphrase_pairs[0].0;
                let text_b = template.paraphrase_pairs[1 % template.paraphrase_pairs.len()].0;
                (
                    claim(text_a, &subject_a, template, Polarity::Affirm, obj_a, qualifiers.clone()),
                    claim(text_b, &subject_a, template, Polarity::Affirm, obj_b, qualifiers.clone()),
                    "Same subject and predicate but the object value differs — must not merge.",
                )
            }
            1 => {
                // Different subject IDs (same kind).
                let subject_b = subject_for_index("account", idx + 1_000);
                let obj = object_value(&template.object_variants[0]);
                let text = template.paraphrase_pairs[0].0;
                (
                    claim(text, &subject_a, template, Polarity::Affirm, obj.clone(), qualifiers.clone()),
                    claim(text, &subject_b, template, Polarity::Affirm, obj, qualifiers.clone()),
                    "Same text and predicate but distinct subjects — must not merge.",
                )
            }
            2 => {
                // Cross-predicate near-text. Pick a neighbour template.
                let other = &templates[(idx + 1) % templates.len()];
                let obj_a = object_value(&template.object_variants[0]);
                let obj_b = object_value(&other.object_variants[0]);
                (
                    claim(
                        template.paraphrase_pairs[0].0,
                        &subject_a,
                        template,
                        Polarity::Affirm,
                        obj_a,
                        qualifiers.clone(),
                    ),
                    claim(
                        other.paraphrase_pairs[0].0,
                        &subject_a,
                        other,
                        Polarity::Affirm,
                        obj_b,
                        qualifiers.clone(),
                    ),
                    "Subjects align but predicates differ — must not merge.",
                )
            }
            _ => {
                // Same predicate + object, but scope qualifier differs.
                let qualifiers_b = QualifierSet {
                    scope: Some(ScopeMarker {
                        normalized: format!("region_alt_{idx}"),
                    }),
                    ..qualifiers.clone()
                };
                let obj = object_value(&template.object_variants[0]);
                let text = template.paraphrase_pairs[0].0;
                (
                    claim(text, &subject_a, template, Polarity::Affirm, obj.clone(), qualifiers.clone()),
                    claim(text, &subject_a, template, Polarity::Affirm, obj, qualifiers_b),
                    "Scope qualifier differs — different bucket within same predicate; must not merge.",
                )
            }
        };

        write_pair(
            &bucket_dir,
            &pair_id,
            "hard_negatives",
            "fork",
            &claim_a,
            &claim_b,
            rationale,
        );
        written += 1;
    }
    written
}

fn emit_contradictions(root: &Path) -> usize {
    let bucket_dir = root.join("contradictions");
    let templates = templates();
    let mut written = 0;
    for (idx, ord) in (0..CONTRADICTION_COUNT).map(|i| (i, i + 1)) {
        let template = &templates[idx % templates.len()];
        let object_idx = (idx / templates.len()) % template.object_variants.len();
        let pair_index = (idx / templates.len()) % template.affirm_negate_pairs.len();
        let (text_a, text_b) = template.affirm_negate_pairs[pair_index];
        let subject = subject_for_index("account", idx);
        let object = object_value(&template.object_variants[object_idx]);
        let qualifiers = qualifier_set_default(idx);
        let pair_id = format!("contradiction_{ord:03}");
        write_pair(
            &bucket_dir,
            &pair_id,
            "contradictions",
            "contradict",
            &claim(text_a, &subject, template, Polarity::Affirm, object.clone(), qualifiers.clone()),
            &claim(text_b, &subject, template, Polarity::Negate, object, qualifiers),
            "Same subject, predicate, qualifiers, and object — opposite polarity.",
        );
        written += 1;
    }
    written
}

fn emit_asymmetric_qualifiers(root: &Path) -> usize {
    let bucket_dir = root.join("asymmetric_qualifiers");
    let templates = templates();
    let mut written = 0;
    let qualifier_kinds = [
        QualifierKind::Time,
        QualifierKind::Region,
        QualifierKind::Scope,
        QualifierKind::Entity,
    ];
    for (idx, ord) in (0..ASYMMETRIC_QUALIFIER_COUNT).map(|i| (i, i + 1)) {
        let template = &templates[idx % templates.len()];
        let object_idx = (idx / templates.len()) % template.object_variants.len();
        let kind = &qualifier_kinds[idx % qualifier_kinds.len()];
        let qualifiers_a = qualifier_set_default(idx);
        let qualifiers_b = with_qualifier(qualifiers_a.clone(), kind, idx);
        let subject = subject_for_index("account", idx);
        let object = object_value(&template.object_variants[object_idx]);
        let text = template.paraphrase_pairs[0].0;
        let pair_id = format!("asymmetric_qualifier_{ord:03}");
        write_pair(
            &bucket_dir,
            &pair_id,
            "asymmetric_qualifiers",
            "fork",
            &claim(text, &subject, template, Polarity::Affirm, object.clone(), qualifiers_a),
            &claim(text, &subject, template, Polarity::Affirm, object, qualifiers_b),
            "One side carries an additional qualifier — must fork rather than merge.",
        );
        written += 1;
    }
    written
}

fn emit_low_trust_duplicates(root: &Path) -> usize {
    let bucket_dir = root.join("low_trust_duplicates");
    let templates = templates();
    let mut written = 0;
    for (idx, ord) in (0..LOW_TRUST_COUNT).map(|i| (i, i + 1)) {
        let template = &templates[idx % templates.len()];
        let object_idx = (idx / templates.len()) % template.object_variants.len();
        let subject = subject_for_index("account", idx);
        let object = object_value(&template.object_variants[object_idx]);
        let qualifiers = qualifier_set_default(idx);
        let text = template.paraphrase_pairs[0].0;
        let pair_id = format!("low_trust_duplicate_{ord:03}");
        let mut claim_payload = claim(
            text,
            &subject,
            template,
            Polarity::Affirm,
            object,
            qualifiers,
        );
        // Mark the pair as weak-corroboration so v2 routes to needs_verification.
        if let Some(obj) = claim_payload.as_object_mut() {
            obj.insert("non_semantic_mergeable".into(), json!(true));
        }
        write_pair(
            &bucket_dir,
            &pair_id,
            "low_trust_duplicates",
            "ambiguous",
            &claim_payload,
            &claim_payload.clone(),
            "Structurally identical claims with weak corroboration — route to needs_verification, not auto-merge.",
        );
        written += 1;
    }
    written
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

enum QualifierKind {
    Time,
    Region,
    Scope,
    Entity,
}

fn with_qualifier(mut base: QualifierSet, kind: &QualifierKind, idx: usize) -> QualifierSet {
    match kind {
        QualifierKind::Time => {
            base.time = Some(TemporalQualifier {
                normalized: format!("2026-Q{}", (idx % 4) + 1),
            });
        }
        QualifierKind::Region => {
            let codes = ["US", "EU", "APAC", "LATAM"];
            base.region = Some(RegionCode {
                code: codes[idx % codes.len()].to_string(),
            });
        }
        QualifierKind::Scope => {
            base.scope = Some(ScopeMarker {
                normalized: format!("phase_{}", (idx % 4) + 1),
            });
        }
        QualifierKind::Entity => {
            base.entity = Some(EntityRef {
                kind: "team".into(),
                id: format!("team_{idx}"),
            });
        }
    }
    base
}

fn qualifier_set_default(idx: usize) -> QualifierSet {
    // Most pairs have an empty qualifier set so the bucket-specific logic can
    // introduce divergence cleanly. A few cycle scope so positives carry a
    // realistic structural fingerprint.
    if idx % 5 == 0 {
        QualifierSet {
            scope: Some(ScopeMarker {
                normalized: format!("phase_{}", (idx % 3) + 1),
            }),
            ..QualifierSet::default()
        }
    } else {
        QualifierSet::default()
    }
}

fn object_value(shape: &ObjectShape) -> ObjectValue {
    match shape {
        ObjectShape::LiteralEnum(value) => ObjectValue::Literal {
            literal_kind: LiteralKind::Enum,
            value: (*value).into(),
        },
        ObjectShape::LiteralText(value) => ObjectValue::Literal {
            literal_kind: LiteralKind::Text,
            value: (*value).into(),
        },
        ObjectShape::LiteralMoney(value) => ObjectValue::Literal {
            literal_kind: LiteralKind::Money,
            value: (*value).into(),
        },
        ObjectShape::LiteralDate(value) => ObjectValue::Literal {
            literal_kind: LiteralKind::Date,
            value: (*value).into(),
        },
        ObjectShape::LiteralPercentage(value) => ObjectValue::Literal {
            literal_kind: LiteralKind::Percentage,
            value: (*value).into(),
        },
        ObjectShape::FreeText(value) => ObjectValue::FreeText {
            canonical: (*value).into(),
        },
        ObjectShape::Resolved(kind, id) => ObjectValue::Resolved {
            entity_ref: EntityRef {
                kind: (*kind).into(),
                id: (*id).into(),
            },
        },
    }
}

fn subject_for_index(kind: &str, idx: usize) -> EntityRef {
    EntityRef {
        kind: kind.into(),
        id: format!("{kind}_{idx}"),
    }
}

fn claim(
    text: &str,
    subject: &EntityRef,
    template: &PredicateTemplate,
    polarity: Polarity,
    object: ObjectValue,
    qualifiers: QualifierSet,
) -> Value {
    let predicate_str = serde_json::to_value(&template.predicate)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| match &template.predicate {
            PredicateRef::Unresolved { text } => format!("unresolved:{text}"),
            _ => "unknown".into(),
        });

    let structured = StructuredClaim {
        subject_ref: subject.clone(),
        predicate: template.predicate.clone(),
        polarity,
        object: object.clone(),
        qualifiers: qualifiers.clone(),
        status: abilities_runtime::structured_claim::ClaimStatus::Confirmed,
        sentiment: Some(Sentiment::Neutral),
    };

    let mut value = serde_json::to_value(&structured).expect("serialize structured");
    let obj = value.as_object_mut().expect("structured is object");
    obj.insert("text".into(), json!(text));
    obj.insert("sensitivity".into(), json!("internal"));
    obj.insert("claim_type".into(), json!(template.claim_type));
    obj.insert("field_path".into(), json!(template.field_path));
    obj.insert("workspace_id".into(), json!("workspace_alpha"));
    // Force the predicate to its registry string so we don't accidentally
    // emit Unresolved wrappers.
    obj.insert("predicate".into(), json!(predicate_str));
    value
}

#[derive(Serialize)]
struct PairFile<'a> {
    pair_id: &'a str,
    bucket: &'a str,
    expected_decision: &'a str,
    source: &'a str,
    claim_a: &'a Value,
    claim_b: &'a Value,
    rationale: &'a str,
}

fn write_pair(
    bucket_dir: &Path,
    pair_id: &str,
    bucket: &str,
    expected_decision: &str,
    claim_a: &Value,
    claim_b: &Value,
    rationale: &str,
) {
    let payload = PairFile {
        pair_id,
        bucket,
        expected_decision,
        source: "synthetic",
        claim_a,
        claim_b,
        rationale,
    };
    let serialized = serde_json::to_string_pretty(&payload).expect("serialize pair");
    let path = bucket_dir.join(format!("{pair_id}.json"));
    fs::write(&path, serialized).expect("write pair");
}

// Keep the type appeasements compile-time-safe; BTreeMap is only used to keep
// the eprintln stable.
#[allow(dead_code)]
fn _unused_btree_map_keepalive() -> BTreeMap<String, usize> {
    BTreeMap::new()
}
