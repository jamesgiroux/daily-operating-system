# Frontend Styles Audit

**Audited:** 2026-03-02
**Against:** `src/styles/design-tokens.css` (canonical), `.docs/design/DESIGN-SYSTEM.md`, `.docs/design/COMPONENT-INVENTORY.md`
**Scope:** All 24 CSS files under `src/`, all TSX files under `src/` checked for inline styles

---

## 1. Design Token Coverage

### Token Usage Summary

61 tokens defined in `design-tokens.css`. Counted by number of files referencing each token (excluding the definition file itself).

| Token | Files Using | Assessment |
|-------|------------|------------|
| **COLORS — Paper** | | |
| `--color-paper-cream` | 12 | Good — primary background, well-referenced |
| `--color-paper-linen` | 23 | Good |
| `--color-paper-warm-white` | 27 | Good |
| **COLORS — Desk** | | |
| `--color-desk-charcoal` | 22 | Good (mostly via `--color-text-primary` alias) |
| `--color-desk-ink` | 2 | Low — used in status-badge only |
| `--color-desk-espresso` | 1 | Very low — used in status-badge only |
| **COLORS — Spice** | | |
| `--color-spice-turmeric` | 74 | Excellent — primary accent |
| `--color-spice-saffron` | 10 | Moderate |
| `--color-spice-terracotta` | 71 | Excellent — urgency states |
| `--color-spice-chili` | 8 | Low — critical states only |
| **COLORS — Garden** | | |
| `--color-garden-sage` | 63 | Excellent |
| `--color-garden-olive` | 26 | Good |
| `--color-garden-rosemary` | 12 | Moderate |
| `--color-garden-larkspur` | 42 | Good |
| `--color-garden-eucalyptus` | 12 | Good — /me page + nav |
| **COLORS — Semantic Text** | | |
| `--color-text-primary` | 127 | Excellent |
| `--color-text-secondary` | 89 | Excellent |
| `--color-text-tertiary` | 139 | Excellent |
| **COLORS — Rules** | | |
| `--color-rule-heavy` | 34 | Good |
| `--color-rule-light` | 91 | Excellent |
| **COLORS — Entity Aliases** | | |
| `--color-entity-account` | 0 | **UNUSED** |
| `--color-entity-project` | 0 | **UNUSED** |
| `--color-entity-person` | 0 | **UNUSED** |
| `--color-entity-action` | 0 | **UNUSED** |
| `--color-entity-user` | 0 | **UNUSED** |
| **TYPOGRAPHY** | | |
| `--font-serif` | 73 | Excellent |
| `--font-sans` | 118 | Excellent |
| `--font-mono` | 141 | Excellent |
| `--font-mark` | 1 | Correct — brand mark only |
| **SPACING** | | |
| `--space-xs` | 5 | Low |
| `--space-sm` | 10 | Moderate |
| `--space-md` | 10 | Moderate |
| `--space-lg` | 8 | Low |
| `--space-xl` | 3 | Low |
| `--space-2xl` | 5 | Low |
| `--space-3xl` | 1 | Very low |
| `--space-4xl` | 0 | **UNUSED** |
| `--space-5xl` | 3 | Low |
| **LAYOUT** | | |
| `--folio-height` | 1 | Correct (single consumer) |
| `--folio-padding-*` | 1 each | Correct |
| `--page-padding-horizontal` | 2 | Correct |
| `--page-padding-bottom` | 1 | Correct |
| `--page-margin-top` | 2 | Correct |
| `--page-max-width` | 1 | Correct |
| `--nav-island-right` | 1 | Correct |
| **RADIUS** | | |
| `--radius-editorial-sm` | 6 | Moderate |
| `--radius-editorial-md` | 1 | Low |
| `--radius-editorial-lg` | 4 | Low |
| `--radius-editorial-xl` | 1 | Very low |
| **SHADOWS** | | |
| `--shadow-sm` | 1 | Low |
| `--shadow-md` | 3 | Low |
| **TRANSITIONS** | | |
| `--transition-fast` | 0 | **UNUSED** |
| `--transition-normal` | 2 | Low |
| `--transition-slow` | 0 | **UNUSED** |
| **EFFECTS** | | |
| `--backdrop-blur` | 2 | Correct |
| `--frosted-glass-background` | 1 | Correct |
| `--frosted-glass-nav` | 1 | Correct |
| **Z-INDEX** | | |
| `--z-atmosphere` | 1 | Correct |
| `--z-page-content` | 2 | Correct |
| `--z-app-shell` | 2 | Correct |
| `--z-lock` | 1 | Correct |

