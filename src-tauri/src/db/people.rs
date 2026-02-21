use super::*;

impl ActionDb {
    // =========================================================================
    // People (I51)
    // =========================================================================

    /// Insert or update a person. Idempotent — won't overwrite manually-set fields
    /// unless the incoming data explicitly provides them.
    /// Upsert a person record. Returns true if the person was newly inserted (not updated).
    pub fn upsert_person(&self, person: &DbPerson) -> Result<bool, DbError> {
        // Check if person exists before upsert to detect new inserts
        let existed: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM people WHERE id = ?1)",
                params![person.id],
                |row| row.get(0),
            )
            .unwrap_or(true);

        self.conn.execute(
            "INSERT INTO people (
                id, email, name, organization, role, relationship, notes,
                tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             ) VALUES (?1, LOWER(?2), ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                name = COALESCE(excluded.name, people.name),
                organization = COALESCE(excluded.organization, people.organization),
                role = COALESCE(excluded.role, people.role),
                relationship = CASE
                    WHEN people.relationship = 'unknown' THEN excluded.relationship
                    ELSE people.relationship
                END,
                notes = COALESCE(excluded.notes, people.notes),
                tracker_path = COALESCE(excluded.tracker_path, people.tracker_path),
                last_seen = CASE
                    WHEN excluded.last_seen > COALESCE(people.last_seen, '') THEN excluded.last_seen
                    ELSE people.last_seen
                END,
                updated_at = excluded.updated_at",
            params![
                person.id,
                person.email,
                person.name,
                person.organization,
                person.role,
                person.relationship,
                person.notes,
                person.tracker_path,
                person.last_seen,
                person.first_seen,
                person.meeting_count,
                person.updated_at,
                person.archived as i32,
            ],
        )?;
        // Mirror to entities table (bridge pattern, like ensure_entity_for_account)
        self.ensure_entity_for_person(person)?;
        // Seed person_emails with the primary email
        self.add_person_email(&person.id, &person.email, true)?;
        Ok(!existed)
    }

    /// Mirror a person to the entities table.
    fn ensure_entity_for_person(&self, person: &DbPerson) -> Result<(), DbError> {
        let entity = crate::entity::DbEntity {
            id: person.id.clone(),
            name: person.name.clone(),
            entity_type: crate::entity::EntityType::Person,
            tracker_path: person.tracker_path.clone(),
            updated_at: person.updated_at.clone(),
        };
        self.upsert_entity(&entity)
    }

    /// Look up a person by email (case-insensitive).
    pub fn get_person_by_email(&self, email: &str) -> Result<Option<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people WHERE email = LOWER(?1)",
        )?;
        let mut rows = stmt.query_map(params![email], Self::map_person_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Look up a person by email, falling back to the `person_emails` alias table.
    ///
    /// 1. Exact match on `people.email`
    /// 2. Exact match on `person_emails.email` → join back to `people`
    pub fn get_person_by_email_or_alias(&self, email: &str) -> Result<Option<DbPerson>, DbError> {
        // Fast path: exact match on primary email
        if let Some(person) = self.get_person_by_email(email)? {
            return Ok(Some(person));
        }
        // Fallback: check person_emails alias table
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived
             FROM person_emails pe
             JOIN people p ON p.id = pe.person_id
             WHERE pe.email = LOWER(?1)
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![email], Self::map_person_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Search for a person by constructing `local_part@sibling` for each sibling domain.
    ///
    /// Returns the first match found (checks both `people.email` and `person_emails`).
    pub fn find_person_by_domain_alias(
        &self,
        email: &str,
        sibling_domains: &[String],
    ) -> Result<Option<DbPerson>, DbError> {
        let local_part = match email.rfind('@') {
            Some(pos) => &email[..pos],
            None => return Ok(None),
        };
        for domain in sibling_domains {
            let candidate = format!("{}@{}", local_part, domain);
            if let Some(person) = self.get_person_by_email_or_alias(&candidate)? {
                return Ok(Some(person));
            }
        }
        Ok(None)
    }

    /// Collect sibling domains for an email address.
    ///
    /// Uses `account_domains` to find accounts that own this email's domain,
    /// then collects all domains from those accounts. Also includes `user_domains`
    /// if this email's domain is among them. Skips personal email domains.
    pub fn get_sibling_domains_for_email(
        &self,
        email: &str,
        user_domains: &[String],
    ) -> Result<Vec<String>, DbError> {
        let domain = crate::prepare::email_classify::extract_domain(email);
        if domain.is_empty() {
            return Ok(Vec::new());
        }
        // Never alias personal email domains
        if crate::google_api::classify::PERSONAL_EMAIL_DOMAINS.contains(&domain.as_str()) {
            return Ok(Vec::new());
        }

        let mut siblings = std::collections::HashSet::new();

        // Path A: account_domains — find accounts owning this domain, collect all their domains
        let accounts = self.lookup_account_candidates_by_domain(&domain)?;
        for account in &accounts {
            let domains = self.get_account_domains(&account.id)?;
            for d in domains {
                if d != domain {
                    siblings.insert(d);
                }
            }
        }

        // Path B: user_domains — if this domain is among user's configured domains
        let user_domains_lower: Vec<String> = user_domains.iter().map(|d| d.to_lowercase()).collect();
        if user_domains_lower.contains(&domain) {
            for d in &user_domains_lower {
                if *d != domain {
                    siblings.insert(d.clone());
                }
            }
        }

        Ok(siblings.into_iter().collect())
    }

    /// Record an email alias for a person (INSERT OR IGNORE).
    pub fn add_person_email(
        &self,
        person_id: &str,
        email: &str,
        is_primary: bool,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO person_emails (person_id, email, is_primary, added_at)
             VALUES (?1, LOWER(?2), ?3, ?4)",
            params![person_id, email, is_primary as i32, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// List all known email addresses for a person.
    pub fn get_person_emails(&self, person_id: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT email FROM person_emails WHERE person_id = ?1 ORDER BY is_primary DESC, email",
        )?;
        let rows = stmt.query_map(params![person_id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get a person by ID.
    pub fn get_person(&self, id: &str) -> Result<Option<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], Self::map_person_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all people, optionally filtered by relationship.
    pub fn get_people(&self, relationship: Option<&str>) -> Result<Vec<DbPerson>, DbError> {
        let people = match relationship {
            Some(rel) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, email, name, organization, role, relationship, notes,
                            tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
                     FROM people WHERE relationship = ?1 AND archived = 0 ORDER BY name",
                )?;
                let rows = stmt.query_map(params![rel], Self::map_person_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, email, name, organization, role, relationship, notes,
                            tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
                     FROM people WHERE archived = 0 ORDER BY name",
                )?;
                let rows = stmt.query_map([], Self::map_person_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(people)
    }

    /// Get all people with pre-computed temperature/trend signals (I106).
    /// Uses a single batch query with LEFT JOIN subqueries instead of 3N individual queries.
    pub fn get_people_with_signals(
        &self,
        relationship: Option<&str>,
    ) -> Result<Vec<PersonListItem>, DbError> {
        let sql = "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                          p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at,
                          p.archived,
                          COALESCE(cnt30.c, 0) AS count_30d,
                          COALESCE(cnt90.c, 0) AS count_90d,
                          last_m.max_start,
                          acct_names.names AS account_names
                   FROM people p
                   LEFT JOIN (
                       SELECT ma.person_id, COUNT(*) AS c FROM meeting_attendees ma
                       JOIN meetings_history m ON m.id = ma.meeting_id
                       WHERE m.start_time >= date('now', '-30 days') GROUP BY ma.person_id
                   ) cnt30 ON cnt30.person_id = p.id
                   LEFT JOIN (
                       SELECT ma.person_id, COUNT(*) AS c FROM meeting_attendees ma
                       JOIN meetings_history m ON m.id = ma.meeting_id
                       WHERE m.start_time >= date('now', '-90 days') GROUP BY ma.person_id
                   ) cnt90 ON cnt90.person_id = p.id
                   LEFT JOIN (
                       SELECT ma.person_id, MAX(m.start_time) AS max_start FROM meeting_attendees ma
                       JOIN meetings_history m ON m.id = ma.meeting_id GROUP BY ma.person_id
                   ) last_m ON last_m.person_id = p.id
                   LEFT JOIN (
                       SELECT ep.person_id, GROUP_CONCAT(e.name, ', ') AS names
                       FROM entity_people ep
                       JOIN entities e ON e.id = ep.entity_id AND e.entity_type = 'account'
                       GROUP BY ep.person_id
                   ) acct_names ON acct_names.person_id = p.id
                   WHERE p.archived = 0 AND (?1 IS NULL OR p.relationship = ?1)
                   ORDER BY p.name";

        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![relationship], |row| {
            let count_30d: i32 = row.get(13)?;
            let count_90d: i32 = row.get(14)?;
            let last_meeting: Option<String> = row.get(15)?;

            let temperature = match &last_meeting {
                Some(dt) => compute_temperature(dt),
                None => "cold".to_string(),
            };
            let trend = compute_trend(count_30d, count_90d);

            Ok(PersonListItem {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                organization: row.get(3)?,
                role: row.get(4)?,
                relationship: row.get(5)?,
                notes: row.get(6)?,
                tracker_path: row.get(7)?,
                last_seen: row.get(8)?,
                first_seen: row.get(9)?,
                meeting_count: row.get(10)?,
                updated_at: row.get(11)?,
                archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
                temperature,
                trend,
                account_names: row.get(16)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get people linked to an entity (account/project).
    pub fn get_people_for_entity(&self, entity_id: &str) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived
             FROM people p
             JOIN entity_people ep ON p.id = ep.person_id
             WHERE ep.entity_id = ?1
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map(params![entity_id], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get entities linked to a person.
    pub fn get_entities_for_person(
        &self,
        person_id: &str,
    ) -> Result<Vec<crate::entity::DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, e.entity_type, e.tracker_path, e.updated_at
             FROM entities e
             JOIN entity_people ep ON e.id = ep.entity_id
             WHERE ep.person_id = ?1
             ORDER BY e.name",
        )?;
        let rows = stmt.query_map(params![person_id], |row| {
            let et: String = row.get(2)?;
            Ok(crate::entity::DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: crate::entity::EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Link a person to an entity (account/project). Idempotent.
    pub fn link_person_to_entity(
        &self,
        person_id: &str,
        entity_id: &str,
        rel: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
             VALUES (?1, ?2, ?3)",
            params![entity_id, person_id, rel],
        )?;
        Ok(())
    }

    /// Unlink a person from an entity.
    pub fn unlink_person_from_entity(
        &self,
        person_id: &str,
        entity_id: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM entity_people WHERE entity_id = ?1 AND person_id = ?2",
            params![entity_id, person_id],
        )?;
        Ok(())
    }

    /// Record that a person attended a meeting. Idempotent.
    /// Also updates `people.meeting_count` and `people.last_seen`.
    pub fn record_meeting_attendance(
        &self,
        meeting_id: &str,
        person_id: &str,
    ) -> Result<(), DbError> {
        // Insert attendance record (idempotent)
        let inserted = self.conn.execute(
            "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id)
             VALUES (?1, ?2)",
            params![meeting_id, person_id],
        )?;

        // Only update meeting_count if we actually inserted a new row
        if inserted > 0 {
            // Get the meeting's start_time to update last_seen
            let start_time: Option<String> = self
                .conn
                .query_row(
                    "SELECT start_time FROM meetings_history WHERE id = ?1",
                    params![meeting_id],
                    |row| row.get(0),
                )
                .ok();

            if let Some(ref st) = start_time {
                self.conn.execute(
                    "UPDATE people SET
                        meeting_count = meeting_count + 1,
                        last_seen = CASE
                            WHEN ?1 > COALESCE(last_seen, '') THEN ?1
                            ELSE last_seen
                        END,
                        updated_at = ?2
                     WHERE id = ?3",
                    params![st, Utc::now().to_rfc3339(), person_id],
                )?;
            }
        }
        Ok(())
    }

    /// Get people who attended a meeting.
    pub fn get_meeting_attendees(&self, meeting_id: &str) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived
             FROM people p
             JOIN meeting_attendees ma ON p.id = ma.person_id
             WHERE ma.meeting_id = ?1
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map(params![meeting_id], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get meetings a person attended, most recent first.
    pub fn get_person_meetings(
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
             ORDER BY m.start_time DESC
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

    /// Compute person-level signals (meeting frequency, temperature, trend).
    pub fn get_person_signals(&self, person_id: &str) -> Result<PersonSignals, DbError> {
        let count_30d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_attendees ma ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1
                   AND m.start_time >= date('now', '-30 days')",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let count_90d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_attendees ma ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1
                   AND m.start_time >= date('now', '-90 days')",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let last_meeting: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(m.start_time) FROM meetings_history m
                 JOIN meeting_attendees ma ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let temperature = match &last_meeting {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        };
        let trend = compute_trend(count_30d, count_90d);

        Ok(PersonSignals {
            meeting_frequency_30d: count_30d,
            meeting_frequency_90d: count_90d,
            last_meeting,
            temperature,
            trend,
        })
    }

    /// Search people by name, email, or organization.
    pub fn search_people(&self, query: &str, limit: i32) -> Result<Vec<DbPerson>, DbError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people
             WHERE name LIKE ?1 OR email LIKE ?1 OR organization LIKE ?1
             ORDER BY name
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a single whitelisted field on a person.
    pub fn update_person_field(&self, id: &str, field: &str, value: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        // Whitelist fields to prevent SQL injection
        let sql = match field {
            "name" => "UPDATE people SET name = ?1, updated_at = ?3 WHERE id = ?2",
            "notes" => "UPDATE people SET notes = ?1, updated_at = ?3 WHERE id = ?2",
            "role" => "UPDATE people SET role = ?1, updated_at = ?3 WHERE id = ?2",
            "organization" => "UPDATE people SET organization = ?1, updated_at = ?3 WHERE id = ?2",
            "relationship" => "UPDATE people SET relationship = ?1, updated_at = ?3 WHERE id = ?2",
            _ => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("Field '{}' is not updatable", field),
                )))
            }
        };
        self.conn.execute(sql, params![value, id, now])?;
        Ok(())
    }

    /// Helper: map a row to `DbPerson`.
    fn map_person_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbPerson> {
        Ok(DbPerson {
            id: row.get(0)?,
            email: row.get(1)?,
            name: row.get(2)?,
            organization: row.get(3)?,
            role: row.get(4)?,
            relationship: row.get::<_, String>(5)?,
            notes: row.get(6)?,
            tracker_path: row.get(7)?,
            last_seen: row.get(8)?,
            first_seen: row.get(9)?,
            meeting_count: row.get(10)?,
            updated_at: row.get(11)?,
            archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
            linkedin_url: None,
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: None,
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: None,
        })
    }

    // =========================================================================
    // Hygiene Gap Detection (I145 — ADR-0058)
    // =========================================================================

    /// People with email-derived names: no spaces, contains @, or single word.
    /// These likely need real names resolved from email headers or manual input.
    pub fn get_unnamed_people(&self) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people
             WHERE name NOT LIKE '% %' OR name LIKE '%@%'
             ORDER BY meeting_count DESC",
        )?;
        let rows = stmt.query_map([], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// People never classified because user_domain wasn't set at creation time.
    pub fn get_unknown_relationship_people(&self) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people
             WHERE relationship = 'unknown'
             ORDER BY meeting_count DESC",
        )?;
        let rows = stmt.query_map([], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Entities with content files in content_index but no intelligence cache row
    /// or NULL enriched_at. Returns (entity_id, entity_type) pairs.
    pub fn get_entities_without_intelligence(&self) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT ci.entity_id, ci.entity_type
             FROM content_index ci
             LEFT JOIN entity_intelligence ei ON ei.entity_id = ci.entity_id
             WHERE ei.enriched_at IS NULL OR ei.entity_id IS NULL
             ORDER BY ci.entity_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Entities where enriched_at is older than threshold AND new content exists
    /// since last enrichment. Returns (entity_id, entity_type, enriched_at) tuples.
    pub fn get_stale_entity_intelligence(
        &self,
        stale_days: i32,
    ) -> Result<Vec<(String, String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT ei.entity_id, ei.entity_type, ei.enriched_at
             FROM entity_intelligence ei
             WHERE ei.enriched_at < datetime('now', ?1 || ' days')
               AND EXISTS (
                   SELECT 1 FROM content_index ci
                   WHERE ci.entity_id = ei.entity_id
                     AND ci.modified_at > ei.enriched_at
               )
             ORDER BY ei.enriched_at ASC",
        )?;
        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Content files with no extracted summary. These can be backfilled mechanically.
    pub fn get_unsummarized_content_files(&self) -> Result<Vec<DbContentFile>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, filename, relative_path, absolute_path,
                    format, file_size, modified_at, indexed_at, extracted_at, summary,
                    embeddings_generated_at, content_type, priority
             FROM content_index
             WHERE summary IS NULL
               AND format IN ('Markdown', 'PlainText', 'Pdf', 'Docx', 'Xlsx', 'Pptx', 'Html', 'Rtf')
             ORDER BY priority DESC, modified_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DbContentFile {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                filename: row.get(3)?,
                relative_path: row.get(4)?,
                absolute_path: row.get(5)?,
                format: row.get(6)?,
                file_size: row.get(7)?,
                modified_at: row.get(8)?,
                indexed_at: row.get(9)?,
                extracted_at: row.get(10)?,
                summary: row.get(11)?,
                embeddings_generated_at: row.get(12)?,
                content_type: row.get(13)?,
                priority: row.get(14)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a person's relationship classification.
    pub fn update_person_relationship(
        &self,
        person_id: &str,
        relationship: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE people SET relationship = ?1, updated_at = ?3 WHERE id = ?2",
            params![relationship, person_id, now],
        )?;
        Ok(())
    }

    /// Recompute a person's meeting count from the meeting_attendees junction table.
    pub fn recompute_person_meeting_count(&self, person_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE people SET meeting_count = (
                SELECT COUNT(*) FROM meeting_attendees WHERE person_id = ?1
             ), updated_at = ?2
             WHERE id = ?1",
            params![person_id, now],
        )?;
        Ok(())
    }

    /// Update a person's name (for email display name resolution).
    pub fn update_person_name(&self, person_id: &str, name: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE people SET name = ?1, updated_at = ?3 WHERE id = ?2",
            params![name, person_id, now],
        )?;
        Ok(())
    }

    /// Merge two people: transfer all references from `remove_id` to `keep_id`, then delete `remove_id`.
    ///
    /// Transfers meeting attendees, entity links, and action associations.
    /// Uses INSERT OR IGNORE to handle overlapping meeting/entity links gracefully.
    /// Wrapped in a transaction for atomicity.
    pub fn merge_people(&self, keep_id: &str, remove_id: &str) -> Result<(), DbError> {
        // Verify both exist (before transaction)
        let keep = self
            .get_person(keep_id)?
            .ok_or_else(|| DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
        let _remove = self
            .get_person(remove_id)?
            .ok_or_else(|| DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        self.with_transaction(|tx| {
            // 1. Transfer meeting_attendees (INSERT OR IGNORE handles shared meetings)
            tx.conn.execute(
                "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id)
                 SELECT meeting_id, ?1 FROM meeting_attendees WHERE person_id = ?2",
                params![keep_id, remove_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "DELETE FROM meeting_attendees WHERE person_id = ?1",
                params![remove_id],
            ).map_err(|e| e.to_string())?;

            // 2. Transfer entity_people links
            tx.conn.execute(
                "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
                 SELECT entity_id, ?1, relationship_type FROM entity_people WHERE person_id = ?2",
                params![keep_id, remove_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "DELETE FROM entity_people WHERE person_id = ?1",
                params![remove_id],
            ).map_err(|e| e.to_string())?;

            // 3. Transfer actions
            tx.conn.execute(
                "UPDATE actions SET person_id = ?1 WHERE person_id = ?2",
                params![keep_id, remove_id],
            ).map_err(|e| e.to_string())?;

            // 4. Delete removed person's intelligence cache
            tx.conn.execute(
                "DELETE FROM entity_intelligence WHERE entity_id = ?1",
                params![remove_id],
            ).map_err(|e| e.to_string())?;

            // 5. Delete removed person's entity row
            tx.conn.execute(
                "DELETE FROM entities WHERE id = ?1 AND entity_type = 'person'",
                params![remove_id],
            ).map_err(|e| e.to_string())?;

            // 6. Delete removed person's content_index rows
            tx.conn.execute(
                "DELETE FROM content_index WHERE entity_id = ?1",
                params![remove_id],
            ).map_err(|e| e.to_string())?;

            // 6b. Transfer email aliases from removed person to kept person
            tx.conn.execute(
                "UPDATE OR IGNORE person_emails SET person_id = ?1 WHERE person_id = ?2",
                params![keep_id, remove_id],
            ).map_err(|e| e.to_string())?;
            // Clean up any that couldn't be transferred (duplicate email for same person)
            tx.conn.execute(
                "DELETE FROM person_emails WHERE person_id = ?1",
                params![remove_id],
            ).map_err(|e| e.to_string())?;
            // Ensure the removed person's primary email is recorded as an alias of the kept person
            tx.add_person_email(keep_id, &_remove.email, false)
                .map_err(|e| e.to_string())?;

            // 7. Delete removed person
            tx.conn
                .execute("DELETE FROM people WHERE id = ?1", params![remove_id])
                .map_err(|e| e.to_string())?;

            // 8. Recompute kept person's meeting count
            tx.recompute_person_meeting_count(keep_id)
                .map_err(|e| e.to_string())?;

            // Merge notes if the removed person had any
            if let Some(ref remove_notes) = _remove.notes {
                if !remove_notes.is_empty() {
                    let merged_notes = match &keep.notes {
                        Some(existing) if !existing.is_empty() => {
                            format!(
                                "{}\n\n--- Merged from {} ---\n{}",
                                existing, _remove.name, remove_notes
                            )
                        }
                        _ => format!("--- Merged from {} ---\n{}", _remove.name, remove_notes),
                    };
                    let now = Utc::now().to_rfc3339();
                    tx.conn.execute(
                        "UPDATE people SET notes = ?1, updated_at = ?2 WHERE id = ?3",
                        params![merged_notes, now, keep_id],
                    ).map_err(|e| e.to_string())?;
                }
            }

            Ok(())
        }).map_err(DbError::Migration)
    }

    /// Delete a person and all their references (attendance, entity links, actions, intelligence).
    /// Wrapped in a transaction for atomicity.
    pub fn delete_person(&self, person_id: &str) -> Result<(), DbError> {
        // Verify exists (before transaction)
        let _person = self
            .get_person(person_id)?
            .ok_or_else(|| DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        self.with_transaction(|tx| {
            // Cascade deletes
            tx.conn.execute(
                "DELETE FROM meeting_attendees WHERE person_id = ?1",
                params![person_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "DELETE FROM entity_people WHERE person_id = ?1",
                params![person_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "UPDATE actions SET person_id = NULL WHERE person_id = ?1",
                params![person_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "DELETE FROM entity_intelligence WHERE entity_id = ?1",
                params![person_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "DELETE FROM entities WHERE id = ?1 AND entity_type = 'person'",
                params![person_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "DELETE FROM content_index WHERE entity_id = ?1",
                params![person_id],
            ).map_err(|e| e.to_string())?;
            tx.conn.execute(
                "DELETE FROM person_emails WHERE person_id = ?1",
                params![person_id],
            ).map_err(|e| e.to_string())?;
            tx.conn
                .execute("DELETE FROM people WHERE id = ?1", params![person_id])
                .map_err(|e| e.to_string())?;

            Ok(())
        }).map_err(DbError::Migration)
    }


}
