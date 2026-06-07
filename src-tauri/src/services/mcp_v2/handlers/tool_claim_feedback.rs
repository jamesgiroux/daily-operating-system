//! `dailyos.submit.claim_feedback` MCP handler.

use abilities_runtime::abilities::feedback::FeedbackAction;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::services::claims::{
    ClaimFeedbackInput, ClaimFeedbackReplayInput, McpFeedbackDelegation,
};
use crate::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::mcp_v2::handler_context::McpHandlerContext;
use crate::services::mcp_v2::target_handles::{
    mint_target_handle, public_handle_hash, resolve_target_handle,
    resolved_target_watermark_matches, MintTargetHandle, ResolveTargetHandle,
    TargetHandleResolutionError, TargetKind,
};
use crate::services::sensitivity::{renderable_claim_text, RenderActor, RenderSurface};

use super::tool_utils::{
    actor_origin_hashes, bad_params, internal_trace, mutation_cursor_for_target,
    reject_raw_id_params, required_str, source_provenance_watermark, unavailable_payload,
    validate_bounded_string,
};

const SCHEMA_VERSION: &str = "mcp.claim_feedback.v1";

pub struct ClaimFeedbackHandler {
    description: ToolDescription,
}

impl ClaimFeedbackHandler {
    pub fn new(description: ToolDescription) -> Self {
        Self { description }
    }
}

