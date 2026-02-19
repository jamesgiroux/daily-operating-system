# I342 Phase 3: Decision Checklist

**Date:** 2026-02-18
**Input:** Six surface audits from Phase 1-2 (`.docs/research/i342-*.md`)
**Purpose:** Every item needs a verdict. Mark each: **YES** (do it), **NO** (leave it), or **DEFER** (not now).

---

## A. Cuts — Things to Remove

### A1. Weekly Forecast: "Your Meetings" chapter
**What:** Multi-day meeting list organized by day with prep status badges and account subtitles.
**Why cut:** Thin list duplicating the daily briefing's Schedule section with less capability. No actions possible here — user must navigate to meeting detail anyway. 10 of 33 weekly elements are duplicated elsewhere.
**Impact:** Removes ~120 lines from WeekPage.tsx. Users who want to see meetings for future days use the daily briefing on that day or meeting detail directly.
**Verdict:** yes

### A2. Weekly Forecast: "Commitments" chapter
**What:** Read-only list of overdue and due-this-week actions with summary line.
**Why cut:** Same actions appear on daily Priorities (richer, with capacity awareness and checkboxes) and Actions page (full CRUD). This is the least-capable version on any surface.
**Impact:** Removes ~130 lines from WeekPage.tsx. No capability loss — same data lives on two better surfaces.
**Verdict:** yes

### A3. Weekly Forecast: "Prefill Prep" and "Draft Agenda" buttons
**What:** Action buttons on deep work block cards that trigger meeting prep prefill and agenda email draft.
**Why cut:** Meeting-prep actions belong on the meeting detail page, not the weekly planning surface. The weekly's job is "see the shape of your week," not "prep for meetings."
**Impact:** Removes buttons from Open Time cards. Move these affordances to meeting detail if not already there.
**Verdict:** yes

### A4. Daily Briefing: "Later This Week" action group
**What:** Third group in Priorities section showing max 3 actions due later this week (after overdue and due-today).
**Why cut:** The daily briefing's job is TODAY. "Later This Week" items aren't actionable today and leak into weekly territory. If the weekly forecast exists, this is its content.
**Impact:** Priorities section becomes: Overdue + Due Today only. Users see later actions on Actions page or weekly forecast.
**Verdict:** yes

### A5. Meeting Detail: Second FinisMarker
**What:** Two FinisMarkers on the same page — one mid-page after "Your Plan" (signaling "you're briefed"), one at the very end after Deep Dive.
**Why cut:** Two "you're done" signals is contradictory. The mid-page one after Your Plan is the real completion signal. The Deep Dive is optional reference — it doesn't need its own finis.
**Impact:** Remove the final FinisMarker (~3 lines). Keep the mid-page one.
**Verdict:** yes

### A6. Meeting Detail: Appendix sub-sections that duplicate entity detail
**What:** Strategic Programs, Current State, Key Principles in the meeting detail appendix.
**Why cut:** These are entity-level data that live richer on account/project detail pages. The meeting briefing's job is "prepare for this meeting," not "replicate the account dossier."
**Impact:** Appendix shrinks from 9 to 6 sub-sections. Full Intelligence Summary, Since Last Meeting, Full Context, Questions to Surface, Extended Stakeholder Map, and References remain.
**Verdict:** yes

### A7. Meeting Detail: Duplicate transcript attachment button
**What:** "Attach Transcript" appears in both the folio bar (always visible) and as a prominent dashed-border CTA in the page body for past meetings.
**Why cut one:** Two buttons for the same action. The body-level version is more contextual and discoverable for past meetings.
**Impact:** Remove the folio bar transcript button for past meetings (keep body CTA), or vice versa. Keep one.
**Verdict:** folio bar (which one to keep: body / folio bar / both)

