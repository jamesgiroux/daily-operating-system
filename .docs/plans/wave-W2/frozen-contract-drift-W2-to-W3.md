# W2 → W3 Frozen-Contract Drift Report

**Date:** 2026-04-30
**Author:** Claude (drafted while W2-A is in-flight; codex L0 plan drafts running in parallel for W3-A through W3-H)
**Purpose:** W2 merge-gate artifact per `.docs/plans/v1.4.0-waves.md` §"W2 merge gate" — *"Frozen-contract verification: confirms W3 ticket text matches current substrate state — drift report attached."*
**Status:** PRELIMINARY — W2-A (DOS-209) has not yet merged. This report cross-references W2-A's frozen plan (`wave-W2/DOS-209-plan.md` v3) + W2-B's merged trait (commit `fe14839c`) against the 9 W3 ticket bodies pulled from Linear on 2026-04-30. **Re-run on actual W2-A merge** to swap in real APIs for plan-text APIs.

## Methodology

For each W3 agent slot, cross-reference the Linear ticket's structural assumptions against the concrete shapes pinned in W2 plans. Three categories of finding:

- ✅ **Aligned** — ticket assumption matches W2 frozen contract.
- ⚠️ **Watch** — ticket text is silent or compatible but the W3 plan must explicitly confirm in §3 Key decisions.
- ❌ **Drift** — ticket text contradicts the W2 frozen contract; requires either ticket amendment or W3 plan §10 escalation.

## W2 frozen surfaces (snapshot)

From `wave-W2/DOS-209-plan.md` v3 (cycle 3, L6-authorized) + `wave-W2/DOS-259-plan.md` v3 + commit `fe14839c`:

**`ServiceContext`** (W2-A, in flight) — fields: `db` (private), `signals` (private), `intel_queue` (private), `mode` (pub read), `clock: &dyn Clock` (pub read), `rng: &dyn SeededRng` (pub read), `external: ExternalClients` (pub read, mode-aware), `tx: Option<TxHandle>` (private).

**Constructors** — `new_live`, `new_simulate`, `new_evaluate`, plus `test_live()` under `#[cfg(test)]` only. No `Default`. No zero-arg.

**`check_mutation_allowed()`** — `Err(WriteBlockedByMode)` outside Live; first line of every mutation in `services/`.

**Transaction API** — primary: `with_transaction_async(ctx, |tx_ctx| async { ... })` with HRTB closure. Fallback: sync closure executed inside the SQLite writer lane from async callers (no `.await` inside transaction body). Nested calls return `NestedTransactionsForbidden`. `TxCtx` exposes transaction-scoped DB writes, mode, clock, rng, signal staging — but **no external clients, no IntelligenceProvider**.

**Capability boundary (DOS-304 disposition)** — capability handles ARE the boundary. `ctx.services.db()` returns scoped read capability with no `conn_ref`, no raw SQL, no write verbs. Raw `ActionDb` is denied to ability-facing code by construction. The registry macro from DOS-210 is lint/metadata/trybuild coverage only — not the enforcement boundary. **This closes DOS-304 in W2-A; W3-A inherits this as a fact, not a question.**

**`IntelligenceProvider` trait (W2-B, merged `fe14839c`)** — `Send + Sync`; `complete(prompt: PromptInput, tier: ModelTier) -> Result<Completion, ProviderError>`. `Completion` carries `text: String` + `FingerprintMetadata`. `ProviderKind = ClaudeCode | Ollama | OpenAI | Other(&'static str)`. Glean is `Other("glean")`. Replay is `Other("replay")` and only available under `#[cfg(test)]` or Evaluate-mode wiring.

**`select_provider(ctx: &AbilityContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`** (W2-B) — single source of provider selection. Reads `ServiceContext.execution_mode` for routing only; everything else AbilityContext-owned. **Critical:** AbilityContext does not exist yet — owned by W3-A/DOS-210. Until then, callers route via the **AppState-owned provider Arc** per ADR-0091. W3-A landing migrates `intel_queue.rs` and `services/intelligence.rs` callers off the bridge onto `select_provider(ability_ctx, tier)`.

**`PlannedMutationSet`, `PlannedMutation`, `ProvenanceRef`, `plan_*` naming** — explicitly **deferred** out of W2-A. Lands in W3-A (registry surface) with `ProvenanceRef` supplied by W3-B before W4-C consumes the bridge. W3-A's plan should confirm intent here.

**CI invariants active at W2 merge** — services-only mutations; no `Utc::now()` / `thread_rng()` in `services/` or `abilities/`; Intelligence Loop 5-question check on schema PRs.

---

## Per-agent drift matrix

