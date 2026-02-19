# I342: Meeting Detail Surface — JTBD Definition + Element Inventory

**Surface:** Meeting Detail Page (internally "Meeting Intelligence Report")
**Primary file:** `src/pages/MeetingDetailPage.tsx` (~2902 lines)
**Route:** `/meeting/$meetingId`
**Date:** 2026-02-18

---

## Phase 1: JTBD Analysis

### Surface-Level JTBD

**Situation:** A user has an upcoming meeting (or a past meeting they need to debrief). They want to understand the context, the people, and the stakes before walking in — or capture what happened after walking out.

**Motivation:** Reduce pre-meeting anxiety. Arrive informed. Know what matters to discuss, who will be in the room and their disposition, what risks exist, and what happened since the last time. For past meetings: ensure nothing falls through the cracks — decisions, actions, and outcomes are captured.

**Desired outcome:** The user closes this surface feeling *briefed*. For future meetings: "I know what to say, what to watch for, and who I'm dealing with." For past meetings: "The outcomes are captured, actions are tracked, I can move on."

**Boundary:** This surface should contain everything needed to *prepare for* or *close out* a single specific meeting. It should not be a general account dashboard, a relationship management tool, or a full action queue. It answers: "What do I need to know about *this* meeting?"

### What Triggers Opening This Surface?

1. **Clicking a meeting from the Daily Briefing schedule** — the most common entry point. Upcoming meetings expand inline on the daily briefing; the "Read full intelligence" link bridges to this page. Past meetings navigate directly on click.
2. **Pre-meeting anxiety** — "My QBR with Acme is in 20 minutes, what should I know?" The urgency banner (line 627-644) directly addresses this.
3. **Post-meeting debrief** — after attaching a transcript, the user reviews outcomes.
4. **Notification or reminder** — from calendar integration or the app itself.
5. **Deep-dive curiosity** — the daily briefing showed a risk or signal; user wants the full picture.

### How Deep Should It Go vs. What the Daily Briefing Shows?

The Daily Briefing (via `BriefingMeetingCard`) shows a *compressed summary*: one-line context, key people, one signal per category (Discuss/Watch/Win), and a "Before this meeting" action checklist. It is a *scanning* surface.

The Meeting Detail should be the *reading* surface — the complete briefing document. Everything the daily briefing teases, this page delivers in full. The daily briefing says "there's a risk." This page explains the risk. The daily briefing names key people. This page gives their disposition, history, and assessment.

Currently the meeting detail goes *very* deep — it includes an appendix with full intelligence summaries, strategic programs, since-last-meeting items, current state, questions, key principles, extended stakeholder maps, and references. Some of this depth may exceed what the "briefed for this meeting" job requires.

### What Does "Done" Look Like?

**For a future meeting:** The user has scanned the key insight, reviewed risks, understood who's in the room, and has their plan (agenda items they want to cover). The FinisMarker at the end of Act II ("You're Briefed") signals completion. Everything after that is optional deep-dive.

**For a past meeting:** Outcomes are captured (either via transcript or manual entry), actions are triaged (accepted or dismissed), and the user has reviewed the summary. The outcomes section at the top confirms this is handled.

---

### Section-Level JTBDs

#### 1. Outcomes Section (post-meeting only)
- **File location:** `OutcomesSection` component, lines 1747-1833
- **Job:** "Show me what happened in this meeting so I can close the loop."
- **Renders when:** `outcomes` is non-null (transcript has been processed)
- **Positioned:** Always at top of the page when present, above all pre-meeting context

#### 2. Past Meeting Transcript Prompt
- **File location:** Lines 522-582
- **Job:** "Remind me to attach a transcript so outcomes can be captured."
- **Renders when:** `isPastMeeting` is true
- **Note:** Also shows QuillSyncBadge for automatic transcript sync status

#### 3. Act I: "Ground Me" — The Brief (headline section)
- **File location:** Lines 622-775, `id="headline"`
- **Job:** "Orient me instantly — what is this meeting, why does it matter, what's the one thing I need to know?"
- **Sub-elements:** Urgency banner, kicker, hero title, lifecycle badge, metadata line, entity chips, key insight pull quote, entity readiness checklist

