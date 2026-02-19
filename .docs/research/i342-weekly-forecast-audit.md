# I342 Weekly Forecast Surface Audit

**Date:** 2026-02-18
**Auditor:** weekly-forecast-auditor (Claude)
**Surface:** Weekly Forecast (`/week`, `WeekPage.tsx`)
**Status:** Complete

---

## Phase 1: JTBD Definition

### Surface-Level JTBD

**Situation:** It's Sunday evening or Monday morning. The user is about to enter a new work week and wants to understand what's coming — what days are heavy, which meetings matter, whether any commitments are at risk, and where they'll find time to do real work.

**Motivation:** Reduce the anxiety of uncertainty about the upcoming week. Shift from reactive ("what's happening today?") to proactive ("what does this week actually require of me?"). Make one planning pass that informs the whole week.

**Desired outcome:** The user closes this surface knowing: (1) their top 3 priorities, (2) which days are heavy vs. light, (3) which meetings need preparation, (4) where deep work fits, and (5) whether any commitments are overdue or at risk.

**Boundary:** This surface is about the *shape* and *strategy* of the week. It is NOT the place to execute work (complete actions, prep meetings, respond to emails). It is NOT a daily schedule. It is NOT an account health dashboard.

### When Does the User Reach for This Surface?

1. **Monday morning** (primary) — "What does my week look like?"
2. **Sunday evening** (secondary) — "Should I be worried about this week?"
3. **Mid-week reorientation** (rare) — "I've lost the plot. What was I supposed to be focused on?"

### What Does "Done" Look Like?

The user has a mental model of the week's shape and has identified where their attention should go. They may navigate to meeting detail pages to prep, or to the actions page to triage, but the weekly forecast itself is consumed, not interacted with. "Done" = "I've read it and I know what my week means."

### What Explicitly Does NOT Belong Here?

- Today's detailed schedule (that's the daily briefing's job)
- Action execution (completing, creating, editing actions)
- Meeting preparation (that's the meeting detail page's job)
- Email triage
- Account/project detail or health trends
- Historical meeting review

---

### Section-Level JTBDs

#### 1. Hero Section (Week Narrative)

**Job:** Orient the user with a single synthesized statement about what this week means. Set the emotional tone — is this a crunch week, a recovery week, a high-stakes external week?

**Elements:**
- Week number + date range (mono, centered)
- AI-generated narrative headline (serif, 32px, centered)
- Vitals line (readiness summary + meeting count)
- Enrichment retry button (when incomplete)

**Assessment:** This section has a clear, unique job. The weekly narrative is NOT available on the daily briefing. The daily has a "hero headline" that describes today; the weekly describes the arc of the entire week. This is genuinely distinct.

#### 2. The Three (Top Priorities)

**Job:** Force-rank the user's week to exactly three things. If everything is important, nothing is. This section answers: "If I could only accomplish three things this week, what should they be?"

**Elements:**
- Circled numbers (1-3)
- Priority title (serif, linked to action or meeting detail)
- Reason text (why this is a top priority)
- Context line (account, due date, day/time)

**Assessment:** This is the weekly forecast's strongest unique section. The daily briefing does NOT have a "top three" — it has a Focus section (single focus area) and a Priorities section (capacity-ranked list). The weekly's "Three" is a different job: week-level strategic prioritization vs. daily capacity allocation. However, the implementation is mechanical — it takes the AI top priority and fills slots 2-3 from overdue/due-this-week actions by score. The "strategic" quality depends entirely on the AI enrichment.

#### 3. The Shape (Day Density Visualization)

**Job:** Show the visual rhythm of the week at a glance. Which days are packed, which are light. This is the "shape" of the week — a concept that doesn't exist on any other surface.

**Elements:**
- Per-day row: day label, meeting bar (turmeric fill), meeting count + density label
- Achievability indicator (I279): "X of Y achievable" per day
- Prioritized actions per day (feasible actions, max 3, shown under the bar)
- Epigraph (synthesized: "Front-loaded. Wednesday is the crux — Clear Friday for recovery.")

**Assessment:** This is the weekly forecast's most visually distinctive section and its strongest argument for existing. No other surface shows the multi-day rhythm. The daily briefing knows about today's density but has no concept of how today compares to the rest of the week. The achievability indicators and per-day action slots (I279) add real value by connecting actions to available capacity across the week. The epigraph is uniquely delightful — a one-line characterization of the week's topology.

