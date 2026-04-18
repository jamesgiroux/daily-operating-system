-- DOS-15: Glean leading-signal enrichment for Health & Outlook tab.
-- Stores the 7-bucket JSON blob produced by the supplemental Glean prompt
-- defined in .docs/mockups/glean-prompt-health-outlook-signals.md.
-- Nullable — users without Glean silently fall back to NULL.
ALTER TABLE entity_assessment ADD COLUMN health_outlook_signals_json TEXT;
