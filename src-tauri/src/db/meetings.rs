use super::*;

/// SQL fragment for a 3-table JOIN returning all DbMeeting columns.
fn full_meeting_join_sql() -> &'static str {
    "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
            m.attendees, m.notes_path, mt.summary, m.created_at,
            m.calendar_event_id, m.description, mp.prep_context_json,
            mp.user_agenda_json, mp.user_notes, mp.prep_frozen_json, mp.prep_frozen_at,
            mp.prep_snapshot_path, mp.prep_snapshot_hash, mt.transcript_path, mt.transcript_processed_at,
            mt.intelligence_state, mt.intelligence_quality, mt.last_enriched_at,
            mt.signal_count, mt.has_new_signals, mt.last_viewed_at
     FROM meetings m
     LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
     LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id"
}

/// Map a row from the full 3-table JOIN into a DbMeeting.
fn map_full_meeting_row(row: &rusqlite::Row) -> rusqlite::Result<DbMeeting> {
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
}

impl ActionDb {
    // =========================================================================
    // Meetings
    // =========================================================================

    /// Count meetings linked to an account within the last `days` days (I555).
    pub fn count_account_meetings_in_days(
        &self,
        account_id: &str,
        days: i32,
    ) -> Result<i64, DbError> {
        let days_param = format!("-{days}");
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*)
             FROM meetings m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1
               AND m.start_time >= datetime('now', ?2 || ' days')",
            params![account_id, days_param],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Count meetings linked to an account within a period ending `offset_days` ago,
    /// spanning `days` before that offset (I555). Used for previous-period comparison.
    pub fn count_account_meetings_in_period(
        &self,
        account_id: &str,
        days: i32,
        offset_days: i32,
    ) -> Result<i64, DbError> {
        let end_param = format!("-{offset_days}");
        let start_offset = offset_days + days;
        let start_param = format!("-{start_offset}");
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*)
             FROM meetings m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1
               AND m.start_time >= datetime('now', ?2 || ' days')
               AND m.start_time < datetime('now', ?3 || ' days')",
            params![account_id, start_param, end_param],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get the champion's person_id for an account from account_stakeholders (I555).
    pub fn get_champion_person_id(&self, account_id: &str) -> Result<Option<String>, DbError> {
        let result = self.conn.query_row(
            "SELECT person_id FROM account_stakeholders
             WHERE account_id = ?1 AND role = 'champion'
             LIMIT 1",
            params![account_id],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Query recent meetings for an account within `lookback_days`, limited to `limit` results.
    pub fn get_meeting_history(
        &self,
        account_id: &str,
        lookback_days: i32,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
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
        let sql = format!("{} WHERE m.id = ?1", full_meeting_join_sql());
        let mut stmt = self.conn.prepare(&sql)?;

        let mut rows = stmt.query_map(params![id], map_full_meeting_row)?;

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
            "SELECT mp.meeting_id, mp.prep_context_json
             FROM meeting_prep mp
             WHERE mp.prep_context_json IS NOT NULL
               AND trim(mp.prep_context_json) != ''",
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
            "INSERT INTO meeting_prep (meeting_id, prep_context_json)
             VALUES (?2, ?1)
             ON CONFLICT(meeting_id) DO UPDATE SET prep_context_json = excluded.prep_context_json",
            params![prep_context_json, meeting_id],
        )?;
        Ok(())
    }

    /// Update the frozen prep JSON for a meeting.
    pub fn update_prep_frozen_json(&self, meeting_id: &str, json: &str) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE meeting_prep SET prep_frozen_json = ?1, prep_frozen_at = ?2 WHERE meeting_id = ?3",
            params![json, now, meeting_id],
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
            "INSERT INTO meeting_prep (meeting_id, user_agenda_json, user_notes)
             VALUES (?3, ?1, ?2)
             ON CONFLICT(meeting_id) DO UPDATE SET
                 user_agenda_json = excluded.user_agenda_json,
                 user_notes = excluded.user_notes",
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
            "UPDATE meeting_prep
             SET prep_frozen_json = ?1,
                 prep_frozen_at = ?2,
                 prep_snapshot_path = ?3,
                 prep_snapshot_hash = ?4
             WHERE meeting_id = ?5
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

    /// Persist transcript metadata directly on the meeting transcript row.
    pub fn update_meeting_transcript_metadata(
        &self,
        meeting_id: &str,
        transcript_path: &str,
        processed_at: &str,
        summary_opt: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO meeting_transcripts (meeting_id, transcript_path, transcript_processed_at, summary)
             VALUES (?4, ?1, ?2, ?3)
             ON CONFLICT(meeting_id) DO UPDATE SET
                 transcript_path = excluded.transcript_path,
                 transcript_processed_at = excluded.transcript_processed_at,
                 summary = COALESCE(excluded.summary, meeting_transcripts.summary)",
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
        let mut sql = "UPDATE meeting_transcripts SET intelligence_state = ?1".to_string();
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
        sql.push_str(&format!(" WHERE meeting_id = ?{idx}"));
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
            "UPDATE meeting_transcripts SET has_new_signals = 1 WHERE meeting_id = ?1",
            params![meeting_id],
        )?;
        Ok(())
    }

