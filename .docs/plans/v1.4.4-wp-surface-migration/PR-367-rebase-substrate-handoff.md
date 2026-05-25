# PR #367 rebase — substrate handoff

**Status:** rebased and substrate-aligned. Branch ref `pr367-rebased` at `b17579d2`. Not yet pushed. Path A executed: 17 divergent renderers reverted to dev's post-#366 contract; PR's reference-verbatim DOM rewrites for those surfaces deferred to a separate follow-up ticket. Two follow-up tickets named at the bottom of this doc.

## What changed underneath this PR

PR #367 was authored on a fork point of `d155120` (2026-05-23 early). Between that fork and `public/dev` HEAD (`52c9255c`), 27 commits landed — three of them substrate-load-bearing for account-detail renderers:

- **#365 — foreground DB contention** (merged 2026-05-23 21:13). Routed reads off the primary worker, added `account_overview_integration_fixture.rs` to the block-kit harness, and **explicitly reverted the meeting empty-state markup** (`3cd7258c "Fix meeting block empty-state drift"`) that PR #367 also touched.
- **#366 — account fact claim producer + enrichment finalization pipeline** (merged 2026-05-24 13:08). Added `account_fact_claims.rs` (2,518 lines), `glean_finalization.rs` (1,329 lines), `entity_intelligence/neighborhood.rs` (1,377 lines), 3,119 lines of new code in `intelligence.rs`, the `082_account_fact_columns.sql` migration, and hardened `wp/dailyos/blocks/_shared/envelope/envelope-resolver.php` with the `dailyos_envelope_consume_claim` wrapper contract.
- **#370–#373** — entity-detail intelligence wiring (React-side), W5 actions/email claim routing, release-gate hardening.

PR #367's WP renderer rewrites were authored against a pre-#366 snapshot. That timing is the whole story.

## The post-#366 envelope-consumption contract

In `wp/dailyos/blocks/_shared/envelope/envelope-resolver.php` (public/dev tip):

