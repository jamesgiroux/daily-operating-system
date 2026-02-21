# I342 Surface Redesign — From First Principles

**Date:** 2026-02-19
**Method:** Each element must earn its keep. If you can't state the specific job it does for the user on THIS surface, it's gone.
**Input:** i342 surface audits (2026-02-18), cross-cutting audit

---

## Product Mental Model

**DailyOS is the intelligence layer.** It ingests signals from everywhere — calendar, email, meetings, accounts — and derives insights no single app can produce on its own.

- **80% consumption.** The system prepares. The user reads, orients, and absorbs.
- **20% production.** The user acts on, corrects, or uses the intelligence to produce something.

DailyOS is not replacing collaboration tools, calendars, or email. It unites them. The intelligence context lives here, so some production work *has* to happen here — because nowhere else has the context.

**What DailyOS does NOT replace:** email clients, calendars, task managers, scheduling tools, document editors, CRM, analytics, company information synthesis (aka Glean), or project management.

---

## Design Principle

> "Make every chapter or element earn its keep."

### Filter for consumption elements

An element earns its keep by answering:
1. What specific job does this do for the user on THIS surface?
2. Is there a better owner for this job elsewhere in the app?

### Filter for production elements

Production work earns its keep only if: **the user cannot do it anywhere else without losing context**, because the intelligence or data lives in DailyOS.

Examples that earn their place:
- Marking a proposed action done (DailyOS proposed it, tracks it)
- Editing an AI-drafted agenda (drafted from DailyOS intelligence)
- Sharing that agenda (output lives here)
- Generating a risk report (account intelligence lives nowhere else)

Examples that don't earn their place:
- General email composition
- Scheduling meetings
- Managing a backlog that doesn't depend on DailyOS signals

---

## Surface Decisions

### 1. Daily Briefing

**Surface job:** Transition from not-working to working with confidence. Answer three questions in 2-5 minutes: what's happening today, what needs my attention before I start, where should I focus my non-meeting time.

**Decisions:**

| Element | Decision | Rationale |
|---------|----------|-----------|
| Hero headline | **Keep, compress** | Sets tone for the day. Merge with Focus into one "Day Frame" section — no need for two separate sections doing the same framing job. |
| Focus / capacity line | **Keep, merge with Hero** | Answers "where do I direct my energy + how much time do I have." High value. |
| "The Meeting" / Lead Story | **Repurpose as "Up Next"** | "Most consequential" is an invisible algorithm — user always assumes it means "next." Make it temporal. The next meeting gets richer inline treatment. |
| Consequential later meeting | **Light callout only** | If a high-stakes meeting is later in the day AND needs morning prep, surface a single flag within the schedule. Not a full lead story. |
| Lead Story: Key People flow | **Cut** | Belongs on Meeting Detail. Not needed for "what's my next meeting." |
| Lead Story: Prep Grid | **Cut** | Belongs on Meeting Detail. One line of context is enough on the briefing. |
| Lead Story: Entity chips + picker | **Cut from briefing** | Editing affordance on a reading surface. Meeting Detail owns entity assignment. |
| Lead Story: Action checklist | **Keep, move into Actions section** | Meeting-relevant actions earn their keep on the briefing. Belong with other actions, not in the lead story. |
| Schedule | **Keep** | Core. Non-negotiable temporal map. |
| Schedule: Up Next treatment | **Expand by default** | The next upcoming meeting gets inline context (1 sentence) + pre-meeting actions. Replaces separate Lead Story section. |
| Review: Proposed action triage | **Keep, filter tighter** | DailyOS proposed these from its signals — triage belongs where the intelligence lives. But limit to meeting-relevant or high-signal only. |
| Priorities: Overdue + Due Today | **Keep, filter tighter** | 2-3 max. Meeting-relevant or quick wins only. Not AI priority rank. The rest belongs on Actions page. |
| Priorities: "Later This Week" | **Cut** | Not the daily briefing's job. Actions page and Weekly Forecast own this. |
| Priorities: Email sub-section | **Keep** | Earns its keep: triage preview + sole gateway to Emails page. 3-4 urgent emails max. |
| Finis | **Keep** | Psychological closure. Non-negotiable. |

**Proposed structure:**

