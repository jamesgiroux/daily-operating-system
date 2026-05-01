use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use super::envelope::{
    AbilityExecutionMode, AbilityOutput, AbilityVersion, Actor, ComposedProvenance, CompositionId,
    InputsSnapshot, InvocationId, PromptFingerprint, Provenance, ProvenanceWarning, SchemaVersion,
    SourceTimestampFallback, PROVENANCE_SCHEMA_VERSION,
};
use super::field::{FieldAttribution, FieldAttributionError, FieldPath, SourceRef};
use super::source::{SourceAttribution, SourceIndex};
use super::subject::{SubjectAttribution, SubjectFitStatus};
use super::trust::TrustAssessment;
use crate::abilities::registry::AbilityCategory;

pub const SOFT_PROVENANCE_BUDGET_BYTES: usize = 100 * 1024;
pub const HARD_PROVENANCE_BUDGET_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ProvenanceBuilderConfig {
    pub ability_name: String,
    pub ability_version: AbilityVersion,
    pub ability_schema_version: SchemaVersion,
    pub invocation_id: InvocationId,
    pub produced_at: DateTime<Utc>,
    pub inputs_snapshot: InputsSnapshot,
    pub actor: Actor,
    pub mode: AbilityExecutionMode,
    pub category: AbilityCategory,
    pub declared_composition_ids: BTreeSet<CompositionId>,
}

