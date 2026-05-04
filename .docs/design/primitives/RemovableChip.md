# RemovableChip

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `RemovableChip`
**`data-ds-spec`:** `primitives/RemovableChip.md`
**Variants:** `default`
**Design system version introduced:** 0.3.0

## Job

RemovableChip displays a selected or applied item with a built-in removal affordance. It is for editable collections where each chip is an object the user can remove without opening another control.

## When to use it

- Selected filters, included domains, connected labels, or small editable item collections.
- Settings rows where removing one item should be immediate and local.
- Values that need to remain visible after selection.
- Compact groups where each item has the same remove behavior.

## When NOT to use it

- Passive labels, statuses, or categories; use `Pill`.
- Named semantic badges that only communicate meaning; use the corresponding badge primitive.
- Large editable records; use a list row or table row pattern.
- Command buttons; use Button primitives.
- Non-removable chips; prefer `Pill` or a named display primitive so the interaction is unambiguous.

## States / variants

- `default` — label and × affordance are visible inside one compact chip.
- `hover` — chip boundary and × affordance increase contrast.
- `focus-visible` — keyboard ring appears on the remove target or chip wrapper.
- `removing` — optional transient state while persistence completes.
- `disabled` — chip remains readable but × is muted and inactive.

## Tokens consumed

- `.docs/design/tokens/color.md` — chip background, border, label text, remove affordance, hover state.
- `.docs/design/tokens/typography.md` — compact label typography via `--font-sans`.
- `.docs/design/tokens/spacing.md` — chip padding, label-to-remove gap, group wrap gap.
- `--color-desk-charcoal-4`, `--color-text-secondary` — neutral chip and affordance roles.
- `--font-sans`, `--space-xs`, `--space-sm` — compact chip rhythm.

## API sketch

```tsx
<RemovableChip
  label="gmail.com"
  onRemove={() => removeDomain("gmail.com")}
  aria-label="Remove gmail.com"
/>
```

## Source

- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx` lines 73-80 (`Chip`)
- **Code:** to be implemented in `src/components/ui/RemovableChip.tsx`

## Surfaces that consume it

- Settings (canonical): `.docs/design/surfaces/Settings.md`
- Future SurfaceMasthead consumers where compact editable metadata collections appear.

## Naming notes

`RemovableChip` is the primitive-tier canonical name in the four-tier taxonomy. The mockup code calls it `Chip`; implementation should promote the clearer interaction name.

## History

- 2026-05-03 — Proposed primitive for Wave 3 (Settings substrate).
