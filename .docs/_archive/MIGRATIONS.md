# Schema Migrations

Schema changes tracked by version. The migration runner (ADR-0071) shipped in v0.7.3.

**How it works:** Numbered SQL migrations live in `src-tauri/src/migrations/`. Each is embedded at compile time via `include_str!` and tracked in the `schema_version` table. On startup, `migrations::run_migrations()` applies any pending migrations. Pre-migration backup is created automatically. See `src-tauri/src/migrations.rs` for implementation.

**Convention:** To add a new migration, create `migrations/NNN_description.sql` and add it to the `MIGRATIONS` const array in `migrations.rs`. Version numbers are sequential integers.

---

## 0.7.3 (Current)

### Migration Framework (ADR-0071)

**Migration 001 — Baseline Schema**
File: `src-tauri/src/migrations/001_baseline.sql`

Consolidates the original `schema.sql` + all 22 inline ALTER TABLE migrations from `db.rs` into one complete schema. For new databases, this creates all tables. For existing databases (pre-0.7.3), the bootstrap function marks v1 as applied — the SQL never runs.

Tables: `actions`, `accounts`, `projects`, `meetings_history`, `processing_log`, `captures`, `entities`, `meeting_prep_state`, `people`, `meeting_attendees`, `entity_people`, `meeting_entities`, `content_index`, `entity_intelligence`, `account_events`, `schema_version`.

**Bootstrap behavior:** If `actions` table exists but `schema_version` doesn't → marks v1 as applied (zero SQL executed). Existing data untouched.

**Forward-compat guard:** If `schema_version` > max known migration → returns error telling user to update DailyOS.

**Pre-migration backup:** Hot copy to `<db_path>.pre-migration.bak` via `rusqlite::backup::Backup` before applying any pending migrations.

### Other Changes

- Version aligned to 0.7.3 across `tauri.conf.json`, `Cargo.toml`, `package.json`
- Auto-updater plugin added (`tauri-plugin-updater`, `tauri-plugin-process`)
- macOS code signing + notarization in CI
- `schema.sql` deleted (absorbed into migration 001)
- Inline migrations in `db.rs` removed (~160 lines)

### Config Format Changes
None.

### Archive Format Changes
None.

### Breaking Changes
None. Existing databases are detected and bootstrapped automatically.

---

## 0.7.0–0.7.2 (Pre-Migration Framework)

Initial alpha releases. Schema established but not versioned. Inline ALTER TABLE migrations ran on every startup in `db.rs`. All inline migrations are now consolidated into migration 001.

### SQLite Schema (as of 0.7.2)
- `accounts`, `projects`, `people`, `actions`, `meetings_history`, `meeting_entities` tables
- `entity_intelligence`, `content_index`, `captures`, `processing_log` tables
- `entities`, `meeting_prep_state`, `meeting_attendees`, `entity_people` tables
- `account_events` table
- No `schema_version` table (added in 0.7.3)

### Config Format
`~/.dailyos/config.json`:
```json
{
  "workspace_path": "/Users/.../Documents/DailyOS",
  "profile": "customer-success",
  "user_name": "...",
  "user_company": "...",
  "user_title": "...",
  "user_focus": "...",
  "user_domain": "company.com",
  "developer_mode": false
}
```

### Archive Format
- `_today/data/` — JSON files (schedule.json, actions.json, emails.json, etc.)
- `_today/Today.md` — markdown briefing
- Entity directories: `dashboard.json`, `intelligence.json`, `dashboard.md`

---

## Adding Future Migrations

1. Create `src-tauri/src/migrations/NNN_description.sql`
2. Add to `MIGRATIONS` array in `migrations.rs`:
   ```rust
   Migration { version: N, sql: include_str!("migrations/NNN_description.sql") },
   ```
3. Document the migration in this file under a new version heading
4. Test: fresh DB (all migrations run) + existing DB (only new migration runs)
