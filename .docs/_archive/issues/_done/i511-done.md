# I511 — Local Schema Decomposition

**Priority:** P0
**Area:** Backend / DB
**Version:** v1.0.0 (Phase 1)
**Depends on:** Nothing — first in the chain
**Blocks:** I512 (ServiceLayer), I513 (workspace elimination), I508 (intelligence schema redesign)
**Absorbs:** I381 (db/mod.rs domain migration)

## Problem

The current schema has three structural problems:

1. **`meetings_history` is a god table (25 columns).** It mixes scheduling data (title, start_time, attendees), prep data (prep_frozen_json, prep_context_json, user_agenda_json), transcript data (transcript_path, transcript_processed_at), intelligence lifecycle (intelligence_state, intelligence_quality, last_enriched_at), and user interaction state (last_viewed_at, has_new_signals). These are different concerns with different write frequencies and different consumers.

2. **`entity_intelligence` is a blob table (19 columns).** It stores both the AI assessment (executive_assessment, risks_json, stakeholder_insights_json) and the quality/health metrics (health_score, health_trend, coherence_score). The assessment changes on every enrichment. The quality metrics are computed from signals and should be independently queryable. I508 needs these separated to implement the 6-dimension intelligence schema.

3. **`entity_people` and `account_team` are parallel systems.** `entity_people` links any entity to people (3 columns, polymorphic). `account_team` links accounts to people with roles (4 columns, account-specific). Both represent "people associated with an account" but use different schemas, different queries, and different service paths. 47 backend references to `entity_people` + 66 to `account_team` = 113 call sites maintaining two parallel systems.

## Design

### Decomposition 1: meetings_history → 3 tables

**`meetings`** — scheduling and calendar data only:
```sql
CREATE TABLE meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT,
    attendees TEXT,             -- JSON array of attendee info
    calendar_event_id TEXT,
    description TEXT,
    created_at TEXT NOT NULL,
    last_viewed_at TEXT
);
```

**`meeting_prep`** — prep data, frozen briefings, user notes:
```sql
CREATE TABLE meeting_prep (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    prep_context_json TEXT,     -- context gathered for prep
    user_agenda_json TEXT,      -- user-set agenda items
    user_notes TEXT,            -- user's personal notes
    prep_frozen_json TEXT,      -- frozen briefing snapshot
    prep_frozen_at TEXT,
    prep_snapshot_path TEXT,
    prep_snapshot_hash TEXT
);
```

**`meeting_transcripts`** — transcript processing state and outcomes:
```sql
CREATE TABLE meeting_transcripts (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    transcript_path TEXT,
    transcript_processed_at TEXT,
    notes_path TEXT,
    intelligence_state TEXT NOT NULL DEFAULT 'detected',
    intelligence_quality TEXT NOT NULL DEFAULT 'sparse',
    last_enriched_at TEXT,
    signal_count INTEGER NOT NULL DEFAULT 0,
    has_new_signals INTEGER NOT NULL DEFAULT 0
);
```

**Why not 4 tables?** The v1.0.0 plan says "meetings → 4 tables" (meetings, meeting_prep, meeting_intelligence, meeting_outcomes). After examining the schema: intelligence lifecycle (state, quality, enriched_at) and transcript data (path, processed_at) are tightly coupled — intelligence state tracks *transcript* processing state. Splitting them creates a join on every transcript processing call for no benefit. Outcomes (wins, risks, decisions, actions) are already stored via the captures system (`insert_capture()`), not on `meetings_history`. So the 4th table already exists conceptually.

### Decomposition 2: entity_intelligence → entity_assessment + entity_quality (updated)

`entity_quality` already exists (migration 040, 9 columns — alpha/beta Bayesian scoring). The decomposition renames the existing `entity_intelligence` to clarify its role as the assessment layer.

**`entity_assessment`** — AI-generated intelligence (replaces `entity_intelligence`):
```sql
CREATE TABLE entity_assessment (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL DEFAULT 'account',
    enriched_at TEXT,
    source_file_count INTEGER DEFAULT 0,
    executive_assessment TEXT,
    risks_json TEXT,
    recent_wins_json TEXT,
    current_state_json TEXT,
    stakeholder_insights_json TEXT,
    next_meeting_readiness_json TEXT,
    company_context_json TEXT,
    value_delivered TEXT,
    success_metrics TEXT,
    open_commitments TEXT,
    relationship_depth TEXT
);
```

