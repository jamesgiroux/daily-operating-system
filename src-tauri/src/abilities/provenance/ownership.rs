//! DOS-288 production ownership validator.
//!
//! ADR-0102 keeps provenance on the `AbilityOutput<T>` wrapper rather than on
//! domain DTOs. The DOS-288 contract builds on that shape: before a production
//! surface renders claim-attached output as confident, it must prove the
//! rendered fields belong to the output subject, have source evidence that can
//! be linked to that subject, and do not trip the existing cross-entity
//! coherence factor.
//!
//! This module is intentionally an assembly layer. It composes the shipped
//! provenance finalization invariants (`ProvenanceBuilder::finalize` subject
//! and field attribution outcomes), `SubjectAttribution` fit/competing-subject
//! data, the cycle-7
//! `services::claims::claim_allowed_for_prompt_input` helper, and
//! `abilities::trust::cross_entity_coherence`. It does not introduce a second
//! subject-fit algorithm or a replacement contamination heuristic.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    AbilityOutput, CompositionId, FieldAttribution, FieldPath, Provenance, SourceAttribution,
    SourceIdentifier, SourceIndex, SourceRef, SubjectAttribution, SubjectBindingKind,
    SubjectFitStatus, SubjectRef,
};
use crate::abilities::registry::{AbilityCategory, AbilityDescriptor};
use crate::abilities::trust::factors::cross_entity_coherence;
use crate::abilities::trust::{
    CrossEntityCoherenceInput, CrossEntityHitKind, EntityFootprint, TargetFootprint, TrustConfig,
};
use crate::db::claims::IntelligenceClaim;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OwnershipPolicy {
    pub requested_render_policy: OwnershipRenderPolicy,
    pub target_footprint: Option<TargetFootprint>,
    #[serde(default)]
    pub portfolio_footprints: Vec<EntityFootprint>,
    #[serde(default)]
    pub cross_entity_context_expected: bool,
    #[serde(default)]
    pub trust_config: TrustConfig,
    #[serde(default)]
    pub require_entity_link_evidence: bool,
    #[serde(default = "default_true")]
    pub allow_user_confirmed_subject_override: bool,
    #[serde(default)]
    pub reject_sources_outside_subject_scope: bool,
    #[serde(default)]
    pub source_entity_links: Vec<SourceEntityLinkEvidence>,
    #[serde(default)]
    pub canonical_subject_groups: Vec<CanonicalSubjectGroup>,
    #[serde(default)]
    pub prompt_input_claims: Vec<IntelligenceClaim>,
}

impl OwnershipPolicy {
    pub fn confident() -> Self {
        Self::default()
    }

    pub fn requiring_entity_link_evidence(mut self) -> Self {
        self.require_entity_link_evidence = true;
        self
    }

    pub fn rejecting_sources_outside_subject_scope(mut self) -> Self {
        self.reject_sources_outside_subject_scope = true;
        self
    }

    pub fn with_target_footprint(
        mut self,
        target_footprint: TargetFootprint,
        portfolio_footprints: Vec<EntityFootprint>,
    ) -> Self {
        self.target_footprint = Some(target_footprint);
        self.portfolio_footprints = portfolio_footprints;
        self
    }
}

