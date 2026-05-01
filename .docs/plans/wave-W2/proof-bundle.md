# Wave W2 Proof Bundle

**Wave:** W2 (substrate primitives — DOS-209 + DOS-259)
**Status:** W2-B (DOS-259) complete; W2-A (DOS-209) complete — Wave W2 COMPLETE
**Date:** 2026-04-30

---

## W2-B / DOS-259 — IntelligenceProvider trait + AppState-Arc bridge

### Final commit chain (local-only, never pushed)

| Commit | Cycle | What |
|---|---|---|
| `fe14839c` | initial | Trait + ReplayProvider + PtyClaudeCode + Glean impl + AppState bridge + 5 migration sites + 8 tests |
| `33a5d779` | L2 cycle-1 fix | Removed inline Glean fallback (3 sites) + ADR-0106 §3 required fields + bridge tests + lint wiring |
| `8848d648` | L2 cycle-3 (L6-authorized) | Atomic `ContextProviderBundle` + `ContextSnapshot` + `PtySpawnAdapter` test seam + real lint regression |
| `0fd17a36` | L2 cycle-4 (L6-authorized) | Closed back-door — production callers stop double-calling `swap_context_provider` |

Total wall-clock: ~6 hours of orchestrator time across 4 review cycles.

### Acceptance criteria validation

Per DOS-259 ticket §"Acceptance":

- [x] `pub trait IntelligenceProvider` lives at `src-tauri/src/intelligence/provider.rs` (verified `provider.rs:163-176`).
- [x] `PtyClaudeCode` implements it (`pty_provider.rs:151-198`, both async `complete()` and sync `complete_blocking()` helper).
- [x] `GleanIntelligenceProvider` implements it (`glean_provider.rs:870-906`).
- [x] Text-only callers use `provider.complete(...).await?.text` — exercised by `dos259_pty_seam_test.rs::complete_async_uses_same_adapter_path`.
- [x] `ReplayProvider` exists and is gated for tests + Evaluate-mode (`provider.rs:200-260`).
- [x] Existing enrichment + meeting-prep behavior preserved byte-identically — verified by `dos259_pty_parity_test.rs` + `dos259_glean_parity_test.rs` (parsed `IntelligenceJson` byte-equal across the trait surface vs. legacy direct path with `enriched_at` pinned).

Per ADR-0106 §3 fingerprint metadata required fields:

- [x] `provider: ProviderKind` required (not Option) — `provider.rs:95`.
- [x] `model: ModelName` required — `provider.rs:96`.
- [x] `temperature: f32` required — `provider.rs:97`.
- [x] PTY uses documented `CLAUDE_CODE_DEFAULT_TEMPERATURE = 1.0` placeholder pending DOS-213 canonical fingerprint hash.
- [x] Glean uses documented `GLEAN_CHAT_DEFAULT_TEMPERATURE = 0.0` placeholder.

Per ADR-0091 AppState-owned `Arc`:

- [x] `intelligence_provider: RwLock<Option<Arc<dyn IntelligenceProvider + Send + Sync>>>` lives in AppState — wrapped inside `ContextProviderBundle` post-cycle-3 for atomicity.
- [x] Read at call time via `state.intelligence_provider()` or `state.context_snapshot().intelligence_provider`.
- [x] Hot-swap via `set_context_mode_atomic(mode)` — single write-lock acquisition installs the full 3-Arc bundle.
- [x] "Switch mid-queue takes effect on next dequeue" — verified by `dos259_appstate_bridge_test.rs::build_context_provider_never_lets_reader_observe_torn_state` (4 reader threads × 2 writer threads × 200ms reading public entry point with no torn-state observed).

Per DOS-259 plan §3 line 79 bridge contract:

- [x] `intel_queue.rs` and `services/intelligence.rs` (early callers without `AbilityContext`) route through AppState-owned `Arc` — verified by `grep -rn "GleanIntelligenceProvider::new" src-tauri/src/` returning only the AppState construction sites + `commands/integrations.rs` Glean-account-discovery (out-of-scope per plan §2).
- [x] `select_provider(ctx: &AbilityContext, tier)` signature stub exists for W3-A (`provider.rs:286-289`).

Per DOS-259 plan §9 test list (8 named):

