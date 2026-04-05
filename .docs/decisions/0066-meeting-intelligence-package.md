# ADR-0066: Meeting Intelligence Package — Persistent Prep, Unified Card-to-Detail

**Date:** 2026-02-11
**Status:** Proposed
**Deciders:** James, Claude
**Relates to:** ADR-0063 (MeetingPreview Type), ADR-0064 (Prep Page Redesign), ADR-0065 (Editability)

## Context

### The prep disappears from the card after a meeting

MeetingCard on the dashboard has a Collapsible expansion that shows inline prep summary (At a Glance, Discuss, Watch, Wins). When a transcript is attached and outcomes are captured, the expansion replaces prep with outcomes (`MeetingOutcomes` renders *instead of* `MeetingPrepContent` at MeetingCard.tsx:623-631). The "View Prep" button also vanishes for past-processed meetings.

The data is still in SQLite — `prep_context_json` survives permanently in `meetings_history` via COALESCE upsert. But the UI discards it once outcomes arrive. The user can't get back to "what did I know going in?" from the meeting card.

### The card and detail page are visually disconnected

The MeetingCard inline expansion shows a 4-section colored grid (compact, scannable). The full MeetingDetailPage shows 14 stacked Cards (verbose, reference-oriented). Different components, different data types (`MeetingPrep` vs `FullMeetingPrep`), different visual language. They don't feel like views into the same meeting — they feel like two different features.

### No single "meeting record" exists

Today a meeting's intelligence is scattered:
- **Prep context**: `meetings_history.prep_context_json` (AI-generated before meeting)
- **Outcomes**: `meeting_outcomes` table (captured from transcript after meeting)
- **Actions**: `actions` table, linked by `meeting_id`
- **Captures**: `meeting_captures` table (wins, risks, decisions from transcript)
- **User agenda/notes**: proposed in ADR-0065 (not yet implemented)

There's no unified concept of "everything we know about this meeting." The MeetingCard is the closest thing, but it shows either prep OR outcomes, never both. The detail page only shows prep — it has no post-meeting view.

### What users need: a meeting package they can look back on

Before a meeting: "What should I know going in?" → prep
After a meeting: "What happened and what did we commit to?" → outcomes
Later: "What was the full picture for that meeting with Acme last month?" → both

The meeting card should be a time capsule — prep context, user notes, agenda, outcomes, captures, actions — all in one place. Not a flat dump, but a narrative: what we knew → what happened → what we committed to.

## Decision

### The Meeting Intelligence Package

Every meeting has a unified intelligence package that accumulates over its lifecycle:

| Phase | Content | Source | Mutable? |
|-------|---------|--------|----------|
| **Pre-meeting** | Intelligence brief, account snapshot, agenda (user/calendar/AI), user notes, talking points, risks, open items, people context | Prep enrichment + user input | Until meeting ends |
| **During** | Live notes (future), agenda adjustments | User input | During meeting |
| **Post-meeting** | Summary, wins, risks, decisions, actions, transcript link | Outcome capture | Until next enrichment |
| **Permanent** | All of the above, frozen | — | Read-only |

The package grows — nothing is discarded. Pre-meeting prep doesn't disappear when outcomes arrive; outcomes augment the prep.

### Card Expansion: Phased, Not Replaced

The MeetingCard expansion changes behavior:

**Current (broken):**
- Has prep, no outcomes → show MeetingPrepContent
- Has outcomes → show MeetingOutcomes (prep hidden)

**Proposed:**
- Has prep, no outcomes → show prep summary (current behavior, but aligned with Tier 1 from ADR-0064)
- Has outcomes AND prep → show outcomes first, then collapsed prep summary underneath
- Past meeting, no outcomes, has prep → show prep summary + "Attach Transcript" prompt
- Past meeting, no prep, no outcomes → minimal record (title, attendees, date)

The card always shows the **most actionable layer on top** — outcomes if they exist (because actions need attention), prep underneath for reference. Both are always accessible.

### Unified Visual Language: Card ↔ Detail

The card expansion is a **compressed version** of the detail page, not a different view. The same information hierarchy from ADR-0064 applies at both levels:

