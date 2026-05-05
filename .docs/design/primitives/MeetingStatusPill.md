# MeetingStatusPill

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `MeetingStatusPill`
**`data-ds-spec`:** `primitives/MeetingStatusPill.md`
**Variants:** `upcoming`, `in-progress`, `past`, `cancelled`
**Design system version introduced:** `0.4.0`

## Job

Show the temporal state of a meeting in one compact, readable pill so the user can scan schedule rows and in-progress meetings.

## When to use it

- In `MeetingCard` title rows when a meeting is happening now.
- When schedule or timeline UI needs a compact temporal label.
- When the status belongs to the meeting row as a whole, not to an individual action, finding, or transcript claim.

## When NOT to use it

- For generic labels, tags, or commitment ownership chips; use `Pill`.
- For FolioBar processing chrome; use the FolioBar status dot and text.
- For claim-level freshness or provenance; use the relevant trust primitive.

## States / variants

- `upcoming` — sage tone.
- `in-progress` — turmeric tone, often rendered as "NOW".
- `past` — neutral tone.
- `cancelled` — terracotta tone.
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
  state="in-progress"
  size="compact"
>
  NOW
</MeetingStatusPill>
```

## Source

- **Code:** shipped in `src/components/meeting/MeetingStatusPill.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html` lines 39-43

## Surfaces that consume it

- `MeetingCard` in DailyBriefing and WeekPage.
- MeetingDetail schedule/hero contexts when temporal state is shown.

## Naming notes

Canonical name is `MeetingStatusPill`. It is a meeting-specific primitive in the four-tier taxonomy and composes `Pill`; do not rename to `StatusBadge`, which is too broad for meeting recap state.

## History

- 2026-05-03 — Proposed for Wave 4.
