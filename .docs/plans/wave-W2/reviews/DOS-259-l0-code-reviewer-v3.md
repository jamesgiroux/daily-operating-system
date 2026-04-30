# L0 review cycle 3 — DOS-259 plan v3 — code-reviewer (substituted for codex challenge)

**Reviewer:** code-reviewer (cycle 3; substituted for /codex challenge per L6 process note re: codex:rescue runtime stall pattern)
**Plan revision under review:** v3 (2026-04-29)
**Authorized cycle-3 scope:** F2 (provider seam routing) only
**Verdict:** APPROVE

## F2 closure verification

### §3 pins select_provider on &AbilityContext: yes

Quote (plan v3 §3, line 77):

> "Single source of ability-context provider selection is `intelligence::provider::select_provider(ctx: &AbilityContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`. ServiceContext.execution_mode is the only thing the factory reads from ServiceContext-adjacent context; everything else is AbilityContext-owned."

This is the correct signature per (a) DOS-259 ticket §Architectural surfaces touched ("Transform abilities take `&dyn IntelligenceProvider` via `AbilityContext`"), (b) ADR-0104 §"Known Limitations and Implementation Open Questions" #5 ("`IntelligenceProvider` is held on `AbilityContext` rather than `ServiceContext` because multiple providers may co-exist..."), and (c) the L6 ruling specific decision #2.

The signature also explicitly walls off `ServiceContext` to its single legitimate input — `execution_mode` for mode routing — which is consistent with ADR-0104 §3.3's note that the provider abstraction is mode-aware *via injection on AbilityContext*, not via a ServiceContext-owned reference.

### AppState-Arc bridge for early callers: clear and ADR-0091-consistent: yes

The bridge is named in two places:

- §2 line 27: "Remove the inline `GleanIntelligenceProvider::new(endpoint)` at `services/intelligence.rs:226` and the main batch-enrichment Glean construction at `intel_queue.rs:1403` from caller code by routing through the AppState-owned provider `Arc` bridge until `AbilityContext` is available."
- §3 line 79 (the dedicated bridge paragraph): "Bridge for early callers: `intel_queue.rs` and `services/intelligence.rs` do not have an `AbilityContext` today. In ability-execution contexts (W3+ when `AbilityContext` exists), callers use `select_provider(ability_ctx, tier)`. Current/early callers without `AbilityContext` route via the AppState-owned provider `Arc` per ADR-0091; the Arc swap on settings change continues to work. When `AbilityContext` lands in W3-A (DOS-210 ability registry), those early callers migrate to the AbilityContext-routed factory."

This is consistent with ADR-0091 §"AppState Change" line 60: `pub intelligence_provider: Arc<dyn IntelligenceProvider + Send + Sync>` is exactly the field that backs the bridge. The settings-swap behavior promised in §3 (and §4) maps directly to ADR-0091 §"Provider Selection in Settings" lines 67–72 ("read at call time, so a switch mid-queue takes effect on the next dequeue"). The bridge is a re-use of an already-decided substrate, not an invention.

### Cross-section consistency (§2/§3/§4/§7): yes

I cross-checked all four sections for ownership-model contradictions:

- §2 (Approach, line 19): names `select_provider(ability_ctx, tier)` as the routing function. Consistent with §3.
- §2 line 27: names the AppState-Arc bridge for the two early callers (`intel_queue.rs`, `services/intelligence.rs`). Consistent with §3 line 79.
- §3 line 77: factory signature on `&AbilityContext`; ServiceContext only contributes `execution_mode`.
- §3 line 79: bridge text matches §2 and is explicit about W3-A migration timing.
- §4 (Security, line 85): "Evaluate-mode replay routing is structurally enforced via the AbilityContext-bearing factory plus the ADR-0091 AppState `Arc`-swap pattern." This is the load-bearing security invariant for Evaluate, and it is consistent with the §3 ownership model. The Live/Evaluate/Simulate routing in §3 line 77 ("Live returns the configured AppState provider `Arc`, Evaluate returns replay, and Simulate returns a non-live provider whose `complete()` fails closed with `ProviderError::ModeNotSupported`") matches §4's structural-enforcement claim.
- §7 (Coordination, line 101): "`ServiceContext` does not own the provider `Arc`; W3-A's `AbilityContext` owns the provider `Arc` and provider seam. W2-B ships the trait, implementations, and routing factory; the routing factory is wired into `AbilityContext` when the W3-A/DOS-210 registry lands. Until then, callers use the AppState-`Arc` bridge per ADR-0091." This is verbatim consistent with §3.

No contradictions. The four sections describe one ownership model.

### §7 W2-B-first merge order preserved from v2: yes

