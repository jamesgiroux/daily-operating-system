# ChampionHealthBlock

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `ChampionHealthBlock`
**`data-ds-spec`:** `patterns/ChampionHealthBlock.md`
**Variants:** `arc="strengthening" | "steady" | "weakening" | "lost"`
**Design system version introduced:** 0.4.0

## Job

Render the current state of a champion relationship in one focused block so the user can understand who is still advocating, how that advocacy is changing, and why it matters.

## When to use it

- In MeetingDetail's "Champion Health" chapter.
- When one named person has a relationship status arc and supporting evidence.
- When the risk paragraph explains what the user must protect or repair next.

## When NOT to use it

- For multiple role changes — use `RoleTransitionRow`.
- For generic meeting risks — use `FindingsTriad`.
- For a list of commitments owned by the champion — use `CommitmentRow`.

## States / variants

- **steady** — status arc communicates stable support.
- **strengthening** — status arc communicates increased advocacy.
- **weakening** — status arc communicates support under pressure; risk text should be present.
- **lost** — status arc communicates the person is no longer a champion; use strongest risk tone.
- **no evidence** — render the status and risk paragraph, but show "No quote captured" in secondary text.

## Composition

- `EntityName` or `Text` primitive for the champion name.
- `Pill` or status text primitive for the relationship arc, including the leading status dot.
- Evidence quote primitive for the supporting transcript quote.
- Attribution primitive for speaker, timestamp, and call-position context.
- Risk paragraph primitive with emphasized `Risk:` label.

## Tokens consumed

- `--color-text-primary` — champion name and evidence quote.
- `--color-text-secondary` — attribution.
- `--color-spice-chili` — risk paragraph in weakening or lost states.
- `--color-garden-rosemary` — steady or strengthening status tone.
- `--font-sans` — name, status, and risk paragraph.
- `--font-serif` — evidence quote.
- `--font-mono` — attribution metadata.
- `--space-sm`, `--space-md`, `--space-lg` — block rhythm.

## API sketch

```tsx
<ChampionHealthBlock
  champion={{
    name: "Marco Devine",
    status: "Still champion",
    arc: "weakening"
  }}
  evidence={{
    quote: "I'll go to bat for the Helpline numbers internally — but the per-seat math has to come down or I won't have cover.",
    attribution: "Marco · 11:52 — last 4 minutes of the call"
  }}
  risk="Marco is now using procurement as cover. If pricing isn't resolved by mid-May, he stops being a champion and becomes a neutral."
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/ChampionHealthBlock.tsx`
- **Mockup origin:** `.docs/mockups/claude-design-project/mockups/meeting/current/after.html:243-259`
- **Reference render:** Meeting "Champion Health" chapter.

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) — canonical consumer.

## Naming notes

`ChampionHealthBlock` is singular by design. If a surface needs many people, compose multiple blocks or use a separate relationship-list pattern.

## History

- 2026-05-03 — Proposed for Wave 4