#### 4. Act II: The Risks
- **File location:** Lines 781-842, `id="risks"`
- **Job:** "What could go wrong? What should I watch for?"
- **Renders when:** `hasRisks` (topRisks.length > 0)
- **Note:** Featured risk gets serif italic treatment with terracotta border; high-urgency risks pulse

#### 5. Act II: The Room
- **File location:** Lines 844-856, `id="the-room"`
- **Job:** "Who will I be talking to, and what's their story?"
- **Renders when:** `hasRoom` (unifiedAttendees.length > 0)
- **Delegate:** `UnifiedAttendeeList` component (lines 1051-1335)

#### 6. Act II: Your Plan
- **File location:** Lines 858-890, `id="your-plan"`
- **Job:** "What am I going to cover? What's my agenda?"
- **Renders when:** `hasPlan` (agendaDisplayItems.length > 0 or editable)
- **Delegate:** `UnifiedPlanEditor` component (lines 1341-1741)

#### 7. "You're Briefed" FinisMarker (mid-page)
- **File location:** Lines 893-897
- **Job:** "Signal that the essential briefing is complete. Everything below is optional."

#### 8. Act III: Deep Dive — Supporting Intelligence
- **File location:** Lines 899-993, `id="deep-dive"`
- **Job:** "If I want to go deeper, here's the supporting material."
- **Sub-sections:** Recent Wins, Open Items, Email Signals, Appendix

#### 9. Appendix (collapsed by default)
- **File location:** `AppendixSection` component, lines 2177-2422
- **Job:** "Raw reference material for the truly curious."
- **Sub-sections:** Full Intelligence Summary, Since Last Meeting, Strategic Programs, Full Context, Current State, Questions to Surface, Key Principles, Extended Stakeholder Map, References

---

## Phase 2: Element Inventory

### Global / Shell Elements

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 1 | **Folio label** | shellConfig, line 325 | "Intelligence Report" | Identity — tells user what surface they're on | No | Yes, but uses system vocabulary ("Intelligence Report") — ADR-0083 says this should be "Meeting Briefing" |
| 2 | **Back link** | shellConfig, line 328 | "Today" with onClick | Navigation back to daily briefing | No | Yes |
| 3 | **Chapter nav** | CHAPTERS constant, lines 112-118 | 5 chapters: The Brief, Risks, The Room, Your Plan, Deep Dive | In-page navigation for long documents | No | Yes — the page is long enough to warrant it |
| 4 | **Folio save status** | line 330 | "Saving..." / "Saved" | Feedback when user edits agenda/attendees | No | Yes |
| 5 | **Prefill button** | folioActions, lines 333-341 | "Prefill" (editable meetings only) | Copies AI-proposed agenda into user's editable layer | No | Questionable — this is a power-user feature that may confuse. The verb "prefill" is system vocabulary. |
| 6 | **Draft Agenda button** | folioActions, lines 342-344 | "Draft Agenda" | Generates an email draft with agenda to send to attendees | No | Useful but secondary. The dialog it opens (`AgendaDraftDialog`) is copy-only, not send. |
| 7 | **Transcript button** | folioActions, lines 345-352 | Paperclip icon + "Transcript" / "Processing..." | Opens file picker to attach a transcript | Also appears in body for past meetings (line 563-580) | Yes for past meetings. Duplicate: appears in both folio bar AND as a prominent body-level CTA for past meetings. |
| 8 | **Refresh button** | folioActions, lines 353-359 | RefreshCw icon + "Refresh" | Reloads meeting intelligence | No | Yes |

### Loading / Error / Empty States

| # | Element | Location | What it renders | Job it serves | Notes |
|---|---------|----------|-----------------|---------------|-------|
| 9 | **Loading state** | lines 366-368 | `EditorialLoading` with 5 skeleton rows | Communicate that data is loading | Standard editorial pattern |
| 10 | **Error state** | lines 370-373 | `EditorialError` with retry button | Show error and allow retry | Standard editorial pattern |
| 11 | **Empty state (no data)** | lines 376-413 | Clock icon + "Prep not ready yet" + "Meeting context will appear here after the daily briefing runs." | Tell user the system hasn't generated content yet | Uses system vocabulary: "Prep not ready yet." ADR-0083 says "Needs prep" concept should disappear. |
| 12 | **Empty state (no content, has data)** | lines 584-617 | Clock icon + "Prep is being generated" + "Meeting context will appear here once AI enrichment completes." | Tell user enrichment is in progress | Uses system vocabulary: "Prep is being generated," "AI enrichment." ADR-0083 says this should be invisible or "Building context." |

