# ADR-0131 — Structured + Embedding Claim Canonicalization

**Status:** Accepted
**Date:** 2026-05-10
**Authors:** James Giroux, Claude
**Relates to:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0113](0113-claim-anatomy-and-dedup.md), [ADR-0114](0114-scoring-unification.md), [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)
**Supersedes (partial):** Lexical/heuristic semantic canonicalization in W4-B (DOS-280) v1.4.1 implementation
**Linear:** DOS-280 W4-B — implementing inline as v1.4.1 convergence move, not deferred
**Label:** `architectural-swap` — interim L3-codex-challenge step required between L2 APPROVE and merge

## Context

DOS-280 (W4-B of v1.4.1) shipped semantic canonicalization of paraphrased claims using a manually-implemented matcher:

- Custom tokenizer with stopwords, negators, status-bearing terms
- Bag-of-terms similarity via Jaccard + coverage thresholds
- Domain-curated synonym lists (`budget`/`funding`, `signing`/`signature`)
- Regex-based qualifier extraction (regions, quarters, named entities)
- Status compatibility check (Confirmed/Pending/Unknown)
- Tombstone/sensitivity/temporal-scope gates around the similarity primitive

L2 adversarial review hit the 15-cycle escalation threshold per [feedback_review_loop_l6_policy.md](../../.claude/projects/-Users-jamesgiroux-Documents-dailyos-repo/memory/feedback_review_loop_l6_policy.md), and the bug pattern was diagnostic:

| Bug class | Cycles |
|---|---|
| Tokenization edge cases (`U.S.`, `hasn't`, capitalization) | c3, c8, c9, c10, c11, c12, c13 |
| Qualifier extraction coverage (regions, quarters, named entities, one-sided numerics) | c2, c8, c14, c15 |
| Legacy migration recovery (provenance scanning, partial states) | c4, c14, c15 |
| Tombstone/dormant interaction across lookups | c5, c6, c7, c9, c10, c11 |
| Threshold tuning (over/under-collapse) | c2, c13 |

The cycle pattern reveals the architecture is fragile: token-bag + Jaccard + curated synonym lists has unbounded surface area. Every fix introduces a new edge case for the next cycle to find. The tier-gating, tombstone, and dormant-filter work (cycles 4-11) is real and worth keeping; the similarity primitive is the part that won't converge cleanly under iterative patching.

DailyOS already ships `nomic-embed-text-v1.5` (quantized) via fastembed in `src-tauri/src/embeddings.rs` with `embed()` and `embed_batch()` exposed plus asymmetric retrieval prefixes (`QUERY_PREFIX` / `DOCUMENT_PREFIX`). The model is plug-and-play for canonicalization.

[ADR-0114](0114-scoring-unification.md) §scoring-unification establishes the structural-extractor pattern at the trust scoring boundary: "extractor reads the database, injects the clock, produces typed factor inputs." This ADR extends that pattern to the canonicalization decision.

## Decision

Replace the bag-of-terms semantic canonicalization primitive with **structured field extraction + embedding-driven field comparison**, landed as an **additive shadow-mode layer** with a parity-gated cutover (no claim_id rewrite, no rip-out).

### 1. Claim extraction emits typed fields

The W2-G `CLAIM_TYPE_REGISTRY` extension mechanism already enforces typed claim shapes (CommitmentClaim, etc.). This ADR generalizes that to require every claim to carry **canonical structural primitives** alongside its free-text `text`:

```rust
pub struct StructuredClaim {
    pub subject_ref: SubjectRef,        // entity-typed (already exists)
    pub predicate: PredicateRef,        // canonical relation id (closed namespaced registry — §11)
    pub polarity: Polarity,             // Affirm | Negate — literal-equality only, never embedding-compared (§2)
    pub object: ObjectValue,            // typed value or free-text fallback
    pub qualifiers: QualifierSet,       // typed structural scope
    pub status: ClaimStatus,            // Confirmed | Pending | Unknown — epistemic state, separate from polarity
    pub sentiment: Option<Sentiment>,   // informational, salience = 0 for canonicalization
}

pub enum Polarity {
    Affirm,
    Negate,
}

pub enum ObjectValue {
    Resolved { entity_ref: EntityRef },              // shared with subject_ref entity-type infra (§12)
    Literal { kind: LiteralKind, value: String },    // number, text, date, money, percentage, enum
    FreeText { canonical: String },                  // fallback; embedding-compared
}

pub struct QualifierSet {
    pub time: Option<TemporalQualifier>,             // Q3, 2026, 30d, etc.
    pub region: Option<RegionCode>,                  // US/EU/APAC/EMEA/null
    pub scope: Option<ScopeMarker>,                  // Phase 2, vN, etc.
    pub entity: Option<EntityRef>,                   // named entity reference
    pub numerics: Vec<NumericQualifier>,             // one-sided value ranges (c2/c8 coverage)
    pub extras: BTreeMap<QualifierKey, QualifierValue>, // closed-enum escape hatch — additive without free-text
}
```

Extractor abilities populate these fields during claim ingestion. Where the LLM cannot extract a structured value confidently, the field stays `None` and the claim is marked `non_semantic_mergeable: true` (fail-closed — see §7 Migration).

**Polarity is first-class and separate from `ClaimStatus`** because polarity is *predicate negation* ("is" vs "is not") while `ClaimStatus` is *epistemic state* (do we believe it). Conflating them was the root cause of the c3/c11/c12 negation-merge bugs. Polarity uses literal equality only; the embedding comparator never sees polarity.

### 2. Canonicalization decisions operate on fields, not free text

For each comparable field, the comparator returns:

- **Match** — literal equality (for typed fields) OR embedding cosine ≥ `HIGH_THRESHOLD` (default 0.85) for free-text/predicate-alias surfaces
- **Distinct** — literal differs (for typed fields) OR embedding cosine < `LOW_THRESHOLD` (default 0.60)
- **Ambiguous** — embedding cosine in [`LOW_THRESHOLD`, `HIGH_THRESHOLD`]

**Field comparison rules:**

| Field | Comparison mode | None/Some asymmetry |
|---|---|---|
| `subject_ref` | Typed equality on entity_ref (no embedding) | None = Distinct |
| `predicate` | Registry-id equality + alias lookup (§11) | None = Distinct |
| `polarity` | Literal equality only | None impossible — required field |
| `qualifiers.time` | Typed equality on normalized form | `Some(x) ⟷ None` = **Distinct** (fail-closed) |
| `qualifiers.region` | Typed equality | `Some(x) ⟷ None` = **Distinct** |
| `qualifiers.scope` | Typed equality | `Some(x) ⟷ None` = **Distinct** |
| `qualifiers.entity` | Typed equality on entity_ref | `Some(x) ⟷ None` = **Distinct** |
| `qualifiers.numerics` | Set equality on normalized ranges | Disjoint sets = Distinct |
| `qualifiers.extras` | Set equality on QualifierKey/Value pairs | Disjoint = Distinct |
| `object` (Resolved) | Typed equality on entity_ref | None = Distinct |
| `object` (Literal) | Typed equality on normalized form | None = Distinct |
| `object` (FreeText) | Embedding comparator (HIGH/LOW thresholds) | None = Distinct |
| `status` | Status-lattice rule (§10 carryover) | None = Distinct |

