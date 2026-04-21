-- DOS-XXX: Work-tab foundation — commitments as Actions, focus pins, nudge memory.
--
-- Four schema deltas land together since they're co-dependent on the Work-tab
-- rebuild:
--   1. `actions.action_kind` distinguishes AI-inferred commitments from generic
--      tasks/focus pins without creating parallel tables. Existing `actions`
--      rows default to 'task' — no behavior change for any surface that
--      doesn't explicitly filter on kind.
--   2. `ai_commitment_bridge` is the identity + tombstone table for AI-inferred
--      commitments. LLM emits a stable `commitment_id` per open_commitments
--      entry; the bridge maps commitment_id → action_id and remembers
--      tombstones so completed/dismissed commitments don't resurrect on
--      re-enrichment when the LLM rephrases them.
--   3. `account_focus_pins` overlays the AI focus-prioritization engine with
--      user pinning (action_id + rank). Focus items are derived views over
--      existing Actions plus this pin overlay — no separate narrative entity.
--   4. `nudge_dismissals` persists per-(entity, nudge_key) dismissals so
--      dismissed nudges don't re-synthesize on every render.

-- 1. action_kind on actions
ALTER TABLE actions ADD COLUMN action_kind TEXT NOT NULL DEFAULT 'task'
    CHECK(action_kind IN ('task', 'commitment'));
CREATE INDEX IF NOT EXISTS idx_actions_kind ON actions(action_kind);

-- 2. ai_commitment_bridge
CREATE TABLE IF NOT EXISTS ai_commitment_bridge (
    commitment_id TEXT PRIMARY KEY,
    entity_type   TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    action_id     TEXT REFERENCES actions(id) ON DELETE SET NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at  TEXT NOT NULL,
    tombstoned    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_commitment_bridge_entity
    ON ai_commitment_bridge(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_ai_commitment_bridge_action
    ON ai_commitment_bridge(action_id);

-- 3. account_focus_pins — user overlay on top of AI focus prioritization
CREATE TABLE IF NOT EXISTS account_focus_pins (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    action_id  TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    rank       INTEGER NOT NULL,
    pinned_at  TEXT NOT NULL,
    PRIMARY KEY (account_id, action_id)
);
CREATE INDEX IF NOT EXISTS idx_account_focus_pins_rank
    ON account_focus_pins(account_id, rank);

-- 4. nudge_dismissals
CREATE TABLE IF NOT EXISTS nudge_dismissals (
    entity_type  TEXT NOT NULL,
    entity_id    TEXT NOT NULL,
    nudge_key    TEXT NOT NULL,
    dismissed_at TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id, nudge_key)
);
