# ADR-0081: Event-Driven Meeting Intelligence

**Date:** 2026-02-18
**Status:** Accepted
**Participants:** James Giroux, Claude Code

**Builds on:** [ADR-0030](0030-weekly-prep-with-daily-refresh.md) (composable workflow operations), [ADR-0066](0066-meeting-intelligence-package.md) (meeting intelligence package), [ADR-0080](0080-signal-intelligence-architecture.md) (signal intelligence architecture)
**Enriches:** [ADR-0052](0052-week-page-redesign.md) (week page redesign — readiness checks evolve into intelligence quality indicators)
**Supersedes:** I220 (meeting forecast — subsumed by advance intelligence generation), I200/I202 (week page proactive suggestions/prep prefill — subsumed by live intelligence surface)
**Target:** v0.13.0

---

## Context

### The Schedule-Based Model

DailyOS runs on two clocks. A daily run at 6am generates individual meeting preps for today's meetings. A weekly run on Sunday/Monday generates a summary overview but writes no individual meeting preps. These are the only moments intelligence is created.

This creates four compounding problems:

**1. No advance preparation.** Meeting preps are born at 6am on the day of the meeting. A customer QBR on Thursday can't have its intelligence report on Monday. The user can't work on the agenda ahead of time, can't share it with colleagues, can't send it to the customer for input. The entire collaborative dimension of meeting preparation is locked out.

**2. "Needs prep" is meaningless.** `resolve_prep_status()` checks whether a prep file exists and has `proposedAgenda` or `talkingPoints`. On Monday morning looking at Thursday's meeting, the badge says "needs prep" — not because intelligence is inadequate, but because no file exists yet. The badge is a status indicator for the system's batch processing, not the user's preparedness.

**3. Internal meetings get second-class intelligence.** The classification in `classify.rs` routes meetings into three tiers: `account-prep` (full entity intelligence), `person-prep` (thinner context), and `no-prep` (personal, all-hands, training). Internal 1:1s and team syncs get person-prep at best. But with internal teams becoming accounts (ADR-0070), there's no reason an Engineering team standup shouldn't get entity-quality intelligence from the team entity.

**4. Intelligence is static within a day.** Between 6am generation and the meeting itself, new signals arrive — emails referencing the meeting, transcript insights from earlier meetings, entity intelligence updates, calendar changes. None of these update the meeting intelligence report. The user walks in with a snapshot from hours ago, potentially missing the most recent context.

### What Changed Since ADR-0030

ADR-0030 (Composable Workflow Operations) proposed exactly the right decomposition: `meeting:prep` as an independent atomic operation callable by any orchestrator, with the weekly orchestrator generating preps for the full week and calendar polling triggering preps for newly detected meetings. That was written February 2026 during the Rust migration.

The per-meeting prep operation exists — it's what `prepare_today()` calls internally. But it's locked inside the daily orchestrator. Nothing calls it independently. The weekly orchestrator (`prepare_week()`) generates only a summary `week-overview.json` with no individual preps. Calendar polling detects changes but doesn't trigger intelligence generation.

ADR-0066 (Meeting Intelligence Package) proposed the right data model: a unified `MeetingIntelligence` struct that accumulates over a meeting's lifecycle (pre-meeting → during → post-meeting → permanent), with ID-based routing and a single `get_meeting_intelligence` command.

ADR-0080 (Signal Intelligence Architecture) proposed the right event infrastructure: a signal bus where data sources produce typed, weighted, time-decaying signals that drive entity resolution, enrichment, and proactive hygiene.

The architecture is designed. This ADR implements the operational model that brings it to life.

### The Philosophical Shift

The user articulated this precisely: "Not every meeting gets an intelligence briefing and it should. An intelligence report, by its definition, is everything someone would need to know to create an agenda if they needed to. It's so they can walk into the room with confidence."

And critically: "Even meetings with no context should generate an intelligence report. It's our current place where we sync transcripts and generate intelligence for the next time. It might be related to an internal team or a project we're learning about. We won't know until we get there."

The intelligence report isn't just prep for walking in ready. It's the system's learning node for that meeting — the place where transcripts are processed, entities are discovered, and context accumulates for future encounters. Skipping the report means skipping the learning.

---

## Decision

