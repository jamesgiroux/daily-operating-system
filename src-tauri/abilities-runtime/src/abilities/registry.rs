//! Ability registry, AbilityContext, and typed/erased invocation.
//!
//! Per ADR-0102 §181-258. Type definitions consumed by the `#[ability]`
//! proc macro (W3-A part 3) for `inventory::submit!` registration.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::{AbilityOutput, CompositionId};
use crate::abilities::tracer::AbilityTracer;
use crate::intelligence::provider::IntelligenceProvider;
use crate::services::context::{ClaimDismissalSurface, ExecutionMode, ServiceContext};

const UNKNOWN_SCHEMA_ABILITY: &str = "<unknown>";

/// ADR-0102 §76-95: ability category drives mutation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum AbilityCategory {
    Read,
    Transform,
    Publish,
    Maintenance,
}

/// Stable, non-PII identifier for a paired [`Actor::SurfaceClient`] instance.
///
/// Per ADR-0111 §8, every SurfaceClient invocation must be auditable by
/// instance identity. The identity is opaque to the substrate: WordPress
/// site GUIDs, Obsidian vault IDs, browser-extension installation IDs, etc.
/// all flow through this newtype.
///
/// W1-A0 audit emission consumes this for `actor_instance`. W1-B
/// `SurfaceClientBridge` consumes it for per-instance scope grants.
///
/// `Display` / `Debug` produce the raw inner string. Callers are expected
/// not to embed PII in the identifier itself; the type does no scrubbing.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SurfaceClientId(String);

impl SurfaceClientId {
    /// Construct a new identifier from an owned string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SurfaceClientId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A named scope grant a [`Actor::SurfaceClient`] instance carries into
/// each invocation.
///
/// Per ADR-0111 §8, scopes gate which abilities a specific SurfaceClient
/// instance may invoke. W1-B will extend `AbilityPolicy` with
/// `required_scopes: Vec<SurfaceScope>`; the `SurfaceClientBridge` will
/// enforce that every required scope is present in the instance's grant
/// before registry lookup.
///
/// The newtype is intentionally string-backed: the canonical scope
/// vocabulary (e.g. `read.account_overview`, `read.composition`,
/// `submit.feedback`, `manage.pairing`) is extensible per ability without a
/// substrate recompile, while [`ScopeSet`] enforces a runtime-registered
/// allowlist at deserialization. ADR-0111 §8 names scopes as "the defined
/// enum"; the substrate models that as an allowlist owned by the runtime
/// (registered as abilities declare their scopes via `#[ability]` in W1-B).
#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct SurfaceScope(String);

impl SurfaceScope {
    /// Construct a new scope value from an owned string.
    pub fn new(scope: impl Into<String>) -> Self {
        Self(scope.into())
    }

    /// Borrow the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SurfaceScope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Construction / deserialization errors for [`ScopeSet`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeSetError {
    /// The set was empty. Per ADR-0111 §8 edge case, a
    /// SurfaceClient with no scopes is a misconfiguration, not a paired
    /// surface, and must be rejected at construction.
    Empty,
    /// One or more scopes are outside the runtime-registered allowlist.
    /// Carries the offending scope values for audit / error surfacing.
    UnknownScopes(Vec<SurfaceScope>),
}

impl std::fmt::Display for ScopeSetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeSetError::Empty => formatter.write_str(
                "ScopeSet construction rejected: SurfaceClient requires at least one scope",
            ),
            ScopeSetError::UnknownScopes(scopes) => {
                write!(
                    formatter,
                    "ScopeSet construction rejected: unknown scope(s) outside the registered allowlist: {}",
                    scopes
                        .iter()
                        .map(SurfaceScope::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl std::error::Error for ScopeSetError {}

/// Process-global allowlist of known scope values.
///
/// State semantics:
/// - `None` (uninitialized) = lenient bootstrap mode (any scope accepted).
/// - `Some(set)` (initialized) = strict; only scopes in `set` are accepted at
///   [`ScopeSet`] construction and deserialization. An explicitly seeded
///   empty set rejects every scope, which is rare in practice but legal.
///
/// Initialization semantics:
/// - **Production:** one-time, process-lifetime. The first call to
///   [`ScopeSet::initialize_allowlist`] wins and flips
///   [`SCOPE_ALLOWLIST_INITIALIZED`]; subsequent calls return `Err` with the
///   would-be set so the caller can no-op (the W1-B `from_descriptors_checked`
///   path relies on this for idempotent re-registry within one process).
/// - **Tests:** [`ScopeSet::set_allowlist_for_tests`] overwrites the lock
///   unconditionally so each test can install a deterministic per-test
///   allowlist regardless of registry initialization order.
///
/// The container is [`RwLock`] (not [`OnceLock`]) so tests can replace the
/// allowlist; the `INITIALIZED` atomic preserves the "one-time in production"
/// guarantee outside the test surface.
static SCOPE_ALLOWLIST: RwLock<Option<BTreeSet<SurfaceScope>>> = RwLock::new(None);

/// Tracks whether production initialization has occurred. Read by
/// [`ScopeSet::initialize_allowlist`] to reject double-initialization without
/// silently overwriting a previously seeded set. Tests bypass this via
/// [`ScopeSet::set_allowlist_for_tests`], which both writes the lock and
/// leaves the flag unchanged.
static SCOPE_ALLOWLIST_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// A typed, non-empty set of [`SurfaceScope`] values carried by every
/// [`Actor::SurfaceClient`] invocation. Per ADR-0111 §8 W1-A
/// acceptance criteria.
///
/// Construction enforces two invariants:
///
/// 1. **Non-empty.** An empty grant is a misconfiguration; see
///    [`ScopeSetError::Empty`].
/// 2. **Allowlisted.** If the process-global allowlist has been initialized
///    (via W1-B `#[ability]` macro registration or `set_allowlist_for_tests`),
///    every scope must be present in it; otherwise the constructor accepts
///    any scope (lenient bootstrap mode, intentional for v1.4.2 substrate
///    staging — W1-B is what populates the allowlist).
///
/// Deserialization (via `serde`) routes through [`ScopeSet::new`] so both
/// invariants apply on the wire boundary as well as the constructor.
///
/// The inner storage is a `BTreeSet` for deterministic ordering on
/// serialization and audit-log emission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema)]
pub struct ScopeSet(BTreeSet<SurfaceScope>);

impl ScopeSet {
    /// Construct a [`ScopeSet`] from any iterator of [`SurfaceScope`].
    ///
    /// Returns [`ScopeSetError::Empty`] if the resulting set is empty
    /// (including the case where the input iterator yielded only duplicates
    /// that collapsed away — still empty after dedup is still empty, which
    /// is impossible without an empty input).
    ///
    /// Returns [`ScopeSetError::UnknownScopes`] when the global allowlist is
    /// initialized and any of the scopes are not in it.
    pub fn new(scopes: impl IntoIterator<Item = SurfaceScope>) -> Result<Self, ScopeSetError> {
        let set: BTreeSet<SurfaceScope> = scopes.into_iter().collect();
        if set.is_empty() {
            return Err(ScopeSetError::Empty);
        }
        let guard = SCOPE_ALLOWLIST
            .read()
            .expect("SCOPE_ALLOWLIST RwLock poisoned");
        if let Some(allowed) = guard.as_ref() {
            let unknown: Vec<SurfaceScope> = set
                .iter()
                .filter(|scope| !allowed.contains(*scope))
                .cloned()
                .collect();
            if !unknown.is_empty() {
                return Err(ScopeSetError::UnknownScopes(unknown));
            }
        }
        drop(guard);
        Ok(Self(set))
    }

    /// True if `scope` is present in this set.
    pub fn contains(&self, scope: &SurfaceScope) -> bool {
        self.0.contains(scope)
    }

    /// Iterate the scopes in deterministic (sorted) order.
    pub fn iter(&self) -> impl Iterator<Item = &SurfaceScope> {
        self.0.iter()
    }

    /// Always returns `false` — a [`ScopeSet`] cannot be empty by
    /// construction. Provided for ergonomic parity with collection APIs.
    #[allow(clippy::unused_self)]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Number of scopes in the set. Always >= 1.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Initialize the global scope allowlist. One-time in production:
    /// the first call wins and flips [`SCOPE_ALLOWLIST_INITIALIZED`];
    /// subsequent calls return `Err(set)` carrying the rejected set so the
    /// caller (typically W1-B's `from_descriptors_checked`) can no-op when
    /// the substrate is rebuilt within the same process.
    ///
    /// W1-B `#[ability]` macro registration calls this once at registry
    /// boot, after collecting every ability's declared `required_scopes`.
    /// Tests must use [`Self::set_allowlist_for_tests`] instead — this
    /// constructor refuses to overwrite a previously seeded allowlist.
    pub fn initialize_allowlist(
        scopes: impl IntoIterator<Item = SurfaceScope>,
    ) -> Result<(), BTreeSet<SurfaceScope>> {
        let set: BTreeSet<SurfaceScope> = scopes.into_iter().collect();
        // Use compare_exchange so concurrent first-callers see exactly one
        // winner. `Acquire`/`Release` pair the flag flip with the lock write.
        match SCOPE_ALLOWLIST_INITIALIZED.compare_exchange(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                let mut guard = SCOPE_ALLOWLIST
                    .write()
                    .expect("SCOPE_ALLOWLIST RwLock poisoned");
                *guard = Some(set);
                Ok(())
            }
            Err(_) => Err(set),
        }
    }

    /// Test-only: unconditionally overwrite the allowlist with `scopes`.
    /// Tests use this to install a deterministic per-test allowlist that is
    /// guaranteed to include every scope the test constructs, independent of
    /// whatever production initialization has happened earlier in the process
    /// (e.g. an unrelated registry build that seeded an empty union).
    ///
    /// Does **not** affect [`SCOPE_ALLOWLIST_INITIALIZED`]; production code
    /// paths still observe the one-time initialization invariant.
    #[doc(hidden)]
    pub fn set_allowlist_for_tests(scopes: impl IntoIterator<Item = SurfaceScope>) {
        let mut guard = SCOPE_ALLOWLIST
            .write()
            .expect("SCOPE_ALLOWLIST RwLock poisoned");
        *guard = Some(scopes.into_iter().collect());
    }

    /// Test-only: clear the allowlist, returning to lenient bootstrap mode
    /// where any scope is accepted. Companion to [`Self::set_allowlist_for_tests`]
    /// for tests that need to exercise the unseeded path explicitly.
    #[doc(hidden)]
    pub fn clear_allowlist_for_tests() {
        let mut guard = SCOPE_ALLOWLIST
            .write()
            .expect("SCOPE_ALLOWLIST RwLock poisoned");
        *guard = None;
    }
}

impl Serialize for ScopeSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ScopeSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = BTreeSet::<SurfaceScope>::deserialize(deserializer)?;
        ScopeSet::new(raw).map_err(serde::de::Error::custom)
    }
}

/// Who is invoking. ADR-0102 §250-258, amended 2026-05-10 (Accepted) to add
/// [`Actor::SurfaceClient`] as the fourth actor class per ADR-0111 §8.
///
/// The first three variants are unit; `SurfaceClient` carries the paired
/// instance identity AND its scope grant as a struct variant per ADR-0111
/// §8 and W1-A acceptance criteria. Per-request enforcement uses
/// `scopes` directly; the bridge does not re-derive scopes from a side
/// channel.
///
/// `Copy` was removed in the W1-A amendment because `SurfaceClient` carries
/// an owned [`SurfaceClientId`] (`String`) and [`ScopeSet`] (`BTreeSet`).
/// Callers that previously relied on implicit copy now `.clone()` explicitly
/// or pass by reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum Actor {
    Agent,
    User,
    Admin,
    System,
    /// Third-party local surface invoking on behalf of a paired user.
    /// See ADR-0111 §8. Both identity and scope grant ride on the variant;
    /// the bridge (W1-B) reads `scopes` to enforce `required_scopes` from
    /// `AbilityPolicy` before registry lookup.
    SurfaceClient {
        /// Stable, non-PII instance identity. Surfaces audit emission's
        /// `actor_instance` field.
        instance: SurfaceClientId,
        /// The scope grant this instance carries for this request. Always
        /// non-empty by [`ScopeSet`] construction. Surfaces audit emission's
        /// `actor_scopes` field.
        scopes: ScopeSet,
    },
}

