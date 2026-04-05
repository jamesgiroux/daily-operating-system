# ADR-0099: Remote-First Architecture — Server-Canonical with Local Offline Cache

**Date:** 2026-03-02
**Status:** WITHDRAWN (2026-03-03). First-principles review found this architecture violates DailyOS's core identity: "server is canonical" contradicts "your brain shouldn't have a landlord"; syncing all intelligence to a shared DB contradicts "sharing happens at the output layer, never the signal layer"; the full sync/auth/RLS model contradicts "DailyOS is for the alone part." See `.docs/research/2026-03-03-architecture-first-principles-review.md` for analysis. The three real needs (governance, team views, org access) are solved by output-layer publication + Glean Agents, not server-canonical sync.
**Supersedes:** ADR-0018 (hybrid storage markdown+SQLite), ADR-0048 (three-tier data model with workspace files)
**Modifies:** ADR-0092 (encryption — extends to transit+rest), ADR-0095 (dual-mode context — collapses modes), ADR-0098 (governance — becomes server-side policy)
**Extends:** ADR-0091 (IntelligenceProvider), ADR-0094 (audit log)
**Research:** `2026-03-01-portfolio-intelligence-architecture.md` (Options A/B/C analysis), `2026-02-28-hook-gap-analysis.md` (VP problem analysis), `.docs/architecture/REARCHITECTURE-PROPOSAL.md` (6 workstreams)

---

## Context

### The Inflection Point

DailyOS was built local-first for good reasons: privacy, offline capability, zero infrastructure, direct user ownership. These principles produced a working product through v0.16.0 — briefings, meeting intelligence, reports, signal bus, editorial design. The local-first bet was correct for the first 98 ADRs.

But three forces now converge:

**1. The portfolio problem is unsolvable locally.** The 2026-03-01 portfolio intelligence architecture research proved that IC-to-VP intelligence flow requires a shared data layer. No amount of local-first engineering solves "the VP needs to see the IC's intelligence for accounts they don't personally touch." Every CS platform (Gainsight, Vitally, ChurnZero) uses a single canonical database with role-filtered views. DailyOS cannot compete for the VP seat without this.

**2. Governance must be institutional, not per-device.** ADR-0098 designed purge-on-revocation as a local operation — each device independently detects token failure and purges. In a team deployment, this is insufficient. When an IC leaves the company, their data must be purged from the shared layer by an administrator, not by their abandoned laptop detecting an expired token. Governance requires a server that enforces policy.

**3. The workspace file duality is the #1 source of bugs.** The architecture audit identified workspace files (`_today/data/`, `intelligence.json`, directive JSON) as the primary reliability problem. Stale skeleton files block real data. Two sources of truth (disk + DB) diverge silently. The rearchitecture proposal already recommends eliminating workspace files (Workstream 5). Going remote-first makes this elimination complete — there is no local file layer at all.

### What We Learned from the Architecture Audit

The full codebase audit (14 architecture documents, 98 ADRs, 40K+ lines of Rust, 50K+ lines of TypeScript) revealed that the local-first architecture's structural problems — god modules, voluntary service boundaries, data model entropy, silent pipeline failures — are all amplified by the workspace file duality. Every pipeline reads from or writes to disk files that may be stale, missing, or inconsistent with the DB. Removing the file layer is prerequisite to fixing the pipelines. Going remote-first provides the forcing function.

### The Linear Precedent

Linear's architecture is the direct reference model:

- **Local store:** IndexedDB (browser) or SQLite (desktop) — full local copy of workspace data
- **Sync:** Every mutation creates an immutable `SyncAction` with a server-assigned sequential ID
- **Offline:** Local changes are optimistic, queued for push on reconnect
- **Conflict resolution:** Last-writer-wins by server-assigned sequential ID (total ordering)
- **Team visibility:** All users in a workspace see the same data; scoping is at the query level
- **Server is canonical:** Local changes that the server rejects are rolled back