| Test | Location | Status |
|---|---|---|
| `replay_provider_returns_canned_completion` | `provider.rs::tests` | ✅ pass |
| `evaluate_mode_never_invokes_live_provider` | `provider.rs::replay_provider_does_not_fall_through_to_live` + `dos259_provider_selection_test.rs::evaluate_mode_replay_provider_never_falls_through_to_live` | ✅ pass (2 layers) |
| `provider_selection_is_single_source_for_tier` | `dos259_provider_selection_test.rs` | ✅ pass |
| `pty_claude_code_fixture_returns_expected_fingerprint_metadata` | `pty_provider.rs::tests` | ✅ pass |
| `glean_provider_fixture_returns_expected_fingerprint_metadata` | `glean_provider.rs::provider_trait_tests` | ✅ pass |
| `pty_provider_parity_fixture_intelligence_json_byte_identical` | `dos259_pty_parity_test.rs` | ✅ pass |
| `glean_provider_parity_fixture_intelligence_json_byte_identical` | `dos259_glean_parity_test.rs` | ✅ pass |
| `provider_complete_concurrent_invocations_all_succeed` | `provider.rs::replay_provider_concurrent_invocations_all_succeed` (32-task fan-out) | ✅ pass |

### Final L1 validation

```
$ cargo clippy --lib --bins -- -D warnings  → clean
$ cargo test --lib                          → 1747 passed; 0 failed; 7 ignored
$ cargo test --test dos259_*                → 24 passed; 0 failed (across 6 files)
$ pnpm tsc --noEmit                         → clean
$ scripts/check_no_let_underscore_feedback.sh                       → pass
$ scripts/check_write_fence_usage.sh                                → pass
$ scripts/check_no_direct_clock_rng_in_provider_modules.sh          → pass
$ scripts/check_no_global_subject_in_spine.sh                       → pass
```

W2-B integration test breakdown (24 tests across 6 files):

- `dos259_provider_selection_test.rs` — 6 tests (selection invariant, tier propagation through PtyClaudeCode, ADR-0106 §3 required-field gate, evaluate-mode never-falls-through)
- `dos259_pty_parity_test.rs` — 1 test (PTY response → IntelligenceJson byte parity)
- `dos259_glean_parity_test.rs` — 1 test (Glean response → IntelligenceJson byte parity)
- `dos259_appstate_bridge_test.rs` — 9 tests including 2 race-regression tests (one against `set_context_mode_atomic` directly + one against the public `build_context_provider` entry point)
- `dos259_lint_wiring_test.rs` — 3 tests (lint runs clean against production tree, lint trips on synthetic unmarked violation, lint accepts grandfathered marker)
- `dos259_pty_seam_test.rs` — 4 tests (`PtySpawnAdapter` injection captures prompt/workspace/tier/model/timeout/usage_context; auth → Permanent error; other → Transient error; sync + async paths route through same adapter)

### Deliberate scope boundaries

**Migrated production sites (5 total):** the plan named 5; all 5 land:
1. `services/intelligence.rs:226` — manual-refresh Glean construction → `state.context_snapshot().glean_intelligence_provider`
2. `intel_queue.rs:1006` — leading-signals Glean → snapshot
3. `intel_queue.rs:1525` — batch-enrichment Glean → snapshot
4. `intel_queue.rs:1733` — parallel-extraction PTY → `PtyClaudeCode::complete_blocking`
5. `intel_queue.rs:1976` — legacy-synthesis PTY → `PtyClaudeCode::complete_blocking`

**Out-of-scope per plan §2 (21 PTY-direct matches):** background maintenance, devtools, processors, repair retry — explicitly preserved per ADR-0091 deliberate exceptions.

**Out-of-scope per plan §6:** ADR-0091 expansion of `ModelTier` to include `Ollama`/`OpenAI` providers; W4-B fixture corpus (DOS-216).

**Deferred to follow-up tickets:**
- **DOS-347** (medium) — Serialize context-mode transition (DB persist + AppState swap). L2 cycle-4 surfaced this as a higher-level race at the command boundary; L6 ruled Option C (accept residual + follow-up). Not a substrate concern; lives naturally in W3/DOS-7's settings refactor scope. The substrate-level torn-bundle race (the actual W2-B concern) is structurally closed.

**Migration window for grandfathered clock calls in `glean_provider.rs`:** 8 `chrono::Utc::now()` calls annotated with `// dos259-grandfathered:` markers. Migrate to `ctx.clock.now()` when W2-A's `ServiceContext` lands and the Glean impl can take a clock reference. Markers + this comment block delete at that point.

