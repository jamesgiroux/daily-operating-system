//! Substrate-grade envelope cache (L3 cycle-2 F4 hardening).
//!
//! ## Problem (code-reviewer cycle-1 F3 + L3 cycle-1 codex F4)
//!
//! `commands::claim_feedback::submit_claim_feedback_command` previously
//! constructed an `EnvelopeSet` from the targeted claim's OWN id, making the
//! `validate_envelope_target` binding check tautological — the set was built
//! to contain exactly the claim being mutated, so the check passed by
//! construction. AC-477.2 demands that the binding contract is enforced at the
//! actual Tauri call site.
//!
//! L3 cycle-1 codex challenge F4 surfaced a follow-on cross-actor-poisoning risk:
//! the cycle-2 cache keyed entries on `envelope_render_id` ALONE, so a renderer
//! that minted an envelope for one principal/surface could see that envelope
//! satisfy a feedback request from a different principal as long as the (RNG)
//! render id leaked. ADR-0125 (claim sensitivity / temporal scope) ties the
//! receipt boundary to the actor; the cache MUST honour that binding.
//!
//! ## Fix shape (cycle-2)
//!
//! 1. The TS hook receives an `envelopeRenderId` along with the rendered
//!    receipt — issued by the previous `render_claim_receipt` round-trip — and
//!    threads it back into `submit_claim_feedback_command`.
//! 2. The command looks up the cached envelope keyed by
//!    `(envelope_render_id, actor_principal_id, surface)` and uses that set as
//!    the EnvelopeSet basis.
//! 3. `validate_envelope_target` then binds the target's claim id against the
//!    cached envelope's claim ids — a real binding check, not a tautology, AND
//!    the lookup itself enforces actor + surface match (cross-principal
//!    lookups are rejected with [`EnvelopeCacheError::PrincipalMismatch`]).
//! 4. Cache miss → [`EnvelopeCacheError::EnvelopeRequired`]. The cycle-1
//!    tautological single-claim fallback is removed; callers MUST supply a
//!    valid `envelope_render_id` minted by a prior envelope render.

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes; receipts re-fetched on signal
const CACHE_MAX_ENTRIES: usize = 2_048;

/// Composite cache key: render id + actor principal + surface.
///
/// Render id alone is insufficient — it's an RNG-generated string that could
/// in principle leak between actors. Binding the lookup to `(actor, surface)`
/// makes cross-principal poisoning a hard error rather than a silent satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    envelope_render_id: String,
    actor_principal_id: String,
    surface: String,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    ability: String,
    claim_ids: BTreeSet<String>,
    proposal_ids: BTreeSet<String>,
    inserted_at: Instant,
}

/// Errors surfaced by [`lookup_envelope_for_render`]. Distinct from a `None`
/// return so the command layer can map them onto the appropriate
/// `BadRequest::EnvelopeRequired` / `BadRequest::PrincipalMismatch` wire
/// responses without ambiguity.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EnvelopeCacheError {
    /// No entry for `envelope_render_id` at all (or entry expired).
    #[error("envelope required: no envelope binding for render id `{0}` (cache miss or expired)")]
    EnvelopeRequired(String),
    /// An entry for `envelope_render_id` exists, but actor or surface does not
    /// match. Cross-principal lookup attempt — surfaces as forbidden.
    #[error(
        "principal mismatch: envelope `{render_id}` was minted for a different actor/surface"
    )]
    PrincipalMismatch { render_id: String },
}

struct EnvelopeCache {
    inner: RwLock<HashMap<CacheKey, CacheEntry>>,
    /// Secondary index: render id → key. Used to detect "entry exists but key
    /// mismatch" so we can return `PrincipalMismatch` instead of silently
    /// `EnvelopeRequired`.
    render_index: RwLock<HashMap<String, CacheKey>>,
}

