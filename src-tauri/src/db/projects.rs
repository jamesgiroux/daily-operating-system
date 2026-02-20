use super::*;

impl ActionDb {
    // =========================================================================
    // Projects (I50)
    // =========================================================================

    /// Helper: map a row to `DbProject`.
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
            updated_at: row.get(7)?,
            archived: row.get::<_, i32>(8).unwrap_or(0) != 0,
            keywords: row.get(9).unwrap_or(None),
            keywords_extracted_at: row.get(10).unwrap_or(None),
            metadata: row.get(11).unwrap_or(None),
        })
    }

    /// Insert or update a project. Also mirrors to the `entities` table.
    pub fn upsert_project(&self, project: &DbProject) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO projects (
                id, name, status, milestone, owner, target_date,
                tracker_path, updated_at, archived, keywords, keywords_extracted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                milestone = excluded.milestone,
                owner = excluded.owner,
                target_date = excluded.target_date,
                tracker_path = excluded.tracker_path,
                updated_at = excluded.updated_at",
            params![
                project.id,
                project.name,
                project.status,
                project.milestone,
                project.owner,
                project.target_date,
                project.tracker_path,
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
                    tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
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
                    tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
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
                    tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM projects WHERE archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a single whitelisted field on a project.
    pub fn update_project_field(&self, id: &str, field: &str, value: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
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

    /// Update keywords for a project (I305).
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

    /// Update keywords for an account (I305).
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

    /// Remove a keyword from a project's keyword list (I305 — user curation).
    pub fn remove_project_keyword(
        &self,
        project_id: &str,
        keyword: &str,
    ) -> Result<(), DbError> {
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

    /// Remove a keyword from an account's keyword list (I305 — user curation).
    pub fn remove_account_keyword(
        &self,
        account_id: &str,
        keyword: &str,
    ) -> Result<(), DbError> {
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

    /// Invalidate meeting prep data (I305 — prep invalidation on entity correction).
    /// NULLs prep columns and returns the old prep_snapshot_path for disk cleanup.
    pub fn invalidate_meeting_prep(&self, meeting_id: &str) -> Result<Option<String>, DbError> {
        let old_path: Option<String> = self
            .conn
            .query_row(
                "SELECT prep_snapshot_path FROM meetings_history WHERE id = ?1",
                params![meeting_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        self.conn.execute(
            "UPDATE meetings_history SET
                prep_context_json = NULL,
                prep_frozen_json = NULL,
                prep_frozen_at = NULL,
                prep_snapshot_path = NULL
             WHERE id = ?1",
            params![meeting_id],
        )?;

        Ok(old_path)
    }

    /// Get meetings from last N days with no entity links (I305 — hygiene detection).
    /// Returns (id, title, calendar_event_id, start_time) tuples.
    pub fn get_unlinked_meetings(
        &self,
        since: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, Option<String>, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.calendar_event_id, m.start_time
             FROM meetings_history m
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

    /// Get pending/waiting actions for a project.
    pub fn get_project_actions(&self, project_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT actions.id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, actions.updated_at, person_id, acc.name AS account_name
             FROM actions
             LEFT JOIN accounts acc ON actions.account_id = acc.id
             WHERE project_id = ?1
               AND status IN ('pending', 'waiting')
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
                    m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
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

    /// Link a meeting to a project in the meeting_entities junction table.
    pub fn link_meeting_to_project(
        &self,
        meeting_id: &str,
        project_id: &str,
    ) -> Result<(), DbError> {
        self.link_meeting_entity(meeting_id, project_id, "project")
    }

    /// Link a meeting to any entity in the junction table (I52 generic).
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

    /// Remove a meeting-entity link from the junction table.
    pub fn unlink_meeting_entity(&self, meeting_id: &str, entity_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = ?2",
            params![meeting_id, entity_id],
        )?;
        Ok(())
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
            "SELECT me.meeting_id, e.id, e.name, me.entity_type
             FROM meeting_entities me
             JOIN entities e ON e.id = me.entity_id
             WHERE me.meeting_id IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = meeting_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                LinkedEntity {
                    id: row.get(1)?,
                    name: row.get(2)?,
                    entity_type: row.get(3)?,
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
    /// external attendees to that entity via the `entity_people` junction table.
    ///
    /// Returns the number of new person-entity links created (excludes existing links).
    pub fn cascade_meeting_entity_to_people(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<usize, DbError> {
        let entity_id = account_id.or(project_id);
        let entity_id = match entity_id {
            Some(eid) => eid,
            None => return Ok(0),
        };

        // Link all external attendees of this meeting to the entity (idempotent).
        let count = self.conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
             SELECT ?1, ma.person_id, 'attendee'
             FROM meeting_attendees ma
             JOIN people p ON ma.person_id = p.id
             WHERE ma.meeting_id = ?2
               AND p.relationship = 'external'",
            params![entity_id, meeting_id],
        )?;

        Ok(count)
    }


}