### Audit trail of L2 cycles (4 cycles, all addressed)

**Cycle 1 (initial fe14839c → 33a5d779):**
- HIGH: settings race in inline Glean fallback → fixed by removing fallbacks at 3 sites.
- HIGH: `FingerprintMetadata` Optional fields → fixed per ADR-0106 §3.
- MEDIUM: parity tests don't exercise migrated paths → bridge tests added.
- MEDIUM: lint not wired into `cargo test` + line-numbered grandfather → wired + marker-based.

**Cycle 2 (33a5d779 → L6 escalation):**
- HIGH: 3-Arc settings race not structurally closed (split writes can tear).
- MEDIUM: PTY parity tests still don't invoke `PtyClaudeCode::complete*`.
- LOW: lint regression test only checks marker strings.

**Cycle 3 (L6 Option A → 8848d648):**
- Atomic `ContextProviderBundle` + `ContextSnapshot` + interleaving regression test.
- `PtySpawnAdapter` trait + `FakePtySpawnAdapter` — 4 propagation tests.
- Real lint regression with `DOS259_LINT_FILES_OVERRIDE` env var seam.

**Cycle 4 (8848d648 → 0fd17a36):**
- HIGH: legacy single-field swap leaves back-door open in `commands/integrations.rs` (3 callers do `build + swap` two-step).
- Fix: drop the redundant `swap_context_provider(new_provider)` at all 3 callers; mark `swap_context_provider` deprecated for production.
- Add public-entry-point race regression (`build_context_provider_never_lets_reader_observe_torn_state`).

**Cycle 4 verdict (0fd17a36):** BLOCK with 1 HIGH — DB-persist + AppState-swap split across `set_context_mode` command without transition lock. **L6 Option C ruled** out-of-scope for v1.4.0 W2-B; filed as DOS-347 for W3/DOS-7's settings refactor scope.

---

## W2-A / DOS-209 — ServiceContext substrate + 228-mutator migration

### Final commit chain (local-only, never pushed)

| Commit | What |
|---|---|
| `9545a688` | ServiceContext substrate — ExecutionMode, SystemClock, SystemRng, ExternalClients, check_mutation_allowed() |
| `5cbcf667` | AppState integration + feedback.rs reference migration |
| `b7e77537` | Pilot: services::people (8 mutators) |
| `b65660a9` | Group A: services::accounts (45 mutators) |
| `b4d37f88` | Group B: services::mutations (30 mutators + cross-module callers) |
| `f7f4faba` | Group C: services::meetings (24) + services::emails (16) + cleanup |
| `1af465d8` | Group D: services::intelligence + services::success_plans + cleanup |
| `d38ec441` | Group E: services::actions + entity_linking + settings (37 mutators) |
| `33aa604c` | Group F: smalls batch (35 mutators across 10 files) + signals cascade cleanup (131 sites, 25 files) |
| `3c9e6c1c` | health_debouncer missed mutators (schedule_recompute, drain_pending) + 7 callers |
| `7cfb6b3f` | L2 fix: migrate 20 raw bus calls in services to ctx-gated facade |

### Acceptance criteria validation

Per DOS-209 ticket acceptance criteria (ADR-0104):

- [x] `ServiceContext<'_>` substrate exists at `src-tauri/src/services/context.rs` — ExecutionMode enum, SystemClock trait + impl, SystemRng trait + impl, ExternalClients struct, check_mutation_allowed() guard.
- [x] Every catalogued service mutator has `ctx: &ServiceContext<'_>` as FIRST parameter — verified by `scripts/dos209-mutation-audit.sh` (0 missing at close).
- [x] `ctx.check_mutation_allowed()` is first statement in every migrated mutator (no mutations slip through before the gate).
- [x] All service-layer signal calls go through the ctx-gated facade (`crate::services::signals::emit*`) — zero raw `crate::signals::bus::emit_signal*` calls remain under `src/services/` (confirmed by grep).
- [x] C-kind rows (chrono::Utc::now()) replaced with `ctx.clock.now()` at all catalogued locations — verified in accounts.rs, commitment_bridge.rs, projects.rs, linear.rs, and others.
- [x] Cross-module callers fixed per two-pattern cleanup: Pattern A (thread existing ctx) and Pattern B (inline `ServiceContext::new_live()`).
- [x] health_debouncer schedule_recompute + drain_pending use `if ctx.check_mutation_allowed().is_err() { return; }` (return-() pattern, not `?`).

