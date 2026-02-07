# ADR-0046: Entity-mode architecture with orthogonal integrations

**Date:** 2026-02-07
**Status:** Accepted
**Supersedes:** [ADR-0026](0026-extension-architecture.md) (extension architecture with profile-activated modules)
**Builds on:** [ADR-0045](0045-entity-abstraction.md) (entity abstraction)

## Context

ADR-0026 established extensions as profile-activated module bundles (CSM → Core + CS Extension). ADR-0043 moved meeting intelligence to core, making extensions thinner — they only provide data sources and vocabulary. ADR-0045 introduced a profile-agnostic entity abstraction (Account, Project, Person, Other). ADR-0039 added feature toggle granularity within extensions.

Real users span diverse roles: Customer Success, Sales, Marketing, Product, Engineering, Finance, Legal, Consulting. Building a profile per role creates a combinatorial explosion. The actual organizing principle isn't job title — it's how work is structured:

- **Account-based roles** (CS, Sales, Legal, Accounting, Consulting) track relationships with organizations. Meetings are about accounts. Attention signals are renewal dates, relationship health, contact recency.
- **Project-based roles** (Marketing, Product, Engineering, Finance, Ops) track initiatives with deliverables. Meetings are about projects. Attention signals are milestones, blockers, status staleness.
- **Many roles do both.** A PM tracks feature projects AND key customer relationships. A consultant tracks client accounts AND project workstreams.

Meanwhile, the integration ecosystem (Gong, Salesforce, Linear, Gainsight, Asana, Granola, Fathom, etc.) is orthogonal to how work is organized. Any transcript source works with any entity mode. Any CRM feeds account entities. Any task tool feeds project entities.

The profile concept ("customer-success", "general") conflates three independent concerns: entity organization, domain vocabulary, and data source connections. Separating them unlocks composability.

## Decision

Three-layer architecture: **Core + Entity Mode + Integrations.** Domain overlays provide optional vocabulary and schema enrichment.

### Layer 1: Core (unchanged)

Briefing, inbox, actions, calendar, archive, meeting intelligence. Always active. Scope defined by ADR-0043.

### Layer 2: Entity Mode (replaces profile)

Users choose how their work is organized:

| Mode | Entity type | Tracks | Attention signals |
|------|------------|--------|-------------------|
| Account-based | Organizations/clients | Relationships, interactions, renewals | Relationship health, stale contact, upcoming deadlines |
| Project-based | Initiatives/workstreams | Deliverables, milestones, status | Milestone proximity, blocked items, status staleness |
| Both | Both simultaneously | Relationships and initiatives | Unified, sorted by urgency |

Entity mode determines: entity schema (which overlay tables are active), association logic (how meetings link to entities), attention signals (what "needs attention" means), and vocabulary (labels in the UI).

**"Both" mode is first-class from day one.** The dashboard renders a unified attention view sorted by urgency — entity type is a badge, not a section. The sidebar provides Accounts and Projects as separate navigation sections. Designing for "both" from the start avoids an architectural retrofit.

### People as universal sub-entity

People are not a third entity mode. People are the connective tissue across both modes:

- Every account has key people (champion, stakeholder, contact)
- Every project has key people (owner, contributor, sponsor)
- People carry interaction history and relationship context regardless of entity mode
- Leaders who track teams and individuals do so in the context of account-based or project-based work

People enrich meeting prep (stakeholder context), meeting association (attendee matching), and relationship intelligence (communication patterns, last contact).

### Layer 3: Integrations (orthogonal to entity mode)

Integrations are MCP data source connectors (ADR-0027). Any integration works with any entity mode.

| Category | Examples | Feeds into |
|----------|----------|-----------|
| Transcript sources | Gong, Fathom, Granola, Quill | Meeting intelligence (core) |
| CRM / relationship | Salesforce, Gainsight, HubSpot, Clay | Account entities |
| Task / project mgmt | Asana, Linear, Jira, Monday | Project entities |
| Communication | Gmail (existing), Outlook, Slack | Briefing (core) |

Each integration is an MCP server. The app is an MCP client. Community can build integrations without touching core code.

### Domain overlays (replaces extensions)

