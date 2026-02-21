use super::*;

impl ActionDb {
    // =========================================================================
    // Stakeholder Signals (I43)
    // =========================================================================

    /// Compute stakeholder signals for an account: meeting frequency, last contact,
    /// and relationship temperature. Returns `None` if account not found.
    pub fn get_stakeholder_signals(&self, account_id: &str) -> Result<StakeholderSignals, DbError> {
        // Meeting counts for 30/90 day windows (via junction table)
        let count_30d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 INNER JOIN meeting_entities me ON m.id = me.meeting_id
                 WHERE me.entity_id = ?1
                   AND m.start_time >= date('now', '-30 days')",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let count_90d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 INNER JOIN meeting_entities me ON m.id = me.meeting_id
                 WHERE me.entity_id = ?1
                   AND m.start_time >= date('now', '-90 days')",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Last meeting date
        let last_meeting: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(m.start_time) FROM meetings_history m
                 INNER JOIN meeting_entities me ON m.id = me.meeting_id
                 WHERE me.entity_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        // Last contact from accounts table (updated_at is touched on each interaction)
        let last_contact: Option<String> = self
            .conn
            .query_row(
                "SELECT updated_at FROM accounts
                 WHERE id = ?1 OR LOWER(name) = LOWER(?1)",
                params![account_id],
                |row| row.get(0),
            )
            .ok();

        // Temperature: based on days since last meeting
        let temperature = match &last_meeting {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        };

        // Trend: compare 30d vs 90d rate
        let trend = compute_trend(count_30d, count_90d);

        Ok(StakeholderSignals {
            meeting_frequency_30d: count_30d,
            meeting_frequency_90d: count_90d,
            last_meeting,
            last_contact,
            temperature,
            trend,
        })
    }

    // =========================================================================
    // Processing Log
    // =========================================================================

    /// Insert a processing log entry.
    pub fn insert_processing_log(&self, entry: &DbProcessingLog) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO processing_log (id, filename, source_path, destination_path, classification, status, processed_at, error_message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.filename,
                entry.source_path,
                entry.destination_path,
                entry.classification,
                entry.status,
                entry.processed_at,
                entry.error_message,
                entry.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get recent processing log entries.
    pub fn get_processing_log(&self, limit: i32) -> Result<Vec<DbProcessingLog>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, source_path, destination_path, classification, status, processed_at, error_message, created_at
             FROM processing_log
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(DbProcessingLog {
                id: row.get(0)?,
                filename: row.get(1)?,
                source_path: row.get(2)?,
                destination_path: row.get(3)?,
                classification: row.get(4)?,
                status: row.get(5)?,
                processed_at: row.get(6)?,
                error_message: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Get the latest processing status for each filename in the processing_log.
    ///
    /// Returns a map of `filename -> (status, error_message)` using the most recent
    /// log entry per filename. Uses the existing `idx_processing_created` index.
    pub fn get_latest_processing_status(
        &self,
    ) -> Result<HashMap<String, (String, Option<String>)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.filename, p.status, p.error_message
             FROM processing_log p
             INNER JOIN (
                 SELECT filename, MAX(created_at) AS max_created
                 FROM processing_log
                 GROUP BY filename
             ) latest ON p.filename = latest.filename AND p.created_at = latest.max_created",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        let mut map = HashMap::new();
        for row in rows {
            let (filename, status, error_message) = row?;
            map.insert(filename, (status, error_message));
        }
        Ok(map)
    }

    // =========================================================================
    // Captures (post-meeting wins/risks)
    // =========================================================================

    /// Map a row to DbCapture. Expects columns:
    /// id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
    fn map_capture_row(row: &rusqlite::Row) -> rusqlite::Result<DbCapture> {
        Ok(DbCapture {
            id: row.get(0)?,
            meeting_id: row.get(1)?,
            meeting_title: row.get(2)?,
            account_id: row.get(3)?,
            project_id: row.get(4)?,
            capture_type: row.get(5)?,
            content: row.get(6)?,
            captured_at: row.get(7)?,
        })
    }

    /// Insert a capture (win, risk, or action) from a post-meeting prompt.
    pub fn insert_capture(
        &self,
        meeting_id: &str,
        meeting_title: &str,
        account_id: Option<&str>,
        capture_type: &str,
        content: &str,
    ) -> Result<(), DbError> {
        self.insert_capture_with_project(
            meeting_id,
            meeting_title,
            account_id,
            None,
            capture_type,
            content,
        )
    }

    /// Insert a capture with optional project_id (I52).
    pub fn insert_capture_with_project(
        &self,
        meeting_id: &str,
        meeting_title: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
        capture_type: &str,
        content: &str,
    ) -> Result<(), DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO captures (id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, meeting_id, meeting_title, account_id, project_id, capture_type, content, now],
        )?;
        Ok(())
    }