The `Some(x) ⟷ None` = Distinct rule was the root cause of cycles c2/c8 (unscoped vs scoped budget collapse). It is now codified, not derived.

**Decision rule (composite):**

```
canonical_match(a, b):
    1. CandidateFilter (§5) — if a or b fails tier compat, tombstone shadow, dormant filter, or account scope → Distinct (no comparator runs)
    2. for each field f in [subject_ref, predicate, polarity, qualifiers, object, status]:
         compute f.compare(a, b)
    3. decision:
         if any field == Distinct  → fork (insert as new claim)
         if any field == Ambiguous → fail-closed fork + persist as ambiguous-pair (§8 lifecycle)
         if all fields == Match    → merge as corroboration
```

There is **no** "Ambiguous_with_user_confirmation auto-merge" path. Ambiguous always forks (with the lifecycle hook in §8).

[ADR-0113](0113-claim-anatomy-and-dedup.md)'s contradiction semantics are preserved: claims that match on subject/predicate/qualifiers/polarity but distinct on object are **contradictions**, not canonicalization candidates — routed to contradiction-edge insertion, not fork-or-merge.

### 3. Embedding usage, cache, and transaction safety

`EmbeddingModel::embed_batch` is the primitive. Both texts being compared receive `DOCUMENT_PREFIX` (symmetric similarity, not asymmetric retrieval).

**Cache layer:**
- Key: `(model_version, normalized_text, prefix)` — model_version is required to prevent stale vectors after model upgrade
- Value: `Vec<f32>` plus computation timestamp
- Eviction: strict byte-accounted LRU
- Bounds: 16,384 entries OR 96 MB byte cap, whichever hits first (configurable; 32,768 / 160 MB upper cap for heavier workloads)
- Invalidation: claim text/structured-field mutation invalidates affected normalized strings; model upgrade invalidates entire cache
- **Not persisted in W4-B.** Audit-side durability lives in `canonicalization_decisions` (§6), not the cache. A persistent `claim_embeddings` table is a future v1.4.2+ optimization for replay scenarios, gated separately.

**Transaction safety:** Embedding inference MUST NOT run inside a SQLite write transaction. The canonicalize pipeline:
1. Open read tx → gather candidate rows via **safe-superset SQL enumeration** (account/workspace/tier indexed prefilter ONLY; tombstone/migration-status/non_semantic_mergeable rejection happens in-comparator via CandidateFilter, not in SQL — see §5)
2. Close read tx
3. Compute embeddings + cache hits/misses (outside any tx)
4. Open write tx → re-check candidate row state (handles concurrent writes) → apply canonical_match decision → persist `canonicalization_decisions` audit row → commit

Logs emit aggregate counters only: `cache_hits_total`, `cache_misses_total`, `cache_evictions_total`. Never log `normalized_text` or text prefixes.

**Fallback path:** When the embedding model is in `HashFallback` mode (offline/dev/init-failed):
- Comparator degrades to **strict literal-equality only** for free-text/predicate-alias fields. No cosine comparison. No `Ambiguous` band.
- `EmbeddingModel::status()` reports `HashFallback` distinctly from `Ready`. The status is exposed as a runtime health signal via `build_intelligence_context()` so trust-band rendering can downgrade affected claims to `use_with_caution`.
- **In Live mode**, semantic auto-merge is **disabled** when `HashFallback` is active — fail-closed fork only. The migration cutover gate (§7) refuses to advance into "structural-comparator authoritative" unless the real model is available.

### 4. Salience and confidence weighting

Each field carries a salience weight. The decision rule weights matches by field salience:

- **Subject + predicate + polarity + qualifiers + status** are load-bearing — must Match for any canonicalization (salience = 1.0, gating)
- **Object** is comparison-soft — Match required (Match by literal or embedding); Ambiguous still forks per §2
- **Sentiment** is informational, not gating (salience = 0.0 for canonicalization purposes)

**Claim-type-specific salience overrides** are pinned in the `CLAIM_TYPE_REGISTRY` per ADR-0125. Examples (initial):
- `CommitmentClaim` weights `owner` (= subject_ref) and `due` (= qualifiers.time) more heavily than `text`
- `TopicClaim` weights `predicate` registry equality more heavily; tolerates `qualifiers.region == None ⟷ None` matches
- `RiskClaim` requires `polarity == Affirm` (negated risks are explicitly forked into a "risk-cleared" claim path)

Salience defaults are tunable only through the calibration fixture (§9), never through bare code constants.

### 5. Pre-comparator CandidateFilter — defense-in-depth ordering

The W4-B work on **tier compatibility** (sensitivity, temporal_scope), **tombstone shadow logic**, **dormant/surfacing-state filtering**, and **account/workspace scoping** all transfer to the new primitive — but they MUST run BEFORE `canonical_match` as a centralized `CandidateFilter`.

```rust
fn candidate_filter(query: &StructuredClaim, candidate: &StructuredClaim, ctx: &Ctx)
  -> FilterDecision /* Pass | RejectAsDistinct(primary: Reason, secondary: Vec<Reason>) */
{
    // Strict reason-precedence ordering — the FIRST matching gate is the primary audited reason.
    // Additional gates that would also have fired are collected as secondary reasons for audit completeness.
    let mut secondary = Vec::new();
    let mut primary: Option<Reason> = None;
    let mut record = |reason: Reason| {
        if primary.is_none() { primary = Some(reason); } else { secondary.push(reason); }
    };

    // (1) Migration-status gates fire FIRST — preserves the fail-closed promise that
    // pending_backfill / legacy_unmigrated rows are explicitly audited even when other gates would also fire.
    if query.canonical_status == PendingBackfill        { record(QueryPendingBackfill); }
    if candidate.canonical_status == PendingBackfill    { record(CandidatePendingBackfill); }
    if query.canonical_status == LegacyUnmigrated || query.non_semantic_mergeable     { record(QueryLegacyUnmigrated); }
    if candidate.canonical_status == LegacyUnmigrated || candidate.non_semantic_mergeable { record(CandidateLegacyUnmigrated); }

    // (2) Tombstone gates — both query AND candidate sides
    if query_matches_tombstone_shadow(query, ctx)       { record(QueryTombstoned); }
    if candidate_matches_tombstone_shadow(candidate, ctx) { record(CandidateTombstoned); }

    // (3) Scope + tier gates
    if !same_account(query, candidate, ctx)             { record(AccountScope); }
    if !same_workspace(query, candidate, ctx)           { record(WorkspaceScope); }
    if !tier_compatible(query, candidate)               { record(TierMismatch); }
    if dormant_or_surfaced(candidate, ctx)              { record(DormantOrSurfaced); }

    match primary {
        Some(reason) => RejectAsDistinct { primary: reason, secondary },
        None => Pass,
    }
}
```