DailyOS adopts this model with one difference: intelligence synthesis still runs on the client (via IntelligenceProvider — Claude Code, Ollama, or OpenAI API). The server stores the result, not the process.

---

## Decision

### The Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Remote DB (Supabase)                       │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  Postgres    │  │  Auth        │  │  Row-Level Security │  │
│  │  (canonical) │  │  (identity)  │  │  (scope filtering)  │  │
│  └─────────────┘  └──────────────┘  └────────────────────┘  │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  Realtime    │  │  Edge Fns    │  │  Storage (files)    │  │
│  │  (sync)      │  │  (rollups)   │  │  (exports)          │  │
│  └─────────────┘  └──────────────┘  └────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
           ▲              │              ▲
           │ push         │ broadcast    │ query
           │ mutations    │ changes      │ rollups
           │              ▼              │
┌──────────────────────────────────────────────────────────────┐
│                  Sync Engine (Rust)                           │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  SyncAction  │  │  Conflict    │  │  Delta Sync        │  │
│  │  Queue       │  │  Resolution  │  │  (/sync/delta)     │  │
│  └─────────────┘  └──────────────┘  └────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
           ▲              │
           │ write        │ read
           │              ▼
┌──────────────────────────────────────────────────────────────┐
│              Local SQLite (Offline Cache)                     │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  Same schema │  │  sync_actions│  │  sync_state        │  │
│  │  as remote   │  │  (outbox)    │  │  (last_sync_id)    │  │
│  └─────────────┘  └──────────────┘  └────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
           ▲              │
           │ service      │ read
           │ layer        ▼
┌──────────────────────────────────────────────────────────────┐
│              Application (Tauri + React)                      │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  ServiceLayer│  │  Intelligence│  │  Frontend (React)   │  │
│  │  (mutations) │  │  Pipeline    │  │  (editorial UI)     │  │
│  └─────────────┘  └──────────────┘  └────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### Principle 1: Server is Canonical

The remote Postgres database (via Supabase) is the single source of truth. Local SQLite is an offline cache that mirrors the server schema exactly. When online, all mutations write to local SQLite AND generate a `SyncAction` that is pushed to the server. The server assigns a sequential ID and broadcasts the change to all team members.

This is the Linear model. Local-first in experience (instant writes, offline capable), server-first in authority (server resolves conflicts, enforces governance).

### Principle 2: Intelligence Synthesis Remains Client-Side

The IntelligenceProvider abstraction (ADR-0091) is unchanged. Claude Code, Ollama, or OpenAI API calls still originate from the user's machine. The AI synthesis runs locally. Only the **result** (the `IntelligenceJson` output) syncs to the server.

This preserves:
- Privacy — raw prompts stay local; only structured output syncs
- Flexibility — users choose their LLM provider
- Cost — no server-side LLM infrastructure
- The IntelligenceProvider trait is unmodified

### Principle 3: Governance is Server-Side

ADR-0098's purge-on-revocation becomes a **server-side policy**, not a client-side operation:

- Admin revokes a user's access → server deletes their data from Postgres → broadcast deletion to all clients caching that user's data
- Data source revocation (Glean, Google) → server-side cascade, not per-laptop detection
- Audit log (ADR-0094) moves from a local append-only file to a server-side table with tamper-evident hashing
- Retention policies enforced by the server, not by each client independently

### Principle 4: Workspace Files Are Eliminated

No more `_today/data/`, no `intelligence.json` on disk, no directive JSON files, no `entity_intelligence/{id}.json`. The DB (local cache ← server canonical) is the only data layer. Workspace files are eliminated entirely — not demoted to "export only," but removed from the architecture.

If a user needs an offline export, it's generated on demand from the DB, not maintained as a live file tree.

### Principle 5: Same App, Wider Scope

Every user runs the same Tauri app. The difference between IC, territory lead, and VP is **scope** — which rows they can see — not which features they have.