### Outcomes Section (Post-Meeting)

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 13 | **"Meeting Outcomes" heading** | OutcomesSection, line 1767 | Serif h2 "Meeting Outcomes" | Section identity | No | Yes |
| 14 | **Summary paragraph** | line 1773 | Free-text meeting summary from transcript analysis | Quick overview of what happened | No | Yes — the most important post-meeting element |
| 15 | **Wins subsection** | lines 1778-1783 | Trophy icon + "Wins" + count + bullet list | Celebrate positive outcomes | No | Yes — gives the user something to reference in follow-ups |
| 16 | **Risks subsection** | lines 1787-1792 | AlertTriangle icon + "Risks" + count + bullet list | Flag post-meeting risks | No | Yes |
| 17 | **Decisions subsection** | lines 1796-1801 | CircleDot icon + "Decisions" + count + bullet list | Record decisions made | No | Yes — critical for accountability |
| 18 | **Actions subsection** | lines 1806-1829 | "Actions" heading + `OutcomeActionRow` list | Triage and track action items from transcript | Partial overlap with Actions page | Yes — but note these are *meeting-scoped* actions, while the Actions page shows *all* actions |
| 19 | **OutcomeActionRow** | lines 1889-2085 | Accept/Reject for proposed, checkbox for accepted, priority cycling, due date | Per-action triage and completion | Actions page has similar but more comprehensive action rows | Essential for in-context triage |
| 20 | **"Pre-Meeting Context" / "Meeting Prep" divider** | lines 516-519 | Mono heading below outcomes | Separate post-meeting outcomes from pre-meeting briefing content | No | Yes — provides clear temporal boundary |

### Past Meeting Transcript Prompt

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 21 | **Transcript prompt banner** | lines 522-582 | Dashed border box with "No outcomes captured yet" / "Update outcomes" + Attach Transcript button | Prompt user to attach transcript for past meetings | Duplicates the folio bar Transcript button | The body-level version is more discoverable; consider removing the folio bar duplicate for past meetings |
| 22 | **QuillSyncBadge** | lines 557-561, component at 2792-2901 | Status badge showing automatic transcript sync state (pending/polling/fetching/processing/completed/failed) | Inform user about automatic transcript retrieval | No | Yes — reduces need for manual transcript attachment |

### Act I: The Brief (headline section)

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 23 | **Urgency banner** | lines 627-644 | Clock icon + "Meeting starts in X minutes" (yellow < 2h, terracotta < 15m) | Create appropriate urgency, orient the user in time | No | Yes — one of the surface's most valuable elements |
| 24 | **Kicker overline** | lines 647-649 | "Meeting Intelligence Report" (mono, uppercase) | Page identity | No | Uses system vocabulary. Should be "Meeting Briefing" per ADR-0083. |
| 25 | **Hero title** | lines 652-654 | Meeting title at 76px serif | Identify the meeting | Duplicates daily briefing schedule row title, but at hero scale | Yes |
| 26 | **Lifecycle badge** | lines 655-670 | Turmeric dot + lifecycle stage (e.g., "expansion") | Show account relationship stage | Entity detail pages also show this | Useful context, but derived from entity data — could be a link to entity instead |
| 27 | **Metadata line** | lines 673-687 | Time range, meeting type, primary entity name (dot-separated) | Quick reference metadata | Daily briefing shows time + entity in the schedule row | Yes — the full metadata set is richer here |
| 28 | **Entity chips** | lines 690-701 | `MeetingEntityChips` — colored chips for linked accounts/projects/people with add/remove | Allow user to correct or add entity links for this meeting | Daily briefing expansion also shows these (compact mode) | Yes — entity linking is a core feature. Duplication with daily briefing is acceptable (different contexts). |
| 29 | **Key insight pull quote** | lines 704-732 | First sentence from intelligence summary, displayed as a 28px serif italic blockquote with turmeric border | Deliver the single most important thing to know | Not directly duplicated, but the daily briefing's `prep.context` serves a similar orienting role | Yes — this is the "headline" of the briefing |
| 30 | **Empty insight fallback** | lines 733-743 | "Intelligence builds as you meet with this account." | Explain why there's no key insight | No | Uses system vocabulary ("Intelligence builds"). Should use product vocabulary. |
| 31 | **Entity readiness checklist** | lines 746-774 | "Before This Meeting" heading + up to 4 bullet items with turmeric border | Action items to complete before the meeting | Daily briefing has `MeetingActionChecklist` ("Before this meeting") — similar but different data source | Overlapping job — the daily briefing's "Before this meeting" actions are DB-backed actions, while these are intelligence-derived readiness items. Both answer "what should I do before this meeting?" |

