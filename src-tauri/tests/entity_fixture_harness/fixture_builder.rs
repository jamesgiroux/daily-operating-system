//! Programmatic fixture builders.
//!
//! Each fixture is a real `EntityIntelligenceEnvelope` serialized to JSON
//! under `tests/entity_fixture_harness/fixtures/`. This module owns the
//! authoring logic. The `#[test] generate_fixtures` function is gated on
//! `DOS461_REGEN=1` and rewrites every fixture file on disk — so fixtures
//! stay reproducible without check-in churn.
//!
//! Fixture content is PII-safe synthetic per CLAUDE.md "No customer-specific
//! data in source code". All names/domains follow the generic patterns from
//! `subsidiary.com` / `parent.com` / `user@example.com` etc.

use std::collections::BTreeMap;

use abilities_runtime::abilities::get_entity_intelligence::{
    CandidateSetRef, EmptyReason, EntityFact, EntityIntelligenceEnvelope, EntityKind,
    EnvelopeProvenance, EnvelopeProvenanceSource, EnvelopeSection, EnvelopeTrustSummary, Freshness,
    InclusionReason, MetadataProposal, NormalizedSubject, OpenLoopWithReceipt, Paginated,
    ProvenanceRef, ReceiptTargetRef, RecordEntry, SectionState, SubjectScope, ThreadSummary,
    Touchpoint, TouchpointBundle, TouchpointKind, ENVELOPE_SCHEMA_VERSION,
};
use abilities_runtime::abilities::provenance::SubjectRef;
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::sensitivity::{
    ClaimVerificationState, RedactionAffordance, RenderPolicy, RenderPolicyKind, RenderSurface,
    RenderableClaimText,
};
use abilities_runtime::types::{ClaimSensitivity, ClaimState, SurfacingState};
use chrono::{Duration, TimeZone, Utc};

use crate::harness::fixtures_dir;

/// Anchor a reproducible "now" for fixture timestamps. Synthetic.
fn now_anchor() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap()
}

fn render_policy_render(claim_id: &str, sensitivity: ClaimSensitivity) -> RenderPolicy {
    RenderPolicy {
        kind: RenderPolicyKind::Render,
        sensitivity,
        surface: RenderSurface::TauriEntityDetail,
        claim_id: Some(claim_id.to_string()),
        affordance: None,
    }
}

fn render_policy_redacted(claim_id: &str, sensitivity: ClaimSensitivity) -> RenderPolicy {
    RenderPolicy {
        kind: RenderPolicyKind::Redacted,
        sensitivity,
        surface: RenderSurface::TauriEntityDetail,
        claim_id: Some(claim_id.to_string()),
        affordance: Some(RedactionAffordance::ConfidentialHidden {
            label: "Confidential claim hidden".to_string(),
        }),
    }
}

fn rendered(claim_id: &str, text: &str, sensitivity: ClaimSensitivity) -> RenderableClaimText {
    RenderableClaimText {
        text: text.to_string(),
        policy: render_policy_render(claim_id, sensitivity),
    }
}

fn rendered_redacted(claim_id: &str, sensitivity: ClaimSensitivity) -> RenderableClaimText {
    RenderableClaimText {
        text: "Confidential claim hidden".to_string(),
        policy: render_policy_redacted(claim_id, sensitivity),
    }
}

// Test-fixture builder helper — positional args keep call sites compact and
// scannable in this single-purpose authoring module. The 8 args are all
// distinct fact attributes; restructuring into a builder would obscure
// fixture intent without runtime value.
#[allow(clippy::too_many_arguments)]
fn fact(
    claim_id: &str,
    subject: SubjectRef,
    claim_type: &str,
    text: &str,
    band: TrustBand,
    freshness: Freshness,
    sensitivity: ClaimSensitivity,
    lifecycle: ClaimState,
) -> EntityFact {
    EntityFact {
        claim_id: claim_id.to_string(),
        subject_ref: subject,
        field_path: Some("body".to_string()),
        claim_type: claim_type.to_string(),
        rendered_text: rendered(claim_id, text, sensitivity.clone()),
        trust_band: band,
        freshness,
        source_asof: Some(now_anchor() - Duration::days(2)),
        sensitivity,
        lifecycle_state: lifecycle,
        surfacing_state: SurfacingState::Active,
        verification_state: ClaimVerificationState::Active,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    }
}

