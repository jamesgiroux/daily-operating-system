-- Intelligence lifecycle columns for meetings_history (ADR-0081)
ALTER TABLE meetings_history ADD COLUMN intelligence_state TEXT NOT NULL DEFAULT 'detected';
ALTER TABLE meetings_history ADD COLUMN intelligence_quality TEXT NOT NULL DEFAULT 'sparse';
ALTER TABLE meetings_history ADD COLUMN last_enriched_at TEXT;
ALTER TABLE meetings_history ADD COLUMN signal_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE meetings_history ADD COLUMN has_new_signals INTEGER NOT NULL DEFAULT 0;
ALTER TABLE meetings_history ADD COLUMN last_viewed_at TEXT;
