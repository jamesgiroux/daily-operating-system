# Pill

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `Pill`
**`data-ds-spec`:** `primitives/Pill.md`
**Variants:** `tone="sage" | "turmeric" | "terracotta" | "larkspur" | "neutral"`
**Design system version introduced:** 0.1.0

## Job

The visual primitive for any inline status / label / category badge. Generic, unopinionated, tone-driven. Named primitives that convey semantic meaning (`EntityChip`, `TypeBadge`, `QualityBadge`, etc.) compose Pill underneath.

## When to use it

- Inline labels carrying status or categorical meaning (a tag, a state, a label)
- When you need a recognizably "pill" affordance — rounded, subtle background tint, mono or sans label
- When a named primitive (EntityChip, TypeBadge) doesn't yet exist for your case — but consider creating one

## When NOT to use it

- For removable items with × — that's `Chip` (different interaction)
- For action buttons — use `Button`
- When the meaning is entity identity, freshness, quality, or trust — use the corresponding named primitive that composes Pill

## States / variants

- `tone="sage"` — success, healthy, ready (background sage-15, text rosemary)
- `tone="turmeric"` — emphasis, account, active (background turmeric-15, text turmeric-darkened)
- `tone="terracotta"` — urgency, error, overdue (background terracotta-15, text chili)
- `tone="larkspur"` — people, calm, internal (background larkspur-15, text darkened larkspur)
- `tone="neutral"` — generic state, no semantic color (background charcoal-4, text secondary)

Optional `dot` variant adds a leading colored dot (matches tone).

## Tokens consumed

- `--color-garden-sage-15`, `--color-garden-rosemary` (sage tone)
- `--color-spice-turmeric-15` (turmeric tone)
- `--color-spice-terracotta-15`, `--color-spice-chili` (terracotta tone)
- `--color-garden-larkspur-15` (larkspur tone)
- `--color-desk-charcoal-4`, `--color-text-secondary` (neutral tone)
- `--font-sans` (label)
- `--space-xs`, `--space-sm` (padding)

## API sketch

```tsx
<Pill tone="sage" dot>Ready</Pill>
<Pill tone="turmeric">Account</Pill>
<Pill tone="neutral">Draft</Pill>
```

CSS class form (matches `_shared/.pill`):

```html
<span class="pill" data-tone="sage" data-ds-name="Pill" data-ds-spec="primitives/Pill.md">
  <span class="pill-dot"></span>Ready
</span>
```

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/_shared/primitives.css` (`.pill`)
- **Code:** to be implemented as React primitive in `src/components/ui/Pill.tsx` (Wave 1 follow-on); current ad-hoc usages live across surfaces (see Audit 03 for the 7+ pill drift cases)

## Surfaces that consume it

Wave 1 consumers (direct or via composing primitives): DailyBriefing, AccountDetail, MeetingDetail, Settings, ProjectDetail, PersonDetail.

## Naming notes

`Pill` is the visual primitive. Named primitives compose it: `EntityChip`, `TypeBadge`, `QualityBadge`, `TrustBandBadge`, `ProvenancePill`, `MeetingStatusPill`. `Chip` (removable) is its own primitive.

## History

- 2026-05-02 — Promoted to canonical from `_shared/.pill`. Audit 03 surfaced 7+ existing pill variants across surfaces — consolidation to this primitive begins with v1.4.3.
