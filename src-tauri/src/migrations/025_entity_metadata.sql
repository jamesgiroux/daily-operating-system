-- I311: Entity metadata storage for preset-driven custom fields.
-- CS preset maps arr/health/nps to existing columns via column_mapping.
-- Other presets store custom fields in this JSON metadata column.
ALTER TABLE accounts ADD COLUMN metadata TEXT DEFAULT '{}';
ALTER TABLE projects ADD COLUMN metadata TEXT DEFAULT '{}';