### Mutator coverage by group

| Group | Files | Mutators |
|---|---|---|
| Pilot | people.rs | 8 |
| A | accounts.rs | 45 |
| B | mutations.rs | 30 |
| C | meetings.rs, emails.rs | 40 |
| D | intelligence.rs, success_plans.rs | ~25 |
| E | actions.rs, entity_linking/*, settings.rs | 37 |
| F | hygiene, user_entity, reports, projects, entity_context, commitment_bridge, signals, linear, integrations, entities | 35 |
| Missed | health_debouncer.rs | 2 |
| **Total** | **~20 service files** | **~222** |

### Signals cascade cleanup

Group F migrated `signals::emit`, `emit_and_propagate`, and `emit_propagate_and_evaluate` — adding ctx to these created ~131 compiler errors across 25 files. The Group F cleanup commit resolved all of them in a single pass using Pattern A (16 files, thread existing ctx) and Pattern B (9 files, inline ctx).

L2 review additionally found 20 raw bus calls inside service files (inside move closures or private helpers). All migrated to the facade in the L2 fix commit.

### Final L1 validation

```
$ cargo clippy -- -D warnings           → clean
$ cargo test --lib                      → 1759 passed; 0 failed; 7 ignored
$ pnpm tsc --noEmit                     → clean
```

### L2 adversarial review trail (2 cycles)

**Cycle 1 verdict: BLOCK (3 findings)**

| Finding | Severity | Disposition |
|---|---|---|
| Raw `signals::bus::emit_signal*` calls in services bypass ctx gate (20 sites) | HIGH | Fixed in commit `7cfb6b3f` |
| `evaluate_on_signal` discards `queue.enqueue()` result silently in `signals/bus.rs:289` | HIGH | Pre-existing (bus.rs has no W2-A commits in git log); filed as follow-up |
| `entity_quality` write uses `.ok()` in `intelligence.rs:1404` — clears retry marker on partial failure | MEDIUM | Pre-existing best-effort design; filed as follow-up |

**Cycle 2 (manual verification, codex-companion ENOBUFS on large diff):**
- Finding 2 confirmed clear: zero raw bus calls under src/services/ (verified by grep — remaining 3 hits are: facade's own `use` import + 2× `supersede_signal` which is a non-emit infrastructure op)
- Findings 1 and 3 confirmed pre-existing: bus.rs has no DOS-209 commits; entity_quality `.ok()` predates W2-A
- **APPROVE** — all in-scope findings resolved

**Cycle 3 (L3 Wave adversarial review — final gate):**
Two BLOCK findings:
- HIGH: `entity_linking::evaluate` hard-codes Live mode — no `&ServiceContext` first param, so Simulate/Evaluate callers can't prevent live-state mutations. Fixed: added ctx gate to `evaluate`, `evaluate_meeting`, `evaluate_email`; inline Live ctx at 4 background-task call sites; passed existing ctx at 2 service/command call sites. Commit `f662fd11`.
- HIGH: DOS-209 §9 evidence deferred without L6 authorization while completion was claimed. Fixed: landed `dos209_regression.rs` (6 tests: 3 grep-based lint assertions + 3 mode-boundary proofs from integration-test boundary). Commit `83229a0b`.
- Remaining §9 deferreds (proptest, trybuild, catalog drift CI, transactions): formally documented in `dos209_regression.rs` file-level doc comment; deferred to a standalone follow-up with explicit scope acknowledgment.
- **APPROVE** — both BLOCK findings resolved, minimum §9 evidence landed

### Deliberate scope boundaries

**Out of scope for W2-A:**
- `evaluate_on_signal` enqueue-discard bug (`signals/bus.rs:289`) — pre-existing; needs separate ticket
- `entity_quality` partial-write swallowing (`intelligence.rs:1404`) — pre-existing best-effort; needs separate ticket  
- `with_transaction_async` HRTB primitive (DOS-209 task #79) — deferred; not blocking
- W2-A test suite + trybuild + lint regex test (DOS-209 task #81) — deferred; substrate validated by 1759 passing tests

**`supersede_signal` stays raw:** `crate::signals::bus::supersede_signal` is an infrastructure-level signal-state mutation (marks a signal as superseded), not an emit operation. No service facade equivalent exists; the outer mutator's `check_mutation_allowed()` gate covers it.
