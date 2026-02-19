# I342 Daily Briefing — JTBD Definition + Element Inventory

**Date:** 2026-02-18
**Surface:** Daily Briefing (route: `/`, nav label: "Today")
**Primary component:** `src/components/dashboard/DailyBriefing.tsx`
**Orchestration:** `DashboardPage` in `src/router.tsx` (lines 188-202)

---

## Phase 1: Jobs-to-Be-Done

### Surface-Level JTBD

**Situation:** It is morning (or the user is returning to their desk mid-day). They are about to start working and need to understand what their day looks like — what meetings are coming, what needs their attention, and where risk is hiding.

**Motivation:** Resolve the anxiety of "what do I need to know before my day starts?" Transition from not-working to working without the cognitive overhead of opening Calendar, Gmail, a CRM, and a task manager separately. Feel prepared, not surprised.

**Desired Outcome:** The user reads top-to-bottom and reaches the Finis marker feeling: "I know what my day looks like, I know what's important, I know where the risk is, and I know what I need to do before my first meeting." They close the surface or navigate to a detail page with confidence.

**Boundary:** This surface is for **today's situational awareness and morning preparation**. It is NOT:
- A task management interface (Actions page owns execution/triage)
- A meeting prep deep-dive (Meeting Detail page owns that)
- A relationship research tool (Entity Detail pages own that)
- A weekly planning surface (Week page owns that)
- An email client (Emails page owns reading/responding)

**"Done" signal:** The user has scanned the hero summary, noted their focus, reviewed the featured meeting's headline context, scanned the schedule for surprises, triaged any proposed actions, glanced at priorities/emails, and hit the Finis marker. Total time: 2-5 minutes. If they spend more than 5 minutes, the surface is either too dense or pulling them into work that belongs on a detail page.

### When Do They Return Mid-Day?

The user returns to the daily briefing when:
1. They finish a meeting and want to re-orient: "what's next?"
2. They've been heads-down on deep work and lost track of time — quick schedule check
3. They want to mark an action as done and see what's left

The mid-day return is about **re-orientation**, not re-reading. The surface should support a quick scan (schedule section + priorities) without requiring re-reading the hero narrative or lead story.

---

### Section-Level JTBDs

#### 1. Hero Section
- **Job:** Set the tone for the day in one glance. Answer "what kind of day is this?" before any details.
- **Situation:** User has just opened the app. Eyes land here first.
- **Done:** User has an emotional/cognitive frame for their day ("busy meeting day," "clear for deep work," "one big external meeting").
- **Boundary:** This is a headline, not a summary. It should NOT try to enumerate everything — that's what the sections below do.

#### 2. Focus Section
- **Job:** Answer "what should I prioritize today and how much capacity do I have?" in concrete terms.
- **Situation:** User has read the headline and wants to know where to direct their energy.
- **Done:** User knows the single most important thing to focus on, how much free time they have, and how many meetings will interrupt.
- **Boundary:** Focus is a strategic recommendation, not a task list. It points direction; the Priorities section handles the specific items.

#### 3. Lead Story (Featured Meeting)
- **Job:** Surface the one meeting that most needs the user's awareness right now — the highest-stakes external meeting. Answer "what's the big one today?"
- **Situation:** User wants to know if there's a meeting that needs special attention before diving into the schedule.
- **Done:** User knows the meeting exists, who it's with, the headline narrative, and has a bridge to the full briefing if they need to go deeper. They do NOT need to complete their prep from this section.
- **Boundary:** This is a **highlight**, not the full meeting prep. The Meeting Detail page owns the deep dive. The lead story should give enough context to (a) trigger the user to click through if they haven't prepped yet, or (b) reassure them they're ready.

#### 4. Schedule Section
- **Job:** Provide a temporal map of the day. Answer "what's on my calendar, in what order, and which meetings have prep available?"
- **Situation:** User wants to orient themselves chronologically. What's next? What's later?
- **Done:** User can see every meeting, its time, type, and whether prep exists. They can expand upcoming meetings for a quick look, or click past meetings to see outcomes.
- **Boundary:** The schedule is a map, not a dashboard. Each row should be scannable in 2-3 seconds. Deep context belongs in the expansion panel or Meeting Detail page.

