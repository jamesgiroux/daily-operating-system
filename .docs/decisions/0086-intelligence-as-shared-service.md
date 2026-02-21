# ADR-0086: Intelligence as Shared Service

**Status:** Accepted
**Date:** 2026-02-21
**Context:** Meeting prep queue implementation, week workflow simplification

## Decision

Entity intelligence (`intelligence.json`) is the shared enrichment layer. Meeting briefings are mechanical consumers of that intelligence, not independent AI enrichment targets.

## Architecture

```
Signals (email, calendar, Clay, transcripts, corrections)
    |
    v
Intel Queue (AI enrichment via PTY)
    |
    v
intelligence.json (per entity: account, project, person)
    |
    v  (consumed mechanically)
Meeting Prep Queue (gather_meeting_context -> build_prep_json)
    |
    v
prep_frozen_json (per meeting, in DB)
    |
    v
Frontend (MeetingCard, MeetingDetailPage)
```

### The Chain

1. **Signals drive entity intelligence.** Email patterns, calendar changes, Clay enrichment, transcript processing, and user corrections all emit signals via the signal bus.
2. **Signals trigger intelligence refresh.** The propagation engine evaluates rules, the intel queue picks up entities needing enrichment, and Claude Code produces a fresh `intelligence.json`.
3. **Intelligence updates trigger meeting prep regeneration.** When `intel_queue` writes new intelligence for an entity, it finds future meetings linked to that entity, clears their `prep_frozen_json`, and enqueues them in the `MeetingPrepQueue`.
4. **Meeting prep is mechanical.** `gather_meeting_context()` reads `intelligence.json` from disk and assembles the briefing from: executive assessment, risks, stakeholder insights, readiness items, open actions, meeting history, email signals. No AI call needed.
5. **Freshness reflects entity intelligence age.** A meeting's intelligence quality badge tracks how recently its linked entity's intelligence was refreshed, not when the meeting's own prep was generated.

### What This Replaces

Previously, `get_meeting_timeline` spawned inline `tokio::spawn` calls to `generate_meeting_intelligence()` for each sparse future meeting. This caused:
- Race conditions from parallel PTY calls
- Machine flooding (multiple Claude Code processes)
- Data format mismatches (`ai_intelligence` JSON vs `FullMeetingPrep`)
- Expensive per-meeting AI enrichment for data that was already available mechanically

The weekly forecast workflow (`enrich_week`) also ran an expensive AI call to produce `weekNarrative`, `topPriority`, and time block suggestions that the frontend never rendered.

### Key Principle

**AI enrichment happens at the entity level, not the meeting level.** Meetings consume entity intelligence mechanically. This means:
- One AI call per entity enriches every meeting linked to it
- Meeting prep regeneration is instant (no PTY, no wait)
- The same intelligence serves the account dashboard, person detail, and meeting briefing
- Token spend scales with entity count, not meeting count

## Consequences

- Meeting briefing quality is bounded by entity intelligence quality
- If intelligence.json is stale, all meetings for that entity show stale prep
- The refresh button on the Week page clears and requeues meeting preps (instant), not AI enrichment
- Entity intelligence enrichment is the lever for improving meeting briefing quality
- The weekly forecast workflow runs mechanical delivery only (dayShapes), no AI step
