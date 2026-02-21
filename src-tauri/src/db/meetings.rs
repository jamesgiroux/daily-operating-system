use super::*;

impl ActionDb {
    // =========================================================================
    // Meetings
    // =========================================================================

    /// Query recent meetings for an account within `lookback_days`, limited to `limit` results.
    pub fn get_meeting_history(
        &self,
        account_id: &str,
        lookback_days: i32,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1
               AND m.start_time >= date('now', ?2 || ' days')
             ORDER BY m.start_time DESC
             LIMIT ?3",
        )?;

        let days_param = format!("-{lookback_days}");
        let rows = stmt.query_map(params![account_id, days_param, limit], |row| {
            Ok(DbMeeting {
                id: row.get(0)?,
                title: row.get(1)?,
                meeting_type: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                attendees: row.get(5)?,
                notes_path: row.get(6)?,
                summary: row.get(7)?,
                created_at: row.get(8)?,
                calendar_event_id: row.get(9)?,
                description: None,
                prep_context_json: None,
                user_agenda_json: None,
                user_notes: None,
                prep_frozen_json: None,
                prep_frozen_at: None,
                prep_snapshot_path: None,
                prep_snapshot_hash: None,
                transcript_path: None,
                transcript_processed_at: None,
                intelligence_state: None,
                intelligence_quality: None,
                last_enriched_at: None,
                signal_count: None,
                has_new_signals: None,
                last_viewed_at: None,
            })
        })?;

