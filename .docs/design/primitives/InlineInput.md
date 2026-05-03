# InlineInput

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `InlineInput`
**`data-ds-spec`:** `primitives/InlineInput.md`
**Variants:** `font="mono" | "sans"`
**Design system version introduced:** 0.3.0

## Job

InlineInput lets users edit a short piece of text in place without leaving the surrounding settings row. It keeps the current value readable first, then exposes editability through a small pencil affordance and focus styling.

## When to use it

- Short, single-line values that are edited directly in context.
- Settings values that auto-save after change or blur.
- Identifiers, names, URLs, keys, or labels where the field should feel quieter than a full form input.
- Values that need either mono or sans rendering based on content type.

## When NOT to use it

- Multi-line content; use a textarea primitive or a richer editor.
- Choice from a fixed set; use `Segmented`, `Switch`, select, or radio controls.
- Destructive or transactional actions; use Button primitives with explicit labels.
- Inline labels or statuses; use `Pill` or a named badge primitive.

## States / variants

- `default` — value reads as plain inline content with subtle input boundary and pencil affordance.
- `hover` — boundary and pencil become more visible to signal click-to-edit.
- `focus` / `editing` — input receives keyboard focus, pencil remains present, and boundary uses the active accent.
- `disabled` — value is readable but muted; no editing or hover affordance.
- `font="mono"` — renders the value with `--font-mono` for URLs, keys, and technical identifiers.
- `font="sans"` — renders the value with `--font-sans` for names and prose-like labels.
- `width` — optional minimum width for stable layout in form rows.

## Tokens consumed

- `.docs/design/tokens/color.md` — text, muted affordance, input boundary, active accent, disabled state.
- `.docs/design/tokens/typography.md` — `--font-mono`, `--font-sans`, inline control sizing.
- `.docs/design/tokens/spacing.md` — horizontal padding, control height, pencil gap.
- `--color-desk-charcoal-4`, `--color-text-secondary` — quiet boundary and affordance roles.
- `--space-xs`, `--space-sm` — internal spacing.

## API sketch

```tsx
<InlineInput
  value={profileName}
  onChange={setProfileName}
  font="sans"
  width={220}
  aria-label="Profile name"
/>
```

## Source

- **Mockup origin:** `.docs/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx` lines 18-29 (`InlineInput`)
- **Code:** to be implemented in `src/components/ui/InlineInput.tsx`

## Surfaces that consume it

- Settings (canonical): `.docs/design/surfaces/Settings.md`
- Future SurfaceMasthead consumers where masthead metadata becomes directly editable.

## Naming notes

`InlineInput` is the primitive-tier name in the four-tier taxonomy. The mockup code already uses `InlineInput`; no rename is pending.

## History

- 2026-05-03 — Proposed primitive for Wave 3 (Settings substrate).