**Audit-reason precedence rationale:** Migration-status gates (pending_backfill, legacy_unmigrated) fire FIRST so that rows in those states are always audited with their migration-status reason as the primary, even when they would ALSO be rejected by tombstone, tier, or scope gates. This preserves the §7 fail-closed contract that pending_backfill rows are visible in audit as such — critical for Phase B parity tracking and Phase C pending_backfill drain verification (§9). Secondary reasons are persisted in `canonicalization_decisions.reason_secondary` JSONB so the full rejection profile is recoverable.

**SQL-side enumeration vs authoritative audit gate:** SQL-side candidate enumeration filters by **account/workspace + tier ONLY** — these are indexed performance prefilters that DO NOT need to produce audit reasons. **SQL-side enumeration MUST NOT filter on tombstone, migration-status (`pending_backfill`/`legacy_unmigrated`), or `non_semantic_mergeable`**. Those gates produce audit reasons that downstream telemetry (pending_backfill drain rate, tombstone-bypass adversarial fixtures) depends on observing. The in-comparator CandidateFilter pseudocode above is the **first authoritative rejection point** and the audit source of truth — it sees a safe superset of candidate rows (account/workspace/tier-eligible only) and rejects with primary+secondary reasons. Suite S fixture asserts the actual SQL enumeration path returns tombstoned + pending_backfill candidates so CandidateFilter can audit them as `CandidatePendingBackfill` (primary) with `CandidateTombstoned` in secondary.

Only `Pass` candidates reach `canonical_match`. This eliminates the entire class of "tombstone bypass via comparator path," "cross-tier merge via embedding similarity," and "legacy-unmigrated silent fork" findings. Suite S regression covers each filter assertion individually plus combinatorial coverage.

**Tombstone shadow — both query AND candidate sides:** The original cycle-1..15 correction-resurrection bug class included paraphrased re-ingestion of tombstoned claims, where the NEW incoming claim (the query) matches a prior tombstone. The CandidateFilter pseudocode explicitly evaluates both `query_matches_tombstone_shadow` and `candidate_matches_tombstone_shadow`. Suite S includes a fixture where a paraphrased new query is blocked even when no live candidate is tombstoned.

**Structural tombstone-shadow contract:** Today's tombstone gate is text/hash-keyed (`item_hash`, exact canonical text, keyless text tiers). The shadow-mode rollout (§7) backfills structural-identity tombstone keys onto existing tombstones. Until the shadow-mode parity gate clears, **both** legacy text/hash tombstone gates AND new structural-identity tombstone gates run in parallel (belt + suspenders) for both query and candidate sides. Helper deletion (§10) is gated on shadow-mode parity proving the structural gate has zero false negatives vs the legacy gate.

**`non_semantic_mergeable` / `legacy_unmigrated` are pre-comparator gates:** Rows marked `non_semantic_mergeable = true` or `canonical_status = legacy_unmigrated` are excluded from `canonical_match_v2` evaluation entirely. They continue through the legacy comparator path (Phase A/B) or remain as standalone claims (Phase C). The audited reason is `QueryLegacyUnmigrated` or `CandidateLegacyUnmigrated`. Suite S fixture proves no merge/fork is applied to these rows under v2.

### 6. Auditability

Every canonicalization decision writes a per-field score record. The same table holds both shadow (Phase A/B) and live (Phase C) decisions, distinguished by a `mode` column — no separate `_shadow` table, so `ambiguous_claim_pairs.decision_id` references a single canonical FK target:

```sql
CREATE TABLE canonicalization_decisions (
  decision_id           TEXT PRIMARY KEY,
  claim_id_a            TEXT NOT NULL,
  claim_id_b            TEXT NOT NULL,
  decision              TEXT NOT NULL,  -- 'merge' | 'fork' | 'fork_ambiguous' | 'fork_contradiction' | 'fork_filtered'
  mode                  TEXT NOT NULL,  -- 'shadow' (Phase A/B; observe-only, NEVER mutates canonical state) | 'live' (Phase C; authoritative)
  is_authoritative      BOOLEAN NOT NULL GENERATED ALWAYS AS (mode = 'live') STORED,
  field_scores          JSONB NOT NULL, -- { subject: 0.97, predicate: 'match', object: 0.93, qualifiers: 'match', polarity: 'match', status: 'match' }
  reason                TEXT NOT NULL,  -- primary reason: 'all_match' | 'subject_distinct' | 'qualifier_mismatch:region' | 'ambiguous:object' | 'query_pending_backfill' | 'candidate_pending_backfill' | 'query_tombstoned' | 'candidate_tombstoned' | 'query_legacy_unmigrated' | 'candidate_legacy_unmigrated' | 'tier_mismatch' | 'account_scope' | 'workspace_scope' | 'dormant_or_surfaced'
  reason_secondary      JSONB,          -- array of additional gate reasons that would have fired; primary reason is the precedence-first match per CandidateFilter ordering
  threshold_band        TEXT,           -- 'high' | 'ambiguous' | 'low' (when embedding used)
  embedding_model_version TEXT,         -- e.g., 'nomic-embed-text-v1.5-Q'
  comparator_threshold_version TEXT,    -- pinned constants block version
  field_provenance      JSONB NOT NULL, -- per-field source attribution preserved across merge — see conflict-resolution rule below
  canonicalization_mode TEXT NOT NULL,  -- 'full' | 'hash_fallback' (when fallback active)
  supersedes_decision_id TEXT REFERENCES canonicalization_decisions(decision_id),  -- replay chain: this decision supersedes a prior decision for the same claim pair (e.g., model upgrade re-canonicalization)
  idempotency_key       TEXT NOT NULL UNIQUE,  -- deterministic: hash(canonical_pair_key, mode, embedding_model_version, comparator_threshold_version, claim_a_revision_hash, claim_b_revision_hash) where canonical_pair_key = hash(min(claim_id_a, claim_id_b), max(claim_id_a, claim_id_b)) — pair-order-invariant AND revision-sensitive so (A,B) and (B,A) produce same key under identical inputs, but structural changes to either claim produce a fresh key
  claim_a_revision_hash TEXT NOT NULL,  -- hash(canonical_status, structural_field_content_hash, backfill_epoch) — changes when claim_a's structural fields or canonical_status mutate
  claim_b_revision_hash TEXT NOT NULL,  -- same as claim_a_revision_hash for claim_b
  evaluated_at          TIMESTAMP NOT NULL
);
```

**Idempotency contract:** The `idempotency_key UNIQUE` constraint uses a **pair-order-invariant AND revision-sensitive** canonical key. Revision hashes are aligned to sorted claim identity BEFORE hashing — never in call order:

```
let low  = min(claim_id_a, claim_id_b);
let high = max(claim_id_a, claim_id_b);
let rev_low  = claim_revision_hash(load_claim(low));   // aligned to LOW claim, not call order
let rev_high = claim_revision_hash(load_claim(high));  // aligned to HIGH claim
let claim_revision_hash(c) = hash(c.canonical_status, c.structural_field_content_hash, c.backfill_epoch);
let idempotency_key = hash(
  low, rev_low,
  high, rev_high,
  mode,
  embedding_model_version,
  comparator_threshold_version
);
```

