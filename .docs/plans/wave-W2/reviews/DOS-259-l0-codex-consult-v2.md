# L0 review cycle 2 — DOS-259 plan v2 — codex consult mode

**Reviewer:** /codex consult (cycle 2)
**Plan revision under review:** v2 (2026-04-28)
**Verdict:** REVISE

## Cycle 1 finding closure verified

### F1 (parity + concurrent-invocation tests) — closed: yes
Closure location: §8 lines 102-104 and §9 lines 108-110
Verification: §9 now names `pty_provider_parity_fixture_intelligence_json_byte_identical`, `glean_provider_parity_fixture_intelligence_json_byte_identical`, and `provider_complete_concurrent_invocations_all_succeed`. §8 also makes the parity harness concrete: pinned PTY/Glean response fixtures, pre-refactor baseline `IntelligenceJson` snapshots, canonical serialized byte comparison, completion text byte equality, and prompt-template identity/hash equality where the W2 hook supplies it. The Trust Compiler part is resolved by an explicit scope correction in §8 line 104: W4-A does not exist yet, and meeting prep is either untouched or gets byte-identical fixture parity if touched.

## Fresh findings (if any)

### F2 — Provider routing is pinned to `ServiceContext`, but the frozen contract routes provider access through `AbilityContext` (severity: High)

Location in plan v2: §3 line 76 and §7 line 98.

Observed plan content: §3 defines the single routing site as `intelligence::provider::select_provider(ctx: &ServiceContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`. §7 then says the W2-B/W2-A coordination contract expects `ServiceContext` to expose execution mode and access to the configured live provider Arc.

Observed frozen contract: Linear DOS-259 says the abilities contract surface is "Transform abilities take `&dyn IntelligenceProvider` via `AbilityContext`." ADR-0104, which ADR-0106 depends on for Evaluate-mode replay, makes the same split: `ServiceContext` carries mode-aware services and external clients, while the intelligence provider is injected through `AbilityContext`.

Why this blocks implementation: an implementer cannot satisfy both surfaces without inventing a bridge. If they follow §7 literally, `ServiceContext` grows or exposes the configured live provider Arc, which contradicts the AbilityContext-owned provider seam. If they follow the frozen contract, `select_provider(ctx: &ServiceContext, tier)` is the wrong signature and cannot be the only provider-selection source for future Transform abilities. This is exactly the kind of substrate seam W2 is supposed to freeze before W3 consumes it.

What needs to change: keep the single selection site, but pin the right caller shape. For example: current service/intel-queue callers can route through an AppState/provider-registry-owned live `Arc`, while future ability callers receive `ctx.intelligence` on `AbilityContext`; `ServiceContext` may supply `ExecutionMode`, but it must not be the owner or exposure path for the live provider Arc. The plan should name the concrete signature and bridge so W2-B and W3-A do not invent different shapes.

## Verdict rationale

F1 is closed. The trait shape, provider modules, replay scope, parity evidence, merge ordering, lint coverage, and test names are otherwise implementable from v2 without major invention.

I am still returning REVISE because §3/§7 freeze the provider-selection seam to `ServiceContext` in a way that conflicts with the Linear `AbilityContext` contract. This is not irreconcilable; it is a narrow plan edit. But leaving it to code time would force either W2-B or W3-A to rework the substrate boundary.

## If APPROVE

N/A. After the provider-routing owner/signature is corrected, the plan is otherwise close to approval.
