# L0 review — DOS-259 plan — architect-reviewer

**Reviewer:** architect-reviewer (substrate / schema profile)
**Plan revision:** v1 (2026-04-28)
**Verdict:** REVISE

## Findings

### F1 — Trait shape not frozen verbatim from ADR-0106 §3 — open variants surfaced as questions instead of decisions (severity: High)
ADR-0106 §3 pins the trait verbatim:
```
async fn complete(&self, prompt: PromptInput, tier: ModelTier) -> Result<Completion, Error>;
fn provider_kind(&self) -> ProviderKind;
fn current_model(&self, tier: ModelTier) -> ModelName;
```
ADR-0106's `Completion` is `{ text, fingerprint_metadata }` — exactly two fields. ADR-0091 §"The IntelligenceProvider Trait" further requires `Send + Sync` on the trait bound. The plan's §10 surfaces these as open questions rather than freezing them:
- "Confirm `Error` vs `ProviderError` naming in the trait signature."
- "Confirm whether `Completion` is limited to `text` plus `fingerprint_metadata`; cost/latency are not in DOS-259."
- "Confirm `ProviderKind` variants: ticket needs PTY, Glean, Replay, while ADR-0106 examples name ClaudeCode/Ollama/OpenAI/Other."
- "Confirm whether current `ModelTier` in `pty.rs` is reused unchanged or moved/re-exported from `provider.rs`; ticket examples say Fast/Standard/Max, current code has Synthesis/Extraction/Background/Mechanical."

Location in plan: §10 "Open questions"
What needs to change: This is an extraction refactor. The trait shape is already frozen by ADR-0106 §3 + DOS-259 ticket text. The plan must commit to:
1. `Completion` = `{ text: String, fingerprint_metadata: FingerprintMetadata }`, no extra fields. Cost/latency are not in DOS-259 — say "deferred" not "open."
2. `Error` in the trait signature is `ProviderError` (ADR-0091 already names it) — pick one and write it down.
3. `ProviderKind` variants reuse ADR-0106's enum (`ClaudeCode | Ollama | OpenAI | Other(&'static str)`) — `Glean` is `Other("glean")` or a new `Glean` variant; `Replay` is gated to `#[cfg(test)]` / Evaluate mode. Note: the ticket-text variants "PTY/Glean/Replay" are TRANSPORTS, not provider kinds — don't conflate.
4. `ModelTier` is `Synthesis | Extraction | Mechanical` per ADR-0091 (existing code is the source of truth). The "Fast/Standard/Max" in the ticket is illustrative naming the ticket flagged but didn't lock — surface this once and pick the existing enum to avoid churn.
5. The trait carries `Send + Sync` bounds (ADR-0091).

These are not architectural choices left to the implementer — they are already-decided shape. Open questions in §10 should be true ambiguity, not "did I read the ADR right."

### F2 — Parity test is hand-waved, not concrete (severity: High)
§9 lists `manual_enrichment_parity_dev_fixture_byte_identical` and `meeting_prep_parity_dev_fixture_byte_identical`, but §8 says "Parity must be proven with dev-data snapshots for enrichment, Trust Compiler shadow where present, and meeting prep byte-identical pre/post." Trust Compiler does not exist yet (W4-A). "Byte-identical" against what golden file? Captured how? Replayed how?

For a pure extraction refactor, the parity test is the load-bearing artifact — it is the difference between "this is safe" and "this might silently change behavior." The plan needs to name:
- Which fixture(s) drive the parity test (dev workspace? canned PTY response? recorded Glean response?)
- How the "before" snapshot is captured (one-time recorded JSON committed to the repo, or computed at test time from a pinned commit?)
- What is compared (full `IntelligenceJson` byte-equal? prompt template hash equal? completion text equal?)
- For PTY parity specifically, where the non-determinism (process spawning, stderr noise, model nondeterminism) is held constant — almost certainly requires a fixture-backed PTY stub, not a real PTY.

