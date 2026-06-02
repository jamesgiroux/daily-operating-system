CREATE TABLE IF NOT EXISTS composition_layout_overlays (
  entity_type TEXT NOT NULL,
  surface_key TEXT NOT NULL,
  overlay_schema_version INTEGER NOT NULL DEFAULT 1,
  layout_revision INTEGER NOT NULL DEFAULT 1,
  overlay_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (entity_type, surface_key),
  CHECK (entity_type IN ('account', 'project', 'person')),
  CHECK (surface_key IN ('entity_page')),
  CHECK (overlay_schema_version = 1),
  CHECK (layout_revision >= 1),
  CHECK (length(overlay_json) <= 32768)
);
