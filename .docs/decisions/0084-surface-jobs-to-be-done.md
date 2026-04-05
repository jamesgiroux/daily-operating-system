# ADR-0084: Surface Jobs-to-Be-Done

**Date:** 2026-02-18
**Status:** Accepted
**Participants:** James Giroux, Claude Code
**Related:** ADR-0073 (Editorial design language), ADR-0077 (Magazine layout), ADR-0081 (Event-driven meeting intelligence), ADR-0083 (Product vocabulary), I342 (Surface JTBD critique)

---

## Context

DailyOS has been built at speed. Features were added because they were possible, scope expanded six times, and a full design system refresh was applied to content that was never questioned. The app has never had a "why is this here?" pass. Every time we've been critical, the app has been better for it.

A six-agent audit (I342 Phases 1-2) inventoried every visible element across all five surface types: Daily Briefing, Weekly Forecast, Meeting Briefing, Actions, and Entity Detail. The audit found significant content duplication (four independent action row implementations, three meeting row implementations, the Lead Story duplicating 60-70% of Meeting Detail), dead code, system vocabulary throughout, and sections that couldn't articulate their job.

This ADR defines the job each surface does, the boundaries between surfaces, and the section-level jobs within each surface. It is the reference for all subsequent UI work, including I341 (vocabulary audit) and I330/I331 (0.13.0 surface rebuilds).

---

## Decision

### 1. Surface Jobs-to-Be-Done

#### Daily Briefing (Today)

**Job:** Morning situational awareness and preparation for today.

**Situation:** It is morning (or the user is returning mid-day for re-orientation). They need to understand what today demands.

**Desired outcome:** The user reads top-to-bottom in 2-5 minutes and reaches the Finis marker feeling: "I know what my day looks like, I know what's important, I know where the risk is, and I know what I need to do."

**Boundary:** This surface is for TODAY. It is not a task management interface, not a meeting prep deep-dive, not a relationship research tool, not a weekly planner, not an email client.

**Section jobs:**

| Section | Job | One sentence |
|---------|-----|-------------|
| Hero | Set the tone in one glance | "What kind of day is this?" |
| Focus | Direct energy and quantify capacity | "What should I prioritize and how much time do I have?" |
| Lead Story | Highlight the one meeting that most needs awareness | A brief intelligence narrative — "why this meeting matters" — plus bridge link to the full briefing |
| Schedule | Temporal map of the day; expanded cards show meeting facts (agenda, attendees, docs) | "What's on my calendar?" Card = what is this meeting. Briefing = how do I walk in ready. |
| Review | Triage AI-suggested actions | "Has the system found things I should be tracking?" |
| Priorities | Surface the most urgent actions | "What should I work on when I'm not in meetings?" |
| Emails | Surface 3-4 emails needing attention | "Is anything in my inbox urgent?" |
| Finis | Signal completion | "You're briefed." |

#### Weekly Forecast (This Week)

**Job:** Week-shape planning — understand the topology of the week, identify top priorities, and find deep work time.

**Situation:** Monday morning (or Sunday evening). The user wants to understand what the week requires and where their attention should go.

**Desired outcome:** The user knows their top 3 priorities, which days are heavy vs. light, and where deep work fits. A focused 2-minute read.

**Boundary:** This surface shows the SHAPE and STRATEGY of the week. It does not duplicate the daily schedule, the action queue, or the meeting list. Those surfaces own that content. The weekly forecast is consumed, not interacted with (no meeting prep, no action management, no email triage).

**Section jobs:**

| Section | Job | One sentence |
|---------|-----|-------------|
| Hero | Orient with a synthesized week narrative | "What does this week mean?" |
| The Three | Force-rank to exactly three priorities | "If I could only accomplish three things, what should they be?" |
| The Shape | Visualize the multi-day rhythm | "Which days are packed, which are light, where's the crux?" |
| Open Time | Identify deep work slots across the week | "Where can I do real work, and what should I use that time for?" |
| Finis | Signal completion | "Your week is forecasted." |

**Note:** "Your Meetings" and "Commitments" chapters are removed. They duplicated the daily briefing and actions page with less capability.

#### Meeting Briefing (Meeting Detail)