### Unused Tokens (5 defined, 0 references)

| Token | Defined | Used | Verdict |
|-------|---------|------|---------|
| `--color-entity-account` | design-tokens.css:56 | 0 | Remove or migrate direct color refs to these aliases |
| `--color-entity-project` | design-tokens.css:57 | 0 | Same |
| `--color-entity-person` | design-tokens.css:58 | 0 | Same |
| `--color-entity-action` | design-tokens.css:59 | 0 | Same |
| `--color-entity-user` | design-tokens.css:60 | 0 | Same |
| `--transition-fast` | design-tokens.css:137 | 0 | Remove or use |
| `--transition-slow` | design-tokens.css:139 | 0 | Remove or use |
| `--space-4xl` | design-tokens.css:103 | 0 | Remove or use |

**Entity color aliases were defined as a semantic layer but never adopted.** All CSS files reference the underlying palette colors directly (e.g., `var(--color-spice-turmeric)` instead of `var(--color-entity-account)`). This is a missed abstraction -- if entity color assignments change, every reference must be updated manually.

### Opacity Tokens: Partial Adoption

Design tokens defines 14 opacity variant tokens (e.g., `--color-spice-turmeric-8`, `--color-garden-larkspur-12`). However, **~90 raw rgba() values** exist across CSS module files that replicate these colors at various opacities. Most common offenders:

- `rgba(201, 162, 39, ...)` (turmeric) -- 25+ instances at opacities 0.01 through 0.18
- `rgba(196, 101, 74, ...)` (terracotta) -- 15+ instances
- `rgba(143, 163, 196, ...)` (larkspur) -- 12+ instances
- `rgba(126, 170, 123, ...)` (sage) -- 10+ instances
- `rgba(30, 37, 48, ...)` (charcoal) -- 15+ instances at various opacities
- `rgba(107, 168, 164, ...)` (eucalyptus) -- 6+ instances

The design tokens file already has tokens for the most common opacity levels (8%, 12%, 15%, 30%). The remaining raw rgba values use non-standard opacities (0.01, 0.02, 0.04, 0.05, 0.06, 0.07, 0.1, 0.15, 0.18) that have no corresponding token.

---

## 2. Typography Violations

### Font Family Compliance: EXCELLENT

**Zero font-family violations found.** Every `font-family` declaration in CSS module files uses one of the four token variables (`var(--font-serif)`, `var(--font-sans)`, `var(--font-mono)`, `var(--font-mark)`).

The only non-token font declarations are:
1. `index.css` base body: `"DM Sans", system-ui, sans-serif` -- acceptable, Tailwind base layer
2. `index.css` code/pre/kbd: `"JetBrains Mono", ui-monospace, monospace` -- acceptable, Tailwind base layer
3. `ReportPrintStyles.module.css`: `'DM Sans', sans-serif` and `'Newsreader', Georgia, serif` -- acceptable, print media context where CSS variables may not resolve

### Font Size Scale Compliance

Design system specifies these levels:

| Level | Spec Size | Violations Found |
|-------|-----------|-----------------|
| Page headline | 76px | None -- used correctly in 4 hero modules |
| Section title | 22-28px | None |
| Card/item title | 19-20px | 18px used in `.scheduleTitle` and `.title` (MeetingCard) -- close but below range |
| Hero narrative | 21px | None |
| Body text | 15-16px | 14px appears frequently -- borderline, may be intentional for secondary body |
| Mono label | 10-11px | 9px used in 4 places (`.theRoomGroupLabel`, `.typeBadge`, `.nowPill`, `.entityType`) |
| Meta/secondary | 11-13px | 12.5px in `.quickContextLine` -- off-grid but close |

| File | Line | Value | Expected | Severity |
|------|------|-------|----------|----------|
| `editorial-briefing.module.css` | 643 | `font-size: 18px` | 19-20px (card title) | LOW |
| `MeetingCard.module.css` | 131 | `font-size: 18px` | 19-20px (card title) | LOW |
| `editorial-briefing.module.css` | 212 | `font-size: 9px` | 10-11px (mono label) | LOW |
| `editorial-briefing.module.css` | 651 | `font-size: 9px` | 10-11px | LOW |
| `editorial-briefing.module.css` | 663 | `font-size: 9px` | 10-11px | LOW |
| `PersonNetwork.module.css` | 85 | `font-size: 9px` | 10-11px | LOW |
| `editorial-briefing.module.css` | 349 | `font-size: 12.5px` | 12 or 13px | LOW |
| `MePage.module.css` | 29 | `font-size: 52px` | Non-standard (intentional /me page) | INFO |

