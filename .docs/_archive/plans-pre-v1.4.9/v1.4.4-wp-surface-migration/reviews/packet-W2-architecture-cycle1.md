# W2 Entity Surfaces — Architecture Review (L0 cycle 1)

**Reviewer:** ce-architecture-strategist
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md` V1.0 (2026-05-21)
**Base branch:** `wave/v1.4.4-w1-stage1a` HEAD `deb682b0`
**Verdict:** **APPROVE with 2 MEDIUM findings + 1 LOW finding** (no BLOCKING).

W2 is a renderer-only sub-wave whose architectural skeleton is fully inherited from wave §10 invariants and W1 substrate landings; the packet does not reinvent or contradict those locks. The 6-area cross-check below surfaces only refinement-class findings that fit path-α / inline-AC remedy — none rises to a C4 substrate reopen.

---

## Area 1 — Outer/inner block contract (ADR-0130 §4 Reading A)

**Assessment: COMPLIANT.**
- §5.1 block.json shows `parent: null`, `providesContext` for `entityType` / `entityId` / `envelopeHandle`, `apiVersion: 3`. Inner blocks declare `usesContext` for the same triple, NO `parent` field (inserter-global per ADR-0129 §2). §10 application row "Outer/inner block contract" + AC-462.2 + AC #W2.3 + AC #W2.6 carry this consistently.
- `do_blocks($content)` discipline named in §10 application row. `templateLock: false` named at AC-462.8 + Q1.
- `Composition.sections[].blocks[]` 1-to-1 mapping is visible in §5.1 chapter cluster comments (Health = section[0], Context = section[1], Work = section[2], Record = section[3]).

**Finding A1 (LOW):** `templateLock: false` and the default `template` array are named in AC-462.8 + Q1 but NOT shown in the §5.1 block.json sketch. **Remedy:** add the `template` array to the block.json sketch in §5.1 explicitly (illustrative; the four composites share the shape). Path-α inline edit, not a re-review.

## Area 2 — W1 substrate consumption (3-arg invoke_ability)

**Assessment: COMPLIANT.**
- §5.1 render-functions.php contract codifies the 3-arg `invoke_ability($producer, $payload, $scope_set)`; "A 2-arg invocation FAILS `check_w1_consumer_skeleton.sh`" stated explicitly. AC #W2.4 inherits as a CI gate.
- §6 substrate-reuse table cites concrete SHAs for every W1 producer: DOS-459 envelope (`2b3915ef`), DOS-460 touchpoints (`6444568c`), DOS-477 envelope-set validation (`e9b0ed41` + `7bbbc6f7`), DOS-339/701 receipts (`0243df65` + `0a586218`), DOS-340 boundary (`e74d49dc`), DOS-341 `build_receipt_for_audience` (`e4d72de6` + `6b437c5f`), DOS-8 typed feedback (`7bbbc6f7`).
- §5.6 metadata-proposal accept/dismiss/edit binding to DOS-477 `validate_envelope_target` is correct: AC-328.5 enforces. Class-sweep gate per K-in solution `prompt-channel-sensitivity-class-sweep-2026-05-18.md` applied at AC-328.9.

## Area 3 — List shell pagination (`useAbilityCursor()`)

**Assessment: COMPLIANT.**
- Hook spec authored at §5.5: signature, `ListEnvelope<T>` shape `{ items, next_cursor, total_hint }`, watermark-driven reset, opaque cursor server-encoded. Matches wave §10 invariant "Entity list pagination contract" verbatim.
- Location `wp/dailyos/blocks/_shared/hooks/useAbilityCursor.ts` is the shared lane (not per-block), satisfying the wave §10 + anchored decision #2 demand for a single hook authority.
- AC #W2.2 forbids `useEntityRecords` explicitly; AC-L.2 reinforces per-block.

**Finding A2 (MEDIUM):** the packet does not declare whether `useAbilityCursor` is **authored in W2** or inherited from a prior stub. Grep `wp/dailyos/blocks/_shared/` was not performed in §3 K-in. **Remedy:** add a one-line K-in clause to §3 confirming "no prior `_shared/hooks/useAbilityCursor.ts` exists at branch HEAD; W2 authors it." If it does exist, cite the SHA. Trivial fix; path-α.

## Area 4 — DOS-725 project tint (CSS custom property, not theme.json palette)

**Assessment: COMPLIANT.**
- §5.2 + AC #W2.7 + §10 application row all carry the same rule: `--dailyos-project-tint: var(--color-garden-olive)` inline on outer wrapper as the narrow CSS-custom-property exception per memory `feedback_no_inline_css`. NOT a theme.json palette slug. ADR-0077 amendment ticket filed at L1.
- `chrome_config()` `$stub_tints['dailyos_project'] = 'olive'` ships in same commit window (AC-483.3). Matches W1 V1.1 §13 decision.

## Area 5 — W1→W2 substrate sufficiency (spot-checks)

**Assessment: SUFFICIENT.** Three spot-checks:
- **Trust band rendering** — consumed via TrustBandBadge primitive (v1.4.3 W2 lineage; DOS-692 a11y patch this wave). Envelope source: `EntityFact.trust_band` per ADR-0125. No producer gap.
- **Provenance display** — ProvenanceTag (DOS-691 envelope wiring) + `ProvenanceRef` from envelope, actor-filtered via DOS-341 `build_receipt_for_audience`. ADR-0108 actor-filtered render projection. No producer gap.
- **Entity-link chip** — EntityChip (v1.4.3 W2 primitive). Envelope source: `SubjectRef` resolved via DOS-459 envelope facts. No producer gap.

**Finding A3 (MEDIUM):** Meeting Detail (§5.4) names `EntityIntelligenceEnvelope` slices `agenda`, `attendees`, `related_entities`, `context_bundle`, `post_meeting_capture`, `claims_for_review`. The packet defers verification to L1 (Q4: "Verify at L1 kickoff" whether `get_entity_intelligence` produces a meeting-shaped envelope). This is the right escalation rule (Q4 invokes C4 if a gap is found) but the packet should **bind the verification to L1 kickoff Day 1**, before any Meeting Detail inner-block coding starts, to avoid mid-implementation reopen. **Remedy:** strengthen Q4 to "L1 kickoff first action: verify meeting envelope shape via `get_entity_intelligence` integration test; if gap, halt §5.4 and reopen W1 per C4."

## Area 6 — Forward-coupling to W3 / W4

**Assessment: PROPERLY LOOSE.**
- W3 forward-coupling: §5.4 names FolioBar readiness signal as a chrome-lane primitive driven by `MeetingPrepStatusChanged` (chrome.js consumer), NOT a W2 push surface. §10 row "Refresh model" treats this as inheritance-with-exception, not a new contract. W3 stays unblocked.
- W4 forward-coupling: AC #W2.11 inline-edit-affordance contract emits `FeedbackAction` through `record_claim_feedback`; the actual W4 action surfaces (Actions / Activity Log / Action Detail / Lint Mode / review queue) stay out of scope (§8 row). No premature wiring.
- Person merge picker (§5.3): path α (`MergeIntent` feedback) intentionally avoids a substrate write commitment; path β escalation rule is preserved as wave AC #W5 C4 — clean forward-decoupling.

---

## Recommendation

**APPROVE.** All 3 findings are path-α inline edits to the V1.0 packet — none requires another full reviewer cycle. Author should:
1. Fold A1 + A2 + A3 as a V1.1 amendment (single commit) before unanimous L0 close.
2. Reviewer panel ratifies V1.1 by inspection; no second-cycle review needed for this reviewer.

**No BLOCKING findings.** No C4 substrate reopen. W1 → W2 substrate boundary is intact and complete.

**Class-pattern watch (per memory `feedback_systemic_look_for_recurring_issue_classes`):** A1 + A2 are both "spec named in prose but not in code-shape sketch" — a single class. If another reviewer surfaces a third instance, switch from inline-fix to a class-wide pass over §5 sketches.