### Act II: The Risks

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 32 | **ChapterHeading "The Risks"** | line 784 | Rule + serif "The Risks" | Section identity | No | Yes |
| 33 | **Featured risk** (first risk) | lines 788-814 | Serif italic blockquote with terracotta border, pulse animation if high urgency | Highlight the primary risk | Daily briefing expansion shows first risk as a single "Watch" line | Yes — the full treatment is appropriate for the detail surface |
| 34 | **Subordinate risks** (2nd-3rd) | lines 815-838 | Body-scale text with light rule, terracotta left border if high urgency | Present additional risks with lower visual weight | No | Yes |

### Act II: The Room

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 35 | **ChapterHeading "The Room"** | line 847 | Rule + serif "The Room" | Section identity | No | Yes |
| 36 | **UnifiedAttendeeList** | lines 1051-1335 | For each attendee: avatar circle, name (linked to person detail if personId exists), role, temperature dot + label, engagement badge, "New contact" label, assessment (serif italic), metadata (org, meeting count, last seen), notes | Comprehensive stakeholder briefing — who's in the room and what's their disposition | Daily briefing shows `KeyPeopleFlow` (name, role, avatar — compact one-liner). Entity detail pages show person lists. | Yes — this is the deep version. Serves a different job than the daily briefing's compact list. |
| 37 | **Attendee hover tooltip** | lines 1134-1148 | On name hover: last met date, meeting count, assessment | Quick reference without expanding | No | Nice progressive disclosure |
| 38 | **Temperature dot + label** | lines 1155-1170 | Colored dot (hot=sage, warm=turmeric, cool=grey, cold=terracotta) + label | Relationship health at a glance | Uses system vocabulary ("hot/warm/cool/cold") — these are internal signal terms | Useful but the terminology is system vocabulary |
| 39 | **Engagement badge** | lines 1171-1186 | Bordered pill with "champion" / "detractor" / "neutral" | Stakeholder disposition | No | Useful — but "champion/detractor" are system/sales vocabulary |
| 40 | **"New contact" label** | lines 1187-1198 | Green text "New contact" when meetingCount === 0 | Flag first-time attendees | No | Yes — knowing you haven't met someone is valuable |
| 41 | **Assessment text** | lines 1201-1217 | Serif italic, truncated to 200 chars | AI assessment of this person's disposition | No | Yes — this is the "killer insight" per the code comment |
| 42 | **Metadata line** (per attendee) | lines 1219-1242 | Organization, meeting count, last seen date | Supporting context | No | Meeting count and last seen are useful; organization less so if role already implies it |
| 43 | **Notes** (per attendee) | lines 1244-1256 | Italic tertiary text | Additional context | No | Rarely populated; acceptable as optional |
| 44 | **Hide attendee button** (x) | lines 1276-1311 | "x" button per row (editable meetings only) | User curation — dismiss irrelevant attendees | No | Nice for curation, but discoverable enough? The x is at 0.35 opacity. |
| 45 | **"+ N more" button** | lines 1316-1332 | Shows remaining count after first 4 | Prevent overwhelm | No | Yes — good progressive disclosure |