This guarantees `canonicalize(A, B)` and `canonicalize(B, A)` produce the **exact same** idempotency_key under any inputs, because sorting happens before revision-hash lookup. Implementations MUST sort first, then load revision data, then hash — never the reverse. A CI invariant test asserts `compute_idempotency_key(claim_a, claim_b)` and `compute_idempotency_key(claim_b, claim_a)` return identical strings for any claim pair.

`structural_field_content_hash` is computed over the typed canonicalization inputs (`predicate_ref`, `polarity`, `object_value`, `qualifiers`, `status`) and persisted as a column on `intelligence_claims` for stable read access. **`status` is included** because the §2 comparator decision rule compares `status` between claims (status-lattice rule from §10) — a Pending → Confirmed transition is a canonicalization input change and MUST trigger re-evaluation. `sentiment` is excluded (salience 0 per §4, not gating). `subject_ref` is implicit in `claim_id` and already in `canonical_pair_key`. `backfill_epoch` is a monotonically increasing counter persisted as a column on `intelligence_claims`, default 0, incremented in the same transaction as each structural backfill OR each `ClaimStatus` mutation (see §7 Phase A migration v160).

Suite S fixture asserts: a `pending` claim canonicalized to a fork decision, then transitioned to `confirmed`, produces a NEW idempotency_key on next canonicalize call (new revision hash) and writes a `supersedes_decision_id` chain link — proving status-only changes are revision-sensitive.

This guarantees:
1. **Pair-order invariance** — (A, B) and (B, A) produce the same key under identical inputs (the second call is a no-op INSERT catching `unique_violation`).
2. **Revision sensitivity** — when a claim's structural fields mutate (re-extraction succeeds) OR canonical_status transitions (`pending_backfill` → `live` or `live` → re-extracted), `claim_revision_hash` changes → `idempotency_key` changes → fresh decision evaluation is allowed and writes a new row with `supersedes_decision_id` pointing at the prior decision.
3. **Model/threshold sensitivity** — model upgrade or threshold version bump produces a fresh key for the same pair (existing behavior).

Live state-mutation operations always consume the **latest non-superseded** row in the chain (a row is "latest" if no other row references it via `supersedes_decision_id`).

**Suite S fixtures cover:**
- Replay invariance: invoking canonicalize on (A, B) then (B, A) under identical model+threshold+revision produces ONE decision row.
- Revision re-evaluation: a pair first audited as `CandidatePendingBackfill` is re-evaluated after the claim transitions to `live` (revision_hash changes), producing a NEW decision row that supersedes the prior `pending_backfill` rejection — proving stale decisions cannot freeze after backfill catches up.
- Model upgrade: replay under new `embedding_model_version` produces a fresh chain link with `supersedes_decision_id` set, regardless of (A, B) or (B, A) order.

**Signals tie-in (§13):** `structural_backfill_changed { claim_id, field_set }` is the operational trigger for revision-hash bumps. Implementations MUST increment `backfill_epoch` on each `structural_backfill_changed` emission so the next canonicalize call sees a new `claim_revision_hash`.

On embedding-model upgrade, a one-shot re-canonicalization migration runs against decisions where `embedding_model_version` is stale **AND** `mode = 'live'`. Shadow-mode decisions are never replayed for live recanonicalization — they remain as historical observations. Replayed live decisions write new rows with a `supersedes_decision_id` link to the prior decision; the replay idempotency_key uses the full revision-sensitive contract (`hash(canonical_pair_key, mode, embedding_model_version, comparator_threshold_version, claim_a_revision_hash, claim_b_revision_hash)`). Suite S replay fixture covers model-upgrade replay with both (A, B) and (B, A) input order, asserting a single canonical supersedes chain. Trust-band rendering can detect "this canonical_id predates current model" via the model_version field and downgrade to `use_with_caution` until re-validated.

**Mode-aware replay contract:** Any operation that consumes `canonicalization_decisions` for the purpose of mutating claim state (canonical_id, dedup_key, contradiction edges, merged provenance) MUST filter on `mode = 'live'` (or equivalently `is_authoritative = true`). Shadow decisions are observational only and have no authority to mutate downstream state under any code path, including replay, ambiguous-pair lifecycle, and re-canonicalization migrations.

**Per-field provenance preservation on Merge.** `field_provenance` is a JSONB array of `{ field, source_id, source_asof, trust_score }` records, one entry per contributing source. **Conflict-resolution rule** when both source claims contribute a value to the same field at merge time:
1. Higher-trust source wins (`trust_score` per W3-B factor library)
2. On trust tie, newer `source_asof` wins
3. On both tie, the first claim by `claim_id` lexicographic order wins (deterministic)
4. The full attribution list is preserved in the JSONB array regardless of which wins — the authoritative value is at index 0, alternatives follow

`ClaimEnvelope::field_attribution()` exposes the full ordered list so field-attribution callouts (per ADR-0083) can render both the authoritative source AND alternatives ("primary source: X; also reported by Y").

### 7. Migration — additive, dual-read, shadow-mode parity, no claim_id rewrite

The migration strategy is **strictly additive**. Existing `claim_id`, `dedup_key`, `item_hash`, and contradiction/corroboration edges are **not rewritten**. New structural canonicalization runs **alongside** the legacy path until shadow-mode parity proves equivalence.

**Phase A — additive backfill (no canonicalization state change, fail-closed defaults):**
- Forward migration v160 adds these columns to `intelligence_claims`:
  - `predicate_ref`, `polarity`, `object_value`, `qualifiers`, `structural_canonical_id` — nullable (populated by backfill)
  - `canonical_status` enum (`'pending_backfill' | 'legacy_unmigrated' | 'live'`) — **default `pending_backfill`** for all existing rows; new claims start `pending_backfill` until extractor populates structural fields
  - `non_semantic_mergeable` bool — **default `true`** for all existing rows; cleared to `false` only when backfill/extraction populates the structural fields successfully AND `canonical_status` transitions to `live`
  - `structural_field_content_hash` TEXT — persisted hash over `predicate_ref`, `polarity`, `object_value`, `qualifiers`, AND `status` (epistemic ClaimStatus is a comparator input per §2 and MUST be in the revision hash); recomputed and stored in the same transaction as any of those fields mutating; participates in the §6 idempotency key
  - `backfill_epoch` INTEGER NOT NULL DEFAULT 0 — monotonically increasing counter incremented (in the same transaction) on each `structural_backfill_changed` emission AND each `status` mutation for this claim; participates in the §6 idempotency key; migration test asserts the epoch persists across restart and increments transactionally
