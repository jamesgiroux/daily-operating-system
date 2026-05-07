-- Watch action triage persistence.
--
-- Action-level snoozes keep Watch parked rows durable across refreshes.
-- Meeting links attach an action to a meeting without rewriting the action's
-- original source fields.

CREATE TABLE IF NOT EXISTS action_snoozes (
    action_id      TEXT PRIMARY KEY REFERENCES actions(id) ON DELETE CASCADE,
    snoozed_until TEXT NOT NULL,
    reason        TEXT NOT NULL,
    source        TEXT NOT NULL
                  CHECK(source IN ('unknown', 'actions_page', 'daily_briefing', 'meeting_detail')),
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    cleared_at    TEXT
);

CREATE INDEX IF NOT EXISTS idx_action_snoozes_until
    ON action_snoozes(snoozed_until)
    WHERE cleared_at IS NULL;

CREATE TABLE IF NOT EXISTS action_meeting_links (
    action_id  TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    source     TEXT NOT NULL
               CHECK(source IN ('unknown', 'actions_page', 'daily_briefing', 'meeting_detail')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (action_id, meeting_id)
);

CREATE INDEX IF NOT EXISTS idx_action_meeting_links_meeting
    ON action_meeting_links(meeting_id);
