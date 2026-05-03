-- schema epoch row for migration cutover safety.
--
-- Workers capture the current schema_epoch at job pickup and re-check it
-- at write-back. If the epoch advanced because a migration ran
-- mid-flight), the write is rejected and the work re-queued.
--
-- Note on discarded design option: an earlier plan proposed
-- `ALTER TABLE intel_queue ADD COLUMN schema_epoch ...`, but `intel_queue`
-- is the in-memory `IntelligenceQueue` Rust struct (`src-tauri/src/intel_queue.rs`),
-- not a database table. The DDL is unimplementable as written. This
-- migration ships only the workspace-global `migration_state.schema_epoch`
-- row; workers capture it at dequeue and the WriteFence rechecks it at
-- write-back. See `src-tauri/src/intelligence/write_fence.rs`.
--
-- migration_state is shared with (which writes a 'global_claim_epoch'
-- row). CREATE IF NOT EXISTS keeps the two migrations independent.

CREATE TABLE IF NOT EXISTS migration_state (
    key   TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

INSERT OR IGNORE INTO migration_state (key, value) VALUES ('schema_epoch', 1);