/// Discriminator over [`Actor`] variants — the "kind" of actor, without
/// any per-invocation instance data. Used in [`AbilityPolicy::allowed_actors`]
/// to declare which actor classes may invoke an ability.
///
/// Per ADR-0102 §7.6 (W0-D amended 2026-05-10) W1-B, the policy
/// slice describes which actor *kinds* an ability admits — not specific
/// actor instances. [`Actor::SurfaceClient`] is a struct variant carrying
/// owned [`SurfaceClientId`] and [`ScopeSet`] data, so it cannot itself be
/// const-constructed in a `&'static [Actor]` slice. `ActorKind` is the
/// const-friendly discriminator that closes that gap and keeps descriptor
/// storage `inventory::submit!`-compatible.
///
/// Runtime invocation gates compare the incoming [`Actor`]'s `.kind()`
/// against the policy slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ActorKind {
    /// Mirrors [`Actor::Agent`].
    Agent,
    /// Mirrors [`Actor::User`].
    User,
    /// Mirrors [`Actor::Admin`].
    Admin,
    /// Mirrors [`Actor::System`].
    System,
    /// Mirrors [`Actor::SurfaceClient`]. Per-invocation instance and scope
    /// data live on the runtime variant; only the kind appears in the policy.
    SurfaceClient,
}

impl Actor {
    /// Project this [`Actor`] to its [`ActorKind`] discriminator.
    ///
    /// Used by registry / bridge invocation gates that check
    /// `descriptor.policy.allowed_actors.contains(&actor.kind())`.
    pub const fn kind(&self) -> ActorKind {
        match self {
            Actor::Agent => ActorKind::Agent,
            Actor::User => ActorKind::User,
            Actor::Admin => ActorKind::Admin,
            Actor::System => ActorKind::System,
            Actor::SurfaceClient { .. } => ActorKind::SurfaceClient,
        }
    }
}

/// MCP tool-surface exposure tier per ADR-0102 §7.1 (W0-D amended 2026-05-10).
///
/// Governs how an ability appears in MCP introspection (`list_tools` /
/// `list_abilities`). Independent of [`AbilityPolicy::client_side_executable`]
/// — the two fields govern different trust boundaries per Phase 0 artifact 05
/// lines 389-412. An ability may be `Invocable` over MCP while not
/// client-side executable, or vice versa.
///
/// Variants:
/// - `None`: not enumerated by any MCP bridge.
/// - `MetadataOnly`: name + description enumerated; invoke schema withheld.
/// - `Invocable`: full schema enumerated; agent may invoke.
///
/// Default per `AbilityPolicy::default()` is `None` — the closed default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpExposure {
    /// Hidden from every MCP enumeration surface.
    #[default]
    None,
    /// Enumerated with name + description only; invoke schema not exposed.
    MetadataOnly,
    /// Enumerated with full schema; agents may invoke.
    Invocable,
}

/// Per-ability rate limit override for SurfaceClient bridge invocation.
///
/// The override is lower-only: bridge/runtime defaults remain the ceiling,
/// and ability policy may only tighten them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct AbilityRateLimit {
    pub requests_per_minute: u32,
    pub burst_per_second: u32,
}

impl AbilityRateLimit {
    pub const fn new(requests_per_minute: u32, burst_per_second: u32) -> Self {
        Self {
            requests_per_minute,
            burst_per_second,
        }
    }

    pub fn lowered_by(self, override_limit: Self) -> Self {
        Self {
            requests_per_minute: self
                .requests_per_minute
                .min(override_limit.requests_per_minute),
            burst_per_second: self.burst_per_second.min(override_limit.burst_per_second),
        }
    }

    pub fn effective_lower_only(default_limit: Self, override_limit: Option<Self>) -> Self {
        override_limit.map_or(default_limit, |limit| default_limit.lowered_by(limit))
    }
}

/// Per-ability policy (which actors may invoke, which modes, etc.).
///
/// Per ADR-0102 §7.1 (W0-D amended 2026-05-10) W1-B, the
/// schema carries four additional fields beyond the v1.4.1 baseline:
///
/// - `required_scopes`: scope vocabulary a [`Actor::SurfaceClient`] must
///   present at the bridge boundary (W2-B) before registry lookup. Stored
///   as `&'static [&'static str]` so descriptors remain `static`-friendly
///   for `inventory::submit!`. Callers that need typed [`SurfaceScope`]
///   values should call [`AbilityPolicy::required_scopes_typed`]. This
///   reconciles AC line 446's informal `Vec<SurfaceClientScope>` with the
///   substrate's static-descriptor invariant — see W1-B commit for rationale.
/// - `mcp_exposure`: tri-state MCP enumeration tier (see [`McpExposure`]).
/// - `client_side_executable`: whether a SurfaceClient may invoke after
///   policy/scope/actor checks. Independent of `mcp_exposure` per Phase 0
///   artifact 05 lines 389-412.
/// - `rate_limit`: optional lower-only per-ability limiter override for W2-D.
///
/// Closed defaults (via [`AbilityPolicy::default`]):
/// - `allowed_actors: &[ActorKind::User]` — least-privilege actor floor
///   (W1-B AC §449, ADR-0102 §7.6 W0-D amended 2026-05-10).
/// - `required_scopes: &[]`
/// - `mcp_exposure: McpExposure::None`
/// - `client_side_executable: false`
/// - `rate_limit: None`
///
/// Defaults preserve v1.4.0/v1.4.1 behavior: an ability with no
/// `SurfaceClient` in `allowed_actors` may keep empty `required_scopes`,
/// and the macro compile-error gate (W1-B) only fires when
/// `allowed_actors` includes `SurfaceClient`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityPolicy {
    /// Actor kinds (see [`ActorKind`]) admitted by this ability. Per
    /// ADR-0102 §7.6 (W0-D amended 2026-05-10) W1-B, the slice
    /// stores `ActorKind` (not [`Actor`]) so that [`Actor::SurfaceClient`]
    /// — a struct variant carrying owned per-invocation state — can be
    /// listed alongside the unit variants without breaking `inventory::submit!`'s
    /// `static`-only descriptor invariant.
    pub allowed_actors: &'static [ActorKind],
    pub allowed_modes: &'static [ExecutionMode],
    pub requires_confirmation: bool,
    pub may_publish: bool,
    /// Scope vocabulary a [`Actor::SurfaceClient`] must carry before this
    /// ability is reachable. Empty = no scope required (preserves
    /// pre-v1.4.2 behavior for non-SurfaceClient abilities).
    pub required_scopes: &'static [&'static str],
    /// MCP tool-surface enumeration tier. Default `None`.
    pub mcp_exposure: McpExposure,
    /// Whether a [`Actor::SurfaceClient`] may invoke after policy/scope/
    /// actor checks. Default `false`.
    pub client_side_executable: bool,
    /// Optional lower-only rate-limit override for this ability.
    pub rate_limit: Option<AbilityRateLimit>,
}

