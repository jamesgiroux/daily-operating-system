//! W4-A composition render orchestrator.
//!
//! Single entry-point that bridges the WordPress block surface to the
//! abilities-runtime composition projector. PHP calls
//! `/v1/surface/project-composition` → this service:
//!
//! 1. Looks up the in-memory cache keyed by
//!    `(composition_id, composition_version, scopes_canonical_id)`.
//! 2. On hit: returns the cached `ProjectedComposition` + a refreshed
//!    `cache_hint_token` and emits `projection_cache_served`.
//! 3. On miss: invokes the W4-A0 producer ability
//!    (`dailyos/account-overview`) → gets `AbilityOutput<Composition>` →
//!    runs W4-D `project_composition_for_surface(composition, ctx)` →
//!    drains `Vec<AuditIntent>` → caches the result → returns
//!    `ProjectedComposition` + a new `cache_hint_token`.
//!
//! The substrate owns the cache and the scope-identity authority per
//! packet §6.2 V3 / §6.12. PHP receives only an opaque `cache_hint_token`
//! that it echoes back; it never derives or interprets the token.
//!
//! Cache scope-identity key: SHA256 of the sorted scope strings of the
//! authenticated `Actor::SurfaceClient`. Scope-change → key change →
//! natural miss-and-recompute (no separate invalidation path needed).

use std::sync::Arc;
use std::time::{Duration, Instant};

use abilities_runtime::abilities::composition::Composition;
use abilities_runtime::abilities::{
    project_composition_for_surface as project_for_surface_fn, AuditIntent,
    FallbackProjectionContext, ProjectedComposition, SurfaceKind,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use dashmap::DashMap;
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};

use crate::abilities::registry::ScopeSet;
use crate::abilities::Actor;

/// Cache TTL per packet §6.2 V3 (matches W4-E nonce lifetime).
pub const CACHE_TTL: Duration = Duration::from_secs(60);

/// Fallback projection policy version — bumps trigger cache invalidation
/// because the policy version contributes to the projected output. Pinned
/// here so the route handler doesn't have to know how to construct the
/// `FallbackProjectionContext`.
pub const FALLBACK_POLICY_VERSION: u32 = 1;

#[derive(Clone, Hash, Eq, PartialEq)]
struct CacheKey {
    composition_id: String,
    composition_version: i64,
    scopes_canonical_id: String,
}

struct CacheEntry {
    projection: ProjectedComposition,
    cache_hint_token: String,
    cached_at: Instant,
}

/// W4-A render orchestrator. Lives in `AppState` as a singleton.
pub struct CompositionRenderOrchestrator {
    cache: DashMap<CacheKey, CacheEntry>,
    rng: SystemRandom,
}

impl Default for CompositionRenderOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositionRenderOrchestrator {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            rng: SystemRandom::new(),
        }
    }

    fn make_cache_key(
        actor: &Actor,
        composition_id: &str,
        composition_version: i64,
    ) -> Option<CacheKey> {
        let scopes = match actor {
            Actor::SurfaceClient { scopes, .. } => scopes,
            _ => return None,
        };
        Some(CacheKey {
            composition_id: composition_id.to_string(),
            composition_version,
            scopes_canonical_id: scopes_canonical_id(scopes),
        })
    }

    /// Cache lookup. Returns the cached projection + a refreshed
    /// `cache_hint_token` on hit; `None` on miss or expired entry.
    pub fn cache_lookup(
        self: &Arc<Self>,
        actor: &Actor,
        composition_id: &str,
        composition_version: i64,
    ) -> Option<CachedProjection> {
        let key = Self::make_cache_key(actor, composition_id, composition_version)?;
        let entry = self.cache.get(&key)?;
        if entry.cached_at.elapsed() >= CACHE_TTL {
            return None;
        }
        Some(CachedProjection {
            projection: entry.projection.clone(),
            cache_hint_token: entry.cache_hint_token.clone(),
        })
    }

    /// Store an entry. The opaque `cache_hint_token` is freshly generated
    /// per insert so a captured token does not survive an entry-replace.
    pub fn cache_store(
        self: &Arc<Self>,
        actor: &Actor,
        composition_id: &str,
        composition_version: i64,
        projection: ProjectedComposition,
    ) -> Option<String> {
        let key = Self::make_cache_key(actor, composition_id, composition_version)?;
        let mut token_bytes = [0u8; 16];
        self.rng.fill(&mut token_bytes).ok()?;
        let cache_hint_token = URL_SAFE_NO_PAD.encode(token_bytes);
        let entry = CacheEntry {
            projection: projection.clone(),
            cache_hint_token: cache_hint_token.clone(),
            cached_at: Instant::now(),
        };
        self.cache.insert(key, entry);
        Some(cache_hint_token)
    }

    /// Test helper: cache size.
    #[doc(hidden)]
    pub fn __test_cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Test helper: drop all entries (e.g. after scope rotation in a test).
    #[doc(hidden)]
    pub fn __test_clear(&self) {
        self.cache.clear();
    }
}

/// Cache hit payload.
#[derive(Debug, Clone)]
pub struct CachedProjection {
    pub projection: ProjectedComposition,
    pub cache_hint_token: String,
}