| ADR-0064 Tier | Card Expansion (compressed) | Detail Page (full) |
|---------------|----------------------------|-------------------|
| **Tier 1: Walk-In Frame** | Intelligence brief (2 lines) + 4 snapshot pills | Full brief + 6-8 snapshot items |
| **Tier 2: Agenda** | First 3 agenda items (one line each) | Full annotated agenda with context |
| **Tier 3: Deep Context** | Not shown (detail page territory) | People, history, programs, references |
| **Outcomes** | Summary + win/risk/decision counts + action list | Full outcomes with all detail |

The card is Tier 1 + compressed Tier 2 + outcomes summary. The detail page is all three tiers plus full outcomes. "View Full Prep" on the card opens the detail page — the user mentally zooms in, not switches context.

### Detail Page Becomes the Meeting Record

MeetingDetailPage evolves from "prep viewer" to "meeting record." It shows the full intelligence package:

**Before meeting:**
- Tier 1: Walk-In Frame (intelligence brief + account snapshot)
- Tier 2: Agenda (annotated) or signal summary
- Tier 3: Deep Context (collapsible)
- (Outcomes section empty or hidden)

**After meeting:**
- **Outcomes section** promoted to top (summary, wins, risks, decisions, actions)
- Tier 1: Walk-In Frame (what we knew going in — read-only, historical)
- Tier 2: Agenda (what was planned — read-only)
- Tier 3: Deep Context (reference)

The page flips its hierarchy post-meeting: outcomes first (what happened), then prep (what we expected). The prep becomes historical context rather than actionable intelligence.

### Routing: ID-Based, Not File-Based

Current routing uses `prepFile` (a filename like `01-1630-customer-acme-prep.md`). This breaks after archive cleanup deletes the file. The detail page should route by meeting ID:

**Current:** `/meeting/$prepFile` → loads from `_today/data/preps/{file}.json`
**Proposed:** `/meeting/$meetingId` → loads from `meetings_history` by ID

This means:
- Today's meetings: load prep from `_today/data/preps/` (fresh, pre-archive)
- Past meetings: load from `meetings_history.prep_context_json` (persisted)
- The page works regardless of whether the disk file still exists

This is a breaking route change. The MeetingCard's "View Prep" link switches from `prepFile` param to `meetingId`. Meeting history links from AccountDetailPage (ADR-0063's MeetingPreview) already use `meeting.id`.

### Data Loading: Unified Command

Replace the current `get_meeting_prep` (file-based) with a unified command:

```rust
#[tauri::command]
fn get_meeting_intelligence(meeting_id: String) -> Result<MeetingIntelligence>
```

Returns:
```rust
pub struct MeetingIntelligence {
    // Meeting identity
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub meeting_type: String,
    pub account_id: Option<String>,
    pub account_name: Option<String>,

    // Pre-meeting intelligence (ADR-0064 tiers)
    pub prep_context: Option<PrepContext>,
    pub calendar_description: Option<String>,  // ADR-0064 Phase 1
    pub user_agenda: Option<Vec<String>>,       // ADR-0065
    pub user_notes: Option<String>,             // ADR-0065

    // Post-meeting intelligence
    pub outcomes: Option<MeetingOutcomes>,
    pub captures: Vec<MeetingCapture>,
    pub actions: Vec<Action>,

    // People context
    pub attendees: Vec<AttendeeContext>,

    // Metadata
    pub is_past: bool,
    pub has_transcript: bool,
}
```

One command, one type, one page. The detail page renders sections conditionally based on what's populated. No more separate `get_meeting_prep` + `get_meeting_outcomes` + `get_meeting_attendees` calls.

### Persistence Lifecycle

| Event | What Happens | Data Location |
|-------|-------------|---------------|
| Meeting appears on calendar | `meetings_history` row created | SQLite |
| Daily briefing runs | Prep enriched, `_today/data/preps/*.json` created | Disk (ephemeral) |
| User adds agenda/notes | `user_agenda_json` + `user_notes` written | SQLite |
| Meeting happens | No automatic change | — |
| Transcript attached | Outcomes captured, stored in `meeting_outcomes` | SQLite |
| Archive runs | Prep JSON read from disk → `prep_context_json` in DB → disk files deleted | SQLite (permanent) |
| User views past meeting | `get_meeting_intelligence` reads all from SQLite | SQLite |