impl Default for AbilityPolicy {
    /// Per ADR-0102 §7.6 (W0-D amended 2026-05-10) W1-B
    /// AC §449: the closed default is `[User]` — the least-privilege
    /// actor floor — not `[]` (closed-to-everyone). The other W1-B
    /// fields default to closed forms (`required_scopes: &[]`,
    /// `mcp_exposure: McpExposure::None`, `client_side_executable: false`,
    /// `rate_limit: None`).
    fn default() -> Self {
        Self {
            allowed_actors: &[ActorKind::User],
            allowed_modes: &[],
            requires_confirmation: false,
            may_publish: false,
            required_scopes: &[],
            mcp_exposure: McpExposure::None,
            client_side_executable: false,
            rate_limit: None,
        }
    }
}

impl AbilityPolicy {
    /// Materialize `required_scopes` as typed [`SurfaceScope`] values.
    ///
    /// The canonical storage shape is `&'static [&'static str]` so the
    /// descriptor stays `static`-constructible (required by
    /// `inventory::submit!`). Bridge/runtime code that needs typed scopes
    /// for [`ScopeSet`] enforcement (W2-B) calls this helper.
    pub fn required_scopes_typed(&self) -> Vec<SurfaceScope> {
        self.required_scopes
            .iter()
            .map(|s| SurfaceScope::new(*s))
            .collect()
    }

    /// Apply this policy's optional lower-only rate-limit override.
    pub fn effective_rate_limit(&self, default_limit: AbilityRateLimit) -> AbilityRateLimit {
        AbilityRateLimit::effective_lower_only(default_limit, self.rate_limit)
    }
}

/// Composition entry per descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposesEntry {
    pub id: CompositionId,
    pub ability: &'static str,
    pub optional: bool,
}

/// Signal policy metadata for ADR-0115. W3-A records, does not emit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignalPolicy {
    pub emits_on_output_change: &'static [&'static str],
    pub coalesce: bool,
}

pub type ErasedAbilityFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;
pub type ErasedAbilityInvoker =
    for<'a> fn(&'a AbilityContext<'a>, serde_json::Value) -> ErasedAbilityFuture<'a>;

/// One ability's frozen description. The proc macro emits this via
/// inventory::submit! in part 3. For part 2 we define the shape and the
/// registry that collects it.
#[derive(Debug, Clone)]
pub struct AbilityDescriptor {
    pub name: &'static str,
    pub version: &'static str,
    pub schema_version: u32,
    pub category: AbilityCategory,
    pub policy: AbilityPolicy,
    pub composes: &'static [ComposesEntry],
    pub mutates: &'static [&'static str],
    pub experimental: bool,
    pub registered_at: Option<&'static str>,
    pub signal_policy: SignalPolicy,
    pub invoke_erased: ErasedAbilityInvoker,
    pub input_schema: fn() -> serde_json::Value,
    pub output_schema: fn() -> serde_json::Value,
}

inventory::collect!(AbilityDescriptor);

/// Ability error kinds — ADR-0102 Amendment A §466-483.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AbilityErrorKind {
    Validation,
    Capability,
    OptionalComposedReadFailed {
        composition_id: CompositionId,
        reason: String,
    },
    HardError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AbilityError {
    pub kind: AbilityErrorKind,
    pub message: String,
}

pub type AbilityResult<T> = Result<AbilityOutput<T>, AbilityError>;

pub trait ConfirmationProof: Send + Sync {}

impl<T> ConfirmationProof for T where T: Send + Sync {}

/// AbilityContext wraps ServiceContext and adds provider/tracer seams,
/// actor, and confirmation.
///
///  hard boundary: this is the ONLY way ability code accesses runtime;
/// raw ActionDb / AppState / SQL handles / fs writers / live queues are NEVER
/// surfaced here.
pub struct AbilityContext<'a> {
    services: &'a ServiceContext<'a>,
    pub provider: &'a dyn IntelligenceProvider,
    pub tracer: &'a dyn AbilityTracer,
    pub actor: Actor,
    pub confirmation: Option<&'a dyn ConfirmationProof>,
    entity_context_claim_surface: ClaimDismissalSurface,
}

impl<'a> AbilityContext<'a> {
    pub fn new(
        services: &'a ServiceContext<'a>,
        provider: &'a dyn IntelligenceProvider,
        tracer: &'a dyn AbilityTracer,
        actor: Actor,
        confirmation: Option<&'a dyn ConfirmationProof>,
        entity_context_claim_surface: ClaimDismissalSurface,
    ) -> Self {
        Self {
            services,
            provider,
            tracer,
            actor,
            confirmation,
            entity_context_claim_surface,
        }
    }

    pub fn services(&self) -> &ServiceContext<'a> {
        self.services
    }

    pub fn mode(&self) -> ExecutionMode {
        self.services.mode
    }

    pub fn entity_context_claim_surface(&self) -> ClaimDismissalSurface {
        self.entity_context_claim_surface
    }

    pub fn for_entity_context_claim_surface(
        &self,
        entity_context_claim_surface: ClaimDismissalSurface,
    ) -> Self {
        Self {
            services: self.services,
            provider: self.provider,
            tracer: self.tracer,
            actor: self.actor.clone(),
            confirmation: self.confirmation,
            entity_context_claim_surface,
        }
    }
}

/// Registry violations.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryViolation {
    DuplicateAbilityName(String),
    SchemaClosure(SchemaClosureError),
    UnknownComposes {
        ability: String,
        target: String,
    },
    CompositionCycle(Vec<String>),
    CategoryViolation {
        ability: String,
        category: AbilityCategory,
        transitively_composes: AbilityCategory,
    },
    ExperimentalMissingRegisteredAt(String),
    ExperimentalExpired {
        ability: String,
        age_days: i64,
    },
    ExperimentalInProduction,
    MetadataDrift {
        ability: String,
        observed: String,
        declared: String,
    },
}

#[derive(Debug)]
pub struct AbilityRegistry {
    by_name: HashMap<&'static str, AbilityDescriptor>,
}

impl AbilityRegistry {
    /// Collect from inventory and validate. Fails closed on any violation.
    pub fn from_inventory_checked() -> Result<Self, Vec<RegistryViolation>> {
        let descriptors = inventory::iter::<AbilityDescriptor>
            .into_iter()
            .cloned()
            .collect();
        Self::from_descriptors_checked(descriptors)
    }

    pub fn global_checked() -> Result<&'static Self, &'static [RegistryViolation]> {
        static REGISTRY: OnceLock<Result<AbilityRegistry, Vec<RegistryViolation>>> =
            OnceLock::new();
        match REGISTRY.get_or_init(Self::from_inventory_checked) {
            Ok(registry) => Ok(registry),
            Err(violations) => Err(violations.as_slice()),
        }
    }

    pub fn from_descriptors_checked(
        descriptors: Vec<AbilityDescriptor>,
    ) -> Result<Self, Vec<RegistryViolation>> {
        let mut violations = Vec::new();
        let mut by_name = HashMap::new();

        validate_descriptor_schema_closures(&descriptors, &mut violations);

        for descriptor in descriptors {
            if by_name.contains_key(descriptor.name) {
                violations.push(RegistryViolation::DuplicateAbilityName(
                    descriptor.name.to_string(),
                ));
            } else {
                by_name.insert(descriptor.name, descriptor);
            }
        }

        validate_unknown_composes(&by_name, &mut violations);
        let cycle_count_before = violations.len();
        validate_cycles(&by_name, &mut violations);
        let graph_has_hard_errors = violations[cycle_count_before..]
            .iter()
            .any(|violation| matches!(violation, RegistryViolation::CompositionCycle(_)))
            || violations
                .iter()
                .any(|violation| matches!(violation, RegistryViolation::UnknownComposes { .. }));
        if !graph_has_hard_errors {
            validate_category_transitivity(&by_name, &mut violations);
        }
        validate_experimental(&by_name, &mut violations);

        if violations.is_empty() {
            // W1-B: seed the global SurfaceScope allowlist with the union
            // of every registered ability's required_scopes. Idempotent:
            // a second registry build (e.g. tests) sees the existing
            // allowlist and is a no-op. Lenient bootstrap mode (an
            // allowlist that hasn't been initialized yet) accepts any
            // scope at ScopeSet construction; once seeded here, unknown
            // scopes are rejected at the wire boundary. See ADR-0111 §8
            //
            let mut union: BTreeSet<SurfaceScope> = by_name
                .values()
                .flat_map(|descriptor| descriptor.policy.required_scopes.iter())
                .map(|s| SurfaceScope::new(*s))
                .collect();
            union.insert(SurfaceScope::new("read.account_overview"));
            union.insert(SurfaceScope::new("submit.feedback"));
            // OnceLock::set returns Err if already initialized — that's
            // the intended path on subsequent registry builds within a
            // single process. We do not surface the result.
            if ScopeSet::initialize_allowlist(union).is_err() {
                // Intentional no-op: another registry has already seeded
                // the global allowlist in this process.
            }
            Ok(Self { by_name })
        } else {
            Err(violations)
        }
    }

    #[doc(hidden)]
    pub fn from_descriptors_unchecked_for_runtime_validation_tests(
        descriptors: Vec<AbilityDescriptor>,
    ) -> Self {
        Self {
            by_name: descriptors
                .into_iter()
                .map(|descriptor| (descriptor.name, descriptor))
                .collect(),
        }
    }

    /// Iterate every descriptor in the registry, no actor filter.
    ///
    /// Used by the W1-C `emit_ability_inventory` binary to project
    /// the full ability set into the surface-facing inventory artifact.
    /// Tooling-facing only — runtime callers should prefer
    /// [`AbilityRegistry::iter_for`] so the actor gate stays in force.
    pub fn iter_all(&self) -> impl Iterator<Item = &AbilityDescriptor> {
        self.by_name.values()
    }

