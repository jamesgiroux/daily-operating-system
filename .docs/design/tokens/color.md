# Color tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**Design system version introduced:** 0.1.0

## Job

The complete color vocabulary for DailyOS. Every color rendered by the app should resolve to one of these tokens — directly, via tint, via semantic alias, or via entity alias. Direct hex values in source files are a smell.

## Families

DailyOS color is organized into named families, each with a semantic role. The four-family palette (paper / desk / spice / garden) was set by ADR-0076 (brand identity).

### Paper — grounds and backgrounds

- `--color-paper-cream` `#f5f2ef` — page ground, base surface
- `--color-paper-linen` `#e8e2d9` — secondary paper surface
- `--color-paper-warm-white` `#faf8f6` — primary card surface

### Desk — frame and primary text

- `--color-desk-charcoal` `#1e2530` — primary dark ink
- `--color-desk-ink` `#2a2b3d` — alternate dark ink

### Spice — warm accents (accounts, active, urgency)

- `--color-spice-turmeric` `#c9a227` — primary accent, accounts, warm emphasis
- `--color-spice-saffron` `#deb841` — secondary warm accent
- `--color-spice-terracotta` `#c4654a` — urgency, errors, overdue
- `--color-spice-chili` `#9b3a2a` — deep red, critical states

### Garden — cool accents (people, projects, success, forward)

- `--color-garden-sage` `#7eaa7b` — success, healthy, prepped
- `--color-garden-olive` `#6b7c52` — projects, muted secondary
- `--color-garden-rosemary` `#4a6741` — deep green success variant
- `--color-garden-larkspur` `#8fa3c4` — people, calm, forward
- `--color-garden-eucalyptus` `#6ba8a4` — user (/me), self, professional context

### Text — semantic text colors

- `--color-text-primary` → `--color-desk-charcoal` — headlines, primary text
- `--color-text-secondary` `#5a6370` — body, secondary content
- `--color-text-tertiary` `#6b7280` — labels, hints (WCAG AA compliant)

### Rule — dividers, borders, grid lines

- `--color-rule-heavy` `rgba(30, 37, 48, 0.12)` — primary dividers
- `--color-rule-light` `rgba(30, 37, 48, 0.06)` — soft separators

### Surface — semantic backgrounds (DOS-62)

Ghost tokens consumed by `ActionRow`, `Emails`, `DatabaseRecovery`, `ContextSource`. Map to paper palette so backgrounds render opaque.

- `--color-surface` → `--color-paper-cream`
- `--color-surface-primary` → `--color-paper-warm-white`
- `--color-surface-secondary` → `--color-paper-linen`
- `--color-surface-inset` → `--color-desk-charcoal-4`
- `--color-surface-subtle` → `--color-black-4`

### Named tokens — surface identity (DOS-357, design system D1)

Use these when the rendered element represents a specific surface kind in context (an account hero, a person card, the user's own `/me` surface). Direct uses of underlying paint tokens remain valid where the meaning is the paint color rather than the surface identity.

- `--color-account` → `--color-spice-turmeric`
- `--color-project` → `--color-garden-olive`
- `--color-person`  → `--color-garden-larkspur`
- `--color-action`  → `--color-spice-terracotta`
- `--color-self`    → `--color-garden-eucalyptus`

Each named token has alpha variants matching the underlying paint's alpha set (e.g. `--color-account-8`, `--color-account-15`). See `src/styles/design-tokens.css` for the exact list per family.

> Renamed from `--color-entity-*` (2026-05-03). "Entity" was internal jargon — surface authors think "account, project, person." `user` → `self` to match the `/me` surface name. Use the named tokens at any callsite where the color carries surface semantics; the indirection lets us swap the underlying paint (e.g., rebrand accounts from turmeric to a different family) without grepping every callsite.

### When named vs paint

| Use named (`--color-account`) | Use paint (`--color-spice-turmeric`) |
|---|---|
| The color *means* "this is an account thing" — account hero, account-tinted badge, account-context border | The color is decorative — a turmeric divider, an editorial accent, a warm-paper highlight that has nothing to do with being an account |
| Swapping the underlying paint should propagate everywhere this color is used | Swapping the underlying paint would feel wrong because "no, this should always be turmeric regardless of branding" |
| State colors that *also* happen to be used for an entity in some places — health-yellow that's incidentally turmeric | State colors should stay as paint or get their own state token (`--color-trust-*`, `--color-state-*` when added) |

### Trust band — user-facing trust render bands (per v1.4.0 substrate)

> **Status: proposed** — to be added during Wave 1 implementation. v1.4.0 substrate ships `likely_current / use_with_caution / needs_verification` as render trust bands (DOS-320). The design system needs token-level color decisions for each band.

Naming convention: `--color-trust-likely-current`, `--color-trust-use-with-caution`, `--color-trust-needs-verification`. Likely mapping (TBD):
- `likely_current` → garden-sage family
- `use_with_caution` → spice-saffron family
- `needs_verification` → spice-terracotta family

### Tint variants

Every accent token has opacity tints in standard percentages (4, 5, 6, 7, 8, 10, 12, 15, 18, 20, 25, 30, 60). Use the named token, not raw `rgba()`. Examples:

- `--color-spice-turmeric-8` `rgba(201, 162, 39, 0.08)`
- `--color-garden-sage-15` `rgba(126, 170, 123, 0.15)`

Full tint list lives in `src/styles/design-tokens.css`.

### Overlay — modal backdrops

- `--color-overlay-light` `rgba(0, 0, 0, 0.4)`
- `--color-overlay-medium` `rgba(0, 0, 0, 0.5)`

### Black opacity — neutral shadows, subtle backgrounds

- `--color-black-2` through `--color-black-8`

## When to use which

- **Backgrounds:** start with `--color-surface-*` (semantic). Fall through to `--color-paper-*` when the semantic alias doesn't fit.
- **Text:** always `--color-text-*` (semantic). Never raw hex; never desk tokens directly.
- **Accents:** prefer named tokens (`--color-account`, `--color-project`, `--color-person`, `--color-action`, `--color-self`) when the color signals surface identity. Use raw spice/garden tokens when the color signals state (success, urgency, warning) without surface-identity meaning.
- **Trust UI:** use `--color-trust-*` (when added in Wave 1) for surface-level trust rendering. These are derived from the v1.4.0 substrate trust band contract.
- **Borders / rules:** `--color-rule-heavy` for primary dividers, `--color-rule-light` for soft.
- **Modal backdrops:** `--color-overlay-*`.

## When NOT to use direct paint tokens

- Any UI signaling surface identity (account, project, person, action, self) → use the named token.
- Trust band rendering → use trust token (when added).
- Surface backgrounds → use semantic surface alias.

## Source

- **Code:** `src/styles/design-tokens.css`
- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/_shared/tokens.css` (will move to `.docs/design/reference/_shared/tokens.css` per DS-XCUT-02)

## History

- 2026-05-02 — Promoted to canonical. DOS-357 reintroduced entity color aliases per design system D1 (synthesis).
- DOS-62 — surface aliases established
- DOS-61 — entity aliases removed (later reverted by DOS-357)
- ADR-0076 — original brand identity definition
