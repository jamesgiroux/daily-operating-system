# Portfolio Intelligence Architecture: IC-to-VP Data Flow

**Date:** 2026-03-01
**Status:** Complete — feeds into I489 revision and new prerequisite issues
**Context:** I489 (VP Account Review) assumes a VP can generate reports about accounts they don't own. But DailyOS is single-user, local-first — the VP's instance has no access to IC intelligence data. This research examines how intelligence flows upward through an org hierarchy, what infrastructure is needed, and how DailyOS's existing Google Drive integration could serve as the shared layer.

---

## The Problem

DailyOS's intelligence architecture works at one level: the individual contributor. Each IC's instance produces `intelligence.json` per account from their calendar, email, transcripts, and (optionally) Glean. This intelligence is excellent — deep, cited, signal-driven.

But intelligence doesn't flow upward. A territory lead can't see their team's accounts. A VP can't generate a portfolio health view. The reports in Phase 2 (I489-I491, I496-I498) assume access to intelligence data that doesn't exist outside the IC's local machine.

The question isn't "how do we build reports?" — it's "how does intelligence move up the org chart?"

### What Exists Today

DailyOS already has sophisticated **local** hierarchy intelligence:

- **Parent/child accounts** with n-level nesting, recursive CTEs for ancestor/descendant queries
- **Bidirectional signal propagation** — child signals propagate up at 60% confidence with 48-hour sibling fusion; parent signals propagate down at 50% confidence with a 0.7 threshold
- **Portfolio synthesis** — parent accounts include children's intelligence in their enrichment prompt, producing `PortfolioIntelligence` (health summary, hotspots, cross-BU patterns, narrative)
- **Prep invalidation cascade** — child intelligence changes → parent prep cleared → parent re-enriched

This is a complete intelligence hierarchy — **within one user's local instance**. The gap is cross-user: multiple ICs, each with their own local DB, producing intelligence that never converges.

### What Doesn't Exist

- No multi-user support (zero authentication, zero remote sync, zero access control)
- No shared data store (each instance is isolated SQLite)
- No concept of "my team's accounts" vs "all accounts"
- No user hierarchy (no manager/report relationships)
- No remote aggregation of any kind

---

## Part 1: How CS Platforms Solve This

### Universal Pattern: One Store, Filtered Views

Every CS platform uses the same architecture: **a single canonical database with role-based query filters.**

**Gainsight:** Multi-tenant PostgreSQL + Redshift. CSM sees "my accounts" via `WHERE csm_id = current_user`. Manager sees "my team's accounts" via `WHERE csm_id IN (my_reports)`. VP sees all. The User object has a `Manager` field defining the hierarchy. No aggregation from separate stores — just wider filter scope.

**Vitally:** Organization > Account > Users hierarchy. Rollup fields on Organization auto-summarize child Account data. "Hubs" are tailored workspaces — a CSM creates a personal Hub scoped to their book, a manager creates a team Hub. Each applies a scope toggle. Same underlying data, different views.

**ChurnZero:** Same pattern. Single data store, segments filter by owner/team, executive dashboards show pre-computed rollups.

**Key insight:** No CS platform aggregates from separate IC databases. The "aggregation" is pre-computed rollups on a shared store, not synthesis from distributed data. The VP doesn't run queries across IC machines — they query the same database the IC writes to.

### What This Means for DailyOS

The industry pattern assumes a shared server database. DailyOS is local-first by design — that's a feature, not a bug (privacy, offline capability, no vendor dependency). But scaling to teams requires some form of shared layer. The question is: what serves as the shared canonical store while preserving local-first principles?

---

## Part 2: Local-First Tools That Scaled to Teams

### Linear: Changeset Replay with Server Authority

Linear's architecture is the most relevant reference:

- **Local store:** IndexedDB per user-workspace. Full local copy of workspace data.
- **Sync:** Every mutation creates an immutable `SyncAction` with a server-assigned sequential ID. Changes queue locally, sync to server on connectivity. Server broadcasts SyncActions to all team members.
- **Conflict resolution:** Last-writer-wins by sequential SyncAction ID. Server assigns IDs, enforcing total ordering. No CRDT complexity.
- **Offline:** Local changes are optimistic. On reconnect, fetch delta via `/sync/delta?lastSyncId=X`, replay missed SyncActions.
- **Team visibility:** All users in a workspace see the same data. Scoping is UI-level (my issues vs. team issues), not data-level.

**Key design:** The local database is a subset of the server database. Local changes that the server rejects are rolled back. Server is always canonical.

### Notion: SQLite Cache + CRDT Merge

- **Local store:** SQLite via WASM. Pages marked for offline use get CRDT data model.
- **Sync:** Push-based per-page channels. Clients compare timestamps, fetch newer versions.
- **Conflict resolution:** CRDTs for offline pages. Server arbitrates final state.
- **Partial replication:** Only first 50 rows of databases sync offline. Full data lives on server.

### Obsidian: File-Based Sync (Cautionary)

- **Local store:** Plain markdown files. Each user has a local vault.
- **Sync:** Bidirectional file sync to shared remote vault.
- **Conflict resolution:** Conflict copies. Manual resolution required.
- **Lesson:** File-based sync for teams is fragile. Conflict copies don't work for structured intelligence data.

### Pattern Summary

| Tool | Sync Model | Conflict Resolution | Canonical Store |
|------|-----------|-------------------|-----------------|
| Linear | Changeset replay | LWW by server-assigned ID | Server |
| Notion | Push per-page | CRDTs for offline | Server |
| Obsidian | File sync | Conflict copies | None (distributed) |

**Conclusion:** Every successful local-first team tool has a server as the canonical store. Local is the fast cache; server is the truth. The question is what form the server takes.

---

## Part 3: Architecture Options for DailyOS

### Option A: Google Drive as Shared Intelligence Store

DailyOS already has a full Google Drive integration (I426):
- OAuth `drive` scope requested
- Drive API client with Changes API polling (O(1) per cycle)
- File download/upload, markdown conversion
- `drive_watched_sources` table with entity linkage
- Background poller with adaptive intervals
- Frontend: Google Picker, import modal, connector card

**How it would work:**

```
IC's DailyOS ──enriches──> local SQLite ──pushes──> Shared Drive
                                                       │
                                         Changes API (poll every 10 min)
                                                       │
Territory Lead's DailyOS ──pulls──> local SQLite ──> portfolio synthesis
                                                       │
VP's DailyOS ──pulls──> local SQLite ──> org-wide synthesis
```

- **Shared Drive** = org's intelligence layer. Folder hierarchy: `/territories/west/accounts/acme/intelligence.json`
- **IC writes** to Shared Drive after each enrichment cycle (existing Drive API client)
- **Territory lead reads** their territory folder via Changes API (already built)
- **VP reads** all folders
- **Conflict resolution:** ETag-based optimistic locking (`If-Match` header, 412 on conflict → re-read, field-level merge, retry)
- **Scoping:** Shared Drive "Limited Access Folders" — VP has `organizer` role (sees all), territory leads have `writer` on their territory folder only

**Advantages:**
- Build on existing infrastructure (Drive API client, poller, OAuth already integrated)
- Zero new server infrastructure — Google Drive IS the server
- Governance via Google Workspace (admin-managed, DLP, audit trail, retention)
- Files survive employee departure (Shared Drives are org-owned)
- Well within API rate limits (500 files, 20 users ≈ 0.1% of 12,000 queries/min quota)

**Disadvantages:**
- Polling latency (10-minute cycles, not real-time)
- File-level granularity — no row-level queries, no SQL aggregation
- JSON merge complexity for concurrent edits (rare but possible)
- `drive.file` scope insufficient for cross-user access — requires full `drive` scope (already requested but restricted, may need Google security review for verified app)
- No compute on the shared layer — all synthesis happens client-side
- VP with 200 accounts would need to pull 200 JSON files to generate portfolio view