impl EnvelopeCache {
    fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            render_index: RwLock::new(HashMap::new()),
        }
    }

    fn get(&self, key: &CacheKey) -> Result<CacheEntry, EnvelopeCacheError> {
        let now = Instant::now();

        // Fast path: exact key hit + fresh.
        let entry = {
            let guard = self.inner.read();
            guard.get(key).cloned()
        };
        if let Some(entry) = entry {
            if now.duration_since(entry.inserted_at) > CACHE_TTL {
                // Expired — evict and treat as missing.
                let mut guard = self.inner.write();
                guard.remove(key);
                let mut idx = self.render_index.write();
                idx.remove(&key.envelope_render_id);
                return Err(EnvelopeCacheError::EnvelopeRequired(
                    key.envelope_render_id.clone(),
                ));
            }
            return Ok(entry);
        }

        // Disambiguate: is the render id present under a different actor/surface?
        // If so, this is a cross-principal lookup attempt.
        let render_present = {
            let idx = self.render_index.read();
            idx.contains_key(&key.envelope_render_id)
        };
        if render_present {
            Err(EnvelopeCacheError::PrincipalMismatch {
                render_id: key.envelope_render_id.clone(),
            })
        } else {
            Err(EnvelopeCacheError::EnvelopeRequired(
                key.envelope_render_id.clone(),
            ))
        }
    }

    fn insert(&self, key: CacheKey, entry: CacheEntry) {
        let mut guard = self.inner.write();
        let mut idx = self.render_index.write();
        // Cheap bounded-eviction policy: when over capacity, drop the oldest
        // entry by `inserted_at`. The cache is keyed by RNG-generated render
        // IDs, so this is a safety valve, not a primary eviction mechanism.
        if guard.len() >= CACHE_MAX_ENTRIES {
            if let Some(oldest_key) = guard
                .iter()
                .min_by_key(|(_, v)| v.inserted_at)
                .map(|(k, _)| k.clone())
            {
                idx.remove(&oldest_key.envelope_render_id);
                guard.remove(&oldest_key);
            }
        }
        idx.insert(key.envelope_render_id.clone(), key.clone());
        guard.insert(key, entry);
    }
}

static CACHE: OnceLock<EnvelopeCache> = OnceLock::new();

fn cache() -> &'static EnvelopeCache {
    CACHE.get_or_init(EnvelopeCache::new)
}

/// Record an envelope's claim/proposal ids under
/// `(envelope_render_id, actor_principal_id, surface)`.
///
/// W2 wiring task: every ability that produces a renderable envelope
/// (`get_entity_intelligence`, `get_daily_briefing`, etc.) calls this with the
/// envelope's full claim_id + proposal_id set, the requesting actor's
/// principal id, and the rendering surface so the Tauri command can later
/// validate target bindings against the real envelope, not the target itself,
/// AND reject cross-principal lookups.
pub fn record_envelope_for_render(
    envelope_render_id: String,
    actor_principal_id: String,
    surface: String,
    ability: String,
    claim_ids: BTreeSet<String>,
    proposal_ids: BTreeSet<String>,
) {
    cache().insert(
        CacheKey {
            envelope_render_id,
            actor_principal_id,
            surface,
        },
        CacheEntry {
            ability,
            claim_ids,
            proposal_ids,
            inserted_at: Instant::now(),
        },
    );
}

/// Resolved envelope used by `validate_envelope_target`. Returned by
/// [`lookup_envelope_for_render`] when the cache hit is fresh AND
/// `(actor, surface)` match the entry that was recorded.
#[derive(Debug)]
pub struct CachedEnvelopeView {
    pub ability: String,
    pub claim_ids: BTreeSet<String>,
    pub proposal_ids: BTreeSet<String>,
}

/// Look up the cached envelope by `(render_id, actor_principal_id, surface)`.
///
/// Returns:
/// - `Ok(view)` on a fresh, principal-matched hit.
/// - `Err(EnvelopeRequired)` when no entry exists for that render id at all
///   (or it has expired).
/// - `Err(PrincipalMismatch)` when an entry exists for the render id but the
///   actor or surface does not match — cross-principal lookup attempt.
pub fn lookup_envelope_for_render(
    envelope_render_id: &str,
    actor_principal_id: &str,
    surface: &str,
) -> Result<CachedEnvelopeView, EnvelopeCacheError> {
    let key = CacheKey {
        envelope_render_id: envelope_render_id.to_string(),
        actor_principal_id: actor_principal_id.to_string(),
        surface: surface.to_string(),
    };
    cache().get(&key).map(|entry| CachedEnvelopeView {
        ability: entry.ability,
        claim_ids: entry.claim_ids,
        proposal_ids: entry.proposal_ids,
    })
}

