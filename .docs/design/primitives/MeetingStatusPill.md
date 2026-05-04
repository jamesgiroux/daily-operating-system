# MeetingStatusPill

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `MeetingStatusPill`
**`data-ds-spec`:** `primitives/MeetingStatusPill.md`
**Variants:** `wrapped`, `processing`, `failed`
**Design system version introduced:** `0.4.0`

## Job

Show the extraction state of a meeting recap in one compact, readable pill so the user can trust whether the meeting intelligence is ready, still being produced, or needs attention.

## When to use it

- In the `SurfaceMasthead` accessory slot for MeetingDetail.
- When status copy must pair a state icon with meeting-specific timing or recording metadata.
- When the status belongs to the meeting recap as a whole, not to an individual action, finding, or transcript claim.

## When NOT to use it

- For generic labels, tags, or commitment ownership chips; use `Pill`.
- For FolioBar processing chrome; use the FolioBar status dot and text.
- For claim-level freshness or provenance; use the relevant trust primitive.

## States / variants

- `wrapped` — sage background, rosemary text, check icon, copy like "Wrapped 8 minutes ago · 56 min recorded".
- `processing` — saffron background, turmeric text, progress/spinner icon, copy like "Processing transcript".
- `failed` — terracotta background, chili text, warning icon, copy like "Extraction failed".
- Disabled is not exposed; the pill is informational.

## Composition

Composes the `Pill` primitive with meeting-specific status tokens, an icon slot, and a short status label. It should keep the same pill shape, density, and inspectable data attributes as `Pill`.

## Tokens consumed

- `--font-mono` — uppercase status typography.
- `--color-garden-sage-15` — wrapped background.
- `--color-garden-rosemary` — wrapped foreground.
- `--color-spice-saffron-15` — processing background.
- `--color-spice-turmeric` — processing foreground.
- `--color-spice-terracotta-15` — failed background.
- `--color-spice-chili` — failed foreground.
- `--space-xs`, `--space-sm` — icon and label spacing.

## API sketch

```tsx
<MeetingStatusPill
  state="wrapped"
  wrappedAtLabel="8 minutes ago"
  recordedDurationLabel="56 min"
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/MeetingStatusPill.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html` lines 39-43

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) canonical

## Naming notes

Canonical name is `MeetingStatusPill`. It is a meeting-specific primitive in the four-tier taxonomy and composes `Pill`; do not rename to `StatusBadge`, which is too broad for meeting recap state.

## History

- 2026-05-03 — Proposed for Wave 4.