    /// Clear new signals flag (when user views the meeting).
    pub fn clear_meeting_new_signals(&self, meeting_id: &str) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE meeting_transcripts SET has_new_signals = 0, last_viewed_at = ?2 WHERE meeting_id = ?1",
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
        let next_attempt = (Utc::now() + chrono::Duration::minutes(2))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
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
        let next_attempt = (Utc::now() + chrono::Duration::minutes(2))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
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
        let next_attempt = (Utc::now() + chrono::Duration::minutes(delay_minutes))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

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

    /// Get recent meetings as matching candidates with entity context (I474).
    ///
    /// Returns meetings from the last `days_back` days with their primary
    /// linked entity_id (if any). Used by the inbox processor to match
    /// incoming documents to historical meetings.
    pub fn get_recent_meetings_for_matching(
        &self,
        days_back: i32,
    ) -> Result<Vec<(String, String, String, Option<String>)>, DbError> {
        let offset = format!("-{} days", days_back);
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.start_time,
                    (SELECT me.entity_id FROM meeting_entities me
                     WHERE me.meeting_id = m.id LIMIT 1) AS entity_id
             FROM meetings m
             WHERE m.start_time >= datetime('now', ?1)
             ORDER BY m.start_time DESC",
        )?;
        let rows = stmt.query_map(params![offset], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get recent meetings as (id, title, start_time) tuples for transcript matching.
    pub fn get_meetings_for_transcript_matching(
        &self,
        days_back: i32,
    ) -> Result<Vec<(String, String, String)>, DbError> {
        let offset = format!("-{} days", days_back);
        let mut stmt = self.conn.prepare(
            "SELECT id, title, start_time FROM meetings
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
    pub fn get_backfill_eligible_meeting_ids(
        &self,
        days_back: i32,
    ) -> Result<Vec<String>, DbError> {
        let offset = format!("-{} days", days_back);
        let mut stmt = self.conn.prepare(
            "SELECT m.id FROM meetings m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id AND me.entity_type = 'account'
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             WHERE mt.transcript_path IS NULL AND mt.transcript_processed_at IS NULL
               AND m.start_time >= datetime('now', ?1)
               AND m.end_time < datetime('now')
               AND m.meeting_type IN ('customer','qbr','partnership')
               AND m.id NOT IN (SELECT meeting_id FROM quill_sync_state WHERE source = 'quill')
             ORDER BY m.start_time DESC",
        )?;
        let rows = stmt.query_map(params![offset], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get meetings in a date range [start, end) as lightweight tuples.
    ///
    /// Returns (id, title, meeting_type, start_time, end_time, prep_frozen_json IS NOT NULL).
    /// Used by I513 to build WeekOverview from DB instead of JSON files.
    pub fn get_meetings_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<(String, String, String, String, Option<String>, bool)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    (mp.prep_frozen_json IS NOT NULL) AS has_prep
             FROM meetings m
             LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
             WHERE m.start_time >= ?1 AND m.start_time < ?2
             ORDER BY m.start_time ASC",
        )?;
        let rows = stmt.query_map(params![start, end], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, bool>(5)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    // =========================================================================
    // Meetings
    // =========================================================================

    /// Insert or update a meeting history record (split across 3 tables).
    pub fn upsert_meeting(&self, meeting: &DbMeeting) -> Result<(), DbError> {
        // 1. meetings table (core scheduling data)
        self.conn.execute(
            "INSERT INTO meetings (
                id, title, meeting_type, start_time, end_time,
                attendees, notes_path, created_at,
                calendar_event_id, description
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                meeting_type = excluded.meeting_type,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                attendees = excluded.attendees,
                notes_path = excluded.notes_path,
                calendar_event_id = excluded.calendar_event_id,
                description = excluded.description",
            params![
                meeting.id,
                meeting.title,
                meeting.meeting_type,
                meeting.start_time,
                meeting.end_time,
                meeting.attendees,
                meeting.notes_path,
                meeting.created_at,
                meeting.calendar_event_id,
                meeting.description,
            ],
        )?;

        // 2. meeting_prep table
        self.conn.execute(
            "INSERT INTO meeting_prep (
                meeting_id, prep_context_json, user_agenda_json, user_notes,
                prep_frozen_json, prep_frozen_at, prep_snapshot_path, prep_snapshot_hash
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(meeting_id) DO UPDATE SET
                prep_context_json = COALESCE(excluded.prep_context_json, meeting_prep.prep_context_json),
                user_agenda_json = COALESCE(excluded.user_agenda_json, meeting_prep.user_agenda_json),
                user_notes = COALESCE(excluded.user_notes, meeting_prep.user_notes),
                prep_frozen_json = COALESCE(meeting_prep.prep_frozen_json, excluded.prep_frozen_json),
                prep_frozen_at = COALESCE(meeting_prep.prep_frozen_at, excluded.prep_frozen_at),
                prep_snapshot_path = COALESCE(meeting_prep.prep_snapshot_path, excluded.prep_snapshot_path),
                prep_snapshot_hash = COALESCE(meeting_prep.prep_snapshot_hash, excluded.prep_snapshot_hash)",
            params![
                meeting.id,
                meeting.prep_context_json,
                meeting.user_agenda_json,
                meeting.user_notes,
                meeting.prep_frozen_json,
                meeting.prep_frozen_at,
                meeting.prep_snapshot_path,
                meeting.prep_snapshot_hash,
            ],
        )?;

        // 3. meeting_transcripts table
        self.conn.execute(
            "INSERT INTO meeting_transcripts (
                meeting_id, summary, transcript_path, transcript_processed_at
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(meeting_id) DO UPDATE SET
                summary = COALESCE(excluded.summary, meeting_transcripts.summary),
                transcript_path = COALESCE(excluded.transcript_path, meeting_transcripts.transcript_path),
                transcript_processed_at = COALESCE(excluded.transcript_processed_at, meeting_transcripts.transcript_processed_at)",
            params![
                meeting.id,
                meeting.summary,
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
        let sql = format!(
            "{} WHERE m.calendar_event_id = ?1 LIMIT 1",
            full_meeting_join_sql()
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![calendar_event_id], map_full_meeting_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Ensure a meeting exists in the meetings table (INSERT OR IGNORE).
    /// Used by calendar polling to create lightweight records so
    /// record_meeting_attendance() can query start_time.
    /// Does NOT overwrite existing rows — reconcile.rs owns updates.
    pub fn ensure_meeting_in_history(
        &self,
        input: EnsureMeetingHistoryInput<'_>,
    ) -> Result<MeetingSyncOutcome, DbError> {
        // Check if meeting already exists and detect changes
        let mut stmt = self.conn.prepare(
            "SELECT title, start_time, attendees, description FROM meetings WHERE id = ?1",
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
                // New meeting — insert into meetings table
                self.conn.execute(
                    "INSERT INTO meetings
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
                // Create stub rows in child tables
                self.conn.execute(
                    "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
                    params![input.id],
                )?;
                self.conn.execute(
                    "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES (?1)",
                    params![input.id],
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
                        "UPDATE meetings SET title = ?1, start_time = ?2, end_time = ?3,
                         attendees = ?4, description = ?5, meeting_type = ?6
                         WHERE id = ?7",
                        params![
                            input.title,
                            input.start_time,
                            input.end_time,
                            input.attendees,
                            input.description,
                            input.meeting_type,
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

    // ─── I555: Interaction dynamics, champion health, role changes ────────

    /// Store interaction dynamics for a meeting.
    pub fn upsert_interaction_dynamics(
        &self,
        meeting_id: &str,
        dynamics: &super::types::InteractionDynamics,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meeting_interaction_dynamics
             (meeting_id, talk_balance_customer_pct, talk_balance_internal_pct,
              speaker_sentiments_json, question_density, decision_maker_active,
              forward_looking, monologue_risk, competitor_mentions_json, escalation_language_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                meeting_id,
                dynamics.talk_balance_customer_pct,
                dynamics.talk_balance_internal_pct,
                serde_json::to_string(&dynamics.speaker_sentiments).ok(),
                dynamics.question_density,
                dynamics.decision_maker_active,
                dynamics.forward_looking,
                dynamics.monologue_risk as i32,
                serde_json::to_string(&dynamics.competitor_mentions).ok(),
                serde_json::to_string(&dynamics.escalation_language).ok(),
            ],
        )?;
        Ok(())
    }

    /// Get interaction dynamics for a meeting.
    pub fn get_interaction_dynamics(
        &self,
        meeting_id: &str,
    ) -> Result<Option<super::types::InteractionDynamics>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT meeting_id, talk_balance_customer_pct, talk_balance_internal_pct,
                    speaker_sentiments_json, question_density, decision_maker_active,
                    forward_looking, monologue_risk, competitor_mentions_json, escalation_language_json
             FROM meeting_interaction_dynamics WHERE meeting_id = ?1",
        )?;
        let result = stmt.query_row(rusqlite::params![meeting_id], |row| {
            Ok(super::types::InteractionDynamics {
                meeting_id: row.get(0)?,
                talk_balance_customer_pct: row.get(1)?,
                talk_balance_internal_pct: row.get(2)?,
                speaker_sentiments: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                question_density: row.get(4)?,
                decision_maker_active: row.get(5)?,
                forward_looking: row.get(6)?,
                monologue_risk: row.get::<_, Option<i32>>(7)?.unwrap_or(0) != 0,
                competitor_mentions: row
                    .get::<_, Option<String>>(8)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                escalation_language: row
                    .get::<_, Option<String>>(9)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            })
        });
        match result {
            Ok(d) => Ok(Some(d)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Store champion health assessment for a meeting.
    pub fn upsert_champion_health(
        &self,
        meeting_id: &str,
        assessment: &super::types::ChampionHealthAssessment,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meeting_champion_health
             (meeting_id, champion_name, champion_status, champion_evidence, champion_risk)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                meeting_id,
                assessment.champion_name,
                assessment.champion_status,
                assessment.champion_evidence,
                assessment.champion_risk,
            ],
        )?;
        Ok(())
    }

    /// Get champion health for a meeting.
    pub fn get_champion_health(
        &self,
        meeting_id: &str,
    ) -> Result<Option<super::types::ChampionHealthAssessment>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT meeting_id, champion_name, champion_status, champion_evidence, champion_risk
             FROM meeting_champion_health WHERE meeting_id = ?1",
        )?;
        let result = stmt.query_row(rusqlite::params![meeting_id], |row| {
            Ok(super::types::ChampionHealthAssessment {
                meeting_id: row.get(0)?,
                champion_name: row.get(1)?,
                champion_status: row.get(2)?,
                champion_evidence: row.get(3)?,
                champion_risk: row.get(4)?,
            })
        });
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Store role changes detected in a meeting.
    pub fn insert_role_changes(
        &self,
        meeting_id: &str,
        changes: &[super::types::RoleChange],
    ) -> Result<(), DbError> {
        // Clear existing for this meeting first (idempotent re-processing)
        self.conn.execute(
            "DELETE FROM meeting_role_changes WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
        )?;
        for change in changes {
            self.conn.execute(
                "INSERT INTO meeting_role_changes (id, meeting_id, person_name, old_status, new_status, evidence_quote)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    change.id,
                    meeting_id,
                    change.person_name,
                    change.old_status,
                    change.new_status,
                    change.evidence_quote,
                ],
            )?;
        }
        Ok(())
    }

    /// Clear all transcript extraction data for a meeting so it can be reprocessed.
    ///
    /// Removes captures, transcript-sourced actions, interaction dynamics,
    /// champion health, role changes, and captured commitments. Resets the
    /// meeting's summary and transcript_processed_at so the pipeline treats
    /// it as a fresh extraction.
    pub fn clear_meeting_extraction_data(&self, meeting_id: &str) -> Result<usize, DbError> {
        let mut cleared = 0usize;
        cleared += self.conn.execute(
            "DELETE FROM captures WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
        )?;
        cleared += self.conn.execute(
            "DELETE FROM actions WHERE source_id = ?1 AND source_type IN ('transcript', 'post_meeting')",
            rusqlite::params![meeting_id],
        )?;
        cleared += self.conn.execute(
            "DELETE FROM meeting_interaction_dynamics WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
        )?;
        cleared += self.conn.execute(
            "DELETE FROM meeting_champion_health WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
        )?;
        cleared += self.conn.execute(
            "DELETE FROM meeting_role_changes WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
        )?;
        cleared += self.conn.execute(
            "DELETE FROM captured_commitments WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
        )?;
        // Reset transcript metadata so pipeline treats this as fresh
        self.conn.execute(
            "UPDATE meeting_transcripts SET summary = NULL, transcript_processed_at = NULL WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
        )?;
        Ok(cleared)
    }

    /// Get role changes for a meeting.
    pub fn get_role_changes(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<super::types::RoleChange>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, person_name, old_status, new_status, evidence_quote
             FROM meeting_role_changes WHERE meeting_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(rusqlite::params![meeting_id], |row| {
            Ok(super::types::RoleChange {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                person_name: row.get(2)?,
                old_status: row.get(3)?,
                new_status: row.get(4)?,
                evidence_quote: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get enriched captures for a meeting (with metadata columns).
    pub fn get_enriched_captures(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<super::types::EnrichedCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, capture_type,
                    content, sub_type, urgency, impact, evidence_quote, speaker, captured_at
             FROM captures WHERE meeting_id = ?1
             ORDER BY CASE urgency WHEN 'red' THEN 0 WHEN 'yellow' THEN 1 WHEN 'green_watch' THEN 2 ELSE 3 END,
                      captured_at",
        )?;
        let rows = stmt.query_map(rusqlite::params![meeting_id], |row| {
            Ok(super::types::EnrichedCapture {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                account_id: row.get(3)?,
                capture_type: row.get(4)?,
                content: row.get(5)?,
                sub_type: row.get(6)?,
                urgency: row.get(7)?,
                impact: row.get(8)?,
                evidence_quote: row.get(9)?,
                speaker: row.get(10)?,
                captured_at: row.get(11)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get enriched captures for an account within a date range (for reports).
    pub fn get_account_enriched_captures(
        &self,
        account_id: &str,
        days: i32,
    ) -> Result<Vec<super::types::EnrichedCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, capture_type,
                    content, sub_type, urgency, impact, evidence_quote, speaker, captured_at
             FROM captures
             WHERE account_id = ?1
               AND captured_at >= datetime('now', ?2)
             ORDER BY CASE urgency WHEN 'red' THEN 0 WHEN 'yellow' THEN 1 WHEN 'green_watch' THEN 2 ELSE 3 END,
                      captured_at DESC",
        )?;
        let days_param = format!("-{days} days");
        let rows = stmt.query_map(rusqlite::params![account_id, days_param], |row| {
            Ok(super::types::EnrichedCapture {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                account_id: row.get(3)?,
                capture_type: row.get(4)?,
                content: row.get(5)?,
                sub_type: row.get(6)?,
                urgency: row.get(7)?,
                impact: row.get(8)?,
                evidence_quote: row.get(9)?,
                speaker: row.get(10)?,
                captured_at: row.get(11)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get post-meeting intelligence bundle (I558).
    pub fn get_meeting_post_intelligence(
        &self,
        meeting_id: &str,
    ) -> Result<super::types::MeetingPostIntelligence, DbError> {
        let dynamics = self.get_interaction_dynamics(meeting_id)?;
        let champion = self.get_champion_health(meeting_id)?;
        let role_changes = self.get_role_changes(meeting_id)?;
        let captures = self.get_enriched_captures(meeting_id)?;
        Ok(super::types::MeetingPostIntelligence {
            interaction_dynamics: dynamics,
            champion_health: champion,
            role_changes,
            enriched_captures: captures,
        })
    }
}
