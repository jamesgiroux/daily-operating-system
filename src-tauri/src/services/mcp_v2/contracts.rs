use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescription {
    pub name: ScopedName,
    pub summary: String,
    pub when_to_call: String,
    #[serde(rename = "when_NOT_to_call")]
    pub when_not_to_call: String,
    pub side: Side,
    pub parameters: Vec<ParamSpec>,
    pub returns: ReturnSpec,
    pub examples: Vec<ToolExample>,
    pub scopes_required: Vec<Scope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Read,
    SubmitCorrection,
    Write,
}

/// Canonical namespace prefix for all MCP v2 tools and new-since-amendment
/// scopes per ADR-0102 §E (2026-05-19 amendment). The form is
/// `<NAMESPACE>.<verb>.<noun>` — for example `dailyos.read.account_status`.
pub const CANONICAL_NAMESPACE: &str = "dailyos";

/// Canonical verb vocabulary for MCP v2 tool names and new scopes per
/// ADR-0102 §E (2026-05-19 amendment).
///
/// - `Read`, `Write`, `Submit` are the three core verb classes.
/// - `Search`, `List`, `Get`, `Prepare` are read-class subtypes allowed
///   when the action shape is more specific than generic `read`.
///
/// The enum is the typed encoding of the namespace freeze; runtime
/// validation against the per-client manifest is the gateway's
/// responsibility (`auth.rs`), and the validator may also accept
/// pre-amendment substrate-shipped scopes (e.g. `read.workspace_graph`,
/// `write.workspace_place_document`, `read.entity_names`) verbatim per
/// §E. [`Scope`] itself stays a permissive newtype to keep that
/// substrate-shipped form expressible.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verb {
    Read,
    Write,
    Submit,
    Search,
    List,
    Get,
    Prepare,
}

impl Verb {
    /// Wire-canonical lowercase token for this verb (`"read"`, `"write"`,
    /// `"submit"`, `"search"`, `"list"`, `"get"`, `"prepare"`). Stable
    /// across releases; used by the gateway's manifest validator and by
    /// tool-name composition helpers.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Submit => "submit",
            Self::Search => "search",
            Self::List => "list",
            Self::Get => "get",
            Self::Prepare => "prepare",
        }
    }

    /// The full ordered set of allowed verbs. Stable surface for the
    /// manifest allowlist + W1-B taxonomy validation pass.
    pub const ALL: &'static [Verb] = &[
        Verb::Read,
        Verb::Write,
        Verb::Submit,
        Verb::Search,
        Verb::List,
        Verb::Get,
        Verb::Prepare,
    ];
}