### W3-A — DOS-210 (Ability Registry + #[ability] macro)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `AbilityContext` wraps `ServiceContext` | "wraps `ServiceContext` with actor, tracer, confirmation token" | ServiceContext frozen per shape above; provider Arc lives on AbilityContext, NOT ServiceContext | ✅ |
| `#[ability(category = …)]` AST inspection | 2026-04-20 decision: compile-time AST inspection of `services::*` mutation calls | mutation taxonomy & catalogue locked in W2-A `dos209_mutation_catalog.txt` (committed before W3) | ⚠️ — W3-A plan must reference the committed catalogue as the macro's allowlist seed; do not hand-roll a new list |
| `composes` DAG cycle detection | trybuild + property test 100 random DAGs + 100 cyclic | no W2 contract; W3-A owns | ✅ |
| `experimental = true` flag | bypasses category enforcement, provenance requirements, fixture requirements for one cycle | no W2 contract | ✅ |
| `PlannedMutationSet` / `ProvenanceRef` | not in DOS-210 ticket text | W2-A defers to W3-A | ⚠️ — W3-A plan must explicitly own these and confirm landing in this PR or in DOS-211/W3-B coordination |
| Capability handles vs registry as enforcement boundary | DOS-210 ticket implies registry macro is the enforcement | W2-A closed: capability handles ARE the boundary; registry is lint/metadata/trybuild only | ❌ — **drift on framing.** W3-A plan must restate the registry as a structural-redundancy lint over the capability boundary, not as the boundary itself. Surface in §3 Key decisions. |

### W3-B — DOS-211 (Provenance Envelope + Builder)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `Provenance.thread_ids: Vec<ThreadId>` | DOS-296 adds it — see DOS-296 row | landing-coordination concern with W3-F | ⚠️ — see W3-F |
| `SourceAttribution.source_asof` | required by DOS-299 | DOS-211 owns the envelope shape | ⚠️ — W3-B + W3-G must agree on field placement before W3-G's backfill code lands |
| `prompt_fingerprint` field | "per ADR-0106" | W2-B `FingerprintMetadata` is the producer side; envelope holds the projection | ✅ — confirm in §3 |
| Trust assessment shape | computed not author-set | aligns with W2-B's deferral of cost/latency from `Completion` | ✅ |
| `composition_id` stable per `composes` entry (NOT `child_idx`) | ticket lock | W2 silent; W3-A's registry needs to emit stable `composition_id` | ⚠️ — coordination with W3-A needed |
| `provenance_schema_version = 1` | ADR-0105 §1 forward-compat | W2 silent | ✅ |

### W3-C — DOS-7 (intelligence_claims + 9-mechanism consolidation)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `services/claims.rs::commit_claim` is the only writer | ticket lock | aligns with services-only-mutations CI invariant active at W2 merge | ✅ |
| `Utc::now()` not in claim writes; clock injected via `ServiceContext` | ticket DoD | W2-A frozen — `clock: &dyn Clock` injected, lint rejects new `Utc::now()` in services/abilities | ✅ |
| Transaction shape: `db.with_transaction(\|tx\| ...)` (sync, in DOS-301 code snippets) vs W2-A's `with_transaction_async` (HRTB) | DOS-7 + DOS-301 ticket bodies use sync `with_transaction` in pseudo-Rust | W2-A primary API is `with_transaction_async`; sync fallback is "sync-within-async" only when HRTB slips | ⚠️ — **clarify in W3-C plan §3 which API**. The sync-flavored snippet is illustrative; commit_claim is async-callable from Tauri commands, so async transaction is the right surface. Codex's DOS-7 draft already noted this as Open Question #3 (migration hook placement) — fold the answer here. |
| `is_suppressed()` returns `SuppressionDecision` enum with `Malformed` variant | DOS-308 cycle-2 absorption into W3-C | W2 silent; substrate change in W3 | ✅ |
| Hard-delete CHAIN refactor on `account_stakeholder_roles` | replace `set_team_member_role` and `remove_account_team_member` hard deletes with tombstone claim writes | W2-A mutation catalogue includes both sites with `D+TX+SIG` tags — they will receive `ctx.check_mutation_allowed()?` first; W3-C replaces the body content | ✅ — confirm in §7 that the W2-A catalogue addition + W3-C body replacement is sequenced correctly (W2-A merges first, W3-C rebases) |
| `claim_corroborations.strength` math (amendment A): noisy-OR aggregate, log-diminishing same-source reinforcement | DOS-7 amendment A | codex DOS-7 draft Open Question #2 surfaced a formula/example mismatch (saturates 0.5→1.0 on first reinforcement, prose expects ~0.7) | ❌ — **drift in ticket internal consistency.** Not W2-vs-W3 drift but a contract bug. Surface to L6 before W3-C codes. |
| Amendment C `winner_claim_id BLOB` vs project pattern of TEXT UUIDs | DOS-7 amendment C SQL snippet | codex DOS-7 draft Open Question #1 | ❌ — **drift in ticket SQL vs project pattern.** L6 escalation candidate. |
| `work_tab_actions.dismissed_at` (mechanism 7) source mismatch | DOS-7 names mechanism 7 as `work_tab_actions.dismissed_at` | codex DOS-7 draft Open Question #6: live migration 108 shows `nudge_dismissals`, not `work_tab_actions` | ❌ — **drift between ticket and live schema.** L6 candidate before backfill code lands. |

