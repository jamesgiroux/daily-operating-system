//! Ability surface inventory format (DOS-546 W1-C).
//!
//! Implements the canonical inventory shape from Phase 0 artifact 05
//! (`.docs/plans/dos-546/phase-0/05-ability-surface-inventory.md`). The
//! inventory is the surface-facing catalog that binds an ability contract
//! to concrete exposure rules, copy, permissions, and composition behavior.
//! Consumers (Wave 3 WP plugin, Wave 3 custom MCP server, SurfaceClient
//! introspection, Wave 4 block code) read the serialized inventory from
//! `tools/dailyos-abilities.json`.
//!
//! ## Schema source of truth
//!
//! The Rust struct shape mirrors the canonical TypeScript interface from
//! artifact 05 verbatim — every field name and enum tag matches. Schema
//! drift is prevented by `scripts/check_ability_inventory.sh`, which
//! regenerates the JSON from the live `AbilityRegistry` and diffs against
//! the committed `tools/dailyos-abilities.json`.
//!
//! ## Additive-only contract
//!
//! Per the W1-C issue acceptance criteria, the inventory schema is
//! additive-only across consuming releases. Future fields land as
//! `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`
//! so previously-serialized inventories remain valid input for newer
//! consumers and vice versa.
//!
//! ## Field mapping from `AbilityDescriptor`
//!
//! Today's `AbilityDescriptor` carries the runtime contract for an
//! ability (name, category, policy, schemas, composition). The
//! surface-facing fields below that do not yet have a runtime descriptor
//! source — `description`, `wp_permission`, `idempotency_class`,
//! `annotations`, `composition_kind.block_types` — start with closed
//! defaults and will be populated as ability authors fill out their
//! inventory entries in subsequent waves (per the issue scope: "this
//! issue lands the inventory format ... it does not populate the
//! inventory for every existing ability").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::abilities::registry::{
    AbilityCategory, AbilityDescriptor, ActorKind, McpExposure,
};

/// Actor classes admitted at a surface-facing inventory boundary.
///
/// Mirrors artifact 05 `AbilityActor`. The serialized tags are kept
/// in `snake_case` so the JSON matches the TypeScript union exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilityActor {
    /// Mirrors [`ActorKind::User`].
    User,
    /// Mirrors [`ActorKind::Agent`] and [`ActorKind::System`] for the
    /// inventory, which collapses the runtime distinction into one
    /// "runtime" actor class. The agent / system split is internal to
    /// the substrate and not surfaced to WP / MCP consumers.
    Runtime,
    /// Network-facing MCP host or agent.
    McpClient,
    /// Trusted in-process surface bridge (WordPress Studio, etc.).
    SurfaceClient,
}

/// Inventory-side category alias, kept in `snake_case` for the TS
/// consumer. Mirrors artifact 05 `AbilityCategory`. Runtime enforcement
/// lives in [`AbilityCategory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryCategory {
    /// No domain mutation, no external write.
    Read,
    /// No domain mutation; may synthesize from claim substrate.
    Transform,
    /// Writes externally or creates a shareable artifact.
    Publish,
    /// Mutates internal state through services.
    Maintenance,
}

impl From<AbilityCategory> for InventoryCategory {
    fn from(category: AbilityCategory) -> Self {
        match category {
            AbilityCategory::Read => Self::Read,
            AbilityCategory::Transform => Self::Transform,
            AbilityCategory::Publish => Self::Publish,
            AbilityCategory::Maintenance => Self::Maintenance,
        }
    }
}

/// MCP exposure tier — alias of [`McpExposure`] with `snake_case` JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryMcpExposure {
    /// Hidden from MCP enumeration.
    None,
    /// Name + description enumerated; invoke schema withheld.
    MetadataOnly,
    /// Full schema enumerated; agents may invoke.
    Invocable,
}

impl From<McpExposure> for InventoryMcpExposure {
    fn from(exposure: McpExposure) -> Self {
        match exposure {
            McpExposure::None => Self::None,
            McpExposure::MetadataOnly => Self::MetadataOnly,
            McpExposure::Invocable => Self::Invocable,
        }
    }
}

