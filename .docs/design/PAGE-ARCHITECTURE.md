# Page Architecture

**Last audited:** 2026-02-20

Every page in DailyOS has a stated job (ADR-0084). This document maps each page's intended structure against what's actually built.

---

## 1. Daily Briefing

**File:** `src/components/dashboard/DailyBriefing.tsx` (rendered by dashboard route)
**Job:** Morning situational awareness. Read top-to-bottom in 2-5 minutes.
**Atmosphere:** Turmeric + Larkspur
**CSS:** `src/styles/editorial-briefing.module.css` (BEST PRACTICE — use this as reference)

### Intended Structure (ADR-0084 + I342 redesign)

```
1. Day Frame (Hero + Focus merged)
   - 1-sentence AI narrative
   - Capacity: X hours free, N meetings
   - Focus: where to direct non-meeting energy

2. Schedule
   - Full temporal map
   - Up Next: first upcoming meeting expanded by default
   - Past meetings: outcomes summary

3. Attention (Review + Priorities merged)
   - Proposed action triage (high-signal, capped)
   - 2-3 actions: meeting-relevant or overdue
   - 3-4 urgent emails

4. Finis
```

### Current State: MOSTLY ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Day Frame | Done | Hero + Focus merged. Capacity line present. |
| Schedule | Done | BriefingMeetingCard with Up Next expansion. |
| Attention | Done | AttentionSection replaces separate Review + Priorities. |
| Finis | Done | FinisMarker present. |

**Remaining issues:**
- Deep work block count in capacity line references cut Open Time concept (minor)
- Some vocabulary strings not yet translated (I341 — see VIOLATIONS.md)

---

## 2. Weekly Forecast

**File:** `src/pages/WeekPage.tsx`
**Job:** Week-shape planning. Understand topology, identify priorities, find deep work time.
**Atmosphere:** Larkspur

### Intended Structure (ADR-0084)

```
1. Hero (week narrative)
2. The Three (force-ranked priorities)
3. The Shape (multi-day density)
4. Meeting Intelligence Timeline (±7 days) — I330
5. Finis
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Hero | Done | Week narrative, Newsreader 76px. |
| The Three | Done | Force-ranked priorities. |
| The Shape | Done | Day-by-day density rendering. |
| Timeline | Done (0.13.0) | ±7-day meeting timeline with IntelligenceQualityBadge. |
| Finis | Done | FinisMarker present. |

**Cuts executed:**
- "Your Meetings" chapter: removed
- "Commitments" chapter: removed
- Open Time: removed

---

## 3. Meeting Briefing

**File:** `src/pages/MeetingDetailPage.tsx`
**Job:** Brief you before a meeting OR close it out after.
**Atmosphere:** Turmeric (intense, focused)

### Intended Structure (ADR-0084 + I342)

```
[Post-meeting: Outcomes section at top]

1. The Brief
   - Key insight pull quote
   - Meeting metadata + entity chips
   - Before This Meeting (merged readiness + actions)

2. The Risks

3. The Room
   - Attendees with disposition, assessment, engagement

4. Your Plan
   - AI-proposed + user agenda items

5. Finis

[Deep dive: CUT]
[Appendix: CUT]
```

### Current State: MOSTLY ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Outcomes | Done | Summary, wins, risks, actions for past meetings. |
| The Brief | Done | Key insight, metadata, entity chips. |
| The Risks | Done | Risk rows. |
| The Room | Done | Attendee list with rich context. |
| Your Plan | Done | Agenda editor with draft/prefill. |
| Finis | Done | FinisMarker present. |
| Deep Dive | Cut | Correctly removed. |
| Appendix | Cut | Correctly removed (was 9 sub-sections). |

**Remaining issues:**
- Folio bar still has transcript button for past meetings (A7 — should be body CTA only)
- "Meeting Intelligence Report" kicker text (I341 vocabulary — should be "Meeting Briefing")
- "Prep not ready yet" text (I341 — should be "Not ready yet")
- Extensive inline `style={{}}` usage — worst offender in codebase. Needs CSS module migration.

---

## 4. Actions

**File:** `src/pages/ActionsPage.tsx`
**Job:** Commitment inventory. Triage, execute, clear.
**Atmosphere:** Terracotta

### Intended Structure (ADR-0084 + I342)

```
Tabs: Suggested | Pending | Completed

Suggested (default when suggestions exist):
  - Triage queue

Pending (primary view):
  - Meeting-centric groups (future: I342 structural tier)
  - Temporal fallback: Overdue / Today / Upcoming

Completed:
  - Record of progress
