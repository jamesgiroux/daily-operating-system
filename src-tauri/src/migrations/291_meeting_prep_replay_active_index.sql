CREATE INDEX IF NOT EXISTS idx_meeting_prep_correction_journal_active_scan
    ON meeting_prep_correction_journal(lifecycle_state, meeting_stable_key, field_path, updated_at);