- The `canonicalization_decisions` audit table (§6) with `mode = 'shadow' | 'live'` column and the `ambiguous_claim_pairs` table (§8) — single source of truth, no `_shadow` table.
- Backfill via re-extraction transitions rows: `pending_backfill` → `live` (extraction succeeded, structural fields populated, `non_semantic_mergeable = false`) OR `pending_backfill` → `legacy_unmigrated` (extraction failed permanently, stays `non_semantic_mergeable = true`).
- CandidateFilter (§5) accepts ONLY rows with `canonical_status = 'live'` and `non_semantic_mergeable = false`. All other rows (pending_backfill, legacy_unmigrated, or any partial structural-field state) are rejected with audited reason `QueryPendingBackfill`/`CandidatePendingBackfill` or `QueryLegacyUnmigrated`/`CandidateLegacyUnmigrated`. This is **fail-closed by default** — v2 cannot evaluate a row until backfill explicitly transitions it to live.
- Trust-band rendering on affected rows downgrades to `use_with_caution` for both `pending_backfill` AND `legacy_unmigrated` states; `trust_band_downgraded` signal fires per §13 with `reason = 'pending_backfill'` or `reason = 'legacy_unmigrated'`. When `pending_backfill` transitions to `live` (extraction succeeded), `trust_band_cleared` fires and the claim's normal trust band rendering resumes. This IS a behavior change for affected rows (the prior wording "no behavior change" was overloaded). Canonicalization state (canonical_id, dedup_key, item_hash, contradiction/corroboration edges) does NOT change in Phase A.
- Existing `canonical_id` derivation untouched. Existing canonicalize path still uses the legacy lexical comparator.
- New `canonical_match_v2` runs in **shadow mode**: invoked on every commit_claim, decisions written to `canonicalization_decisions` with `mode = 'shadow'`, but **never altering claim state**.

**Phase B — shadow parity gate:**
- Suite E parity report compares every shadow-mode v2 decision against the legacy v1 decision over a labeled corpus + production-drained fixture set.
- Gate metrics — pinned in §9 calibration fixture (ALL must hold):
  - **True-merge precision** ≥ 0.98 — of pairs v2 decides `merge`, ≥98% are labeled `should_merge`
  - **True-merge recall** ≥ 0.95 — of pairs labeled `should_merge`, v2 actually decides `merge` for ≥95% (prevents over-forking gaming the precision metric)
  - **True-fork recall** ≥ 0.95 — of pairs labeled `should_fork`, v2 forks ≥95%
  - **Contradiction detection** ≥ 0.97 — of pairs labeled `should_contradict`, v2 routes to contradiction edge for ≥97%
  - **False-merge ceiling** ≤ 0.5% (false-merge double-weighted)
  - **Ambiguous-rate cap** ≤ 5% per label bucket — v2 can't park decisions in Ambiguous to dodge precision/recall targets
  - **Tombstone-bypass rate** = 0 (Suite S adversarial: paraphrased re-resurrection on both query AND candidate sides)
  - **Cross-tier merge rate** = 0 (Suite S)
  - **Cross-account merge rate** = 0 (Suite S)
  - **Cross-workspace merge rate** = 0 (Suite S — workspace scope is a distinct CandidateFilter gate from account)
  - **Legacy-unmigrated merge rate** = 0 (Suite S — `non_semantic_mergeable` / `canonical_status = legacy_unmigrated` rows never enter v2)
  - **`pending_backfill_count` = 0** at Phase C cutover gate — hard precondition. No claim may remain in `pending_backfill` state when v2 becomes authoritative. Backfill must transition every row to either `live` (extraction succeeded) or `legacy_unmigrated` (extraction failed after bounded retry).
  - **`pending_backfill` age cap** during Phase A/B: any row stuck in `pending_backfill` longer than `PENDING_BACKFILL_MAX_AGE` (default: 24 hours) AND past `PENDING_BACKFILL_MAX_RETRIES` (default: 3) is **terminalized** to `legacy_unmigrated` with audit reason `PendingBackfillTerminalized`. This prevents indefinite stranding by a stuck extractor.
  - **`pending_backfill_rejection_rate`** ≤ 1% of canonicalize calls during Phase A/B (CandidateFilter rejection rate where primary reason is `Query/CandidatePendingBackfill`). Above this rate, backfill is materially behind and Phase B is not ready to gate Phase C.
- Divergences from v1 are logged with structural-extraction context for post-hoc review, bucketed by reason (`v1_merge_v2_fork`, `v1_fork_v2_merge`, `v1_fork_v2_ambiguous`, etc.) with allowed-divergence categories pinned in the parity report.
- Cutover requires unanimous parity gate clear + L0-style codex challenge against the parity report (interim L3 gate per the `architectural-swap` ADR label).

**Phase C — cutover (gated on Phase B clear):**
- `commit_claim` switches from `semantic_signatures_near_duplicate` → `canonical_match_v2` as the authoritative comparator.
- Legacy text/hash tombstone gates remain in CandidateFilter (§5) — they are belt + suspenders, not replaced.
- Legacy lexical helper module deleted in the cutover commit; CI grep guard added against re-introduction by name.
- Existing `canonical_id` values stay on existing rows. New canonicalization decisions write `structural_canonical_id` and update `canonical_id` only on merge events going forward.
- Migration v161 records the cutover and the model_version + threshold_version pin.

**Failure mode on backfill:** If structural extraction fails for a row mid-migration, the row stays at its legacy `canonical_id`, gets `canonical_status = legacy_unmigrated`, `non_semantic_mergeable = true`, and trust-band renders `use_with_caution`. It is never silently merged or forked under the new comparator until re-extraction succeeds.

### 8. Ambiguous lifecycle

`Ambiguous` decisions persist as a first-class lifecycle outcome — never silently dropped.

```sql
CREATE TABLE ambiguous_claim_pairs (
  pair_id              TEXT PRIMARY KEY,
  claim_id_a           TEXT NOT NULL,
  claim_id_b           TEXT NOT NULL,
  field_scores         JSONB NOT NULL,
  decision_id          TEXT NOT NULL REFERENCES canonicalization_decisions(decision_id),
  user_resolution      TEXT,           -- NULL | 'merged' | 'forked' | 'contradicted' | 'needs_user_decision'
  user_resolved_at     TIMESTAMP,
  reconcile_attempts   INT NOT NULL DEFAULT 0,
  next_reconcile_at    TIMESTAMP,      -- exponential backoff: created_at + (2^attempts) * base_interval, gated on extractor-schema/threshold updates
  last_schema_version  TEXT NOT NULL,  -- predicate registry + threshold version at last evaluation
  created_at           TIMESTAMP NOT NULL
);
```

**Lifecycle constants** — pinned in `comparator_thresholds.rs` alongside the threshold constants, version-tagged, tunable only through the §9 calibration fixture:
- `AMBIGUOUS_BASE_INTERVAL`: 7 days
- `AMBIGUOUS_MAX_ATTEMPTS`: 5
- `AMBIGUOUS_BACKOFF_BASE`: 2 (exponential — attempts 1..5 retry at +7d, +14d, +28d, +56d, +112d from creation)

