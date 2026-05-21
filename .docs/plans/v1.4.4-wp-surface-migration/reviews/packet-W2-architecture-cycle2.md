# W2 Architecture L0 — Cycle 2 Review

**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md` V1.1 at `ffccb485` on `wave/v1.4.4-w1-stage1a` (HEAD `c5c0578f`).
**Reviewer:** ce-architecture-strategist.
**Date:** 2026-05-21.
**Cycle-1 verdict:** APPROVE w/ A1 LOW + A2 MEDIUM + A3 MEDIUM.

## Verdict

**APPROVE — no new findings. L0 architecture lane closes.**

All three cycle-1 findings resolved; focus-area spot-checks land clean; convergence signal is real.

## Cycle-1 resolution

- **A1 (templateLock + template array)** — RESOLVED. §5.1 ships `templateLock: false` + 23-entry `template` array (lines 178–219). §10 promotes to wave-level invariant "Code-shape sketch obligation" (line 779); §5.2/5.3/5.4 carry the shape by delta. Generalizes rather than patches one site.
- **A2 (`useAbilityCursor` authorship)** — RESOLVED. §3 K-in (lines 140–141) declares hook as NEW W2 substrate, cites grep at `c5c0578f` confirming no prior file at the shared-hooks path. Unambiguous.
- **A3 (Meeting envelope verification timing)** — RESOLVED via Path A. §5.4 + §6 + §13 Q4 cite `87df7cf6` (`EntityKind::Meeting` extension) merged at `c5c0578f`. AC-MD.5 (line 487) upgrades to "confirmed-landed."

## Focus-area spot-checks

**1. Projection mapping per ADR-0130 §4.** Spot-checked four blocks across three entities:
- `dailyos/stakeholder-grid` (line 259): Facts (people) ∪ Touchpoints (last-touch). Two W1 sections, renderer-side composition. Sound.
- `dailyos/outlook-panel` (line 254): Health.outlook + projected `renewalCallVerdict`. Explicitly notes the verdict is a projection over factors, not a new section. Sound.
- `dailyos/watch-list-milestones` (line 371): Facts (milestones) ∪ OpenLoops (milestone-bound). Cross-section projection-only. Sound.
- `dailyos/related-entities` (line 474): Facts.related_entities resolved via SubjectRef (ADR-0125). Traverses, doesn't fabricate. Sound.

No projection rule introduces a non-W1 section variant. AC-W2.6 codifies.

**2. `envelopeHandle` contract + DOS-477 cache composition.** §5.1 lines 223–232: outer invokes once; cache key `(envelope_render_id, actor_principal_id, surface)`; inner blocks pass handle via `usesContext` and short-circuit. Composition with DOS-477 cache is clean — surface dimension distinguishes WP/MCP/Tauri, actor dimension respects audience-keyed receipt boundary, render_id derives deterministically from `(entity_type, entity_id, depth, sections, watermark)`. CI gate ("a 2-arg invocation FAILS `check_w1_consumer_skeleton.sh`") gives the contract an enforced floor.

**3. DOS-725 tint as CSS custom property + CI gate.** §5.2 lines 353–359 + AC-W2.7 + §10 invariant (line 770). Pattern `style="--dailyos-project-tint: var(--color-garden-olive);"` on outer wrapper only, asserted via `^--[a-z-]+:\s*var\(--[a-z-]+\);?$`. Architecturally sound — narrowest possible inline-CSS exception (custom-property *assignment* only, no declaratives); ADR-0077 amendment gated as L1 prereq not L0-blocking; CI gate fails closed and opens narrowly; composes cleanly with ADR-0132.

**4. AgentMcp Option B aggregate vs ADR-0108 + W1 AC-341.12.** §10 invariant (line 781) + AC-W2.5 + §13 Q3. Shape `{ count, recency: Recent|Aging|Stale, content: redacted }` honors ADR-0108 §3 (display-safe; no source-internal identifiers) and ADR-0128 (audience-keyed MCP contract). Server-side coarse recency-tier resolution prevents client-side reconstruction of underlying `source_asof` distribution — correct placement. Composes correctly with `build_receipt_for_audience` (audience filter is the gate; aggregate is what passes through for the touchpoint surface). Negative-fixture obligation makes it testable.

**5. New §10 invariants.** Three additions:
- "Code-shape sketch obligation" (line 779) — meta-invariant about the packet. Reinforces outer/inner contract; no conflict.
- "Empty-state pattern" (line 780) — `data-empty-reason="<reason>"`. Composes with refresh-model (empty visible on render) and AgentMcp filter (reason strings clear sanitization). The "inherit `SectionState::Empty { reason }` where applicable, surface-local otherwise" clause handles cross-section cases.
- "AgentMcp touchpoint aggregate" (line 781) — bounded to AgentMcp audience + touchpoint surface; doesn't bleed.

No conflicts among the three or with existing wave-level invariants.

## New findings

**None.** Per memory `feedback_l0_review_loop_diminishing_returns_means_scope_is_wrong`, zero net-new in cycle 2 is the convergence signal — fold-and-close, not fold-and-cycle.

## Recommendation

**Close L0 architecture lane.** Once the other four lanes (codex-challenge, codex-consult, design-lens, wp-skill) converge similarly, panel reaches unanimous APPROVE and W2 L1 begins after ADR-0077 amendment files (AC-W2.7 prerequisite per §13 Q7).
