-- Health tab action wiring: persist triage snooze and
-- resolution state per card.
--
-- Triage cards on the Health tab carry Snooze and "Confirm resolved" actions.
-- Before this table, both actions were no-ops — card dismissals did not
-- survive refresh. We persist per (entity, triage_key) so rendering-time
-- filtering can hide cards that are snoozed (snoozed_until > now) or resolved
-- (resolved_at IS NOT NULL).
--
-- `triage_key` is the stable card id the frontend emits
-- (e.g. `glean-usage-trend`, `local-risk-3`). Card ids are stable across
-- renders for a given enrichment cycle; a re-enrichment that emits a new
-- card gets a new key, which is the correct behavior — a brand-new signal
-- is not the same card the user snoozed yesterday.

CREATE TABLE IF NOT EXISTS triage_snoozes (
    entity_type   TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    triage_key    TEXT NOT NULL,
    snoozed_until TEXT,
    resolved_at   TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_type, entity_id, triage_key)
);

CREATE INDEX IF NOT EXISTS idx_triage_snoozes_entity
    ON triage_snoozes(entity_type, entity_id);
