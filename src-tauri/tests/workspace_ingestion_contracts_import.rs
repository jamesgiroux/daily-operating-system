//! v1.4.5 W1-A — shared-types import gate.
//!
//! External integration test that asserts every type and trait in
//! `services::workspace_ingestion::contracts` and `services::workspace_ingestion::lifecycle`
//! is `pub` and importable from outside the crate via the canonical
//! `dailyos_lib::services::workspace_ingestion::*` path. Downstream lanes
//! (W1-B/W1-C/W2-A/W3-A/W3-B/W3-C) and out-of-crate tests all depend on this
//! surface staying importable; the gate prevents accidental `pub(crate)`
//! demotion.

use dailyos_lib::services::workspace_ingestion::contracts::{
    Extractor, FileIdentity, NullExtractor, NullSignalEmitter, RejectionReason, SignalEmitter,
    SourceAttribution, WorkspaceCategory, WorkspaceClaimProposal, WorkspaceFileKind,
};
use dailyos_lib::services::workspace_ingestion::lifecycle::{
    LifecycleError, LifecycleState, UserOverride, WorkspaceFileLifecycle, escalate_to_pending,
};

#[test]
fn contracts_public_surface_importable_externally() {
    // The act of compiling this test crate proves importability. Bind every
    // import to a named binding so clippy's `let_underscore_must_use` lint
    // stays happy.
    let category: WorkspaceCategory = WorkspaceCategory::Presentations;
    assert!(matches!(category, WorkspaceCategory::Presentations));

    let rejection: RejectionReason = RejectionReason::PathTraversalAttempt;
    assert!(matches!(rejection, RejectionReason::PathTraversalAttempt));

    let kind: WorkspaceFileKind = WorkspaceFileKind::Inbox;
    assert!(matches!(kind, WorkspaceFileKind::Inbox));

    let extractor: Box<dyn Extractor> = Box::new(NullExtractor);
    let emitter: Box<dyn SignalEmitter> = Box::new(NullSignalEmitter);
    drop(extractor);
    drop(emitter);

    // Field-access-driven retention so cargo doesn't tree-shake the imports.
    assert!(std::mem::size_of::<FileIdentity>() > 0);
    assert!(std::mem::size_of::<WorkspaceClaimProposal>() > 0);
    assert!(std::mem::size_of::<SourceAttribution>() > 0);
}

#[test]
fn lifecycle_public_surface_importable_externally() {
    let state: LifecycleState = LifecycleState::Pending;
    assert!(matches!(state, LifecycleState::Pending));

    assert!(std::mem::size_of::<WorkspaceFileLifecycle>() > 0);
    assert!(std::mem::size_of::<UserOverride>() > 0);

    // escalate_to_pending is W1-A stub; just prove it's callable from external.
    let err = escalate_to_pending("file-stub", "actor-stub").expect_err("W1-A stub returns Err");
    assert!(matches!(err, LifecycleError::DbError(_)));
}