**Job:** Two temporal jobs — (1) "Brief me before this meeting" and (2) "Close out this meeting."

**Situation:** The user has an upcoming meeting and wants to arrive informed, or a past meeting where outcomes need capturing.

**Desired outcome:** Future: "I know what to say, what to watch for, and who I'm dealing with." Past: "Outcomes are captured, actions are tracked, I can move on."

**Boundary:** Everything needed to prepare for or close out a single specific meeting. Not a general account dashboard, not a relationship tool, not a full action queue.

**Section jobs:**

| Section | Job | One sentence |
|---------|-----|-------------|
| Outcomes (past) | Show what happened so the user can close the loop | "What came out of this meeting?" |
| Act I: The Brief | Orient instantly — what, why, one key insight | "What's the one thing I need to know?" |
| Act II: The Risks | Surface what could go wrong | "What should I watch for?" |
| Act II: The Room | Comprehensive stakeholder briefing | "Who will I be talking to?" |
| Act II: Your Plan | The user's personal agenda | "What am I going to cover?" |
| FinisMarker | "You're briefed" — essential content ends here | Everything below is optional reference |
| Act III: Deep Dive | Optional supporting material | "If I want to go deeper..." |
| Appendix | Raw reference (collapsed) | Intelligence summary, since last meeting, full context, extended stakeholders, references |

**Note:** Appendix reduced from 9 to 5 sub-sections. Strategic Programs, Current State, Key Principles removed (entity detail owns this). Questions to Surface removed (redundant with Your Plan). Second FinisMarker removed.

#### Actions

**Job:** Commitment inventory — triage, execution, and satisfaction.

**Situation:** The user has accumulated commitments across meetings, emails, and AI suggestions. They need a single place to see, triage, and work through them.

**Desired outcome:** Nothing is slipping. Suggestions triaged, completed work checked off, obligations clear.

**Boundary:** Manages the commitment inventory. Does not manage meeting prep, relationship intelligence, or weekly planning.

**Section jobs:**

| Section | Job | One sentence |
|---------|-----|-------------|
| Proposed tab | Triage queue — decide which suggestions are real commitments | "Is this real?" (defaults here when suggestions exist) |
| Pending tab | Execution queue — work through to-dos | "What do I need to do?" (with temporal grouping: overdue/today/upcoming) |
| Completed/Waiting/All | Status dashboard | "Confirm progress" or "find a past action" |
| Create form | Capture a new commitment | "Turn a thought into a tracked action" |

#### Entity Detail (Account / Project / Person)

**Job:** Relationship dossier — "give me the full picture of this relationship or initiative."

**Situation:** Before a customer meeting, during quarterly reviews, when a health indicator changes, or when someone asks "what's going on with X?"

**Desired outcome:** Can articulate the current state, risks, key people, and what needs attention. Enough to act confidently.

**Boundary:** The dossier, not the action queue. Builds understanding, not to-do lists. Actions appear consistently across all entity types as context, but the Actions page owns management.

**Structural note:** Three genuinely different surfaces sharing a skeleton. Bookend chapters (hero, timeline, appendix) are shared. Middle "analysis" chapters are unique per type and reflect different jobs:
- **Accounts:** State of Play (working/struggling), The Room (stakeholders), Watch List (risks/wins/unknowns + programs), The Work (actions + meetings)
- **Projects:** Trajectory (momentum/headwinds), The Horizon (milestones, timeline risk, decisions), The Landscape (risks/wins/unknowns + milestones), The Team
- **People:** The Dynamic/Rhythm (adaptive framing), The Network (connected entities), The Landscape (risks/wins/unknowns)

---

### 2. Phase 3 Changelist

The following changes are approved for Phase 4 execution.

#### Cuts