#### 4. Your Meetings (Multi-Day Meeting List)

**Job:** Show all meetings for the week, organized by day, with prep status and account context. Focus on external meetings that need attention.

**Elements:**
- Day groupings with day name subheading
- Per-meeting row: colored dot (external/internal + prep status), time, title, prep status badge
- Subtitle line: account + meeting type
- Links to meeting detail pages

**Assessment:** This is the most duplicated section on the weekly forecast. The daily briefing has a full Schedule section with richer meeting cards (BriefingMeetingCard with entity chips, key people, prep grids, action checklists). The weekly version is a stripped-down list — it shows meetings exist but doesn't help you prepare for them. It filters to show only days with external meetings, but the information per meeting is thin: time, title, account, and a "prepped/needs prep" label. The user cannot act on this information here — they must navigate to the meeting detail page anyway.

**Duplication verdict: HIGH.** The daily briefing's Schedule section shows today's meetings in much richer detail. The weekly forecast's meeting list covers future days but adds minimal value over simply knowing those meetings exist on the calendar.

#### 5. Open Time (Deep Work Blocks)

**Job:** Identify where in the week the user can do deep work, and suggest what to use that time for based on their priorities and meeting context.

**Elements:**
- Total deep work hours (epigraph)
- Pull quote (AI reason connecting best block to need)
- Per-block card: day + time range + duration, suggested use, reason, action buttons
- "Prefill Prep" button (for meeting-linked suggestions)
- "Draft agenda" button (opens AgendaDraftDialog)
- Link to related action or meeting

**Assessment:** This section has a genuinely unique job. The daily briefing shows today's available minutes and deep work block count (in the Focus section), but it does NOT suggest what to do with that time across the week. The weekly's Open Time section connects available capacity to priorities and meetings — it's proactive scheduling intelligence.

However, the action buttons ("Prefill Prep" and "Draft agenda") are meeting-prep actions that arguably belong on the meeting detail page, not the weekly planning surface. The weekly forecast should identify the opportunity ("You have 2 hours Wednesday before the QBR — use it to prep"); the meeting detail page should provide the tools to act on it.

#### 6. Commitments (Action Summary)

**Job:** Surface actions that are overdue or due this week, excluding the top three (to avoid repetition).

**Elements:**
- Per-action row: overdue/pending circle, title, due context, account, priority badge
- Links to action detail pages
- Summary line: total count + overdue count + "X more" link to Actions page

**Assessment:** This is heavily duplicated with two other surfaces:
1. **Daily Briefing Priorities section** — shows the same overdue + due-this-week actions, with capacity awareness (effort minutes, feasibility) and completion checkboxes. The daily version is richer and actionable.
2. **Actions Page** — the canonical home for all action management, with search, filtering, temporal grouping, and full CRUD.

The weekly's Commitments section is a read-only summary with no completion capability. The user sees the same actions here, on the daily, and on the Actions page. The weekly version adds no unique perspective — it's the same list, less capable.

**Duplication verdict: HIGH.** This section exists on three surfaces with the weekly being the least capable version.

#### 7. Finis Marker

**Job:** Signal that the document is complete. "When you've read it, you're briefed."

**Elements:**
- Three asterisk brand marks (turmeric)
- "Your week is forecasted" closing line

**Assessment:** Appropriate editorial element. Shared component (`FinisMarker`) used on multiple surfaces.

---

## Phase 2: Element Inventory

### Complete Element Inventory

