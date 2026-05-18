# Packet C (Starter Kit) — code-reviewer L0 verdict

**Packet:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md` V1.0
**Linear:** DOS-678
**Reviewer:** code-reviewer (claude domain)
**Date:** 2026-05-18

## Verdict

**CONDITIONAL APPROVE** — Plan is sound in shape, the W4-F-pattern extraction thesis is correct, scope bounds are clean, the §10 commit-group landing is coherent, and the Intelligence-Loop exemption in §1 holds. But §4–§5 contain three substrate-reuse mis-citations that will derail implementation if not fixed at L0:

1. The block-registration mechanism in `wp/dailyos/includes/class-dailyos-plugin.php:154-170` is a **`glob()` directory scan**, not an explicit enumeration — §5.1 step 3, §6.6, and AC #3 are predicated on the wrong existing pattern.
2. The projection-rule registration story in §5.4 names `register_custom_block_schema` + `CustomBlockSchema::new(...)`, but the production projection path for `dailyos/account-overview` dispatches through the `BlockType` enum + `known_projection_rules()` Vec + `<block>_rule()` constructor in `fallback_projection.rs:1236-1252,1415-1423`. `register_custom_block_schema` is only ever called in tests (`tests/dos570_fallback_projection.rs:101`).
3. The token-generator at §5.6 cites three source paths but **two of them are wrong / missing**: `src/styles/tokens.css` does not exist (actual path: `src/styles/design-tokens.css`); `wp/dailyos/theme/theme.json` does not exist anywhere in the repo (W3 magazine theme creates it).

Fold these into V1.1 and the packet is ready to land. Findings below; per-focus.

---

## Focus 1 — W4-F pattern extraction (correct or divergent?)

**Correct in shape.** §4 reuse table captures the right primitives: block.json + render.php + render-functions.php + `#[ability(...)]` macro + `commit_composition` / `finalize_provenance` chain + signed runtime client + fake-runtime-client fixture pattern. The "kit IS extraction, not invention" framing (§4 bottom paragraph + §6.4) is exactly right and matches what v1.4.2 W4-F shipped at `src-tauri/abilities-runtime/src/abilities/account_overview.rs:83-120` and `wp/dailyos/blocks/account-overview/render-functions.php`.

**Two divergence risks to fix:**

- F1.1 — Block registration: §5.1 step 3 says the CLI "updates `class-dailyos-plugin.php` to register the new block (insert into the block-metadata enumeration at `:154-170`)." But that range is:
  ```php
  $block_files = glob( DAILYOS_PLUGIN_DIR . 'blocks/*/block.json' );
  foreach ( $block_files as $block_file ) {
      register_block_type_from_metadata( dirname( $block_file ) );
  }
  ```
  That is `glob` directory-scan auto-discovery — exactly the "scan the `blocks/` directory at PHP runtime" alternative §6.6 says was **rejected** at W4-F. The CLI does NOT need to edit this file at all; new blocks register automatically when their directory exists. §5.1 step 3, §6.6, AC #3, fixture #3, and CI invariant #1's "explicit registration" thread all need rework. Open question §12.3 (insertion-anchor brittleness) becomes moot.
- F1.2 — Projection rule shape: §5.4 template uses `register_custom_block_schema(CustomBlockSchema::new("dailyos/{{ABILITY_NAME_KEBAB}}")...)`. The actual `account-overview` production path is: (a) add a `BlockType` variant in `src-tauri/abilities-runtime/src/abilities/composition.rs:330+`, (b) add a `<block>_rule()` constructor in `fallback_projection.rs` (see `account_overview_rule()` at `:1415-1423` returning a `BlockProjectionRule` with `block_type`, `composition_kind`, `type_namespace`, `render_annotations`, `fields`, `default_trust_band`), (c) add the dispatch arm in `rule_for_block_type()` at `:1236-1247`, (d) add to `known_projection_rules()` at `:1250-1252`, (e) define the `FIELDS` const for the binding list. `register_custom_block_schema` exists but is test-only. Either §5.4 switches to documenting the real `BlockType` + `BlockProjectionRule` flow, or v1.4.3 W1 promotes `register_custom_block_schema` into production wiring as a kit precondition — and the latter is net-new substrate work that should be called out, not silently assumed.

## Focus 2 — §4 substrate-reuse citation accuracy