impl Default for OwnershipPolicy {
    fn default() -> Self {
        Self {
            requested_render_policy: OwnershipRenderPolicy::Confident,
            target_footprint: None,
            portfolio_footprints: Vec::new(),
            cross_entity_context_expected: false,
            trust_config: TrustConfig::default(),
            require_entity_link_evidence: false,
            allow_user_confirmed_subject_override: true,
            reject_sources_outside_subject_scope: false,
            source_entity_links: Vec::new(),
            canonical_subject_groups: Vec::new(),
            prompt_input_claims: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceEntityLinkEvidence {
    pub source_index: SourceIndex,
    pub subject: SubjectRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CanonicalSubjectGroup {
    pub subjects: Vec<SubjectRef>,
    #[serde(default)]
    pub explicit_user_confirmed_merge: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipRenderPolicy {
    Confident,
    Ambiguous,
    Suppressed,
    NeedsVerification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OwnershipReport {
    pub subject: SubjectRef,
    pub rendered_paths_checked: Vec<FieldPath>,
    pub source_refs_resolved: Vec<ResolvedSourceRef>,
    pub competing_subjects: Vec<OwnershipCompetingSubject>,
    pub cross_entity_coherence_hits: Vec<OwnershipCrossEntityHit>,
    pub prompt_input_claims_checked: usize,
    pub render_policy: OwnershipRenderPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedSourceRef {
    pub field_path: FieldPath,
    pub source_ref: OwnershipSourceRef,
    pub entity_link_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OwnershipSourceRef {
    Source {
        source_index: SourceIndex,
    },
    Child {
        composition_id: CompositionId,
        field_path: FieldPath,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OwnershipCompetingSubject {
    pub field_path: Option<FieldPath>,
    pub subject: SubjectRef,
    pub confidence: f32,
    pub reason_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OwnershipCrossEntityHit {
    pub field_path: FieldPath,
    pub kind: CrossEntityHitKind,
    pub source_subject: Option<SubjectRef>,
    pub token_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OwnershipError {
    #[error("missing subject attribution")]
    MissingSubject,
    #[error("ability output envelope is not valid provenance-carrying JSON")]
    InvalidAbilityEnvelope,
    #[error("missing field attribution for rendered path")]
    MissingFieldAttribution { field_path: FieldPath },
    #[error("source ref has no entity-link evidence")]
    SourceRefWithoutEntityLinkEvidence {
        field_path: FieldPath,
        source_ref: OwnershipSourceRef,
    },
    #[error("subject fit is ambiguous or blocked")]
    AmbiguousOrBlockedSubjectFit {
        field_path: Option<FieldPath>,
        status: SubjectFitStatus,
    },
    #[error("cross-subject canonical merge is not explicitly confirmed")]
    CrossSubjectCanonicalMerge { subjects: Vec<SubjectRef> },
    #[error("confident render attempted despite low cross-entity coherence")]
    ConfidentRenderLowCrossEntityCoherence {
        field_path: FieldPath,
        value: f64,
        threshold: f64,
        hit_count: usize,
    },
}

pub fn validate_subject_ownership<T: Serialize>(
    output: &AbilityOutput<T>,
    rendered_paths: &[FieldPath],
    policy: OwnershipPolicy,
) -> Result<OwnershipReport, OwnershipError> {
    let data =
        serde_json::to_value(output.data()).map_err(|_| OwnershipError::InvalidAbilityEnvelope)?;
    validate_provenance_ownership(output.provenance(), &data, rendered_paths, policy)
}

pub fn validate_ability_output_value_ownership(
    output_json: Value,
    rendered_paths: &[FieldPath],
    policy: OwnershipPolicy,
) -> Result<OwnershipReport, OwnershipError> {
    let output = serde_json::from_value::<AbilityOutput<Value>>(output_json)
        .map_err(|_| OwnershipError::InvalidAbilityEnvelope)?;
    validate_subject_ownership(&output, rendered_paths, policy)
}

pub fn validate_serialized_subject_ownership(
    data: Value,
    provenance: Value,
    diagnostics: Value,
    rendered_paths: &[FieldPath],
    policy: OwnershipPolicy,
) -> Result<OwnershipReport, OwnershipError> {
    let ability_version = provenance
        .get("ability_version")
        .cloned()
        .ok_or(OwnershipError::InvalidAbilityEnvelope)?;
    validate_ability_output_value_ownership(
        serde_json::json!({
            "data": data,
            "provenance": provenance,
            "ability_version": ability_version,
            "diagnostics": diagnostics,
        }),
        rendered_paths,
        policy,
    )
}

pub fn build_ownership_policy_for_invocation(
    ability_meta: &AbilityDescriptor,
    input_json: &Value,
    response_provenance: &Value,
) -> Result<OwnershipPolicy, OwnershipError> {
    let provenance = serde_json::from_value::<Provenance>(response_provenance.clone())
        .map_err(|_| OwnershipError::InvalidAbilityEnvelope)?;
    let target_subject = target_subject_for_invocation(ability_meta, input_json)
        .unwrap_or_else(|| provenance.subject.subject.clone());
    let mut scope_subjects = vec![target_subject.clone()];
    collect_provenance_subjects(&provenance, &mut scope_subjects);
    let scope_subjects = distinct_subjects(&scope_subjects);
    let related_subjects = scope_subjects
        .iter()
        .filter(|subject| !subject_matches(subject, &target_subject))
        .cloned()
        .collect::<Vec<_>>();
    let target_footprint = TargetFootprint {
        subject: target_subject,
        names: Vec::new(),
        domains: Vec::new(),
        related_subjects: related_subjects.clone(),
        allowed_aliases: Vec::new(),
    };
    let portfolio_footprints = related_subjects
        .iter()
        .cloned()
        .map(entity_footprint_for_subject)
        .collect::<Vec<_>>();

    let mut policy =
        OwnershipPolicy::confident().with_target_footprint(target_footprint, portfolio_footprints);
    policy.require_entity_link_evidence = true;
    policy.source_entity_links = source_entity_links_for_provenance(&provenance, &scope_subjects);
    policy.canonical_subject_groups = canonical_subject_groups_for_provenance(&provenance);
    if ability_meta.category == AbilityCategory::Transform {
        policy.prompt_input_claims = prompt_input_claims_from_invocation(input_json);
    }
    Ok(policy)
}

fn validate_provenance_ownership(
    provenance: &Provenance,
    data: &Value,
    rendered_paths: &[FieldPath],
    policy: OwnershipPolicy,
) -> Result<OwnershipReport, OwnershipError> {
    validate_subject_fit(&provenance.subject, None)?;
    if matches!(provenance.subject.subject, SubjectRef::Unknown) {
        return Err(OwnershipError::MissingSubject);
    }
    validate_canonical_subject_groups(&provenance.subject.subject, &policy)?;

    let paths = paths_to_check(provenance, rendered_paths);
    let mut source_refs_resolved = Vec::new();
    let mut competing_subjects = competing_subjects_for(None, &provenance.subject);
    let mut cross_entity_hits = Vec::new();
    let mut render_policy = policy.requested_render_policy;

    if policy
        .prompt_input_claims
        .iter()
        .any(|claim| !crate::services::claims::claim_allowed_for_prompt_input(claim))
    {
        render_policy = OwnershipRenderPolicy::Suppressed;
    }

    for rendered_path in &paths {
        let field_attributions = field_attributions_for_path(provenance, rendered_path);
        if field_attributions.is_empty() {
            return Err(OwnershipError::MissingFieldAttribution {
                field_path: rendered_path.clone(),
            });
        }

        for (field_path, attribution) in field_attributions {
            validate_field_subject(&provenance.subject, &field_path, attribution, &policy)?;
            competing_subjects.extend(competing_subjects_for(
                Some(field_path.clone()),
                &attribution.subject,
            ));
            resolve_source_refs(
                provenance,
                &field_path,
                attribution,
                &policy,
                &mut source_refs_resolved,
            )?;
        }

        let coherence = cross_entity_coherence(
            &CrossEntityCoherenceInput {
                claim_text: rendered_text_at_path(data, rendered_path),
                target_footprint: policy
                    .target_footprint
                    .clone()
                    .unwrap_or_else(|| target_footprint_for_subject(&provenance.subject.subject)),
                portfolio_footprints: policy.portfolio_footprints.clone(),
                cross_entity_context_expected: policy.cross_entity_context_expected,
            },
            &policy.trust_config,
        );

        cross_entity_hits.extend(coherence.hits.iter().map(|hit| OwnershipCrossEntityHit {
            field_path: rendered_path.clone(),
            kind: hit.kind,
            source_subject: hit.source_subject.clone(),
            token_hash: redacted_hash(&hit.token),
        }));

        if coherence.value < policy.trust_config.likely_current_min {
            if matches!(
                policy.requested_render_policy,
                OwnershipRenderPolicy::Confident
            ) {
                return Err(OwnershipError::ConfidentRenderLowCrossEntityCoherence {
                    field_path: rendered_path.clone(),
                    value: coherence.value,
                    threshold: policy.trust_config.likely_current_min,
                    hit_count: coherence.hits.len(),
                });
            }
            if !matches!(render_policy, OwnershipRenderPolicy::Suppressed) {
                render_policy = OwnershipRenderPolicy::NeedsVerification;
            }
        }
    }

    Ok(OwnershipReport {
        subject: provenance.subject.subject.clone(),
        rendered_paths_checked: paths,
        source_refs_resolved,
        competing_subjects,
        cross_entity_coherence_hits: cross_entity_hits,
        prompt_input_claims_checked: policy.prompt_input_claims.len(),
        render_policy,
    })
}

fn paths_to_check(provenance: &Provenance, rendered_paths: &[FieldPath]) -> Vec<FieldPath> {
    if rendered_paths.is_empty() {
        provenance.field_attributions.keys().cloned().collect()
    } else {
        rendered_paths.to_vec()
    }
}

fn validate_subject_fit(
    subject: &SubjectAttribution,
    field_path: Option<FieldPath>,
) -> Result<(), OwnershipError> {
    if subject.is_confident() {
        Ok(())
    } else {
        Err(OwnershipError::AmbiguousOrBlockedSubjectFit {
            field_path,
            status: subject.fit.status.clone(),
        })
    }
}

fn validate_field_subject(
    envelope_subject: &SubjectAttribution,
    field_path: &FieldPath,
    attribution: &FieldAttribution,
    policy: &OwnershipPolicy,
) -> Result<(), OwnershipError> {
    validate_subject_fit(&attribution.subject, Some(field_path.clone()))?;
    if attribution.subject.is_coherent_with(envelope_subject) {
        return Ok(());
    }
    if policy.allow_user_confirmed_subject_override
        && matches!(
            attribution.subject.binding,
            SubjectBindingKind::UserConfirmed
        )
    {
        return Ok(());
    }
    Err(OwnershipError::CrossSubjectCanonicalMerge {
        subjects: vec![
            envelope_subject.subject.clone(),
            attribution.subject.subject.clone(),
        ],
    })
}

fn field_attributions_for_path<'a>(
    provenance: &'a Provenance,
    rendered_path: &FieldPath,
) -> Vec<(FieldPath, &'a FieldAttribution)> {
    provenance
        .field_attributions
        .iter()
        .filter(|(field_path, _)| {
            *field_path == rendered_path
                || rendered_path.covers(field_path)
                || field_path.covers(rendered_path)
        })
        .map(|(field_path, attribution)| (field_path.clone(), attribution))
        .collect()
}

fn competing_subjects_for(
    field_path: Option<FieldPath>,
    subject: &SubjectAttribution,
) -> Vec<OwnershipCompetingSubject> {
    subject
        .competing_subjects
        .iter()
        .map(|competing| OwnershipCompetingSubject {
            field_path: field_path.clone(),
            subject: competing.subject.clone(),
            confidence: competing.confidence,
            reason_hash: redacted_hash(&competing.reason),
        })
        .collect()
}

fn resolve_source_refs(
    provenance: &Provenance,
    field_path: &FieldPath,
    attribution: &FieldAttribution,
    policy: &OwnershipPolicy,
    resolved: &mut Vec<ResolvedSourceRef>,
) -> Result<(), OwnershipError> {
    for source_ref in &attribution.source_refs {
        match source_ref {
            SourceRef::Source { source_index } => {
                let Some(source) = provenance.sources.get(source_index.as_usize()) else {
                    return Err(OwnershipError::SourceRefWithoutEntityLinkEvidence {
                        field_path: field_path.clone(),
                        source_ref: OwnershipSourceRef::Source {
                            source_index: *source_index,
                        },
                    });
                };
                let entity_link_evidence = source_has_entity_link_evidence(
                    source,
                    *source_index,
                    &attribution.subject.subject,
                    policy,
                );
                if (policy.require_entity_link_evidence && !entity_link_evidence)
                    || (policy.reject_sources_outside_subject_scope
                        && !source_is_in_subject_scope(source, *source_index, policy))
                {
                    return Err(OwnershipError::SourceRefWithoutEntityLinkEvidence {
                        field_path: field_path.clone(),
                        source_ref: OwnershipSourceRef::Source {
                            source_index: *source_index,
                        },
                    });
                }
                resolved.push(ResolvedSourceRef {
                    field_path: field_path.clone(),
                    source_ref: OwnershipSourceRef::Source {
                        source_index: *source_index,
                    },
                    entity_link_evidence,
                });
            }
            SourceRef::Child {
                composition_id,
                field_path: child_field_path,
            } => {
                let Some(child) = provenance
                    .children
                    .iter()
                    .find(|child| &child.composition_id == composition_id)
                else {
                    return Err(OwnershipError::MissingFieldAttribution {
                        field_path: field_path.clone(),
                    });
                };
                let child_attributions =
                    field_attributions_for_path(&child.provenance, child_field_path);
                if child_attributions.is_empty() {
                    return Err(OwnershipError::MissingFieldAttribution {
                        field_path: child_field_path.clone(),
                    });
                }
                for (resolved_child_path, child_attribution) in child_attributions {
                    validate_field_subject(
                        &child.provenance.subject,
                        &resolved_child_path,
                        child_attribution,
                        policy,
                    )?;
                    resolve_source_refs(
                        &child.provenance,
                        &resolved_child_path,
                        child_attribution,
                        policy,
                        resolved,
                    )?;
                }
                resolved.push(ResolvedSourceRef {
                    field_path: field_path.clone(),
                    source_ref: OwnershipSourceRef::Child {
                        composition_id: composition_id.clone(),
                        field_path: child_field_path.clone(),
                    },
                    entity_link_evidence: true,
                });
            }
        }
    }

    Ok(())
}

fn source_has_entity_link_evidence(
    source: &SourceAttribution,
    source_index: SourceIndex,
    subject: &SubjectRef,
    policy: &OwnershipPolicy,
) -> bool {
    policy
        .source_entity_links
        .iter()
        .any(|link| link.source_index == source_index && subject_matches(subject, &link.subject))
        || source
            .identifiers
            .iter()
            .any(|identifier| identifier_matches_subject(identifier, subject))
}

fn source_is_in_subject_scope(
    source: &SourceAttribution,
    source_index: SourceIndex,
    policy: &OwnershipPolicy,
) -> bool {
    let Some(target_footprint) = policy.target_footprint.as_ref() else {
        return true;
    };

    policy
        .source_entity_links
        .iter()
        .filter(|link| link.source_index == source_index)
        .any(|link| subject_is_in_policy_scope(&link.subject, policy))
        || source
            .identifiers
            .iter()
            .any(|identifier| identifier_is_in_policy_scope(identifier, target_footprint, policy))
}

fn subject_is_in_policy_scope(subject: &SubjectRef, policy: &OwnershipPolicy) -> bool {
    let Some(target_footprint) = policy.target_footprint.as_ref() else {
        return true;
    };
    subject_matches(subject, &target_footprint.subject)
        || target_footprint
            .related_subjects
            .iter()
            .any(|related| subject_matches(subject, related))
        || policy
            .portfolio_footprints
            .iter()
            .any(|footprint| subject_matches(subject, &footprint.subject))
}

fn identifier_is_in_policy_scope(
    identifier: &SourceIdentifier,
    target_footprint: &TargetFootprint,
    policy: &OwnershipPolicy,
) -> bool {
    match identifier {
        SourceIdentifier::Entity { entity_id, .. } => {
            subject_contains_id(&target_footprint.subject, &entity_id.0)
                || target_footprint
                    .related_subjects
                    .iter()
                    .any(|subject| subject_contains_id(subject, &entity_id.0))
                || policy
                    .portfolio_footprints
                    .iter()
                    .any(|footprint| subject_contains_id(&footprint.subject, &entity_id.0))
        }
        SourceIdentifier::Meeting { meeting_id } => {
            subject_is_in_policy_scope(&SubjectRef::Meeting(meeting_id.0.clone()), policy)
        }
        SourceIdentifier::Signal { .. }
        | SourceIdentifier::EmailThread { .. }
        | SourceIdentifier::Document { .. }
        | SourceIdentifier::UserEntry { .. }
        | SourceIdentifier::GleanAssessment { .. }
        | SourceIdentifier::ProviderCompletion { .. }
        | SourceIdentifier::OpaqueGleanSource { .. } => false,
    }
}

fn identifier_matches_subject(identifier: &SourceIdentifier, subject: &SubjectRef) -> bool {
    match identifier {
        SourceIdentifier::Entity { entity_id, .. } => subject_contains_id(subject, &entity_id.0),
        SourceIdentifier::Meeting { meeting_id } => {
            subject_matches(subject, &SubjectRef::Meeting(meeting_id.0.clone()))
        }
        SourceIdentifier::Signal { .. }
        | SourceIdentifier::EmailThread { .. }
        | SourceIdentifier::Document { .. }
        | SourceIdentifier::UserEntry { .. }
        | SourceIdentifier::GleanAssessment { .. }
        | SourceIdentifier::ProviderCompletion { .. }
        | SourceIdentifier::OpaqueGleanSource { .. } => false,
    }
}

fn subject_contains_id(subject: &SubjectRef, id: &str) -> bool {
    match subject {
        SubjectRef::Account(value)
        | SubjectRef::Project(value)
        | SubjectRef::Person(value)
        | SubjectRef::Meeting(value)
        | SubjectRef::User(value) => value == id,
        SubjectRef::Multi(subjects) => subjects
            .iter()
            .any(|subject| subject_contains_id(subject, id)),
        SubjectRef::Global | SubjectRef::Unknown => false,
    }
}

fn subject_matches(left: &SubjectRef, right: &SubjectRef) -> bool {
    left.matches_or_contains(right) || right.matches_or_contains(left)
}

fn validate_canonical_subject_groups(
    envelope_subject: &SubjectRef,
    policy: &OwnershipPolicy,
) -> Result<(), OwnershipError> {
    for group in &policy.canonical_subject_groups {
        let subjects = distinct_subjects(&group.subjects);
        if subjects.len() <= 1 || group.explicit_user_confirmed_merge {
            continue;
        }
        if subjects
            .iter()
            .all(|subject| envelope_subject.matches_or_contains(subject))
        {
            continue;
        }
        return Err(OwnershipError::CrossSubjectCanonicalMerge { subjects });
    }
    Ok(())
}

fn distinct_subjects(subjects: &[SubjectRef]) -> Vec<SubjectRef> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for subject in subjects {
        let key = serde_json::to_string(subject).unwrap_or_else(|_| format!("{subject:?}"));
        if seen.insert(key) {
            out.push(subject.clone());
        }
    }
    out
}

fn rendered_text_at_path(data: &Value, field_path: &FieldPath) -> String {
    let value = if field_path.is_root() {
        data
    } else {
        data.pointer(field_path.as_str()).unwrap_or(data)
    };

    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn target_footprint_for_subject(subject: &SubjectRef) -> TargetFootprint {
    TargetFootprint {
        subject: subject.clone(),
        names: Vec::new(),
        domains: Vec::new(),
        related_subjects: Vec::new(),
        allowed_aliases: Vec::new(),
    }
}

fn entity_footprint_for_subject(subject: SubjectRef) -> EntityFootprint {
    EntityFootprint {
        subject,
        names: Vec::new(),
        domains: Vec::new(),
        infrastructure_ids: Vec::new(),
    }
}

fn target_subject_for_invocation(
    ability_meta: &AbilityDescriptor,
    input_json: &Value,
) -> Option<SubjectRef> {
    match ability_meta.name {
        "get_entity_context" => subject_from_entity_fields(input_json),
        "prepare_meeting" => string_field(input_json, "meeting_id").map(SubjectRef::Meeting),
        _ => subject_from_input_json(input_json),
    }
    .or_else(|| subject_from_input_json(input_json))
}

fn subject_from_input_json(input_json: &Value) -> Option<SubjectRef> {
    ["subject_ref", "subject"]
        .into_iter()
        .find_map(|key| input_json.get(key).and_then(subject_from_json_value))
        .or_else(|| subject_from_entity_fields(input_json))
        .or_else(|| string_field(input_json, "account_id").map(SubjectRef::Account))
        .or_else(|| string_field(input_json, "project_id").map(SubjectRef::Project))
        .or_else(|| string_field(input_json, "person_id").map(SubjectRef::Person))
        .or_else(|| string_field(input_json, "meeting_id").map(SubjectRef::Meeting))
        .or_else(|| string_field(input_json, "user_id").map(SubjectRef::User))
}

fn subject_from_entity_fields(input_json: &Value) -> Option<SubjectRef> {
    let entity_type = string_field(input_json, "entity_type")?;
    let entity_id = string_field(input_json, "entity_id")?;
    subject_from_kind_and_id(&entity_type, &entity_id)
}

fn subject_from_json_value(value: &Value) -> Option<SubjectRef> {
    if let Ok(subject) = serde_json::from_value::<SubjectRef>(value.clone()) {
        return Some(subject);
    }
    if let Some(raw) = value.as_str() {
        if let Ok(json) = serde_json::from_str::<Value>(raw) {
            return subject_from_json_value(&json);
        }
        return None;
    }
    let object = value.as_object()?;
    let kind = object
        .get("kind")
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)?;
    let id = object
        .get("id")
        .or_else(|| object.get("entity_id"))
        .and_then(Value::as_str)?;
    subject_from_kind_and_id(kind, id)
}

fn subject_from_kind_and_id(kind: &str, id: &str) -> Option<SubjectRef> {
    if id.trim().is_empty() {
        return None;
    }
    match kind.trim().to_ascii_lowercase().as_str() {
        "account" | "accounts" => Some(SubjectRef::Account(id.to_string())),
        "project" | "projects" => Some(SubjectRef::Project(id.to_string())),
        "person" | "people" => Some(SubjectRef::Person(id.to_string())),
        "meeting" | "meetings" => Some(SubjectRef::Meeting(id.to_string())),
        "user" | "users" => Some(SubjectRef::User(id.to_string())),
        "global" => Some(SubjectRef::Global),
        _ => None,
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn collect_provenance_subjects(provenance: &Provenance, out: &mut Vec<SubjectRef>) {
    collect_subject_ref_members(&provenance.subject.subject, out);
    for attribution in provenance.field_attributions.values() {
        collect_subject_ref_members(&attribution.subject.subject, out);
    }
    for child in &provenance.children {
        collect_provenance_subjects(&child.provenance, out);
    }
}

fn collect_subject_ref_members(subject: &SubjectRef, out: &mut Vec<SubjectRef>) {
    match subject {
        SubjectRef::Multi(subjects) => {
            for subject in subjects {
                collect_subject_ref_members(subject, out);
            }
        }
        SubjectRef::Unknown | SubjectRef::Global => {}
        subject => out.push(subject.clone()),
    }
}

fn source_entity_links_for_provenance(
    provenance: &Provenance,
    scope_subjects: &[SubjectRef],
) -> Vec<SourceEntityLinkEvidence> {
    let mut links = Vec::new();
    for (index, source) in provenance.sources.iter().enumerate() {
        let source_index = SourceIndex(index);
        let explicit_subjects = explicit_source_subjects(source, scope_subjects);
        if explicit_subjects.is_empty() {
            links.extend(
                inferred_source_subjects(provenance, source_index)
                    .into_iter()
                    .map(|subject| SourceEntityLinkEvidence {
                        source_index,
                        subject,
                    }),
            );
        } else {
            links.extend(
                explicit_subjects
                    .into_iter()
                    .map(|subject| SourceEntityLinkEvidence {
                        source_index,
                        subject,
                    }),
            );
        }
    }
    dedupe_source_entity_links(links)
}

fn explicit_source_subjects(
    source: &SourceAttribution,
    scope_subjects: &[SubjectRef],
) -> Vec<SubjectRef> {
    let mut subjects = Vec::new();
    for identifier in &source.identifiers {
        match identifier {
            SourceIdentifier::Entity { entity_id, .. } => {
                let matching_subjects = scope_subjects
                    .iter()
                    .filter(|subject| subject_contains_id(subject, &entity_id.0))
                    .cloned()
                    .collect::<Vec<_>>();
                if matching_subjects.is_empty() {
                    subjects.push(SubjectRef::Account(entity_id.0.clone()));
                } else {
                    subjects.extend(matching_subjects);
                }
            }
            SourceIdentifier::Meeting { meeting_id } => {
                subjects.push(SubjectRef::Meeting(meeting_id.0.clone()));
            }
            SourceIdentifier::Signal { .. }
            | SourceIdentifier::EmailThread { .. }
            | SourceIdentifier::Document { .. }
            | SourceIdentifier::UserEntry { .. }
            | SourceIdentifier::GleanAssessment { .. }
            | SourceIdentifier::ProviderCompletion { .. }
            | SourceIdentifier::OpaqueGleanSource { .. } => {}
        }
    }
    distinct_subjects(&subjects)
}

fn inferred_source_subjects(provenance: &Provenance, source_index: SourceIndex) -> Vec<SubjectRef> {
    let subjects = provenance
        .field_attributions
        .values()
        .filter(|attribution| {
            attribution.source_refs.iter().any(|source_ref| {
                matches!(
                    source_ref,
                    SourceRef::Source {
                        source_index: candidate
                    } if *candidate == source_index
                )
            })
        })
        .flat_map(|attribution| {
            let mut subjects = Vec::new();
            collect_subject_ref_members(&attribution.subject.subject, &mut subjects);
            subjects
        })
        .collect::<Vec<_>>();
    distinct_subjects(&subjects)
}

fn dedupe_source_entity_links(
    links: Vec<SourceEntityLinkEvidence>,
) -> Vec<SourceEntityLinkEvidence> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for link in links {
        let key = serde_json::to_string(&link).unwrap_or_else(|_| format!("{link:?}"));
        if seen.insert(key) {
            out.push(link);
        }
    }
    out
}

fn canonical_subject_groups_for_provenance(provenance: &Provenance) -> Vec<CanonicalSubjectGroup> {
    let mut groups = Vec::new();
    collect_canonical_subject_group(&provenance.subject, &mut groups);
    for attribution in provenance.field_attributions.values() {
        collect_canonical_subject_group(&attribution.subject, &mut groups);
    }
    for child in &provenance.children {
        groups.extend(canonical_subject_groups_for_provenance(&child.provenance));
    }
    dedupe_canonical_subject_groups(groups)
}

fn collect_canonical_subject_group(
    attribution: &SubjectAttribution,
    groups: &mut Vec<CanonicalSubjectGroup>,
) {
    if attribution.competing_subjects.is_empty() {
        return;
    }
    let mut subjects = Vec::new();
    collect_subject_ref_members(&attribution.subject, &mut subjects);
    for competing in &attribution.competing_subjects {
        collect_subject_ref_members(&competing.subject, &mut subjects);
    }
    let subjects = distinct_subjects(&subjects);
    if subjects.len() > 1 {
        groups.push(CanonicalSubjectGroup {
            subjects,
            explicit_user_confirmed_merge: matches!(
                attribution.binding,
                SubjectBindingKind::UserConfirmed
            ),
        });
    }
}

fn dedupe_canonical_subject_groups(
    groups: Vec<CanonicalSubjectGroup>,
) -> Vec<CanonicalSubjectGroup> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for group in groups {
        let key = serde_json::to_string(&group).unwrap_or_else(|_| format!("{group:?}"));
        if seen.insert(key) {
            out.push(group);
        }
    }
    out
}

fn prompt_input_claims_from_invocation(input_json: &Value) -> Vec<IntelligenceClaim> {
    let mut claims = Vec::new();
    collect_intelligence_claims(input_json, &mut claims);
    dedupe_intelligence_claims(claims)
}

fn collect_intelligence_claims(value: &Value, claims: &mut Vec<IntelligenceClaim>) {
    if let Ok(claim) = serde_json::from_value::<IntelligenceClaim>(value.clone()) {
        claims.push(claim);
        return;
    }

    match value {
        Value::Array(values) => {
            for value in values {
                collect_intelligence_claims(value, claims);
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                collect_intelligence_claims(value, claims);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn dedupe_intelligence_claims(claims: Vec<IntelligenceClaim>) -> Vec<IntelligenceClaim> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for claim in claims {
        if seen.insert(claim.id.clone()) {
            out.push(claim);
        }
    }
    out
}

fn redacted_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..8])
}