impl McpToolHandler for ClaimFeedbackHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(
        &self,
        ctx: &McpHandlerContext,
        actor: &McpActor,
        params: Value,
    ) -> Result<Value, ToolError> {
        reject_raw_id_params(&params)?;
        let feedback_target_handle = required_str(&params, "feedback_target_handle")?;
        let action = parse_feedback_action(&required_str(&params, "action")?)?;
        let metadata = sanitize_mcp_feedback_metadata(action, params.get("metadata"))?;
        let (client_id_hash, conversation_handle_hash, tool_name) = actor_origin_hashes(actor)?;

        let result = ctx.with_conn(|db| {
            let resolved = match resolve_target_handle(
                db,
                ResolveTargetHandle {
                    actor,
                    handle: &feedback_target_handle,
                    expected_kind: TargetKind::Claim,
                    current_watermark_material: None,
                },
            ) {
                Ok(resolved) => resolved,
                Err(TargetHandleResolutionError::Unavailable { refresh_required }) => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        refresh_required,
                        Some(&feedback_target_handle),
                    ));
                }
                Err(TargetHandleResolutionError::Internal(detail)) => {
                    return Err(internal_trace("mcp_feedback_handle_resolve", detail));
                }
            };
            let claim_id = resolved
                .target_ref
                .get("claim_id")
                .and_then(Value::as_str)
                .ok_or_else(|| bad_params("feedback target handle is unavailable"))?
                .to_string();
            let claim = match crate::services::claims::load_claim_by_id(db.conn_ref(), &claim_id)
                .map_err(|error| internal_trace("mcp_feedback_claim_load", error))?
            {
                Some(claim) => claim,
                None => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        true,
                        Some(&feedback_target_handle),
                    ));
                }
            };
            let current_watermark = claim_watermark_for_handle(&claim);
            if !resolved_target_watermark_matches(&resolved, &current_watermark)
                .map_err(|error| internal_trace("mcp_feedback_handle_watermark", error))?
            {
                return Ok(unavailable_payload(
                    SCHEMA_VERSION,
                    &self.description.name,
                    true,
                    Some(&feedback_target_handle),
                ));
            }
            if claim.claim_state != crate::db::claims::ClaimState::Active
                || claim.surfacing_state != crate::db::claims::SurfacingState::Active
                || renderable_claim_text(
                    &claim,
                    RenderSurface::McpTool,
                    &RenderActor::agent("agent:mcp"),
                )
                .is_none()
            {
                return Ok(unavailable_payload(
                    SCHEMA_VERSION,
                    &self.description.name,
                    true,
                    Some(&feedback_target_handle),
                ));
            }
            let service_metadata =
                match materialize_mcp_feedback_metadata(db, actor, action, &metadata)? {
                    MetadataMaterialization::Available(metadata) => metadata,
                    MetadataMaterialization::Unavailable { refresh_required } => {
                        return Ok(unavailable_payload(
                            SCHEMA_VERSION,
                            &self.description.name,
                            refresh_required,
                            Some(&feedback_target_handle),
                        ));
                    }
                };
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let service_ctx =
                ServiceContext::new_live(&clock, &rng, &external).with_actor("mcp_client");
            let delegation = McpFeedbackDelegation {
                client_id_hash: &client_id_hash,
                conversation_handle_hash: &conversation_handle_hash,
                tool_name: &tool_name,
            };
            let feedback_input = ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action,
                actor: "mcp_client".to_string(),
                actor_id: Some(client_id_hash.clone()),
                payload_json: service_metadata.clone().map(|value| value.to_string()),
            };
            let replay_input = ClaimFeedbackReplayInput {
                feedback: feedback_input,
                replay_event_id: mcp_feedback_replay_event_id(
                    &feedback_target_handle,
                    action,
                    service_metadata.as_ref(),
                    &client_id_hash,
                    &conversation_handle_hash,
                    &tool_name,
                )?,
                submitted_at: service_ctx.clock.now().to_rfc3339(),
            };
            if let Some(outcome) =
                match crate::services::claims::recorded_claim_feedback_replay_outcome_from_mcp(
                    db,
                    replay_input.clone(),
                    delegation,
                ) {
                    Ok(outcome) => outcome,
                    Err(
                        crate::services::claims::ClaimError::UnknownClaimId(_)
                        | crate::services::claims::ClaimError::ClaimNotFound(_),
                    ) => {
                        return Ok(unavailable_payload(
                            SCHEMA_VERSION,
                            &self.description.name,
                            true,
                            Some(&feedback_target_handle),
                        ));
                    }
                    Err(error) => {
                        return Err(ToolError::UpstreamFailure {
                            detail: error.to_string(),
                        });
                    }
                }
            {
                return Ok(json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_name": self.description.name.as_str(),
                    "status": "already_recorded",
                    "action": outcome.action.as_str(),
                    "verification_state": serde_json::to_value(outcome.new_verification_state)
                        .unwrap_or(Value::String("unknown".to_string())),
                    "applied_at_pending": outcome.applied_at_pending,
                    "repair_queued": outcome.repair_job_id.is_some(),
                    "mutation_cursor": mutation_cursor_for_target(
                        &self.description.name,
                        "already_recorded",
                        "claim",
                        &feedback_target_handle,
                    ),
                }));
            }
            let outcome = match crate::services::claims::record_claim_feedback_replay_from_mcp(
                &service_ctx,
                db,
                replay_input,
                delegation,
            ) {
                Ok(outcome) => outcome,
                Err(
                    crate::services::claims::ClaimError::UnknownClaimId(_)
                    | crate::services::claims::ClaimError::ClaimNotFound(_),
                ) => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        true,
                        Some(&feedback_target_handle),
                    ));
                }
                Err(error) => {
                    return Err(ToolError::UpstreamFailure {
                        detail: error.to_string(),
                    });
                }
            };

            let claim = crate::services::claims::load_claim_by_id(db.conn_ref(), &claim_id)
                .map_err(|error| ToolError::UpstreamFailure {
                    detail: error.to_string(),
                })?
                .ok_or_else(|| ToolError::UpstreamFailure {
                    detail: "feedback target claim could not be reloaded".to_string(),
                })?;
            let watermark = claim_watermark_for_handle(&claim);
            let next_handle = mint_target_handle(
                db,
                MintTargetHandle {
                    actor,
                    originating_tool: &self.description.name,
                    result_item_path: "/feedback_target",
                    target_kind: TargetKind::Claim,
                    target_ref: json!({
                        "claim_id": claim.id,
                        "claim_version": claim.claim_version,
                    }),
                    sensitivity_tier: "internal",
                    provenance_material: "mcp_claim_feedback",
                    watermark_material: &watermark,
                },
            )
            .map_err(|error| internal_trace("mcp_feedback_handle_mint", error))?;

            Ok(json!({
                "schema_version": SCHEMA_VERSION,
                "tool_name": self.description.name.as_str(),
                "status": "ok",
                "action": outcome.action.as_str(),
                "verification_state": serde_json::to_value(outcome.new_verification_state)
                    .unwrap_or(Value::String("unknown".to_string())),
                "applied_at_pending": outcome.applied_at_pending,
                "repair_queued": outcome.repair_job_id.is_some(),
                "feedback_target_handle": next_handle,
                "mutation_cursor": mutation_cursor_for_target(
                    &self.description.name,
                    "ok",
                    "claim",
                    &next_handle,
                ),
            }))
        });
        result.unwrap_or_else(|| {
            Err(ToolError::Internal {
                trace_id: "mcp_claim_feedback_missing_owned_connection".to_string(),
            })
        })
    }
}

