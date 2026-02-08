# ADR-0048: Three-tier data model — filesystem, SQLite, app memory

**Date:** 2026-02-07
**Status:** Accepted
**Supersedes:** [ADR-0018](0018-hybrid-storage-markdown-sqlite.md) (Hybrid storage: Markdown + SQLite)
**Builds on:** [ADR-0004](0004-hybrid-json-markdown-architecture.md) (JSON + Markdown), [ADR-0031](0031-actions-source-of-truth.md) (Actions: SQLite as working store), [ADR-0047](0047-entity-dashboard-architecture.md) (Entity dashboard architecture)

## Context

ADR-0018 established a two-tier model: "Markdown is the source of truth. SQLite is a disposable cache — it can be rebuilt from files." This was a useful simplification early on, ensuring we treated user-owned files as the real data and didn't lock state inside a database.

That model no longer describes reality. Examining what SQLite actually holds today:

| Data | SQLite table | Can it be rebuilt from markdown? |
|------|-------------|--------------------------------|
| Action completion state | `actions.completed_at`, `priority` | Not reliably — lazy writeback means recent state may not be in files |
| Capture records (wins/risks/decisions) | `captures` | Only by re-running AI transcript extraction |
| Meeting history | `meetings_history` | Partially — archive has some, but accumulated fields are SQLite-only |
| Account structured fields | `accounts` (ARR, health, ring) | No — these come from user input or integrations |
| Entity metadata | `entities.metadata` | No |
| Processing history | `processing_log` | No |
| Stakeholder signals | Computed from `meetings_history` + `accounts` | No — requires recomputation |
| People graph (planned) | `people`, `entity_people` | No — accumulated from meeting attendees + user input |

SQLite is not disposable. Deleting it would lose significant operational state that ranges from difficult to impossible to reconstruct. Every sprint has added more state to SQLite — action tracking, captures, intelligence signals, entity fields — each time bending the "disposable cache" fiction.

ADR-0031 (actions) already broke from 0018 by declaring SQLite the "working store" for actions with lazy markdown writeback. ADR-0047 (entity dashboards) established a three-way sync where JSON is canonical and SQLite mirrors structured fields for queryability. The pattern has evolved; the principle statement hasn't kept up.

Meanwhile, the core insight behind ADR-0018 remains correct: **users own their data as portable files.** The workspace directory must be meaningful without the app. Markdown and JSON files are what make the archive valuable to Claude Desktop, Claude Code, and any future AI tool. This principle doesn't require the fiction that SQLite is disposable.

## Decision

### Three-tier data model

| Tier | Role | Durability | What lives here |
|------|------|------------|----------------|
| **Filesystem** (markdown + JSON) | The durable, portable layer. What survives app deletion. What the ecosystem consumes. What the user owns. | Permanent — user's files | Archive, dashboard.md, dashboard.json, transcripts, user notes, config.json, briefing data |
| **SQLite** | The working store. Fast queries, operational state, accumulated intelligence. Not disposable, but rebuildable from filesystem with defined effort. | Durable — backed up, protected | Actions, captures, accounts, entities, meeting history, people graph, processing log |
| **App memory** | Truly ephemeral in-process state. Lost on restart, no recovery needed. | Ephemeral | Calendar event cache, workflow status flags, transcript processing state, Google auth tokens |

### Principles

**1. The filesystem is the durable layer.**

Everything important must eventually have a filesystem representation — as markdown, JSON, or both. If the app and SQLite both disappear, the workspace directory retains the user's operational intelligence. This is non-negotiable (Principle 5: Local-First, Always).

**2. SQLite is the working store, not a cache.**

SQLite holds operational state that the app reads and writes at high frequency: action completion, capture records, entity fields, meeting history, relationship signals. It is not disposable. Deleting it loses real work. The app should protect it accordingly (backups, graceful corruption handling).

**3. Filesystem writeback is eventual, not immediate.**

Data flows from SQLite to the filesystem, but not on every write. Writeback happens at natural synchronization points:

