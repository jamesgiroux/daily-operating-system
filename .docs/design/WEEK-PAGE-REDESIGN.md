# Weekly Forecast Redesign

**Status:** Design approved, ready for implementation
**Date:** 2026-02-20
**Context:** The weekly surface shifted from a static generated document (refresh-to-read) to an always-live rolling view. The page architecture needs to match.

---

## Design Thesis

The weekly forecast is a **reference surface**, not a morning paper. You land on it multiple times across the week to understand the terrain ahead, check meeting readiness, and plan preparation. Every element must survive repeated visits without feeling stale.

The timeline is the spine. Everything else frames it.

---

## Page Architecture

```
1. Week Header (compact, left-aligned)
2. This Week — The Shape (density map, full Mon–Fri)
3. The Timeline (±7 days, ChapterHeading)
4. Finis
```

### What was cut

| Element | Why |
|---------|-----|
| Centered 56px narrative hero | Stale by mid-week. Theatrical reveal for a single-read document, not a multi-visit surface. |
| "The Three" (numbered priorities) | Over-explains what the meetings already show. A QBR on Thursday is self-evidently the focus. The daily briefing handles daily focus — the weekly surface doesn't need to duplicate that job. |
| AI `weekNarrative` in page body | One-time narrative goes stale in a rolling context. Can still generate for backend use but not rendered as page centerpiece. |

---

## Section Specs

### 1. Week Header

Compact, left-aligned. Orients the user and shows readiness at a glance.

```
WEEK 8                              mono, 13px, 0.08em tracking, uppercase
Feb 17 – 21                         larkspur color

                                    serif (Newsreader), 28px, weight 400
                                    Left-aligned, not centered

4 ready · 2 building · 1 overdue   mono, 12px, tertiary
                                    Computed from timeline data (always live)
```

**Data source:** Readiness vitals derived from timeline intelligence quality (the existing `folioReadinessStats` computation). Does NOT depend on WeekOverview — works from timeline data alone.

**Design tokens:**
- Week number: `--font-mono`, 13px, `--color-garden-larkspur`, 0.08em letter-spacing, uppercase
- Date range: `--font-serif`, 28px, weight 400, `--color-text-primary`
- Vitals: `--font-mono`, 12px, `--color-text-tertiary`. Color-code individual stats: sage for ready, terracotta for building/overdue.

**Spacing:** `--space-5xl` (80px) top padding. `--space-lg` (24px) between date and vitals. `--space-3xl` (56px) below section.

---

### 2. This Week — The Shape

A glanceable density map of the full calendar week (Mon–Fri). Not a chapter — a visualization with a light label.

```
THIS WEEK                           mono label, 10px, uppercase, tertiary
                                    Light rule below (1px, --color-rule-heavy)

MON  ████████░░░░░░░░░░░░░░░░  3m · moderate          (muted, past)
TUE  ██████████████░░░░░░░░░░  4m · heavy             (muted, past)
WED  ████░░░░░░░░░░░░░░░░░░░░  1m · light    ← today  (larkspur accent)
THU  ████████████████████░░░░  6m · packed
FRI  ██████░░░░░░░░░░░░░░░░░░  2m · moderate

Front-loaded. Thursday is the crux — clear Friday for recovery.
```

**Layout:**
- "THIS WEEK" mono label above, same style as margin grid labels (10px, 500 weight, 0.08em, uppercase, tertiary)
- 1px rule below label
- Each day row: `[day label 36px] [density bar flex] [count + density 88px]`
- Achievability indicator when available: `[feasible/total 40px]` right-aligned, sage/terracotta
- Epigraph below bars: mono, 12px, tertiary — output of `computeShapeEpigraph()`

**Today marking:**
- Today's day label in `--color-garden-larkspur` instead of tertiary
- Today's density bar fill in larkspur instead of default
- Small "today" label or `←` indicator after the density count

**Past days:**
- Day label, bar, and count at `opacity: 0.4`
- Bar fill color stays the same (just muted by opacity)

**Data source:** Derived from timeline meetings (mechanical: count per day, estimate 45min each). Enriched by `data.dayShapes` when AI overview is available (adds `focusImplications`, `prioritizedActions`). The Shape renders on first load without AI dependency.

