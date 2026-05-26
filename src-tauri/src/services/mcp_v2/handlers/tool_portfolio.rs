//! `dailyos.read.portfolio_attention` MCP tool handler.
//!
//! Wraps the claim-backed `portfolio_attention` ability. The handler is
//! intentionally thin: actor projection, runtime invocation, and host-safe
//! response presentation. It does not query SQLite directly.

use abilities_runtime::abilities::registry::{AbilityRegistry, McpExposure};
use abilities_runtime::abilities::tracer::NOOP_ABILITY_TRACER;
use serde_json::{json, Value};

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
use crate::services::mcp_v2::runtime_projection::{compact_text, humanize_token, string_at};

const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));
const ABILITY_NAME: &str = "portfolio_attention";
const TOOL_NAME: &str = "dailyos.read.portfolio_attention";
const PORTFOLIO_ATTENTION_SCHEMA_VERSION: u32 = 1;
const PORTFOLIO_ATTENTION_RESPONSE_SCHEMA_VERSION: u32 = 1;
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 25;
const MAX_ANSWER_ITEMS: usize = 5;

pub struct PortfolioAttentionHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
}

impl PortfolioAttentionHandler {
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

    pub fn from_runtime(
        description: ToolDescription,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, &'static str> {
        let registry = AbilityRegistry::global_checked()
            .map_err(|_| "ability registry violations present at startup")?;
        Ok(Self::new(description, registry, runtime))
    }
}

impl McpToolHandler for PortfolioAttentionHandler {
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
        let input = portfolio_attention_input(&params)?;

        self.runtime.block_on(async {
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let services = attach_live_workspace_readers(
                ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR_LABEL),
            );

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
                input,
            )
            .await
            .map_err(map_invoke_error)?;
            let invocation_id = response.invocation_id.0.to_string();
            Ok(present_portfolio_attention_response(
                response.data,
                response.rendered_provenance.value,
                Some(&invocation_id),
            ))
        })
    }
}

fn portfolio_attention_input(params: &Value) -> Result<Value, ToolError> {
    let limit = extract_limit(params)?;
    let entity_types = extract_entity_types(params)?;
    let cursor = extract_cursor(params)?;
    let mut input = json!({
        "schemaVersion": PORTFOLIO_ATTENTION_SCHEMA_VERSION,
        "pageSize": limit,
    });
    if let Some(entity_types) = entity_types {
        input["entityTypes"] = Value::Array(entity_types.into_iter().map(Value::String).collect());
    }
    if let Some(cursor) = cursor {
        input["cursor"] = Value::String(cursor);
    }
    Ok(input)
}

fn extract_limit(params: &Value) -> Result<u32, ToolError> {
    let Some(value) = params.get("limit").or_else(|| params.get("pageSize")) else {
        return Ok(DEFAULT_LIMIT);
    };
    let parsed = match value {
        Value::Number(number) => number.as_u64().and_then(|value| u32::try_from(value).ok()),
        Value::String(raw) => raw.trim().parse::<u32>().ok(),
        Value::Null => Some(DEFAULT_LIMIT),
        _ => None,
    }
    .ok_or_else(|| ToolError::BadParams {
        detail: "limit must be a positive integer".to_string(),
    })?;
    if parsed == 0 {
        Ok(DEFAULT_LIMIT)
    } else if parsed > MAX_LIMIT {
        Err(ToolError::BadParams {
            detail: format!("limit must be <= {MAX_LIMIT}"),
        })
    } else {
        Ok(parsed)
    }
}

fn extract_cursor(params: &Value) -> Result<Option<String>, ToolError> {
    let Some(value) = params.get("cursor") else {
        return Ok(None);
    };
    match value {
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Value::Null => Ok(None),
        _ => Err(ToolError::BadParams {
            detail: "cursor must be a string".to_string(),
        }),
    }
}

