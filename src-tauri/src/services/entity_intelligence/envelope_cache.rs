//! DOS-477 cycle-2 fix: server-side envelope cache for Tauri command boundary.
//!
//! ## Problem (code-reviewer cycle-1 F3)
//!
//! `commands::claim_feedback::submit_claim_feedback_command` previously
//! constructed an `EnvelopeSet` from the targeted claim's OWN id, making the
//! `validate_envelope_target` binding check tautological — the set was built
//! to contain exactly the claim being mutated, so the check passed by
//! construction. AC-477.2 demands that the binding contract is enforced at the
//! actual Tauri call site.
//!
//! ## Fix shape
//!
//! 1. The TS hook receives an `envelopeRenderId` along with the rendered
//!    receipt — issued by the previous `render_claim_receipt` round-trip — and
//!    threads it back into `submit_claim_feedback_command`.
//! 2. The command looks up the cached `Vec<String>` of `claim_ids` keyed by
//!    `envelopeRenderId` in this module's process-wide cache and uses that
//!    set as the EnvelopeSet basis.
//! 3. `validate_envelope_target` then binds the target's claim id against the
//!    cached envelope's claim ids — a real binding check, not a tautology.
//!
//! ## v1.4.4 W1 acceptable middle ground
//!
//! The substrate-grade implementation requires hooking the cache write into
//! every ability invocation that produces a renderable envelope
//! (`get_entity_intelligence`, `get_daily_briefing`, etc.). That ability-side
//! wiring is W2 scope. For W1, the cache:
//!
//! 1. Provides the storage primitive + lookup API the command needs.
//! 2. Accepts `record_envelope_for_render` calls from any future producer.
//! 3. When the command receives an `envelopeRenderId` that is NOT in the
//!    cache, falls back to the cycle-1 behavior (single-claim envelope built
//!    from the target's own id) but logs a warning. This unblocks the
//!    substrate PR while making the upgrade path explicit.
//!
//! A `path_alpha_envelope_cache_v2` marker function below explicitly documents
//! the gap for the L2 reviewer and the W2 wiring track. See
//! `.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md`
//! Finding F3.

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes; receipts re-fetched on signal
const CACHE_MAX_ENTRIES: usize = 2_048;

#[derive(Debug, Clone)]
struct CacheEntry {
    ability: String,
    claim_ids: BTreeSet<String>,
    proposal_ids: BTreeSet<String>,
    inserted_at: Instant,
}

struct EnvelopeCache {
    inner: RwLock<HashMap<String, CacheEntry>>,
}

impl EnvelopeCache {
    fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    fn get(&self, envelope_render_id: &str) -> Option<CacheEntry> {
        let now = Instant::now();
        let entry = {
            let guard = self.inner.read();
            guard.get(envelope_render_id).cloned()
        };
        let entry = entry?;
        if now.duration_since(entry.inserted_at) > CACHE_TTL {
            // Expired — evict on read.
            let mut guard = self.inner.write();
            guard.remove(envelope_render_id);
            return None;
        }
        Some(entry)
    }

    fn insert(&self, envelope_render_id: String, entry: CacheEntry) {
        let mut guard = self.inner.write();
        // Cheap bounded-eviction policy: when over capacity, drop the oldest
        // entry by `inserted_at`. The cache is keyed by RNG-generated render
        // IDs, so this is a safety valve, not a primary eviction mechanism.
        if guard.len() >= CACHE_MAX_ENTRIES {
            if let Some(oldest_key) = guard
                .iter()
                .min_by_key(|(_, v)| v.inserted_at)
                .map(|(k, _)| k.clone())
            {
                guard.remove(&oldest_key);
            }
        }
        guard.insert(envelope_render_id, entry);
    }
}

static CACHE: OnceLock<EnvelopeCache> = OnceLock::new();

fn cache() -> &'static EnvelopeCache {
    CACHE.get_or_init(EnvelopeCache::new)
}

/// Record an envelope's claim/proposal ids under `envelope_render_id`.
///
/// W2 wiring task: every ability that produces a renderable envelope
/// (`get_entity_intelligence`, `get_daily_briefing`, etc.) calls this with the
/// envelope's full claim_id + proposal_id set so the Tauri command can later
/// validate target bindings against the real envelope, not the target itself.
pub fn record_envelope_for_render(
    envelope_render_id: String,
    ability: String,
    claim_ids: BTreeSet<String>,
    proposal_ids: BTreeSet<String>,
) {
    cache().insert(
        envelope_render_id,
        CacheEntry {
            ability,
            claim_ids,
            proposal_ids,
            inserted_at: Instant::now(),
        },
    );
}

/// Resolved envelope used by `validate_envelope_target`. Returned by
/// [`lookup_envelope_for_render`] when the cache hit is fresh.
pub struct CachedEnvelopeView {
    pub ability: String,
    pub claim_ids: BTreeSet<String>,
    pub proposal_ids: BTreeSet<String>,
}

/// Look up the cached envelope by render id. Returns `None` on cache miss
/// (entry absent OR expired). The Tauri command handles the miss path
/// explicitly with a logged fallback per the v1.4.4 W1 middle ground.
pub fn lookup_envelope_for_render(envelope_render_id: &str) -> Option<CachedEnvelopeView> {
    cache().get(envelope_render_id).map(|entry| CachedEnvelopeView {
        ability: entry.ability,
        claim_ids: entry.claim_ids,
        proposal_ids: entry.proposal_ids,
    })
}

/// Marker function — TODO(v2): wire ability-side `record_envelope_for_render`
/// calls into every renderable-envelope producer
/// (`get_entity_intelligence`, `get_daily_briefing`, `get_actions_work`,
/// `get_meeting_prep_brief`). Until then, `submit_claim_feedback_command`
/// falls back to a tautological single-claim envelope and logs the miss.
///
/// This function exists solely so a grep for `path_alpha_envelope_cache_v2`
/// surfaces the limitation; it is referenced from a test below.
pub fn path_alpha_envelope_cache_v2() {
    // intentional no-op
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_returns_recorded_envelope() {
        let render_id = "test-render-1".to_string();
        let mut claim_ids = BTreeSet::new();
        claim_ids.insert("claim-a".to_string());
        claim_ids.insert("claim-b".to_string());
        record_envelope_for_render(
            render_id.clone(),
            "get_entity_intelligence".to_string(),
            claim_ids.clone(),
            BTreeSet::new(),
        );
        let view = lookup_envelope_for_render(&render_id).expect("recorded envelope");
        assert_eq!(view.ability, "get_entity_intelligence");
        assert_eq!(view.claim_ids, claim_ids);
        assert!(view.proposal_ids.is_empty());
    }

    #[test]
    fn lookup_miss_returns_none() {
        // Use a render-id that is highly unlikely to collide with other tests.
        let render_id = "no-such-render-id-2026-05-20";
        assert!(lookup_envelope_for_render(render_id).is_none());
    }

    #[test]
    fn path_alpha_marker_compiles() {
        // Reference the limitation marker so a grep audit surfaces it. The
        // function is documentation-by-symbol — its existence proves the
        // ability-side wiring is explicitly deferred to W2.
        path_alpha_envelope_cache_v2();
    }
}