#[derive(Debug, Clone)]
struct SanitizedMcpFeedbackMetadata {
    payload_json: Option<Value>,
    source_provenance_handle: Option<String>,
}

enum MetadataMaterialization {
    Available(Option<Value>),
    Unavailable { refresh_required: bool },
}

fn sanitize_mcp_feedback_metadata(
    action: FeedbackAction,
    metadata: Option<&Value>,
) -> Result<SanitizedMcpFeedbackMetadata, ToolError> {
    let Some(metadata) = metadata else {
        return match action {
            FeedbackAction::WrongSource
            | FeedbackAction::NeedsNuance
            | FeedbackAction::SurfaceInappropriate
            | FeedbackAction::NotRelevantHere => Err(bad_params(format!(
                "`metadata` is required for `{}` feedback",
                action.as_str()
            ))),
            _ => Ok(SanitizedMcpFeedbackMetadata {
                payload_json: None,
                source_provenance_handle: None,
            }),
        };
    };
    let object = metadata
        .as_object()
        .ok_or_else(|| bad_params("`metadata` must be a JSON object when present"))?;
    reject_disallowed_metadata_keys_recursive(metadata)?;

    let allowed_keys = allowed_metadata_keys(action);
    for key in object.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            return Err(bad_params(format!(
                "`metadata.{key}` is not accepted for `{}` feedback",
                action.as_str()
            )));
        }
        if disallowed_metadata_key(key) {
            return Err(bad_params(format!(
                "`metadata.{key}` cannot contain caller-supplied identifiers"
            )));
        }
    }

    match action {
        FeedbackAction::WrongSource => {
            let handle = required_metadata_string(
                object,
                "source_provenance_handle",
                action,
                1,
                256,
                StringPolicy::AllowPublicHandle,
            )?;
            if !handle.starts_with("mth_") {
                return Err(bad_params(
                    "`metadata.source_provenance_handle` must be a server-issued MCP handle",
                ));
            }
            Ok(SanitizedMcpFeedbackMetadata {
                payload_json: None,
                source_provenance_handle: Some(handle),
            })
        }
        FeedbackAction::NeedsNuance => {
            let corrected_text = required_metadata_string(
                object,
                "corrected_text",
                action,
                1,
                2_000,
                StringPolicy::RejectPublicHandle,
            )?;
            Ok(SanitizedMcpFeedbackMetadata {
                payload_json: Some(json!({ "corrected_text": corrected_text })),
                source_provenance_handle: None,
            })
        }
        FeedbackAction::SurfaceInappropriate => {
            let surface = required_metadata_string(
                object,
                "surface",
                action,
                1,
                120,
                StringPolicy::RejectPublicHandle,
            )?;
            Ok(SanitizedMcpFeedbackMetadata {
                payload_json: Some(json!({ "surface": surface })),
                source_provenance_handle: None,
            })
        }
        FeedbackAction::NotRelevantHere => {
            let invocation_id = required_metadata_string(
                object,
                "invocation_id",
                action,
                1,
                160,
                StringPolicy::RejectPublicHandle,
            )?;
            let note =
                optional_metadata_string(object, "note", 1, 200, StringPolicy::RejectPublicHandle)?;
            let mut payload = serde_json::Map::new();
            payload.insert("invocation_id".to_string(), Value::String(invocation_id));
            if let Some(note) = note {
                payload.insert("note".to_string(), Value::String(note));
            }
            Ok(SanitizedMcpFeedbackMetadata {
                payload_json: Some(Value::Object(payload)),
                source_provenance_handle: None,
            })
        }
        _ => {
            if !object.is_empty() {
                return Err(bad_params(format!(
                    "`metadata` is not accepted for `{}` feedback",
                    action.as_str()
                )));
            }
            Ok(SanitizedMcpFeedbackMetadata {
                payload_json: None,
                source_provenance_handle: None,
            })
        }
    }
}