- IC sees their accounts (RLS: `WHERE owner_id = auth.uid()`)
- Territory lead sees their territory (RLS: `WHERE territory_id IN (my_territories)`)
- VP sees all (RLS: no filter, or `WHERE org_id = auth.org()`)

The editorial design, intelligence pipeline, report suite, and signal bus are identical across roles. The frontend queries the same tables with different scope.

---

## The Sync Engine

### SyncAction Model

```rust
pub struct SyncAction {
    /// Client-generated UUID (for dedup)
    pub client_id: String,
    /// Server-assigned sequential ID (for ordering)
    pub server_id: Option<i64>,
    /// The table being mutated
    pub table_name: String,
    /// The row being mutated
    pub row_id: String,
    /// The mutation type
    pub action: SyncActionType,  // Insert | Update | Delete
    /// Field-level changes (for updates)
    pub changeset: serde_json::Value,
    /// Source priority for conflict resolution (ADR-0098)
    pub data_source: DataSource,
    /// Timestamp of the local mutation
    pub created_at: String,
    /// The user who made the change
    pub user_id: String,
}

pub enum SyncActionType {
    Insert,
    Update { fields: Vec<String> },
    Delete,
}
```

### Sync Flow

**Online (normal):**
1. User action → `ServiceLayer` mutation → local SQLite write
2. `ServiceLayer` generates `SyncAction` → inserts into local `sync_actions` outbox
3. Background sync processor reads outbox → pushes to server via Supabase Realtime
4. Server assigns `server_id` → broadcasts to all team members
5. Other clients receive broadcast → apply to their local SQLite cache
6. Local outbox entry marked as synced

**Offline:**
1. User action → `ServiceLayer` mutation → local SQLite write
2. `SyncAction` generated → queued in local `sync_actions` outbox
3. App detects offline (Supabase Realtime disconnected)
4. Queue grows while offline
5. On reconnect: push all queued `SyncAction`s in order
6. Server assigns `server_id`s → broadcasts
7. If conflict: server's field-level LWW with `data_source` priority resolves it
8. Client receives resolution → applies to local cache (may roll back optimistic writes)

### Conflict Resolution

**Field-level last-writer-wins (LWW) with source priority:**

1. Server assigns a sequential `server_id` to every `SyncAction`
2. For the same `(table, row_id, field)`, the action with the highest `server_id` wins
3. **Exception:** If two actions conflict and one has higher `data_source` priority (User > Glean > AI), the higher-priority source wins regardless of `server_id`
4. This reuses ADR-0098's source priority model — user corrections always survive

**Example:**
- IC sets account health to "at_risk" (source: `user`, server_id: 1001)
- Glean enrichment sets health to "healthy" (source: `glean`, server_id: 1002)
- Resolution: IC's `user` source wins despite lower server_id (User > Glean)

### Delta Sync

On app launch (or reconnect after offline):
```
GET /sync/delta?last_sync_id={local_max_server_id}
→ Returns all SyncActions since that ID
→ Client applies in order
→ Updates local sync_state.last_sync_id
```

This is exactly Linear's model. No CRDT complexity. No merge conflicts for structured data. Total ordering via server-assigned IDs.

---

## Authentication and Team Model

### Identity (Supabase Auth)

```sql
-- Supabase auth.users (managed by Supabase Auth)
-- DailyOS extends with:
CREATE TABLE user_profiles (
    id UUID PRIMARY KEY REFERENCES auth.users(id),
    display_name TEXT NOT NULL,
    email TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'ic',  -- 'ic' | 'lead' | 'vp' | 'admin'
    manager_id UUID REFERENCES user_profiles(id),
    territory_id UUID REFERENCES territories(id),
    -- Professional context (from ADR-0089/0090 user entity)
    value_proposition TEXT,
    current_priorities_json TEXT,
    role_preset TEXT DEFAULT 'customer_success',
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE territories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    org_id UUID NOT NULL REFERENCES organizations(id),
    parent_territory_id UUID REFERENCES territories(id),  -- for nested territories
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    glean_config_json TEXT,  -- Glean connection settings (org-level)
    created_at TIMESTAMPTZ DEFAULT now()
);
```