fn empty_section_state() -> SectionState {
    SectionState::Empty {
        reason: EmptyReason::NotRequested,
    }
}

fn present_section_state(count: u64) -> SectionState {
    SectionState::Present { item_count: count }
}

fn default_sections() -> BTreeMap<EnvelopeSection, SectionState> {
    let mut map = BTreeMap::new();
    for s in EnvelopeSection::ALL.iter() {
        map.insert(*s, empty_section_state());
    }
    map
}

fn account_subject(id: &str) -> NormalizedSubject {
    NormalizedSubject {
        kind: EntityKind::Account,
        id: id.to_string(),
        subject_ref: SubjectRef::Account(id.to_string()),
        display_label: format!("Account {id}"),
    }
}

fn project_subject(id: &str) -> NormalizedSubject {
    NormalizedSubject {
        kind: EntityKind::Project,
        id: id.to_string(),
        subject_ref: SubjectRef::Project(id.to_string()),
        display_label: format!("Project {id}"),
    }
}

fn person_subject(id: &str) -> NormalizedSubject {
    NormalizedSubject {
        kind: EntityKind::Person,
        id: id.to_string(),
        subject_ref: SubjectRef::Person(id.to_string()),
        display_label: format!("Person {id}"),
    }
}

fn base_envelope(subject: NormalizedSubject) -> EntityIntelligenceEnvelope {
    EntityIntelligenceEnvelope {
        schema_version: ENVELOPE_SCHEMA_VERSION,
        subject,
        sections: default_sections(),
        facts: Paginated::empty_stable(),
        health_story: None,
        metadata_proposals: Paginated::empty_stable(),
        open_loops: Paginated::empty_stable(),
        touchpoints: Paginated::empty_stable(),
        threads: Paginated::empty_stable(),
        record_entries: Paginated::empty_stable(),
        trust: EnvelopeTrustSummary::unscored(),
        provenance: EnvelopeProvenance::empty(),
        sensitivity: ClaimSensitivity::Internal,
    }
}

fn glean_source(id: &str) -> EnvelopeProvenanceSource {
    EnvelopeProvenanceSource {
        id: id.to_string(),
        label: "Glean document (redacted)".to_string(),
        source_type: Some("glean".to_string()),
        as_of: Some(now_anchor()),
        redacted: true,
    }
}

fn generic_source(id: &str, label: &str, source_type: &str) -> EnvelopeProvenanceSource {
    EnvelopeProvenanceSource {
        id: id.to_string(),
        label: label.to_string(),
        source_type: Some(source_type.to_string()),
        as_of: Some(now_anchor()),
        redacted: false,
    }
}

fn touchpoint_bundle(
    primary: SubjectRef,
    also: Vec<SubjectRef>,
    upcoming: Vec<Touchpoint>,
    recent: Vec<Touchpoint>,
) -> TouchpointBundle {
    TouchpointBundle {
        upcoming: Paginated::stable(upcoming),
        recent: Paginated::stable(recent),
        candidate_set: CandidateSetRef {
            window_start: Some(now_anchor() - Duration::days(30)),
            window_end: Some(now_anchor() + Duration::days(14)),
            filter_description: "default 30/14 window".to_string(),
        },
        empty_reason: None,
        subject_scope: SubjectScope {
            primary,
            also_includes: also,
        },
    }
}

fn make_touchpoint(
    primary_subject: SubjectRef,
    when_offset_days: i64,
    kind: TouchpointKind,
) -> Touchpoint {
    Touchpoint {
        meeting_id: Some("meeting-synthetic-1".to_string()),
        kind,
        when: now_anchor() + Duration::days(when_offset_days),
        subject_ref: primary_subject,
        inclusion_reason: InclusionReason::SubjectMatch,
        exclusion_reason: None,
        trust_band: TrustBand::LikelyCurrent,
        freshness: Freshness::Current,
        provenance: ProvenanceRef::from_ids(["src-tp-1".to_string()]),
    }
}

// ---- per-subject fact-only fixtures ---------------------------------------