```

### Current State: PARTIAL

| Section | Status | Notes |
|---------|--------|-------|
| Tab structure | Done | Suggested/Pending/Completed (Waiting/All tabs cut). |
| Smart default | Done | Defaults to Suggested when suggestions exist. |
| Temporal grouping | Done | Overdue / This Week / Later on pending tab. |
| Meeting-centric view | NOT DONE | Deferred to 0.12.2+. Currently temporal only. |
| Auto-expiry | NOT DONE | Tooltip says "30+ days auto-archived" but no backend. |
| FinisMarker | MISSING | Uses custom "That's everything" footer instead. Should use FinisMarker. |

**Remaining issues:**
- "proposed" tab label should be "Suggested" (I341)
- "AI Suggested" label should be "Suggested" (I341)
- Deceptive auto-archive tooltip (claims behavior that doesn't exist)
- No FinisMarker

---

## 5. Account Detail

**File:** `src/pages/AccountDetailEditorial.tsx`
**Job:** Relationship dossier. Full picture to show up informed.
**Atmosphere:** Turmeric (warm, entity identity)
**Accent:** Turmeric

### Intended Structure (I342 surface redesign)

```
1. Hero (name, assessment lede, health/lifecycle badges)
2. Vitals Strip (ARR, health, lifecycle, renewal, NPS, meeting frequency)
3. State of Play (working/struggling)
4. The Room (stakeholders + your team)
5. Watch List (risks/wins/unknowns)
6. The Work (upcoming meetings + actions)
7. The Record (unified timeline)
8. Appendix (lifecycle events, company context, BUs, notes, files)
9. Finis
```

### Current State: MOSTLY ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Hero | Done | AccountHero with CSS module. |
| Vitals | Done | VitalsStrip component. |
| State of Play | Done | StateOfPlay component. |
| The Room | Done | StakeholderGallery with inline editing. |
| Watch List | Done | WatchList + WatchListPrograms (extracted). |
| The Work | Done | TheWork component. |
| The Record | Done | UnifiedTimeline. |
| Appendix | Done | AccountAppendix (reduced). |
| Finis | Done | FinisMarker present. |

**Remaining issues:**
- AccountFieldsDrawer still active (I343 — should be inline editing)
- Hardcoded rgba colors in inline styles
- Some magic number spacing values

---

## 6. Project Detail

**File:** `src/pages/ProjectDetailEditorial.tsx`
**Job:** Initiative dossier. Momentum, milestones, team.
**Accent:** Olive

### Intended Structure

```
1. Hero (name, assessment, status, owner)
2. Vitals (status, days to target, milestone progress, meeting frequency)
3. Trajectory (momentum/headwinds)
4. The Horizon (milestones, timeline risk, decisions)
5. The Landscape (Watch List)
6. The Team (stakeholders)
7. The Work (actions — promoted from appendix)
8. The Record (timeline)
9. Appendix (milestones full list, description, notes, files)
10. Finis
```

### Current State: MOSTLY ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| All sections | Present | Follows entity skeleton correctly. |
| Finis | Done | |

**Remaining issues:**
- ProjectFieldsDrawer still active (I343)
- Same hardcoded color / spacing issues as AccountDetail

---

## 7. Person Detail

**File:** `src/pages/PersonDetailEditorial.tsx`
**Job:** Relationship dossier for individual people.
**Accent:** Larkspur

### Intended Structure

```
1. Hero (name, assessment, temperature, email, social)
2. Vitals (temperature, meeting frequency, last met, meeting count)
3. The Dynamic/Rhythm (relationship analysis)
4. The Network (connected entities)
5. The Landscape (Watch List)
6. The Work (actions — from 1:1 meetings, open heuristic question)
7. The Record (timeline)
8. Appendix (profile, notes, files)
9. Finis
```

### Current State: ALIGNED

All sections present. FinisMarker present. Same inline style issues as other entity pages.

---

## 8. Risk Briefing

**File:** `src/pages/RiskBriefingPage.tsx`
**Job:** Slide-deck risk report generated from account intelligence.
**Layout:** Scroll-snap sections (not standard magazine layout)

### Structure

```
1. Cover (entity name, date)
2. Bottom Line (executive summary)
3. What Happened (events driving risk)
4. The Stakes (what's at risk)
5. The Ask (recommended action)
6. The Plan (action plan)
7. Finis
```

### Current State: COMPLIANT

Best performer in the audit. Clean token usage, consistent patterns throughout.

---

## 9. Emails

**File:** `src/pages/EmailsPage.tsx`
**Job:** Email intelligence surface. Gateway from daily briefing.
**Layout:** Magazine layout with margin grid

### Current State: GOOD

Uses margin grid pattern, section rules, FinisMarker. Good compliance overall.

**Remaining issues:**
- Not in FloatingNavIsland navigation (I358)
- No meeting-centric redesign yet (I358)

---

## 10. Settings

**File:** `src/pages/SettingsPage.tsx`
**Job:** Connections hub. You / Connections / System.
**Layout:** Magazine layout, 900px max-width

### Current State: ALIGNED

Redesigned per I349 as connections hub. ChapterHeading sections. FinisMarker present.

---

## 11. Entity List Pages

**Files:** `AccountsPage.tsx`, `ProjectsPage.tsx`, `PeoplePage.tsx`
**Job:** Browse and find entities.
**Shared:** EntityListShell + EntityRow components

These use the shared list pattern from I223.

---

## Compliance Summary

| Page | Structure | Tokens | Typography | Layout | Vocab | Finis | Overall |
|------|-----------|--------|------------|--------|-------|-------|---------|
| Daily Briefing | A | A | A | A | B | A | **A** |
| Weekly Forecast | A | A | A | A | B | A | **A** |
| Meeting Detail | A | C | A | C | C | A | **B-** |
| Actions | B | A | A | B | C | F | **C+** |
| Account Detail | A | B | A | B | B | A | **B+** |
| Project Detail | A | B | A | B | B | A | **B+** |
| Person Detail | A | B | A | B | B | A | **B+** |
| Risk Briefing | A | A | A | A | A | A | **A+** |
| Emails | A | A | A | A | B | A | **A** |
| Settings | A | A | A | A | A | A | **A** |

**Best:** RiskBriefingPage, DailyBriefing, EmailsPage, SettingsPage
**Worst:** MeetingDetailPage (inline style debt), ActionsPage (missing FinisMarker, vocabulary, deceptive tooltip)