### Option B: Remote Database

A purpose-built remote database as the canonical store.

**How it would work:**

```
IC's DailyOS ──enriches──> local SQLite ──sync mutations──> Remote DB
                                                               │
                                                    Real-time subscriptions
                                                               │
Territory Lead's DailyOS <──delta sync──> Remote DB
                                                               │
VP's DailyOS <──rollup queries──> Remote DB (pre-computed aggregates)
```

- **Remote DB** (Supabase/Turso/Planetscale) = canonical store
- **Local SQLite** remains the IC's primary store (fast, offline-capable)
- **Sync:** Changeset replay (Linear model). Local mutations generate SyncActions, pushed to remote. Server broadcasts to team.
- **Conflict resolution:** Server-assigned sequential IDs (total ordering). Field-level LWW with source priority.
- **Scoping:** Row-level security policies. VP sees all rows; territory lead sees their territory's rows.
- **Aggregation:** Server-side materialized views — portfolio health rollups computed on write, not on VP's query.
- **Embeddings:** Frontier embedding models (Voyage-3, Cohere Embed v4) run server-side. Better semantic search for signal relevance, gap detection, and meeting prep.

**Advantages:**
- Real-time sync (subscriptions, not polling)
- SQL-level queries for aggregation (VP dashboard doesn't pull 200 files)
- Server-side rollups — VP sees pre-computed portfolio health instantly
- Row-level security for proper scoping
- Frontier embedding models (significantly better than local models)
- Proper conflict resolution with server-assigned ordering
- Scales to larger orgs without per-file overhead

**Disadvantages:**
- New infrastructure (hosting, monitoring, cost)
- Requires authentication layer (user accounts, team management)
- Adds a dependency — DailyOS is no longer fully local-first
- More complex sync implementation than file push/pull
- Ongoing operational cost (database hosting, embedding API calls)

### Option C: Hybrid (Google Drive + Remote DB)

Use both:
- **Google Drive** for intelligence file sync (the human-readable artifacts that ICs and managers can browse directly in Drive)
- **Remote DB** for structured queries, rollups, embeddings, and real-time team features
- **Local SQLite** remains the IC's working store

**How it would work:**

```
IC's DailyOS ──enriches──> local SQLite ──pushes──> Google Drive (files)
                                       └──syncs──> Remote DB (structured)
                                                       │
VP's DailyOS <──queries──> Remote DB (rollups, search, embeddings)
                                                       │
Territory Lead ──browses──> Google Drive (read account files directly)
             └──queries──> Remote DB (territory dashboard)
```

**Advantages:**
- Google Drive is browsable (managers can read intelligence.json in a folder without opening DailyOS)
- Remote DB handles the hard problems (aggregation, embeddings, real-time sync)
- Graceful degradation — if remote DB is down, Drive still works for file sharing

**Disadvantages:**
- Two sync targets doubles the write complexity
- Consistency between Drive and DB must be maintained
- More moving parts = more failure modes

---

## Part 4: The Hierarchy Model

Regardless of which sync option, DailyOS needs a model for how intelligence flows through the org hierarchy.

### Org Hierarchy Schema

```
VP of Accounts
  └── Territory Lead: West
  │     └── IC: Alice (15 accounts)
  │     └── IC: Bob (20 accounts)
  └── Territory Lead: East
        └── IC: Carol (18 accounts)
        └── IC: Dave (12 accounts)
```

This requires:
1. **User identity** — who is using this instance of DailyOS
2. **Team membership** — which users belong to which team/territory
3. **Manager relationship** — who reports to whom (for scope filtering)
4. **Account assignment** — which IC owns which accounts

### Intelligence Flow (Upward, One-Way)

```
Layer 1: IC Intelligence (per-account)
  - Source: calendar, email, transcripts, Glean
  - Output: intelligence.json per account
  - Consumer: IC's briefings, meeting prep, reports

Layer 2: Territory Intelligence (cross-account within territory)
  - Source: all IC intelligence.json files in the territory
  - Output: territory summary (health distribution, exceptions, trends)
  - Consumer: territory lead's briefing, portfolio view
  - Synthesis: territory lead's DailyOS runs enrichment over pulled IC data
    (same pattern as parent account portfolio synthesis — already built)

Layer 3: VP Intelligence (cross-territory)
  - Source: all territory summaries + exception accounts
  - Output: org-wide portfolio health, renewal pipeline, risk distribution
  - Consumer: VP briefing, portfolio dashboard, VP Account Review reports
  - Synthesis: VP's DailyOS runs enrichment over territory summaries
    (same pattern, one more level up)
```

**The fractal insight from your research doc is architecturally real:** the same portfolio synthesis that works for parent/child accounts works for territory-lead/IC and VP/territory-lead. The enrichment prompt is parameterized by scope, not fundamentally different.

### What Each Layer Produces

| Layer | Produces | Consumes From Below |
|-------|----------|---------------------|
| IC | `intelligence.json` per account | Calendar, email, transcripts, Glean |
| Territory Lead | Territory summary + exception list | IC intelligence.json files in territory |
| VP | Portfolio health + renewal pipeline + risk matrix | Territory summaries + exception accounts |

### What I489 (VP Account Review) Actually Needs

For a VP to generate a VP Account Review for a specific account:
1. The VP's DailyOS must have access to the IC's intelligence.json for that account
2. The VP's DailyOS must have access to the IC's stakeholder data, signal history, meeting cadence
3. The VP's enrichment prompt must be framed for leadership (strategic, not operational)

This is NOT an aggregation problem — it's an **access** problem. The VP doesn't synthesize across accounts for I489. They need the IC's rich intelligence for ONE account, viewed through a leadership lens.

For I491 (Portfolio Health Summary), it IS an aggregation problem — the VP synthesizes across ALL accounts.

### What This Means for the Issue Sequence

Before any report can work at the VP level:
1. **Shared intelligence layer** must exist (Google Drive, remote DB, or hybrid)
2. **User identity + team model** must exist (who am I, who's on my team)
3. **Account assignment** must exist (which accounts belong to which IC/territory)
4. **Sync mechanism** must exist (IC writes flow to shared layer)
5. **Scope-filtered reads** must work (VP can read any account, territory lead reads their territory)

THEN:
6. I489 (VP Account Review) = IC intelligence + VP-level prompt framing
7. I491 (Portfolio Health Summary) = territory/org-wide aggregation + synthesis

---

## Part 5: Architecture Decision — Option C (Hybrid)

**Decision:** Option C — Google Drive + Remote Database. Made 2026-03-02.

**Rationale:** Even if we started with Google Drive only (Option A), we'd need a remote DB by ~20 users. Building both tracks from the start avoids a migration later. The hybrid preserves what's already built (I426 Drive integration) while adding the structured query/rollup/embedding layer that portfolio views demand.

**Critical insight: The VP is also an IC.** A VP has their own meetings, relationships, intelligence.json, and projects. The portfolio view is additive — it layers team intelligence on top of the VP's personal DailyOS experience. They get the benefit of DailyOS as an IC with the view of a VP. This means every user runs the same app with the same local-first architecture; scope widens with role, but the core experience is identical.

### What Each Layer Does in the Hybrid

| Layer | Purpose | What Lives Here |
|-------|---------|----------------|
| **Local SQLite** | IC's working store — fast, offline, primary | All personal data, intelligence.json, meetings, signals |
| **Google Drive** | Human-browsable shared intelligence layer | intelligence.json files in org folder hierarchy, browsable by managers without opening DailyOS |
| **Remote DB** | Structured queries, rollups, embeddings, real-time sync | Pre-computed portfolio health, territory aggregates, frontier embeddings, user/team model, account assignments |

### What Ships When

**v1.1.0 (single-user, no sync):** Per-account reports work with the current user's intelligence. The VP's own meetings and emails with an account produce intelligence — reports render from that. No multi-user infrastructure needed.

- I489 (VP Account Review) — per-account, current user's data
- I490 (Renewal Readiness) — per-account
- I496 (Stakeholder Map) — per-account
- I497 (Success Plan) — per-account
- I498 (Coaching Patterns) — per-account, cross-account norms from current user's book

**v1.2.0+ (multi-user, shared intelligence):** Portfolio-level features require the hybrid infrastructure.

- I491 (Portfolio Health Summary) — cross-account aggregation from team's intelligence
- I492 (Portfolio Health page) — aggregated portfolio view

### Prerequisite Issues for Multi-User (v1.2.0+)

1. **User identity + team model** — extend user_entity with team/manager relationships, role hierarchy
2. **Google Drive intelligence sync** — extend I426 to push intelligence.json to Shared Drive after enrichment, pull other users' intelligence via Changes API
3. **Remote DB provisioning** — Supabase/Turso/Planetscale setup, schema mirroring intelligence dimensions, row-level security policies
4. **Changeset sync** — Linear-style SyncAction model: local mutations → server, server broadcasts to team. Field-level LWW with source priority.
5. **Account assignment model** — which IC owns which accounts (for scope filtering at both Drive and DB levels)
6. **Scope-filtered portfolio synthesis** — territory lead's DailyOS pulls IC intelligence and runs portfolio enrichment (reuse existing parent account pattern)
7. **Frontier embeddings** — server-side embedding pipeline (Voyage-3 or Cohere Embed v4) for semantic search, signal relevance, gap detection

---

## Part 6: Design Principles (From Portfolio Research Doc)

These non-negotiables from the discovery conversation must hold through any architectural choice:

1. **Zero build** — IC-level sync should be automatic after setup. No manual export/import.
2. **80/20 consumption** — VP reads synthesized intelligence, doesn't configure dashboards.
3. **Editorial calm** — Portfolio views maintain the magazine aesthetic, not Gainsight-style grids.
4. **Narrative over metrics** — Every number tells a story. Health distribution includes "why."
5. **Chief of Staff framing** — "Here are the 3 accounts that need your attention, and here's why."
6. **No maintenance tax** — Intelligence flows upward without anyone manually pushing it.
7. **One-way to start** — Intelligence flows UP. VP context doesn't flow back down to ICs (prevents echo chambers). Bidirectional context happens through conversation, not data pipes.

---

## Sources

### Industry Research
- Gainsight: Multi-tenant PostgreSQL + Redshift architecture, MDA time-series, User.Manager hierarchy
- Vitally: Organization > Account > Users, rollup fields, Hub-based scoping
- Linear: Changeset replay, SyncAction model, IndexedDB local store, server-assigned sequential IDs
- Notion: SQLite/WASM cache, CRDT merge for offline, push-based per-page channels
- Obsidian: File-based sync, conflict copies (cautionary reference)

### Internal
- DailyOS `db/accounts.rs`: Parent/child hierarchy, recursive CTEs, ParentAggregate
- DailyOS `signals/rules.rs`: Hierarchy up/down propagation, 48-hour sibling fusion
- DailyOS `intelligence/prompts.rs`: Portfolio synthesis, build_portfolio_children_context
- DailyOS `google_drive/`: Full Drive API client, Changes API poller, watched sources
- `.docs/research/2026-02-28-portfolio-layer-research.md`: VP briefing discovery conversation
- `.docs/research/2026-02-28-hook-gap-analysis.md`: VP problem analysis, three-layer model

### Google Drive API
- Shared Drives overview, Limited Access Folders, Changes API
- OAuth scopes: drive.file vs. drive
- Rate limits: 12,000 queries/min, 3 writes/sec/user
- ETag-based optimistic locking, revision history