fn account_stale_fact() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let f = fact(
        "claim-acct-stale-1",
        SubjectRef::Account("acct-zero".to_string()),
        "account.profile.note",
        "Last known account note — sourced 8 months ago",
        TrustBand::UseWithCaution,
        Freshness::Stale,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic email thread",
        "email_thread",
    ));
    env
}

fn account_corrected_superseded() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let superseded = fact(
        "claim-acct-superseded-1",
        SubjectRef::Account("acct-zero".to_string()),
        "account.profile.contract_tier",
        "tier-bronze (superseded)",
        TrustBand::UseWithCaution,
        Freshness::Aging,
        ClaimSensitivity::Internal,
        ClaimState::Withdrawn,
    );
    let current = fact(
        "claim-acct-superseded-2",
        SubjectRef::Account("acct-zero".to_string()),
        "account.profile.contract_tier",
        "tier-gold",
        TrustBand::LikelyCurrent,
        Freshness::Current,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    let recorded = RecordEntry {
        claim_id: "claim-acct-superseded-1".to_string(),
        subject_ref: SubjectRef::Account("acct-zero".to_string()),
        claim_type: "account.profile.contract_tier".to_string(),
        recorded_at: now_anchor() - Duration::days(45),
        rendered_text: rendered(
            "claim-acct-superseded-1",
            "tier-bronze (superseded)",
            ClaimSensitivity::Internal,
        ),
        trust_band: TrustBand::UseWithCaution,
        sensitivity: ClaimSensitivity::Internal,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(2));
    env.sections
        .insert(EnvelopeSection::Record, present_section_state(1));
    env.facts = Paginated::stable(vec![superseded, current]);
    env.record_entries = Paginated::stable(vec![recorded]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Salesforce sync (synthetic)",
        "salesforce",
    ));
    env
}

fn account_low_trust() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let f = fact(
        "claim-acct-low-1",
        SubjectRef::Account("acct-zero".to_string()),
        "account.profile.renewal_signal",
        "Mentioned wanting to renew in passing on Slack",
        TrustBand::NeedsVerification,
        Freshness::Aging,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Slack message (synthetic)",
        "slack",
    ));
    env
}

fn account_metadata_proposal() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let p = MetadataProposal {
        proposal_id: "proposal-acct-1".to_string(),
        subject_ref: SubjectRef::Account("acct-zero".to_string()),
        field_path: "account.profile.tier".to_string(),
        current_value: Some("bronze".to_string()),
        proposed_value: "gold".to_string(),
        trust_band: TrustBand::UseWithCaution,
        sensitivity: ClaimSensitivity::Internal,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::MetadataProposals, present_section_state(1));
    env.metadata_proposals = Paginated::stable(vec![p]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Inbound contract email (synthetic)",
        "email_thread",
    ));
    env
}

fn account_open_loop() -> EntityIntelligenceEnvelope {
    use abilities_runtime::abilities::list_open_loops::{OpenLoop, OpenLoopSubject};
    let mut env = base_envelope(account_subject("acct-zero"));
    let ol = OpenLoopWithReceipt {
        open_loop: OpenLoop {
            id: "open-loop-acct-1".to_string(),
            subject: OpenLoopSubject {
                entity_type: "account".to_string(),
                entity_id: "acct-zero".to_string(),
            },
            loop_kind: "follow_up".to_string(),
            description: "Follow up on synthetic renewal conversation".to_string(),
            owner: Some("user:owner".to_string()),
            due_date: Some("2026-05-30".to_string()),
            status: Some("open".to_string()),
            source_asof: Some(now_anchor().to_rfc3339()),
            claim_type: "open_loop.follow_up".to_string(),
        },
        receipt_target: ReceiptTargetRef {
            claim_id: "claim-acct-receipt-1".to_string(),
            subject_ref: SubjectRef::Account("acct-zero".to_string()),
            field_path: Some("body".to_string()),
        },
        trust_band: TrustBand::LikelyCurrent,
        freshness: Freshness::Current,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::OpenLoops, present_section_state(1));
    env.open_loops = Paginated::stable(vec![ol]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic email thread",
        "email_thread",
    ));
    env
}