impl ProvenanceBuilderConfig {
    pub fn new(ability_name: impl Into<String>, produced_at: DateTime<Utc>) -> Self {
        Self {
            ability_name: ability_name.into(),
            ability_version: AbilityVersion::new(1, 0),
            ability_schema_version: SchemaVersion(1),
            invocation_id: InvocationId::new("invocation-fixture"),
            produced_at,
            inputs_snapshot: InputsSnapshot::default(),
            actor: Actor::System {
                component: "fixture".into(),
            },
            mode: AbilityExecutionMode::Evaluate,
            category: AbilityCategory::Read,
            declared_composition_ids: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProvenanceBuilder {
    config: ProvenanceBuilderConfig,
    sources: Vec<SourceAttribution>,
    children: Vec<ComposedProvenance>,
    field_attributions: BTreeMap<FieldPath, FieldAttribution>,
    subtree_attributions: BTreeMap<FieldPath, FieldAttribution>,
    subject: Option<SubjectAttribution>,
    prompt_fingerprint: Option<PromptFingerprint>,
    warnings: Vec<ProvenanceWarning>,
    thread_ids: Vec<super::ThreadId>,
}

impl ProvenanceBuilder {
    pub fn new(config: ProvenanceBuilderConfig) -> Self {
        Self {
            config,
            sources: Vec::new(),
            children: Vec::new(),
            field_attributions: BTreeMap::new(),
            subtree_attributions: BTreeMap::new(),
            subject: None,
            prompt_fingerprint: None,
            warnings: Vec::new(),
            thread_ids: Vec::new(),
        }
    }

    pub fn set_subject(&mut self, subject: SubjectAttribution) -> &mut Self {
        self.subject = Some(subject);
        self
    }

    pub fn set_prompt_fingerprint(&mut self, prompt_fingerprint: PromptFingerprint) -> &mut Self {
        self.prompt_fingerprint = Some(prompt_fingerprint);
        self
    }

    pub fn add_thread_id(&mut self, thread_id: super::ThreadId) -> &mut Self {
        self.thread_ids.push(thread_id);
        self
    }

    pub fn add_warning(&mut self, warning: ProvenanceWarning) -> &mut Self {
        self.warnings.push(warning);
        self
    }

    pub fn add_source(&mut self, source: SourceAttribution) -> SourceIndex {
        let index = SourceIndex(self.sources.len());
        if source.source_asof.is_none() {
            self.warnings.push(ProvenanceWarning::SourceTimestampUnknown {
                source_index: index,
                fallback: SourceTimestampFallback::ObservedAt,
            });
        }
        self.sources.push(source);
        index
    }

    pub fn compose(
        &mut self,
        composition_id: CompositionId,
        child: Provenance,
    ) -> Result<&mut Self, ProvenanceError> {
        if !self.config.declared_composition_ids.is_empty()
            && !self.config.declared_composition_ids.contains(&composition_id)
        {
            return Err(ProvenanceError::InvalidCompositionId { composition_id });
        }

        self.children
            .push(ComposedProvenance::new(composition_id, child));
        Ok(self)
    }

    pub fn attribute(
        &mut self,
        field_path: FieldPath,
        attribution: FieldAttribution,
    ) -> Result<&mut Self, ProvenanceError> {
        attribution.validate().map_err(ProvenanceError::Field)?;
        self.field_attributions.insert(field_path, attribution);
        Ok(self)
    }

    pub fn attribute_subtree(
        &mut self,
        field_path: FieldPath,
        attribution: FieldAttribution,
    ) -> Result<&mut Self, ProvenanceError> {
        attribution.validate().map_err(ProvenanceError::Field)?;
        self.subtree_attributions.insert(field_path, attribution);
        Ok(self)
    }

    pub fn pass_through(
        &mut self,
        field_path: FieldPath,
        subject: SubjectAttribution,
        source_index: SourceIndex,
    ) -> Result<&mut Self, ProvenanceError> {
        self.attribute(field_path, FieldAttribution::direct(subject, source_index))
    }

    /// Finalizes an ability output by serializing `data`, walking its JSON-pointer
    /// leaves, and verifying every leaf has field attribution.
    ///
    /// This is the DOS-211 interpretation of ADR-0105 §7 / plan §41:
    /// "build time" means `ProvenanceBuilder::finalize()` time at the ability
    /// return boundary, not Rust compile time. The builder rejects missing
    /// attribution before returning `AbilityOutput<T>`.
    pub fn finalize<T: Serialize>(mut self, data: T) -> Result<AbilityOutput<T>, ProvenanceError> {
        let serialized = serde_json::to_value(&data).map_err(ProvenanceError::SerializeData)?;
        let subject = self.subject.clone().ok_or(ProvenanceError::MissingSubject)?;

        self.apply_subtree_attributions(&serialized)?;
        self.validate_leaf_coverage(&serialized)?;
        self.validate_field_attributions(&subject)?;

        let trust = TrustAssessment::compute(
            &self.sources,
            &self.children,
            &self.field_attributions,
            self.config.category,
            self.prompt_fingerprint.is_some(),
        );

        let mut provenance = Provenance {
            provenance_schema_version: PROVENANCE_SCHEMA_VERSION,
            ability_name: self.config.ability_name,
            ability_version: self.config.ability_version.clone(),
            ability_schema_version: self.config.ability_schema_version,
            invocation_id: self.config.invocation_id,
            produced_at: self.config.produced_at,
            inputs_snapshot: self.config.inputs_snapshot,
            actor: self.config.actor,
            mode: self.config.mode,
            trust,
            sources: self.sources,
            thread_ids: self.thread_ids,
            prompt_fingerprint: self.prompt_fingerprint,
            children: self.children,
            field_attributions: self.field_attributions,
            subject,
            warnings: self.warnings,
        };

        enforce_size_budget(&mut provenance)?;

        Ok(AbilityOutput::new(data, provenance))
    }

    fn apply_subtree_attributions(&mut self, serialized: &Value) -> Result<(), ProvenanceError> {
        if self.subtree_attributions.is_empty() {
            return Ok(());
        }

        let leaf_paths = json_leaf_paths(serialized)?;
        for leaf_path in leaf_paths {
            if self.field_attributions.contains_key(&leaf_path) {
                continue;
            }

            if let Some((_, attribution)) = self
                .subtree_attributions
                .iter()
                .filter(|(candidate, _)| candidate.covers(&leaf_path))
                .max_by_key(|(candidate, _)| candidate.as_str().len())
            {
                self.field_attributions
                    .insert(leaf_path, attribution.clone());
            }
        }

        Ok(())
    }

    fn validate_leaf_coverage(&self, serialized: &Value) -> Result<(), ProvenanceError> {
        for leaf_path in json_leaf_paths(serialized)? {
            if !self.field_attributions.contains_key(&leaf_path) {
                return Err(ProvenanceError::MissingFieldAttribution {
                    field_path: leaf_path,
                });
            }
        }

        Ok(())
    }

    fn validate_field_attributions(
        &self,
        envelope_subject: &SubjectAttribution,
    ) -> Result<(), ProvenanceError> {
        if !envelope_subject.is_confident() {
            return Err(ProvenanceError::NonConfidentSubject {
                field_path: None,
                status: envelope_subject.fit.status.clone(),
            });
        }

        let child_ids = self
            .children
            .iter()
            .map(|child| child.composition_id.clone())
            .collect::<BTreeSet<_>>();

        for (field_path, attribution) in &self.field_attributions {
            attribution.validate().map_err(ProvenanceError::Field)?;

            if !attribution.subject.is_confident() {
                return Err(ProvenanceError::NonConfidentSubject {
                    field_path: Some(field_path.clone()),
                    status: attribution.subject.fit.status.clone(),
                });
            }

            if !attribution.subject.is_coherent_with(envelope_subject) {
                return Err(ProvenanceError::SubjectMismatch {
                    field_path: field_path.clone(),
                });
            }

            for source_ref in &attribution.source_refs {
                match source_ref {
                    SourceRef::Source { source_index } => {
                        if source_index.as_usize() >= self.sources.len() {
                            return Err(ProvenanceError::InvalidSourceIndex {
                                field_path: field_path.clone(),
                                source_index: *source_index,
                            });
                        }
                    }
                    SourceRef::Child {
                        composition_id, ..
                    } => {
                        if !child_ids.contains(composition_id) {
                            return Err(ProvenanceError::InvalidCompositionId {
                                composition_id: composition_id.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn json_leaf_paths(value: &Value) -> Result<Vec<FieldPath>, ProvenanceError> {
    let mut paths = Vec::new();
    collect_leaf_paths(value, String::new(), &mut paths)?;
    Ok(paths)
}

fn collect_leaf_paths(
    value: &Value,
    current_path: String,
    paths: &mut Vec<FieldPath>,
) -> Result<(), ProvenanceError> {
    match value {
        Value::Object(map) if !map.is_empty() => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                let child_path = format!("{}/{}", current_path, escape_json_pointer_token(key));
                collect_leaf_paths(&map[key], child_path, paths)?;
            }
            Ok(())
        }
        Value::Array(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                let child_path = format!("{current_path}/{index}");
                collect_leaf_paths(item, child_path, paths)?;
            }
            Ok(())
        }
        _ => {
            paths.push(FieldPath::new(current_path).map_err(ProvenanceError::Field)?);
            Ok(())
        }
    }
}

fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn enforce_size_budget(provenance: &mut Provenance) -> Result<(), ProvenanceError> {
    let mut bytes = serialized_size(provenance)?;
    if bytes > SOFT_PROVENANCE_BUDGET_BYTES {
        provenance
            .warnings
            .push(ProvenanceWarning::SoftSizeLimitExceeded {
                bytes,
                soft_budget_bytes: SOFT_PROVENANCE_BUDGET_BYTES,
            });
        bytes = serialized_size(provenance)?;
    }

    while bytes > HARD_PROVENANCE_BUDGET_BYTES {
        let Some(path) = deepest_child_path(provenance) else {
            return Err(ProvenanceError::ProvenanceTooLarge {
                bytes,
                hard_budget_bytes: HARD_PROVENANCE_BUDGET_BYTES,
            });
        };

        let elision = elide_child_at_path(provenance, &path);
        provenance.warnings.push(ProvenanceWarning::DepthElided {
            skipped_levels: elision.skipped_levels,
            elided_children: elision.elided_children,
            aggregate_source_count: elision.aggregate_source_count,
            effective_trust: elision.effective_trust,
        });
        bytes = serialized_size(provenance)?;
    }

    Ok(())
}

fn serialized_size(provenance: &Provenance) -> Result<usize, ProvenanceError> {
    serde_json::to_vec(provenance)
        .map(|bytes| bytes.len())
        .map_err(ProvenanceError::SerializeProvenance)
}

fn deepest_child_path(provenance: &Provenance) -> Option<Vec<usize>> {
    fn walk(provenance: &Provenance, prefix: Vec<usize>, best: &mut Option<Vec<usize>>) {
        for (index, child) in provenance.children.iter().enumerate() {
            let mut path = prefix.clone();
            path.push(index);
            if best.as_ref().is_none_or(|candidate| path.len() > candidate.len()) {
                *best = Some(path.clone());
            }
            walk(&child.provenance, path, best);
        }
    }

    let mut best = None;
    walk(provenance, Vec::new(), &mut best);
    best
}

fn elide_child_at_path(provenance: &mut Provenance, path: &[usize]) -> ElisionSummary {
    let mut current = provenance;
    for index in &path[..path.len() - 1] {
        current = &mut current.children[*index].provenance;
    }

    let child = &mut current.children[*path.last().expect("path is non-empty")].provenance;
    let summary = summarize_subtree(child);
    child.sources.clear();
    child.children.clear();
    child.field_attributions.clear();
    child.warnings.clear();
    child.warnings.push(ProvenanceWarning::DepthElided {
        skipped_levels: summary.skipped_levels,
        elided_children: summary.elided_children,
        aggregate_source_count: summary.aggregate_source_count,
        effective_trust: summary.effective_trust,
    });
    summary
}

fn summarize_subtree(provenance: &Provenance) -> ElisionSummary {
    let mut summary = ElisionSummary {
        skipped_levels: 1,
        elided_children: provenance.children.len() as u32,
        aggregate_source_count: provenance.sources.len() as u32,
        effective_trust: provenance.trust.effective,
    };

    for child in &provenance.children {
        let child_summary = summarize_subtree(&child.provenance);
        summary.skipped_levels = summary.skipped_levels.max(child_summary.skipped_levels + 1);
        summary.elided_children += 1 + child_summary.elided_children;
        summary.aggregate_source_count += child_summary.aggregate_source_count;
        if child_summary.effective_trust != summary.effective_trust {
            summary.effective_trust = super::EffectiveTrust::Untrusted;
        }
    }

    summary
}

struct ElisionSummary {
    skipped_levels: u32,
    elided_children: u32,
    aggregate_source_count: u32,
    effective_trust: super::EffectiveTrust,
}

#[derive(Debug, thiserror::Error)]
pub enum ProvenanceError {
    #[error("failed to serialize ability output for provenance coverage check: {0}")]
    SerializeData(serde_json::Error),
    #[error("failed to serialize provenance envelope for size budget check: {0}")]
    SerializeProvenance(serde_json::Error),
    #[error("missing top-level subject attribution")]
    MissingSubject,
    #[error("missing field attribution for {field_path:?}")]
    MissingFieldAttribution { field_path: FieldPath },
    #[error("field attribution invariant failed: {0}")]
    Field(FieldAttributionError),
    #[error("field attribution references invalid source index {source_index:?} at {field_path:?}")]
    InvalidSourceIndex {
        field_path: FieldPath,
        source_index: SourceIndex,
    },
    #[error("invalid composition id {composition_id:?}")]
    InvalidCompositionId { composition_id: CompositionId },
    #[error("subject fit is not confident for {field_path:?}: {status:?}")]
    NonConfidentSubject {
        field_path: Option<FieldPath>,
        status: SubjectFitStatus,
    },
    #[error("field subject is not coherent with envelope subject at {field_path:?}")]
    SubjectMismatch { field_path: FieldPath },
    #[error("provenance remains too large after child elision: {bytes} > {hard_budget_bytes}")]
    ProvenanceTooLarge {
        bytes: usize,
        hard_budget_bytes: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::provenance::{
        Confidence, DataSource, FieldAttribution, FieldPath, PromptTemplateId, PromptVersion,
        SanitizedExplanation, SourceAttribution, SubjectBindingKind, SubjectFitAssessment,
        SubjectRef,
    };
    use chrono::TimeZone;
    use serde::Serialize;
    use serde_json::json;

    #[derive(Debug, Serialize)]
    struct FixtureOutput {
        name: String,
        risk: u8,
    }

    fn produced_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap()
    }

    fn subject() -> SubjectAttribution {
        SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()))
    }

    fn source() -> SourceAttribution {
        SourceAttribution::new(
            DataSource::Google,
            Vec::new(),
            produced_at(),
            Some(produced_at()),
            1.0,
            None,
        )
        .unwrap()
    }

    fn builder() -> ProvenanceBuilder {
        let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
            "fixture_ability",
            produced_at(),
        ));
        builder.set_subject(subject());
        builder
    }

    #[test]
    fn provenance_finalize_rejects_missing_field_attribution() {
        let mut builder = builder();
        let source_index = builder.add_source(source());
        builder
            .pass_through(FieldPath::new("/name").unwrap(), subject(), source_index)
            .unwrap();

        let err = builder
            .finalize(FixtureOutput {
                name: "acct-1".into(),
                risk: 2,
            })
            .unwrap_err();

        assert!(matches!(
            err,
            ProvenanceError::MissingFieldAttribution { field_path } if field_path == FieldPath::new("/risk").unwrap()
        ));
    }

    #[test]
    fn provenance_direct_projection_autopopulates_field_attribution() {
        let mut builder = builder();
        let source_index = builder.add_source(source());
        builder
            .pass_through(FieldPath::new("/name").unwrap(), subject(), source_index)
            .unwrap()
            .pass_through(FieldPath::new("/risk").unwrap(), subject(), source_index)
            .unwrap();

        let output = builder
            .finalize(FixtureOutput {
                name: "acct-1".into(),
                risk: 2,
            })
            .unwrap();

        assert_eq!(output.provenance.field_attributions.len(), 2);
    }

    #[test]
    fn provenance_finalize_rejects_ambiguous_subject_fit() {
        let ambiguous_subject = SubjectAttribution::new(
            SubjectRef::Account("acct-1".into()),
            SubjectBindingKind::Inferred,
            Vec::new(),
            Vec::new(),
            SubjectFitAssessment::ambiguous("fixture", 0.4).unwrap(),
        )
        .unwrap();
        let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
            "fixture_ability",
            produced_at(),
        ));
        builder.set_subject(ambiguous_subject.clone());
        let source_index = builder.add_source(source());
        builder
            .attribute(
                FieldPath::new("/name").unwrap(),
                FieldAttribution::direct(ambiguous_subject, source_index),
            )
            .unwrap();

        let err = builder.finalize(json!({ "name": "acct-1" })).unwrap_err();

        assert!(matches!(
            err,
            ProvenanceError::NonConfidentSubject {
                field_path: None,
                status: SubjectFitStatus::Ambiguous
            }
        ));
    }

    #[test]
    fn subtree_attribution_applies_to_leaf_paths() {
        let mut builder = builder();
        let source_index = builder.add_source(source());
        builder
            .attribute_subtree(
                FieldPath::new("/account").unwrap(),
                FieldAttribution::direct(subject(), source_index),
            )
            .unwrap();

        let output = builder
            .finalize(json!({ "account": { "name": "acct-1", "risk": 2 } }))
            .unwrap();

        assert!(output
            .provenance
            .field_attributions
            .contains_key(&FieldPath::new("/account/name").unwrap()));
        assert!(output
            .provenance
            .field_attributions
            .contains_key(&FieldPath::new("/account/risk").unwrap()));
    }

    #[test]
    fn prompt_fingerprint_sets_untrusted() {
        let mut builder = builder();
        let source_index = builder.add_source(source());
        builder
            .pass_through(FieldPath::new("/name").unwrap(), subject(), source_index)
            .unwrap();
        builder.set_prompt_fingerprint(PromptFingerprint {
            provider: "replay".into(),
            model: crate::abilities::provenance::ModelName("replay".into()),
            prompt_template_id: PromptTemplateId("fixture".into()),
            prompt_template_version: PromptVersion("1.0.0".into()),
            canonical_prompt_hash: crate::abilities::provenance::HashValue::new("hash"),
            temperature: 0.0,
            top_p: None,
            seed: Some(42),
            tokens_input: None,
            tokens_output: None,
            provider_completion_id: None,
        });

        let output = builder.finalize(json!({ "name": "acct-1" })).unwrap();

        assert_eq!(
            output.provenance.trust.effective,
            crate::abilities::provenance::EffectiveTrust::Untrusted
        );
    }

    #[test]
    fn provenance_size_budget_warns_then_depth_elides() {
        let mut child = super::super::envelope::provenance_for_test(
            "child",
            produced_at(),
            subject(),
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            None,
            Vec::new(),
        );
        child.field_attributions.insert(
            FieldPath::new("/blob").unwrap(),
            FieldAttribution::constant(subject()),
        );
        child.warnings.push(ProvenanceWarning::SourceUnresolvable {
            source_index: SourceIndex(0),
            reason: "x".repeat(HARD_PROVENANCE_BUDGET_BYTES),
        });

        let mut builder = builder();
        let source_index = builder.add_source(source());
        builder
            .compose(CompositionId::new("child"), child)
            .unwrap()
            .pass_through(FieldPath::new("/name").unwrap(), subject(), source_index)
            .unwrap();

        let output = builder.finalize(json!({ "name": "acct-1" })).unwrap();

        assert!(output
            .provenance
            .warnings
            .iter()
            .any(|warning| matches!(warning, ProvenanceWarning::SoftSizeLimitExceeded { .. })));
        assert!(output
            .provenance
            .warnings
            .iter()
            .any(|warning| matches!(warning, ProvenanceWarning::DepthElided { .. })));
    }

    #[test]
    fn declared_confidence_with_explanation_finalizes() {
        let mut builder = builder();
        let source_index = builder.add_source(source());
        let explanation = SanitizedExplanation::new("Structured fixture basis").unwrap();
        let confidence = Confidence::declared(0.8, &explanation).unwrap();
        builder
            .attribute(
                FieldPath::new("/name").unwrap(),
                FieldAttribution::new(
                    subject(),
                    crate::abilities::provenance::DerivationKind::Direct,
                    vec![SourceRef::Source { source_index }],
                    confidence,
                    Some(explanation),
                )
                .unwrap(),
            )
            .unwrap();

        let output = builder.finalize(json!({ "name": "acct-1" })).unwrap();

        assert_eq!(output.provenance.field_attributions.len(), 1);
    }

    #[test]
    fn json_leaf_walk_escapes_pointer_tokens() {
        let paths = json_leaf_paths(&json!({ "a/b": { "c~d": true } })).unwrap();

        assert_eq!(paths, vec![FieldPath::new("/a~1b/c~0d").unwrap()]);
    }
}
