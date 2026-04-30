# L0 review cycle 3 — DOS-259 plan v3 — architect-reviewer

**Reviewer:** architect-reviewer (substrate/schema profile, cycle 3; cycle-2 was APPROVE)
**Plan revision under review:** v3 (2026-04-29)
**Verdict:** APPROVE

## F2 closure verification

### §3 select_provider on &AbilityContext: yes

Quote, plan v3 §3 line 77: *"Single source of ability-context provider selection is `intelligence::provider::select_provider(ctx: &AbilityContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`. ServiceContext.execution_mode is the only thing the factory reads from ServiceContext-adjacent context; everything else is AbilityContext-owned."*

Cross-check ADR-0104 §2 (lines 209): *"Intelligence provider is mode-aware via injection. In `Live`, `AbilityContext.intelligence` is the production provider. In `Simulate`/`Evaluate`, it is a replay provider constructed from the `EvalFixture` and matched by prompt hash."* — matches v3's seam ownership.

Cross-check L6 decision item 2: *"confirm `select_provider` moves to `&AbilityContext` per ADR-0104 / DOS-259 ticket text"* — v3 implements the move verbatim.

### AppState-Arc bridge ADR-0091-consistent: yes

Quote, plan v3 §3 lines 79: *"Bridge for early callers: `intel_queue.rs` and `services/intelligence.rs` do not have an `AbilityContext` today. […] Current/early callers without `AbilityContext` route via the AppState-owned provider `Arc` per ADR-0091; the Arc swap on settings change continues to work. When `AbilityContext` lands in W3-A (DOS-210 ability registry), those early callers migrate to the AbilityContext-routed factory."*

Cross-check ADR-0091 lines 60-72: AppState owns `intelligence_provider: Arc<dyn IntelligenceProvider + Send + Sync>`; switching takes effect immediately for the next enrichment call; provider read at call time. v3's bridge uses exactly that pattern — AppState holds storage, callers read at call time, settings-change swap is preserved. There is no contradiction; v3 explicitly states `select_provider()` does not replace AppState storage and only enforces per-call mode routing in ability-execution contexts.

The only nuance worth naming: v3 effectively defines two read sites until W3-A — (a) early callers reading the AppState `Arc` directly, (b) future ability callers using `select_provider(ability_ctx, tier)`. That is a transitional duality, not a contract conflict, because Live mode in `select_provider` returns "the configured AppState provider `Arc`" (§3 line 77), so both paths converge on the same Arc in Live and the Evaluate/Simulate routing only applies where AbilityContext exists.

### §4 Evaluate-mode replay still structurally enforced: yes

Quote, plan v3 §4 lines 84-85: *"Evaluate mode must be structurally incapable of network or PTY invocation. Missing replay data returns a fixture-missing `ProviderError`; it never falls through to Live. Simulate mode is non-generative for this refactor and must fail closed through `ModeNotSupported`. The security argument still holds: Evaluate-mode replay routing is structurally enforced via the AbilityContext-bearing factory plus the ADR-0091 AppState `Arc`-swap pattern."*

Mode-routing integrity is preserved because:
1. Evaluate-mode callers necessarily run inside an `AbilityContext` (ADR-0104 §2 makes evaluation/simulation an ability-execution concept), so they always go through `select_provider(ability_ctx, tier)`, which structurally returns replay.
2. Early callers (intel_queue, services/intelligence.rs) that lack AbilityContext only run in Live — they are the production write paths, not evaluation/simulate paths. There is no scenario where an Evaluate request reaches an `AbilityContext`-less caller, so the bridge cannot leak Live behavior into Evaluate.
3. Simulate fail-closed via `ModeNotSupported` is unchanged from v2.

The cycle-2 contamination-class invariant survives the seam shift.

### Downstream consumer compatibility (W3-B, W4-A, W4-B, DOS-213): yes

- **W3-B Provenance** consumes `Completion.fingerprint_metadata`. The trait shape is unchanged from v2; the seam shift only moves the *factory* off ServiceContext. Field-by-field mapping to `prompt_fingerprint` storage still holds.
- **W4-A Trust Compiler** reads `Completion.text`, `provider_kind()`, `current_model(tier)`. Unchanged surface; byte-identical parity preserved (§8).
- **W4-B Evaluation Harness (DOS-216)** inherits the `ReplayProvider` constructor seam plus the now-correct `select_provider(&AbilityContext, …)` factory — strictly *better* compatibility than v2, since W4-B will route through the same AbilityContext-bearing factory ADR-0104 §2 already named.
- **DOS-213** populates the `canonical_prompt_hash` slot on `FingerprintMetadata`. Unchanged; trait shape is verbatim from ADR-0106.

The seam shift improves W4-B compatibility (factory now lives where ADR-0104 says it should) and is neutral for the other three.

## Scope discipline

### Cycle-3 changes confined to provider-seam sections: yes

Diff against v2 (verified by reading both):

