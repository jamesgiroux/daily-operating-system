# L0 review cycle 3 — DOS-259 plan v3 — codex consult mode

**Reviewer:** /codex consult (cycle 3; cycle-2 verdict was REVISE)
**Plan revision under review:** v3 (2026-04-29)
**Verdict:** APPROVE

## F2 closure verification

### §3 select_provider takes &AbilityContext: yes — "`intelligence::provider::select_provider(ctx: &AbilityContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`"

Source: `.docs/plans/wave-W2/DOS-259-plan.md` §3, line 77:

> "Single source of ability-context provider selection is `intelligence::provider::select_provider(ctx: &AbilityContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`."

The rejected v2 shape was quoted in `.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult-v2.md` §Fresh findings, line 19:

> "§3 defines the single routing site as `intelligence::provider::select_provider(ctx: &ServiceContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`."

### AppState-Arc bridge per ADR-0091: yes — §2, §3, §7

Plan v3 documents the early-caller bridge in three places:

- `.docs/plans/wave-W2/DOS-259-plan.md` §2, line 27: "Remove the inline `GleanIntelligenceProvider::new(endpoint)` at `services/intelligence.rs:226` and the main batch-enrichment Glean construction at `intel_queue.rs:1403` from caller code by routing through the AppState-owned provider `Arc` bridge until `AbilityContext` is available."
- `.docs/plans/wave-W2/DOS-259-plan.md` §3, line 79: "Current/early callers without `AbilityContext` route via the AppState-owned provider `Arc` per ADR-0091; the Arc swap on settings change continues to work."
- `.docs/plans/wave-W2/DOS-259-plan.md` §7, line 101: "Until then, callers use the AppState-`Arc` bridge per ADR-0091."

ADR-0091 supplies the storage/bridge basis in `.docs/decisions/0091-intelligence-provider-abstraction.md` §AppState Change, lines 57-60:

```rust
pub intelligence_provider: Arc<dyn IntelligenceProvider + Send + Sync>,
```

ADR-0091 also says provider changes are read per call in `.docs/decisions/0091-intelligence-provider-abstraction.md` §Provider Selection in Settings, line 72:

> "Does not affect in-flight enrichment. The provider is read at call time, so a switch mid-queue takes effect on the next dequeue."

### Cross-section consistency: yes

The F2 seam is consistent across the required sections:

- `.docs/plans/wave-W2/DOS-259-plan.md` §2, line 19: "Create `src-tauri/src/intelligence/provider.rs` for the trait, `Completion`, `PromptInput`, `FingerprintMetadata`, `ProviderError`, `ProviderKind`, `ModelName`, `ReplayProvider`, and `select_provider(ability_ctx, tier)`."
- `.docs/plans/wave-W2/DOS-259-plan.md` §3, line 77: "ServiceContext.execution_mode is the only thing the factory reads from ServiceContext-adjacent context; everything else is AbilityContext-owned."
- `.docs/plans/wave-W2/DOS-259-plan.md` §4, line 85: "Evaluate-mode replay routing is structurally enforced via the AbilityContext-bearing factory plus the ADR-0091 AppState `Arc`-swap pattern."
- `.docs/plans/wave-W2/DOS-259-plan.md` §7, line 101: "`ServiceContext` does not own the provider `Arc`; W3-A's `AbilityContext` owns the provider `Arc` and provider seam."

This matches ADR-0104's split. `.docs/decisions/0104-execution-mode-and-mode-aware-services.md` §3.3, line 209 says:

> "**Intelligence provider is mode-aware via injection.** In `Live`, `AbilityContext.intelligence` is the production provider. In `Simulate`/`Evaluate`, it is a replay provider constructed from the `EvalFixture` and matched by prompt hash."

`.docs/decisions/0104-execution-mode-and-mode-aware-services.md` §Known Limitations and Implementation Open Questions, line 348 says:

> "`IntelligenceProvider` is held on `AbilityContext` rather than `ServiceContext` because multiple providers may co-exist (different abilities can use different providers), whereas external API clients are scoped to the service layer and share replay fixtures."

### Scope discipline: yes

The L6-authorized DOS-259 cycle-3 change was narrow. `.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md` §Recommended options, lines 73-75 authorized only:

> "Move `select_provider` signature to take `&AbilityContext` (or whatever the equivalent W3-A registry-derived context is). Acknowledge that for early callers (intel_queue, services/intelligence.rs) that don't yet have an `AbilityContext`, the routing happens via an `AppState`-owned provider Arc per ADR-0091, and the factory only runs in ability-execution contexts."

and:

> "State explicitly that `ServiceContext.execution_mode` is the only thing the factory reads from `ServiceContext`; everything else is `AbilityContext`-owned."

Plan v3 revision history matches that scope in `.docs/plans/wave-W2/DOS-259-plan.md` §Revision history, line 6:

> "v3 (2026-04-29) — L6-authorized cycle-3 revision. Closed cycle-2 consult F2 (provider routing seam moved from &ServiceContext to &AbilityContext per ADR-0104 split + DOS-259 ticket §Architectural surfaces touched)."

No separate v2 plan snapshot is present in this workspace for a mechanical full-file diff; `.docs/plans/wave-W2/DOS-259-plan.md` and its W2 review directory are currently untracked. Scope discipline is therefore verified as an intent diff against the cycle-2 consult's quoted v2 defect plus the L6-authorized edit list. The v3 text I found maps to F2 only: §2 line 27, §3 lines 77-79, §4 line 85, and §7 line 101.

## Fresh findings (if any)

None.

## Verdict rationale

F2 is closed. Cycle 2 rejected the plan because provider routing was pinned to `&ServiceContext`; v3 pins the signature to `&AbilityContext`, names the AppState `Arc` bridge for early non-ability callers, and states that `ServiceContext.execution_mode` is the only ServiceContext-adjacent input to the factory.

The frozen Linear DOS-259 contract is also satisfied. Linear DOS-259 §Architectural surfaces touched says: "Abilities contract — Transform abilities take `&dyn IntelligenceProvider` via `AbilityContext`." Plan v3 §3/§7 now aligns with that contract and with ADR-0104's quoted `AbilityContext` provider ownership. I found no fresh defect introduced by the authorized cycle-3 edits.

## If APPROVE

Confirm: "F2 closure verified; cycle-2 REVISE conditions resolved; plan frozen."