/// Retry / dedup class for surface bridges. Mirrors artifact 05
/// `IdempotencyClass`. Default for new inventory entries is derived
/// from the runtime category: `Publish` / `Maintenance` → `SideEffect`,
/// everything else → `Idempotent`. Ability authors override per entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyClass {
    /// Same input always yields the same output; safe to dedup.
    Idempotent,
    /// Retries are safe but may produce duplicate side effects under
    /// concurrency.
    SafeRetry,
    /// Has observable side effects; retries require explicit keys.
    SideEffect,
}

impl IdempotencyClass {
    /// Conservative default per artifact 05 §"Field Specifications":
    /// `publish` and `maintenance` are `side_effect` unless an explicit
    /// idempotency key contract is declared. Everything else defaults
    /// to `idempotent`.
    fn default_for(category: AbilityCategory) -> Self {
        match category {
            AbilityCategory::Publish | AbilityCategory::Maintenance => Self::SideEffect,
            AbilityCategory::Read | AbilityCategory::Transform => Self::Idempotent,
        }
    }
}

/// Known composition block types. Mirrors artifact 05
/// `CompositionBlockType`. Open-ended via the `Custom` variant which
/// must be paired with `annotations.custom_block_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionBlockType {
    AccountOverview,
    ClaimSummary,
    EvidenceList,
    HealthSnapshot,
    RelationshipMap,
    RiskCallout,
    ActionList,
    MarkdownDocument,
    Custom,
}

/// Composition kind discriminator. Modeled as a struct rather than the
/// TS-side discriminated union so it round-trips cleanly through serde
/// without bespoke deserialization. JSON shape matches artifact 05:
/// `{ "produces_blocks": bool, "block_types": [...] }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionKind {
    /// `true` when the ability produces composition blocks; `false`
    /// otherwise. When `false`, `block_types` must be empty (enforced
    /// by the CI gate that runs the schema validator).
    pub produces_blocks: bool,
    /// Block types this ability may emit. Empty when
    /// `produces_blocks = false`.
    pub block_types: Vec<CompositionBlockType>,
}

impl CompositionKind {
    /// Canonical "no composition" value: matches artifact 05's
    /// `{ produces_blocks: false, block_types: [] }`.
    pub const fn none() -> Self {
        Self {
            produces_blocks: false,
            block_types: Vec::new(),
        }
    }
}

/// Free-form annotation value. Mirrors artifact 05 — values may be
/// strings, numbers, booleans, or null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnnotationValue {
    /// Null annotation slot.
    Null,
    /// Boolean annotation.
    Bool(bool),
    /// Integer annotation (e.g. `surface_priority`).
    Int(i64),
    /// String annotation.
    String(String),
}

/// One inventory entry. Mirrors artifact 05's
/// `AbilitySurfaceInventoryEntry` field-for-field. Fields are emitted
/// in a stable order (the order below) so the diff-based CI gate
/// produces deterministic output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbilitySurfaceInventoryEntry {
    /// Canonical ability id (`namespace/slug`).
    pub name: String,
    /// One-paragraph human description used by WP / MCP / SurfaceClient
    /// discovery. Scanned by `scripts/check_ability_descriptions.sh`
    /// for PII + internal vocabulary.
    pub description: String,
    /// Behavioral category — must agree with the runtime descriptor.
    pub category: InventoryCategory,
    /// Free-form generator hints. Reserved keys per artifact 05.
    pub annotations: BTreeMap<String, AnnotationValue>,
    /// WordPress capability slug, or `none` for runtime-only abilities.
    pub wp_permission: String,
    /// Actor classes that may see and invoke through their surface
    /// bridge. Sorted ascending for deterministic serialization.
    pub allowed_actors: Vec<AbilityActor>,
    /// Fine-grained runtime scopes. Sorted ascending per artifact 05
    /// cross-field rule 1.
    pub required_scopes: Vec<String>,
    /// MCP exposure tier.
    pub mcp_exposure: InventoryMcpExposure,
    /// Whether a trusted SurfaceClient may invoke after policy /
    /// scope / actor checks.
    pub client_side_executable: bool,
    /// Retry / dedup classification.
    pub idempotency_class: IdempotencyClass,
    /// Composition block production.
    pub composition_kind: CompositionKind,
}