### W3-D — DOS-301 (derived-state writers)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `services/derived_state.rs` is sole writer of legacy AI surfaces | ticket lock + CI invariant | aligns with services-only-mutations | ✅ |
| Sync DB projection inside transaction; sync-best-effort file POST-DB-commit | ticket final design | W2-A `with_transaction_async` supports sync work via `tx_ctx`; file projection runs after the tx-ctx closure returns Ok | ✅ |
| Per-rule SAVEPOINT inside the tx | ticket failure-isolation requirement | W2-A `TxCtx` exposes raw transactional writes; SAVEPOINT is a SQLite primitive, not a TxCtx method — W3-D may need to extend `TxCtx` with `savepoint(&str)` helper | ⚠️ — W3-D plan must call out whether `TxCtx::savepoint` lands in W2-A's frozen surface or in W3-D's own scope |
| `validators/json_columns.rs` shared between propose_claim and `db/accounts.rs:1205-1212` | ticket lock | W2-A catalogue lists `accounts::update_account_field_inner` and others writing those columns; the validators are pure value-checks, not stateful, no W2 dependency | ✅ |
| Mode-awareness: Simulate/Evaluate write nothing to legacy | ticket lock | aligns with W2-A `check_mutation_allowed()` returning WriteBlockedByMode | ✅ |
| `claim_projection_status` table | ticket lock | W2 silent; substrate add | ✅ |

### W3-E — DOS-294 (typed claim feedback)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `FeedbackAction` enum + state machine (active → contested → needs_user_decision terminal) | ticket lock + DOS-307 fold-in | W2 silent | ✅ |
| `claim_feedback` mutation API in `services/claims.rs` | ticket lock | services-only-mutations active | ✅ |
| Feedback render policy stub: `needs_user_decision` honoured by render path | ticket DoD | W3-D + DOS-320 (W6) own render | ⚠️ — W3-E plan should pin the test stub location; production render is W6's |

### W3-F — DOS-296 (thread_id substrate)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `Provenance.thread_ids: Vec<ThreadId>` additive field | ticket lock; `provenance_schema_version` stays at 1 | DOS-211 (W3-B) owns the envelope; this lands inside W3-B's initial schema rather than as a separate addition | ⚠️ — W3-F + W3-B must coordinate so `thread_ids` lands in W3-B's first schema version, not an immediate ALTER |
| `thread_id TEXT NULL` column on `intelligence_claims` | ticket lock | DOS-7 (W3-C) writes the migration first; W3-F adds the column on top | ⚠️ — migration numbering coordination |
| `dedup_key` does NOT include `thread_id` | ticket lock | aligns with DOS-7 dedup formula | ✅ |

### W3-G — DOS-299 (source_asof + freshness fallback)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `FreshnessContext { timestamp_known: bool }` extension | ADR-0114 R1.3 small extension lands here | W2 silent; trust-input prep is `src-tauri/src/scoring/extract/trust.rs` territory | ✅ |
| `LegacyUnattributed` `DataSource` variant | per ADR-0107 amendment | W2-B taxonomy is in `intelligence/provider.rs::ProviderKind` — distinct from `DataSource` taxonomy in `abilities/provenance/` (DOS-211) | ⚠️ — W3-G plan must explicitly cite `DataSource` (provenance-side) vs `ProviderKind` (provider-side) to avoid terminology bleed |
| Audit + normalize ~10 `Utc::now()` write paths | ticket DoD | aligns with W2-A no-`Utc::now()`-in-services lint | ✅ |
| Backfill ≥95% coverage | ticket DoD | W1-B fence + DOS-308 quarantine table both required upstream | ✅ |
| Trust factor count = 5 canonical (per ADR-0114 R1.4) + 1 composer-local helper | ticket explicit lock | W2 silent | ✅ |

### W3-H — DOS-300 (temporal_scope + sensitivity + claim_type registry)

| Surface | Ticket text | W2 contract | Status |
|---|---|---|---|
| `TemporalScope` enum + `ClaimSensitivity` enum + `CLAIM_TYPE_REGISTRY` const slice | ticket lock per ADR-0125 | W2 silent | ✅ |
| Compile-time exhaustiveness check (`cargo test --test claim_type_registry_exhaustiveness`) | ticket lock | W2 silent | ✅ |
| Pattern mirrors ADR-0115 Signal Policy Registry exactly | ticket lock | aligns with active CI invariants | ✅ |
| Defaults applied at claim-write time when row leaves field at default | ticket lock | requires `commit_claim` (DOS-7) to perform the lookup | ⚠️ — W3-H plan must coordinate so DOS-7's `commit_claim` body calls into the registry; cannot be additive after DOS-7 lands |