```
1. Day Frame
   — 1-sentence AI narrative ("what kind of day is this")
   — Capacity: X hours free, N meetings
   — Focus: where to direct non-meeting energy

2. Schedule
   — Full temporal map
   — First upcoming meeting: expanded by default (1-sentence context + prep status)
   — High-stakes later meeting: light flag if needs morning attention
   — Past meetings: outcomes summary on click

3. Attention
   — 2-3 actions: meeting-relevant for today OR overdue/quick wins
   — Proposed action triage (high-signal only, capped)
   — 3-4 urgent emails (gateway to Emails page)

4. Finis
```

**Note on emails nav:** The briefing is currently the sole entry point to the Emails page (not in nav island by design). The email sub-section therefore serves double duty: triage preview + navigation gateway. If Emails ever needs broader access, that's a nav island decision, not a briefing decision.

---

### 2. Meeting Detail

**Surface job:** Prepare you for a specific meeting — or close it out after. Everything on this surface must earn its keep by answering "does this help me walk in informed, or close the loop after?" Entity-level depth belongs on the entity page; link there.

**Two modes, one surface:** The surface adapts based on temporal state. Pre-meeting: full briefing mode. Post-meeting: outcomes at top, pre-meeting content accessible below.

**Decisions:**

| Element | Decision | Rationale |
|---------|----------|-----------|
| Urgency banner | **Keep** | "Meeting in X minutes" is the most time-sensitive element on any surface. Non-negotiable. |
| Key insight pull quote | **Keep** | The one thing to know about this meeting. Core. |
| The Room (attendee list) | **Keep** | Who you're talking to and their disposition. Intelligence only DailyOS has. Fully earned. |
| The Risks | **Keep** | What to watch for. Earned. |
| Your Plan (agenda editor) | **Keep** | Strongest justification for DailyOS as production surface. Context lives here. Can't do this elsewhere. |
| Draft Agenda | **Keep** | Produces output from DailyOS context. Earns its keep. |
| "Before This Meeting" (readiness + actions) | **Keep, merge** | Two sources currently (intelligence-derived readiness + DB-backed actions) answer the same question. Merge into one list. |
| Finis | **Keep, one only** | After Your Plan. That's the end. |
| Deep Dive zone (Act III) | **Cut entirely** | Recent Wins, Open Items, Email Signals — link to entity detail for this depth. |
| Appendix (all 9 sub-sections) | **Cut entirely** | Strategic Programs, Current State, Key Principles, Full Context, Questions to Surface, etc. — entity-level data. Entity detail owns it. |
| Second FinisMarker | **Gone** | Removed with the deep dive. One end: after Your Plan. |
| Outcomes section | **Keep** | Summary, wins, risks, decisions, actions from transcript. Core post-meeting job. |
| Action triage from outcomes | **Keep** | Meeting-scoped, DailyOS-derived. Triage belongs where the intelligence lives. |
| Transcript attachment | **Keep, body only** | Remove folio bar duplicate. Body-level CTA is more contextual and discoverable. |
| Prefill button | **Move into Your Plan section** | Remove from folio bar. Too opaque as a toolbar button. Lives naturally as an affordance within the plan editor when AI-proposed items are available. |
| Refresh button | **Keep in folio bar** | Standard. |
| Draft Agenda button | **Keep in folio bar** | Produces shareable output — appropriate at toolbar level. |

**Proposed structure:**

```
1. Urgency banner (if < 2h away)

2. The Brief
   — Key insight (the one thing to know)
   — Meeting metadata + entity chips
   — Before this meeting (merged: readiness items + tracked actions)

3. The Risks

4. The Room
   — Attendee list with disposition, assessment, engagement

5. Your Plan
   — AI-proposed + user agenda items
   — Prefill affordance lives here (not folio bar)

6. Finis

— — — — —

[Post-meeting only, above The Brief:]
Outcomes
   — Summary, wins, risks, decisions
   — Action triage
   — Transcript attachment CTA (if no transcript yet)
```

**Dependency note:** Cutting the deep dive only works if the entity detail page is good enough to hold that context confidently. Meeting detail can link there — but it has to be worth navigating to.

---

### 3. Weekly Forecast

**Surface job:** Monitor and act on meeting intelligence across a rolling ±7-day window — before preparation becomes urgent. Also the strategy surface for the week: what it means, where to direct energy, which days are the crux.

**Not just Monday morning.** The primary trigger is mid-week: a follow-up lands for next week while yesterday's context is still fresh. The daily briefing only knows today; meeting detail for a future meeting has no intelligence yet. This surface holds the connection.

**Decisions:**

