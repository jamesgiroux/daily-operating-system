use super::*;

impl ActionDb {
    // =========================================================================
    // Accounts
    // =========================================================================

    /// Insert or update an account. Also mirrors to the `entities` table (ADR-0045).
    pub fn upsert_account(&self, account: &DbAccount) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO accounts (
                id, name, lifecycle, arr, health, contract_start, contract_end,
                nps, tracker_path, parent_id, is_internal, updated_at, archived,
                keywords, keywords_extracted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                lifecycle = excluded.lifecycle,
                arr = excluded.arr,
                health = excluded.health,
                contract_start = excluded.contract_start,
                contract_end = excluded.contract_end,
                nps = excluded.nps,
                tracker_path = excluded.tracker_path,
                parent_id = excluded.parent_id,
                is_internal = excluded.is_internal,
                updated_at = excluded.updated_at",
            params![
                account.id,
                account.name,
                account.lifecycle,
                account.arr,
                account.health,
                account.contract_start,
                account.contract_end,
                account.nps,
                account.tracker_path,
                account.parent_id,
                account.is_internal as i32,
                account.updated_at,
                account.archived as i32,
                account.keywords,
                account.keywords_extracted_at,
            ],
        )?;
        // Keep entity mirror in sync
        self.ensure_entity_for_account(account)?;
        Ok(())
    }

    /// Touch `updated_at` on an account as a last-contact signal.
    ///
    /// Matches by ID or by case-insensitive name. Returns `true` if a row
    /// was updated, `false` if no account matched.
    pub fn touch_account_last_contact(&self, account_name: &str) -> Result<bool, DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE accounts SET updated_at = ?1
             WHERE id = ?2 OR LOWER(name) = LOWER(?2)",
            params![now, account_name],
        )?;
        Ok(rows > 0)
    }

    /// Get an account by ID.
    pub fn get_account(&self, id: &str) -> Result<Option<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], Self::map_account_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get an account by name (case-insensitive).
    pub fn get_account_by_name(&self, name: &str) -> Result<Option<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE LOWER(name) = LOWER(?1)",
        )?;

        let mut rows = stmt.query_map(params![name], Self::map_account_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all accounts, ordered by name.
    pub fn get_all_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts WHERE archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get top-level accounts (no parent), ordered by name.
    pub fn get_top_level_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts WHERE parent_id IS NULL AND archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get child accounts for a parent, ordered by name.
    pub fn get_child_accounts(&self, parent_id: &str) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts WHERE parent_id = ?1 AND archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map(params![parent_id], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Walk the parent_id chain to get all ancestors (I316: n-level nesting).
    pub fn get_account_ancestors(&self, account_id: &str) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE ancestors(id) AS (
                SELECT parent_id FROM accounts WHERE id = ?1
                UNION ALL
                SELECT a.parent_id FROM accounts a JOIN ancestors anc ON a.id = anc.id
                WHERE a.parent_id IS NOT NULL
            )
            SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                   nps, tracker_path, parent_id, is_internal, updated_at, archived,
                   keywords, keywords_extracted_at, metadata
            FROM accounts
            WHERE id IN (SELECT id FROM ancestors WHERE id IS NOT NULL)
            ORDER BY id",
        )?;
        let rows = stmt.query_map(params![account_id], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get all descendants using recursive CTE with depth limit (I316: n-level nesting).
    pub fn get_descendant_accounts(&self, ancestor_id: &str) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE descendants(id, depth) AS (
                SELECT id, 1 FROM accounts WHERE parent_id = ?1
                UNION ALL
                SELECT a.id, d.depth + 1 FROM accounts a
                JOIN descendants d ON a.parent_id = d.id
                WHERE d.depth < 10
            )
            SELECT acc.id, acc.name, acc.lifecycle, acc.arr, acc.health,
                   acc.contract_start, acc.contract_end, acc.nps, acc.tracker_path,
                   acc.parent_id, acc.is_internal, acc.updated_at, acc.archived,
                   acc.keywords, acc.keywords_extracted_at, acc.metadata
            FROM accounts acc
            JOIN descendants d ON acc.id = d.id
            WHERE acc.archived = 0
            ORDER BY acc.name",
        )?;
        let rows = stmt.query_map(params![ancestor_id], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Set domains for an account (replace-all).
    pub fn set_account_domains(&self, account_id: &str, domains: &[String]) -> Result<(), DbError> {
        let normalized = crate::helpers::normalize_domains(domains);
        self.conn.execute(
            "DELETE FROM account_domains WHERE account_id = ?1",
            params![account_id],
        )?;
        for domain in normalized {
            self.conn.execute(
                "INSERT OR IGNORE INTO account_domains (account_id, domain) VALUES (?1, ?2)",
                params![account_id, &domain],
            )?;
        }
        Ok(())
    }

    /// Get account domains for an account.
    pub fn get_account_domains(&self, account_id: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT domain FROM account_domains WHERE account_id = ?1 ORDER BY domain")?;
        let rows = stmt.query_map(params![account_id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get all accounts with their domains in a single JOIN query.
    ///
    /// Eliminates N+1 queries when callers need domains for many accounts.
    /// Returns `Vec<(DbAccount, Vec<String>)>` — each tuple is an account + its domains.
    pub fn get_all_accounts_with_domains(
        &self,
        include_archived: bool,
    ) -> Result<Vec<(DbAccount, Vec<String>)>, DbError> {
        let query = if include_archived {
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start,
                    a.contract_end, a.nps, a.tracker_path, a.parent_id, a.is_internal,
                    a.updated_at, a.archived, a.keywords, a.keywords_extracted_at, a.metadata,
                    ad.domain
             FROM accounts a
             LEFT JOIN account_domains ad ON a.id = ad.account_id
             ORDER BY a.id, ad.domain"
        } else {
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start,
                    a.contract_end, a.nps, a.tracker_path, a.parent_id, a.is_internal,
                    a.updated_at, a.archived, a.keywords, a.keywords_extracted_at, a.metadata,
                    ad.domain
             FROM accounts a
             LEFT JOIN account_domains ad ON a.id = ad.account_id
             WHERE a.archived = 0
             ORDER BY a.id, ad.domain"
        };

        let mut stmt = self.conn.prepare(query)?;
        let mut rows = stmt.query([])?;

        let mut result: Vec<(DbAccount, Vec<String>)> = Vec::new();
        let mut current_id: Option<String> = None;

        while let Some(row) = rows.next()? {
            let account_id: String = row.get(0)?;
            let domain: Option<String> = row.get(16)?;

            if current_id.as_deref() != Some(&account_id) {
                // New account — push a new entry
                let account = DbAccount {
                    id: account_id.clone(),
                    name: row.get(1)?,
                    lifecycle: row.get(2)?,
                    arr: row.get(3)?,
                    health: row.get(4)?,
                    contract_start: row.get(5)?,
                    contract_end: row.get(6)?,
                    nps: row.get(7)?,
                    tracker_path: row.get(8)?,
                    parent_id: row.get(9)?,
                    is_internal: row.get::<_, i32>(10).unwrap_or(0) != 0,
                    updated_at: row.get(11)?,
                    archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
                    keywords: row.get(13).unwrap_or(None),
                    keywords_extracted_at: row.get(14).unwrap_or(None),
                    metadata: row.get(15).unwrap_or(None),
                };
                let domains = domain.into_iter().collect();
                result.push((account, domains));
                current_id = Some(account_id);
            } else if let Some(d) = domain {
                // Same account — append domain
                if let Some(last) = result.last_mut() {
                    last.1.push(d);
                }
            }
        }

        Ok(result)
    }

    /// Lookup non-archived account candidates by email domain.
    pub fn lookup_account_candidates_by_domain(
        &self,
        domain: &str,
    ) -> Result<Vec<DbAccount>, DbError> {
        let normalized = domain.trim().to_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start, a.contract_end,
                    a.nps, a.tracker_path, a.parent_id, a.is_internal,
                    a.updated_at, a.archived
             FROM accounts a
             INNER JOIN account_domains d ON d.account_id = a.id
             WHERE d.domain = ?1
               AND a.archived = 0
             ORDER BY a.is_internal ASC, a.name ASC",
        )?;
        let rows = stmt.query_map(params![normalized], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Copy domains from parent to child (idempotent).
    pub fn copy_account_domains(&self, parent_id: &str, child_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO account_domains (account_id, domain)
             SELECT ?1, domain FROM account_domains WHERE account_id = ?2",
            params![child_id, parent_id],
        )?;
        Ok(())
    }

    /// Root internal organization account (top-level internal account).
    pub fn get_internal_root_account(&self) -> Result<Option<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE is_internal = 1 AND parent_id IS NULL AND archived = 0
             ORDER BY updated_at DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], Self::map_account_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// All active internal accounts.
    pub fn get_internal_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts
             WHERE is_internal = 1 AND archived = 0
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get account team members with person details.
    pub fn get_account_team(&self, account_id: &str) -> Result<Vec<DbAccountTeamMember>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT at.account_id, at.person_id, p.name, p.email, at.role, at.created_at
             FROM account_team at
             JOIN people p ON p.id = at.person_id
             WHERE at.account_id = ?1
             ORDER BY at.role, p.name",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountTeamMember {
                account_id: row.get(0)?,
                person_id: row.get(1)?,
                person_name: row.get(2)?,
                person_email: row.get(3)?,
                role: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Add an account team member role link (idempotent).
    pub fn add_account_team_member(
        &self,
        account_id: &str,
        person_id: &str,
        role: &str,
    ) -> Result<(), DbError> {
        let role = role.trim().to_lowercase();
        self.conn.execute(
            "INSERT OR IGNORE INTO account_team (account_id, person_id, role, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![account_id, person_id, role, Utc::now().to_rfc3339()],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
             VALUES (?1, ?2, 'associated')",
            params![account_id, person_id],
        )?;
        Ok(())
    }

    /// Remove an account team role link.
    /// If no roles remain for this person on this account, also removes the entity_people link.
    pub fn remove_account_team_member(
        &self,
        account_id: &str,
        person_id: &str,
        role: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM account_team
             WHERE account_id = ?1 AND person_id = ?2 AND LOWER(role) = LOWER(?3)",
            params![account_id, person_id, role.trim()],
        )?;

        // Clean up entity_people link if no roles remain
        let remaining_roles: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM account_team
             WHERE account_id = ?1 AND person_id = ?2",
            params![account_id, person_id],
            |row| row.get(0),
        )?;

        if remaining_roles == 0 {
            self.conn.execute(
                "DELETE FROM entity_people
                 WHERE entity_id = ?1 AND person_id = ?2",
                params![account_id, person_id],
            )?;
        }

        Ok(())
    }

    /// Import notes from migration for unmatched legacy account-team fields.
    pub fn get_account_team_import_notes(
        &self,
        account_id: &str,
    ) -> Result<Vec<DbAccountTeamImportNote>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, legacy_field, legacy_value, note, created_at
             FROM account_team_import_notes
             WHERE account_id = ?1
             ORDER BY id",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountTeamImportNote {
                id: row.get(0)?,
                account_id: row.get(1)?,
                legacy_field: row.get(2)?,
                legacy_value: row.get(3)?,
                note: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Aggregate child account signals for a parent account (I114).
    ///
    /// Returns total ARR, worst health, nearest renewal, and BU count.
    pub fn get_parent_aggregate(&self, parent_id: &str) -> Result<ParentAggregate, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(arr), 0),
                    MIN(CASE health WHEN 'red' THEN 0 WHEN 'yellow' THEN 1 WHEN 'green' THEN 2 ELSE 3 END),
                    MIN(contract_end)
             FROM accounts WHERE parent_id = ?1",
        )?;
        let row = stmt.query_row(params![parent_id], |row| {
            let bu_count: usize = row.get(0)?;
            let total_arr: f64 = row.get(1)?;
            let worst_health_int: i32 = row.get(2)?;
            let nearest_renewal: Option<String> = row.get(3)?;
            Ok(ParentAggregate {
                bu_count,
                total_arr: if total_arr > 0.0 {
                    Some(total_arr)
                } else {
                    None
                },
                worst_health: match worst_health_int {
                    0 => Some("red".to_string()),
                    1 => Some("yellow".to_string()),
                    2 => Some("green".to_string()),
                    _ => None,
                },
                nearest_renewal,
            })
        })?;
        Ok(row)
    }

    /// Get meetings for an account, most recent first.
    pub fn get_meetings_for_account(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![account_id, limit], |row| {
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

    /// Get past meetings for an account with prep context (ADR-0063).
    /// Used only on account detail page where prep preview cards are needed.
    pub fn get_meetings_for_account_with_prep(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id, m.prep_context_json
             FROM meetings_history m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![account_id, limit], |row| {
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
                prep_context_json: row.get(10)?,
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

    /// Get upcoming (future) meetings for an account, soonest first.
    pub fn get_upcoming_meetings_for_account(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
               AND m.start_time >= datetime('now')
             ORDER BY m.start_time ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![account_id, limit], |row| {
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

    /// Update a single whitelisted field on an account.
    pub fn update_account_field(&self, id: &str, field: &str, value: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        // parent_id uses NULL for empty values (top-level accounts)
        if field == "parent_id" {
            if value.is_empty() {
                self.conn.execute(
                    "UPDATE accounts SET parent_id = NULL, updated_at = ?2 WHERE id = ?1",
                    params![id, now],
                )?;
            } else {
                // Prevent self-reference
                if value == id {
                    return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                        "Cannot set an account as its own parent".to_string(),
                    )));
                }
                // Prevent circular reference: check that value is not a descendant of id
                let descendants = self.get_descendant_accounts(id).unwrap_or_default();
                if descendants.iter().any(|d| d.id == value) {
                    return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                        "Cannot set a descendant as parent (circular reference)".to_string(),
                    )));
                }
                self.conn.execute(
                    "UPDATE accounts SET parent_id = ?1, updated_at = ?3 WHERE id = ?2",
                    params![value, id, now],
                )?;

                // Propagate is_internal: if the child is internal, the parent should be too
                let child_is_internal: i32 = self.conn.query_row(
                    "SELECT is_internal FROM accounts WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                ).unwrap_or(0);
                if child_is_internal == 1 {
                    self.conn.execute(
                        "UPDATE accounts SET is_internal = 1, updated_at = ?2 WHERE id = ?1 AND is_internal = 0",
                        params![value, now],
                    )?;
                }
            }
            return Ok(());
        }
        let sql = match field {
            "name" => "UPDATE accounts SET name = ?1, updated_at = ?3 WHERE id = ?2",
            "health" => "UPDATE accounts SET health = ?1, updated_at = ?3 WHERE id = ?2",
            "lifecycle" => "UPDATE accounts SET lifecycle = ?1, updated_at = ?3 WHERE id = ?2",
            "arr" => "UPDATE accounts SET arr = CAST(?1 AS REAL), updated_at = ?3 WHERE id = ?2",
            "nps" => "UPDATE accounts SET nps = CAST(?1 AS INTEGER), updated_at = ?3 WHERE id = ?2",
            "contract_start" => {
                "UPDATE accounts SET contract_start = ?1, updated_at = ?3 WHERE id = ?2"
            }
            "contract_end" => {
                "UPDATE accounts SET contract_end = ?1, updated_at = ?3 WHERE id = ?2"
            }
            _ => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("Field '{}' is not updatable", field),
                )))
            }
        };
        self.conn.execute(sql, params![value, id, now])?;
        Ok(())
    }

    // =========================================================================
    // Entity Metadata (I311)
    // =========================================================================

    /// Update JSON metadata for an entity (account or project).
    pub fn update_entity_metadata(
        &self,
        entity_type: &str,
        entity_id: &str,
        metadata_json: &str,
    ) -> Result<(), String> {
        let table = match entity_type {
            "account" => "accounts",
            "project" => "projects",
            _ => return Err(format!("Invalid entity type for metadata: {}", entity_type)),
        };
        let sql = format!("UPDATE {} SET metadata = ?1 WHERE id = ?2", table);
        self.conn
            .execute(&sql, params![metadata_json, entity_id])
            .map_err(|e| format!("Failed to update metadata: {}", e))?;
        Ok(())
    }

    /// Get JSON metadata for an entity (account or project).
    pub fn get_entity_metadata(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<String, String> {
        let table = match entity_type {
            "account" => "accounts",
            "project" => "projects",
            _ => return Err(format!("Invalid entity type for metadata: {}", entity_type)),
        };
        let sql = format!(
            "SELECT COALESCE(metadata, '{{}}') FROM {} WHERE id = ?1",
            table
        );
        self.conn
            .query_row(&sql, params![entity_id], |row| row.get(0))
            .map_err(|e| format!("Failed to get metadata: {}", e))
    }


    // =========================================================================
    // Domain reclassification (I184 — reclassify on domain change)
    // =========================================================================

    /// Reclassify all people's relationship based on current user domains.
    /// People whose email domain matches ANY domain → "internal", otherwise → "external".
    /// Returns the number of people whose relationship changed.
    pub fn reclassify_people_for_domains(&self, user_domains: &[String]) -> Result<usize, DbError> {
        if user_domains.is_empty() {
            return Ok(0);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT id, email, relationship FROM people")?;
        let people: Vec<(String, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut update_stmt = self
            .conn
            .prepare("UPDATE people SET relationship = ?1 WHERE id = ?2")?;

        let mut total = 0;
        for (id, email, current_rel) in &people {
            let domain = email.split('@').nth(1).unwrap_or("");
            if domain.is_empty() {
                continue;
            }
            let new_rel = if user_domains.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
                "internal"
            } else {
                "external"
            };
            if new_rel != current_rel {
                update_stmt.execute(params![new_rel, id])?;
                total += 1;
            }
        }

        Ok(total)
    }

    /// Reclassify meeting types based on current attendee relationships.
    /// Call after `reclassify_people_for_domains` to fix meetings whose type
    /// was stale due to domain changes. Returns the number updated.
    ///
    /// Only touches domain-dependent types (customer, external, one_on_one, internal).
    /// Title-derived types (qbr, training, all_hands, team_sync, personal) are left alone
    /// since they don't depend on domain classification.
    pub fn reclassify_meeting_types_from_attendees(&self) -> Result<usize, DbError> {
        let mut total = 0;

        // Meetings classified as customer/external/one_on_one but ALL attendees are now internal → internal
        total += self.conn.execute(
            "UPDATE meetings_history SET meeting_type = 'internal'
             WHERE meeting_type IN ('customer', 'external', 'one_on_one')
               AND id IN (
                   SELECT ma.meeting_id
                   FROM meeting_attendees ma
                   JOIN people p ON ma.person_id = p.id
                   GROUP BY ma.meeting_id
                   HAVING COUNT(CASE WHEN p.relationship != 'internal' THEN 1 END) = 0
               )",
            [],
        )?;

        // Meetings classified as internal but ANY attendee is now external → customer
        total += self.conn.execute(
            "UPDATE meetings_history SET meeting_type = 'customer'
             WHERE meeting_type = 'internal'
               AND id IN (
                   SELECT DISTINCT ma.meeting_id
                   FROM meeting_attendees ma
                   JOIN people p ON ma.person_id = p.id
                   WHERE p.relationship = 'external'
               )",
            [],
        )?;

        Ok(total)
    }

    /// Get meetings for any entity (generic, via junction table).
    pub fn get_meetings_for_entity(
        &self,
        entity_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_id = ?1
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![entity_id, limit], |row| {
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

    /// Compute activity signals for a project.
    pub fn get_project_signals(&self, project_id: &str) -> Result<ProjectSignals, DbError> {
        // Meeting counts via junction table
        let count_30d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_id = ?1 AND me.entity_type = 'project'
                   AND m.start_time >= date('now', '-30 days')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let count_90d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_id = ?1 AND me.entity_type = 'project'
                   AND m.start_time >= date('now', '-90 days')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let last_meeting: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(m.start_time) FROM meetings_history m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_id = ?1 AND me.entity_type = 'project'",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        // Days until target date
        let target_date: Option<String> = self
            .conn
            .query_row(
                "SELECT target_date FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let days_until_target = target_date.as_ref().and_then(|td| {
            chrono::NaiveDate::parse_from_str(td, "%Y-%m-%d")
                .ok()
                .map(|date| {
                    let today = Utc::now().date_naive();
                    (date - today).num_days()
                })
        });

        // Open action count
        let open_action_count: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM actions
                 WHERE project_id = ?1 AND status IN ('pending', 'waiting')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let temperature = match &last_meeting {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        };
        let trend = compute_trend(count_30d, count_90d);

        Ok(ProjectSignals {
            meeting_frequency_30d: count_30d,
            meeting_frequency_90d: count_90d,
            last_meeting,
            days_until_target,
            open_action_count,
            temperature,
            trend,
        })
    }


    /// Helper: map a row to `DbAccount`.
    pub(crate) fn map_account_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbAccount> {
        Ok(DbAccount {
            id: row.get(0)?,
            name: row.get(1)?,
            lifecycle: row.get(2)?,
            arr: row.get(3)?,
            health: row.get(4)?,
            contract_start: row.get(5)?,
            contract_end: row.get(6)?,
            nps: row.get(7)?,
            tracker_path: row.get(8)?,
            parent_id: row.get(9)?,
            is_internal: row.get::<_, i32>(10).unwrap_or(0) != 0,
            updated_at: row.get(11)?,
            archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
            keywords: row.get(13).unwrap_or(None),
            keywords_extracted_at: row.get(14).unwrap_or(None),
            metadata: row.get(15).unwrap_or(None),
        })
    }

    // =========================================================================
    // Archive (Sprint 12)
    // =========================================================================

    /// Archive or unarchive an account. Cascade: archiving a parent archives all children.
    pub fn archive_account(&self, id: &str, archived: bool) -> Result<usize, DbError> {
        let val = if archived { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();

        // Archive/unarchive the account itself
        let changed = self.conn.execute(
            "UPDATE accounts SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![val, now, id],
        )?;

        // If archiving a parent, cascade to children
        if archived {
            self.conn.execute(
                "UPDATE accounts SET archived = 1, updated_at = ?1 WHERE parent_id = ?2",
                params![now, id],
            )?;
        }

        Ok(changed)
    }

    /// Archive or unarchive a project.
    pub fn archive_project(&self, id: &str, archived: bool) -> Result<usize, DbError> {
        let val = if archived { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();
        Ok(self.conn.execute(
            "UPDATE projects SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![val, now, id],
        )?)
    }

    /// Archive or unarchive a person.
    pub fn archive_person(&self, id: &str, archived: bool) -> Result<usize, DbError> {
        let val = if archived { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();
        Ok(self.conn.execute(
            "UPDATE people SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![val, now, id],
        )?)
    }

    /// Restore an archived account, optionally restoring archived children.
    /// Returns the number of child accounts restored.
    pub fn restore_account(&self, id: &str, restore_children: bool) -> Result<usize, DbError> {
        let now = Utc::now().to_rfc3339();

        // Unarchive the account itself
        self.conn.execute(
            "UPDATE accounts SET archived = 0, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;

        // Optionally restore archived children
        let children_restored = if restore_children {
            self.conn.execute(
                "UPDATE accounts SET archived = 0, updated_at = ?1 WHERE parent_id = ?2 AND archived = 1",
                params![now, id],
            )?
        } else {
            0
        };

        Ok(children_restored)
    }

    // =========================================================================
    // I198: Account Merge
    // =========================================================================

    /// Merge source account into target account.
    /// Reassigns all associated records and archives the source.
    /// Wrapped in a transaction for atomicity.
    pub fn merge_accounts(&self, from_id: &str, into_id: &str) -> Result<MergeResult, DbError> {
        self.with_transaction(|tx| {
            let conn = tx.conn_ref();

            // Reassign actions
            let actions_moved = conn.execute(
                "UPDATE actions SET account_id = ?2 WHERE account_id = ?1",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;

            // Reassign meeting_entities (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE meeting_entities SET entity_id = ?2
                 WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;
            // Clean up remaining dupes
            let meetings_moved = conn.execute(
                "DELETE FROM meeting_entities WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id],
            ).map_err(|e| e.to_string())?;

            // Reassign entity_people (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE entity_people SET entity_id = ?2
                 WHERE entity_id = ?1",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;
            let people_moved = conn.execute(
                "DELETE FROM entity_people WHERE entity_id = ?1",
                params![from_id],
            ).map_err(|e| e.to_string())?;

            // Reassign account_team (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE account_team SET account_id = ?2
                 WHERE account_id = ?1",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM account_team WHERE account_id = ?1",
                params![from_id],
            ).map_err(|e| e.to_string())?;

            // Reassign account_events
            let events_moved = conn.execute(
                "UPDATE account_events SET account_id = ?2 WHERE account_id = ?1",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;

            // Reassign signal_events
            conn.execute(
                "UPDATE OR IGNORE signal_events SET entity_id = ?2
                 WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM signal_events WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id],
            ).map_err(|e| e.to_string())?;

            // Reassign content_index
            conn.execute(
                "UPDATE OR IGNORE content_index SET entity_id = ?2
                 WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM content_index WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id],
            ).map_err(|e| e.to_string())?;

            // Reassign account_domains (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE account_domains SET account_id = ?2
                 WHERE account_id = ?1",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM account_domains WHERE account_id = ?1",
                params![from_id],
            ).map_err(|e| e.to_string())?;

            // Reassign children
            let children_moved = conn.execute(
                "UPDATE accounts SET parent_id = ?2 WHERE parent_id = ?1",
                params![from_id, into_id],
            ).map_err(|e| e.to_string())?;

            // Archive source account
            conn.execute(
                "UPDATE accounts SET archived = 1 WHERE id = ?1",
                params![from_id],
            ).map_err(|e| e.to_string())?;

            Ok(MergeResult {
                actions_moved,
                meetings_moved,
                people_moved,
                events_moved,
                children_moved,
            })
        }).map_err(DbError::Migration)
    }

    /// Get archived accounts (top-level + children).
    pub fn get_archived_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM accounts WHERE archived = 1 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get archived projects.
    pub fn get_archived_projects(&self) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date, tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at, metadata
             FROM projects WHERE archived = 1 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get archived people with signals.
    pub fn get_archived_people_with_signals(&self) -> Result<Vec<PersonListItem>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship,
                    p.notes, p.tracker_path, p.last_seen, p.first_seen, p.meeting_count,
                    p.updated_at, p.archived,
                    COALESCE(m30.cnt, 0) as freq_30d,
                    COALESCE(m90.cnt, 0) as freq_90d,
                    acct_names.names AS account_names
             FROM people p
             LEFT JOIN (
                 SELECT person_id, COUNT(*) as cnt
                 FROM meeting_attendees ma
                 JOIN meetings_history mh ON ma.meeting_id = mh.id
                 WHERE mh.start_time >= datetime('now', '-30 days')
                 GROUP BY person_id
             ) m30 ON m30.person_id = p.id
             LEFT JOIN (
                 SELECT person_id, COUNT(*) as cnt
                 FROM meeting_attendees ma
                 JOIN meetings_history mh ON ma.meeting_id = mh.id
                 WHERE mh.start_time >= datetime('now', '-90 days')
                 GROUP BY person_id
             ) m90 ON m90.person_id = p.id
             LEFT JOIN (
                 SELECT ep.person_id, GROUP_CONCAT(e.name, ', ') AS names
                 FROM entity_people ep
                 JOIN entities e ON e.id = ep.entity_id AND e.entity_type = 'account'
                 GROUP BY ep.person_id
             ) acct_names ON acct_names.person_id = p.id
             WHERE p.archived = 1
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map([], |row| {
            let last_seen: Option<String> = row.get(8)?;
            let freq_30d: i32 = row.get(13)?;
            let freq_90d: i32 = row.get(14)?;
            let temperature = Self::compute_temperature(&last_seen);
            let trend = Self::compute_trend(freq_30d, freq_90d);
            Ok(PersonListItem {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                organization: row.get(3)?,
                role: row.get(4)?,
                relationship: row.get(5)?,
                notes: row.get(6)?,
                tracker_path: row.get(7)?,
                last_seen,
                first_seen: row.get(9)?,
                meeting_count: row.get(10)?,
                updated_at: row.get(11)?,
                archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
                temperature,
                trend,
                account_names: row.get(15)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }


    // =========================================================================
    // Account Events (I143 — renewal tracking)
    // =========================================================================

    /// Record a lifecycle event for an account.
    pub fn record_account_event(
        &self,
        account_id: &str,
        event_type: &str,
        event_date: &str,
        arr_impact: Option<f64>,
        notes: Option<&str>,
    ) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO account_events (account_id, event_type, event_date, arr_impact, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account_id, event_type, event_date, arr_impact, notes],
        )?;

        // Auto-archive on churn
        if event_type == "churn" {
            self.conn.execute(
                "UPDATE accounts SET archived = 1, updated_at = ?2 WHERE id = ?1",
                params![account_id, chrono::Utc::now().to_rfc3339()],
            )?;
        }

        Ok(self.conn.last_insert_rowid())
    }

    /// Get lifecycle events for an account, ordered by date descending.
    pub fn get_account_events(&self, account_id: &str) -> Result<Vec<DbAccountEvent>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, event_type, event_date, arr_impact, notes, created_at
             FROM account_events WHERE account_id = ?1 ORDER BY event_date DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountEvent {
                id: row.get(0)?,
                account_id: row.get(1)?,
                event_type: row.get(2)?,
                event_date: row.get(3)?,
                arr_impact: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Check if an account has any churn events (for auto-rollover logic).
    pub fn has_churn_event(&self, account_id: &str) -> Result<bool, DbError> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM account_events WHERE account_id = ?1 AND event_type = 'churn'",
            params![account_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get accounts with renewal_date (contract_end) in the past and no churn event.
    pub fn get_accounts_past_renewal(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start, a.contract_end,
                    a.nps, a.tracker_path, a.parent_id, a.is_internal, a.updated_at, a.archived
             FROM accounts a
             WHERE a.contract_end IS NOT NULL
               AND a.contract_end < date('now')
               AND a.archived = 0
               AND a.id NOT IN (
                   SELECT account_id FROM account_events WHERE event_type = 'churn'
               )
             ORDER BY a.contract_end",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }


}