fn account_upcoming_touchpoint() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let bundle = touchpoint_bundle(
        SubjectRef::Account("acct-zero".to_string()),
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Account("acct-zero".to_string()),
            3,
            TouchpointKind::Meeting,
        )],
        Vec::new(),
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn account_recent_touchpoint() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let bundle = touchpoint_bundle(
        SubjectRef::Account("acct-zero".to_string()),
        Vec::new(),
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Account("acct-zero".to_string()),
            -5,
            TouchpointKind::EmailThread,
        )],
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn account_thread_summary() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let t = ThreadSummary {
        thread_id: "thread-acct-1".to_string(),
        title: Some("Synthetic email thread about renewals".to_string()),
        last_activity_at: Some(now_anchor() - Duration::days(1)),
        message_count: 4,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::Threads, present_section_state(1));
    env.threads = Paginated::stable(vec![t]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic email thread",
        "email_thread",
    ));
    env
}

fn account_glean_citation() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let f = fact(
        "claim-acct-glean-1",
        SubjectRef::Account("acct-zero".to_string()),
        "account.docs.snippet",
        "Synthetic doc snippet sourced via Glean",
        TrustBand::LikelyCurrent,
        Freshness::Current,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance.sources.push(glean_source("src-glean-1"));
    env
}

fn account_confidential_user_only_claim() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let confidential = EntityFact {
        rendered_text: rendered_redacted("claim-acct-conf-1", ClaimSensitivity::Confidential),
        sensitivity: ClaimSensitivity::Confidential,
        ..fact(
            "claim-acct-conf-1",
            SubjectRef::Account("acct-zero".to_string()),
            "account.profile.internal_note",
            "[redacted body]",
            TrustBand::LikelyCurrent,
            Freshness::Current,
            ClaimSensitivity::Confidential,
            ClaimState::Active,
        )
    };
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![confidential]);
    env.sensitivity = ClaimSensitivity::Confidential;
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic internal note",
        "internal_note",
    ));
    env
}

fn account_wrong_subject() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    // Foreign-subject claim — subject_ref points at a DIFFERENT account.
    let foreign = fact(
        "claim-acct-wrong-1",
        SubjectRef::Account("acct-other".to_string()),
        "account.profile.note",
        "Claim about acct-other surfaced under acct-zero — should be retracted",
        TrustBand::NeedsVerification,
        Freshness::Aging,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![foreign]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic miswired source",
        "email_thread",
    ));
    env
}

fn account_project_account_overlap() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let bundle = touchpoint_bundle(
        SubjectRef::Account("acct-zero".to_string()),
        vec![SubjectRef::Project("project-zero".to_string())],
        vec![make_touchpoint(
            SubjectRef::Account("acct-zero".to_string()),
            2,
            TouchpointKind::Meeting,
        )],
        Vec::new(),
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn account_parent_child() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let bundle = touchpoint_bundle(
        SubjectRef::Account("acct-zero".to_string()),
        vec![SubjectRef::Account("acct-parent".to_string())],
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Account("acct-zero".to_string()),
            -7,
            TouchpointKind::Meeting,
        )],
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn account_claim_retracted_mid_render() -> EntityIntelligenceEnvelope {
    // Envelope DOES NOT carry claim-id `account-claim-retracted-1` — that
    // claim was retracted between fetch and paint. The cached DOM still
    // references it. Per AC-461.6b the harness MUST distinguish this from
    // a bypass.
    let mut env = base_envelope(account_subject("acct-zero"));
    let f = fact(
        "claim-acct-current-1",
        SubjectRef::Account("acct-zero".to_string()),
        "account.profile.note",
        "Current note — replaces the retracted one",
        TrustBand::LikelyCurrent,
        Freshness::Current,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic email thread",
        "email_thread",
    ));
    env
}

// ---- project fixtures ------------------------------------------------------

fn project_envelope_base() -> EntityIntelligenceEnvelope {
    base_envelope(project_subject("project-zero"))
}

fn project_stale_fact() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let f = fact(
        "claim-proj-stale-1",
        SubjectRef::Project("project-zero".to_string()),
        "project.trajectory.note",
        "Trajectory last updated 6 months ago — stale",
        TrustBand::UseWithCaution,
        Freshness::Stale,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic project note",
        "internal_note",
    ));
    env
}