### A8. Dead code cleanup
**What:**
- `AppSidebar.tsx` + sidebar shell branch in router (dead — all routes use magazine shell)
- `WatchItem.tsx` editorial component (dead — replaced by `WatchItemRow` inside `WatchList.tsx`)
- `ActionList.tsx` + `ActionItem.tsx` in dashboard/ (legacy — only onboarding tour)
- `EmailList.tsx` in dashboard/ (not imported anywhere)
- `entityMode` handling in dead sidebar code

**Why cut:** Dead code is cognitive overhead. If it's not reachable, delete it.
**Impact:** ~500-800 lines removed. Zero functional change.
**Verdict:** yes

---

## B. Moves — Things on the Wrong Surface

### B1. Daily Briefing: EntityPicker (add/remove) on entity chips
**What:** `MeetingEntityChips` on the daily briefing include an EntityPicker ("Link entity...") allowing users to add/remove entity links from the reading surface.
**Where it belongs:** Meeting detail page (which already has the same component).
**Why move:** The daily briefing is a reading surface. Editing affordances (add/remove entities) break the read-top-to-bottom flow. Keep the chips as read-only display on the briefing; move the picker to meeting detail only.
**Impact:** Entity chips still appear on briefing (read-only). Correction happens on meeting detail.
**Verdict:** No

### B2. Weekly Forecast: AgendaDraftDialog
**What:** Modal dialog triggered from deep work block cards for drafting agenda emails.
**Where it belongs:** Meeting detail page.
**Why move:** Agenda drafting is meeting-prep work, not weekly planning work.
**Impact:** Already exists on meeting detail. Just remove the trigger from WeekPage.
**Verdict:** No

### B3. Meeting Detail: "Questions to Surface" in appendix
**What:** AI-suggested questions in the appendix, separate from the "Your Plan" agenda editor.
**Where it belongs:** Merged into "Your Plan" as proposed agenda items, or removed.
**Why move:** Two "what to discuss" sections on the same page (agenda editor + questions list) is confusing. If the questions are good, they should be proposed agenda items.
**Impact:** Appendix loses one section. Your Plan potentially gains better AI-proposed items.
**Verdict:** cut entirely (merge into Your Plan / keep separate / cut entirely)

---

## C. Merges — Duplicated Implementations to Consolidate

### C1. Four action row implementations → one shared component
**What:** `DailyBriefing:PrioritizedActionItem`, `WeekPage:commitments`, `ActionsPage:ActionRow`, `TheWork:ActionRow` — four independent implementations of the same concept (title + due + context + optional checkbox).
**Proposed:** Extract a shared `ActionRow` editorial component with density variants (`compact` for briefing, `full` for actions page).
**Impact:** Reduces maintenance surface. Ensures consistent styling across all surfaces. ~200 lines of duplicated code consolidated.
**Verdict:** Yes

### C2. Proposed action triage UI → one shared component
**What:** Accept/reject UI duplicated between DailyBriefing "Review" section and ActionsPage "Proposed" tab. Near-identical code, independent implementations.
**Proposed:** Extract shared `ProposedActionRow` component with `compact` prop (24px buttons for briefing, 28px for actions page).
**Impact:** ~80 lines consolidated. Consistent triage experience everywhere.
**Verdict:** yes

### C3. Three meeting row implementations → shared component(s)
**What:** `BriefingMeetingCard` (elaborate), `WeekPage` inline rows (compact), `TheWork` inline rows (minimal).
**Proposed:** This is harder — `BriefingMeetingCard` has expansion panels, temporal states, action checklists that others don't. Consider: extract a base `MeetingRow` for compact use (WeekPage, TheWork), keep `BriefingMeetingCard` as the elaborated version that extends it.
**Impact:** WeekPage and TheWork share a base component. BriefingMeetingCard stays complex.
**Verdict:** yes

### C4. Intelligence field update pattern → shared hook
**What:** All three entity detail pages copy-paste `handleUpdateIntelField` callback (~20 lines each). Same pattern, same logic.
**Proposed:** Extract `useIntelligenceFieldUpdate` hook.
**Impact:** ~60 lines consolidated. DRY improvement.
**Verdict:** yes