impl AbilitySurfaceInventoryEntry {
    /// Build an inventory entry by projecting fields from a runtime
    /// [`AbilityDescriptor`].
    ///
    /// Fields with no runtime source today (`description`,
    /// `wp_permission`, `annotations`, `composition_kind.block_types`)
    /// take closed defaults per artifact 05's "closed defaults"
    /// principle. Ability authors will populate them via an
    /// `#[ability]` macro extension or sibling `inventory.toml` in
    /// subsequent waves; until then the inventory faithfully reflects
    /// what the runtime declares and the CI gate prevents drift.
    pub fn from_descriptor(descriptor: &AbilityDescriptor) -> Self {
        let mut allowed_actors: Vec<AbilityActor> = descriptor
            .policy
            .allowed_actors
            .iter()
            .map(|kind| AbilityActor::from(*kind))
            .collect();
        allowed_actors.sort();
        allowed_actors.dedup();

        let mut required_scopes: Vec<String> = descriptor
            .policy
            .required_scopes
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        required_scopes.sort();
        required_scopes.dedup();

        Self {
            name: descriptor.name.to_string(),
            description: String::new(),
            category: InventoryCategory::from(descriptor.category),
            annotations: BTreeMap::new(),
            wp_permission: "none".to_string(),
            allowed_actors,
            required_scopes,
            mcp_exposure: InventoryMcpExposure::from(descriptor.policy.mcp_exposure),
            client_side_executable: descriptor.policy.client_side_executable,
            idempotency_class: IdempotencyClass::default_for(descriptor.category),
            composition_kind: CompositionKind::none(),
        }
    }
}

impl From<ActorKind> for AbilityActor {
    fn from(kind: ActorKind) -> Self {
        match kind {
            // Artifact 05's `runtime` actor collapses the substrate's
            // agent + system distinction; neither is surfaced to WP /
            // MCP consumers as a separate principal.
            ActorKind::Agent | ActorKind::System | ActorKind::Admin => Self::Runtime,
            ActorKind::User => Self::User,
            ActorKind::SurfaceClient => Self::SurfaceClient,
        }
    }
}

/// The full serialized inventory artifact.
///
/// Wraps the entry list in a versioned envelope so the additive-only
/// contract (W1-C AC bullet 7) is enforceable: consumers read
/// `schema_version`, refuse loads above the supported version, and
/// extend with optional fields on minor bumps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbilitySurfaceInventory {
    /// Schema version of the inventory envelope. Starts at 1 in W1-C.
    pub schema_version: u32,
    /// Entries, sorted ascending by `name` for deterministic output.
    pub abilities: Vec<AbilitySurfaceInventoryEntry>,
}

impl AbilitySurfaceInventory {
    /// Schema version emitted by this crate. Bump when the inventory
    /// envelope grows a non-additive field (which the contract forbids
    /// — so this bumps only on explicit additive extensions).
    pub const SCHEMA_VERSION: u32 = 1;

