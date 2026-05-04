# EscalationQuote

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `EscalationQuote`
**`data-ds-spec`:** `patterns/EscalationQuote.md`
**Variants:** `default`
**Design system version introduced:** `0.4.0`

## Job

Lift the quote where the room meaningfully changed direction, with attribution and timestamp, so the recap makes the turning point visible without requiring transcript scanning.

## When to use it

- In MeetingDetail's Conversation chapter for the single most important escalation or turning quote.
- When a quote needs editorial emphasis and explicit attribution.
- When clicking or otherwise referencing the timestamp can connect back to transcript evidence.

## When NOT to use it

- For small evidence quotes inside Findings; use the finding evidence treatment.
- For generic testimonials or customer quotes outside meeting analysis.
- For multiple quotes in a list; choose the strongest turning quote or use a different evidence pattern.

## States / variants

- `default` — large italic serif quote with smaller attribution line beneath.
- Hover/focus — timestamp affordance may indicate deep-link availability without changing quote layout.
- Missing attribution — do not render; the quote requires speaker and timestamp to be trustworthy.

## Composition

Composes a highlighted quote container, serif quote text, and mono/sans attribution line. It may wrap the timestamp in a transcript deep-link target when available.

## Tokens consumed

- `--font-serif` — quote typography.
- `--font-sans` — attribution line.
- `--color-text-primary` — quote text.
- `--color-text-secondary` — attribution text.
- `--color-spice-saffron-15` — highlighted background.
- `--color-spice-turmeric` — accent or border.
- `--space-md`, `--space-lg` — inset and vertical spacing.

## API sketch

```tsx
<EscalationQuote
  quote="If we're going to defend this number to our CFO, we need to see what flexibility looks like before we sign anything."
  speaker="Aoife Murphy"
  role="Procurement Lead"
  timestampLabel="11:14 AM"
  transcriptHref="/meetings/123/transcript?t=11:14"
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/EscalationQuote.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html` lines 183-186

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) canonical

## Naming notes

Canonical name is `EscalationQuote`. "Escalation" names the meeting-analysis job: this is the quote where stakes or pressure changed, not a decorative pull quote.

## History

- 2026-05-03 — Proposed for Wave 4.