| # | Element | Location (file:line) | What It Renders | Job | Duplicated On | Could User Do Without? | Verdict |
|---|---------|---------------------|-----------------|-----|--------------|----------------------|---------|
| 1 | Week number + date range | WeekPage.tsx:599-610 | "WEEK 8 - FEB 17-21, 2026" | Temporal orientation | None (daily has today's date) | No — establishes context | KEEP |
| 2 | Week narrative headline | WeekPage.tsx:613-643 | AI-generated 1-3 sentence week summary | Set the week's tone and meaning | None (daily has today-specific hero) | No — this is the core value proposition | KEEP |
| 3 | Vitals line (readiness + meetings) | WeekPage.tsx:646-661 | "2 meetings need prep - 12 meetings" | Quick health check | Daily folio bar has similar stats | Yes — duplicated, low-information | RETHINK |
| 4 | Enrichment retry button | WeekPage.tsx:664-691 | "AI enrichment incomplete. Retry enrichment" | Recovery from failed enrichment | None | No — needed for error recovery | KEEP |
| 5 | "The Three" circled numbers | WeekPage.tsx:720-727 | Circled digit glyphs | Visual hierarchy for priorities | None | No — supports scannability | KEEP |
| 6 | Priority title (The Three) | WeekPage.tsx:729-740 | Linked priority name | Identify the priority | Daily Briefing Focus (single focus) | No — different job (3 vs 1) | KEEP |
| 7 | Priority reason (The Three) | WeekPage.tsx:741-751 | Why this is top priority | Justify the ranking | None | No — explains the AI's reasoning | KEEP |
| 8 | Priority context line | WeekPage.tsx:752-762 | Account, due date, meeting time | Ground the priority in reality | Daily Priorities has similar context | Borderline — nice but not essential | KEEP |
| 9 | Day shape bars | WeekPage.tsx:826-903 | Horizontal bars showing meeting density per day | Visualize the week's rhythm | None | No — unique, core value | KEEP |
| 10 | Day density label | WeekPage.tsx:889-902 | "3 meetings - moderate" | Quantify day's load | None | Borderline — bar communicates this visually | KEEP |
| 11 | Achievability indicator (I279) | WeekPage.tsx:905-921 | "3 of 5 achievable" per day | Connect capacity to commitments | None (daily has capacity but not per-day-of-week) | No — unique cross-day intelligence | KEEP |
| 12 | Prioritized actions per day | WeekPage.tsx:925-953 | Up to 3 feasible action titles under each day | Show what fits each day | None | No — unique day-level action fit | KEEP |
| 13 | Shape epigraph | WeekPage.tsx:817 | "Front-loaded. Wednesday is the crux." | One-line week characterization | None | Delightful but not essential | KEEP |
| 14 | Meeting day groupings | WeekPage.tsx:978-1002 | Day name subheadings | Organize meetings by day | Daily has today's schedule | Partial — daily covers today | RETHINK |
| 15 | Meeting row (dot + time + title) | WeekPage.tsx:1006-1079 | Individual meeting entry | Show meeting exists + status | Daily BriefingMeetingCard (much richer) | Yes — adds little over daily + calendar | CUT/MOVE |
| 16 | Meeting prep status badge | WeekPage.tsx:1061-1077 | "prepped" / "needs prep" | Show readiness state | Daily has prep grid; Meeting detail has full prep | Yes — duplicated, not actionable here | CUT/MOVE |
| 17 | Meeting subtitle (account + type) | WeekPage.tsx:1082-1098 | "Acme Corp - customer" | Context for the meeting | Daily BriefingMeetingCard has entity byline | Yes — duplicated | CUT/MOVE |
| 18 | Meeting link to detail page | WeekPage.tsx:1100-1117 | Clickable row navigating to /meeting/$meetingId | Navigate to prep | Daily schedule links to same place | Yes — user can get here from daily | CUT/MOVE |
| 19 | Deep work hours epigraph | WeekPage.tsx:1150-1157 | "6 hours of deep work available this week" | Quantify available capacity | Daily Focus shows today's available minutes | No — weekly total is unique | KEEP |
| 20 | Pull quote (deep work) | WeekPage.tsx:1160-1162 | AI reason for top block | Connect capacity to need | None | No — editorial value, unique | KEEP |
| 21 | Deep work block card | WeekPage.tsx:1166-1299 | Day, time range, duration, suggestion | Identify deep work slots | Daily Focus shows today's blocks | Partially — today's blocks are on daily | RETHINK |
| 22 | "Prefill Prep" button | WeekPage.tsx:1244-1258 | Prefills meeting prep from suggestion | Act on a suggestion | Meeting detail page has full prep tools | Yes — wrong surface for this action | MOVE |
| 23 | "Draft agenda" button | WeekPage.tsx:1259-1271 | Opens AgendaDraftDialog | Draft an agenda email | Meeting detail page should own this | Yes — wrong surface for this action | MOVE |
| 24 | Deep work link to action/meeting | WeekPage.tsx:1274-1296 | Arrow link to related entity | Navigate to context | Duplicated navigation | Borderline | KEEP |
| 25 | Commitment row (action) | WeekPage.tsx:1334-1427 | Action with overdue indicator, title, due, priority | Show commitment status | Daily Priorities + Actions page | Yes — less capable version of both | CUT |
| 26 | Commitment summary line | WeekPage.tsx:1432-1459 | "8 total - 3 overdue - 5 more" | Quantify commitment load | Actions page has full counts | Yes — duplicated | CUT |
| 27 | "More" link to Actions page | WeekPage.tsx:1446-1455 | Link to /actions | Navigate to full list | Daily has "View all X actions" link | Yes — duplicated navigation | CUT |
| 28 | ErrorCard | WeekPage.tsx:1515-1536 | Error message display | Surface errors | Standard pattern | No — needed for error handling | KEEP |
| 29 | WorkflowProgress | WeekPage.tsx:1544-1554 | Phase-step loading screen | Show generation progress | None (daily has its own progress) | No — needed for UX | KEEP |
| 30 | Refresh button (folio bar) | WeekPage.tsx:287-308 | "Refresh" / phase label | Trigger week workflow | Daily has its own refresh | No — needed for this workflow | KEEP |
| 31 | FolioBar readiness stats | WeekPage.tsx:314-326 | "3/5 prepped - 2 overdue" | At-a-glance health | Daily folio has same stats | Yes — duplicated | RETHINK |
| 32 | Chapter navigation (CHAPTERS) | WeekPage.tsx:95-101 | The Three, The Shape, Meetings, Open Time, Commitments | In-page scroll nav | None | No — needed for editorial structure | KEEP |
| 33 | AgendaDraftDialog | WeekPage.tsx:1500-1507 | Modal dialog for agenda drafts | Draft and copy agenda email | Could be triggered from meeting detail instead | MOVE — wrong surface | MOVE |

---

## Phase 3: Collapse Hypothesis Assessment

### The Question

> Does the weekly forecast justify its existence as a separate surface, or is its job better served as a section within the daily briefing?

### Evidence For Collapsing (Surface Should NOT Exist Separately)

1. **Heavy duplication:** 10 of 33 elements are duplicated on the daily briefing or actions page. The "Your Meetings" and "Commitments" chapters are less-capable versions of content that lives richer elsewhere.

2. **Low interaction density:** The surface is almost entirely read-only. The only actions are: Refresh, Prefill Prep, Draft Agenda, and navigation links. Two of those actions (Prefill Prep, Draft Agenda) belong on the meeting detail page, not here.

3. **Narrow usage window:** The JTBD analysis reveals this is primarily a Monday-morning surface. It doesn't earn its nav space the other 6 days of the week. By Tuesday afternoon, the weekly forecast is stale context — the user has already internalized the week's shape.

4. **Daily already looks forward:** The daily briefing's Priorities section already has a "Later This Week" group showing upcoming actions. The Focus section shows today's capacity. The daily is already nudging toward a multi-day perspective.

5. **Separate workflow cost:** The weekly forecast requires its own `run_workflow("week")` call, separate from the daily briefing. This is a significant UX and infrastructure cost — two workflows to run, two surfaces to check, two places where "enrichment incomplete" can appear.

6. **Chief-of-staff metaphor:** A chief of staff gives you one briefing. "Here's your day, and here's what's coming this week that matters." Not two separate documents.

### Evidence Against Collapsing (Surface SHOULD Exist)

1. **The Shape is genuinely unique.** No other surface provides the multi-day density visualization. The daily briefing shows today's capacity; the weekly shows the *topology* of the week — front-loaded vs. back-loaded, which day is the crux, where recovery time exists. This is real, unique intelligence.

2. **The Three is a different job than daily Focus.** The daily Focus is a single-sentence focus area + capacity allocation. "The Three" is a force-ranked strategic prioritization for the entire week. These are different cognitive tools.

3. **Deep work across the week is unique.** The daily shows today's available blocks. The weekly shows where deep work fits *across all five days* — essential for deciding when to schedule focused work.

4. **The weekly narrative is distinct from the daily hero.** "Your week is dominated by partner meetings — protect Thursday for the board prep" is a different kind of insight than "Three meetings today, starting with the Acme QBR at 10."

5. **Collapsing adds complexity to the daily.** The daily briefing is already a substantial document (Hero, Focus, Lead Story, Schedule, Review, Priorities, Finis). Adding "Coming Up This Week" sections would make it even longer. The weekly is a finite document you read once; bloating the daily with weekly context risks making it something you skim rather than read.

### Verdict: Partial Collapse

The weekly forecast has **three sections with unique, justified jobs** that no other surface provides:

1. **The Shape** — multi-day density visualization with per-day achievability
2. **The Three** — week-level strategic prioritization
3. **Open Time** — multi-day deep work identification and suggestion (minus the prep-action buttons)

And **three sections that are redundant copies of better surfaces:**

1. **Your Meetings** — a thin list that duplicates the daily Schedule and adds no actionable value
2. **Commitments** — a read-only copy of what's on the daily Priorities and Actions page
3. **Vitals line** — duplicated in the folio bar and on the daily

**Recommendation:** The weekly forecast should exist, but it should be dramatically smaller. It should contain:
- Hero (week number, narrative, shape epigraph)
- The Three
- The Shape (with achievability)
- Open Time (without meeting-prep action buttons)

And it should **cut**:
- Your Meetings chapter (move meeting-attention-needed signals to readiness indicators only)
- Commitments chapter (the daily and Actions page own this)
- Action buttons for meeting prep (Prefill Prep, Draft Agenda — move to meeting detail)

This would make the weekly forecast a focused, 2-minute read: "Here's what your week means, here are your three priorities, here's the shape of your days, and here's where deep work fits." No duplication, no dead-end lists, no actions that belong elsewhere.

### Alternative: Collapse to Daily Section

If collapsing is preferred, the unique value could be preserved as a "This Week" section in the **Monday daily briefing only** (or any day the user hasn't seen it yet). This section would contain:
- A compact Shape visualization (5-bar horizontal chart)
- The Three priorities
- Deep work availability summary

This loses the editorial grandeur of a separate document but preserves the information. The weekly narrative could become the daily's hero headline on Mondays. The tradeoff is between product simplicity (one fewer surface) and the value of a dedicated planning moment.

---

## Summary: What This Surface Uniquely Provides

| Capability | Available on Daily? | Available on Actions? | Available on Meeting Detail? | Unique to Weekly? |
|---|---|---|---|---|
| Week narrative (AI synthesis of the week) | No | No | No | **Yes** |
| Top Three priorities (force-ranked) | No (Focus is 1 item) | No | No | **Yes** |
| Multi-day shape visualization | No (today only) | No | No | **Yes** |
| Per-day achievability (I279) | No | No | No | **Yes** |
| Shape epigraph ("Front-loaded...") | No | No | No | **Yes** |
| Deep work blocks across the week | No (today only) | No | No | **Yes** |
| Deep work suggestions with reasons | No | No | No | **Yes** |
| Meeting list by day | Partially (today only) | No | No | Partially |
| Meeting prep status | Yes (richer) | No | Yes (richest) | No |
| Overdue actions | Yes | Yes | No | No |
| Due-this-week actions | Yes ("Later This Week") | Yes | No | No |
| Action completion | Yes | Yes | Yes | No |
| Prefill Prep button | No | No | Should be here | No |
| Draft Agenda button | No | No | Should be here | No |

---

## Appendix: Files Read

- `src/pages/WeekPage.tsx` (1555 lines) — main surface component
- `src/pages/weekPageViewModel.ts` (503 lines) — view model with all derived data logic
- `src/pages/weekPageViewModel.test.ts` (252 lines) — unit tests
- `src/types/index.ts` — WeekOverview, WeekDay, WeekMeeting, WeekAction, DayShape, ReadinessCheck, TopPriority, TimeBlock, DailyFocus, DashboardData types
- `src/components/dashboard/DailyBriefing.tsx` (866 lines) — daily briefing for comparison
- `src/components/editorial/ChapterHeading.tsx` — shared editorial component
- `src/components/editorial/PullQuote.tsx` — shared editorial component
- `src/components/editorial/FinisMarker.tsx` — shared editorial component
- `src/components/editorial/GeneratingProgress.tsx` — shared workflow progress component
- `src/components/ui/agenda-draft-dialog.tsx` — agenda draft dialog (used only by WeekPage)
- `src/components/layout/FloatingNavIsland.tsx` — navigation (weekly is a first-class nav item)
- `src/components/layout/AppSidebar.tsx` — sidebar nav (weekly as "This Week")
- `src/router.tsx` — route definitions
- `.docs/decisions/0083-product-vocabulary.md` — product vocabulary reference
- `.docs/BACKLOG.md` (I342 section) — issue definition and collapse hypothesis
