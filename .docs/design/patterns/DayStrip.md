# DayStrip

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `DayStrip`
**`data-ds-spec`:** `patterns/DayStrip.md`
**Variants:** today (mark + "Today"); non-today (date center, relabeled sides); window-edge (disabled side); compact mobile previews
**Design system version introduced:** 0.1.0

## Job

Provide day-to-day briefing navigation directly below `FolioBar`: previous day,
current day, next day. In the D-spine direction, this is the proposed replacement
for a separate Weekly Forecast surface.

The briefing surface renders a **±7-day window** (up to 7 days past, 7 ahead),
not just "today" (James, 2026-06-10). The strip is the chrome that answers
"what day am I looking at?" while walking that window:

- **Center label**: "Today" with the pulsing turmeric mark when the rendered
  day IS today; otherwise the full date ("Thursday, April 23") with NO mark —
  the mark means "live day," never decoration.
- **Side links** walk one day at a time and carry the label ONLY — no
  preview text (James, 2026-06-10): "Yesterday"/"Tomorrow" when that side IS
  yesterday/tomorrow, otherwise the short date ("Wed, Apr 22").
- **Window bounds**: at +7/−7 the outbound side renders disabled (no href,
  `--color-text-quaternary`), preserving the 1fr/auto/1fr grid so the center
  never shifts.
- **Substrate**: none required — the strip is pure chrome (dates + routing),
  no producer payload, no claims.

## When to use it

- DailyBriefing D-spine reference candidate.
- DailyBriefing route only if the v1.4.0 redesign explicitly adopts day-scoped
  navigation.

## When NOT to use it

- General app navigation. `FloatingNavIsland` remains the canonical app nav.
- Section navigation inside a long page; use `FloatingNavIsland` chapters.

## Composition

- Fixed strip under `FolioBar`
- Previous-day link (label only)
- Center current-day label with turmeric mark. Use "Today" visibly when the
  date is already present in `FolioBar`; keep the exact date in the accessible
  label if needed.
- Next-day link (label only)
- All visible strip text uses `--font-mono` because DayStrip is chrome, not
  editorial body copy.

## Source

- **Mockup substrate:** `/Users/jamesgiroux/Downloads/dailyos-design-system 2/project/mockups/briefing/variations/D-spine.html`
- **Reference styles:** `.docs/design/reference/_shared/styles/DayStrip.module.css`

## Surfaces that consume it

- `DailyBriefingDSpine` proposed reference surface (`.docs/design/reference/surfaces/briefing-d-spine.html`)