    /// Query recent captures (wins/risks) for an account within `days_back` days.
    ///
    /// Used by meeting:prep (ADR-0030 / I33) to surface recent wins and risks
    /// in meeting preparation context.
    pub fn get_captures_for_account(
        &self,
        account_id: &str,
        days_back: i32,
    ) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE account_id = ?1
               AND captured_at >= date('now', ?2 || ' days')
             ORDER BY captured_at DESC",
        )?;

        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![account_id, days_param], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Query recent captures for a project within `days_back` days (I52).
    pub fn get_captures_for_project(
        &self,
        project_id: &str,
        days_back: i32,
    ) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE project_id = ?1
               AND captured_at >= date('now', ?2 || ' days')
             ORDER BY captured_at DESC",
        )?;

        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![project_id, days_param], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Query all captures (wins, risks, decisions) for a specific meeting.
    pub fn get_captures_for_meeting(&self, meeting_id: &str) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE meeting_id = ?1
             ORDER BY captured_at",
        )?;

        let rows = stmt.query_map(params![meeting_id], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Get recent captures from meetings a person attended within `days_back` days.
    pub fn get_captures_for_person(
        &self,
        person_id: &str,
        days_back: i32,
    ) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.meeting_id, c.meeting_title, c.account_id, c.project_id, c.capture_type, c.content, c.captured_at
             FROM captures c
             JOIN meeting_attendees ma ON ma.meeting_id = c.meeting_id
             WHERE ma.person_id = ?1
               AND c.captured_at >= date('now', ?2 || ' days')
             ORDER BY c.captured_at DESC
             LIMIT 20",
        )?;

        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![person_id, days_param], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Insert an email intelligence signal, deduped by `(email_id, entity_id, signal_type, signal_text)`.
    /// Known signal types from AI enrichment. Unknown types are rejected to prevent
    /// hallucinated categories from polluting the database.
    const VALID_SIGNAL_TYPES: &'static [&'static str] = &[
        "expansion",
        "question",
        "timeline",
        "sentiment",
        "feedback",
        "relationship",
    ];

    const VALID_ENTITY_TYPES: &'static [&'static str] = &["account", "project"];

    /// Insert an email signal, returning `true` if a new row was inserted.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_email_signal(
        &self,
        email_id: &str,
        sender_email: Option<&str>,
        person_id: Option<&str>,
        entity_id: &str,
        entity_type: &str,
        signal_type: &str,
        signal_text: &str,
        confidence: Option<f64>,
        sentiment: Option<&str>,
        urgency: Option<&str>,
        detected_at: Option<&str>,
    ) -> Result<bool, DbError> {
        if !Self::VALID_SIGNAL_TYPES.contains(&signal_type) {
            log::warn!(
                "Ignoring unknown email signal type '{}' for entity {}",
                signal_type,
                entity_id
            );
            return Ok(false);
        }
        if !Self::VALID_ENTITY_TYPES.contains(&entity_type) {
            log::warn!(
                "Ignoring unknown entity type '{}' for email signal",
                entity_type
            );
            return Ok(false);
        }

        self.conn.execute(
            "INSERT OR IGNORE INTO email_signals (
                email_id, sender_email, person_id, entity_id, entity_type,
                signal_type, signal_text, confidence, sentiment, urgency, detected_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, COALESCE(?11, datetime('now')))",
            params![
                email_id,
                sender_email,
                person_id,
                entity_id,
                entity_type,
                signal_type,
                signal_text,
                confidence,
                sentiment,
                urgency,
                detected_at,
            ],
        )?;
        Ok(self.conn.changes() > 0)
    }

    /// List recent email signals for an entity.
    pub fn list_recent_email_signals_for_entity(
        &self,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<DbEmailSignal>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email_id, sender_email, person_id, entity_id, entity_type,
                    signal_type, signal_text, confidence, sentiment, urgency, detected_at
             FROM email_signals
             WHERE entity_id = ?1
             ORDER BY detected_at DESC, id DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![entity_id, limit as i64], |row| {
            Ok(DbEmailSignal {
                id: row.get(0)?,
                email_id: row.get(1)?,
                sender_email: row.get(2)?,
                person_id: row.get(3)?,
                entity_id: row.get(4)?,
                entity_type: row.get(5)?,
                signal_type: row.get(6)?,
                signal_text: row.get(7)?,
                confidence: row.get(8)?,
                sentiment: row.get(9)?,
                urgency: row.get(10)?,
                detected_at: row.get(11)?,
            })
        })?;

        let mut signals = Vec::new();
        for row in rows {
            signals.push(row?);
        }
        Ok(signals)
    }

    /// Batch-query email signals for multiple email IDs.
    /// Returns all signals whose email_id is in the provided list.
    pub fn list_email_signals_by_email_ids(
        &self,
        email_ids: &[String],
    ) -> Result<Vec<DbEmailSignal>, DbError> {
        if email_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=email_ids.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT id, email_id, sender_email, person_id, entity_id, entity_type,
                    signal_type, signal_text, confidence, sentiment, urgency, detected_at
             FROM email_signals
             WHERE email_id IN ({})
             ORDER BY detected_at DESC, id DESC",
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = email_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt.query_map(&*params, |row| {
            Ok(DbEmailSignal {
                id: row.get(0)?,
                email_id: row.get(1)?,
                sender_email: row.get(2)?,
                person_id: row.get(3)?,
                entity_id: row.get(4)?,
                entity_type: row.get(5)?,
                signal_type: row.get(6)?,
                signal_text: row.get(7)?,
                confidence: row.get(8)?,
                sentiment: row.get(9)?,
                urgency: row.get(10)?,
                detected_at: row.get(11)?,
            })
        })?;

        let mut signals = Vec::new();
        for row in rows {
            signals.push(row?);
        }
        Ok(signals)
    }

    // ================================================================
    // Email dismissals (I342 — The Correspondent relevance learning)
    // ================================================================

    /// Record a user dismissal of an email-extracted item for relevance learning.
    pub fn dismiss_email_item(
        &self,
        item_type: &str,
        email_id: &str,
        item_text: &str,
        sender_domain: Option<&str>,
        email_type: Option<&str>,
        entity_id: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO email_dismissals (item_type, email_id, item_text, sender_domain, email_type, entity_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![item_type, email_id, item_text, sender_domain, email_type, entity_id],
        )?;
        Ok(())
    }

    /// Get all dismissed item texts for filtering (keyed by item_type + item_text).
    pub fn list_dismissed_email_items(&self) -> Result<std::collections::HashSet<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT item_type || ':' || item_text FROM email_dismissals"
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut dismissed = std::collections::HashSet::new();
        for row in rows {
            dismissed.insert(row?);
        }
        Ok(dismissed)
    }

    // ================================================================
    // Email thread tracking (I318)
    // ================================================================

    /// Upsert an email thread position record.
    pub fn upsert_email_thread(
        &self,
        thread_id: &str,
        subject: &str,
        last_sender_email: &str,
        last_message_date: &str,
        message_count: i32,
        user_is_last_sender: bool,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO email_threads (thread_id, subject, last_sender_email, last_message_date,
                    message_count, user_is_last_sender, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
             ON CONFLICT(thread_id) DO UPDATE SET
                subject = excluded.subject,
                last_sender_email = excluded.last_sender_email,
                last_message_date = excluded.last_message_date,
                message_count = excluded.message_count,
                user_is_last_sender = excluded.user_is_last_sender,
                updated_at = datetime('now')",
            params![
                thread_id,
                subject,
                last_sender_email,
                last_message_date,
                message_count,
                user_is_last_sender as i32,
            ],
        )?;
        Ok(())
    }

    /// Get threads awaiting the user's reply (ball in your court).
    pub fn get_threads_awaiting_reply(&self) -> Result<Vec<(String, String, String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT thread_id, subject, last_sender_email, last_message_date
             FROM email_threads
             WHERE user_is_last_sender = 0
               AND updated_at >= datetime('now', '-7 days')
             ORDER BY last_message_date DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    // ================================================================
    // Hygiene actions log (I353 Phase 2)
    // ================================================================

    /// Log a hygiene action triggered by a signal.
    pub fn log_hygiene_action(
        &self,
        source_signal_id: Option<&str>,
        action_type: &str,
        entity_id: &str,
        entity_type: &str,
        confidence: f64,
        result: &str,
    ) -> Result<(), DbError> {
        let id = format!("ha-{}", uuid::Uuid::new_v4());
        self.conn.execute(
            "INSERT INTO hygiene_actions_log (id, source_signal_id, action_type, entity_id, entity_type, confidence, result)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, source_signal_id, action_type, entity_id, entity_type, confidence, result],
        )?;
        Ok(())
    }


    /// Query all captures (wins/risks/decisions) recorded on a given date.
    ///
    /// Used by the daily impact rollup (I36) to aggregate outcomes into
    /// the weekly impact file during the archive workflow.
    pub fn get_captures_for_date(&self, date: &str) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE date(captured_at) = ?1
             ORDER BY account_id, captured_at",
        )?;

        let rows = stmt.query_map(params![date], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Update the content of a capture (win/risk/decision).
    pub fn update_capture(&self, id: &str, content: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE captures SET content = ?1 WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    }

}