#### 5. Review Section (Proposed Actions)
- **Job:** Let the user triage AI-suggested actions with minimal friction. Answer "has the system found things I should be tracking?"
- **Situation:** The AI has generated proposed actions from meetings or emails. User needs to accept or dismiss them.
- **Done:** User has reviewed each proposed action and made an accept/dismiss decision (or deferred by navigating to Actions page).
- **Boundary:** This is triage, not execution. The user decides "yes, track this" or "no, dismiss" — they don't work on the actions here. Limit to 5 items; the rest live on the Actions page.

#### 6. Priorities Section (or Loose Threads fallback)
- **Job:** Answer "what should I work on when I'm not in meetings?" Surface the most urgent actions with capacity context.
- **Situation:** User has oriented on schedule and featured meeting, now wants to know what tasks need attention.
- **Done:** User knows their overdue items, today's due items, and has a sense of whether the day is achievable. They can mark quick items as done or click through for details.
- **Boundary:** This is a curated view (AI-prioritized, capacity-aware), not the full action queue. The Actions page owns the complete list and filtering. Show at most ~5 "Due Today" and ~3 "Later This Week" items.

#### 7. Email Sub-Section (within Priorities)
- **Job:** Surface the 3-4 emails that need the user's attention today. Answer "is anything in my inbox urgent?"
- **Situation:** User is scanning priorities and should see email alongside actions — both are "things to do."
- **Done:** User sees sender, subject, and recommended action for each important email. They can click through to the Emails page for details.
- **Boundary:** This is a triage surface, not an inbox. 3-4 emails max. The Emails page owns the full list.

#### 8. Finis Marker
- **Job:** Signal "you've read everything. You're briefed." Provide the psychological closure of reaching the end of a finite document.
- **Situation:** User has scrolled to the bottom.
- **Done:** User sees the three asterisks and knows there's nothing more to read.
- **Boundary:** Pure closure signal. No content, no actions.

---

## Phase 2: Element Inventory

### State Machine: Four Top-Level States

The Daily Briefing has four states controlled by `DashboardPage` (`src/router.tsx:188-202`):

| State | Component | Trigger |
|-------|-----------|---------|
| Loading | `DashboardSkeleton` | Initial mount, before data arrives |
| Empty | `DashboardEmpty` | No briefing data exists yet |
| Error | `DashboardError` | Backend returned an error |
| Success | `DailyBriefing` | Data loaded successfully |

---

### Loading State: `DashboardSkeleton`

**File:** `src/components/dashboard/DashboardSkeleton.tsx`

| Element | What it renders | Job served | Earning its space? |
|---------|----------------|------------|-------------------|
| Hero skeleton | 3 pulse bars (headline + 2 narrative lines) | Anticipate hero layout, prevent layout shift | **Keep** — standard loading pattern |
| Focus skeleton | Margin grid + vertical bar + pulse bars | Anticipate focus layout | **Keep** |
| Lead story skeleton | Margin grid + title/meta/narrative/prep grid pulses | Anticipate lead story | **Keep** |
| Schedule skeleton | 4 time/title rows with pulse bars | Anticipate schedule | **Keep** |
| Priorities skeleton | 3 circle+text rows | Anticipate priorities | **Keep** |

**Verdict:** Skeleton is well-structured and matches the real layout. No issues.

---

### Empty State: `DashboardEmpty`

**File:** `src/components/dashboard/DashboardEmpty.tsx`

| Element | What it renders | Job served | Earning its space? |
|---------|----------------|------------|-------------------|
| BrandMark (sunrise) | Turmeric asterisk at top | Brand presence | **Keep** — warm, establishes identity |
| "No briefing yet" heading | Serif 36px heading | Explain the blank state | **Keep** |
| Message paragraph | Dynamic message from backend | Context-specific explanation | **Keep** |
| "Generate Briefing" button | Dark charcoal button, mono font | Primary CTA to start first briefing | **Rethink** — ADR-0083 says "Prepare my day" for cold start, not "Generate Briefing" |
| Google Connect card | Dashed border card with Mail icon + "Connect" button | Prompt unauthed users to connect Google | **Keep** — critical for onboarding |
| Footnote | Italic serif "Grab a coffee" | Warmth, brand voice | **Keep** — but duplicated in BRIEFING_QUOTES array (line 24 of same file) |
| GeneratingProgress screen | Phase steps + rotating quotes + timer (replaces empty state when workflow running) | Show progress during generation | **Keep** — good UX for long operations |