### 1. Meeting Intelligence Is an Always-On Entity

Every meeting on the user's calendar (except all-hands: 50+ attendees) has a **meeting intelligence record** that is born when the meeting is detected and lives permanently. This is the `MeetingIntelligence` struct from ADR-0066, but with a crucial change: it's created *immediately on detection*, not on the day of the meeting.

**Lifecycle:**

| Stage | Trigger | What Happens |
|-------|---------|-------------|
| **Detection** | Calendar sync or poll detects new meeting | `meetings_history` row created. Classification runs. Entity resolution runs (I305). Skeleton intelligence record exists. |
| **Initial enrichment** | Async, within minutes of detection | `meeting:prep` (ADR-0030) generates intelligence: attendee context, entity intelligence, historical notes, agenda (calendar description or AI-generated), talking points, signals. Written to SQLite. |
| **Incremental enrichment** | Signal events (email, transcript, entity update, calendar change) | Intelligence record updated incrementally. New signals merged, not regenerated from scratch. |
| **Pre-meeting refresh** | 2 hours before meeting (I147) | Final enrichment pass with latest signals. Freshest possible intelligence. |
| **Post-meeting capture** | Transcript attached | Outcomes, decisions, actions, wins, risks captured. Intelligence record augmented, not replaced. |
| **Archive** | Nightly reconciliation | Ephemeral disk files cleaned up. SQLite record is permanent. |

The intelligence record grows — nothing is discarded. Pre-meeting intelligence doesn't disappear when outcomes arrive (ADR-0066's core decision). The record is always accessible via `get_meeting_intelligence(meeting_id)`.

### 2. Every Meeting Gets Intelligence

Expand classification to generate intelligence for all meetings except all-hands (50+ attendees).

**Current classification tiers:**

| Tier | Types | Intelligence |
|------|-------|-------------|
| Account-prep | customer, qbr, partnership, demo | Full entity intelligence |
| Person-prep | internal, team_sync, one_on_one | Thin person context |
| No-prep | personal, all_hands, training | Nothing |

**New classification:**

| Tier | Types | Intelligence |
|------|-------|-------------|
| Entity intelligence | customer, qbr, partnership, demo, team_sync (with internal team entity) | Full entity intelligence from associated account/project/team |
| Person intelligence | one_on_one, internal (small group, no entity association) | Attendee context, relationship history, open items, recent interactions |
| Minimal intelligence | training, personal | Minimal record — attendees, calendar description, any signals. Still generates for learning purposes. |
| Skip | all_hands (50+ attendees) | No intelligence. Badge count too high, entity resolution meaningless. |

The key change: `training` and `personal` no longer skip entirely — they get minimal records that serve as learning nodes for future encounters. `team_sync` meetings associated with internal team entities (ADR-0070) get full entity intelligence, not thin person-prep.

### 3. Advance Intelligence Generation

The weekly orchestrator generates individual meeting intelligence for every meeting in the forecast window (current week + next week = ~10 business days). The daily orchestrator no longer generates preps from scratch — it assembles from pre-computed intelligence.

**Weekly orchestrator (revised from ADR-0030):**

1. `calendar:fetch` (current week + next week, ~10 business days)
2. For each classified meeting without existing intelligence:
   - `meeting:prep` — generate intelligence, write to SQLite
3. For each meeting with existing intelligence older than 48 hours:
   - Incremental refresh with latest signals
4. Gap analysis + readiness summary
5. Write `week-overview.json` (summary, not individual preps)

**Daily orchestrator (revised):**

1. Check today's meetings — intelligence should already exist from weekly/polling
2. For any meeting missing intelligence (edge case: meeting added after last weekly run, before polling caught it):
   - `meeting:prep` — generate now
3. For today's meetings with intelligence older than 12 hours:
   - Signal-aware refresh (incorporate overnight email, entity updates)
4. `email:fetch`, `action:sync` — as before
5. Overview synthesis — narrative layer over pre-computed intelligence
6. Write `schedule.json`, `overview.json` etc.

**Calendar polling (reactive):**

- New meeting detected → `meeting:prep` within minutes
- Meeting changed (title, attendees, time) → incremental re-enrichment
- Meeting cancelled → intelligence record marked cancelled, freed from surfaces