fn project_corrected_superseded() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let recorded = RecordEntry {
        claim_id: "claim-proj-record-1".to_string(),
        subject_ref: SubjectRef::Project("project-zero".to_string()),
        claim_type: "project.scope".to_string(),
        recorded_at: now_anchor() - Duration::days(30),
        rendered_text: rendered(
            "claim-proj-record-1",
            "Scope: deliver primitive blocks (superseded)",
            ClaimSensitivity::Internal,
        ),
        trust_band: TrustBand::UseWithCaution,
        sensitivity: ClaimSensitivity::Internal,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::Record, present_section_state(1));
    env.record_entries = Paginated::stable(vec![recorded]);
    env.provenance
        .sources
        .push(generic_source("src-1", "Synthetic Linear update", "linear"));
    env
}

fn project_low_trust() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let f = fact(
        "claim-proj-low-1",
        SubjectRef::Project("project-zero".to_string()),
        "project.horizon.estimate",
        "Approx. ship Q3 — informal estimate",
        TrustBand::NeedsVerification,
        Freshness::Aging,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance
        .sources
        .push(generic_source("src-1", "Synthetic Slack message", "slack"));
    env
}

fn project_metadata_proposal() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let p = MetadataProposal {
        proposal_id: "proposal-proj-1".to_string(),
        subject_ref: SubjectRef::Project("project-zero".to_string()),
        field_path: "project.metadata.lead".to_string(),
        current_value: Some("person-prior".to_string()),
        proposed_value: "person-current".to_string(),
        trust_band: TrustBand::UseWithCaution,
        sensitivity: ClaimSensitivity::Internal,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::MetadataProposals, present_section_state(1));
    env.metadata_proposals = Paginated::stable(vec![p]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic project email",
        "email_thread",
    ));
    env
}

fn project_open_loop() -> EntityIntelligenceEnvelope {
    use abilities_runtime::abilities::list_open_loops::{OpenLoop, OpenLoopSubject};
    let mut env = project_envelope_base();
    let ol = OpenLoopWithReceipt {
        open_loop: OpenLoop {
            id: "open-loop-proj-1".to_string(),
            subject: OpenLoopSubject {
                entity_type: "project".to_string(),
                entity_id: "project-zero".to_string(),
            },
            loop_kind: "work_item".to_string(),
            description: "Synthetic work item — finalize block JSON".to_string(),
            owner: Some("user:project-lead".to_string()),
            due_date: Some("2026-06-10".to_string()),
            status: Some("in_progress".to_string()),
            source_asof: Some(now_anchor().to_rfc3339()),
            claim_type: "open_loop.work_item".to_string(),
        },
        receipt_target: ReceiptTargetRef {
            claim_id: "claim-proj-receipt-1".to_string(),
            subject_ref: SubjectRef::Project("project-zero".to_string()),
            field_path: Some("status".to_string()),
        },
        trust_band: TrustBand::LikelyCurrent,
        freshness: Freshness::Current,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::OpenLoops, present_section_state(1));
    env.open_loops = Paginated::stable(vec![ol]);
    env.provenance
        .sources
        .push(generic_source("src-1", "Synthetic Linear issue", "linear"));
    env
}

fn project_upcoming_touchpoint() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let bundle = touchpoint_bundle(
        SubjectRef::Project("project-zero".to_string()),
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Project("project-zero".to_string()),
            4,
            TouchpointKind::Meeting,
        )],
        Vec::new(),
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn project_recent_touchpoint() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let bundle = touchpoint_bundle(
        SubjectRef::Project("project-zero".to_string()),
        Vec::new(),
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Project("project-zero".to_string()),
            -4,
            TouchpointKind::Document,
        )],
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn project_thread_summary() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let t = ThreadSummary {
        thread_id: "thread-proj-1".to_string(),
        title: Some("Synthetic project review thread".to_string()),
        last_activity_at: Some(now_anchor() - Duration::days(2)),
        message_count: 7,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::Threads, present_section_state(1));
    env.threads = Paginated::stable(vec![t]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic project thread",
        "email_thread",
    ));
    env
}