| Element | Decision | Rationale |
|---------|----------|-----------|
| Week narrative | **Keep** | "What does this week mean?" Unique to this surface. |
| The Three | **Keep** | Week-level force-ranked priorities. Different job from daily Focus (1 item, today). |
| The Shape | **Keep** | Multi-day density visualization. Nothing else shows how Monday compares to Thursday. |
| "Your Meetings" chapter (current thin list) | **Replace** | Thin list with meaningless "needs prep" badges. Replaced by meeting intelligence timeline. |
| Meeting intelligence timeline (±7 days) | **New — I330** | Past: outcomes, follow-ups generated, context seeds. Future: intelligence quality, prep gaps, days until, new signals. Shows connections between related meetings (e.g. demo → follow-up). |
| Open Time / deep work blocks | **Cut** | Too deterministic. Telling the user how to use their calendar is a calendar tool's job. If intelligence ever gets good enough to say "your EBR is thin, you have Thursday 2-4pm" — that signal belongs inside the meeting timeline, not a separate section. |
| Commitments chapter | **Cut** | Read-only copy of daily priorities + actions page. Less capable than both. |
| Prefill / Draft Agenda buttons | **Cut** | Meeting prep actions belong on meeting detail. |

**Proposed structure:**

```
1. Week narrative + The Three
   — What does this week mean (AI synthesis)
   — Three force-ranked priorities

2. The Shape
   — Multi-day density bars
   — Per-day achievability indicators

3. Meeting intelligence timeline (±7 days)
   — Past meetings: outcomes, follow-ups, context seeds
   — Future meetings: intelligence quality indicator, prep gap, days until, new signals
   — Connections between related meetings surfaced

4. Finis
```

**Hard dependency:** The meeting intelligence timeline (I330) only works with ADR-0081 in place — intelligence accumulating continuously from when a meeting appears on the calendar. Without event-driven intelligence, future meetings have nothing to show. This is 0.13.0 work. Until then, the current surface ships with cuts (Meetings chapter, Commitments, action buttons) and gains (The Three, The Shape) as scoped in 0.12.1.

---

### 4. Actions

**Surface job:** The authoritative commitment inventory. Triage AI-suggested actions, surface what's relevant to upcoming meetings, and ensure nothing slips — without creating a guilt-inducing backlog.

**Design philosophy:** Actions are time-aware and meeting-aware, not permanent. The system surfaces them when relevant. If they go stale, they retire. The user's job is to act — not to maintain.

**Core decisions:**

| Element | Decision | Rationale |
|---------|----------|-----------|
| Manual action creation | **Keep** | User creates manual tasks. Context-aware (entity linking, due date) makes it useful vs. other tools. |
| Suggested tab as default | **Keep, smart default** | Land here when suggestions exist. Triage is the highest-priority job. |
| Temporal grouping on pending | **Add** | Flat list of 30 actions is unreadable. Overdue / today / upcoming grouping needed. |
| Meeting-centric primary view | **New** | Primary organizing axis is upcoming meetings, not urgency. "You've got Acme Thursday — here are 3 related actions." Next N meetings with associated actions, not a fixed day window. |
| "Everything else" | **Secondary** | Actions with no upcoming meeting context shown below meeting-relevant section. Self-managing with auto-expiry so never overwhelming. |
| By-account view | **Add as option** | Secondary view: show all actions for a given entity. Useful for account reviews. |
| Auto-expiry at 30 days | **New** | Pending actions expire quietly after 30 days. No manual grooming needed. Zero-guilt — if 30 days passed, the moment has passed. |
| "Waiting" tab | **Cut as tab** | Undercooked — no workflow to populate it from the list. Keep "waiting" as a status badge on individual rows. |
| "All" tab | **Cut** | Power-user edge case. Not worth the tab weight. |
| Completed tab | **Keep** | Satisfaction signal. Record of progress. |
| Priority filter tabs | **Rethink** | Within meeting-centric groups, priority is implicit (P1 first). Standalone priority filter tabs may be redundant. |
| Flat list search | **Fix** | Currently searches accountId not account name. Needs to search title, account name, and context. |
| FolioBar date | **Remove** | Actions aren't day-scoped. The date adds no information here. |
| Bulk triage | **Consider** | If 15 suggestions queue overnight, one-by-one accept/reject creates fatigue. Bulk "dismiss all" at minimum. |

**Proposed structure:**

