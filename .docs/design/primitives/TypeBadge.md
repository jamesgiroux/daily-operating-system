# TypeBadge

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `TypeBadge`
**`data-ds-spec`:** `primitives/TypeBadge.md`
**Variants:** `accountType="customer" | "internal" | "partner"` (extensible)
**Design system version introduced:** 0.1.0

## Job

Render an account-type categorical badge (Customer / Internal / Partner) with the appropriate color treatment. The mockup `_shared/.type-badge` design pattern, productionized.

## When to use it

- On account hero (top of AccountDetail) — signals what kind of relationship this account is
- In account list rows for quick scanning
- Anywhere you need to surface an account-type categorization

## When NOT to use it

- For entity reference inline — use `EntityChip`
- For status / state — use `Pill` with appropriate tone
- For arbitrary categorical labels without account-type meaning — use `Pill`

## States / variants

- `accountType="customer"` — turmeric tone (background turmeric-12, text turmeric)
- `accountType="internal"` — larkspur tone (background larkspur-15, text larkspur)
- `accountType="partner"` — rosemary tone (background rosemary-12, text rosemary)
- Optional `editable` — adds a chevron and dropdown affordance (existing AccountTypeBadge in src does this)

Mockup `.type-badge` adds an optional `chev` for the dropdown variant.

## Tokens consumed

- `--color-spice-turmeric-12`, `--color-spice-turmeric` (customer)
- `--color-garden-larkspur-15`, `--color-garden-larkspur` (internal)
- `--color-garden-rosemary-12`, `--color-garden-rosemary` (partner)
- `--font-mono` (label, small-caps treatment)
- `--space-xs`

## Composition

Visually composes `Pill` (compact variant) + uppercase label + optional chevron for dropdown mode.

## API sketch

```tsx
<TypeBadge accountType="customer" />
<TypeBadge accountType="internal" />
<TypeBadge accountType="partner" editable onChange={handleTypeChange} />
```

## Source

- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/surfaces/_shared/primitives.css` (`.type-badge`)
- **Existing src/ component:** `AccountTypeBadge` inside `src/components/account/AccountHero.tsx` (lines 113-166) — local dropdown reimplementation flagged in Audit 02. Wave 1 extracts as canonical primitive; AccountHero migrates to consume.

## Surfaces that consume it

AccountDetail (hero), AccountsPage (list rows), EmailDetail (account context).

## Naming notes

Audit 02 surfaced `AccountTypeBadge` (the local dropdown implementation) as a "should be reused but reinvented" case. The canonical primitive is `TypeBadge` (generic name; account is the only current use but extensible to other type categoricals if needed). The local AccountTypeBadge dropdown logic factors into the `editable` variant of this primitive.

## History

- 2026-05-02 — Promoted to canonical from mockup `_shared/.type-badge` + production `AccountTypeBadge`.
- Audit 02 — flagged AccountTypeBadge as local-reimplementation candidate.