fn project_glean_citation() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let f = fact(
        "claim-proj-glean-1",
        SubjectRef::Project("project-zero".to_string()),
        "project.docs.snippet",
        "Synthetic project doc snippet via Glean",
        TrustBand::LikelyCurrent,
        Freshness::Current,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance
        .sources
        .push(glean_source("src-glean-proj-1"));
    env
}

fn project_confidential_user_only_claim() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let f = EntityFact {
        rendered_text: rendered_redacted("claim-proj-conf-1", ClaimSensitivity::UserOnly),
        sensitivity: ClaimSensitivity::UserOnly,
        ..fact(
            "claim-proj-conf-1",
            SubjectRef::Project("project-zero".to_string()),
            "project.private.note",
            "[redacted body]",
            TrustBand::LikelyCurrent,
            Freshness::Current,
            ClaimSensitivity::UserOnly,
            ClaimState::Active,
        )
    };
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.sensitivity = ClaimSensitivity::UserOnly;
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic private note",
        "internal_note",
    ));
    env
}

fn project_wrong_subject() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let foreign = fact(
        "claim-proj-wrong-1",
        SubjectRef::Project("project-other".to_string()),
        "project.scope.note",
        "Claim about project-other surfaced under project-zero",
        TrustBand::NeedsVerification,
        Freshness::Aging,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![foreign]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic miswired source",
        "email_thread",
    ));
    env
}

fn project_project_account_overlap() -> EntityIntelligenceEnvelope {
    let mut env = project_envelope_base();
    let bundle = touchpoint_bundle(
        SubjectRef::Project("project-zero".to_string()),
        vec![SubjectRef::Account("acct-zero".to_string())],
        vec![make_touchpoint(
            SubjectRef::Project("project-zero".to_string()),
            1,
            TouchpointKind::Meeting,
        )],
        Vec::new(),
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

// ---- person fixtures -------------------------------------------------------

fn person_envelope_base() -> EntityIntelligenceEnvelope {
    base_envelope(person_subject("person-zero"))
}

fn person_stale_fact() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let f = fact(
        "claim-person-stale-1",
        SubjectRef::Person("person-zero".to_string()),
        "person.profile.role",
        "Last known role from 9 months ago",
        TrustBand::UseWithCaution,
        Freshness::Stale,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance
        .sources
        .push(generic_source("src-1", "Synthetic profile", "profile_note"));
    env
}

fn person_corrected_superseded() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let recorded = RecordEntry {
        claim_id: "claim-person-record-1".to_string(),
        subject_ref: SubjectRef::Person("person-zero".to_string()),
        claim_type: "person.profile.role".to_string(),
        recorded_at: now_anchor() - Duration::days(60),
        rendered_text: rendered(
            "claim-person-record-1",
            "Role: prior-title (superseded)",
            ClaimSensitivity::Internal,
        ),
        trust_band: TrustBand::UseWithCaution,
        sensitivity: ClaimSensitivity::Internal,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::Record, present_section_state(1));
    env.record_entries = Paginated::stable(vec![recorded]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic profile note",
        "profile_note",
    ));
    env
}

fn person_low_trust() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let f = fact(
        "claim-person-low-1",
        SubjectRef::Person("person-zero".to_string()),
        "person.preferences.signal",
        "Mentioned preferring async — informal",
        TrustBand::NeedsVerification,
        Freshness::Aging,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance
        .sources
        .push(generic_source("src-1", "Synthetic Slack message", "slack"));
    env
}

fn person_open_loop() -> EntityIntelligenceEnvelope {
    use abilities_runtime::abilities::list_open_loops::{OpenLoop, OpenLoopSubject};
    let mut env = person_envelope_base();
    let ol = OpenLoopWithReceipt {
        open_loop: OpenLoop {
            id: "open-loop-person-1".to_string(),
            subject: OpenLoopSubject {
                entity_type: "person".to_string(),
                entity_id: "person-zero".to_string(),
            },
            loop_kind: "follow_up".to_string(),
            description: "Synthetic 1:1 follow-up".to_string(),
            owner: Some("user:owner".to_string()),
            due_date: Some("2026-05-25".to_string()),
            status: Some("open".to_string()),
            source_asof: Some(now_anchor().to_rfc3339()),
            claim_type: "open_loop.follow_up".to_string(),
        },
        receipt_target: ReceiptTargetRef {
            claim_id: "claim-person-receipt-1".to_string(),
            subject_ref: SubjectRef::Person("person-zero".to_string()),
            field_path: Some("body".to_string()),
        },
        trust_band: TrustBand::LikelyCurrent,
        freshness: Freshness::Current,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::OpenLoops, present_section_state(1));
    env.open_loops = Paginated::stable(vec![ol]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic 1:1 notes",
        "internal_note",
    ));
    env
}

