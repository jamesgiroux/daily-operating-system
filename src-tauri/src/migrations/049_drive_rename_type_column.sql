-- Migration 049: Rename drive_watched_sources.type → file_type
--
-- An early version of migration 048 created the column as `type`.
-- The Rust code expects `file_type`. This renames it for existing databases.
-- On fresh installs where the column is already `file_type`, this will error
-- with "no such column: type" — the migration runner tolerates this gracefully.

ALTER TABLE drive_watched_sources RENAME COLUMN type TO file_type;