**Vocabulary issues:**
- "Generate Briefing" button: should be "Prepare my day" per ADR-0083
- "No briefing yet" heading: acceptable per ADR-0083 ("Briefing" is the product term for daily surface)

---

### Error State: `DashboardError`

**File:** `src/components/dashboard/DashboardError.tsx`

| Element | What it renders | Job served | Earning its space? |
|---------|----------------|------------|-------------------|
| "Something went wrong" heading | Serif 28px heading | Communicate failure state | **Keep** |
| Error message | Dynamic message from backend | Specific error context | **Keep** |
| "Try again" button | Outlined mono button | Retry mechanism | **Keep** |

**Verdict:** Clean, minimal error state. No issues.

---

### Success State: `DailyBriefing` — Full Element Inventory

**File:** `src/components/dashboard/DailyBriefing.tsx`

---

#### HERO SECTION (lines 251-260)

| # | Element | Code Location | What it renders | Job | Duplicated? | Could user do job without it? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|------------------------------|-------------------|
| H1 | Hero headline | `DailyBriefing.tsx:252` | `data.overview.summary` or fallback text, rendered as serif 76px `h1` | Set the tone — "what kind of day is this?" | No | No — this IS the first impression | **Keep** |
| H2 | Staleness indicator | `DailyBriefing.tsx:255-259` | "Last updated {time}" in mono 11px, only shown when `freshness === "stale"` | Warn user that data may be outdated | No | Yes — but it prevents acting on stale data | **Keep** — conditional display is correct |

**Hero observations:**
- The fallback text when `overview.summary` is empty is "A clear day. Nothing needs you." (no meetings) or "Your day is ready." — good chief-of-staff voice.
- The 76px headline is impactful on first read but wastes vertical space on mid-day returns. No way to collapse or minimize it.

---

#### FOCUS SECTION (lines 263-283)

| # | Element | Code Location | What it renders | Job | Duplicated? | Could user do job without it? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|------------------------------|-------------------|
| F1 | Margin label "Focus" | Line 266 | Mono uppercase label in left margin | Section identification | No | No | **Keep** |
| F2 | Focus text | Line 269 | `data.overview.focus` — AI-generated focus recommendation | Answer "what should I prioritize?" | No | Yes — but it's the highest-value AI output on the surface | **Keep** — high value |
| F3 | Capacity line | Lines 272-278 | "{X}h available / {N} deep work blocks / {N} meetings" in mono | Quantify the day's shape | Partially — meeting count also visible in schedule section header | Experienced users might not need it | **Keep** — provides actionable framing for focus |

**Focus observations:**
- The capacity line is conditionally rendered (only when `data.focus` exists). Good — doesn't waste space when AI hasn't run.
- The deep work block count only shows blocks >= 60 minutes. Smart filter.
- The focus text and capacity line together answer "what + how much time" — strong combination.

---

#### LEAD STORY / FEATURED MEETING (lines 286-368)

