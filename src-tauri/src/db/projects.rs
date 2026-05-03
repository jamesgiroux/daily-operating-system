use super::*;

impl ActionDb {
    // =========================================================================
    // Projects
    // =========================================================================

    /// Helper: map a row to `DbProject`.
    ///
    /// Column order: id, name, status, milestone, owner, target_date, tracker_path,
    /// parent_id, updated_at, archived, keywords, keywords_extracted_at, metadata
    pub(crate) fn map_project_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbProject> {
        Ok(DbProject {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row
                .get::<_, Option<String>>(2)?
                .unwrap_or_else(|| "active".to_string()),
            milestone: row.get(3)?,
            owner: row.get(4)?,
            target_date: row.get(5)?,
            tracker_path: row.get(6)?,
            parent_id: row.get(7)?,
            updated_at: row.get(8)?,
            archived: row.get::<_, i32>(9).unwrap_or(0) != 0,
            keywords: row.get(10).unwrap_or(None),
            keywords_extracted_at: row.get(11).unwrap_or(None),
            metadata: row.get(12).unwrap_or(None),
            // dashboard.json fields promoted to DB (migration 083)
            description: row.get(13).unwrap_or(None),
            milestones: row.get(14).unwrap_or(None),
            notes: row.get(15).unwrap_or(None),
        })
    }