| Writeback path | When it runs | What it writes |
|----------------|-------------|----------------|
| Action completion → markdown | Post-enrichment hooks | `[x]` markers in source files |
| Captures → impact markdown | Archive workflow (end of day) | Weekly impact capture file |
| Account fields → dashboard.json | Dashboard regeneration trigger | Structured fields section |
| Dashboard.json → dashboard.md | Any dashboard data change | Full rendered artifact |
| Meeting outcomes → archive | Archive workflow | Day summary with outcomes |

**4. Rebuild from filesystem is a defined operation, not an implicit guarantee.**

If SQLite is lost, a `rebuild_database` command reconstructs what it can from the filesystem:

| What can be rebuilt | From what |
|--------------------|-----------|
| Account/entity records | `dashboard.json` files in workspace |
| Action items (incomplete) | Archive markdown files, briefing JSON |
| Meeting history (partial) | Archive `day-summary.json` files |
| Basic captures | Archive impact files |

| What cannot be rebuilt without re-running AI |
|---------------------------------------------|
| AI-extracted captures from transcripts |
| Computed stakeholder signals |
| Processing history |
| Enrichment metadata |

This is an honest accounting. The rebuild operation is a safety net, not a promise of perfect reconstruction. Users who want full protection use the backup mechanism.

**5. External tools interact with the filesystem layer.**

Claude Desktop, Claude Code, ChatGPT, and any other tool reads and writes the filesystem — markdown and JSON files. They never touch SQLite directly. The app bridges the filesystem and SQLite layers, syncing changes in both directions per ADR-0047's sync model.

### What changes from ADR-0018

| ADR-0018 said | ADR-0048 says |
|---------------|---------------|
| "SQLite is a disposable cache" | SQLite is a working store — not disposable, but rebuildable |
| "Markdown is the source of truth" | Filesystem (markdown + JSON) is the durable layer; SQLite is the operational layer |
| "SQLite can be rebuilt from files" | Rebuild is a defined command with known limitations, not an implicit guarantee |
| Implicit: all state derives from markdown | Explicit: SQLite holds state that may not yet be in files; writeback is eventual |

### What doesn't change

- Users own their data as portable files (Principle 5)
- The workspace directory is meaningful without the app
- Markdown is the human-readable, AI-consumable format
- JSON is the machine-readable structured format
- The archive is the product, not the app

## Consequences

**Easier:**
- SQLite can hold more state without guilt — relationship graphs, computed intelligence, enrichment metadata, interaction patterns
- ADR-0047's three-way sync (JSON ↔ SQLite ↔ markdown) is the natural model, not an exception
- Computed intelligence (portfolio signals, relationship temperature, delegation staleness) can be cached in SQLite with proper invalidation
- No need to contort every piece of state into a markdown representation for the "disposable cache" fiction
- ADR-0031's "SQLite as working store for actions" pattern extends cleanly to all operational data

**Harder:**
- SQLite needs a backup strategy (periodic backup, corruption recovery)
- Need a `rebuild_database` command for disaster recovery
- Need to audit existing SQLite-only state and ensure important data has a filesystem writeback path
- More discipline required: "does this data need to reach the filesystem eventually?" becomes a design question for every new feature
- Code comments and docs that reference "disposable cache" need updating

**Trade-offs:**
- Chose honesty over simplicity — the "disposable cache" model was simpler to reason about but no longer true. The three-tier model is more complex but accurately describes the system.
- Chose eventual writeback over synchronous writeback — writes to SQLite don't block on filesystem writes. This means there's a window where SQLite has state that files don't. The backup mechanism covers this window.
- Chose defined rebuild over guaranteed rebuild — explicitly documenting what can and can't be rebuilt is more honest than claiming full reconstruction. Users who need stronger guarantees use backups.
- Accepted that the filesystem layer is no longer "the single source of truth" — it's the durable layer, and SQLite is the operational layer. Truth is distributed across both, with the filesystem being the survivor of last resort.