Nothing is lost. The disk files are a performance optimization (avoid DB reads for today's data). SQLite is the permanent record.

## Consequences

### Easier
- Users can look back at any past meeting and see the complete picture — prep + outcomes + actions
- The card expansion becomes a true preview of the detail page, not a different interface
- ID-based routing means meeting records work indefinitely, not just until archive cleanup
- One command replaces 3+ separate data fetches for the detail page
- The meeting record grows over time rather than being a snapshot

### Harder
- Route migration from `prepFile` to `meetingId` is a breaking change (all meeting links need updating)
- `MeetingIntelligence` is a larger struct — more data per request (but it's all local SQLite, sub-millisecond)
- Card expansion needs to render both prep summary AND outcomes — layout complexity increases
- Detail page needs two rendering modes (pre-meeting focus vs post-meeting focus)
- `get_meeting_intelligence` command needs to join across 4+ tables

### Trade-offs
- Unified command is convenient but couples the detail page to a single large query. If perf becomes an issue, we can split into lazy-loaded sections later.
- Post-meeting prep becomes read-only — the user can't retroactively edit their pre-meeting notes. This is intentional: the prep is a historical record of "what we knew going in." Post-meeting reflections go to outcomes.
- Card expansion showing both prep and outcomes may feel busy for meetings with rich data. The hierarchy (outcomes on top, prep collapsed underneath) should mitigate this, but we'll need to test.

## Implementation Phases

**Phase 1: Route Migration** (mechanical, no new features)
- Change route from `/meeting/$prepFile` to `/meeting/$meetingId`
- Update MeetingCard "View Prep" link to use meeting ID
- Update AccountDetailPage MeetingPreview links
- Create `get_meeting_intelligence` command (initially just wrapping existing queries)
- Fallback: if meeting has prep file on disk (today's meetings), load from there; otherwise from DB

**Phase 2: Card Expansion Unification** (frontend)
- Card shows outcomes + collapsed prep when both exist
- Card shows prep summary when no outcomes
- "View Full Record" replaces "View Prep" as the action button text (works for both states)
- Align card prep summary visual language with ADR-0064 Tier 1

**Phase 3: Detail Page as Meeting Record** (frontend + backend)
- Detail page renders from `MeetingIntelligence` struct
- Pre-meeting mode: prep-focused layout (ADR-0064 tiers)
- Post-meeting mode: outcomes on top, prep as historical context below
- Attendee context, actions, captures all on one page

**Phase 4: Full Intelligence Command** (backend optimization)
- Consolidate `get_meeting_prep` + `get_meeting_outcomes` + attendee queries into single `get_meeting_intelligence`
- Join across meetings_history, meeting_outcomes, actions, meeting_captures, people
- Return `MeetingIntelligence` struct with all sections populated

## 2026-02-18 Alignment Note (ADR-0081)

ADR-0081 (Event-Driven Meeting Intelligence, 0.13.0) builds on this ADR and extends the `MeetingIntelligence` concept:

- **`IntelligenceQuality` field added** to `MeetingIntelligence`. Each meeting record carries a quality assessment: level (Sparse/Developing/Ready/Fresh), staleness (Current/Aging/Stale), signal count, coverage flags (has entity context, has attendee history, has recent signals), and "new signals since last view" tracking. This drives the UX badges that replace binary "needs prep" (I329).
- **Lifecycle starts at detection, not day-of.** This ADR's persistence lifecycle shows "Daily briefing runs → Prep enriched." ADR-0081 shifts this: intelligence enriches when the meeting is first detected on the calendar (via weekly run or calendar polling), not on the day of the meeting. The `MeetingIntelligence` record exists days to weeks before the meeting.
- **Signal-driven incremental updates.** Between initial enrichment and the meeting, signals (email, transcripts, entity updates, calendar changes) mark the intelligence record for refresh. The record grows continuously, not just at scheduled checkpoints.
- **Implementation:** I326 implements the lifecycle. Phase 1 of this ADR (route migration to `meetingId`, `get_meeting_intelligence` command) is a prerequisite and should ship first or concurrently with I326.