### Account Ownership

```sql
CREATE TABLE account_assignments (
    account_id UUID NOT NULL REFERENCES accounts(id),
    user_id UUID NOT NULL REFERENCES user_profiles(id),
    role TEXT NOT NULL DEFAULT 'owner',  -- 'owner' | 'collaborator' | 'viewer'
    assigned_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (account_id, user_id)
);
```

### Row-Level Security

```sql
-- IC sees their accounts
CREATE POLICY account_ic_policy ON accounts
    FOR ALL USING (
        id IN (SELECT account_id FROM account_assignments WHERE user_id = auth.uid())
    );

-- Territory lead sees their territory's accounts
CREATE POLICY account_lead_policy ON accounts
    FOR ALL USING (
        id IN (
            SELECT aa.account_id FROM account_assignments aa
            JOIN user_profiles up ON up.id = aa.user_id
            WHERE up.territory_id = (SELECT territory_id FROM user_profiles WHERE id = auth.uid())
        )
    );

-- VP/Admin sees all accounts in their org
CREATE POLICY account_vp_policy ON accounts
    FOR ALL USING (
        org_id = (SELECT org_id FROM user_profiles WHERE id = auth.uid())
    );
```

---

## Schema: Remote Postgres

The remote schema incorporates the data model consolidation from the rearchitecture proposal (Workstream 3) but designed for Postgres first:

### Core tables (mirrored locally in SQLite)