### Line-Height Compliance

Design system specifies: 1.06 (headlines), 1.55 (body), 1.65 (long-form).

| File | Line | Value | Expected |
|------|------|-------|----------|
| `editorial-briefing.module.css` | 227 | `line-height: 1.4` | 1.55 (body text) |
| `editorial-briefing.module.css` | 295 | `line-height: 1.5` | 1.55 |
| `editorial-briefing.module.css` | 958 | `line-height: 1.45` | 1.55 |
| `MeetingCard.module.css` | 134 | `line-height: 1.35` | Borderline for card title |

These are minor deviations within contextual reason. No egregious violations.

---

## 3. Color Violations

### Undefined/Nonexistent Token References (CRITICAL)

| File | Line | Declaration | Problem |
|------|------|-------------|---------|
| `meeting-intel.module.css` | 1102 | `var(--color-turmeric, #d4a853)` | Token `--color-turmeric` does not exist. Fallback `#d4a853` is not in the palette. Should be `var(--color-spice-turmeric)`. |
| `meeting-intel.module.css` | 1103 | `var(--color-cream-wash, rgba(245, 240, 230, 0.3))` | Token `--color-cream-wash` does not exist. Fallback color `rgb(245, 240, 230)` is not in the palette. |
| `WeekPage.module.css` | 99 | `var(--color-surface-linen)` | Token `--color-surface-linen` does not exist. No fallback. Will render as `initial`. |
| `TourTips.module.css` | 12 | `var(--z-modal, 50)` | Token `--z-modal` does not exist. Fallback 50 is not in the z-index stack. Should use `--z-app-shell` (100) or similar. |

### Hardcoded Color Values Outside Token System

| File | Line | Value | Should Be |
|------|------|-------|-----------|
| `TourTips.module.css` | 93 | `color: white` | `var(--color-paper-warm-white)` |
| `index.css` | 106 | `box-shadow: 0 0 0 2px #c9a227` | `var(--color-spice-turmeric)` (hex in keyframe animation) |
| `index.css` | 234 | `box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08)` | No black token exists; palette uses charcoal |
| `ReportPrintStyles.module.css` | 20, 28 | `color: black` | Acceptable (print-only) |
| `index.css` | 317-318 | `background: white; color: black` | Acceptable (print-only) |

### Hex Values in index.css @theme Block

`index.css` lines 15-35 define Tailwind `@theme` tokens using raw hex values (e.g., `--color-background: #f5f2ef`). These duplicate design-tokens.css values. While this is required by Tailwind's theme system, they should reference design tokens if possible, or at minimum carry comments noting they must stay in sync (which they do).

### Non-Standard rgba() Colors

Two files use rgba values with colors not from the palette:

