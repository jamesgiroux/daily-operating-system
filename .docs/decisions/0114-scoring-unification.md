# ADR-0114: Scoring Unification — Shared Primitives, Per-Composer Surfaces

**Status:** Proposed
**Date:** 2026-04-19
**Target:** v1.4.0 substrate (shared factor library, trust extractor) / v1.6.0 (health composer migration)
**Extends:** [ADR-0097](0097-account-health-scoring-architecture.md)
**Related:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0107](0107-source-taxonomy-alignment.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md)
**Consumed by:** [DOS-5](https://linear.app/a8c/issue/DOS-5) Trust Compiler, [DOS-10](https://linear.app/a8c/issue/DOS-10) Freshness decay, [DOS-238](https://linear.app/a8c/issue/DOS-238) Factor library implementation

## Context

DailyOS has two composers that produce scores from entity signals: the **Trust Compiler** ([DOS-5](https://linear.app/a8c/issue/DOS-5)) which scores per-claim trustworthiness for the provenance envelope, and the **Account Health Composer** ([ADR-0097](0097-account-health-scoring-architecture.md)) which scores per-dimension health for briefings. As drafted, both reimplement the same primitive calculations — source reliability from Bayesian posteriors, freshness decay, corroboration boosts, contradiction penalties, user-feedback modulation. The math is identical; only the composition differs (weighted geometric mean for trust, lifecycle-weighted arithmetic for health).

Duplicating factor math is a correctness hazard. A fix to the freshness decay function in the trust path that does not also land in the health path produces silently divergent behavior — a claim that is "stale" for trust but "fresh" for health. Worse, the six health dimensions ([ADR-0097](0097-account-health-scoring-architecture.md)) already include freshness-like decay inside their composition, creating three implementations at risk of drifting.

An adversarial review during v1.4.0 planning surfaced a second concern: "pure factors" cannot take an `EntityId` parameter and still be pure. A function that takes an ID implicitly reads the database, carries a hidden clock, or depends on global state — all of which break determinism under [ADR-0104](0104-execution-mode-and-mode-aware-services.md)'s `Evaluate` mode and defeat [ADR-0110](0110-evaluation-harness-for-abilities.md)'s fixture-replay guarantees.

This ADR specifies the shared factor library, the per-composer extractor pattern, what stays inside each composer, and the confidence evidence structure that propagates to the UI. The goal is one source of truth per primitive, zero hidden state, and deterministic behavior under fixed inputs.

## Decision

### 1. Shared primitive factors

A single module, `src-tauri/src/scoring/factors.rs`, contains pure functions for cross-composer primitives. Every function in this module:

- Takes explicit typed inputs; no `EntityId`, no `&AbilityContext`, no `&Db`, no globals.
- Takes any clock-dependent quantity (e.g., "age of this claim") pre-computed by the caller, not a clock reference.
- Is deterministic: the same inputs always produce the same output.
- Is side-effect free.
- Is tested with table-driven property tests that cover boundary cases.

The canonical set is small and closed — new factors require an ADR amendment.

```rust
// src-tauri/src/scoring/factors.rs

pub fn source_reliability(beta_alpha: f64, beta_beta: f64) -> f64;

pub fn freshness_weight(
    source: &DataSource,
    age_days: f64,
    ctx: &FreshnessContext,
) -> f64;

pub fn corroboration_weight(independent_source_count: u32) -> f64;

pub fn contradiction_penalty(flags: &ContradictionSet) -> f64;

pub fn user_feedback_weight(feedback: Option<&UserFeedback>) -> f64;

pub fn meeting_relevance_weight(
    signal_type_affinity: f64,
    meeting_context_match: f64,
    attendee_overlap: f64,
) -> f64;
```

Supporting types, also pure:

```rust
pub struct FreshnessContext {
    pub upcoming_renewal_within_days: Option<u32>,
    pub claim_type: ClaimType,
}

pub struct ContradictionSet {
    pub coherence_check_failed: bool,
    pub peer_contradictions: u32,
}

pub struct UserFeedback {
    pub action: FeedbackAction,        // StillTrue | Outdated | WrongSource
    pub at: DateTime<Utc>,             // Caller pre-computes age if needed
}
```

Absent from this list: anything that requires entity graph traversal, any aggregation across multiple claims, any composition rule. Those belong in composers.

### 2. Per-surface extractors

Each composer has its own extractor that reads the database, injects the clock, and produces the typed factor inputs.

**Trust extractor:** `src-tauri/src/scoring/extract/trust.rs`

```rust
pub async fn extract_trust_inputs(
    claim: &Claim,
    ctx: &ServiceContext<'_>,
) -> Result<TrustScoringInputs, Error>;

pub struct TrustScoringInputs {
    pub source_alpha: f64,
    pub source_beta: f64,
    pub source: DataSource,
    pub age_days: f64,                            // Computed from ctx.clock.now() - claim.created_at
    pub freshness_ctx: FreshnessContext,
    pub independent_source_count: u32,
    pub contradictions: ContradictionSet,
    pub feedback: Option<UserFeedback>,
    pub meeting_relevance: MeetingRelevanceInputs,
}
```

**Health extractor:** remains co-located with the health composer at `src-tauri/src/intelligence/health_scoring.rs`. Per-dimension extractors produce dimension-specific input structs that call the shared primitives where applicable.

Extractors are where database access and clock injection happen. Factors never touch either. Extractors are tested with fixture-loaded databases; factors are tested with table-driven pure inputs.

### 3. Relationship dimensions stay in `health::dimensions`

The six health dimensions from [ADR-0097](0097-account-health-scoring-architecture.md) — `meeting_cadence`, `email_engagement`, `stakeholder_coverage`, `champion_health`, `financial_proximity`, `signal_momentum` — are **not** promoted into `scoring::factors`. They are health-composer-internal compositions that may call shared primitives internally but express dimension-specific semantics (thresholds, lifecycle weights, per-dimension decay curves) that are not meaningful outside health.

Rationale: `meeting_cadence` is not a universal primitive — "are meetings happening often enough?" depends on stage, segment, and contract value. Trust scoring has no equivalent. Promoting it would add surface area without sharing.

Health dimensions MAY invoke shared primitives like `freshness_weight` and `contradiction_penalty` from within their own composition. They may not maintain their own implementations of these — if a dimension needs a tweaked variant, the variant is either (a) a config parameter to the shared factor, or (b) a local helper in the dimension module that clearly documents why the shared factor didn't fit.

### 4. Per-composer combinators

Composers combine factors their own way. This is the point of the split.

**Trust Compiler** ([DOS-5](https://linear.app/a8c/issue/DOS-5)): weighted geometric mean over six factors.

```rust
trust_score = exp(Σ w_i × log(factor_i))
```

Weights, clamps, and bands live in `config/trust_compiler.toml`. The Trust Compiler reads primitives from `scoring::factors` and composes; it does not reimplement any primitive.

**Account Health Composer** ([ADR-0097](0097-account-health-scoring-architecture.md)): lifecycle-weighted arithmetic mean over the six dimensions.

```rust
dimension_score = weighted_composition_of_dimension_specific_signals
account_health = Σ lifecycle_weight(i) × dimension_score(i)
```

Lifecycle weights, dimension thresholds, and confidence adjustments are health-composer internal.

Future composers (e.g., a v1.6.0+ "meeting priority" score, a v2.x "portfolio alert" score) will likewise have their own combinators and reuse `scoring::factors`.

### 5. Confidence evidence, not shared bands

Each composer produces its own band taxonomy — trust uses `likely_current | use_with_caution | needs_verification`, health uses a dimension-specific color coding, future composers will define their own. Bands are not shared; forcing a shared band taxonomy would couple unrelated surfaces to a single scale.

What **is** shared: a `ConfidenceEvidence` struct that every composer emits alongside its score, consumed uniformly by UI rendering.

```rust
pub struct ConfidenceEvidence {
    pub score: f64,                      // The composer's score, normalized [0, 1]
    pub band_label: String,              // Composer-defined band name
    pub factor_breakdown: Vec<FactorEvidence>,
    pub caveats: Vec<ConfidenceCaveat>,
}

pub struct FactorEvidence {
    pub name: &'static str,              // "freshness", "corroboration", ...
    pub value: f64,
    pub contribution: f64,               // Weight × log(value) for geometric; weight × value for arithmetic
}

pub enum ConfidenceCaveat {
    FewSources,
    StaleSource { source: DataSource, age_days: f64 },
    UnresolvedContradiction,
    InsufficientSignalDensity,
}
```

UI surfaces render `ConfidenceEvidence` uniformly: a score, a band, an expandable breakdown, and a list of caveats. Bands differ per composer; the rendering shell is common. This is what [ADR-0108](0108-provenance-rendering-and-privacy.md) consumes.

### 6. Determinism and mode awareness

Factors are pure by construction. Extractors use the injected clock from [ADR-0104](0104-execution-mode-and-mode-aware-services.md) for any age computation. Under `ExecutionMode::Evaluate`:

- Extractors hit the fixture-loaded SQLite per [ADR-0110](0110-evaluation-harness-for-abilities.md).
- Clock is the fixture's `clock.txt`.
- Factor outputs are fully deterministic given fixture inputs.
- Composer outputs are fully deterministic given factor outputs.

This means both trust scores and health scores are replayable in the evaluation harness with no additional work beyond fixture creation. Regression tests for scoring changes become cheap.

### 7. Evaluation

`src-tauri/tests/scoring/` holds the factor-level property tests:

- Each factor has a table-driven test with ≥10 boundary and typical cases.
- Factor output ranges, monotonicity, and clamp behavior are property-tested against 10K random inputs.
- Extractors have fixture-based tests that verify correct input assembly from representative DB states.
- Composers have evaluation-harness fixtures ([ADR-0110](0110-evaluation-harness-for-abilities.md)) verifying end-to-end score outputs against hand-computed expected values.

### 8. Config

Scoring-related configuration consolidates under one namespace:

```
config/scoring.toml
├── [trust]
│   ├── weights = { ... }
│   ├── clamps = { ... }
│   └── bands = { ... }
├── [health]
│   ├── lifecycle_weights = { ... }
│   └── dimension_thresholds = { ... }
└── [factors]
    ├── freshness_half_lives = { ... }   # consumed by freshness_weight
    └── corroboration_scale = 0.5          # consumed by corroboration_weight
```

Boot-time validation: all weights sum where required, no thresholds outside [0, 1], and composers fail-fast on malformed config.

## Consequences

### Positive

- **One source of truth per primitive.** Fixing `freshness_weight` lands everywhere automatically. Drift between trust and health becomes impossible by construction.
- **Determinism preserved.** Pure factors are replayable in the evaluation harness; no hidden state, no implicit DB reads.
- **Composers stay expressive.** Each composer keeps its own combinator — geometric mean, lifecycle weighting, whatever future surfaces need — without being forced into a shared scale.
- **UI rendering uniform.** One `ConfidenceEvidence` shape means the product's explanation affordance works identically for trust and health without cross-coupling their semantics.
- **Adversarial review finding addressed.** No factor takes `EntityId`; purity is structural, not conventional.
- **Factor library is small and closed.** Six primitives, each well-bounded. Growth requires ADR amendment — forces us to be deliberate about what becomes universal.

### Negative / risks

- **Extractors are real work per composer.** Each new composer pays a boilerplate cost to assemble inputs. Accepted — the cost is one-time per surface and keeps factor-level code clean.
- **Refactoring health to use shared primitives is deferred.** The v1.4.0 cycle ships the library + trust extractor only. Health composer migration is v1.6.0 Hardening. During this window, health and trust compute freshness separately; drift is possible and must be caught by cross-composer property tests.
- **"Why isn't `meeting_cadence` shared?" is a recurring question.** The answer is consistent: health dimensions are compositions, not primitives. Reviewers and future contributors will want a shared "relationship scorer." Resist it — it is semantic coupling disguised as reuse.
- **Config namespace split requires migration.** Existing `config/trust_compiler.toml` references merge into `config/scoring.toml` with backward-compatible alias. Ships behind a feature flag.

### Neutral

- No user-visible change. This is internal architecture.
- `DataSource` taxonomy unchanged. [ADR-0107](0107-source-taxonomy-alignment.md) continues to own source classification; this ADR consumes it.
- `scoring::factors` is a new module but does not grow over time. A healthy sign.

---

## Revision R1 — 2026-04-19 — Reality Check

Post-draft adversarial review (codex) and codebase reference passes flagged that this ADR talks about creating a factor library as if one does not exist. In fact, factor-like code already exists in scattered places. Revision below.

### R1.1 Existing factor code — this ADR is refactor, not greenfield

Ground truth:

- `src-tauri/src/signals/bus.rs:60` already contains `source_base_weight` and `default_half_life` constants plus the Bayesian source-reliability update logic.
- `src-tauri/src/signals/decay.rs` contains freshness decay math.
- `src-tauri/src/signals/fusion.rs` contains corroboration and contradiction combinators.
- `src-tauri/src/intelligence/health_scoring.rs` has its own unrelated hardcoded constants and uses `Utc::now()` directly inside `compute_account_health`.

The original ADR treats `scoring::factors` as a net-new module. It is not. This ADR is a **consolidation refactor** that extracts the scattered primitives into a single pure-function module and makes the two composers (trust, health) consume it. Framing correction: amend §1's opening line to "A single module, `src-tauri/src/scoring/factors.rs`, **consolidates** pure functions for cross-composer primitives **currently scattered across `signals/bus.rs`, `signals/decay.rs`, `signals/fusion.rs`, and inline in `intelligence/health_scoring.rs`**."

The implementation plan adds a **Phase 0** that precedes everything in the original §1–8:

- **Phase 0 (v1.4.0):** Audit the existing factor math; reconcile formula drift between health scoring and signal fusion; consolidate into `scoring::factors`; leave current call sites untouched but add the new module alongside.
- **Phase 1 (v1.4.0):** Trust Compiler ([DOS-5](https://linear.app/a8c/issue/DOS-5)) consumes `scoring::factors` via the trust extractor.
- **Phase 2 (v1.6.0):** Health scoring composer migrated to consume `scoring::factors`; `compute_account_health` made pure; DB-history writes extracted to a maintenance ability per [ADR-0103](0103-maintenance-ability-safety-constraints.md).

### R1.2 Health scoring determinism requires extracting writes first

Codex flagged: `compute_account_health` is not pure today — it mutates DB history inside the compute call. The ADR's claim of "deterministic scoring under fixture replay" is false until writes are split out.

**Revised:** Phase 2 explicitly extracts the history-write side effect. The refactor:

```rust
// Before
fn compute_account_health(entity_id: &str, db: &Db) -> HealthScore { /* computes + writes history */ }

// After
fn compute_account_health_pure(inputs: HealthScoringInputs, clock: &Clock) -> HealthScore { /* pure */ }
fn record_health_score(entity_id: &str, score: &HealthScore, db: &Db, ctx: &ServiceContext) -> Result<()> { /* mutation */ }
```

Callers invoke both in sequence. Tests invoke only the pure variant. This matches the abilities contract in [ADR-0102](0102-abilities-as-runtime-contract.md) §3: read/transform abilities do not mutate.

### R1.3 UserFeedback input — pass precomputed age, not timestamps

Codex flagged: `UserFeedback { action, at: DateTime<Utc> }` smuggles time into a factor. Factors must not see clocks.

**Revised:** the factor signature becomes:

```rust
pub fn user_feedback_weight(feedback: Option<&UserFeedbackInput>) -> f64;

pub struct UserFeedbackInput {
    pub action: FeedbackAction,
    pub age_days: f64,             // Pre-computed by the extractor using injected clock
}
```

The extractor does the age computation. The factor never touches a clock. This extends the existing `FreshnessContext` pattern uniformly.

### R1.4 `meeting_relevance_weight` is surface-specific — demote from canonical

Codex flagged: `meeting_relevance_weight(signal_type_affinity, meeting_context_match, attendee_overlap)` is specific to the Trust Compiler's meeting-prep path. It does not generalize. Promoting it to `scoring::factors` creates a factor that health scoring will never call and future composers will either ignore or misuse.

**Revised:** `meeting_relevance_weight` moves from canonical factors to a trust-composer-local helper at `src-tauri/src/scoring/extract/trust.rs`. It is consumed only by Trust Compiler. Canonical factors drop to five: `source_reliability`, `freshness_weight`, `corroboration_weight`, `contradiction_penalty`, `user_feedback_weight`.

### R1.5 `freshness_weight` takes DataSource — make dependency explicit

Codex flagged: `freshness_weight(source: &DataSource, ...)` bakes the source taxonomy into a generic scoring primitive. If that's the intent, it must be explicit about the dependency.

**Revised:** `scoring::factors` depends on [ADR-0107](0107-source-taxonomy-alignment.md) by design. The `DataSource` enum is the only external type factors take. The half-life table per source lives in `config/scoring.toml` under `[factors.freshness_half_lives]`. A future source taxonomy change requires a coordinated update to the config; that is a feature, not a bug.

### R1.6 Config loading path — fill in the hand-wave

The original §8 named `config/scoring.toml` but did not specify ownership, reload, defaults, or migration.

**Revised:**

- Config is owned by a `ScoringConfig` singleton loaded at boot from `config/scoring.toml` in the app support directory. Defaults compiled into the binary — config file is additive override, not required.
- Reload on config file change is **not** supported in v1.4.0; a restart applies new values. This is intentional: hot-reloading scoring config invalidates every cached trust score; better to require explicit restart.
- Migration from today's hardcoded constants in `signals/bus.rs` and elsewhere happens in Phase 0: consolidate constants into `scoring::factors` defaults, emit config file with those defaults on first boot, delete the hardcoded constants.

### R1.7 Inter-ADR fix — trust score home defined

Codex flagged: [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) says trust depends on accepted/corroborated/retracted outcomes, but this ADR doesn't define where trust score **lives** relative to the canonical scoring stack.

**Revised:** trust score lives on the claim row (`intelligence_claims.trust_score` per [DOS-7](https://linear.app/a8c/issue/DOS-7)), computed by the Trust Compiler ([DOS-5](https://linear.app/a8c/issue/DOS-5)) which is the single composer that reads `scoring::factors` for trust purposes. The `corroboration_weight` factor reads from `claim_corroborations` (ADR-0113 R1.6); `user_feedback_weight` reads from `user_feedback_signals` ([DOS-8](https://linear.app/a8c/issue/DOS-8)); `contradiction_penalty` reads from `claim_contradictions` (ADR-0113 §7). All three inputs are claim-adjacent; the compiler stitches them in the trust extractor.

### R1.8 Scope for v1.4.0 — revised

- Phase 0 (factor consolidation + existing code refactor).
- Phase 1 (Trust Compiler uses the library).
- Config loader with compiled-in defaults.
- Pure function property tests.

Out of scope for v1.4.0:
- Phase 2 (health composer migration) — v1.6.0 Hardening.
- Hot config reload — future, and requires cache-invalidation design.