- `dailyos_envelope_section($env, $section_key)` — presence check, returns `{present, item_count, reason}`. Reads `$env['sections'][$section_key]` for `kind == 'present'`.
- `dailyos_envelope_collect_claim_refs($env, $section_keys, $filters)` — enumerates matching `claim_ref` items from `$env[$section_key]['items']`, applying `field_paths` filter. Returns array of refs shaped `{claim_id, audience_key, subject_ref, field_path, ...}`. Tolerates both `claimId`/`claim_id` and `audienceKey`/`audience_key` on items.
- `dailyos_envelope_consume_claim($claim_ref, $scope_set)` — the contract wrapper (line ~462):
  1. If `renderedText` is already on `$claim_ref` → return it (the post-#366 happy path; `get_entity_intelligence` server-side audience-filters per DOS-341).
  2. Else look in `dailyos_envelope_claim_item_cache_get($claim_id)` and merge if found.
  3. Else fall back to runtime `invoke_ability('claim_receipt', ...)` via `apply_filters('dailyos_runtime_client_for_block', null)`, with shaped target `{kind: 'claim', claimId, subject, fieldPath}` + `surface: 'entity_detail'`.
  4. Returns `null` if no path produces `renderedText`.
- `dailyos_receipt_rendered_text($payload, $fallback)` + `dailyos_receipt_trust_band($payload)` — typed display extraction, both camelCase and snake_case tolerant.

The wrapper is what makes consumption substrate-correct. It enforces the audience-filter contract, handles the fallback into runtime, and tolerates envelope shape variants. Reading items directly is the fast path only — it works when the envelope is fully primed but skips the contract.

## The pattern PR #367 was carrying

The PR introduced a per-block `*_facts_index()` helper pattern that reads `$envelope['facts']['items']` directly and looks up by camelCase field-path:

```php
function dailyos_outlook_panel_facts_index( ?array $envelope ): array {
    $facts = $envelope['facts'] ?? [];
    $items = is_array( $facts ) ? ( $facts['items'] ?? [] ) : [];
    // …indexes by field_path → item
}

function dailyos_outlook_panel_renewal_section( array $facts ): string {
    $claim     = $facts['agreementOutlook'] ?? null;
    $prose     = is_array( $claim ) ? (string) ( $claim['renderedText'] ?? '' ) : '';
    $claim_id  = is_array( $claim ) ? (string) ( $claim['claimId'] ?? '' ) : '';
    $trust_band = is_array( $claim ) ? (string) ( $claim['trustBand'] ?? '' ) : '';
    // …renders directly
}
```

This was a reasonable adaptation to the pre-#366 substrate reality. The producer audit (`.docs/plans/abilities-runtime-producer-audit-2026-05-23.md`) describes that earlier reality:

> `dailyos.read.account_status` calls `get_entity_intelligence` for facts/open loops/touchpoints/record, but the handler returns only part of that envelope

Pre-#366, the envelope didn't carry `renderedText` reliably. WordPress renderers built workarounds. #366 finalized the contract: the producer now emits display-safe `renderedText` server-side and the wrapper short-circuits when present. The workaround is no longer needed and now contradicts the W0 contract from the abilities-runtime producer remediation plan (`.docs/plans/abilities-runtime-producer-remediation-waves.html`):

> **Non-goal:** do not teach MCP or WordPress to bypass the runtime by reading account-shaped schema fields directly.

> Every MCP-projected text, identity, source label, and provenance ref must pass through `RenderableClaimText` or a renderable evidence wrapper with sensitivity, provenance, and sanitizer semantics.

`dailyos_envelope_consume_claim` is the WordPress-side renderable evidence wrapper. Renderers that skip it ship MCP/WP substrate bypass — exactly what the remediation plan deprecates.

## What this rebase did, file by file

### Reverted to dev — substrate contract enforced (17 files)

Each of these had PR's `facts_index` pattern. Replaced with `public/dev` content (the canonical post-#366 consumers via `dailyos_envelope_collect_claim_refs` + `dailyos_envelope_consume_claim`). The DOM/class divergence between PR's reference-verbatim rewrites and dev's design-system markup is real (4–42 class-name diffs per file) — that work is **not lost**, just deferred to a clean follow-up on top of dev's substrate (ticket A below).

```
wp/dailyos/blocks/account-detail/inner/about-intelligence/render-functions.php
wp/dailyos/blocks/account-detail/inner/about-this-dossier/render-functions.php
wp/dailyos/blocks/account-detail/inner/account-pull-quote/render-functions.php
wp/dailyos/blocks/account-detail/inner/account-technical-footprint/render-functions.php
wp/dailyos/blocks/account-detail/inner/commercial-shape/render-functions.php
wp/dailyos/blocks/account-detail/inner/file-list/render-functions.php
wp/dailyos/blocks/account-detail/inner/linear-issues-chapter/render-functions.php
wp/dailyos/blocks/account-detail/inner/outlook-panel/render-functions.php
wp/dailyos/blocks/account-detail/inner/quote-wall/render-functions.php
wp/dailyos/blocks/account-detail/inner/relationship-fabric/render-functions.php
wp/dailyos/blocks/account-detail/inner/sentiment-hero/render-functions.php
wp/dailyos/blocks/account-detail/inner/stakeholder-grid/render-functions.php
wp/dailyos/blocks/account-detail/inner/strategic-landscape/render-functions.php
wp/dailyos/blocks/account-detail/inner/supporting-tension/render-functions.php
wp/dailyos/blocks/account-detail/inner/triage-section/render-functions.php
wp/dailyos/blocks/account-detail/inner/unified-timeline/render-functions.php
wp/dailyos/blocks/account-detail/inner/value-commitments/render-functions.php
```

Post-amend audit: `facts_index` purged from `wp/dailyos/` entirely. All 21 dev-equivalent account-detail inner blocks consume via `dailyos_envelope_consume_claim`. PHP `-l` clean on all 17. `cargo check -p abilities-runtime --tests` clean.

### Kept from dev during initial conflict resolution (9 files)

Already substrate-correct; the original git conflicts were trivial:

```
wp/dailyos/blocks/account-detail/inner/account-hero/render-functions.php       (docblock wording)
wp/dailyos/blocks/account-detail/inner/on-track-chapter/render-functions.php   (WPCS array alignment)
wp/dailyos/blocks/meeting-attendees-section/render-functions.php               (dev's 3cd7258c reverted PR's empty-state)
wp/dailyos/blocks/meeting-header/render-functions.php                          (same)
wp/dailyos/blocks/meeting-detail/render-functions.php                          (comment indent)
wp/dailyos/blocks/meeting-post-meeting-capture/render-functions.php            (docblock added by both)
wp/dailyos/blocks/meeting-related-entities/render-functions.php                (same)
wp/dailyos/blocks/meeting-touchpoints-feed/render-functions.php                (WPCS indent + translator-comment)
wp/dailyos/blocks/entity-intake/render-functions.php                           (docblock wording)
```

### PR-net-new blocks kept (3 files, follow-up ticket B)

Net-new chapters added by PR; dev has no equivalent. These use the same pre-#366 direct-envelope-read pattern but kept as-is for this rebase since taking dev means deleting them outright. They need to be rewired through `dailyos_envelope_consume_claim` before the v1.4.4 wave closes.

```
wp/dailyos/blocks/account-detail/inner/reports/render-functions.php
wp/dailyos/blocks/account-detail/inner/the-record/render-functions.php   (single direct read: $envelope['record_entries']['items'])
wp/dailyos/blocks/account-detail/inner/watch-list/render-functions.php
```

### Non-renderer files

| File | Resolution | Reason |
|---|---|---|
| `src-tauri/abilities-runtime/tests/fixtures/account_overview_integration_fixture.rs` | dev | Add/add. Dev's `b250f3b3` is what the block-kit harness invariant expects (branch label `"projected-block"`). |
| `tasks/lessons.md` | union | All entries preserved. |
| `wp/dailyos/dev-tools/mock-runtime-client.php` | PR | PR added ~1,370 lines of parity-surface seeding; conflict zones were PHPCS array alignment only. PHPCS may flag — `phpcbf` should auto-fix. |
| `wp/dailyos/tests/blocks/AccountDetailBlockTest.php` | PR | Same — overlap was `'id' =>` vs `'id'   =>` alignment. |

## Why Path A and not a surgical rewire-in-place

The surgical option — keep PR's reference-verbatim DOM, rewire each block's claim consumption to go through `dailyos_envelope_consume_claim` — was rejected because it would:

1. Require 17 manual claim-shape translations within this PR. Each one maps PR's camelCase field-path queries (`'agreementOutlook'`, `'pullQuote'`, etc.) onto the right `dailyos_envelope_collect_claim_refs` filter set. Mistakes silently weaken trust/audience enforcement on the affected renderer.
2. Compress 17 substrate-touching diffs into the same PR as the v1.4.4 visual pass, contradicting the L2-bounded-by-AC review rule.
3. Carry risk to #366's contract from this PR's review surface.

Path A inverts that: substrate stays exactly as #366 merged it, and the visual-parity re-layer becomes a tight, scoped, reviewer-friendly ticket on the correct base.

## Follow-up tickets

### Ticket A — Re-layer reference-verbatim DOM on dev's post-#366 renderers

**Scope.** Re-apply PR #367's reference-verbatim DOM markup to the 17 account-detail inner-block renderers reverted to dev's version during this rebase. Preserve every `dailyos_envelope_consume_claim` call. The work is mechanical: each file becomes a small diff against current dev that swaps the rendered HTML structure without altering the claim-consumption path.

**Path-α check.** Per `feedback_visual_parity_is_never_path_alpha_for_wp_waves` — this is canonical v1.4.4 wave work, not maintenance.

**Source material.** PR #367 commit history on `public/feat/v1.4.4-wp-visual-parity` (pre-rebase) contains the reference-verbatim rewrites at commits `fa418208` (outlook-panel), `374d3f12` (stakeholder-grid), `8826a785` (value-commitments, strategic-landscape), `01ef9d12` (linear-issues-chapter→the-work), `21ff8847` (unified-timeline→state-of-play), `db1fa007` (sentiment-hero), and others. Also `.docs/design/reference/surfaces/account.html` lines 161-240 + region anchors. The `.docs/plans/v1.4.4-wp-surface-migration/ACCOUNT-DETAIL-*.html` paste artifacts in this rebased PR's diff are the post-rebase DOM-paste targets for parity validation.

**Acceptance.** Class set matches `.docs/design/reference/surfaces/account.html`. Every claim row still goes through `dailyos_envelope_consume_claim`. `php -l` clean. Block-kit integration fixtures still pass. Parity check via `wp/dailyos/dev-tools/parity-check.py` clean per section anchor.

**Files.** The 17 listed in "Reverted to dev" above.

### Ticket B — Rewire 3 net-new account-detail blocks through the post-#366 contract

**Scope.** `reports`, `the-record`, `watch-list` were added in PR #367 with the pre-#366 direct-envelope-read pattern. Rewire each to consume via `dailyos_envelope_collect_claim_refs` + `dailyos_envelope_consume_claim`.

**`the-record`** has a single direct read of `$envelope['record_entries']['items']` (around line 139). The other two are mostly scaffolds with no envelope consumption yet — wire them through the wrapper before they grow their claim-bearing rows.

**Wave assignment.** Per the abilities-runtime producer remediation plan (`.docs/plans/abilities-runtime-producer-remediation-waves.html`), this fits the W3 "remaining producers" wave alongside `threads`, `metadata_proposals`, and work/open-loop producer gaps.

**Acceptance.** No direct `$envelope[...]['items']` reads outside `_shared/envelope/`. All claim-bearing rows render trust band + claim id from receipt payloads. Block-kit integration fixtures cover all three.

## Mechanical state of the worktree

- Branch ref: `pr367-rebased` → `b17579d2cd17d4d34f3e5fb38a98b1b3df6d4e36`
- Worktree: `.worktrees/pr367-rebase` at the project root (recreate with `git worktree add .worktrees/pr367-rebase pr367-rebased` if it gets cleaned).
- Squashed from the original 29 commits on `feat/v1.4.4-wp-visual-parity` (history recoverable from `public/feat/v1.4.4-wp-visual-parity` and `git reflog`).
- Conflicts (16) resolved per Path A.
- PHP syntax (`php -l`) clean on all replaced files.
- `cargo check -p abilities-runtime --tests` clean.
- Local `dev` branch untouched at `f2effcb3` (verified post-amend).

## To ship

```
git push public pr367-rebased:feat/v1.4.4-wp-visual-parity --force-with-lease
```

The force-with-lease is required because history was rewritten (squash + rebase). Local pre-rebase history is preserved on `public/feat/v1.4.4-wp-visual-parity`'s reflog and the `pr367-branch` reflog if it needs to be inspected.

PR body needs updating to reflect Path A — current PR description still describes the pre-rebase scope including the 17 renderer rewrites that no longer ship in this PR. Suggested new framing in the PR body:

> Account-detail surface picks up new chapter scaffolds (`reports`, `the-record`, `watch-list`), DOM-paste workflow tooling, parity-surface seeding for the dev environment, theme CSS lifts, the block-kit integration fixture, and 9 contract-compatible renderer adjustments — all on top of #366's claim substrate as it landed.
>
> The 17 reference-verbatim account-detail inner-block rewrites originally scoped to this PR have been deferred to follow-up ticket A. They were authored against the pre-#366 envelope (no server-side `renderedText`) and adopted a per-block direct-read pattern that the abilities-runtime producer remediation plan now explicitly deprecates (`.docs/plans/abilities-runtime-producer-remediation-waves.html` W0 non-goal). Re-applying the reference-verbatim DOM on top of dev's `dailyos_envelope_consume_claim` consumers is a small, scoped diff per file and the right shape for review.
>
> Net-new blocks (`reports`, `the-record`, `watch-list`) ship with the same pre-#366 read pattern and need rewiring before v1.4.4 wave close — follow-up ticket B.

## References

- `wp/dailyos/blocks/_shared/envelope/envelope-resolver.php` on `public/dev` — the wrapper contract
- `src-tauri/src/services/account_fact_claims.rs` on `public/dev` — producer side (`CLAIM_TYPE = "account_fact"`, field-keyed claims like `arr_range_low`, `renewal_likelihood`, etc.)
- `wp/dailyos/blocks/account-detail/inner/sentiment-hero/render-functions.php` on `public/dev` — canonical post-#366 consumer pattern (`dailyos_envelope_collect_claim_refs` + `dailyos_envelope_consume_claim`)
- `.docs/plans/abilities-runtime-producer-remediation-waves.html` — the wave plan whose W0 non-goal language this rebase aligns with
- `.docs/plans/abilities-runtime-producer-audit-2026-05-23.md` — source audit explaining the pre-#366 envelope gap that PR #367's pattern was working around
- `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` — wave plan
- Engineering Ladder: `.docs/plans/engineering-ladder.md`