**`entity_quality`** — algorithmic quality + health metrics (existing table, extended):
```sql
-- Already exists. Add columns:
ALTER TABLE entity_quality ADD COLUMN health_score REAL;
ALTER TABLE entity_quality ADD COLUMN health_trend TEXT;
ALTER TABLE entity_quality ADD COLUMN coherence_score REAL;
ALTER TABLE entity_quality ADD COLUMN coherence_flagged INTEGER DEFAULT 0;
```

**Migration:** Move `health_score`, `health_trend`, `coherence_score`, `coherence_flagged` from `entity_intelligence` to `entity_quality`. Drop those columns from the source. Rename `entity_intelligence` → `entity_assessment`.

### Decomposition 3: entity_people + account_team → account_stakeholders

**`account_stakeholders`** — unified people-to-account linkage:
```sql
CREATE TABLE account_stakeholders (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL,
    role TEXT,                  -- from account_team (CSM, champion, etc.)
    relationship_type TEXT DEFAULT 'associated',  -- from entity_people
    data_source TEXT NOT NULL DEFAULT 'user',     -- ADR-0098 provenance
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id)
);
```

**Key decisions:**
- Primary key is `(account_id, person_id)` — one row per person per account. A person can have one role per account (not multiple). If a person is both "champion" and "technical_contact," use the higher-priority role.
- `data_source` tracks provenance per ADR-0098 (User > Clay > Glean > AI). Enables purge-on-revocation.
- `entity_people` rows where entity_type != 'account' (projects, people) migrate to a separate `entity_members` table or are handled by existing `project_members` / `person_relationships`.

**What about non-account entity_people?** `entity_people` currently links projects and people-entities too. Those are separate concerns:
- Project → person links: already covered by project-specific queries in `db/projects.rs`
- Person → person links: covered by `person_relationships` table
- Account → person links: this migration

Only the account→person case merges with `account_team`.

## Migration Strategy

### Phase: Single migration file, atomic transaction

```sql
BEGIN;

-- 1. Create new tables
CREATE TABLE meetings (...);
CREATE TABLE meeting_prep (...);
CREATE TABLE meeting_transcripts (...);
CREATE TABLE entity_assessment (...);
CREATE TABLE account_stakeholders (...);

-- 2. Copy data
INSERT INTO meetings SELECT id, title, meeting_type, start_time, end_time, attendees, calendar_event_id, description, created_at, last_viewed_at FROM meetings_history;

INSERT INTO meeting_prep SELECT id, prep_context_json, user_agenda_json, user_notes, prep_frozen_json, prep_frozen_at, prep_snapshot_path, prep_snapshot_hash FROM meetings_history;

INSERT INTO meeting_transcripts SELECT id, transcript_path, transcript_processed_at, notes_path, intelligence_state, intelligence_quality, last_enriched_at, signal_count, has_new_signals FROM meetings_history;

INSERT INTO entity_assessment SELECT entity_id, entity_type, enriched_at, source_file_count, executive_assessment, risks_json, recent_wins_json, current_state_json, stakeholder_insights_json, next_meeting_readiness_json, company_context_json, value_delivered, success_metrics, open_commitments, relationship_depth FROM entity_intelligence;

-- Move health/quality columns to entity_quality
ALTER TABLE entity_quality ADD COLUMN health_score REAL;
ALTER TABLE entity_quality ADD COLUMN health_trend TEXT;
ALTER TABLE entity_quality ADD COLUMN coherence_score REAL;
ALTER TABLE entity_quality ADD COLUMN coherence_flagged INTEGER DEFAULT 0;

UPDATE entity_quality SET
    health_score = (SELECT health_score FROM entity_intelligence WHERE entity_intelligence.entity_id = entity_quality.entity_id),
    health_trend = (SELECT health_trend FROM entity_intelligence WHERE entity_intelligence.entity_id = entity_quality.entity_id),
    coherence_score = (SELECT coherence_score FROM entity_intelligence WHERE entity_intelligence.entity_id = entity_quality.entity_id),
    coherence_flagged = (SELECT coherence_flagged FROM entity_intelligence WHERE entity_intelligence.entity_id = entity_quality.entity_id);

-- Merge entity_people (account type) + account_team → account_stakeholders
INSERT OR IGNORE INTO account_stakeholders (account_id, person_id, relationship_type, data_source)
    SELECT entity_id, person_id, relationship_type, 'user'
    FROM entity_people
    WHERE entity_id IN (SELECT id FROM accounts);

UPDATE account_stakeholders SET role = (
    SELECT role FROM account_team
    WHERE account_team.account_id = account_stakeholders.account_id
    AND account_team.person_id = account_stakeholders.person_id
    LIMIT 1
) WHERE EXISTS (
    SELECT 1 FROM account_team
    WHERE account_team.account_id = account_stakeholders.account_id
    AND account_team.person_id = account_stakeholders.person_id
);

-- Also insert account_team rows that weren't in entity_people
INSERT OR IGNORE INTO account_stakeholders (account_id, person_id, role, data_source)
    SELECT account_id, person_id, role, 'user'
    FROM account_team
    WHERE NOT EXISTS (
        SELECT 1 FROM account_stakeholders
        WHERE account_stakeholders.account_id = account_team.account_id
        AND account_stakeholders.person_id = account_team.person_id
    );

-- 3. Drop old tables
DROP TABLE meetings_history;
DROP TABLE entity_intelligence;
DROP TABLE entity_people;
DROP TABLE account_team;

-- 4. Recreate indexes
CREATE INDEX idx_meetings_start ON meetings(start_time);
CREATE INDEX idx_meetings_calendar ON meetings(calendar_event_id);
CREATE INDEX idx_meeting_transcripts_state ON meeting_transcripts(intelligence_state);
CREATE INDEX idx_entity_assessment_type ON entity_assessment(entity_type);
CREATE INDEX idx_account_stakeholders_person ON account_stakeholders(person_id);

COMMIT;
```

