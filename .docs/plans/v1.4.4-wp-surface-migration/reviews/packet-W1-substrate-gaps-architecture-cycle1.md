# Architecture Review — v1.4.4 W1 Substrate Gaps L0 Packet — Cycle 1

**Reviewer:** ce-architecture-strategist
**Date:** 2026-05-20
**Packet under review:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` (V1.0)
**Scope:** Producer/projection/renderer split per ADR-0130; signals + invalidation per CLAUDE.md Intelligence Loop integration check; migration slot reservation v250–v269 against parallel waves; service ownership rule; W1 sub-lane sequencing; Read-ability non-mutation contract; envelope composition coherence (DOS-459 + DOS-460 + DOS-477 + DOS-507).

---

## VERDICT: BLOCKED

One critical migration-slot collision, one structural Intelligence-Loop integration gap, and one Read-ability non-mutation violation in the sequencing section. Several CONDITIONAL items the packet can resolve in V1.1 fold. Architecture intent is sound — the producer/projection/renderer split is honored cleanly and the envelope/receipt composition is internally consistent. The blockers are concrete and surgical, not directional.

---

## Findings

### F1 — CRITICAL — Migration slot block v250–v269 collides with v1.4.6 W1 reservation v260–v279

**Section cite:** §9 "Migration slots" — table claims v250–v269 (20 slots).

**Architectural concern:** v1.4.6-waves.md §327 + §414 ("Reserved block: **v260–v279** (20 slots)") reserves v260–v279 for the v1.4.6 Salience & Recommendations wave (`recommendation_claims`, `salience_factors`, `surfacing_decisions`, `triggers_log`, `deviation_baselines`, `engagement_telemetry`). v1.4.6 wave is already through cycle-4 of its plan with these slots load-bearing for W1-A/W1-B substrate. v1.4.6 §327 also explicitly states "v1.4.4 holds v240–v249" — i.e., the cross-version coordination table says v1.4.4 owns v240–v249, NOT v250–v269. This packet claims v250–v269 unilaterally, which collides with v260–v269 on the v1.4.6 side AND breaks the cross-version coordination table v1.4.6 already committed to.

The packet's §9 reasoning ("v241–v249 unused in practice and remain reserved for W1-RECEIPT follow-on") is internally coherent but inconsistent with the cross-version reservation table on v1.4.6. v1.4.4 expanded its block past v249 without an amendment to v1.4.6's coordination table or to the v1.4.x maintenance-waves doc (per CLAUDE.md "Parallel-wave migration slot reservations").

**Recommended fix:** Claim v240–v249 (the cross-version-coordinated block) — the dissolved-v1.4.4 archive ALREADY reserved v240–v249 (DOS-701 took v240; v241–v249 are 9 unused slots in the original block); W1 needs only 2 migration slots in practice (v241 + v242 for DOS-335). If 9 slots is insufficient, file an amendment to BOTH this packet AND `.docs/plans/v1.4.6-waves.md` §327 + §414 reserving a non-overlapping block (e.g., v280–v289, the next clean window past v1.4.6's v279). Slot block change requires updating both wave plans + (per CLAUDE.md §"Parallel-wave migration slot reservations") posting in the v1.4.x maintenance-waves coordination doc before claiming.

---

### F2 — CRITICAL — Read-ability non-mutation contract violation in §5.5 DOS-335 sequencing

**Section cite:** §5.5 DOS-335 — `services::meeting_prep_status` exposes `compute_status` (read), `enqueue_refresh` (write), `record_user_authored` (write). §13 Q7 ("concurrent writer ordering") confirms there are TWO write paths. §13 Q9 confirms `get_daily_briefing` is Read-only.

**Architectural concern:** §5.5 declares the meeting prep status DTO is consumed by `get_daily_briefing` (Read ability) per §5.10. But the meeting prep status SERVICE has write paths (`enqueue_refresh`, `record_user_authored`). ADR-0102 §3 is explicit: "Read ability: No service mutation **anywhere in the call graph**. May emit ephemeral logs and telemetry but never writes domain state or emits propagating signals." If `compute_status` lazily calls `enqueue_refresh` when status would resolve to `Stale`, the briefing ability's transitive service-call graph includes a mutation, and the call-graph check at registration time per ADR-0102 §3 ("the registry refuses to bind the ability") will reject `get_daily_briefing` as a Read.

The packet partially anticipates this in §13 Q9 ("auto-enqueue prep, or strictly return the state?") and recommends Read-only with W3 surface handling the enqueue separately, but DOES NOT lock the contract: §5.5's `compute_status` signature is not annotated `#[read_only]` / `must_not_mutate`, and there is no static assertion that the briefing's call into `compute_status` cannot transitively reach `enqueue_refresh`. As written, an L1 implementer could plausibly add lazy-refresh-on-stale to `compute_status` (a reasonable performance pattern) and silently violate ADR-0102 §3.

