# ADR 0132 — Pill primitive dual-existence (chrome + block)

**Status**: Accepted
**Date**: 2026-05-19
**Decision drivers**: v1.4.4 W3 chrome lane (DOS-722)

## Context

After the v1.4.4 W3 chrome lane lands, the `Pill` primitive exists in two places in the WordPress layer:

- **Chrome version** at `wp/dailyos/theme/assets/chrome/styles/Pill.module.css` (selectors `.Pill_pill`, `.Pill_pillDot`, etc.). Runtime-injected via `chrome.js` as a child primitive inside FloatingNavIsland tooltips, FolioBar status indicators, AtmosphereLayer watermark labels. Chrome consumers cannot use Gutenberg block markup because chrome injects after WP renders the page.
- **Block version** at `wp/dailyos/blocks/pill/` (selectors `.dailyos-pill`, `.dailyos-pill__dot`, etc.). Ships with `block.json`, `render.php`, `edit.js`. Authorable via Site Editor — users can drop a Pill into post/page body content.

Same conceptual primitive, two universes. Selector prefixes differ (`.Pill_*` vs `.dailyos-pill*`), so there is no CSS collision today. But without an explicit invariant, the next primitive that goes dual (Pill, Badge, Chip…) drifts in the same direction with no signal to anyone.

## Decision

**Accept Pill's dual existence as a documented pattern.** Do not consolidate.

Pill is special because it serves two genuinely different roles:

- **As a content-authoring primitive** — users want to drop Pills into body content via Site Editor. Block path is the right fit.
- **As a chrome-internal primitive** — chrome.js needs to render Pills client-side inside DOM it has already injected (status dots, watermark labels, etc.). Block markup cannot survive that path; CSS-class instantiation is the only option.

The cost of forcing a single source-of-truth here is much higher than the cost of maintaining duality:

- **Option A** (block-only): chrome.js would have to construct Gutenberg block markup at runtime, which is novel and fragile, and it cancels the chrome.js architecture's main advantage (sync from canonical Tauri reference).
- **Option B** (chrome-only): drops Site Editor authoring of Pills entirely, contradicting [`project_wp_block_custom_vs_core_strategy`](../../tools/dailyos-mcp/note) (many small blocks → editable content).

Both A and B trade a small ongoing maintenance cost (keeping two Pill CSS files token-aligned) for a much larger structural cost. Option C trades nothing.

## Invariant

Pill is the only primitive permitted to live in both `wp/dailyos/theme/assets/chrome/styles/` AND `wp/dailyos/blocks/`. New primitives must pick one universe.

This is enforced by `wp/dailyos/scripts/check-chrome-block-collision.sh` (the allowlist gate). The script fails if any primitive name other than `pill` appears in both directories. The allowlist starts at `pill` only and is amended only by a follow-on ADR.

## Consequences

**Positive:**

- Chrome.js stays canonical-synced; no special-cased block construction at runtime.
- Site Editor authoring of Pills works unchanged.
- Future primitives get a clear, automated signal when they try to go dual: pick one universe or write an ADR.

**Negative:**

- Two CSS files must stay token-aligned for the Pill. Both consume the same `--color-spice-*`, `--color-garden-*` palette tokens via `var(--*)` references, so theme.json updates flow into both without code edits — but new Pill states (e.g., a `disabled` variant) have to land in both files.
- L0 reviewers must check that any new primitive proposal explicitly states which universe it belongs to.

## Allowlist gate operational notes

`wp/dailyos/scripts/check-chrome-block-collision.sh`:

- Scans `wp/dailyos/theme/assets/chrome/styles/` for module CSS files.
- Scans `wp/dailyos/blocks/` for block directories.
- Lowercases module basenames (`Pill.module.css` → `pill`) and compares against block directory names.
- Exits 0 if every collision is on the allowlist (`pill`).
- Exits 1 with a per-collision diagnostic otherwise.

The script ships in this same PR. Wire it into CI alongside the existing `wp/dailyos/scripts/run-grep-gates.sh` invocations (one-line addition to the existing lint job).

## References

- L0 packet `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` V1.3 §5.2 AC #6 + §10 invariants + AC #28
- Linear DOS-722 (this ticket)
- Memory: many-small-blocks strategy
- [ADR 0130](0130-surface-independent-composition-contract.md) — surface-independent composition contract
