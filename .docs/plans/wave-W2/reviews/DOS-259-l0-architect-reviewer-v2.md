# L0 review cycle 2 — DOS-259 plan v2 — architect-reviewer

**Reviewer:** architect-reviewer (substrate / schema profile, cycle 2)
**Plan revision under review:** v2 (2026-04-28)
**Verdict:** APPROVE

## Cycle 1 finding closures verified

### F1 (trait shape frozen verbatim from ADR-0106 §3 + ADR-0091) — closed: yes
Closure location: §3 "Key decisions", lines 30-74.
Verification: v2 §3 reproduces the ADR-0106 §3 trait surface verbatim — `Completion = { text, fingerprint_metadata }`, `complete(&self, prompt: PromptInput, tier: ModelTier) -> Result<Completion, Error>`, `provider_kind() -> ProviderKind`, `current_model(tier) -> ModelName`. ADR-0091's `Send + Sync` bound is then explicitly composed in: "the shipped trait is `IntelligenceProvider: Send + Sync`; `complete()` returns `Result<Completion, ProviderError>`". Cost/latency are stated as "deferred, not open." `ProviderKind` reuses ADR-0106's enum (`ClaudeCode | Ollama | OpenAI | Other(&'static str)`) with `Glean = Other("glean")` and `Replay = Other("replay")` gated to `#[cfg(test)]` / Evaluate. `ModelTier` is locked to ADR-0091's `Synthesis | Extraction | Mechanical` with the ticket's `Fast/Standard/Max` flagged as illustrative. §10 now contains only one true ambiguity (`ModelName` placement), which is correct discipline.

### F2 (parity test concrete; Trust Compiler shadow dropped) — closed: yes
Closure location: §8 "Failure modes + rollback", lines 100-104.
Verification: v2 names the fixture (pinned PTY and Glean response fixtures), the baseline capture method (pre-refactor `IntelligenceJson` snapshots committed alongside the PR), the comparison granularity (canonical serialized `IntelligenceJson` byte equality + completion text byte equality + prompt-template identity/hash equality where W2 supplies it), and the harness shape (replay same prompt/response through both pre- and post-refactor paths). Trust Compiler shadow is explicitly dropped: "Drop Trust Compiler shadow parity from W2-B: W4-A does not exist yet." Meeting prep is correctly handled with a non-touch grep assertion or, if touched, the same fixture parity. DOS-213 boundary is preserved ("DOS-213 still owns production canonical hash computation").

### F3 (Evaluate-mode routing site named; AppState Arc handling stated) — closed: yes
Closure location: §3 lines 76-77.
Verification: v2 names `intelligence::provider::select_provider(ctx: &ServiceContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>` as the routing site. Mode branching is explicit: Live returns configured live provider, Evaluate returns replay, Simulate fails closed via `ProviderError::ModeNotSupported` rather than reaching PTY/HTTP. AppState Arc handling is stated unambiguously: "AppState keeps the configured live provider `Arc` per ADR-0091 and swaps it on settings changes; `select_provider()` does not replace storage, it enforces per-call mode routing." The two-construct compatibility question from cycle 1 is now resolved — AppState owns storage, factory owns per-call mode routing.

### F4 (provider selection single source; inline Glean construction at services/intelligence.rs:226 named) — closed: yes
Closure location: §2 lines 16-26.
Verification: v2 commits to `select_provider(ctx, tier)` as the single source, names the inline `GleanIntelligenceProvider::new(endpoint)` at `services/intelligence.rs:226` for removal, and additionally pins the main batch-enrichment Glean construction at `intel_queue.rs:1403` for removal. Post-refactor production consumers count is committed at 2 (`services/intelligence.rs` and `intel_queue.rs`). The 21 remaining `PtyManager::for_tier` matches are correctly preserved as ADR-0091's deliberate exceptions (reports/processors/executors/devtools/etc.). Glean account discovery and leading-signal helpers are correctly kept Glean-specific (not `complete()` consumers).