    /// Insert or update a project. Also mirrors to the `entities` table.
    pub fn upsert_project(&self, project: &DbProject) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO projects (
                id, name, status, milestone, owner, target_date,
                tracker_path, parent_id, updated_at, archived, keywords, keywords_extracted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                milestone = excluded.milestone,
                owner = excluded.owner,
                target_date = excluded.target_date,
                tracker_path = excluded.tracker_path,
                parent_id = excluded.parent_id,
                updated_at = excluded.updated_at",
            params![
                project.id,
                project.name,
                project.status,
                project.milestone,
                project.owner,
                project.target_date,
                project.tracker_path,
                project.parent_id,
                project.updated_at,
                project.archived as i32,
                project.keywords,
                project.keywords_extracted_at,
            ],
        )?;
        self.ensure_entity_for_project(project)?;
        Ok(())
    }

    /// Mirror a project to the entities table (bridge pattern).
    pub fn ensure_entity_for_project(&self, project: &DbProject) -> Result<(), DbError> {
        let entity = DbEntity {
            id: project.id.clone(),
            name: project.name.clone(),
            entity_type: EntityType::Project,
            tracker_path: project.tracker_path.clone(),
            updated_at: project.updated_at.clone(),
        };
        self.upsert_entity(&entity)
    }

    /// Get a project by ID.
    pub fn get_project(&self, id: &str) -> Result<Option<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, parent_id, updated_at, archived,
                    keywords, keywords_extracted_at, metadata,
                    description, milestones, notes
             FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], Self::map_project_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get a project by name (case-insensitive).
    pub fn get_project_by_name(&self, name: &str) -> Result<Option<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, parent_id, updated_at, archived,
                    keywords, keywords_extracted_at, metadata,
                    description, milestones, notes
             FROM projects WHERE LOWER(name) = LOWER(?1)",
        )?;
        let mut rows = stmt.query_map(params![name], Self::map_project_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all projects, ordered by name.
    pub fn get_all_projects(&self) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, parent_id, updated_at, archived,
                    keywords, keywords_extracted_at, metadata,
                    description, milestones, notes
             FROM projects WHERE archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a single whitelisted field on a project.
    pub fn update_project_field(&self, id: &str, field: &str, value: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        // parent_id uses NULL for empty values (top-level projects)
        if field == "parent_id" {
            if value.is_empty() {
                self.conn.execute(
                    "UPDATE projects SET parent_id = NULL, updated_at = ?2 WHERE id = ?1",
                    params![id, now],
                )?;
            } else {
                // Prevent self-reference
                if value == id {
                    return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                        "Cannot set a project as its own parent".to_string(),
                    )));
                }
                // Prevent circular reference: check that value is not a descendant of id
                let descendants = self.get_descendant_projects(id).unwrap_or_default();
                if descendants.iter().any(|d| d.id == value) {
                    return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                        "Cannot set a descendant as parent (circular reference)".to_string(),
                    )));
                }
                self.conn.execute(
                    "UPDATE projects SET parent_id = ?1, updated_at = ?3 WHERE id = ?2",
                    params![value, id, now],
                )?;
            }
            return Ok(());
        }
        let sql = match field {
            "name" => "UPDATE projects SET name = ?1, updated_at = ?3 WHERE id = ?2",
            "status" => "UPDATE projects SET status = ?1, updated_at = ?3 WHERE id = ?2",
            "milestone" => "UPDATE projects SET milestone = ?1, updated_at = ?3 WHERE id = ?2",
            "owner" => "UPDATE projects SET owner = ?1, updated_at = ?3 WHERE id = ?2",
            "target_date" => "UPDATE projects SET target_date = ?1, updated_at = ?3 WHERE id = ?2",
            _ => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("Field '{}' is not updatable", field),
                )))
            }
        };
        self.conn.execute(sql, params![value, id, now])?;
        Ok(())
    }

    /// Get top-level projects (no parent), ordered by name.
    pub fn get_top_level_projects(&self) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, parent_id, updated_at, archived,
                    keywords, keywords_extracted_at, metadata,
                    description, milestones, notes
             FROM projects WHERE parent_id IS NULL AND archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get child projects for a parent, ordered by name.
    pub fn get_child_projects(&self, parent_id: &str) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, parent_id, updated_at, archived,
                    keywords, keywords_extracted_at, metadata,
                    description, milestones, notes
             FROM projects WHERE parent_id = ?1 AND archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map(params![parent_id], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Walk the parent_id chain to get all ancestors.
    pub fn get_project_ancestors(&self, project_id: &str) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE ancestors(id) AS (
                SELECT parent_id FROM projects WHERE id = ?1
                UNION ALL
                SELECT p.parent_id FROM projects p JOIN ancestors anc ON p.id = anc.id
                WHERE p.parent_id IS NOT NULL
            )
            SELECT id, name, status, milestone, owner, target_date,
                   tracker_path, parent_id, updated_at, archived,
                   keywords, keywords_extracted_at, metadata
            FROM projects
            WHERE id IN (SELECT id FROM ancestors WHERE id IS NOT NULL)
            ORDER BY id",
        )?;
        let rows = stmt.query_map(params![project_id], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get all descendants using recursive CTE with depth limit.
    pub fn get_descendant_projects(&self, ancestor_id: &str) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE descendants(id, depth) AS (
                SELECT id, 1 FROM projects WHERE parent_id = ?1
                UNION ALL
                SELECT p.id, d.depth + 1 FROM projects p
                JOIN descendants d ON p.parent_id = d.id
                WHERE d.depth < 10
            )
            SELECT proj.id, proj.name, proj.status, proj.milestone, proj.owner,
                   proj.target_date, proj.tracker_path, proj.parent_id,
                   proj.updated_at, proj.archived,
                   proj.keywords, proj.keywords_extracted_at, proj.metadata
            FROM projects proj
            JOIN descendants d ON proj.id = d.id
            WHERE proj.archived = 0
            ORDER BY proj.name",
        )?;
        let rows = stmt.query_map(params![ancestor_id], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Aggregate child project signals for a parent project.
    pub fn get_project_parent_aggregate(
        &self,
        parent_id: &str,
    ) -> Result<ProjectParentAggregate, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*),
                    SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'on_hold' OR status = 'on-hold' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END),
                    MIN(target_date)
             FROM projects WHERE parent_id = ?1 AND archived = 0",
        )?;
        let row = stmt.query_row(params![parent_id], |row| {
            Ok(ProjectParentAggregate {
                child_count: row.get::<_, usize>(0)?,
                active_count: row.get::<_, usize>(1).unwrap_or(0),
                on_hold_count: row.get::<_, usize>(2).unwrap_or(0),
                completed_count: row.get::<_, usize>(3).unwrap_or(0),
                nearest_target_date: row.get(4)?,
            })
        })?;
        Ok(row)
    }

    /// Update keywords for a project.
    pub fn update_project_keywords(
        &self,
        project_id: &str,
        keywords_json: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE projects SET keywords = ?1, keywords_extracted_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![keywords_json, now, project_id],
        )?;
        Ok(())
    }

    /// Update keywords for an account.
    pub fn update_account_keywords(
        &self,
        account_id: &str,
        keywords_json: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE accounts SET keywords = ?1, keywords_extracted_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![keywords_json, now, account_id],
        )?;
        Ok(())
    }

    /// Remove a keyword from a project's keyword list (user curation).
    pub fn remove_project_keyword(&self, project_id: &str, keyword: &str) -> Result<(), DbError> {
        let current: Option<String> = self
            .conn
            .query_row(
                "SELECT keywords FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        if let Some(json_str) = current {
            if let Ok(mut keywords) = serde_json::from_str::<Vec<String>>(&json_str) {
                keywords.retain(|k| k != keyword);
                let updated = serde_json::to_string(&keywords).map_err(|e| {
                    DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                })?;
                self.conn.execute(
                    "UPDATE projects SET keywords = ?1 WHERE id = ?2",
                    params![updated, project_id],
                )?;
            }
        }
        Ok(())
    }

    /// Remove a keyword from an account's keyword list (user curation).
    pub fn remove_account_keyword(&self, account_id: &str, keyword: &str) -> Result<(), DbError> {
        let current: Option<String> = self
            .conn
            .query_row(
                "SELECT keywords FROM accounts WHERE id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        if let Some(json_str) = current {
            if let Ok(mut keywords) = serde_json::from_str::<Vec<String>>(&json_str) {
                keywords.retain(|k| k != keyword);
                let updated = serde_json::to_string(&keywords).map_err(|e| {
                    DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                })?;
                self.conn.execute(
                    "UPDATE accounts SET keywords = ?1 WHERE id = ?2",
                    params![updated, account_id],
                )?;
            }
        }
        Ok(())
    }

    /// Invalidate meeting prep data (prep invalidation on entity correction).
    /// NULLs prep columns and returns the old prep_snapshot_path for disk cleanup.
    pub fn invalidate_meeting_prep(&self, meeting_id: &str) -> Result<Option<String>, DbError> {
        let old_path: Option<String> = self
            .conn
            .query_row(
                "SELECT prep_snapshot_path FROM meeting_prep WHERE meeting_id = ?1",
                params![meeting_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        self.conn.execute(
            "UPDATE meeting_prep SET
                prep_context_json = NULL,
                prep_frozen_json = NULL,
                prep_frozen_at = NULL,
                prep_snapshot_path = NULL
             WHERE meeting_id = ?1",
            params![meeting_id],
        )?;

        Ok(old_path)
    }

    /// Get meetings from last N days with no entity links (hygiene detection).
    /// Returns (id, title, calendar_event_id, start_time) tuples.
    pub fn get_unlinked_meetings(
        &self,
        since: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, Option<String>, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.calendar_event_id, m.start_time
             FROM meetings m
             LEFT JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE m.start_time >= ?1 AND me.meeting_id IS NULL
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get suggested/pending actions for a project.
    pub fn get_project_actions(&self, project_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name,
                    actions.action_kind
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE project_id = ?1
               AND status IN ('backlog', 'unstarted', 'started')
             ORDER BY priority, due_date",
        )?;
        let rows = stmt.query_map(params![project_id], Self::map_action_row)?;
        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get meetings linked to a project via the meeting_entities junction table.
    pub fn get_meetings_for_project(
        &self,
        project_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id, mt.transcript_path
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_id = ?1 AND me.entity_type = 'project'
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit], |row| {
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
                transcript_path: row.get(10)?,
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

    /// Link a meeting to a project in the meeting_entities junction table.
    pub fn link_meeting_to_project(
        &self,
        meeting_id: &str,
        project_id: &str,
    ) -> Result<(), DbError> {
        self.link_meeting_entity(meeting_id, project_id, "project")
    }

    /// Link a meeting to any entity in the junction table (generic).
    pub fn link_meeting_entity(
        &self,
        meeting_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES (?1, ?2, ?3)",
            params![meeting_id, entity_id, entity_type],
        )?;
        Ok(())
    }

    /// Link a meeting to an entity with per-junction confidence + primary flag.
    /// If the link already exists, upgrades `confidence` when the new value is
    /// higher (never downgrades — a later low-confidence sweep should not
    /// stomp a high-confidence manual link).
    ///
    /// single-primary invariant. When writing a new primary, this
    /// runs inside a transaction that FIRST demotes any existing
    /// `is_primary = 1` rows with the same `(meeting_id, entity_type)` to
    /// `is_primary = 0`, THEN upserts the new primary. The previous
    /// implementation used `is_primary = MAX(is_primary, excluded.is_primary)`
    /// which could never demote — so a later batch electing a different
    /// primary left two rows with `is_primary = 1` for the same
    /// `(meeting_id, entity_type)`, breaking the UI's single-primary
    /// assumption. When `is_primary = false`, no demotion is performed
    /// (suggestion writes cannot steal primaryhood, and we must still not
    /// downgrade an existing primary via the upsert path).
    pub fn link_meeting_entity_with_confidence(
        &self,
        meeting_id: &str,
        entity_id: &str,
        entity_type: &str,
        confidence: f64,
        is_primary: bool,
    ) -> Result<(), DbError> {
        let primary_int: i64 = if is_primary { 1 } else { 0 };

        if is_primary {
            // Transactional demote-then-upsert: guarantees exactly one
            // `is_primary = 1` row per `(meeting_id, entity_type)`.
            let tx = self.conn.unchecked_transaction()?;
            tx.execute(
                "UPDATE meeting_entities
                    SET is_primary = 0
                  WHERE meeting_id = ?1
                    AND entity_type = ?2
                    AND entity_id <> ?3
                    AND is_primary = 1",
                params![meeting_id, entity_type, entity_id],
            )?;
            tx.execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
                 VALUES (?1, ?2, ?3, ?4, 1)
                 ON CONFLICT(meeting_id, entity_id) DO UPDATE SET
                     confidence = MAX(confidence, excluded.confidence),
                     is_primary = 1",
                params![meeting_id, entity_id, entity_type, confidence],
            )?;
            tx.commit()?;
        } else {
            // Suggestion write: never promote, never demote an existing primary.
            self.conn.execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(meeting_id, entity_id) DO UPDATE SET
                     confidence = MAX(confidence, excluded.confidence)",
                params![meeting_id, entity_id, entity_type, confidence, primary_int],
            )?;
        }
        Ok(())
    }

    /// Remove a meeting-entity link from the junction table.
    pub fn unlink_meeting_entity(&self, meeting_id: &str, entity_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = ?2",
            params![meeting_id, entity_id],
        )?;
        Ok(())
    }

    /// Record that the user has dismissed an auto-resolved entity
    /// from a meeting. The dismissal persists across calendar-sync and
    /// resolver sweeps so the entity will not silently re-link on its own.
    /// The accompanying `unlink_meeting_entity` call is the caller's job —
    /// this helper only writes the dictionary entry.
    pub fn record_meeting_entity_dismissal(
        &self,
        meeting_id: &str,
        entity_id: &str,
        entity_type: &str,
        dismissed_by: Option<&str>,
    ) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO meeting_entity_dismissals
                 (meeting_id, entity_id, entity_type, dismissed_at, dismissed_by)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(meeting_id, entity_id, entity_type) DO UPDATE SET
                 dismissed_at = excluded.dismissed_at,
                 dismissed_by = excluded.dismissed_by",
            params![meeting_id, entity_id, entity_type, now, dismissed_by],
        )?;
        Ok(())
    }

    /// Remove a dismissal record so the entity can auto-link again
    /// on the next resolver pass. Used by the undo / restore flow.
    pub fn remove_meeting_entity_dismissal(
        &self,
        meeting_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<bool, DbError> {
        let removed = self.conn.execute(
            "DELETE FROM meeting_entity_dismissals
             WHERE meeting_id = ?1 AND entity_id = ?2 AND entity_type = ?3",
            params![meeting_id, entity_id, entity_type],
        )?;
        Ok(removed > 0)
    }

    /// Is the given (meeting, entity, type) currently dismissed?
    /// Used by both calendar-sync and resolver persistence paths to gate
    /// re-insertion.
    pub fn is_meeting_entity_dismissed(
        &self,
        meeting_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<bool, DbError> {
        let exists: bool = self
            .conn
            .prepare(
                "SELECT 1 FROM meeting_entity_dismissals
                 WHERE meeting_id = ?1 AND entity_id = ?2 AND entity_type = ?3",
            )?
            .exists(params![meeting_id, entity_id, entity_type])?;
        Ok(exists)
    }

    /// List all (entity_id, entity_type) pairs dismissed for a
    /// meeting. Used when persisting a batch of resolution outcomes so we
    /// can filter the whole batch in a single query rather than probing
    /// per-entity.
    pub fn list_dismissed_meeting_entities(
        &self,
        meeting_id: &str,
    ) -> Result<std::collections::HashSet<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id, entity_type FROM meeting_entity_dismissals
             WHERE meeting_id = ?1",
        )?;
        let rows = stmt.query_map(params![meeting_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut set = std::collections::HashSet::new();
        for r in rows {
            set.insert(r?);
        }
        Ok(set)
    }

    /// Get all linked entities for a meeting with confidence + primary
    /// flags, ordered by (is_primary DESC, confidence DESC, name ASC) so
    /// `[0]` is the single best primary entity and lower-confidence siblings
    /// trail as suggestions.
    pub fn get_meeting_linked_entities(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<LinkedEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, me.entity_type, me.confidence, me.is_primary
             FROM meeting_entities me
             JOIN entities e ON e.id = me.entity_id
             WHERE me.meeting_id = ?1
             ORDER BY me.is_primary DESC, me.confidence DESC, e.name ASC",
        )?;
        let rows = stmt.query_map(params![meeting_id], |row| {
            let confidence: f64 = row.get(3)?;
            let is_primary: i64 = row.get(4)?;
            let is_primary = is_primary != 0;
            let suggested = !is_primary && confidence < 0.60;
            Ok(LinkedEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                confidence,
                is_primary,
                suggested,
                role: None,
                applied_rule: None,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get all entities linked to a meeting via the junction table.
    pub fn get_meeting_entities(&self, meeting_id: &str) -> Result<Vec<DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, e.entity_type, e.tracker_path, e.updated_at
             FROM entities e
             JOIN meeting_entities me ON me.entity_id = e.id
             WHERE me.meeting_id = ?1",
        )?;
        let rows = stmt.query_map(params![meeting_id], |row| {
            let et: String = row.get(2)?;
            Ok(DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Batch query: get linked entities for multiple meetings at once.
    /// Returns a map from meeting_id → Vec<LinkedEntity>.
    ///
    /// Results are ordered by confidence DESC within each meeting so
    /// `entities[0]` is always the highest-confidence (primary) link. Low-
    /// confidence siblings (<0.60) are flagged `suggested = true` for muted
    /// UI rendering.
    pub fn get_meeting_entity_map(
        &self,
        meeting_ids: &[String],
    ) -> Result<HashMap<String, Vec<LinkedEntity>>, DbError> {
        if meeting_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders: Vec<String> = (0..meeting_ids.len())
            .map(|i| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT me.meeting_id, e.id, e.name, me.entity_type, me.confidence, me.is_primary
             FROM meeting_entities me
             JOIN entities e ON e.id = me.entity_id
             WHERE me.meeting_id IN ({})
             ORDER BY me.is_primary DESC, me.confidence DESC, e.name ASC",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = meeting_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            let meeting_id: String = row.get(0)?;
            let id: String = row.get(1)?;
            let name: String = row.get(2)?;
            let entity_type: String = row.get(3)?;
            let confidence: f64 = row.get(4)?;
            let is_primary: i64 = row.get(5)?;
            let is_primary = is_primary != 0;
            // suggestion tier is anything below the ResolvedWithFlag
            // cutoff (0.60) that is also not flagged as primary. UI paints
            // these muted with a "suggested" affordance.
            let suggested = !is_primary && confidence < 0.60;
            Ok((
                meeting_id,
                LinkedEntity {
                    id,
                    name,
                    entity_type,
                    confidence,
                    is_primary,
                    suggested,
                    role: None,
                    applied_rule: None,
                },
            ))
        })?;
        let mut map: HashMap<String, Vec<LinkedEntity>> = HashMap::new();
        for row in rows {
            let (meeting_id, entity) = row?;
            map.entry(meeting_id).or_default().push(entity);
        }
        Ok(map)
    }

    /// Clear all entity links for a given meeting.
    pub fn clear_meeting_entities(&self, meeting_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM meeting_entities WHERE meeting_id = ?1",
            params![meeting_id],
        )?;
        Ok(())
    }

    /// Cascade entity reassignment to actions linked to this meeting.
    pub fn cascade_meeting_entity_to_actions(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<usize, DbError> {
        let mut total = 0;
        total += self.conn.execute(
            "UPDATE actions SET account_id = ?1 WHERE source_id = ?2",
            params![account_id, meeting_id],
        )?;
        total += self.conn.execute(
            "UPDATE actions SET project_id = ?1 WHERE source_id = ?2",
            params![project_id, meeting_id],
        )?;
        Ok(total)
    }

    /// Cascade entity reassignment to captures linked to this meeting.
    pub fn cascade_meeting_entity_to_captures(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<usize, DbError> {
        let mut total = 0;
        total += self.conn.execute(
            "UPDATE captures SET account_id = ?1 WHERE meeting_id = ?2",
            params![account_id, meeting_id],
        )?;
        total += self.conn.execute(
            "UPDATE captures SET project_id = ?1 WHERE meeting_id = ?2",
            params![project_id, meeting_id],
        )?;
        Ok(total)
    }

    /// Cascade meeting entity links to all non-internal attendees.
    /// When a meeting is linked to an account/project, automatically link all
    /// external attendees to that entity via the appropriate junction table.
    ///
    /// Returns the number of new person-entity links created (excludes existing links).
    pub fn cascade_meeting_entity_to_people(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<usize, DbError> {
        // Route: accounts → account_stakeholders, projects → entity_members
        // Only auto-add after attendee appears in 2+ meetings with the account
        if let Some(acct_id) = account_id {
            self.conn.execute(
                "INSERT INTO account_stakeholders (account_id, person_id)
                 SELECT ?1, ma.person_id
                 FROM meeting_attendees ma
                 JOIN people p ON ma.person_id = p.id
                 WHERE ma.meeting_id = ?2
                   AND p.relationship = 'external'
                   AND (SELECT COUNT(DISTINCT ma2.meeting_id)
                        FROM meeting_attendees ma2
                        JOIN meeting_entities me2 ON me2.meeting_id = ma2.meeting_id
                        WHERE ma2.person_id = ma.person_id
                          AND me2.entity_id = ?1 AND me2.entity_type = 'account') >= 2
                 ON CONFLICT(account_id, person_id) DO NOTHING",
                params![acct_id, meeting_id],
            )?;
            let count = self.conn.execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 SELECT as_.account_id, as_.person_id, 'associated', 'google'
                 FROM account_stakeholders as_
                 WHERE as_.account_id = ?1
                 ON CONFLICT(account_id, person_id, role) DO NOTHING",
                params![acct_id],
            )?;
            return Ok(count);
        }

        if let Some(proj_id) = project_id {
            let count = self.conn.execute(
                "INSERT OR IGNORE INTO entity_members (entity_id, person_id, relationship_type)
                 SELECT ?1, ma.person_id, 'attendee'
                 FROM meeting_attendees ma
                 JOIN people p ON ma.person_id = p.id
                 WHERE ma.meeting_id = ?2
                   AND p.relationship = 'external'",
                params![proj_id, meeting_id],
            )?;
            return Ok(count);
        }

        Ok(0)
    }
}
