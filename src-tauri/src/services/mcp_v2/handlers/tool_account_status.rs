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

use abilities_runtime::abilities::registry::{AbilityRegistry, McpExposure};
use abilities_runtime::abilities::tracer::NOOP_ABILITY_TRACER;
use serde_json::{json, Map, Value};

use crate::bridges::types::{
    invoke_registry_json_for_actor, AbilityInvokeError, RequestScopedInvocation,
    BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
};
use crate::bridges::{BridgeActor, BridgeSurface};
use crate::services::context::{
    attach_live_workspace_readers, ClaimDismissalSurface, ExternalClients, ServiceContext,
    SystemClock, SystemRng,
};
use crate::services::mcp_v2::actor_policy::{project_actor, ToolGrant, ToolRateLimit};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};

const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));

/// Registered ability name in the abilities-runtime registry.
const ABILITY_NAME: &str = "get_entity_intelligence";

/// `EntityIntelligenceInput` schema version pinned by the ability contract.
const ENTITY_INTELLIGENCE_SCHEMA_VERSION: u32 = 2;
const ACCOUNT_STATUS_RESPONSE_SCHEMA_VERSION: u32 = 1;
const MAX_ASSESSMENT_ITEMS: usize = 8;

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

    fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
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

        let subject = extract_subject(&params)?;

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
                "entityId": subject.clone(),
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
            Ok(present_account_status_response(&subject, response.data))
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
    // Phase-A passthrough: treat `subject` as the account_id directly. The
    // follow-on subject resolver replaces this with slug/account resolution.
    Ok(subject.to_string())
}

pub fn present_account_status_response(subject: &str, envelope: Value) -> Value {
    present_account_status_response_with_label(subject, envelope, None)
}

pub fn present_account_status_response_with_label(
    subject: &str,
    envelope: Value,
    display_label_override: Option<&str>,
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

    let (provenance, source_id_map) = build_provenance_summary(&envelope);
    let facts = collect_fact_summaries(&envelope, &source_id_map);
    let open_loops = collect_open_loop_summaries(&envelope, &source_id_map);
    let relationships = collect_relationship_summaries(&envelope, &source_id_map);
    let answer = build_account_status_answer(
        &label,
        &facts,
        &open_loops,
        &relationships,
        &envelope,
        &provenance,
    );

    json!({
        "schemaVersion": ACCOUNT_STATUS_RESPONSE_SCHEMA_VERSION,
        "surface": "dailyos.read.account_status",
        "producer": ABILITY_NAME,
        "subject": subject_value,
        "answer": answer,
        "assessment": {
            "facts": facts,
            "openLoops": open_loops,
            "relationships": relationships,
        },
        "trust": envelope.get("trust").cloned().unwrap_or(Value::Null),
        "sensitivity": envelope.get("sensitivity").cloned().unwrap_or(Value::Null),
        "provenance": provenance,
        "sections": envelope.get("sections").cloned().unwrap_or_else(|| json!({})),
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

fn build_provenance_summary(envelope: &Value) -> (Value, BTreeMap<String, String>) {
    let mut source_id_map = BTreeMap::new();
    let mut sources = Vec::new();

    if let Some(raw_sources) = envelope
        .pointer("/provenance/sources")
        .and_then(Value::as_array)
    {
        for (index, source) in raw_sources.iter().enumerate() {
            let display_id = format!("source_{}", index + 1);
            if let Some(raw_id) = source.get("id").and_then(Value::as_str) {
                source_id_map.insert(raw_id.to_string(), display_id.clone());
            }

            let mut projected = Map::new();
            projected.insert("id".to_string(), Value::String(display_id));
            insert_string_or_clone(&mut projected, "label", source, "/label");
            insert_string_or_clone(&mut projected, "sourceType", source, "/sourceType");
            insert_string_or_clone(&mut projected, "asOf", source, "/asOf");
            if let Some(redacted) = source.get("redacted").and_then(Value::as_bool) {
                projected.insert("redacted".to_string(), Value::Bool(redacted));
            }
            sources.push(Value::Object(projected));
        }
    }

    let redaction_applied = envelope
        .pointer("/provenance/redactionApplied")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    (
        json!({
            "sources": sources,
            "redactionApplied": redaction_applied,
            "rawClaimIdsIncluded": false,
        }),
        source_id_map,
    )
}

fn collect_fact_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    envelope
        .pointer("/facts/items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| fact_summary(item, source_id_map))
                .take(MAX_ASSESSMENT_ITEMS)
                .collect()
        })
        .unwrap_or_default()
}

fn fact_summary(item: &Value, source_id_map: &BTreeMap<String, String>) -> Option<Value> {
    let text = string_at(item, "/renderedText/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;

    let mut summary = Map::new();
    summary.insert("text".to_string(), Value::String(text));
    insert_string_or_clone(&mut summary, "fieldPath", item, "/fieldPath");
    insert_string_or_clone(&mut summary, "claimType", item, "/claimType");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_string_or_clone(&mut summary, "sourceAsOf", item, "/sourceAsof");
    insert_string_or_clone(&mut summary, "sensitivity", item, "/sensitivity");
    insert_string_or_clone(&mut summary, "lifecycleState", item, "/lifecycleState");
    insert_string_or_clone(
        &mut summary,
        "verificationState",
        item,
        "/verificationState",
    );
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn collect_open_loop_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    envelope
        .pointer("/openLoops/items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| open_loop_summary(item, source_id_map))
                .take(MAX_ASSESSMENT_ITEMS)
                .collect()
        })
        .unwrap_or_default()
}

fn collect_relationship_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    envelope
        .pointer("/relationships/items")
        .and_then(Value::as_array)
        .map(|bundles| {
            let mut summaries = Vec::new();
            for bundle in bundles {
                if let Some(participants) = bundle
                    .pointer("/participants/items")
                    .and_then(Value::as_array)
                {
                    summaries.extend(
                        participants.iter().filter_map(|item| {
                            relationship_participant_summary(item, source_id_map)
                        }),
                    );
                }
                if let Some(edges) = bundle.pointer("/edges/items").and_then(Value::as_array) {
                    summaries.extend(
                        edges
                            .iter()
                            .filter_map(|item| relationship_edge_summary(item, source_id_map)),
                    );
                }
            }
            summaries.truncate(MAX_ASSESSMENT_ITEMS);
            summaries
        })
        .unwrap_or_default()
}

