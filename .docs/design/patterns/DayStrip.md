# DayStrip

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `DayStrip`
**`data-ds-spec`:** `patterns/DayStrip.md`
**Variants:** previous/current/next; compact mobile previews
**Design system version introduced:** 0.1.0

## Job

Provide day-to-day briefing navigation directly below `FolioBar`: previous day,
current day, next day. In the Daily Briefing redesign direction, this is the proposed replacement
for a separate Weekly Forecast surface.

## When to use it

- DailyBriefing Daily Briefing redesign reference candidate.
- DailyBriefing route only if the v1.4.0 redesign explicitly adopts day-scoped
  navigation.

## When NOT to use it

- General app navigation. `FloatingNavIsland` remains the canonical app nav.
- Section navigation inside a long page; use `FloatingNavIsland` chapters.

## Composition

- Fixed strip under `FolioBar`
- Previous-day link with short preview
- Center current-day label with turmeric mark. Use "Today" visibly when the
  date is already present in `FolioBar`; keep the exact date in the accessible
  label if needed.
- Next-day link with short preview
- All visible strip text uses `--font-mono` because DayStrip is chrome, not
  editorial body copy.

## Source

- **Mockup substrate:** `/Users/jamesgiroux/Downloads/dailyos-design-system 2/project/mockups/briefing/variations/Daily Briefing redesign.html`
- **Reference styles:** `.docs/design/reference/_shared/styles/DayStrip.module.css`

## Surfaces that consume it

- `DailyBriefingRedesign` proposed reference surface (`.docs/design/reference/surfaces/briefing-redesign.html`)