fn person_upcoming_touchpoint() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let bundle = touchpoint_bundle(
        SubjectRef::Person("person-zero".to_string()),
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Person("person-zero".to_string()),
            1,
            TouchpointKind::Meeting,
        )],
        Vec::new(),
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn person_recent_touchpoint() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let bundle = touchpoint_bundle(
        SubjectRef::Person("person-zero".to_string()),
        Vec::new(),
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Person("person-zero".to_string()),
            -3,
            TouchpointKind::EmailThread,
        )],
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

fn person_thread_summary() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let t = ThreadSummary {
        thread_id: "thread-person-1".to_string(),
        title: Some("Synthetic 1:1 email thread".to_string()),
        last_activity_at: Some(now_anchor() - Duration::hours(20)),
        message_count: 3,
        provenance: ProvenanceRef::from_ids(["src-1".to_string()]),
    };
    env.sections
        .insert(EnvelopeSection::Threads, present_section_state(1));
    env.threads = Paginated::stable(vec![t]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic email thread",
        "email_thread",
    ));
    env
}

fn person_glean_citation() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let f = fact(
        "claim-person-glean-1",
        SubjectRef::Person("person-zero".to_string()),
        "person.docs.snippet",
        "Synthetic person doc snippet via Glean",
        TrustBand::LikelyCurrent,
        Freshness::Current,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance
        .sources
        .push(glean_source("src-glean-person-1"));
    env
}

fn person_confidential_user_only_claim() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let f = EntityFact {
        rendered_text: rendered_redacted("claim-person-conf-1", ClaimSensitivity::UserOnly),
        sensitivity: ClaimSensitivity::UserOnly,
        ..fact(
            "claim-person-conf-1",
            SubjectRef::Person("person-zero".to_string()),
            "person.private.note",
            "[redacted body]",
            TrustBand::LikelyCurrent,
            Freshness::Current,
            ClaimSensitivity::UserOnly,
            ClaimState::Active,
        )
    };
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.sensitivity = ClaimSensitivity::UserOnly;
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic user-only note",
        "internal_note",
    ));
    env
}

fn person_wrong_subject() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    let foreign = fact(
        "claim-person-wrong-1",
        SubjectRef::Person("person-other".to_string()),
        "person.profile.role",
        "Claim about person-other surfaced under person-zero",
        TrustBand::NeedsVerification,
        Freshness::Aging,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![foreign]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic miswired source",
        "email_thread",
    ));
    env
}

fn person_ambiguous_association() -> EntityIntelligenceEnvelope {
    let mut env = person_envelope_base();
    // Ambiguous: same email may map to multiple Person subjects. Encode via
    // subject_scope.also_includes carrying the alternate person.
    let bundle = touchpoint_bundle(
        SubjectRef::Person("person-zero".to_string()),
        vec![SubjectRef::Person("person-zero-alt".to_string())],
        Vec::new(),
        vec![make_touchpoint(
            SubjectRef::Person("person-zero".to_string()),
            -1,
            TouchpointKind::EmailThread,
        )],
    );
    env.sections
        .insert(EnvelopeSection::Touchpoints, present_section_state(1));
    env.touchpoints = Paginated::stable(vec![bundle]);
    env
}

// ---- canonical "good" envelope used by red-first proofs --------------------

