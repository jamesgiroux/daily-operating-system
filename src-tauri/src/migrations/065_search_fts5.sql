-- Full-text search index (FTS5)
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
    entity_id UNINDEXED,
    entity_type UNINDEXED,
    name,
    secondary_text,
    route UNINDEXED
);
