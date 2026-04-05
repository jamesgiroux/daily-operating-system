# ADR-0043: Meeting intelligence is core, not extension

**Date:** 2026-02-07
**Status:** Accepted
**Refines:** [ADR-0026](0026-extension-architecture.md) (extension architecture boundary)

## Context

ADR-0026 drew the core/extension boundary with meeting prep, portfolio attention, and stakeholder context on the extension side (CS extension). But the CLI skills inventory reveals these are universal needs, not CS-specific ones:

- **Meeting prep** — Everyone meets with other people. Gathering recent interaction history, open action items, and relationship context before a meeting is valuable whether the meeting is a customer QBR or a cross-team sync.
- **Portfolio attention** — Everyone tracks entities that need attention. For CS it's accounts with renewal dates; for a PM it's projects with milestones; for a manager it's direct reports with review cycles. The computation (recency signals, upcoming deadlines, stale contacts) is identical — the entity label and data source change.
- **Post-meeting analysis** — Capturing outcomes, relationship temperature, and follow-up actions after a meeting is role-agnostic.
- **Decision support** — Surfacing decisions due, stale delegations, and cancelable meetings (the `/cos` pattern) cross-references calendar + actions + entities. No part of that is CS-specific.
- **Stakeholder context** — Relationship history, interaction frequency, and working agreement context (the `/veep` consumption side) are useful in any meeting with any person.

The pattern: **everyone has meetings with people. Profile changes what you call the people and where you look up their context, not whether you need context.**

## Decision

Meeting intelligence capabilities are core functionality. Profile determines classification vocabulary and data sources, not whether the capability exists.

**Moves to core:**

| Capability | What it does | Profile provides |
|------------|-------------|-----------------|
| Meeting prep context | Gathers recent interactions, open actions, relationship history for upcoming meetings | Entity type (account vs project vs person), data source paths |
| Portfolio attention signals | Flags entities needing attention based on recency, deadlines, staleness | Entity label, threshold values, deadline field names |
| Post-meeting capture | Records outcomes, relationship signals, follow-up actions | Outcome categories (wins/risks for CS, decisions/blockers for PM) |
| Decision surfacing | Identifies decisions due, stale delegations, time protection opportunities | Data sources for decisions and delegations |
| Stakeholder context | Surfaces interaction history and relationship signals in meeting preps | Where to look for relationship data |

**Stays in extensions:**

| Capability | Why extension-specific |
|------------|----------------------|
| CS-specific metrics (ARR, renewal dates, health scores, ring classification) | Domain vocabulary — only meaningful to CS |
| Google Sheets sync (Last Engagement Date writeback) | CS operational workflow |
| CRM integration (Clay MCP) | Product-specific data source |
| Account dashboard template/generation | CS document format |
| Success plan templates | CS planning format |
| Personal impact capture, career narrative | ProDev extension (ADR-0041) |

**The boundary:** Core provides the *intelligence engine* (gather context, compute signals, surface insights). Extensions provide *domain-specific data sources and vocabulary* that feed into that engine.

**How profiles participate:**

```
Core intelligence engine:
  - "Give me context for this meeting" → calls profile's entity resolver
  - "Which entities need attention?" → calls profile's attention signals
  - "What happened after this meeting?" → calls profile's outcome categories

Profile contributes (via extension hooks):
  - Entity resolver: CS → account lookup, PM → project lookup
  - Attention signals: CS → renewal + last contact, PM → milestone + sprint
  - Outcome categories: CS → wins/risks, PM → decisions/blockers
```

## Consequences

- Meeting prep, portfolio attention, and post-meeting capture become available to all profiles out of the box — General users get a useful (if simpler) experience without CS extension
- I42 (CoS executive intelligence) is confirmed as core, not extension
- I43 (political intelligence) consumption side is core (stakeholder context in preps); production side (situation analyses) remains extension/Phase 4+
- I40 (daily-csm CLI parity) narrows: only CS-specific data sources and vocabulary remain extension territory. Portfolio triage logic moves to core with CS providing the entity definitions
- Extensions become thinner — they contribute data sources and labels, not entire features
- Core needs a profile-agnostic entity abstraction: "a thing you track that has interactions, actions, and deadlines." CS calls it an account; PM calls it a project; a manager calls it a person
- ADR-0026's core/extension table is refined — the "Accounts page, portfolio triage" row moves to core (with profile-specific rendering)
