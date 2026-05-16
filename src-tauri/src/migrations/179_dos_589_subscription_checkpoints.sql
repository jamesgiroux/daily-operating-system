-- W4-B-signals: durable subscription registry for the version-event
-- dispatcher.
--
-- Rows survive process restart so a SurfaceClient (or User/Agent transport)
-- that reconnects with `from_cursor` can resume ordered, scope-filtered replay
-- from its last-acked event. Live-handle state (outbound queue, IPC channel)
-- is NOT persisted; only the durable contract that lets the dispatcher
-- recompute scope, locate the cursor in `version_events`, and continue.
--
-- Identity model (packet §10 V2):
--   subscription_id  - server-allocated UUIDv4 per Subscribe call (durable key).
--   actor_kind +     - the actor class and a non-PII stable instance id
--   actor_instance     (SurfaceClientId for SurfaceClient; per-session UUID for
--                      User/Agent transports).
--   scopes_digest    - SHA256 hex of the sorted scope strings joined by '\n'.
--   subject_filter   - SHA256 hex of canonical JSON for the subject filter.
--     _digest          The raw filter is reconstructed by callers from claim/
--                      composition ids on Subscribe; durable row stores digest
--                      only so opaque IDs and no customer names live here.
--
-- Cursor envelope (packet §3 V3): the dispatcher signs every client-visible
-- cursor with a per-subscription HMAC key. The key lives alongside the
-- checkpoint so reconnect can authenticate the envelope before resolving the
-- UUIDv4 row identity in `version_events`. Storing the key in SQLite is OK
-- because the DB is the trust boundary for substrate state; the envelope's
-- only job is to prevent a foreign cursor from probing event_seq lifetime.

CREATE TABLE IF NOT EXISTS subscription_checkpoints (
    subscription_id TEXT PRIMARY KEY
        CHECK (length(subscription_id) = 36 AND subscription_id GLOB '*-*-*-*-*'),
    actor_kind TEXT NOT NULL
        CHECK (actor_kind IN ('user', 'agent', 'admin', 'system', 'surface_client')),
    actor_instance TEXT NOT NULL,
    scopes_digest TEXT NOT NULL CHECK (length(scopes_digest) = 64),
    subject_filter_digest TEXT NOT NULL CHECK (length(subject_filter_digest) = 64),
    subscriber_local_key BLOB NOT NULL CHECK (length(subscriber_local_key) = 32),
    last_acked_event_seq INTEGER NOT NULL DEFAULT 0
        CHECK (last_acked_event_seq >= 0),
    last_acked_cursor_uuid TEXT
        CHECK (last_acked_cursor_uuid IS NULL
               OR (length(last_acked_cursor_uuid) = 36
                   AND last_acked_cursor_uuid GLOB '*-*-*-*-*')),
    last_scanned_event_seq INTEGER NOT NULL DEFAULT 0
        CHECK (last_scanned_event_seq >= 0),
    replay_required INTEGER NOT NULL DEFAULT 0 CHECK (replay_required IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_subscription_checkpoints_reconnect
    ON subscription_checkpoints (
        actor_kind,
        actor_instance,
        scopes_digest,
        subject_filter_digest
    );