Location in plan: §8 "Parity must be proven with dev-data snapshots..." and §9 `manual_enrichment_parity_dev_fixture_byte_identical`
What needs to change: Drop "Trust Compiler shadow" (it doesn't exist yet). Replace with one concrete parity setup: pinned PTY response fixture replayed through both pre-refactor inline path and post-refactor `PtyClaudeCode::complete()`, asserting byte-identical `IntelligenceJson` output. Same for Glean. The plan needs to name HOW the pre-refactor baseline is captured — likely a JSON snapshot committed alongside the PR.

### F3 — Replay provider mode-boundary wiring is named in §3 but not coherent with §9 test names (severity: High)
§3 says: "Replay fixtures are constructor-supplied in tests and fixture-backed under Evaluate mode; they must never fall through to live PTY or Glean." That is the architectural commitment. But §9 lists `evaluate_mode_never_invokes_live_provider` as a test — which means Evaluate mode must structurally route to the replay provider, not just be "fixture-backed." The plan does not name:
- The construction site that performs the routing. Is it the registry/factory in `provider.rs`? Or `services/intelligence.rs::enrich_entity` branching on `ctx.execution_mode`? Or `AppState` swapping the `Arc<dyn IntelligenceProvider>`?
- Whether the registry receives an `ExecutionMode` from W2-A's `ServiceContext` at construction time, or per-call. ADR-0091 carries the provider on `AppState` (one Arc swap on settings change); ADR-0106 §4 implies replay is per-mode, which suggests per-call routing. These are not the same thing.

This is the mode-boundary risk the W2 wave plan explicitly calls out ("Mode boundary leakage is the architectural risk here"). Leaving it as "constructor-supplied in tests" is acceptable for unit tests but does not satisfy the `evaluate_mode_never_invokes_live_provider` integration test, which by name requires structural routing.

Location in plan: §3 "Replay fixtures are constructor-supplied in tests and fixture-backed under Evaluate mode" + §9 `evaluate_mode_never_invokes_live_provider`
What needs to change: Name the routing site explicitly. Recommended: a `select_provider(ctx: &ServiceContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>` factory in `provider.rs` that inspects `ctx.execution_mode` and returns the replay provider in Evaluate, the configured live provider in Live, and panics or returns `ProviderError::ModeNotSupported` for Simulate (or however §3 wants Simulate to behave — currently silent on it). State whether the live `Arc<dyn IntelligenceProvider>` continues to live on `AppState` per ADR-0091, or moves into the factory.

### F4 — Provider selection "single source" not pinned to one named site (severity: Medium)
§3 says: "Provider selection lives in one registry/factory in `provider.rs`; callers ask for a provider by tier/config and never open-code Glean-vs-PTY checks." That's directionally right. But:
- §2 says `intelligence/mod.rs` exports change and "callers in `services/intelligence.rs`, `intel_queue.rs`, and any text-only call sites" migrate to `provider.complete(...).await?.text`. That implies callers are still picking a provider Arc from somewhere (AppState? factory?) and calling `.complete()` — the selection step is not pinned.
- The current code (`services/intelligence.rs:208-230`) shows two selection sites today: `state.context_provider()` returns context-provider info (Glean vs local) AND there's an inline `GleanIntelligenceProvider::new(endpoint)` construction at line 226 inside `enrich_entity`. The plan should explicitly say the inline construction at line 226 is removed and replaced by a single `select_provider()` call.

Location in plan: §3 "Provider selection lives in one registry/factory in `provider.rs`"
What needs to change: Name the function signature and the call sites that consume it. One sentence: "Single entry point is `intelligence::provider::select_provider(ctx, tier) -> Arc<dyn IntelligenceProvider>`. Removes the inline `GleanIntelligenceProvider::new(endpoint)` at `services/intelligence.rs:226` and any equivalent PTY-construction sites. `intel_queue.rs` and `services/intelligence.rs` are the only callers." That makes the F3 routing site and the F4 selection site one and the same — which is what "lives in one place" means.

### F5 — W2-A merge-order coordination contradicts itself (severity: Medium)
§7 says: "Rebase order: land W2-A context primitives first, then W2-B provider migration, then run the W2 merge gate."

But §6 says: "Do not introduce `Utc::now()` or `thread_rng()` in providers; use W2-A's `ctx.clock` and `ctx.rng` equivalents."

If providers store destructured pieces (per §3) rather than `&ServiceContext`, and W2-A lands first, then W2-B's providers need access to `Clock` and `SeededRng` traits at construction. Fine. But the W2 wave plan I was instructed to consider (`v1.4.0-waves.md` §"Wave 2") and the prompt's hint state: "the cleanest approach is W2-B opens PR first (file restructured, mutation surface unchanged); W2-A rebases on top (adds gates to remaining mutation functions)." That's the opposite ordering.

The architectural argument for W2-B-first: W2-B's only shared file with W2-A is `services/intelligence.rs`, where W2-B *deletes* PTY orchestration and W2-A *adds* `check_mutation_allowed()?` to remaining mutations. If W2-A goes first, W2-B will have to rebase past 60 `check_mutation_allowed()?` insertions including ones in functions W2-B is restructuring or deleting. If W2-B goes first, W2-A's sweep has fewer functions to gate (the PTY block is gone) and no restructure to fight.

The argument for W2-A-first (the plan's pick): providers can directly consume `Clock`/`SeededRng` traits from W2-A without a placeholder. But this is a constructor-arg problem, not a code-coupling problem — W2-B can land trait-shaped placeholders that W2-A fills in or that get a one-line type swap on rebase.

Location in plan: §7 "Rebase order: land W2-A context primitives first, then W2-B provider migration"
What needs to change: Either justify why W2-A-first is correct (specifically: which destructured pieces from W2-A are load-bearing for compile in W2-B such that placeholders won't work) or flip to W2-B-first. The current plan's §7 justification ("land W2-A context primitives first") does not engage with the merge-conflict surface in `services/intelligence.rs`, which is the actual coordination cost. Note: the wave-plan hint is normative — if the plan disagrees, surface it as an §10 question for L6, don't silently invert it.

### F6 — Replay provider scope ambiguous between W2-B and W4-B (severity: Medium)
§2: "The replay provider will be Evaluate-mode gated, with test-only fixture construction for CI."
§9: `replay_provider_returns_canned_completion`, `evaluate_mode_never_invokes_live_provider`
§3: "Replay fixtures are constructor-supplied in tests and fixture-backed under Evaluate mode."

DOS-259 acceptance: "Replay provider exists under `#[cfg(test)]` or gated to Evaluate mode."

W4-B (DOS-216 Evaluation Harness) owns the fixture loader, governance, prompt-fingerprint regression classifier — i.e. the *fixture format and storage*. W2-B should ship: (a) the `ReplayProvider` impl that takes a `HashMap<Hash, Completion>` (or equivalent) at construction, (b) the routing in Evaluate mode. W2-B should NOT ship: fixture file format, fixture directory layout, anonymization rules — those are W4-B per the wave plan.

The plan does not draw this line. §9 says `pty_claude_code_fixture_returns_expected_fingerprint_metadata` and `glean_provider_fixture_returns_expected_fingerprint_metadata` — that suggests fixture files exist at W2 for parity tests. Is that the same fixture format W4-B will use? Or a W2-only ad-hoc?

Location in plan: §2 "with test-only fixture construction for CI" + §9 fixture-named tests
What needs to change: One sentence in §2 or §3: "Replay provider's fixture interface is in-memory `HashMap<Hash, Completion>` (or constructor-supplied closure). Fixture file format, on-disk layout, anonymization, and CI integration are W4-B/DOS-216 — explicitly out of scope here." Then the W2 parity tests are unit tests against the in-memory fixture, not against a file format that W4-B will later own and possibly change.

### F7 — Honoring obligation for W2-A's CI invariants is named in §6 but not test-bound (severity: Low)
§6: "Do not introduce `Utc::now()` or `thread_rng()` in providers; use W2-A's `ctx.clock` and `ctx.rng` equivalents."

Good. But the W2 CI invariants activate at the end of W2 (`no Utc::now() / thread_rng() in services/ or abilities/`). The provider modules live at `intelligence/provider.rs`, `intelligence/pty_provider.rs`, `intelligence/glean_provider.rs` — under `intelligence/`, not `services/` or `abilities/`. Strictly read, the lint W2-A configures will not catch a violation in W2-B's files.

Two ways to handle:
1. W2-B explicitly extends the lint to cover `intelligence/` provider files (preferred — closes the gap structurally).
2. W2-B promises in §6 to honor manually and adds a test `provider_modules_contain_no_direct_clock_or_rng` (acceptable — but tests rot).

Location in plan: §6 "Do not introduce `Utc::now()` or `thread_rng()` in providers"
What needs to change: Add one line — either "W2-B extends W2-A's CI lint glob to include `src-tauri/src/intelligence/{provider,pty_provider,glean_provider}.rs`" or "W2-B adds a unit test asserting no direct clock/rng calls in provider files." The current plan acknowledges the obligation but doesn't enforce it.

### F8 — Refactor blast radius not numerically grounded (severity: Low)
§2 lists the migration sites as `services/intelligence.rs`, `intel_queue.rs`, "and any text-only call sites." The "any text-only call sites" is exactly the kind of soft language the wave plan flagged ("Plan §2 should give a number or grep target").

ADR-0091 says there are 15 `PtyManager::for_tier` construction sites, of which 2 are intel_queue compliant and 13 are deliberate exceptions outside the abstraction's scope. So the call-site migration count is 2 (intel_queue) + the inline Glean construction in `services/intelligence.rs:226`, plus any callers added since. The plan should commit to a number from a grep:
- `rg -n 'PtyManager::for_tier' src-tauri/src/` → expected ~15
- `rg -n 'GleanIntelligenceProvider::new' src-tauri/src/` → expected ~1
- `rg -n '\.complete\(.*tier' src-tauri/src/` → post-refactor consumer count

Location in plan: §2 "any text-only call sites"
What needs to change: Replace with concrete numbers from grep commands run against the current tree, plus a list of which sites migrate (the 2 ADR-0091 sites + the inline Glean construction) vs. which stay PTY-direct (the 13 deliberate exceptions per ADR-0091). The blast radius for this refactor is small and well-bounded — naming it explicitly removes the surface area for "did I miss a caller" review back-and-forth.

### F9 — DOS-304 blocker question is the right open question but not actionable (severity: Low)
§10: "Confirm whether Linear's `blockedBy: DOS-304` relation is informational or must clear before W2-B coding, because the wave plan schedules DOS-259 in W2."

DOS-304 is in the wave-plan's "deliberately NOT in this plan" list ("contract-only ... decisions are already absorbed into the 22-issue spine"). So DOS-304 is the kind of decision-reference ticket whose `blockedBy` relation is informational, not gating. The plan's right to surface it but should commit to one read and move on rather than leave it as a runtime question for L0 reviewers.

Location in plan: §10 "Confirm Linear's `blockedBy: DOS-304` relation is informational or must clear..."
What needs to change: Per the wave plan's own classification of DOS-304 as contract-only, treat the `blockedBy` relation as informational unless DOS-304 has a 2026-04-24+ amendment that materially changes the W2-B contract. State that as the read in §10 rather than asking.

## End-state alignment assessment

This plan does move the v1.4.0 substrate forward in roughly the right shape. The trait extraction targets the seam ADR-0106 §3 froze; the metadata-only `FingerprintMetadata` return shape is what W3-B Provenance's `prompt_fingerprint` consumes; the replay-provider gating on Evaluate mode is what W4-B's eval harness will replay through; and post-spine DOS-213 prompt fingerprinting builds on the `canonical_prompt_hash` hook this plan acknowledges (deferring the hash itself to DOS-213, which is correct). The hot-path concern (§5) is rightly minor — async-trait dispatch overhead is dwarfed by PTY/HTTP latency. The security boundary framing (§4) correctly identifies replay as structurally network-incapable, which is the load-bearing property for hermetic CI later.

What this plan would foreclose if it shipped as-written: it would freeze a trait whose `ProviderKind`/`ModelTier`/`Error` naming wasn't pinned, forcing W3-B and W4-B to either rename on consumption or rebase past trait churn — i.e. the worst form of "extraction refactor that wasn't actually frozen." It would ship a parity test whose specifics aren't named, increasing the chance of silent behavior drift that only surfaces when W4-A Trust Compiler reads a slightly-different `Completion.text` shape and produces shifted scores. And it would leave the Evaluate-mode routing site nameless, which is exactly the substrate gap that lets a future agent accidentally wire a live provider into Evaluate mode (the contamination class the W4-A fold-in is meant to prevent). All three are correctable inside the L0 cycle without touching code — they are plan-clarity issues, not architectural mistakes.

## Verdict rationale

The architectural shape is sound — extraction refactor against an ADR-frozen trait, with the right downstream-consumer awareness. But the plan defers too many already-decided shape questions to "open questions" in §10, hand-waves the parity test that is the load-bearing safety property of an extraction refactor, and leaves the Evaluate-mode routing site nameless when that's the highest-risk mode-boundary surface in the whole wave.

## If REVISE

1. Freeze the trait shape verbatim from ADR-0106 §3 in §3 of the plan (not §10): `Completion = { text, fingerprint_metadata }`; `Error = ProviderError`; `ProviderKind` enum reuses ADR-0106 variants with `Glean` added explicitly; `ModelTier` reuses existing `Synthesis | Extraction | Mechanical`; trait carries `Send + Sync`. Move only true ambiguities to §10 (e.g. `ModelName` newtype location).
2. Make the parity test concrete: name the fixture, the capture method for the pre-refactor baseline, and the comparison granularity. Drop "Trust Compiler shadow" — it doesn't exist yet. Replace with PTY+Glean fixture-backed parity.
3. Name the Evaluate-mode routing site: a single `select_provider(ctx, tier)` factory in `provider.rs` that branches on `ctx.execution_mode`. State explicitly whether `AppState` keeps its provider Arc (per ADR-0091) or whether the factory replaces it.
4. Resolve the W2-A merge order: either justify W2-A-first with a concrete compile-coupling argument, or flip to W2-B-first per the wave plan's hint. Either way, name which file restructures conflict and how they're resolved.
5. Draw the W2-B/W4-B replay-provider line: in-memory fixture interface here; on-disk format, governance, anonymization at W4-B/DOS-216.
6. Close the lint gap on `intelligence/` provider files — either extend W2-A's CI glob or add a unit test ensuring no direct `Utc::now()` / `thread_rng()` in provider modules.
7. Replace "any text-only call sites" with grep-derived numbers: which 2-3 sites migrate, which 13 PTY-direct sites stay (per ADR-0091).
8. Treat DOS-304 `blockedBy` as informational per the wave plan's "deliberately NOT in this plan" classification — state the read, don't leave it open.