    pub fn iter_for(&self, actor: Actor) -> impl Iterator<Item = &AbilityDescriptor> {
        self.by_name.values().filter(move |descriptor| {
            if descriptor.experimental && actor != Actor::System {
                return false;
            }
            if actor == Actor::Agent && descriptor.category == AbilityCategory::Maintenance {
                return false;
            }
            descriptor.policy.allowed_actors.contains(&actor.kind())
        })
    }

    pub async fn invoke_read(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Read)
            .await
    }

    pub async fn invoke_transform(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Transform)
            .await
    }

    pub async fn invoke_publish(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Publish)
            .await
    }

    pub async fn invoke_maintenance(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Maintenance)
            .await
    }

    pub async fn invoke_by_name_json(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        let descriptor = self.descriptor(name)?;
        validate_invocation_policy(ctx, descriptor)?;
        (descriptor.invoke_erased)(ctx, input).await
    }

    /// Render docs as deterministic filename/body pairs.
    pub fn render_docs(&self) -> BTreeMap<String, String> {
        let descriptors: BTreeMap<&str, &AbilityDescriptor> = self
            .by_name
            .iter()
            .map(|(name, descriptor)| (*name, descriptor))
            .collect();

        let mut rendered = BTreeMap::new();
        for (name, descriptor) in descriptors {
            let input_schema = serde_json::to_string_pretty(&(descriptor.input_schema)())
                .unwrap_or_else(|_| "{}".to_string());
            let output_schema = serde_json::to_string_pretty(&(descriptor.output_schema)())
                .unwrap_or_else(|_| "{}".to_string());
            rendered.insert(
                format!("{name}.md"),
                render_descriptor_doc(descriptor, &input_schema, &output_schema),
            );
        }
        rendered
    }

    async fn invoke_with_category(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
        expected_category: AbilityCategory,
    ) -> Result<serde_json::Value, AbilityError> {
        let descriptor = self.descriptor(name)?;
        if descriptor.category != expected_category {
            return Err(AbilityError {
                kind: AbilityErrorKind::Validation,
                message: format!(
                    "ability `{}` is {:?}, expected {:?}",
                    descriptor.name, descriptor.category, expected_category
                ),
            });
        }
        validate_invocation_policy(ctx, descriptor)?;
        (descriptor.invoke_erased)(ctx, input).await
    }

    fn descriptor(&self, name: &str) -> Result<&AbilityDescriptor, AbilityError> {
        self.by_name.get(name).ok_or_else(|| AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("unknown ability `{name}`"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaClosureError {
    pub ability_name: String,
    pub pointer: String,
}

impl SchemaClosureError {
    fn new(ability_name: impl Into<String>, pointer: impl Into<String>) -> Self {
        Self {
            ability_name: ability_name.into(),
            pointer: pointer.into(),
        }
    }
}

impl std::fmt::Display for SchemaClosureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pointer = if self.pointer.is_empty() {
            "<root>"
        } else {
            self.pointer.as_str()
        };
        write!(
            formatter,
            "ability `{}` input schema object at `{}` must set additionalProperties: false",
            self.ability_name, pointer
        )
    }
}

impl std::error::Error for SchemaClosureError {}

pub fn validate_schema_closure(schema: &serde_json::Value) -> Result<(), SchemaClosureError> {
    validate_schema_closure_for_ability(UNKNOWN_SCHEMA_ABILITY, schema)
}

pub fn validate_schema_closure_for_ability(
    ability_name: &str,
    schema: &serde_json::Value,
) -> Result<(), SchemaClosureError> {
    validate_schema_closure_at(schema, "", ability_name)
}

pub fn close_schema_objects(schema: &mut serde_json::Value) {
    close_schema_objects_at(schema);
}

fn validate_descriptor_schema_closures(
    descriptors: &[AbilityDescriptor],
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors {
        if let Err(error) =
            validate_schema_closure_for_ability(descriptor.name, &(descriptor.input_schema)())
        {
            violations.push(RegistryViolation::SchemaClosure(error));
        }
    }
}

fn validate_schema_closure_at(
    schema: &serde_json::Value,
    pointer: &str,
    ability_name: &str,
) -> Result<(), SchemaClosureError> {
    let Some(object) = schema.as_object() else {
        return Ok(());
    };

    if is_object_schema(object)
        && object.get("additionalProperties") != Some(&serde_json::Value::Bool(false))
    {
        return Err(SchemaClosureError::new(ability_name, pointer));
    }

    walk_schema_children(object, pointer, |child, child_pointer| {
        validate_schema_closure_at(child, &child_pointer, ability_name)
    })
}

fn close_schema_objects_at(schema: &mut serde_json::Value) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };

    if is_object_schema(object) {
        object.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
    }

    walk_schema_children_mut(object);
}

fn is_object_schema(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    has_object_type(object) || (object.get("type").is_none() && object.contains_key("properties"))
}

fn has_object_type(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    match object.get("type") {
        Some(serde_json::Value::String(schema_type)) => schema_type == "object",
        Some(serde_json::Value::Array(schema_types)) => schema_types
            .iter()
            .any(|schema_type| schema_type.as_str() == Some("object")),
        _ => false,
    }
}

fn walk_schema_children<F>(
    object: &serde_json::Map<String, serde_json::Value>,
    pointer: &str,
    mut walk: F,
) -> Result<(), SchemaClosureError>
where
    F: FnMut(&serde_json::Value, String) -> Result<(), SchemaClosureError>,
{
    for keyword in [
        "properties",
        "patternProperties",
        "definitions",
        "$defs",
        "dependentSchemas",
    ] {
        if let Some(serde_json::Value::Object(children)) = object.get(keyword) {
            for (name, child) in children {
                walk(child, pointer_child(pointer, keyword, name))?;
            }
        }
    }

    for keyword in [
        "items",
        "additionalItems",
        "contains",
        "propertyNames",
        "not",
        "if",
        "then",
        "else",
    ] {
        if let Some(child) = object.get(keyword) {
            walk_schema_or_schema_array(child, &pointer_segment(pointer, keyword), &mut walk)?;
        }
    }

    for keyword in ["oneOf", "anyOf", "allOf", "prefixItems"] {
        if let Some(child) = object.get(keyword) {
            walk_schema_array(child, &pointer_segment(pointer, keyword), &mut walk)?;
        }
    }

    Ok(())
}

fn walk_schema_or_schema_array<F>(
    value: &serde_json::Value,
    pointer: &str,
    walk: &mut F,
) -> Result<(), SchemaClosureError>
where
    F: FnMut(&serde_json::Value, String) -> Result<(), SchemaClosureError>,
{
    match value {
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk(item, pointer_segment(pointer, &index.to_string()))?;
            }
            Ok(())
        }
        _ => walk(value, pointer.to_string()),
    }
}

fn walk_schema_array<F>(
    value: &serde_json::Value,
    pointer: &str,
    walk: &mut F,
) -> Result<(), SchemaClosureError>
where
    F: FnMut(&serde_json::Value, String) -> Result<(), SchemaClosureError>,
{
    let serde_json::Value::Array(items) = value else {
        return Ok(());
    };

    for (index, item) in items.iter().enumerate() {
        walk(item, pointer_segment(pointer, &index.to_string()))?;
    }

    Ok(())
}

fn walk_schema_children_mut(object: &mut serde_json::Map<String, serde_json::Value>) {
    for keyword in [
        "properties",
        "patternProperties",
        "definitions",
        "$defs",
        "dependentSchemas",
    ] {
        if let Some(serde_json::Value::Object(children)) = object.get_mut(keyword) {
            for child in children.values_mut() {
                close_schema_objects_at(child);
            }
        }
    }

    for keyword in [
        "items",
        "additionalItems",
        "contains",
        "propertyNames",
        "not",
        "if",
        "then",
        "else",
    ] {
        if let Some(child) = object.get_mut(keyword) {
            close_schema_or_schema_array(child);
        }
    }

    for keyword in ["oneOf", "anyOf", "allOf", "prefixItems"] {
        if let Some(serde_json::Value::Array(children)) = object.get_mut(keyword) {
            for child in children {
                close_schema_objects_at(child);
            }
        }
    }
}

fn close_schema_or_schema_array(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                close_schema_objects_at(item);
            }
        }
        _ => close_schema_objects_at(value),
    }
}

fn pointer_child(pointer: &str, keyword: &str, child: &str) -> String {
    pointer_segment(&pointer_segment(pointer, keyword), child)
}

fn pointer_segment(pointer: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    if pointer.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{pointer}/{escaped}")
    }
}

fn validate_unknown_composes(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors_sorted(by_name) {
        for entry in descriptor.composes {
            if !by_name.contains_key(entry.ability) {
                violations.push(RegistryViolation::UnknownComposes {
                    ability: descriptor.name.to_string(),
                    target: entry.ability.to_string(),
                });
            }
        }
    }
}

