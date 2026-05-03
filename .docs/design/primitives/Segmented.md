# Segmented

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `Segmented`
**`data-ds-spec`:** `primitives/Segmented.md`
**Variants:** `tint="turmeric" | "eucalyptus" | "larkspur"`
**Design system version introduced:** 0.3.0

## Job

Segmented lets users choose one value from a small fixed set without opening a menu. It keeps options visible side by side and uses tinted selected-state styling to make the current choice scannable.

## When to use it

- Two to five mutually exclusive options with short labels.
- Settings preferences where seeing all available choices matters.
- Density, cadence, tone, role preset, or other compact mode choices.
- Controls that should auto-save immediately on selection.

## When NOT to use it

- Long option lists; use select, combobox, or a searchable picker.
- Independent toggles; use `Switch` or checkbox controls.
- Commands or actions; use Button primitives.
- Text entry or user-supplied values; use `InlineInput`.

## States / variants

- `default` — group has a subtle tinted container and neutral unselected buttons.
- `selected` — active option sets `aria-pressed="true"` and uses stronger tint contrast.
- `hover` — unselected option gains visible affordance without looking selected.
- `focus-visible` — focused button receives keyboard ring inside the group.
- `disabled` — entire group or individual option is muted and non-interactive.
- `tint="turmeric"` — default Settings accent.
- `tint="eucalyptus"` — alternate accent for system or health contexts.
- `tint="larkspur"` — alternate accent for people or calm contexts.

## Tokens consumed

- `.docs/design/tokens/color.md` — tint backgrounds, selected text, neutral borders, focus ring.
- `.docs/design/tokens/typography.md` — compact button label typography via `--font-sans`.
- `.docs/design/tokens/spacing.md` — group padding, option padding, inter-option gap.
- `--color-spice-turmeric-15`, `--color-garden-larkspur-15` — known tint roles.
- `--font-sans`, `--space-xs`, `--space-sm` — label and group rhythm.

## API sketch

```tsx
<Segmented
  value={density}
  onChange={setDensity}
  tint="turmeric"
  options={[
    { value: "compact", label: "Compact" },
    { value: "regular", label: "Regular" },
    { value: "comfy", label: "Comfy" },
  ]}
  aria-label="Settings density"
/>
```

## Source

- **Mockup origin:** `.docs/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx` lines 39-51 (`Segmented`)
- **Code:** to be implemented in `src/components/ui/Segmented.tsx`

## Surfaces that consume it

- Settings (canonical): `.docs/design/surfaces/Settings.md`
- Future SurfaceMasthead consumers where compact mode selection is needed.

## Naming notes

`Segmented` is the primitive-tier name in the four-tier taxonomy. The mockup code already uses `Segmented`; no rename is pending.

## History

- 2026-05-03 — Proposed primitive for Wave 3 (Settings substrate).
