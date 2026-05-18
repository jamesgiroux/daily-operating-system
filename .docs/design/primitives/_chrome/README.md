## Primitive chrome

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-18
**`data-ds-name`:** `PrimitiveChrome`
**`data-ds-spec`:** `primitives/_chrome/README.md`
**Design system version introduced:** 0.1.0 (v1.4.3 W2 PR-D1, DOS-682)

## Job

Shared empty / loading / error state renderers consumed by every Wave 1 primitive. Self-contained; token-driven; surface-agnostic in API; per-surface in implementation (Tauri React + WP PHP partials).

Distinct from `src/components/editorial/EmptyState.tsx` — that component is editorial page-scope chrome (h2 + paragraph + buttons), NOT primitive-slot chrome.

## When to use it

- Any Wave 1 primitive (Pill, HealthBadge, StatusDot, Avatar, TrustBandBadge, IntelligenceQualityBadge, FreshnessIndicator, ProvenanceTag, EntityChip, TypeBadge, ScoreBand) needs to render an empty / loading / error state in-slot.
- The primitive's render function MUST consume the shared chrome rather than inlining state-specific markup.

## When NOT to use it

- Page-scope empty states (entire entity-detail page with zero data) — use `editorial/EmptyState`.
- Per-claim error rendering with inline action affordances (Retry button on a feedback writeback failure) — that's surface-tier feedback chrome owned by v1.4.4 W4, not primitive-tier.

## Non-negotiable contract (V1.3 enumerated at L0 per cycle-3 design F2)

The chrome service spec MUST cover all six items below. Design-reviewer L4 sign-off cannot pass with any item missing.

### 1. Named theme.json palette entries consumed

- `--color-desk-charcoal-4` — empty + loading background
- `--color-text-tertiary` — empty + loading text
- `--color-spice-terracotta-15` — error background
- `--color-spice-terracotta` — error text

These map to `var(--wp--preset--color--desk-charcoal-4)` / `wp--preset--color--text-tertiary` / `wp--preset--color--spice-terracotta-15` / `wp--preset--color--spice-terracotta` in the WP block render path via the W1 translate-tauri.mjs token-mapping manifest (DOS-685).

### 2. Skeleton density + spacing tokens

- `--space-xs` (4px) — internal gap between optional dot/label
- `--font-mono`, font-size 10px, font-weight 400, letter-spacing 0.06em, text-transform uppercase — chrome label typography
- padding 3px 10px — chrome label box
- border-radius 3px — chrome label corner

### 3. MUST NOT consume `src/components/editorial/EmptyState.tsx` boundary

Primitive chrome and editorial chrome are different components with different scopes. Primitives MUST NOT import or render `editorial/EmptyState` for in-slot state rendering. CI fixture (negative fixture §8 #12) asserts no `from .*editorial/EmptyState` imports under `src/components/ui/_chrome/` or `wp/dailyos/blocks/`. Cycle-2 code-reviewer F-2 + V1.2 §5.8 rename context.

### 4. Focus management for empty/error CTA targets

For W2 (display-only primitives) chrome states have NO CTAs and NO focus management is required. Default `outline: none` on the chrome span is acceptable.

For v1.4.4 W4 (editable variants + feedback router) when chrome renders an actionable element (e.g., "Retry" button in error state), focus management contract:
- Action element MUST be keyboard focusable (`tabindex="0"` if not a native button/anchor)
- Visible focus ring uses `--color-account` outline (matches TrustBandBadge focus pattern)
- After successful action, focus returns to the primitive's natural focus target (the primitive's main element)

### 5. RTL + dark-mode coverage

Chrome must render correctly under both axes per §7.1 Axis 2:
- **RTL**: text/dot order mirrors via `[dir="rtl"]` parent — primitive chrome inherits.
- **Dark mode**: tokens auto-resolve via theme. Chrome surface MUST NOT hardcode light-mode color values; all colors flow through `--color-*` tokens (CI gate per §5.9 token-mapping manifest).

### 6. When to escalate to `editorial/EmptyState`

| Case | Use |
|---|---|
| Single primitive slot with no data (one ScoreBand renders no data) | Primitive chrome (`PrimitiveEmpty`) |
| Single primitive slot, claim resolution pending | Primitive chrome (`PrimitiveLoading`) |
| Single primitive slot, projection error | Primitive chrome (`PrimitiveError`) |
| Entity-detail page with zero claims for the entity | Editorial chrome (`editorial/EmptyState`) |
| Surface composition where ALL primitives would render empty | Editorial chrome (`editorial/EmptyState`) — the surface layer makes this call, not the individual primitive |

Surfaces decide whether to render primitive-slot chrome or escalate to editorial chrome based on the breadth of the empty state. The primitive layer always defaults to its own chrome; surfaces upgrade when warranted.

## Source

- **Code (Tauri):** `src/components/ui/_chrome/PrimitiveChrome.tsx` + `PrimitiveChrome.module.css`. Exports `PrimitiveEmpty`, `PrimitiveLoading`, `PrimitiveError`.
- **WP partials:** `wp/dailyos/blocks/_shared/chrome/render-empty.php`, `render-loading.php`, `render-error.php`. Functions: `dailyos_chrome_render_empty(?string $label)`, `_loading`, `_error`.
- **Rust integration fixture:** `src-tauri/abilities-runtime/tests/chrome_service_integration_fixture.rs` — 44 in-test compositions (4 state branches × 11 primitives) exercising the chrome partials in isolation, using mock primitive fixtures (NOT dependent on per-primitive block dirs from PR-D2/D3/D4).
- **PHPUnit test:** `wp/dailyos/tests/blocks/chrome/ChromeServiceTest.php` — snapshot + no-inline-style assertions on each partial.
- **CI gate:** `.github/workflows/block-kit-integration.yml` "Chrome service coverage" step (scoped to existing block dirs only — does not fail PR-D1 for absent dirs per V1.4 cycle-4 codex consult condition).

## Surfaces that consume it

Every Wave 1 primitive (11 total — see `README.md`). The chrome service is a foundation dependency that PR-D2/D3/D4 build on.

## History

- 2026-05-18 — Initial author for v1.4.3 W2 PR-D1 per DOS-682 §5.8 + V1.4 design-system anchoring. `proposed` until each consuming primitive's WP block lands; promoted to `integrated` per the WP-block-consumption-counts rule once any block.json under `wp/dailyos/blocks/<slug>/` requires this partial.