impl std::fmt::Display for Verb {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Compose a canonical [`ScopedName`] from the namespace prefix, a verb,
/// and a noun per ADR-0102 §E. Equivalent to `format!("{}.{}.{}", ...)`.
pub fn compose_scoped_name(verb: Verb, noun: &str) -> ScopedName {
    ScopedName::new(format!(
        "{}.{}.{}",
        CANONICAL_NAMESPACE,
        verb.as_str(),
        noun
    ))
}

/// Compose a canonical [`Scope`] from the namespace prefix, a verb,
/// and a noun per ADR-0102 §E. Equivalent to `format!("{}.{}.{}", ...)`.
pub fn compose_scope(verb: Verb, noun: &str) -> Scope {
    Scope::new(format!(
        "{}.{}.{}",
        CANONICAL_NAMESPACE,
        verb.as_str(),
        noun
    ))
}

/// canonical form dailyos.<verb>.<noun> per ADR-0102 amendment §E.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopedName(pub String);

impl ScopedName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ScopedName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// canonical form dailyos.<verb>.<noun> for new tools; pre-2026-05-19 substrate-shipped scopes — read.workspace_graph, write.workspace_place_document, read.entity_names — stay unprefixed verbatim per ADR-0102 amendment §E; manifest validation rejects out-of-allowlist.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Scope(pub String);

impl Scope {
    pub fn new(scope: impl Into<String>) -> Self {
        Self(scope.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamSpec {
    pub name: String,
    pub schema: ParamSchema,
    pub required: bool,
    pub description: String,
}

/// valid JSON Schema fragment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParamSchema(pub serde_json::Value);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnSpec {
    pub schema: ParamSchema,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExample {
    pub prompt: String,
    pub invocation: serde_json::Value,
    pub expected_response_shape: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub page_size: u32,
}

/// wire envelope mirror of the runtime type defined in abilities_runtime::abilities::registry::OpaqueConversationHandle.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpaqueConversationHandle(pub String);

impl OpaqueConversationHandle {
    pub fn new(handle: impl Into<String>) -> Self {
        Self(handle.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OpaqueConversationHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Server-issued opaque per-message nonce for transport replay protection
/// per ADR-0102 §C.bis.replay (cycle-8 amendment) + lifecycle per §C.bis.refresh
/// (cycle-9 amendment). The wire shape is a transparent opaque string;
/// the server-side ledger keys nonces by `(client_id, nonce)` and tracks
/// `issued_at` / `expires_at` / `consumed_at`.
///
/// Nonces flow asymmetrically: clients present `request_nonce` on the
/// request envelope, server returns `next_request_nonce` on the response
/// envelope. The seed nonce is issued at pairing handshake.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpaqueNonce(pub String);

impl OpaqueNonce {
    pub fn new(nonce: impl Into<String>) -> Self {
        Self(nonce.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OpaqueNonce {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// wire envelope mirror of the runtime type.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpClientId(pub String);

impl McpClientId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for McpClientId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// resolved/hydrated gateway-side envelope; carries granted_scopes because the gateway loads them from the server-side manifest before dispatching. Distinct from the abilities-runtime Actor::McpClient variant which carries no scopes per ADR-0102 amendment §B asymmetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum McpActor {
    Client {
        client_id: McpClientId,
        conversation_handle: Option<OpaqueConversationHandle>,
        tool_name: ScopedName,
        granted_scopes: Vec<Scope>,
    },
}

/// Top-level MCP request envelope per ADR-0102 §D (2026-05-19 amendment).
///
/// The `conversation_handle` rides at the envelope level, separate from
/// the tool's typed `params` payload. `None` on the first call in a
/// fresh conversation; the gateway mints a new handle and returns it on
/// the response envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolRequestEnvelope {
    /// Server-issued per-message nonce signed in the transport HMAC per
    /// ADR-0102 §C.bis.replay (cycle-8 amendment). Echoed from the
    /// `next_request_nonce` of the prior response envelope, or the seed
    /// nonce issued at pairing handshake on the very first call.
    /// Required on every invocation.
    pub request_nonce: OpaqueNonce,
    /// Server-minted opaque token from a prior response, echoed by the
    /// caller. Absent on the very first call in a conversation.
    pub conversation_handle: Option<OpaqueConversationHandle>,
    /// Canonical tool name being invoked. Manifest-validated at the
    /// gateway.
    pub tool_name: ScopedName,
    /// Tool-specific parameters. Validated against the tool's typed
    /// `ParamSpec` schema before handler invocation.
    pub params: serde_json::Value,
}

/// Top-level MCP response envelope per ADR-0102 §D (2026-05-19
/// amendment).
///
/// The `conversation_handle` is always present on the response: the
/// gateway either echoes the caller-presented handle or returns a
/// freshly minted one on the first call (transparent mint per §D
/// lifecycle). The tool result is carried in `result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResponseEnvelope {
    /// Server-minted opaque token. Caller persists for follow-up
    /// invocations in the same conversation.
    pub conversation_handle: OpaqueConversationHandle,
    /// Server-issued next per-message nonce per ADR-0102 §C.bis.refresh
    /// (cycle-9 amendment). Caller MUST present this as `request_nonce`
    /// on the next invocation (or the pairing-seed nonce expires). The
    /// new nonce is pre-issued with `consumed_at IS NULL` and
    /// `expires_at = now() + 5min` in `mcp_transport_nonce_ledger`.
    pub next_request_nonce: OpaqueNonce,
    /// Tool result. `Ok` carries the handler's typed JSON return value;
    /// `Err` carries a [`ToolError`].
    pub result: McpToolResult,
}

/// Tool outcome carried by [`McpToolResponseEnvelope`].
///
/// Externally tagged on the wire: `{"kind":"ok","value":...}` or
/// `{"kind":"error","error":...}`. Splitting success / error at the
/// envelope layer (rather than `Result<Value, ToolError>`) keeps the
/// wire shape stable under serde's default `Result` representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpToolResult {
    /// Handler succeeded; `value` is the typed JSON return payload.
    Ok { value: serde_json::Value },
    /// Handler or gateway returned a typed error.
    Error { error: ToolError },
}

pub trait McpToolHandler: Send + Sync {
    fn description(&self) -> &ToolDescription;

    fn invoke(
        &self,
        actor: &McpActor,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError>;
}

/// Typed errors propagated to the MCP wire.
///
/// `ConversationRequired` is intentionally absent — the gateway mints
/// handles transparently on first write per ADR-0102 §D (2026-05-19
/// amendment). Auth-state cases — `PairingRevoked`,
/// `ConversationRevoked`, `ExposureForbidden` — live as their own
/// variants rather than abusing `Unauthorized::missing_scope` with
/// sentinel strings: the [`Scope`] type is reserved for
/// `<namespace>.<verb>.<noun>` (or pre-amendment substrate) values
/// per §E, so auth-state sentinels do not belong on that field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ToolError {
    BadParams {
        detail: String,
    },
    /// Scope-deficit authorization failure — the caller's manifest grant
    /// did not include a required scope. The named `missing_scope` is a
    /// canonical or substrate-shipped [`Scope`] per ADR-0102 §E.
    Unauthorized {
        missing_scope: Scope,
    },
    /// MCP client pairing was revoked (per ADR-0102 §C). The caller must
    /// re-pair before further invocations succeed.
    PairingRevoked,
    /// `OpaqueConversationHandle` was revoked (per ADR-0102 §D). The
    /// caller must restart with a fresh conversation (the gateway will
    /// mint a new handle on the next call).
    ConversationRevoked,
    /// The caller's manifest does not include an invocable grant for
    /// this tool — either no grant exists for `tool_name` OR the grant's
    /// `exposure` tier is `None` / `MetadataOnly` per ADR-0102 §G. Use
    /// this rather than abusing `Unauthorized::missing_scope` with a
    /// sentinel `Scope` value: exposure is an auth-state distinct from
    /// scope-deficit (per ADR-0102 §C/§G cycle-7 amendment).
    ExposureForbidden {
        tool_name: ScopedName,
    },
    NotFound {
        resource: String,
    },
    RateLimited {
        retry_after_seconds: u32,
    },
    UpstreamFailure {
        detail: String,
    },
    Internal {
        trace_id: String,
    },
}

#[cfg(test)]
mod tests {
    //! Golden Rust JSON parity fixtures for the MCP v2 wire-shape types.
    //!
    //! These tests lock the canonical JSON wire shape so any future
    //! serde rename / variant reordering is caught at the contract
    //! layer rather than downstream. TS clients (handlers, frontends,
    //! tooling) consume these shapes exactly.

    use super::*;
    use serde_json::json;

    #[test]
    fn mcp_client_id_wire_is_transparent_string() {
        let id = McpClientId::new("mcp-client-alpha");
        let encoded = serde_json::to_string(&id).expect("serializes");
        assert_eq!(encoded, "\"mcp-client-alpha\"");
        let decoded: McpClientId = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded, id);
    }

    #[test]
    fn opaque_conversation_handle_wire_is_transparent_string() {
        let handle = OpaqueConversationHandle::new("conv-abc-123");
        let encoded = serde_json::to_string(&handle).expect("serializes");
        assert_eq!(encoded, "\"conv-abc-123\"");
        let decoded: OpaqueConversationHandle =
            serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded, handle);
    }

    #[test]
    fn scope_wire_is_transparent_string() {
        let scope = Scope::new("dailyos.read.account_status");
        let encoded = serde_json::to_string(&scope).expect("serializes");
        assert_eq!(encoded, "\"dailyos.read.account_status\"");
        let decoded: Scope = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded, scope);
    }

    #[test]
    fn scope_accepts_pre_amendment_unprefixed_substrate_form() {
        // Per ADR-0102 amendment §E: scopes shipped by pre-2026-05-19
        // substrate (e.g. `write.workspace_place_document`,
        // `read.workspace_graph`) stay unprefixed verbatim. The `Scope`
        // newtype is intentionally permissive at the type layer; the
        // manifest validation in `auth` rejects out-of-allowlist values.
        let legacy = Scope::new("write.workspace_place_document");
        let encoded = serde_json::to_string(&legacy).expect("serializes");
        assert_eq!(encoded, "\"write.workspace_place_document\"");
    }

    #[test]
    fn scoped_name_wire_is_transparent_string() {
        let name = ScopedName::new("dailyos.read.account_status");
        let encoded = serde_json::to_string(&name).expect("serializes");
        assert_eq!(encoded, "\"dailyos.read.account_status\"");
        let decoded: ScopedName = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded, name);
    }

    #[test]
    fn mcp_actor_client_wire_uses_camel_case_fields() {
        // Wire shape MUST match: snake_case Rust fields → camelCase JSON
        // keys, per the substrate's wire-side parity rule for MCP v2
        // contracts. `tool_name` / `client_id` / `conversation_handle`
        // / `granted_scopes` all rename.
        let actor = McpActor::Client {
            client_id: McpClientId::new("mcp-client-alpha"),
            conversation_handle: Some(OpaqueConversationHandle::new("conv-abc-123")),
            tool_name: ScopedName::new("dailyos.read.account_status"),
            granted_scopes: vec![
                Scope::new("dailyos.read.account_status"),
                Scope::new("read.entity_names"),
            ],
        };
        let encoded = serde_json::to_value(&actor).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "Client": {
                    "clientId": "mcp-client-alpha",
                    "conversationHandle": "conv-abc-123",
                    "toolName": "dailyos.read.account_status",
                    "grantedScopes": ["dailyos.read.account_status", "read.entity_names"],
                }
            })
        );
        let decoded: McpActor = serde_json::from_value(encoded).expect("deserializes");
        assert_eq!(decoded, actor);
    }