### Act II: Your Plan

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 46 | **ChapterHeading "Your Plan"** | line 861 | Rule + serif "Your Plan" | Section identity | No | Yes |
| 47 | **Prefill notice** | lines 863-878 | Turmeric left-border notice "Prefill appended new agenda/notes content." | Confirm prefill action | No | Transient feedback (5s) |
| 48 | **UnifiedPlanEditor** | lines 1341-1741 | Numbered list merging AI-proposed + user-authored agenda items | Core interactive element — the user's meeting plan | No | Yes — this is the only place the user's personal agenda lives |
| 49 | **Proposed agenda items** | Within UnifiedPlanEditor | Numbered items with topic + why, click-to-edit, dismiss (x) | AI suggestions the user can accept, edit, or dismiss | No | Yes |
| 50 | **User agenda items** | Within UnifiedPlanEditor | Same visual treatment as proposed, but user-authored | User's own agenda items | No | Yes |
| 51 | **Inline editing** (topic + why) | lines 1512-1564 | Two input fields: topic and "Why this matters..." | Edit existing items | No | Yes — core editability |
| 52 | **Ghost input** | lines 1642-1698 | Faded next-number + "Add agenda item..." placeholder + conditional "Why this matters..." | Add new agenda items | No | Yes |
| 53 | **Calendar Description toggle** | lines 1701-1738 | Collapsed `<details>` with chevron: "Calendar Description" | Show the original Google Calendar event description | No | Nice reference — keeps it out of the way but accessible |
| 54 | **"No agenda prepared yet."** | lines 1473-1478 | Italic tertiary fallback | Empty state for plan section | No | Uses system vocabulary ("agenda prepared") |

### Mid-Page FinisMarker

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 55 | **FinisMarker** (mid-page) | lines 895-897 | Three brand asterisks | Signal "You're briefed" — essential content ends here | There's a second FinisMarker at the very end (line 996-998) | Yes — important pacing signal. Having TWO FinisMarkers is potentially confusing. |

### Act III: Deep Dive — Supporting Intelligence

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 56 | **"Supporting Intelligence" overline** | line 905 | Mono uppercase "Supporting Intelligence" | Section identity for optional deep-dive content | No | Uses system vocabulary ("Intelligence"). Could be "Background" or "Reference Material." |
| 57 | **Recent Wins** | lines 909-924 | Sage-accented bullet list, max 4 | Positive momentum to reference in conversation | Also derived in daily briefing prep grid ("Wins" column) | Useful but overlaps with daily briefing and outcomes. If the meeting has occurred, wins appear in outcomes too. |
| 58 | **Open Items** | lines 927-940 | `ActionItem` list with bullet dots, due dates, overdue indicators | Existing action items related to this meeting/account | Actions page shows all actions; entity detail shows entity-scoped actions | Useful for pre-meeting awareness of open commitments |
| 59 | **Email Signals** | lines 943-984 | Signal type badge (mono uppercase) + date + signal text, max 4 | Recent email-derived intelligence about this account/meeting | No dedicated email surface exists | Useful but niche — only populated when email scanning is active |
| 60 | **AppendixSection toggle** | lines 2200-2226 | Collapsed "Open Supporting Context" button | Gate for reference material so it doesn't overwhelm | No | Good pattern — keeps optional content hidden by default |

### Appendix Sub-Sections (All Behind Toggle)

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 61 | **Full Intelligence Summary** | lines 2229-2257 | Multi-paragraph text with copy button | Complete AI assessment (the key insight is the first sentence of this) | Entity detail pages may show similar | Marginal — the key insight already surfaces the most important part |
| 62 | **Since Last Meeting** | lines 2259-2275 | Bullet list with turmeric dots + copy button | What happened between the last meeting with this account and now | No dedicated surface for this | Useful for recurring meetings; less useful for first-time meetings |
| 63 | **Strategic Programs** | lines 2277-2301 | Bullet list with checkmark/circle icons + copy button | Account's active strategic programs | Entity detail page likely shows similar | Marginal for meeting prep. More relevant on entity detail. |
| 64 | **Full Context** | lines 2303-2323 | Pre-formatted text block + copy button | Raw meeting context (the full version of what the key insight summarizes) | No | Marginal — who reads the full context after seeing the key insight? |
| 65 | **Current State** | lines 2325-2337 | Bullet list + copy button | Current state of the account/relationship | Entity detail shows this | Duplicated with entity detail. Consider: does this belong here? |
| 66 | **Questions to Surface** | lines 2339-2367 | Numbered list + copy button | AI-suggested questions to ask during the meeting | No | Useful for meeting prep, but overlaps with the "Your Plan" section's proposed agenda |
| 67 | **Key Principles** | lines 2370-2396 | Blockquote list with turmeric border + copy button | Guiding principles for the account relationship | Entity detail likely shows similar | Marginal for meeting prep |
| 68 | **Extended Stakeholder Map** | lines 2398-2406 | `StakeholderInsightList` for people not in the meeting attendee list | Broader relationship context beyond attendees | Entity detail shows person relationships | Useful for complex account situations, but this is deep reference material |
| 69 | **References** | lines 2408-2417 | `ReferenceRow` list — label, path, lastUpdated | Source file references for the intelligence | No | Meta-information. Useful for transparency but not for meeting prep. |