fn relationship_participant_summary(
    item: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Option<Value> {
    let display_label = string_at(item, "/displayLabel/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;

    let mut summary = Map::new();
    summary.insert("kind".to_string(), Value::String("participant".to_string()));
    summary.insert("displayLabel".to_string(), Value::String(display_label));
    if let Some(role) = string_at(item, "/role/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())
    {
        summary.insert("role".to_string(), Value::String(role));
    }
    if let Some(relationship) = string_at(item, "/relationship/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())
    {
        summary.insert("relationship".to_string(), Value::String(relationship));
    }
    if let Some(count) = item
        .get("normalizedTouchpointCount")
        .and_then(Value::as_u64)
    {
        summary.insert(
            "normalizedTouchpointCount".to_string(),
            Value::Number(count.into()),
        );
    }
    insert_string_or_clone(&mut summary, "lastSeenAt", item, "/lastSeenAt");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn relationship_edge_summary(
    item: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Option<Value> {
    let relationship = string_at(item, "/edgeType")
        .map(compact_text)
        .filter(|value| !value.is_empty())
        .map(|value| relationship_label_for_edge_type(&value))
        .unwrap_or_else(|| "Relationship".to_string());
    let mut summary = Map::new();
    summary.insert("kind".to_string(), Value::String("edge".to_string()));
    summary.insert("relationship".to_string(), Value::String(relationship));
    if let Some(display_label) = string_at(item, "/relatedDisplayLabel/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())
    {
        summary.insert("displayLabel".to_string(), Value::String(display_label));
    }
    if let Some(related_entity_type) = relationship_subject_type(item.get("relatedSubjectRef")) {
        summary.insert(
            "relatedEntityType".to_string(),
            Value::String(related_entity_type.to_string()),
        );
    }
    insert_string_or_clone(&mut summary, "inclusionReason", item, "/inclusionReason");
    insert_string_or_clone(&mut summary, "observedAt", item, "/observedAt");
    insert_string_or_clone(&mut summary, "sourceAsOf", item, "/sourceAsof");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn relationship_label_for_edge_type(edge_type: &str) -> String {
    match edge_type {
        "hierarchy_parent" => "Parent relationship",
        "hierarchy_child" => "Child relationship",
        "stakeholder" => "Stakeholder",
        "member" => "Member",
        "meeting_subject" => "Meeting subject",
        "meeting_attendance" => "Meeting attendance",
        "meeting_link" => "Meeting link",
        "person_relationship" => "Person relationship",
        _ => "Relationship",
    }
    .to_string()
}

fn relationship_subject_type(subject_ref: Option<&Value>) -> Option<&'static str> {
    subject_ref.and_then(|value| {
        if let Some(value) = value.as_str() {
            return value
                .split_once(':')
                .map(|(entity_type, _)| entity_type)
                .or(Some(value))
                .and_then(known_entity_type);
        }
        value
            .as_object()
            .and_then(|object| object.keys().find_map(|key| known_entity_type(key)))
    })
}

fn known_entity_type(entity_type: &str) -> Option<&'static str> {
    match entity_type {
        "account" => Some("account"),
        "project" => Some("project"),
        "person" => Some("person"),
        "meeting" => Some("meeting"),
        _ => None,
    }
}

fn open_loop_summary(item: &Value, source_id_map: &BTreeMap<String, String>) -> Option<Value> {
    let open_loop = item.get("openLoop").unwrap_or(item);
    let description = string_at(open_loop, "/description")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;

    let mut summary = Map::new();
    summary.insert("description".to_string(), Value::String(description));
    insert_string_or_clone(&mut summary, "loopKind", open_loop, "/loop_kind");
    insert_string_or_clone(&mut summary, "owner", open_loop, "/owner");
    insert_string_or_clone(&mut summary, "dueDate", open_loop, "/due_date");
    insert_string_or_clone(&mut summary, "status", open_loop, "/status");
    insert_string_or_clone(&mut summary, "sourceAsOf", open_loop, "/source_asof");
    insert_string_or_clone(&mut summary, "claimType", open_loop, "/claim_type");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn build_account_status_answer(
    label: &str,
    facts: &[Value],
    open_loops: &[Value],
    relationships: &[Value],
    envelope: &Value,
    provenance: &Value,
) -> String {
    if facts.is_empty() && open_loops.is_empty() && relationships.is_empty() {
        if let Some(advisory) = relationship_partial_failure_advisory(envelope) {
            return format!(
                "DailyOS could not read relationship intelligence for {label}: {advisory}."
            );
        }
        return format!("DailyOS does not yet have claim-backed account intelligence for {label}.");
    }

    let mut lines = vec![format!("DailyOS account briefing for {label}.")];

    if !facts.is_empty() {
        lines.push(String::new());
        lines.push("Assessment:".to_string());
        for fact in facts.iter().take(5) {
            if let Some(text) = string_at(fact, "/text") {
                lines.push(format!("- {}{}", text, evidence_suffix(fact)));
            }
        }
    }

    if !open_loops.is_empty() {
        lines.push(String::new());
        lines.push("Open loops:".to_string());
        for open_loop in open_loops.iter().take(5) {
            if let Some(description) = string_at(open_loop, "/description") {
                lines.push(format!("- {}{}", description, open_loop_suffix(open_loop)));
            }
        }
    }

    if !relationships.is_empty() {
        let participants = relationships
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

        let edges = relationships
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

    let source_count = provenance
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
    envelope
        .pointer("/relationships/items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|bundle| {
            bundle
                .get("emptyReason")
                .and_then(|reason| reason.get("partial_failure"))
                .and_then(|failure| failure.get("advisory"))
                .and_then(Value::as_str)
                .map(compact_text)
                .filter(|value| !value.is_empty())
        })
}

fn evidence_suffix(item: &Value) -> String {
    let mut parts = Vec::new();
    push_humanized_part(&mut parts, "trust", item, "/trustBand");
    push_humanized_part(&mut parts, "freshness", item, "/freshness");
    if let Some(source_as_of) = string_at(item, "/sourceAsOf") {
        parts.push(format!("as of {source_as_of}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}

fn open_loop_suffix(item: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(status) = string_at(item, "/status") {
        parts.push(format!("status: {}", humanize_token(status)));
    }
    if let Some(due_date) = string_at(item, "/dueDate") {
        parts.push(format!("due {due_date}"));
    }
    push_humanized_part(&mut parts, "trust", item, "/trustBand");
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}

fn push_humanized_part(parts: &mut Vec<String>, label: &str, item: &Value, pointer: &str) {
    if let Some(value) = string_at(item, pointer) {
        parts.push(format!("{label}: {}", humanize_token(value)));
    }
}

fn insert_string_or_clone(
    target: &mut Map<String, Value>,
    key: &str,
    source: &Value,
    pointer: &str,
) {
    if let Some(value) = source.pointer(pointer).filter(|value| !value.is_null()) {
        target.insert(key.to_string(), value.clone());
    }
}

fn insert_source_refs(
    target: &mut Map<String, Value>,
    item: &Value,
    source_id_map: &BTreeMap<String, String>,
) {
    let refs = item
        .pointer("/provenance/sourceIds")
        .and_then(Value::as_array)
        .map(|source_ids| {
            source_ids
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|raw_id| source_id_map.get(raw_id))
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !refs.is_empty() {
        target.insert("sourceRefs".to_string(), Value::Array(refs));
    }
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn humanize_token(value: &str) -> String {
    value.replace('_', " ")
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

    #[test]
    fn account_status_presenter_returns_prose_and_display_provenance() {
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
        assert_eq!(
            payload["assessment"]["facts"][0]["sourceRefs"][0],
            "source_1"
        );
        assert_eq!(
            payload["assessment"]["relationships"][0]["sourceRefs"][0],
            "source_3"
        );
        assert_eq!(payload["provenance"]["sources"][0]["id"], "source_1");
        assert_eq!(payload["provenance"]["rawClaimIdsIncluded"], false);

        let serialized = serde_json::to_string(&payload).unwrap();
        assert!(!serialized.contains("claim_source:claim-1"));
        assert!(!serialized.contains("relationship_source:person-1"));
        assert!(!serialized.contains("relationship_source:edge-1"));
        assert!(!serialized.contains("raw-edge-source"));
        assert!(!serialized.contains("meeting-1"));
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

        let payload =
            present_account_status_response_with_label("acct-1", envelope, Some("Example Account"));

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

        let serialized = serde_json::to_string(&payload).unwrap();
        assert!(!serialized.contains("relationship_source:edge-1"));
        assert!(!serialized.contains("raw-source"));
        assert!(!serialized.contains("person-1"));
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
            .contains("could not read relationship intelligence"));
        assert!(!payload["answer"]
            .as_str()
            .unwrap()
            .contains("does not yet have claim-backed account intelligence"));
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
