//! `dailyos.read.account_status` MCP tool handler.
//!
//! Wraps the claim-backed `get_entity_intelligence` ability from
//! `abilities-runtime`, then projects the envelope into an MCP-friendly
//! account briefing with prose plus compact provenance.
//! Sync `McpToolHandler::invoke` is called from within
//! `tokio::task::spawn_blocking` at the transport boundary, so
//! `runtime.block_on(...)` on a captured handle is safe.
//!
//! See `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md` for the L0
//! contract.

use std::collections::BTreeMap;
use std::sync::Arc;

use abilities_runtime::abilities::registry::{AbilityRegistry, McpExposure};
use abilities_runtime::abilities::tracer::NOOP_ABILITY_TRACER;
use serde_json::{json, Value};

use crate::bridges::types::{
    invoke_registry_json_for_actor, AbilityInvokeError, RequestScopedInvocation,
    BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
};
use crate::bridges::{BridgeActor, BridgeSurface};
use crate::db::{ActionDb, DbAccount, DbError, LocalKeychain};
use crate::helpers::normalize_key;
use crate::services::context::{
    attach_live_workspace_readers, ClaimDismissalSurface, ExternalClients, ServiceContext,
    SystemClock, SystemRng,
};
use crate::services::mcp_v2::actor_policy::{project_actor, ToolGrant, ToolRateLimit};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::mcp_v2::handler_context::McpHandlerContext;
use crate::services::mcp_v2::runtime_projection::{
    compact_text, evidence_suffix, humanize_token, open_loop_suffix, project_runtime_evidence,
    string_at, RuntimeEvidenceProjection,
};

const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));

/// Registered ability name in the abilities-runtime registry.
const ABILITY_NAME: &str = "get_entity_intelligence";
const TOOL_NAME: &str = "dailyos.read.account_status";

/// `EntityIntelligenceInput` schema version pinned by the ability contract.
const ENTITY_INTELLIGENCE_SCHEMA_VERSION: u32 = 2;
const ACCOUNT_STATUS_RESPONSE_SCHEMA_VERSION: u32 = 2;

pub struct AccountStatusHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
}

impl AccountStatusHandler {
    pub fn new(
        description: ToolDescription,
        registry: &'static AbilityRegistry,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self {
            description,
            registry,
            runtime,
        }
    }

    /// Convenience constructor for `mcp_v2/main.rs::run_serve`. Workspace
    /// readers are attached at invocation time via
    /// `attach_live_workspace_readers` (each reader opens its own ActionDb
    /// from LocalKeychain). Runtime handle is passed in explicitly because
    /// registration runs in the binary's synchronous startup path before
    /// `runtime.block_on(...)` begins.
    pub fn from_runtime(
        description: ToolDescription,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, &'static str> {
        let registry = AbilityRegistry::global_checked()
            .map_err(|_| "ability registry violations present at startup")?;
        Ok(Self::new(description, registry, runtime))
    }
}

impl McpToolHandler for AccountStatusHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(
        &self,
        ctx: &McpHandlerContext,
        actor: &McpActor,
        params: Value,
    ) -> Result<Value, ToolError> {
        let McpActor::Client {
            client_id,
            conversation_handle,
            tool_name,
            granted_scopes,
        } = actor;

        // Synthesize a ToolGrant from the already-resolved McpActor scopes.
        // The gateway validated grant before dispatch; this is purely for
        // the ADR-0102 §B "manifest resolved before runtime actor
        // construction" projection contract that project_actor preserves.
        let synthetic_grant = ToolGrant {
            tool_name: tool_name.clone(),
            scopes_granted: granted_scopes.clone(),
            exposure: McpExposure::Invocable,
            rate_limit: ToolRateLimit {
                max_calls: 0,
                window_seconds: 0,
            },
        };
        let runtime_actor =
            project_actor(client_id, &synthetic_grant, conversation_handle.as_ref());

        let subject_input = extract_subject(&params)?;
        let resolved_subject = match resolve_account_subject(ctx, &subject_input)? {
            AccountSubjectResolution::Resolved(subject) => subject,
            AccountSubjectResolution::NotFound { input } => {
                return Ok(account_subject_not_found_response(&input));
            }
            AccountSubjectResolution::Ambiguous { input, candidates } => {
                return Ok(account_subject_ambiguous_response(&input, &candidates));
            }
        };

        self.runtime.block_on(async {
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let services = attach_live_workspace_readers(
                ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR_LABEL),
            );

            let resolved_input = json!({
                "schemaVersion": ENTITY_INTELLIGENCE_SCHEMA_VERSION,
                "entityType": "account",
                "entityId": resolved_subject.entity_id.clone(),
                "depth": "standard",
                "sections": ["facts", "open_loops", "relationships", "touchpoints", "record"],
            });

            // Use the request-scoped dispatch path — V2 MCP needs to pass
            // the full `Actor::McpClient { client_id, conversation_handle }`
            // variant through to the registry so get_entity_intelligence's
            // `allowed_actors` can apply the MCP render policy.
            // The legacy `invoke_registry_json` calls `BridgeActor::Agent
            // .registry_actor()` which would erase the client identity.
            let invocation = RequestScopedInvocation {
                registry_actor: runtime_actor,
                response_actor: BridgeActor::McpClient,
                surface: BridgeSurface::McpTool,
                claim_dismissal_surface: ClaimDismissalSurface::McpTool,
                dry_run: false,
                confirmation: None,
                confirmation_store: None,
            };
            let response = invoke_registry_json_for_actor(
                self.registry,
                &services,
                &BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
                &NOOP_ABILITY_TRACER,
                invocation,
                ABILITY_NAME,
                resolved_input,
            )
            .await
            .map_err(map_invoke_error)?;
            let invocation_id = response.invocation_id.0.to_string();
            let mut payload = present_account_status_response_with_context(
                &resolved_subject.entity_id,
                response.data,
                Some(&resolved_subject.display_label),
                Some(&invocation_id),
                None,
            );
            attach_account_subject_resolution(&mut payload, &resolved_subject);
            Ok(payload)
        })
    }
}