```
Suggested (default when non-empty)
— Triage queue. Clear it.
— [Bulk dismiss option]

Pending (primary view)
— Meeting-relevant (next N meetings with associated actions)
  [Acme QBR · Thursday]
    · Action 1 (P1)
    · Action 2 (P2)
  [Jefferies follow-up · Monday]
    · Action 1
— Everything else
  — Recent actions without meeting context
  — (self-cleaning: auto-expires at 30 days)

Completed
— Record of progress

[View by account — secondary option]
```

**New contract with the user:** Never groom. Never delete. Act when the system surfaces something. If 30 days pass, the system retires it quietly.

---

### 5. Entity Detail (Accounts / Projects / People)

**Surface job:** The dossier. Full picture of a relationship or initiative — to show up informed, make good decisions, and not miss anything. For a CSM, the account entity is the centre of the universe: everything flows in (meetings, signals, actions, notes) and out (meeting context, reports, MCP queries).

**Three surfaces, one skeleton.** Shared bookend chapters (hero, timeline, appendix). Unique middle chapters that reflect genuinely different jobs. Accent colors differ per type (turmeric / olive / larkspur).

**Decisions:**

| Element | Decision | Rationale |
|---------|----------|-----------|
| Actions on all entity types | **Standardize (D5)** | Actions appear as main chapter on all three entity types, not just accounts. |
| Actions on person detail | **1:1 meeting heuristic (open question)** | Show actions from 1:1 meetings with that person — where they serve as the relationship, not just an attendee. Actions from group meetings or where they're just responsible don't belong here. Needs validation. |
| TheWork (accounts) | **Keep as main chapter** | Accounts are relationship-managed. Actions are central to the daily work. |
| Actions on projects | **Promote from appendix to main chapter** | Consistent with D5. Projects are milestone-tracked — actions are relevant, just secondary to momentum/horizon. |
| Resolution Keywords | **Remove from UI entirely** | Work silently. Neither user understands what "resolution" means in this context. Entity matching should be invisible. |
| Value Delivered (account appendix) | **Cut** | Available via MCP to Claude Desktop for deep queries. Doesn't need to occupy page space. |
| Portfolio Summary (account appendix) | **Cut** | Same reasoning. Historical/analytical — MCP territory. |
| Company Context (hero + appendix) | **Consolidate to one location** | Currently appears in both hero and appendix. One place only — appendix is the more appropriate home for reference material. |
| Lifecycle events (account appendix) | **Keep** | Real ongoing job: recording expansion, renewal, churn events. |
| Notes (all appendices) | **Keep** | Editable, entity-specific, no substitute. |
| Files (all appendices) | **Keep** | Direct file access for context-sensitive reference. |
| Business Units (account appendix) | **Keep** | Parent/child account hierarchy is account-specific and needed for CSM book management. |
| Field-editing drawers | **Eliminate (I343)** | Inline editing everywhere data is displayed. AccountFieldsDrawer, ProjectFieldsDrawer → delete as I343 lands. |
| TeamManagementDrawer | **Exception — keep as modal** | Adding team members involves search + create. But viewing/removing stays inline in StakeholderGallery. |
| Watch List (risks/wins/unknowns) | **Keep** | One of the most useful chapters on any entity page. Clear framework, actionable. |
| WatchListPrograms (account) | **Move out of WatchList** | Active initiatives feel more like TheWork than watch items. Consider own section or fold into TheWork. |
| Meeting readiness callout (on entity detail) | **Cut** | Meeting Detail now owns meeting readiness. Entity detail links to the meeting; it doesn't replicate prep. |
| "Build Intelligence" button | **Rename** | System vocabulary. "Check for updates" or just "Refresh." |
| Intelligence timestamps in heroes | **Rename** | "Account Intelligence" → remove label or just show timestamp. |

**Structure per entity type:**