**Mode guard on lifecycle:** `ambiguous_claim_pairs` rows track BOTH shadow and live decisions (the table is shared), but **only pairs whose `decision_id` references a `mode = 'live'` decision are surfaced to the user, re-evaluated for auto-resolve, or allowed to mutate claim state on resolution**. Shadow-mode ambiguous pairs are observational data for parity-report analysis only — resolving a shadow pair (via any code path) MUST NOT mutate canonical_id, edges, or trust bands. Suite S fixture asserts this contract: a user-action-equivalent resolution invoked on a shadow ambiguous pair is rejected with audit reason `ShadowPairResolutionAttempted` and produces zero state changes.

**Lifecycle hooks (LIVE pairs only):**
- Surfaces feed into the existing correction/corroboration UI as "possible duplicates" with both claims displayed + per-field score breakdown. User resolution updates the claim graph (merge | fork | contradict).
- **Trigger for re-evaluation is the schema-update event**, NOT wall-clock alone — `last_schema_version` is compared to current predicate-registry + threshold version on each ambiguous_pair_created/resolved signal cycle. Wall-clock `next_reconcile_at` is a backstop; the primary path is event-driven on extractor-schema or threshold version bumps.
- **Aging/retry:** after each extractor schema or threshold update, ambiguous pairs whose `last_schema_version` is stale AND past `next_reconcile_at` are re-evaluated. If the new decision is no longer Ambiguous, the pair auto-resolves and is removed from `ambiguous_claim_pairs`. Exceeding `AMBIGUOUS_MAX_ATTEMPTS` moves the pair to `user_resolution = 'needs_user_decision'` permanently.
- Trust-band rendering: claims that participate in unresolved ambiguous pairs render `use_with_caution` until resolved (this prevents trust-band trapping where legitimate duplicates persist as forks with under-counted corroboration).

Suite E adds a fixture asserting ambiguous duplicates can later converge without manual DB cleanup — the convergence test ships with W4-B.

### 9. Threshold calibration fixture

Thresholds 0.60 and 0.85 are **configurable constants** with version-pinned defaults, calibrated against a labeled fixture corpus.

**Calibration corpus** (lives in `src-tauri/suites/E/canonicalization-thresholds/`):
- 500+ labeled claim pairs across claim types (CommitmentClaim, TopicClaim, RiskClaim, etc.)
- Pair labels: `should_merge` | `should_fork` | `should_contradict` | `ambiguous_acceptable`
- Composition: positive paraphrases (40%), hard negatives (30%), contradiction pairs (15%), one-sided qualifier asymmetry (10%), low-trust duplicates (5%)

**Gate metrics** (must hold before auto-merge enables — single source of truth for §7 Phase B and W4-B Done-When):
- True-merge precision ≥ 0.98 on `should_merge` corpus — of pairs v2 decides `merge`, ≥98% are labeled `should_merge`
- **True-merge recall** ≥ 0.95 on `should_merge` corpus — of pairs labeled `should_merge`, v2 actually decides `merge` for ≥95% (prevents over-forking gaming precision)
- True-fork recall ≥ 0.95 on `should_fork` corpus
- Contradiction detection ≥ 0.97 on `should_contradict` corpus
- False-merge ceiling ≤ 0.5% — any false-merge counts double-weight in the gate
- **Ambiguous-rate cap ≤ 5%** per label bucket — prevents parking decisions in Ambiguous to dodge precision/recall
- **Tombstone-bypass rate = 0** (Suite S adversarial: paraphrased re-resurrection on BOTH query AND candidate sides)
- **Cross-tier merge rate = 0** (Suite S)
- **Cross-account merge rate = 0** (Suite S)
- **Cross-workspace merge rate = 0** (Suite S — workspace scope is a distinct CandidateFilter gate from account)
- **Legacy-unmigrated merge rate = 0** (Suite S — rows with `non_semantic_mergeable = true` or `canonical_status = legacy_unmigrated` never enter v2)
- **`pending_backfill_count = 0`** — hard Phase C cutover precondition. Backfill must drain every row to `live` or `legacy_unmigrated` before v2 becomes authoritative.
- **`pending_backfill_rejection_rate` ≤ 1%** during Phase A/B canonicalize calls (proves backfill is catching up to ingestion rate).
- **Terminalization** — `pending_backfill` rows past `PENDING_BACKFILL_MAX_AGE = 24h` AND `PENDING_BACKFILL_MAX_RETRIES = 3` become `legacy_unmigrated` automatically with audit reason `PendingBackfillTerminalized`. Constants pinned in `comparator_thresholds.rs`.

**Threshold changes** require corpus regeneration + gate re-clear, not constant tweaks in code. Threshold constants live in a single `comparator_thresholds.rs` module with a version tag; bumping the version requires re-running the calibration suite.

### 10. Helper-intent carryover

The legacy lexical helper module encoded semantics the new comparator must preserve. Each helper's intent maps to a new mechanism; deletion is allowed only when the carryover is verified.

| Legacy helper | Carryover mechanism | Verification |
|---|---|---|
| `canonicalize_semantic_text` | Removed — embeddings + structural fields supersede tokenization | Suite E cycle-1..15 contract fixtures green |
| `lookup_semantic_term` | `predicate` registry equality + alias table (§11) | Predicate-alias fixture in Suite E |
| `is_semantic_negator` | `Polarity` first-class field | Suite E negation fixtures (c3/c11/c12) |
| `is_semantic_stopword` | Embedding tokenization handles stopwords inherently | Cycle-1..15 contract fixtures |
| `semantic_signature_for_text` | `StructuredClaim` typed fields + embedding for FreeText | n/a — replaced |
| `semantic_stem` | Embedding tokenization | Cycle-1..15 contract fixtures |
| `combine_semantic_status` | **`ClaimStatus` lattice rule** — Confirmed + Pending → Confirmed; Confirmed + Unknown → Confirmed; Pending + Unknown → Pending; never downgrade | Status-lattice unit test in Suite E |
| `semantic_status_compatible` | `status` field comparator with explicit lattice | Status-compat fixture |
| `semantic_high_salience_qualifiers` | `QualifierSet` typed columns persisted (not metadata) — `numerics` + `extras` cover the named-entity + one-sided-numeric cases from c8/c14/c15 | Qualifier-coverage fixtures |
| `is_semantic_named_entity` | `qualifiers.entity` typed EntityRef | Entity-qualifier fixture |
| `is_semantic_low_salience_token` | Salience weights per claim-type (§4) | Salience-weight unit test |
| `metadata_with_semantic_qualifiers` | Qualifiers persist as typed columns on `intelligence_claims`, not derived metadata round-trip | Migration test asserting qualifiers survive re-canonicalization pass without metadata round-trip |
| `semantic_qualifiers_from_metadata` | Removed — typed columns are the source of truth | n/a — replaced |
| `is_semantic_metadata_qualifier` | Removed | n/a |
| `semantic_claim_qualifiers` | `StructuredClaim.qualifiers` direct access | n/a |
| `semantic_signatures_near_duplicate` | `canonical_match_v2` | Suite E + threshold calibration |

Deletion is gated on §7 Phase C cutover. The CI grep guard rejects re-introduction of any helper by name post-cutover.