### 4. Intelligence Quality Indicators

Replace binary "needs prep" with a multi-dimensional quality indicator that communicates what the system knows and doesn't know.

**Current:** `resolve_prep_status()` → `prep_needed | prep_ready | context_needed | done`

**New:** `assess_intelligence_quality()` → structured quality assessment:

```rust
pub struct IntelligenceQuality {
    pub level: QualityLevel,       // Sparse, Developing, Ready, Fresh
    pub signal_count: u32,         // How many signals contributed
    pub last_enriched: String,     // When intelligence was last updated
    pub has_entity_context: bool,  // Associated with an entity?
    pub has_attendee_history: bool, // Prior interactions with these people?
    pub has_recent_signals: bool,  // Email, transcript, or entity signals in last 48h?
    pub staleness: Staleness,      // Current, Aging, Stale
}

pub enum QualityLevel {
    Sparse,     // Minimal: calendar title + attendees, no enrichment yet
    Developing, // Some context: entity linked, basic history, few signals
    Ready,      // Good: entity intelligence, attendee context, agenda, signals
    Fresh,      // Just enriched: latest signals incorporated, confident
}

pub enum Staleness {
    Current,  // Enriched within last 12 hours
    Aging,    // Enriched 12-48 hours ago
    Stale,    // Enriched 48+ hours ago — refresh recommended
}
```

**UI badges:**

| Quality | Badge | Color | Meaning |
|---------|-------|-------|---------|
| Sparse | "Sparse" | Muted/grey | System has little context — will learn from this meeting |
| Developing | "Building" | Amber | Intelligence accumulating, not yet rich |
| Ready | "Ready" | Sage/green | Good intelligence, confident prep |
| Fresh | Checkmark | Sage/green | Just refreshed, latest signals incorporated |
| Stale+Ready | "Ready" + refresh icon | Sage with indicator | Good intelligence but aging — tap to refresh |
| New signals | Blue dot | Larkspur | New information since last viewed |

The "new signals" dot is critical: it communicates that the system learned something since you last looked. This replaces the vague "needs prep" with actionable information — "there's something new to review."

### 5. Weekly Forecast as Live Intelligence Surface

The weekly forecast transforms from a static overview document into the primary surface for interacting with upcoming meeting intelligence.

**Current:** Week page shows day shapes (density bars), meeting list with "needs prep" badges, readiness checks, actions. Generated once (Sunday/Monday), static until next generation.

**New:** Week page becomes a meeting intelligence browser:

- Each meeting shows its intelligence quality badge and last-enriched timestamp
- Clicking a meeting opens its intelligence report (same `MeetingDetailPage`, same route)
- Meetings with new signals since last view show a blue dot
- Readiness checks (ADR-0052) evolve from "no agenda" / "no prep" to intelligence-quality-driven: "3 meetings with sparse context" / "2 meetings with stale intelligence"
- The page updates throughout the week as signals arrive and intelligence accumulates

**The narrative shifts.** The week narrative (AI-generated) goes from "you have 12 meetings this week" to "your week centers on the Acme renewal Thursday and cross-functional sync Wednesday. Both have strong intelligence. The Friday intro meeting with Globex has sparse context — the system is still learning about this relationship."

**Live vs. static:** The week overview summary (`week-overview.json`) regenerates on the weekly run and on-demand. But individual meeting intelligence is always live — fetched directly from SQLite via `get_meeting_intelligence()`. The overview provides the narrative frame; meetings provide the detail. The overview can go stale without the meetings going stale.

### 6. Daily Briefing as Intelligence Assembly

The daily briefing's relationship with meeting intelligence inverts. Today it *generates* intelligence. In the new model, it *assembles* and *narrates* pre-existing intelligence.

**What changes:**

| Aspect | Current | New |
|--------|---------|-----|
| Meeting preps | Generated during `prepare_today()` | Pre-exist from weekly/polling; daily run does freshness check + signal refresh |
| "Run Briefing" button | Triggers full pipeline including prep generation | Triggers signal refresh + narrative regeneration (faster, cheaper) |
| Briefing generation time | 3-5 minutes (heavy: calendar fetch + classification + per-meeting AI enrichment) | 30-60 seconds (light: signal diff + narrative synthesis over pre-computed intelligence) |
| Meeting section | Schedule list with "needs prep" badges | Schedule list with intelligence quality badges + "new signals" dots |
| Stale intelligence | Can't refresh without re-running entire briefing | Per-meeting refresh available — tap a meeting, tap refresh |

