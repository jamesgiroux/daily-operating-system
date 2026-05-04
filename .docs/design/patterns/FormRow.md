# FormRow

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `FormRow`
**`data-ds-spec`:** `patterns/FormRow.md`
**Variants:** `default`, `dense`, `stacked`, `readonly`
**Design system version introduced:** `0.3.0`

## Job

The universal setting row for exposing one configurable value with clear label/help context, a control, and optional right-aligned secondary content. It gives Settings one predictable contract for every editable or inspectable preference.

## When to use it

- A settings section needs to expose a single configurable value or small value group.
- The row needs a persistent label plus optional help text beside an interactive control.
- A setting needs secondary context, metadata, or an inline action in a right-aligned aux slot.
- A future settings-like surface needs the same label/help | control | aux rhythm.

## When NOT to use it

- A whole connector, account, or integration needs its own status, actions, and nested rows; use `ConnectorSurface`.
- A section needs an eyebrow, title, epi, meta, or section action; use `SectionHead`.
- A control can stand alone inside dense chrome without explanatory copy; render the primitive directly.

## Composition

`FormRow` composes a label block, a control slot, and an aux slot.

- **Label block** - required `label`, optional `help`; left column, text only.
- **Control slot** - required; accepts form primitives such as `InlineInput`, `Switch`, `Segmented`, `RemovableChip`, `Button`, or a small composition of those primitives.
- **Aux slot** - optional; right-aligned secondary content such as auto-save state, policy copy, count, timestamp, or contextual action.

The embedded control owns its own interaction state, validation display, and ARIA semantics. `FormRow` owns row alignment, spacing, and association between label/help and the control where IDs are supplied.

## Variants

- **default** - regular Settings row, three-column layout: label/help | control | aux.
- **dense** - tighter vertical spacing for repeated simple settings in diagnostics or activity-heavy sections.
- **stacked** - narrow-container layout; label/help sits above control, aux follows as secondary metadata.
- **readonly** - same row structure, but the control slot presents non-editable value content or an inspect-only primitive.

Disabled, loading, and error states are rendered by the control primitive in the control slot, not by replacing the row contract.

## Tokens consumed

- `--font-sans` - label, help, aux copy.
- `--font-mono` - optional machine values when supplied by the control or aux content.
- `--color-text-primary` - setting label.
- `--color-text-secondary` - help text.
- `--color-text-tertiary` - aux metadata and muted row details.
- `--color-border-subtle` - optional row separator in dense lists.
- `--space-xs`, `--space-sm`, `--space-md`, `--space-lg` - label/help gap, column gap, vertical padding.
- `--radius-sm` - inherited by child controls when primitives render framed affordances.

## API sketch

```tsx
type FormRowProps = {
  label: React.ReactNode;
  help?: React.ReactNode;
  aux?: React.ReactNode;
  children: React.ReactNode;
  variant?: 'default' | 'dense' | 'stacked' | 'readonly';
  controlId?: string;
};

<FormRow
  label="Display name"
  help="Shown in briefings, meeting notes, and shared exports."
  aux="Auto-saved"
  controlId="display-name"
>
  <InlineInput id="display-name" value="James Giroux" mono={false} />
</FormRow>
```

## Source

- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx` lines 5-16 (`Row` label/help, child control, aux slots).
- **Form primitive references:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/parts.jsx` lines 18-55 (`InlineInput`, `Switch`, `Segmented`, `Btn`) and lines 73-80 (`Chip`).
- **Consumer reference:** `.docs/design/surfaces/Settings.md` lines 58-65 names `FormRow` as the universal Settings pattern.
- **Code:** to be implemented in `src/components/settings/FormRow.tsx`.

## Surfaces that consume it

- `Settings` - primary Wave 3 consumer for Identity, Connectors, Briefing & AI, Data, Activity, System, and Diagnostics rows.
- Future settings-like configuration surfaces that need the same label/help | control | aux contract.

## Naming notes

Canonical name is `FormRow`. The mockup code name is `Row`; promote as `FormRow` to avoid colliding with generic layout rows and to make the form-setting contract inspectable.

Do not rename to `SettingRow` unless the pattern becomes Settings-only. The intended contract is broader than the Settings surface.

## History

- 2026-05-03 — Proposed pattern for Wave 3.