| Row | Claim | Actual state | Verdict |
|---|---|---|---|
| block.json scaffold | 26 lines | 26 lines, matches `wp/dailyos/blocks/account-overview/block.json` | OK |
| render.php delegation | 25 lines | 25 lines | OK |
| render-functions.php | 232 lines (post-Packet-B V1.1.1) | 232 lines today, but typed switch from Packet B §5.6 is NOT in the file yet (only `is_wp_error` → banner at `:55-65`). §4 honestly tags this "post-Packet-B V1.1.1" so the forward-dep is declared. | OK (forward-dep, called out) |
| Producer ability shape `:83-120` | `account_overview.rs:83-120` | Confirmed: `#[ability(...)]` at `:83`, fn at `:106`, `normalize_input` at `:110`, `commit_composition` at `:114`, `finalize` at `:119` | OK |
| Projection rule shape `:393-612` | `fallback_projection.rs` `project_known_block` + `validate_field_bindings` | `project_known_block` at `:393`, `validate_field_bindings` at `:568` — these are **dispatchers**, not the per-block registration shape that §5.4 template purports to be. The template should anchor on `account_overview_rule()` at `:1415-1423` + `rule_for_block_type` at `:1236-1247` + `known_projection_rules` at `:1250-1252` instead. | INCORRECT anchor |
| `register_blocks_from_metadata` enumeration `:154-170` | method name is `register_blocks` (not `..._from_metadata`); uses `glob` directory scan, not enumeration | Method exists at `:149-163`; calls `register_block_type_from_metadata` per file. **It IS the directory scan, not an enumeration.** | INCORRECT name + INCORRECT primitive shape (see F1.1) |
| `surface_runtime::project_composition` at `surface_runtime/mod.rs:2280-2440` | actual function is `surface_project_composition_response` at `src-tauri/src/surface_runtime/mod.rs:2232` (cite range approximately covers the body) | Range close enough; function name slightly off but identifiable | MINOR (function name) |
| Test fixture pattern `AccountOverviewBlockTest.php` | exists at `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php` | confirmed | OK |

## Focus 3 — §10 commit-group landing