fn materialize_mcp_feedback_metadata(
    db: &crate::db::ActionDb,
    actor: &McpActor,
    action: FeedbackAction,
    metadata: &SanitizedMcpFeedbackMetadata,
) -> Result<MetadataMaterialization, ToolError> {
    if !matches!(action, FeedbackAction::WrongSource) {
        return Ok(MetadataMaterialization::Available(
            metadata.payload_json.clone(),
        ));
    }
    let Some(source_handle) = metadata.source_provenance_handle.as_deref() else {
        return Err(bad_params(
            "`metadata.source_provenance_handle` is required for `wrong_source` feedback",
        ));
    };
    let resolved = match resolve_target_handle(
        db,
        ResolveTargetHandle {
            actor,
            handle: source_handle,
            expected_kind: TargetKind::SourceProvenance,
            current_watermark_material: None,
        },
    ) {
        Ok(resolved) => resolved,
        Err(TargetHandleResolutionError::Unavailable { refresh_required }) => {
            return Ok(MetadataMaterialization::Unavailable { refresh_required });
        }
        Err(TargetHandleResolutionError::Internal(detail)) => {
            return Err(internal_trace("mcp_feedback_source_handle_resolve", detail));
        }
    };
    let current_watermark = source_provenance_watermark(&resolved.target_ref);
    if !resolved_target_watermark_matches(&resolved, &current_watermark)
        .map_err(|error| internal_trace("mcp_feedback_source_watermark", error))?
    {
        return Ok(MetadataMaterialization::Unavailable {
            refresh_required: true,
        });
    }
    let handle_hash = public_handle_hash(source_handle)
        .map_err(|error| internal_trace("mcp_feedback_source_handle_hash", error))?;
    Ok(MetadataMaterialization::Available(Some(json!({
        "source_ref": json!({
            "kind": "mcp_source_provenance_handle",
            "handle_hash": handle_hash,
        })
        .to_string(),
    }))))
}

fn allowed_metadata_keys(action: FeedbackAction) -> &'static [&'static str] {
    match action {
        FeedbackAction::WrongSource => &["source_provenance_handle"],
        FeedbackAction::NeedsNuance => &["corrected_text"],
        FeedbackAction::SurfaceInappropriate => &["surface"],
        FeedbackAction::NotRelevantHere => &["invocation_id", "note"],
        _ => &[],
    }
}

fn disallowed_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "claim_id"
            | "claimId"
            | "action_id"
            | "actionId"
            | "entity_id"
            | "entityId"
            | "account_id"
            | "accountId"
            | "project_id"
            | "projectId"
            | "person_id"
            | "personId"
            | "meeting_id"
            | "meetingId"
            | "source_id"
            | "sourceId"
            | "source_ref"
            | "sourceRef"
            | "workspace_id"
            | "workspaceId"
            | "file_path"
            | "filePath"
            | "path"
            | "idempotency_key"
            | "idempotencyKey"
            | "feedback_target_handle"
            | "feedbackTargetHandle"
            | "entity_handle"
            | "entityHandle"
    )
}

