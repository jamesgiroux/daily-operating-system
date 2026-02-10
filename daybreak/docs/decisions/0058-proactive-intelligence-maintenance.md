# ADR-0058: Proactive Intelligence Maintenance

**Date:** 2026-02-09
**Status:** Accepted
**Builds on:** [ADR-0057](0057-entity-intelligence-architecture.md) (entity intelligence architecture), [ADR-0042](0042-per-operation-pipelines.md) (per-operation pipelines), [ADR-0048](0048-three-tier-data-model.md) (three-tier data model)

## Context

The intelligence architecture (ADR-0057) is reactive: it triggers enrichment when content changes, when the user clicks refresh, or when a calendar event shifts. But it never asks "what don't I know?" The system waits for signals instead of seeking them.

This creates a class of gaps that accumulate silently:

- **People without names.** Calendar discovery creates people from email addresses. `jsmith@acme.com` sits in the people table with their email as their display name, even though their real name appears in email signatures, transcripts, and meeting notes the system already has.
- **Stale intelligence.** An account was enriched 6 weeks ago. Three new transcripts and two email threads have arrived since. The intelligence is outdated but nothing triggers a refresh because no single file change was recent enough to fire the watcher.
- **Orphaned meetings.** A meeting happened with `@acme.com` attendees but was never linked to the Acme Corp entity — the auto-linker missed it because the meeting title was "Q1 Planning" with no account name.
- **Unknown relationships.** 40% of people in the system have `relationship: "unknown"` — but the system knows their email domain, and could match `@acme.com` to the Acme Corp account to infer "external."
- **Missing entity intelligence.** An account was created two weeks ago and has six files in its workspace directory, but intelligence enrichment never ran because the watcher wasn't active when the files were added.
- **Degrading people data.** A person's role changed (visible in recent email signatures) but the system still shows their old title from six months ago.

These gaps violate P6 (AI-Native) — the system should do the synthesis work, not the user. They also violate P2 (Prepared, Not Empty) — the system should be ready before the user looks, not waiting for a trigger.

### Existing infrastructure

The building blocks for proactive maintenance already exist:

- **IntelligenceQueue** (`intel_queue.rs`): priority-based dedup queue with debounce. Currently handles `ContentChange`, `CalendarChange`, and `Manual` priorities.
- **Scheduler** (`scheduler.rs`): cron-like job scheduling with sleep/wake detection and missed-job recovery.
- **Content index** (`content_index` table): tracks files per entity with `modified_at` and `extracted_at` timestamps.
- **Entity intelligence** (`entity_intelligence` table): stores `enriched_at` per entity — the staleness signal.
- **Staleness queries**: `get_stale_accounts()` and `get_stale_delegations()` already detect time-based decay.
- **People signals**: `get_people_with_signals()` provides temperature, trend, last_seen, meeting counts.
- **Meeting-entity junction** (`meeting_entities` table): links meetings to entities, with auto-linking on upsert (Sprint 9).

What's missing is a **scanner** that identifies gaps and a **processor** that fills them autonomously.

## Decision

### Architecture: Hygiene Scanner + Proactive Processor

Add two new subsystems that run in the background:

```
Hygiene Scanner (periodic)          Proactive Processor (continuous)
┌─────────────────────┐            ┌──────────────────────┐
│ Scan for gaps:       │            │ Process gap queue:    │
│  - stale intelligence│   enqueue  │  - AI enrichment     │
│  - unnamed people    │ ────────→  │  - name resolution   │
│  - orphaned meetings │            │  - domain matching   │
│  - missing enrichment│            │  - relationship      │
│  - unindexed files   │            │    inference          │
└─────────────────────┘            └──────────────────────┘
       ↑                                     │
       │ scheduled                           │ results
       │ (daily + calendar-driven)           ↓
  Scheduler                          IntelligenceQueue
                                     (existing, extended)
```

The scanner runs on a schedule (nightly full scan, pre-meeting targeted scan). It produces a list of **hygiene tasks** — discrete, prioritized units of work. The proactive processor consumes these tasks, using AI for tasks that require synthesis and mechanical operations for tasks that don't.

### Hygiene task taxonomy