### 11. Predicate registry — closed namespaced, alias-supported

Resolved from prior "open question": `PredicateRef` is a **closed namespaced registry**, not free-form.

- Registry lives in `abilities-runtime/src/predicates/registry.rs` with explicit enum-like definitions per claim-type.
- Predicate aliases are explicit: each canonical predicate carries a `Vec<String>` of synonyms.
- **Alias matching order** (strict — bypassing this order opens a polarity-bypass channel):
  1. **Polarity equality** is the first gate. Predicates with different polarity (Affirm vs Negate) never alias — "approved" and "rejected" or "not approved" do not match via alias resolution even if their canonical forms embed close. This is enforced before any literal or embedding comparison runs.
  2. **Literal equality** against canonical form OR any explicit alias string.
  3. **Embedding cosine** ≥ HIGH_THRESHOLD against canonical form — ONLY if literal failed AND polarity matched. Embedding comparison NEVER crosses polarity boundary.
- Adding a new predicate requires an ADR amendment (bounded surface — analogous to claim-type addition per ADR-0125).
- Extractor abilities receive the registry at construction and must emit `PredicateRef::Resolved(id)` or `PredicateRef::Unresolved(text)`. Unresolved predicates set `non_semantic_mergeable = true` until the registry is updated.

This eliminates the "open predicate-namespace risk" — bounded by construction, extensible by ADR — and closes the polarity-bypass-via-alias-embedding channel.

### 12. Object resolution boundary — shared with subject_ref

Resolved from prior "open question": `ObjectValue::Resolved` uses the same entity-type infrastructure as `subject_ref`. When the extractor cannot resolve to a known entity, it falls through to `Literal` (for typed values like dates/money/percentages) or `FreeText` (for free-form text).

- `Resolved` and `Literal` paths use typed equality only (no embedding).
- `FreeText` is the only `object` variant that triggers the embedding comparator.
- The migration backfill attempts `Resolved` first, then `Literal`, then `FreeText`. Backfill that lands in `FreeText` is logged for future extractor schema improvements.

### 13. Intelligence Loop signal/invalidation contract

Per CLAUDE.md §Critical Rules — every new table, schema column, claim field, or user-visible intelligence surface must answer the 5 Intelligence Loop questions. This section maps W4-B's surfaces.

**New signals emitted:**
- `structural_backfill_changed { claim_id, field_set }` — Phase A re-extraction updated structural fields on an existing claim
- `canonicalization_decision_created { decision_id, mode, claim_id_a, claim_id_b, decision }` — every `canonical_match_v2` evaluation (shadow OR live)
- `canonical_merge_applied { merged_claim_id, source_claim_ids, decision_id }` — Phase C only; corroboration count + merged provenance changed
- `ambiguous_pair_created { pair_id, claim_id_a, claim_id_b }` — new ambiguous pair recorded
- `ambiguous_pair_resolved { pair_id, resolution }` — user or schema-update resolved a pair
- `trust_band_downgraded { claim_id, prior_band, new_band, reason }` — affected by `pending_backfill`, `legacy_unmigrated`, `hash_fallback`, or unresolved ambiguous-pair participation
- `trust_band_cleared { claim_id, new_band }` — downgrade condition cleared (e.g., re-extraction succeeded, model restored, ambiguous pair resolved)

**Propagation / invalidation targets** (synchronous unless noted):
- `build_intelligence_context(claim_id)` — invalidates cached context on any signal touching `claim_id` or its corroboration set
- `gather_account_context(account_id)` — invalidates on `canonical_merge_applied`, `ambiguous_pair_created/resolved`, `trust_band_downgraded/cleared` affecting claims owned by `account_id`
- **Prep outputs** (`build_daily_readiness`, `list_open_loops`, `detect_risk_shift`) — invalidate on any merge/downgrade affecting claims they consumed; re-prep on next read
- **Callouts** (`field_attribution`, "possible duplicates" surface) — re-render on `canonicalization_decision_created` with `decision = fork_ambiguous` AND on `ambiguous_pair_resolved`
- **Tauri surface** — emits `claim_updated` event keyed on `claim_id`; renderer subscribes
- **MCP surface** — respects subject-ownership enforcement (DOS-288); signals do not leak across subject ownership; MCP-bridge re-tests in W4 merge gate

**Bounded-sync propagation:** User-driven `ambiguous_pair_resolved` (user clicks "merge" in the possible-duplicates UI) MUST update `build_intelligence_context` and any visible callout within the same UI frame. Schema-update-driven auto-resolution is async (batch over the affected pairs, signal each).

**Feedback loop (5th question):**
- User correction on an `ambiguous_pair` → updates `user_resolution` + the affected claim graph (merge | fork | contradict) + feeds the parity calibration corpus as a labeled pair for future threshold tuning
- User dismissal of a "possible duplicates" callout → records `user_resolution = 'forked'` permanently
- User corroboration on a merged claim → strengthens the `field_provenance` confidence; counts toward trust-band uplift per W3-B factor library
- User contradiction on a previously-merged claim → un-merges, writes contradiction edge, raises `legacy_unmigrated` review priority on participating sources
- Source-reliability feedback: a source whose ambiguous-pair resolutions disproportionately resolve to `contradicted` has its source-reliability factor downgraded per ADR-0114

## Why this is meaningfully better

**Eliminated by construction:**
- Synonym list curation — embeddings cluster `budget`/`funding`/`spend` naturally; predicate registry handles structural synonyms explicitly
- Tokenization edge cases — embeddings ignore tokenization
- Regex qualifier extraction — qualifiers become typed extraction outputs
- Threshold soup — single calibrated threshold band per field type, gated by §9 fixture
- Free-text bag-of-terms — replaced by structural equality + embedding only for `FreeText` object fallback
- Negation merge errors — `Polarity` first-class

**Preserved (via §5 CandidateFilter + §10 carryover):**
- Tier compatibility gating
- Tombstone shadow logic (legacy + structural in parallel until cutover)
- Dormant/surfacing-state filtering
- Account/workspace scoping
- Migration framework + fail-closed verification
- Cycle-1..15 contract fixtures (negation, region, tombstone, dormant, low-trust, scope) — transfer as no-regression bar