### Risk mitigation

1. **Backup before migration.** The migration runner should `VACUUM INTO 'dailyos-pre-migration.db'` before executing.
2. **Atomic transaction.** The entire migration is one transaction. If any step fails, nothing changes.
3. **Data validation after migration.** Post-migration check: `SELECT COUNT(*) FROM meetings` = `SELECT COUNT(*) FROM meetings_history` (pre-migration count stored in memory). Same for all tables.
4. **Backward compatibility during development.** Use a feature branch. The old schema works until the branch merges. No flag-based rollout — this is a hard cut.

## Code Changes Required

### Backend query migration

| Current table | New table(s) | Affected modules | Estimated call sites |
|--------------|-------------|-----------------|---------------------|
| `meetings_history` | `meetings` + `meeting_prep` + `meeting_transcripts` | `db/meetings.rs`, `services/meetings.rs`, `prepare/`, `processor/`, `intel_queue.rs`, `scheduler.rs` | ~150+ |
| `entity_intelligence` | `entity_assessment` + `entity_quality` | `intelligence/io.rs`, `intelligence/prompts.rs`, `self_healing/`, `prepare/meeting_context.rs`, `db/accounts.rs` | ~88 |
| `entity_people` + `account_team` | `account_stakeholders` | `db/accounts.rs`, `prepare/meeting_context.rs`, `intelligence/prompts.rs`, `signals/propagation.rs`, `devtools/` | ~113 |

**Strategy:** Find-and-replace table names in SQL strings. Many queries will need JOIN additions (e.g., a query that reads `meetings_history.title` AND `meetings_history.prep_frozen_json` now needs `meetings JOIN meeting_prep`). Create helper functions for common joins to avoid duplicating join logic.

### Frontend type changes

Frontend types don't reference table names directly. The Tauri commands return the same data shapes — the command handlers join internally. **Frontend changes should be zero or minimal** if command return types don't change.

## Acceptance Criteria

1. Migration runner is fail-hard in `run_migrations`: failed SQL is not swallowed and failed migrations are never marked applied.
2. Pre-migration backup is guaranteed before pending migrations, with timestamped files, retention capped to last 10, and encrypted-db fallback backup path.
3. `055_schema_decomposition.sql` executes atomically in a transaction.
4. Post-migration schema integrity checks fail closed when required tables/columns are missing.
5. `account_stakeholders.data_source` support is present through migration path.
6. Role merge policy is deterministic by explicit business priority (no alphabetical `MIN(role)` fallback).
7. Legacy tables are removed only after successful data copy and schema transition.
8. `cargo test` for migration module passes.

## Out of Scope

- Production recovery UI is tracked in I539.
- I508 intelligence schema redesign (new dimensions, new fields on entity_assessment) — separate issue, depends on this
- Query performance optimization — migrate first, optimize later
- Historical data backfill — migration handles existing data; no re-enrichment
- Non-account entity_people rows — projects handle their own member linkage
