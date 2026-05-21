# Design-Lens Review — v1.4.4 W2 Entity Surfaces (cycle 1)

**Reviewer:** ce-design-lens-reviewer  
**Packet:** `L0-packet-W2-entity-surfaces.md` V1.0  
**Date:** 2026-05-20  
**Verdict:** CONDITIONAL APPROVE — 2 blocking conditions

---

## Dimensional ratings

**Information architecture: 9/10** — it's a 9 because the 22-chapter ordering in §5.1 traces 1-to-1 from `AccountDetailPage.tsx`, section groupings (Health / Context / Work / Record) are named, and Q1 locks canonical ordering as default. A 10 would specify whether empty section headings collapse or render with an empty-state block. Advisory; not blocking.

**Interaction state coverage: 6/10** — it's a 6 because AC-462.6 lists required QA states correctly, but three of them lack specified content:

1. **Empty state per chapter:** when an envelope slice returns no data (e.g., `stakeholders: []`), what does the corresponding inner block render? A chip? A hidden PHP conditional? The packet inherits "`AccountDetailPage.md`'s" "evidence gaps must be visible" rule but does not resolve it to a pattern. 48+ inner blocks across four composites will each produce a different answer at L1.
2. **Corrected/superseded state on the originating chapter:** §5.6 specifies the proposal lifecycle in the drawer correctly. The post-accept visual state on the chapter that contained the original claim is unspecified — does it re-render silently on next fetch, or show a "correction applied" affordance?
3. **AgentMcp redacted placeholder content:** Q3 resolves to Option A (placeholder) but the display string is proposed ("Redacted touchpoint"), not locked, and no ARIA label is specified.

A 10 would have per-chapter empty-state content and the post-correction chapter render pattern locked.

**User flow completeness: 8/10** — it's an 8 because the cross-wave flow (briefing → meeting → entity → action) is correctly inherited and AC-MD.4 covers cross-stack navigation. The back-navigation affordance from entity detail to the originating briefing surface is unspecified (browser back vs FolioBar breadcrumb vs FloatingNavIsland). Advisory; not blocking.

**Responsive / accessibility: 7/10** — it's a 7 because DOS-692 and the `accessibility-tester` opt-in are explicit. No breakpoint strategy is stated for the 22-chapter magazine layout at narrow viewports. Advisory; the design system likely covers this, but it should be confirmed.

**Unresolved design decisions: 6/10** — it's a 6 because Q3 is marked "resolves at L0 close" but the specific display string and ARIA label are not locked in the packet, and the empty-state content pattern (described above) will surface as an unresolved question for each of the 48+ inner blocks at L1. Both are implementer-blocking.

**Inline edit affordance: 9/10** — it's a 9 because AC-W2.11 locks the `FeedbackAction` → `record_claim_feedback` contract, §5.6 specifies all three action wire shapes, and AC-328.8 guards against persistent-nag failure. The in-flight visual state on the originating entity-detail chapter (while a correction round-trips) is unaddressed — but that state belongs to W4's matrix and the pull model makes it low risk. Advisory.

**AI slop check: PASS** — entirely grounded in DailyOS-specific substrate terms. No generic SaaS phrasing.

---

## Findings

### F1 — BLOCKING: Empty-state content pattern unspecified across 48+ inner blocks (Confidence: 100)

AC-462.6 names "empty" as a required visible QA state. `AccountDetailPage.md` mandates gaps be visible, not silently hidden. Neither the packet nor the wave plan specifies what an inner block renders when its envelope slice is empty (null, zero-item list, or missing key). With 5 parallel codex fan-out tasks each implementing a chapter cluster, each agent will independently resolve this, producing inconsistent behavior that requires a sweep at L4 or W6.

**Condition for APPROVE:** lock one of: (a) a shared empty-state pattern ("every list-type inner block uses a `dailyos/empty-state-chip` with slice-specific vocabulary label"), or (b) an explicit hidden-conditional decision ("PHP `if empty($slice) return ''` is the approved pattern; empty sections do not render"). Add as a §10 invariant or §5.1 implementation note.

### F2 — BLOCKING: Q3 display string and ARIA label not locked (Confidence: 100)

§13 Q3 resolves to Option A (placeholder) with "Redacted touchpoint" as the proposed text, but the packet says "resolves at L0 close with /cso panel review." The display string and ARIA label are still open at packet-author time. Announcing "Redacted touchpoint" to a screen reader reveals that a touchpoint exists — which may be intentional under Option A's trust-contract rationale, but must be explicitly affirmed. The `dailyos/touchpoints-feed` inner block cannot be implemented without a locked string.

**Condition for APPROVE:** lock display string (e.g., "Redacted touchpoint" confirmed) and ARIA label (e.g., `aria-label="Touchpoint available; details redacted for this view"`) in the packet before L0 closes. `/cso` sign-off on the trust-contract rationale included in the same amendment.

---

## Conditions for APPROVE

1. Locked empty-state content pattern for inner blocks added as a §10 invariant or §5.1 note — addresses F1.
2. Q3 display string and ARIA label locked; `/cso` sign-off confirmed — addresses F2.

No scope change required. Both are targeted V1.1 packet amendments.
