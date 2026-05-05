# SuggestedActionRow

**Tier:** pattern
**Status:** canonical/shipped
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `SuggestedActionRow`
**`data-ds-spec`:** `patterns/SuggestedActionRow.md`
**Variants:** `full`, `compact`, `showBorder=true | false`
**Design system version introduced:** 0.4.0

## Job

Render an AI-suggested action item with accept / dismiss controls and optional attribution showing where the suggestion came from. The shipped consumer is the ActionsPage backlog view.

The component also exposes a compact mode for denser suggestion lists, but the current routed app uses the full backlog row.

## When to use it

- ActionsPage backlog suggestions, where AI-proposed work waits for accept/dismiss
- Future surfaces that need "AI proposes; user disposes" interaction

## When NOT to use it

- For confirmed commitments — use `CommitmentRow` (different state semantics)
- For any non-suggestion row — use the appropriate canonical pattern

## Composition

```
[Dashed turmeric left marker]
[Full mode only: "Suggested" label + optional priority label]
[Action title - serif 15-17px, weight 400]
[Optional context line - sans 12-14px]
[Full mode: source/account metadata; compact mode: sourceLabel only]
[Controls - square Accept / Dismiss icon buttons]
```

Flex row: content grows left, controls stay fixed on the right.

## States

- **waiting** — full or compact row with both Accept + Dismiss visible.
- **accepted** — removed from the suggestion list and tracked as an action.
- **dismissed** — removed from the suggestion list; the dismissal remains a feedback signal.

## Variants by context

**`full`** (ActionsPage backlog):
- shows the "Suggested" mono label
- shows priority when present (`Urgent`, `High`, `Medium`, `Low`)
- shows `context`, then source/account metadata

**`compact`**:
- no "Suggested" label or priority label
- tighter 15px title and 12px context
- only shows `sourceLabel` metadata

## Composition contract

Uses inline styles in `src/components/shared/SuggestedActionRow.tsx` today. Extracting these styles into CSS is a cleanup target, but the component itself is shipped and should stay represented in the reference.

## Tokens consumed

- `--color-spice-turmeric` (dashed marker and label)
- `--color-garden-sage` (accept control)
- `--color-spice-terracotta` (dismiss control and urgent priority)
- `--color-rule-light` (row border-bottom)
- `--font-serif` (title), `--font-sans` (context and metadata), `--font-mono` (label/priority/source)

## API sketch

```tsx
<SuggestedActionRow
  action={{
    id: "act_123",
    title: "Schedule the Apr 24 renewal-pricing meeting",
    priority: 1,
    context: "Apr 24 at 1pm works for us.",
    sourceLabel: "Meeting capture",
    accountName: "Acme Corp",
  }}
  onAccept={() => acceptSuggestion("act_123")}
  onReject={() => rejectSuggestion("act_123")}
  showBorder
/>
```

## Source

- **Code:** `src/components/shared/SuggestedActionRow.tsx`
- **Current consumer:** `src/pages/ActionsPage.tsx` backlog tab

## Surfaces that consume it

ActionsPage backlog tab. MeetingDetail suggested follow-ups use `ActionRow` with `variant="outcome"` today, not `SuggestedActionRow`. Account/Project/Person Work surfaces use `WorkSurface` and `ActionRow` compact rows today.

## Naming notes

`SuggestedActionRow` — clear "suggested action" framing distinguishes from `CommitmentRow` (confirmed) and generic action rows.

## History

- 2026-05-03 — Proposed pattern for Wave 4.
- 2026-05-05 — Corrected consumer list and API to match shipped `SuggestedActionRow.tsx`.
