# SignalGrid

**Tier:** pattern
**Status:** shipped-local/extraction-needed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `SignalGrid`
**`data-ds-spec`:** `patterns/SignalGrid.md`
**Variants:** `default`; `tone="neutral" | "positive" | "caution"`
**Design system version introduced:** 0.4.0

## Job

Summarize the conversation's behavioral signals in a compact 2x2 grid so the user can scan meeting quality and risk without reading the transcript.

## When to use it

- In MeetingDetail's "Conversation" chapter after `TalkBalanceBar`.
- For exactly four concise conversation signals.
- When each signal has a short label and a short value, such as "Question density" and "High (1.4/min)".

## When NOT to use it

- For speaker time distribution — use `TalkBalanceBar`.
- For long qualitative quotes — use `EscalationQuote` or an evidence quote inside `FindingsTriad`.
- For lists longer than four signals — use a table or sectioned list pattern.

## States / variants

- **default** — four cells in a 2x2 grid with label + value on one line when space allows.
- **positive** — apply subtle positive value emphasis for healthy signals.
- **caution** — apply caution emphasis for risk-bearing values such as monologue risk.
- **missing value** — render the label with "Not enough signal" in secondary text.
- **mobile** — stack into one column while preserving the same reading order.

## Composition

- `Text` primitive for the signal key, styled as compact mono text.
- `Text` primitive for the signal value, styled as primary or tone-aware value text.
- `Divider` or cell border primitive for the quiet grid boundaries.
- Parent chapter supplies surrounding title and any following quote; `SignalGrid` only owns the four metric cells.

## Tokens consumed

- `--color-text-secondary` — signal keys.
- `--color-text-primary` — signal values.
- `--color-spice-terracotta` — caution value tone.
- `--color-garden-rosemary` — positive value tone.
- `--color-rule-light` — cell borders.
- `--font-mono` — signal keys.
- `--font-sans` — signal values.
- `--space-sm`, `--space-md` — cell padding and gaps.

## API sketch

```tsx
<SignalGrid
  signals={[
    { key: "Question density", value: "High (1.4/min)", tone: "positive" },
    { key: "Decision maker active", value: "Marco — yes", tone: "positive" },
    { key: "Forward-looking", value: "62% of statements" },
    { key: "Monologue risk", value: "Yes — minute 18–24", tone: "caution" }
  ]}
/>
```

## Source

- **Roadmap target:** `src/components/meeting/SignalGrid.tsx` (not shipped yet)
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html:176-181`
- **Reference render:** Meeting "Conversation" chapter.

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) — canonical consumer.

## Naming notes

`SignalGrid` is intentionally broader than `ConversationSignalGrid`; keep the surface-specific context in the consuming chapter.

## History

- 2026-05-03 — Proposed for Wave 4
