# CommitmentRow

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `CommitmentRow`
**`data-ds-spec`:** `patterns/CommitmentRow.md`
**Variants:** `owner="yours" | "theirs"`; `state="captured" | "pending" | "complete"`
**Design system version introduced:** 0.4.0

## Job

Render a confirmed commitment captured from the meeting, making ownership explicit before the user reviews suggested follow-up actions.

## When to use it

- In MeetingDetail's "Commitments & Actions" chapter under "Commitments captured".
- When the commitment has already been made in the room or transcript.
- When the main distinction is ownership: YOURS or THEIRS.

## When NOT to use it

- For AI-proposed follow-up items that need accept / dismiss controls — use [SuggestedActionRow](SuggestedActionRow.md).
- For stale work items that need task controls — use the surface's pending action row.
- For findings, decisions, or evidence-backed observations — use `FindingsTriad`.

## States / variants

- **captured** — default state for newly extracted confirmed commitments.
- **pending** — commitment remains open after capture; may be shown near task controls by the consuming surface.
- **complete** — muted text and completed affordance when the commitment is settled.
- **yours** — neutral ownership pill labeled "YOURS".
- **theirs** — saffron ownership pill labeled "THEIRS".

## Composition

- `Pill` primitive for the YOURS / THEIRS ownership tag.
- `Text` primitive for the commitment body.
- Optional `Divider` primitive when rendered in a list.
- The row does not compose `Button`; confirmed commitments are not accept / dismiss suggestions.

## Tokens consumed

- `--color-spice-saffron-15` — THEIRS pill background.
- `--color-spice-turmeric` — THEIRS pill text.
- `--color-text-primary` — commitment text.
- `--color-text-secondary` — completed or muted commitment text.
- `--color-rule-light` — list divider.
- `--font-mono` — ownership pill label.
- `--font-sans` — commitment body.
- `--space-sm`, `--space-md` — pill and row spacing.

## API sketch

```tsx
<CommitmentRow
  owner="yours"
  state="captured"
  text="Send three pricing scenarios for the Apr 24 follow-up — covering 0%, 6%, and 12% reduction with renewal-term tradeoffs."
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/CommitmentRow.tsx`
- **Mockup origin:** `.docs/mockups/claude-design-project/mockups/meeting/current/after.html:261-277`
- **Contrast reference:** `SuggestedActionRow` mockup begins at `.docs/mockups/claude-design-project/mockups/meeting/current/after.html:279-325`
- **Reference render:** Meeting "Commitments & Actions" chapter.

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) — canonical consumer.

## Naming notes

`CommitmentRow` means confirmed commitment. Keep `SuggestedActionRow` for suggested work so ownership and confirmation state do not collapse into one pattern.

## History

- 2026-05-03 — Proposed for Wave 4
