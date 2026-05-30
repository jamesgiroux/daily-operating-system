# WP-skill-grounded reviewer — W2 Entity Surfaces L0 cycle 2

**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md` V1.1 (`ffccb485`)
**Reviewer scope:** block.json + render.php discipline; context propagation; pagination hook shape; pattern vs template-part discipline; theme.json compatibility.
**Skills invoked:** `wp-block-development`, `wp-block-themes`.

## VERDICT: **APPROVE** (all 4 cycle-1 HIGH findings resolved; 1 net-new LOW informational; path-α items already routed to maintenance per V1.1 fold)

V1.1 lands all 4 cycle-1 packet edits cleanly and tightens the contract at the right resolution. Sub-wave is convergent — no new HIGH, no class-pattern recurrence. Ready for L0 close.

---

## Per-finding resolution

### H1 — `templateLock: false` + default `template` array — **RESOLVED**

§5.1 line 191-216 declares `templateLock: false` and a 23-entry `template` array on the `dailyos/account-detail` outer block.json sketch (matches the chapter list in §5.1's projection mapping). §5.2 line 341 inherits the same shape for Project Detail (14 entries). §5.3 line 401 inherits for Person Detail (12 entries). §5.4 line 464 inherits for Meeting Detail (10 entries). Filesystem pattern at `wp/dailyos/patterns/{entity}-detail-default.php` is named as the authoritative ordering source; block.json `template` mirrors it so empty-context insert renders meaningfully — clean separation. Architecture A1 fold also generalizes this as §10 code-shape sketch obligation invariant (line 779). Locked correctly.

### H2 — `dailyos/envelopeHandle` resolution contract — **RESOLVED**

§5.1 lines 223-232 names the contract explicitly: (1) outer invokes `get_entity_intelligence` ONCE via 3-arg `invoke_ability`; (2) DOS-477 envelope cache (`services::entity_intelligence::auth::envelope_cache`) keys on `(envelope_render_id, actor_principal_id, surface)` where `envelope_render_id` is the deterministic hash of `(entity_type, entity_id, depth, sections, watermark)`; (3) outer writes `envelope_render_id` to context, inner blocks `usesContext: ["dailyos/envelopeHandle"]` and short-circuit to cached envelope; (4) per-request scope, evicts on response close. AC-462.4 + AC #W2.4 enforce 3-arg `invoke_ability` via `check_w1_consumer_skeleton.sh`. Inner block code-shape sketch at line 289-298 demonstrates the resolve pattern. Contract is now testable, not prose-only.

### H3 — Inline-style boundary gate — **RESOLVED**

§5.2 line 348-351 + AC #W2.7 (line 723) lock the wrapper as `get_block_wrapper_attributes(['style' => '--dailyos-project-tint: var(--color-garden-olive);'])`. New CI script `src-tauri/scripts/check_no_inline_style_exception.sh` files at L1 kickoff and asserts every `style=` attribute body in `wp/dailyos/blocks/**/*.php` matches `^--[a-z-]+:\s*var\(--[a-z-]+\);?$`. ADR-0077 amendment named as W2 L1 prerequisite; until amendment lands, `chrome_config()` emits olive default via `$stub_tints['dailyos_project'] = 'olive'`. Exception is now bounded, gated, and CI-enforced — no daylight between the `feedback_no_inline_css` rule and the project-tint carve-out.

### H4 — "synced patterns" → "filesystem patterns" — **RESOLVED**

Swept throughout: §5.1 line 306, §5.2 line 389, §5.3 line 437, §5.4 line 489, AC #W2.10 (line 729). Each occurrence now reads "filesystem pattern" or "theme-registered via `register_block_pattern`" and explicitly disclaims "NOT synced patterns (DB-stored reusable blocks)." AC-462.8 (line 318) was already correct on `templateLock: false`; the prose around it now correctly identifies the pattern as filesystem-registered with insert-then-detach semantics. WP vocabulary is now correct.

---

## New findings

### L1 — `do_blocks($content)` language in §10 invariant still misleading (LOW informational)

**File:** line 768.
**Finding:** §10 invariant still says "render.php calls `do_blocks($content)` for inner-block reordering." The Stage 1b skeleton at `00f38b3b` actually emits `'<div class="dailyos-inner-blocks-slot">' . $content . '</div>'` — the block parser pre-renders `$content` before render.php receives it; calling `do_blocks($content)` would double-render. Cycle-1 flagged this as L1 informational; V1.1 didn't sweep the wave-level invariant. Not a blocker — the §5.1–5.4 code sketches don't actually call `do_blocks()`, so the invariant is contradicted-but-harmless. Recommendation: in the next wave-plan revision, rephrase to "render.php emits the pre-rendered `$content` so user reorders persist." File against maintenance project at L0 close; doesn't block W2 L1.

---

## Path-α status

M1 / M2 / M3 from cycle-1 already routed to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per V1.1 §13 Q7 + §10 invariants. Confirmed correctly filed; no further action.

---

## Convergence

- Cycle-1: 4 HIGH + 3 path-α.
- Cycle-2: 0 HIGH, 1 LOW informational.
- Net-new findings: 1 (below the 5+ scope-reset threshold per `l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20`).

W2 sub-wave packet is L0-close-ready from the WP-skill perspective. Re-review not needed unless other reviewers request V1.2 edits affecting block.json / render.php / pattern surfaces.