### C5. Keywords pattern → shared component
**What:** Accounts and projects both implement identical keyword parsing, rendering, and removal logic (~120 lines each).
**Proposed:** Extract shared `ResolutionKeywords` component.
**Impact:** ~120 lines consolidated.
**Verdict:** yes

---

## D. Rethinks — Things That Need Redesign

### D1. Daily Briefing: Lead Story scope
**What:** The featured meeting section renders narrative, key people, prep grid, action checklist, entity chips, and a bridge link. This is 60-70% of the meeting detail page rendered inline.
**Options:**
- **(a) Compact it:** Keep narrative + bridge link. Cut key people, prep grid, entity chips from lead story. The bridge link does its job — it pulls users to the full briefing.
- **(b) Keep it rich:** The lead story IS the value of the daily briefing for the most important meeting. Cutting it makes the briefing thinner.
- **(c) Middle ground:** Keep narrative + key people (compact, no temperature dots). Cut prep grid. Keep action checklist (directly actionable). Cut entity chips (read-only if B1 approved, but still space).
**Verdict:** a (a / b / c / other)

### D2. Daily Briefing: Featured meeting duplication in Schedule
**What:** The featured meeting appears fully in both the Lead Story AND the Schedule expansion panel. Expanding it in the schedule shows the same content the user already read above.
**Options:**
- **(a) Suppress expansion** for the featured meeting in the schedule (since lead story already covers it). Show a "See above" or disabled expand state.
- **(b) Remove lead story entirely.** Feature the meeting in-place in the schedule with a richer card than other meetings.
- **(c) Leave it.** The duplication exists but users don't notice because they rarely expand the featured meeting in the schedule after reading the lead story.
**Verdict:** Featured Meeting should be a unique intelligence narrative not a rehash of what exists elsewhere. it should be BRIEF, succinct. the schedule expansion panel should show the information it shows which is materially different than the featured meeting.

### D3. Key People temperature dots
**What:** Relationship temperature dots on key people in BriefingMeetingCard and meeting detail are hardcoded to `hot` class. They carry no real signal.
**Options:**
- **(a) Cut the dots.** Name + role is sufficient until real temperature data exists.
- **(b) Feed real data.** Wire the dots to actual signal bus data (requires I306/I307 signal infrastructure).
- **(c) Defer.** Remove the hardcoded class so no dot shows. Add back when real data arrives.
**Verdict:** c (a / b / c)

### D4. Meeting Detail: Folio bar action density
**What:** Up to 4 buttons simultaneously: Prefill, Draft Agenda, Transcript, Refresh.
**Options:**
- **(a) Keep Refresh only in folio bar.** Move Prefill, Draft Agenda, Transcript into the page body where they have more context.
- **(b) Keep Refresh + Transcript in folio bar.** Move Prefill and Draft Agenda into the page body.
- **(c) Leave it.** Power users like having everything accessible.
**Verdict:** This is all going to change with 0.13.0 anyway as there won't be refresh, transcript is fine to keep and draft agenda too, prefill can be removed (a / b / c)

