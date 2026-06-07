//! `dailyos.submit.note` MCP handler.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::mcp_v2::handler_context::McpHandlerContext;
use crate::services::mcp_v2::target_handles::{
    mint_replacing_target_handle, public_handle_hash, resolve_target_handle,
    resolved_target_watermark_matches, MintTargetHandle, ResolveTargetHandle,
    TargetHandleResolutionError, TargetKind,
};
use crate::signals::propagation::PropagationEngine;

use super::tool_utils::{
    bad_params, current_entity_watermark, deterministic_uuid_from_replay_key,
    mcp_submit_replay_key, mutation_cursor_for_target, reject_raw_id_params, required_str,
    source_provenance_watermark, unavailable_payload, validate_bounded_string,
};

const SCHEMA_VERSION: &str = "mcp.note_submit.v1";

pub struct NoteHandler {
    description: ToolDescription,
    signal_engine: Arc<PropagationEngine>,
}

impl NoteHandler {
    pub fn new(description: ToolDescription, signal_engine: Arc<PropagationEngine>) -> Self {
        Self {
            description,
            signal_engine,
        }
    }
}

impl McpToolHandler for NoteHandler {
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
        let text = validate_bounded_string(&required_str(&params, "text")?, "text", 1, 2000)?;
        let entity_handle = required_str(&params, "entity_handle")?;
        let subject_text = super::tool_utils::optional_str(&params, "subject_text")?
            .unwrap_or_else(|| "MCP note".to_string());
        let title = validate_bounded_string(&subject_text, "subject_text", 1, 200)?;
        let source_handle = super::tool_utils::optional_str(&params, "source_provenance_handle")?;