Domain-specific field schemas and templates are contributed by **domain overlays**:

- CS overlay: ARR, health, ring fields on accounts + CS dashboard templates
- Sales overlay: pipeline stage, deal size fields on accounts + pipeline views
- ProDev overlay: personal impact capture + career narrative (ADR-0041)

Domain overlays are thinner than ADR-0026 extensions — they contribute fields, templates, and vocabulary. They don't provide features (core per ADR-0043) and they don't provide data sources (integrations).

### Data model

| Table | Scope | Fields |
|-------|-------|--------|
| `entities` | Universal | id, name, entity_type, tracker_path, updated_at |
| `accounts` | Account-mode overlay | CS-specific: ring, ARR, health, renewal, csm, champion |
| `projects` | Project-mode overlay | status, milestone, owner, target_date |
| `people` | Universal sub-entity | name, email, organization, role, last_contact |
| `entity_people` | Junction | entity_id, person_id, relationship_type |
| `meeting_entities` | Many-to-many | meeting_id, entity_id (replaces `account_id` FK) |

The existing `entities` + `accounts` bridge pattern (ADR-0045) extends naturally: `projects` is a parallel overlay table with the same bridge mechanism.

### Meeting-entity association

- **Account-based:** attendee email domain → organization → account entity (existing pattern, automatic)
- **Project-based:** integration-sourced (Linear/Asana provides project↔meeting links), AI inference during enrichment, manually correctable
- **Both mode:** meetings can associate with accounts, projects, or both via many-to-many `meeting_entities`

Account association has a natural key (email domain). Project association doesn't — it's integration-first, AI-assisted, manually correctable. This asymmetry is acceptable: project tools (Linear, Asana) carry structured meeting↔project relationships that CRMs lack.

### Onboarding

Profile selector becomes entity-mode selector:

1. "How do you organize your work?" → Account-based / Project-based / Both
2. "What tools do you use?" → Integration checklist (Gmail, Gong, Salesforce, Linear, etc.)
3. (Optional shortcut) "What's your role?" → Pre-selects entity mode + domain overlay

Job title is a convenience shortcut to configuration, not the organizing principle. "I'm a CSM" pre-selects account-based + CS overlay + recommends Gainsight. The system doesn't know or care about "CSM" — it knows account-based entity mode with these integrations.

### Config evolution

```json
{
  "entityMode": "both",
  "integrations": {
    "gmail": { "enabled": true },
    "gcal": { "enabled": true },
    "gong": { "enabled": false }
  },
  "domainOverlay": "customer-success",
  "features": {
    "accountTracking": true,
    "projectTracking": true,
    "postMeetingCapture": true
  }
}
```

Migration: `profile: "customer-success"` maps to `entityMode: "account"` + `domainOverlay: "customer-success"`. `profile: "general"` maps to `entityMode: "project"` + no overlay.

## Consequences

**Easier:**
- Adding new roles: pick entity mode + integrations. No custom profile code.
- Integrations are independent — ship one at a time, any combination works.
- "Both" mode supported from day one — no architectural retrofit.
- People as universal sub-entity enriches meeting prep and stakeholder context for all users.
- Community can build MCP integrations without touching core.
- Thinner overlays are simpler to build and maintain than full extensions.

**Harder:**
- "Both" mode UX requires unified attention view with type-aware rendering.
- Project-entity association lacks a natural key (unlike account's domain matching).
- Three entity tables (entities + accounts + projects) to keep synchronized via bridge pattern.
- MCP client infrastructure must be production-ready before integrations work.
- Migration from `profile` config to `entityMode` + `integrations` schema.
- People table introduces a new data source that needs population strategy.

**Trade-offs:**
- Chose "both as first-class" over "ship simple, add both later" — more upfront complexity, prevents retrofit.
- Chose people as sub-entity over third mode — matches how users actually organize their work.
- Chose MCP for integrations over custom protocol — leverages ecosystem, community-contributable, already decided in ADR-0027.
- Domain overlays are thinner than ADR-0026 extensions — less powerful but sufficient given ADR-0043's narrowed scope.
- Accepted that project association is weaker than account association without integrations — project-based mode improves with tool connections, account-based mode works standalone.