**Recommended fix:** In §5.5, explicitly split the service module:
- `services::meeting_prep_status::read` (pure read; no `&mut`; no signal emit; safe for Read-ability call graph)
- `services::meeting_prep_status::write` (`enqueue_refresh` + `record_user_authored`; called only from non-Read paths)

Add an acceptance criterion AC-335.12: "`compute_status` call graph contains zero mutations; verified by a trybuild test or call-graph lint at registration time per ADR-0102 §3." Land the static check as part of W1, not deferred. Mirror in §5.10 AC-507.7 (already lists `no-write` test but doesn't bind it to the call-graph mechanism).

---

### F3 — HIGH — Signals + invalidation answer is load-bearing missing for DOS-335 prep status writes

**Section cite:** §5.5 DOS-335 Intelligence Loop check item 3 ("Signals + invalidation"): "New invalidation path: entity-link signal (DOS-258) → `enqueue_refresh(RefreshReason::EntityRelinked)`. Recent-correction signal → `StaleReason::RecentCorrection`. **No new signals emitted; consumes existing.**"

**Architectural concern:** Per CLAUDE.md Critical Rule, every new table/schema-column/claim field/user-visible intelligence surface must answer "What signals does it emit, and which propagation/invalidation paths refresh derived state?" The packet says "no new signals" — but DOS-335 lands TWO new migrations (v250 indexed view, v251 dismissals table) AND introduces a new lifecycle state (`UserSuppressed` / `UserDismissed`). When a user dismisses prep status, downstream consumers (FolioBar readiness chrome — substrate-rendered per W3 chrome lane; Meeting Briefing block; Daily Briefing rollup) need to know the dismissal happened so they re-render. If "no new signals" is literally true, the only refresh path is poll-on-render, which means FolioBar will show stale prep status until next FolioBar tick.

This is the same pattern as ADR-0080's signal-driven invalidation: a state change without a signal emission means consumers either poll or drift. Neither is acceptable for surface-rendered chrome.

**Recommended fix:** Either (a) add a signal — `MeetingPrepStatusChanged { meeting_id, transition: PrepStatus → PrepStatus }` — pre-declared in `signals/policy_registry.rs` at W1 kickoff (mirror the v1.4.6 W0 cycle-4 fix C pattern at v1.4.6-waves.md §348); or (b) document the explicit poll-on-render contract with cycle bounds (FolioBar tick interval guarantees ≤ N seconds staleness) and surface this in §5.5 + §10 invariants. Option (a) is the substrate-coherent answer; option (b) ships drift. The 5-question gate per CLAUDE.md cannot pass with "no signal" if there's user-visible state to refresh.

---

### F4 — HIGH — Receipt emission (DOS-339) Intelligence Loop signal answer is under-specified

**Section cite:** §5.6 DOS-339 Intelligence Loop check item 3: "Receipt state updates on claim lifecycle / feedback / contradiction / source state changes."

**Architectural concern:** The receipt is the shared DTO consumed by every W2–W5 surface. When a feedback action (DOS-8) fires → `record_claim_feedback` writes a row → claim verification state transitions → trust band recomputes → receipt SHOULD re-emit to all subscribers. The packet asserts the chain exists via "existing claim-lifecycle signals" but does NOT name (a) the specific signal type that triggers receipt re-emit, (b) the policy-registry entry that fans the signal out to receipt subscribers, or (c) what coalescing rule prevents N feedback actions on the same claim within 100ms from generating N receipt re-emits to the same WP block render PHP. Without (c), the W2/W4 surfaces will receive thundering-herd re-renders during burst feedback (e.g., user marks 5 stale claims in rapid succession in Actions/Work).

The packet's §5.7 DOS-8 contract returns the updated receipt in the response envelope (AC-8 not numbered for this; visible in `ClaimFeedbackResponse.receipt`) — which is the right pattern for the actor's own surface — but does NOT specify the signal-driven re-emit path for OTHER surfaces showing the same claim (e.g., Account Detail block showing the same claim that Actions/Work just gave feedback on).

**Recommended fix:** In §5.6, name the signal type explicitly (existing or new — likely existing `ClaimVerificationStateChanged` per ADR-0080 substrate; verify at L1). Specify the coalesce policy on the receipt-subscription hook (AC-339.1 already references `useClaimReceiptSubscription`; bind the coalesce window — recommendation: 250ms trailing-edge debounce per `(target.claim_id, surface)` pair). Add AC-339.6: "Multi-surface fan-out tested: feedback action on claim C from Actions/Work re-emits receipt to all subscribed Entity Detail / Briefing / Activity Log surfaces within 1 invalidation cycle; coalesce window prevents > N re-emits per (claim, surface) per second."

---

### F5 — MEDIUM — Envelope shape coherence: DOS-459 envelope + DOS-460 list-shape pagination + DOS-507 BriefingState compose without conflict, but the cursor decision creates a missing AC

**Section cite:** §13 Q11 + §13 wave-level locked decisions ("Pagination is server-side via cursor. Every list-shape envelope (DOS-459 entity list variant, DOS-460 touchpoints/open-loops list) MUST return `next_cursor` from day one").

**Architectural concern:** The DTOs in §5.1 (`EntityIntelligenceEnvelope`) and §5.2 (`TouchpointBundle`, `OpenLoopWithReceipt`) do NOT carry a `next_cursor` field. §5.1's envelope ships `touchpoints: TouchpointBundle` and `open_loops: Vec<OpenLoopWithReceipt>` as plain collections. Per the wave-level decision, both must be paginated. Either (a) the inner types need to be `Paginated<T>` wrappers carrying `{ items, next_cursor }`, or (b) the envelope returns the first page inline and the W2 block re-invokes the ability with a continuation cursor for subsequent pages. The packet locks the decision in prose (§13 wave-level constraints) but doesn't reflect it in the DTO sketches in §5.1 / §5.2 — which is the part reviewers verify at L0 plan.

DOS-507 (`get_daily_briefing`) ships `upcoming_meetings: Vec<MeetingBriefRef>` — same shape gap; daily briefing's upcoming meetings list is a candidate for cursor pagination too.

**Recommended fix:** Update §5.1 envelope DTO sketch: change `touchpoints: TouchpointBundle` → `touchpoints: Paginated<TouchpointBundle>` (or move `next_cursor: Option<Cursor>` onto the bundle itself). Same for `open_loops`, `metadata_proposals`, `record_entries`, `threads`. Add AC-459.9: "Every list-shape field in the envelope carries a typed cursor; consumer pagination tested via re-invocation; cursor is opaque server-signed (per §13 Q11 default) and survives schema changes." Add equivalent AC-507.10 for `upcoming_meetings` in DOS-507. Without this, W2 block render PHP will read the envelope and assume the full set is returned — silently breaks on large datasets.

---

### F6 — MEDIUM — Sub-lane sequencing in §13 Q10 has a dependency gap

**Section cite:** §13 Q10 — Stage 1a parallel (§5.6 wiring + §5.5 + §5.1), Stage 1b parallel after 1a (§5.7 + §5.8 + §5.9 + §5.2 + §5.4), Stage 1c parallel after 1b (§5.3 + §5.10).

**Architectural concern:** §5.4 (DOS-477) is placed in Stage 1b, but §5.4 `validate_envelope_target` is called by §5.7 (DOS-8) as the first step of `submit_claim_feedback` (see code sketch in §5.7 lines 599-606: "1. validate target via services::entity_intelligence::auth::validate_envelope_target"). If §5.4 and §5.7 ship in the same stage, §5.7's L1 will compile-fail until §5.4 lands. Either (a) §5.4 moves to Stage 1a (so the boundary helper exists before consumers wire it), or (b) the two are explicitly serialized within Stage 1b. The packet says "parallel after 1a" — implying they can interleave — but the dependency edge §5.7 → §5.4 is unidirectional.

Similar but smaller gap: §5.10 DOS-507 (Stage 1c) depends on §5.5 DOS-335 — packet calls this out (§13 Q10 last bullet) — but §5.5 ships migrations v250-v251 in Stage 1a; if §5.5's migrations land in Stage 1a but the DTO contract isn't frozen until Stage 1a closes, §5.10 can't start until 1a fully closes. The serial dependency is fine; the packet should make the migration-vs-contract-freeze boundary explicit.

**Recommended fix:** Move §5.4 (DOS-477) from Stage 1b to Stage 1a (it's a substrate primitive consumed by Stage-1b items; not a consumer itself). Restate the staging:
- **Stage 1a (parallel):** §5.6 wiring, §5.5, §5.1, §5.4
- **Stage 1b (parallel):** §5.7, §5.8, §5.9, §5.2 (composes envelope from §5.1), §5.3 (harness needs envelope + boundary)
- **Stage 1c:** §5.10 only (depends on §5.1 + §5.5)

Add an AC-W1.9 documenting the staging in the acceptance section so wave-coordinator agents follow the order at L1 dispatch.

---

### F7 — LOW — `get_entity_intelligence` envelope and `get_entity_context` coexistence creates duplicate-source-of-truth risk

**Section cite:** §13 Q1 + Q6 — both ask the question, both recommend coexistence with v1.5.x consolidation.

**Architectural concern:** Per ADR-0102 §1 ("the abilities layer is the runtime contract of the product") + ADR-0130 §1 ("the substrate owns composition"), having two abilities that produce entity context (`get_entity_context` for provider prompts; `get_entity_intelligence` envelope for surfaces) means two source-of-truth paths for "what does the substrate believe about this entity right now." When the two drift (which they will — they'll have different empty-state vocab, different freshness thresholds, different sensitivity-redaction defaults), users and provider prompts will see different views of the same entity, which is the exact failure mode ADR-0102 §Context names. The packet's recommendation is "coexist + file v1.5.x consolidation ticket" — that's reasonable as a phased migration but the L0 packet doesn't actually file the v1.5.x ticket OR document the drift-detection mechanism for v1.4.4 lifetime.

**Recommended fix:** Either (a) make `get_entity_context` a *thin internal projection* of `get_entity_intelligence` (the envelope ability is the producer; `get_entity_context` becomes a projection that returns the legacy shape from the envelope — single source of truth, two consumer shapes); or (b) accept coexistence and add AC-459.10: "Drift detection — `tests/abilities/entity_context_envelope_parity.rs` asserts that the field-level overlap between `get_entity_context` and `get_entity_intelligence` outputs is stable; new fields on either side fail the test until both are updated. v1.5.x consolidation ticket FILED IN LINEAR as part of W1 retro, not deferred." Recommendation: (a) is the ADR-0102-coherent answer; (b) is the path-α maintenance answer per memory `feedback_l2_path_alpha_to_maintenance_project`. Reviewer call.

---

### F8 — LOW — Service ownership rule satisfied; spot-check passes

**Section cite:** §10 invariant 1 ("All mutations route through `services/`") + §5.5 (DOS-335 writes go through `services::meeting_prep_status`) + §5.7 (DOS-8 routes via `services::claims::record_claim_feedback`) + §5.6 (DOS-339 wiring uses `services::claim_receipt::*`).

**Architectural concern:** Spot-check passes for all three named sub-tickets — DOS-335 prep status writes route through service-owned `record_user_authored` + `enqueue_refresh`; DOS-8 feedback writes go through existing `services::claims::record_claim_feedback` per code sketch §5.7 line 605; DOS-339 receipt emission uses `services::claim_receipt::render::render_receipt_for`. No direct DB writes proposed in any sub-ticket.

Caveat: §5.7 mints `idempotency_key` server-side (good — caller-supplied rejected per AC-8.2) but the mint location is not specified. If the key is minted in `submit_claim_feedback` BEFORE the service call, the mint is in an ability/command (acceptable — it's not a DB mutation, it's a key generation). If it's minted inside the service, it's also fine. Specify in V1.1 fold for clarity, but not a finding.

**Recommended fix:** Annotate §5.7 to specify idempotency_key mint location (recommend: inside the service, before the `record_claim_feedback` call, so retry logic in the ability layer sees a stable key). Not a blocker.

---

### F9 — LOW — Producer/projection/renderer split per ADR-0130 honored cleanly

**Section cite:** §10 invariant 6 ("Producer→projection→renderer split") + §6 substrate-consumed table row "ADR-0130 producer→projection→renderer composition contract — Every sub-ticket §5.1–§5.10 respects the split."

**Architectural concern:** ADR-0130 §1 + §5 require composition-producing abilities to return `AbilityOutput<Composition>` and surfaces to consume the composition via renderers. `get_entity_intelligence` (§5.1) returns `EntityIntelligenceEnvelope` (NOT `Composition`), and `get_daily_briefing` (§5.10) returns `DailyBriefingOutput` (NOT `Composition`). This is intentional — neither ability is composing a Gutenberg-block-shaped page; they're returning typed envelopes that block-render PHP projects into Gutenberg blocks. Per ADR-0130 §5, only abilities that return composed pages need the `Composition` type. The W1 abilities are envelope producers; the W2/W3 block-render PHP is the projection layer; the Gutenberg blocks are the renderer. Clean.

ADR-0130 §2 amendment (ProvenanceRef) applies to compositions, not envelopes — but the same "provenance lives once" invariant applies to `AbilityOutput<T>` per ADR-0102 §6. The packet's envelope sketches show provenance on individual `EntityFact` items (§5.1 line 209 "every EntityFact carries source_asof, trust_band, freshness, and provenance: ProvenanceSource[]"). This needs the same `ProvenanceRef` treatment to honor ADR-0102 §6 + ADR-0105 §8 lives-once.

**Recommended fix:** In §5.1, retype `EntityFact.provenance: Vec<ProvenanceSource>` and `OpenLoopWithReceipt.provenance: Vec<ProvenanceSource>` (§5.2) to `provenance: ProvenanceRef` (per ADR-0130 §2 amendment). Top-level `EnvelopeProvenance` stays on the `AbilityOutput<EntityIntelligenceEnvelope>` wrapper, not duplicated per-fact. This is a substrate-coherent fix that prevents the 64KB serialized-provenance-cap blowup ADR-0108 names. Not a blocker for V1.0 → V1.1, but should land before L1 implementation begins.

---

## Summary

**Block on:** F1 (migration slot collision), F2 (Read-ability call-graph violation), F3 (DOS-335 signals gap).

**Resolve in V1.1 fold:** F4 (receipt fan-out coalesce), F5 (cursor in DTO sketches), F6 (sub-lane sequencing fix), F9 (ProvenanceRef on per-fact provenance).

**Path-α candidates for maintenance ticket per memory `feedback_l2_path_alpha_to_maintenance_project`:** F7 (envelope/context coexistence drift detection), F8 (idempotency_key mint location).

**What the packet got right:**
- K-in record is thorough (11 ADRs consumed verbatim; 4 cross-references from `docs/solutions/` cited; no documented prior substrate reinvented).
- Producer/projection/renderer split per ADR-0130 honored cleanly across §5.1–§5.10.
- "Wiring IS the work" obligation in AC-W1.1 + AC-W1.2 enforces the substrate→consumer-skeleton dependency.
- Reviewer panel scoping (default 4-panel + `/cso` per-sub-ticket) correctly distinguishes read-path from boundary-crossing work per engineering-ladder.md Amendment 3.
- Substrate-consumed table in §6 anchors every reused primitive to a shipped location.
- §13 explicitly carries the wave-level locked decisions (cursor pagination, DOS-336 deferred, Tauri flag-flip at W6, 1-outer + N-inner block shape) — these will land as V1.1 fold answers, not new findings.

Re-dispatch after V1.1 fold addresses F1 + F2 + F3.