fn scopes_canonical_id(scopes: &ScopeSet) -> String {
    let sorted: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
    let mut hasher = Sha256::new();
    hasher.update(sorted.join("\n").as_bytes());
    hex::encode(hasher.finalize())
}

/// Deserialize a `Composition` JSON value (the `data` field returned by the
/// abilities bridge) and project it for the SurfaceClient surface.
///
/// Returns `(projection, audits)` matching W4-D's contract. The caller is
/// responsible for draining `audits` through `emit_surface_audit` and
/// inserting the result into the cache via [`CompositionRenderOrchestrator::cache_store`].
pub fn project_from_ability_data(
    data: &serde_json::Value,
    actor: Actor,
    fallback_policy_version: u32,
) -> Result<(ProjectedComposition, Vec<AuditIntent>), OrchestratorError> {
    let composition: Composition = serde_json::from_value(data.clone())
        .map_err(|e| OrchestratorError::CompositionDeserialize(e.to_string()))?;
    let ctx = FallbackProjectionContext::new(
        actor,
        SurfaceKind::SurfaceClient,
        fallback_policy_version,
    );
    project_for_surface_fn(&composition, &ctx)
        .map_err(|e| OrchestratorError::ProjectionFailed(format!("{e:?}")))
}

/// Map a SurfaceClient request's composition_id back to the producer ability
/// name. v1.4.2 ships a single producer: `dailyos/account-overview`.
/// composition IDs of the form `dailyos/account-overview:account:{account_id}`
/// resolve to the account-overview ability; any other shape is rejected so
/// the orchestrator never invokes a foreign producer.
pub fn resolve_producer_ability_name(composition_id: &str) -> Option<&'static str> {
    if composition_id.starts_with("dailyos/account-overview:") {
        Some("dailyos/account-overview")
    } else {
        None
    }
}

/// Extract the account_id encoded in an `account-overview` composition_id.
/// Pattern: `dailyos/account-overview:account:{account_id}`.
pub fn extract_account_id_from_composition_id(composition_id: &str) -> Option<&str> {
    composition_id.strip_prefix("dailyos/account-overview:account:")
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("composition deserialize failed: {0}")]
    CompositionDeserialize(String),
    #[error("projection failed: {0}")]
    ProjectionFailed(String),
    #[error("unknown producer for composition: {0}")]
    UnknownProducer(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};

    fn surface_actor() -> Actor {
        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")]).unwrap();
        Actor::SurfaceClient {
            instance: SurfaceClientId::new("sc-test"),
            scopes,
        }
    }

    #[test]
    fn cache_round_trip() {
        // Build a minimal projection via JSON; ProjectedComposition is
        // Deserialize.
        let projection: ProjectedComposition = serde_json::from_value(serde_json::json!({
            "composition_id": "dailyos/account-overview:account:acct-1",
            "composition_version": 1,
            "fallback_policy_version": 1,
            "blocks": [],
            "diagnostics": [],
            "unknown_block_count": 0,
            "unknown_block_cap": 4,
            "dropped_unknown_block_count": 0
        })).expect("projection deserialize");

        let orchestrator = Arc::new(CompositionRenderOrchestrator::new());
        let actor = surface_actor();
        let token = orchestrator
            .cache_store(&actor, "dailyos/account-overview:account:acct-1", 1, projection)
            .expect("cache_store with SurfaceClient");
        assert!(!token.is_empty(), "non-empty cache_hint_token");

        let hit = orchestrator
            .cache_lookup(&actor, "dailyos/account-overview:account:acct-1", 1)
            .expect("cache hit");
        assert_eq!(hit.cache_hint_token, token);
    }

    #[test]
    fn cache_miss_on_scope_change() {
        let projection: ProjectedComposition = serde_json::from_value(serde_json::json!({
            "composition_id": "dailyos/account-overview:account:acct-2",
            "composition_version": 1,
            "fallback_policy_version": 1,
            "blocks": [],
            "diagnostics": [],
            "unknown_block_count": 0,
            "unknown_block_cap": 4,
            "dropped_unknown_block_count": 0
        })).expect("projection deserialize");
        let orchestrator = Arc::new(CompositionRenderOrchestrator::new());
        let actor_a = surface_actor();
        let _ = orchestrator
            .cache_store(&actor_a, "dailyos/account-overview:account:acct-2", 1, projection);

        // Different scope set → different canonical id → miss.
        let scopes_b = ScopeSet::new([
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
        ])
        .expect("scopes");
        let actor_b = Actor::SurfaceClient {
            instance: SurfaceClientId::new("sc-test"),
            scopes: scopes_b,
        };
        let hit = orchestrator.cache_lookup(&actor_b, "dailyos/account-overview:account:acct-2", 1);
        assert!(hit.is_none(), "scope change must produce miss");
    }

    #[test]
    fn unknown_producer_rejected() {
        assert_eq!(
            resolve_producer_ability_name("dailyos/account-overview:account:x"),
            Some("dailyos/account-overview")
        );
        assert!(resolve_producer_ability_name("foreign/ability").is_none());
    }

    #[test]
    fn account_id_extracts() {
        assert_eq!(
            extract_account_id_from_composition_id(
                "dailyos/account-overview:account:acct-42"
            ),
            Some("acct-42")
        );
        assert!(extract_account_id_from_composition_id("dailyos/other:foo").is_none());
    }
}
