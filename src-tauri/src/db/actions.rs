use super::*;

impl ActionDb {
    // =========================================================================
    // Actions
    // =========================================================================

    /// Query pending actions where `due_date` is within `days_ahead` days or is NULL.
    ///
    /// Results are ordered: overdue first, then by priority, then by due date.
    /// Includes correlated subqueries for the next upcoming meeting per action's account (I342).
    pub fn get_due_actions(&self, days_ahead: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    (SELECT m.title FROM meeting_entities me
                     JOIN meetings_history m ON me.meeting_id = m.id
                     WHERE me.entity_id = actions.account_id
                       AND m.start_time >= date('now')
                       AND m.start_time < date('now', '+3 days')
                     ORDER BY m.start_time ASC LIMIT 1) AS next_meeting_title,
                    (SELECT m.start_time FROM meeting_entities me
                     JOIN meetings_history m ON me.meeting_id = m.id
                     WHERE me.entity_id = actions.account_id
                       AND m.start_time >= date('now')
                       AND m.start_time < date('now', '+3 days')
                     ORDER BY m.start_time ASC LIMIT 1) AS next_meeting_start
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'pending'
               AND (due_date IS NULL OR due_date <= date('now', ?1 || ' days'))
             ORDER BY
               CASE WHEN due_date < date('now') THEN 0 ELSE 1 END,
               priority,
               due_date",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], |row| {
            let mut action = Self::map_action_row(row)?;
            action.next_meeting_title = row.get(17)?;
            action.next_meeting_start = row.get(18)?;
            Ok(action)
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Query pending + waiting actions for focus prioritization.
    ///
    /// Includes actions with no due date so the ranker can decide feasibility.
    /// Ordered by urgency first, then priority/due date.
    pub fn get_focus_candidate_actions(&self, days_ahead: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status IN ('pending', 'waiting')
               AND (due_date IS NULL OR due_date <= date('now', ?1 || ' days'))
             ORDER BY
               CASE
                 WHEN due_date < date('now') THEN 0
                 WHEN due_date = date('now') THEN 1
                 WHEN due_date IS NULL THEN 3
                 ELSE 2
               END,
               priority,
               due_date",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Query pending and waiting actions for a specific account.
    pub fn get_account_actions(&self, account_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE account_id = ?1
               AND status IN ('pending', 'waiting')
             ORDER BY priority, due_date",
        )?;

        let rows = stmt.query_map(params![account_id], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Query actions for a specific person using hybrid 1:1 heuristic (I351).
    ///
    /// Returns actions where this person is the primary external relationship:
    /// 1. Actions directly assigned via `person_id`
    /// 2. Actions from meetings where `meeting_type = 'one_on_one'` AND person attended
    /// 3. Actions from 2-attendee meetings where person attended
    pub fn get_person_actions(&self, person_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT a.id, a.title, a.priority, a.status, a.created_at, a.due_date,
                    a.completed_at, a.account_id, a.project_id, a.source_type, a.source_id,
                    a.source_label, a.context, a.waiting_on, a.updated_at, a.person_id,
                    acc.name AS account_name
             FROM actions a
             LEFT JOIN accounts acc ON a.account_id = acc.id
             WHERE a.status IN ('pending', 'completed')
               AND (
                 -- Direct person assignment
                 a.person_id = ?1
                 -- OR meeting-sourced where person is primary relationship
                 OR (
                   a.source_type IN ('post_meeting', 'transcript')
                   AND a.source_id IN (
                     SELECT m.id FROM meetings_history m
                     JOIN meeting_attendees ma ON m.id = ma.meeting_id
                     WHERE ma.person_id = ?1
                       AND (
                         m.meeting_type = 'one_on_one'
                         OR (SELECT COUNT(*) FROM meeting_attendees WHERE meeting_id = m.id) = 2
                       )
                   )
                 )
               )
             ORDER BY
               CASE a.status WHEN 'pending' THEN 0 ELSE 1 END,
               a.created_at DESC
             LIMIT 20",
        )?;

        let rows = stmt.query_map(params![person_id], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get upcoming (future) meetings for a person, soonest first.
    pub fn get_upcoming_meetings_for_person(
        &self,
        person_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             JOIN meeting_attendees ma ON m.id = ma.meeting_id
             WHERE ma.person_id = ?1
               AND m.start_time >= datetime('now')
             ORDER BY m.start_time ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![person_id, limit], |row| {
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
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Mark an action as completed with the current timestamp.
    pub fn complete_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET status = 'completed', completed_at = ?1, updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Reopen a completed action, clearing the completed_at timestamp.
    pub fn reopen_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET status = 'pending', completed_at = NULL, updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Get a single action by its ID.
    pub fn get_action_by_id(&self, id: &str) -> Result<Option<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE actions.id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], Self::map_action_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all actions completed within the last N hours (for display in the UI).
    pub fn get_completed_actions(&self, since_hours: u32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'completed'
               AND completed_at >= datetime('now', ?1)
             ORDER BY completed_at DESC",
        )?;

        let hours_param = format!("-{} hours", since_hours);
        let rows = stmt.query_map(params![hours_param], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get actions recently marked as completed (within the last N hours)
    /// that have a source_label set (so we know which file to update).
    pub fn get_recently_completed(&self, since_hours: u32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'completed'
               AND completed_at >= datetime('now', ?1)
               AND source_label IS NOT NULL
             ORDER BY completed_at DESC",
        )?;

        let hours_param = format!("-{} hours", since_hours);
        let rows = stmt.query_map(params![hours_param], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Insert or update an action, but never overwrite a user-set `completed` status.
    ///
    /// Checks two conditions before inserting:
    /// 1. **Title-based guard**: If a matching action (same title + account) is already
    ///    completed under *any* ID, skip the insert. This catches cross-source duplicates
    ///    where the same action arrives from briefing vs inbox vs post-meeting capture
    ///    with different ID schemes.
    /// 2. **ID-based guard**: If an action with this exact ID is already completed, skip.
    ///
    /// This ensures that daily briefing syncs don't resurrect completed actions (I23).
    pub fn upsert_action_if_not_completed(&self, action: &DbAction) -> Result<(), DbError> {
        // Guard 1: Title-based cross-source dedup — skip if ANY action with the
        // same title+account already exists (pending, waiting, or completed).
        let title_exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM actions
                 WHERE LOWER(TRIM(title)) = LOWER(TRIM(?1))
                   AND (account_id = ?2 OR (?2 IS NULL AND account_id IS NULL))
                 LIMIT 1",
                params![action.title, action.account_id],
                |_row| Ok(true),
            )
            .unwrap_or(false);

        if title_exists {
            return Ok(());
        }

        // Guard 2: ID-based check — don't overwrite a completed action
        let existing_status: Option<String> = self
            .conn
            .query_row(
                "SELECT status FROM actions WHERE id = ?1",
                params![action.id],
                |row| row.get(0),
            )
            .ok();

        if existing_status.as_deref() == Some("completed") {
            return Ok(());
        }

        self.upsert_action(action)
    }

    /// Insert or update an action. Uses SQLite `ON CONFLICT` (upsert).
    pub fn upsert_action(&self, action: &DbAction) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO actions (
                id, title, priority, status, created_at, due_date, completed_at,
                account_id, project_id, source_type, source_id, source_label,
                context, waiting_on, updated_at, person_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                priority = excluded.priority,
                status = excluded.status,
                due_date = excluded.due_date,
                completed_at = excluded.completed_at,
                account_id = excluded.account_id,
                project_id = excluded.project_id,
                source_type = excluded.source_type,
                source_id = excluded.source_id,
                source_label = excluded.source_label,
                context = excluded.context,
                waiting_on = excluded.waiting_on,
                updated_at = excluded.updated_at,
                person_id = excluded.person_id",
            params![
                action.id,
                action.title,
                action.priority,
                action.status,
                action.created_at,
                action.due_date,
                action.completed_at,
                action.account_id,
                action.project_id,
                action.source_type,
                action.source_id,
                action.source_label,
                action.context,
                action.waiting_on,
                action.updated_at,
                action.person_id,
            ],
        )?;
        Ok(())
    }

    /// Get pending actions from non-briefing sources (post-meeting capture, inbox).
    ///
    /// These actions live in SQLite but are NOT in `actions.json` (which only
    /// contains briefing-generated actions). Used by `get_dashboard_data()` to
    /// merge captured actions into the dashboard view (I17).
    pub fn get_non_briefing_pending_actions(&self) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status IN ('pending', 'waiting')
               AND source_type IN ('post_meeting', 'inbox', 'ai-inbox', 'transcript', 'import', 'manual')
             ORDER BY priority, created_at DESC",
        )?;

        let rows = stmt.query_map([], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get all action titles from the database (for dedup in Rust delivery).
    pub fn get_all_action_titles(&self) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT LOWER(TRIM(title)) FROM actions")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut titles = Vec::new();
        for row in rows {
            titles.push(row?);
        }
        Ok(titles)
    }

    // =========================================================================
    // Proposed Actions (I256)
    // =========================================================================

    /// Get all proposed actions.
    pub fn get_proposed_actions(&self) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'proposed'
             ORDER BY priority, created_at DESC",
        )?;

        let rows = stmt.query_map([], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Accept a proposed action, moving it to pending status.
    pub fn accept_proposed_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'pending', updated_at = ?1
             WHERE id = ?2 AND status = 'proposed'",
            params![now, id],
        )?;
        if changed == 0 {
            return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }
        Ok(())
    }

    /// Reject a proposed action by archiving it and recording the rejection signal.
    pub fn reject_proposed_action(&self, id: &str) -> Result<(), DbError> {
        self.reject_proposed_action_with_source(id, "unknown")
    }

    /// Reject a proposed action, recording the source surface for correction learning.
    pub fn reject_proposed_action_with_source(
        &self,
        id: &str,
        source: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1,
             rejected_at = ?1, rejection_source = ?3
             WHERE id = ?2 AND status = 'proposed'",
            params![now, id, source],
        )?;
        if changed == 0 {
            return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }
        Ok(())
    }

    /// Archive an action (any status -> archived).
    pub fn archive_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Auto-archive stale pending actions older than N days.
    /// Returns the number of actions archived.
    pub fn archive_stale_actions(&self, days: i64) -> Result<usize, DbError> {
        let now = Utc::now().to_rfc3339();
        let cutoff_param = format!("-{} days", days);
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1
             WHERE status = 'pending'
               AND completed_at IS NULL
               AND created_at < datetime('now', ?2)",
            params![now, cutoff_param],
        )?;
        Ok(changed)
    }