### Final FinisMarker

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 70 | **FinisMarker** (end of page) | lines 996-998 | Three brand asterisks | Signal end of document | Duplicate of the mid-page FinisMarker (#55) | Having two FinisMarkers dilutes the "you're done" signal |

### Supporting Components (Dialogs / Overlays)

| # | Element | Location | What it renders | Job it serves | Duplicated? | Essential? |
|---|---------|----------|-----------------|---------------|-------------|------------|
| 71 | **AgendaDraftDialog** | lines 1003-1010, component in `agenda-draft-dialog.tsx` | Modal dialog with generated email subject + body, copy button | Draft an agenda email to send to attendees | No | Useful but secondary — the user still has to manually paste and send |
| 72 | **EntityPicker** (within MeetingEntityChips) | Imported via `entity-picker.tsx` | Dropdown picker for linking accounts/projects/people | Correct or add entity links | Daily briefing expansion also has this | Essential for entity resolution correction |

---

## Conditional States Summary

The page has significant conditional rendering based on temporal state and data availability:

| Condition | What changes |
|-----------|-------------|
| **Future meeting, editable** | Full briefing mode: urgency banner possible, entity chips editable, plan editor with ghost input, attendee hide buttons, prefill button in folio, FinisMarker at mid-page |
| **Future meeting, no prep data** | Empty state: "Prep not ready yet" |
| **Future meeting, prep generating** | Empty state: "Prep is being generated" |
| **Past meeting, no outcomes** | Transcript prompt banner with "No outcomes captured yet" + Attach Transcript CTA; pre-meeting content shown at 0.7 opacity |
| **Past meeting, with outcomes** | Outcomes section at top (summary, wins, risks, decisions, actions); pre-meeting content below at 0.7 opacity with "Pre-Meeting Context" heading |
| **Past meeting, Quill sync in progress** | QuillSyncBadge shows processing state (pending/polling/fetching/processing) |
| **Past meeting, Quill sync complete** | QuillSyncBadge shows completed state with match confidence |
| **Meeting in < 2 hours** | Urgency banner appears (turmeric if > 15min, terracotta if <= 15min) |
| **Has intelligence summary** | Key insight pull quote renders (first sentence extracted) |
| **No intelligence summary** | Fallback text: "Intelligence builds as you meet with this account." |
| **Has entity readiness items** | "Before This Meeting" checklist renders |
| **Has risks** | "The Risks" chapter renders (featured + subordinate treatment) |
| **Has attendees** | "The Room" chapter renders |
| **Has agenda items OR is editable** | "Your Plan" chapter renders |
| **Has deep-dive content** | "Deep Dive" section renders (wins, open items, email signals, appendix) |
| **Has appendix content** | Appendix toggle renders (collapsed by default) |

---

## Key Observations

### 1. Two FinisMarkers Dilute the Completion Signal
The page renders a FinisMarker after "Your Plan" (Act II, line 895) AND at the very end (line 996). The first one says "You're Briefed" — the second one says... "You're Briefed Again"? This undermines the editorial pacing. If the job of FinisMarker is to signal "you can stop reading," having two is contradictory.

**Recommendation:** One FinisMarker after "Your Plan." The Deep Dive section exists below it but doesn't need its own finis.

### 2. System Vocabulary Throughout
Multiple user-visible strings use system vocabulary that ADR-0083 flags for translation:
- Folio label: "Intelligence Report" (should be "Meeting Briefing")
- Kicker: "Meeting Intelligence Report" (should be "Meeting Briefing")
- Empty state: "Prep not ready yet" (should not reference "prep")
- Empty state: "AI enrichment completes" (should be invisible or "Building context")
- Fallback text: "Intelligence builds as you meet..." (system vocabulary)
- Section label: "Supporting Intelligence" (should be "Background" or "Reference")
- Daily briefing bridge link: "Read full intelligence" (should be "Read full briefing")

### 3. Appendix May Be Overloaded
The appendix contains 9 sub-sections (Full Intelligence Summary, Since Last Meeting, Strategic Programs, Full Context, Current State, Questions to Surface, Key Principles, Extended Stakeholder Map, References). Many of these overlap with entity detail pages. The appendix's job is unclear — is it "raw material for the curious" or "things that didn't fit elsewhere"?

**Observation:** Questions to Surface overlaps with proposed agenda items in "Your Plan." Strategic Programs, Current State, and Key Principles are entity-level data more suited to the account/project detail page. The appendix may be a dumping ground for data that doesn't have a clear home.

### 4. Folio Bar Action Density
The folio bar can show 4 buttons simultaneously: Prefill, Draft Agenda, Transcript, Refresh. For a bar meant to be unobtrusive, this is dense. The "Prefill" action is especially obscure — it copies AI-proposed agenda items into the user's editable layer, which is a workflow detail most users won't understand.

### 5. Duplicate Transcript Attachment
For past meetings, the Attach Transcript button appears in *both* the folio bar (always visible) and the body (prominent dashed-border CTA). The body version is more contextual and discoverable. The folio bar version is redundant.

### 6. The Opacity Dimming Pattern
When outcomes exist (past meeting), the pre-meeting prep content is rendered at `opacity: 0.7` (line 620). This is a smart signal that the pre-meeting content is now secondary. However, it's subtle — a user might think the content is broken rather than intentionally dimmed.

### 7. Entity Readiness vs. Meeting Actions Overlap
"Before This Meeting" (entity readiness items from intelligence, element #31) and the daily briefing's "Before this meeting" action checklist serve the same job. The intelligence-derived readiness items are contextual suggestions; the daily briefing's action checklist contains actual tracked actions. Having both "before this meeting" lists across surfaces without clear distinction could confuse.

### 8. Progressive Disclosure Works Well
The three-act structure (Ground Me / Brief Me / Go Deeper) with editorial-reveal animations and a collapsed appendix is strong editorial pacing. The mid-page FinisMarker (despite the duplication issue) correctly signals "you've read enough."

### 9. Copy Buttons on Deep-Dive Sections
The deep-dive and appendix sections include copy-to-clipboard buttons (via `CopyButton` and `SectionLabel`). This suggests a workflow of copying intelligence into emails or documents. This is a real job ("I need to share this context with someone") that the surface serves well.

### 10. Attendee Data Merging Complexity
`buildUnifiedAttendees` (lines 2530-2587) merges four data sources: `attendeeContext`, `attendees`, `stakeholderInsights`, and `stakeholderSignals`. This complexity is invisible to the user but means the attendee display can be inconsistent depending on which sources populated. The code handles this well, but the variety of possible states (has assessment but no temperature, has meeting count but no last seen, etc.) makes the per-attendee display highly variable.

---

## Duplication Map (Meeting Detail vs. Other Surfaces)

| Element | Meeting Detail | Daily Briefing | Entity Detail | Actions Page |
|---------|---------------|----------------|---------------|-------------|
| Meeting title | Hero (76px) | Schedule row | Timeline entry | Action's meeting context |
| Entity chips | Full chips with add/remove | Compact chips in expansion | Entity is the page itself | N/A |
| Key people | Full UnifiedAttendeeList | KeyPeopleFlow (compact) | Person relationships section | N/A |
| Risks | Full chapter with featured treatment | Single "Watch" line in expansion | Entity risks section | N/A |
| Agenda/Plan | Full interactive editor | N/A | N/A | N/A |
| Actions (from outcomes) | OutcomeActionRow with triage | "X actions captured" count on past meetings | Timeline shows actions | Full action rows with all states |
| Recent Wins | Bullet list in deep dive | Single "Win" line in expansion | Entity wins | N/A |
| Entity readiness | "Before This Meeting" checklist | "Before this meeting" action checklist (different data source) | N/A | Related actions |
| Email signals | Compact list in deep dive | N/A | N/A | N/A |
| Transcript sync | QuillSyncBadge | N/A | N/A | N/A |
