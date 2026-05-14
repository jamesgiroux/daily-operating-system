//! ADR-0108 provenance rendering and privacy substrate.
//!
//! `render_provenance_for` is the single typed entry point for turning a full
//! provenance envelope into a surface-safe rendered view. Bridge code that only
//! has serialized JSON should use `render_serialized_provenance_for`, which
//! falls back to the same surface rules for older/minimal test envelopes.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::OnceLock;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use super::{
    Actor, ComposedProvenance, CompositionId, DataSource, DerivationKind, FieldAttribution,
    FieldPath, Provenance, ProvenanceMaskReason, ProvenanceMasked, ProvenanceOrMasked,
    ProvenanceWarning, ScoringClass, SourceAttribution, SourceIdentifier, SourceRef,
};

pub const TAURI_INITIAL_PROVENANCE_BUDGET_BYTES: usize = 2 * 1024;
pub const MCP_DEFAULT_PROVENANCE_BUDGET_BYTES: usize = 10 * 1024;
pub const P2_PUBLICATION_FOOTNOTE_BUDGET_CHARS: usize = 500;
pub const EXPLANATION_BUDGET_CHARS: usize = 500;
pub const P2_DETAIL_CONFIRMATION_AUDIENCE: &str = "p2_publication_detail";
const MCP_SUMMARY_SOURCE_CLASS_LIMIT: usize = 16;
const P2_DETAIL_CONFIRMATION_TOKEN_VERSION: &str = "p2d1";
const P2_DETAIL_DEFAULT_USER_ID: &str = "user";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    TauriApp,
    McpTool,
    McpToolDetail,
    P2Publication,
    LogStructured,
}

