# WP-skill-grounded reviewer — W2 Entity Surfaces L0 cycle 1

**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md` V1.0
**Reviewer scope:** block.json + render.php discipline; context propagation; pagination hook shape; pattern vs template-part discipline; theme.json compatibility.
**Skills invoked:** `wp-block-development`, `wp-block-themes` (`wp-wpcli-and-ops` not relevant — no CLI scoped to W2).

## VERDICT: **REQUEST-CHANGES** (4 high-severity packet edits required before L0 close; 3 path-α items routed to maintenance)

W2 substrate consumption is sound and 1-to-1 traces to W1 producers. AC list is bounded by the originating tickets. The packet edits below are spec-level (block.json contracts, hook semantics, naming/terminology) — they tighten the build contract for L1, they don't expand scope.

---

## High — block

### H1. `templateLock` is named in AC-462.8 but missing from every block.json snippet in §5.1–5.4
**File:** L0-packet-W2-entity-surfaces.md:165–180 (Account block.json sample); §5.2/5.3/5.4 inherit the same gap.
**Finding:** The Account Detail block.json snippet does not declare `template` or `templateLock`. AC-462.8 commits to `templateLock: false`, and §10's wave invariant says default-template ordering ships via synced pattern. With no `template` array in block.json, the outer block has no editor default; users get an empty composite on insert. With no explicit `templateLock`, the lock state inherits from core defaults (currently `false`, but volatile across WP versions per Block Editor Handbook).
**Docs:** [Inner Blocks — template + templateLock](https://developer.wordpress.org/block-editor/reference-guides/block-api/block-registration/#inner-blocks).
**Fix:** Add to every outer block.json snippet (or explicitly state "default template ships via synced pattern, not block.json `template` field — `templateLock: false` declared in `<InnerBlocks templateLock={false} />` JSX in edit.js"). Pick one path and lock it.

### H2. `dailyos/envelopeHandle` context key has no resolution contract
**File:** L0-packet-W2-entity-surfaces.md:172, 232.
**Finding:** §5.1 introduces `providesContext: { "dailyos/envelopeHandle": "envelope_handle" }` with the prose "per-request runtime cache keyed by `(account_id, surface, actor)`." The W1 skeleton at `wp/dailyos/blocks/account-detail/render-functions.php` does NOT declare this attribute, does NOT populate it, and there is no named substrate for the runtime cache. Without a contract, inner blocks either (a) re-invoke `get_entity_intelligence` per-block (defeating §5.1's "once per render" claim) or (b) consume an undefined handle.
**Docs:** [Block Context — providesContext + usesContext](https://developer.wordpress.org/block-editor/reference-guides/block-api/block-context/) (context values must resolve from declared attributes).
**Fix:** Either (a) name the cache (e.g., `dailyos_request_envelope_cache` static-array singleton in `class-dailyos-runtime-client.php` keyed by `(actor, entity_type, entity_id, depth, sections)`) AND add `envelope_handle` to `attributes` with an explicit "set at edit-time by save() OR resolved at render via key derivation" rule, or (b) drop `envelopeHandle` from `providesContext` and have inner blocks key off `(entityType, entityId)` plus rely on the request-scoped cache. Path (b) is simpler; path (a) is more explicit. Either way, lock it.

### H3. Inline `style="--dailyos-project-tint: ..."` contradicts no-inline-CSS rule with no exception scope
**File:** L0-packet-W2-entity-surfaces.md:587 (AC #W2.7), :634 (§10 invariant row).
**Finding:** Wave §10 calls per-project tint a "narrow exception" to memory `feedback_no_inline_css`, but the W2 packet doesn't actually exercise that exception correctly — emitting `style="..."` on the outer wrapper is per-instance inline CSS, indistinguishable from the banned pattern unless the exception is bounded ("runtime-computed values via CSS custom property" per memory). The packet should make explicit: the style attribute carries ONLY the `--dailyos-project-tint` custom property, never declarative styles; rendered via `get_block_wrapper_attributes(['style' => '...'])` not raw concatenation; and an ESLint/PHPCS gate or `check_no_inline_style.sh` enforces the boundary.
**Docs:** [get_block_wrapper_attributes](https://developer.wordpress.org/reference/functions/get_block_wrapper_attributes/) — `style` is accepted as the sole prop. No DailyOS gate exists today for "style-attr-only-carries-custom-properties."
**Fix:** Add to §10/AC-W2.7: "Outer wrapper emits style via `get_block_wrapper_attributes(['style' => '--dailyos-project-tint: var(--color-garden-olive);'])`; CI gate (new, file at L1) asserts the style attribute body matches `^--[a-z-]+:\s*var\(--[a-z-]+\);?$` on outer block render output."

### H4. "Synced patterns" terminology is wrong for filesystem `patterns/*.php`
**File:** L0-packet-W2-entity-surfaces.md:62, 234, 386, 593 (AC #W2.10).
**Finding:** Throughout the packet, theme `patterns/*.php` files are called "synced patterns." In WP vocabulary, **synced patterns** are DB-stored reusable blocks (formerly "reusable blocks"); **filesystem patterns** are `patterns/*.php` registered via `register_block_pattern` from theme. They have different update semantics: synced patterns propagate edits to all instances; filesystem patterns are inserted-then-detached. The packet wants the latter (user reorders without affecting other instances), but uses the term for the former.
**Docs:** [Block Patterns — filesystem registration](https://developer.wordpress.org/themes/features/block-patterns/) vs [Synced patterns](https://wordpress.org/documentation/article/reusable-blocks/).
**Fix:** Replace "synced pattern" with "filesystem pattern" or "theme-registered block pattern" throughout (§5.1 line 234; §5.2 line 303; §5.3 line 349; §5.4 line 386; §10 line 641; AC #W2.10). Then verify the AC matches the chosen semantic — "user reorders via Site Editor" only works for inserted instances if the pattern is filesystem (not synced). Locked filesystem semantics resolve this; just fix the language.

---

## Medium — path-α to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`

### M1. `executeAbility()` (WP 7.0 client-side Abilities API) is forward-looking; no fallback documented
**File:** L0-packet-W2-entity-surfaces.md:402–404, :633.
**Finding:** WP 7.0 has not shipped at packet-author time (current stable: 6.x). The packet builds AC-W2.2 on a WP-core API that doesn't exist in the target environment. The W2 implementation date is unbounded by WP-core ship; if WP 7.0 slips, W2 ships with a stub.
**Docs:** Abilities API roadmap was previewed in 6.9 dev notes (see [make.wordpress.org/core](https://make.wordpress.org/core/)) but is not stable API yet.
**Recommendation:** File maintenance ticket "WP 7.0 abilities API fallback — REST endpoint shim for `executeAbility()`" so W2 lands against a real callable even on 6.9. Doesn't block L0; does block L1 if WP-core slip materializes.

### M2. `usesContext` declared as inner-block convention; no template lock on per-inner-block context override
**File:** L0-packet-W2-entity-surfaces.md:232.
**Finding:** Inner blocks declare `usesContext: ["dailyos/entityType","dailyos/entityId","dailyos/envelopeHandle"]`. The packet doesn't address: what happens if a user inserts an inner block (e.g., `dailyos/touchpoints-feed`) outside an entity-detail outer (since `parent: null`)? Inner block needs a graceful "no context — render empty" path. Not a blocker but worth a one-liner in §10.
**Recommendation:** Path-α — file maintenance issue "Inserter-global primitives need null-context render guard" once L1 surfaces actual block boundaries.

### M3. Theme.json per-block `styles.blocks["dailyos/*"]` not declared anywhere in packet
**File:** L0-packet-W2-entity-surfaces.md (whole packet).
**Finding:** W1 substrate consumed list (§6) doesn't include theme.json per-block style entries. If W2 blocks declare any `supports.color` / `supports.spacing` / etc., they should appear under `theme.json` `styles.blocks` for the active theme. The packet is silent. If W2 doesn't enable any `supports.*`, this is moot — but the block.json snippets don't declare `supports` at all except `html: false, reusable: false, inserter: true`, so per-block styles are out of scope by omission, not by intent.
**Recommendation:** Path-α — add a one-liner to §10 invariants: "W2 blocks declare only `supports.html`, `supports.reusable`, `supports.inserter`; no `supports.color/spacing/typography`; theme.json `styles.blocks` updates deferred to v1.4.5+."

---

## Low — informational

- **L1.** `do_blocks($content)` invariant from §10 is satisfied by the existing W1 skeleton's `'<div class="dailyos-inner-blocks-slot">' . $content . '</div>'` pattern — core pre-renders `$content` via the block parser before passing into render.php for dynamic blocks. No action needed; calling out for clarity. The W1 skeleton is correct; §10's mention of `do_blocks($content)` is technically wrong (you don't double-render). Recommend rephrasing §10 invariant to "server-side render emits the pre-rendered `$content` so user reorders persist," not "calls `do_blocks($content)`."
- **L2.** Block category `dailyos` is consistently declared. Verify category registration in `wp/dailyos/dailyos.php` via `block_categories_all` filter exists at L1.
- **L3.** No `viewScriptModule` declared for any block; consistent with read-only PHP-rendered approach. If DOS-689 EvidenceDrawer needs interactivity (drawer open/close), `viewScriptModule` lands at L1 — note in §5.7 sub-spec.

---

## Blocker summary

**Must edit before L0 close (H1–H4):** templateLock contract; envelopeHandle resolution; inline-style boundary gate; synced-vs-filesystem-patterns terminology.

**Path-α (M1–M3):** ship-blocking-at-L1, not L0; file in maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`.

**Re-review at:** packet V1.1 carrying H1–H4 fixes.