Four-group split (Templates+CLI / Harness / Generators / CI+fixtures) is coherent and the within-group cohesion is real. **Hidden dependency:** group 4 (CI workflow + AC #11 fixtures + AC #12 manifest) depends on group 2 (harness invocation contract) AND group 3 (translator output). If group 3 is rebased late or split off into PR-C2 per §10's optional 2-PR fallback, group 4's CI invariants #5 and #6 can't run; the §10 split rule says "PR-C2 = groups 3+4" which is fine in shape, but the packet should be explicit that the harness CI invariants from group 4 that ONLY depend on groups 1+2 (#1, #2, #3) can move into PR-C1, while translator-dependent invariants (#5, #6) stay with PR-C2. Otherwise PR-C1 lands "kit core" with no CI gate on its own primitives. Minor — fixable as a §10 footnote.

**One missing dependency callout:** group 1's CLI step "Update `class-dailyos-plugin.php` to register the new block" (§5.1 step 3) is invalidated by F1.1. With F1.1 fixed, the CLI has nothing to do in that file → group 1 simplifies.

## Focus 4 — §11 NOT-owned list (gaps W2 will hit)

Mostly correct boundary. One real gap, one borderline:

- F4.1 — **Existing Tauri component → block-name conflict resolution is undefined.** Translator §5.7 emits `wp/dailyos/blocks/<name>/`. W2 will hit cases where multiple Tauri sources resolve to the same `<name>` (e.g., `Pill.tsx` + `TrustPill.tsx` + `EngagementPill.tsx`). §11 doesn't claim ownership; §5.7 doesn't define the policy. Add a §6.7 directional decision (e.g., "translator takes `--block-name <slug>` override; defaults to kebab-cased filename") or punt explicitly to W2.
- F4.2 — **No mention of `BlockType` enum extension story.** If F1.2 is resolved by sticking with the production `BlockType` pattern (vs. promoting `register_custom_block_schema`), then W2 needs the kit to either (a) extend `BlockType` per primitive — which is substrate edit, blurs the "kit doesn't touch substrate" §6.4 line, or (b) wire `register_custom_block_schema` into the production projection path as kit precondition (also substrate). One of those needs to be in §11 or §5.

Other §11 entries (W0 / W2 / W3 / W4 / W5 / docblocks / code-mod / interactivity / variants / delete-rename / promotion) are clean.

## Focus 5 — §7 ACs (testable or squishy?)

Six of eight are concrete. Two are weak:

- AC #1 — "WordPress block editor lists 'Test Block' in the DailyOS category." Requires a running WP instance, not just CI. The L1 proof-bundle line in §15 says "macOS hands-on confirming the scaffolded block renders in WP editor" so the human-verification fallback is there — but AC #1 should explicitly tag this as a hands-on AC (not CI-greppable), to align with the L4-before-L2 discipline (`feedback_l4_before_l2_for_user_facing.md`).
- AC #6 — "Translation utility produces working composite: byte-equal (modulo timestamps + IDs)." "Modulo timestamps + IDs" is squishy — fixture #8 calls it `c1_translator_composite_byte_parity` but the diff filter (what counts as a timestamp, what counts as an ID, are CSS class hashes IDs?) is undefined. Either name the exact `diff -I '<regex>'` invocation, or downgrade to "structural parity: same DOM shape, same class set, same attribute keys" with a documented HTML normalizer.

All others (#2 contract-mismatch diagnostic, #3 [reframe per F1.1], #4 idempotency, #5 translator-passes-harness, #7 verbatim switch table, #8 CI workflow) are concrete.

## Focus 6 — Intelligence-Loop integration check

**Exemption holds.** Per CLAUDE.md "Critical Rules", the 5-question gate applies to "new table, schema column, claim field, or user-visible intelligence surface." This packet adds zero of those. The kit's `BlockIntegrationFixture` struct is test-infrastructure; the templates emit code that consumes existing claim/composition primitives. §1 paragraph 3 is correctly worded and the §6.4 "no new substrate primitives" decision codifies the exemption.

One risk to monitor (not a blocker): if F1.2 resolves toward "promote `register_custom_block_schema` into production," that is net-new substrate plumbing and should re-engage Q4 (runtime + surfaces) at minimum. Surface it in the V1.1 amendment.

## Focus 7 — CLI tool conventions

**`pnpm dailyos:new-block` is idiomatic.** `package.json:25-31` already uses the `pnpm <namespace>:<verb>` shape with `.mjs` files invoked via `node` (`eval:abilities` → `node scripts/eval-abilities.mjs`; `evidence:validate` → `node scripts/validate-evidence-record.mjs`; `evidence:contract-smoke` → `node scripts/evidence-contract-smoke.mjs`). Putting the new script at `wp/dailyos/scripts/new-block.mjs` and wiring `pnpm dailyos:new-block` to `node wp/dailyos/scripts/new-block.mjs` fits the established pattern exactly.

The existing `wp/dailyos/scripts/run-grep-gates.sh` is bash, but that's a CI-only quick check; for an interactive author-facing tool with arg parsing, name interpolation, and file generation, Node `.mjs` is the right call. Confirm.

One nit: open question §12.4 (interactive vs flags) — recommend **flags-only with a single required positional + optional flags**, matching the rest of `package.json` script ergonomics (no inquirer-style prompts in any other DailyOS CLI). Interactive prompts won't run in CI fixture #1.

## Summary of required V1.1 edits

1. **F1.1 / §5.1 / §6.6 / AC #3 / fixture #3 / CI invariant #1** — strip "CLI edits class-dailyos-plugin.php" — registration is `glob`-based already; CLI just creates the directory.
2. **F1.2 / §4 row "Projection rule shape" / §5.4** — re-anchor projection-rule template on the `BlockType` + `<block>_rule()` + `known_projection_rules()` pattern at `fallback_projection.rs:1236-1252,1415-1423`, OR explicitly call out promoting `register_custom_block_schema` into production as kit substrate work.
3. **§5.6 token sources** — fix `src/styles/tokens.css` → `src/styles/design-tokens.css`; call out that `wp/dailyos/theme/theme.json` is W3-owned and the generator's third source either doesn't exist for v1.4.3 W1 (drop to 2 sources) or W1 stub-creates an empty `theme.json` for the generator's idempotent baseline. (Note: v1.4.3-waves.md:75 has the same error; fix there too.)
4. **§4 row "Projection rule shape `:393-612`"** — re-anchor to `:1236-1252,1415-1423` (registration), keep `:393` (dispatcher) as a separate row for the "called by" reference.
5. **§4 row "Existing block render route"** — function name is `surface_project_composition_response` not `project_composition`; line range is approximately `:2232-2440`.
6. **§4 row "WP block registration"** — method name is `register_blocks` not `register_blocks_from_metadata`; clarify it is `glob`-based.
7. **§7 AC #1** — tag as hands-on (L4-before-L2 discipline).
8. **§7 AC #6 / fixture #8** — name the normalizer or downgrade to structural-parity wording.
9. **§10 / §11** — F4.1 block-name conflict policy; F4.2 `BlockType` extension story (whoever owns it after F1.2 resolves).
10. **§12.4 → §6.7** — promote "CLI is flags-only, no interactive prompts" to a directional decision.

None of these is a BLOCK. All are precision/citation fixes that should land before L1 implementation begins, because the implementing agent will read §4 + §5 verbatim and end up writing code that targets the wrong primitives.

## Reviewer cycle position

This is cycle-1. Expectation: V1.1 amends §4/§5/§7/§10/§11/§6.7 per above, re-run code-reviewer + codex-challenge + codex-consult + DX-review + CSO. If V1.1 closes all 10 items, code-reviewer expects to return APPROVE on cycle-2 without further substrate-reuse findings.
