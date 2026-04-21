-- DOS-258 Lane C: pending_thread_inheritance queue.
--
-- When a child email arrives before its parent (Gmail API out-of-order delivery),
-- P2 (thread inheritance) cannot resolve the parent's primary entity yet.
-- This table records the child owner so the next evaluation pass for the thread
-- can flush the queue and retroactively apply P2 inheritance.
--
-- Flush trigger: after evaluate() sets a primary for any email whose thread_id
-- has entries in this table, re-evaluate each waiting child.
-- See DOS-258 plan-eng-review comment, point 7 ("~30 lines").

CREATE TABLE IF NOT EXISTS pending_thread_inheritance (
    thread_id        TEXT NOT NULL,
    child_owner_type TEXT NOT NULL DEFAULT 'email',
    child_owner_id   TEXT NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (thread_id, child_owner_id)
);

CREATE INDEX IF NOT EXISTS idx_pending_thread_inheritance_thread
    ON pending_thread_inheritance (thread_id);
