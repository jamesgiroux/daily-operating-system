-- DOS-258 Lane A: entity_linking_evaluations — append-only provenance audit.
--
-- One row per evaluate() call. Records: which owner was evaluated, which
-- trigger fired, which rule matched (rule_id), what entity was chosen
-- (entity_id / entity_type / role), the full graph snapshot version used,
-- and a JSON blob containing the complete candidate set + per-phase outputs
-- + rejected reasons.
--
-- This makes every link debuggable: acceptance criterion #17 requires that
-- any specific link can be explained in < 5 seconds. The evidence_json blob
-- carries matched_text, rejected_candidates, parent_email_id, rule inputs,
-- and per-phase decisions — see DOS-258 Codex finding 14 + eng-review note
-- on promoted fields.
--
-- Nightly cron trims rows older than 30 days:
--   DELETE FROM entity_linking_evaluations WHERE created_at < datetime('now', '-30 days');
-- This is set up in Lane C; the table is created here so migrations run clean.

CREATE TABLE IF NOT EXISTS entity_linking_evaluations (
    id            INTEGER PRIMARY KEY,
    owner_type    TEXT NOT NULL,
    owner_id      TEXT NOT NULL,
    link_trigger  TEXT NOT NULL,  -- CalendarPoll | EmailFetch | CalendarUserEdit | ...
    rule_id       TEXT,           -- matched rule id, e.g. 'P4a'; NULL if no primary chosen
    entity_id     TEXT,           -- chosen primary entity; NULL if primary=none
    entity_type   TEXT,
    role          TEXT,
    graph_version INTEGER NOT NULL,
    evidence_json TEXT NOT NULL,  -- full candidate set, phase outputs, rejected reasons
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Owner + time supports the "explain this link" debug query (AC #17).
CREATE INDEX IF NOT EXISTS idx_linking_evals_owner
    ON entity_linking_evaluations (owner_type, owner_id, created_at);

-- Timestamp-only index for the nightly trim cron.
CREATE INDEX IF NOT EXISTS idx_linking_evals_created
    ON entity_linking_evaluations (created_at);
