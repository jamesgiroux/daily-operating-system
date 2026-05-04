# Switch

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `Switch`
**`data-ds-spec`:** `primitives/Switch.md`
**Variants:** `size="sm"`
**Design system version introduced:** 0.3.0

## Job

Switch gives users a compact on/off control for a single binary setting. It is optimized for rows where the label and help text explain the consequence, while the control itself only communicates current state and toggling.

## When to use it

- A single binary preference such as enabled/disabled, shown/hidden, or allowed/blocked.
- Settings rows that auto-save immediately after the toggle changes.
- Dense Settings sections where a full checkbox label would duplicate the row label.
- Controls that can be expressed clearly as on or off.

## When NOT to use it

- Mutually exclusive choices with more than two options; use `Segmented` or radio controls.
- Actions that run a command, repair, export, or delete; use Button primitives.
- Values that require text entry; use `InlineInput`.
- Cases where the label must be part of the input itself; use a checkbox pattern.

## States / variants

- `off` — `aria-checked="false"`; thumb rests at the start and track uses neutral styling.
- `on` — `aria-checked="true"`; thumb rests at the end and track uses active accent styling.
- `hover` — track contrast increases to show the control is interactive.
- `focus-visible` — keyboard ring surrounds the full switch.
- `disabled` — muted track and thumb, no state change.
- `size="sm"` — compact Settings row switch; the default for this primitive.

## Tokens consumed

- `.docs/design/tokens/color.md` — neutral track, active track, thumb, focus ring, disabled state.
- `.docs/design/tokens/typography.md` — no visible label text; relies on surrounding row typography.
- `.docs/design/tokens/spacing.md` — switch width, height, thumb inset, focus-ring offset.
- `--color-desk-charcoal-4`, `--color-text-secondary` — neutral and muted roles.
- `--space-xs` — compact inset and row alignment.

## API sketch

```tsx
<Switch
  checked={briefingEnabled}
  onCheckedChange={setBriefingEnabled}
  aria-label="Enable daily briefing"
/>
```

## Source

- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx` lines 31-37 (`Switch`)
- **Code:** to be implemented in `src/components/ui/Switch.tsx`

## Surfaces that consume it

- Settings (canonical): `.docs/design/surfaces/Settings.md`
- Future SurfaceMasthead consumers where a masthead-level binary control is needed.

## Naming notes

`Switch` is the primitive-tier name in the four-tier taxonomy. The mockup code already uses `Switch`; no rename is pending.

## History

- 2026-05-03 — Proposed primitive for Wave 3 (Settings substrate).
