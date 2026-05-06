# BriefingLoadingState

**Tier:** pattern
**Status:** proposed
**Owner:** DOS-429 (W5, ships TSX as part of DailyBriefingRedesign.tsx)
**Last updated:** 2026-05-06
**`data-ds-name`:** `BriefingLoadingState`
**`data-ds-spec`:** `patterns/BriefingLoadingState.md`
**Reference render:** `.docs/design/reference/surfaces/briefing-redesign-loading.html`

## Job

Render the briefing while data is being assembled. Triggered when `BriefingLoadState.status === "loading"`. Replaces the success-state body until the model arrives.

## Anatomy

Centered single-column stage. No margin grid, no sections — just an editorial holding state:

```
┌────────────────────────────────────┐
│                                    │
│      Reading your day…             │  ← serif italic 32px, tertiary
│                                    │
│              •                     │  ← 8px pulsing dot, turmeric
│                                    │
│      GATHERING TODAY'S SIGNALS     │  ← mono 11px caps, tertiary
│                                    │
└────────────────────────────────────┘
```

## Variants

Single variant. Content is fixed editorial copy in this version — no service-rendered fields on the contract for the loading state (`BriefingLoadState.loading` carries `{ status: "loading" }` only).

## Composition rules

- Centered max-width 640px column.
- Pulsing dot animation `briefing-loading-pulse` 1.4s ease-in-out infinite. Pure CSS, not skeleton-content motion.
- FolioBar is rendered (chrome scaffolding visible) but the body slot is the loading stage.

## What it doesn't do

- Render skeletons of section content. The redesign's editorial register prefers a calm holding state to a content-shaped skeleton; "Reading your day…" is more honest than fake meeting cards.
- Block interaction with chrome. FolioBar nav and FloatingNavIsland remain interactive.

## Open questions

- Should the loading copy vary by time of day? ("Reading your morning" vs "Reading your day"). Deferred until usage data informs.

## Spec status

**proposed** — TSX ships in W5 as part of DOS-429. Reference HTML at `briefing-redesign-loading.html` is the canonical render today.
