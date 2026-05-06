# SignalDot

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `SignalDot`
**`data-ds-spec`:** `primitives/SignalDot.md`
**Variants:** `kind="meeting" | "action" | "email" | "lifecycle" | "gongCall" | "zendeskTicket" | "slackThread" | "linearIssue"`
**Design system version introduced:** 0.6.0

## Job

Render a single signal-feed bullet: a tinted dot marking the signal source, a `when` label, and a `what` description with optional inline emphasis. Reads as one editorial line — what changed, when, where it came from. The dot color carries the source kind so a stack of bullets is scannable as a multi-channel feed at a glance.

## When to use it

- Inline signal-feed bullet inside a row pattern (today: `MovingRow`)
- When the signal source kind needs to register at-a-glance via color
- When the row already has a primary subject (entity name) and the signal is supporting context, not the headline

## When NOT to use it

- For a status indicator on an entity itself — that's `StatusDot`
- For trust band rendering — that's `TrustBandBadge`
- For an actionable bullet (with primary affordance) — that's `ActionRow` or a pattern with explicit affordance
- For a tone reinforcement on a `Pill` — that's `<Pill dot>` (decorative, not structural)

## States / variants

Eight `kind` variants, each tied to a token alias (see `tokens/color.md` → "Signal kind"):

| Kind | Token | Example signal |
|---|---|---|
| `meeting` | `--color-signal-meeting` | "Pricing alignment — in progress" |
| `action` | `--color-signal-action` | "Send pricing memo — overdue" |
| `email` | `--color-signal-email` | "Legal flagged 3 MSA clauses" |
| `lifecycle` | `--color-signal-lifecycle` | "Moving to renewing" |
| `gongCall` | `--color-signal-gong-call` | "Call recorded with champion" |
| `zendeskTicket` | `--color-signal-zendesk-ticket` | "Support ticket escalated" |
| `slackThread` | `--color-signal-slack-thread` | "Asked about Q2 narrative draft" |
| `linearIssue` | `--color-signal-linear-issue` | "Issue NW-42 marked done" |

Lifecycle variants on top of `kind`:

- **`SignalDot_overdue`** — text shifts to `--color-spice-terracotta`. Driven by `urgency: "overdue"` on the contract.
- **`SignalDot_corrected`** — solid sage outline ring on the dot. Driven by `correctionState: "corrected"` (LifecycleMixin, DOS-411 wire-in via DOS-428).
- **`SignalDot_contested`** — dashed terracotta outline ring on the dot. Driven by `correctionState: "contested"`.

## Composition

Primitive — no sub-primitives. Renders:

```html
<span class="SignalDot" data-kind="meeting" data-ds-name="SignalDot" data-ds-spec="primitives/SignalDot.md">
  <span class="SignalDot_dot"></span>
  <span class="SignalDot_when">10:00</span>
  <span class="SignalDot_what">Pricing alignment — <em>in progress</em></span>
  <button class="SignalDot_threadAction" data-ds-name="SignalDot.threadAction">→ thread</button>
</span>
```

Grid: `12px 70px 1fr auto`. The fourth column collapses when `threadAction` is absent. The thread `<button>` stops event propagation so the parent row's link does not fire.

## Tokens consumed

- `--color-signal-meeting`, `--color-signal-action`, `--color-signal-email`, `--color-signal-lifecycle`, `--color-signal-gong-call`, `--color-signal-zendesk-ticket`, `--color-signal-slack-thread`, `--color-signal-linear-issue` — dot fill, one per kind
- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary` — what / when text
- `--color-spice-terracotta` — overdue text
- `--color-garden-sage` — corrected outline
- `--font-mono` — `when` label
- `--font-sans` — `what` text
- `--space-xs`, `--space-sm` — gap between dot/when/what

## API sketch

```tsx
<SignalDot
  kind="meeting"
  when="10:00"
  whatSegments={[
    { text: "Pricing alignment — " },
    { text: "in progress", emphasized: true },
  ]}
  urgency="normal"
/>

<SignalDot
  kind="action"
  when="2d"
  whatSegments={[{ text: "Send pricing memo — overdue" }]}
  urgency="overdue"
  threadAction={{ label: "→ thread", href: "/actions/abc" }}
/>
```

The component picks `SignalDot_<kind>` from `kind`, applies `SignalDot_overdue` from `urgency`, and applies `SignalDot_corrected` / `SignalDot_contested` from optional `correctionState` (LifecycleMixin).

Contract type (from `BriefingViewModel`):

```ts
interface MovingSignalViewModel extends TrustMixin, LifecycleMixin {
  kind: SignalDotKind;
  when: string;
  whatSegments: { text: string; emphasized?: boolean }[];
  urgency: "normal" | "overdue";
  threadAction?: { label: string; href: string };
}
```

## Source

- **Code:** ships W1 (DOS-422) at `src/components/dashboard/SignalDot.tsx` + `src/components/dashboard/SignalDot.module.css`
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign.html` (consumes the canonical module CSS via `_shared/styles/SignalDot.module.css` mirror)
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/` (Daily Briefing redesign exploration)

## Surfaces that consume it

- DailyBriefing (via `MovingRow`)

## Naming notes

`SignalDot` is the canonical name. `<Pill dot>` is unrelated — Pill's dot is decorative tone reinforcement; SignalDot is a structural primitive whose entire job is to mark a signal kind. Do not collapse them. See `NAMING.md`.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W1 under DOS-422.