    /// Build the inventory artifact by enumerating every registered
    /// ability descriptor. Sorts entries by `name` so the serialized
    /// JSON is deterministic across builds — required for the diff
    /// gate to be meaningful.
    pub fn from_descriptors<'a, I>(descriptors: I) -> Self
    where
        I: IntoIterator<Item = &'a AbilityDescriptor>,
    {
        let mut abilities: Vec<AbilitySurfaceInventoryEntry> = descriptors
            .into_iter()
            .map(AbilitySurfaceInventoryEntry::from_descriptor)
            .collect();
        abilities.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            schema_version: Self::SCHEMA_VERSION,
            abilities,
        }
    }

    /// Serialize to canonical, pretty-printed JSON with a trailing
    /// newline. Pretty output is intentional: the artifact is reviewed
    /// in PRs and diffed by humans; the line-by-line diff is the
    /// primary failure surface for the CI gate.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        let mut out = serde_json::to_string_pretty(self)?;
        out.push('\n');
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use serde_json::Value;

    fn empty_schema() -> Value {
        serde_json::json!({ "type": "object" })
    }

    fn ok_erased<'a>(
        _ctx: &'a crate::abilities::registry::AbilityContext<'a>,
        _input: Value,
    ) -> crate::abilities::registry::ErasedAbilityFuture<'a> {
        Box::pin(async { Ok(Value::Null) })
    }

    fn fixture_descriptor(name: &'static str, category: AbilityCategory) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "0.1.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: &[ActorKind::User, ActorKind::SurfaceClient],
                allowed_modes: &[],
                requires_confirmation: false,
                may_publish: false,
                required_scopes: &["claims:read", "accounts:read"],
                mcp_exposure: McpExposure::Invocable,
                client_side_executable: true,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy {
                emits_on_output_change: &[],
                coalesce: false,
            },
            invoke_erased: ok_erased,
            input_schema: empty_schema,
            output_schema: empty_schema,
        }
    }

    #[test]
    fn empty_inventory_serializes_deterministically() {
        let inventory = AbilitySurfaceInventory::from_descriptors(std::iter::empty());
        let json = inventory.to_canonical_json().expect("serialize");
        assert!(json.starts_with("{\n"));
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"abilities\": []"));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn descriptor_projection_sorts_actors_and_scopes() {
        let descriptor = fixture_descriptor("dailyos/test", AbilityCategory::Transform);
        let entry = AbilitySurfaceInventoryEntry::from_descriptor(&descriptor);
        // Actors must be sorted; required_scopes must be sorted.
        assert_eq!(
            entry.allowed_actors,
            vec![AbilityActor::User, AbilityActor::SurfaceClient]
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            entry.required_scopes,
            vec!["accounts:read".to_string(), "claims:read".to_string()]
        );
        assert_eq!(entry.category, InventoryCategory::Transform);
        assert_eq!(entry.mcp_exposure, InventoryMcpExposure::Invocable);
        assert!(entry.client_side_executable);
        // Closed defaults for fields without a runtime source.
        assert!(entry.description.is_empty());
        assert_eq!(entry.wp_permission, "none");
        assert!(entry.annotations.is_empty());
        assert!(!entry.composition_kind.produces_blocks);
        assert!(entry.composition_kind.block_types.is_empty());
    }

    #[test]
    fn idempotency_default_tracks_category() {
        assert_eq!(
            IdempotencyClass::default_for(AbilityCategory::Read),
            IdempotencyClass::Idempotent
        );
        assert_eq!(
            IdempotencyClass::default_for(AbilityCategory::Transform),
            IdempotencyClass::Idempotent
        );
        assert_eq!(
            IdempotencyClass::default_for(AbilityCategory::Publish),
            IdempotencyClass::SideEffect
        );
        assert_eq!(
            IdempotencyClass::default_for(AbilityCategory::Maintenance),
            IdempotencyClass::SideEffect
        );
    }

    #[test]
    fn inventory_sorts_entries_by_name() {
        let a = fixture_descriptor("zeta/one", AbilityCategory::Read);
        let b = fixture_descriptor("alpha/two", AbilityCategory::Read);
        let inventory = AbilitySurfaceInventory::from_descriptors([&a, &b]);
        assert_eq!(inventory.abilities[0].name, "alpha/two");
        assert_eq!(inventory.abilities[1].name, "zeta/one");
    }

    #[test]
    fn actor_kind_collapses_runtime_principals() {
        assert_eq!(AbilityActor::from(ActorKind::Agent), AbilityActor::Runtime);
        assert_eq!(AbilityActor::from(ActorKind::System), AbilityActor::Runtime);
        assert_eq!(AbilityActor::from(ActorKind::Admin), AbilityActor::Runtime);
        assert_eq!(AbilityActor::from(ActorKind::User), AbilityActor::User);
        assert_eq!(
            AbilityActor::from(ActorKind::SurfaceClient),
            AbilityActor::SurfaceClient
        );
    }
}
