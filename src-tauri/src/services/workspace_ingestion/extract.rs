//! Workspace claim extraction.
//!
//! W3-A intentionally emits only narrow `UserNote` claims for verified
//! account/project/person links. Authority-bearing metadata comes from
//! `ExtractionContext`, not frontmatter or body text.

use abilities_runtime::abilities::provenance::source::{DocumentId, SourceIdentifier};
use std::fs::File;

use super::contracts::{
    ClaimSensitivity, ClaimType, DataSource, DroppedFact, DroppedFactSource, ExtractionContext,
    ExtractionError, ExtractionReport, ExtractionWarning, Extractor, SourceAttribution,
    WorkspaceCategory, WorkspaceClaimProposal, WorkspaceFileKind,
};
use super::lifecycle::LifecycleState;

const MAX_USER_NOTE_CHARS: usize = 4_000;
const FRONTMATTER_AUTHORITY_WARNING: &str = "frontmatter_authority_ignored";

pub struct WorkspaceExtractor;

impl Extractor for WorkspaceExtractor {
    fn extract(
        &self,
        _file: &mut File,
        context: &ExtractionContext<'_>,
    ) -> Result<ExtractionReport, ExtractionError> {
        extract_workspace_content(context.content, context)
    }
}

fn extract_workspace_content(
    content: &str,
    context: &ExtractionContext<'_>,
) -> Result<ExtractionReport, ExtractionError> {
    let (frontmatter, body) = split_frontmatter(content);
    let mut report = ExtractionReport::default();
    record_frontmatter_authority(frontmatter, &mut report);

    let Some(subject) = context.linked_subject else {
        report.dropped_facts.push(DroppedFact::new(
            "unlinked_subject",
            DroppedFactSource::Body,
        ));
        return Ok(report);
    };

    if !subject.is_claim_supported() {
        report.dropped_facts.push(
            DroppedFact::new("unsupported_subject_kind", DroppedFactSource::Body)
                .with_claim_type_candidate(ClaimType::UserNote.as_str()),
        );
        return Ok(report);
    }

    if !is_note_like(context) {
        report.dropped_facts.push(
            DroppedFact::new("unsupported_content_shape", DroppedFactSource::Body)
                .with_claim_type_candidate(ClaimType::UserNote.as_str()),
        );
        return Ok(report);
    }

    let text = truncate_user_note_text(body.trim());
    if text.is_empty() {
        report
            .dropped_facts
            .push(DroppedFact::new("empty_body", DroppedFactSource::Body));
        return Ok(report);
    }

    let data_source = DataSource::WorkspaceFile {
        kind: context.source_type.clone(),
    };
    let source_attribution = SourceAttribution::new(
        data_source.clone(),
        vec![SourceIdentifier::Document {
            document_id: DocumentId::new(context.file_id.to_string()),
            chunk_id: None,
        }],
        context.observed_at,
        Some(context.source_asof),
        0.5,
        None,
    )
    .map_err(|error| ExtractionError::Provenance(error.to_string()))?;

    report.proposals.push(WorkspaceClaimProposal {
        claim_type: ClaimType::UserNote,
        subject: subject.clone(),
        text,
        field_path: None,
        topic_key: None,
        source_asof: context.source_asof,
        observed_at: context.observed_at,
        data_source,
        sensitivity: ClaimSensitivity::UserOnly,
        source_attribution,
        ingestion_run_id: context.ingestion_run_id.to_string(),
        lifecycle_state: LifecycleState::Ingested,
        source_ref: format!("workspace_file:{}", context.file_id),
    });

    Ok(report)
}

fn is_note_like(context: &ExtractionContext<'_>) -> bool {
    matches!(context.resolved_category, Some(WorkspaceCategory::Notes))
        || matches!(context.source_type, WorkspaceFileKind::Inbox)
}

fn truncate_user_note_text(text: &str) -> String {
    if text.chars().count() <= MAX_USER_NOTE_CHARS {
        return text.to_string();
    }
    text.chars().take(MAX_USER_NOTE_CHARS).collect()
}

fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    else {
        return (None, content);
    };
    let Some(end) = rest.find("\n---") else {
        return (None, content);
    };

    let frontmatter = &rest[..end];
    let body_start = end + "\n---".len();
    let body = rest[body_start..]
        .strip_prefix("\r\n")
        .or_else(|| rest[body_start..].strip_prefix('\n'))
        .unwrap_or(&rest[body_start..]);
    (Some(frontmatter), body)
}