fn extract_entity_types(params: &Value) -> Result<Option<Vec<String>>, ToolError> {
    let Some(value) = params
        .get("entityTypes")
        .or_else(|| params.get("entity_types"))
    else {
        return Ok(None);
    };
    let values = match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(|value| value.trim().to_ascii_lowercase())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| ToolError::BadParams {
                        detail: "entityTypes must be an array of strings".to_string(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::String(raw) => raw
            .split(',')
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>(),
        Value::Null => return Ok(None),
        _ => {
            return Err(ToolError::BadParams {
                detail: "entityTypes must be an array of strings".to_string(),
            });
        }
    };
    if values.is_empty() {
        return Ok(None);
    }
    for value in &values {
        if !matches!(value.as_str(), "account" | "project" | "person" | "meeting") {
            return Err(ToolError::BadParams {
                detail: format!("unsupported entityType `{value}`"),
            });
        }
    }
    Ok(Some(values))
}

pub fn present_portfolio_attention_response(
    portfolio: Value,
    rendered_provenance: Value,
    invocation_id: Option<&str>,
) -> Value {
    let items = portfolio
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let status = if items.is_empty() { "empty" } else { "ok" };
    let answer = build_portfolio_attention_answer(&items);
    let attention_items = items.iter().map(project_attention_item).collect::<Vec<_>>();

    json!({
        "schemaVersion": PORTFOLIO_ATTENTION_RESPONSE_SCHEMA_VERSION,
        "toolName": TOOL_NAME,
        "surface": TOOL_NAME,
        "producer": ABILITY_NAME,
        "status": status,
        "answer": answer,
        "attentionItems": attention_items,
        "pagination": {
            "nextCursor": string_at(&portfolio, "/nextCursor"),
            "totalHint": portfolio.get("totalHint").cloned().unwrap_or(Value::Null),
            "cursorState": portfolio.get("cursorState").cloned().unwrap_or(Value::Null),
        },
        "trust": portfolio_trust_summary(&items),
        "provenance": {
            "invocationId": invocation_id,
            "generatedAt": string_at(&portfolio, "/generatedAt"),
            "sources": collect_sources(&items),
            "rendered": rendered_provenance,
            "rawClaimIdsIncluded": false,
            "rawProvenanceIncluded": false
        },
        "sourceEnvelope": {
            "rawEnvelopeIncluded": false,
            "producer": ABILITY_NAME
        }
    })
}

fn build_portfolio_attention_answer(items: &[Value]) -> String {
    if items.is_empty() {
        return "No claim-backed portfolio attention items were found for the current filters."
            .to_string();
    }
    let mut lines = vec![format!(
        "{} portfolio attention item(s) are currently ranked by claims, trust, freshness, open loops, and salience.",
        items.len()
    )];
    for item in items.iter().take(MAX_ANSWER_ITEMS) {
        let rank = item.get("rank").and_then(Value::as_u64).unwrap_or(0);
        let label = subject_label(item);
        let score = item
            .pointer("/score/total")
            .and_then(Value::as_f64)
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "unscored".to_string());
        let reason = item
            .get("reasons")
            .and_then(Value::as_array)
            .and_then(|reasons| reasons.first())
            .and_then(|reason| reason.get("summary"))
            .and_then(Value::as_str)
            .map(compact_text)
            .unwrap_or_else(|| "attention signal present".to_string());
        lines.push(format!("{rank}. {label} (score {score}) - {reason}"));
    }
    lines.join("\n")
}