```sql
-- Accounts (from rearchitecture: unified entity model)
CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    domain TEXT,
    lifecycle_stage TEXT DEFAULT 'active',
    arr NUMERIC,
    renewal_date DATE,
    parent_account_id UUID REFERENCES accounts(id),
    metadata_json JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- Meetings (decomposed from meetings_history — 4 tables)
CREATE TABLE meetings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    title TEXT NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    calendar_event_id TEXT,
    organizer TEXT,
    organizer_email TEXT,
    location TEXT,
    meeting_type TEXT DEFAULT 'external',
    attendee_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE meeting_prep (
    meeting_id UUID PRIMARY KEY REFERENCES meetings(id),
    prep_frozen_json JSONB,
    prep_frozen_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE meeting_intelligence (
    meeting_id UUID PRIMARY KEY REFERENCES meetings(id),
    intelligence_state TEXT DEFAULT 'pending',
    intelligence_quality JSONB,
    last_enriched_at TIMESTAMPTZ,
    signal_count INTEGER DEFAULT 0,
    has_new_signals BOOLEAN DEFAULT FALSE,
    last_viewed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- Entity Assessment (decomposed from entity_intelligence — 2 tables)
CREATE TABLE entity_assessment (
    entity_id UUID PRIMARY KEY,
    entity_type TEXT NOT NULL,
    org_id UUID NOT NULL,
    -- v1.1.0 I508 six dimensions
    strategic_assessment_json JSONB,
    relationship_health_json JSONB,
    engagement_cadence_json JSONB,
    value_outcomes_json JSONB,
    commercial_context_json JSONB,
    external_health_json JSONB,
    -- Legacy fields (migrated)
    executive_assessment TEXT,
    risks_json JSONB,
    recent_wins_json JSONB,
    enriched_at TIMESTAMPTZ,
    last_enriched_at TIMESTAMPTZ,
    enriched_by UUID REFERENCES user_profiles(id),  -- who triggered enrichment
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE entity_quality (
    entity_id UUID PRIMARY KEY,
    entity_type TEXT NOT NULL,
    health_score REAL,
    health_trend TEXT,
    health_confidence REAL,
    coherence_score REAL,
    coherence_flagged BOOLEAN DEFAULT FALSE,
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- Unified stakeholder table (replaces entity_people + account_team)
CREATE TABLE account_stakeholders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES accounts(id),
    person_id UUID NOT NULL REFERENCES people(id),
    role TEXT,
    title TEXT,
    email TEXT,
    data_source TEXT DEFAULT 'user',
    engagement_level TEXT,
    last_meeting_date DATE,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(account_id, person_id)
);

-- Signals (unchanged schema, server-side now)
CREATE TABLE signal_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    signal_type TEXT NOT NULL,
    source TEXT NOT NULL,
    value JSONB,
    confidence REAL DEFAULT 0.5,
    decayed_weight REAL DEFAULT 0.5,
    half_life_days INTEGER DEFAULT 30,
    emitted_by UUID REFERENCES user_profiles(id),
    created_at TIMESTAMPTZ DEFAULT now()
);

-- Sync infrastructure
CREATE TABLE sync_actions (
    server_id BIGSERIAL PRIMARY KEY,
    client_id TEXT NOT NULL UNIQUE,
    user_id UUID NOT NULL REFERENCES user_profiles(id),
    org_id UUID NOT NULL REFERENCES organizations(id),
    table_name TEXT NOT NULL,
    row_id TEXT NOT NULL,
    action_type TEXT NOT NULL,  -- 'insert' | 'update' | 'delete'
    changeset JSONB NOT NULL,
    data_source TEXT DEFAULT 'user',
    created_at TIMESTAMPTZ DEFAULT now()
);

-- Materialized views for portfolio rollups
CREATE MATERIALIZED VIEW portfolio_health AS
SELECT
    up.id AS user_id,
    up.territory_id,
    COUNT(DISTINCT a.id) AS account_count,
    SUM(a.arr) AS total_arr,
    AVG(eq.health_score) AS avg_health,
    COUNT(CASE WHEN eq.health_score < 40 THEN 1 END) AS at_risk_count,
    COUNT(CASE WHEN a.renewal_date BETWEEN now() AND now() + interval '90 days' THEN 1 END) AS renewals_90d
FROM accounts a
JOIN account_assignments aa ON aa.account_id = a.id
JOIN user_profiles up ON up.id = aa.user_id
LEFT JOIN entity_quality eq ON eq.entity_id = a.id::text
GROUP BY up.id, up.territory_id;

-- Refresh on every intelligence update
CREATE OR REPLACE FUNCTION refresh_portfolio_health()
RETURNS TRIGGER AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY portfolio_health;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_refresh_portfolio
AFTER INSERT OR UPDATE ON entity_quality
FOR EACH STATEMENT EXECUTE FUNCTION refresh_portfolio_health();
```

### Server-Side Governance

```sql
-- Admin purges a departed user's data
CREATE OR REPLACE FUNCTION purge_user_data(target_user_id UUID)
RETURNS TABLE(table_name TEXT, rows_deleted BIGINT) AS $$
BEGIN
    -- Delete intelligence they produced
    DELETE FROM entity_assessment WHERE enriched_by = target_user_id;
    -- Remove their account assignments
    DELETE FROM account_assignments WHERE user_id = target_user_id;
    -- Remove their signals
    DELETE FROM signal_events WHERE emitted_by = target_user_id;
    -- Deactivate their profile (don't delete — audit trail)
    UPDATE user_profiles SET role = 'deactivated' WHERE id = target_user_id;
    -- Broadcast deletion via sync_actions
    INSERT INTO sync_actions (client_id, user_id, org_id, table_name, row_id, action_type, changeset)
    VALUES (gen_random_uuid()::text, target_user_id, ..., 'user_profiles', target_user_id::text, 'delete', '{}');
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Source revocation cascades server-side
CREATE OR REPLACE FUNCTION purge_data_source(source_name TEXT, target_org_id UUID)
RETURNS void AS $$
BEGIN
    DELETE FROM signal_events WHERE source = source_name AND org_id = target_org_id;
    DELETE FROM account_stakeholders WHERE data_source = source_name
        AND account_id IN (SELECT id FROM accounts WHERE org_id = target_org_id);
    -- Flag intelligence for re-enrichment
    UPDATE entity_assessment SET enriched_at = NULL
        WHERE org_id = target_org_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
```