fn record_frontmatter_authority(frontmatter: Option<&str>, report: &mut ExtractionReport) {
    let Some(frontmatter) = frontmatter else {
        return;
    };

    let mut ignored = false;
    for line in frontmatter.lines() {
        let Some((key, _value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if !is_authority_frontmatter_key(key) {
            continue;
        }
        ignored = true;
        report.dropped_facts.push(
            DroppedFact::new(
                FRONTMATTER_AUTHORITY_WARNING,
                DroppedFactSource::Frontmatter,
            )
            .with_field(key.to_ascii_lowercase()),
        );
    }

    if ignored {
        report.warnings.push(ExtractionWarning::new(
            FRONTMATTER_AUTHORITY_WARNING,
            "frontmatter authority fields ignored",
        ));
    }
}

fn is_authority_frontmatter_key(key: &str) -> bool {
    matches!(
        key.trim().to_ascii_lowercase().as_str(),
        "confidence"
            | "source-of-truth"
            | "source_of_truth"
            | "trust"
            | "evidence_weight"
            | "subject_ref"
            | "account"
            | "company"
            | "domain"
            | "owner"
            | "actor"
            | "sensitivity"
            | "privacy"
            | "source_asof"
            | "source-asof"
            | "data_source"
            | "data-source"
            | "claim_type"
            | "claim-type"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use crate::services::workspace_ingestion::contracts::{FileIdentity, ResolvedLinkedSubject};
    use chrono::{TimeZone, Utc};
    use std::path::PathBuf;

    fn identity() -> FileIdentity {
        FileIdentity {
            canonical_path: PathBuf::from("/tmp/workspace/Accounts/acme/notes.md"),
            device: 1,
            inode: 2,
        }
    }

    fn subject(entity_type: EntityType) -> ResolvedLinkedSubject {
        ResolvedLinkedSubject {
            entity_type,
            entity_id: "acme".to_string(),
            entity_name: Some("Acme".to_string()),
            link_id: "link-1".to_string(),
        }
    }

    fn context<'a>(
        identity: &'a FileIdentity,
        linked_subject: Option<&'a ResolvedLinkedSubject>,
        category: Option<&'a WorkspaceCategory>,
    ) -> ExtractionContext<'a> {
        ExtractionContext {
            file_id: "wf-1",
            identity,
            content: "",
            source_type: WorkspaceFileKind::EntityDoc,
            source_asof: Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap(),
            resolved_category: category,
            linked_subject,
            ingestion_run_id: "run-1",
            observed_at: Utc.with_ymd_and_hms(2026, 5, 21, 12, 0, 0).unwrap(),
            invocation_actor: "user",
        }
    }

    #[test]
    fn emits_user_only_user_note_for_verified_note_subject() {
        let identity = identity();
        let subject = subject(EntityType::Account);
        let category = WorkspaceCategory::Notes;
        let report = extract_workspace_content(
            "---\ndoc_type: note\n---\nImportant local context.",
            &context(&identity, Some(&subject), Some(&category)),
        )
        .expect("extract");

        assert_eq!(report.proposals.len(), 1);
        let proposal = &report.proposals[0];
        assert_eq!(proposal.claim_type, ClaimType::UserNote);
        assert_eq!(proposal.text, "Important local context.");
        assert_eq!(proposal.subject.entity_id, "acme");
        assert_eq!(proposal.sensitivity, ClaimSensitivity::UserOnly);
        assert_eq!(proposal.source_ref, "workspace_file:wf-1");
    }

    #[test]
    fn ignores_frontmatter_authority_and_keeps_body_sensitivity_floor() {
        let identity = identity();
        let subject = subject(EntityType::Account);
        let category = WorkspaceCategory::Notes;
        let report = extract_workspace_content(
            "---\ndoc_type: note\nconfidence: 1.0\nprivacy: public\nsubject_ref: other\n---\nBody.",
            &context(&identity, Some(&subject), Some(&category)),
        )
        .expect("extract");

        assert_eq!(report.proposals.len(), 1);
        assert_eq!(report.proposals[0].sensitivity, ClaimSensitivity::UserOnly);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.code == FRONTMATTER_AUTHORITY_WARNING));
        assert_eq!(
            report
                .dropped_facts
                .iter()
                .filter(|drop| drop.reason == FRONTMATTER_AUTHORITY_WARNING)
                .count(),
            3
        );
    }

    #[test]
    fn crlf_frontmatter_authority_is_ignored_not_claim_text() {
        let identity = identity();
        let subject = subject(EntityType::Account);
        let category = WorkspaceCategory::Notes;
        let report = extract_workspace_content(
            "---\r\nprivacy: public\r\nconfidence: 1.0\r\n---\r\nBody.",
            &context(&identity, Some(&subject), Some(&category)),
        )
        .expect("extract");

        assert_eq!(report.proposals.len(), 1);
        assert_eq!(report.proposals[0].text, "Body.");
        assert_eq!(
            report
                .dropped_facts
                .iter()
                .filter(|drop| drop.reason == FRONTMATTER_AUTHORITY_WARNING)
                .count(),
            2
        );
    }

    #[test]
    fn unlinked_content_drops_without_claim() {
        let identity = identity();
        let category = WorkspaceCategory::Notes;
        let report = extract_workspace_content(
            "Useful context.",
            &context(&identity, None, Some(&category)),
        )
        .expect("extract");

        assert!(report.proposals.is_empty());
        assert!(report
            .dropped_facts
            .iter()
            .any(|drop| drop.reason == "unlinked_subject"));
    }

    #[test]
    fn unsupported_subject_kind_drops_without_claim() {
        let identity = identity();
        let subject = subject(EntityType::Other);
        let category = WorkspaceCategory::Notes;
        let report = extract_workspace_content(
            "Useful context.",
            &context(&identity, Some(&subject), Some(&category)),
        )
        .expect("extract");

        assert!(report.proposals.is_empty());
        assert!(report
            .dropped_facts
            .iter()
            .any(|drop| drop.reason == "unsupported_subject_kind"));
    }
}
