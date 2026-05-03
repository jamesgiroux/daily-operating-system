# DayChart

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `DayChart`
**`data-ds-spec`:** `patterns/DayChart.md`
**Variants:** default; legend optional; `chartHeight` configurable (70-160px)
**Design system version introduced:** 0.1.0

## Job

Visual at-a-glance shape of the day — hour-tick row + colored horizontal bars (one per meeting) + a NOW indicator line + an optional legend. Not a calendar; not a list; a *picture* of the day's shape.

## When to use it

- DailyBriefing's "Today" section (above the meeting list) — anchor that lets the user see "shape of the day" before reading individual meetings
- Potential extension: any surface that needs to render a single-day timeline (e.g., a meeting's day-in-context view)

## When NOT to use it

- Multi-day views (use a calendar pattern)
- Lists of meetings without time-shape meaning (use a list)

## Composition

```
[Optional legend — mono uppercase, color swatches with labels]
[Hour ticks — 11 columns from 7AM to 5PM (or workday range)]
[Day bars container — relative positioned, 110px tall by default]
   Each bar:
     - Absolutely positioned by left% (start time) + width% (duration)
     - Colored by meeting type:
       - customer (turmeric) / customer.warn (terracotta)
       - internal (linen + heavy border)
       - oo (one-on-one — larkspur)
       - cancel (diagonal stripes, dashed border)
     - Past meetings: opacity 0.45
     - Now meeting: 2px terracotta outline + soft glow shadow
     - Bar content: serif title (small) + mono time
   NOW line:
     - Vertical 2px terracotta line spanning bar height + 10px above/below
     - "NOW · 10:18" label above
     - Dot at bottom
```

Hover interaction: subtle Y translate + box-shadow on bars.

## Variants

- **Default** — full chart with legend
- **No legend** — chart only (compact)
- **Configurable height** — `chartHeight` 70-160px (used by D-spine TweaksPanel)

## Tokens consumed

- `--font-mono` (ticks, legend, bar times, NOW label)
- `--font-serif` (bar titles)
- `--color-spice-turmeric` (customer bars, swatch)
- `--color-spice-terracotta` (warn / NOW line / now-bar outline)
- `--color-paper-linen` + `--color-rule-heavy` (internal bars)
- `--color-garden-larkspur` (1:1 bars)
- `--color-desk-charcoal-4` (cancel stripes)
- `--color-rule-medium` (chart top/bottom border, hour-tick separators)
- `--space-md`, `--space-2xl` (vertical rhythm)
- `--shadow-sm` (bar hover)

## API sketch

```tsx
<DayChart
  hours={["7 AM", "8", "9", "10", "11", "12 PM", "1", "2", "3", "4", "5"]}
  meetings={[
    { id: "1", type: "internal", state: "past", startPct: 18, durationPct: 5, title: "Eng standup", time: "9:00 · 30M" },
    { id: "2", type: "customer", state: "now", startPct: 27, durationPct: 7, title: "Acme Renewal", time: "10:00 · 45M" },
    // ...
  ]}
  nowPosition={30}
  nowLabel="NOW · 10:18"
  showLegend
/>
```

## Source

- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` (`.day-chart`, `.day-bars`, `.bar`, `.now-line`)
- **Code:** to be implemented in `src/components/dashboard/DayChart.tsx`

## Surfaces that consume it

DailyBriefing (canonical). Potential reuse on MeetingDetail (day-in-context view), maybe in account "today's calls with this account" view.

## Naming notes

`DayChart` — generic enough to live outside DailyBriefing if it gets reused. Don't rename to "Schedule", "DayBar", "Timeline" — those have their own meanings.

## History

- 2026-05-02 — Proposed pattern for v1.4.3 from D-spine mockup.