### F5 (W2-A merge order flipped or justified) — closed: yes
Closure location: §7 lines 96-98.
Verification: v2 flipped to W2-B-first, matching the wave-plan hint: "W2-B opens first and restructures `services/intelligence.rs` / `intel_queue.rs` around provider invocation before W2-A sweeps mutation gates. W2-A then rebases on the smaller mutation surface and adds `check_mutation_allowed()` to the remaining service mutations." The merge-conflict surface (`services/intelligence.rs`) is named, and the coordination contract (`select_provider(ctx, tier)` signature + ServiceContext expectations) is explicit. Shared-file owner note correctly partitions ownership: W2-B removes provider orchestration, W2-A gates remaining writes. This is the architecturally cheaper merge order.

### F6 (W2-B/W4-B replay scope line drawn) — closed: yes
Closure location: §2 line 28.
Verification: v2 draws the line cleanly: "Replay scope is W2-B only at the trait/routing layer: `ReplayProvider` takes an in-memory `HashMap<Hash, Completion>` or constructor-supplied lookup closure. Fixture file format, on-disk layout, anonymization, capture governance, and CI harness integration are explicitly W4-B/DOS-216." This means the W2 parity tests are unit tests against in-memory fixtures, not against a file format W4-B will later own and possibly change. The seam W4-B inherits is the `ReplayProvider` constructor signature, which is stable across fixture-format choices.

### F7 (lint gap closed by glob extension) — closed: yes
Closure location: §6 line 92.
Verification: v2 takes the structural option: "W2-B extends the W2-A lint coverage to include `src-tauri/src/intelligence/{provider,pty_provider,glean_provider}.rs`, closing the gap left by the default `services/` + `abilities/` glob." This is the preferred resolution from cycle 1 — extending the lint glob is structurally enforced (vs. a unit-test promise that rots).

### F8 (blast radius numerically grounded) — closed: yes
Closure location: §2 lines 20-26.
Verification: v2 commits grep counts run against `src-tauri/src/` on 2026-04-28: `PtyManager::for_tier` → 23 matches, `GleanIntelligenceProvider::new` → 4 matches, `.complete(.*tier)` → 0 pre-refactor. Migration target is named: 2 PTY sites (intel_queue.rs:1607 parallel extraction, intel_queue.rs:1819 legacy synthesis), services/intelligence.rs:226 inline Glean, intel_queue.rs:1403 batch Glean. The 21 PTY-direct sites that stay are partitioned by category (reports, processors, executors, devtools, background maintenance, repair retry) per ADR-0091's deliberate exceptions. Glean account discovery (`commands/integrations.rs:3769`) and leading-signal enrichment (`intel_queue.rs:894`) are explicitly excluded as Glean-specific product calls, not `complete()` consumers. (Note: cycle-1 estimate was ~15 PTY matches; v2's 23 reflects current tree and is the authoritative number.)

### F9 (DOS-304 informational read committed) — closed: yes
Closure location: §10 line 116.
Verification: v2 commits the read: "Linear still shows DOS-259 blocked by DOS-304, but `v1.4.0-waves.md` classifies DOS-304 as a contract-only decision-reference ticket whose decisions are already absorbed into the 22-issue spine. W2-B does not gate on DOS-304 resolution unless DOS-304 receives a new amendment after 2026-04-28 that materially changes the DOS-259 contract." That is exactly the cycle-1 prescribed shape — informational unless materially amended.

## Fresh findings (if any)

None blocking. Two minor observations for the implementer, neither requiring REVISE:

**Observation 1 — `ProviderError::ModeNotSupported` variant referenced in §3 and §4 but not defined.** The variant is mentioned in two places but `ProviderError`'s variant list is not enumerated. ADR-0106 names the type but doesn't pin variants. This is implementation-ambiguity, not architectural-ambiguity — adding the variant in `provider.rs` is mechanical and the plan's behavior is unambiguous (Simulate must fail closed). Surface only if implementation discovers a variant collision.

