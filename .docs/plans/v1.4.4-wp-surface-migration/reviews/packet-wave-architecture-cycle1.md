# L0 Architecture Review — v1.4.4 Wave Plan (cycle 1)

**Reviewer:** ce-architecture-strategist (System Architecture Expert)
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` (V1.0, 2026-05-20)
**Anchors read:** ADR-0129, ADR-0130 (full), wp-foundation-roadmap-reorientation.md, L0-packet-W3-chrome-lane-pulled-forward.md (V1.3.1), L0-packet-W1-substrate-gaps.md (V1.0), CLAUDE.md §Critical Rules.
**Date:** 2026-05-20

## VERDICT: CONDITIONAL APPROVE

Wave architecture is sound. The producer/projection/renderer split is honored, substrate-in-same-wave (C4) is operationalized in W1 against real Linear scope, and the four locked architectural decisions (§13) are individually defensible. Findings below are corrections that must land in V1.1 before sub-L0 packets author against this wave plan — none require restructuring the wave program.

The most load-bearing concern is **F1 (HIGH)**: the locked outer/inner-block decision needs an explicit ADR-0130 mapping so W2/W3/W4 sub-L0 authors don't drift into block-tree-as-document-model territory. Everything else is either tightening (F2–F5) or substrate consistency (F6–F8).

---

## Findings

### F1 — HIGH — §13 item #1 (outer/inner block decision) lacks ADR-0130 mapping

**Architectural concern.** The locked decision "one outer `dailyos/account-detail` block + N inner pieces" is presented as a UX/granularity decision, but it is actually a **producer/projection/renderer boundary decision**. ADR-0130 §2 + §4 + §5 say:

- Compositions are produced by abilities returning `AbilityOutput<Composition>`.
- Surfaces render `Composition.sections[].blocks[]`; surfaces do not author composition.
- Each `Block` is a typed `BlockType` from the ADR-0130 §3 taxonomy.

The packet's outer/inner decision sits in tension with this unless the wave plan names which side of the producer/renderer seam owns the outer block:

- **Reading A (renderer-side outer):** the `dailyos/account-detail` Gutenberg block is a Gutenberg-only convenience wrapper around the substrate's `Composition { kind: EntityPage }`. Its `InnerBlocks` are the per-`Block` Gutenberg projections. Outer block carries NO substrate semantics beyond invoking the producer and laying out the resulting sections. **This is the only reading consistent with ADR-0130 §4 ("Renderers are surface-side code. They do not modify the composition; they project it.").**

- **Reading B (producer-side outer):** the substrate exposes an "account-detail composition" whose sections are the chapters and whose `Block`s are inner blocks. The Gutenberg outer block is a 1-to-1 projection of the top-level Composition. This is fine too — but it requires `CompositionKind::EntityPage` and inner blocks to map cleanly to the BlockType taxonomy, which §5.2 of the packet leaves implicit.

The packet picks neither reading explicitly. Sub-L0 authors will fork on this — W2 author may treat the outer block as a substrate primitive (B), W3 author may treat the outer briefing block as a Gutenberg convenience (A), and the resulting blocks won't share a contract.

**Recommended fix.** Add a sentence to §10 invariants and to §13 item #1: "The outer composite block is a Gutenberg-side projection of `AbilityOutput<Composition>` per ADR-0130 §4. The outer block's `InnerBlocks` correspond 1-to-1 with the `Composition`'s `sections[].blocks[]`. The outer block's `render.php` invokes the composition-producing ability via SurfaceClient; per-inner-block `render.php` projects a single `Block` per its `BlockType`. **The substrate does not know about Gutenberg outer-vs-inner; the renderer does.**" This is Reading A and it's the only one that preserves ADR-0129 §1's "no surface is sacred" + ADR-0130's renderer-not-author boundary.

### F2 — HIGH — Cursor-pagination decision (§13 item #2) under-specifies envelope contract impact

**Architectural concern.** §13 locks "server-side pagination via cursor" for list shapes, and notes "W1 envelope contract for list shapes (DOS-459 / DOS-460) carries `cursor` + `next_cursor` from day one." This is the right direction, but the wave packet doesn't surface the architectural implication:

1. **`Composition` model has no cursor field today.** ADR-0130 §2 `Composition` carries `sections`, `salience`, `generated_at`, `generated_by` — no pagination primitives. Adding `cursor` / `next_cursor` to `Composition` is an ADR-0130 amendment, not a W1 ticket detail.

2. **OR** the envelope is NOT a `Composition` — it's a typed list payload that lives outside Composition. In that case, DOS-459 / DOS-460 envelopes are read-ability outputs that the W2 entity-detail composite **does not consume via the Composition path**. That fragments the producer/projection/renderer contract: some W2 inner blocks render `Composition.Block`s, others render typed entity-intelligence envelopes. Sub-L0 authors will not know which.

3. **The "fixture passes the v1.4.3 W1 starter kit harness" acceptance in §5.1 + AC #W2 only covers Composition-shaped output.** Cursor-paginated list shapes need their own fixture variant or the harness can't validate the contract end-to-end.

**Recommended fix.** Add §10 invariant: "List-shaped substrate (entity list shells, touchpoints feed, open-loops feed, activity log) uses cursor-paginated `AbilityOutput<List<T>>` envelopes distinct from `AbilityOutput<Composition>`. Composite blocks consuming list shapes invoke the list ability separately from the entity-detail Composition ability. Per-ADR amendment to ADR-0130 §2 OR a sibling ADR is filed if `Composition` itself needs a cursor primitive (decided at W1 sub-L0 with `/codex consult` + this reviewer)." Without this, the cursor-pagination decision drifts into "W2 invents its own pagination" and the contract carries no weight.

### F3 — MEDIUM — Tauri shell flag-flip at W6 (§13 item #5) needs runtime-host carve-out

**Architectural concern.** The decision "single PR hides Tauri React UI behind a build flag" is right at the surface tier. But the packet's §10 invariant "Runtime stays side-process (C3)" + memory `feedback_tauri_ui_freeze` carve out that **Tauri continues hosting runtime + MCP server + keychain + dev/admin surfaces.** The W6 sub-L0 will have to draw a line between:

- **UI to flag-flip** — `src/pages/*` magazine surfaces, entity detail pages, briefing pages, action surfaces.
- **UI to KEEP visible** — runtime-host dev/admin surfaces (ability runtime debugging, keychain state, MCP server status, settings runtime-tier).

The packet does not name this boundary. A naive flag-flip that hides ALL Tauri React UI breaks the runtime-host role; a flag-flip that hides "magazine surfaces only" needs an explicit list at W0 (it overlaps with the W0 surface audit's "active vs carry-forward vs inactive" classification but the audit doesn't currently distinguish dev/admin from end-user).

**Recommended fix.** Amend §5.0 (W0 audit outputs) to add a fifth list: **Dev/admin surfaces** — surfaces that stay visible post-flag-flip because they're runtime-host concerns, not end-user surfaces. Settings-runtime, keychain debug, ability runtime status, MCP server status. These are NOT migration targets and they are NOT flag-flip targets. Reference this carve-out in §13 item #5: "Flag-flip hides W0-classified *end-user* Tauri React UI only. Dev/admin surfaces stay visible and are explicitly out of v1.4.4 scope."

### F4 — MEDIUM — Substrate-in-same-wave (C4) is asserted but not enforced at the wave-packet level

**Architectural concern.** AC #W2 + AC #W5 + §10 invariant row 1 all repeat the rule: "If a W2/W3/W4/W5 sub-L0 packet names a substrate item not in v1.4.0–v1.4.3 + v1.4.4 W1 inventory, that item lands in W1 same-wave, not deferred." But the only enforcement mechanism named is "CI gate: per-block integration test against the v1.4.3 starter kit harness must pass with realistic substrate fixture, not stub."

The harness validates that producer→projection→renderer works for the substrate that EXISTS at sub-L0 time. It does not enforce that a gap surfaced mid-W2 round-trips back to W1 instead of becoming a "tracked for v1.4.5" deferral. Memory `feedback_no_deferrals_period` says deferrals aren't an escape valve, but the packet doesn't operationalize that into a sub-L0 acceptance criterion.

**Recommended fix.** Add to AC #W5: "Mid-wave substrate gap escalation procedure: when a W2/W3/W4/W5 sub-L0 packet surfaces a substrate gap not in the W1 inventory, the gap is added to W1 scope via W1 sub-L0 amendment (not as a new Linear ticket in v1.4.5+). The downstream wave sub-L0 packet documents the W1 amendment reference and the unblock criterion. **Reviewer panel rejects sub-L0 packets that defer gaps to v1.4.5+ for substrate v1.4.4 surfaces consume.**" This makes C4 enforceable by the L0 reviewer panel, not just by a CI harness that can't see deferred work.

### F5 — MEDIUM — §6 Substrate inventory omits ADR-0080 signal propagation cleanly

**Architectural concern.** §6 lists substrate from v1.4.0/v1.4.1: "Signal propagation + invalidation per ADR-0080." Good. §10 invariant "Producer commit on cache miss (v1.4.3 carried forward). No signal-propagation invalidation bus required at v1.4.4 scope." Also good in isolation.

But these two statements together leave W3 briefing surfaces and W2 entity surfaces in an unspecified state with respect to **freshness signaling**. The Intelligence Loop integration check (CLAUDE.md) requires every claim-bearing surface to answer "What signals does it emit, and which propagation/invalidation paths refresh derived state?" If v1.4.4 explicitly says "no signal-propagation bus required," then the answer to that question for every W2/W3/W4 composite block is "producer re-runs on cache miss; no push-invalidation." That's a real architectural posture — but it should be stated as such, because consumers (and L2 reviewers) will look for signal-propagation paths and find none.

**Recommended fix.** Add to §10 invariants: "Refresh model is producer-pull on cache miss + explicit user refresh action; no push-invalidation bus consumed at v1.4.4. W2/W3/W4 composite blocks invoke the producing ability on render; staleness is conveyed via `FreshnessIndicator` primitive (v1.4.3 shipped) and trust-band downgrade per ADR-0105. **The Intelligence Loop §3 (signals + invalidation) question is answered uniformly across v1.4.4 blocks: 'pull on render + user-initiated refresh; no push invalidation at this version.'** Signal-propagation-driven refresh is a v1.4.6 Salience concern (signal correlation drives recommendations) or v1.4.9 Self-Healing concern."

### F6 — MEDIUM — Forward-coupling risk with v1.4.5/v1.4.6 substrate (§9 + §13 item #6)

**Architectural concern.** §9 says "v1.4.5 Workspace Memory and v1.4.6 Salience also need migration slots in the same v1.4.x sequence. v1.4.4 W1 sub-L0 must coordinate slot blocks with v1.4.5/v1.4.6 pre-L0 to avoid the v1.4.1 W3-C/W4-A/W4-B v155 collision pattern." Good. §13 item #6 says "v1.4.6 lands the [DOS-336 candidate extension] hook itself. W1 leaves claim review queue alone."

Two forward-coupling risks the packet does not name:

1. **Schema-stability of the W1 envelope (DOS-459) across v1.4.5 + v1.4.6.** v1.4.5 Workspace Memory adds source/ingestion claims; v1.4.6 Salience adds `RecommendationProposal` claims. Both will want to appear inside the `get_entity_intelligence` envelope shipped at v1.4.4 W1. The envelope's `schema_version` lifecycle (named in §5.1 as a W1 open question) is the forward-coupling pivot. If W1 picks "major-version-only ABI," v1.4.5 + v1.4.6 will need a major-version bump on the envelope they didn't ship. If W1 picks "semver," consumers need to negotiate. **This is an architectural decision the W1 sub-L0 will make; the wave packet should flag it as forward-coupling-critical.**

2. **DOS-336 review-queue hook placement.** The decision to defer the hook to v1.4.6 is correct (memory `feedback_check_substrate_before_authoring_primitives` — consumer drives shape). But §13 item #6 says "v1.4.6 accepts a small migration against post-v1.4.4 review-queue code." That migration will land in v1.4.6 W1, against schema that's been live ~1 release. Migration slot coordination needs to account for this — v1.4.6's review-queue extension migration cannot collide with any v1.4.4 W4 sub-L0 migration touching review queue.

**Recommended fix.** Add §9 paragraph: "Forward-coupling hot spots: (a) DOS-459 envelope `schema_version` lifecycle is forward-coupling-critical for v1.4.5/v1.4.6 envelope extension; W1 sub-L0 picks semver-style with **additive-only minor bumps** (new fields land at minor; breaking changes at major); v1.4.5/v1.4.6 envelope extensions are minor bumps. (b) Review-queue migration slot in v1.4.4 W4 is coordinated with v1.4.6 review-queue hook migration slot to avoid v155-pattern collision."

### F7 — LOW — Chrome runtime-injection scope (§10 row 9) needs explicit "block-tree wins by default" rule

**Architectural concern.** §10 invariant row 9 carries the chrome lane's runtime-injection invariant verbatim: "Runtime DOM-injection permitted ONLY for FolioBar / FloatingNavIsland / AtmosphereLayer / MagazinePageLayout. Everything else stays Gutenberg." Good — that matches the chrome lane L0 §10.

The wave packet inherits this without naming the **default**. A naive reading of §13 item #3 (W3 FolioBar consumption) leaves room for a fifth runtime-injection module to creep in via a "briefing pattern needs chrome state bridge" argument. Memory `feedback_chrome_overlap_audit_before_new_pattern` calls this out — DS patterns can quietly duplicate chrome signaling.

**Recommended fix.** Tighten §10 row 9: "Default is Gutenberg blocks for ALL body-content and ALL surfaces other than the 4-module shell set. Adding a 5th runtime-injection module requires (a) wave-plan amendment with explicit parity-vs-authorability cost analysis, (b) `ce-design-lens-reviewer` + WP-skill-grounded reviewer + this reviewer panel re-approval, and (c) a documented invariant violation in the next retro K-out. **Default bias: when in doubt, ship a Gutenberg block; runtime injection is the exception.**" This converts §10 row 9 from a list into a rule.

### F8 — LOW — §10 invariant row 10 ("Canonical source-of-truth for chrome assets") under-specifies non-chrome canonical sources

**Architectural concern.** §10 row 10 says "`.docs/design/reference/_shared/` is canonical [for chrome assets]; one-way sync via `sync-chrome.sh`." But W2/W3/W4 composite blocks will ALSO consume canonical patterns from `.docs/design/patterns/` and primitives from `.docs/design/primitives/README.md`. The packet inherits the chrome-canonical discipline but does not extend it to the broader design system.

Memory `project_wp_primitives_translation_not_new_design` says v1.4.3 primitives are translations of existing canonical design — same applies to v1.4.4 composites. The packet should make explicit that W2/W3/W4 composites consume canonical design (HTML reference, pattern docs, primitives) without inventing new patterns inside `wp/dailyos/blocks/`.

**Recommended fix.** Extend §10 row 10 (or add row 12): "Design-system canonicity rule: W2/W3/W4 composite blocks translate existing canonical design (`.docs/design/reference/_shared/`, `.docs/design/patterns/*`, `.docs/design/primitives/README.md`) into block.json + render.php. **No new visual patterns are invented inside `wp/dailyos/blocks/`.** If a composite needs a pattern that does not exist canonically, the pattern lands canonical-first (per the chrome lane V1.3 §5.4.1 precedent) with `ce-design-lens-reviewer` approval, then is consumed by the block."

---

## Compliance Check

| Architectural principle | Status | Note |
|---|---|---|
| ADR-0130 §4 producer/projection/renderer split | Partial | F1 requires explicit outer/inner mapping |
| ADR-0130 §5 abilities-produce-compositions | Honored | W1 ticket map (§5.1) names producing abilities |
| ADR-0130 §3 BlockType taxonomy | Implicit | W2/W3/W4 sub-L0 packets must map inner blocks to taxonomy entries; wave packet does not enforce |
| ADR-0129 §1 no-surface-sacred | Honored if F1 lands | Outer-block decision must be Reading A |
| Substrate-in-same-wave (C4) | Asserted, not enforced | F4 |
| Many-blocks-not-few (anchored decision #1) | Honored | Outer-1/inner-N preserves it; F1 ensures producer/renderer boundary |
| Surface-agnostic substrate (anchored decision #6) | Honored | W1 substrate items live in `src-tauri/`; renderers in `wp/dailyos/blocks/` |
| Tauri UI freeze (anchored decision #4) | Honored with carve-out | F3 names the runtime-host carve-out explicitly |
| Intelligence Loop check (CLAUDE.md) | Deferred to sub-L0 | Correct — wave packet doesn't introduce claim fields itself |
| Wave-level L2 bounding (memory `feedback_l2_must_review_against_acceptance_criteria`) | Honored | §10 row 12 names path-α offload to maintenance project |
| No circular dependencies introduced | Honored | v1.4.4 depends on v1.4.3; v1.4.5/v1.4.6 depend on v1.4.4; clean DAG |

---

## Risk Analysis

- **Cross-wave envelope schema drift (F6).** Highest forward-coupling risk. Pick semver+additive-only at W1; surfaces F6's fix.
- **Outer/inner block contract drift (F1).** Highest within-wave risk. Two sub-L0 packets reading the §13 decision differently will produce two different block contracts.
- **Substrate gap deferral (F4).** Highest behavioral risk under L2 pressure. Reviewer panels must reject sub-L0 deferral patterns.
- **Flag-flip blast radius (F3).** Low likelihood, high blast if mis-scoped — runtime-host UI hidden = doctor surfaces unavailable post-flip. Carve-out is cheap and prevents it.

No architectural smells beyond what findings name. Inappropriate intimacy, leaky abstractions, dependency rule violations: none surfaced. The wave is a well-shaped substrate-down, renderer-up vertical-slice program with one over-loaded UX/architecture decision (F1) that needs disambiguation.

---

## Recommendations summary

Land V1.1 with:

1. F1 fix in §10 + §13 item #1 (outer block = renderer-side projection per ADR-0130 §4).
2. F2 fix as new §10 invariant + ADR-0130 amendment decision deferred to W1 sub-L0 with this reviewer in the panel.
3. F3 fix in §5.0 (5th audit list: dev/admin surfaces) + §13 item #5 (flag-flip scope).
4. F4 fix in AC #W5 (mid-wave gap escalation procedure).
5. F5 fix in §10 (refresh model invariant; Intelligence Loop §3 answer is "pull-on-render + user refresh").
6. F6 fix in §9 (forward-coupling hot spots; schema_version semver+additive-only; review-queue slot coordination).
7. F7 fix in §10 row 9 (Gutenberg default; runtime-injection is exception).
8. F8 fix in §10 row 10 or new row 12 (design-system canonicity rule).

Once V1.1 lands all eight, this reviewer flips to APPROVE. None of these require restructuring the wave program or re-negotiating the 4 locked decisions in §13.