---

## What Changes in the Rearchitecture Proposal

The 6 workstreams from `REARCHITECTURE-PROPOSAL.md` are modified:

### Workstream 1: Service Layer → Sync-Aware Service Layer

**Before:** `ServiceLayer` wraps local DB writes and ensures signal emission.
**After:** `ServiceLayer` wraps local DB writes, generates `SyncAction`s, and ensures signal emission. Every mutation has three effects:
1. Local SQLite write (immediate, for UI responsiveness)
2. `SyncAction` generated and queued (for server sync)
3. Signal emission (for propagation, prep invalidation, etc.)

```rust
impl ServiceLayer {
    pub async fn update_account(&self, id: &str, fields: AccountUpdate) -> Result<DbAccount> {
        // 1. Local write
        let account = self.state.with_db_write(|db| db.update_account(id, &fields))?;

        // 2. Generate SyncAction
        self.sync_engine.enqueue(SyncAction {
            client_id: Uuid::new_v4().to_string(),
            table_name: "accounts".into(),
            row_id: id.into(),
            action: SyncActionType::Update { fields: fields.changed_fields() },
            changeset: serde_json::to_value(&fields)?,
            data_source: DataSource::User,
            ..Default::default()
        });

        // 3. Signal emission
        self.emit_signal_and_propagate(/* ... */).await?;

        Ok(account)
    }
}
```

### Workstream 3: Data Model → Design for Postgres First, Mirror to SQLite

**Before:** Decompose SQLite tables.
**After:** Design the canonical schema in Postgres (above). Local SQLite mirrors it exactly. The decomposition is the same (meetings → 4 tables, entity_intelligence → 2 tables, unified stakeholders) but the schema is designed for Postgres types (UUID, TIMESTAMPTZ, JSONB) and translated to SQLite equivalents (TEXT for UUID, TEXT for TIMESTAMPTZ, TEXT for JSONB).

### Workstream 5: Workspace File Elimination → Complete

**Before:** Workspace files become export-only.
**After:** Workspace files are eliminated entirely. No `_today/data/`, no `intelligence.json`, no directive files. All data lives in DB (local cache ↔ server canonical). The `prepare/orchestrate.rs` pipeline writes to DB tables instead of JSON files. The `workflow/deliver.rs` pipeline reads from DB tables instead of directive files.

### NEW Workstream 7: Sync Engine

A new workstream that doesn't exist in the current proposal:

**Scope:**
- `SyncAction` queue (local outbox table)
- Background sync processor (push outbox, receive broadcasts)
- Delta sync on reconnect (`/sync/delta?last_sync_id=X`)
- Conflict resolution (field-level LWW with source priority)
- Offline detection and queue management
- Supabase Realtime client integration

**Files:**
| File | Purpose |
|------|---------|
| `src-tauri/src/sync/mod.rs` | SyncEngine struct, public API |
| `src-tauri/src/sync/outbox.rs` | Local outbox management, queue/dequeue |
| `src-tauri/src/sync/push.rs` | Push SyncActions to Supabase |
| `src-tauri/src/sync/receive.rs` | Receive broadcasts, apply to local cache |
| `src-tauri/src/sync/conflict.rs` | Field-level LWW resolution |
| `src-tauri/src/sync/delta.rs` | Delta sync on reconnect |
| `src-tauri/src/sync/online.rs` | Online/offline detection, retry logic |

### NEW Workstream 8: Authentication and Team Model

**Scope:**
- Supabase Auth integration (email + Google OAuth)
- User profile management
- Organization and territory model
- Account assignment (IC ↔ account ownership)
- RLS policy design
- Team page in UI (Settings → Team)
- Onboarding modification (join org vs. create org)