impl Surface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TauriApp => "tauri_app",
            Self::McpTool => "mcp_tool",
            Self::McpToolDetail => "mcp_tool_detail",
            Self::P2Publication => "p2_publication",
            Self::LogStructured => "log_structured",
        }
    }

    fn byte_budget(self) -> Option<usize> {
        match self {
            Self::TauriApp => Some(TAURI_INITIAL_PROVENANCE_BUDGET_BYTES),
            Self::McpTool | Self::McpToolDetail => Some(MCP_DEFAULT_PROVENANCE_BUDGET_BYTES),
            Self::P2Publication | Self::LogStructured => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RenderedProvenance {
    pub surface: Surface,
    pub value: Value,
}

impl RenderedProvenance {
    pub fn new(surface: Surface, value: Value) -> Self {
        Self { surface, value }
    }

    pub fn serialized_len(&self) -> Result<usize, serde_json::Error> {
        serde_json::to_vec(self).map(|bytes| bytes.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceIdRedacted {
    pub data_source_class: String,
    pub scoring_class: ScoringClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChildElided {
    pub ability_name: String,
    pub data_sources_summary: Vec<String>,
}

#[derive(Debug, Default)]
struct RenderContext {
    warnings: Vec<ProvenanceWarning>,
    composition_labels: BTreeMap<String, String>,
    child_composition_labels: VecDeque<String>,
    next_composition_label: usize,
}

impl RenderContext {
    fn for_provenance(prov: &Provenance) -> Self {
        let mut ctx = Self::default();
        ctx.prime_from_provenance(prov);
        ctx
    }

    fn for_serialized_provenance(value: &Value) -> Self {
        let mut ctx = Self::default();
        ctx.prime_from_serialized_provenance(serialized_full_provenance_payload(value));
        ctx
    }

    fn opaque_composition_label(&mut self, composition_id: &CompositionId) -> String {
        self.opaque_composition_label_for_raw(composition_id.as_str())
    }

    fn opaque_child_composition_label(&mut self, composition_id: &CompositionId) -> String {
        self.child_composition_labels
            .pop_front()
            .unwrap_or_else(|| self.opaque_composition_label(composition_id))
    }

    fn opaque_composition_label_for_raw(&mut self, composition_id: &str) -> String {
        if let Some(label) = self.composition_labels.get(composition_id) {
            return label.clone();
        }

        self.next_composition_label += 1;
        let label = format!("c{}", self.next_composition_label);
        self.composition_labels
            .insert(composition_id.to_string(), label.clone());
        label
    }

    fn register_child_composition_id(&mut self, composition_id: &str) {
        let label = self.opaque_composition_label_for_raw(composition_id);
        self.child_composition_labels.push_back(label);
    }

    fn prime_from_provenance(&mut self, prov: &Provenance) {
        for child in &prov.children {
            self.register_child_composition_id(child.composition_id.as_str());
            self.prime_from_provenance(child.provenance.as_ref());
        }

        for attribution in prov.field_attributions.values() {
            self.prime_from_field_attribution(attribution);
        }
        for warning in &prov.warnings {
            self.prime_from_warning(warning);
        }
    }

    fn prime_from_field_attribution(&mut self, attribution: &FieldAttribution) {
        if let DerivationKind::Composed { composition_id } = &attribution.derivation {
            self.opaque_composition_label(composition_id);
        }

        for source_ref in &attribution.source_refs {
            if let SourceRef::Child { composition_id, .. } = source_ref {
                self.opaque_composition_label(composition_id);
            }
        }
    }

    fn prime_from_warning(&mut self, warning: &ProvenanceWarning) {
        if let ProvenanceWarning::OptionalComposedReadFailed { composition_id, .. } = warning {
            self.opaque_composition_label(composition_id);
        }
    }

    fn prime_from_serialized_provenance(&mut self, value: &Value) {
        let Some(object) = value.as_object() else {
            return;
        };
        let child_candidates = serialized_direct_composition_refs(object);

        if let Some(children) = object.get("children").and_then(Value::as_array) {
            let mut used_candidates = vec![false; child_candidates.len()];
            for (index, child) in children.iter().enumerate() {
                if let Some(raw_id) = serialized_child_composition_id(
                    child,
                    index,
                    &child_candidates,
                    &mut used_candidates,
                ) {
                    self.register_child_composition_id(&raw_id);
                }
                self.prime_from_serialized_provenance(child);
            }
        }

        self.prime_from_serialized_composition_refs(value);
    }

    fn prime_from_serialized_composition_refs(&mut self, value: &Value) {
        collect_composition_refs(value, true, &mut |composition_id| {
            self.opaque_composition_label_for_raw(composition_id);
        });
    }
}

fn serialized_full_provenance_payload(value: &Value) -> &Value {
    value
        .as_object()
        .filter(|object| object.get("kind").and_then(Value::as_str) == Some("full"))
        .and_then(|object| object.get("value"))
        .unwrap_or(value)
}

fn serialized_direct_composition_refs(object: &Map<String, Value>) -> Vec<String> {
    let mut refs = Vec::new();
    for (key, value) in object {
        if key == "children" {
            continue;
        }
        collect_composition_refs(value, false, &mut |composition_id| {
            if !refs.iter().any(|existing| existing == composition_id) {
                refs.push(composition_id.to_string());
            }
        });
    }
    refs
}

fn serialized_child_composition_id(
    child: &Value,
    index: usize,
    candidates: &[String],
    used_candidates: &mut [bool],
) -> Option<String> {
    if let Some(composition_id) = child.get("composition_id").and_then(Value::as_str) {
        return Some(composition_id.to_string());
    }

    if let Some(ability_name) = child.get("ability_name").and_then(Value::as_str) {
        if let Some(candidate_index) =
            find_single_unused_candidate(candidates, used_candidates, |candidate| {
                candidate == ability_name
            })
        {
            used_candidates[candidate_index] = true;
            return Some(candidates[candidate_index].clone());
        }

        if let Some(candidate_index) =
            find_single_unused_candidate(candidates, used_candidates, |candidate| {
                candidate
                    .strip_prefix(ability_name)
                    .is_some_and(|suffix| suffix.starts_with(':'))
            })
        {
            used_candidates[candidate_index] = true;
            return Some(candidates[candidate_index].clone());
        }
    }

    if index < candidates.len() && !used_candidates[index] {
        used_candidates[index] = true;
        return Some(candidates[index].clone());
    }

    if let Some(candidate_index) = used_candidates.iter().position(|used| !*used) {
        used_candidates[candidate_index] = true;
        return Some(candidates[candidate_index].clone());
    }

    child
        .get("ability_name")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn find_single_unused_candidate(
    candidates: &[String],
    used_candidates: &[bool],
    matches: impl Fn(&str) -> bool,
) -> Option<usize> {
    let mut found = None;
    for (index, candidate) in candidates.iter().enumerate() {
        if !used_candidates[index] && matches(candidate) {
            if found.is_some() {
                return None;
            }
            found = Some(index);
        }
    }
    found
}

fn collect_composition_refs(value: &Value, skip_children: bool, visit: &mut impl FnMut(&str)) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_composition_refs(item, skip_children, visit);
            }
        }
        Value::Object(object) => {
            if let Some(composition_id) = object.get("composition_id").and_then(Value::as_str) {
                visit(composition_id);
            }
            for (key, item) in object {
                if skip_children && key == "children" {
                    continue;
                }
                collect_composition_refs(item, skip_children, visit);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldAllowlist {
    permitted_field_paths: Vec<&'static str>,
}

impl FieldAllowlist {
    fn new(permitted_field_paths: Vec<&'static str>) -> Self {
        Self {
            permitted_field_paths,
        }
    }

    fn permits_exact(&self, path: &[String]) -> bool {
        self.permitted_field_paths
            .iter()
            .any(|pattern| field_path_pattern_matches(pattern, path))
    }

    fn permits_descendant(&self, path: &[String]) -> bool {
        self.permitted_field_paths
            .iter()
            .any(|pattern| field_path_is_pattern_prefix(path, pattern))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderOptions {
    pub p2_detail_confirmation: Option<P2DetailConfirmationToken>,
}

impl RenderOptions {
    pub fn with_p2_detail_confirmation(token: P2DetailConfirmationToken) -> Self {
        Self {
            p2_detail_confirmation: Some(token),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P2DetailConfirmationToken {
    serialized: String,
    claims: P2DetailConfirmationClaims,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct P2DetailConfirmationClaims {
    audience: String,
    invocation_id: crate::abilities::provenance::InvocationId,
    user_id: String,
    expires_at: DateTime<Utc>,
    nonce: String,
}

impl P2DetailConfirmationToken {
    pub fn new(token: impl Into<String>) -> Option<Self> {
        let (serialized, claims) = parse_issued_p2_detail_confirmation_token(&token.into())?;
        Some(Self { serialized, claims })
    }

    pub fn issue(
        audience: impl Into<String>,
        invocation_id: crate::abilities::provenance::InvocationId,
        user_id: impl Into<String>,
        expiry_secs: i64,
    ) -> Option<Self> {
        Self::issue_at(audience, invocation_id, user_id, expiry_secs, Utc::now())
    }

    pub fn as_str(&self) -> &str {
        self.serialized.as_str()
    }

    fn issue_at(
        audience: impl Into<String>,
        invocation_id: crate::abilities::provenance::InvocationId,
        user_id: impl Into<String>,
        expiry_secs: i64,
        issued_at: DateTime<Utc>,
    ) -> Option<Self> {
        let audience = audience.into();
        let user_id = user_id.into();
        if audience.trim().is_empty() || user_id.trim().is_empty() {
            return None;
        }
        let claims = P2DetailConfirmationClaims {
            audience,
            invocation_id,
            user_id,
            expires_at: issued_at.checked_add_signed(Duration::seconds(expiry_secs))?,
            nonce: uuid::Uuid::new_v4().to_string(),
        };
        let serialized = serialize_p2_detail_confirmation_token(&claims)?;
        Some(Self { serialized, claims })
    }

    fn is_valid_for_p2_detail(
        &self,
        invocation_id: crate::abilities::provenance::InvocationId,
        user_id: &str,
    ) -> bool {
        let Some((_, verified_claims)) =
            parse_issued_p2_detail_confirmation_token(self.serialized.as_str())
        else {
            return false;
        };
        verified_claims == self.claims
            && self.claims.audience == P2_DETAIL_CONFIRMATION_AUDIENCE
            && self.claims.invocation_id == invocation_id
            && self.claims.user_id == user_id
            && Utc::now() < self.claims.expires_at
    }
}

type HmacSha256 = Hmac<Sha256>;

fn serialize_p2_detail_confirmation_token(claims: &P2DetailConfirmationClaims) -> Option<String> {
    let payload = serde_json::to_vec(claims).ok()?;
    let payload = URL_SAFE_NO_PAD.encode(payload);
    let signature = sign_p2_detail_confirmation_payload(&payload);
    Some(format!(
        "{P2_DETAIL_CONFIRMATION_TOKEN_VERSION}.{payload}.{signature}"
    ))
}

fn parse_issued_p2_detail_confirmation_token(
    token: &str,
) -> Option<(String, P2DetailConfirmationClaims)> {
    let serialized = token.trim().to_string();
    let mut parts = serialized.split('.');
    let version = parts.next()?;
    let payload = parts.next()?;
    let signature = parts.next()?;
    if parts.next().is_some() || version != P2_DETAIL_CONFIRMATION_TOKEN_VERSION {
        return None;
    }

    let expected_signature = sign_p2_detail_confirmation_payload(payload);
    if !constant_time_eq(signature.as_bytes(), expected_signature.as_bytes()) {
        return None;
    }

    let claims: P2DetailConfirmationClaims =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    if claims.audience.trim().is_empty() || claims.user_id.trim().is_empty() {
        return None;
    }
    Some((serialized, claims))
}

fn sign_p2_detail_confirmation_payload(payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(p2_detail_confirmation_secret())
        .expect("static P2 detail confirmation signing key is valid");
    mac.update(payload.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

fn p2_detail_confirmation_secret() -> &'static [u8; 32] {
    static SECRET: OnceLock<[u8; 32]> = OnceLock::new();
    SECRET.get_or_init(|| {
        let mut secret = [0; 32];
        secret[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        secret[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        secret
    })
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0, |diff, (left, right)| diff | (left ^ right))
        == 0
}

pub fn render_provenance_for(
    prov: &Provenance,
    actor: Actor,
    surface: Surface,
) -> RenderedProvenance {
    render_provenance_for_with_options(prov, actor, surface, RenderOptions::default())
}

pub fn render_provenance_for_with_options(
    prov: &Provenance,
    actor: Actor,
    surface: Surface,
    options: RenderOptions,
) -> RenderedProvenance {
    render_provenance_for_with_context(
        prov,
        actor,
        surface,
        options,
        RenderContext::for_provenance(prov),
    )
}

fn render_provenance_for_with_context(
    prov: &Provenance,
    actor: Actor,
    surface: Surface,
    options: RenderOptions,
    mut ctx: RenderContext,
) -> RenderedProvenance {
    let value = match surface {
        Surface::TauriApp => render_tauri_app(prov, &actor, &mut ctx),
        Surface::McpTool => render_mcp_provenance(prov, &actor, surface, &mut ctx),
        Surface::McpToolDetail => render_mcp_provenance(prov, &actor, surface, &mut ctx),
        Surface::P2Publication => render_p2_publication(
            prov,
            &actor,
            &mut ctx,
            options.p2_detail_confirmation.as_ref(),
        ),
        Surface::LogStructured => render_log_structured(prov),
    };
    let value = if surface == Surface::LogStructured {
        value
    } else {
        let render_warnings = std::mem::take(&mut ctx.warnings);
        attach_render_warnings(value, prov, &actor, surface, render_warnings, &mut ctx)
    };
    enforce_render_budget(RenderedProvenance::new(surface, value))
}

pub fn render_serialized_provenance_for(
    provenance: Value,
    actor: Actor,
    surface: Surface,
) -> RenderedProvenance {
    render_serialized_provenance_for_with_options(
        provenance,
        actor,
        surface,
        RenderOptions::default(),
    )
}

pub fn render_serialized_provenance_for_with_options(
    provenance: Value,
    actor: Actor,
    surface: Surface,
    options: RenderOptions,
) -> RenderedProvenance {
    let ctx = RenderContext::for_serialized_provenance(&provenance);
    if let Ok(envelope) = serde_json::from_value::<ProvenanceOrMasked>(provenance.clone()) {
        return match envelope {
            ProvenanceOrMasked::Full(prov) => {
                render_provenance_for_with_context(&prov, actor, surface, options, ctx)
            }
            ProvenanceOrMasked::Masked(masked) => {
                render_masked_provenance_for(&masked, actor, surface)
            }
        };
    }

    match serde_json::from_value::<Provenance>(provenance) {
        Ok(prov) => render_provenance_for_with_context(&prov, actor, surface, options, ctx),
        Err(_) => enforce_render_budget(RenderedProvenance::new(
            surface,
            render_unparseable_legacy_provenance(&actor, surface),
        )),
    }
}

fn project_provenance_for_render(
    prov: &Provenance,
    actor: &Actor,
    surface: Surface,
    ctx: &mut RenderContext,
) -> Value {
    let allowlist = field_allowlist_for(surface, actor);
    let mut value = serde_json::to_value(prov.clone())
        .unwrap_or_else(|_| json!({ "error": "provenance_unserializable" }));
    normalize_projected_provenance_value(&mut value);
    let mut value = project_value_to_allowlist(value, &allowlist).unwrap_or_else(|| json!({}));
    sanitize_projected_warning_fields(&mut value, prov, actor, surface, ctx);
    sanitize_explanations_in_value(value, ctx)
}

fn field_allowlist_for(surface: Surface, actor: &Actor) -> FieldAllowlist {
    match (surface, projection_actor_class(actor)) {
        (Surface::TauriApp, ProjectionActorClass::User | ProjectionActorClass::Human) => {
            tauri_user_field_allowlist()
        }
        (Surface::TauriApp, _) => minimal_field_allowlist(),
        (Surface::McpTool, ProjectionActorClass::Agent) => mcp_tool_agent_field_allowlist(),
        (Surface::McpToolDetail, ProjectionActorClass::Agent) => {
            mcp_tool_detail_agent_field_allowlist()
        }
        (Surface::McpTool | Surface::McpToolDetail, _) => mcp_user_field_allowlist(),
        (Surface::P2Publication, _) => p2_publication_field_allowlist(),
        (Surface::LogStructured, _) => log_structured_field_allowlist(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionActorClass {
    User,
    Agent,
    Human,
    System,
    External,
}

fn projection_actor_class(actor: &Actor) -> ProjectionActorClass {
    match actor {
        Actor::User => ProjectionActorClass::User,
        Actor::Agent { .. } => ProjectionActorClass::Agent,
        Actor::Human { .. } => ProjectionActorClass::Human,
        Actor::System { .. } => ProjectionActorClass::System,
        Actor::External { .. } => ProjectionActorClass::External,
    }
}

fn tauri_user_field_allowlist() -> FieldAllowlist {
    FieldAllowlist::new(vec![
        "provenance_schema_version",
        "ability_name",
        "ability_version.major",
        "ability_version.minor",
        "ability_schema_version",
        "invocation_id",
        "produced_at",
        "actor",
        "mode",
        "trust.effective",
        "trust.contains_stored_synthesis",
        "sources.[].data_source",
        "sources.[].identifiers",
        "sources.[].observed_at",
        "sources.[].source_asof",
        "sources.[].evidence_weight",
        "sources.[].scoring_class",
        "thread_ids",
        "children.[].composition_id",
        "children.[].provenance.provenance_schema_version",
        "children.[].provenance.ability_name",
        "children.[].provenance.ability_version.major",
        "children.[].provenance.ability_version.minor",
        "children.[].provenance.ability_schema_version",
        "children.[].provenance.invocation_id",
        "children.[].provenance.produced_at",
        "children.[].provenance.actor",
        "children.[].provenance.mode",
        "children.[].provenance.trust.effective",
        "children.[].provenance.trust.contains_stored_synthesis",
        "children.[].provenance.sources.[].data_source",
        "children.[].provenance.sources.[].identifiers",
        "children.[].provenance.sources.[].observed_at",
        "children.[].provenance.sources.[].source_asof",
        "children.[].provenance.sources.[].evidence_weight",
        "children.[].provenance.sources.[].scoring_class",
        "children.[].provenance.thread_ids",
        "children.[].provenance.field_attributions.*.subject",
        "children.[].provenance.field_attributions.*.derivation",
        "children.[].provenance.field_attributions.*.source_refs",
        "children.[].provenance.field_attributions.*.confidence",
        "children.[].provenance.field_attributions.*.explanation",
        "children.[].provenance.field_attributions.*.trust_band",
        "children.[].provenance.subject",
        "children.[].provenance.warnings",
        "field_attributions.*.subject",
        "field_attributions.*.derivation",
        "field_attributions.*.source_refs",
        "field_attributions.*.confidence",
        "field_attributions.*.explanation",
        "field_attributions.*.trust_band",
        "subject",
        "warnings",
    ])
}

fn mcp_tool_agent_field_allowlist() -> FieldAllowlist {
    FieldAllowlist::new(vec![
        "provenance_schema_version",
        "ability_name",
        "invocation_id",
        "produced_at",
        "actor",
        "mode",
        "trust.effective",
        "trust.contains_stored_synthesis",
        "sources.[].data_source",
    ])
}

fn mcp_tool_detail_agent_field_allowlist() -> FieldAllowlist {
    FieldAllowlist::new(vec![
        "provenance_schema_version",
        "ability_name",
        "ability_version.major",
        "ability_version.minor",
        "ability_schema_version",
        "invocation_id",
        "produced_at",
        "actor",
        "mode",
        "trust.effective",
        "trust.contains_stored_synthesis",
        "sources.[].data_source",
        "sources.[].observed_at",
        "sources.[].source_asof",
        "sources.[].evidence_weight",
        "sources.[].scoring_class",
        "children.[].composition_id",
        "children.[].provenance.ability_name",
        "children.[].provenance.produced_at",
        "children.[].provenance.trust.effective",
        "children.[].provenance.trust.contains_stored_synthesis",
        "children.[].provenance.sources.[].data_source",
        "field_attributions.*.derivation",
        "field_attributions.*.source_refs",
        "field_attributions.*.confidence",
        "field_attributions.*.explanation",
        "field_attributions.*.trust_band",
        "warnings",
    ])
}

fn mcp_user_field_allowlist() -> FieldAllowlist {
    FieldAllowlist::new(vec![
        "ability_name",
        "produced_at",
        "trust.effective",
        "trust.contains_stored_synthesis",
        "sources.[].data_source",
        "sources.[].observed_at",
        "sources.[].source_asof",
        "field_attributions.*.derivation",
        "field_attributions.*.source_refs",
        "field_attributions.*.confidence",
        "field_attributions.*.explanation",
        "field_attributions.*.trust_band",
        "warnings",
    ])
}

fn p2_publication_field_allowlist() -> FieldAllowlist {
    FieldAllowlist::new(vec![
        "ability_name",
        "produced_at",
        "sources.[].data_source",
        "sources.[].observed_at",
        "sources.[].source_asof",
        "field_attributions.*.derivation",
        "field_attributions.*.source_refs",
        "field_attributions.*.confidence",
        "field_attributions.*.explanation",
        "field_attributions.*.trust_band",
        "warnings",
    ])
}

fn log_structured_field_allowlist() -> FieldAllowlist {
    FieldAllowlist::new(vec![
        "invocation_id",
        "ability_name",
        "ability_version.major",
        "ability_version.minor",
        "ability_schema_version",
        "produced_at",
        "trust.effective",
        "warnings",
    ])
}

fn minimal_field_allowlist() -> FieldAllowlist {
    FieldAllowlist::new(vec!["ability_name", "produced_at", "warnings"])
}

fn normalize_projected_provenance_value(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                normalize_projected_provenance_value(item);
            }
        }
        Value::Object(object) => {
            if object.contains_key("provider_completion")
                || object.contains_key("ProviderCompletion")
            {
                *value = json!({ "kind": "provider_output_redacted" });
                return;
            }

            for forbidden in [
                "prompt_fingerprint",
                "canonical_prompt_hash",
                "provider_completion_id",
                "completion_id",
                "seed",
            ] {
                object.remove(forbidden);
            }

            if let Some(data_source) = object.get_mut("data_source") {
                *data_source = rendered_data_source_class_value(data_source);
            }

            for item in object.values_mut() {
                normalize_projected_provenance_value(item);
            }
        }
        _ => {}
    }
}

fn project_value_to_allowlist(value: Value, allowlist: &FieldAllowlist) -> Option<Value> {
    project_value_to_allowlist_at(value, allowlist, &mut Vec::new())
}

fn project_value_to_allowlist_at(
    value: Value,
    allowlist: &FieldAllowlist,
    path: &mut Vec<String>,
) -> Option<Value> {
    match value {
        primitive @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => {
            allowlist.permits_exact(path).then_some(primitive)
        }
        Value::Array(items) => {
            if allowlist.permits_exact(path) && !allowlist.permits_descendant(path) {
                return Some(Value::Array(items));
            }
            path.push("[]".to_string());
            let rendered = if allowlist.permits_descendant(path) {
                let projected_items = items
                    .into_iter()
                    .filter_map(|item| project_value_to_allowlist_at(item, allowlist, path))
                    .collect::<Vec<_>>();
                Some(Value::Array(projected_items))
            } else {
                None
            };
            path.pop();
            rendered
        }
        Value::Object(object) => {
            if allowlist.permits_exact(path) && !allowlist.permits_descendant(path) {
                return Some(Value::Object(object));
            }
            let mut rendered = Map::new();
            for (key, item) in object {
                path.push(key.clone());
                if allowlist.permits_exact(path) || allowlist.permits_descendant(path) {
                    if let Some(projected) = project_value_to_allowlist_at(item, allowlist, path) {
                        rendered.insert(key, projected);
                    }
                }
                path.pop();
            }
            (!rendered.is_empty()).then_some(Value::Object(rendered))
        }
    }
}

fn field_path_pattern_matches(pattern: &str, path: &[String]) -> bool {
    let pattern_segments = pattern.split('.').collect::<Vec<_>>();
    pattern_segments.len() == path.len()
        && pattern_segments
            .iter()
            .zip(path)
            .all(|(pattern, segment)| field_path_segment_matches(pattern, segment))
}

fn field_path_is_pattern_prefix(path: &[String], pattern: &str) -> bool {
    let pattern_segments = pattern.split('.').collect::<Vec<_>>();
    if path.len() >= pattern_segments.len() {
        return false;
    }
    pattern_segments
        .iter()
        .zip(path)
        .all(|(pattern, segment)| field_path_segment_matches(pattern, segment))
}

fn field_path_segment_matches(pattern: &str, segment: &str) -> bool {
    pattern == "*" || pattern == segment
}

pub fn sanitize_explanation_for_render(
    field: &FieldPath,
    explanation: &str,
) -> (String, Option<ProvenanceWarning>) {
    let raw = explanation.trim();
    let lowered = raw.to_ascii_lowercase();
    if banned_explanation_tokens()
        .iter()
        .any(|token| lowered.contains(token))
    {
        return (
            "[explanation removed by sanitizer]".to_string(),
            Some(ProvenanceWarning::ExplanationFiltered {
                field: field.clone(),
                reason: "banned_token".to_string(),
            }),
        );
    }

    let without_markdown_markers = strip_markdown_line_markers(raw);
    let without_markdown_links = markdown_link_regex().replace_all(
        &without_markdown_markers,
        |captures: &regex::Captures<'_>| {
            let label = captures
                .get(1)
                .map_or("", |capture| capture.as_str())
                .trim();
            if label.is_empty() {
                "[url removed]".to_string()
            } else {
                format!("{label} [url removed]")
            }
        },
    );
    let without_schemes = uri_scheme_regex().replace_all(&without_markdown_links, "[url removed]");
    let without_domains = bare_domain_regex().replace_all(&without_schemes, "[url removed]");
    let without_backticks = without_domains.replace('`', "");
    let bounded = truncate_chars(&without_backticks, EXPLANATION_BUDGET_CHARS);
    (html_entity_encode(&bounded), None)
}

fn render_tauri_app(prov: &Provenance, actor: &Actor, ctx: &mut RenderContext) -> Value {
    let summary = json!({
        "ability_name": prov.ability_name,
        "produced_at": prov.produced_at,
        "source_count": prov.sources.len(),
        "trust": render_mcp_trust(prov),
        "composition_depth": composition_depth(prov),
        "data_sources_summary": source_class_summary(&prov.sources),
        "contains_ai_generated_sources": prov.sources.iter().any(|source| source.synthesis_marker.is_some()),
    });
    let details = json!({
        "sources": render_sources_for_tauri(&prov.sources),
        "field_attributions": render_field_attributions_for_mcp(&prov.field_attributions, Surface::TauriApp, ctx),
        "children": prov.children.iter().map(|child| render_child_for_tauri(child, actor, ctx)).collect::<Vec<_>>(),
    });
    let full_json = project_provenance_for_render(prov, actor, Surface::TauriApp, ctx);
    let field_attributions = render_field_attributions(&prov.field_attributions, ctx);
    json!({
        "ability_name": prov.ability_name,
        "produced_at": prov.produced_at,
        "sources": render_sources_for_tauri(&prov.sources),
        "field_attributions": field_attributions,
        "about_this": {
            "label": "About this",
            "levels": ["summary", "details", "full_json"],
            "summary": summary,
            "details": details,
            "full_json": full_json,
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpPolicyActor {
    User,
    Agent,
    Admin,
}

impl McpPolicyActor {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpRenderShape {
    Summary,
    Detail,
}

impl McpRenderShape {
    fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Detail => "detail",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpProvenanceField {
    ProvenanceSchemaVersion,
    AbilityName,
    AbilityVersion,
    AbilitySchemaVersion,
    InvocationId,
    ProducedAt,
    ProvenanceActor,
    RenderActor,
    Mode,
    Trust,
    Sources,
    SourceCount,
    DataSourcesSummary,
    Children,
    ChildCount,
    FieldAttributions,
    FieldAttributionCount,
    Subject,
    DetailAvailable,
}

#[derive(Debug, Clone, Copy)]
struct McpRenderPolicy {
    actor: McpPolicyActor,
    surface: Surface,
    shape: McpRenderShape,
    fields: &'static [McpProvenanceField],
}

impl McpRenderPolicy {
    fn allows(self, field: McpProvenanceField) -> bool {
        self.fields.contains(&field)
    }
}

const AGENT_MCP_SUMMARY_FIELDS: &[McpProvenanceField] = &[
    McpProvenanceField::ProvenanceSchemaVersion,
    McpProvenanceField::AbilityName,
    McpProvenanceField::InvocationId,
    McpProvenanceField::ProducedAt,
    McpProvenanceField::ProvenanceActor,
    McpProvenanceField::RenderActor,
    McpProvenanceField::Mode,
    McpProvenanceField::Trust,
    McpProvenanceField::SourceCount,
    McpProvenanceField::DataSourcesSummary,
    McpProvenanceField::ChildCount,
    McpProvenanceField::FieldAttributionCount,
    McpProvenanceField::DetailAvailable,
];

const USER_MCP_SUMMARY_FIELDS: &[McpProvenanceField] = &[
    McpProvenanceField::AbilityName,
    McpProvenanceField::ProducedAt,
    McpProvenanceField::RenderActor,
    McpProvenanceField::SourceCount,
    McpProvenanceField::DataSourcesSummary,
    McpProvenanceField::DetailAvailable,
];

const ADMIN_MCP_SUMMARY_FIELDS: &[McpProvenanceField] = &[
    McpProvenanceField::ProvenanceSchemaVersion,
    McpProvenanceField::AbilityName,
    McpProvenanceField::AbilityVersion,
    McpProvenanceField::AbilitySchemaVersion,
    McpProvenanceField::InvocationId,
    McpProvenanceField::ProducedAt,
    McpProvenanceField::ProvenanceActor,
    McpProvenanceField::RenderActor,
    McpProvenanceField::Mode,
    McpProvenanceField::Trust,
    McpProvenanceField::SourceCount,
    McpProvenanceField::DataSourcesSummary,
    McpProvenanceField::ChildCount,
    McpProvenanceField::FieldAttributionCount,
    McpProvenanceField::DetailAvailable,
];

const AGENT_MCP_DETAIL_FIELDS: &[McpProvenanceField] = &[
    McpProvenanceField::ProvenanceSchemaVersion,
    McpProvenanceField::AbilityName,
    McpProvenanceField::AbilityVersion,
    McpProvenanceField::AbilitySchemaVersion,
    McpProvenanceField::InvocationId,
    McpProvenanceField::ProducedAt,
    McpProvenanceField::ProvenanceActor,
    McpProvenanceField::RenderActor,
    McpProvenanceField::Mode,
    McpProvenanceField::Trust,
    McpProvenanceField::Sources,
    McpProvenanceField::Children,
    McpProvenanceField::FieldAttributions,
    McpProvenanceField::Subject,
];

const USER_MCP_DETAIL_FIELDS: &[McpProvenanceField] = &[
    McpProvenanceField::AbilityName,
    McpProvenanceField::ProducedAt,
    McpProvenanceField::RenderActor,
    McpProvenanceField::Trust,
    McpProvenanceField::Sources,
    McpProvenanceField::FieldAttributions,
    McpProvenanceField::Subject,
];

const ADMIN_MCP_DETAIL_FIELDS: &[McpProvenanceField] = &[
    McpProvenanceField::ProvenanceSchemaVersion,
    McpProvenanceField::AbilityName,
    McpProvenanceField::AbilityVersion,
    McpProvenanceField::AbilitySchemaVersion,
    McpProvenanceField::InvocationId,
    McpProvenanceField::ProducedAt,
    McpProvenanceField::ProvenanceActor,
    McpProvenanceField::RenderActor,
    McpProvenanceField::Mode,
    McpProvenanceField::Trust,
    McpProvenanceField::Sources,
    McpProvenanceField::Children,
    McpProvenanceField::FieldAttributions,
    McpProvenanceField::Subject,
];

fn render_mcp_provenance(
    prov: &Provenance,
    actor: &Actor,
    surface: Surface,
    ctx: &mut RenderContext,
) -> Value {
    let Some(policy) = mcp_render_policy(actor, surface) else {
        return render_mcp_fail_closed(surface);
    };

    let mut object = Map::new();
    object.insert("render_level".to_string(), json!(policy.shape.as_str()));
    object.insert("surface".to_string(), json!(policy.surface.as_str()));

    if policy.allows(McpProvenanceField::ProvenanceSchemaVersion) {
        object.insert(
            "provenance_schema_version".to_string(),
            json!(prov.provenance_schema_version),
        );
    }
    if policy.allows(McpProvenanceField::AbilityName) {
        object.insert("ability_name".to_string(), json!(prov.ability_name));
    }
    if policy.allows(McpProvenanceField::AbilityVersion) {
        object.insert("ability_version".to_string(), json!(prov.ability_version));
    }
    if policy.allows(McpProvenanceField::AbilitySchemaVersion) {
        object.insert(
            "ability_schema_version".to_string(),
            json!(prov.ability_schema_version),
        );
    }
    if policy.allows(McpProvenanceField::InvocationId) {
        object.insert("invocation_id".to_string(), json!(prov.invocation_id));
    }
    if policy.allows(McpProvenanceField::ProducedAt) {
        object.insert("produced_at".to_string(), json!(prov.produced_at));
    }
    if policy.allows(McpProvenanceField::ProvenanceActor) {
        object.insert("actor".to_string(), json!(actor_class(&prov.actor)));
    }
    if policy.allows(McpProvenanceField::RenderActor) {
        object.insert("render_actor".to_string(), json!(policy.actor.as_str()));
    }
    if policy.allows(McpProvenanceField::Mode) {
        object.insert("mode".to_string(), json!(prov.mode));
    }
    if policy.allows(McpProvenanceField::Trust) {
        object.insert("trust".to_string(), render_mcp_trust(prov));
    }
    if policy.allows(McpProvenanceField::Sources) {
        object.insert(
            "sources".to_string(),
            json!(render_sources_for_mcp(&prov.sources)),
        );
    }
    if policy.allows(McpProvenanceField::SourceCount) {
        object.insert("source_count".to_string(), json!(prov.sources.len()));
    }
    if policy.allows(McpProvenanceField::DataSourcesSummary) {
        object.insert(
            "data_sources_summary".to_string(),
            json!(bounded_source_class_summary(&prov.sources)),
        );
    }
    if policy.allows(McpProvenanceField::Children) {
        object.insert(
            "children".to_string(),
            json!(prov
                .children
                .iter()
                .map(|child| render_child_for_mcp(child, 1, ctx))
                .collect::<Vec<_>>()),
        );
    }
    if policy.allows(McpProvenanceField::ChildCount) {
        object.insert("child_count".to_string(), json!(prov.children.len()));
    }
    if policy.allows(McpProvenanceField::FieldAttributions) {
        object.insert(
            "field_attributions".to_string(),
            render_field_attributions_for_mcp(&prov.field_attributions, surface, ctx),
        );
    }
    if policy.allows(McpProvenanceField::FieldAttributionCount) {
        object.insert(
            "field_attribution_count".to_string(),
            json!(prov.field_attributions.len()),
        );
    }
    if policy.allows(McpProvenanceField::Subject) {
        object.insert("subject".to_string(), render_subject_redacted());
    }
    if policy.allows(McpProvenanceField::DetailAvailable) {
        object.insert("detail_available".to_string(), json!(true));
    }

    let mut value = Value::Object(object);
    redact_internal_mcp_values(&mut value);
    value
}

fn mcp_render_policy(actor: &Actor, surface: Surface) -> Option<McpRenderPolicy> {
    let actor = mcp_policy_actor(actor)?;
    let (shape, fields) = match (actor, surface) {
        (McpPolicyActor::Agent, Surface::McpTool) => {
            (McpRenderShape::Summary, AGENT_MCP_SUMMARY_FIELDS)
        }
        (McpPolicyActor::Agent, Surface::McpToolDetail) => {
            (McpRenderShape::Detail, AGENT_MCP_DETAIL_FIELDS)
        }
        (McpPolicyActor::User, Surface::McpTool) => {
            (McpRenderShape::Summary, USER_MCP_SUMMARY_FIELDS)
        }
        (McpPolicyActor::User, Surface::McpToolDetail) => {
            (McpRenderShape::Detail, USER_MCP_DETAIL_FIELDS)
        }
        (McpPolicyActor::Admin, Surface::McpTool) => {
            (McpRenderShape::Summary, ADMIN_MCP_SUMMARY_FIELDS)
        }
        (McpPolicyActor::Admin, Surface::McpToolDetail) => {
            (McpRenderShape::Detail, ADMIN_MCP_DETAIL_FIELDS)
        }
        _ => return None,
    };

    Some(McpRenderPolicy {
        actor,
        surface,
        shape,
        fields,
    })
}

fn mcp_policy_actor(actor: &Actor) -> Option<McpPolicyActor> {
    match actor {
        Actor::User => Some(McpPolicyActor::User),
        Actor::Agent { .. } => Some(McpPolicyActor::Agent),
        Actor::Human { role, .. } if role.eq_ignore_ascii_case("admin") => {
            Some(McpPolicyActor::Admin)
        }
        _ => None,
    }
}

fn render_mcp_fail_closed(surface: Surface) -> Value {
    json!({
        "kind": "provenance_redacted",
        "reason": "actor_surface_not_allowed",
        "surface": surface.as_str(),
    })
}

fn render_unparseable_legacy_provenance(actor: &Actor, surface: Surface) -> Value {
    let actor = hashed_actor_for_unparseable_legacy(actor);
    let surface = surface.as_str();
    json!({
        "actor": actor,
        "surface": surface,
        "status": "legacy_envelope_unparseable",
        "warning": format!("detail unavailable for actor {actor} on surface {surface}"),
    })
}

fn render_masked_provenance_for(
    masked: &ProvenanceMasked,
    actor: Actor,
    surface: Surface,
) -> RenderedProvenance {
    let value = match surface {
        Surface::TauriApp => render_masked_tauri_app(masked, &actor),
        Surface::McpTool | Surface::McpToolDetail => render_masked_mcp(masked, &actor, surface),
        Surface::P2Publication => render_masked_p2_publication(masked),
        Surface::LogStructured => render_masked_log_structured(masked),
    };
    enforce_render_budget(RenderedProvenance::new(surface, value))
}

fn render_masked_tauri_app(masked: &ProvenanceMasked, actor: &Actor) -> Value {
    let mut object = Map::new();
    object.insert("kind".to_string(), json!("provenance_masked"));
    object.insert("status".to_string(), json!("masked"));
    object.insert(
        "original_ability_name".to_string(),
        json!(masked.original_ability_name),
    );
    object.insert(
        "original_produced_at".to_string(),
        json!(masked.original_produced_at),
    );
    object.insert("masked_at".to_string(), json!(masked.masked_at));
    object.insert(
        "mask_reason".to_string(),
        render_provenance_mask_reason(&masked.mask_reason, actor, Surface::TauriApp),
    );
    object.insert(
        "sources_masked".to_string(),
        render_masked_sources(&masked.sources_masked, actor, Surface::TauriApp),
    );

    if matches!(
        projection_actor_class(actor),
        ProjectionActorClass::User | ProjectionActorClass::Human
    ) {
        object.insert(
            "original_invocation_id".to_string(),
            json!(masked.original_invocation_id),
        );
    }

    Value::Object(object)
}

fn render_masked_mcp(masked: &ProvenanceMasked, actor: &Actor, surface: Surface) -> Value {
    let Some(policy) = mcp_render_policy(actor, surface) else {
        return render_mcp_fail_closed(surface);
    };

    let mut object = Map::new();
    object.insert("kind".to_string(), json!("provenance_masked"));
    object.insert("status".to_string(), json!("masked"));
    object.insert("render_level".to_string(), json!(policy.shape.as_str()));
    object.insert("surface".to_string(), json!(policy.surface.as_str()));

    if policy.allows(McpProvenanceField::AbilityName) {
        object.insert(
            "original_ability_name".to_string(),
            json!(masked.original_ability_name),
        );
    }
    if policy.allows(McpProvenanceField::InvocationId) {
        object.insert(
            "original_invocation_id".to_string(),
            json!(masked.original_invocation_id),
        );
    }
    if policy.allows(McpProvenanceField::ProducedAt) {
        object.insert(
            "original_produced_at".to_string(),
            json!(masked.original_produced_at),
        );
    }
    if policy.allows(McpProvenanceField::RenderActor) {
        object.insert("render_actor".to_string(), json!(policy.actor.as_str()));
    }

    object.insert(
        "mask_reason".to_string(),
        render_provenance_mask_reason(&masked.mask_reason, actor, surface),
    );
    if policy.allows(McpProvenanceField::Sources) {
        object.insert(
            "sources_masked".to_string(),
            render_masked_sources(&masked.sources_masked, actor, surface),
        );
    } else if policy.allows(McpProvenanceField::DataSourcesSummary) {
        object.insert(
            "sources_masked".to_string(),
            json!({
                "count": masked.sources_masked.len(),
                "data_sources_summary": bounded_data_source_class_summary(&masked.sources_masked),
            }),
        );
    }

    Value::Object(object)
}

fn render_masked_p2_publication(masked: &ProvenanceMasked) -> Value {
    json!({
        "kind": "provenance_masked",
        "status": "masked",
        "mask_reason": render_provenance_mask_reason(&masked.mask_reason, &Actor::User, Surface::P2Publication),
        "source_classes": bounded_data_source_class_summary(&masked.sources_masked),
        "detail": {
            "included": false,
            "reason": "masked",
        },
    })
}

fn render_masked_log_structured(masked: &ProvenanceMasked) -> Value {
    json!({
        "kind": "provenance_masked",
        "status": "masked",
        "original_invocation_id": masked.original_invocation_id,
        "original_ability_name": masked.original_ability_name,
        "original_produced_at": masked.original_produced_at,
        "masked_at": masked.masked_at,
        "mask_reason": render_provenance_mask_reason(&masked.mask_reason, &Actor::System { component: "log".to_string() }, Surface::LogStructured),
        "sources_masked_count": masked.sources_masked.len(),
        "source_classes": bounded_data_source_class_summary(&masked.sources_masked),
    })
}

fn hashed_actor_for_unparseable_legacy(actor: &Actor) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"dailyos.provenance.legacy_actor.v1");
    hasher.update([0]);
    match serde_json::to_vec(actor) {
        Ok(bytes) => hasher.update(bytes),
        Err(_) => hasher.update(actor_class(actor).as_bytes()),
    }
    let digest = hasher.finalize();
    format!("sha256:{}", hex::encode(&digest[..8]))
}

fn render_p2_publication(
    prov: &Provenance,
    actor: &Actor,
    ctx: &mut RenderContext,
    confirmation: Option<&P2DetailConfirmationToken>,
) -> Value {
    let source_classes = source_class_summary(&prov.sources);
    let raw_footnote = format!("Based on {}.", human_join(&source_classes));
    let footnote = truncate_chars(&raw_footnote, P2_PUBLICATION_FOOTNOTE_BUDGET_CHARS);
    if raw_footnote.chars().count() > P2_PUBLICATION_FOOTNOTE_BUDGET_CHARS {
        ctx.warnings.push(ProvenanceWarning::TruncatedForRender {
            surface: Surface::P2Publication.as_str().to_string(),
            budget_bytes: P2_PUBLICATION_FOOTNOTE_BUDGET_CHARS,
            original_bytes: raw_footnote.len(),
        });
    }
    let detail_confirmed = confirmation
        .zip(p2_detail_confirmation_user_id(actor))
        .map(|(token, user_id)| token.is_valid_for_p2_detail(prov.invocation_id, user_id))
        .unwrap_or(false);
    let detail = if detail_confirmed {
        json!({
            "requires_user_confirmation": true,
            "included": true,
            "sources": render_sources_for_p2_detail(&prov.sources),
            "field_attributions": render_field_attributions_for_mcp(&prov.field_attributions, Surface::P2Publication, ctx),
        })
    } else {
        json!({
            "requires_user_confirmation": true,
            "included": false,
            "reason": "confirmation_required",
        })
    };
    json!({
        "footnote": footnote,
        "source_classes": source_classes,
        "detail": detail,
        "contains_ai_generated_sources": prov.sources.iter().any(|source| source.synthesis_marker.is_some()),
    })
}

fn p2_detail_confirmation_user_id(actor: &Actor) -> Option<&str> {
    match actor {
        Actor::User => Some(P2_DETAIL_DEFAULT_USER_ID),
        Actor::Human { id, .. } if !id.trim().is_empty() => Some(id.as_str()),
        _ => None,
    }
}

fn render_log_structured(prov: &Provenance) -> Value {
    json!({
        "invocation_id": prov.invocation_id,
        "ability_name": prov.ability_name,
        "ability_version": prov.ability_version,
        "ability_schema_version": prov.ability_schema_version,
        "produced_at": prov.produced_at,
        "source_count": prov.sources.len(),
        "child_count": prov.children.len(),
        "warning_count": prov.warnings.len(),
        "trust_effective": prov.trust.effective,
    })
}

fn render_sources_for_tauri(sources: &[SourceAttribution]) -> Vec<Value> {
    sources
        .iter()
        .map(|source| {
            let data_source_class = source_class_label(&source.data_source);
            json!({
                "data_source": data_source_class.as_str(),
                "data_source_class": data_source_class.as_str(),
                "data_source_display": data_source_class.as_str(),
                "identifiers": render_identifiers_for_tauri(&source.identifiers),
                "observed_at": source.observed_at,
                "source_asof": source.source_asof,
                "evidence_weight": source.evidence_weight,
                "scoring_class": source.scoring_class,
                "ai_generated": source.synthesis_marker.is_some(),
            })
        })
        .collect()
}

fn render_sources_for_mcp(sources: &[SourceAttribution]) -> Vec<Value> {
    sources
        .iter()
        .map(|source| {
            let data_source_class = source_class_label(&source.data_source);
            let redacted = SourceIdRedacted {
                data_source_class: data_source_class.clone(),
                scoring_class: source.scoring_class.clone(),
            };
            let identifiers = if source.identifiers.is_empty() {
                Vec::new()
            } else {
                vec![json!({
                    "kind": "source_id_redacted",
                    "data_source_class": redacted.data_source_class,
                    "scoring_class": redacted.scoring_class,
                })]
            };
            json!({
                "data_source": data_source_class.as_str(),
                "data_source_class": data_source_class.as_str(),
                "data_source_display": data_source_class.as_str(),
                "identifiers": identifiers,
                "observed_at": source.observed_at,
                "source_asof": source.source_asof,
                "evidence_weight": source.evidence_weight,
                "scoring_class": source.scoring_class,
                "ai_generated": source.synthesis_marker.is_some(),
            })
        })
        .collect()
}

fn render_sources_for_p2_detail(sources: &[SourceAttribution]) -> Vec<Value> {
    sources
        .iter()
        .map(|source| {
            let data_source_class = source_class_label(&source.data_source);
            json!({
                "data_source": data_source_class.as_str(),
                "data_source_class": data_source_class.as_str(),
                "data_source_display": data_source_class.as_str(),
                "observed_at": source.observed_at,
                "source_asof": source.source_asof,
                "scoring_class": source.scoring_class,
                "ai_generated": source.synthesis_marker.is_some(),
            })
        })
        .collect()
}

fn render_identifiers_for_tauri(identifiers: &[SourceIdentifier]) -> Vec<Value> {
    identifiers
        .iter()
        .filter_map(|identifier| match identifier {
            SourceIdentifier::ProviderCompletion { .. } => {
                Some(json!({ "kind": "provider_output_redacted" }))
            }
            other => serde_json::to_value(other).ok(),
        })
        .collect()
}

fn render_child_for_tauri(
    child: &ComposedProvenance,
    actor: &Actor,
    ctx: &mut RenderContext,
) -> Value {
    project_provenance_for_render(child.provenance.as_ref(), actor, Surface::TauriApp, ctx)
}

fn render_child_for_mcp(
    child: &ComposedProvenance,
    depth: usize,
    ctx: &mut RenderContext,
) -> Value {
    let composition_id = ctx.opaque_child_composition_label(&child.composition_id);
    if depth > 2 {
        let elided = ChildElided {
            ability_name: child.provenance.ability_name.clone(),
            data_sources_summary: source_class_summary(&child.provenance.sources),
        };
        return json!({
            "kind": "child_elided",
            "ability_name": elided.ability_name,
            "data_sources_summary": elided.data_sources_summary,
        });
    }

    json!({
        "composition_id": composition_id,
        "provenance_schema_version": child.provenance.provenance_schema_version,
        "ability_name": child.provenance.ability_name,
        "ability_version": child.provenance.ability_version,
        "ability_schema_version": child.provenance.ability_schema_version,
        "invocation_id": child.provenance.invocation_id,
        "produced_at": child.provenance.produced_at,
        "trust": render_mcp_trust(&child.provenance),
        "sources": render_sources_for_mcp(&child.provenance.sources),
            "children": child.provenance.children
                .iter()
                .map(|grandchild| render_child_for_mcp(grandchild, depth + 1, ctx))
                .collect::<Vec<_>>(),
        "field_attributions": render_field_attributions_for_mcp(&child.provenance.field_attributions, Surface::McpToolDetail, ctx),
        "subject": render_subject_redacted(),
    })
}

fn render_field_attributions(
    field_attributions: &std::collections::BTreeMap<FieldPath, FieldAttribution>,
    ctx: &mut RenderContext,
) -> Value {
    let mut rendered = Map::new();
    for (field_path, attribution) in field_attributions {
        let mut value = serde_json::to_value(attribution)
            .unwrap_or_else(|_| json!({ "error": "field_attribution_unserializable" }));
        if let Some(explanation) = attribution.explanation.as_ref() {
            let (sanitized, warning) =
                sanitize_explanation_for_render(field_path, explanation.as_str());
            if let Some(warning) = warning {
                ctx.warnings.push(warning);
            }
            if let Value::Object(object) = &mut value {
                object.insert("explanation".to_string(), Value::String(sanitized));
            }
        }
        rendered.insert(field_path.as_str().to_string(), value);
    }
    Value::Object(rendered)
}

fn render_field_attributions_for_mcp(
    field_attributions: &std::collections::BTreeMap<FieldPath, FieldAttribution>,
    surface: Surface,
    ctx: &mut RenderContext,
) -> Value {
    let mut value = render_field_attributions(field_attributions, ctx);
    redact_subjects_in_value(&mut value);
    if surface_redacts_composition_ids(surface) {
        redact_composition_ids_in_value(&mut value, ctx);
    }
    value
}

fn render_mcp_trust(prov: &Provenance) -> Value {
    json!({
        "effective": prov.trust.effective,
        "contains_stored_synthesis": prov.trust.contains_stored_synthesis,
    })
}

fn surface_redacts_composition_ids(surface: Surface) -> bool {
    matches!(
        surface,
        Surface::McpTool | Surface::McpToolDetail | Surface::P2Publication
    )
}

fn render_composition_id_for_surface(
    composition_id: &CompositionId,
    surface: Surface,
    ctx: &mut RenderContext,
) -> Value {
    if surface_redacts_composition_ids(surface) {
        Value::String(ctx.opaque_composition_label(composition_id))
    } else {
        json!(composition_id)
    }
}

fn redact_composition_ids_in_value(value: &mut Value, ctx: &mut RenderContext) {
    match value {
        Value::Array(items) => {
            for item in items {
                redact_composition_ids_in_value(item, ctx);
            }
        }
        Value::Object(object) => {
            if let Some(raw) = object
                .get("composition_id")
                .and_then(Value::as_str)
                .map(str::to_string)
            {
                object.insert(
                    "composition_id".to_string(),
                    Value::String(ctx.opaque_composition_label_for_raw(&raw)),
                );
            }
            for item in object.values_mut() {
                redact_composition_ids_in_value(item, ctx);
            }
        }
        _ => {}
    }
}

fn redact_subjects_in_value(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                redact_subjects_in_value(item);
            }
        }
        Value::Object(object) => {
            if object.contains_key("subject") {
                object.insert("subject".to_string(), render_subject_redacted());
            }
            if object.contains_key("competing_subjects") {
                object.insert("competing_subjects".to_string(), Value::Array(Vec::new()));
            }
            for item in object.values_mut() {
                redact_subjects_in_value(item);
            }
        }
        _ => {}
    }
}

fn redact_internal_mcp_values(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                redact_internal_mcp_values(item);
            }
        }
        Value::Object(object) => {
            for (key, item) in object.iter_mut() {
                let normalized = key.trim().to_ascii_lowercase();
                if matches!(
                    normalized.as_str(),
                    "signal_id"
                        | "entity_id"
                        | "document_id"
                        | "chunk_id"
                        | "context_entry_id"
                        | "assessment_id"
                        | "internal_id"
                ) {
                    *item = json!({ "kind": "source_id_redacted" });
                    continue;
                }
                if matches!(
                    normalized.as_str(),
                    "account" | "project" | "person" | "meeting" | "user"
                ) && item.is_string()
                {
                    *item = Value::String("[subject redacted]".to_string());
                    continue;
                }
                redact_internal_mcp_values(item);
            }
        }
        _ => {}
    }
}

fn sanitize_explanations_in_value(value: Value, ctx: &mut RenderContext) -> Value {
    fn walk(value: Value, path: &mut Vec<String>, ctx: &mut RenderContext) -> Value {
        match value {
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| {
                        path.push(index.to_string());
                        let rendered = walk(item, path, ctx);
                        path.pop();
                        rendered
                    })
                    .collect(),
            ),
            Value::Object(object) => {
                let mut rendered = Map::new();
                for (key, item) in object {
                    path.push(key.clone());
                    let item = if key == "explanation" {
                        match item {
                            Value::String(text) => {
                                let field = FieldPath::new(json_pointer_from_path(path))
                                    .unwrap_or_else(|_| FieldPath::root());
                                let (sanitized, warning) =
                                    sanitize_explanation_for_render(&field, &text);
                                if let Some(warning) = warning {
                                    ctx.warnings.push(warning);
                                }
                                Value::String(sanitized)
                            }
                            other => walk(other, path, ctx),
                        }
                    } else {
                        walk(item, path, ctx)
                    };
                    rendered.insert(key, item);
                    path.pop();
                }
                Value::Object(rendered)
            }
            other => other,
        }
    }

    walk(value, &mut Vec::new(), ctx)
}

fn attach_render_warnings(
    mut value: Value,
    prov: &Provenance,
    actor: &Actor,
    surface: Surface,
    render_warnings: Vec<ProvenanceWarning>,
    ctx: &mut RenderContext,
) -> Value {
    let mut warnings = Vec::new();
    for warning in prov.warnings.iter().chain(render_warnings.iter()) {
        warnings.push(render_warning_for_surface(
            warning, prov, actor, surface, ctx,
        ));
    }
    if let Value::Object(object) = &mut value {
        object.insert("warnings".to_string(), Value::Array(warnings));
    }
    value
}

fn sanitize_projected_warning_fields(
    value: &mut Value,
    prov: &Provenance,
    actor: &Actor,
    surface: Surface,
    ctx: &mut RenderContext,
) {
    let Value::Object(object) = value else {
        return;
    };

    if object.contains_key("warnings") {
        object.insert(
            "warnings".to_string(),
            Value::Array(
                prov.warnings
                    .iter()
                    .map(|warning| render_warning_for_surface(warning, prov, actor, surface, ctx))
                    .collect(),
            ),
        );
    }

    if let Some(Value::Array(children)) = object.get_mut("children") {
        for (child_value, child) in children.iter_mut().zip(prov.children.iter()) {
            sanitize_projected_warning_fields(
                child_value,
                child.provenance.as_ref(),
                actor,
                surface,
                ctx,
            );
            if let Some(provenance_value) = child_value.get_mut("provenance") {
                sanitize_projected_warning_fields(
                    provenance_value,
                    child.provenance.as_ref(),
                    actor,
                    surface,
                    ctx,
                );
            }
        }
    }

    if let Some(provenance_value) = object.get_mut("provenance") {
        sanitize_projected_warning_fields(provenance_value, prov, actor, surface, ctx);
    }
}

fn render_warning_for_surface(
    warning: &ProvenanceWarning,
    prov: &Provenance,
    actor: &Actor,
    surface: Surface,
    ctx: &mut RenderContext,
) -> Value {
    let include_full_details = allows_full_warning_details(actor, surface);
    let include_composition_id = include_full_details
        || matches!(surface, Surface::McpToolDetail)
            && mcp_render_policy(actor, surface)
                .is_some_and(|policy| policy.allows(McpProvenanceField::Children));

    match warning {
        ProvenanceWarning::DepthElided {
            skipped_levels,
            elided_children,
            aggregate_source_count,
            effective_trust,
        } => json!({
            "kind": "depth_elided",
            "skipped_levels": skipped_levels,
            "elided_children": elided_children,
            "aggregate_source_count": aggregate_source_count,
            "effective_trust": effective_trust,
        }),
        ProvenanceWarning::SourceStale {
            source_index,
            age_seconds,
        } => {
            let mut object = warning_object("source_stale");
            object.insert(
                "source_class".to_string(),
                json!(warning_source_class(prov, *source_index)),
            );
            object.insert("age_seconds".to_string(), json!(age_seconds));
            if include_full_details {
                object.insert("source_index".to_string(), json!(source_index));
            }
            Value::Object(object)
        }
        ProvenanceWarning::SourceUnresolvable {
            source_index,
            reason,
        } => {
            let mut object = warning_object("source_unresolvable");
            object.insert(
                "source_class".to_string(),
                json!(warning_source_class(prov, *source_index)),
            );
            if include_full_details {
                object.insert("source_index".to_string(), json!(source_index));
                object.insert("reason".to_string(), json!(reason));
            }
            Value::Object(object)
        }
        ProvenanceWarning::AttributionIncomplete { field } => {
            let mut object = warning_object("attribution_incomplete");
            object.insert("field".to_string(), json!(field));
            Value::Object(object)
        }
        ProvenanceWarning::SourceRevoked => Value::Object(warning_object("source_revoked")),
        ProvenanceWarning::Masked { reason } => {
            let mut object = warning_object("masked");
            object.insert(
                "reason".to_string(),
                render_warning_mask_reason(reason, include_full_details),
            );
            Value::Object(object)
        }
        ProvenanceWarning::SourceTimestampUnknown {
            source_index,
            fallback,
        } => {
            let mut object = warning_object("source_timestamp_unknown");
            object.insert(
                "source_class".to_string(),
                json!(warning_source_class(prov, *source_index)),
            );
            object.insert("fallback".to_string(), json!(fallback));
            if include_full_details {
                object.insert("source_index".to_string(), json!(source_index));
            }
            Value::Object(object)
        }
        ProvenanceWarning::SourceTimestampImplausible {
            source_index,
            reason,
        } => {
            let mut object = warning_object("source_timestamp_implausible");
            object.insert(
                "source_class".to_string(),
                json!(warning_source_class(prov, *source_index)),
            );
            if include_full_details {
                object.insert("source_index".to_string(), json!(source_index));
                object.insert("reason".to_string(), json!(reason));
            }
            Value::Object(object)
        }
        ProvenanceWarning::SubjectFitQualified { field, status } => {
            let mut object = warning_object("subject_fit_qualified");
            object.insert("status".to_string(), json!(status));
            if let Some(field) = field {
                object.insert("field".to_string(), json!(field));
            }
            Value::Object(object)
        }
        ProvenanceWarning::OptionalComposedReadFailed {
            composition_id,
            reason,
        } => {
            let mut object = warning_object("optional_composed_read_failed");
            if include_composition_id {
                object.insert(
                    "composition_id".to_string(),
                    render_composition_id_for_surface(composition_id, surface, ctx),
                );
            }
            if include_full_details {
                object.insert("reason".to_string(), json!(reason));
            }
            Value::Object(object)
        }
        ProvenanceWarning::SoftSizeLimitExceeded {
            bytes,
            soft_budget_bytes,
        } => json!({
            "kind": "soft_size_limit_exceeded",
            "bytes": bytes,
            "soft_budget_bytes": soft_budget_bytes,
        }),
        ProvenanceWarning::ExplanationFiltered { field, reason } => {
            let mut object = warning_object("explanation_filtered");
            object.insert("field".to_string(), json!(field));
            if include_full_details {
                object.insert("reason".to_string(), json!(reason));
            }
            Value::Object(object)
        }
        ProvenanceWarning::TruncatedForRender {
            surface,
            budget_bytes,
            original_bytes,
        } => truncated_for_render_warning_value(surface, *budget_bytes, *original_bytes),
    }
}

fn warning_object(kind: &str) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("kind".to_string(), json!(kind));
    object
}

fn allows_full_warning_details(actor: &Actor, surface: Surface) -> bool {
    matches!(surface, Surface::TauriApp)
        && matches!(
            projection_actor_class(actor),
            ProjectionActorClass::User | ProjectionActorClass::Human
        )
}

fn warning_source_class(
    prov: &Provenance,
    source_index: crate::abilities::provenance::SourceIndex,
) -> String {
    prov.sources
        .get(source_index.as_usize())
        .map(|source| canonical_data_source_class(&source.data_source))
        .unwrap_or_else(|| "unknown".to_string())
}

fn render_warning_mask_reason(
    reason: &crate::abilities::provenance::MaskReason,
    include_full_details: bool,
) -> Value {
    match reason {
        crate::abilities::provenance::MaskReason::SourceRevoked => {
            json!({ "kind": "source_revoked" })
        }
        crate::abilities::provenance::MaskReason::ActorNotAuthorized => {
            json!({ "kind": "actor_not_authorized" })
        }
        crate::abilities::provenance::MaskReason::Sensitive => json!({ "kind": "sensitive" }),
        crate::abilities::provenance::MaskReason::Other(reason) => {
            if include_full_details {
                json!({ "kind": "other", "reason": reason })
            } else {
                json!({ "kind": "other" })
            }
        }
    }
}

fn enforce_render_budget(mut rendered: RenderedProvenance) -> RenderedProvenance {
    let Some(budget) = rendered.surface.byte_budget() else {
        return rendered;
    };
    let Ok(original_bytes) = rendered.serialized_len() else {
        return rendered;
    };
    if original_bytes <= budget {
        return rendered;
    }

    add_truncation_warning(
        &mut rendered.value,
        rendered.surface,
        budget,
        original_bytes,
    );
    shrink_for_budget(&mut rendered.value);

    if rendered
        .serialized_len()
        .is_ok_and(|current_bytes| current_bytes <= budget)
    {
        return rendered;
    }

    rendered.value = json!({
        "summary": "Provenance truncated for render budget",
        "warnings": [truncated_for_render_warning_value(rendered.surface.as_str(), budget, original_bytes)],
    });
    rendered
}

fn add_truncation_warning(
    value: &mut Value,
    surface: Surface,
    budget_bytes: usize,
    original_bytes: usize,
) {
    let warning =
        truncated_for_render_warning_value(surface.as_str(), budget_bytes, original_bytes);

    let Value::Object(object) = value else {
        return;
    };
    match object.get_mut("warnings") {
        Some(Value::Array(warnings)) => warnings.push(warning),
        _ => {
            object.insert("warnings".to_string(), Value::Array(vec![warning]));
        }
    }
}

fn truncated_for_render_warning_value(
    surface: &str,
    budget_bytes: usize,
    original_bytes: usize,
) -> Value {
    json!({
        "kind": "truncated_for_render",
        "surface": surface,
        "budget_bytes": budget_bytes,
        "original_bytes": original_bytes,
    })
}

fn shrink_for_budget(value: &mut Value) {
    let Value::Object(object) = value else {
        return;
    };

    if let Some(Value::Object(about_this)) = object.get_mut("about_this") {
        about_this.remove("full_json");
        about_this.remove("details");
        about_this.insert("details_available".to_string(), Value::Bool(true));
        about_this.insert("full_json_available".to_string(), Value::Bool(true));
    }

    object.remove("children");
    object.remove("sources");
    truncate_object(object, "field_attributions", 16);
}

fn truncate_object(object: &mut Map<String, Value>, key: &str, max_items: usize) {
    let Some(Value::Object(values)) = object.get_mut(key) else {
        return;
    };
    if values.len() <= max_items {
        return;
    }

    let keep = values
        .iter()
        .take(max_items)
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Map<_, _>>();
    *values = keep;
    values.insert(
        "__elided__".to_string(),
        json!({
            "kind": "items_elided",
        }),
    );
}

fn source_class_summary(sources: &[SourceAttribution]) -> Vec<String> {
    sources
        .iter()
        .map(|source| source_class_label(&source.data_source))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn data_source_class_summary(data_sources: &[DataSource]) -> Vec<String> {
    data_sources
        .iter()
        .map(canonical_data_source_class)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn source_class_label(data_source: &DataSource) -> String {
    match data_source {
        DataSource::User => "user_provided".to_string(),
        DataSource::Other(_) => "extracted".to_string(),
        _ => data_source.display_name(),
    }
}

fn canonical_data_source_class(data_source: &DataSource) -> String {
    match data_source {
        DataSource::User => "user".to_string(),
        DataSource::Google => "google".to_string(),
        DataSource::Glean { .. } => "glean".to_string(),
        DataSource::Clay => "clay".to_string(),
        DataSource::Ai => "ai".to_string(),
        DataSource::CoAttendance => "co_attendance".to_string(),
        DataSource::LocalEnrichment => "local_enrichment".to_string(),
        DataSource::Other(_) => "extracted".to_string(),
        DataSource::LegacyUnattributed => "legacy_unattributed".to_string(),
    }
}

fn rendered_data_source_class_value(value: &Value) -> Value {
    serde_json::from_value::<DataSource>(value.clone())
        .map(|data_source| Value::String(source_class_label(&data_source)))
        .unwrap_or_else(|_| Value::String("source".to_string()))
}

fn bounded_source_class_summary(sources: &[SourceAttribution]) -> Vec<String> {
    let mut summary = source_class_summary(sources);
    if summary.len() > MCP_SUMMARY_SOURCE_CLASS_LIMIT {
        summary.truncate(MCP_SUMMARY_SOURCE_CLASS_LIMIT);
        summary.push("additional sources elided".to_string());
    }
    summary
}

fn bounded_data_source_class_summary(data_sources: &[DataSource]) -> Vec<String> {
    let mut summary = data_source_class_summary(data_sources);
    if summary.len() > MCP_SUMMARY_SOURCE_CLASS_LIMIT {
        summary.truncate(MCP_SUMMARY_SOURCE_CLASS_LIMIT);
        summary.push("additional sources elided".to_string());
    }
    summary
}

fn render_masked_sources(data_sources: &[DataSource], actor: &Actor, surface: Surface) -> Value {
    let include_display = matches!(surface, Surface::TauriApp)
        && matches!(
            projection_actor_class(actor),
            ProjectionActorClass::User | ProjectionActorClass::Human
        );

    Value::Array(
        data_sources
            .iter()
            .map(|data_source| {
                let mut object = Map::new();
                object.insert(
                    "source_class".to_string(),
                    json!(canonical_data_source_class(data_source)),
                );
                if include_display {
                    object.insert(
                        "data_source_display".to_string(),
                        json!(source_class_label(data_source)),
                    );
                }
                Value::Object(object)
            })
            .collect(),
    )
}

fn render_provenance_mask_reason(
    reason: &ProvenanceMaskReason,
    actor: &Actor,
    surface: Surface,
) -> Value {
    let include_display = matches!(surface, Surface::TauriApp)
        && matches!(
            projection_actor_class(actor),
            ProjectionActorClass::User | ProjectionActorClass::Human
        );

    match reason {
        ProvenanceMaskReason::SourceRevoked { data_source } => {
            let mut object = Map::new();
            object.insert("kind".to_string(), json!("source_revoked"));
            object.insert(
                "source_class".to_string(),
                json!(canonical_data_source_class(data_source)),
            );
            if include_display {
                object.insert(
                    "data_source_display".to_string(),
                    json!(source_class_label(data_source)),
                );
            }
            Value::Object(object)
        }
        ProvenanceMaskReason::GleanDisconnected => {
            json!({ "kind": "glean_disconnected", "source_class": "glean" })
        }
        ProvenanceMaskReason::UserDeletedEntry => {
            json!({ "kind": "user_deleted_entry", "source_class": "user" })
        }
        ProvenanceMaskReason::RetentionExpired => json!({ "kind": "retention_expired" }),
    }
}

fn composition_depth(prov: &Provenance) -> usize {
    prov.children
        .iter()
        .map(|child| 1 + composition_depth(&child.provenance))
        .max()
        .unwrap_or(0)
}

fn actor_class(actor: &Actor) -> &'static str {
    match actor {
        Actor::User => "user",
        Actor::Agent { .. } => "agent",
        Actor::System { .. } => "system",
        Actor::Human { .. } => "human",
        Actor::External { .. } => "external",
    }
}

fn render_subject_redacted() -> Value {
    json!({
        "kind": "subject_redacted",
    })
}

fn human_join(items: &[String]) -> String {
    match items {
        [] => "available sources".to_string(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        many => {
            let (last, rest) = many.split_last().expect("many is non-empty");
            format!("{}, and {last}", rest.join(", "))
        }
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let mut rendered = value.chars().take(keep).collect::<String>();
    rendered.push_str("...");
    rendered
}

fn html_entity_encode(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => rendered.push_str("&amp;"),
            '<' => rendered.push_str("&lt;"),
            '>' => rendered.push_str("&gt;"),
            '"' => rendered.push_str("&quot;"),
            '\'' => rendered.push_str("&#x27;"),
            _ => rendered.push(ch),
        }
    }
    rendered
}

fn strip_markdown_line_markers(value: &str) -> String {
    value
        .lines()
        .map(|line| markdown_line_marker_regex().replace(line, ""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_line_marker_regex() -> &'static Regex {
    static MARKDOWN_LINE_MARKER_RE: OnceLock<Regex> = OnceLock::new();
    MARKDOWN_LINE_MARKER_RE.get_or_init(|| {
        Regex::new(r"^\s{0,3}(?:#{1,6}\s*|[-*+]\s+|\d{1,3}[.)]\s+)")
            .expect("static markdown line marker sanitizer regex compiles")
    })
}

fn markdown_link_regex() -> &'static Regex {
    static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();
    MARKDOWN_LINK_RE.get_or_init(|| {
        Regex::new(r#"!?\[([^\]\n]*)\]\([^\s)\n]+(?:\s+"[^"\n]*")?\)"#)
            .expect("static markdown link sanitizer regex compiles")
    })
}

fn uri_scheme_regex() -> &'static Regex {
    static URI_SCHEME_RE: OnceLock<Regex> = OnceLock::new();
    URI_SCHEME_RE.get_or_init(|| {
        Regex::new(r"(?i)\b[a-z][a-z0-9+.-]*:[^\s<>\]]+")
            .expect("static URI scheme sanitizer regex compiles")
    })
}

fn bare_domain_regex() -> &'static Regex {
    static BARE_DOMAIN_RE: OnceLock<Regex> = OnceLock::new();
    BARE_DOMAIN_RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}(?::\d{2,5})?(?:/[^\s<>\]]*)?",
        )
        .expect("static bare domain sanitizer regex compiles")
    })
}

fn banned_explanation_tokens() -> &'static [&'static str] {
    &[
        "ignore previous",
        "disregard previous",
        "forget previous",
        "you are now",
        "system:",
        "developer:",
        "assistant:",
        "user:",
        "new instructions",
        "prompt injection",
        "jailbreak",
        "<script",
    ]
}

fn json_pointer_from_path(path: &[String]) -> String {
    path.iter().fold(String::new(), |mut pointer, segment| {
        pointer.push('/');
        pointer.push_str(&segment.replace('~', "~0").replace('/', "~1"));
        pointer
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::TimeZone;
    use serde_json::json;

    use super::*;
    use crate::abilities::provenance::{
        provenance_for_test, Confidence, DerivationKind, FieldAttribution, GleanDownstream,
        PromptFingerprint, PromptTemplateId, PromptVersion, ProviderRef, SanitizedExplanation,
        SourceIdentifier, SourceName, SourceRef, SubjectAttribution, SubjectRef,
    };

    fn produced_at() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap()
    }

    fn subject() -> SubjectAttribution {
        SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()))
    }

    fn source() -> SourceAttribution {
        SourceAttribution::new(
            DataSource::Glean {
                downstream: GleanDownstream::Slack,
            },
            vec![
                SourceIdentifier::Signal {
                    signal_id: crate::abilities::provenance::SignalId::new("sig-1"),
                },
                SourceIdentifier::Entity {
                    entity_id: crate::abilities::provenance::EntityId::new("acct-1"),
                    field: Some("health".to_string()),
                },
                SourceIdentifier::ProviderCompletion {
                    completion_id: "completion-secret-214".to_string(),
                    provider: ProviderRef::new("provider-secret-214"),
                },
            ],
            produced_at(),
            Some(produced_at()),
            0.8,
            None,
        )
        .unwrap()
    }

    fn other_source(name: &str) -> SourceAttribution {
        SourceAttribution::new(
            DataSource::Other(SourceName::new(name)),
            Vec::new(),
            produced_at(),
            Some(produced_at()),
            0.4,
            None,
        )
        .unwrap()
    }

    fn provenance() -> Provenance {
        let explanation =
            SanitizedExplanation::new("Based on <meeting> and https://example.com/detail.")
                .unwrap();
        provenance_for_test(
            "fixture",
            produced_at(),
            subject(),
            vec![source()],
            Vec::new(),
            BTreeMap::from([(
                FieldPath::new("/summary").unwrap(),
                FieldAttribution::new(
                    subject(),
                    DerivationKind::Direct,
                    vec![SourceRef::Source {
                        source_index: crate::abilities::provenance::SourceIndex(0),
                    }],
                    Confidence::declared(0.8, &explanation).unwrap(),
                    Some(explanation),
                )
                .unwrap(),
            )]),
            Some(PromptFingerprint {
                provider: "replay".to_string(),
                model: crate::abilities::provenance::ModelName("model".to_string()),
                prompt_template_id: PromptTemplateId("template".to_string()),
                prompt_template_version: PromptVersion("1".to_string()),
                canonical_prompt_hash: crate::abilities::provenance::HashValue::new("secret"),
                temperature: 0.0,
                top_p: None,
                seed: Some(42),
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: Some("provider-completion-secret-214".to_string()),
            }),
            Vec::new(),
        )
    }

    fn human_actor(user_id: &str) -> Actor {
        Actor::Human {
            role: "user".to_string(),
            id: user_id.to_string(),
        }
    }

    fn valid_p2_detail_token(provenance: &Provenance, user_id: &str) -> P2DetailConfirmationToken {
        P2DetailConfirmationToken::issue(
            P2_DETAIL_CONFIRMATION_AUDIENCE,
            provenance.invocation_id,
            user_id,
            60,
        )
        .unwrap()
    }

    #[test]
    fn mcp_render_redacts_internal_source_ids_and_prompt_seed() {
        let rendered = render_provenance_for(
            &provenance(),
            Actor::Agent {
                name: "mcp".to_string(),
                version: "test".to_string(),
            },
            Surface::McpToolDetail,
        );
        let serialized = serde_json::to_string(&rendered).unwrap();

        assert!(!serialized.contains("sig-1"));
        assert!(!serialized.contains("acct-1"));
        assert!(!serialized.contains("canonical_prompt_hash"));
        assert!(!serialized.contains("seed"));
        assert!(!serialized.contains("completion-secret-214"));
        assert!(!serialized.contains("provider-completion-secret-214"));
        assert!(serialized.contains("source_id_redacted"));
    }

    #[test]
    fn mcp_and_p2_detail_use_opaque_composition_labels() {
        let secret_composition_id = CompositionId::new("get_entity_context:account:acct-secret");
        let mut child = provenance();
        child.field_attributions.insert(
            FieldPath::new("/child_composed").unwrap(),
            FieldAttribution::composed(
                subject(),
                secret_composition_id.clone(),
                FieldPath::new("/summary").unwrap(),
                Confidence::composed_min(0.8).unwrap(),
            )
            .unwrap(),
        );

        let mut provenance = provenance();
        provenance.children.push(ComposedProvenance::new(
            secret_composition_id.clone(),
            child,
        ));
        provenance.field_attributions.insert(
            FieldPath::new("/child_summary").unwrap(),
            FieldAttribution::composed(
                subject(),
                secret_composition_id,
                FieldPath::new("/summary").unwrap(),
                Confidence::composed_min(0.8).unwrap(),
            )
            .unwrap(),
        );

        let mcp_rendered = render_provenance_for(
            &provenance,
            Actor::Agent {
                name: "mcp".to_string(),
                version: "test".to_string(),
            },
            Surface::McpToolDetail,
        );
        let mcp_serialized = serde_json::to_string(&mcp_rendered).unwrap();

        assert!(!mcp_serialized.contains("account:acct-secret"));
        assert_eq!(
            mcp_rendered.value["children"][0]["composition_id"],
            json!("c1")
        );
        assert_eq!(
            mcp_rendered
                .value
                .pointer("/field_attributions/~1child_summary/derivation/composed/composition_id")
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(
            mcp_rendered
                .value
                .pointer("/field_attributions/~1child_summary/source_refs/0/child/composition_id")
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(
            mcp_rendered
                .value
                .pointer("/children/0/field_attributions/~1child_composed/derivation/composed/composition_id")
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(
            mcp_rendered
                .value
                .pointer("/children/0/field_attributions/~1child_composed/source_refs/0/child/composition_id")
                .cloned(),
            Some(json!("c1"))
        );

        let p2_token = valid_p2_detail_token(&provenance, "user-1");
        let p2_rendered = render_provenance_for_with_options(
            &provenance,
            human_actor("user-1"),
            Surface::P2Publication,
            RenderOptions::with_p2_detail_confirmation(p2_token),
        );
        let p2_serialized = serde_json::to_string(&p2_rendered).unwrap();

        assert!(!p2_serialized.contains("account:acct-secret"));
        assert_eq!(
            p2_rendered
                .value
                .pointer(
                    "/detail/field_attributions/~1child_summary/derivation/composed/composition_id"
                )
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(
            p2_rendered
                .value
                .pointer(
                    "/detail/field_attributions/~1child_summary/source_refs/0/child/composition_id"
                )
                .cloned(),
            Some(json!("c1"))
        );
    }

    #[test]
    fn serialized_mcp_detail_keeps_child_and_refs_on_same_opaque_label() {
        let secret_composition_id = CompositionId::new("get_entity_context:account:acct-secret");
        let mut child = provenance();
        child.ability_name = "get_entity_context".to_string();
        child.field_attributions.insert(
            FieldPath::new("/child_composed").unwrap(),
            FieldAttribution::composed(
                subject(),
                secret_composition_id.clone(),
                FieldPath::new("/summary").unwrap(),
                Confidence::composed_min(0.8).unwrap(),
            )
            .unwrap(),
        );

        let mut provenance = provenance();
        provenance.children.push(ComposedProvenance::new(
            secret_composition_id.clone(),
            child,
        ));
        provenance.field_attributions.insert(
            FieldPath::new("/child_summary").unwrap(),
            FieldAttribution::composed(
                subject(),
                secret_composition_id.clone(),
                FieldPath::new("/summary").unwrap(),
                Confidence::composed_min(0.8).unwrap(),
            )
            .unwrap(),
        );
        provenance
            .warnings
            .push(ProvenanceWarning::OptionalComposedReadFailed {
                composition_id: secret_composition_id.clone(),
                reason: "private child read failure token-214".to_string(),
            });

        let serialized_provenance = serde_json::to_value(&provenance).unwrap();
        assert_eq!(
            serialized_provenance["children"][0]["ability_name"],
            json!("get_entity_context")
        );
        assert!(serialized_provenance["children"][0]
            .get("composition_id")
            .is_none());

        let rendered = render_serialized_provenance_for(
            serialized_provenance,
            Actor::Agent {
                name: "mcp".to_string(),
                version: "test".to_string(),
            },
            Surface::McpToolDetail,
        );
        let rendered_serialized = serde_json::to_string(&rendered).unwrap();

        assert!(!rendered_serialized.contains(secret_composition_id.as_str()));
        assert_eq!(rendered.value["children"][0]["composition_id"], json!("c1"));
        assert_eq!(
            rendered
                .value
                .pointer("/field_attributions/~1child_summary/derivation/composed/composition_id")
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(
            rendered
                .value
                .pointer("/field_attributions/~1child_summary/source_refs/0/child/composition_id")
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(
            rendered
                .value
                .pointer("/children/0/field_attributions/~1child_composed/derivation/composed/composition_id")
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(
            rendered
                .value
                .pointer("/children/0/field_attributions/~1child_composed/source_refs/0/child/composition_id")
                .cloned(),
            Some(json!("c1"))
        );
        assert_eq!(rendered.value["warnings"][0]["composition_id"], json!("c1"));
    }

    #[test]
    fn mcp_policy_matrix_filters_by_actor_and_surface() {
        let provenance = provenance();
        let agent_summary = render_provenance_for(
            &provenance,
            Actor::Agent {
                name: "mcp".to_string(),
                version: "test".to_string(),
            },
            Surface::McpTool,
        );
        let agent_detail = render_provenance_for(
            &provenance,
            Actor::Agent {
                name: "mcp".to_string(),
                version: "test".to_string(),
            },
            Surface::McpToolDetail,
        );
        let user_detail = render_provenance_for(&provenance, Actor::User, Surface::McpToolDetail);
        let admin_detail = render_provenance_for(
            &provenance,
            Actor::Human {
                role: "admin".to_string(),
                id: "admin-1".to_string(),
            },
            Surface::McpToolDetail,
        );
        let unsupported = render_provenance_for(
            &provenance,
            Actor::System {
                component: "batch".to_string(),
            },
            Surface::McpToolDetail,
        );

        assert_eq!(agent_summary.value["render_level"], json!("summary"));
        assert!(agent_summary.value.get("sources").is_none());
        assert!(agent_summary.value.get("field_attributions").is_none());
        assert!(
            agent_summary.serialized_len().unwrap() < MCP_DEFAULT_PROVENANCE_BUDGET_BYTES,
            "MCP tool summary should fit within the default budget"
        );

        assert_eq!(agent_detail.value["render_level"], json!("detail"));
        assert!(agent_detail.value.get("sources").is_some());
        assert!(agent_detail.value.get("field_attributions").is_some());
        assert!(agent_detail.value.get("prompt_fingerprint").is_none());

        assert_eq!(user_detail.value["render_actor"], json!("user"));
        assert!(user_detail.value.get("invocation_id").is_none());
        assert!(user_detail.value.get("prompt_fingerprint").is_none());

        assert_eq!(admin_detail.value["render_actor"], json!("admin"));
        assert!(admin_detail.value.get("invocation_id").is_some());
        assert!(admin_detail.value.get("prompt_fingerprint").is_none());

        assert_eq!(unsupported.value["kind"], json!("provenance_redacted"));
        assert!(unsupported.value.get("ability_name").is_none());
    }

    #[test]
    fn surface_actor_render_tuples_never_emit_provenance_secrets() {
        let mut prov = provenance();
        prov.sources.push(other_source("secret-source-name-214"));
        let mut child = provenance();
        child
            .sources
            .push(other_source("child-secret-source-name-214"));
        prov.children.push(ComposedProvenance::new(
            crate::abilities::provenance::CompositionId::new("child"),
            child,
        ));

        let agent = Actor::Agent {
            name: "mcp".to_string(),
            version: "test".to_string(),
        };
        let p2_token = valid_p2_detail_token(&prov, "user-1");
        let cases = vec![
            (
                "mcp_tool_agent",
                render_provenance_for(&prov, agent.clone(), Surface::McpTool),
            ),
            (
                "mcp_tool_detail_agent",
                render_provenance_for(&prov, agent.clone(), Surface::McpToolDetail),
            ),
            (
                "tauri_app_user",
                render_provenance_for(&prov, Actor::User, Surface::TauriApp),
            ),
            (
                "p2_publication_user",
                render_provenance_for(&prov, Actor::User, Surface::P2Publication),
            ),
            (
                "p2_publication_detail_user",
                render_provenance_for_with_options(
                    &prov,
                    human_actor("user-1"),
                    Surface::P2Publication,
                    RenderOptions::with_p2_detail_confirmation(p2_token),
                ),
            ),
            (
                "log_structured_agent",
                render_provenance_for(&prov, agent, Surface::LogStructured),
            ),
        ];

        for (label, rendered) in cases {
            assert_render_omits_forbidden_provenance_fields(label, &rendered);
        }
    }

    fn assert_render_omits_forbidden_provenance_fields(label: &str, rendered: &RenderedProvenance) {
        let serialized = serde_json::to_string(rendered).unwrap();
        for forbidden in [
            "prompt_fingerprint",
            "canonical_prompt_hash",
            "seed",
            "provider_completion_id",
            "completion_id",
            "completion-secret-214",
            "provider-completion-secret-214",
            "secret-source-name-214",
            "child-secret-source-name-214",
            "source_name",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "{label} leaked {forbidden}: {serialized}"
            );
        }
    }

    #[test]
    fn explanation_sanitizer_encodes_strips_urls_and_bounds() {
        let rendered = render_provenance_for(&provenance(), Actor::User, Surface::TauriApp);
        let explanation = rendered.value["field_attributions"]["/summary"]["explanation"]
            .as_str()
            .unwrap();

        assert!(explanation.contains("&lt;meeting&gt;"));
        assert!(explanation.contains("[url removed]"));
        assert!(!explanation.contains("https://example.com"));
    }

    #[test]
    fn explanation_sanitizer_strips_unsafe_schemes_domains_and_markdown_links() {
        let field = FieldPath::new("/summary").unwrap();
        let cases = [
            ("javascript:alert(1)", "javascript:"),
            ("mailto:person@example.com", "mailto:"),
            ("See example.com/private for details", "example.com"),
            (
                "Review [source](https://example.com/detail)",
                "https://example.com",
            ),
        ];

        for (raw, forbidden) in cases {
            let (sanitized, warning) = sanitize_explanation_for_render(&field, raw);
            assert_eq!(warning, None);
            assert!(
                sanitized.contains("[url removed]"),
                "expected URL sentinel for {raw}: {sanitized}"
            );
            assert!(
                !sanitized.contains(forbidden),
                "sanitized explanation still contains {forbidden}: {sanitized}"
            );
            assert!(!sanitized.contains("]("));
        }
    }

    #[test]
    fn explanation_sanitizer_strips_markdown_code_and_line_markers() {
        let field = FieldPath::new("/summary").unwrap();
        let (sanitized, warning) = sanitize_explanation_for_render(
            &field,
            "# `Heading`\n- Visit ftp://files.example.com/archive",
        );

        assert_eq!(warning, None);
        assert!(!sanitized.contains('`'));
        assert!(!sanitized.contains('#'));
        assert!(!sanitized.contains("- Visit"));
        assert!(!sanitized.contains("ftp://"));
        assert!(sanitized.contains("[url removed]"));
    }

    #[test]
    fn warning_projection_redacts_mcp_reasons_and_keeps_tauri_user_reason() {
        let mut provenance = provenance();
        provenance.warnings = vec![
            ProvenanceWarning::SourceUnresolvable {
                source_index: crate::abilities::provenance::SourceIndex(0),
                reason: "private resolver path /internal/customer-214".to_string(),
            },
            ProvenanceWarning::OptionalComposedReadFailed {
                composition_id: crate::abilities::provenance::CompositionId::new("child"),
                reason: "private child read failure token-214".to_string(),
            },
        ];

        let mcp_rendered = render_provenance_for(
            &provenance,
            Actor::Agent {
                name: "mcp".to_string(),
                version: "test".to_string(),
            },
            Surface::McpToolDetail,
        );
        let mcp_serialized = serde_json::to_string(&mcp_rendered).unwrap();

        assert_eq!(
            mcp_rendered.value["warnings"][0],
            json!({
                "kind": "source_unresolvable",
                "source_class": "glean",
            })
        );
        assert_eq!(
            mcp_rendered.value["warnings"][1]["kind"],
            json!("optional_composed_read_failed")
        );
        assert!(!mcp_serialized.contains("private resolver path"));
        assert!(!mcp_serialized.contains("private child read failure"));
        assert!(!mcp_serialized.contains("token-214"));

        let tauri_rendered = render_provenance_for(&provenance, Actor::User, Surface::TauriApp);
        assert_eq!(
            tauri_rendered.value["warnings"][0]["reason"],
            json!("private resolver path /internal/customer-214")
        );
        assert_eq!(
            tauri_rendered.value["warnings"][1]["reason"],
            json!("private child read failure token-214")
        );
    }

    #[test]
    fn serialized_masked_envelope_renders_mask_metadata_without_legacy_fallback() {
        let masked = ProvenanceMasked {
            original_invocation_id: provenance().invocation_id,
            original_ability_name: "fixture".to_string(),
            original_produced_at: produced_at(),
            masked_at: produced_at(),
            mask_reason: ProvenanceMaskReason::SourceRevoked {
                data_source: DataSource::Other(SourceName::new("private-source-name-214")),
            },
            sources_masked: vec![
                DataSource::Glean {
                    downstream: GleanDownstream::Slack,
                },
                DataSource::Other(SourceName::new("private-source-name-214")),
            ],
        };
        let envelope = serde_json::to_value(ProvenanceOrMasked::Masked(masked)).unwrap();

        let rendered = render_serialized_provenance_for(
            envelope,
            Actor::Agent {
                name: "mcp".to_string(),
                version: "test".to_string(),
            },
            Surface::McpToolDetail,
        );
        let serialized = serde_json::to_string(&rendered).unwrap();

        assert_eq!(rendered.value["kind"], json!("provenance_masked"));
        assert_eq!(rendered.value["status"], json!("masked"));
        assert_eq!(
            rendered.value["mask_reason"],
            json!({
                "kind": "source_revoked",
                "source_class": "extracted",
            })
        );
        assert_eq!(
            rendered.value["sources_masked"][0]["source_class"],
            json!("glean")
        );
        assert_eq!(
            rendered.value["sources_masked"][1]["source_class"],
            json!("extracted")
        );
        assert!(!serialized.contains("legacy_envelope_unparseable"));
        assert!(!serialized.contains("private-source-name-214"));
    }

    #[test]
    fn unparseable_legacy_fallback_returns_fail_closed_shape_for_every_surface() {
        let actor = Actor::Agent {
            name: "mcp-private-agent".to_string(),
            version: "private-version".to_string(),
        };
        let actor_hash = hashed_actor_for_unparseable_legacy(&actor);
        let legacy = json!({
            "ability_name": "legacy-secret-ability",
            "invocation_id": "secret-invocation",
            "produced_at": "2026-05-01T12:00:00Z",
            "actor": "system-actor-raw",
            "mode": "evaluate",
            "trust": {
                "effective": "strict_internal_trust",
                "contains_stored_synthesis": true
            },
            "data_sources_summary": ["customer_private_drive"],
            "sources": [{
                "signal_id": "sig-private-214",
                "entity_id": "entity-private-214",
                "chunk_id": "chunk-private-214",
                "thread_id": "thread-private-214",
                "message_id": "message-private-214",
                "meeting_id": "meeting-private-214",
                "document_id": "document-private-214",
                "entry_id": "entry-private-214",
                "assessment_id": "assessment-private-214",
                "completion_id": "completion-private-214",
                "opaque_id": "opaque-private-214"
            }],
            "prompt": "raw prompt secret",
            "raw_completion": "raw completion secret",
            "field_attributions": {
                "/summary": {
                    "explanation": "See [raw](javascript:alert(1)) and <meeting>"
                }
            }
        });

        for surface in [
            Surface::McpTool,
            Surface::McpToolDetail,
            Surface::TauriApp,
            Surface::P2Publication,
            Surface::LogStructured,
        ] {
            let rendered = render_serialized_provenance_for(legacy.clone(), actor.clone(), surface);
            assert_eq!(
                rendered.value,
                json!({
                    "actor": actor_hash.as_str(),
                    "surface": surface.as_str(),
                    "status": "legacy_envelope_unparseable",
                    "warning": format!(
                        "detail unavailable for actor {} on surface {}",
                        actor_hash,
                        surface.as_str()
                    ),
                })
            );
        }
    }

    #[test]
    fn unparseable_legacy_mcp_fallback_never_copies_raw_legacy_fields() {
        let legacy = json!({
            "ability_name": "legacy-secret-ability",
            "actor": "system-actor-raw",
            "trust": { "effective": "strict_internal_trust" },
            "data_sources_summary": ["customer_private_drive"],
            "sources": [{
                "signal_id": "sig-private-214",
                "entity_id": "entity-private-214",
                "chunk_id": "chunk-private-214",
                "thread_id": "thread-private-214",
                "message_id": "message-private-214",
                "meeting_id": "meeting-private-214",
                "document_id": "document-private-214",
                "entry_id": "entry-private-214",
                "assessment_id": "assessment-private-214",
                "completion_id": "completion-private-214",
                "opaque_id": "opaque-private-214"
            }]
        });

        let rendered = render_serialized_provenance_for(
            legacy,
            Actor::Human {
                role: "admin".to_string(),
                id: "admin-private-id".to_string(),
            },
            Surface::McpToolDetail,
        );
        let serialized = serde_json::to_string(&rendered).unwrap();

        for leaked in [
            "legacy-secret-ability",
            "system-actor-raw",
            "strict_internal_trust",
            "customer_private_drive",
            "sig-private-214",
            "entity-private-214",
            "chunk-private-214",
            "thread-private-214",
            "message-private-214",
            "meeting-private-214",
            "document-private-214",
            "entry-private-214",
            "assessment-private-214",
            "completion-private-214",
            "opaque-private-214",
            "admin-private-id",
        ] {
            assert!(
                !serialized.contains(leaked),
                "unparseable legacy fallback leaked {leaked}: {serialized}"
            );
        }
        assert_eq!(
            rendered.value["status"],
            json!("legacy_envelope_unparseable")
        );
    }

    #[test]
    fn p2_render_lists_source_classes_only() {
        let rendered = render_provenance_for(&provenance(), Actor::User, Surface::P2Publication);

        assert!(rendered.value["footnote"]
            .as_str()
            .unwrap()
            .contains("Glean Slack"));
        assert_eq!(
            rendered.value["detail"]["requires_user_confirmation"],
            json!(true)
        );
        assert_eq!(rendered.value["detail"]["included"], json!(false));
        assert_eq!(
            rendered.value["detail"]["reason"],
            json!("confirmation_required")
        );
        assert!(rendered.value["detail"].get("field_attributions").is_none());
        assert!(serde_json::to_string(&rendered).unwrap().chars().count() > 0);
    }

    #[test]
    fn p2_render_sanitizes_other_source_class_names() {
        let mut provenance = provenance();
        provenance
            .sources
            .push(other_source("secret customer import: acme"));
        let rendered = render_provenance_for(&provenance, Actor::User, Surface::P2Publication);
        let serialized = serde_json::to_string(&rendered).unwrap();

        assert_eq!(
            rendered.value["source_classes"],
            json!(["Glean Slack", "extracted"])
        );
        assert!(rendered.value["footnote"]
            .as_str()
            .unwrap()
            .contains("extracted"));
        assert!(!serialized.contains("secret customer import"));
    }

    #[test]
    fn p2_detail_requires_issued_confirmation_token() {
        let provenance = provenance();
        let token = valid_p2_detail_token(&provenance, "user-1");
        let rendered = render_provenance_for_with_options(
            &provenance,
            human_actor("user-1"),
            Surface::P2Publication,
            RenderOptions::with_p2_detail_confirmation(token),
        );

        assert_eq!(
            rendered.value["detail"]["requires_user_confirmation"],
            json!(true)
        );
        assert_eq!(rendered.value["detail"]["included"], json!(true));
        assert!(rendered.value["detail"]["sources"].is_array());
        assert!(rendered.value["detail"]["field_attributions"].is_object());

        let serialized = serde_json::to_string(&rendered).unwrap();
        assert!(!serialized.contains("sig-1"));
        assert!(!serialized.contains("acct-1"));
    }

    #[test]
    fn p2_detail_rejects_forged_confirmation_tokens() {
        assert!(P2DetailConfirmationToken::new("confirmed-p2-detail").is_none());

        let provenance = provenance();
        let valid_token = valid_p2_detail_token(&provenance, "user-1");
        let forged_token = P2DetailConfirmationToken {
            serialized: "forged".to_string(),
            claims: valid_token.claims.clone(),
        };

        let rendered = render_provenance_for_with_options(
            &provenance,
            human_actor("user-1"),
            Surface::P2Publication,
            RenderOptions::with_p2_detail_confirmation(forged_token),
        );
        assert_eq!(rendered.value["detail"]["included"], json!(false));
        assert_eq!(
            rendered.value["detail"]["reason"],
            json!("confirmation_required")
        );
    }

    #[test]
    fn p2_detail_rejects_expired_confirmation_tokens() {
        let provenance = provenance();
        let token = P2DetailConfirmationToken::issue_at(
            P2_DETAIL_CONFIRMATION_AUDIENCE,
            provenance.invocation_id,
            "user-1",
            1,
            chrono::Utc::now() - chrono::Duration::seconds(2),
        )
        .unwrap();

        let rendered = render_provenance_for_with_options(
            &provenance,
            human_actor("user-1"),
            Surface::P2Publication,
            RenderOptions::with_p2_detail_confirmation(token),
        );
        assert_eq!(rendered.value["detail"]["included"], json!(false));
    }

    #[test]
    fn p2_detail_rejects_mismatched_audience_and_invocation() {
        let provenance = provenance();
        let mismatched_audience = P2DetailConfirmationToken::issue(
            "different_audience",
            provenance.invocation_id,
            "user-1",
            60,
        )
        .unwrap();
        let mismatched_invocation = P2DetailConfirmationToken::issue(
            P2_DETAIL_CONFIRMATION_AUDIENCE,
            crate::abilities::provenance::InvocationId::new(
                uuid::Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap(),
            ),
            "user-1",
            60,
        )
        .unwrap();

        for token in [mismatched_audience, mismatched_invocation] {
            let rendered = render_provenance_for_with_options(
                &provenance,
                human_actor("user-1"),
                Surface::P2Publication,
                RenderOptions::with_p2_detail_confirmation(token),
            );
            assert_eq!(rendered.value["detail"]["included"], json!(false));
        }
    }
}
