-- Migration 017: Entity keywords for intelligent meeting-entity resolution (I305)
--
-- Adds keyword columns to projects and accounts tables for auto-extracted
-- terms used by the entity resolver signal cascade.

ALTER TABLE projects ADD COLUMN keywords TEXT;
ALTER TABLE projects ADD COLUMN keywords_extracted_at TEXT;

ALTER TABLE accounts ADD COLUMN keywords TEXT;
ALTER TABLE accounts ADD COLUMN keywords_extracted_at TEXT;
