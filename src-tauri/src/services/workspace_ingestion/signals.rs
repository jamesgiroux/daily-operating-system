//! Workspace signal emission.
//!
//! This layer is deliberately thin: workspace ingestion owns the privacy-safe
//! payload shape, while actual signal persistence and propagation go through
//! `services::signals`.

use serde::{Deserialize, Serialize};

use crate::services;

use super::contracts::{RejectionReason, SignalEmitContext, SignalEmitError, SignalEmitter};

pub const WORKSPACE_FILE_INGESTED: &str = "workspace_file_ingested";
pub const WORKSPACE_FILE_REJECTED: &str = "workspace_file_rejected";
pub const WORKSPACE_FILE_PENDING_ENTITY_ASSIGNMENT: &str =
    "workspace_file_pending_entity_assignment";
pub const WORKSPACE_FILE_QUARANTINED: &str = "workspace_file_quarantined";
pub const WORKSPACE_FILE_ENTITY_LINK_CHANGED: &str = "workspace_file_entity_link_changed";
pub const WORKSPACE_SOURCE_POLICY_CHANGED: &str = "workspace_source_policy_changed";
pub const ENTITY_INTELLIGENCE_UPDATED: &str = "entity_intelligence_updated";

const WORKSPACE_SIGNAL_SOURCE: &str = "workspace_ingestion";
const WORKSPACE_FILE_ENTITY_TYPE: &str = "workspace_file";
const WORKSPACE_SYSTEM_ENTITY_TYPE: &str = "workspace_ingestion";
const WORKSPACE_SYSTEM_ENTITY_ID: &str = "workspace_ingestion";
const WORKSPACE_SOURCE_ENTITY_TYPE: &str = "workspace_source";
const WORKSPACE_SIGNAL_CONFIDENCE: f64 = 1.0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileIngestedSignal {
    pub file_id: String,
    pub ingestion_run_id: String,
    pub entity_type: String,
    pub entity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEntityIntelligenceUpdatedSignal {
    pub file_id: String,
    pub ingestion_run_id: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileRejectedSignal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFilePendingEntityAssignmentSignal {
    pub file_id: String,
    pub ingestion_run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileQuarantinedSignal {
    pub file_id: String,
    pub reason_code: String,
    pub actor_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileEntityLinkChangedSignal {
    pub file_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub actor_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSourcePolicyChangedSignal {
    pub source_handle: String,
    pub policy_action_code: String,
    pub reason_code: String,
    pub actor_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_receipt_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceSourcePolicyChangedInput<'a> {
    pub source_handle: &'a str,
    pub policy_action: &'a str,
    pub reason: &'a str,
    pub actor: &'a str,
    pub entity_type: Option<&'a str>,
    pub entity_id: Option<&'a str>,
    pub policy_receipt_id: Option<&'a str>,
}

#[derive(Debug, Default)]
pub struct WorkspaceSignalEmitter;

impl SignalEmitter for WorkspaceSignalEmitter {
    fn emit_file_ingested(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        ingestion_run_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), SignalEmitError> {
        let payload = WorkspaceFileIngestedSignal {
            file_id: file_id.to_string(),
            ingestion_run_id: ingestion_run_id.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
        };
        emit_invalidating(
            ctx,
            WORKSPACE_FILE_INGESTED,
            entity_type,
            entity_id,
            &payload,
        )?;
        let intelligence_payload = WorkspaceEntityIntelligenceUpdatedSignal {
            file_id: file_id.to_string(),
            ingestion_run_id: ingestion_run_id.to_string(),
            reason_code: WORKSPACE_FILE_INGESTED.to_string(),
        };
        emit_invalidating(
            ctx,
            ENTITY_INTELLIGENCE_UPDATED,
            entity_type,
            entity_id,
            &intelligence_payload,
        )
    }

    fn emit_file_rejected(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: Option<&str>,
        reason: RejectionReason,
    ) -> Result<(), SignalEmitError> {
        let payload = WorkspaceFileRejectedSignal {
            file_id: file_id.map(str::to_string),
            reason_code: rejection_reason_code(&reason).to_string(),
        };
        let entity_type = file_id
            .map(|_| WORKSPACE_FILE_ENTITY_TYPE)
            .unwrap_or(WORKSPACE_SYSTEM_ENTITY_TYPE);
        let entity_id = file_id.unwrap_or(WORKSPACE_SYSTEM_ENTITY_ID);
        emit_local(
            ctx,
            WORKSPACE_FILE_REJECTED,
            entity_type,
            entity_id,
            &payload,
        )
    }

    fn emit_file_pending_entity_assignment(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        ingestion_run_id: &str,
    ) -> Result<(), SignalEmitError> {
        let payload = WorkspaceFilePendingEntityAssignmentSignal {
            file_id: file_id.to_string(),
            ingestion_run_id: ingestion_run_id.to_string(),
        };
        emit_local(
            ctx,
            WORKSPACE_FILE_PENDING_ENTITY_ASSIGNMENT,
            WORKSPACE_FILE_ENTITY_TYPE,
            file_id,
            &payload,
        )
    }

    fn emit_file_quarantined(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        reason: &str,
        actor: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
    ) -> Result<(), SignalEmitError> {
        let payload = WorkspaceFileQuarantinedSignal {
            file_id: file_id.to_string(),
            reason_code: quarantine_reason_code(reason).to_string(),
            actor_kind: actor_kind(actor).to_string(),
            entity_type: entity_type.map(str::to_string),
            entity_id: entity_id.map(str::to_string),
        };
        match (entity_type, entity_id) {
            (Some(entity_type), Some(entity_id)) => emit_invalidating(
                ctx,
                WORKSPACE_FILE_QUARANTINED,
                entity_type,
                entity_id,
                &payload,
            ),
            _ => emit_local(
                ctx,
                WORKSPACE_FILE_QUARANTINED,
                WORKSPACE_FILE_ENTITY_TYPE,
                file_id,
                &payload,
            ),
        }
    }

    fn emit_link_changed(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        entity_type: &str,
        entity_id: &str,
        actor: &str,
    ) -> Result<(), SignalEmitError> {
        let payload = WorkspaceFileEntityLinkChangedSignal {
            file_id: file_id.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            actor_kind: actor_kind(actor).to_string(),
        };
        emit_invalidating(
            ctx,
            WORKSPACE_FILE_ENTITY_LINK_CHANGED,
            entity_type,
            entity_id,
            &payload,
        )
    }
}

pub fn emit_pre_pipeline_rejection(
    ctx: &SignalEmitContext<'_, '_>,
    reason: RejectionReason,
) -> Result<(), SignalEmitError> {
    WorkspaceSignalEmitter.emit_file_rejected(ctx, None, reason)
}

pub fn emit_source_policy_changed(
    ctx: &SignalEmitContext<'_, '_>,
    input: WorkspaceSourcePolicyChangedInput<'_>,
) -> Result<(), SignalEmitError> {
    let payload = WorkspaceSourcePolicyChangedSignal {
        source_handle: input.source_handle.to_string(),
        policy_action_code: source_policy_action_code(input.policy_action).to_string(),
        reason_code: source_policy_reason_code(input.reason).to_string(),
        actor_kind: actor_kind(input.actor).to_string(),
        entity_type: input.entity_type.map(str::to_string),
        entity_id: input.entity_id.map(str::to_string),
        policy_receipt_id: input.policy_receipt_id.map(str::to_string),
    };
    match (input.entity_type, input.entity_id) {
        (Some(entity_type), Some(entity_id)) => emit_invalidating(
            ctx,
            WORKSPACE_SOURCE_POLICY_CHANGED,
            entity_type,
            entity_id,
            &payload,
        ),
        _ => emit_local(
            ctx,
            WORKSPACE_SOURCE_POLICY_CHANGED,
            WORKSPACE_SOURCE_ENTITY_TYPE,
            input.source_handle,
            &payload,
        ),
    }
}

fn emit_local<T: Serialize>(
    ctx: &SignalEmitContext<'_, '_>,
    signal_type: &'static str,
    entity_type: &str,
    entity_id: &str,
    payload: &T,
) -> Result<(), SignalEmitError> {
    let value = serialize_payload(signal_type, payload)?;
    services::signals::emit(
        ctx.services,
        ctx.db,
        entity_type,
        entity_id,
        signal_type,
        WORKSPACE_SIGNAL_SOURCE,
        Some(value.as_str()),
        WORKSPACE_SIGNAL_CONFIDENCE,
    )
    .map(|_| ())
    .map_err(|message| SignalEmitError::Emit {
        signal_type,
        message,
    })
}

fn emit_invalidating<T: Serialize>(
    ctx: &SignalEmitContext<'_, '_>,
    signal_type: &'static str,
    entity_type: &str,
    entity_id: &str,
    payload: &T,
) -> Result<(), SignalEmitError> {
    if entity_type.trim().is_empty() || entity_id.trim().is_empty() {
        return Err(SignalEmitError::MissingEntityTarget(signal_type));
    }
    let engine = ctx
        .propagation
        .ok_or(SignalEmitError::MissingPropagationEngine(signal_type))?;
    let value = serialize_payload(signal_type, payload)?;
    services::signals::emit_and_propagate(
        ctx.services,
        ctx.db,
        engine,
        entity_type,
        entity_id,
        signal_type,
        WORKSPACE_SIGNAL_SOURCE,
        Some(value.as_str()),
        WORKSPACE_SIGNAL_CONFIDENCE,
    )
    .map(|_| ())
    .map_err(|message| SignalEmitError::Emit {
        signal_type,
        message,
    })
}

fn serialize_payload<T: Serialize>(
    signal_type: &'static str,
    payload: &T,
) -> Result<String, SignalEmitError> {
    serde_json::to_string(payload).map_err(|error| SignalEmitError::Serialize {
        signal_type,
        message: error.to_string(),
    })
}

pub fn rejection_reason_code(reason: &RejectionReason) -> &'static str {
    match reason {
        RejectionReason::PathTraversalAttempt => "path_traversal_attempt",
        RejectionReason::SymlinkRefused => "symlink_refused",
        RejectionReason::SymlinkRaced => "symlink_raced",
        RejectionReason::OutsideWorkspace => "outside_workspace",
        RejectionReason::FileTooLarge => "file_too_large",
        RejectionReason::UnsupportedFormat => "unsupported_format",
    }
}

pub fn quarantine_reason_code(reason: &str) -> &'static str {
    match reason.trim() {
        "" => "unspecified",
        "user" | "user_requested" | "user_quarantined" => "user_requested",
        "policy" | "policy_violation" => "policy_violation",
        "malware" | "unsafe_content" => "unsafe_content",
        "source_revoked" | "source_withdrawn" => "source_withdrawn",
        _ => "manual_quarantine",
    }
}

pub fn source_policy_action_code(action: &str) -> &'static str {
    match action.trim() {
        "allow" | "allowed" | "enable" | "enabled" | "restore" | "restored" => "enabled",
        "deny" | "denied" | "disable" | "disabled" | "quarantine" | "quarantined" => "disabled",
        "ignore" | "ignored" => "ignored",
        "scratchpad" => "scratchpad",
        "archive" | "archived" => "archived",
        "delete" | "deleted" => "deleted",
        "category" | "category_changed" | "recategorized" => "category_changed",
        _ => "updated",
    }
}

pub fn source_policy_reason_code(reason: &str) -> &'static str {
    match reason.trim() {
        "" => "unspecified",
        "user" | "user_requested" => "user_requested",
        "policy" | "policy_update" => "policy_update",
        "unsafe_content" | "malware" => "unsafe_content",
        "source_withdrawn" | "source_revoked" => "source_withdrawn",
        "category" | "category_changed" => "category_changed",
        _ => "manual_update",
    }
}

pub fn actor_kind(actor: &str) -> &'static str {
    match actor.split(':').next().unwrap_or(actor).trim() {
        "system" => "system",
        "agent" => "agent",
        "admin" => "admin",
        "surface_client" => "surface_client",
        "mcp_client" => "mcp_client",
        _ => "user",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};

    fn with_signal_context<T>(
        propagation: Option<&crate::signals::propagation::PropagationEngine>,
        f: impl FnOnce(&SignalEmitContext<'_, '_>) -> T,
    ) -> T {
        let db = test_db();
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external);
        let signal_ctx = SignalEmitContext::new(&services, &db, propagation);
        f(&signal_ctx)
    }

    #[test]
    fn payload_serialization_excludes_freeform_quarantine_reason_and_actor() {
        let payload = WorkspaceFileQuarantinedSignal {
            file_id: "wf-1".to_string(),
            reason_code: quarantine_reason_code("contains/private/path.txt").to_string(),
            actor_kind: actor_kind("user-12345").to_string(),
            entity_type: Some("account".to_string()),
            entity_id: Some("a1".to_string()),
        };

        let value = serialize_payload(WORKSPACE_FILE_QUARANTINED, &payload).expect("serialize");
        assert!(value.contains("\"reason_code\":\"manual_quarantine\""));
        assert!(value.contains("\"actor_kind\":\"user\""));
        assert!(!value.contains("private/path"));
        assert!(!value.contains("user-12345"));
    }

    #[test]
    fn rejection_reasons_are_controlled_codes() {
        assert_eq!(
            rejection_reason_code(&RejectionReason::PathTraversalAttempt),
            "path_traversal_attempt"
        );
        assert_eq!(
            rejection_reason_code(&RejectionReason::UnsupportedFormat),
            "unsupported_format"
        );
    }

    #[test]
    fn source_policy_payload_excludes_freeform_action_reason_and_actor() {
        let payload = WorkspaceSourcePolicyChangedSignal {
            source_handle: "source_123".to_string(),
            policy_action_code: source_policy_action_code("disable/private/path.txt").to_string(),
            reason_code: source_policy_reason_code("because user@example.com said so").to_string(),
            actor_kind: actor_kind("surface_client:wp-site-1").to_string(),
            entity_type: Some("account".to_string()),
            entity_id: Some("a1".to_string()),
            policy_receipt_id: Some("policy_receipt_1".to_string()),
        };

        let value =
            serialize_payload(WORKSPACE_SOURCE_POLICY_CHANGED, &payload).expect("serialize");
        assert!(value.contains("\"policy_action_code\":\"updated\""));
        assert!(value.contains("\"reason_code\":\"manual_update\""));
        assert!(value.contains("\"actor_kind\":\"surface_client\""));
        assert!(!value.contains("private/path"));
        assert!(!value.contains("user@example.com"));
        assert!(!value.contains("wp-site-1"));
    }

    #[test]
    fn invalidating_workspace_signals_require_propagation_engine() {
        with_signal_context(None, |ctx| {
            let emitter = WorkspaceSignalEmitter;
            let err = emitter
                .emit_file_ingested(ctx, "wf-1", "run-1", "account", "a1")
                .expect_err("invalidating signal requires propagation engine");
            assert!(matches!(
                err,
                SignalEmitError::MissingPropagationEngine("workspace_file_ingested")
            ));
        });
    }

    #[test]
    fn rejected_signal_persists_privacy_safe_audit_payload() {
        let db = test_db();
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external);
        let signal_ctx = SignalEmitContext::new(&services, &db, None);
        let emitter = WorkspaceSignalEmitter;

        emitter
            .emit_file_rejected(
                &signal_ctx,
                Some("wf-1"),
                RejectionReason::PathTraversalAttempt,
            )
            .expect("emit rejected");

        let events = services::signals::get_by_type(
            &db,
            WORKSPACE_FILE_ENTITY_TYPE,
            "wf-1",
            WORKSPACE_FILE_REJECTED,
        )
        .expect("load emitted signal");
        assert_eq!(events.len(), 1);
        let value = events[0].value.as_deref().expect("payload");
        assert!(value.contains("\"file_id\":\"wf-1\""));
        assert!(value.contains("\"reason_code\":\"path_traversal_attempt\""));
        assert!(!value.contains('/'));
    }

    #[test]
    fn pre_pipeline_rejection_signal_uses_system_scope_without_file_id() {
        let db = test_db();
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external);
        let signal_ctx = SignalEmitContext::new(&services, &db, None);

        emit_pre_pipeline_rejection(&signal_ctx, RejectionReason::SymlinkRaced)
            .expect("emit pre-pipeline rejected");

        let events = services::signals::get_by_type(
            &db,
            WORKSPACE_SYSTEM_ENTITY_TYPE,
            WORKSPACE_SYSTEM_ENTITY_ID,
            WORKSPACE_FILE_REJECTED,
        )
        .expect("load emitted signal");
        assert_eq!(events.len(), 1);
        let value = events[0].value.as_deref().expect("payload");
        assert!(value.contains("\"reason_code\":\"symlink_raced\""));
        assert!(!value.contains("\"file_id\""));
    }
}
