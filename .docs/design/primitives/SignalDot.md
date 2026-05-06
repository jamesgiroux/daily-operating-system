# SignalDot

**Tier:** primitive
**Status:** proposed
**Owner:** DOS-422 (W1)
**Last updated:** 2026-05-06
**`data-ds-name`:** `SignalDot`
**`data-ds-spec`:** `primitives/SignalDot.md`
**Module CSS (canonical):** `_shared/styles/SignalDot.module.css` (mirror); `src/components/dashboard/SignalDot.module.css` (source — ships W1)
**Token mapping:** `tokens/color.md` → "Signal kind"

## Job

Render a single signal-feed item: a tinted dot indicating signal kind, a `when` label, and a `what` description (with optional inline emphasis spans). Used in the Daily Briefing Moving section's signal feed (3-5 dots per Moving entity row).

## Kinds

Eight signal sources, each with a dedicated token alias:

| Kind | Token | Example signal |
|---|---|---|
| `meeting` | `--color-signal-meeting` | "Pricing alignment — in progress" |
| `action` | `--color-signal-action` | "Send pricing memo to Jen — overdue" |
| `email` | `--color-signal-email` | "Legal flagged 3 MSA clauses" |
| `lifecycle` | `--color-signal-lifecycle` | "Moving to renewing" |
| `gong-call` | `--color-signal-gong-call` | "Call recorded with Acme champion" |
| `zendesk-ticket` | `--color-signal-zendesk-ticket` | "Support ticket #4521 escalated" |
| `slack-thread` | `--color-signal-slack-thread` | "Asked about the Q2 narrative draft" |
| `linear-issue` | `--color-signal-linear-issue` | "Issue NW-42 marked done" |

## Variants

- **Default** — single dot + when + what, no decoration on the dot.
- **Overdue** (`SignalDot_overdue`) — text color shifts to `--color-spice-terracotta`. Triggered by `urgency: "overdue"` on the contract.
- **Corrected** (`SignalDot_corrected`) — solid sage outline ring around the dot (DOS-411 wire-in via DOS-428).
- **Contested** (`SignalDot_contested`) — dashed terracotta outline ring around the dot.

## Anatomy

```
[•] [WHEN] [what — with optional <em>emphasis</em> spans]
```

Three columns in CSS grid: `12px 70px 1fr`.

## Contract type

The contract field that drives a SignalDot is `MovingSignalViewModel`:

```ts
interface MovingSignalViewModel extends TrustMixin, LifecycleMixin {
  kind: SignalDotKind;
  when: string;
  whatSegments: { text: string; emphasized?: boolean }[];
  urgency: "normal" | "overdue";
  threadAction?: { label: string; href: string };
}
```

The view's only job: pick the correct `SignalDot_<kind>` class, render `when` + segments (each `emphasized: true` segment wrapped in `<em>`), apply `SignalDot_overdue` if urgency is overdue, apply correction outline if `correctionState` is set.

## What it doesn't do

- Pluralization or count rendering — those are the parent row's job.
- Deciding whether to render at all — service-side filter in `MovingService`.
- Composing the `what` string — service produces `whatSegments` already typed.

## Open questions (for W1 ship)

- Does the dot need a focus ring for keyboard accessibility, or is the parent row's link the focus target?
- Hover treatment: tooltip with full claim source, or no hover state?

## Spec status

**proposed** — TSX + final module CSS ship in W1 (DOS-422). Reference HTML at `.docs/design/reference/surfaces/briefing-redesign.html` consumes this primitive via the canonical module CSS today.
