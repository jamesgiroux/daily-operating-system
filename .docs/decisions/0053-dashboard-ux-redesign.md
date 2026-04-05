# ADR-0053: Dashboard UX redesign — readiness-first overview

**Date:** 2026-02-08
**Status:** Accepted

## Context

A full UX review of the daily briefing/dashboard page identified several information architecture problems:

1. **StatsRow is low-value.** Four number cards (Meetings, Customer, Actions Due, Inbox) occupy prime above-the-fold real estate but add almost no signal. Three of the four duplicate information visible in the main content grid below. The fourth (Inbox) reads 0 permanently because inboxCount tracks unprocessed files, not a meaningful daily metric.

2. **Numbers without context.** "5 Meetings" tells the user nothing they won't see in the timeline. The user's real morning question is "how prepared am I?" — not "how many meetings do I have?"

3. **Right sidebar priority is inverted.** EmailList renders above ActionList. Actions are user-owned obligations; emails are externally imposed. The user's commitments should take visual priority over other people's requests.

4. **Greeting wastes space.** "Good morning" adds zero information — the user knows the time of day. The space would be better used to surface the Focus statement more prominently.

5. **ActionList default is too conservative.** `maxVisible = 3` forces navigation on typical days with 5-7 actions. The ActionItem component is compact enough to show 5 without layout problems.

### Design principles at play

- **P2 (Prepared, Not Empty):** Default state should be "ready" — the readiness strip directly answers "am I prepared?"
- **P7 (Consumption Over Production):** 80% reading, 20% writing — optimize for the morning scan
- **P9 (Show the Work, Hide the Plumbing):** Surface prep coverage and overdue counts, not raw meeting totals

## Decision

### 1. Replace StatsRow with a Readiness Strip

Replace the four generic number cards with four **contextual readiness signals**:

| Slot | Signal | Source | Color logic |
|------|--------|--------|-------------|
| Prep coverage | `3 of 5 prepped` | `meetings.filter(hasPrep) / meetings.filter(!cancelled)` | Primary when 100%, destructive when < 50% |
| Agendas needed | `1 agenda needed` | External meetings without prep (`!hasPrep && isExternalType`) | Warning when > 0, hidden or success when 0 |
| Overdue actions | `2 overdue` | `actions.filter(isOverdue)` — falls back to `X due today` if none overdue | Destructive when overdue, muted otherwise |
| Next meeting | `Next: 10:30 Acme QBR` | First future meeting by time, or "No more meetings" after last ends | Muted — informational |

All four are computed from `DashboardData` fields that already exist. No new backend work required.

### 2. Flip ActionList above EmailList

In the right sidebar column, render ActionList first, EmailList second. Actions are the user's own obligations and should appear above externally-imposed email.

### 3. Drop greeting, promote Focus

Remove the "Good morning/afternoon/evening" line. If `overview.focus` exists, render it more prominently beneath the date — as a callout with the Target icon, not as a subtle link.

### 4. Increase ActionList maxVisible to 5

Change from 3 to 5 default visible actions. Covers the common daily action load (3-7 items) without requiring navigation.

### 5. Full-width summary (optional, lower priority)

Move the "Today" AI summary card from a 2-column grid layout to full-width beneath date/focus and above the readiness strip. This creates a clear top-to-bottom flow: **Date → Focus → Summary → Readiness → Schedule**.

## Consequences

### Easier
- Morning scan directly answers "am I prepared?" without mental math
- Action visibility improved — most users see all their actions without navigating
- Information hierarchy matches cognitive priority: prep readiness > obligations > incoming

### Harder
- Readiness strip requires computing prep coverage and next-meeting from existing data (trivial, all data is in DashboardData)
- Losing the "Customer" count card — but this count is already visible as meeting type badges in the timeline
- The readiness strip is more information-dense per card, requiring careful typography

### Trade-offs
- StatsRow was visually simple (four big numbers). Readiness strip is denser but more useful.
- Dropping "Inbox" card means no at-a-glance inbox signal. This is acceptable — inbox processing is async and the count was always 0 anyway.
- Greeting removal trades warmth for density. The date + focus + summary still provide orientation.

## Implementation

Five backlog items (I97–I101), ordered by value and dependency:

- **I97:** Readiness strip component (replaces StatsRow) — highest value
- **I98:** Flip ActionList above EmailList — trivial
- **I99:** Drop greeting, promote focus — small
- **I100:** Increase ActionList maxVisible to 5 — trivial
- **I101:** Full-width summary layout — optional, lower priority