fn reject_disallowed_metadata_keys_recursive(value: &Value) -> Result<(), ToolError> {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                if key != "source_provenance_handle" && disallowed_metadata_key(key) {
                    return Err(bad_params(format!(
                        "`metadata.{key}` cannot contain caller-supplied identifiers"
                    )));
                }
                reject_disallowed_metadata_keys_recursive(nested)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                reject_disallowed_metadata_keys_recursive(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringPolicy {
    AllowPublicHandle,
    RejectPublicHandle,
}

fn required_metadata_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    action: FeedbackAction,
    min_chars: usize,
    max_chars: usize,
    policy: StringPolicy,
) -> Result<String, ToolError> {
    let value = object.get(key).ok_or_else(|| {
        bad_params(format!(
            "`metadata.{key}` is required for `{}` feedback",
            action.as_str()
        ))
    })?;
    metadata_string_value(value, key, min_chars, max_chars, policy)
}

fn optional_metadata_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    min_chars: usize,
    max_chars: usize,
    policy: StringPolicy,
) -> Result<Option<String>, ToolError> {
    object
        .get(key)
        .map(|value| metadata_string_value(value, key, min_chars, max_chars, policy))
        .transpose()
}

fn metadata_string_value(
    value: &Value,
    key: &str,
    min_chars: usize,
    max_chars: usize,
    policy: StringPolicy,
) -> Result<String, ToolError> {
    let value = value
        .as_str()
        .ok_or_else(|| bad_params(format!("`metadata.{key}` must be a string")))?;
    let value = validate_bounded_string(value, &format!("metadata.{key}"), min_chars, max_chars)?;
    if policy == StringPolicy::RejectPublicHandle && value.contains("mth_") {
        return Err(bad_params(format!(
            "`metadata.{key}` cannot contain MCP target handles"
        )));
    }
    Ok(value)
}

fn claim_watermark_for_handle(claim: &crate::db::claims::IntelligenceClaim) -> String {
    format!(
        "claim:{}:{}:{:?}:{:?}",
        claim.id, claim.claim_version, claim.claim_state, claim.verification_state
    )
}

fn mcp_feedback_replay_event_id(
    handle: &str,
    action: FeedbackAction,
    metadata: Option<&Value>,
    client_id_hash: &str,
    conversation_handle_hash: &str,
    tool_name: &str,
) -> Result<String, ToolError> {
    let handle_hash = public_handle_hash(handle)
        .map_err(|error| internal_trace("mcp_feedback_replay_handle_hash", error))?;
    let mut hasher = Sha256::new();
    update_replay_hash_part(&mut hasher, &handle_hash);
    update_replay_hash_part(&mut hasher, action.as_str());
    update_replay_hash_part(&mut hasher, client_id_hash);
    update_replay_hash_part(&mut hasher, conversation_handle_hash);
    update_replay_hash_part(&mut hasher, tool_name);
    if let Some(metadata) = metadata {
        update_replay_hash_part(&mut hasher, &canonical_json_string(metadata));
    }
    Ok(format!("mcp:{:x}", hasher.finalize()))
}

fn update_replay_hash_part(hasher: &mut Sha256, value: &str) {
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
    hasher.update(b"|");
}

fn canonical_json_string(value: &Value) -> String {
    canonical_json_value(value).to_string()
}

fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json_value).collect()),
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                if let Some(value) = map.get(key) {
                    out.insert(key.clone(), canonical_json_value(value));
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn parse_feedback_action(value: &str) -> Result<FeedbackAction, ToolError> {
    match value {
        "confirm_current" => Ok(FeedbackAction::ConfirmCurrent),
        "mark_false" => Ok(FeedbackAction::MarkFalse),
        "needs_nuance" => Ok(FeedbackAction::NeedsNuance),
        "wrong_source" => Ok(FeedbackAction::WrongSource),
        "not_relevant_here" => Ok(FeedbackAction::NotRelevantHere),
        "surface_inappropriate" => Ok(FeedbackAction::SurfaceInappropriate),
        "cannot_verify" => Ok(FeedbackAction::CannotVerify),
        other => Err(bad_params(format!("unsupported feedback action `{other}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use crate::db::claims::{ClaimSensitivity, IntelligenceClaim, TemporalScope};
    use crate::db::{ActionDb, DbAccount};
    use crate::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, ParamSchema, ReturnSpec, Scope, ScopedName, Side,
    };
    use crate::services::mcp_v2::handler_context::McpHandlerContext;
    use crate::services::mcp_v2::local_runtime;

    const TOOL_NAME: &str = "dailyos.submit.claim_feedback";
    const TS: &str = "2026-06-01T12:00:00Z";

    fn description() -> ToolDescription {
        ToolDescription {
            name: ScopedName::new(TOOL_NAME),
            summary: String::new(),
            when_to_call: String::new(),
            when_not_to_call: String::new(),
            side: Side::SubmitCorrection,
            parameters: vec![],
            returns: ReturnSpec {
                schema: ParamSchema(json!({})),
                description: String::new(),
            },
            examples: vec![],
            scopes_required: vec![Scope::new(TOOL_NAME)],
        }
    }

    fn actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("client-claim-feedback-test"),
            conversation_handle: Some(OpaqueConversationHandle::new(
                "conversation-claim-feedback-test",
            )),
            tool_name: ScopedName::new(TOOL_NAME),
            granted_scopes: vec![Scope::new(TOOL_NAME)],
        }
    }

    fn handler() -> ClaimFeedbackHandler {
        ClaimFeedbackHandler::new(description())
    }

    fn seed_claim(db: &ActionDb) -> IntelligenceClaim {
        db.upsert_account(&DbAccount {
            id: "acct-feedback-test".to_string(),
            name: "Feedback Test Account".to_string(),
            updated_at: TS.to_string(),
            ..Default::default()
        })
        .expect("seed account");
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("user:test");
        let proposal = ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: r#"{"kind":"account","id":"acct-feedback-test"}"#.to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: "Procurement risk is elevated.".to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: None,
            source_asof: Some(TS.to_string()),
            observed_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        };
        match commit_claim(&ctx, db, proposal).expect("commit claim") {
            CommittedClaim::Inserted { claim }
            | CommittedClaim::Reinforced { claim, .. }
            | CommittedClaim::Tombstoned { claim } => claim,
            CommittedClaim::Forked { primary_claim, .. } => primary_claim,
        }
    }

    fn mint_feedback_handle(db: &ActionDb, actor: &McpActor, claim: &IntelligenceClaim) -> String {
        let watermark = claim_watermark_for_handle(claim);
        mint_target_handle(
            db,
            MintTargetHandle {
                actor,
                originating_tool: &ScopedName::new("dailyos.read.account_status"),
                result_item_path: "/assessment/facts/0",
                target_kind: TargetKind::Claim,
                target_ref: json!({
                    "claim_id": claim.id,
                    "claim_version": claim.claim_version,
                }),
                sensitivity_tier: "internal",
                provenance_material: "test-claim-feedback-target",
                watermark_material: &watermark,
            },
        )
        .expect("mint feedback handle")
    }

    fn mint_missing_feedback_handle(db: &ActionDb, actor: &McpActor) -> String {
        mint_target_handle(
            db,
            MintTargetHandle {
                actor,
                originating_tool: &ScopedName::new("dailyos.read.account_status"),
                result_item_path: "/assessment/facts/0",
                target_kind: TargetKind::Claim,
                target_ref: json!({ "claim_id": "missing-claim" }),
                sensitivity_tier: "internal",
                provenance_material: "test-missing-claim-feedback-target",
                watermark_material: "claim:missing-claim:unknown",
            },
        )
        .expect("mint missing claim handle")
    }

    fn mint_source_handle(db: &ActionDb, actor: &McpActor) -> String {
        let target_ref = json!({
            "label": "DailyOS source",
            "source_type": "meeting",
            "source_asof": TS,
            "trust_band": "likely_current",
            "redaction_applied": false,
        });
        let watermark = source_provenance_watermark(&target_ref);
        mint_target_handle(
            db,
            MintTargetHandle {
                actor,
                originating_tool: &ScopedName::new("dailyos.read.account_status"),
                result_item_path: "/provenance/sources/0",
                target_kind: TargetKind::SourceProvenance,
                target_ref,
                sensitivity_tier: "internal",
                provenance_material: &watermark,
                watermark_material: &watermark,
            },
        )
        .expect("mint source handle")
    }

    #[test]
    fn claim_feedback_rejects_raw_metadata_keys_recursively() {
        let err = handler()
            .invoke(
                &McpHandlerContext::without_connection(),
                &actor(),
                json!({
                    "feedback_target_handle": "mth_placeholder",
                    "action": "needs_nuance",
                    "metadata": {
                        "corrected_text": "The risk needs more context.",
                        "nested": { "source_id": "raw-source-id" }
                    }
                }),
            )
            .expect_err("raw metadata keys must be rejected before DB access");
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn claim_feedback_rejects_wrong_source_raw_source_ref_metadata() {
        let err = handler()
            .invoke(
                &McpHandlerContext::without_connection(),
                &actor(),
                json!({
                    "feedback_target_handle": "mth_placeholder",
                    "action": "wrong_source",
                    "metadata": {
                        "source_ref": "fixture://raw-source"
                    }
                }),
            )
            .expect_err("caller-supplied source_ref must be rejected");
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn claim_feedback_missing_claim_handle_returns_unavailable() {
        local_runtime::with_target_handle_key_for_tests([41_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db = ActionDb::open_at_unencrypted(dir.path().join("missing-claim-feedback.db"))
                .expect("open db");
            let actor = actor();
            let handle = mint_missing_feedback_handle(&db, &actor);
            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&db));

            let payload = handler()
                .invoke(
                    &ctx,
                    &actor,
                    json!({
                        "feedback_target_handle": handle,
                        "action": "confirm_current"
                    }),
                )
                .expect("missing claim handle returns a no-op payload");

            assert_eq!(payload["status"], "unavailable");
            assert_eq!(payload["refresh_required"], true);
        });
    }

    #[test]
    fn wrong_source_feedback_persists_source_handle_hash_not_public_handle() {
        local_runtime::with_target_handle_key_for_tests([42_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db = ActionDb::open_at_unencrypted(dir.path().join("wrong-source-feedback.db"))
                .expect("open db");
            let actor = actor();
            let claim = seed_claim(&db);
            let feedback_handle = mint_feedback_handle(&db, &actor, &claim);
            let source_handle = mint_source_handle(&db, &actor);
            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&db));

            let payload = handler()
                .invoke(
                    &ctx,
                    &actor,
                    json!({
                        "feedback_target_handle": feedback_handle,
                        "action": "wrong_source",
                        "metadata": {
                            "source_provenance_handle": source_handle
                        }
                    }),
                )
                .expect("wrong_source feedback response");
            assert_eq!(payload["status"], "ok");

            let persisted_payload: String = db
                .lock()
                .unwrap()
                .conn_ref()
                .query_row(
                    "SELECT payload_json FROM claim_feedback WHERE claim_id = ?1",
                    [&claim.id],
                    |row| row.get(0),
                )
                .expect("feedback payload");
            assert!(
                !persisted_payload.contains("mth_"),
                "public MCP handles must not be persisted in feedback payloads: {persisted_payload}"
            );
            assert!(
                persisted_payload.contains("mcp_source_provenance_handle"),
                "wrong_source stores a service-derived source handle hash"
            );
        });
    }
}
