# I342 Actions Page Audit — JTBD Definition + Element Inventory

**Date:** 2026-02-18
**Surface:** Actions (`/actions`, `/actions/$actionId`)
**Auditor:** actions-auditor agent

---

## Phase 1: JTBD Definition

### Surface-Level JTBD

**Situation:** The user has accumulated commitments — from meetings, emails, AI suggestions, and their own initiative — across multiple days and accounts. They need a single place to see, triage, and work through these commitments.

**Motivation:** Reduce anxiety about dropped balls. Know what they owe, to whom, and when. Separate "decide if this matters" (triage) from "get this done" (execution). See progress accumulate.

**Desired outcome:** The user closes the Actions page feeling confident nothing is slipping. Either they've triaged new suggestions, or they've checked off completed work, or they've confirmed that nothing needs urgent attention. "Done" = clarity about obligations.

**Boundary:** This surface manages the **commitment inventory**. It does NOT manage meeting preparation (that's the daily/meeting briefing), relationship intelligence (entity detail), or weekly planning (week forecast). It's a queue you work through AND a dashboard you check — the "proposed" tab is triage, the "pending" tab is execution, the "completed" tab is satisfaction.

### Is it a queue or a dashboard?

**Both, depending on the tab.** The surface conflates two workflows:

1. **Triage queue** ("proposed" tab): AI has suggested actions. User reviews, accepts, or dismisses. This is a finite, clearable inbox. The JTBD is "decide which of these are real commitments."

2. **Execution queue** ("pending" tab): Accepted actions awaiting completion. This is a checklist. The JTBD is "work through my to-dos and check them off."

3. **Status dashboard** ("completed" / "waiting" / "all" tabs): Review historical actions for satisfaction or context. The JTBD is "confirm progress" or "find a past action."

The "proposed -> accepted" triage and "pending -> done" execution are related but distinct jobs. The current UI collapses them into a single surface with tab-switching. This is reasonable at current scale but could create confusion if proposed volume grows — triage fatigue competing with execution focus.

### Section-Level JTBDs

| Section | JTBD | One-Sentence Job |
|---------|------|-----------------|
| Page header (title + item count) | Orient the user: "I'm looking at my full action inventory" | Ground the user in the surface identity and scale |
| Status filter tabs | Switch between triage (proposed), execution (pending), review (completed), and waiting modes | Let the user focus on one workflow at a time |
| Priority filter tabs | Narrow by urgency within any status view | Focus on what matters most when the list is long |
| Search input | Find a specific action by keyword | Locate a known action without scanning |
| Proposed count badge | Signal that triage work exists | Create pull toward the triage queue |
| FolioBar stats (N to review, N pending, N overdue) | Ambient awareness of action health | Quick glance without entering the surface |
| "+ Add" button (FolioBar) | Capture a new commitment from anywhere on this page | Manual action creation entry point |
| Action create form | Structured capture of a new action with metadata | Turn a thought into a tracked commitment |
| Action rows (pending/completed/all) | Display individual actions with status, context, and urgency | Each row is a unit of work to check off or review |
| Proposed action rows | Display AI suggestions with accept/reject controls | Each row is a decision: "is this real?" |
| End mark ("That's everything") | Signal completeness of the current view | Confirm the user has seen everything |

### When does the user reach for this?

1. **Morning triage:** Open the proposed tab, accept or dismiss AI suggestions from overnight processing.
2. **Mid-day execution:** Open the pending tab, check off completed items.
3. **Pre-meeting check:** Search for actions related to an upcoming meeting's account (though this is better served by the meeting briefing).
4. **End-of-day review:** Scan completed tab to feel accomplished. Check pending for anything missed.
5. **Ad-hoc capture:** Hit "+ Add" to log a new action from a phone call, Slack message, or thought.

### What does "done" look like?

- **Triage done:** Proposed tab shows "All clear — No AI suggestions waiting for review."
- **Execution done:** Pending tab shows empty state ("All clear" / personality-dependent copy).
- **Review done:** User has visually scanned the list and confirmed nothing is urgent.

---

## Phase 2: Element Inventory

### Key Files

| File | Role |
|------|------|
| `src/pages/ActionsPage.tsx` | Main surface — page header, filters, action rows, create form |
| `src/pages/ActionDetailPage.tsx` | Individual action detail — editable fields, status toggle, context, references |
| `src/hooks/useActions.ts` | Data hook — loads actions from SQLite, applies filters, provides CRUD |
| `src/hooks/useProposedActions.ts` | Data hook — loads proposed (AI-suggested) actions, accept/reject |
| `src/components/dashboard/ActionList.tsx` | Dashboard action list (used in onboarding tour, NOT in daily briefing) |
| `src/components/dashboard/ActionItem.tsx` | Dashboard action row (used by ActionList, shadcn/Tailwind style) |
| `src/components/dashboard/DailyBriefing.tsx` | Daily briefing — renders actions inline (Priorities section, Review section) |
| `src/pages/MeetingDetailPage.tsx` | Meeting detail — renders actions in "Open Items" section and "Outcomes" section |
| `src/pages/WeekPage.tsx` | Week forecast — renders action summary (overdue, due this week, top 3 feasible) |

### Types

| Type | File | Description |
|------|------|-------------|
| `DbAction` | `src/types/index.ts:118` | SQLite-backed action (id, title, priority, status, dueDate, accountId, sourceLabel, context, etc.) |
| `Action` | `src/types/index.ts:101` | Lighter action type (used by dashboard/onboarding components, has `isOverdue` flag) |
| `ActionDetail` | `src/types/index.ts:1275` | Extends DbAction with `accountName` and `sourceMeetingTitle` for the detail page |
| `CreateActionParams` | `src/hooks/useActions.ts:8` | Params for creating a new action |

---

### Element-by-Element Inventory

#### A. Page Header

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 1 | **Page title "Actions"** | `ActionsPage.tsx:172` | Serif h1, 36px | Identity — tells user which surface they're on | FolioBar also shows "Actions" label | No — essential orientation |
| 2 | **Item count** | `ActionsPage.tsx:181` | Mono text, e.g. "12 items" | Scale signal — how many actions in current filtered view | FolioBar shows different counts (pending, overdue) | Yes — nice-to-have, not essential for the job |
| 3 | **Section rule** | `ActionsPage.tsx:186` | 1px heavy horizontal rule | Visual separator between header and filters | N/A | No — editorial design language requires it |

#### B. Status Filter Tabs

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 4 | **"proposed" tab** | `ActionsPage.tsx:191-231` | Mono uppercase, underlined when active | Switch to triage queue | Daily briefing has inline "Review" section for proposed | No — primary triage entry point |
| 5 | **Proposed count badge** | `ActionsPage.tsx:214-229` | Turmeric pill with count (e.g. "3") | Signal triage work exists without clicking the tab | Daily briefing shows proposed count. FolioBar shows "N to review" | Probably — the FolioBar already signals this |
| 6 | **"pending" tab** | `ActionsPage.tsx:191-231` | Same style as proposed | Switch to execution queue (default view) | N/A | No — this is the primary view |
| 7 | **"completed" tab** | `ActionsPage.tsx:191-231` | Same style | Switch to review/satisfaction view | N/A | Debatable — users may rarely check this |
| 8 | **"waiting" tab** | `ActionsPage.tsx:191-231` | Same style | Show actions blocked on others | N/A | Debatable — low volume expected, could be a filter within pending |
| 9 | **"all" tab** | `ActionsPage.tsx:191-231` | Same style | Show everything regardless of status | N/A | Probably — power user feature, rarely needed |

#### C. Priority Filter Tabs

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 10 | **"all" priority tab** | `ActionsPage.tsx:235-258` | Mono uppercase, underlined when active | Show all priorities | N/A | Yes — this is the default |
| 11 | **"P1" / "P2" / "P3" tabs** | `ActionsPage.tsx:235-258` | Same style | Filter to specific priority level | N/A | Probably — useful when list is long, but adds visual weight |

#### D. Search

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 12 | **Search input** | `ActionsPage.tsx:262-278` | Full-width text input with command icon placeholder | Find specific action by keyword in title, account, or context | CommandMenu (Cmd+K) also provides global search | Maybe — useful for 20+ actions, rarely needed for <10 |

#### E. Create Form

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 13 | **"+ Add" button** (FolioBar) | `ActionsPage.tsx:86-103` | Terracotta bordered button, mono uppercase | Toggle the create form open | N/A | No — primary entry point for manual creation |
| 14 | **Title input** | `ActionsPage.tsx:680-700` | Serif 17px input with placeholder "What needs to be done?" | Capture the action's name | N/A | No — required field |
| 15 | **"+ details" toggle** | `ActionsPage.tsx:706-719` | Mono text link | Expand optional metadata fields | N/A | No — progressive disclosure is good UX |
| 16 | **PriorityPicker** | `ActionsPage.tsx:775` | Priority selector (P1/P2/P3) via `@/components/ui/priority-picker` | Set urgency level | Used on ActionDetailPage too | No — reasonable metadata |
| 17 | **DatePicker** | `ActionsPage.tsx:776-779` | Date input for due date | Set deadline | Used on ActionDetailPage too | No — reasonable metadata |
| 18 | **EntityPicker** | `ActionsPage.tsx:781-785` | Account/project linker via `@/components/ui/entity-picker` | Link action to an account or project | Used on ActionDetailPage too | Debatable — useful for power users, adds complexity for casual use |
| 19 | **Source input** | `ActionsPage.tsx:788-805` | Text input with placeholder "Source (e.g., Slack, call with Jane)" | Record where the action came from | N/A | Yes — rarely filled for manual actions |
| 20 | **Context textarea** | `ActionsPage.tsx:807-825` | Multiline text input for additional context | Add notes or background | N/A | Yes — nice-to-have, not essential |
| 21 | **"Create" button** | `ActionsPage.tsx:828-845` | Terracotta bordered button | Submit the new action | N/A | No — required to complete creation |
| 22 | **"Cancel" button** | `ActionsPage.tsx:846-859` | Plain text button | Abort creation | N/A | No — escape hatch |

#### F. Action Rows (Pending / Completed / All)

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 23 | **Checkbox (circle)** | `ActionsPage.tsx:410-432` | 20px circle, green fill when completed, terracotta border when overdue | Toggle complete/reopen | Daily briefing has checkboxes. Meeting detail has checkboxes. | No — the core interaction |
| 24 | **Action title** (as Link) | `ActionsPage.tsx:436-449` | Serif 17px text, links to `/actions/$actionId` | Identify the action; navigate to detail | Daily briefing shows titles. Meeting detail shows titles. Week forecast shows titles. | No — essential |
| 25 | **Context line** (due date + account + source) | `ActionsPage.tsx:450-464` | Sans 13px secondary text, dot-separated | Provide urgency, relationship, and provenance context | Daily briefing shows due info. Meeting detail shows due date. | Partially — due date is essential, account/source are nice-to-have |
| 26 | **Overdue indicator** (days overdue text + terracotta color) | `ActionsPage.tsx:384-388` | e.g. "3 days overdue" in terracotta | Urgency signal | Daily briefing has "Overdue" group label. Week forecast has overdue grouping. | No — critical urgency signal |
| 27 | **Priority badge** | `ActionsPage.tsx:468-484` | Mono 11px, color-coded (P1=terracotta, P2=turmeric, P3=tertiary) | Show urgency level at a glance | Daily briefing shows priority. Meeting detail shows priority. | Probably not — compact and useful |
| 28 | **Row separator** (bottom border) | `ActionsPage.tsx:404` | 1px light rule between rows | Visual separation | N/A | No — editorial design convention |

#### G. Proposed Action Rows

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 29 | **Dashed left border** (turmeric) | `ActionsPage.tsx:516` | 2px dashed turmeric left border | Visual differentiation from accepted actions | Daily briefing uses same dashed border. Meeting detail uses same pattern. | No — important visual cue for "not yet committed" |
| 30 | **"AI Suggested" label** | `ActionsPage.tsx:523-534` | Mono 10px uppercase, turmeric | Identify provenance — this came from the system, not the user | Daily briefing does NOT show this label (renders inline). Meeting detail shows "proposed" via dashed border. | Debatable — the dashed border already signals this. Per ADR-0083, "Suggested" is the product term. |
| 31 | **Action title** (not linked) | `ActionsPage.tsx:551-561` | Serif 17px text, NOT a link to detail | Identify the suggested action | Same title appears on daily briefing and meeting detail | No — essential |
| 32 | **Priority badge** (in header row) | `ActionsPage.tsx:535-549` | Mono 11px, color-coded | Show suggested priority | Same on daily briefing | Debatable — priority of a suggestion is less meaningful before acceptance |
| 33 | **Context line** (source + account) | `ActionsPage.tsx:562-574` | Sans 13px, dot-separated | Show where the suggestion came from | Daily briefing shows sourceLabel | Partially — source is useful for triage decisions |
| 34 | **Accept button** (checkmark) | `ActionsPage.tsx:579-598` | 28x28 bordered button with sage checkmark SVG | Accept the suggestion into the action queue | Daily briefing has accept button. Meeting detail has accept button. | No — core triage interaction |
| 35 | **Reject button** (X) | `ActionsPage.tsx:599-619` | 28x28 bordered button with terracotta X SVG | Dismiss the suggestion | Daily briefing has reject button. Meeting detail has reject button. | No — core triage interaction |

#### H. Empty States

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 36 | **Proposed empty** ("All clear") | `ActionsPage.tsx:296-299` | EditorialEmpty: "All clear" / "No AI suggestions waiting for review." | Confirm triage is complete | N/A | No — important completion signal |
| 37 | **Pending/All empty** (personality-based) | `ActionsPage.tsx:314-323` | EditorialEmpty with personality-driven copy (professional/friendly/playful) | Confirm execution queue is clear | N/A | No — important completion signal |

#### I. End Mark

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 38 | **"That's everything."** | `ActionsPage.tsx:339-360` | Serif italic, centered, after heavy rule | Signal completeness — user has seen all items in current view | Daily briefing has similar end marks | No — important for finite-document feel (ADR-0077) |

#### J. FolioBar Integration

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 39 | **"Actions" folio label** | `ActionsPage.tsx:81` | Top bar label | Identify the surface in the magazine shell | Page title also says "Actions" | No — shell requires it |
| 40 | **Date text** | `ActionsPage.tsx:83` | Current date, uppercase | Temporal orientation | Every page has this | Not essential — actions aren't day-scoped |
| 41 | **Readiness stats** | `ActionsPage.tsx:69-75` | e.g. "3 to review", "12 pending", "2 overdue" | Ambient health signal | Somewhat duplicates the tab badges and item count | Probably — useful at a glance, but overlaps with page content |
| 42 | **"+ Add" button** | `ActionsPage.tsx:86-103` | Terracotta bordered button | Quick-create entry point | N/A | No — essential for manual capture |

#### K. Action Detail Page (`/actions/$actionId`)

| # | Element | Code Location | What It Renders | Job | Duplicated Elsewhere? | Could User Do Without It? |
|---|---------|---------------|-----------------|-----|-----------------------|---------------------------|
| 43 | **Status toggle circle** | `ActionDetailPage.tsx:212-233` | Circle/Check icon, clickable | Toggle complete/reopen | ActionsPage row checkbox | No — primary action |
| 44 | **Editable title** | `ActionDetailPage.tsx:237-250` | Serif 28px, click-to-edit | View and rename the action | ActionsPage shows title (read-only) | No — essential |
| 45 | **Priority pill** | `ActionDetailPage.tsx:263-278` | Mono 9px pill with color background | Show current priority | ActionsPage row badge | No — key metadata |
| 46 | **Status text** ("Open" / "Completed") | `ActionDetailPage.tsx:281-290` | Mono 10px uppercase | Show current status | ActionsPage row shows via opacity/strikethrough | Debatable — checkbox already shows this |
| 47 | **"Waiting: X" pill** | `ActionDetailPage.tsx:293-308` | Mono 9px pill, grey background | Show who is blocking this action | ActionsPage has "waiting" tab but no per-row waiting indicator | Useful — only visible when relevant |
| 48 | **"From meeting" link** | `ActionDetailPage.tsx:311-331` | Mono 9px pill, links to meeting detail | Navigate to the source meeting | ActionsPage row shows sourceLabel text | Yes — useful provenance link |
| 49 | **PriorityPicker** (editable) | `ActionDetailPage.tsx:336-339` | Interactive priority selector | Change priority | Create form has same component | No — essential for editing |
| 50 | **Context section** (editable textarea) | `ActionDetailPage.tsx:350-371` | Editable text area with "Context" label | View/edit action context and notes | ActionsPage row shows context snippet | No — this is the detail view's primary value-add |
| 51 | **Reference section** | `ActionDetailPage.tsx:375-498` | Structured key-value metadata | View/edit action metadata | N/A | No — this is where metadata lives |
| 52 | **Account link** (editable) | `ActionDetailPage.tsx:380-431` | EntityPicker or linked account name with remove button | Link/unlink action from account | Create form has EntityPicker | No — important relationship |
| 53 | **Due date** (editable calendar) | `ActionDetailPage.tsx:434-447` | Calendar popover, urgency-colored text | Set/change/clear due date | Create form has DatePicker | No — essential metadata |
| 54 | **Created date** (read-only) | `ActionDetailPage.tsx:450-453` | Formatted date text | Record of when action was created | N/A | Debatable — informational only |
| 55 | **Completed date** (read-only, conditional) | `ActionDetailPage.tsx:456-461` | Formatted date text, only shown when completed | Record of when action was finished | N/A | Debatable — informational only |
| 56 | **Source** (meeting link or editable text) | `ActionDetailPage.tsx:464-496` | Link to source meeting OR editable text for manual source | Provenance tracking | ActionsPage row shows sourceLabel | No — useful context |
| 57 | **"Mark Complete" / "Reopen" button** | `ActionDetailPage.tsx:516-536` | Mono 11px bordered button | Complete or reopen the action | Circle toggle at top does the same | Debatable — redundant with the circle toggle |
| 58 | **Save status indicator** | `ActionDetailPage.tsx:502-515` | "Saving..." / "Saved" text | Confirm edits persisted | N/A | No — essential feedback for inline editing |

---

### Cross-Surface Duplication Map

Actions appear on **four surfaces** beyond the Actions page itself. Here's what overlaps:

| Capability | Actions Page | Daily Briefing | Meeting Detail | Week Forecast |
|-----------|-------------|----------------|----------------|---------------|
| **View pending actions** | Full list, filtered | Priorities section (overdue/today/later), max ~13 | Open Items section | Action summary (overdue + due this week) |
| **Complete an action** | Checkbox toggle | Checkbox toggle | Checkbox toggle | No |
| **View proposed actions** | Full list, dedicated tab | "Review" section, max 5 | Outcomes section (per-meeting) | No |
| **Accept proposed action** | Accept button | Accept button | Accept button | No |
| **Reject/dismiss proposed** | Reject button | Reject button | Reject button | No |
| **Create action** | "+ Add" form | No | No | No |
| **Edit action** | Via detail page link | No | No | No |
| **Search actions** | Search input | No | No | No |
| **Filter by status** | Status tabs | Implicit (pending only) | Implicit (meeting-scoped) | Implicit (overdue + this week) |
| **Filter by priority** | Priority tabs | Implicit (P1/P2 first) | No | No |
| **See action detail** | Link to `/actions/$actionId` | Link to `/actions/$actionId` | No (inline only) | Link to `/actions/$actionId` |
| **See overdue count** | FolioBar stat | Readiness stat | No | Overdue count in summary |

**Key observations about duplication:**

1. **Proposed action triage is triplicated.** Accept/reject appears on the Actions page "proposed" tab, the Daily Briefing "Review" section, and the Meeting Detail "Outcomes" section. The Daily Briefing caps at 5 and links to Actions for more. Meeting Detail shows only actions sourced from that meeting. This is intentional -- users should be able to triage from wherever they encounter suggestions.

2. **Action completion is triplicated.** Checkboxes appear on the Actions page, Daily Briefing, and Meeting Detail. Again intentional -- users should be able to check off work wherever they see it.

3. **The Actions page is the only surface that allows creation, editing, filtering, and searching.** Other surfaces are consumption-oriented views. The Actions page is the management surface.

4. **Two component families exist:**
   - **Editorial inline components** (ActionsPage, ActionDetailPage, DailyBriefing) — hand-styled with CSS-in-JS, editorial design tokens
   - **Dashboard/shadcn components** (ActionItem.tsx, ActionList.tsx) — Tailwind/shadcn style, used only in onboarding tour

   The dashboard components (`ActionList`, `ActionItem`) are **legacy** — they use the old `Action` type and shadcn/Tailwind styling. They're only imported by the onboarding `DashboardTour`. The main app surfaces all use editorial-styled inline components.

5. **The MeetingDetailPage has its own `ActionItem` and `OutcomeActionRow` components** defined locally (not imported from shared components). These are meeting-scoped variants that include priority cycling and contextual display. This is not shared code — three different files render "action rows" with three different implementations.

---

### Vocabulary Audit (per ADR-0083)

| Current UI String | Product Term (ADR-0083) | Location | Status |
|-------------------|------------------------|----------|--------|
| "proposed" (status tab) | **Suggested** | `ActionsPage.tsx:20` | Needs update |
| "AI Suggested" (label on proposed rows) | **Suggested** (just "Suggested", drop "AI") | `ActionsPage.tsx:533` | Mostly correct, but consider dropping "AI" per chief-of-staff voice |
| "Reject" (button title) | **Dismiss** | `ActionsPage.tsx:601` title attr | Needs update |
| "Intelligence Report" (MeetingDetail folio label) | **Briefing** or **Meeting Briefing** | `MeetingDetailPage.tsx:325` | Needs update (not actions-specific but visible from action context) |
| "Open Items" (MeetingDetail section) | Consider "Actions" or "To-dos" | `MeetingDetailPage.tsx:930` | Ambiguous — "open items" is vague |
| "No AI suggestions waiting for review" (empty state) | Could be "No suggestions to review" (remove "AI") | `ActionsPage.tsx:298` | Minor refinement |

---

### Structural Observations

1. **The page has no temporal grouping.** Actions are a flat list sorted by whatever the backend returns. The Daily Briefing groups by urgency (overdue / today / later). The Week Forecast groups by overdue vs. due-this-week. The Actions page itself has no grouping — just filters. This means a user with 30 pending actions sees them as one undifferentiated list. The priority filter helps, but temporal urgency grouping (like the briefing does) would serve the execution job better.

2. **The "waiting" status is undercooked.** There's a tab for "waiting" status, and the ActionDetail page has a "Waiting: X" pill, but the `DbAction` type's `waitingOn` field is a plain string. There's no structured workflow for moving actions into waiting status from the list view — you'd have to open the detail page. The tab exists but the workflow to populate it is thin.

3. **No bulk operations.** No "select all" / "complete selected" / "dismiss all proposed." If 15 AI suggestions queue up overnight, the user must accept/reject them one by one. This could create triage fatigue.

4. **The search is local-only.** `useActions.ts:190-198` filters by title, accountId, and context — not by account name, source label, priority, or status. Searching "Acme" won't find actions where `accountName` is "Acme" because the filter checks `accountId` (an opaque ID). Searching "P1" won't filter by priority because priority isn't a search field.

5. **Default status filter is "pending", not "proposed."** `useActions.ts:46` defaults `statusFilter` to `"pending"`. This means new users land on the execution queue, not the triage queue. If proposed actions exist, the only signal is the small count badge on the "proposed" tab and the FolioBar "N to review" stat.

6. **The `ActionDetail` page has redundant status toggle.** Both the circle icon at the top (line 212) and the "Mark Complete" button at the bottom (line 516) do the same thing. This is belt-and-suspenders but adds unnecessary UI weight.

7. **Three separate action row implementations exist:**
   - `ActionsPage.tsx:ActionRow` (editorial inline, for pending/completed/all)
   - `ActionsPage.tsx:ProposedActionRow` (editorial inline, for proposed tab)
   - `MeetingDetailPage.tsx:OutcomeActionRow` (editorial inline, for meeting outcomes)
   - `src/components/dashboard/ActionItem.tsx` (shadcn/Tailwind, legacy — onboarding only)

   None of these share code. Each re-implements checkbox, title, priority badge, and context display with slight variations. A shared `ActionRow` editorial component would reduce maintenance surface.

8. **Date on FolioBar is irrelevant.** The Actions page shows `folioDateText: formattedDate` (today's date). But actions are not day-scoped — they span weeks/months. The date adds no information. Other surfaces (Daily Briefing = today, Week Forecast = this week) have temporal relevance. Actions does not.
