# RoleTransitionRow

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `RoleTransitionRow`
**`data-ds-spec`:** `patterns/RoleTransitionRow.md`
**Variants:** `default`
**Design system version introduced:** `0.4.0`

## Job

Show a person's relationship role change in one scan-friendly row so the user can understand who gained influence, who weakened, and which stakeholders now matter after the meeting.

## When to use it

- In MeetingDetail's Role Changes chapter.
- When a person has a before-status and after-status that should be compared directly.
- When the role change is derived from meeting intelligence, not manually assigned profile metadata.

## When NOT to use it

- For static people lists or attendee chips; use person/entity primitives.
- For Champion Health narrative blocks; use the champion health pattern.
- For action owner assignment or commitment ownership.

## States / variants

- `default` — person name followed by before-status pill, arrow, and after-status pill.
- Strengthening — after-status may use positive or emphasized treatment when influence increases.
- Weakening — after-status may use caution treatment when champion strength decreases.
- Unknown before-status — use "No relationship" rather than hiding the left side of the chain.

## Composition

Composes person name text, two `Pill` primitive instances, and a transition arrow. Rows stack vertically with stable alignment so multiple transitions can be compared quickly.

## Tokens consumed

- `--font-sans` — person name and pill text.
- `--color-text-primary` — person name.
- `--color-text-secondary` — transition arrow.
- `--color-surface-raised` — status pill background when neutral.
- `--border-subtle` — status pill edge.
- `--space-sm`, `--space-md` — row gap and pill chain spacing.

## API sketch

```tsx
<RoleTransitionRow
  personName="Marco Devine"
  beforeStatus="Champion"
  afterStatus="Champion (weakening)"
  tone="weakening"
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/RoleTransitionRow.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html` lines 328-343

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) canonical

## Naming notes

Canonical name is `RoleTransitionRow`. Use "role transition" rather than "role change" in code because the row renders the before-to-after chain, not only the final status.

## History

- 2026-05-03 — Proposed for Wave 4.