| Task Type | Detection | Resolution | Requires AI? | Priority Signal |
|-----------|-----------|------------|-------------|----------------|
| **Stale intelligence** | `enriched_at` older than threshold + new content exists | Re-run entity enrichment | Yes | Meeting proximity |
| **Unnamed people** | `name` matches email pattern (contains `@`) | Mine emails, transcripts, meeting notes for real name | Yes (extraction) | Meeting frequency |
| **Unknown relationship** | `relationship = "unknown"` + email domain matchable to entity | Match domain → account, infer external/internal | No (mechanical) | Meeting proximity |
| **Orphaned meetings** | Meeting has attendees from entity domain but no junction row | Create junction link | No (mechanical) | Recency |
| **Missing intelligence** | Entity has files in content_index but no `entity_intelligence` row | Run full intelligence build | Yes | Meeting proximity |
| **Unindexed files** | Files in entity directory newer than last `extracted_at` | Index and trigger enrichment | Partial (extraction) | File recency |
| **Stale people data** | Person's `updated_at` older than threshold + recent email/transcript mentions | Extract current role/org from recent signals | Yes (extraction) | Meeting proximity |
| **Duplicate people** | Multiple person records with same name or overlapping email domains | Flag for user review (don't auto-merge) | No (detection only) | N/A |
| **Entity association gaps** | Person has meetings with entity attendees but no `entity_people` link | Create association link | No (mechanical) | Meeting frequency |

### Calendar-driven prioritization

The scanner assigns priority based on **meeting proximity** — how soon the user will encounter this entity in a meeting:

```
Priority 1 (Critical):  Entity has a meeting today or tomorrow
Priority 2 (High):      Entity has a meeting this week
Priority 3 (Medium):    Entity has a meeting next week
Priority 4 (Low):       Entity has no upcoming meetings
Priority 5 (Background): No meetings and no recent contact (>30 days)
```

This means: the night before a meeting with Acme Corp, the system ensures Acme's intelligence is fresh, its people have real names, its meetings are properly linked, and its stakeholder data is current. For entities with no upcoming meetings, hygiene runs at lowest priority, filling gaps when there's nothing more urgent.

Last-contact tracking reinforces this: if you haven't met with an account in 30+ days but their intelligence hasn't been refreshed, the scanner still queues a refresh — but at background priority, not critical.

### Email as intelligence signal

Email is a rich, underused signal source. The system already fetches and classifies emails (ADR-0024, ADR-0029). The proactive processor can mine emails for:

- **People names.** Email `From:` headers contain display names: `"Sarah Chen" <schen@acme.com>`. When a person record exists with only `schen@acme.com`, the display name resolves it mechanically — no AI needed.
- **Role and title.** Email signatures contain job titles, departments, phone numbers. AI extraction from the most recent email body yields current role data.
- **Domain-to-entity mapping.** A new email from `@newcorp.com` that isn't associated with any entity is a signal to prompt entity creation or auto-match.
- **Relationship freshness.** Last email date from a person updates their `last_seen` and temperature signals without waiting for a calendar meeting.
- **Entity activity.** Volume and recency of email from an entity's domain indicates engagement level — feeding into entity intelligence assessments.

### Overnight batch processing

The hygiene scanner runs its full scan in the **overnight window** (configurable, default 2:00 AM). This is when the system does its most expensive work:

1. Full entity scan: check every entity's `enriched_at` against content freshness
2. People audit: scan all people records for resolution opportunities
3. Meeting backfill: check recent meetings for missing entity links
4. Content index reconciliation: verify index matches filesystem

The scanner respects a **budget** — maximum number of AI enrichment calls per overnight run (default: 20). Mechanical tasks (domain matching, junction linking) have no budget limit since they're fast and free.

The user sees the results in their morning briefing:

> *Overnight, DailyOS refreshed intelligence for 3 accounts with meetings this week, resolved names for 8 people, and linked 5 meetings to their accounts.*

This is P9 (Show the Work, Hide the Plumbing) — the user sees outputs, not the process.

### Pre-meeting targeted scan

In addition to the overnight batch, the scanner runs a **targeted scan** 2 hours before each meeting (aligning with the existing calendar poller cadence). This scan only checks:

- Is the entity intelligence for linked entities current?
- Are all attendee people records resolved (names, roles)?
- Are all attendees linked to the correct entities?

If gaps are found, they're queued at Priority 1 and processed immediately. This ensures the user is never looking at a meeting card with email-address attendees when the system has their names.

### Integration with existing IntelligenceQueue

The proactive processor extends the existing `IntelligenceQueue` rather than creating a parallel system:

```rust
pub enum IntelPriority {
    ProactiveCritical = 0,  // NEW: meeting today/tomorrow
    ContentChange = 1,      // existing
    CalendarChange = 2,     // existing
    ProactiveHigh = 3,      // NEW: meeting this week
    Manual = 4,             // existing (was 3)
    ProactiveMedium = 5,    // NEW: meeting next week
    ProactiveBackground = 6,// NEW: no upcoming meetings
}
```

Proactive tasks interleave with reactive tasks by priority. A content change still takes precedence over a scheduled refresh, but a pre-meeting critical refresh takes precedence over everything.

### Mechanical vs. AI resolution

Not all hygiene tasks require AI. The processor separates tasks into two paths:

**Mechanical (instant, free):**
- Domain matching: `schen@acme.com` → Acme Corp entity → relationship: external
- Email display name extraction: `From: "Sarah Chen"` → update person name
- Junction linking: attendee domain matches entity → create `meeting_entities` row
- Entity association: person attends meetings with entity → create `entity_people` row
- Duplicate detection: flag (don't auto-merge) people with matching names across email addresses

**AI-powered (queued, budgeted):**
- Intelligence refresh: re-run enrichment with new content
- Role extraction: parse email signatures for current title/department
- Name disambiguation: when multiple signals conflict, synthesize the correct name
- Meeting-entity inference: when domain matching fails, use meeting title/content to guess entity association
- Stale intelligence triage: determine if intelligence needs a full rebuild or incremental update

### Additional proactive opportunities

Beyond the core gap detection, the scanner can identify higher-order opportunities:

1. **Relationship drift detection.** Person temperature went from "hot" to "cold" (meeting frequency dropped). The daily briefing surfaces: "You haven't met with Sarah Chen in 6 weeks — she was previously a weekly contact."

2. **Action staleness escalation.** An action in `waiting` status for 14+ days hasn't been nudged. The system surfaces it proactively rather than waiting for the user to browse the actions list. (Partially exists via `get_stale_delegations` — proactive processing adds the "do something about it" layer.)

3. **Portfolio balance alerts.** Entity meeting distribution is skewed — 80% of meetings are with 2 of 12 accounts. The weekly briefing could note: "5 accounts have had no contact in the last 30 days."

4. **Intelligence confidence scoring.** Not all intelligence is equally confident. An account enriched from 12 source files has higher confidence than one enriched from 2. The scanner can flag low-confidence entities for attention.

5. **Prep completeness audit.** Before the daily briefing generates, scan today's meetings: do all external meetings have entity intelligence? Do all attendees have resolved profiles? Surface gaps as "3 of 5 external meetings are fully prepped."

6. **Cross-entity relationship mapping.** Person X appears in meetings for Account A and Account B. The system can infer organizational connections and surface them: "Sarah Chen connects your Nielsen and Heroku accounts — she moved from Nielsen to Heroku in January."

7. **Calendar pattern learning.** After several weeks, the system recognizes recurring meeting patterns: "This is your monthly sync with Nielsen — last month's key topic was the exit clause." Meeting prep becomes richer automatically.

8. **Entity lifecycle transitions.** An account's meeting frequency increased 3x in the last month. Intelligence assessment should note: "Engagement intensity suggests this account is entering an active evaluation or expansion phase."

## Consequences

**Easier:**
- Intelligence stays fresh without user intervention — the system seeks gaps instead of waiting for triggers
- People records resolve from email addresses to real names automatically from existing data
- Meeting-entity links form correctly even when meeting titles don't mention account names
- Morning briefing includes a "system health" signal: the user knows the system is working for them
- Entity intelligence is always current before meetings — calendar-driven prioritization ensures prep quality
- Stale relationships surface proactively, not when the user happens to notice

**Harder:**
- Budget management is critical — unlimited AI calls would be expensive and noisy
- Mechanical operations (domain matching, junction linking) need to be idempotent and safe
- False-positive detection (incorrect name resolution, wrong entity linking) requires graceful correction
- Overnight processing needs to handle app-not-running scenarios (macOS sleep, quit)
- Testing requires simulating staleness conditions and time progression

**Trade-offs:**
- Chose calendar-driven prioritization over equal treatment — entities with upcoming meetings get attention first, background entities get leftovers. This means rarely-met entities may have stale intelligence for weeks. Acceptable: if you're not meeting them, stale intelligence costs nothing.
- Chose overnight batch + pre-meeting targeted over continuous scanning — continuous would be fresher but wasteful. The two-window approach (overnight bulk + pre-meeting targeted) covers 95% of cases with minimal resource use.
- Chose budget caps over unlimited processing — means some gaps persist across days. But P1 (Zero-Guilt) says the system should never create obligation. A queue that processes what it can and carries the rest forward is guilt-free.
- Chose mechanical-first resolution — email display names and domain matching are tried before AI. Cheaper, faster, and more reliable for the 60% of cases that are mechanical. AI handles the remaining 40% that require judgment.
- Chose flag-not-merge for duplicate detection — auto-merging people records is dangerous (two different Sarah Chens at different companies). The system detects and flags; the user decides. This is P4 (Opinionated Defaults, Escapable Constraints).