fn validate_cycles(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        Unvisited,
        Visiting,
        Done,
    }

    fn visit(
        name: &'static str,
        by_name: &HashMap<&'static str, AbilityDescriptor>,
        color: &mut HashMap<&'static str, Color>,
        stack: &mut Vec<&'static str>,
        violations: &mut Vec<RegistryViolation>,
    ) {
        color.insert(name, Color::Visiting);
        stack.push(name);

        if let Some(descriptor) = by_name.get(name) {
            let mut targets: Vec<&'static str> = descriptor
                .composes
                .iter()
                .filter_map(|entry| by_name.get(entry.ability).map(|target| target.name))
                .collect();
            targets.sort_unstable();

            for target in targets {
                match color.get(target).copied().unwrap_or(Color::Unvisited) {
                    Color::Unvisited => visit(target, by_name, color, stack, violations),
                    Color::Visiting => {
                        if let Some(pos) = stack.iter().position(|stacked| *stacked == target) {
                            let mut cycle: Vec<String> = stack[pos..]
                                .iter()
                                .map(|entry| (*entry).to_string())
                                .collect();
                            cycle.push(target.to_string());
                            violations.push(RegistryViolation::CompositionCycle(cycle));
                        }
                    }
                    Color::Done => {}
                }
            }
        }

        stack.pop();
        color.insert(name, Color::Done);
    }

    let mut color: HashMap<&'static str, Color> = by_name
        .keys()
        .map(|name| (*name, Color::Unvisited))
        .collect();
    let mut stack = Vec::new();

    for name in names_sorted(by_name) {
        if matches!(color.get(name), Some(Color::Unvisited)) {
            visit(name, by_name, &mut color, &mut stack, violations);
        }
    }
}

fn validate_category_transitivity(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors_sorted(by_name) {
        let sensitive_category = matches!(
            descriptor.category,
            AbilityCategory::Read | AbilityCategory::Transform
        );

        if sensitive_category && !descriptor.mutates.is_empty() {
            violations.push(RegistryViolation::CategoryViolation {
                ability: descriptor.name.to_string(),
                category: descriptor.category,
                transitively_composes: descriptor.category,
            });
            continue;
        }

        for descendant_name in descendant_names(descriptor.name, by_name) {
            let descendant = &by_name[descendant_name];
            if sensitive_category
                && (matches!(
                    descendant.category,
                    AbilityCategory::Publish | AbilityCategory::Maintenance
                ) || !descendant.mutates.is_empty())
            {
                violations.push(RegistryViolation::CategoryViolation {
                    ability: descriptor.name.to_string(),
                    category: descriptor.category,
                    transitively_composes: descendant.category,
                });
                break;
            }

            if descriptor.category == AbilityCategory::Maintenance
                && !descriptor.policy.may_publish
                && descendant.category == AbilityCategory::Publish
            {
                violations.push(RegistryViolation::CategoryViolation {
                    ability: descriptor.name.to_string(),
                    category: descriptor.category,
                    transitively_composes: descendant.category,
                });
                break;
            }
        }
    }
}

fn validate_experimental(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors_sorted(by_name) {
        if !descriptor.experimental {
            continue;
        }

        if !cfg!(feature = "experimental") {
            violations.push(RegistryViolation::ExperimentalInProduction);
        }

        let Some(registered_at) = descriptor.registered_at else {
            violations.push(RegistryViolation::ExperimentalMissingRegisteredAt(
                descriptor.name.to_string(),
            ));
            continue;
        };

        let Ok(parsed) = DateTime::parse_from_rfc3339(registered_at) else {
            violations.push(RegistryViolation::ExperimentalMissingRegisteredAt(
                descriptor.name.to_string(),
            ));
            continue;
        };

        let age_days = Utc::now()
            .signed_duration_since(parsed.with_timezone(&Utc))
            .num_days();
        if age_days > 90 {
            violations.push(RegistryViolation::ExperimentalExpired {
                ability: descriptor.name.to_string(),
                age_days,
            });
        }
    }
}

fn validate_invocation_policy(
    ctx: &AbilityContext<'_>,
    descriptor: &AbilityDescriptor,
) -> Result<(), AbilityError> {
    if !descriptor.policy.allowed_actors.contains(&ctx.actor.kind()) {
        return Err(AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!(
                "actor {:?} is not allowed to invoke `{}`",
                ctx.actor, descriptor.name
            ),
        });
    }

    if !descriptor.policy.allowed_modes.contains(&ctx.mode()) {
        return Err(AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!(
                "mode {:?} is not allowed to invoke `{}`",
                ctx.mode(),
                descriptor.name
            ),
        });
    }

    let requires_confirmation =
        descriptor.policy.requires_confirmation || descriptor.category == AbilityCategory::Publish;
    if requires_confirmation && ctx.confirmation.is_none() {
        return Err(AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!("ability `{}` requires confirmation", descriptor.name),
        });
    }

    Ok(())
}

fn descendant_names(
    name: &'static str,
    by_name: &HashMap<&'static str, AbilityDescriptor>,
) -> Vec<&'static str> {
    fn walk(
        current: &'static str,
        by_name: &HashMap<&'static str, AbilityDescriptor>,
        seen: &mut HashSet<&'static str>,
        out: &mut Vec<&'static str>,
    ) {
        let Some(descriptor) = by_name.get(current) else {
            return;
        };

        let mut targets: Vec<&'static str> = descriptor
            .composes
            .iter()
            .filter_map(|entry| by_name.get(entry.ability).map(|target| target.name))
            .collect();
        targets.sort_unstable();

        for target in targets {
            if seen.insert(target) {
                out.push(target);
                walk(target, by_name, seen, out);
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    walk(name, by_name, &mut seen, &mut out);
    out
}

fn descriptors_sorted<'a>(
    by_name: &'a HashMap<&'static str, AbilityDescriptor>,
) -> Vec<&'a AbilityDescriptor> {
    let mut descriptors: Vec<&'a AbilityDescriptor> = by_name.values().collect();
    descriptors.sort_by_key(|descriptor| descriptor.name);
    descriptors
}

fn names_sorted(by_name: &HashMap<&'static str, AbilityDescriptor>) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = by_name.keys().copied().collect();
    names.sort_unstable();
    names
}

fn render_descriptor_doc(
    descriptor: &AbilityDescriptor,
    input_schema: &str,
    output_schema: &str,
) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    push_yaml_string(&mut out, "name", descriptor.name);
    push_yaml_string(&mut out, "version", descriptor.version);
    out.push_str(&format!("schema_version: {}\n", descriptor.schema_version));
    out.push_str(&format!("category: {:?}\n", descriptor.category));
    out.push_str(&format!("experimental: {}\n", descriptor.experimental));
    push_yaml_string_list(
        &mut out,
        "allowed_actors",
        descriptor
            .policy
            .allowed_actors
            .iter()
            .map(|actor| format!("{actor:?}")),
    );
    push_yaml_string_list(
        &mut out,
        "allowed_modes",
        descriptor
            .policy
            .allowed_modes
            .iter()
            .map(|mode| mode.as_str().to_string()),
    );
    out.push_str(&format!(
        "requires_confirmation: {}\n",
        descriptor.policy.requires_confirmation
    ));
    out.push_str(&format!("may_publish: {}\n", descriptor.policy.may_publish));
    push_yaml_string_list(
        &mut out,
        "mutates",
        descriptor.mutates.iter().map(|value| (*value).to_string()),
    );
    out.push_str("composes:");
    if descriptor.composes.is_empty() {
        out.push_str(" []\n");
    } else {
        out.push('\n');
        for entry in descriptor.composes {
            out.push_str(&format!("  - id: {}\n", yaml_string(entry.id.as_str())));
            out.push_str(&format!("    ability: {}\n", yaml_string(entry.ability)));
            out.push_str(&format!("    optional: {}\n", entry.optional));
        }
    }
    out.push_str("signal_policy:\n");
    push_yaml_string_list_indented(
        &mut out,
        "emits_on_output_change",
        descriptor
            .signal_policy
            .emits_on_output_change
            .iter()
            .map(|value| (*value).to_string()),
        "  ",
    );
    out.push_str(&format!(
        "  coalesce: {}\n",
        descriptor.signal_policy.coalesce
    ));
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n", descriptor.name));
    out.push_str("## Policy\n\n");
    out.push_str(&format!("- Category: `{:?}`\n", descriptor.category));
    out.push_str(&format!(
        "- Requires confirmation: `{}`\n",
        descriptor.policy.requires_confirmation
    ));
    out.push_str(&format!(
        "- May publish from maintenance: `{}`\n\n",
        descriptor.policy.may_publish
    ));
    out.push_str("## Input Schema\n\n```json\n");
    out.push_str(input_schema);
    out.push_str("\n```\n\n## Output Schema\n\n```json\n");
    out.push_str(output_schema);
    out.push_str("\n```\n\n## Composition And Mutation Notes\n\n");
    out.push_str(&format!(
        "- Composes: `{}` entries\n",
        descriptor.composes.len()
    ));
    out.push_str(&format!(
        "- Mutates: `{}` entries\n",
        descriptor.mutates.len()
    ));
    out
}

fn push_yaml_string(out: &mut String, key: &str, value: &str) {
    out.push_str(&format!("{key}: {}\n", yaml_string(value)));
}

fn push_yaml_string_list<I>(out: &mut String, key: &str, values: I)
where
    I: IntoIterator<Item = String>,
{
    push_yaml_string_list_indented(out, key, values, "");
}

fn push_yaml_string_list_indented<I>(out: &mut String, key: &str, values: I, indent: &str)
where
    I: IntoIterator<Item = String>,
{
    let values: Vec<String> = values.into_iter().collect();
    out.push_str(indent);
    out.push_str(key);
    out.push(':');
    if values.is_empty() {
        out.push_str(" []\n");
    } else {
        out.push('\n');
        for value in values {
            out.push_str(indent);
            out.push_str("  - ");
            out.push_str(&yaml_string(&value));
            out.push('\n');
        }
    }
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string to JSON cannot fail")
}

