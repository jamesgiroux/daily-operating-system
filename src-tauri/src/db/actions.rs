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
                    actions.action_kind,
                    (SELECT m.title FROM meeting_entities me
                     JOIN meetings m ON me.meeting_id = m.id
                     WHERE me.entity_id = actions.account_id
                       AND m.start_time >= date('now')
                       AND m.start_time < date('now', '+3 days')
                     ORDER BY m.start_time ASC LIMIT 1) AS next_meeting_title,
                    (SELECT m.start_time FROM meeting_entities me
                     JOIN meetings m ON me.meeting_id = m.id
                     WHERE me.entity_id = actions.account_id
                       AND m.start_time >= date('now')
                       AND m.start_time < date('now', '+3 days')
                     ORDER BY m.start_time ASC LIMIT 1) AS next_meeting_start,
                    actions.needs_decision,
                    actions.decision_owner,
                    actions.decision_stakes,
                    all_links.linear_identifier,
                    all_links.linear_url
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             LEFT JOIN action_linear_links all_links ON actions.id = all_links.action_id
             WHERE status IN ('unstarted', 'started')
               AND (due_date IS NULL OR due_date <= date('now', ?1 || ' days'))
             ORDER BY
               CASE WHEN due_date < date('now') THEN 0 ELSE 1 END,
               priority,
               due_date",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], |row| {
            let mut action = Self::map_action_row(row)?;
            action.next_meeting_title = row.get(18)?;
            action.next_meeting_start = row.get(19)?;
            let nd: i32 = row.get(20)?;
            action.needs_decision = nd != 0;
            action.decision_owner = row.get(21)?;
            action.decision_stakes = row.get(22)?;
            action.linear_identifier = row.get(23)?;
            action.linear_url = row.get(24)?;
            Ok(action)
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Query pending actions for focus prioritization.
    ///
    /// Includes actions with no due date so the ranker can decide feasibility.
    /// Ordered by urgency first, then priority/due date.
    pub fn get_focus_candidate_actions(&self, days_ahead: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status IN ('unstarted', 'started')
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

    /// Query suggested and pending actions for a specific account.
    pub fn get_account_actions(&self, account_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE account_id = ?1
               AND status IN ('backlog', 'unstarted')
             ORDER BY priority, due_date",
        )?;

        let rows = stmt.query_map(params![account_id], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// DOS Work-tab Phase 3: open commitments for the Commitments chapter.
    ///
    /// Returns rows where `action_kind = 'commitment'` AND status is one of
    /// (unstarted, started) — i.e. ACCEPTED only. Backlog commitments are
    /// AI-inferred suggestions that the user has not yet accepted, and live
    /// in the Suggestions chapter via `get_account_suggestions`. Accepting a
    /// suggested commitment promotes backlog → unstarted and moves the row
    /// into this chapter.
    ///
    /// Sort: status ASC (unstarted before started), then `created_at DESC` so
    /// the most recently emitted commitments lead each bucket. Includes
    /// Linear link fields.
    pub fn get_account_commitments(&self, account_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind,
                    all_links.linear_identifier, all_links.linear_url
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             LEFT JOIN action_linear_links all_links ON actions.id = all_links.action_id
             WHERE actions.account_id = ?1
               AND actions.action_kind = 'commitment'
               AND actions.status IN ('unstarted', 'started')
             ORDER BY
               CASE actions.status
                 WHEN 'unstarted' THEN 0
                 WHEN 'started' THEN 1
                 ELSE 2
               END,
               actions.created_at DESC",
        )?;

        let rows = stmt.query_map(params![account_id], |row| {
            let mut action = Self::map_action_row(row)?;
            action.linear_identifier = row.get(18)?;
            action.linear_url = row.get(19)?;
            Ok(action)
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// DOS Work-tab Phase 3: backlog suggestions for the Suggestions chapter.
    ///
    /// Returns rows with `status = 'backlog'` for the account, regardless of
    /// `action_kind`. Backlog commitments AND backlog tasks both show as
    /// suggestions until the user accepts (backlog → unstarted) or rejects.
    pub fn get_account_suggestions(&self, account_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind,
                    all_links.linear_identifier, all_links.linear_url
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             LEFT JOIN action_linear_links all_links ON actions.id = all_links.action_id
             WHERE actions.account_id = ?1
               AND actions.status = 'backlog'
             ORDER BY priority, actions.created_at DESC",
        )?;

        let rows = stmt.query_map(params![account_id], |row| {
            let mut action = Self::map_action_row(row)?;
            action.linear_identifier = row.get(18)?;
            action.linear_url = row.get(19)?;
            Ok(action)
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// DOS Work-tab Phase 3: recently completed actions for the Recently
    /// landed chapter.
    ///
    /// Returns rows with `status = 'completed'` AND `completed_at >= now - 30
    /// days` for the account. Sort: `completed_at DESC`. Capped at 20 rows.
    pub fn get_account_recently_landed(
        &self,
        account_id: &str,
    ) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind,
                    all_links.linear_identifier, all_links.linear_url
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             LEFT JOIN action_linear_links all_links ON actions.id = all_links.action_id
             WHERE actions.account_id = ?1
               AND actions.status = 'completed'
               AND actions.completed_at >= datetime('now', '-30 days')
             ORDER BY actions.completed_at DESC
             LIMIT 20",
        )?;

        let rows = stmt.query_map(params![account_id], |row| {
            let mut action = Self::map_action_row(row)?;
            action.linear_identifier = row.get(18)?;
            action.linear_url = row.get(19)?;
            Ok(action)
        })?;

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
                    acc.name AS account_name, a.action_kind
             FROM actions a
             LEFT JOIN accounts acc ON a.account_id = acc.id
             WHERE a.status IN ('unstarted', 'completed')
               AND (
                 -- Direct person assignment
                 a.person_id = ?1
                 -- OR meeting-sourced where person is primary relationship
                 OR (
                   a.source_type IN ('post_meeting', 'transcript')
                   AND a.source_id IN (
                     SELECT m.id FROM meetings m
                     LEFT JOIN meeting_attendees ma ON m.id = ma.meeting_id
                     LEFT JOIN meeting_entities me ON m.id = me.meeting_id
                     WHERE (ma.person_id = ?1
                        OR (me.entity_type = 'person' AND me.entity_id = ?1))
                        AND (
                          m.meeting_type = 'one_on_one'
                          OR (SELECT COUNT(*) FROM meeting_attendees WHERE meeting_id = m.id) = 2
                        )
                   )
                 )
               )
             ORDER BY
               CASE a.status WHEN 'unstarted' THEN 0 ELSE 1 END,
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
            "SELECT DISTINCT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             LEFT JOIN meeting_attendees ma ON m.id = ma.meeting_id
             LEFT JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE (
                   ma.person_id = ?1
                   OR (me.entity_type = 'person' AND me.entity_id = ?1)
                 )
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
            "UPDATE actions SET status = 'unstarted', completed_at = NULL, updated_at = ?1
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
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind,
                    actions.needs_decision, actions.decision_owner, actions.decision_stakes,
                    all_links.linear_identifier, all_links.linear_url
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             LEFT JOIN action_linear_links all_links ON actions.id = all_links.action_id
             WHERE actions.id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let mut action = Self::map_action_row(row)?;
            let nd: i32 = row.get(18)?;
            action.needs_decision = nd != 0;
            action.decision_owner = row.get(19)?;
            action.decision_stakes = row.get(20)?;
            action.linear_identifier = row.get(21)?;
            action.linear_url = row.get(22)?;
            Ok(action)
        })?;

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
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind,
                    all_links.linear_identifier, all_links.linear_url
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             LEFT JOIN action_linear_links all_links ON actions.id = all_links.action_id
             WHERE status = 'completed'
               AND completed_at >= datetime('now', ?1)
             ORDER BY completed_at DESC",
        )?;

        let hours_param = format!("-{} hours", since_hours);
        let rows = stmt.query_map(params![hours_param], |row| {
            let mut action = Self::map_action_row(row)?;
            action.linear_identifier = row.get(18)?;
            action.linear_url = row.get(19)?;
            Ok(action)
        })?;

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
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
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
    /// 1. **Title-based guard**:
    ///    - For transcript/post-meeting actions, dedup within the same source
    ///      (`source_type` + `source_id`) so one meeting doesn't suppress another.
    ///    - For all other actions, dedup by title + account across sources.
    /// 2. **ID-based guard**: If an action with this exact ID is already completed, skip.
    ///
    /// This ensures that daily briefing syncs don't resurrect completed actions (I23).
    pub fn upsert_action_if_not_completed_with_status(
        &self,
        action: &DbAction,
    ) -> Result<bool, DbError> {
        // Guard 0: Rejection pattern suppression (DOS-18).
        // Check before dedup so previously rejected patterns are caught early.
        if self.is_action_suppressed(
            &action.title,
            action.account_id.as_deref(),
            action.source_type.as_deref(),
        ) {
            return Ok(false);
        }

        let is_meeting_scoped_source = matches!(
            action.source_type.as_deref(),
            Some("transcript") | Some("post_meeting")
        ) && action
            .source_id
            .as_deref()
            .is_some_and(|source_id| !source_id.trim().is_empty());

        // Guard 1: Title-based dedup.
        // Meeting-scoped sources dedup per meeting/source; all others dedup by title+account.
        let title_exists = if is_meeting_scoped_source {
            self.conn
                .query_row(
                    "SELECT 1 FROM actions
                     WHERE LOWER(TRIM(title)) = LOWER(TRIM(?1))
                       AND source_type = ?2
                       AND source_id = ?3
                     LIMIT 1",
                    params![action.title, action.source_type, action.source_id],
                    |_row| Ok(true),
                )
                .unwrap_or(false)
        } else {
            self.conn
                .query_row(
                    "SELECT 1 FROM actions
                     WHERE LOWER(TRIM(title)) = LOWER(TRIM(?1))
                       AND (account_id = ?2 OR (?2 IS NULL AND account_id IS NULL))
                     LIMIT 1",
                    params![action.title, action.account_id],
                    |_row| Ok(true),
                )
                .unwrap_or(false)
        };

        if title_exists {
            log::debug!(
                "Action dedup: '{}' already exists for source {}:{}",
                action.title,
                action.source_type.as_deref().unwrap_or("none"),
                action.source_id.as_deref().unwrap_or("none")
            );
            return Ok(false);
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

        // Don't overwrite completed, cancelled, or archived actions (DOS-55 dedup guard)
        if matches!(
            existing_status.as_deref(),
            Some("completed") | Some("cancelled") | Some("archived")
        ) {
            return Ok(false);
        }

        self.upsert_action(action)?;
        Ok(true)
    }

    pub fn upsert_action_if_not_completed(&self, action: &DbAction) -> Result<(), DbError> {
        self.upsert_action_if_not_completed_with_status(action)
            .map(|_| ())
    }

    /// Insert or update an action. Uses SQLite `ON CONFLICT` (upsert).
    pub fn upsert_action(&self, action: &DbAction) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO actions (
                id, title, priority, status, created_at, due_date, completed_at,
                account_id, project_id, source_type, source_id, source_label,
                context, waiting_on, updated_at, person_id, action_kind
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
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
                person_id = excluded.person_id,
                action_kind = excluded.action_kind",
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
                action.action_kind,
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
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'unstarted'
               AND source_type IN ('post_meeting', 'inbox', 'ai-inbox', 'transcript', 'import', 'manual', 'user_manual', 'intelligence')
             ORDER BY priority, created_at DESC",
        )?;

        let rows = stmt.query_map([], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get counts of pending actions by priority (I513: for DB-built WeekOverview).
    ///
    /// Returns (total_pending, p1_count, p2_count, overdue_count).
    pub fn get_pending_action_counts(&self) -> Result<(i64, i64, i64, i64), DbError> {
        let total: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE status = 'unstarted'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let p1: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE status = 'unstarted' AND priority = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let p2: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE status = 'unstarted' AND priority <= 2",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let overdue: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE status = 'unstarted' AND due_date < date('now')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok((total, p1, p2, overdue))
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
    // Suggested Actions (I256)
    // =========================================================================

    /// Get all suggested actions (no owner filter).
    ///
    /// Prefer `get_suggested_actions_for_user` in production — this variant
    /// returns every backlog row regardless of who owns it, which on a real
    /// workspace drowns the user in other people's commitments (AI
    /// extraction tags every speaker in transcripts as a potential owner).
    /// Kept for devtools, tests, and explicit "show everyone's" frontend
    /// toggles.
    pub fn get_suggested_actions(&self) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'backlog'
             ORDER BY priority, created_at DESC",
        )?;

        let rows = stmt.query_map([], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get suggested actions scoped to the current user + unassigned rows.
    ///
    /// AI extraction populates `context` with an `"owner: <name>"` prefix
    /// for every commitment it finds in a transcript — including ones where
    /// the user is a passive observer, not the owner. On a real workspace
    /// this produces hundreds of "suggested actions" that belong to other
    /// people (other TAMs, customer stakeholders, internal peers). The user
    /// can't realistically triage that firehose.
    ///
    /// This variant filters to rows whose `context` either:
    ///   - starts with `"owner: <user_name>"` (case-insensitive), or
    ///   - has no recognisable owner prefix (ambiguous — still worth
    ///     surfacing for triage)
    ///
    /// If `user_name` is None or empty, falls back to no filter (safer than
    /// showing nothing when identity isn't configured).
    pub fn get_suggested_actions_for_user(
        &self,
        user_name: Option<&str>,
    ) -> Result<Vec<DbAction>, DbError> {
        let trimmed = user_name.map(|n| n.trim()).filter(|n| !n.is_empty());
        let Some(name) = trimmed else {
            return self.get_suggested_actions();
        };

        // LIKE pattern: match "owner: <name>" prefix, case-insensitive via LOWER().
        // Unassigned rows (context NULL / empty / no owner prefix) are included
        // so the user doesn't miss ambiguous work that still needs triage.
        let owner_prefix = format!("owner: {}", name.to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'backlog'
               AND (
                     context IS NULL
                  OR context = ''
                  OR LOWER(context) NOT LIKE 'owner:%'
                  OR LOWER(context) LIKE ?1 || '%'
               )
             ORDER BY priority, created_at DESC",
        )?;

        let rows = stmt.query_map([owner_prefix.as_str()], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Accept a suggested action, moving it to pending status.
    pub fn accept_suggested_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'unstarted', updated_at = ?1
             WHERE id = ?2 AND status = 'backlog'",
            params![now, id],
        )?;
        if changed == 0 {
            return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }
        Ok(())
    }

    /// Reject a suggested action by archiving it and recording the rejection signal.
    pub fn reject_suggested_action(&self, id: &str) -> Result<(), DbError> {
        self.reject_suggested_action_with_source(id, "unknown")
    }

    /// Reject a suggested action, recording the source surface for correction learning.
    pub fn reject_suggested_action_with_source(
        &self,
        id: &str,
        source: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1,
             rejected_at = ?1, rejection_source = ?3
             WHERE id = ?2 AND status = 'backlog'",
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
    ///
    /// DOS-12 zero-guilt exemptions: P1/Urgent (priority=1), waiting_on set,
    /// or objective-linked actions are never auto-archived.
    pub fn archive_stale_actions(&self, days: i64) -> Result<usize, DbError> {
        let now = Utc::now().to_rfc3339();
        let cutoff_param = format!("-{} days", days);
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1
             WHERE status = 'unstarted'
               AND completed_at IS NULL
               AND priority > 1
               AND waiting_on IS NULL
               AND id NOT IN (SELECT action_id FROM action_objective_links)
               AND (
                   (due_date IS NOT NULL AND due_date <= date('now', ?2))
                   OR
                   (due_date IS NULL AND created_at < datetime('now', ?2))
               )",
            params![now, cutoff_param],
        )?;
        Ok(changed)
    }

    /// Auto-archive suggested actions older than N days.
    /// Returns the number of actions archived.
    pub fn auto_archive_old_suggested(&self, days: i64) -> Result<usize, DbError> {
        let now = Utc::now().to_rfc3339();
        let cutoff_param = format!("-{} days", days);
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1
             WHERE status = 'backlog'
               AND created_at < datetime('now', ?2)",
            params![now, cutoff_param],
        )?;
        Ok(changed)
    }

    /// Query actions captured from a transcript or post-meeting flow for a specific meeting.
    pub fn get_actions_for_meeting(&self, meeting_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE source_id = ?1
               AND source_type IN ('transcript', 'post_meeting')
             ORDER BY priority, created_at",
        )?;

        let rows = stmt.query_map(params![meeting_id], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Query actions by source type + source IDs.
    pub fn get_actions_by_source_type_and_ids(
        &self,
        source_type: &str,
        source_ids: &[String],
    ) -> Result<Vec<DbAction>, DbError> {
        if source_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = (2..=(source_ids.len() + 1))
            .map(|i| format!("?{i}"))
            .collect();
        let sql = format!(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE actions.source_type = ?1
               AND actions.source_id IN ({})
             ORDER BY created_at DESC",
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let mut params_vec: Vec<&dyn rusqlite::types::ToSql> =
            Vec::with_capacity(source_ids.len() + 1);
        params_vec.push(&source_type);
        for source_id in source_ids {
            params_vec.push(source_id as &dyn rusqlite::types::ToSql);
        }

        let rows = stmt.query_map(params_vec.as_slice(), Self::map_action_row)?;
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

    /// Get pending actions with `waiting_on` set that are older than `stale_days`.
    ///
    /// These represent stale delegations — things handed off to someone else
    /// that haven't been resolved. Ordered by staleness (oldest first).
    pub fn get_stale_delegations(&self, stale_days: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE status = 'unstarted'
               AND waiting_on IS NOT NULL
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
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE needs_decision = 1
               AND status = 'unstarted'
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
    /// Archived accounts are excluded (DOS-286).
    pub fn get_renewal_alerts(&self, days_ahead: i32) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, account_type, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE archived = 0
               AND contract_end IS NOT NULL
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
    /// Archived accounts are excluded (DOS-286).
    pub fn get_stale_accounts(&self, stale_days: i32) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, account_type, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE archived = 0
               AND updated_at <= datetime('now', ?1 || ' days')
             ORDER BY updated_at ASC",
        )?;

        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Check whether an entity is archived (DOS-286).
    ///
    /// Returns `true` only if the entity exists and is archived. Unknown entities
    /// or unsupported entity types return `false` — callers should not treat a
    /// missing entity as archived. Used by enrichment enqueue paths to avoid
    /// spending AI budget on accounts the user has explicitly archived.
    pub fn is_entity_archived(&self, entity_id: &str, entity_type: &str) -> bool {
        let table = match entity_type {
            "account" => "accounts",
            "person" => "people",
            "project" => "projects",
            _ => return false,
        };
        let query = format!("SELECT archived FROM {table} WHERE id = ?1");
        self.conn
            .query_row(&query, params![entity_id], |row| row.get::<_, bool>(0))
            .unwrap_or(false)
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

    /// Resolve a decision: clear needs_decision flag (DOS-17).
    pub fn resolve_decision(&self, id: &str) -> Result<bool, DbError> {
        let rows = self.conn.execute(
            "UPDATE actions SET needs_decision = 0 WHERE id = ?1 AND needs_decision = 1",
            params![id],
        )?;
        Ok(rows > 0)
    }

    /// Scan unstarted actions for decision-indicating keywords and flag matches (DOS-17).
    ///
    /// Returns the number of actions newly flagged.
    pub fn scan_and_flag_decisions(&self) -> Result<usize, DbError> {
        let keywords = [
            "approval",
            "decision",
            "sign-off",
            "pending review",
            "blocked on",
            "needs alignment",
            "budget",
            "legal",
            "escalat",
        ];

        // Build a WHERE clause that checks title and context for each keyword
        let like_clauses: Vec<String> = keywords
            .iter()
            .flat_map(|kw| {
                vec![
                    format!("LOWER(title) LIKE '%{}%'", kw),
                    format!("LOWER(context) LIKE '%{}%'", kw),
                ]
            })
            .collect();
        let where_clause = like_clauses.join(" OR ");

        let sql = format!(
            "UPDATE actions SET needs_decision = 1
             WHERE status IN ('backlog', 'unstarted')
               AND needs_decision = 0
               AND ({})",
            where_clause
        );

        let rows = self.conn.execute(&sql, [])?;
        Ok(rows)
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
            action_kind: row.get(17)?,
            next_meeting_title: None,
            next_meeting_start: None,
            needs_decision: false,
            decision_owner: None,
            decision_stakes: None,
            linear_identifier: None,
            linear_url: None,
        })
    }

    // =========================================================================
    // Rejection Pattern Learning (DOS-18)
    // =========================================================================

    /// Check whether a proposed action should be suppressed based on rejection patterns.
    ///
    /// Checks three pattern types in order:
    /// 1. `exact_title` — normalized title matches a previously rejected title for this account
    /// 2. `source_fatigue` — the source type has a high rejection rate for this account
    /// 3. `keyword` — significant keywords from the title have been repeatedly rejected
    pub fn is_action_suppressed(
        &self,
        title: &str,
        account_id: Option<&str>,
        source_type: Option<&str>,
    ) -> bool {
        let normalized = title.to_lowercase().trim().to_string();

        // Check exact_title suppression
        let exact_match = self
            .conn
            .query_row(
                "SELECT 1 FROM rejected_action_patterns
                 WHERE pattern_type = 'exact_title'
                   AND suppressed = 1
                   AND pattern_value = ?1
                   AND (account_id = ?2 OR (?2 IS NULL AND account_id IS NULL))
                 LIMIT 1",
                params![normalized, account_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if exact_match {
            log::debug!(
                "Action suppressed by rejection pattern: exact_title match '{}'",
                normalized
            );
            return true;
        }

        // Check source_fatigue suppression
        if let Some(src) = source_type {
            let fatigue = self
                .conn
                .query_row(
                    "SELECT 1 FROM rejected_action_patterns
                     WHERE pattern_type = 'source_fatigue'
                       AND suppressed = 1
                       AND pattern_value = ?1
                       AND (account_id = ?2 OR (?2 IS NULL AND account_id IS NULL))
                     LIMIT 1",
                    params![src, account_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if fatigue {
                log::debug!(
                    "Action suppressed by rejection pattern: source_fatigue for '{}'",
                    src
                );
                return true;
            }
        }

        // Check keyword suppression
        let keywords = extract_significant_keywords(&normalized);
        for kw in &keywords {
            let kw_match = self
                .conn
                .query_row(
                    "SELECT 1 FROM rejected_action_patterns
                     WHERE pattern_type = 'keyword'
                       AND suppressed = 1
                       AND pattern_value = ?1
                       AND (account_id = ?2 OR (?2 IS NULL AND account_id IS NULL))
                     LIMIT 1",
                    params![kw, account_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if kw_match {
                log::debug!("Action suppressed by rejection pattern: keyword '{}'", kw);
                return true;
            }
        }

        false
    }

    /// Record rejection patterns from a rejected action (DOS-18).
    ///
    /// Called by the service layer after a successful rejection. Records:
    /// - `exact_title`: always suppressed after first rejection
    /// - `source_fatigue`: suppressed when >70% of source's actions for this account are rejected
    /// - `keyword`: suppressed when 3+ actions with the keyword have been rejected
    pub fn record_rejection_pattern(&self, action: &DbAction) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let normalized_title = action.title.to_lowercase().trim().to_string();

        // (a) Exact title suppression — always suppressed on first rejection
        self.upsert_rejection_pattern(
            action.account_id.as_deref(),
            "exact_title",
            &normalized_title,
            1,
            &now,
        )?;

        // (b) Source fatigue — check rejection rate for this source+account over 30 days
        if let Some(ref source) = action.source_type {
            let stats: Option<(i64, i64)> = self
                .conn
                .query_row(
                    "SELECT
                        COUNT(*) as total,
                        SUM(CASE WHEN status = 'archived' AND rejected_at IS NOT NULL THEN 1 ELSE 0 END) as rejected
                     FROM actions
                     WHERE source_type = ?1
                       AND (account_id = ?2 OR (?2 IS NULL AND account_id IS NULL))
                       AND created_at >= datetime('now', '-30 days')",
                    params![source, action.account_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((total, rejected)) = stats {
                if total > 0 && (rejected as f64 / total as f64) > 0.7 {
                    self.upsert_rejection_pattern(
                        action.account_id.as_deref(),
                        "source_fatigue",
                        source,
                        1,
                        &now,
                    )?;
                }
            }
        }

        // (c) Keyword suppression — check each keyword for 3+ rejections
        let keywords = extract_significant_keywords(&normalized_title);
        for kw in &keywords {
            let rejected_count: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(DISTINCT a.id) FROM actions a
                     WHERE LOWER(a.title) LIKE '%' || ?1 || '%'
                       AND a.status = 'archived'
                       AND a.rejected_at IS NOT NULL
                       AND (a.account_id = ?2 OR (?2 IS NULL AND a.account_id IS NULL))",
                    params![kw, action.account_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if rejected_count >= 3 {
                self.upsert_rejection_pattern(
                    action.account_id.as_deref(),
                    "keyword",
                    kw,
                    rejected_count,
                    &now,
                )?;
            }
        }

        Ok(())
    }

    /// Upsert a rejection pattern, handling NULL account_id correctly.
    ///
    /// SQLite treats NULL as distinct in unique indexes, so we use an explicit
    /// check-then-insert/update approach instead of ON CONFLICT.
    fn upsert_rejection_pattern(
        &self,
        account_id: Option<&str>,
        pattern_type: &str,
        pattern_value: &str,
        count: i64,
        now: &str,
    ) -> Result<(), DbError> {
        let existing_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM rejected_action_patterns
                 WHERE pattern_type = ?1
                   AND pattern_value = ?2
                   AND (account_id = ?3 OR (?3 IS NULL AND account_id IS NULL))
                 LIMIT 1",
                params![pattern_type, pattern_value, account_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing_id {
            self.conn.execute(
                "UPDATE rejected_action_patterns
                 SET rejection_count = rejection_count + 1,
                     last_rejected_at = ?1,
                     suppressed = 1
                 WHERE id = ?2",
                params![now, id],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO rejected_action_patterns
                    (account_id, pattern_type, pattern_value, rejection_count,
                     first_rejected_at, last_rejected_at, suppressed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)",
                params![account_id, pattern_type, pattern_value, count, now],
            )?;
        }

        Ok(())
    }
}

/// Stop words filtered out during keyword extraction for rejection pattern matching.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "to", "for", "with", "and", "or", "is", "in", "on", "at", "of", "by", "be",
    "do", "it", "if", "no", "so", "up", "as", "my", "we", "he", "she", "me", "am", "are", "was",
    "has", "had", "not", "but", "can", "all", "its", "our", "this", "that", "will", "from", "they",
    "been", "have", "their", "what", "when", "make", "like", "just", "get", "into", "also", "than",
    "them", "then", "some", "her", "him", "his", "how", "out", "who",
];

/// Extract significant keywords from an action title for rejection pattern matching.
///
/// Normalizes to lowercase, splits on whitespace, and filters out stop words
/// and very short tokens (<=2 chars).
fn extract_significant_keywords(normalized_title: &str) -> Vec<String> {
    normalized_title
        .split_whitespace()
        .filter(|w| w.len() > 2 && !STOP_WORDS.contains(w))
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::db::test_utils::test_db;
    use crate::db::types::DbAction;
    use chrono::Utc;
    use rusqlite::params;

    #[test]
    fn archive_stale_actions_archives_past_due_pending_actions() {
        let db = test_db();
        db.conn
            .execute(
                "INSERT INTO actions (
                    id, title, priority, status, created_at, due_date, updated_at
                 ) VALUES (
                    ?1, ?2, 3, 'unstarted', datetime('now'), date('now', '-40 days'), datetime('now')
                 )",
                params!["action-past-due", "Old follow-up"],
            )
            .expect("insert pending action");

        let archived = db.archive_stale_actions(30).expect("archive stale actions");
        assert_eq!(archived, 1);

        let status: String = db
            .conn
            .query_row(
                "SELECT status FROM actions WHERE id = ?1",
                params!["action-past-due"],
                |row| row.get(0),
            )
            .expect("read action");
        assert_eq!(status, "archived");
    }

    #[test]
    fn archive_stale_actions_archives_old_undated_pending_actions() {
        let db = test_db();
        db.conn
            .execute(
                "INSERT INTO actions (
                    id, title, priority, status, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, 3, 'unstarted', datetime('now', '-40 days'), datetime('now', '-40 days')
                 )",
                params!["action-undated", "Eventually follow up"],
            )
            .expect("insert undated action");

        let archived = db.archive_stale_actions(30).expect("archive stale actions");
        assert_eq!(archived, 1);

        let status: String = db
            .conn
            .query_row(
                "SELECT status FROM actions WHERE id = ?1",
                params!["action-undated"],
                |row| row.get(0),
            )
            .expect("read action");
        assert_eq!(status, "archived");
    }

    #[test]
    fn archive_stale_actions_keeps_recent_pending_actions_active() {
        let db = test_db();
        db.conn
            .execute(
                "INSERT INTO actions (
                    id, title, priority, status, created_at, due_date, updated_at
                 ) VALUES (
                    ?1, ?2, 3, 'unstarted', datetime('now', '-5 days'), date('now', '+2 days'), datetime('now', '-5 days')
                 )",
                params!["action-fresh", "Upcoming follow-up"],
            )
            .expect("insert fresh action");

        let archived = db.archive_stale_actions(30).expect("archive stale actions");
        assert_eq!(archived, 0);

        let status: String = db
            .conn
            .query_row(
                "SELECT status FROM actions WHERE id = ?1",
                params!["action-fresh"],
                |row| row.get(0),
            )
            .expect("read action");
        assert_eq!(status, "unstarted");
    }

    #[test]
    fn reject_suggested_action_with_source_persists_surface() {
        let db = test_db();
        db.conn
            .execute(
                "INSERT INTO actions (
                    id, title, priority, status, created_at, updated_at
                 ) VALUES (?1, ?2, 3, 'backlog', datetime('now'), datetime('now'))",
                params!["action-1", "Follow up"],
            )
            .expect("insert action");

        db.reject_suggested_action_with_source("action-1", "daily_briefing")
            .expect("reject action");

        let row: (String, Option<String>) = db
            .conn
            .query_row(
                "SELECT status, rejection_source FROM actions WHERE id = ?1",
                params!["action-1"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read action");

        assert_eq!(row.0, "archived");
        assert_eq!(row.1.as_deref(), Some("daily_briefing"));
    }

    #[test]
    fn record_rejection_pattern_creates_exact_title_entry() {
        let db = test_db();
        let action = DbAction {
            id: "a1".into(),
            title: "  Schedule Weekly Check-In  ".into(),
            priority: 3,
            status: "archived".into(),
            created_at: Utc::now().to_rfc3339(),
            due_date: None,
            completed_at: None,
            account_id: Some("acct-1".into()),
            project_id: None,
            source_type: Some("briefing".into()),
            source_id: None,
            source_label: None,
            action_kind: crate::action_status::KIND_TASK.to_string(),
            context: None,
            waiting_on: None,
            updated_at: Utc::now().to_rfc3339(),
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
            needs_decision: false,
            decision_owner: None,
            decision_stakes: None,
            linear_identifier: None,
            linear_url: None,
        };

        db.record_rejection_pattern(&action)
            .expect("record rejection");

        let count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM rejected_action_patterns
                 WHERE pattern_type = 'exact_title'
                   AND pattern_value = 'schedule weekly check-in'
                   AND account_id = 'acct-1'
                   AND suppressed = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "exact_title pattern should be created");
    }

    #[test]
    fn is_action_suppressed_blocks_exact_title_match() {
        let db = test_db();

        // Seed a suppressed pattern
        db.conn
            .execute(
                "INSERT INTO rejected_action_patterns
                    (account_id, pattern_type, pattern_value, rejection_count,
                     first_rejected_at, last_rejected_at, suppressed)
                 VALUES ('acct-1', 'exact_title', 'check in with team', 1,
                         datetime('now'), datetime('now'), 1)",
                [],
            )
            .expect("seed pattern");

        assert!(
            db.is_action_suppressed("Check In With Team", Some("acct-1"), None),
            "should suppress exact title match"
        );
        assert!(
            !db.is_action_suppressed("Check In With Team", Some("acct-other"), None),
            "should not suppress for different account"
        );
        assert!(
            !db.is_action_suppressed("Something Else", Some("acct-1"), None),
            "should not suppress unrelated title"
        );
    }

    #[test]
    fn is_action_suppressed_blocks_source_fatigue() {
        let db = test_db();

        db.conn
            .execute(
                "INSERT INTO rejected_action_patterns
                    (account_id, pattern_type, pattern_value, rejection_count,
                     first_rejected_at, last_rejected_at, suppressed)
                 VALUES ('acct-1', 'source_fatigue', 'briefing', 5,
                         datetime('now'), datetime('now'), 1)",
                [],
            )
            .expect("seed pattern");

        assert!(
            db.is_action_suppressed("Any new action", Some("acct-1"), Some("briefing")),
            "should suppress fatigued source"
        );
        assert!(
            !db.is_action_suppressed("Any new action", Some("acct-1"), Some("transcript")),
            "should not suppress different source"
        );
    }

    #[test]
    fn is_action_suppressed_blocks_keyword_match() {
        let db = test_db();

        db.conn
            .execute(
                "INSERT INTO rejected_action_patterns
                    (account_id, pattern_type, pattern_value, rejection_count,
                     first_rejected_at, last_rejected_at, suppressed)
                 VALUES ('acct-1', 'keyword', 'quarterly', 4,
                         datetime('now'), datetime('now'), 1)",
                [],
            )
            .expect("seed pattern");

        assert!(
            db.is_action_suppressed("Prepare quarterly review", Some("acct-1"), None),
            "should suppress keyword match"
        );
        assert!(
            !db.is_action_suppressed("Prepare weekly review", Some("acct-1"), None),
            "should not suppress without matching keyword"
        );
    }

    #[test]
    fn extract_significant_keywords_filters_stop_words() {
        let keywords = super::extract_significant_keywords("follow up with the team for a review");
        assert!(keywords.contains(&"follow".to_string()));
        assert!(keywords.contains(&"team".to_string()));
        assert!(keywords.contains(&"review".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"for".to_string()));
        // "up" is 2 chars, filtered by length
        assert!(!keywords.contains(&"up".to_string()));
    }

    #[test]
    fn upsert_rejection_pattern_increments_on_duplicate() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        db.upsert_rejection_pattern(Some("acct-1"), "exact_title", "test action", 1, &now)
            .expect("first upsert");
        db.upsert_rejection_pattern(Some("acct-1"), "exact_title", "test action", 1, &now)
            .expect("second upsert");

        let count: i64 = db
            .conn
            .query_row(
                "SELECT rejection_count FROM rejected_action_patterns
                 WHERE account_id = 'acct-1' AND pattern_type = 'exact_title'
                   AND pattern_value = 'test action'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "rejection_count should be incremented");
    }

    /// Regression: get_suggested_actions_for_user must scope to the user's own
    /// commitments + unassigned rows. Without this filter a real workspace
    /// returns 355 rows of which ~90% are other people's commitments.
    #[test]
    fn suggested_actions_for_user_filters_by_owner_prefix() {
        let db = test_db();

        let seeds = [
            ("a-mine", "Send the deck", "owner: James Giroux (TAM)"),
            ("b-mine-case", "Schedule sync", "owner: james giroux"),
            ("c-other", "Prep contract", "owner: Renan Basteris (Account Manager)"),
            ("d-other", "Migration plan", "owner: Sean Langlands"),
            ("e-unassigned", "Follow up on Q", ""),
            ("f-unassigned-null", "Review transcript", ""),
            (
                "g-unassigned-prose",
                "Coordinate with team",
                "Internal coordination without owner prefix",
            ),
        ];
        for (id, title, ctx) in seeds {
            db.conn
                .execute(
                    "INSERT INTO actions (id, title, priority, status, created_at, updated_at, context, action_kind)
                     VALUES (?1, ?2, 2, 'backlog', datetime('now'), datetime('now'), ?3, 'commitment')",
                    params![id, title, if ctx.is_empty() { None } else { Some(ctx) }],
                )
                .expect("insert seed");
        }

        // Mine + unassigned only.
        let scoped = db
            .get_suggested_actions_for_user(Some("James Giroux"))
            .expect("scoped");
        let ids: std::collections::HashSet<_> = scoped.iter().map(|a| a.id.clone()).collect();
        assert!(ids.contains("a-mine"), "expected a-mine in scoped set");
        assert!(ids.contains("b-mine-case"), "case-insensitive owner match");
        assert!(
            ids.contains("e-unassigned") && ids.contains("f-unassigned-null"),
            "empty-context rows must surface for triage"
        );
        assert!(
            ids.contains("g-unassigned-prose"),
            "rows without owner: prefix must surface"
        );
        assert!(!ids.contains("c-other"), "other-owner rows must be filtered out");
        assert!(!ids.contains("d-other"), "other-owner rows must be filtered out");

        // Unscoped fallback (None) = full list, same as get_suggested_actions().
        let unscoped = db.get_suggested_actions_for_user(None).expect("unscoped");
        assert_eq!(
            unscoped.len(),
            7,
            "None user_name should return all seeded rows"
        );

        // Empty user_name also falls back to full list (defensive).
        let empty = db.get_suggested_actions_for_user(Some("   ")).expect("empty");
        assert_eq!(empty.len(), 7, "whitespace-only name falls back to full list");
    }
}