fn project_attention_item(item: &Value) -> Value {
    let evidence = item
        .get("evidence")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|evidence| {
                    json!({
                        "text": string_at(evidence, "/text").map(compact_text),
                        "claimType": string_at(evidence, "/claimType").map(humanize_token),
                        "sourceType": string_at(evidence, "/sourceType").map(humanize_token),
                        "sourceAsOf": string_at(evidence, "/sourceAsof").or_else(|| string_at(evidence, "/sourceAsOf")),
                        "observedAt": string_at(evidence, "/observedAt"),
                        "trustBand": string_at(evidence, "/trustBand"),
                        "salience": evidence.get("salience").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!({
        "rank": item.get("rank").cloned().unwrap_or(Value::Null),
        "subject": item.get("subject").cloned().unwrap_or(Value::Null),
        "score": item.get("score").cloned().unwrap_or(Value::Null),
        "trustBand": item.get("trustBand").cloned().unwrap_or(Value::Null),
        "reasons": item.get("reasons").cloned().unwrap_or_else(|| json!([])),
        "evidence": evidence,
    })
}

fn portfolio_trust_summary(items: &[Value]) -> Value {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for item in items {
        if let Some(band) = string_at(item, "/trustBand") {
            *counts.entry(band.to_string()).or_default() += 1;
        }
    }
    json!({
        "itemCount": items.len(),
        "bands": counts,
    })
}

fn collect_sources(items: &[Value]) -> Value {
    let mut sources = Vec::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for item in items {
        let Some(evidence_items) = item.get("evidence").and_then(Value::as_array) else {
            continue;
        };
        for evidence in evidence_items {
            let source_type = string_at(evidence, "/sourceType").unwrap_or("unknown");
            let as_of = string_at(evidence, "/sourceAsof")
                .or_else(|| string_at(evidence, "/sourceAsOf"))
                .unwrap_or("");
            let key = format!("{source_type}|{as_of}");
            if !seen.insert(key) {
                continue;
            }
            sources.push(json!({
                "label": humanize_token(source_type),
                "sourceType": source_type,
                "asOf": if as_of.is_empty() { None } else { Some(as_of) },
                "redacted": false,
            }));
        }
    }
    Value::Array(sources)
}

fn subject_label(item: &Value) -> String {
    item.pointer("/subject/label")
        .and_then(Value::as_str)
        .or_else(|| item.pointer("/subject/entityId").and_then(Value::as_str))
        .map(compact_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Unlabeled subject".to_string())
}

fn map_invoke_error(err: AbilityInvokeError) -> ToolError {
    eprintln!("mcp_v2 dailyos.read.portfolio_attention invoke failed: {err:?}");
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

    #[test]
    fn accepts_numeric_limit_strings_from_hosts() {
        let input = portfolio_attention_input(&json!({ "limit": "15", "cursor": "opaque" }))
            .expect("input");
        assert_eq!(input["pageSize"], 15);
        assert_eq!(input["cursor"], "opaque");
    }

    #[test]
    fn rejects_invalid_entity_types() {
        let err = portfolio_attention_input(&json!({ "entityTypes": ["account", "ticket"] }))
            .expect_err("invalid entity type");
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn presenter_returns_host_safe_attention_items() {
        let payload = present_portfolio_attention_response(
            json!({
                "schemaVersion": 1,
                "generatedAt": "2026-05-24T12:00:00Z",
                "items": [{
                    "rank": 1,
                    "subject": {
                        "entityType": "account",
                        "entityId": "acct-1",
                        "label": "Example Account"
                    },
                    "score": {
                        "total": 0.91,
                        "risk": 0.65,
                        "openLoops": 1.0,
                        "freshness": 1.0,
                        "trust": 0.88,
                        "salience": 0.9
                    },
                    "trustBand": "likely_current",
                    "reasons": [{
                        "kind": "open_loop",
                        "summary": "1 active open loop needs follow-up",
                        "weight": 1.0
                    }],
                    "evidence": [{
                        "text": "Follow up on unresolved launch commitment.",
                        "claimType": "open_loop",
                        "sourceType": "local_enrichment",
                        "sourceAsOf": "2026-05-23T12:00:00Z",
                        "observedAt": "2026-05-23T12:00:00Z",
                        "trustBand": "likely_current",
                        "salience": 0.9
                    }]
                }]
            }),
            json!({ "sources": [] }),
            Some("invocation-1"),
        );

        assert_eq!(payload["toolName"], TOOL_NAME);
        assert_eq!(payload["status"], "ok");
        assert!(payload["answer"]
            .as_str()
            .expect("answer")
            .contains("Example Account"));
        assert_eq!(payload["sourceEnvelope"]["rawEnvelopeIncluded"], false);
        assert_eq!(payload["provenance"]["rawClaimIdsIncluded"], false);
        assert_eq!(
            payload["attentionItems"][0]["evidence"][0]["claimType"],
            "open loop"
        );
    }
}