- Revision history (line 6): adds v3 entry citing F2 closure.
- §2 line 27: adds *"through the AppState-owned provider `Arc` bridge until `AbilityContext` is available"* to the inline-Glean-removal sentence — necessary corollary of the seam shift.
- §3 lines 77, 79: signature change `&ServiceContext` → `&AbilityContext`; adds the explicit ServiceContext.execution_mode carve-out; adds the bridge paragraph for early callers.
- §4 line 85: updates the security argument's closing sentence to cite "the AbilityContext-bearing factory plus the ADR-0091 AppState `Arc`-swap pattern".

Everything else — trait shape (§3 lines 33-73), replay scope (§2 line 29), parity-test setup (§8), test list (§9), coding standards (§6), wave merge order (§7), open questions (§10) — is byte-identical to v2. The L6 ruling's hard constraint (cycle 3 = provider-seam routing change ONLY) is honored.

## Fresh findings (if any)

None blocking. Two minor implementation observations, neither requiring REVISE:

**Observation 1 — bridge duality is a transitional state, not a permanent design.** §3 line 79 states early callers migrate to the AbilityContext-routed factory when DOS-210 lands. Worth a single line in the W3-A plan when it is written, to ensure the migration is not silently dropped. Out of scope for DOS-259 plan freeze.

**Observation 2 — ServiceContext.execution_mode read.** §3 line 77 says "ServiceContext.execution_mode is the only thing the factory reads from ServiceContext-adjacent context." Read literally, this implies `select_provider(&AbilityContext, …)` reaches into a ServiceContext through AbilityContext. ADR-0104 says `AbilityContext` carries services and the provider; it does not explicitly pin where execution_mode lives relative to AbilityContext. Either AbilityContext exposes mode directly or it carries a ServiceContext reference; both are consistent with ADR-0104 §2. Implementation detail, not a contract violation.

## End-state alignment (cycle 3 delta)

Yes — the seam shift from `&ServiceContext` to `&AbilityContext` strictly *improves* downstream consumer compatibility relative to v2. ADR-0104 §2 already specified the correct seam (provider injected via AbilityContext, mode-aware routing through `ctx.services.external` patterns); v2's `&ServiceContext` factory contradicted that and would have forced W4-B and post-spine consumers to invent a bridge between the two contexts. v3 collapses that bridge by putting the factory where ADR-0104 said it belonged. W3-B Provenance and W4-A Trust Compiler are unaffected because they consume the trait surface (`Completion`, `fingerprint_metadata`, `provider_kind`, `current_model`), which is unchanged from v2. W4-B Evaluation Harness benefits because its replay provider construction now flows through the AbilityContext factory the harness will already own. DOS-213 is unaffected because the `canonical_prompt_hash` slot lives on `FingerprintMetadata`, not on the factory signature.

The transitional duality (early callers via AppState Arc, ability callers via factory) is the correct shape for this stage of v1.4.0: ADR-0091's AppState-owned `Arc` is real and load-bearing for production today, and the factory only meaningfully discriminates modes inside ability-execution contexts which W3+ introduces. v3 names the migration trigger (W3-A / DOS-210) so the duality has an end date. Mode-boundary integrity holds because Evaluate/Simulate by construction run inside AbilityContext, so they always traverse the factory's mode-discriminating path; the AppState-Arc bridge serves only Live writes from early callers, where there is no mode to discriminate.

## Verdict rationale

The L6-authorized cycle-3 revision lands cleanly. F2 (provider routing seam) is closed by moving `select_provider` to `&AbilityContext` per L6 decision item 2 and ADR-0104 §2; the change is sourced verbatim from the DOS-259 ticket's "Architectural surfaces touched" line. ADR-0091's AppState-owned `Arc` and settings-change swap are not contradicted — they are explicitly preserved as the bridge for early callers (intel_queue, services/intelligence.rs) until DOS-210 introduces AbilityContext in W3-A. Mode-routing integrity is intact because Evaluate/Simulate execute only in AbilityContext-bearing contexts, so the factory always discriminates mode for those paths; the Arc bridge serves only Live early-caller writes where mode is moot. Downstream consumers (W3-B, W4-A, W4-B, DOS-213) are equal-or-better off relative to v2 — W4-B is strictly better because the factory now lives where ADR-0104 specified. Scope discipline is exact: only revision-history, §2 (one bridge clause), §3 (signature + bridge paragraph), and §4 (security closing sentence) changed; trait shape, replay scope, parity setup, test list, coding standards, merge order, and open questions are byte-identical to v2. No silent re-litigation.

Cycle-2 already verified all nine cycle-1 findings closed; cycle 3 verifies F2-redo closes cleanly without disturbing any other closure. APPROVE is the calibrated verdict — REVISE would require a substantive contract violation, and there is none.

## If APPROVE

F2 closure architecturally sound; AppState-Arc bridge ADR-0091-consistent; mode-routing integrity preserved; cycle-3 scope-disciplined; plan frozen.