        let result = ctx.with_conn(|db| {
            let resolved_entity = match resolve_target_handle(
                db,
                ResolveTargetHandle {
                    actor,
                    handle: &entity_handle,
                    expected_kind: TargetKind::Entity,
                    current_watermark_material: None,
                },
            ) {
                Ok(resolved) => resolved,
                Err(TargetHandleResolutionError::Unavailable { refresh_required }) => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        refresh_required,
                        Some(&entity_handle),
                    ));
                }
                Err(TargetHandleResolutionError::Internal(detail)) => {
                    return Err(ToolError::Internal {
                        trace_id: format!("mcp_note_entity_handle_resolve:{detail}"),
                    });
                }
            };
            let Some(current_entity_watermark) =
                current_entity_watermark(db, &resolved_entity.target_ref)?
            else {
                return Ok(unavailable_payload(
                    SCHEMA_VERSION,
                    &self.description.name,
                    true,
                    Some(&entity_handle),
                ));
            };
            if !resolved_target_watermark_matches(&resolved_entity, &current_entity_watermark)
                .map_err(|error| ToolError::Internal {
                    trace_id: format!("mcp_note_entity_handle_watermark:{error}"),
                })?
            {
                return Ok(unavailable_payload(
                    SCHEMA_VERSION,
                    &self.description.name,
                    true,
                    Some(&entity_handle),
                ));
            }
            let entity_type = resolved_entity
                .target_ref
                .get("entity_type")
                .and_then(Value::as_str)
                .ok_or_else(|| bad_params("entity handle is unavailable"))?
                .to_string();
            let entity_id = resolved_entity
                .target_ref
                .get("entity_id")
                .and_then(Value::as_str)
                .ok_or_else(|| bad_params("entity handle is unavailable"))?
                .to_string();
            let source_ref = if let Some(handle) = source_handle.as_deref() {
                match resolve_target_handle(
                    db,
                    ResolveTargetHandle {
                        actor,
                        handle,
                        expected_kind: TargetKind::SourceProvenance,
                        current_watermark_material: None,
                    },
                ) {
                    Ok(resolved_source) => {
                        let current_source_watermark =
                            source_provenance_watermark(&resolved_source.target_ref);
                        if !resolved_target_watermark_matches(
                            &resolved_source,
                            &current_source_watermark,
                        )
                        .map_err(|error| ToolError::Internal {
                            trace_id: format!("mcp_note_source_handle_watermark:{error}"),
                        })? {
                            return Ok(unavailable_payload(
                                SCHEMA_VERSION,
                                &self.description.name,
                                true,
                                Some(handle),
                            ));
                        }
                        let handle_hash =
                            public_handle_hash(handle).map_err(|error| ToolError::Internal {
                                trace_id: format!("mcp_note_source_handle_hash:{error}"),
                            })?;
                        Some(
                            json!({
                                "kind": "mcp_source_provenance_handle",
                                "handle_hash": handle_hash,
                            })
                            .to_string(),
                        )
                    }
                    Err(TargetHandleResolutionError::Unavailable { refresh_required }) => {
                        return Ok(unavailable_payload(
                            SCHEMA_VERSION,
                            &self.description.name,
                            refresh_required,
                            Some(handle),
                        ));
                    }
                    Err(TargetHandleResolutionError::Internal(detail)) => {
                        return Err(ToolError::Internal {
                            trace_id: format!("mcp_note_source_handle_resolve:{detail}"),
                        });
                    }
                }
            } else {
                None
            };

            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let service_ctx =
                ServiceContext::new_live(&clock, &rng, &external).with_actor("mcp_client");
            let observed_at = service_ctx.clock.now().to_rfc3339();
            let replay_key = mcp_submit_replay_key(
                actor,
                &self.description.name,
                "submit.note",
                &json!({
                    "entity_type": entity_type.clone(),
                    "entity_id": entity_id.clone(),
                    "title": title.clone(),
                    "text": text.clone(),
                    "source_ref": source_ref.clone(),
                }),
            )?;
            let deterministic_claim_id =
                deterministic_uuid_from_replay_key("note_claim", &replay_key)?;
            let entry = crate::services::entity_context::create_entry_with_db_with_claim_id(
                &service_ctx,
                db,
                &self.signal_engine,
                &entity_type,
                &entity_id,
                &title,
                &text,
                "mcp_client",
                &observed_at,
                source_ref.as_deref(),
                Some(&deterministic_claim_id),
            )
            .map_err(|error| ToolError::UpstreamFailure { detail: error })?;
            let claim = crate::services::claims::load_claim_by_id(db.conn_ref(), &entry.id)
                .map_err(|error| ToolError::UpstreamFailure {
                    detail: error.to_string(),
                })?
                .ok_or_else(|| ToolError::UpstreamFailure {
                    detail: "created note claim could not be reloaded".to_string(),
                })?;
            let watermark = claim_watermark_for_handle(&claim);
            let note_handle = mint_replacing_target_handle(
                db,
                MintTargetHandle {
                    actor,
                    originating_tool: &self.description.name,
                    result_item_path: "/note",
                    target_kind: TargetKind::Claim,
                    target_ref: json!({ "claim_id": entry.id.clone() }),
                    sensitivity_tier: "internal",
                    provenance_material: &format!("mcp_submit_note:{}:note", entry.id),
                    watermark_material: &watermark,
                },
            )
            .map_err(|error| ToolError::Internal {
                trace_id: format!("mcp_note_handle_mint:{error}"),
            })?;
            let feedback_target_handle = mint_replacing_target_handle(
                db,
                MintTargetHandle {
                    actor,
                    originating_tool: &self.description.name,
                    result_item_path: "/feedback_target",
                    target_kind: TargetKind::Claim,
                    target_ref: json!({ "claim_id": entry.id.clone() }),
                    sensitivity_tier: "internal",
                    provenance_material: &format!("mcp_submit_note:{}:feedback", entry.id),
                    watermark_material: &watermark,
                },
            )
            .map_err(|error| ToolError::Internal {
                trace_id: format!("mcp_note_feedback_handle_mint:{error}"),
            })?;
            Ok(json!({
                "schema_version": SCHEMA_VERSION,
                "tool_name": self.description.name.as_str(),
                "status": "ok",
                "note_handle": note_handle,
                "feedback_target_handle": feedback_target_handle,
                "mutation_cursor": mutation_cursor_for_target(
                    &self.description.name,
                    "ok",
                    "claim",
                    &note_handle,
                ),
            }))
        });
        result.unwrap_or_else(|| {
            Err(ToolError::Internal {
                trace_id: "mcp_submit_note_missing_owned_connection".to_string(),
            })
        })
    }
}

