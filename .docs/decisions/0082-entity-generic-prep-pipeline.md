# ADR-0082: Entity-Generic Prep Pipeline

**Date:** 2026-02-18
**Status:** Accepted
**Participants:** James Giroux, Claude Code

**Builds on:** [ADR-0057](0057-entity-intelligence-architecture.md) (entity intelligence), [ADR-0080](0080-signal-intelligence-architecture.md) (signal intelligence), [ADR-0081](0081-event-driven-meeting-intelligence.md) (event-driven meeting intelligence)
**Target:** v0.10.0 (all four phases)

---

## Context

### The Four-Fix Symptom

A user linked a meeting from "Salesforce" (account) to "Agentforce" (project). Making that change stick required fixes in four separate files across five code paths:

1. **Entity resolver** -- legacy `account_id` column (confidence 0.99) beat the junction table entry (0.95)
2. **`resolve_account_compat`** -- filtered for accounts only, skipped the project match, fell through to a lower-confidence account
3. **Prep JSON builder** -- fell back to the classified `meeting.account` even when the resolver returned no account
4. **Dashboard command** -- stale `account` field from schedule.json persisted in the byline

Each fix was small and correct. But needing four of them to honor one user action reveals that the pipeline was designed around accounts as the primary entity type, with projects and people bolted on after.

### The Architectural Debt

The prep pipeline grew organically from v0.7 through v0.10:

| Stage | What it does | Entity support |
|-------|-------------|----------------|
| **Classification** (`classify.rs`) | Maps calendar events to meeting types via attendee domains + account hints | Account-only. `account_hints` built from `Accounts/` directory. No project or person hints. |
| **Directive building** (`orchestrate.rs`) | Builds the directive with meetings grouped by type | Carries `meeting.account` from classification. No `meeting.project` or `meeting.entity`. |
| **Meeting context** (`meeting_context.rs`) | Resolves entity and injects intelligence context | Calls `resolve_account_compat` which filters for accounts. Projects that win resolution are invisible to this stage. |
| **Prep generation** (`deliver.rs`) | Writes prep JSON files | Has a singular `account` field. Falls back to classified account when context has none. No `entity` or `project` field. |
| **Schedule generation** (`deliver.rs`) | Writes schedule.json for dashboard | Embeds `account` from classification. No project or entity field. |
| **Dashboard hydration** (`commands.rs`) | Overlays DB entity links onto schedule meetings | Sets `m.account` for account links. Projects linked via junction table don't update the byline. |
| **Frontend display** (`BriefingMeetingCard.tsx`) | Shows entity in meeting card subtitle | Reads `meeting.account`. No concept of `meeting.entity` or `meeting.project`. |

Every stage assumes the primary entity is an account. Projects exist in the junction table and entity chips but are invisible to classification, context building, prep generation, and the byline.

### What ADR-0081 Requires

ADR-0081 (Event-Driven Meeting Intelligence, 0.13.0) declares: "Every meeting gets intelligence." It proposes entity intelligence, person intelligence, and minimal intelligence tiers. It assumes the prep pipeline can resolve to any entity type and inject the right context.

The current pipeline can't do this. I326 (per-meeting intelligence lifecycle) and I327 (advance intelligence generation) will hit the same wall we just hit -- needing per-file hacks to route project or person intelligence into a pipeline built for accounts.

### The Relationship Dimension

Accounts and projects are organizational entities -- they exist independent of who's in the room. But people are different. A person entity means different things in different meeting contexts:

- **Group meeting with 8 attendees** -- Sarah is a stakeholder. Her intelligence contributes to the meeting prep as attendee context, not as the meeting's primary entity.
- **1:1 with Sarah** -- Sarah is the relationship. The meeting is *about* the relationship between you and Sarah. What did you discuss last time? What's outstanding between you? What does she care about? What's her communication style?

This distinction matters because 1:1s are the most common meeting type for managers, executives, and relationship-driven roles. They represent ongoing relationships that accumulate intelligence over time -- exactly the same pattern as accounts and projects.