| # | Element | Code Location | What it renders | Job | Duplicated? | Could user do job without it? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|------------------------------|-------------------|
| L1 | Margin label "The Meeting" | Line 289 | Mono label | Section identification | No | No | **Keep** |
| L2 | Section rule | Line 291 | Horizontal line | Visual section separator | No | Yes | **Keep** — editorial convention |
| L3 | Meeting title | Line 294-299 | Serif h2 with optional "NOW" pill | Identify the featured meeting | Yes — same meeting appears in schedule | No | **Keep** — the duplication is intentional (highlight + schedule presence) |
| L4 | NOW pill (inline) | Line 296-298 | Gold inline badge when meeting is in-progress | Temporal awareness | Also on schedule row | No — critical for "what's happening right now?" | **Keep** |
| L5 | Meta line | Lines 302-321 | "Time - EndTime / Duration / Entity byline / Attendee count" | Quick reference facts | Same info in schedule row (partially) | The full meta line is richer here | **Keep** — justified for featured meeting |
| L6 | Narrative context | Lines 325-327 | `featured.prep.context` paragraph | "What's the context for this meeting?" | Also in meeting detail page | Yes — but reading the full detail page takes more time | **Keep** — the one-paragraph summary is high value |
| L7 | Key People flow | Lines 330-332 | Avatars + names + roles + relationship dots | "Who will I be meeting with?" | Also in meeting detail page + expansion panels | Users could check the calendar invite | **Rethink** — in the LEAD STORY, stakeholders are useful. But the temperature dots are always `hot` class (hardcoded at line 170 of BriefingMeetingCard.tsx). If the dot doesn't carry real signal, cut it. |
| L8 | Prep Grid (Discuss/Watch/Wins) | Lines 335 | 2-column grid with up to 1 item per category | "What are the key talking points?" | Also in meeting detail page | Users could click through to detail | **Rethink** — shows only 1 item per category (sliced at lines 196, 208, 220 of BriefingMeetingCard.tsx). At that point, is 1 bullet per category useful or just a teaser? Consider whether this earns its space vs. a single "context" paragraph (L6) + the bridge link (L11). |
| L9 | Meeting Action Checklist | Lines 338-342 | "Before this meeting" actions with completion circles | "What do I need to do before this meeting?" | Actions also in Priorities section | No — this is meeting-specific action grouping | **Keep** — high value, directly actionable |
| L10 | Entity chips (MeetingEntityChips) | Lines 345-354 | Linked account/project chips with add/remove + EntityPicker | "Which account/project is this meeting for?" + manual correction | Also on meeting detail page and schedule expansion panels | Yes — entity resolution usually works automatically | **Rethink** — the EntityPicker ("Link entity...") is a power-user correction tool. On the briefing, it introduces editing affordances into a reading surface. Consider: keep the chips (read-only display) but move the picker (add/remove) to the meeting detail page only. |
| L11 | "Read full intelligence" link | Lines 358-365 | Arrow link to meeting detail page | Bridge to deep dive | No | No — this is how they navigate deeper | **Keep** — but rename per ADR-0083: "Read full intelligence" uses system vocabulary. Should be "Read full briefing" or simply "Full meeting briefing" |

**Lead Story observations:**
- The featured meeting selection logic (`selectFeaturedMeeting`, lines 77-95) filters to external meetings with prep, weighted by type (QBR > customer > partnership > external > training). This is sound — the "biggest" meeting gets the highlight.
- The lead story is the largest section on the surface. It contains: title, meta, narrative, key people, prep grid, action checklist, entity chips, and a bridge link. That is 8 sub-elements. Consider whether Key People + Prep Grid are doing enough work here vs. the space they consume, given the bridge link exists.
- The `BriefingMeetingCard.tsx:170` hardcoded `s.keyPeopleTempHot` class means the relationship temperature dot is always the same color. This is a broken signal — either feed real data or remove the dot.

---

#### SCHEDULE SECTION (lines 372-400)

| # | Element | Code Location | What it renders | Job | Duplicated? | Could user do job without it? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|------------------------------|-------------------|
| S1 | Margin label "Schedule" + count | Lines 375-378 | "Schedule" label + "{N} meetings" sub-label | Section identification + volume signal | Meeting count also in focus capacity line | No | **Keep** |
| S2 | Section rule | Line 380 | Horizontal line | Visual separator | No | Yes | **Keep** — editorial convention |
| S3 | Schedule rows (BriefingMeetingCard) | Lines 382-395 | One row per meeting (time + title + subtitle + interactive states) | Temporal map of the day | No — this IS the schedule | No | **Keep** |

**Per Schedule Row (BriefingMeetingCard):**

| # | Element | Code Location | What it renders | Job | Duplicated? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|-------------------|
| S3a | Time column | `BriefingMeetingCard.tsx:414-416` | "9:30 AM" + "1h" duration | When is this meeting? | No | **Keep** |
| S3b | Title | `BriefingMeetingCard.tsx:420` | Meeting title | What is this meeting? | Same as lead story title (for featured meeting) | **Keep** |
| S3c | NOW pill | `BriefingMeetingCard.tsx:421` | Gold "NOW" badge for in-progress meeting | Temporal awareness | Also on lead story | **Keep** |
| S3d | Past arrow | `BriefingMeetingCard.tsx:422` | Right arrow for past meetings | Affordance: "click to see outcomes" | No | **Keep** |
| S3e | Expand/collapse hint | `BriefingMeetingCard.tsx:423-425` | Text "expand"/"collapse" | Affordance: "click for more info" | No | **Keep** |
| S3f | Subtitle | `BriefingMeetingCard.tsx:429-431` | Entity byline or meeting type + attendee count | Meeting context at a glance | Entity byline also in lead story meta | **Keep** |
| S3g | Prep dot | `BriefingMeetingCard.tsx:431` | Small green dot when `hasPrep` | "Prep exists for this meeting" | No | **Keep** — but should change to quality indicator per I329 |
| S3h | Past meeting outcomes line | `BriefingMeetingCard.tsx:433-449` | "{N} actions captured / {N} needs review" | Post-meeting awareness | No | **Keep** — connects the briefing to its ongoing purpose |
| S3i | Cancelled row | `BriefingMeetingCard.tsx:374-388` | Line-through title + "Cancelled" | "This meeting was cancelled" | No | **Keep** — prevents confusion |