fn claim_watermark_for_handle(claim: &crate::db::claims::IntelligenceClaim) -> String {
    format!(
        "claim:{}:{}:{:?}:{:?}",
        claim.id, claim.claim_version, claim.claim_state, claim.verification_state
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::db::{ActionDb, DbAccount};
    use crate::services::entity_context::USER_NOTE_CLAIM_TYPE;
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, ParamSchema, ReturnSpec, Scope, ScopedName, Side,
    };
    use crate::services::mcp_v2::handlers::tool_utils::entity_watermark_from_parts;
    use crate::services::mcp_v2::local_runtime;
    use crate::services::mcp_v2::target_handles::mint_target_handle;

    fn description() -> ToolDescription {
        ToolDescription {
            name: ScopedName::new("dailyos.submit.note"),
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
            scopes_required: vec![Scope::new("dailyos.submit.note")],
        }
    }

    fn actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("client-note-submit-test"),
            conversation_handle: Some(OpaqueConversationHandle::new(
                "conversation-note-submit-test",
            )),
            tool_name: ScopedName::new("dailyos.submit.note"),
            granted_scopes: vec![Scope::new("dailyos.submit.note")],
        }
    }

    fn account(id: &str, name: &str, updated_at: &str, archived: bool) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            updated_at: updated_at.to_string(),
            archived,
            ..Default::default()
        }
    }

    fn seed_entity_handle(db: &ActionDb, actor: &McpActor, account: &DbAccount) -> String {
        let watermark = entity_watermark_from_parts(
            "account",
            &account.id,
            &account.updated_at,
            account.archived,
        );
        mint_target_handle(
            db,
            MintTargetHandle {
                actor,
                originating_tool: &ScopedName::new("dailyos.read.account_status"),
                result_item_path: "/subject",
                target_kind: TargetKind::Entity,
                target_ref: json!({
                    "entity_type": "account",
                    "entity_id": account.id,
                }),
                sensitivity_tier: "internal",
                provenance_material: "test-account",
                watermark_material: &watermark,
            },
        )
        .expect("mint entity handle")
    }

    fn handler() -> NoteHandler {
        NoteHandler::new(description(), Arc::new(PropagationEngine::new()))
    }

    #[test]
    fn submit_note_replay_reuses_durable_claim_row() {
        local_runtime::with_target_handle_key_for_tests([31_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db = ActionDb::open_at_unencrypted(dir.path().join("submit-note-replay.db"))
                .expect("open db");
            let actor = actor();
            let account = account(
                "acct-note-replay",
                "Replay Account",
                "2026-06-01T00:00:00Z",
                false,
            );
            db.upsert_account(&account).expect("seed account");
            let entity_handle = seed_entity_handle(&db, &actor, &account);
            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&db));
            let handler = handler();
            let params = json!({
                "entity_handle": entity_handle,
                "subject_text": "Renewal note",
                "text": "The renewal owner asked for an updated rollout plan.",
            });

            let first = handler
                .invoke(&ctx, &actor, params.clone())
                .expect("first note response");
            let second = handler
                .invoke(&ctx, &actor, params)
                .expect("second note response");

            assert_eq!(first["status"], "ok");
            assert_eq!(second["status"], "ok");
            let count: i64 = db
                .lock()
                .unwrap()
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM intelligence_claims WHERE claim_type = ?1 AND text = ?2",
                    [
                        USER_NOTE_CLAIM_TYPE,
                        "The renewal owner asked for an updated rollout plan.",
                    ],
                    |row| row.get(0),
                )
                .expect("claim count");
            assert_eq!(count, 1, "MCP retry must not duplicate note claims");
            let handle_count: i64 = db
                .lock()
                .unwrap()
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM mcp_target_handles WHERE originating_tool = ?1",
                    ["dailyos.submit.note"],
                    |row| row.get(0),
                )
                .expect("target handle count");
            assert_eq!(
                handle_count, 2,
                "MCP retry must replace, not duplicate, note and feedback handles"
            );
        });
    }

    #[test]
    fn submit_note_feedback_handle_matches_live_claim_watermark() {
        local_runtime::with_target_handle_key_for_tests([33_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db = ActionDb::open_at_unencrypted(dir.path().join("submit-note-feedback.db"))
                .expect("open db");
            let actor = actor();
            let account = account(
                "acct-note-feedback",
                "Feedback Account",
                "2026-06-01T00:00:00Z",
                false,
            );
            db.upsert_account(&account).expect("seed account");
            let entity_handle = seed_entity_handle(&db, &actor, &account);
            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&db));

            let payload = handler()
                .invoke(
                    &ctx,
                    &actor,
                    json!({
                        "entity_handle": entity_handle,
                        "subject_text": "Feedback-ready note",
                        "text": "The implementation owner asked to confirm the launch checklist.",
                    }),
                )
                .expect("note response");

            let feedback_handle = payload["feedback_target_handle"]
                .as_str()
                .expect("feedback handle");
            let guard = db.lock().unwrap();
            let resolved = resolve_target_handle(
                &guard,
                ResolveTargetHandle {
                    actor: &actor,
                    handle: feedback_handle,
                    expected_kind: TargetKind::Claim,
                    current_watermark_material: None,
                },
            )
            .expect("feedback handle resolves");
            let claim_id = resolved.target_ref["claim_id"]
                .as_str()
                .expect("claim id from sealed target ref");
            let claim = crate::services::claims::load_claim_by_id(guard.conn_ref(), claim_id)
                .expect("load note claim")
                .expect("note claim exists");
            assert!(
                resolved_target_watermark_matches(&resolved, &claim_watermark_for_handle(&claim))
                    .expect("watermark hash"),
                "returned feedback handle must be immediately valid for claim_feedback"
            );
        });
    }

    #[test]
    fn submit_note_rejects_archived_entity_handle() {
        local_runtime::with_target_handle_key_for_tests([32_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db = ActionDb::open_at_unencrypted(dir.path().join("submit-note-stale.db"))
                .expect("open db");
            let actor = actor();
            let active = account(
                "acct-note-stale",
                "Stale Note Account",
                "2026-06-01T00:00:00Z",
                false,
            );
            db.upsert_account(&active).expect("seed account");
            let entity_handle = seed_entity_handle(&db, &actor, &active);
            let mut archived = active;
            archived.archived = true;
            archived.updated_at = Utc::now().to_rfc3339();
            db.upsert_account(&archived).expect("archive account");

            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&db));
            let payload = handler()
                .invoke(
                    &ctx,
                    &actor,
                    json!({
                        "entity_handle": entity_handle,
                        "subject_text": "Archived note",
                        "text": "This note should not be committed.",
                    }),
                )
                .expect("handler response");

            assert_eq!(payload["status"], "unavailable");
            assert_eq!(payload["refresh_required"], true);
            let count: i64 = db
                .lock()
                .unwrap()
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM intelligence_claims WHERE claim_type = ?1 AND text = ?2",
                    [USER_NOTE_CLAIM_TYPE, "This note should not be committed."],
                    |row| row.get(0),
                )
                .expect("claim count");
            assert_eq!(count, 0, "stale entity handles must not create notes");
        });
    }
}