---

## Cross-cutting observations

### Drift requiring L6 attention before W3 codes

1. **DOS-7 amendment C `winner_claim_id BLOB` vs project pattern TEXT UUIDs.** Internal contract bug, not W2-vs-W3 drift. Surface immediately.
2. **DOS-7 amendment A corroboration formula vs prose example.** Internal contract bug. Surface immediately.
3. **DOS-7 mechanism 7 source table mismatch (`work_tab_actions.dismissed_at` vs live `nudge_dismissals` in migration 108).** Real-code-vs-ticket drift. Surface immediately.
4. **DOS-210 framing of registry as enforcement boundary vs W2-A's capability-handle resolution.** Not a code conflict, but the ticket text reads as if the registry IS the boundary. W3-A plan must restate; if reviewers find the ticket text load-bearing on the original framing, escalate.

### Coordination pre-flight (no drift, but explicit handshakes required in W3 plans)

1. **W3-A registers `PlannedMutationSet` + `ProvenanceRef`** — W2-A explicitly defers; W3-A must own.
2. **W3-B + W3-F land `thread_ids` in initial schema** — not as an ALTER on the same wave.
3. **W3-B + W3-G agree on `SourceAttribution.source_asof` field placement** — before W3-G's backfill code rebases on W3-B.
4. **W3-C + W3-D agree on migration numbering** + the `TxCtx::savepoint(&str)` helper (whether W3-D extends W2-A's TxCtx or works around it).
5. **W3-C + W3-H land registry-driven defaults inside `commit_claim`** — W3-H cannot ship an ALTER after W3-C.

### CI invariants that activate at W2 merge

These will start failing PRs immediately on W2-A merge, so all 8 W3 plans should pre-declare compliance in §6 Coding standards:

- Services-only mutations (no direct DB writes from commands) — clippy lint + grep CI test.
- No `Utc::now()` or `thread_rng()` in `services/` or `abilities/` — clippy lint.
- Intelligence Loop 5-question check on PRs touching schema — PR template + CI bot comment.

### W3-specific CI invariants that must land in the same wave

These activate at W3 merge, NOT before. W3 plans should call them out in §9 Test evidence:

- `commit_claim` is the only writer to `intelligence_claims` (W3-C grep CI test).
- `record_corroboration` / `reconcile_contradiction` are the only writers to corroboration/contradiction tables (W3-C).
- No `DELETE FROM intelligence_claims | claim_corroborations | claim_contradictions` (W3-C).
- Immutability allowlist on `intelligence_claims` columns (W3-C amendment D, trybuild + clippy).
- `services/derived_state.rs` is the only writer to legacy AI surfaces (W3-D).
- Hard-delete CHAIN refactored — no bare DELETE on `account_stakeholder_roles` outside service (W3-C).
- No `claim_type` with Global subject (W1 trybuild compile-time, but verifies post-W3-H exhaustiveness).

## Recommendation

1. **Surface DOS-7 ticket bugs (C-1, A-1, mechanism-7) to L6 before W3-C plan enters L0 review.** Ticket text amendment is faster than running W3-C's L0 triangle on a contradictory contract.
2. **Re-run this drift report against the actual W2-A merged code** once W2-A lands. Plan-text APIs (especially `with_transaction_async` HRTB shape) may differ from final.
3. **W3 L0 reviews should treat this report as a checklist input** — every drift row above ⚠️ must be addressed in the agent's plan §3 or §7 before the L0 triangle approves.

## Frozen contracts ready to consume in W3

- ✅ `ServiceContext` struct + constructors + `check_mutation_allowed()` (post-W2-A merge)
- ✅ `Clock` + `SeededRng` traits (post-W2-A merge)
- ✅ `ExternalClients` (post-W2-A merge)
- ✅ `with_transaction_async` (post-W2-A merge; sync fallback acceptable per ticket)
- ✅ `IntelligenceProvider` trait + `Completion` + `FingerprintMetadata` + `select_provider` (W2-B merged at `fe14839c`)
- ✅ `AppState`-Arc bridge for early callers (W2-B; deprecated when W3-A lands AbilityContext)

Not ready / W3-owned:
- 🔧 `AbilityContext` — owned by W3-A
- 🔧 `Provenance` envelope — owned by W3-B
- 🔧 `services/claims.rs` — owned by W3-C
- 🔧 `services/derived_state.rs` — owned by W3-D
- 🔧 `validators/json_columns.rs` — owned by W3-D