    /// Auto-archive proposed actions older than N days.
    /// Returns the number of actions archived.
    pub fn auto_archive_old_proposed(&self, days: i64) -> Result<usize, DbError> {
        let now = Utc::now().to_rfc3339();
        let cutoff_param = format!("-{} days", days);
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1
             WHERE status = 'proposed'
               AND created_at < datetime('now', ?2)",
            params![now, cutoff_param],
        )?;
        Ok(changed)
    }


    /// Query actions extracted from a transcript for a specific meeting.
    pub fn get_actions_for_meeting(&self, meeting_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE source_id = ?1 AND source_type = 'transcript'
             ORDER BY priority, created_at",
        )?;

        let rows = stmt.query_map(params![meeting_id], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Update an action's priority.
    pub fn update_action_priority(&self, id: &str, priority: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET priority = ?1, updated_at = ?2 WHERE id = ?3",
            params![priority, now, id],
        )?;
        Ok(())
    }

    // =========================================================================
    // Intelligence Queries (I42 — Executive Intelligence)
    // =========================================================================

    /// Get actions in `waiting` status that are older than `stale_days`.
    ///
    /// These represent stale delegations — things handed off to someone else
    /// that haven't been resolved. Ordered by staleness (oldest first).
    pub fn get_stale_delegations(&self, stale_days: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'waiting'
               AND created_at <= datetime('now', ?1 || ' days')
             ORDER BY created_at ASC",
        )?;

        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get actions flagged as needing a decision, due within `days_ahead` days.
    ///
    /// The `needs_decision` flag is set by AI enrichment during briefing generation.
    /// Actions with no due date are included (they still need decisions).
    pub fn get_flagged_decisions(&self, days_ahead: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE needs_decision = 1
               AND status = 'pending'
               AND (due_date IS NULL OR due_date <= date('now', ?1 || ' days'))
             ORDER BY
               CASE WHEN due_date IS NULL THEN 1 ELSE 0 END,
               due_date ASC,
               priority",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get accounts with `contract_end` within `days_ahead` days.
    ///
    /// Returns accounts approaching renewal, ordered by soonest first.
    pub fn get_renewal_alerts(&self, days_ahead: i32) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE contract_end IS NOT NULL
               AND contract_end >= date('now')
               AND contract_end <= date('now', ?1 || ' days')
             ORDER BY contract_end ASC",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get accounts where `updated_at` is older than `stale_days`.
    ///
    /// Represents accounts that haven't been touched (via meetings, captures,
    /// or manual updates) in a while — a signal to check in.
    pub fn get_stale_accounts(&self, stale_days: i32) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE updated_at <= datetime('now', ?1 || ' days')
             ORDER BY updated_at ASC",
        )?;

        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Flag an action as needing a decision. Called by AI enrichment during
    /// briefing generation to mark actions that require user decisions.
    pub fn flag_action_as_decision(&self, id: &str) -> Result<bool, DbError> {
        let rows = self.conn.execute(
            "UPDATE actions SET needs_decision = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(rows > 0)
    }

    /// Clear all decision flags. Called before re-flagging during enrichment
    /// so that stale flags from previous runs are removed.
    pub fn clear_decision_flags(&self) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE actions SET needs_decision = 0 WHERE needs_decision = 1",
            [],
        )?;
        Ok(())
    }


    /// Helper: map a row to `DbAction`. Reduces repetition across queries.
    ///
    /// Maps the standard 17-column action SELECT. The `next_meeting_title` and
    /// `next_meeting_start` fields default to `None` — callers that include
    /// the correlated subqueries should overwrite them after calling this.
    pub(crate) fn map_action_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbAction> {
        Ok(DbAction {
            id: row.get(0)?,
            title: row.get(1)?,
            priority: row.get(2)?,
            status: row.get(3)?,
            created_at: row.get(4)?,
            due_date: row.get(5)?,
            completed_at: row.get(6)?,
            account_id: row.get(7)?,
            project_id: row.get(8)?,
            source_type: row.get(9)?,
            source_id: row.get(10)?,
            source_label: row.get(11)?,
            context: row.get(12)?,
            waiting_on: row.get(13)?,
            updated_at: row.get(14)?,
            person_id: row.get(15)?,
            account_name: row.get(16)?,
            next_meeting_title: None,
            next_meeting_start: None,
        })
    }

}
