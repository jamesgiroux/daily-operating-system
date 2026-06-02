CREATE TABLE IF NOT EXISTS entity_archive_folders (
    operation_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('account', 'project', 'person')),
    entity_id TEXT NOT NULL,
    original_relative_path TEXT NOT NULL,
    archived_relative_path TEXT,
    folder_state TEXT NOT NULL CHECK (
        folder_state IN (
            'archive_pending',
            'archived',
            'archive_failed',
            'missing_source',
            'restore_pending',
            'restored',
            'restore_failed'
        )
    ),
    archived_at TEXT,
    restored_at TEXT,
    updated_at TEXT NOT NULL,
    last_error_code TEXT,
    PRIMARY KEY (operation_id, entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_entity_archive_folders_entity_state
    ON entity_archive_folders(entity_type, entity_id, folder_state, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_entity_archive_folders_state
    ON entity_archive_folders(folder_state, updated_at DESC);
