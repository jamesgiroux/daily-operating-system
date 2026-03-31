# TODOS

## T001 — Remove dead `relationship_type` column from account_stakeholders

**What:** Drop the `relationship_type TEXT DEFAULT 'associated'` column from `account_stakeholders`.

**Why:** The column is never read by any query in the codebase — `db/accounts.rs` never selects it, no service references it, and `data_source` is the actual provenance column. Dead schema creates confusion about what's authoritative.

**Pros:** Cleaner schema, one less ambiguity between `relationship_type` and `data_source`.

**Cons:** Requires a migration. SQLite 3.35+ supports `ALTER TABLE ... DROP COLUMN` — check the minimum SQLite version bundled with Tauri before executing.

**Context:** Discovered during I652 eng review (2026-03-29). The column was created in migration 055_schema_decomposition.sql and has never been populated with anything other than the default. The I652 migration adds real columns to `account_stakeholders` — this cleanup should follow as a separate migration once I652 ships.

**Depends on / blocked by:** I652 (ship the meaningful schema changes first, clean up afterward).