fn extract_subject(params: &Value) -> Result<String, ToolError> {
    let subject = params
        .get("subject")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::BadParams {
            detail: "missing 'subject' parameter (expected non-empty string)".into(),
        })?
        .trim();
    if subject.is_empty() {
        return Err(ToolError::BadParams {
            detail: "'subject' must be a non-empty string".into(),
        });
    }
    Ok(subject.to_string())
}

#[derive(Debug, Clone, PartialEq)]
struct ResolvedAccountSubject {
    input: String,
    entity_id: String,
    display_label: String,
    resolution_kind: &'static str,
    match_basis: &'static str,
    match_confidence: &'static str,
    match_confidence_score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AccountSubjectCandidate {
    id: String,
    name: String,
}

impl From<DbAccount> for AccountSubjectCandidate {
    fn from(account: DbAccount) -> Self {
        Self {
            id: account.id,
            name: account.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum AccountSubjectResolution {
    Resolved(ResolvedAccountSubject),
    NotFound {
        input: String,
    },
    Ambiguous {
        input: String,
        candidates: Vec<AccountSubjectCandidate>,
    },
}

/// Resolve the account subject against the DB.
///
/// Prefers the sidecar's one owned connection from `ctx`; falls back to a
/// read-only self-open only when the context carries no owned connection
/// (tests / unadopted paths). The owned connection is opened writable, but
/// these are read-only queries, so borrowing it read-only is correct.
fn resolve_account_subject(
    ctx: &McpHandlerContext,
    subject: &str,
) -> Result<AccountSubjectResolution, ToolError> {
    match ctx.with_conn(|db| resolve_account_subject_with_db(db, subject)) {
        Some(result) => result,
        None => {
            // Documented fallback when the context carries no owned connection
            // (tests / in-app process). The sidecar always installs one, so
            // this never fires in the sidecar path.
            let db = ActionDb::open_readonly(Arc::new(LocalKeychain::new())) // mcp-self-open-allowed: ctx-less fallback
                .map_err(map_subject_resolution_db_error)?;
            resolve_account_subject_with_db(&db, subject)
        }
    }
}

fn resolve_account_subject_with_db(
    db: &ActionDb,
    subject: &str,
) -> Result<AccountSubjectResolution, ToolError> {
    if let Some(account) = db
        .get_account(subject)
        .map_err(map_subject_resolution_db_error)?
    {
        return Ok(resolved_account_subject(
            subject,
            AccountSubjectCandidate::from(account),
            "account_id",
            "id",
            "exact",
            1.0,
        ));
    }

    if let Some(account) = db
        .get_account_by_name(subject)
        .map_err(map_subject_resolution_db_error)?
    {
        return Ok(resolved_account_subject(
            subject,
            AccountSubjectCandidate::from(account),
            "account_name",
            "name",
            "exact",
            1.0,
        ));
    }

    let accounts = db
        .get_all_accounts()
        .map_err(map_subject_resolution_db_error)?
        .into_iter()
        .map(AccountSubjectCandidate::from)
        .collect::<Vec<_>>();
    Ok(resolve_account_subject_from_accounts(subject, &accounts))
}

fn resolve_account_subject_from_accounts(
    subject: &str,
    accounts: &[AccountSubjectCandidate],
) -> AccountSubjectResolution {
    let input = subject.trim().to_string();

    if let Some(account) = accounts.iter().find(|account| account.id == input) {
        return resolved_account_subject(&input, account.clone(), "account_id", "id", "exact", 1.0);
    }

    if let Some(account) = accounts
        .iter()
        .find(|account| account.name.eq_ignore_ascii_case(&input))
    {
        return resolved_account_subject(
            &input,
            account.clone(),
            "account_name",
            "name",
            "exact",
            1.0,
        );
    }

    let normalized_input = normalize_key(&input);
    if normalized_input.is_empty() {
        return AccountSubjectResolution::NotFound { input };
    }

    let mut matches = BTreeMap::new();
    for account in accounts.iter().cloned() {
        let id_matches = normalize_key(&account.id) == normalized_input;
        let name_matches = normalize_key(&account.name) == normalized_input;
        if id_matches || name_matches {
            matches.entry(account.id.clone()).or_insert(account);
        }
    }

    let candidates = matches.into_values().collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => AccountSubjectResolution::NotFound { input },
        [account] => resolved_account_subject(
            &input,
            account.clone(),
            "normalized_slug",
            "normalized_id_or_name",
            "high",
            0.95,
        ),
        _ => AccountSubjectResolution::Ambiguous { input, candidates },
    }
}

fn resolved_account_subject(
    input: &str,
    account: AccountSubjectCandidate,
    resolution_kind: &'static str,
    match_basis: &'static str,
    match_confidence: &'static str,
    match_confidence_score: f64,
) -> AccountSubjectResolution {
    AccountSubjectResolution::Resolved(ResolvedAccountSubject {
        input: input.to_string(),
        entity_id: account.id,
        display_label: account.name,
        resolution_kind,
        match_basis,
        match_confidence,
        match_confidence_score,
    })
}

fn attach_account_subject_resolution(payload: &mut Value, subject: &ResolvedAccountSubject) {
    let resolution = account_subject_resolution_json(subject);
    if let Some(object) = payload.as_object_mut() {
        object.insert("resolution".to_string(), resolution);
    }
    if let Some(subject_object) = payload.get_mut("subject").and_then(Value::as_object_mut) {
        subject_object.insert("input".to_string(), Value::String(subject.input.clone()));
        subject_object.insert("id".to_string(), Value::String(subject.entity_id.clone()));
        subject_object.insert(
            "displayLabel".to_string(),
            Value::String(subject.display_label.clone()),
        );
        subject_object.insert(
            "resolutionKind".to_string(),
            Value::String(subject.resolution_kind.to_string()),
        );
        subject_object.insert(
            "matchConfidence".to_string(),
            Value::String(subject.match_confidence.to_string()),
        );
        subject_object.insert(
            "matchConfidenceScore".to_string(),
            json!(subject.match_confidence_score),
        );
    }
}

fn account_subject_resolution_json(subject: &ResolvedAccountSubject) -> Value {
    json!({
        "input": subject.input,
        "entityType": "account",
        "resolvedEntityId": subject.entity_id,
        "displayLabel": subject.display_label,
        "resolutionKind": subject.resolution_kind,
        "matchBasis": subject.match_basis,
        "matchConfidence": subject.match_confidence,
        "matchConfidenceScore": subject.match_confidence_score,
        "caveats": [],
    })
}

fn account_subject_not_found_response(input: &str) -> Value {
    account_subject_resolution_response(
        "not_found",
        input,
        "DailyOS could not resolve the requested subject to an account in the local workspace. Try an exact account name or account id.",
        json!({
            "input": input,
            "entityType": "account",
            "resolvedEntityId": Value::Null,
            "resolutionKind": "not_found",
            "matchConfidence": "none",
            "matchConfidenceScore": 0.0,
            "candidates": [],
            "caveats": ["No matching account id, exact account name, or normalized account slug was found."],
        }),
    )
}

fn account_subject_ambiguous_response(
    input: &str,
    candidates: &[AccountSubjectCandidate],
) -> Value {
    let candidate_values = candidates
        .iter()
        .take(5)
        .map(|candidate| {
            json!({
                "entityId": candidate.id,
                "displayLabel": candidate.name,
            })
        })
        .collect::<Vec<_>>();
    account_subject_resolution_response(
        "clarification_required",
        input,
        "DailyOS found multiple local accounts matching the requested subject. Retry with an exact account id.",
        json!({
            "input": input,
            "entityType": "account",
            "resolvedEntityId": Value::Null,
            "resolutionKind": "ambiguous",
            "matchConfidence": "none",
            "matchConfidenceScore": 0.0,
            "candidates": candidate_values,
            "caveats": ["Multiple accounts matched the same normalized subject."],
        }),
    )
}

fn account_subject_resolution_response(
    status: &str,
    input: &str,
    answer: &str,
    resolution: Value,
) -> Value {
    let section_state = json!({
        "kind": "empty",
        "reason": status,
    });
    let caveat = resolution
        .get("caveats")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .unwrap_or("Subject resolution did not produce a single local account.");
    json!({
        "schemaVersion": ACCOUNT_STATUS_RESPONSE_SCHEMA_VERSION,
        "toolName": TOOL_NAME,
        "surface": TOOL_NAME,
        "producer": ABILITY_NAME,
        "status": status,
        "invocationId": Value::Null,
        "provenanceHandle": Value::Null,
        "subject": {
            "kind": "account",
            "input": input,
            "displayLabel": input,
        },
        "resolution": resolution,
        "answer": answer,
        "assessment": {
            "facts": [],
            "openLoops": [],
            "relationships": [],
            "touchpoints": [],
            "recordEntries": [],
            "priorities": [],
            "caveats": [{ "text": caveat }],
        },
        "trust": {
            "aggregateBand": "unscored",
            "sectionCaveats": {
                "subject": caveat,
            },
        },
        "sensitivity": Value::Null,
        "provenance": {
            "sources": [],
            "redactionApplied": false,
            "rawClaimIdsIncluded": false,
        },
        "sectionStates": {
            "facts": section_state,
            "openLoops": section_state,
            "relationships": section_state,
            "touchpoints": section_state,
            "recordEntries": section_state,
        },
        "sections": {
            "facts": section_state,
            "openLoops": section_state,
            "relationships": section_state,
            "touchpoints": section_state,
            "recordEntries": section_state,
        },
        "truncation": {},
        "sourceEnvelope": {
            "schemaVersion": Value::Null,
            "rawEnvelopeIncluded": false,
        },
    })
}

pub fn present_account_status_response(subject: &str, envelope: Value) -> Value {
    present_account_status_response_with_context(subject, envelope, None, None, None)
}

pub fn present_account_status_response_with_label(
    subject: &str,
    envelope: Value,
    display_label_override: Option<&str>,
) -> Value {
    present_account_status_response_with_context(
        subject,
        envelope,
        display_label_override,
        None,
        None,
    )
}

pub fn present_account_status_response_with_context(
    subject: &str,
    envelope: Value,
    display_label_override: Option<&str>,
    invocation_id: Option<&str>,
    provenance_detail_tool: Option<&str>,
) -> Value {
    let subject_value = envelope
        .get("subject")
        .cloned()
        .unwrap_or_else(|| fallback_subject(subject));
    let label = display_label_override
        .map(compact_text)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            subject_value
                .get("displayLabel")
                .and_then(Value::as_str)
                .map(compact_text)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| subject.to_string());
    let subject_value = if display_label_override.is_some() {
        let mut subject_map = subject_value.as_object().cloned().unwrap_or_default();
        subject_map.insert("displayLabel".to_string(), Value::String(label.clone()));
        Value::Object(subject_map)
    } else {
        subject_value
    };

    let projection = project_runtime_evidence(&envelope);
    let answer = build_account_status_answer(&label, &projection, &envelope);
    let provenance_handle = invocation_id.map(|id| {
        json!({
            "invocationId": id,
            "invocation_id": id,
            "detailAvailable": provenance_detail_tool.is_some(),
            "detailTool": provenance_detail_tool,
            "detailParams": {
                "invocation_id": id
            }
        })
    });

    json!({
        "schemaVersion": ACCOUNT_STATUS_RESPONSE_SCHEMA_VERSION,
        "toolName": TOOL_NAME,
        "surface": TOOL_NAME,
        "producer": ABILITY_NAME,
        "status": projection.status,
        "invocationId": invocation_id,
        "provenanceHandle": provenance_handle,
        "subject": subject_value,
        "answer": answer,
        "assessment": {
            "facts": projection.facts,
            "openLoops": projection.open_loops,
            "relationships": projection.relationships,
            "touchpoints": projection.touchpoints,
            "recordEntries": projection.record_entries,
            "priorities": projection.priorities,
            "caveats": projection.caveats,
        },
        "trust": envelope.get("trust").cloned().unwrap_or(Value::Null),
        "sensitivity": envelope.get("sensitivity").cloned().unwrap_or(Value::Null),
        "provenance": projection.provenance,
        "sectionStates": projection.section_states.clone(),
        "sections": projection.section_states,
        "truncation": projection.truncation,
        "sourceEnvelope": {
            "schemaVersion": envelope.get("schemaVersion").cloned().unwrap_or(Value::Null),
            "rawEnvelopeIncluded": false,
        },
    })
}

fn fallback_subject(subject: &str) -> Value {
    json!({
        "kind": "account",
        "id": subject,
        "displayLabel": subject,
    })
}

fn build_account_status_answer(
    label: &str,
    projection: &RuntimeEvidenceProjection,
    envelope: &Value,
) -> String {
    if projection.facts.is_empty()
        && projection.open_loops.is_empty()
        && projection.relationships.is_empty()
        && projection.touchpoints.is_empty()
        && projection.record_entries.is_empty()
    {
        if let Some(advisory) = relationship_partial_failure_advisory(envelope) {
            return format!(
                "DailyOS could not read requested intelligence for {label}: {advisory}."
            );
        }
        return format!("DailyOS does not yet have claim-backed account intelligence for {label}.");
    }

    let mut lines = vec![format!("DailyOS account briefing for {label}.")];

    if !projection.facts.is_empty() {
        lines.push(String::new());
        lines.push("Assessment:".to_string());
        for fact in projection.facts.iter().take(5) {
            if let Some(text) = string_at(fact, "/text") {
                lines.push(format!("- {}{}", text, evidence_suffix(fact)));
            }
        }
    }

    if !projection.open_loops.is_empty() {
        lines.push(String::new());
        lines.push("Open loops:".to_string());
        for open_loop in projection.open_loops.iter().take(5) {
            if let Some(description) = string_at(open_loop, "/description") {
                lines.push(format!("- {}{}", description, open_loop_suffix(open_loop)));
            }
        }
    }

    if !projection.relationships.is_empty() {
        let participants = projection
            .relationships
            .iter()
            .filter(|item| string_at(item, "/kind") == Some("participant"))
            .take(3)
            .collect::<Vec<_>>();
        if !participants.is_empty() {
            lines.push(String::new());
            lines.push("Top participants by meeting attendance:".to_string());
        }
        for participant in participants {
            if let Some(display_label) = string_at(participant, "/displayLabel") {
                let touchpoint_count = participant
                    .get("normalizedTouchpointCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let role = string_at(participant, "/role")
                    .map(|value| format!("; role: {value}"))
                    .unwrap_or_default();
                lines.push(format!(
                    "- {display_label}: {touchpoint_count} touchpoint(s){role}{}",
                    evidence_suffix(participant)
                ));
            }
        }

        let edges = projection
            .relationships
            .iter()
            .filter(|item| string_at(item, "/kind") == Some("edge"))
            .take(3)
            .collect::<Vec<_>>();
        if !edges.is_empty() {
            lines.push(String::new());
            lines.push("Relationship evidence:".to_string());
        }
        for edge in edges {
            if let Some(relationship) = string_at(edge, "/relationship") {
                let label = string_at(edge, "/displayLabel")
                    .map(|value| format!(" with {value}"))
                    .unwrap_or_default();
                lines.push(format!("- {relationship}{label}{}", evidence_suffix(edge)));
            }
        }
    }

    if !projection.touchpoints.is_empty() {
        lines.push(String::new());
        lines.push("Touchpoint evidence:".to_string());
        for touchpoint in projection.touchpoints.iter().take(5) {
            let timing = string_at(touchpoint, "/timing").unwrap_or("touchpoint");
            let kind = string_at(touchpoint, "/kind")
                .map(humanize_token)
                .unwrap_or_else(|| "touchpoint".to_string());
            if let Some(when) = string_at(touchpoint, "/when") {
                lines.push(format!(
                    "- {} {} on {}{}",
                    humanize_token(timing),
                    kind,
                    when,
                    evidence_suffix(touchpoint)
                ));
            }
        }
    }

    if !projection.record_entries.is_empty() {
        lines.push(String::new());
        lines.push("Record evidence:".to_string());
        for entry in projection.record_entries.iter().take(5) {
            if let Some(text) = string_at(entry, "/text") {
                let recorded_at = string_at(entry, "/recordedAt")
                    .map(|value| format!(" recorded {value}"))
                    .unwrap_or_default();
                lines.push(format!(
                    "- {}{}{}",
                    text,
                    recorded_at,
                    evidence_suffix(entry)
                ));
            }
        }
    }

    let source_count = projection
        .provenance
        .pointer("/sources")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let trust = string_at(envelope, "/trust/aggregateBand")
        .map(humanize_token)
        .unwrap_or_else(|| "unscored".to_string());
    lines.push(String::new());
    lines.push(format!(
        "Provenance: {source_count} source(s) surfaced; trust posture {trust}."
    ));
    lines.join("\n")
}

fn relationship_partial_failure_advisory(envelope: &Value) -> Option<String> {
    match envelope {
        Value::Object(object) => {
            for key in ["partial_failure", "partialFailure"] {
                if let Some(advisory) = object
                    .get(key)
                    .and_then(|failure| failure.get("advisory"))
                    .and_then(Value::as_str)
                    .map(compact_text)
                    .filter(|value| !value.is_empty())
                {
                    return Some(advisory);
                }
            }
            object
                .values()
                .find_map(relationship_partial_failure_advisory)
        }
        Value::Array(items) => items.iter().find_map(relationship_partial_failure_advisory),
        _ => None,
    }
}

fn map_invoke_error(err: AbilityInvokeError) -> ToolError {
    // Surface the underlying error to stderr (captured by Claude Desktop's
    // MCP log) so we can diagnose failures without losing detail to the
    // wire-shape trace_id collapse.
    eprintln!("mcp_v2 dailyos.read.account_status invoke failed: {err:?}");

    let trace_id = match &err {
        AbilityInvokeError::Surface(_) => "surface",
        AbilityInvokeError::Ability(_) => "ability",
        AbilityInvokeError::InvalidEnvelope => "invalid_envelope",
        AbilityInvokeError::ProvenanceTooLarge => "provenance_too_large",
        AbilityInvokeError::ProvenanceSerialize(_) => "provenance_serialize",
    };
    ToolError::Internal {
        trace_id: trace_id.to_string(),
    }
}

fn map_subject_resolution_db_error(err: DbError) -> ToolError {
    eprintln!("mcp_v2 dailyos.read.account_status subject resolution failed: {err:?}");
    ToolError::Internal {
        trace_id: "account_subject_resolution".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, Scope, ScopedName,
    };

    fn stub_actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("test-client".to_string()),
            conversation_handle: Some(OpaqueConversationHandle::new("conv".to_string())),
            tool_name: ScopedName::new("dailyos.read.account_status"),
            granted_scopes: vec![Scope::new("dailyos.read.account_status")],
        }
    }

