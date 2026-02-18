# ADR-0064: Meeting Prep Page Redesign — Briefing-First, Agenda-Anchored

**Date:** 2026-02-11
**Status:** Proposed
**Deciders:** James, Claude
**Relates to:** ADR-0062 (Briefing Artifacts vs Live Queries), ADR-0057 (Entity Intelligence)

## Context

The meeting prep page (`MeetingDetailPage.tsx`) is the highest-value real estate in DailyOS. Every customer meeting gets a prep; the user reads it before walking in. Today the page has three categories of problems:

### 1. Missing data: Calendar description is discarded

Google Calendar event descriptions are fetched (`calendar.rs:206`) but stripped at `orchestrate.rs:462-480` when building `lean_events`. The `meetings_history` table has no `description` column. The prep pipeline never sees them.

This means:
- Agendas pasted into calendar invites are invisible to prep
- Google Doc links attached to events are not followed
- Meeting organizer notes are lost
- Location context (in-person vs remote) isn't used

If a user has already written an agenda for a meeting, the AI generates a competing one from scratch. This violates the Prime Directive — the system should leverage what's already there, not duplicate effort.

### 2. Clunky information hierarchy

The current page renders 11+ optional Card sections stacked vertically with equal visual weight:

1. Proposed Agenda
2. Quick Context (CSM + Lifecycle only, renders raw markdown)
3. Relationship Context
4. Context (prose blob)
5. People in the Room
6. Since Last Meeting
7. Strategic Programs
8. Current State
9. Risks
10. Talking Points
11. Open Items
12. Questions
13. Key Principles
14. References

Every section is a Card with an icon, title, and copy button. There's no narrative hierarchy — risks and talking points get the same visual weight as key principles. The page reads like a database dump, not a briefing.

From the UX research (2026-02-08): "The curation IS the value. Ruthless prioritization — intelligence experts condense all signals into a few pages. The briefing has a beginning, a middle, and an end."

### 3. Quick Context is too thin

Quick Context currently pulls only from account metadata labels: `lifecycle`, `arr`, `renewal`, `health`, `tier`, `csm`, `stage`. These are static fields. It shows nothing from entity intelligence — no recent wins, no active risks, no strategic assessment. For an account with rich intelligence, the prep's "Quick Context" card shows "Lifecycle: Growth" and "CSM: James" while the account detail page shows a full executive assessment.

### 4. No editability, no persistence

The prep page is read-only. If the user has their own notes, agenda additions, or context to add before a meeting, there's nowhere to put them. The prep is regenerated each morning — any manual enrichment would be overwritten. There's no user layer that survives re-enrichment.

### 5. Empty states are unhelpful

When a section is empty (which is common for less-enriched accounts), the section simply doesn't render. This leaves gaps in the reading flow. For a new account with no history, the prep might show just Quick Context + Attendees — which feels broken rather than "we'll learn more as we go."

## Decision

### Core Principle: Agenda-Anchored Prep

If a meeting has an agenda (from Google Calendar description, user-authored, or AI-generated), the **agenda is the organizing principle**. Every other signal (risks, talking points, open items, intelligence) should support the agenda — providing context for why each item matters, surfacing risks relevant to specific topics, connecting open items to agenda items.

If no agenda exists, the prep falls back to the current signal-based layout (risks, talking points, questions) — but these should still be structured as "what to walk in knowing" rather than a flat list.

### Information Hierarchy (Three Tiers)

**Tier 1 — Walk-In Frame** (always visible, top of page)
This is what the user reads in 15 seconds before stepping into the room:
- **Meeting headline**: title, time, account, attendee count
- **Intelligence brief**: 2-3 sentence synthesis from entity intelligence (not the full executive assessment — a meeting-specific distillation). "Acme is in renewal evaluation. The QBR deck was well-received but they've raised concerns about API latency. Champion is supportive but CTO is skeptical."
- **Account snapshot**: Health, ARR, lifecycle, renewal date, latest win, active risk — max 6 items, not raw markdown. This replaces the current Quick Context.

**Tier 2 — The Agenda Layer** (primary content)
If an agenda exists (from any source):
- Numbered agenda items, each annotated with supporting context:
  - Talking points relevant to this topic
  - Open items/actions that connect to this topic
  - Risk callouts if the topic intersects a known risk
  - "Why this matters" context from entity intelligence
- If no agenda exists: AI-generated proposed agenda (current behavior) or a structured signal summary (risks + talking points + questions as a consolidated briefing section)

**Tier 3 — Deep Context** (available on demand)
Reference material for when the user wants to go deeper:
- People in the room (with roles, last interaction, assessment)
- Since last meeting (timeline of changes)
- Strategic programs (current status)
- Full open items list
- References and source files
- Calendar description (raw, if it contains useful notes beyond the agenda)

### Calendar Description Pipeline

**Schema change:** Add `description` TEXT column to `meetings_history`.

**Capture flow:**
1. `fetch_and_classify_today()` — carry `description` through `classified` and `events` arrays
2. `deliver_schedule()` — include `description` in meeting JSON
3. `upsert_meeting()` — persist to `meetings_history.description`
4. `gather_meeting_context()` — include in prep context assembly
5. `deliver_preps()` — pass to AI enrichment prompt with explicit instruction:

> "If the meeting has an existing agenda or description from the calendar invite, treat it as the primary organizing structure. Enrich around it — add talking points that support each agenda item, flag risks relevant to specific topics, connect open items to agenda topics. Do NOT generate a competing agenda."

**Agenda source priority:**
1. User-authored agenda (future: `user_notes` field on meeting, survives re-enrichment)
2. Calendar description (if it contains agenda-like content)
3. AI-generated proposed agenda (current behavior, fallback only)

### Quick Context Enrichment

Replace the current label-only Quick Context with a richer "Account Snapshot" that pulls from entity intelligence:

| Signal | Source | Condition |
|--------|--------|-----------|
| Health | `account.health` | Always if available |
| ARR | `account.arr` | Always if available |
| Lifecycle | `account.lifecycle` | Always if available |
| Renewal | `account.contract_end` | If within 180 days |
| Latest win | `intelligence.recentWins[0]` | If exists, truncated |
| Active risk | `intelligence.risks[0]` (highest urgency) | If exists, truncated |
| Relationship temp | `stakeholderSignals.temperature` | Always |
| Days since contact | `stakeholderSignals.lastMeeting` | If > 14 days |

Cap at 6-8 items. Render as clean key-value pairs, not raw markdown.

### AI Enrichment Safety

**Additive, not destructive.** When AI enrichment runs:
- Calendar description is READ-ONLY input — AI never modifies or replaces it
- User notes (future) are READ-ONLY input — AI incorporates but never overwrites
- AI-generated content is clearly attributed (no mixing AI prose with user-authored text)
- If the user has authored an agenda, AI annotates it; it does not replace it

**Idempotent re-enrichment.** Running enrichment again should produce consistent results. The user's inputs (calendar description, user notes) are always preserved. AI outputs are always regenerated fresh from current signals.

### Empty States

Instead of hiding empty sections, show contextual placeholders that explain what would appear:

| Section | Empty State |
|---------|-------------|
| Intelligence brief | "Intelligence builds as you meet with this account. After your first meeting, context will appear here." |
| Agenda | "No agenda found. Add one to the calendar event description, or the system will generate a proposed agenda from your account context." |
| Risks | "No active risks identified." (single line, not a Card) |
| People | "Attendee details populate after the people database recognizes participants." |

Empty states should be minimal — a single muted line, not a Card with an icon. They communicate "this will fill in" not "something is wrong."

### Editability (Future — Not This ADR)

This ADR acknowledges but defers user editability:
- **User-authored agenda/notes**: Requires a `user_notes` or `user_agenda` field on the meeting that survives re-enrichment. This is a separate data model decision.
- **Inline editing of prep fields**: Requires a merge strategy (user edits + AI enrichment). Complex. Defer.
- **Post-meeting capture on this page**: Already exists via MeetingHistoryDetail. Not duplicated here.

What IS in scope: the prep page should be **designed** to accommodate editability later. The layout should have clear visual separation between "system-generated" and "user-authored" zones.

## Consequences

### Easier
- Calendar descriptions stop being wasted data — users who maintain agendas in Google Calendar get immediate value
- Prep page reads as a briefing document, not a database query
- Intelligence brief gives the 15-second "walk-in frame" that users actually need
- Quick Context becomes genuinely useful with intelligence signals
- Empty states guide rather than confuse

### Harder
- Schema migration for `description` column (trivial, backward compatible)
- Plumbing `description` through 5 pipeline stages (mechanical, no design decisions)
- AI prompt needs "anchor to existing agenda" instruction — requires careful prompt engineering to avoid hallucination or agenda-mangling
- Information hierarchy requires the AI to generate meeting-specific distillations, not just echo the entity assessment
- Layout redesign touches every Card component on the page

### Trade-offs
- Richer Quick Context means more data fetched per prep — but it's all data we already have in SQLite/intelligence.json, just not piped through
- Agenda-anchored prep means AI outputs vary more based on input quality — a sparse calendar description produces less useful annotations than a well-structured agenda
- Three-tier hierarchy means some content moves "below the fold" into Tier 3 — power users who want everything visible will need to expand

## Implementation Phases

**Phase 1: Calendar Description Pipeline** (mechanical, no AI changes)
- Schema migration: add `description` to `meetings_history`
- Carry through fetch → classify → directive → DB
- Display raw description on prep page (Tier 3, "Calendar Notes" section)

**Phase 2: Quick Context Enrichment** (backend data, minimal AI)
- Build richer account snapshot from entity intelligence + signals
- Replace raw label rendering with clean key-value component
- Cap at 6-8 items, no markdown rendering

**Phase 3: Information Hierarchy Restructure** (frontend layout)
- Tier 1 header: headline + intelligence brief + account snapshot
- Tier 2: agenda (annotated) or signal summary
- Tier 3: collapsible deep context sections
- Empty states for all sections
- Remove Card wrappers from Tier 1 (briefing feel, not dashboard feel)

**Phase 4: Agenda-Anchored Enrichment** (AI prompt redesign)
- Detect agenda in calendar description
- Restructure AI prompt: "enrich around this agenda" vs "generate an agenda"
- Annotate agenda items with relevant signals
- Meeting-specific intelligence distillation (not echoed entity assessment)

**Phase 5: User Editability** (deferred, separate ADR)
- `user_notes` field on meeting
- Merge strategy for user + AI content
- Inline editing UI