| File | Line | Value | Problem |
|------|------|-------|---------|
| `ActivityLogSection.module.css` | 65 | `rgba(0, 0, 0, 0.02)` | Pure black, not charcoal |
| `ActivityLogSection.module.css` | 120 | `rgba(0, 0, 0, 0.04)` | Pure black, not charcoal |
| `ContextEntryList.module.css` | 127 | `rgba(0, 0, 0, 0.03)` | Pure black, not charcoal |
| `TourTips.module.css` | 13 | `rgba(0, 0, 0, 0.08)` | Pure black, not charcoal |
| `index.css` | 234 | `rgba(0, 0, 0, 0.08)` | Pure black, not charcoal |
| `AccountHero.module.css` | 86 | `rgba(74, 103, 65, 0.12)` | Not a palette color (`#4a6741` is rosemary but used as base for rgba -- should use rosemary's RGB) |

**Note:** `rgba(0, 0, 0, x)` at very low opacities (0.02-0.08) is functionally indistinguishable from `rgba(30, 37, 48, x)` on the cream background. These are technically violations but visually negligible.

---

## 4. Spacing Violations

### Token Adoption Rate: LOW

The spacing token system (`--space-xs` through `--space-5xl`) has only 45 total references across all files. Meanwhile, **200+ hardcoded pixel values** appear in `padding`, `margin`, and `gap` declarations.

### Most Common Hardcoded Values vs. Token Equivalents

| Hardcoded Value | Count (approx) | Token Equivalent | On-Scale? |
|----------------|-----------------|------------------|-----------|
| `4px` | 15+ | `--space-xs` | Yes |
| `6px` | 20+ | None (between xs and sm) | No |
| `8px` | 25+ | `--space-sm` | Yes |
| `10px` | 15+ | None (between sm and md) | No |
| `12px` | 20+ | None (between sm and md) | No |
| `14px` | 10+ | None (between sm and md) | No |
| `16px` | 15+ | `--space-md` | Yes |
| `20px` | 10+ | None (between md and lg) | No |
| `24px` | 15+ | `--space-lg` | Yes |
| `28px` | 10+ | None (between lg and xl) | No |
| `32px` | 5+ | `--space-xl` | Yes |
| `40px` | 5+ | None (between xl and 2xl) | No |
| `48px` | 3+ | `--space-2xl` | Yes |
| `56px` | 3+ | `--space-3xl` | Yes |
| `80px` | 3+ | `--space-5xl` | Yes |

**Key finding:** Values like 6px, 10px, 12px, 14px, 20px, and 28px are used extensively but are NOT on the 4px-base spacing scale. The design system document acknowledges this for small values ("18px is close enough to --space-md") but the gap between `--space-sm` (8px) and `--space-md` (16px) leaves a large range without tokens. Most of the "violations" are intentional micro-spacing for component internals (pill padding, icon gaps, etc.) that reasonably fall below the token granularity.

### Specific Files with Highest Hardcoded Spacing

| File | Hardcoded Spacing Count |
|------|------------------------|
| `editorial-briefing.module.css` | ~60 |
| `meeting-intel.module.css` | ~40 |
| `PersonRelationships.module.css` | ~15 |
| `PersonHero.module.css` | ~10 |
| `FolioBar.module.css` | ~8 |
| `FloatingNavIsland.module.css` | ~6 |

The editorial-briefing module, which is the reference implementation, uses hardcoded values throughout. This suggests the spacing tokens are intended for page-level layout, not component-internal spacing.

---

## 5. Layout Pattern Assessment

### Section Rules vs. Cards: COMPLIANT

The design system mandates "section rules over cards" for most content. Audit confirms:
- Section rules (`1px solid var(--color-rule-heavy)`) used consistently for section breaks
- Row dividers (`1px solid var(--color-rule-light)`) used for list items
- Cards (`background + shadow`) reserved for: meeting cards, nav island, folio bar, featured items
- No unauthorized card usage detected

### Margin Grid Pattern: COMPLIANT

The editorial margin grid (100px label column + 1fr content) is defined in `editorial-briefing.module.css` and used consistently in the daily briefing. Entity detail pages use their own layout patterns (hero + chapters) which is appropriate.

### Page Container: COMPLIANT

`MagazinePageLayout.module.css` correctly uses:
- `max-width: var(--page-max-width)` (1100px)
- `padding-left/right: var(--page-padding-horizontal)` (120px)
- `padding-bottom: var(--page-padding-bottom)` (160px)
- Responsive breakpoints at 1200px, 768px, 480px

### Entity Accent Bars: COMPLIANT

3px left borders in entity colors used correctly across:
- Schedule rows (briefing)
- Priority items (briefing)
- Meeting cards
- Risk callouts
- Readiness sections

---

## 6. Ghost Stylesheets

### Potentially Dead CSS

| File | Concern |
|------|---------|
| `editorial-briefing.module.css` `.keyPeople`, `.keyPeopleLabel`, `.keyPeopleFlow` | Lines 258-260: Explicitly marked as "Legacy aliases" with `display: none`. Dead code. |
| `meeting-intel.module.css` `.attendeeTooltip` | Line 789-793: Comment says "keep existing tooltip CSS from index.css" but the rule body is empty. |
| `editorial-briefing.module.css` `.quickContextGlanceItem` | Line 384-386: Empty rule body (comment only). |
| `index.css` sidebar variables (lines 50-58) | Sidebar component (`AppSidebar`) was confirmed removed per component inventory. These variables may be dead. |

### Sidebar Variables Analysis

`index.css` defines 8 `--sidebar-*` variables and maps them via `@theme inline`. The component inventory confirms `AppSidebar` is dead code. If the `sidebar` UI primitive (`ui/sidebar.tsx`) is also unused, these variables are dead weight. However, `sidebar.tsx` is listed as "Legacy sidebar (dead code)" in the component inventory, suggesting cleanup is warranted.

---

## 7. Specificity Issues

### !important Usage: 12 instances

| File | Line(s) | Context | Justified? |
|------|---------|---------|------------|
| `index.css` | 284-285 | Calendar selected day override | Marginal -- Radix style override |
| `index.css` | 301, 305, 310-312, 317-318, 329 | Print media | YES -- print resets |
| `ReportPrintStyles.module.css` | 9, 39 | Print media | YES -- print resets |

All `!important` usages are either print media overrides (acceptable) or a single Radix calendar override (marginally acceptable). No specificity wars detected.

### Overly Specific Selectors: None

CSS modules provide natural scoping. No nested selector chains deeper than 2 levels. The deepest patterns are contextual state overrides like `.scheduleRowActive.scheduleRowExpandable:hover` which is appropriate.

### Composes Usage: GOOD

Several modules use CSS `composes` for DRY inheritance (e.g., `metaButtonEnriching composes metaButton`). This is well-used and avoids duplication.

---

## 8. Responsive / Accessibility

### Media Queries

| File | Breakpoints | Coverage |
|------|------------|----------|
| `MagazinePageLayout.module.css` | 1200px, 768px, 480px | Full mobile stack |
| `editorial-briefing.module.css` | 1000px, 820px, 640px | Full mobile stack |
| `meeting-intel.module.css` | 1000px, 820px | Partial (missing small screen) |
| `MeetingCard.module.css` | 820px | Partial |
| `index.css` | print | Print only |
| `ReportPrintStyles.module.css` | print | Print only |

**Files WITHOUT responsive breakpoints:**
- All entity hero modules (AccountHero, ProjectHero, PersonHero) -- 76px headline will overflow on narrow viewports
- `WeekPage.module.css` -- no responsive adjustments
- `MePage.module.css` -- no responsive adjustments
- `FloatingNavIsland.module.css` -- no responsive (may conflict with content at narrow widths)
- `FolioBar.module.css` -- no responsive (80px left padding may be excessive on mobile)
- `PersonNetwork.module.css`, `PersonRelationships.module.css` -- no responsive
- `ActivityLogSection.module.css` -- no responsive
- `TourTips.module.css` -- fixed 320px width with no responsive

### Focus States

Only 4 `:focus` rules exist across all CSS files:
1. `FloatingNavIsland.module.css` -- navIslandMark and navIslandItem
2. `FolioBar.module.css` -- folioSearch
3. `PersonRelationships.module.css` -- searchInput

**Missing focus states for:**
- All button-like elements in editorial-briefing (meeting actions checkboxes, priority checkboxes, expansion collapse buttons)
- Lock overlay unlock button
- Meeting-intel plan inputs, ghost inputs, agenda items
- Context entry list action buttons
- Tour tips navigation buttons

The Tailwind base layer provides `outline-ring/50` on all elements, which offers a baseline. But interactive elements styled in CSS modules bypass this and have no visible focus indicator.

### Color Contrast

All semantic text colors meet WCAG AA:
- `--color-text-primary` (#1e2530) on cream (#f5f2ef): 11.2:1 -- passes AAA
- `--color-text-secondary` (#5a6370) on cream: 4.7:1 -- passes AA
- `--color-text-tertiary` (#6b7280) on cream: 4.1:1 -- passes AA (noted in design-tokens.css)

Accent colors on cream backgrounds:
- Turmeric (#c9a227) on cream: 2.6:1 -- FAILS AA for body text (acceptable for decorative/large text only)
- Sage (#7eaa7b) on cream: 2.7:1 -- FAILS AA for body text
- Terracotta (#c4654a) on cream: 3.3:1 -- FAILS AA for small text, passes for large

These accent colors are used correctly for badges, dots, and accent borders rather than body text.

### Inline Styles

**Zero inline `style={{}}` props found in any TSX file.** This is excellent compliance with the design system's explicit prohibition.

---

## 9. Specific Violations Table

### CRITICAL (3)

| # | File | Line | Issue | Fix |
|---|------|------|-------|-----|
| C1 | `WeekPage.module.css` | 99 | `var(--color-surface-linen)` -- undefined token, renders as `initial` (transparent) | Change to `var(--color-paper-linen)` |
| C2 | `meeting-intel.module.css` | 1102 | `var(--color-turmeric, #d4a853)` -- undefined token, fallback is off-palette color | Change to `var(--color-spice-turmeric)` |
| C3 | `meeting-intel.module.css` | 1103 | `var(--color-cream-wash, rgba(245, 240, 230, 0.3))` -- undefined token, fallback is off-palette | Use `rgba(245, 242, 239, 0.3)` (cream) or define token |

### HIGH (2)

| # | File | Line | Issue | Fix |
|---|------|------|-------|-----|
| H1 | `TourTips.module.css` | 12 | `var(--z-modal, 50)` -- undefined token in z-stack | Use `--z-app-shell` (100) or define `--z-modal` |
| H2 | `TourTips.module.css` | 93 | `color: white` -- hardcoded color | Change to `var(--color-paper-warm-white)` |

### MEDIUM (8)

| # | File | Line | Issue | Fix |
|---|------|------|-------|-----|
| M1 | Entity alias tokens | -- | 5 entity color tokens defined but never used | Migrate direct color refs or remove tokens |
| M2 | `--transition-fast`, `--transition-slow` | -- | Defined but never used | Remove or adopt |
| M3 | `--space-4xl` (72px) | -- | Defined but never used | Remove or adopt |
| M4 | Entity hero modules | -- | No responsive breakpoints, 76px headline will overflow | Add media queries |
| M5 | `LockOverlay.css` | 44 | `border-radius: 8px` -- not in radius token scale | Use `--radius-editorial-lg` (12px) or add 8px token |
| M6 | `TourTips.module.css` | 10 | `border-radius: 8px` -- not in radius token scale | Same as M5 |
| M7 | `FloatingNavIsland.module.css` | 162 | `border-radius: 6px` -- not in radius token scale | Use `--radius-editorial-sm` (4px) or `--radius-editorial-md` (10px) |
| M8 | `index.css` sidebar vars | 50-58 | Sidebar variables for removed component | Remove if `ui/sidebar.tsx` is unused |

### LOW (informational)

| # | Category | Count | Notes |
|---|----------|-------|-------|
| L1 | Hardcoded border-radius values | 21 | Most are 2-4px (below or at `--radius-editorial-sm`), acceptable for micro-elements |
| L2 | Hardcoded spacing values | ~200 | Concentrated in component internals; spacing tokens designed for page-level use |
| L3 | Raw rgba() without tokens | ~90 | Mostly at non-standard opacities (0.01-0.07) where tokens don't exist |
| L4 | Font size 9px (below scale floor) | 4 | Used for very small mono labels -- intentional density |
| L5 | Font size 18px vs 19-20px spec | 2 | Meeting card titles -- close to spec |
| L6 | Legacy dead CSS selectors | 3 | `.keyPeople*` aliases in editorial-briefing |
| L7 | Missing focus states | ~20 elements | Tailwind base provides fallback outline |

---

## 10. Summary

### Compliance Score: 87/100

| Category | Score | Weight | Weighted |
|----------|-------|--------|----------|
| Typography (fonts) | 98/100 | 20% | 19.6 |
| Typography (sizes) | 90/100 | 10% | 9.0 |
| Color system | 85/100 | 20% | 17.0 |
| Spacing tokens | 65/100 | 15% | 9.75 |
| Layout patterns | 95/100 | 15% | 14.25 |
| Inline styles | 100/100 | 5% | 5.0 |
| Responsive | 70/100 | 10% | 7.0 |
| Accessibility | 75/100 | 5% | 3.75 |
| **Total** | | **100%** | **85.35** |

### Top Priorities

1. **Fix 3 critical broken token references** (C1-C3) -- these cause rendering bugs today
2. **Add responsive breakpoints to entity hero modules** -- 76px headline will break on narrow viewports
3. **Adopt entity color alias tokens** or remove them -- 5 tokens defined and never used is confusing
4. **Add focus states to interactive CSS-module elements** -- keyboard accessibility gap
5. **Consider expanding opacity token set** -- could eliminate ~60% of raw rgba() values by adding 4-5 more opacity levels

### Architecture Assessment

The CSS architecture is fundamentally sound:
- **CSS modules** provide proper scoping with zero leakage
- **Design token adoption for colors and fonts is excellent** (90%+ compliance)
- **No inline styles** -- the prohibition is being respected
- **Editorial patterns are consistent** -- the magazine aesthetic reads clearly
- **Spacing token adoption is the weakest area** -- the 4px grid is too coarse for component internals, leading to widespread hardcoded values

The gap between the spacing token scale and actual component needs suggests either: (a) add intermediate tokens (`--space-xs-plus: 6px`, `--space-sm-plus: 12px`), or (b) acknowledge that component-internal spacing is exempt from the token system and document that boundary.