**Added:**
- Auditable per-field decision trail (`canonicalization_decisions`)
- Cache versioned by model_version
- Fallback semantics for offline mode with health-signal exposure
- Ambiguous lifecycle with aging/retry
- Threshold calibration fixture with stated false-positive ceiling
- Shadow-mode parity gate before legacy primitive deletion

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Embedding latency on `commit_claim` hot path | `embed_batch` + LRU cache; inference outside SQLite write tx (§3); claim-type-specific salience minimizes embed-required surface |
| Hash fallback mode silently disables semantic canon | `EmbeddingModel::status()` exposed as health signal; Live-mode auto-merge disabled when fallback active; degraded mode renders `use_with_caution` |
| Threshold tuning regression | Calibration fixture (§9) with false-positive ceiling; threshold changes require corpus regeneration + gate re-clear |
| Extractor schema gaps | Bounded vs current matcher's unbounded text-heuristic gaps — fixable via §11 registry additions; ambiguous lifecycle (§8) catches what extractor misses |
| Migration of existing claims | Additive (§7) — no claim_id rewrite; dual-read shadow mode; parity-gated cutover; legacy helpers preserved until parity proves equivalence |
| Cross-tier / tombstone / cross-account leakage via comparator | CandidateFilter (§5) is the first authoritative in-comparator rejection point with primary+secondary audit reasons; SQL enumeration is a safe-superset account/workspace/tier prefilter only; Suite S adversarial fixtures assert zero leakage |
| Embedding model upgrade invalidates canonical_ids | `embedding_model_version` persisted per decision (§6); one-shot re-canonicalization migration on model upgrade |
| Ambiguous duplicates trap trust bands | Ambiguous lifecycle (§8) with aging/retry; trust-band renders `use_with_caution` until resolved; auto-converge on extractor improvements |
| Per-field provenance lost on merge | `field_provenance` JSONB column (§6); `ClaimEnvelope::field_attribution()` API |

## Adoption sequencing — inline in v1.4.1 W4-B, three phases

**Implemented inline in v1.4.1 W4-B as the convergence move.** Per `feedback_no_deferrals_period.md` and `feedback_no_deferrals_while_fresh_in_memory.md`: the cycle-by-cycle pattern of L2 findings IS the design feedback telling us the primitive is wrong. Deferring to v1.4.2 would lose diagnostic context.

**Phase A — additive structural fields + shadow comparator** (W4-B.1 step 1):
- Migration v160 — see §7 Phase A above for the authoritative column list. Adds (canonical summary): `predicate_ref`, `polarity`, `object_value`, `qualifiers`, `structural_canonical_id`, `canonical_status` (default `pending_backfill`), `non_semantic_mergeable` (default `true`), `structural_field_content_hash`, `backfill_epoch` (default 0); creates `canonicalization_decisions` audit table with `mode` column and `idempotency_key UNIQUE` constraint, `ambiguous_claim_pairs` table with FK to `canonicalization_decisions.decision_id`
- Implement `StructuredClaim` types in `abilities-runtime`
- Implement `PredicateRef` registry (§11) with initial predicates for shipped claim types
- Implement `canonical_match_v2` comparator with embedding cache (§3)
- Implement `CandidateFilter` (§5) with all gates active on BOTH query and candidate sides
- Implement Intelligence Loop signals (§13)
- Wire shadow-mode invocation from `commit_claim` — writes to `canonicalization_decisions` with `mode = 'shadow'`, never alters canonicalization state
- Backfill extractor: re-extract structural fields for existing claims; unresolved → `canonical_status = legacy_unmigrated`, `non_semantic_mergeable = true`, trust-band downgraded to `use_with_caution` for affected rows
- W4-B.1 internal L2 cycle against Phase A scope

**Phase B — calibration + shadow parity gate** (W4-B.2 part 1):
- Build calibration corpus (§9) in `src-tauri/suites/E/canonicalization-thresholds/`
- Run shadow mode against production-drained fixtures + calibration corpus (decisions write to `canonicalization_decisions` with `mode = 'shadow'`)
- Generate parity report: v1 vs v2 decisions over the corpus, bucketed by divergence reason
- Parity gate clears when ALL §9 metrics hold AND Suite S adversarial fixtures pass
- Interim L3-style codex challenge against the parity report + ADR (architectural-swap gate)

**Phase C — cutover + helper deletion** (W4-B.2 part 2, gated on Phase B clear):
- Switch `commit_claim` from `semantic_signatures_near_duplicate` → `canonical_match_v2` as authoritative (decisions now write `mode = 'live'`)
- Delete legacy lexical helper module (§10 table — every helper has a verified carryover)
- Add CI grep guard against re-introduction by name
- Migration v161 records cutover + `embedding_model_version` + `comparator_threshold_version` pin
- W4-B.2 internal L2 cycle covers Phase B + Phase C as a combined commit chain on the local branch
- W4-B PR opens after Phase C commit chain is L2-cleared and parity-report-cleared (single PR exit; no intermediate pushes)

This is the W4-B exit. No v1.4.2 follow-up.

## Substrate fit

This ADR is the right shape for v1.4.x's substrate thesis:
- Claims become **structurally compared**, not lexically guessed
- Canonicalization becomes **explainable** (per-field scores + audit trail) instead of **emergent** (threshold cascade)
- The embedding model already in the runtime gets a second consumer beyond Bayesian scoring — paying the cost once, using it twice
- Bug class shifts from "the matcher missed this token edge case" to "the extractor schema needs predicate X" — bounded and fixable through §11 registry
- Migration is additive — no claim_id churn; downstream tables and signals untouched
- Trust boundary preserved — CandidateFilter (§5) is the first authoritative in-comparator rejection point; SQL-side enumeration is a safe-superset prefilter (account/workspace/tier indexed columns only), with tombstone/migration-status audit reasons captured in-comparator

## What this does NOT do

- Does NOT change the claim substrate's contradiction-vs-merge semantics (per [ADR-0113](0113-claim-anatomy-and-dedup.md))
- Does NOT change provenance attribution or trust scoring (per [ADR-0105](0105-provenance-as-first-class-output.md) / [ADR-0114](0114-scoring-unification.md)) — but it DOES enrich per-field provenance via `field_provenance` (§6)
- Does NOT change sensitivity/temporal_scope gating (per [ADR-0108](0108-provenance-rendering-and-privacy.md) / [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)) — but it DOES move tier gating earlier in the pipeline via CandidateFilter (§5)
- Does NOT rewrite existing `claim_id`, `dedup_key`, `item_hash`, or contradiction/corroboration edges — strictly additive (§7)
- Does NOT persist embeddings in W4-B — `claim_embeddings` table is a future v1.4.2+ optimization
- Does NOT add new predicates without an ADR amendment — `PredicateRef` registry is bounded (§11)

## Open questions — resolved for W4-B

Prior versions of this ADR deferred 5 questions to v1.4.2. Per `feedback_no_deferrals_while_fresh_in_memory.md`, these are now resolved inline:

1. **Predicate registry design** → Closed namespaced registry with explicit alias lists (§11). New predicates require ADR amendment.
2. **Object resolution boundary** → Shared `entity_ref` infrastructure with `subject_ref`; FreeText fallback when entity-typing fails (§12). Only FreeText triggers embedding.
3. **Embedding-cache durability** → In-memory LRU only for W4-B (§3); not persisted. `claim_embeddings` persistent table is a future v1.4.2+ optimization, gated separately.
4. **Migration scope** → Additive + dual-read + shadow-mode parity-gated cutover (§7). No claim_id rewrite. Legacy primitive preserved until parity proves equivalence.
5. **Salience weighting** → Claim-type-specific defaults pinned in `CLAIM_TYPE_REGISTRY` (§4). Tunable through calibration fixture (§9), never through bare code constants.

These resolutions are the W4-B L0 gate, not v1.4.2 ticket scope.
