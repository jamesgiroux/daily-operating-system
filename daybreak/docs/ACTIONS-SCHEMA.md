# Actions State Management

> SQLite-based local action tracking with progressive enhancement path.

---

## Why SQLite

| Option | Pros | Cons |
|--------|------|------|
| CSV | Simple, human-readable | No relations, no transactions, concurrency issues |
| JSON | Flexible schema | Same issues as CSV |
| **SQLite** | ACID, relational, queryable, single file | Slightly more complex |

**Decision:** SQLite. It's local-first (single file), battle-tested, and gives us room to grow.

Location: `~/.daybreak/actions.db` (not in workspace - it's app state, not user content)

---

## Schema

### actions

```sql
CREATE TABLE actions (
    id TEXT PRIMARY KEY,                    -- UUID
    title TEXT NOT NULL,

    -- Classification
    priority TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2',
    status TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending',

    -- Dates
    created_at TEXT NOT NULL,               -- ISO 8601
    due_date TEXT,                          -- ISO 8601 date only
    completed_at TEXT,                      -- ISO 8601

    -- Context
    account_id TEXT,                        -- FK to accounts (nullable for non-CSM)
    project_id TEXT,                        -- PARA project reference
    source_type TEXT,                       -- 'meeting', 'email', 'manual'
    source_id TEXT,                         -- Meeting ID or email thread ID
    source_label TEXT,                      -- "Acme Sync (Feb 3)" for display

    -- Details
    context TEXT,                           -- Additional notes
    waiting_on TEXT,                        -- Who we're waiting on (if status=waiting)

    -- Metadata
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_actions_status ON actions(status);
CREATE INDEX idx_actions_due_date ON actions(due_date);
CREATE INDEX idx_actions_account ON actions(account_id);
```

### accounts (CSM profile only)

```sql
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,                    -- Slug: "acme-corp"
    name TEXT NOT NULL,                     -- "Acme Corp"

    -- Health metrics
    ring INTEGER CHECK(ring BETWEEN 1 AND 4),
    arr REAL,
    health TEXT CHECK(health IN ('green', 'yellow', 'red')),

    -- Dates
    contract_start TEXT,
    contract_end TEXT,

    -- Ownership
    csm TEXT,
    champion TEXT,

    -- Metadata
    tracker_path TEXT,                      -- Path to account folder in PARA
    updated_at TEXT NOT NULL
);
```

### meetings_history

```sql
CREATE TABLE meetings_history (
    id TEXT PRIMARY KEY,                    -- Calendar event ID
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL,             -- 'customer', 'internal', etc.

    -- Time
    start_time TEXT NOT NULL,
    end_time TEXT,

    -- Context
    account_id TEXT,
    attendees TEXT,                         -- JSON array of email addresses

    -- Notes
    notes_path TEXT,                        -- Path to meeting notes in archive
    summary TEXT,                           -- AI-generated summary

    -- Metadata
    created_at TEXT NOT NULL
);

CREATE INDEX idx_meetings_account ON meetings_history(account_id);
CREATE INDEX idx_meetings_start ON meetings_history(start_time);
```

---

## Queries

### Daily actions view (for 80-actions-due.md generation)

```sql
SELECT * FROM actions
WHERE status = 'pending'
  AND (due_date IS NULL OR due_date <= date('now', '+7 days'))
ORDER BY
  CASE WHEN due_date < date('now') THEN 0 ELSE 1 END,  -- Overdue first
  priority,
  due_date;
```

### Account actions (for meeting prep)

```sql
SELECT * FROM actions
WHERE account_id = ?
  AND status IN ('pending', 'waiting')
ORDER BY priority, due_date;
```

### Meeting history (for context)

```sql
SELECT * FROM meetings_history
WHERE account_id = ?
  AND start_time >= date('now', '-30 days')
ORDER BY start_time DESC
LIMIT 3;
```

---

## State Transitions

```
                    ┌─────────────┐
                    │   pending   │
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
           ▼               ▼               ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │  completed  │ │   waiting   │ │  cancelled  │
    └─────────────┘ └──────┬──────┘ └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   pending   │ (when unblocked)
                    └─────────────┘
```

---

## Sync Strategy

Actions can come from multiple sources:

1. **Meeting notes** - Extracted during post-meeting processing
2. **Email** - Extracted during inbox processing
3. **Manual** - User creates directly

All flow into the same SQLite DB. The `source_type` and `source_id` track origin.

### Completion flow

User marks action complete in Daybreak UI:
1. Update SQLite: `status = 'completed'`, `completed_at = now()`
2. If source was a meeting, optionally update the archived meeting notes

---

## Migration from Current System

Current state:
- `master-task-list.md` in `_today/`
- Per-account `actions.md` files

Migration:
1. Parse existing markdown files
2. Insert into SQLite
3. Generate markdown views from SQLite going forward

---

## Rust Implementation

```rust
// In src-tauri/src/db.rs

use rusqlite::{Connection, Result};

pub struct ActionDb {
    conn: Connection,
}

impl ActionDb {
    pub fn open() -> Result<Self> {
        let path = dirs::home_dir()
            .unwrap()
            .join(".daybreak")
            .join("actions.db");

        let conn = Connection::open(path)?;
        conn.execute_batch(include_str!("schema.sql"))?;

        Ok(Self { conn })
    }

    pub fn get_due_actions(&self, days_ahead: i32) -> Result<Vec<Action>> {
        // ...
    }

    pub fn get_account_actions(&self, account_id: &str) -> Result<Vec<Action>> {
        // ...
    }

    pub fn complete_action(&self, id: &str) -> Result<()> {
        // ...
    }
}
```

---

## Progressive Enhancement

**Phase 1 (MVP):**
- Basic actions table
- Import from markdown
- Query for daily view

**Phase 2:**
- Accounts table (CSM profile)
- Meeting history for context
- Completion syncs back to notes

**Phase 3:**
- Recurring actions
- Action templates
- Smart suggestions

---

*Draft: 2026-02-05*