    #[test]
    fn extract_subject_returns_trimmed_value() {
        let params = serde_json::json!({ "subject": "  acme  " });
        let subject = extract_subject(&params).unwrap();
        assert_eq!(subject, "acme");
    }

    #[test]
    fn extract_subject_rejects_missing() {
        let params = serde_json::json!({});
        let err = extract_subject(&params).unwrap_err();
        match err {
            ToolError::BadParams { detail } => assert!(detail.contains("subject")),
            other => panic!("expected BadParams, got {other:?}"),
        }
    }

    #[test]
    fn extract_subject_rejects_empty() {
        let params = serde_json::json!({ "subject": "   " });
        let err = extract_subject(&params).unwrap_err();
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn extract_subject_rejects_non_string() {
        let params = serde_json::json!({ "subject": 42 });
        let err = extract_subject(&params).unwrap_err();
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    fn account_candidate(id: &str, name: &str) -> AccountSubjectCandidate {
        AccountSubjectCandidate {
            id: id.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn account_subject_resolver_prefers_exact_id() {
        let accounts = vec![
            account_candidate("example-account", "Different Name"),
            account_candidate("other-account", "Example Account"),
        ];

        let resolved = resolve_account_subject_from_accounts("example-account", &accounts);

        match resolved {
            AccountSubjectResolution::Resolved(subject) => {
                assert_eq!(subject.entity_id, "example-account");
                assert_eq!(subject.display_label, "Different Name");
                assert_eq!(subject.resolution_kind, "account_id");
                assert_eq!(subject.match_confidence, "exact");
            }
            other => panic!("expected resolved subject, got {other:?}"),
        }
    }

    #[test]
    fn account_subject_resolver_matches_exact_name() {
        let accounts = vec![account_candidate("example-account", "Example Account")];

        let resolved = resolve_account_subject_from_accounts("example account", &accounts);

        match resolved {
            AccountSubjectResolution::Resolved(subject) => {
                assert_eq!(subject.entity_id, "example-account");
                assert_eq!(subject.display_label, "Example Account");
                assert_eq!(subject.resolution_kind, "account_name");
                assert_eq!(subject.match_confidence_score, 1.0);
            }
            other => panic!("expected resolved subject, got {other:?}"),
        }
    }

    #[test]
    fn account_subject_resolver_matches_normalized_slug() {
        let accounts = vec![account_candidate("example-account", "Example Account")];

        let resolved = resolve_account_subject_from_accounts("ExampleAccount", &accounts);

        match resolved {
            AccountSubjectResolution::Resolved(subject) => {
                assert_eq!(subject.entity_id, "example-account");
                assert_eq!(subject.display_label, "Example Account");
                assert_eq!(subject.resolution_kind, "normalized_slug");
                assert_eq!(subject.match_confidence, "high");
            }
            other => panic!("expected resolved subject, got {other:?}"),
        }
    }

    #[test]
    fn account_subject_resolver_returns_ambiguous_for_duplicate_normalized_matches() {
        let accounts = vec![
            account_candidate("example-account", "Example Account"),
            account_candidate("exampleaccount", "ExampleAccount"),
        ];

        let resolved = resolve_account_subject_from_accounts("Example Account", &accounts);

        match resolved {
            AccountSubjectResolution::Resolved(subject) => {
                assert_eq!(subject.entity_id, "example-account");
                assert_eq!(subject.resolution_kind, "account_name");
            }
            other => panic!("exact name should win before ambiguity, got {other:?}"),
        }

        let ambiguous = resolve_account_subject_from_accounts("Example-Account", &accounts);
        match ambiguous {
            AccountSubjectResolution::Ambiguous { candidates, .. } => {
                assert_eq!(candidates.len(), 2);
            }
            other => panic!("expected ambiguous subject, got {other:?}"),
        }
    }

    #[test]
    fn account_status_subject_resolution_edges() {
        let accounts = vec![
            account_candidate("example-account", "Example Account"),
            account_candidate("exampleaccount", "ExampleAccount"),
            account_candidate("another-account", "Another Account"),
        ];

        let exact_id = resolve_account_subject_from_accounts("another-account", &accounts);
        assert!(matches!(
            exact_id,
            AccountSubjectResolution::Resolved(ResolvedAccountSubject {
                resolution_kind: "account_id",
                ..
            })
        ));

        let exact_name = resolve_account_subject_from_accounts("example account", &accounts);
        assert!(matches!(
            exact_name,
            AccountSubjectResolution::Resolved(ResolvedAccountSubject {
                resolution_kind: "account_name",
                ..
            })
        ));

        let normalized = resolve_account_subject_from_accounts("AnotherAccount", &accounts);
        assert!(matches!(
            normalized,
            AccountSubjectResolution::Resolved(ResolvedAccountSubject {
                resolution_kind: "normalized_slug",
                ..
            })
        ));

        let missing = resolve_account_subject_from_accounts("missing-account", &accounts);
        assert!(matches!(missing, AccountSubjectResolution::NotFound { .. }));

        let ambiguous = resolve_account_subject_from_accounts("Example-Account", &accounts);
        assert!(matches!(
            ambiguous,
            AccountSubjectResolution::Ambiguous { .. }
        ));
    }

    #[test]
    fn account_subject_not_found_response_uses_documented_shape() {
        let payload = account_subject_not_found_response("missing-account");

        assert_eq!(payload["schemaVersion"], 2);
        assert_eq!(payload["toolName"], "dailyos.read.account_status");
        assert_eq!(payload["status"], "not_found");
        assert_eq!(payload["resolution"]["resolvedEntityId"], Value::Null);
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("could not resolve"));
        assert_eq!(payload["provenance"]["rawClaimIdsIncluded"], false);
    }

    #[test]
    fn account_subject_resolution_is_attached_to_presented_payload() {
        let mut payload = present_account_status_response(
            "example-account",
            json!({
                "schemaVersion": 2,
                "subject": {
                    "kind": "account",
                    "id": "example-account",
                    "displayLabel": "account:example-account"
                },
                "facts": { "items": [] },
                "openLoops": { "items": [] },
                "relationships": { "items": [] },
                "touchpoints": { "items": [] },
                "recordEntries": { "items": [] },
                "trust": {
                    "aggregateBand": "unscored",
                    "sectionCaveats": {}
                },
                "sensitivity": "internal",
                "provenance": {
                    "sources": [],
                    "redactionApplied": false
                },
                "sections": {}
            }),
        );
        let subject = ResolvedAccountSubject {
            input: "Example Account".to_string(),
            entity_id: "example-account".to_string(),
            display_label: "Example Account".to_string(),
            resolution_kind: "normalized_slug",
            match_basis: "normalized_id_or_name",
            match_confidence: "high",
            match_confidence_score: 0.95,
        };

        attach_account_subject_resolution(&mut payload, &subject);

        assert_eq!(payload["resolution"]["resolvedEntityId"], "example-account");
        assert_eq!(payload["subject"]["input"], "Example Account");
        assert_eq!(payload["subject"]["displayLabel"], "Example Account");
        assert_eq!(payload["subject"]["resolutionKind"], "normalized_slug");
    }

    #[test]
    fn account_status_exec_briefing_fixture_uses_runtime_projection() {
        let envelope = serde_json::json!({
            "schemaVersion": 2,
            "subject": {
                "kind": "account",
                "id": "acct-1",
                "displayLabel": "account:acct-1"
            },
            "facts": {
                "items": [{
                    "claimId": "claim-1",
                    "fieldPath": "/commercial/posture",
                    "claimType": "account_status",
                    "renderedText": {
                        "text": "Renewal risk is elevated because procurement has not replied.",
                        "policy": {}
                    },
                    "trustBand": "likely_current",
                    "freshness": "current",
                    "sourceAsof": "2026-05-22T15:00:00Z",
                    "sensitivity": "internal",
                    "lifecycleState": "active",
                    "verificationState": "active",
                    "provenance": {
                        "sourceIds": ["claim_source:claim-1"]
                    }
                }]
            },
            "openLoops": {
                "items": [{
                    "openLoop": {
                        "id": "claim-2",
                        "subject": {
                            "entity_type": "account",
                            "entity_id": "acct-1"
                        },
                        "loop_kind": "renewal_follow_up",
                        "description": "Confirm commercial owner before renewal review.",
                        "status": "open",
                        "due_date": "2026-05-30",
                        "source_asof": "2026-05-22T15:00:00Z",
                        "claim_type": "open_loop"
                    },
                    "trustBand": "unscored",
                    "freshness": "unknown",
                    "provenance": {
                        "sourceIds": ["open_loop:claim-2"]
                    }
                }]
            },
            "relationships": {
                "items": [{
                    "edges": {
                        "items": [{
                            "edgeId": "stakeholder:person:person-2:raw-edge-source",
                            "edgeType": "stakeholder",
                            "relatedSubjectRef": { "person": "person-2" },
                            "relatedDisplayLabel": {
                                "text": "Commercial Sponsor",
                                "policy": {}
                            },
                            "observedAt": "2026-05-22T15:00:00Z",
                            "sourceAsof": "2026-05-22T15:00:00Z",
                            "trustBand": "likely_current",
                            "freshness": "current",
                            "provenance": {
                                "sourceIds": ["relationship_source:edge-1"]
                            }
                        }]
                    },
                    "participants": {
                        "items": [{
                            "subjectRef": { "person": "person-1" },
                            "displayLabel": {
                                "text": "Example Person",
                                "policy": {}
                            },
                            "role": {
                                "text": "Executive sponsor",
                                "policy": {}
                            },
                            "relationship": {
                                "text": "stakeholder",
                                "policy": {}
                            },
                            "normalizedTouchpointCount": 4,
                            "recentTouchpointIds": ["meeting-1", "meeting-2"],
                            "lastSeenAt": "2026-05-22T15:00:00Z",
                            "trustBand": "likely_current",
                            "freshness": "current",
                            "provenance": {
                                "sourceIds": ["relationship_source:person-1"]
                            }
                        }]
                    }
                }]
            },
            "touchpoints": {
                "items": [{
                    "upcoming": { "items": [] },
                    "recent": {
                        "items": [{
                            "meetingId": "meeting-1",
                            "kind": "meeting",
                            "when": "2026-05-22T15:00:00Z",
                            "inclusionReason": "attendee_match",
                            "trustBand": "unscored",
                            "freshness": "current",
                            "provenance": {
                                "sourceIds": ["touchpoint:meeting-1"]
                            }
                        }]
                    }
                }]
            },
            "recordEntries": {
                "items": [{
                    "claimId": "claim-3",
                    "claimType": "account_priority",
                    "recordedAt": "2026-05-22T15:00:00Z",
                    "renderedText": {
                        "text": "Prioritize the reliability recap before the next renewal review.",
                        "policy": {}
                    },
                    "trustBand": "likely_current",
                    "sensitivity": "internal",
                    "provenance": {
                        "sourceIds": ["record_source:claim-3"]
                    }
                }]
            },
            "trust": {
                "aggregateBand": "likely_current",
                "sectionCaveats": {}
            },
            "sensitivity": "internal",
            "provenance": {
                "sources": [
                    {
                        "id": "claim_source:claim-1",
                        "label": "meeting",
                        "sourceType": "meeting",
                        "asOf": "2026-05-22T15:00:00Z",
                        "redacted": false
                    },
                    {
                        "id": "open_loop:claim-2",
                        "label": "open_loop",
                        "sourceType": "open_loop",
                        "asOf": "2026-05-22T15:00:00Z",
                        "redacted": false
                    },
                    {
                        "id": "relationship_source:person-1",
                        "label": "meeting participation",
                        "sourceType": "meeting_attendees",
                        "asOf": "2026-05-22T15:00:00Z",
                        "redacted": false
                    },
                    {
                        "id": "relationship_source:edge-1",
                        "label": "account stakeholder",
                        "sourceType": "account_stakeholders",
                        "asOf": "2026-05-22T15:00:00Z",
                        "redacted": false
                    },
                    {
                        "id": "touchpoint:meeting-1",
                        "label": "Meeting (redacted)",
                        "sourceType": "meeting",
                        "asOf": "2026-05-22T15:00:00Z",
                        "redacted": true
                    },
                    {
                        "id": "record_source:claim-3",
                        "label": "claim",
                        "sourceType": "claim",
                        "asOf": "2026-05-22T15:00:00Z",
                        "redacted": false
                    }
                ],
                "redactionApplied": false
            },
            "sections": {}
        });

        let payload = present_account_status_response("acct-1", envelope);
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("Renewal risk is elevated"));
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("Confirm commercial owner"));
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("Example Person"));
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("Commercial Sponsor"));
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("Touchpoint evidence"));
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("Prioritize the reliability recap"));
        assert_eq!(payload["schemaVersion"], 2);
        assert_eq!(payload["toolName"], "dailyos.read.account_status");
        assert_eq!(payload["status"], "ok");
        assert_eq!(
            payload["assessment"]["facts"][0]["sourceRefs"][0],
            "source_1"
        );
        assert_eq!(
            payload["assessment"]["relationships"][0]["sourceRefs"][0],
            "source_3"
        );
        assert_eq!(
            payload["assessment"]["touchpoints"][0]["sourceRefs"][0],
            "source_5"
        );
        assert_eq!(
            payload["assessment"]["recordEntries"][0]["sourceRefs"][0],
            "source_6"
        );
        assert_eq!(
            payload["assessment"]["priorities"][0]["text"],
            "Prioritize the reliability recap before the next renewal review."
        );
        assert_eq!(payload["provenance"]["sources"][0]["id"], "source_1");
        assert_eq!(payload["provenance"]["rawClaimIdsIncluded"], false);
        assert_eq!(payload["truncation"]["recordEntries"]["renderedCount"], 1);

        let serialized = serde_json::to_string(&payload).unwrap();
        assert!(!serialized.contains("claim_source:claim-1"));
        assert!(!serialized.contains("relationship_source:person-1"));
        assert!(!serialized.contains("relationship_source:edge-1"));
        assert!(!serialized.contains("touchpoint:meeting-1"));
        assert!(!serialized.contains("record_source:claim-3"));
        assert!(!serialized.contains("raw-edge-source"));
        assert!(!serialized.contains("meeting-1"));
        assert!(!serialized.contains("claim-3"));
        assert!(!serialized.contains("\"claimId\""));
    }

    #[test]
    fn account_status_presenter_keeps_edge_only_relationships_readable() {
        let envelope = json!({
            "schemaVersion": 2,
            "subject": {
                "kind": "account",
                "id": "acct-1",
                "displayLabel": "account:acct-1"
            },
            "facts": { "items": [] },
            "openLoops": { "items": [] },
            "relationships": {
                "items": [{
                    "edges": {
                        "items": [{
                            "edgeId": "stakeholder:person:person-1:raw-source",
                            "edgeType": "stakeholder",
                            "relatedSubjectRef": { "person": "person-1" },
                            "relatedDisplayLabel": {
                                "text": "Commercial Sponsor",
                                "policy": {}
                            },
                            "trustBand": "use_with_caution",
                            "freshness": "unknown",
                            "provenance": {
                                "sourceIds": ["relationship_source:edge-1"]
                            }
                        }]
                    },
                    "participants": { "items": [] }
                }]
            },
            "trust": {
                "aggregateBand": "use_with_caution",
                "sectionCaveats": {}
            },
            "sensitivity": "internal",
            "provenance": {
                "sources": [{
                    "id": "relationship_source:edge-1",
                    "label": "account stakeholder",
                    "sourceType": "account_stakeholders",
                    "redacted": false
                }],
                "redactionApplied": false
            },
            "sections": {}
        });

        let payload = present_account_status_response_with_context(
            "acct-1",
            envelope,
            Some("Example Account"),
            Some("00000000-0000-0000-0000-000000000001"),
            Some("get_provenance"),
        );

        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("DailyOS account briefing for Example Account"));
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("Commercial Sponsor"));
        assert_eq!(payload["assessment"]["relationships"][0]["kind"], "edge");
        assert_eq!(
            payload["assessment"]["relationships"][0]["sourceRefs"][0],
            "source_1"
        );
        assert_eq!(
            payload["provenanceHandle"]["invocationId"],
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(
            payload["provenanceHandle"]["invocation_id"],
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(payload["provenanceHandle"]["detailAvailable"], true);
        assert_eq!(payload["provenanceHandle"]["detailTool"], "get_provenance");
        assert_eq!(
            payload["provenanceHandle"]["detailParams"]["invocation_id"],
            "00000000-0000-0000-0000-000000000001"
        );

        let serialized = serde_json::to_string(&payload).unwrap();
        assert!(!serialized.contains("relationship_source:edge-1"));
        assert!(!serialized.contains("raw-source"));
        assert!(!serialized.contains("person-1"));
    }

    #[test]
    fn account_status_presenter_reports_presenter_truncation() {
        let facts = (0..9)
            .map(|index| {
                json!({
                    "claimId": format!("claim-{index}"),
                    "claimType": "account_status",
                    "renderedText": {
                        "text": format!("Runtime fact {index}."),
                        "policy": {}
                    },
                    "trustBand": "likely_current",
                    "freshness": "current",
                    "provenance": { "sourceIds": [] }
                })
            })
            .collect::<Vec<_>>();
        let envelope = json!({
            "schemaVersion": 2,
            "subject": {
                "kind": "account",
                "id": "acct-1",
                "displayLabel": "Example Account"
            },
            "facts": { "items": facts },
            "openLoops": { "items": [] },
            "relationships": { "items": [] },
            "touchpoints": { "items": [] },
            "recordEntries": { "items": [] },
            "trust": {
                "aggregateBand": "likely_current",
                "sectionCaveats": {}
            },
            "sensitivity": "internal",
            "provenance": {
                "sources": [],
                "redactionApplied": false
            },
            "sections": {}
        });

        let payload = present_account_status_response("acct-1", envelope);

        assert_eq!(payload["assessment"]["facts"].as_array().unwrap().len(), 8);
        assert_eq!(payload["truncation"]["facts"]["renderedCount"], 8);
        assert_eq!(payload["truncation"]["facts"]["rawCount"], 9);
        assert_eq!(payload["truncation"]["facts"]["omittedCount"], 1);
        assert_eq!(payload["truncation"]["facts"]["projectionTruncated"], true);
        assert_eq!(payload["truncation"]["facts"]["presenterTruncated"], true);
    }

    #[test]
    fn account_status_presenter_stays_within_mcp_payload_budget_for_large_runtime_text() {
        let long_text = "This claim has useful detail. ".repeat(400);
        let facts = (0..30)
            .map(|index| {
                json!({
                    "claimId": format!("claim-{index}"),
                    "claimType": "account_status",
                    "renderedText": {
                        "text": format!("{long_text} fact {index}."),
                        "policy": {}
                    },
                    "trustBand": "likely_current",
                    "freshness": "current",
                    "provenance": { "sourceIds": [] }
                })
            })
            .collect::<Vec<_>>();
        let envelope = json!({
            "schemaVersion": 2,
            "subject": {
                "kind": "account",
                "id": "acct-1",
                "displayLabel": "Example Account"
            },
            "facts": { "items": facts },
            "openLoops": { "items": [] },
            "relationships": { "items": [] },
            "touchpoints": { "items": [] },
            "recordEntries": { "items": [] },
            "trust": {
                "aggregateBand": "likely_current",
                "sectionCaveats": {}
            },
            "sensitivity": "internal",
            "provenance": {
                "sources": [],
                "redactionApplied": false
            },
            "sections": {}
        });

        let payload = present_account_status_response("acct-1", envelope);
        let serialized = serde_json::to_vec(&payload).unwrap();

        assert!(
            serialized.len()
                <= crate::services::mcp_v2::runtime_projection::MAX_MCP_PROJECTED_PAYLOAD_BYTES,
            "MCP payload should fit under the 64 KiB local stdio tool budget"
        );
        assert!(String::from_utf8(serialized)
            .unwrap()
            .contains("[truncated]"));
    }

    #[test]
    fn account_status_presenter_uses_documented_status_for_empty_response() {
        let envelope = json!({
            "schemaVersion": 2,
            "subject": {
                "kind": "account",
                "id": "acct-1",
                "displayLabel": "Example Account"
            },
            "facts": { "items": [] },
            "openLoops": { "items": [] },
            "relationships": { "items": [] },
            "touchpoints": { "items": [] },
            "recordEntries": { "items": [] },
            "trust": {
                "aggregateBand": "unscored",
                "sectionCaveats": {}
            },
            "sensitivity": "internal",
            "provenance": {
                "sources": [],
                "redactionApplied": false
            },
            "sections": {}
        });

        let payload = present_account_status_response("acct-1", envelope);

        assert_eq!(payload["status"], "not_found");
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("does not yet have claim-backed account intelligence"));
    }

    #[test]
    fn account_status_presenter_surfaces_relationship_partial_failure() {
        let envelope = json!({
            "schemaVersion": 2,
            "subject": {
                "kind": "account",
                "id": "acct-1",
                "displayLabel": "Example Account"
            },
            "facts": { "items": [] },
            "openLoops": { "items": [] },
            "relationships": {
                "items": [{
                    "edges": { "items": [] },
                    "participants": { "items": [] },
                    "emptyReason": {
                        "partial_failure": {
                            "advisory": "relationships reader unavailable"
                        }
                    }
                }]
            },
            "trust": {
                "aggregateBand": "unscored",
                "sectionCaveats": {
                    "relationships": "relationships reader unavailable"
                }
            },
            "sensitivity": "internal",
            "provenance": {
                "sources": [],
                "redactionApplied": false
            },
            "sections": {
                "relationships": {
                    "kind": "empty",
                    "reason": {
                        "partial_failure": {
                            "advisory": "relationships reader unavailable"
                        }
                    }
                }
            }
        });

        let payload = present_account_status_response("acct-1", envelope);

        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("could not read requested intelligence"));
        assert!(!payload["answer"]
            .as_str()
            .unwrap()
            .contains("does not yet have claim-backed account intelligence"));
        assert_eq!(payload["status"], "unavailable");
        assert_eq!(
            payload["assessment"]["caveats"][0]["text"],
            "relationships reader unavailable"
        );
    }

    #[test]
    fn account_status_presenter_marks_touchpoint_partial_failure() {
        let envelope = json!({
            "schemaVersion": 2,
            "subject": {
                "kind": "account",
                "id": "acct-1",
                "displayLabel": "Example Account"
            },
            "facts": { "items": [] },
            "openLoops": { "items": [] },
            "relationships": { "items": [] },
            "touchpoints": {
                "items": [{
                    "upcoming": { "items": [] },
                    "recent": { "items": [] },
                    "emptyReason": {
                        "partial_failure": {
                            "advisory": "touchpoints reader unavailable"
                        }
                    }
                }]
            },
            "recordEntries": { "items": [] },
            "trust": {
                "aggregateBand": "unscored",
                "sectionCaveats": {}
            },
            "sensitivity": "internal",
            "provenance": {
                "sources": [],
                "redactionApplied": false
            },
            "sections": {
                "touchpoints": {
                    "kind": "empty",
                    "reason": "no_relevant_touchpoints"
                }
            }
        });

        let payload = present_account_status_response("acct-1", envelope);

        assert_eq!(payload["status"], "unavailable");
        assert!(payload["answer"]
            .as_str()
            .unwrap()
            .contains("touchpoints reader unavailable"));
        assert_eq!(
            payload["assessment"]["caveats"][0]["text"],
            "touchpoints reader unavailable"
        );
    }

    #[test]
    fn stub_actor_smoke() {
        // Ensures the actor projection compiles against the current
        // McpActor enum shape (a tripwire if the variant fields change).
        let actor = stub_actor();
        match &actor {
            McpActor::Client {
                client_id,
                tool_name,
                granted_scopes,
                ..
            } => {
                assert_eq!(client_id.as_str(), "test-client");
                assert_eq!(tool_name.as_str(), "dailyos.read.account_status");
                assert_eq!(granted_scopes.len(), 1);
            }
        }
    }
}