    #[test]
    fn mcp_actor_client_wire_with_absent_conversation_handle() {
        let actor = McpActor::Client {
            client_id: McpClientId::new("mcp-client-alpha"),
            conversation_handle: None,
            tool_name: ScopedName::new("dailyos.submit.note"),
            granted_scopes: vec![Scope::new("dailyos.submit.note")],
        };
        let encoded = serde_json::to_value(&actor).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "Client": {
                    "clientId": "mcp-client-alpha",
                    "conversationHandle": null,
                    "toolName": "dailyos.submit.note",
                    "grantedScopes": ["dailyos.submit.note"],
                }
            })
        );
    }

    #[test]
    fn tool_error_wire_is_tagged_snake_case_with_camel_case_fields() {
        // `BadParams` → `bad_params` tag with `detail` field unchanged.
        let err = ToolError::BadParams {
            detail: "missing parameter".to_string(),
        };
        let encoded = serde_json::to_value(&err).expect("serializes");
        assert_eq!(
            encoded,
            json!({ "kind": "bad_params", "detail": "missing parameter" })
        );
        let decoded: ToolError = serde_json::from_value(encoded).expect("deserializes");
        assert_eq!(decoded, err);
    }

    #[test]
    fn tool_error_unauthorized_carries_scope() {
        let err = ToolError::Unauthorized {
            missing_scope: Scope::new("dailyos.write.place_document"),
        };
        let encoded = serde_json::to_value(&err).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "kind": "unauthorized",
                "missingScope": "dailyos.write.place_document",
            })
        );
    }

    #[test]
    fn tool_error_rate_limited_camel_cases_retry_after() {
        let err = ToolError::RateLimited {
            retry_after_seconds: 30,
        };
        let encoded = serde_json::to_value(&err).expect("serializes");
        assert_eq!(
            encoded,
            json!({ "kind": "rate_limited", "retryAfterSeconds": 30 })
        );
    }

    #[test]
    fn tool_error_internal_carries_trace_id() {
        let err = ToolError::Internal {
            trace_id: "trace-abc".to_string(),
        };
        let encoded = serde_json::to_value(&err).expect("serializes");
        assert_eq!(
            encoded,
            json!({ "kind": "internal", "traceId": "trace-abc" })
        );
    }

    #[test]
    fn page_t_wire_camel_cases_next_cursor_and_page_size() {
        let page: Page<String> = Page {
            items: vec!["a".to_string(), "b".to_string()],
            next_cursor: Some("cursor-1".to_string()),
            page_size: 50,
        };
        let encoded = serde_json::to_value(&page).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "items": ["a", "b"],
                "nextCursor": "cursor-1",
                "pageSize": 50,
            })
        );
    }

    #[test]
    fn tool_description_wire_when_not_to_call_keeps_uppercase_alias() {
        // `when_NOT_to_call` deliberately preserves the uppercase NOT
        // on the wire — host models read the field name itself as a
        // signal of attention. Locked here so a future serde rename
        // can't silently downcase it.
        let desc = ToolDescription {
            name: ScopedName::new("dailyos.read.account_status"),
            summary: "Returns current account status.".to_string(),
            when_to_call: "When the user asks about an account they work with.".to_string(),
            when_not_to_call: "Do NOT call for broad enterprise search.".to_string(),
            side: Side::Read,
            parameters: vec![],
            returns: ReturnSpec {
                schema: ParamSchema(json!({})),
                description: "Account status payload.".to_string(),
            },
            examples: vec![],
            scopes_required: vec![Scope::new("dailyos.read.account_status")],
        };
        let encoded = serde_json::to_value(&desc).expect("serializes");
        // The field is renamed to the literal `when_NOT_to_call` token.
        assert!(encoded.get("when_NOT_to_call").is_some());
        // Other fields use camelCase from the struct-level rename_all.
        assert!(encoded.get("whenToCall").is_some());
        assert!(encoded.get("scopesRequired").is_some());
    }

    #[test]
    fn side_wire_uses_pascal_case_variants() {
        // `Side` is an externally-tagged unit enum; default serde
        // serialization keeps the Rust PascalCase variant names.
        let read = Side::Read;
        let encoded = serde_json::to_value(read).expect("serializes");
        assert_eq!(encoded, json!("Read"));

        let submit = Side::SubmitCorrection;
        let encoded = serde_json::to_value(submit).expect("serializes");
        assert_eq!(encoded, json!("SubmitCorrection"));

        let write = Side::Write;
        let encoded = serde_json::to_value(write).expect("serializes");
        assert_eq!(encoded, json!("Write"));
    }

    #[test]
    fn param_spec_and_return_spec_use_camel_case_fields() {
        let param = ParamSpec {
            name: "subject".to_string(),
            schema: ParamSchema(json!({ "type": "string" })),
            required: true,
            description: "Entity to query".to_string(),
        };
        let encoded = serde_json::to_value(&param).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "name": "subject",
                "schema": { "type": "string" },
                "required": true,
                "description": "Entity to query",
            })
        );

        let ret = ReturnSpec {
            schema: ParamSchema(json!({})),
            description: "payload".to_string(),
        };
        let encoded = serde_json::to_value(&ret).expect("serializes");
        assert!(encoded.get("schema").is_some());
        assert!(encoded.get("description").is_some());
    }

    #[test]
    fn tool_example_camel_cases_expected_response_shape() {
        let example = ToolExample {
            prompt: "What's going on with Acme?".to_string(),
            invocation: json!({ "name": "dailyos.read.account_status" }),
            expected_response_shape: json!({}),
        };
        let encoded = serde_json::to_value(&example).expect("serializes");
        assert!(encoded.get("prompt").is_some());
        assert!(encoded.get("invocation").is_some());
        assert!(encoded.get("expectedResponseShape").is_some());
    }

    #[test]
    fn verb_serializes_lowercase_and_round_trips() {
        for verb in Verb::ALL {
            let encoded = serde_json::to_value(verb).expect("serializes");
            assert_eq!(encoded, json!(verb.as_str()));
            let decoded: Verb = serde_json::from_value(encoded).expect("deserializes");
            assert_eq!(decoded, *verb);
        }
    }

    #[test]
    fn verb_as_str_matches_canonical_tokens() {
        assert_eq!(Verb::Read.as_str(), "read");
        assert_eq!(Verb::Write.as_str(), "write");
        assert_eq!(Verb::Submit.as_str(), "submit");
        assert_eq!(Verb::Search.as_str(), "search");
        assert_eq!(Verb::List.as_str(), "list");
        assert_eq!(Verb::Get.as_str(), "get");
        assert_eq!(Verb::Prepare.as_str(), "prepare");
    }

    #[test]
    fn compose_helpers_produce_canonical_dotted_form() {
        let name = compose_scoped_name(Verb::Read, "account_status");
        assert_eq!(name.as_str(), "dailyos.read.account_status");
        let scope = compose_scope(Verb::Submit, "note");
        assert_eq!(scope.as_str(), "dailyos.submit.note");
    }

    #[test]
    fn tool_error_pairing_revoked_wire_shape() {
        let err = ToolError::PairingRevoked;
        let encoded = serde_json::to_value(&err).expect("serializes");
        assert_eq!(encoded, json!({ "kind": "pairing_revoked" }));
        let decoded: ToolError = serde_json::from_value(encoded).expect("deserializes");
        assert_eq!(decoded, err);
    }

    #[test]
    fn tool_error_conversation_revoked_wire_shape() {
        let err = ToolError::ConversationRevoked;
        let encoded = serde_json::to_value(&err).expect("serializes");
        assert_eq!(encoded, json!({ "kind": "conversation_revoked" }));
        let decoded: ToolError = serde_json::from_value(encoded).expect("deserializes");
        assert_eq!(decoded, err);
    }

    #[test]
    fn tool_error_exposure_forbidden_wire_shape() {
        // Per ADR-0102 §C/§G cycle-7 amendment: absent grant or
        // exposure tier of None / MetadataOnly returns a dedicated
        // variant rather than abusing `Unauthorized::missing_scope`
        // with a sentinel `Scope` value. Wire shape mirrors the other
        // typed errors: snake_case tag + camelCase fields.
        let err = ToolError::ExposureForbidden {
            tool_name: ScopedName::new("dailyos.read.account_status"),
        };
        let encoded = serde_json::to_value(&err).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "kind": "exposure_forbidden",
                "toolName": "dailyos.read.account_status",
            })
        );
        let decoded: ToolError = serde_json::from_value(encoded).expect("deserializes");
        assert_eq!(decoded, err);
    }

    #[test]
    fn mcp_tool_request_envelope_wire_shape() {
        let envelope = McpToolRequestEnvelope {
            request_nonce: OpaqueNonce::new("nonce-req-001"),
            conversation_handle: Some(OpaqueConversationHandle::new("conv-abc-123")),
            tool_name: ScopedName::new("dailyos.read.account_status"),
            params: json!({ "subject": "acme" }),
        };
        let encoded = serde_json::to_value(&envelope).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "requestNonce": "nonce-req-001",
                "conversationHandle": "conv-abc-123",
                "toolName": "dailyos.read.account_status",
                "params": { "subject": "acme" },
            })
        );
        let decoded: McpToolRequestEnvelope =
            serde_json::from_value(encoded).expect("deserializes");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn mcp_tool_request_envelope_first_call_omits_handle() {
        let envelope = McpToolRequestEnvelope {
            request_nonce: OpaqueNonce::new("nonce-seed-pair-001"),
            conversation_handle: None,
            tool_name: ScopedName::new("dailyos.submit.note"),
            params: json!({ "body": "kickoff" }),
        };
        let encoded = serde_json::to_value(&envelope).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "requestNonce": "nonce-seed-pair-001",
                "conversationHandle": null,
                "toolName": "dailyos.submit.note",
                "params": { "body": "kickoff" },
            })
        );
    }

    #[test]
    fn mcp_tool_response_envelope_ok_wire_shape() {
        let envelope = McpToolResponseEnvelope {
            conversation_handle: OpaqueConversationHandle::new("conv-abc-123"),
            next_request_nonce: OpaqueNonce::new("nonce-resp-002"),
            result: McpToolResult::Ok {
                value: json!({ "status": "ok" }),
            },
        };
        let encoded = serde_json::to_value(&envelope).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "conversationHandle": "conv-abc-123",
                "nextRequestNonce": "nonce-resp-002",
                "result": { "kind": "ok", "value": { "status": "ok" } },
            })
        );
        let decoded: McpToolResponseEnvelope =
            serde_json::from_value(encoded).expect("deserializes");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn mcp_tool_response_envelope_error_wire_shape() {
        let envelope = McpToolResponseEnvelope {
            conversation_handle: OpaqueConversationHandle::new("conv-abc-123"),
            next_request_nonce: OpaqueNonce::new("nonce-resp-002"),
            result: McpToolResult::Error {
                error: ToolError::RateLimited {
                    retry_after_seconds: 30,
                },
            },
        };
        let encoded = serde_json::to_value(&envelope).expect("serializes");
        assert_eq!(
            encoded,
            json!({
                "conversationHandle": "conv-abc-123",
                "nextRequestNonce": "nonce-resp-002",
                "result": {
                    "kind": "error",
                    "error": { "kind": "rate_limited", "retryAfterSeconds": 30 },
                },
            })
        );
    }

    #[test]
    fn opaque_nonce_wire_is_transparent_string() {
        // Per ADR-0102 §C.bis.replay (cycle-8) + §C.bis.refresh (cycle-9):
        // request_nonce + next_request_nonce ride the wire as opaque
        // strings; the server-side ledger keys them by (client_id,
        // nonce) and tracks issued_at / expires_at / consumed_at.
        let nonce = OpaqueNonce::new("nonce-abc-123");
        let encoded = serde_json::to_string(&nonce).expect("serializes");
        assert_eq!(encoded, "\"nonce-abc-123\"");
        let decoded: OpaqueNonce = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded, nonce);
    }
}