**The "Run Briefing" question.** Does the landing page with "Run Briefing" still make sense?

In the event-based model, there's less need for a big-bang briefing run. Intelligence accumulates continuously. The daily run still has value for:
- Email fetch + classification (inherently daily)
- Action sync (overnight changes)
- Overview narrative synthesis ("here's what's changed overnight for today's meetings")
- Signal freshness check for today's meetings

But the *scope* shrinks. The daily run becomes a "morning refresh" — a lightweight pass that incorporates overnight signals and generates the editorial narrative. It doesn't need to generate meeting intelligence from scratch.

**Implications for the landing page:** The "Run Briefing" button could evolve into:
- **Implicit:** Briefing runs automatically in the background on app launch. No button needed. The user opens the app and the day is ready (aligns with "AI produces, people benefit").
- **Explicit refresh:** A subtle "Refresh" action in the briefing header for on-demand signal incorporation. Not a "generate from scratch" action, but a "check for new signals and update."
- **Per-meeting refresh:** On the meeting detail page, a "Refresh Intelligence" action that re-enriches that specific meeting with the latest signals.

The "no briefing yet" landing page remains necessary for first-of-day cold start (email hasn't been fetched, actions haven't synced, narrative hasn't been written). But if intelligence accumulates continuously, the app is never truly "empty" — meetings always have their intelligence records even before the morning run.

### 7. Signal-Triggered Intelligence Refresh

Connects to I308 (Event-driven signal processing). When specific events occur, affected meeting intelligence refreshes incrementally:

| Signal | Effect |
|--------|--------|
| New email referencing meeting attendees/entity | Meeting intelligence updated with email signal |
| Earlier meeting produces transcript mentioning later meeting's entity | Cross-meeting intelligence propagation |
| Calendar change (new attendee, time change, description updated) | Meeting re-classified, entity re-resolved, intelligence refreshed |
| Entity intelligence updated (new risk, health change, win captured) | All meetings associated with that entity get freshness flag |
| User edits agenda/notes on meeting | Intelligence incorporates user input on next refresh |

Not every signal triggers full re-enrichment (that would be expensive). The system:
1. Records the signal in `signal_events` (ADR-0080)
2. Marks affected meeting intelligence as "has new signals"
3. Shows the blue dot on meeting cards/badges
4. Full re-enrichment happens on next scheduled refresh (pre-meeting refresh, daily run, or user-triggered)

This is **eventual consistency for meeting intelligence**: signals arrive continuously, enrichment happens at natural checkpoints, the user sees "new signals available" and can trigger refresh if they want immediate update.

---

## Consequences

### Positive

- **The EA promise is real.** The system prepares meetings days in advance, not hours. Users open their week and intelligence is already there. This is the "it should just know" principle applied to time, not just entities.
- **Collaborative preparation becomes possible.** With intelligence existing days ahead, users can share agendas, request input, send pre-reads. The single-player read-ahead tool becomes a collaborative preparation surface.
- **Internal meetings stop being second-class.** Team syncs, 1:1s, cross-functional reviews all get intelligence appropriate to their context. Internal team entities (ADR-0070) provide the same enrichment depth as external accounts.
- **The daily run gets faster.** Assembling from pre-computed intelligence is cheaper than generating from scratch. The morning briefing becomes a 30-60 second narrative refresh, not a 3-5 minute pipeline.
- **Learning never stops.** Even sparse meetings generate intelligence records. Transcripts always have somewhere to attach. Entity discovery happens at every meeting, not just ones the system decided were "worth prepping."
- **The app is never empty.** With continuous intelligence accumulation, opening DailyOS always shows something — meeting intelligence exists from the moment meetings appear on the calendar. The "no briefing yet" state is limited to first-of-day narrative/email/action synthesis, not meeting intelligence.

### Negative

- **Storage growth.** Every meeting gets a SQLite record with intelligence JSON. At ~5-15 meetings/day, this is ~100-300 records/month. Manageable, but needs periodic pruning for meetings older than 6 months.
- **AI cost.** Generating intelligence for all meetings (not just external) increases Claude Code usage. Mitigated by: (a) internal meetings get lighter enrichment, (b) incremental updates are cheaper than full regeneration, (c) the weekly batch spreads cost across Sunday night, not all at 6am.
- **Complexity.** The lifecycle (detection → enrichment → incremental update → refresh → capture → archive) has more states than the current model (generate → deliver → archive). Need clear state machine and good error handling for partial enrichment.
- **Cold start for new meetings.** A meeting added 5 minutes before start time gets minimal intelligence (detection + skeleton, no time for enrichment). The pre-meeting refresh (2h before) is the primary enrichment window for reactive meetings.

### Trade-offs

- **All meetings vs. selective.** Chose to generate intelligence for all meetings (except all-hands) rather than being selective. The cost per meeting is small; the learning benefit of universal coverage is large. A meeting that seems unimportant today may be the first touchpoint with a major account tomorrow.
- **Eventual consistency vs. real-time.** Chose signal recording + batch refresh over real-time re-enrichment on every signal. Real-time would be more current but expensive and noisy. The blue dot ("new signals") gives the user control over when to refresh.
- **Weekly forecast as primary surface vs. daily briefing.** The weekly forecast becomes the strategic view (intelligence across the week); the daily briefing becomes the tactical view (what's happening today with overnight updates). Neither is deprecated — they serve different time horizons.

---

## Existing Issues: Supersession and Incorporation

| Issue | Disposition | Rationale |
|-------|-----------|-----------|
| **I220** (Meeting forecast 4-5 days ahead) | **Superseded by I327** | I220 proposed a forecast section bolted onto the daily briefing. I327 makes advance intelligence generation the default for all meetings — not an add-on, the core model. |
| **I200** (Week page proactive suggestions) | **Partially superseded by I330** | The intelligence quality indicators and live intelligence surface subsume "proactive suggestions" about prep gaps. I200's available-block rendering with `suggestedUse` remains valid for time-blocking. |
| **I202** (Prep prefill + draft agenda) | **Superseded by I333** | Draft agenda and prep sharing are collaboration actions on existing intelligence, not prefill into an empty system. I333 reframes this as meeting intelligence collaboration. |
| **I201** (Live proactive suggestions) | **Keeps scope** | Live query-layer suggestions are orthogonal to meeting intelligence. Not superseded. |
| **I301** (RSVP status) | **Keeps scope** | RSVP data becomes a signal feeding meeting intelligence quality. Not superseded but enriches the intelligence record. |

---

## Implementation Path

| Issue | Title | Phase |
|-------|-------|-------|
| **I326** | Per-meeting intelligence lifecycle — detect, enrich, update, archive | Foundation |
| **I327** | Advance intelligence generation (weekly + polling, not day-of) | Foundation |
| **I328** | Classification expansion — all meetings get intelligence | Foundation |
| **I329** | Intelligence quality indicators (replace "needs prep" badge) | UX |
| **I330** | Weekly forecast as live intelligence surface | Surfaces |
| **I331** | Daily briefing intelligence assembly (diff model, fast refresh) | Surfaces |
| **I332** | Signal-triggered meeting intelligence refresh | Pipeline |
| **I333** | Meeting intelligence collaboration — share, request input, draft agenda | Actions |

I326-I328 are the foundation: the lifecycle, the generation timing, and the classification expansion. I329 provides the UX vocabulary for communicating intelligence state. I330-I331 transform the two primary surfaces. I332 connects the signal bus. I333 enables the collaborative dimension.

---

## References

- ADR-0030: Composable workflow operations (the `meeting:prep` atomic operation)
- ADR-0052: Week page redesign (readiness checks, day shapes)
- ADR-0066: Meeting intelligence package (`MeetingIntelligence` struct, unified lifecycle)
- ADR-0070: Internal team entities (internal meetings get entity-quality intelligence)
- ADR-0080: Signal intelligence architecture (event-driven signal bus)
- ADR-0043: Meeting intelligence is core (universal, not CS-specific)
- ADR-0064: Meeting intelligence report layout (three-tier hierarchy)
- ADR-0065: Meeting prep editability (user agenda/notes)