#[cfg(test)]
#[allow(clippy::manual_is_multiple_of, clippy::needless_range_loop)]
mod tests {
    use super::*;
    use crate::abilities::tracer::{AbilityTracer, SpanHandle};
    use crate::intelligence::provider::{ModelName, ModelTier, ProviderKind, ReplayProvider};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use chrono::TimeZone;
    use std::sync::Mutex;

    fn ok_erased<'a>(
        _ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>,
    > {
        Box::pin(async move { Ok(input) })
    }

    fn empty_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "additionalProperties": false
        })
    }

    fn open_object_schema() -> serde_json::Value {
        serde_json::json!({ "type": "object" })
    }

    fn with_input_schema(
        mut descriptor: AbilityDescriptor,
        input_schema: fn() -> serde_json::Value,
    ) -> AbilityDescriptor {
        descriptor.input_schema = input_schema;
        descriptor
    }

    inventory::submit! {
        AbilityDescriptor {
            name: "dos210_inventory_fixture",
            version: "0.1.0",
            schema_version: 1,
            category: AbilityCategory::Read,
            policy: AbilityPolicy {
                allowed_actors: &[],
                allowed_modes: &[],
                requires_confirmation: false,
                may_publish: false,
                required_scopes: &[],
                mcp_exposure: McpExposure::None,
                client_side_executable: false,
                rate_limit: None,
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

    fn descriptor(name: &'static str, category: AbilityCategory) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "0.1.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: &[ActorKind::Agent, ActorKind::User, ActorKind::System],
                allowed_modes: &[
                    ExecutionMode::Live,
                    ExecutionMode::Simulate,
                    ExecutionMode::Evaluate,
                ],
                requires_confirmation: false,
                may_publish: false,
                required_scopes: &[],
                mcp_exposure: McpExposure::None,
                client_side_executable: false,
                rate_limit: None,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy::default(),
            invoke_erased: ok_erased,
            input_schema: empty_schema,
            output_schema: empty_schema,
        }
    }

    fn static_slice<T>(values: Vec<T>) -> &'static [T] {
        Box::leak(values.into_boxed_slice())
    }

    fn push_compose(descriptor: &mut AbilityDescriptor, entry: ComposesEntry) {
        let mut composes = descriptor.composes.to_vec();
        composes.push(entry);
        descriptor.composes = static_slice(composes);
    }

    fn compose(mut descriptor: AbilityDescriptor, target: &'static str) -> AbilityDescriptor {
        let id = CompositionId::new(format!("{}_to_{target}", descriptor.name));
        push_compose(
            &mut descriptor,
            ComposesEntry {
                id,
                ability: target,
                optional: false,
            },
        );
        descriptor
    }

    fn with_mutates(
        mut descriptor: AbilityDescriptor,
        mutates: Vec<&'static str>,
    ) -> AbilityDescriptor {
        descriptor.mutates = static_slice(mutates);
        descriptor
    }

    fn with_actor_policy(
        mut descriptor: AbilityDescriptor,
        actors: Vec<ActorKind>,
    ) -> AbilityDescriptor {
        descriptor.policy.allowed_actors = static_slice(actors);
        descriptor
    }

    fn experimental(
        mut descriptor: AbilityDescriptor,
        registered_at: &'static str,
    ) -> AbilityDescriptor {
        descriptor.experimental = true;
        descriptor.registered_at = Some(registered_at);
        descriptor
    }

    fn registry(descriptors: Vec<AbilityDescriptor>) -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(descriptors).unwrap()
    }

    fn context<'a>(
        services: &'a ServiceContext<'a>,
        actor: Actor,
        confirmation: Option<&'a dyn ConfirmationProof>,
        provider: &'a ReplayProvider,
        tracer: &'a dyn AbilityTracer,
    ) -> AbilityContext<'a> {
        AbilityContext::new(
            services,
            provider,
            tracer,
            actor,
            confirmation,
            ClaimDismissalSurface::Eval,
        )
    }

    fn services<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external)
    }

    fn static_name(prefix: &str, case: usize, index: usize) -> &'static str {
        Box::leak(format!("{prefix}_{case}_{index}").into_boxed_str())
    }

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        *seed
    }

    #[derive(Default)]
    struct RecordingTracer {
        events: Mutex<Vec<String>>,
    }

    impl AbilityTracer for RecordingTracer {
        fn start_span(&self, name: &str) -> SpanHandle {
            self.events.lock().unwrap().push(format!("span:{name}"));
            SpanHandle { id: 217 }
        }

        fn record_event(&self, span: &SpanHandle, name: &str, _fields: serde_json::Value) {
            self.events
                .lock()
                .unwrap()
                .push(format!("event:{}:{name}", span.id));
        }
    }

    fn fixture_provider() -> ReplayProvider {
        ReplayProvider::new(std::collections::HashMap::new())
            .with_provider_kind(ProviderKind::Other("registry-fixture"))
            .with_model_for_tier(ModelTier::Synthesis, ModelName::new("registry-model"))
    }

    #[test]
    fn validate_schema_closure_passes_for_closed_object_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "child": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "value": { "type": "string" }
                    }
                },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false
                    }
                }
            },
            "$defs": {
                "choice": {
                    "oneOf": [
                        {
                            "type": "object",
                            "additionalProperties": false
                        }
                    ]
                }
            }
        });

        validate_schema_closure(&schema).unwrap();
    }

    #[test]
    fn validate_schema_closure_fails_for_top_level_object_missing_additional_properties() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object"
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "");
    }

    #[test]
    fn validate_schema_closure_fails_for_nested_object_in_properties() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "child": { "type": "object" }
            }
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "/properties/child");
    }

    #[test]
    fn validate_schema_closure_fails_for_nested_object_in_array_items() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "type": "object" }
                }
            }
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "/properties/items/items");
    }

    #[test]
    fn validate_schema_closure_fails_for_object_in_one_of() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "oneOf": [
                { "type": "object" }
            ]
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "/oneOf/0");
    }

    #[test]
    fn validate_schema_closure_includes_violating_path_in_error() {
        let error = validate_schema_closure_for_ability(
            "path_fixture",
            &serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "child": { "type": "object" }
                }
            }),
        )
        .unwrap_err();

        assert_eq!(error.ability_name, "path_fixture");
        assert_eq!(error.pointer, "/properties/child");
        assert!(error.to_string().contains("path_fixture"));
        assert!(error.to_string().contains("/properties/child"));
    }

    #[test]
    #[should_panic(expected = "schema closure")]
    fn registry_build_panics_on_descriptor_with_open_input_schema() {
        let result = AbilityRegistry::from_descriptors_checked(vec![with_input_schema(
            descriptor("open_schema", AbilityCategory::Read),
            open_object_schema,
        )]);

        if let Err(violations) = result {
            panic!("schema closure violation rejected registry build: {violations:?}");
        }
    }

    #[test]
    #[should_panic(expected = "schema closure")]
    fn registry_build_panics_on_descriptor_without_additional_properties_false() {
        let result = AbilityRegistry::from_descriptors_checked(vec![with_input_schema(
            descriptor(
                "open_schema_without_additional_properties_false",
                AbilityCategory::Read,
            ),
            open_object_schema,
        )]);

        if let Err(violations) = result {
            panic!("schema closure violation rejected registry build: {violations:?}");
        }
    }

    #[test]
    fn ability_context_exposes_provider_and_tracer() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(217);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();

        let ctx = AbilityContext::new(
            &services,
            &provider,
            &tracer,
            Actor::User,
            None,
            ClaimDismissalSurface::TauriEntityDetail,
        );
        let span = ctx.tracer.start_span("ability_context");
        ctx.tracer
            .record_event(&span, "provider_visible", serde_json::json!({}));

        assert_eq!(
            ctx.provider.provider_kind(),
            ProviderKind::Other("registry-fixture")
        );
        assert_eq!(
            ctx.provider.current_model(ModelTier::Synthesis).as_str(),
            "registry-model"
        );
        assert_eq!(
            tracer.events.lock().unwrap().as_slice(),
            ["span:ability_context", "event:217:provider_visible"]
        );
    }

    #[test]
    fn ability_context_constructed_with_capabilities_does_not_require_action_db() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(217);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();

        let ctx = AbilityContext::new(
            &services,
            &provider,
            &tracer,
            Actor::Agent,
            None,
            ClaimDismissalSurface::TauriEntityDetail,
        );

        assert_eq!(ctx.actor, Actor::Agent);
        assert_eq!(ctx.mode(), ExecutionMode::Live);
        assert_eq!(
            ctx.provider.provider_kind(),
            ProviderKind::Other("registry-fixture")
        );
    }

    #[test]
    fn registry_collects_inventory_descriptors() {
        let registry = AbilityRegistry::from_inventory_checked().unwrap();

        assert!(registry.by_name.contains_key("dos210_inventory_fixture"));
    }

    #[test]
    fn registry_rejects_duplicate_names_with_clear_error() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            descriptor("duplicate", AbilityCategory::Read),
            descriptor("duplicate", AbilityCategory::Transform),
        ])
        .unwrap_err();

        assert!(
            violations.contains(&RegistryViolation::DuplicateAbilityName(
                "duplicate".to_string()
            ))
        );
    }

    #[test]
    fn registry_rejects_unknown_composes() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![compose(
            descriptor("reader", AbilityCategory::Read),
            "missing",
        )])
        .unwrap_err();

        assert!(violations.contains(&RegistryViolation::UnknownComposes {
            ability: "reader".to_string(),
            target: "missing".to_string(),
        }));
    }

    #[test]
    fn registry_rejects_read_composing_publish_transitively() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            compose(descriptor("read", AbilityCategory::Read), "transform"),
            compose(
                descriptor("transform", AbilityCategory::Transform),
                "publish",
            ),
            descriptor("publish", AbilityCategory::Publish),
        ])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::CategoryViolation {
                ability,
                category: AbilityCategory::Read,
                transitively_composes: AbilityCategory::Publish,
            } if ability == "read"
        )));
    }

    #[test]
    fn registry_rejects_transform_composing_maintenance_transitively() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            compose(descriptor("transform", AbilityCategory::Transform), "read"),
            compose(descriptor("read", AbilityCategory::Read), "maintenance"),
            descriptor("maintenance", AbilityCategory::Maintenance),
        ])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::CategoryViolation {
                ability,
                category: AbilityCategory::Transform,
                transitively_composes: AbilityCategory::Maintenance,
            } if ability == "transform"
        )));
    }

    #[test]
    fn registry_iter_for_agent_hides_maintenance_and_admin() {
        let registry = registry(vec![
            descriptor("agent_read", AbilityCategory::Read),
            descriptor("agent_maintenance", AbilityCategory::Maintenance),
            with_actor_policy(
                descriptor("admin_read", AbilityCategory::Read),
                vec![ActorKind::Admin],
            ),
        ]);

        let names: HashSet<&str> = registry
            .iter_for(Actor::Agent)
            .map(|descriptor| descriptor.name)
            .collect();

        assert!(names.contains("agent_read"));
        assert!(!names.contains("agent_maintenance"));
        assert!(!names.contains("admin_read"));
    }

    #[cfg(not(feature = "experimental"))]
    #[test]
    fn registry_rejects_experimental_descriptor_in_production() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![experimental(
            descriptor("experimental_read", AbilityCategory::Read),
            "2999-01-01T00:00:00Z",
        )])
        .unwrap_err();

        assert!(violations.contains(&RegistryViolation::ExperimentalInProduction));
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn registry_iter_for_agent_hides_experimental_from_agent_when_feature_enabled() {
        let registry = registry(vec![experimental(
            descriptor("experimental_read", AbilityCategory::Read),
            "2999-01-01T00:00:00Z",
        )]);

        let names: HashSet<&str> = registry
            .iter_for(Actor::Agent)
            .map(|descriptor| descriptor.name)
            .collect();

        assert!(!names.contains("experimental_read"));
    }

    #[test]
    fn composition_graph_accepts_random_dag() {
        let mut seed = 7;
        for case in 0..100 {
            let names: Vec<&'static str> = (0..8)
                .map(|index| static_name("dag", case, index))
                .collect();
            let mut descriptors: Vec<AbilityDescriptor> = names
                .iter()
                .map(|name| descriptor(name, AbilityCategory::Read))
                .collect();

            for i in 0..descriptors.len() {
                for target in (i + 1)..descriptors.len() {
                    if lcg(&mut seed) % 4 == 0 {
                        push_compose(
                            &mut descriptors[i],
                            ComposesEntry {
                                id: CompositionId::new(format!("{i}_{target}")),
                                ability: names[target],
                                optional: false,
                            },
                        );
                    }
                }
            }

            AbilityRegistry::from_descriptors_checked(descriptors).unwrap();
        }
    }

    #[test]
    fn composition_graph_rejects_random_cycle() {
        let mut seed = 11;
        for case in 0..100 {
            let names: Vec<&'static str> = (0..6)
                .map(|index| static_name("cycle", case, index))
                .collect();
            let mut descriptors: Vec<AbilityDescriptor> = names
                .iter()
                .map(|name| descriptor(name, AbilityCategory::Read))
                .collect();

            for i in 0..descriptors.len() {
                let target = (i + 1) % descriptors.len();
                push_compose(
                    &mut descriptors[i],
                    ComposesEntry {
                        id: CompositionId::new(format!("{i}_{target}")),
                        ability: names[target],
                        optional: false,
                    },
                );
                if lcg(&mut seed) % 3 == 0 {
                    let extra = ((lcg(&mut seed) as usize) % descriptors.len()).max(i);
                    push_compose(
                        &mut descriptors[i],
                        ComposesEntry {
                            id: CompositionId::new(format!("{i}_{extra}_extra")),
                            ability: names[extra],
                            optional: false,
                        },
                    );
                }
            }

            let violations = AbilityRegistry::from_descriptors_checked(descriptors).unwrap_err();
            assert!(violations
                .iter()
                .any(|violation| matches!(violation, RegistryViolation::CompositionCycle(_))));
        }
    }

    #[test]
    fn composition_graph_folds_transitive_mutation_sets() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            compose(descriptor("read", AbilityCategory::Read), "child"),
            with_mutates(
                descriptor("child", AbilityCategory::Read),
                vec!["services::claims::commit_claim"],
            ),
        ])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::CategoryViolation {
                ability,
                category: AbilityCategory::Read,
                transitively_composes: AbilityCategory::Read,
            } if ability == "read"
        )));
    }

    #[test]
    fn experimental_expiry_rejects_over_90_days() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![experimental(
            descriptor("old_experiment", AbilityCategory::Read),
            "2020-01-01T00:00:00Z",
        )])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::ExperimentalExpired { ability, age_days }
                if ability == "old_experiment" && *age_days > 90
        )));
    }

    #[tokio::test]
    async fn invoke_read_rejects_transform_descriptor() {
        let registry = registry(vec![descriptor("transform", AbilityCategory::Transform)]);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();
        let ctx = context(&services, Actor::User, None, &provider, &tracer);

        let err = registry
            .invoke_read(&ctx, "transform", serde_json::json!({}))
            .await
            .unwrap_err();

        assert_eq!(err.kind, AbilityErrorKind::Validation);
    }

    #[tokio::test]
    async fn publish_requires_confirmation_token() {
        let registry = registry(vec![descriptor("publish", AbilityCategory::Publish)]);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();
        let ctx = context(&services, Actor::User, None, &provider, &tracer);

        let err = registry
            .invoke_publish(&ctx, "publish", serde_json::json!({}))
            .await
            .unwrap_err();

        assert_eq!(err.kind, AbilityErrorKind::Capability);
    }

    // -------------------------------------------------------------------
    // W1-A: SurfaceClient actor class — identity / scope newtype tests
    // ADR-0111 §8 and W1-A acceptance criteria.
    // -------------------------------------------------------------------

    #[test]
    fn surface_client_id_round_trip_preserves_value() {
        let id = SurfaceClientId::new("wp-instance-alpha");
        assert_eq!(id.as_str(), "wp-instance-alpha");
        assert_eq!(format!("{id}"), "wp-instance-alpha");
        assert_eq!(format!("{id:?}"), "SurfaceClientId(\"wp-instance-alpha\")");
    }

    #[test]
    fn surface_client_id_serde_round_trip_is_transparent() {
        let id = SurfaceClientId::new("wp-instance-alpha");
        let encoded = serde_json::to_string(&id).expect("serializes");
        // #[serde(transparent)] means the wire form is the inner string only.
        assert_eq!(encoded, "\"wp-instance-alpha\"");
        let decoded: SurfaceClientId = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded, id);
    }

    #[test]
    fn surface_client_id_hash_eq_match_inner_string() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SurfaceClientId::new("alpha"));
        set.insert(SurfaceClientId::new("beta"));
        set.insert(SurfaceClientId::new("alpha")); // dup
        assert_eq!(set.len(), 2);
        assert!(set.contains(&SurfaceClientId::new("alpha")));
        assert!(set.contains(&SurfaceClientId::new("beta")));
        assert!(!set.contains(&SurfaceClientId::new("gamma")));
    }

    #[test]
    fn surface_scope_round_trip_preserves_value() {
        let scope = SurfaceScope::new("read.account_overview");
        assert_eq!(scope.as_str(), "read.account_overview");
        assert_eq!(format!("{scope}"), "read.account_overview");
    }

    #[test]
    fn surface_scope_serde_round_trip_is_transparent() {
        let scope = SurfaceScope::new("write.feedback");
        let encoded = serde_json::to_string(&scope).expect("serializes");
        assert_eq!(encoded, "\"write.feedback\"");
        let decoded: SurfaceScope = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded, scope);
    }

    #[test]
    fn surface_scope_hash_eq_match_inner_string() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SurfaceScope::new("read.x"));
        set.insert(SurfaceScope::new("write.y"));
        set.insert(SurfaceScope::new("read.x")); // dup
        assert_eq!(set.len(), 2);
    }

    /// Test helper: build a non-empty [`ScopeSet`] from string slices.
    ///
    /// Panics if construction fails — only the empty-set construction can
    /// fail in lenient bootstrap mode, and the caller is expected to pass
    /// at least one scope.
    ///
    /// Seeds the process-global allowlist with every scope the caller is
    /// about to construct, so this test helper is safe to call regardless of
    /// whatever production initialization (e.g. an empty union from a
    /// scope-less ability registry) has happened earlier in the suite. See
    /// the doc on [`SCOPE_ALLOWLIST`] for the test-vs-prod split.
    fn scope_set(scopes: &[&str]) -> ScopeSet {
        seed_test_allowlist(scopes);
        ScopeSet::new(scopes.iter().map(|s| SurfaceScope::new(*s)))
            .expect("test scope set must be non-empty")
    }

    /// Install a per-test allowlist that includes every scope `scopes` plus
    /// the canonical W1-A.1 fixture vocabulary (`read.account_overview`,
    /// `submit.feedback`). Centralizes the bypass so individual tests do not
    /// have to remember the allowlist seam.
    fn seed_test_allowlist(scopes: &[&str]) {
        let mut all: BTreeSet<SurfaceScope> =
            scopes.iter().map(|s| SurfaceScope::new(*s)).collect();
        all.insert(SurfaceScope::new("read.account_overview"));
        all.insert(SurfaceScope::new("submit.feedback"));
        ScopeSet::set_allowlist_for_tests(all);
    }

    #[test]
    fn actor_surface_client_round_trip_preserves_identity() {
        let actor = Actor::SurfaceClient {
            instance: SurfaceClientId::new("wp-instance-alpha"),
            scopes: scope_set(&["read.account_overview"]),
        };
        match &actor {
            Actor::SurfaceClient { instance, scopes } => {
                assert_eq!(instance.as_str(), "wp-instance-alpha");
                assert!(scopes.contains(&SurfaceScope::new("read.account_overview")));
            }
            _ => panic!("expected SurfaceClient variant"),
        }
        // Clone preserves variant + identity.
        let cloned = actor.clone();
        assert_eq!(actor, cloned);
    }

    #[test]
    fn actor_surface_client_serde_round_trip() {
        let actor = Actor::SurfaceClient {
            instance: SurfaceClientId::new("wp-instance-alpha"),
            scopes: scope_set(&["read.account_overview", "submit.feedback"]),
        };
        let encoded = serde_json::to_string(&actor).expect("serializes");
        let decoded: Actor = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(actor, decoded);
    }

    #[test]
    fn actor_surface_client_distinct_instances_are_not_equal() {
        let alpha = Actor::SurfaceClient {
            instance: SurfaceClientId::new("alpha"),
            scopes: scope_set(&["read.account_overview"]),
        };
        let beta = Actor::SurfaceClient {
            instance: SurfaceClientId::new("beta"),
            scopes: scope_set(&["read.account_overview"]),
        };
        assert_ne!(alpha, beta);
        // ...and neither matches the unit variants.
        assert_ne!(alpha, Actor::User);
        assert_ne!(alpha, Actor::Agent);
        assert_ne!(alpha, Actor::Admin);
        assert_ne!(alpha, Actor::System);
    }

    #[test]
    fn actor_surface_client_not_in_user_agent_allowed_actors() {
        // W1-A acceptance criterion (negative): an ability marked
        // `allowed_actors: [User, Agent]` must NOT match a SurfaceClient
        // invocation. The full registry-boundary rejection (with PolicyError)
        // is W1-B's bridge work; this guard pins the `.contains` semantics
        // that the registry's `iter_for` filter and `validate_invocation_policy`
        // rely on.
        let allowed: &[Actor] = &[Actor::User, Actor::Agent];
        let invoker = Actor::SurfaceClient {
            instance: SurfaceClientId::new("wp-instance-alpha"),
            scopes: scope_set(&["read.account_overview"]),
        };
        assert!(!allowed.contains(&invoker));
    }

    // -------------------------------------------------------------------
    // W1-A.1: ScopeSet construction + deserialization invariants
    // -------------------------------------------------------------------

    #[test]
    fn scope_set_rejects_empty_construction() {
        let result = ScopeSet::new(std::iter::empty::<SurfaceScope>());
        assert!(matches!(result, Err(ScopeSetError::Empty)));
    }

    #[test]
    fn scope_set_accepts_non_empty_and_preserves_membership() {
        seed_test_allowlist(&["read.account_overview"]);
        let set = ScopeSet::new([SurfaceScope::new("read.account_overview")])
            .expect("non-empty construction succeeds");
        assert!(set.contains(&SurfaceScope::new("read.account_overview")));
        assert!(!set.contains(&SurfaceScope::new("submit.feedback")));
        assert_eq!(set.len(), 1);
        assert!(!set.is_empty());
    }

    #[test]
    fn scope_set_deserialization_rejects_empty_array() {
        let err = serde_json::from_str::<ScopeSet>("[]").expect_err("empty must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("at least one scope"),
            "unexpected error surface: {msg}"
        );
    }

    #[test]
    fn scope_set_deserialization_round_trip_for_non_empty() {
        seed_test_allowlist(&["read.account_overview", "submit.feedback"]);
        let json = "[\"read.account_overview\",\"submit.feedback\"]";
        let decoded: ScopeSet = serde_json::from_str(json).expect("decodes");
        assert!(decoded.contains(&SurfaceScope::new("read.account_overview")));
        assert!(decoded.contains(&SurfaceScope::new("submit.feedback")));
        // Round-trip back to JSON: BTreeSet ordering is deterministic.
        let encoded = serde_json::to_string(&decoded).expect("encodes");
        let redecoded: ScopeSet = serde_json::from_str(&encoded).expect("re-decodes");
        assert_eq!(decoded, redecoded);
    }

    // -------------------------------------------------------------------
    // W1-B: AbilityPolicy schema (required_scopes + mcp_exposure +
    // client_side_executable) and McpExposure serde.
    // ADR-0102 §7.1 + §7.6 (W0-D amended)AC lines 446-454.
    // -------------------------------------------------------------------

    #[test]
    fn ability_policy_default_has_closed_w1b_defaults() {
        let policy = AbilityPolicy::default();
        // Per W1-B AC §449 and ADR-0102 §7.6 (W0-D amended
        // 2026-05-10), the actor floor is `[User]` — least-privilege,
        // not closed-to-everyone.
        assert_eq!(policy.allowed_actors, &[ActorKind::User]);
        assert_eq!(policy.allowed_modes, &[] as &[ExecutionMode]);
        assert!(!policy.requires_confirmation);
        assert!(!policy.may_publish);
        // The three W1-B fields default to the closed forms.
        assert_eq!(policy.required_scopes, &[] as &[&str]);
        assert_eq!(policy.mcp_exposure, McpExposure::None);
        assert!(!policy.client_side_executable);
        assert_eq!(policy.rate_limit, None);
    }

    #[test]
    fn actor_kind_projects_each_variant_correctly() {
        // Ensure every Actor variant projects to its expected ActorKind.
        assert_eq!(Actor::Agent.kind(), ActorKind::Agent);
        assert_eq!(Actor::User.kind(), ActorKind::User);
        assert_eq!(Actor::Admin.kind(), ActorKind::Admin);
        assert_eq!(Actor::System.kind(), ActorKind::System);
        seed_test_allowlist(&["read.account_overview"]);
        let surface = Actor::SurfaceClient {
            instance: SurfaceClientId::new("wp-instance-alpha"),
            scopes: ScopeSet::new([SurfaceScope::new("read.account_overview")])
                .expect("non-empty scope set"),
        };
        assert_eq!(surface.kind(), ActorKind::SurfaceClient);
    }

    #[test]
    fn ability_policy_required_scopes_typed_materializes_surface_scopes() {
        let policy = AbilityPolicy {
            required_scopes: &["read.account_overview", "submit.feedback"],
            ..AbilityPolicy::default()
        };
        let typed = policy.required_scopes_typed();
        assert_eq!(typed.len(), 2);
        assert!(typed.contains(&SurfaceScope::new("read.account_overview")));
        assert!(typed.contains(&SurfaceScope::new("submit.feedback")));
    }

    #[test]
    fn ability_policy_effective_rate_limit_defaults_to_runtime_limit() {
        let policy = AbilityPolicy::default();
        let runtime_limit = AbilityRateLimit::new(120, 10);

        assert_eq!(policy.effective_rate_limit(runtime_limit), runtime_limit);
    }

    #[test]
    fn ability_policy_effective_rate_limit_is_lower_only() {
        let runtime_limit = AbilityRateLimit::new(120, 10);

        let looser_policy = AbilityPolicy {
            rate_limit: Some(AbilityRateLimit::new(240, 20)),
            ..AbilityPolicy::default()
        };
        assert_eq!(
            looser_policy.effective_rate_limit(runtime_limit),
            runtime_limit
        );

        let tighter_policy = AbilityPolicy {
            rate_limit: Some(AbilityRateLimit::new(60, 20)),
            ..AbilityPolicy::default()
        };
        assert_eq!(
            tighter_policy.effective_rate_limit(runtime_limit),
            AbilityRateLimit::new(60, 10)
        );
    }

    #[test]
    fn mcp_exposure_default_is_none() {
        assert_eq!(McpExposure::default(), McpExposure::None);
    }

    #[test]
    fn mcp_exposure_serde_round_trip_for_all_variants() {
        for variant in [
            McpExposure::None,
            McpExposure::MetadataOnly,
            McpExposure::Invocable,
        ] {
            let encoded = serde_json::to_string(&variant).expect("serializes");
            let decoded: McpExposure = serde_json::from_str(&encoded).expect("deserializes");
            assert_eq!(variant, decoded, "round-trip for {variant:?}");
        }
    }

    #[test]
    fn mcp_exposure_wire_form_is_snake_case() {
        // ADR-0102 §7.1 wire form: snake_case discriminant for portability.
        assert_eq!(
            serde_json::to_string(&McpExposure::None).expect("serializes"),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&McpExposure::MetadataOnly).expect("serializes"),
            "\"metadata_only\""
        );
        assert_eq!(
            serde_json::to_string(&McpExposure::Invocable).expect("serializes"),
            "\"invocable\""
        );
    }

    #[test]
    fn ability_policy_required_scopes_storage_is_static_slice() {
        // Type-level pin: storage shape is `&'static [&'static str]` so
        // descriptors remain const-constructible for `inventory::submit!`.
        // The typed accessor is `required_scopes_typed`. If this ever
        // changes, every macro-emitted static breaks at compile time.
        const POLICY: AbilityPolicy = AbilityPolicy {
            allowed_actors: &[],
            allowed_modes: &[],
            requires_confirmation: false,
            may_publish: false,
            required_scopes: &["read.account_overview"],
            mcp_exposure: McpExposure::None,
            client_side_executable: false,
            rate_limit: None,
        };
        assert_eq!(POLICY.required_scopes, &["read.account_overview"]);
    }
}