§7 line 99: "W2-B opens first and restructures `services/intelligence.rs` / `intel_queue.rs` around provider invocation before W2-A sweeps mutation gates. W2-A then rebases on the smaller mutation surface and adds `check_mutation_allowed()` to the remaining service mutations."

This matches the v2 ordering and L6 ruling sub-decision 1A (amend DOS-209 ticket per wave plan; W2-B-first). v3 did not regress this.

## Scope discipline

### Cycle 3 changes confined to F2-related sections: yes

Reading v3 against the cycle-2 review's F2 finding (which named §3 line 76 and §7 line 98 as the conflict locations), v3 changes appear to be:

1. §3 line 77 — signature changed from `&ServiceContext` to `&AbilityContext`, with a one-sentence ServiceContext-scoping clarifier.
2. §3 line 79 — new "Bridge for early callers" paragraph (this is the architectural mechanism that closes F2).
3. §2 line 27 — bridge phrasing added in two clauses ("by routing through the AppState-owned provider `Arc` bridge until `AbilityContext` is available"). The list of early-caller sites is unchanged from v2.
4. §4 line 85 — security-invariant sentence restated to reference the AbilityContext-bearing factory plus AppState-Arc swap (was previously expressed in ServiceContext terms).
5. §7 line 101 — coordination text rewritten to say `ServiceContext` does not own the provider Arc, with the W3-A handoff timing.
6. Revision history line 6 — added.

All five substantive edits are local to the provider-routing seam. §1, §5 (Performance), §6 (Coding standards), §8 (Failure modes), §9 (Test evidence), §10 (Open questions) appear unchanged in substance from v2 (modulo any wording that was already correct). The frozen contract — trait shape from ADR-0106, parity test names, replay scope, lint coverage, merge order — is preserved.

No silent out-of-scope changes detected. v3 honors the L6 ruling's tight scoping.

## Fresh findings (if any)

None at the High or Critical severity bar.

I did probe the three adversarial questions specified in the brief:

1. **Does the AppState-Arc bridge create a hidden dependency that contradicts §3's mode-routing decision?** No. The bridge is explicitly framed as "early callers without AbilityContext route via the AppState-owned provider `Arc` per ADR-0091" — i.e., the bridge applies only to non-ability call sites (intel_queue, services/intelligence.rs) which today have no Evaluate-mode invocation path either, since Evaluate is gated on the AbilityContext-bearing factory. The bridge is a Live-mode-only path until W3-A migrates the callers, at which point it disappears. No mode-routing contradiction.

2. **Does `&AbilityContext` work for both Live and Evaluate routing, or does Evaluate replay need to bypass AbilityContext?** §3 line 77 states the factory returns `Arc<dyn IntelligenceProvider>` and routes Live → AppState Arc, Evaluate → replay, Simulate → fail-closed. Replay is itself an `IntelligenceProvider` impl per the trait contract; it does not bypass AbilityContext, it is *constructed by* the AbilityContext-bearing surface (per ADR-0104 §6 and §"Known Limitations" #5: "the surface … wires the mode-appropriate `IntelligenceProvider` into the `AbilityContext` it constructs"). Architecturally sound.

3. **Does v3 over-correct by breaking something that was correct in v2?** I checked the four cross-section consistency points, the merge order, the trait-shape pin, the parity test names, the replay scope, the lint coverage, and the ADR-0091/0106 citations. Nothing that was load-bearing-correct in v2 has been silently regressed. The "early callers" enumeration in §2 line 27 still names the same two sites that v2 named.

One Low-severity observation, not a finding: §3 line 79 says "When `AbilityContext` lands in W3-A (DOS-210 ability registry), those early callers migrate to the AbilityContext-routed factory." That migration ticket is not numbered. Suggested follow-up for whoever drives W3-A planning: file a follow-up issue in W3-A scope to track the migration, so the AppState-Arc bridge is not orphaned. This is a process note, not a v3 blocker.

## Verdict rationale

F2 closure is real and architecturally sound: the signature is pinned to `&AbilityContext` per the ticket and ADR-0104 §"Known Limitations" #5; the AppState-Arc bridge for early callers is a re-use of ADR-0091's already-decided AppState field, not an invention; cross-section consistency (§2/§3/§4/§7) holds; W2-B-first merge order is preserved; and v3's changes are tightly scoped to the provider-routing seam without silent regressions elsewhere. Per L6 ruling, REVISE on cycle 3 means work pauses for deeper architecture review — that severity bar is reserved for substantive contract violations, none of which this plan presents.

## If APPROVE

F2 closure verified architecturally; AppState-Arc bridge consistent with ADR-0091; cross-section consistency intact; cycle-3 scope discipline maintained; plan frozen.