**Files:**
| File | Purpose |
|------|---------|
| `src-tauri/src/auth/mod.rs` | Supabase Auth client, session management |
| `src-tauri/src/auth/session.rs` | Token storage (Keychain), refresh logic |
| `src-tauri/src/team/mod.rs` | Organization, territory, assignment CRUD |
| `src-tauri/src/team/rls.rs` | RLS-aware query scoping |
| `src/pages/TeamPage.tsx` | Team management UI |
| `src/components/auth/LoginFlow.tsx` | Auth flow |

---

## What Stays the Same

1. **Tauri + React + SQLite (as cache)** — the desktop app architecture is unchanged
2. **Editorial design language** — the UI doesn't change
3. **Signal bus** — same Bayesian fusion, same propagation rules, same feedback loop
4. **IntelligenceProvider** — Claude Code, Ollama, OpenAI — all unchanged
5. **ContextProvider** — LocalContextProvider and GleanContextProvider both work, but now read from local cache (which mirrors server) instead of local-only DB
6. **Report suite** — all report types work the same way; they just have access to more data (team's intelligence, not just current user's)
7. **Meeting prep pipeline** — unchanged; reads from DB, synthesizes via IntelligenceProvider
8. **Role presets** — unchanged; shape vocabulary and UI
9. **Product vocabulary** — unchanged; users never see "sync" or "server"

---

## Migration Path

### Phase 1: Schema + Service Layer (no sync yet)

1. Implement the Postgres schema on Supabase (remote, empty)
2. Implement the decomposed SQLite schema locally (Workstream 3 from rearchitecture)
3. Build the `ServiceLayer` with `SyncAction` generation but **don't push yet** — SyncActions queue locally
4. Migrate all command handlers to use `ServiceLayer`
5. Eliminate workspace files (DB writes only)

**At this point:** The app works exactly as before (single-user, local-only) but with a cleaner schema and no workspace files. SyncActions accumulate locally but go nowhere.

### Phase 2: Sync Engine

1. Implement Supabase Realtime client in Rust
2. Implement push (outbox → server)
3. Implement receive (broadcast → local cache)
4. Implement delta sync (reconnect)
5. Implement conflict resolution