```
Account Detail
  Hero — name, assessment lede, health/lifecycle badges
  Vitals — ARR, health, lifecycle, renewal, NPS, meeting frequency
  State of Play — what's working / where it's struggling
  The Room — stakeholders with engagement, your team
  Watch List — risks / wins / unknowns
  The Work — upcoming meetings (link to meeting detail) + actions (main chapter)
  The Record — unified timeline (meetings, emails, captures)
  Appendix — lifecycle events, company context, BUs, notes, files

Project Detail
  Hero — name, assessment lede, status, owner
  Vitals — status, days to target, milestone progress, meeting frequency
  Trajectory — momentum / headwinds, velocity
  The Horizon — next milestone, timeline risk, decisions pending
  The Landscape — risks / wins / unknowns + milestones
  The Team — stakeholders
  The Work — actions (promoted from appendix, main chapter)
  The Record — unified timeline
  Appendix — milestones (full list), description, notes, files

Person Detail
  Hero — name, assessment lede, relationship/temperature badge, email, social
  Vitals — temperature, meeting frequency with trend, last met, meeting count
  The Dynamic / The Rhythm — relationship strengths / gaps (adaptive framing)
  The Network — connected accounts and projects
  The Landscape — risks / wins / unknowns
  The Work — actions (new, from 1:1 meetings — open question on exact heuristic)
  The Record — unified timeline
  Appendix — profile fields, notes, files, duplicate detection
```

**Open question:** Person detail actions — 1:1 meeting heuristic is the right direction but needs validation. If a person serves as the primary external relationship in a meeting (regardless of attendee count), their actions belong here. If they're just one of many attendees, it's the account's job.

**Role note:** The account entity is the organizing principle for CSM roles. For project-based roles, projects serve the same function. Role presets (ADR-0079) should reflect this emphasis shift — not a different surface, just different default prominence.

---

## Cross-Cutting Decisions

### Information Ownership

Each surface owns its content. Others surface a preview and link.

| Owner | Owns | Others do |
|-------|------|-----------|
| Meeting Detail | All meeting intelligence (context, risks, room, plan) | Surface a preview, link to it |
| Actions Page | Complete commitment inventory, full CRUD | Surface today's relevant subset, link |
| Entity Detail | All entity intelligence, context, history | Surface what's relevant to a meeting, link |
| Daily Briefing | Today's situational awareness | Pulls from all sources, creates pull toward detail pages |
| Weekly Forecast | Week shape, strategy, and ±7-day meeting intelligence | Does not duplicate daily or actions |

### What's New vs. ADR-0084

ADR-0084 was written before this session. The following decisions supersede or extend it:

| Topic | ADR-0084 | Updated decision |
|-------|----------|-----------------|
| Lead Story | Compact: narrative + bridge link | **Replaced entirely by "Up Next" in schedule** — featured meeting concept retired |
| Meeting Detail appendix | Pruned to 5 sub-sections | **Cut entirely** — entity detail owns it all |
| Weekly Forecast: Open Time | Keep (minus prep buttons) | **Cut** — too deterministic, calendar tool territory |
| Weekly Forecast: Meetings chapter | Cut | **Replaced by ±7-day meeting intelligence timeline (I330)** |
| Actions: display model | Temporal grouping (overdue/today/upcoming) | **Meeting-centric primary view** + auto-expiry at 30 days |
| Entity detail: value delivered | Not mentioned | **Cut** — MCP territory |
| Entity detail: portfolio summary | Not mentioned | **Cut** — MCP territory |
| Entity detail: resolution keywords | Rename to "Matching Keywords" | **Remove from UI entirely** — work silently |
| Entity detail: meeting readiness callout | Not addressed | **Cut** — Meeting Detail owns it |
| Entity detail: WatchListPrograms | Not addressed | **Move out of WatchList** — fold into TheWork or own section |

### Dead Code (confirmed cuts)

- `AppSidebar.tsx` + sidebar shell branch — all routes use magazine shell
- `WatchItem.tsx` — replaced by WatchItemRow inside WatchList
- `ActionList.tsx` + `ActionItem.tsx` — legacy, onboarding tour only
- `EmailList.tsx` — not imported anywhere

### Shared Component Consolidation (confirmed)

- Four action row implementations → one shared `ActionRow` with density variants
- Two proposed action triage UIs → one shared `ProposedActionRow` with `compact` prop
- Three meeting row implementations → shared `MeetingRow` base; `BriefingMeetingCard` stays complex
- Three `handleUpdateIntelField` callbacks → shared `useIntelligenceFieldUpdate` hook
- Two keywords implementations → shared `ResolutionKeywords` component (now internal-only, not rendered)

### Open Questions

1. **Person detail actions heuristic**: 1:1 meeting linkage is the right direction. Define "primary external relationship" more precisely before implementation.
2. **Auto-expiry notification**: Does the user get any signal when an action expires, or is it fully silent? Silent is cleaner but could occasionally surprise.
3. **Emails nav**: Briefing is the sole entry point to Emails by design. Revisit if mid-day email access becomes a real need.