**The 1:1 as a relationship surface:** When you have a recurring 1:1 with a colleague, your manager, or an executive, the intelligence you want is:
- Recent interaction history (what you discussed in the last 3 meetings)
- Open items between the two of you (actions assigned to/by this person)
- Relationship signals (engagement cadence, topics they've raised, sentiment)
- Cross-entity context (what projects/accounts you both touch)
- Communication preferences (do they want agendas in advance? do they prefer casual or structured?)

This is the three-file pattern from ADR-0057 applied to people -- but only for people you have 1:1 relationships with, not for every contact in the system.

---

## Decision

### 1. Entity-Generic Data Model

Replace the account-specific fields with entity-generic ones throughout the pipeline.

**Classified meeting:**
```rust
pub struct ClassifiedMeeting {
    pub id: String,
    pub title: String,
    // ...existing fields...
    pub resolved_entities: Vec<ResolvedMeetingEntity>,
}

pub struct ResolvedMeetingEntity {
    pub entity_id: String,
    pub entity_type: EntityType,  // Account, Project, Person
    pub name: String,
    pub confidence: f64,
    pub source: String,           // "classification", "junction", "attendee", "keyword", etc.
}
```

The deprecated `account: Option<String>` field is removed. All entity information flows through `resolved_entities`.

**Prep JSON:**
```json
{
  "meetingId": "...",
  "title": "...",
  "type": "customer",
  "entities": [
    { "id": "agentforce", "type": "project", "name": "Agentforce", "primary": true },
    { "id": "jefferies", "type": "account", "name": "Jefferies" }
  ],
  "entityContext": { ... },
  "attendees": [ ... ]
}
```

The singular `account` field is replaced by an `entities` array. One entity is marked `primary` -- this is the entity whose intelligence drives the prep context. Multiple entities can be associated (a meeting can be about a project for an account with specific people).

**Schedule JSON:**
```json
{
  "meetings": [
    {
      "id": "...",
      "title": "...",
      "entities": [
        { "id": "agentforce", "type": "project", "name": "Agentforce", "primary": true }
      ]
    }
  ]
}
```

### 2. Entity-Generic Classification

Extend `classify_meeting_multi` to resolve against all entity types, not just accounts.

**Current:** `account_hints: HashSet<String>` built from `Accounts/` directory names.

**New:** `entity_hints: Vec<EntityHint>` built from DB accounts + projects + key people:

```rust
pub struct EntityHint {
    pub id: String,
    pub entity_type: EntityType,
    pub name: String,
    pub slugs: Vec<String>,      // normalized name variants for title matching
    pub domains: Vec<String>,    // for attendee domain matching (accounts)
    pub keywords: Vec<String>,   // from entity keywords field (I305)
}
```

Classification becomes: for each meeting, check title against all entity slugs/keywords, check attendee domains against account domains, check attendee count + recurrence for 1:1 detection, and produce `resolved_entities` ranked by confidence. This replaces the current account-only domain matching.

### 3. Entity-Generic Context Building

Replace `resolve_account_compat` with a generic `resolve_primary_entity` that returns the top entity regardless of type.

**Current flow:**
```
meeting_context.rs -> resolve_account_compat() -> Option<AccountMatch>
  -> inject account intelligence, files, dashboard data
```

**New flow:**
```
meeting_context.rs -> resolve_primary_entity() -> Option<EntityMatch>
  -> match entity_type:
      Account -> inject account intelligence, files, dashboard data
      Project -> inject project intelligence, files, project data
      Person  -> inject person intelligence, relationship data
```

The context builder dispatches to type-specific enrichment based on what the resolver returns. Each entity type has its own intelligence source:

| Entity type | Intelligence source | Context injected |
|-------------|-------------------|-----------------|
| Account | `intelligence.json` + `dashboard.json` + SQLite signals | Executive assessment, risks, wins, stakeholder insights, account snapshot |
| Project | `intelligence.json` + `dashboard.json` + SQLite signals | Project status, blockers, milestones, team context |
| Person (1:1) | `intelligence.json` + SQLite relationship data | Relationship assessment, interaction history, open items, cross-entity connections |

### 4. People as Relationship Entities

People get the three-file pattern (ADR-0057) when they have a 1:1 relationship with the user. This is triggered by meeting classification: when a meeting is detected as a 1:1 (2 attendees, recurring, one is the user), the other person becomes a relationship entity.

**Three-file pattern for people:**
```
People/Sarah Chen/
  dashboard.json       # Relationship facts: role, org, cadence, how long you've worked together
  intelligence.json    # Relationship intelligence: communication style, topics, open items, dynamic
  dashboard.md         # Rich artifact for ecosystem consumption
```

**When files are created:** Not for every contact. Files are created when:
- A person is the counterpart in a detected 1:1 (recurring or frequent)
- A person is manually marked as a key relationship
- A person has sufficient interaction history (e.g., 5+ meetings)

**Relationship intelligence vs contact intelligence:**

| Context | What the system provides | File source |
|---------|------------------------|-------------|
| **1:1 meeting prep** | Relationship briefing: what you discussed last time, what's outstanding, what they care about, how to approach this conversation | `intelligence.json` (full relationship intelligence) |
| **Group meeting attendee context** | Stakeholder context: their role, their perspective on the agenda, their relationship to other attendees | SQLite relationship data + `intelligence.json` excerpts |
| **Entity detail page** | Full relationship profile: interaction timeline, cross-entity connections, communication patterns | All three files |

The distinction is that in a 1:1, the person IS the meeting's primary entity. In a group meeting, they're a stakeholder contributing context to the primary entity (account/project).

**1:1 classification signals:**
- Exactly 2 attendees (one is the user)
- Recurring event
- Title patterns: "{Name} / {Name}", "{Name} 1:1", "{Name} <> {Name}"
- Historical: same two people have met 3+ times in the past 90 days

When a meeting is classified as a 1:1, the counterpart person becomes the primary entity with confidence proportional to the relationship strength.

### 5. Entity-Generic Dashboard Display

Replace `meeting.account` with the primary entity from the `entities` array on the frontend.

**BriefingMeetingCard subtitle:**
```tsx
const primaryEntity = meeting.entities?.find(e => e.primary) ?? meeting.entities?.[0];
subtitleParts.push(primaryEntity?.name ?? formatMeetingType(meeting.type));
```

Entity chips already support both accounts and projects. The byline reads from the entities array. For 1:1s, the byline shows the person's name and role.

### 6. Junction Table as Sole Source of Truth

The `meeting_entities` junction table is the sole source of truth for meeting-entity associations. The legacy `account_id` column on `meetings_history` is dropped.

- **Junction table** (`meeting_entities`): Source of truth for all meeting-entity associations. Written by entity resolution and user corrections.
- **Legacy `account_id`** (`meetings_history.account_id`): **Removed.** Single active user, no backward compatibility concerns.
- **Classification hints**: Initial resolution seeds, not persistent associations. Overridden by junction table entries and user corrections.

Entity resolution signals (ADR-0080) priority:

| Priority | Signal source | Confidence | Rationale |
|----------|--------------|------------|-----------|
| 1 | Junction table (user-confirmed) | 1.0 | The user told us |
| 2 | Junction table (auto-resolved) | 0.90-0.95 | System resolved with high confidence |
| 3 | 1:1 person detection | 0.85-0.95 | 2-person recurring meeting, strong pattern |
| 4 | Project keyword match | 0.70-0.85 | Title/description matched project keywords |
| 5 | Attendee group pattern | 0.60-0.80 | Historical co-occurrence |
| 6 | Attendee entity voting | 0.50-0.75 | Person-entity links, majority vote |
| 7 | Domain classification | 0.40-0.65 | External domain matched account |
| 8 | Title heuristic | 0.30-0.50 | Fuzzy string match on entity names |

User corrections (priority 1) always win. The rest fuse via Bayesian combination (ADR-0080 Section 4).

### 7. Implementation: All Four Phases in 0.10.0

Single active user, no external consumers, no backward compatibility concerns. Ship all phases together.

**Phase 1: Data model**
- Add `resolved_entities` to `ClassifiedMeeting`, remove `account`
- Add `entities` array to prep JSON, remove `account`
- Add `entities` to schedule JSON, remove `account`
- Add `entities` to frontend `Meeting` type, remove `account`

**Phase 2: Classification**
- Build `entity_hints` from DB (accounts + projects + 1:1 people)
- Populate `resolved_entities` during classification
- 1:1 detection logic for person entity resolution

**Phase 3: Context building**
- Remove `resolve_account_compat` entirely
- Implement `resolve_primary_entity` with type dispatch
- Add project context injection (intelligence.json + SQLite)
- Add person/relationship context injection for 1:1s

**Phase 4: Cleanup**
- Drop `account_id` column from `meetings_history` (migration)
- Remove `resolve_account_compat` and all backward-compat wrappers
- Remove `signal_explicit_assignment` (legacy account_id signal)
- Remove all `m.account` / `meeting.account` references in frontend
- Clean up `build_account_domain_hints` -> `build_entity_hints`

---

## Consequences

### Positive

- **Entity parity.** Accounts, projects, and people are first-class citizens in the prep pipeline. One change in the entity picker propagates through all stages.
- **1:1 intelligence.** The most common meeting type for managers and executives gets relationship-quality intelligence, not thin person-prep.
- **ADR-0081 unblocked.** The 0.13.0 meeting intelligence sprint can focus on intelligence lifecycle, not pipeline plumbing.
- **Reduced maintenance.** Five code paths that need separate fixes for non-account entities become one generic path.
- **Better resolution.** Entity hints from all types improve classification accuracy. A meeting titled "Agentforce Demo" matches the project directly. A meeting with 2 attendees resolves to the person.
- **Clean codebase.** Removing account_id, resolve_account_compat, and all the backward-compat wrappers eliminates an entire class of bugs.

### Negative

- **Scope.** Touching classification, context building, prep generation, schedule generation, dashboard hydration, and frontend display in one release is significant. Mitigated by single active user.
- **Testing surface.** Every entity type needs test coverage through the full pipeline. Account-only tests are insufficient.
- **Person file proliferation.** The three-file pattern for people could create many directories if the threshold is too low. Mitigated by requiring recurring 1:1 or manual marking.

### Risks

- **Over-abstraction.** Entity types do have meaningful differences (accounts have ARR, projects have milestones, people have relationships). The generic model must allow type-specific behavior at the context injection layer without losing the simplicity of a generic pipeline above it.
- **1:1 detection accuracy.** A 2-person meeting isn't always a 1:1 (could be ad hoc, interview, etc.). Recurring + historical frequency signals mitigate false positives.
- **Relationship scope creep.** People-as-entities could expand to tracking relationship dynamics, communication preferences, personality insights. Keep it focused on meeting prep value: what do I need to know to walk into this 1:1 prepared?

---

## Files Affected

| File | Change |
|------|--------|
| `src-tauri/src/google_api/classify.rs` | Accept `entity_hints`, populate `resolved_entities`, 1:1 detection |
| `src-tauri/src/prepare/orchestrate.rs` | Build entity hints from DB, pass to classifier |
| `src-tauri/src/prepare/meeting_context.rs` | Remove `resolve_account_compat`, implement `resolve_primary_entity` with type dispatch, add project + person context paths |
| `src-tauri/src/prepare/entity_resolver.rs` | Remove `resolve_account_compat`, remove `signal_explicit_assignment`, add 1:1 person signal |
| `src-tauri/src/workflow/deliver.rs` | `build_prep_json` uses `entities` array, `deliver_schedule` includes entities, remove `account` field |
| `src-tauri/src/commands.rs` | Dashboard hydration sets `m.entities` from junction table, removes `m.account` |
| `src-tauri/src/types.rs` | Replace `account` with `entities` on `Meeting` struct |
| `src-tauri/src/db.rs` | Migration to drop `account_id` from `meetings_history` |
| `src-tauri/src/hygiene.rs` | Remove `fix_orphaned_meetings` (no more legacy account_id to orphan) |
| `src/types/index.ts` | Replace `account` with `entities` on `Meeting` TypeScript type |
| `src/components/dashboard/BriefingMeetingCard.tsx` | Read primary entity from `entities` array |
| `src/pages/MeetingDetailPage.tsx` | Display primary entity in header/byline |

---

## References

- ADR-0057: Entity intelligence architecture (three-file pattern, entity-generic intelligence)
- ADR-0080: Signal intelligence architecture (signal fusion, entity resolution cascade)
- ADR-0081: Event-driven meeting intelligence (intelligence lifecycle, every meeting gets intelligence)
- I305: Intelligent meeting-entity resolution (current implementation, first entity resolver)
- I326-I333: 0.13.0 Meeting Intelligence sprint (consumers of this pipeline)