**At this point:** The app syncs to the server. Single-user still works (they're the only member of their org). No team features yet, but data is durable on the server.

### Phase 3: Auth + Teams

1. Implement Supabase Auth integration
2. Implement org/territory/assignment model
3. Implement RLS policies
4. Build onboarding flow (create org vs. join org)
5. Build team management UI

**At this point:** Multiple users can share an org. ICs see their accounts. Leads see their territory. VPs see everything.

### Phase 4: Portfolio Intelligence

1. Server-side materialized views for portfolio rollups
2. Portfolio Health page reads from rollup views
3. VP Account Review reads from team's intelligence
4. Portfolio Health Summary synthesizes across accounts

**At this point:** The VP problem is solved. Intelligence flows up the org chart.

---

## What This Means for the Backlog

### Issues Absorbed by This ADR

The following existing issues are subsumed by this architectural change:

- **I436** (Workspace file deprecation) — fully absorbed; workspace files eliminated in Phase 1
- **I380** (commands.rs service extraction) — absorbed into ServiceLayer workstream
- **I381** (db/mod.rs domain migration) — absorbed into schema redesign
- **I402** (IntelligenceService extraction) — absorbed into ServiceLayer
- **I403** (SignalService formalization) — absorbed into ServiceLayer
- **I404/I405** (AppState decomposition) — absorbed into sync-aware state design
- **I450-I454** (Service extractions) — all absorbed into ServiceLayer
- **I491** (Portfolio Health Summary) — enabled by Phase 4
- **I492** (Portfolio Health page) — enabled by Phase 4

### Issues Modified by This ADR

- **I508** (Intelligence schema redesign) — schema is now Postgres-first, mirrored to SQLite
- **I499-I503** (Health scoring) — `entity_quality` table lives on server; rollups are materialized views
- **I504-I506** (Relationship intelligence) — relationships sync across team
- **I487** (Glean signal emission) — signals sync to server
- **ADR-0098** (Data governance) — becomes server-side policy

### New Issues Required

1. Supabase project provisioning + schema deployment
2. Sync engine implementation (SyncAction model)
3. Supabase Auth integration
4. Organization + territory model
5. Account assignment model + RLS
6. Onboarding flow redesign (auth-first)
7. Offline mode redesign (sync-aware)
8. Data export redesign (server-side export)
9. Server-side embedding pipeline (Voyage-3/Cohere)
10. Admin panel (user management, source revocation, audit)

---

## Risks

### Technical Risks

1. **Sync complexity.** Linear spent years perfecting their sync engine. DailyOS is building one from scratch. Mitigation: start simple (push/pull, no real-time subscriptions), add sophistication incrementally.

2. **Offline → online conflict storms.** A user working offline for 8 hours generates hundreds of SyncActions. On reconnect, field-level LWW may produce unexpected resolutions. Mitigation: conflict UI (show user what was resolved and how).

3. **SQLite ↔ Postgres type mismatches.** UUID, TIMESTAMPTZ, JSONB in Postgres become TEXT in SQLite. Serialization/deserialization must be exact. Mitigation: shared schema definition generates both Postgres migrations and SQLite migrations.

### Product Risks

4. **Deployment complexity.** Current DailyOS: download `.dmg`, open, done. Remote-first: create org, invite team, manage auth. This adds friction. Mitigation: solo mode (create org for yourself, no team, minimal setup) as default onboarding path.

5. **Cost.** Supabase free tier: 500MB database, 50K monthly active users, 2GB file storage. Sufficient for early adoption. Paid tier ($25/mo) for production orgs.

6. **"Your brain has a landlord" optics.** The positioning doc says "Anti-SaaS." Going remote contradicts this messaging. Mitigation: the server is the org's server (or self-hosted Supabase). DailyOS doesn't operate the database. The org does. This is governance, not tenancy.

### Governance Risks

7. **Data residency.** Enterprise orgs may require data in specific regions. Supabase supports region selection. Self-hosted Supabase is an option for strict compliance.

8. **Audit completeness.** Server-side audit log must capture every mutation, every sync conflict resolution, every governance action. The existing ADR-0094 audit design extends naturally.

---

## Consequences

### Positive
- Solves the VP/portfolio problem that is unsolvable locally
- Eliminates workspace file duality (the #1 bug source)
- Enables team governance (admin-controlled purge, RLS, audit)
- Positions DailyOS to compete with Gainsight/Vitally/ChurnZero for team deployments
- Server-side materialized views solve the "VP queries 200 files" problem
- Data survives device loss (synced to server)

### Negative
- Adds infrastructure dependency (Supabase)
- Adds operational complexity (auth, sync, conflict resolution)
- Solo users must create an "org of one" (friction)
- Contradicts ADR-0007's philosophical stance on cloud independence
- Requires significant engineering effort (4 new workstreams)

### What We're Choosing

We're choosing to build a **team-capable product** over a **solo-only tool**. The local-first principles (offline capability, fast reads, privacy of synthesis process) are preserved. What changes is that data is durable on a server and visible to authorized team members. The user's brain still doesn't have a "landlord" — but it does have "colleagues."

---

## Decision Record

| Question | Answer |
|----------|--------|
| Is this reversible? | Yes — the app works offline. If the server disappears, the local cache has all data. Migration back to local-only is possible. |
| Who must approve? | Product owner. This changes the product's deployment model. |
| When does this take effect? | Phase 1 (schema + service layer) begins immediately as part of v1.0.0 rearchitecture. |
| What version ships this? | Phase 1-2: v1.0.0. Phase 3-4: v1.1.0. |
