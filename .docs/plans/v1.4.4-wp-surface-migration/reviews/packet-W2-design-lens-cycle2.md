# Design-Lens Review — v1.4.4 W2 Entity Surfaces (cycle 2)

**Reviewer:** ce-design-lens-reviewer
**Packet:** `L0-packet-W2-entity-surfaces.md` V1.1 (commit `ffccb485`)
**Date:** 2026-05-21
**Verdict:** APPROVE

---

## Cycle-1 blocking resolution

### F1 — Empty-state pattern: RESOLVED

V1.1 §10 invariant "Empty-state pattern" locks the pattern unambiguously: every inner block renders empty as a quiet chip with `data-empty-reason="<reason>"` attribute. `return ''` from render.php is explicitly named as non-conforming. The §5.1 code-shape sketch shows the pattern in working PHP (`return '<div class="dailyos-stakeholder-grid" data-empty-reason="no_stakeholders_in_facts"></div>';`). Reasons inherit `SectionState::Empty { reason: EmptyReason }` from the substrate where available; surface-local reasons are permitted otherwise.

This is a usable, testable pattern. 60+ inner blocks now have a single conforming shape that QA can assert against without per-block negotiation at L1.

**F1: CLOSED.**

### F2 — AgentMcp touchpoint display string and ARIA: RESOLVED WITH ONE ADVISORY

Q3 locks Option B (aggregate signal only) at V1.1. This resolves the core trust-contract question: no per-item existence oracle leaks to the agent. The negative fixture in AC-W2.5 asserts the aggregate-only shape.

However, the ARIA label for the aggregate chip is not stated in the packet. The aggregate signal `{ count, recency, content: redacted }` will render as a chip in non-AgentMcp views; that chip needs a screen-reader announcement. The packet locks *what the chip carries* but not *what the chip announces*. This is implementer-blocking for DOS-692 / the accessibility-tester opt-in, but it is narrow — a single string decision, not an architectural gap.

Advisory (Confidence: 75): add one locked ARIA label to §10 invariant "AgentMcp touchpoint aggregate render" — e.g., `aria-label="N touchpoints; activity level: Recent|Aging|Stale"`. Without it the accessibility-tester pass at L4 will produce a finding that cycles back to this packet retroactively. The aggregate chip is user-visible (non-AgentMcp contexts render it too when content is redacted at the audience layer), so this is not speculative.

This does not block APPROVE. File against DOS-692 AC as a pre-L4 obligation.

**F2: CLOSED on trust contract and interaction shape. ARIA label advisory filed.**

---

## New considerations

### 3 — Projection mapping vs Tauri QA-state parity

V1.1 collapses 22 Tauri chapters (Account Detail) into projection rules over 7 `EnvelopeSection` variants. The question is whether this loses user-visible state distinctions that exist chapter-by-chapter in the Tauri surface.

Verdict: it does not. The 7-section enum is a *substrate* dimension; the 22/14/12/10 inner blocks are *renderer-side projections*, one per chapter. The chapter count is preserved — `dailyos/triage-section` remains distinct from `dailyos/divergence-section` even though both project from `Health`. The visible-QA-state matrix required by AC-462.6 / AC-483.6 / AC-484.5 / AC-MD.6 is per-DOS-ticket, not per-envelope-section. The packet explicitly states chapter ordering is translated 1-to-1 from the Tauri React source files. No user-visible state distinction collapses.

**No finding.**

### 4 — §10 "Code-shape sketch obligation" as a constraint

This invariant requires every block.json declaration named in §5 to ship with a concrete block.json + render.php skeleton. It is useful: it catches spec-named-in-prose-only patterns before they reach L1 (where the gap would be invisible until a codex agent produces an inconsistent shape). The V1.1 packet demonstrates it at §5.1 with the `dailyos/stakeholder-grid` sketch, showing the pattern is achievable without bloating the packet.

One limit: the packet does not extend the sketch to §5.2–5.4 chapters beyond the prose table. The invariant says "every block.json declaration named in §5," but §5.2–5.4 carry only projection tables, not per-block sketches. Given the tables are structurally parallel to §5.1, this is acceptable — §5.1's sketch is the canonical template and §5.2–5.4 inherit the same shape. Implementers have a working pattern to copy; no per-block deviation expected.

Advisory (Confidence: 50): if cycle-2 architecture review surfaces a similar concern independently, confirm the §5.1 sketch is explicitly marked as the cross-entity template in §10. No action required here.

**No blocking finding. Constraint is useful.**

### 5 — Back-navigation from entity-detail to originating briefing

Cycle-1 flagged this as advisory (8/10 user flow completeness). V1.1 does not add a back-navigation specification. The packet explicitly scopes W3 (briefing surfaces) as out-of-scope for W2. AC-MD.4 covers cross-stack navigation from Meeting Detail's `related-entities` to account/project/person detail blocks, but the reverse path (entity detail → originating briefing) remains unspecified.

This is carried forward as an advisory, not a blocking condition. It belongs in W3's L0 packet as an entry-point invariant (briefing → entity → briefing round-trip). Filing here for the W3 author's awareness; W2 APPROVE is not conditional on its resolution.

**Advisory only. Assign to W3 L0 packet scope.**

---

## Conditions for APPROVE

None. Both cycle-1 blocking conditions are resolved. New considerations produce one advisory (ARIA label for aggregate touchpoint chip, pre-L4 obligation on DOS-692) and one cross-wave advisory (back-nav to W3). Neither blocks L0 close.

**APPROVE.**
