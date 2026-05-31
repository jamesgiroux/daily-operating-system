# L0 Packet - v1.4.6 W1-B - DOS-330 Salience Scoring Engine

**Current revision:** V0.3 (L0 approved, 2026-05-26)

## 1. Header

- **Date:** 2026-05-26
- **Project:** v1.4.6 - Salience & Recommendations
- **Wave:** W1 stage 1b (after W1-A RecommendationClaim contract)
- **Issue:** [DOS-330 - Build salience scoring engine with explicit factors](https://linear.app/a8c/issue/DOS-330)
- **Branch:** `codex/v1.4.6-w1-b-dos-330`
- **Worktree:** `/Users/jamesgiroux/Documents/dailyos-repo/.worktrees/codex/v1.4.6-w1-b-dos-330`
- **Base:** `public/dev` @ `4ff1acc0` (W1-A PR #400 merge)
- **Migration slots claimed:** **v270-v272** for W1-B. v270 creates salience storage; v271-v272 remain reserved for same-lane repair if L2/CI requires a follow-up schema migration.
- **Authority docs:** `.docs/plans/v1.4.6-waves.md`; ADR-0102; ADR-0105; ADR-0114; ADR-0123; ADR-0125; ADR-0131.
- **Required L0 gates:** codex challenge + architect/feasibility review + K-in learnings check. Because the plan touches claim-adjacent scoring, migrations, and ability exposure, the L0 packet also calls out the security/privacy invariants reviewers should check.

### Review History

| Cycle | Result | Remediation |
|---|---|---|
| 1 | BLOCK | Split read ability from persistent recompute path; remove MCP/SurfaceClient exposure; remove caller-supplied actor authority; add K-in evidence; seed v270 weights; pin factor extractors to existing durable substrate. |
| 2 | Mixed: architect PASS, K-in PASS, codex challenge BLOCK | Add ADR-0104 mutation gate to persistent recompute; make v270 DDL idempotent/retry-safe; set `mcp_exposure = None` and test hidden MCP/SurfaceClient enumeration. |
| 3 | PASS | Codex challenge, architect/feasibility, and K-in all passed V0.3. Proceed to implementation with watch items: keep novelty off raw `original_text`, verify v270 partial rerun shape/weights, and keep `read.recommendations` scope reconciled before future MCP/SurfaceClient promotion. |

## 2. Live-Dev Reconciliation

W1-A is merged on `dev` and W1-B can build on its contract:

| W1-B dependency | Live `dev` status |
|---|---|
| `services::recommendations::contracts::{SalienceScore, SalienceFactor, SalienceFactorKind, FactorRationale}` | Present from W1-A. |
| `services::recommendations::salience` placeholder | Present; W1-B owns filling it. |
| `ClaimType::Recommendation` and recommendation metadata indexes | Present; v269 migration registered. |
| `commit_claim` route for recommendation writes | Present; W1-B is read-only over claims and does not change `services::claims`. |
| Existing claim trust/freshness/corroboration/contradiction substrate | Present; consumed read-only. |

Important crate-boundary finding: W1-A salience DTOs live in the app crate under `src-tauri/src/services/recommendations/contracts.rs`. The abilities-runtime crate cannot depend on the app crate without reversing the dependency graph. Therefore the W1-B ability surface cannot directly import those service DTOs.

### 2.1 K-in Evidence

The cycle-1 K-in pass found relevant prior art that this packet now treats as binding:

| Source | Plan impact |
|---|---|
| `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` | W1-B must cite substrate-type grep evidence, not only proposed names. This packet uses existing trust/freshness/fusion/canonicalization/feedback rows and does not add parallel primitives. |
| `.docs/decisions/0102-abilities-as-runtime-contract.md` | Ability category follows call-graph effects. A `Read` ability cannot write `salience_factors`; persistent recompute is a service/system path, not the read ability path. |
| `.docs/decisions/0114-scoring-unification.md` | Scoring composers split pure compute from record/write effects. W1-B follows the same shape: extract factors, compute score, optionally record through an explicit service function. |
| `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md` | Recommendations are first-class claims; salience is claim-adjacent derived state, not a parallel recommendation substrate. |
| `.docs/decisions/0108-provenance-rendering-and-privacy.md` | Ability output must be actor-filtered and cannot leak raw claim text, source paths, prompts, output bodies, or private feedback details. |
| `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md` | MCP is consumption-first and v1.4.6 does not expose salience as a direct invocable MCP write/read data API. |
| `.docs/plans/v1.4.6-waves.md` + W1-A L0 packet | v270 is registered with `270_salience_factors.sql` because current v240+ migrations use filename == registered version. Older off-by-one lessons do not apply to this lane. |

## 3. Goal

Ship a deterministic salience engine that:

- computes `SalienceScore` from the ten W1-A `SalienceFactorKind` values;
- stores one inspectable row per claim/factor/evaluation in `salience_factors` when the explicit service recompute path runs;
- keeps salience separate from trust while reading trust/freshness as inputs;
- exposes an ability-runtime `score_salience` read ability with schema version 1 that does not mutate state;
- uses no LLM ranking call and no raw prose rationale assembly in the engine;
- keeps all factor payloads privacy-safe: opaque IDs, numeric values, timestamps, counts, enum codes, and typed rationale objects only.

## 4. Files Owned

### W1-B fills

- `src-tauri/src/services/recommendations/salience.rs`
- `src-tauri/src/migrations/270_salience_factors.sql`
- `src-tauri/src/migrations.rs` registration + migration tests
- `src-tauri/abilities-runtime/src/abilities/recommendations/mod.rs`
- `src-tauri/abilities-runtime/src/abilities/recommendations/salience.rs`

### W1-B narrow support edits required by the ability boundary

- `src-tauri/abilities-runtime/src/abilities/mod.rs` to export the recommendations ability module.
- `src-tauri/abilities-runtime/src/services/context.rs` to add a narrow read-only `SalienceReadHandle`, request/response DTOs at the runtime boundary, and `ServiceContext::score_salience(...)`.
- `src-tauri/src/services/context.rs` to attach the live app adapter that opens SQLite read-only and delegates to non-persistent scoring in `services::recommendations::salience`.

These support edits follow the existing `claim_receipt`, `workspace_graph`, and `list_open_loops` reader-handle pattern. They are not bridge semantic changes and do not touch MCP bus behavior.

### W1-B must not touch

- `src-tauri/src/services/claims.rs` core behavior, including `commit_claim`.
- `src-tauri/src/services/trust_recompute.rs` or trust scoring mutations.
- `signals/policy_registry.rs`, `signals/bus.rs`, or signal bus semantics.
- Other recommendation lane modules except imports required for compile.
- Claim lifecycle/retraction helpers.
- MCP bridge code beyond natural ability-registry discovery from a registered ability.

## 5. Storage Decision

Use DB-backed default weights instead of hardcoded-only weights.

Rationale:

- The wave plan requires the weight table to be inspectable.
- W4-A feedback may later tune weights; storing the default rows now avoids a second source of truth.
- The service will still expose a compiled fallback constant for migration/test resilience, but live computation loads weights from `salience_factors_weights` and verifies all ten kinds exist with total weight `1.0 +/- 0.0001`.

Migration v270 creates and seeds:

```sql
CREATE TABLE IF NOT EXISTS salience_factors_weights (
    factor_kind TEXT PRIMARY KEY CHECK (
        factor_kind IN (
            'importance', 'novelty', 'urgency', 'timing', 'userFit',
            'freshness', 'trust', 'corroboration', 'contradiction',
            'openLoopRelevance'
        )
    ),
    default_weight REAL NOT NULL CHECK (default_weight >= 0.0 AND default_weight <= 1.0),
    schema_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS salience_factors (
    id TEXT PRIMARY KEY,
    evaluation_id TEXT NOT NULL,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE,
    factor_kind TEXT NOT NULL CHECK (
        factor_kind IN (
            'importance', 'novelty', 'urgency', 'timing', 'userFit',
            'freshness', 'trust', 'corroboration', 'contradiction',
            'openLoopRelevance'
        )
    ),
    factor_value REAL CHECK (factor_value IS NULL OR (factor_value >= 0.0 AND factor_value <= 1.0)),
    weight REAL NOT NULL CHECK (weight >= 0.0 AND weight <= 1.0),
    rationale_json TEXT NOT NULL CHECK (json_valid(rationale_json) = 1),
    schema_version INTEGER NOT NULL DEFAULT 1,
    computed_at TEXT NOT NULL,
    UNIQUE (evaluation_id, claim_id, factor_kind)
);

INSERT INTO salience_factors_weights (factor_kind, default_weight, schema_version)
VALUES
    ('importance', 0.20, 1),
    ('novelty', 0.10, 1),
    ('urgency', 0.15, 1),
    ('timing', 0.10, 1),
    ('userFit', 0.10, 1),
    ('freshness', 0.10, 1),
    ('trust', 0.10, 1),
    ('corroboration', 0.05, 1),
    ('contradiction', 0.05, 1),
    ('openLoopRelevance', 0.05, 1)
ON CONFLICT(factor_kind) DO UPDATE SET
    default_weight = excluded.default_weight,
    schema_version = excluded.schema_version,
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now');
```

Indexes:

- `CREATE INDEX IF NOT EXISTS idx_salience_factors_claim_computed ON salience_factors(claim_id, computed_at DESC)`.
- `CREATE INDEX IF NOT EXISTS idx_salience_factors_evaluation ON salience_factors(evaluation_id, claim_id)`.
- `CREATE INDEX IF NOT EXISTS idx_salience_factors_kind ON salience_factors(factor_kind, computed_at DESC)`.

No raw claim text, prompt text, file path, output body, or free-text explanation is stored.

The registered migration version is `270` and the file is `270_salience_factors.sql`, matching the current v240+ convention already used by W1-A. v270 is retry/crash safe: all DDL uses `IF NOT EXISTS`, seed rows are idempotent via `ON CONFLICT`, and tests cover partial application/rerun before the schema version row is recorded.

## 6. Service Shape

`services::recommendations::salience` owns four layers:

1. **Pure scorer**
   - `aggregate_salience(factors: Vec<SalienceFactor>) -> SalienceScore`
   - applies the W1-B weighted average formula, skips `None` values from numerator and denominator, clamps total to `[0.0, 1.0]`, and returns `0.0` for empty present-weight denominator.
   - `compare_salience(left, right)` implements tie-break: higher `total`, higher Importance value, higher Urgency value, smaller `ClaimId`.

2. **Extractor**
   - `extract_salience_inputs(db, claim_id, now, actor_policy) -> SalienceInput`
   - reads the existing claim row and adjacent substrate only.
   - validates claim visibility against the caller policy for read ability use; system recompute may include quiet/suppressed recommendations for internal ranking refresh.
   - computes explicit typed inputs; any unavailable input produces a `value: None` factor, never an invented neutral score.

3. **Pure read orchestrator**
   - `score_salience(ctx, db, request) -> ScoreSalienceResult`
   - validates request schema version.
   - reads the latest persisted evaluation for the claim when available and actor-visible.
   - otherwise computes all ten factors as a non-persistent preview.
   - does not write to SQLite, emit signals, mutate claim/trust state, or update audit rows.
   - returns `SalienceScore` plus `claim_id`, `computed_at`, `schema_version = 1`, and `persistence = Stored { evaluation_id }` or `Preview`.

4. **Persistent service recompute**
   - `recompute_salience_for_claim(ctx, db, request) -> ScoreSalienceResult`
   - calls `ctx.check_mutation_allowed()` as its first public-service mutation gate.
   - validates request schema version.
   - computes all ten factors through the same extractor and pure scorer.
   - writes `salience_factors` rows in one transaction.
   - returns `SalienceScore` plus `evaluation_id`, `claim_id`, `computed_at`, and `schema_version = 1`.
   - is not registered as a Read ability and is not exposed to MCP/SurfaceClient in W1-B. W2-B trigger policy may call this service path when signals invalidate recommendation salience.

The persistent write is confined to W1-B's own audit/history table and is only reachable through app service code. It does not mutate `intelligence_claims`, trust columns, claim lifecycle, or source reliability. In `Simulate` or `Evaluate`, the mutation gate returns `WriteBlockedByMode` before any `salience_factors` rows are written.

## 7. Factor Mapping

| Factor | Existing substrate read | Value rule |
|---|---|---|
| Importance | claim `trust_score` -> `TrustBand`, `data_source` source authority | normalized blend of trust-band score and source authority |
| Novelty | durable canonicalization / semantic evidence rows already present (`canonicalization_decisions`, `claim_semantic_evidence`, same-subject claim metadata) | `1.0 - max_similarity`; `None` when durable similarity evidence is unavailable. W1-B does not add a vector primitive or scan raw claim text. |
| Urgency | `expires_at`, temporal scope, and recent signal age | nearer future deadline and recent signal evidence increase value |
| Timing | signal timestamps and meeting/calendar proximity where present | recent relevant signal + near relevant meeting increases value |
| UserFit | `claim_feedback` and recommendation metadata feedback state read through existing feedback rows only | confirmations/corrections increase fit; dismiss/not-useful/too-noisy reduce fit. For non-User/System read actors, value is `None`; callers cannot provide actor identity in the request. |
| Freshness | ADR-0105 freshness decay via existing trust freshness helpers | decay factor from `source_asof` fallback chain |
| Trust | existing trust band only | band mapping, read-only; no trust recompute |
| Corroboration | `claim_corroborations` count/strength | count/strength normalized to `[0,1]` |
| Contradiction | unresolved `claim_contradictions` edges | inverse penalty: unresolved contradictions lower salience |
| OpenLoopRelevance | open-loop/review/action membership rows and recommendation metadata relations | active open-loop/action/review relation increases value |

All rationale fields use W1-A `FactorRationale` variants. The salience engine constructs typed variants only; render-time prose is downstream.

## 8. Ability Surface

Add `abilities-runtime/src/abilities/recommendations/salience.rs` with the registered read ability:

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSalienceRequest {
    pub schema_version: u32,
    pub claim_id: ClaimId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSalienceResponse {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub computed_at: DateTime<Utc>,
    pub persistence: SaliencePersistence,
    pub salience: SalienceScore,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SaliencePersistence {
    Preview,
    Stored { evaluation_id: String },
}
```

Because of the crate boundary, `ClaimId`, `SalienceScore`, `SalienceFactor`, `SalienceFactorKind`, and `FactorRationale` are mirrored in the ability module with the same serde shapes as W1-A. The app adapter converts between ability DTOs and service DTOs and has a golden parity test to catch drift.

`ScoreSalienceRequest` does not include `for_actor` or `include_quiet`. Actor authority comes from `AbilityContext` / registry bridge, and quiet/suppressed inclusion is reserved for the internal system recompute path. The read ability validates visibility for the calling actor and returns `AbilityUnavailable`/privacy-drop equivalents rather than leaking whether an arbitrary quiet claim exists.

Policy:

- `category = Read`
- `version = "1.0.0"`
- `schema_version = 1`
- `allowed_actors = [User, System]`
- `allowed_modes = [Live, Evaluate]`
- `required_scopes = ["read.recommendations"]`
- `mcp_exposure = None`
- `client_side_executable = false`
- `may_publish = false`
- `requires_confirmation = false`

The ability validates request schema version and fails closed on unknown versions.

## 9. Intelligence Loop Fit

1. **Claim model:** Salience is claim-adjacent derived state, not a new claim. It is keyed by claim ID and records factors used to rank recommendation candidates.
2. **Provenance + trust:** Reads provenance/trust inputs but does not alter trust. Factor rationale is typed and inspectable.
3. **Signals + invalidation:** W1-B exposes persistent recompute as a callable service. W2-B owns trigger policy and signal wiring.
4. **Runtime + surfaces:** W2 surfacing consumes ranked candidates; W3 renders factor/why-this-now breakdowns. W1-B registers a non-mutating Tauri/System read ability; direct MCP/SurfaceClient invocation is deferred to the v1.4.7 MCP wrapper work.
5. **Feedback loop:** W1-B reads current feedback history for UserFit. W4-A owns future weight tuning and recommendation feedback mutations.

## 10. Tests

Required local tests:

- Pure aggregation:
  - all ten weights sum to 1.0;
  - `None` factors are skipped, not zeroed;
  - empty denominator returns `0.0`;
  - clamping covers out-of-range fixture inputs.
- Factor fixtures:
  - routine;
  - unusual/novel;
  - urgent;
  - stale;
  - contradicted;
  - user-corrected/user-dismissed;
  - low-trust but high-salience vs high-trust background to prove salience is not trust.
- Storage:
  - v270 creates both tables and seeded weights;
  - seeded weight count is exactly 10 and sum is `1.0 +/- 0.0001`;
  - partial v270 application followed by rerun succeeds and does not duplicate or drift weights;
  - persistent recompute writes one row per factor per evaluation;
  - persistent recompute in `Simulate`/`Evaluate` returns `WriteBlockedByMode` and writes no rows;
  - privacy check asserts rationale JSON contains no raw claim text, path-like strings, prompt/output fields, or source body text.
- Ability:
  - descriptor registered with version `1.0.0`, schema version 1, `allowed_modes = [Live, Evaluate]`, and `read.recommendations`;
  - descriptor is `Read`, has empty derived `mutates`, `mcp_exposure = None`, `client_side_executable = false`, and is hidden from MCP/SurfaceClient enumeration in W1-B;
  - unknown schema version rejected;
  - request has no caller-supplied actor or quiet/suppressed override;
  - ability DTO golden JSON matches app service DTO golden JSON.
- CI invariants:
  - no LLM/provider imports or calls in `services/recommendations/salience.rs`;
  - no raw string rationale assembly in `salience.rs`.

Full lane validation before PR:

```bash
cargo test --manifest-path src-tauri/Cargo.toml recommendations::salience
cargo test --manifest-path src-tauri/Cargo.toml migration_270
cargo test --manifest-path src-tauri/Cargo.toml --package abilities-runtime recommendations::salience
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm tsc --noEmit
```

## 11. Risks and Open Decisions

| Risk | Decision / mitigation |
|---|---|
| Ability runtime cannot import W1-A service DTOs | Mirror wire DTOs in ability module and add golden parity tests. |
| Adding an ability read handle touches `ServiceContext` outside the original owned-file list | This is required by the existing ability pattern. Keep it narrow and read-only; no bridge semantic changes. Persistent recompute remains an app service call, not a read handle. |
| Novelty may be unavailable if durable semantic/canonicalization evidence is not ready in a test DB | Represent as `value: None` and skip from aggregation. Do not add a new vector primitive or raw-text similarity scan in W1-B. |
| Freshness helper uses clock-sensitive inputs | Use injected `ServiceContext.clock.now()`; pure aggregation takes precomputed factor values. |
| Read ability versus inspectable rows | Split paths. The read ability returns non-persistent preview salience. `recompute_salience_for_claim` persists rows through app service code and satisfies the `salience_factors` acceptance criterion. |
| External callers could infer private feedback/quiet claim state | W1-B allows only User/System actors, removes caller-supplied actor identity, does not expose MCP/SurfaceClient invocation, and returns `None` for UserFit where actor policy cannot safely personalize. |
| `salience_factors_weights` name is awkward | Use the exact wave-plan name for continuity. |

## 12. Done When

- L0 reviews pass unanimously.
- `score_salience` computes all ten factors without LLM/provider calls.
- `salience_factors` and `salience_factors_weights` exist with seeded defaults.
- Read ability returns schema-versioned non-persistent preview salience without mutations.
- Service recompute path persists one row per factor per evaluation.
- Tests cover routine, unusual, urgent, stale, contradicted, user-corrected/dismissed, deterministic tie-break, and salience-not-trust separation.
- `cargo clippy -- -D warnings`, relevant `cargo test`, and `pnpm tsc --noEmit` pass.