**Observation 2 — concurrency test (§9) proves Send + Sync invariant from ADR-0091.** Good addition. One subtle note: a fixture-backed provider's concurrency behavior may be more permissive than a real PTY (which serializes on the subprocess) — the test proves the trait is parallel-safe at the type level, but does not prove `PtyClaudeCode` itself handles N concurrent callers safely against one PTY. That's correct scope for W2-B (DOS-259 doesn't change PTY queueing) and is properly ADR-0091's existing concern.

## End-state alignment assessment (cycle 2)

Yes. v2 freezes a seam downstream consumers can adopt without breaking changes:

- **W3-B Provenance** consumes `Completion.fingerprint_metadata` exactly as ADR-0106 §3 defines. v2 pins those fields verbatim, so W3-B's `prompt_fingerprint` storage maps cleanly onto `FingerprintMetadata` field-by-field with no rename or reshape required.
- **W4-A Trust Compiler** reads `Completion.text` and `provider_kind()` / `current_model(tier)`. v2 commits to byte-identical `Completion.text` via the parity harness, so Trust Compiler will not see drifted scores from extraction-induced text changes. Provider/model identity is stable across the trait surface.
- **W4-B Evaluation Harness (DOS-216)** inherits the `ReplayProvider` constructor seam. v2 explicitly defers fixture format/storage to W4-B, which means W4-B can choose its on-disk format without W2-B re-litigating the trait. The in-memory `HashMap<Hash, Completion>` interface is the stable seam.
- **DOS-213 (post-spine prompt fingerprinting)** consumes the `canonical_prompt_hash` hook. v2 correctly defers production hash computation to DOS-213 while pinning the metadata return shape DOS-213 will populate, so DOS-213 does not need to amend the trait — only fill in the hash.

Mode-boundary integrity is the strongest improvement in v2. The cycle-1 risk — that Evaluate mode could silently fall through to a live provider — is now structurally precluded: `select_provider()` is the single per-call routing site, Simulate fails closed via `ModeNotSupported`, missing replay data returns a fixture-missing error, and the lint glob structurally prevents new clock/rng leaks in provider files. That gives W4-A's contamination-class invariants a sound substrate to build on.

Refactor discipline is sound: extraction-only, services-only-mutate rule preserved (§6), no schema changes, no signal/health/briefing/feedback hooks, no derived-state writes from providers. The Intelligence Loop 5-question check is correctly answered with "no" for every dimension because this PR is pure substrate plumbing.

## Verdict rationale

All nine cycle-1 findings (3 High, 3 Medium, 3 Low) are closed in v2 with verifiable references in §2, §3, §6, §7, §8, §9, and §10. The trait shape is now ADR-frozen verbatim, the parity test is concrete and falsifiable, the Evaluate-mode routing site is named with AppState ownership stated, the W2-A merge order is flipped to the architecturally cheaper option, the W2-B/W4-B scope line is drawn, the lint gap is closed structurally, the blast radius is grep-grounded with site-by-site classification, and DOS-304 is committed as informational. End-state seams (W3-B, W4-A, W4-B, DOS-213) can consume v2's frozen surface without breaking changes. Cycle-2 bias is toward APPROVE when cycle-1 findings close, and they have.

## If APPROVE

The architectural foundation is sound because:

1. The trait surface is frozen by two ADRs (0106 §3 + 0091) and v2 reproduces both verbatim — there is no drift between plan and ADR, which is the load-bearing property of an extraction refactor.
2. Mode-boundary integrity is structural, not promissory: `select_provider()` is the single routing site, Simulate fails closed, Evaluate cannot fall through to Live, and the lint glob prevents new clock/rng leaks. That eliminates the contamination-class risk W4-A would otherwise inherit.
3. Parity is falsifiable: pinned fixtures + canonical `IntelligenceJson` byte equality is a concrete merge-gate artifact, not a promise.
4. The seam is consumable: W3-B, W4-A, W4-B, and DOS-213 each have a named field/function on the v2 surface they can adopt without trait churn.
5. Blast radius is small and named: 4 caller migrations, 21 deliberate exceptions, 2 post-refactor `complete()` consumers — review burden bounded.
6. Refactor discipline preserved: services-only-mutate rule, no schema, no Intelligence Loop dimensions touched. This PR is pure substrate.

Plan is frozen. W2-B implementation can proceed against v2.