### D5. Entity detail: Actions inconsistency across entity types
**What:** Actions are a main chapter on accounts (TheWork), in the appendix on projects, and absent on people.
**Options:**
- **(a) Standardize:** Actions appear as a main chapter on all entity types that have them. People gain an actions section.
- **(b) Keep the difference.** Accounts are relationship-managed (actions are central). Projects are milestone-tracked (actions are secondary). People are context-looked-up (actions aren't the job).
- **(c) Remove from entity detail entirely.** Actions page owns actions. Entity detail links to filtered Actions page view.
**Verdict:** a (a / b / c)

### D6. Meeting Detail: "Before This Meeting" overlap
**What:** Two different "before this meeting" lists from different data sources:
1. Intelligence-derived readiness items (entity readiness checklist on meeting detail)
2. DB-backed action items (MeetingActionChecklist on daily briefing expansion)
Both answer "what should I do before this meeting?" but pull from different sources and appear on different surfaces.
**Options:**
- **(a) Merge into one.** Meeting detail shows both intelligence readiness items and tracked actions in a single "Before This Meeting" section.
- **(b) Keep separate.** Intelligence items are context; actions are commitments. Different things.
- **(c) Show actions only.** Intelligence readiness items become proposed actions that the user triages. One list.
**Verdict:** a but these should really be derived from intelligence and signals (a / b / c)

### D7. Actions Page: Default tab and temporal grouping
**What:** Default filter is "pending" (execution queue), not "proposed" (triage queue). The list has no temporal grouping (flat list vs. daily briefing's overdue/today/later groups).
**Options:**
- **(a) Default to "proposed" when suggestions exist.** Otherwise "pending." Add temporal grouping (overdue/today/upcoming) to the pending tab.
- **(b) Keep "pending" as default.** Add temporal grouping. Rely on the badge count to pull users toward proposed.
- **(c) Smart default.** If proposed count > 0, land on proposed. Otherwise pending. Add grouping either way.
**Verdict:** a (a / b / c)

### D8. Command Menu: Incomplete navigation
**What:** Only covers 4 surfaces (Overview, Inbox, Calendar, Actions). Missing: People, Accounts, Projects, Settings, Emails, History. Labels inconsistent with nav ("Calendar" vs "This Week", "Overview" vs "Today").
**Options:**
- **(a) Add all navigable surfaces.** Align labels with nav (Today, This Week, Actions, People, Accounts, Projects, Settings, Emails, Inbox).
- **(b) Just fix labels.** Match nav labels. Add surfaces incrementally.
**Verdict:** a (a / b)

---

## E. Vocabulary Violations (feeds I341)

These don't need individual decisions — they all need fixing per ADR-0083. Listed for completeness:

| Current | Location | ADR-0083 Translation |
|---------|----------|---------------------|
| "Generate Briefing" | DashboardEmpty.tsx | "Prepare my day" |
| "Read full intelligence" | DailyBriefing, BriefingMeetingCard | "Read full briefing" |
| "Reject" button title | DailyBriefing review section | "Dismiss" |
| "Link entity..." placeholder | meeting-entity-chips.tsx | "Link to account or project" |
| "Intelligence Report" | MeetingDetailPage folio/kicker | "Meeting Briefing" |
| "Prep not ready yet" | MeetingDetailPage empty state | Rephrase without "prep" |
| "AI enrichment completes" | MeetingDetailPage empty state | "Building context" or invisible |
| "Intelligence builds as you meet" | MeetingDetailPage fallback | Product vocabulary |
| "Supporting Intelligence" | MeetingDetailPage section | "Background" or "Reference" |
| "Account Intelligence" / "Project Intelligence" / "Person Intelligence" | All entity heroes | "Account Insights" / just timestamp |
| "Build Intelligence" button | All entity heroes | "Check for updates" or "Refresh" |
| "Resolution Keywords" | Account + Project detail | "Matching Keywords" or remove |
| "Enriching from Clay..." | PersonHero | "Looking up profile..." |
| "proposed" tab label | ActionsPage | "Suggested" |
| "AI Suggested" label | ActionsPage proposed rows | "Suggested" (drop "AI") |
| "hasPrep" dot title | BriefingMeetingCard | Quality indicator per I329 |

---

## Decision Summary Template

Once all verdicts are in, the implementation plan writes itself:

**Cut:** [list items with YES from Section A]
**Move:** [list items with YES from Section B]
**Merge:** [list items with YES from Section C]
**Rethink:** [chosen option for each Section D item]
**Vocabulary:** All Section E items (handed to I341)
**Dead code:** Section A8 items
