# SuggestedActionRow

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `SuggestedActionRow`
**`data-ds-spec`:** `patterns/SuggestedActionRow.md`
**Variants:** `context="meeting" | "work"`; `state="suggested" | "pending" | "accepted" | "dismissed"`
**Design system version introduced:** 0.4.0

## Job

Render an AI-suggested action item with accept / dismiss controls and an attribution line showing where the suggestion came from. Used in MeetingDetail (post-meeting suggested follow-ups) and v1.4.2 Account Detail Work surface (suggestions to promote to commitments).

Both contexts share the same row shape; secondary metadata differs by context (meeting attributes context quote and timestamp; work attributes priority and owner).

## When to use it

- MeetingDetail's "Commitments & Actions" section (suggested action items pending user accept/dismiss)
- AccountDetail Work surface (AI suggestions for new commitments)
- Future surfaces that need "AI proposes; user disposes" interaction

## When NOT to use it

- For confirmed commitments — use `CommitmentRow` (different state semantics)
- For any non-suggestion row — use the appropriate canonical pattern

## Composition

```
[Suggested-pill — saffron tint, "Suggested" label]
[Action title — serif 15-17px, weight 400]
[Action meta — mono, e.g., "P1 · Owner: you · Due Apr 19"]
[Optional context line — italic serif, e.g., the source quote with timestamp]
[Controls — Accept (turmeric) / Dismiss (neutral)]
```

Two-column grid: pill + content (left) | controls (right).

## States

- **suggested** — saffron pill, both Accept + Dismiss visible
- **pending** — neutral pill, single "Mark complete" control (used for committed items now waiting on action)
- **accepted** — collapses out of suggested list (tracked elsewhere as a commitment)
- **dismissed** — collapses out (Audit 04 noted: dismissal IS feedback signal, not a void operation)

## Variants by context

**`context="meeting"`** (Wave 4 / MeetingDetail):
- meta: priority (P1/P2) · owner · due date
- context: source quote with attribution and timestamp ("Apr 24 at 1pm works for us." — Aoife, 11:54)

**`context="work"`** (referenced from Audit 02 / Account Detail Work surface):
- meta: priority + owner only
- context: source attribution if available, otherwise omitted
- Accept promotes to committed item in a separate work tracking system

## Composition contract

Uses:
- `Pill` (suggested-pill: `tone="turmeric"` with explicit saffron variant; pending-pill: `tone="neutral"`)
- `Button` (Accept: kind="primary" turmeric; Dismiss: kind="ghost" neutral)
- `EntityChip` for entity references in title/context (when present)

## Tokens consumed

- `--color-spice-saffron-15`, `--color-spice-turmeric` (suggested pill, accept button)
- `--color-rule-light` (row border-bottom)
- `--font-serif` (title), `--font-sans` (meta, controls), `--font-mono` (priority/owner labels), `--font-serif italic` (context quote)
- `--space-md`, `--space-lg` (vertical rhythm, internal gaps)

## API sketch

```tsx
<SuggestedActionRow
  context="meeting"
  state="suggested"
  title="Schedule the Apr 24 renewal-pricing meeting"
  meta={{ priority: "P1", owner: "you", due: "Apr 19" }}
  contextQuote={{ text: "Apr 24 at 1pm works for us.", attribution: "Aoife", timestamp: "11:54" }}
  onAccept={() => /* commitments service */}
  onDismiss={() => /* dismissal feedback */}
/>
```

## Source

- **Spec:** new for Wave 4 (also referenced from Audit 02)
- **Code:** to be implemented in `src/components/work/SuggestedActionRow.tsx` (extracted from current `WorkSurface.tsx` per Audit 02 promotion-debt note)
- **Existing similar:** `src/components/shared/SuggestedActionRow.tsx` exists as a stub per Audit 01; reconcile during implementation

## Surfaces that consume it

MeetingDetail (canonical Wave 4 use), AccountDetail Work surface (Audit 02 reference), ProjectDetail Work surface (when promoted), PersonDetail Work surface (when promoted).

## Naming notes

`SuggestedActionRow` — clear "suggested action" framing distinguishes from `CommitmentRow` (confirmed) and generic action rows.

## History

- 2026-05-03 — Proposed pattern for Wave 4. Reconciles Audit 02 mention from v1.4.2 Account Detail Work surface; same pattern with context-specific meta.