| ID | What | Surface | Impact |
|----|------|---------|--------|
| A1 | "Your Meetings" chapter | Weekly Forecast | ~120 lines removed |
| A2 | "Commitments" chapter | Weekly Forecast | ~130 lines removed |
| A3 | "Prefill Prep" and "Draft Agenda" buttons on deep work cards | Weekly Forecast | Buttons removed from Open Time |
| A4 | "Later This Week" action group | Daily Briefing | Priorities becomes Overdue + Due Today only |
| A5 | Second FinisMarker | Meeting Detail | Keep mid-page, remove end-of-page |
| A6 | Strategic Programs, Current State, Key Principles in appendix | Meeting Detail | Appendix: 9 → 6 sub-sections |
| A7 | Body-level transcript CTA (keep folio bar version) | Meeting Detail | Remove dashed-border body CTA for past meetings |
| A8 | Dead code: AppSidebar, WatchItem, ActionList, ActionItem, EmailList, entityMode handling | Multiple | ~500-800 lines removed |
| B3 | "Questions to Surface" in appendix | Meeting Detail | Cut entirely (appendix: 6 → 5 sub-sections) |

#### Merges

| ID | What | Deliverable |
|----|------|-------------|
| C1 | Four action row implementations | Shared `ActionRow` editorial component with density variants |
| C2 | Two proposed action triage UIs | Shared `ProposedActionRow` component with `compact` prop |
| C3 | Three meeting row implementations | Shared `MeetingRow` base component; `BriefingMeetingCard` stays complex |
| C4 | Three intelligence field update callbacks | Shared `useIntelligenceFieldUpdate` hook |
| C5 | Two keywords implementations | Shared `ResolutionKeywords` component |

#### Rethinks

| ID | What | Decision |
|----|------|----------|
| D1 | Lead Story scope | **Compact it.** Narrative + bridge link only. A brief, unique intelligence narrative — not a rehash of the meeting detail page. |
| D2 | Featured meeting duplication in Schedule | **Differentiate.** Lead story is a unique intelligence narrative. Schedule expansion panel shows its own information (prep grid, action checklist, entity chips). Materially different content. |
| D3 | Key People temperature dots | **Defer.** Remove hardcoded `hot` class so no dot shows. Add back when real signal bus data arrives (I306/I307). |
| D4 | Folio bar action density | **Remove Prefill.** Keep Transcript + Draft Agenda + Refresh. (All changes in 0.13.0 when refresh concept changes.) |
| D5 | Entity detail actions inconsistency | **Standardize.** Actions appear as a main chapter on all entity types. People gain an actions section. |
| D6 | "Before This Meeting" overlap | **Merge into one.** Single "Before This Meeting" section on meeting detail combining intelligence readiness items and tracked actions. Should be derived from intelligence and signals. |
| D7 | Actions page default tab | **Smart default.** Default to "proposed" when suggestions exist, otherwise "pending." Add temporal grouping (overdue/today/upcoming) to pending tab. |
| D8 | Command Menu navigation | **Complete it.** Add all navigable surfaces. Align labels with nav: Today, This Week, Actions, People, Accounts, Projects, Settings, Emails, Inbox. |

#### Left Alone

| ID | What | Reason |
|----|------|--------|
| B1 | EntityPicker on daily briefing | User prefers keeping editing affordances accessible on the reading surface |
| B2 | AgendaDraftDialog on weekly forecast | Stays available (though deep work card buttons that triggered it are cut via A3) |

---

### 3. Ownership Principles

Established by the cross-cutting analysis, these govern where information lives:

- **Meeting Detail** OWNS all meeting intelligence (prep, stakeholders, context, risks). Other surfaces SURFACE a preview and LINK.
- **Actions Page** OWNS the complete action list with full CRUD. Other surfaces SURFACE today's priorities and LINK.
- **Entity Detail** OWNS all entity intelligence, context, and history. Meeting Detail SURFACES entity context relevant to that meeting and LINKS.
- **Daily Briefing** SURFACES today's situational awareness from all sources. It is a reading surface that creates pull toward detail pages.
- **Weekly Forecast** SURFACES the week's shape, priorities, and deep work. It is consumed, not interacted with.

### 4. Meeting Card Expanded State — Design Decision

The expanded meeting card on the daily briefing schedule is a **pre-meeting orientation layer**. Its job is to answer "what is this meeting and what are we covering" — not to surface strategic context or account intelligence. That distinction keeps the card focused and the information hierarchy clear.

**Card shows (expanded state):**
- Meeting agenda (read-only, sourced from the meeting briefing)
- Calendar description
- Attached documents or links
- Attendees