        let mut meetings = Vec::new();
        for row in rows {
            meetings.push(row?);
        }
        Ok(meetings)
    }

    /// Look up a single meeting by its ID (includes prep_context_json).
    pub fn get_meeting_by_id(&self, id: &str) -> Result<Option<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    attendees, notes_path, summary, created_at,
                    calendar_event_id, description, prep_context_json,
                    user_agenda_json, user_notes, prep_frozen_json, prep_frozen_at,
                    prep_snapshot_path, prep_snapshot_hash, transcript_path, transcript_processed_at,
                    intelligence_state, intelligence_quality, last_enriched_at,
                    signal_count, has_new_signals, last_viewed_at
             FROM meetings_history
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(DbMeeting {
                id: row.get(0)?,
                title: row.get(1)?,
                meeting_type: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                attendees: row.get(5)?,
                notes_path: row.get(6)?,
                summary: row.get(7)?,
                created_at: row.get(8)?,
                calendar_event_id: row.get(9)?,
                description: row.get(10)?,
                prep_context_json: row.get(11)?,
                user_agenda_json: row.get(12)?,
                user_notes: row.get(13)?,
                prep_frozen_json: row.get(14)?,
                prep_frozen_at: row.get(15)?,
                prep_snapshot_path: row.get(16)?,
                prep_snapshot_hash: row.get(17)?,
                transcript_path: row.get(18)?,
                transcript_processed_at: row.get(19)?,
                intelligence_state: row.get(20)?,
                intelligence_quality: row.get(21)?,
                last_enriched_at: row.get(22)?,
                signal_count: row.get(23)?,
                has_new_signals: row.get(24)?,
                last_viewed_at: row.get(25)?,
            })
        })?;

        match rows.next() {
            Some(Ok(meeting)) => Ok(Some(meeting)),
            Some(Err(e)) => Err(DbError::Sqlite(e)),
            None => Ok(None),
        }
    }

    /// Look up a single meeting row with all permanence/transcript columns.
    pub fn get_meeting_intelligence_row(
        &self,
        meeting_id: &str,
    ) -> Result<Option<DbMeeting>, DbError> {
        self.get_meeting_by_id(meeting_id)
    }

    /// Return all meetings that have persisted prep context JSON.
    pub fn list_meeting_prep_contexts(&self) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, prep_context_json
             FROM meetings_history
             WHERE prep_context_json IS NOT NULL
               AND trim(prep_context_json) != ''",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let prep_context_json: String = row.get(1)?;
            Ok((id, prep_context_json))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Update prep context JSON for a single meeting.
    pub fn update_meeting_prep_context(
        &self,
        meeting_id: &str,
        prep_context_json: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history
             SET prep_context_json = ?1
             WHERE id = ?2",
            params![prep_context_json, meeting_id],
        )?;
        Ok(())
    }

    /// Persist user-authored agenda/notes in the meeting row.
    pub fn update_meeting_user_layer(
        &self,
        meeting_id: &str,
        user_agenda_json: Option<&str>,
        user_notes: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history
             SET user_agenda_json = ?1,
                 user_notes = ?2
             WHERE id = ?3",
            params![user_agenda_json, user_notes, meeting_id],
        )?;
        Ok(())
    }

    /// Freeze immutable prep snapshot metadata once. No-op when already frozen.
    pub fn freeze_meeting_prep_snapshot(
        &self,
        meeting_id: &str,
        frozen_json: &str,
        frozen_at: &str,
        snapshot_path: &str,
        snapshot_hash: &str,
    ) -> Result<bool, DbError> {
        let affected = self.conn.execute(
            "UPDATE meetings_history
             SET prep_frozen_json = ?1,
                 prep_frozen_at = ?2,
                 prep_snapshot_path = ?3,
                 prep_snapshot_hash = ?4
             WHERE id = ?5
               AND prep_frozen_at IS NULL",
            params![
                frozen_json,
                frozen_at,
                snapshot_path,
                snapshot_hash,
                meeting_id
            ],
        )?;
        Ok(affected > 0)
    }

    /// Persist transcript metadata directly on the meeting row.
    pub fn update_meeting_transcript_metadata(
        &self,
        meeting_id: &str,
        transcript_path: &str,
        processed_at: &str,
        summary_opt: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history
             SET transcript_path = ?1,
                 transcript_processed_at = ?2,
                 summary = COALESCE(?3, summary)
             WHERE id = ?4",
            params![transcript_path, processed_at, summary_opt, meeting_id],
        )?;
        Ok(())
    }

    /// Update intelligence state for a meeting (ADR-0081).
    pub fn update_intelligence_state(
        &self,
        meeting_id: &str,
        state: &str,
        quality: Option<&str>,
        signal_count: Option<i32>,
    ) -> Result<(), DbError> {
        let mut sql = "UPDATE meetings_history SET intelligence_state = ?1".to_string();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(state.to_string())];
        let mut idx = 2;

        if let Some(q) = quality {
            sql.push_str(&format!(", intelligence_quality = ?{idx}"));
            params.push(Box::new(q.to_string()));
            idx += 1;
        }
        if let Some(sc) = signal_count {
            sql.push_str(&format!(", signal_count = ?{idx}"));
            params.push(Box::new(sc));
            idx += 1;
        }

        sql.push_str(&format!(", last_enriched_at = ?{idx}"));
        params.push(Box::new(chrono::Utc::now().to_rfc3339()));
        idx += 1;
        sql.push_str(&format!(" WHERE id = ?{idx}"));
        params.push(Box::new(meeting_id.to_string()));

        self.conn.execute(
            &sql,
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
        )?;
        Ok(())
    }

    /// Mark meeting as having new signals (ADR-0081).
    pub fn mark_meeting_new_signals(&self, meeting_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history SET has_new_signals = 1 WHERE id = ?1",
            params![meeting_id],
        )?;
        Ok(())
    }

    /// Clear new signals flag (when user views the meeting).
    pub fn clear_meeting_new_signals(&self, meeting_id: &str) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE meetings_history SET has_new_signals = 0, last_viewed_at = ?2 WHERE id = ?1",
            params![meeting_id, now],
        )?;
        Ok(())
    }

    // =========================================================================
    // Quill Sync State
    // =========================================================================

    /// Insert a new sync state row for a meeting with a specific source.
    pub fn insert_quill_sync_state_with_source(
        &self,
        meeting_id: &str,
        source: &str,
    ) -> Result<String, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let next_attempt = (Utc::now() + chrono::Duration::minutes(2)).format("%Y-%m-%d %H:%M:%S").to_string();
        self.conn.execute(
            "INSERT OR IGNORE INTO quill_sync_state (id, meeting_id, source, next_attempt_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, meeting_id, source, next_attempt, now, now],
        )?;
        Ok(id)
    }

    /// Insert a new Quill sync state row for a meeting (state=pending).
    pub fn insert_quill_sync_state(&self, meeting_id: &str) -> Result<String, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let next_attempt = (Utc::now() + chrono::Duration::minutes(2)).format("%Y-%m-%d %H:%M:%S").to_string();
        self.conn.execute(
            "INSERT OR IGNORE INTO quill_sync_state (id, meeting_id, next_attempt_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, meeting_id, next_attempt, now, now],
        )?;
        Ok(id)
    }

    /// Get Quill sync state for a specific meeting.
    pub fn get_quill_sync_state(
        &self,
        meeting_id: &str,
    ) -> Result<Option<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state WHERE meeting_id = ?1",
        )?;
        let mut rows = stmt.query_map(params![meeting_id], map_sync_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get sync state for a specific meeting and source.
    pub fn get_quill_sync_state_by_source(
        &self,
        meeting_id: &str,
        source: &str,
    ) -> Result<Option<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state WHERE meeting_id = ?1 AND source = ?2",
        )?;
        let mut rows = stmt.query_map(params![meeting_id, source], map_sync_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all pending Quill syncs ready for processing (source='quill' only).
    pub fn get_pending_quill_syncs(&self) -> Result<Vec<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state
             WHERE state IN ('pending', 'polling', 'fetching') AND next_attempt_at <= datetime('now')
               AND source = 'quill'",
        )?;
        let rows = stmt.query_map([], map_sync_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Update Quill sync state fields.
    pub fn update_quill_sync_state(
        &self,
        id: &str,
        state: &str,
        quill_meeting_id: Option<&str>,
        match_confidence: Option<f64>,
        error_message: Option<&str>,
        transcript_path: Option<&str>,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let completed_at: Option<String> = if state == "completed" {
            Some(now.clone())
        } else {
            None
        };
        self.conn.execute(
            "UPDATE quill_sync_state
             SET state = ?1,
                 quill_meeting_id = COALESCE(?2, quill_meeting_id),
                 match_confidence = COALESCE(?3, match_confidence),
                 error_message = ?4,
                 transcript_path = COALESCE(?5, transcript_path),
                 completed_at = COALESCE(?6, completed_at),
                 updated_at = ?7
             WHERE id = ?8",
            params![
                state,
                quill_meeting_id,
                match_confidence,
                error_message,
                transcript_path,
                completed_at,
                now,
                id
            ],
        )?;
        Ok(())
    }

    /// Advance attempt counter with exponential backoff (10, 20, 40, 80, 160 min).
    /// Returns true if still has attempts remaining, false if abandoned.
    pub fn advance_quill_sync_attempt(&self, id: &str) -> Result<bool, DbError> {
        let (attempts, max_attempts): (i32, i32) = self.conn.query_row(
            "SELECT attempts, max_attempts FROM quill_sync_state WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let new_attempts = attempts + 1;
        let now = Utc::now().to_rfc3339();

        if new_attempts >= max_attempts {
            self.conn.execute(
                "UPDATE quill_sync_state
                 SET attempts = ?1, state = 'abandoned', last_attempt_at = ?2, updated_at = ?2
                 WHERE id = ?3",
                params![new_attempts, now, id],
            )?;
            return Ok(false);
        }

        // Exponential backoff: 5 * 2^attempts minutes (5, 10, 20, 40, 80 min)
        let delay_minutes = 5i64 * (1i64 << new_attempts);
        let next_attempt = (Utc::now() + chrono::Duration::minutes(delay_minutes)).format("%Y-%m-%d %H:%M:%S").to_string();

        self.conn.execute(
            "UPDATE quill_sync_state
             SET attempts = ?1, last_attempt_at = ?2, next_attempt_at = ?3, updated_at = ?2
             WHERE id = ?4",
            params![new_attempts, now, next_attempt, id],
        )?;
        Ok(true)
    }

    /// Count sync rows in a given state (all sources).
    pub fn count_quill_syncs_by_state(&self, state: &str) -> Result<usize, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM quill_sync_state WHERE state = ?1",
            params![state],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Count sync rows in a given state for a specific source.
    pub fn count_syncs_by_state_and_source(
        &self,
        state: &str,
        source: &str,
    ) -> Result<usize, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM quill_sync_state WHERE state = ?1 AND source = ?2",
            params![state, source],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get abandoned Quill syncs eligible for retry (between min_days and max_days old).
    pub fn get_retryable_abandoned_quill_syncs(
        &self,
        min_days: i32,
        max_days: i32,
    ) -> Result<Vec<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state
             WHERE state = 'abandoned'
               AND created_at >= datetime('now', ?1)
               AND created_at <= datetime('now', ?2)",
        )?;
        let min_offset = format!("-{} days", max_days);
        let max_offset = format!("-{} days", min_days);
        let rows = stmt.query_map(params![min_offset, max_offset], map_sync_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Reset an abandoned Quill sync for retry: set state to pending, clear attempts.
    pub fn reset_quill_sync_for_retry(&self, sync_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE quill_sync_state
             SET state = 'pending', attempts = 0, error_message = NULL,
                 next_attempt_at = ?1, updated_at = ?1
             WHERE id = ?2",
            params![now, sync_id],
        )?;
        Ok(())
    }

    /// Get recent meetings as (id, title, start_time) tuples for transcript matching.
    pub fn get_meetings_for_transcript_matching(
        &self,
        days_back: i32,
    ) -> Result<Vec<(String, String, String)>, DbError> {
        let offset = format!("-{} days", days_back);
        let mut stmt = self.conn.prepare(
            "SELECT id, title, start_time FROM meetings_history
             WHERE start_time >= datetime('now', ?1)
             ORDER BY start_time DESC",
        )?;
        let rows = stmt.query_map(params![offset], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get meeting IDs eligible for Quill backfill: past meetings within `days_back`
    /// that have no transcript and no existing quill_sync_state row.
    ///
    /// Only includes account-linked meetings with relationship-relevant types
    /// (customer, qbr, partnership). Excludes internal, one_on_one, and external
    /// meetings which are too broad and would pull in personal or tangential calls.
    pub fn get_backfill_eligible_meeting_ids(&self, days_back: i32) -> Result<Vec<String>, DbError> {
        let offset = format!("-{} days", days_back);
        let mut stmt = self.conn.prepare(
            "SELECT m.id FROM meetings_history m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id AND me.entity_type = 'account'
             WHERE m.transcript_path IS NULL AND m.transcript_processed_at IS NULL
               AND m.start_time >= datetime('now', ?1)
               AND m.end_time < datetime('now')
               AND m.meeting_type IN ('customer','qbr','partnership')
               AND m.id NOT IN (SELECT meeting_id FROM quill_sync_state)
             ORDER BY m.start_time DESC",
        )?;
        let rows = stmt.query_map(params![offset], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }


    // =========================================================================
    // Meetings
    // =========================================================================

    /// Insert or update a meeting history record.
    pub fn upsert_meeting(&self, meeting: &DbMeeting) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO meetings_history (
                id, title, meeting_type, start_time, end_time,
                attendees, notes_path, summary, created_at,
                calendar_event_id, description, prep_context_json,
                user_agenda_json, user_notes, prep_frozen_json, prep_frozen_at,
                prep_snapshot_path, prep_snapshot_hash, transcript_path, transcript_processed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                meeting_type = excluded.meeting_type,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                attendees = excluded.attendees,
                notes_path = excluded.notes_path,
                summary = excluded.summary,
                calendar_event_id = excluded.calendar_event_id,
                description = excluded.description,
                prep_context_json = COALESCE(excluded.prep_context_json, meetings_history.prep_context_json),
                user_agenda_json = COALESCE(excluded.user_agenda_json, meetings_history.user_agenda_json),
                user_notes = COALESCE(excluded.user_notes, meetings_history.user_notes),
                prep_frozen_json = COALESCE(meetings_history.prep_frozen_json, excluded.prep_frozen_json),
                prep_frozen_at = COALESCE(meetings_history.prep_frozen_at, excluded.prep_frozen_at),
                prep_snapshot_path = COALESCE(meetings_history.prep_snapshot_path, excluded.prep_snapshot_path),
                prep_snapshot_hash = COALESCE(meetings_history.prep_snapshot_hash, excluded.prep_snapshot_hash),
                transcript_path = COALESCE(excluded.transcript_path, meetings_history.transcript_path),
                transcript_processed_at = COALESCE(excluded.transcript_processed_at, meetings_history.transcript_processed_at)",
            params![
                meeting.id,
                meeting.title,
                meeting.meeting_type,
                meeting.start_time,
                meeting.end_time,
                meeting.attendees,
                meeting.notes_path,
                meeting.summary,
                meeting.created_at,
                meeting.calendar_event_id,
                meeting.description,
                meeting.prep_context_json,
                meeting.user_agenda_json,
                meeting.user_notes,
                meeting.prep_frozen_json,
                meeting.prep_frozen_at,
                meeting.prep_snapshot_path,
                meeting.prep_snapshot_hash,
                meeting.transcript_path,
                meeting.transcript_processed_at,
            ],
        )?;

        Ok(())
    }

    /// Look up a meeting by its Google Calendar event ID (I168).
    pub fn get_meeting_by_calendar_event_id(
        &self,
        calendar_event_id: &str,
    ) -> Result<Option<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    attendees, notes_path, summary, created_at,
                    calendar_event_id, description, prep_context_json,
                    user_agenda_json, user_notes, prep_frozen_json, prep_frozen_at,
                    prep_snapshot_path, prep_snapshot_hash, transcript_path, transcript_processed_at,
                    intelligence_state, intelligence_quality, last_enriched_at,
                    signal_count, has_new_signals, last_viewed_at
             FROM meetings_history
             WHERE calendar_event_id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![calendar_event_id], |row| {
            Ok(DbMeeting {
                id: row.get(0)?,
                title: row.get(1)?,
                meeting_type: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                attendees: row.get(5)?,
                notes_path: row.get(6)?,
                summary: row.get(7)?,
                created_at: row.get(8)?,
                calendar_event_id: row.get(9)?,
                description: row.get(10)?,
                prep_context_json: row.get(11)?,
                user_agenda_json: row.get(12)?,
                user_notes: row.get(13)?,
                prep_frozen_json: row.get(14)?,
                prep_frozen_at: row.get(15)?,
                prep_snapshot_path: row.get(16)?,
                prep_snapshot_hash: row.get(17)?,
                transcript_path: row.get(18)?,
                transcript_processed_at: row.get(19)?,
                intelligence_state: row.get(20)?,
                intelligence_quality: row.get(21)?,
                last_enriched_at: row.get(22)?,
                signal_count: row.get(23)?,
                has_new_signals: row.get(24)?,
                last_viewed_at: row.get(25)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Ensure a meeting exists in meetings_history (INSERT OR IGNORE).
    /// Used by calendar polling to create lightweight records so
    /// record_meeting_attendance() can query start_time.
    /// Does NOT overwrite existing rows — reconcile.rs owns updates.
    pub fn ensure_meeting_in_history(
        &self,
        input: EnsureMeetingHistoryInput<'_>,
    ) -> Result<MeetingSyncOutcome, DbError> {
        // Check if meeting already exists and detect changes
        let mut stmt = self.conn.prepare(
            "SELECT title, start_time, attendees, description FROM meetings_history WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![input.id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let existing = rows.next().transpose()?;

        match existing {
            None => {
                // New meeting — insert with all available fields
                self.conn.execute(
                    "INSERT INTO meetings_history
                        (id, title, meeting_type, start_time, end_time, created_at, calendar_event_id, attendees, description)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        input.id,
                        input.title,
                        input.meeting_type,
                        input.start_time,
                        input.end_time,
                        Utc::now().to_rfc3339(),
                        input.calendar_event_id,
                        input.attendees,
                        input.description,
                    ],
                )?;
                Ok(MeetingSyncOutcome::New)
            }
            Some((old_title, old_start, old_attendees, old_description)) => {
                // Detect meaningful changes: title, time, attendees, or description
                let changed = old_title != input.title
                    || old_start != input.start_time
                    || old_attendees.as_deref() != input.attendees
                    || old_description.as_deref() != input.description;
                if changed {
                    self.conn.execute(
                        "UPDATE meetings_history SET title = ?1, start_time = ?2, end_time = ?3,
                         attendees = ?4, description = ?5
                         WHERE id = ?6",
                        params![
                            input.title,
                            input.start_time,
                            input.end_time,
                            input.attendees,
                            input.description,
                            input.id,
                        ],
                    )?;
                    Ok(MeetingSyncOutcome::Changed)
                } else {
                    Ok(MeetingSyncOutcome::Unchanged)
                }
            }
        }
    }

    // =========================================================================
    // Prep State Tracking (ADR-0033)
    // =========================================================================

    /// Record that a meeting prep has been reviewed.
    ///
    /// `meeting_id` is the canonical meeting identity (event-id primary).
    pub fn mark_prep_reviewed(
        &self,
        meeting_id: &str,
        calendar_event_id: Option<&str>,
        title: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO meeting_prep_state (prep_file, calendar_event_id, reviewed_at, title)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(prep_file) DO UPDATE SET
                reviewed_at = excluded.reviewed_at,
                calendar_event_id = excluded.calendar_event_id",
            params![meeting_id, calendar_event_id, now, title],
        )?;
        Ok(())
    }

    /// Get all reviewed meeting IDs. Returns a map of meeting_id → reviewed_at.
    pub fn get_reviewed_preps(&self) -> Result<std::collections::HashMap<String, String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT prep_file, reviewed_at FROM meeting_prep_state")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (file, at) = row?;
            map.insert(file, at);
        }
        Ok(map)
    }


}
