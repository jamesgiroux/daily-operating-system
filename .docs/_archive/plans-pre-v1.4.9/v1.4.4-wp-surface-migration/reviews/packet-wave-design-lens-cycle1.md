# Design-Lens Review — L0 Packet v1.4.4 Wave Plan (WordPress Surface Migration)
**Reviewer:** `compound-engineering:ce-design-lens-reviewer`
**Cycle:** 1
**Date:** 2026-05-20
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` (V1.0)
**Companion docs read:** `.docs/plans/wp-foundation-roadmap-reorientation.md` §"Anchored decisions" + §"v1.4.4"; `.docs/plans/v1.4.4-surface-audit.md`; `.docs/design/INVENTORY.md`; `.docs/design/product/{MISSION,PRODUCT-THESIS}.md`

---

## VERDICT: CONDITIONAL APPROVE

The wave structure, W0–W6 sequencing, and substrate-in-same-wave discipline are sound. The packet earns CONDITIONAL APPROVE rather than APPROVE on two design-blocking gaps: (1) the visible-QA-state matrix promised at wave level is not enumerated per wave — it is asserted for W2 then left implicit for W3/W4/W5, which will cause sub-L0 authors to miss states; and (2) the cross-wave user journey that customer-zero will actually follow (briefing → meeting → entity → action) is not named anywhere in the packet, so the W2/W3/W4 wave-boundary design decisions are unanchored to real user behavior. A third advisory concerns the deferral honesty of the personal-report entry points.

Both blocking gaps can be resolved by targeted additions to the existing packet — no scope change is required. They are enumerated with recommended fixes below.

---

## K-in (design-lens)

`docs/solutions/` grep for: `ux-state`, `interaction-state`, `empty-state`, `trust-band`, `user-flow`, `briefing`, `entity-detail`, `cross-wave`. No relevant prior solutions suppressed or reinvented. The prior design-lens reviews on the chrome lane packet (`packet-W3-chrome-design-lens-cycle3.md`) confirmed cycle-3 APPROVE on that lane; nothing in that record conflicts with findings here.

---

## Finding 1 — BLOCKING (Confidence 100)

**Dimension:** Interaction state coverage: 5/10 — it's a 5 because the packet names the visible-QA-state matrix in AC #W1 by reference to DOS-462's W2 list, then stops. W3 (Briefing), W4 (Action), and W5 (System/history) carry no equivalent enumeration. A 10 would have the matrix applied per wave with every required state listed — including the states most likely to be missed for that wave's content.

**Section cite:** §5.2 (W2), §5.3 (W3), §5.4 (W4), §5.5 (W5), §7 AC #W1.

**Design concern:** AC #W1 says "supports the visible-QA-state matrix called out per W2 in DOS-462 (full / empty / stale / `needs_verification` / corrected/superseded / proposal cue / etc.)." The "etc." at the end plus the "per W2" scoping means sub-L0 authors for W3/W4/W5 have no authoritative state list. The W2 list in DOS-462 was designed for entity detail pages (Health/Context/Work views, claim-backed sections). Briefing surfaces and action surfaces have materially different state vocabularies:

- **W3 Briefing block:** has a "no meetings today" state, a "briefing generating" state, a "all meetings already complete" state, a "prep ready vs prep not started" state (per DOS-335), and an "ambiguity" state where two conflicting claim signals exist for the same account. None of these appear in the W2 entity-detail matrix, and none are called out in §5.3.
- **W4 Action surfaces:** has a "no open actions" state (empty queue), a "claim pending review" state (stuck in review queue), a "contradiction flagged" state distinct from "corrected/superseded", and an inline-edit "in-flight" state where the WP block has emitted the feedback claim but the re-render projection has not returned. The distinction between "contradiction flagged awaiting user decision" and "contradiction resolved, superseded" is load-bearing for trust — DOS-318 is named but the visible states are not enumerated.
- **W5 History block:** distinguishes "no inbox activity this week" (empty) from "inbox activity but no claim outcomes" (populated but unintelligent) — very different user experiences that drive very different empty-state copy.

Without a wave-level commitment to enumerate states per wave, each sub-L0 author will make independent choices, and the QA matrix at W6 parity proof will have no consistent baseline to check against.

**Recommended fix:** Add a §5.x table titled "Visible-QA-state matrix — wave obligations" with one row per wave (W2/W3/W4/W5), listing the states each wave's sub-L0 MUST enumerate before its implementation starts. The wave-level packet need not resolve every state — it needs to name the distinct ones so sub-L0 authors inherit a complete starting checklist. W2's list is already available from DOS-462; W3/W4/W5 lists can be drafted from the surface descriptions above. Estimated addition: 20–30 lines.

---

## Finding 2 — BLOCKING (Confidence 100)

**Dimension:** User flow completeness: 4/10 — it's a 4 because the packet describes the waves as parallel tracks (entity surfaces, then briefing surfaces, then action surfaces) with no named user journey that crosses them. A 10 would have at least one real customer-zero flow described end-to-end, naming which wave owns which moment of that flow, so the wave-boundary decisions are anchored to behavior.

**Section cite:** §4 scope summary table, §5.2–§5.4, §7 AC #W1, reorientation doc §v1.4.4.

**Design concern:** The wave staging (W2 entity before W3 briefing before W4 actions) is presented as an implementation ordering without a stated user-flow reason. For a surface-migration wave program, the ordering matters: if James (customer-zero) opens DailyOS after W2 ships but before W3 ships, the primary entry surface (Daily Briefing, `/` route) is still Tauri while entity-detail pages are WordPress blocks. That is an incoherent runtime experience — not just incomplete, but potentially trust-damaging if the two surfaces render the same account with different trust-band treatments while sitting in different rendering stacks.

More specifically: the natural customer-zero flow is: (a) open Daily Briefing → (b) see a meeting in the briefing → (c) click through to Meeting Detail to prep → (d) from meeting context, open the relevant Account Detail → (e) from account detail, see open actions → (f) act on an action. This flow crosses W3 (briefing), W3 (meeting detail), W2 (account detail), and W4 (action). The packet never names this flow or explains how the experience holds together during the W2/W3/W4 build-out period, when only some surfaces are on WP blocks and others are still Tauri.

Without the flow named, sub-L0 authors for W2 and W3 cannot make informed decisions about: navigation affordances from an entity-detail WP block back to a Tauri briefing surface; deep-link behavior when the floaty nav island is on a WP-rendered page but links to a Tauri route; or whether the wave ordering should be revised to ship Daily Briefing (W3) before entity surfaces (W2) to give the user a coherent "home base" earlier in the wave program.

This is not a request for a full UX spec. It is a request that the packet name the primary cross-wave user flow so wave boundary decisions are grounded.

**Recommended fix:** Add a §5.x subsection titled "Primary dogfooding flow across waves" that describes the customer-zero journey (briefing → meeting → entity → action), names which wave owns each step, and calls out the "mixed-surface" period during W2/W3 build-out. The section should make explicit whether flag-flip-at-W6 means both surfaces are simultaneously rendered (requiring navigation to work across stacks) or one is simply hidden (no cross-stack navigation required). This is the answer to the incoherence question, and the answer is latent in the flag-flip decision (locked §13 decision #5) — it just needs to be surfaced for the design context. Estimated addition: 15–25 lines.

---

## Finding 3 — ADVISORY (Confidence 50)

**Dimension:** Unresolved design decisions: 7/10 — it's a 7 because MePage's personal report entry points (`/me/reports/...`) are in the Active list (W5), but those routes point to surfaces deferred to v1.4.8. A 10 would be explicit about what the user sees when they navigate to a report entry point during and after v1.4.4 — placeholder? removed link? stub page?

**Section cite:** §5.5 (W5), §8 (out-of-scope deferrals), surface-audit.md Active list MePage row.

**Design concern:** MePage (`/me`) is active in W5 and contains "personal-report entry points" as part of its JTBD (INVENTORY.md). The report surfaces those entry points lead to (WeeklyImpact, MonthlyWrapped, BookOfBusiness) are deferred to v1.4.8. When the MePage WP block ships in W5 with real substrate, the question of what those report entry-point links do is not resolved. Options: (a) links are preserved pointing to the Tauri report surfaces (mixed-stack navigation, which the flag-flip decision implies is acceptable through W6); (b) links are suppressed in the WP block until v1.4.8; (c) links are present and lead to a "coming soon" WP block. Each has different UX implications. The packet does not name this choice.

This surfaces as advisory rather than blocking because: the flag-flip-at-W6 decision implies option (a) is the intended path (Tauri surfaces stay accessible until W6), which means the entry-point links simply continue pointing to Tauri routes through the migration window. If that is the intended behavior, the packet should state it explicitly to prevent W5 sub-L0 authors from guessing.

**Recommended fix:** Add a note to §5.5 W5 acceptance shape: "MePage report entry-point links remain as Tauri routes until v1.4.8 (mixed-stack navigation acceptable per flag-flip-at-W6 decision, §13 #5). W5 WP block does not suppress or stub these links." This resolves the ambiguity with one sentence.

---

## Finding 4 — ADVISORY (Confidence 50)

**Dimension:** AI slop check — no structural slop patterns found. The packet avoids generic SaaS phrasing ("user-friendly," "modern and clean") and grounds surface descriptions in real Tauri component paths, real DOS issue numbers, and real substrate calls. No 3-column feature grids, no stock UX language.

One bounded observation: the reorientation doc §v1.4.4 uses "Translation reality: most surfaces already exist as HTML + Tauri components. Migration is straightforward translation work" and the wave packet inherits this framing. This is accurate for the CSS/layout layer but undersells the interaction-reconfiguration work at W4 (inline edit affordances, contradiction UX, claim correction wire shape). The phrase "straightforward translation" could cause a W4 sub-L0 author to under-scope interaction work. Not a blocker — the specific W4 open questions (§5.4 inline-edit-affordance contract, DOS-318 contradiction UX) make the real scope visible — but the wave-level framing should not systematically undercount interaction work.

**No action required** — this is a reading-posture observation for sub-L0 authors entering W4.

---

## Dimensional ratings summary

- **Information architecture:** 9/10. Wave-by-wave staging (W2 entity → W3 briefing → W4 actions → W5 system) is coherent. The one-point gap: the staging rationale is engineering-sequence-driven, not user-flow-driven (Finding 2 addresses this).
- **Interaction state coverage:** 5/10. W2 matrix named by reference; W3/W4/W5 matrices absent (Finding 1).
- **User flow completeness:** 4/10. No named cross-wave user journey (Finding 2).
- **Responsive/accessibility:** 8/10. W2 sub-L0 reviewer matrix calls out `accessibility-tester` correctly; W4 does the same. No explicit keyboard nav or screen reader obligations at wave level — acceptable to defer to sub-L0 packets, but the wave packet should confirm the expectation is inherited rather than silent.
- **Unresolved design decisions:** 7/10. Report entry-point behavior during migration window (Finding 3); FloatingNavIsland/Gutenberg DOM compatibility (deferred to W3 sub-L0 — acceptable); parity-proof artifact format (deferred to W6 sub-L0 — acceptable).

---

## Conditions for APPROVE

1. **Finding 1:** Add a visible-QA-state matrix section enumerating, per wave (W3/W4/W5 at minimum), the states each sub-L0 MUST cover. W2 is already anchored by DOS-462; the remaining three waves need their own starting lists.
2. **Finding 2:** Add a primary-dogfooding-flow subsection naming the cross-wave journey (briefing → meeting → entity → action), the wave that owns each step, and whether mixed-stack navigation is the intended experience during the W2/W3 build-out window (confirm flag-flip semantics for sub-L0 authors).

Finding 3 is advisory and can be folded as a one-line note to §5.5 at the author's discretion.

No scope change is required. Both conditions are additive documentation of decisions that are either already locked (flag-flip §13 #5) or already present implicitly in DOS-462 and the reorientation doc.
