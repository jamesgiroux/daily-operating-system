# AgendaThreadList

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `AgendaThreadList`
**`data-ds-spec`:** `patterns/AgendaThreadList.md`
**Variants:** `confirmed`, `open`, `new`, `overdue`
**Design system version introduced:** `0.4.0`

## Job

Show how the planned meeting agenda actually resolved, item by item, so the user can see what was confirmed, what stayed open, what appeared unexpectedly, and how much room time each thread consumed.

## When to use it

- In MeetingDetail's "What Happened to Your Plan" chapter.
- When agenda items from a pre-meeting briefing need post-meeting outcome tracking.
- When time-spent metadata is important to explain where the conversation went.

## When NOT to use it

- For action items or commitments; use commitment and suggested action rows.
- For risk/win/decision synthesis; use the findings patterns.
- For plain transcript chapters with no planned agenda comparison.

## States / variants

- `confirmed` — check icon for items that happened as expected.
- `open` — open-circle icon for partial or unresolved items.
- `new` — plus icon for new attendees or emergent topics not in the original plan.
- `overdue` — carried-over item with overdue affordance in the detail line.
- Empty — show nothing until at least one agenda thread exists; MeetingDetail owns any section-level empty copy.

## Composition

Composes icon markers, text labels, and metadata text in a vertical list. Icons use the same scale and alignment across variants; detail lines carry item number, time spent, or carried-over status.

## Tokens consumed

- `--font-sans` — item text and metadata.
- `--font-mono` — optional label treatment for metadata.
- `--color-text-primary` — agenda item text.
- `--color-text-secondary` — detail text.
- `--color-garden-rosemary` — confirmed icon.
- `--color-text-tertiary` — open icon.
- `--color-spice-chili` — overdue detail.
- `--space-sm`, `--space-md` — row gap and list rhythm.

## API sketch

```tsx
<AgendaThreadList
  items={[
    { id: "pricing", status: "confirmed", text: "Pricing pressure raised first", detail: "Item 1 of plan", timeSpentLabel: "18 min spent" },
    { id: "audit-log", status: "open", text: "Audit-log export to Marco", carriedOverLabel: "9 days overdue" },
  ]}
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/AgendaThreadList.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html` lines 67-113

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) canonical

## Naming notes

Canonical name is `AgendaThreadList`. "Thread" is intentional: this pattern tracks the agenda as it developed in the room, not just the static briefing plan.

## History

- 2026-05-03 — Proposed for Wave 4.