**Per Schedule Expansion Panel (upcoming meetings with prep):**

| # | Element | Code Location | What it renders | Job | Duplicated? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|-------------------|
| S3j | Expansion narrative | `BriefingMeetingCard.tsx:461-463` | `meeting.prep.context` paragraph | Quick context without navigating | Same text in lead story (for featured meeting) | **Keep** — useful for non-featured meetings |
| S3k | Key People flow | `BriefingMeetingCard.tsx:466-468` | Same as lead story L7 | Who's attending? | Duplicated from lead story for featured meeting | **Rethink** — same temperature dot issue |
| S3l | QuickContext | `BriefingMeetingCard.tsx:471` | 1-line signals: Discuss/Watch/Win | Fast prep check | Similar to lead story PrepGrid (L8) | **Keep** — this is actually better than the PrepGrid because it's more compact (single-line per signal) |
| S3m | Meeting Action Checklist | `BriefingMeetingCard.tsx:474-478` | Same as lead story L9 | Actions before this meeting | Duplicated from lead story for featured meeting | **Keep** — meeting-specific actions |
| S3n | Entity chips (MeetingEntityChips) | `BriefingMeetingCard.tsx:481-490` | Same as lead story L10 | Entity assignment + correction | Duplicated from lead story for featured meeting | **Rethink** — same concern as L10. Editing on a reading surface. |
| S3o | "Read full intelligence" link | `BriefingMeetingCard.tsx:495-499` | Bridge link to meeting detail | Navigate deeper | No | **Keep** — but rename per ADR-0083 |
| S3p | "Collapse" button | `BriefingMeetingCard.tsx:500-509` | Text button to close panel | Return to scan mode | No | **Keep** |

**Schedule observations:**
- All meetings are shown (`scheduleMeetings = meetings` at line 139), including the featured one. The code comment confirms: "still appears in schedule — lead story is a highlight, not a removal." This is correct.
- Past meetings navigate to meeting detail on click; upcoming meetings expand inline. Good modal distinction.
- The featured meeting is fully duplicated: its content appears in both the Lead Story AND the Schedule expansion panel. When expanded in the schedule, the user sees the same narrative, key people, prep grid, actions, and entity chips they already saw in the lead story. This is wasteful.

---

#### REVIEW SECTION — PROPOSED ACTIONS (lines 402-518)

| # | Element | Code Location | What it renders | Job | Duplicated? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|-------------------|
| R1 | Margin label "Review" + count | Lines 406-408 | Turmeric-colored "Review" label + "{N} suggested" | Section identification + volume | No | **Keep** |
| R2 | Section rule | Line 411 | Horizontal line | Visual separator | No | **Keep** |
| R3 | Proposed action rows | Lines 413-495 | Title + source label + accept/reject buttons (max 5) | Triage: accept or dismiss | Also on Actions page (I334) | **Keep** — but see observation below |
| R3a | Action title | Line 436 | Serif 15px text | What's the suggested action? | No | **Keep** |
| R3b | Source label | Lines 439-449 | Mono 11px source context | Where did this come from? | No | **Keep** |
| R3c | Accept button (checkmark) | Lines 453-472 | Small sage-bordered button with checkmark SVG | Accept the suggestion | No | **Keep** |
| R3d | Reject button (X) | Lines 473-491 | Small terracotta-bordered button with X SVG | Dismiss the suggestion | No | **Keep** — but ADR-0083 says "Dismiss" not "Reject" |
| R3e | Dashed left border | Line 424 | Turmeric dashed border on each row | Visual signal: "this is proposed, not confirmed" | No | **Keep** — good visual differentiation |
| R4 | "See all {N} suggestions" link | Lines 497-514 | Arrow link to Actions page (only if > 5 proposed) | Bridge to full triage queue | No | **Keep** |