**Spacing:** `--space-lg` (24px) gap between day rows. `--space-md` (16px) between last bar and epigraph. `--space-3xl` (56px) below section.

---

### 3. The Timeline

The primary content section. Uses `ChapterHeading` — it earns the formality.

```
─── The Timeline ────────────────────────────────
Your meetings, ±7 days.

▸ Earlier                                    12 meetings
─────────────────────────────────────────────────
Yesterday
  [MeetingCard: past, muted]
  [MeetingCard: past, muted]

═══ Today ═══════════════════════════════════════  (larkspur)
Today
  [MeetingCard: active/upcoming]

─── Ahead ───────────────────────────────────────
Tomorrow — Thursday, Feb 20
  [MeetingCard: future, with badges]
  [MeetingCard: future, No prep pill]

Friday, Feb 21
  [MeetingCard: future]

Monday, Feb 24
  [MeetingCard: future, 5 days]
```

**Structure:** Identical to current implementation. No changes to timeline grouping, collapse behavior, or MeetingCard rendering. The unified MeetingCard (v0.13.1) already provides the full editorial treatment with:
- Type-based accent bars (turmeric/linen/larkspur)
- Time + duration display
- Entity bylines
- Intelligence quality badges
- "No prep" pills for sparse meetings
- Days-until countdown
- Past meeting outcome summaries + follow-up counts
- Navigation hints for past meetings

**Focus is implicit, not explicit.** The MeetingCard badges ARE the focus layer. No inline callouts or priority annotations injected into the timeline.

---

### 4. Finis

```
                        * * *
                  Your week at a glance.
```

`<FinisMarker />` with shorter closing text. "Your week at a glance" instead of "Your week is forecasted" — matches the reference-surface tone.

---

## What stays unchanged

- FolioBar with "Weekly Forecast" label, refresh button, readiness stats
- AtmosphereLayer with larkspur color
- FloatingNavIsland with week page active
- Timeline ±7 day range and grouping logic
- MeetingCard rendering (just unified in v0.13.1)
- `computeShapeEpigraph()` — mechanical, doesn't go stale
- Earlier/Recent Past/Today/Ahead timeline grouping
- Auto-enrichment of sparse meetings on load
- Live event listeners (calendar-updated, workflow-completed, intelligence-updated, prep-ready)
- Workflow running state (full-page takeover with GeneratingProgress)

## What changes

| Current | New |
|---------|-----|
| Centered 56px narrative hero | Left-aligned compact week header (28px serif date) |
| "The Three" chapter with ChapterHeading | Removed entirely |
| The Shape as Chapter 2.5 with ChapterHeading | "THIS WEEK" mono label + density bars (no ChapterHeading) |
| Shape derived from AI dayShapes only | Shape derived from timeline (mechanical), enriched by AI when available |
| The Timeline as Chapter 3 | The Timeline as Chapter 2 (promoted) |
| Readiness from WeekOverview checks | Readiness from timeline intelligence quality (always live) |
| weekNarrative as centerpiece | Not rendered |
| "Your week is forecasted." finis | "Your week at a glance." finis |

---

## Data model implications

The page must render fully from **timeline data alone** (always available after calendar sync). The WeekOverview (AI-generated) enriches but is not required:

| Element | Without AI (timeline only) | With AI (WeekOverview) |
|---------|---------------------------|------------------------|
| Week Header | Week number from date math, readiness from timeline | Same |
| The Shape | Mechanical: count meetings per day, 45min estimate | `dayShapes` with real minutes, focusImplications, prioritizedActions |
| Epigraph | Computed from mechanical shape | Computed from AI shape (richer) |
| The Timeline | Full rendering with MeetingCards | Same |

This means the page is useful immediately on first load. The "refresh forecast" button enriches but doesn't gate the experience.

---

## Scroll budget

| Section | Estimated height |
|---------|-----------------|
| Week Header | ~160px |
| The Shape (5 days + epigraph) | ~220px |
| Rule / gap | ~56px |
| Timeline heading | ~60px |
| Timeline content (varies) | ~800-2000px |
| Finis | ~120px |

Above the fold (~700px viewport): Week Header + most of The Shape visible. Timeline begins at first scroll. This is good — you see the topology immediately, then scroll into detail.