fn good_envelope_canonical() -> EntityIntelligenceEnvelope {
    let mut env = base_envelope(account_subject("acct-zero"));
    let f = fact(
        "claim-acct-zero-canonical-1",
        SubjectRef::Account("acct-zero".to_string()),
        "account.profile.note",
        "account-zero is the customer-zero workspace",
        TrustBand::LikelyCurrent,
        Freshness::Current,
        ClaimSensitivity::Internal,
        ClaimState::Active,
    );
    env.sections
        .insert(EnvelopeSection::Facts, present_section_state(1));
    env.facts = Paginated::stable(vec![f]);
    env.provenance.sources.push(generic_source(
        "src-1",
        "Synthetic canonical source",
        "email_thread",
    ));
    env
}

// ---- generation entry point -----------------------------------------------

/// Returns the full set of `(filename, envelope)` pairs the fixture matrix
/// requires.
pub fn all_fixtures() -> Vec<(&'static str, EntityIntelligenceEnvelope)> {
    vec![
        // Account
        (
            "account_metadata_proposal.json",
            account_metadata_proposal(),
        ),
        (
            "account_upcoming_touchpoint.json",
            account_upcoming_touchpoint(),
        ),
        (
            "account_recent_touchpoint.json",
            account_recent_touchpoint(),
        ),
        ("account_thread_summary.json", account_thread_summary()),
        ("account_glean_citation.json", account_glean_citation()),
        ("account_wrong_subject.json", account_wrong_subject()),
        (
            "account_project_account_overlap.json",
            account_project_account_overlap(),
        ),
        ("account_parent_child.json", account_parent_child()),
        ("account_stale_fact.json", account_stale_fact()),
        (
            "account_corrected_superseded.json",
            account_corrected_superseded(),
        ),
        ("account_low_trust.json", account_low_trust()),
        ("account_open_loop.json", account_open_loop()),
        (
            "account_confidential_user_only_claim.json",
            account_confidential_user_only_claim(),
        ),
        (
            "account_claim_retracted_mid_render.json",
            account_claim_retracted_mid_render(),
        ),
        // Project
        (
            "project_metadata_proposal.json",
            project_metadata_proposal(),
        ),
        (
            "project_upcoming_touchpoint.json",
            project_upcoming_touchpoint(),
        ),
        (
            "project_recent_touchpoint.json",
            project_recent_touchpoint(),
        ),
        ("project_thread_summary.json", project_thread_summary()),
        ("project_glean_citation.json", project_glean_citation()),
        ("project_wrong_subject.json", project_wrong_subject()),
        (
            "project_project_account_overlap.json",
            project_project_account_overlap(),
        ),
        ("project_stale_fact.json", project_stale_fact()),
        (
            "project_corrected_superseded.json",
            project_corrected_superseded(),
        ),
        ("project_low_trust.json", project_low_trust()),
        ("project_open_loop.json", project_open_loop()),
        (
            "project_confidential_user_only_claim.json",
            project_confidential_user_only_claim(),
        ),
        // Person
        (
            "person_upcoming_touchpoint.json",
            person_upcoming_touchpoint(),
        ),
        ("person_recent_touchpoint.json", person_recent_touchpoint()),
        ("person_thread_summary.json", person_thread_summary()),
        ("person_glean_citation.json", person_glean_citation()),
        ("person_wrong_subject.json", person_wrong_subject()),
        (
            "person_ambiguous_association.json",
            person_ambiguous_association(),
        ),
        ("person_stale_fact.json", person_stale_fact()),
        (
            "person_corrected_superseded.json",
            person_corrected_superseded(),
        ),
        ("person_low_trust.json", person_low_trust()),
        ("person_open_loop.json", person_open_loop()),
        (
            "person_confidential_user_only_claim.json",
            person_confidential_user_only_claim(),
        ),
        // Red-first canonical good envelope
        ("__good_envelope_canonical.json", good_envelope_canonical()),
    ]
}

/// Re-emit every fixture file to disk. Gated on `DOS461_REGEN=1` env var so
/// `cargo test` doesn't mutate the working tree.
#[test]
fn regenerate_fixtures_on_demand() {
    if std::env::var("DOS461_REGEN").ok().as_deref() != Some("1") {
        return;
    }
    let dir = fixtures_dir();
    std::fs::create_dir_all(&dir).expect("create fixtures dir");
    for (name, env) in all_fixtures() {
        let path = dir.join(name);
        let json = serde_json::to_string_pretty(&env).expect("serialize envelope");
        std::fs::write(&path, json).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }
}