**Review section observations:**
- The section is entirely inline-styled (lines 412-514). No CSS module classes. This is inconsistent with the rest of the briefing which uses `editorial-briefing.module.css`.
- The accept/reject buttons have title attributes "Accept" and "Reject" — should be "Accept" and "Dismiss" per ADR-0083.
- The section only appears when `proposedActions.length > 0`. Good — no empty state needed.
- Capped at 5 items. Smart density control.

---

#### PRIORITIES SECTION (lines 520-539, function at 548-668)

This section has two variants: **PrioritiesSection** (when AI-prioritized actions exist) and **LooseThreadsSection** (fallback when they don't).

**PrioritiesSection:**

| # | Element | Code Location | What it renders | Job | Duplicated? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|-------------------|
| P1 | Margin label "Priorities" | Line 576 | Mono label | Section identification | No | **Keep** |
| P2 | Section rule | Line 578 | Horizontal line | Visual separator | No | **Keep** |
| P3 | AI-synthesized capacity intro | Lines 581-583 | `focus.implications.summary` paragraph | "How achievable is today?" | No | **Keep** — high-value framing |
| P4 | "Overdue" group label | Line 588 | Terracotta-colored group header | Urgency grouping | Also on Actions page | **Keep** |
| P5 | Overdue action items | Lines 589-599 | PrioritizedActionItem (all overdue) | See and act on overdue items | Also on Actions page | **Keep** — high urgency, worth surfacing |
| P6 | "Due Today" group label | Line 606 | Group header | Urgency grouping | Also on Actions page | **Keep** |
| P7 | Due Today action items | Lines 607-617 | PrioritizedActionItem (max 5) | Today's work | Also on Actions page | **Keep** — but 5 is generous; consider 3 |
| P8 | Email group (woven between action groups) | Lines 622-631 | PriorityEmailItem rows (3-4 emails) | "What emails need attention?" | Also on Emails page | **Rethink** — see email section analysis below |
| P9 | "Later This Week" group label | Line 636 | Muted group header | Lower urgency grouping | Also on Actions page | **Rethink** — is "Later This Week" serving the daily briefing's job (today), or leaking weekly scope? |
| P10 | Upcoming action items | Lines 637-647 | PrioritizedActionItem (max 3, tapered weight) | Awareness of upcoming work | Also on Actions page + Week page | **Rethink** — these aren't actionable today. Does showing them here create noise? |
| P11 | "View all {N} actions" link | Lines 653-656 | Arrow link to Actions page | Bridge to full queue | No | **Keep** |
| P12 | "View all emails" link | Lines 658-661 | Arrow link to Emails page | Bridge to full inbox | No | **Keep** |

**Per PrioritizedActionItem (lines 772-836):**

| # | Element | Code Location | What it renders | Job | Duplicated? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|-------------------|
| PA1 | Completion checkbox | Lines 812-826 | Circle button, fills on complete | Mark as done | Same pattern on Actions page | **Keep** — directly actionable |
| PA2 | Action title | Line 828 | Serif text, links to action detail | What's the action? | Also on Actions page | **Keep** |
| PA3 | Context line | Line 829 | "Overdue / Account / ~effort" in mono | Quick context | Account also on Actions page | **Keep** — effort estimate is unique to this view |
| PA4 | "Why" line (overdue only) | Lines 830-832 | AI-generated reason | "Why is this at risk?" | No | **Keep** — unique, high-value signal |

**Per PriorityEmailItem (lines 840-865):**

| # | Element | Code Location | What it renders | Job | Duplicated? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|-------------------|
| E1 | Priority dot | Lines 847-851 | Terracotta (high) or turmeric (medium) dot | Visual priority signal | No | **Keep** |
| E2 | Sender name | Line 855 | Bold sender | Who sent this? | Also on Emails page | **Keep** |
| E3 | Subject | Line 857 | Email subject line | What's it about? | Also on Emails page | **Keep** |
| E4 | Recommended action | Lines 859-861 | AI-suggested next step | "What should I do?" | Also on Emails page | **Keep** — this is what makes the email summary useful |

**LooseThreadsSection (fallback, lines 672-768):**

| # | Element | What it renders | Job | Earning its space? |
|---|---------|-----------------|-----|-------------------|
| LT1 | Margin label "Loose Threads" | Section identification | **Keep** — warm language for fallback state |
| LT2 | Action items (max 5) | Standard action rows with overdue styling | Same as Priorities but without AI ranking | **Keep** — better than nothing |
| LT3 | Email items | Same PriorityEmailItem | Same as Priorities emails | **Keep** |
| LT4 | View all links | Bridge links | Same as Priorities | **Keep** |

**Priorities observations:**
- The "Later This Week" group (P9/P10) leaks into weekly territory. The daily briefing's job is TODAY. Consider whether these items should only appear on the Week page.
- The email sub-section label dynamically changes: "Emails Needing Response" (when high-priority emails exist) vs. "Emails Worth Noting" (when only medium). Good adaptive language.
- The email filtering logic (lines 129-135): high-priority emails take precedence (max 4), falling back to medium (max 3). This is sound — prioritize urgency.

---

#### FINIS MARKER (line 542)

| # | Element | Code Location | What it renders | Job | Duplicated? | Earning its space? |
|---|---------|---------------|-----------------|-----|-------------|-------------------|
| FM1 | Three BrandMark asterisks | `FinisMarker.tsx:14-26` | Turmeric asterisks centered | "You're done reading" | Used on many surfaces | **Keep** — core editorial convention |
| FM2 | Enrichment timestamp | `FinisMarker.tsx:27-38` | "Last enriched: {date}" | Metadata | NOT USED here — no `enrichedAt` prop passed | **N/A** — prop not passed, so timestamp doesn't render |

**Finis observations:**
- The FinisMarker on the daily briefing does NOT show the enrichment timestamp because no `enrichedAt` prop is passed (line 542). This is inconsistent with other surfaces (Meeting Detail, Account Detail, etc.) where it does show. Consider passing `freshness.generatedAt` to show when the briefing was last generated.

---

### Magazine Shell / Folio Bar Elements

These elements are registered by the component via `useRegisterMagazineShell` (lines 186-197):

| # | Element | What it renders | Job | Earning its space? |
|---|---------|-----------------|-----|-------------------|
| MB1 | Folio label "Daily Briefing" | Page identity in top bar | Page identification | **Keep** |
| MB2 | Atmosphere color "turmeric" | Background tint in shell | Brand/mood signal | **Keep** |
| MB3 | Active page "today" | Nav highlighting | Navigation state | **Keep** |
| MB4 | Folio date text | Uppercase formatted date | "What day is it?" | **Keep** |
| MB5 | Readiness stats | "X/Y prepped" (sage) + "N overdue" (terracotta) | At-a-glance readiness | **Keep** — high information density in small space |
| MB6 | Refresh button | RefreshCw icon + "Refresh" / phase label | Trigger briefing regeneration | **Keep** — but label should match ADR-0083 ("Refresh" is correct when data exists; "Preparing..." etc. during run is correct) |

---

### Components NOT Used by Daily Briefing (but in `src/components/dashboard/`)

These files exist in the dashboard directory but are NOT imported by the current `DailyBriefing.tsx`. They appear to be legacy components from a previous layout:

| File | What it is | Used by Daily Briefing? | Used elsewhere? |
|------|-----------|------------------------|-----------------|
| `Header.tsx` | Old sidebar-shell header with title "Today", StatusIndicator, RunNowButton, search | **No** — magazine shell replaced it | Yes — still in `router.tsx` for non-magazine routes |
| `StatusIndicator.tsx` | Workflow status badge (Idle/Running/Ready/Error) | **No** — replaced by folio bar | Yes — used by `Header.tsx` |
| `RunNowButton.tsx` | "Run Now" / "Run Briefing" button with tooltip | **No** — replaced by folio bar refresh button | Yes — used by `Header.tsx` |
| `ActionList.tsx` | Legacy action list with proposed actions | **No** — replaced by inline PrioritizedActionItem + Review section | Unclear — may be unused |
| `ActionItem.tsx` | Legacy action item with badges | **No** — replaced by PrioritizedActionItem | Used by `ActionList.tsx` |
| `EmailList.tsx` | Legacy email list with sync status | **No** — replaced by PriorityEmailItem | Unclear — may be unused |

**Observation:** `ActionList.tsx`, `ActionItem.tsx`, and `EmailList.tsx` appear to be dead code for the daily briefing surface. They may still be used by legacy routes or non-magazine paths. Should be audited for removal.

---

## Summary of Findings

### Elements to Keep (no changes needed)
- Hero headline + staleness indicator
- Focus section (text + capacity)
- Schedule section structure (rows, temporal states, expansion panels)
- Review section (proposed action triage)
- Priorities section (overdue + due today + AI intro)
- Finis marker
- Folio bar elements
- All loading/empty/error states

### Elements to Rethink

| Element | Issue | Recommendation |
|---------|-------|----------------|
| **L7: Key People temperature dots** | Always hardcoded to `hot` class (`BriefingMeetingCard.tsx:170`). Dot carries no real signal. | Either feed real relationship temperature data or remove the dot entirely. The name + role are sufficient. |
| **L8: Prep Grid in Lead Story** | Shows only 1 item per category (Discuss/Watch/Win). At that density, is a 2-column grid worth the space vs. the narrative context paragraph + bridge link? | Consider removing the PrepGrid from the lead story. The narrative (L6) + QuickContext (S3l, used in expansion panels) + bridge link (L11) may be sufficient. Users who need prep details click through. |
| **L10 / S3n: Entity chips with picker** | The EntityPicker introduces editing affordances ("Link entity...", X-to-remove) on a reading surface. The daily briefing's job is morning reading, not data correction. | Keep entity chips as read-only display on the briefing. Move the add/remove picker to the meeting detail page only. |
| **P9/P10: "Later This Week" group** | The daily briefing's job is TODAY. "Later This Week" items aren't actionable today and leak into the Week page's territory. | Move "Later This Week" to the Week page exclusively. The daily briefing should show Overdue + Due Today only. |
| **S3: Featured meeting duplication** | The featured meeting appears fully rendered in both the Lead Story AND the Schedule expansion panel. Expanding it in the schedule shows the same content. | Consider suppressing the expansion panel for the featured meeting (since the lead story already shows its content), or collapsing the lead story into a more compact "highlighted row" in the schedule. |

### Vocabulary Issues (for I341)

| Current text | Location | ADR-0083 translation |
|-------------|----------|---------------------|
| "Generate Briefing" button | `DashboardEmpty.tsx:127` | "Prepare my day" (cold start) |
| "Read full intelligence" link | `DailyBriefing.tsx:363`, `BriefingMeetingCard.tsx:497` | "Read full briefing" or "Full meeting briefing" |
| "Reject" button title | `DailyBriefing.tsx:474` | "Dismiss" |
| "Link entity..." picker placeholder | `meeting-entity-chips.tsx:215` | "Link to account or project" |
| `hasPrep` dot (title: "Prep available") | `BriefingMeetingCard.tsx:431` | Quality indicator per I329 (New/Building/Ready/Updated) |

### Potential Dead Code

| File | Reason |
|------|--------|
| `ActionList.tsx` | Not imported by DailyBriefing or any current page route — may be legacy |
| `ActionItem.tsx` | Only imported by ActionList.tsx — if ActionList is dead, so is this |
| `EmailList.tsx` | Not imported by DailyBriefing or any current page route — may be legacy |
| `Header.tsx` | Still used for non-magazine routes but the daily briefing doesn't use it |

### Structural Observations

1. **The surface is a document, not a dashboard.** The code comment (line 5) says "A morning document, not a dashboard. You read it top to bottom. When you reach the end, you're briefed." The implementation honors this. The Hero-Focus-Lead-Schedule-Review-Priorities-Finis flow is a linear editorial structure. This is working well.

2. **Mid-day return is not optimized.** The hero headline takes ~120px of vertical space. On a mid-day return for re-orientation, the user must scroll past the headline and focus to reach the schedule. Consider: should the hero collapse on subsequent visits? Or should there be an anchor/shortcut to jump to "what's next?"

3. **The Review section is entirely inline-styled.** Every other section uses CSS module classes from `editorial-briefing.module.css`. The Review section (lines 402-518) uses inline `style={{}}` on every element. This is a consistency issue — not blocking, but worth noting for cleanup.

4. **Data flow is clean.** The `useDashboardData` hook handles loading/empty/error/success states, auto-refreshes on workflow completion, calendar updates, prep generation, and window focus. The `useCalendar` hook provides real-time `now` and `currentMeeting` for temporal state. The `useProposedActions` hook provides the triage queue. No data concerns.

5. **Action completion is optimistic.** The `handleComplete` function (line 210-213) updates local state immediately and fires-and-forgets the Tauri command. Good UX pattern. No rollback on failure though — if the backend rejects, the UI will show the action as completed until next refresh.