**Card does NOT show:**
- Wins, risks, talking points, account health, prep notes — these belong exclusively to the meeting briefing. The expanded card deliberately excludes this layer so it doesn't compete with the briefing and stays scannable.

**Agenda as shared data:** The meeting briefing is the single source of truth for the agenda. The expanded card reads from it. If the agenda is updated or generated in the briefing, the card reflects that change. Editing the agenda happens in the briefing only — not inline on the card.

**Summary:** Card = what is this meeting. Briefing = how do I walk in ready.

This resolves the D1/D2 rethinks cleanly:
- The **Lead Story** is a brief intelligence narrative (the "why this meeting matters" headline) + bridge link to the full briefing. No prep grid, no key people, no action checklist.
- The **schedule expansion panel** shows meeting facts (agenda, description, attendees, docs). No intelligence content. Zero overlap with the meeting briefing.
- The **"Before This Meeting"** items (D6) are owned by the briefing. The card doesn't surface them.

### 5. Entity Detail Editing — Inline, Not Drawers

Entity detail pages are documents you read. Editing should be part of reading, not a separate mode. Pulling the user into a drawer to change a field breaks the editorial flow — the data is already displayed on the page, so edit it there.

**Decision:** Eliminate field-editing drawers. All entity data is edited inline on the page where it's displayed.

**What this means:**
- **AccountFieldsDrawer** — delete. Name, health, lifecycle, ARR, NPS, renewal date are all displayed in the hero or vitals strip. They become inline-editable there (click to edit, same as StakeholderGallery names and roles).
- **ProjectFieldsDrawer** — delete. Same pattern. Status, owner, milestone, target date edit inline.
- **PresetFieldsEditor** — renders inline in the appropriate page section, not in a drawer or sidebar.
- **TeamManagementDrawer** — exception. Adding team members involves search and create workflows that warrant a modal. But viewing/removing team members should be inline in the StakeholderGallery "Your Team" section.

**Why this aligns:**
- StakeholderGallery already works this way — inline editable names, roles, engagement badges. It's the model.
- I343 (inline editing service) is scoped to make this the standard: unified `EditableText`, keyboard navigation between fields, signal emission on every edit.
- The JTBD for entity detail is "the dossier." You read it, you correct it, you move on. A drawer says "stop reading and go fill out a form." Inline says "this is your data, tap to fix it."

**Implementation note:** This is I343 scope. The drawers that shipped in 0.11.0 (I312) work and don't need to be ripped out today. But as I343 lands, each drawer's fields migrate to inline editing and the drawer is deleted. The direction is clear: no drawers for data that's already visible on the page.

---

## Consequences

### Positive

- **Every section on every surface has a stated job.** New features must justify which job they serve.
- **Duplication is identified and addressed.** Five merge items consolidate ~500 lines of duplicated code into shared components.
- **The weekly forecast becomes focused.** Four sections that do unique work, zero that duplicate other surfaces.
- **The daily briefing stays in its lane.** Today only. No weekly leakage.
- **The meeting detail appendix is pruned.** From 9 sub-sections to 5. Entity-level data stays on entity pages.
- **Actions become consistent across entity types.** All three entity types show actions.

### Negative

- **Phase 4 is substantial.** Cuts, merges, and rethinks touch every surface. This should be sequenced carefully.
- **Some users may miss removed content** (e.g., meetings chapter on weekly forecast). But the same content exists on better surfaces.

### Risks

- **Lead Story compaction** could reduce the daily briefing's perceived value if the intelligence narrative isn't compelling enough on its own. Mitigate by ensuring the narrative is high-quality, not just the first paragraph of the meeting prep.
- **Shared components** add a maintenance benefit but introduce coupling — changes to the shared ActionRow affect all surfaces. Mitigate with density variants and careful prop design.

---

## References

- I342 Phase 1-2 audit reports: `.docs/research/i342-*.md` (6 files)
- I342 Phase 3 decision checklist: `.docs/research/i342-phase3-decisions.md`
- ADR-0083: Product vocabulary (verbal identity)
- ADR-0073: Editorial design language (visual identity)
- ADR-0077: Magazine layout (structural identity)
- ADR-0081: Event-driven meeting intelligence (I330/I331 context)