/// Marker function — L3 cycle-2 enforcement: the cache is now keyed by
/// `(envelope_render_id, actor_principal_id, surface)`, cross-principal
/// lookups are rejected with [`EnvelopeCacheError::PrincipalMismatch`], and
/// the cycle-1 tautological single-claim fallback in
/// `submit_claim_feedback_command` has been removed in favour of
/// `BadRequest::EnvelopeRequired`. Callers MUST supply a valid render id
/// minted by a prior envelope render — W2 ability-side wiring is the
/// remaining producer task.
///
/// This function exists solely so a grep for `path_alpha_envelope_cache_v2`
/// surfaces the cycle-2 enforcement contract; it is referenced from a test
/// below.
pub fn path_alpha_envelope_cache_v2() {
    // intentional no-op — marker for grep audit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_render_id(seed: &str) -> String {
        // Tests run in parallel — render-ids must be unique per test so the
        // cache state from prior tests does not bleed in.
        format!("test-render::{seed}::{}", uuid::Uuid::new_v4())
    }

    #[test]
    fn lookup_returns_recorded_envelope_on_matching_principal() {
        let render_id = random_render_id("matching");
        let mut claim_ids = BTreeSet::new();
        claim_ids.insert("claim-a".to_string());
        claim_ids.insert("claim-b".to_string());
        record_envelope_for_render(
            render_id.clone(),
            "user:alice".to_string(),
            "tauri_entity_detail".to_string(),
            "get_entity_intelligence".to_string(),
            claim_ids.clone(),
            BTreeSet::new(),
        );
        let view = lookup_envelope_for_render(&render_id, "user:alice", "tauri_entity_detail")
            .expect("matching principal lookup succeeds");
        assert_eq!(view.ability, "get_entity_intelligence");
        assert_eq!(view.claim_ids, claim_ids);
        assert!(view.proposal_ids.is_empty());
    }

    #[test]
    fn lookup_miss_returns_envelope_required() {
        let render_id = random_render_id("miss");
        let err = lookup_envelope_for_render(&render_id, "user:alice", "tauri_entity_detail")
            .expect_err("missing entry rejects");
        assert!(matches!(err, EnvelopeCacheError::EnvelopeRequired(_)));
    }

    #[test]
    fn cross_actor_lookup_rejected_with_principal_mismatch() {
        // F4 (L3 cycle-2): record an envelope for user:alice on the entity
        // detail surface, then attempt lookup as user:bob (same surface). The
        // cache MUST reject with PrincipalMismatch, not silently miss.
        let render_id = random_render_id("cross-actor");
        record_envelope_for_render(
            render_id.clone(),
            "user:alice".to_string(),
            "tauri_entity_detail".to_string(),
            "get_entity_intelligence".to_string(),
            BTreeSet::from(["claim-x".to_string()]),
            BTreeSet::new(),
        );
        let err = lookup_envelope_for_render(&render_id, "user:bob", "tauri_entity_detail")
            .expect_err("cross-actor lookup rejected");
        assert!(matches!(
            err,
            EnvelopeCacheError::PrincipalMismatch { .. }
        ));
    }

    #[test]
    fn cross_surface_lookup_rejected_with_principal_mismatch() {
        // F4 (L3 cycle-2): record under tauri_entity_detail, attempt lookup
        // under mcp_tool. Both are valid surfaces but the binding is per-render
        // id; mismatching surface = PrincipalMismatch.
        let render_id = random_render_id("cross-surface");
        record_envelope_for_render(
            render_id.clone(),
            "user:alice".to_string(),
            "tauri_entity_detail".to_string(),
            "get_entity_intelligence".to_string(),
            BTreeSet::from(["claim-y".to_string()]),
            BTreeSet::new(),
        );
        let err = lookup_envelope_for_render(&render_id, "user:alice", "mcp_tool")
            .expect_err("cross-surface lookup rejected");
        assert!(matches!(
            err,
            EnvelopeCacheError::PrincipalMismatch { .. }
        ));
    }

    #[test]
    fn path_alpha_marker_compiles() {
        // Reference the cycle-2 enforcement marker so a grep audit surfaces it.
        path_alpha_envelope_cache_v2();
    }
}
