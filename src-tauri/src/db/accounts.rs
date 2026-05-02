use super::*;
use rusqlite::OptionalExtension;

impl ActionDb {
    // =========================================================================
    // Accounts
    // =========================================================================

    /// Insert or update an account. Also mirrors to the `entities` table (ADR-0045).
    pub fn upsert_account(&self, account: &DbAccount) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO accounts (
                id, name, lifecycle, arr, health, contract_start, contract_end,
                nps, tracker_path, parent_id, is_internal, account_type, updated_at, archived,
                keywords, keywords_extracted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
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
                account_type = excluded.account_type,
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
                account.account_type.is_internal() as i32,
                account.account_type.as_db_str(),
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
        let sql = format!(
            "SELECT {} FROM accounts WHERE id = ?1",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;

        let mut rows = stmt.query_map(params![id], Self::map_account_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get an account by name (case-insensitive).
    pub fn get_account_by_name(&self, name: &str) -> Result<Option<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts WHERE LOWER(name) = LOWER(?1)",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;

        let mut rows = stmt.query_map(params![name], Self::map_account_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all accounts, ordered by name.
    pub fn get_all_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts WHERE archived = 0 ORDER BY name",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get top-level accounts (no parent), ordered by name.
    pub fn get_top_level_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts WHERE parent_id IS NULL AND archived = 0 ORDER BY name",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get child accounts for a parent, ordered by name.
    pub fn get_child_accounts(&self, parent_id: &str) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts WHERE parent_id = ?1 AND archived = 0 ORDER BY name",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![parent_id], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Walk the parent_id chain to get all ancestors (I316: n-level nesting).
    pub fn get_account_ancestors(&self, account_id: &str) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "WITH RECURSIVE ancestors(id) AS (
                SELECT parent_id FROM accounts WHERE id = ?1
                UNION ALL
                SELECT a.parent_id FROM accounts a JOIN ancestors anc ON a.id = anc.id
                WHERE a.parent_id IS NOT NULL
            )
            SELECT {} FROM accounts
            WHERE id IN (SELECT id FROM ancestors WHERE id IS NOT NULL)
            ORDER BY id",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![account_id], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get all descendants using recursive CTE with depth limit (I316: n-level nesting).
    pub fn get_descendant_accounts(&self, ancestor_id: &str) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "WITH RECURSIVE descendants(id, depth) AS (
                SELECT id, 1 FROM accounts WHERE parent_id = ?1
                UNION ALL
                SELECT a.id, d.depth + 1 FROM accounts a
                JOIN descendants d ON a.parent_id = d.id
                WHERE d.depth < 10
            )
            SELECT {} FROM accounts acc
            JOIN descendants d ON acc.id = d.id
            WHERE acc.archived = 0
            ORDER BY acc.name",
            Self::account_columns_aliased("acc")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![ancestor_id], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Set domains for an account (replace-all). Writes source='user' — this is
    /// the explicit user-entry path (account settings page, onboarding setup).
    pub fn set_account_domains(&self, account_id: &str, domains: &[String]) -> Result<(), DbError> {
        let normalized = crate::helpers::normalize_domains(domains);
        self.conn.execute(
            "DELETE FROM account_domains WHERE account_id = ?1",
            params![account_id],
        )?;
        for domain in normalized {
            self.conn.execute(
                "INSERT OR IGNORE INTO account_domains (account_id, domain, source) \
                 VALUES (?1, ?2, 'user')",
                params![account_id, &domain],
            )?;
        }
        Ok(())
    }

    /// Additively merge domains from meeting attendee inference. Uses the
    /// default source='inferred', meaning these can be purged by
    /// `raw_rebuild_account_domains` before the DOS-258 cutover.
    pub fn merge_account_domains(
        &self,
        account_id: &str,
        domains: &[String],
    ) -> Result<(), DbError> {
        let normalized = crate::helpers::normalize_domains(domains);
        for domain in normalized {
            self.conn.execute(
                "INSERT OR IGNORE INTO account_domains (account_id, domain) VALUES (?1, ?2)",
                params![account_id, &domain],
            )?;
        }
        Ok(())
    }

    /// Additively merge domains from a trusted enrichment provider (Clay, Glean, Google).
    /// Writes source='enrichment' so `raw_rebuild_account_domains` preserves these
    /// while purging attendee-inferred domains.
    pub fn merge_account_domains_enrichment(
        &self,
        account_id: &str,
        domains: &[String],
    ) -> Result<(), DbError> {
        let normalized = crate::helpers::normalize_domains(domains);
        for domain in normalized {
            self.conn.execute(
                "INSERT OR IGNORE INTO account_domains (account_id, domain, source) \
                 VALUES (?1, ?2, 'enrichment')",
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

    /// DOS-258: Return all distinct active external stakeholder emails for an account,
    /// excluding internal-relationship people. Used to derive account_domains from
    /// the stakeholder graph.
    pub fn get_active_external_stakeholder_emails(
        &self,
        account_id: &str,
    ) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT LOWER(p.email) \
             FROM account_stakeholders asa \
             JOIN people p ON p.id = asa.person_id \
             WHERE asa.account_id = ?1 \
               AND asa.status = 'active' \
               AND p.email IS NOT NULL AND p.email != '' \
               AND (p.relationship IS NULL OR p.relationship != 'internal')",
        )?;
        let rows = stmt.query_map(params![account_id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// DOS-258: Return every (account_id, email) tuple for active external stakeholders
    /// across all non-archived customer/partner accounts. Used by the boot sweep.
    pub fn get_all_active_external_stakeholder_emails(
        &self,
    ) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT asa.account_id, LOWER(p.email) \
             FROM account_stakeholders asa \
             JOIN people p ON p.id = asa.person_id \
             JOIN accounts a ON a.id = asa.account_id \
             WHERE asa.status = 'active' \
               AND p.email IS NOT NULL AND p.email != '' \
               AND (p.relationship IS NULL OR p.relationship != 'internal') \
               AND a.archived = 0 \
               AND a.account_type IN ('customer', 'partner')",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
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
        let acols = Self::account_columns_aliased("a");
        let query = if include_archived {
            format!(
                "SELECT {}, ad.domain FROM accounts a \
                 LEFT JOIN account_domains ad ON a.id = ad.account_id \
                 ORDER BY a.id, ad.domain",
                acols
            )
        } else {
            format!(
                "SELECT {}, ad.domain FROM accounts a \
                 LEFT JOIN account_domains ad ON a.id = ad.account_id \
                 WHERE a.archived = 0 \
                 ORDER BY a.id, ad.domain",
                acols
            )
        };

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;

        let mut result: Vec<(DbAccount, Vec<String>)> = Vec::new();
        let mut current_id: Option<String> = None;
        let domain_idx = Self::ACCOUNT_COLUMNS.split(',').count();

        while let Some(row) = rows.next()? {
            let account_id: String = row.get(0)?;
            let domain: Option<String> = row.get(domain_idx)?;

            if current_id.as_deref() != Some(&account_id) {
                // New account — push a new entry via map helper
                let account = Self::map_account_row(row)?;
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
    ///
    /// DOS-258 Tier 4 domain hierarchy: if the domain matches an account that
    /// has `parent_id` set (i.e. a subsidiary), the parent account is
    /// surfaced alongside it as a second candidate. P9 then lets the user
    /// pick between "Subsidiary BU" and "Parent Corp" rather than blindly
    /// picking the subsidiary. When the attendee meant the parent (e.g.,
    /// a group-wide stakeholder attending a subsidiary-domain email thread),
    /// the picker now offers both.
    ///
    /// Schema choice: reuses the existing `accounts.parent_id` column
    /// (migration 001_baseline.sql). No new hierarchy table needed — the
    /// parent walk is a single indexed lookup per direct hit.
    pub fn lookup_account_candidates_by_domain(
        &self,
        domain: &str,
    ) -> Result<Vec<DbAccount>, DbError> {
        let normalized = domain.trim().to_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let sql = format!(
            "SELECT {} FROM accounts a \
             INNER JOIN account_domains d ON d.account_id = a.id \
             WHERE d.domain = ?1 AND a.archived = 0 \
             ORDER BY a.account_type ASC, a.name ASC",
            Self::account_columns_aliased("a")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![normalized], Self::map_account_row)?;
        let mut direct: Vec<DbAccount> = rows.collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        // Walk parent_id for each direct hit and surface the parent as an
        // additional candidate (dedup by id). P9 will see multiple candidates
        // and prompt the user when ambiguous; if the parent ALSO has a
        // direct domain match, the dedup keeps a single entry.
        let parent_ids: Vec<String> = direct.iter().filter_map(|a| a.parent_id.clone()).collect();
        if parent_ids.is_empty() {
            return Ok(direct);
        }

        let parent_sql = format!(
            "SELECT {} FROM accounts a WHERE a.id = ?1 AND a.archived = 0",
            Self::account_columns_aliased("a")
        );
        let mut parent_stmt = self.conn.prepare(&parent_sql)?;
        for pid in parent_ids {
            if direct.iter().any(|a| a.id == pid) {
                continue;
            }
            let mut rows = parent_stmt.query_map(params![pid], Self::map_account_row)?;
            if let Some(Ok(parent)) = rows.next() {
                direct.push(parent);
            }
        }

        Ok(direct)
    }

    /// Copy domains from parent to child (idempotent).
    /// Copy domains from parent to child, preserving the source column.
    pub fn copy_account_domains(&self, parent_id: &str, child_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO account_domains (account_id, domain, source)
             SELECT ?1, domain, source FROM account_domains WHERE account_id = ?2",
            params![child_id, parent_id],
        )?;
        Ok(())
    }

    /// Path 2b: Backfill account_domains from existing meeting→account links.
    ///
    /// Processes all meetings linked to accounts and extracts attendee domains,
    /// populating account_domains table for accounts that currently have 0 domains.
    /// Returns count of account→domain mappings created.
    ///
    /// Safe to run multiple times (idempotent via INSERT OR IGNORE).
    pub fn backfill_account_domains_from_meetings(&self) -> Result<usize, DbError> {
        // Domain-base matching: only store a domain on an account when the domain
        // base (before first dot) matches the account's normalized name or slug.
        // Prevents cross-contamination where a partner's domain (e.g. salesforce.com)
        // gets stored on every account they attend meetings with.
        let user_domains: Vec<String> = crate::state::load_config()
            .map(|c| c.resolved_user_domains())
            .unwrap_or_default();

        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT m.id, m.attendees, me.entity_id, a.name
             FROM meetings m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             INNER JOIN accounts a ON a.id = me.entity_id
             WHERE me.entity_type = 'account'
               AND m.attendees IS NOT NULL
               AND m.attendees != ''
             ORDER BY me.entity_id",
        )?;

        let mut inserted_count = 0;
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let attendees_str: String = row.get(1)?;
            let account_id: String = row.get(2)?;
            let account_name: String = row.get(3)?;

            let account_name_key = crate::helpers::normalize_key(&account_name);
            let account_slug_key = crate::helpers::normalize_key(&account_id);

            let domains = self.extract_domains_from_attendees_str(&attendees_str);
            for domain in &domains {
                if user_domains.iter().any(|ud| ud == domain) {
                    continue;
                }
                let domain_base = domain.split('.').next().unwrap_or(domain);
                let domain_base_key = crate::helpers::normalize_key(domain_base);

                // Only store domain if domain base matches account name or slug
                if !domain_base_key.is_empty()
                    && (domain_base_key == account_name_key
                        || domain_base_key == account_slug_key
                        || (domain_base_key.len() >= 4
                            && account_name_key.contains(&domain_base_key))
                        || (account_name_key.len() >= 4
                            && domain_base_key.contains(&account_name_key)))
                {
                    self.conn.execute(
                        "INSERT OR IGNORE INTO account_domains (account_id, domain) VALUES (?1, ?2)",
                        params![&account_id, &domain],
                    )?;
                    inserted_count += 1;
                }
            }
        }

        Ok(inserted_count)
    }

    /// Helper: Extract unique domains from attendee string (JSON or comma-separated).
    pub fn extract_domains_from_attendees_str(&self, attendees_str: &str) -> Vec<String> {
        // Try parsing as JSON array first
        let attendees_array: Vec<String> =
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(attendees_str) {
                arr
            } else {
                // Fall back to comma-separated parsing
                attendees_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            };

        // Extract domains from emails
        let mut domains = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for email in attendees_array {
            if let Some(domain_part) = email.split('@').nth(1) {
                let domain = domain_part.to_lowercase();
                // Filter out obviously invalid domains and duplicates
                if !domain.is_empty() && !domain.contains(' ') && seen.insert(domain.clone()) {
                    domains.push(domain);
                }
            }
        }

        domains
    }

    /// Root internal organization account (top-level internal account).
    pub fn get_internal_root_account(&self) -> Result<Option<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts \
             WHERE account_type = 'internal' AND parent_id IS NULL AND archived = 0 \
             ORDER BY updated_at DESC LIMIT 1",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map([], Self::map_account_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// All active internal accounts.
    pub fn get_internal_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts \
             WHERE account_type = 'internal' AND archived = 0 \
             ORDER BY name",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get account team members with person details.
    pub fn get_account_team(&self, account_id: &str) -> Result<Vec<DbAccountTeamMember>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT as_.account_id, as_.person_id, p.name, p.email,
                    COALESCE(
                        (SELECT GROUP_CONCAT(asr.role, ',')
                         FROM account_stakeholder_roles asr
                         WHERE asr.account_id = as_.account_id AND asr.person_id = as_.person_id
                           AND asr.dismissed_at IS NULL),
                        ''
                    ) AS roles,
                    as_.created_at
             FROM account_stakeholders as_
             JOIN people p ON p.id = as_.person_id
             WHERE as_.account_id = ?1
             ORDER BY p.name",
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

    /// Get account team members filtered to internal people only (for UI display).
    /// Health scoring uses `get_account_team` which includes all stakeholders.
    pub fn get_account_team_internal(
        &self,
        account_id: &str,
    ) -> Result<Vec<DbAccountTeamMember>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT as_.account_id, as_.person_id, p.name, p.email,
                    COALESCE(
                        (SELECT GROUP_CONCAT(asr.role, ',')
                         FROM account_stakeholder_roles asr
                         WHERE asr.account_id = as_.account_id AND asr.person_id = as_.person_id
                           AND asr.dismissed_at IS NULL),
                        ''
                    ) AS roles,
                    as_.created_at
             FROM account_stakeholders as_
             JOIN people p ON p.id = as_.person_id
             WHERE as_.account_id = ?1 AND p.relationship = 'internal'
             ORDER BY p.name",
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

    /// Full stakeholder data with data_source for the DB-first read model (I652).
    /// Returns ALL stakeholders (user-confirmed + Glean-suggested + Google-sourced)
    /// plus linked people from entity_members. Roles come from account_stakeholder_roles.
    pub fn get_account_stakeholders_full(
        &self,
        account_id: &str,
    ) -> Result<Vec<DbStakeholderFull>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT as_.person_id, p.name, p.email, p.organization, p.role AS person_role,
                    as_.data_source, as_.last_seen_in_glean,
                    as_.created_at,
                    p.linkedin_url, p.photo_url, p.meeting_count, p.last_seen,
                    as_.engagement, as_.data_source_engagement,
                    as_.assessment, as_.data_source_assessment,
                    asr.role AS stakeholder_role,
                    asr.data_source AS stakeholder_role_data_source
             FROM account_stakeholders as_
             JOIN people p ON p.id = as_.person_id
             LEFT JOIN account_stakeholder_roles asr
               ON asr.account_id = as_.account_id
              AND asr.person_id = as_.person_id
              AND asr.dismissed_at IS NULL
             WHERE as_.account_id = ?1
               AND as_.status = 'active'
               AND p.relationship != 'internal'
               AND p.email NOT LIKE '%group-calendar%'
               AND p.email NOT LIKE '%assistant.gong%'
               AND p.email NOT LIKE '%noreply%'
               AND length(p.name) > 3
             ORDER BY
               CASE as_.data_source WHEN 'user' THEN 0 WHEN 'glean' THEN 1 ELSE 2 END,
               p.name,
               as_.person_id,
               asr.role",
        )?;

        let rows = stmt.query_map(params![account_id], |row| {
            let stakeholder = DbStakeholderFull {
                person_id: row.get(0)?,
                person_name: row.get(1)?,
                person_email: row.get(2)?,
                organization: row.get(3)?,
                person_role: row.get(4)?,
                stakeholder_role: String::new(),
                roles: Vec::new(),
                data_source: row
                    .get::<_, Option<String>>(5)?
                    .unwrap_or_else(|| "user".to_string()),
                last_seen_in_glean: row.get(6)?,
                created_at: row.get(7)?,
                linkedin_url: row.get(8)?,
                photo_url: row.get(9)?,
                meeting_count: row.get(10)?,
                last_seen: row.get(11)?,
                engagement: row.get(12)?,
                data_source_engagement: row.get(13)?,
                assessment: row.get(14)?,
                data_source_assessment: row.get(15)?,
            };

            let role = row.get::<_, Option<String>>(16)?.map(|role| {
                Ok::<_, rusqlite::Error>(StakeholderRole {
                    role,
                    data_source: row
                        .get::<_, Option<String>>(17)?
                        .unwrap_or_else(|| "unknown".to_string()),
                })
            }).transpose()?;

            Ok((stakeholder, role))
        })?;

        let mut stakeholders: Vec<DbStakeholderFull> = Vec::new();
        let mut stakeholder_index_by_person_id: HashMap<String, usize> = HashMap::new();

        for row in rows {
            let (stakeholder, role) = row?;
            let idx = if let Some(idx) = stakeholder_index_by_person_id.get(&stakeholder.person_id)
            {
                *idx
            } else {
                let idx = stakeholders.len();
                stakeholder_index_by_person_id.insert(stakeholder.person_id.clone(), idx);
                stakeholders.push(stakeholder);
                idx
            };

            if let Some(role) = role {
                stakeholders[idx].roles.push(role);
            }
        }

        for stakeholder in &mut stakeholders {
            stakeholder.stakeholder_role = if stakeholder.roles.is_empty() {
                "associated".to_string()
            } else {
                stakeholder
                    .roles
                    .iter()
                    .map(|role| role.role.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            };
        }

        Ok(stakeholders)
    }

    /// Get all stakeholder roles for a person across all their linked accounts.
    pub fn get_person_stakeholder_roles(
        &self,
        person_id: &str,
    ) -> Result<Vec<crate::db::types::PersonAccountRole>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT asr.account_id, a.name AS account_name, asr.role, asr.data_source
             FROM account_stakeholder_roles asr
             JOIN accounts a ON a.id = asr.account_id
             WHERE asr.person_id = ?1 AND asr.dismissed_at IS NULL
             ORDER BY a.name, asr.role",
        )?;
        let rows = stmt.query_map(params![person_id], |row| {
            Ok(crate::db::types::PersonAccountRole {
                account_id: row.get(0)?,
                account_name: row.get(1)?,
                role: row.get(2)?,
                data_source: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get typed roles for a specific stakeholder (I652).
    pub fn get_stakeholder_roles(
        &self,
        account_id: &str,
        person_id: &str,
    ) -> Result<Vec<crate::db::types::StakeholderRole>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT role, data_source FROM account_stakeholder_roles
             WHERE account_id = ?1 AND person_id = ?2 AND dismissed_at IS NULL
             ORDER BY role",
        )?;
        let rows = stmt.query_map(params![account_id, person_id], |row| {
            Ok(crate::db::types::StakeholderRole {
                role: row.get(0)?,
                data_source: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Add an account team member with a role (idempotent, I652).
    /// Inserts into account_stakeholders (the link) + account_stakeholder_roles (the role).
    pub fn add_account_team_member(
        &self,
        account_id: &str,
        person_id: &str,
        role: &str,
    ) -> Result<(), DbError> {
        let role = role.trim().to_lowercase();
        let now = Utc::now().to_rfc3339();
        // Ensure the person-account link exists
        self.conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id, data_source, created_at)
             VALUES (?1, ?2, 'user', ?3)
             ON CONFLICT(account_id, person_id) DO UPDATE SET data_source = 'user'",
            params![account_id, person_id, now],
        )?;
        // Insert the role with user provenance
        self.conn.execute(
            "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source, created_at)
             VALUES (?1, ?2, ?3, 'user', ?4)
             ON CONFLICT(account_id, person_id, role) DO UPDATE SET data_source = 'user'",
            params![account_id, person_id, role, now],
        )?;
        Ok(())
    }

    /// Replace all roles for a team member (single-select role change).
    /// Replace the user's pinned roles with a single new user-owned role.
    ///
    /// Semantics: this is the "swap my pinned role" operation. It touches
    /// ONLY rows where `data_source = 'user'`. AI-surfaced roles
    /// (`data_source = 'ai' | 'glean' | 'pty_synthesis'`) survive untouched.
    ///
    /// The old implementation deleted ALL roles for the person-account
    /// pair regardless of provenance. That let a single user dropdown
    /// click wipe out AI-surfaced role rows, and the next enrichment
    /// would then re-insert Champion/Technical/etc. with `data_source='ai'`,
    /// silently losing the human's original pin — the primary cause of
    /// the "AI keeps overwriting my champion" bug the user reported.
    ///
    /// `new_role` empty → remove the user's pinned role(s) entirely;
    /// person stays as stakeholder with any AI-surfaced roles intact.
    pub fn set_team_member_role(
        &self,
        account_id: &str,
        person_id: &str,
        new_role: &str,
    ) -> Result<(), DbError> {
        let role = new_role.trim().to_lowercase();
        // Remove only user-owned roles. AI-surfaced rows are preserved so
        // a role swap in the UI can't silently drop enrichment-discovered
        // context.
        self.conn.execute(
            "DELETE FROM account_stakeholder_roles
             WHERE account_id = ?1 AND person_id = ?2 AND data_source = 'user'",
            params![account_id, person_id],
        )?;
        // Insert the new user-owned role. ON CONFLICT promotes an AI row
        // to user ownership without churning the row (so if the user
        // pins the same role AI had surfaced, provenance flips cleanly).
        if !role.is_empty() {
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source, created_at)
                 VALUES (?1, ?2, ?3, 'user', ?4)
                 ON CONFLICT(account_id, person_id, role) DO UPDATE SET
                    data_source = 'user'",
                params![account_id, person_id, role, now],
            )?;
        }
        Ok(())
    }

    /// Link a person to an account with explicit data source (I505, updated I652).
    ///
    /// Sets `last_seen_in_glean` on insert/update. Does NOT overwrite `data_source`
    /// on the role if the existing role was user-owned (`data_source = 'user'`).
    pub fn link_person_to_account_with_source(
        &self,
        account_id: &str,
        person_id: &str,
        role: &str,
        data_source: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        // Ensure the person-account link exists
        self.conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id, data_source, last_seen_in_glean, created_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(account_id, person_id) DO UPDATE SET
                last_seen_in_glean = excluded.last_seen_in_glean,
                data_source = CASE WHEN account_stakeholders.data_source = 'user'
                    THEN account_stakeholders.data_source ELSE excluded.data_source END",
            params![account_id, person_id, data_source, now],
        )?;
        // Add the role (don't overwrite user-owned roles)
        let role = role.trim().to_lowercase();
        self.conn.execute(
            "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(account_id, person_id, role) DO UPDATE SET
                data_source = CASE WHEN account_stakeholder_roles.data_source = 'user'
                    THEN account_stakeholder_roles.data_source ELSE excluded.data_source END",
            params![account_id, person_id, role, data_source, now],
        )?;
        Ok(())
    }

    /// Remove an account team member link and all their roles (I652).
    pub fn remove_account_team_member(
        &self,
        account_id: &str,
        person_id: &str,
        _role: &str,
    ) -> Result<(), DbError> {
        // Roles cascade via FK, but be explicit
        self.conn.execute(
            "DELETE FROM account_stakeholder_roles
             WHERE account_id = ?1 AND person_id = ?2",
            params![account_id, person_id],
        )?;
        self.conn.execute(
            "DELETE FROM account_stakeholders
             WHERE account_id = ?1 AND person_id = ?2",
            params![account_id, person_id],
        )?;
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

    /// Get pending stakeholder suggestions for an account (I652 phase 2).
    pub fn get_stakeholder_suggestions(
        &self,
        account_id: &str,
    ) -> Result<Vec<StakeholderSuggestionRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT ss.id, ss.account_id, ss.person_id, ss.suggested_name, ss.suggested_email,
                    ss.suggested_role, ss.suggested_engagement, ss.source, ss.status, ss.created_at
             FROM stakeholder_suggestions ss
             WHERE ss.account_id = ?1 AND ss.status = 'pending'
               AND ss.created_at >= datetime('now', '-3 months')
               AND (ss.person_id IS NULL OR NOT EXISTS (
                 SELECT 1 FROM people p WHERE p.id = ss.person_id AND p.relationship = 'internal'
               ))
               AND (ss.suggested_email IS NULL OR NOT EXISTS (
                 SELECT 1 FROM people p2 WHERE p2.email = LOWER(ss.suggested_email) AND p2.relationship = 'internal'
               ))
               AND (ss.suggested_name IS NULL OR NOT EXISTS (
                 SELECT 1 FROM people p3 WHERE LOWER(p3.name) = LOWER(ss.suggested_name) AND p3.relationship = 'internal'
               ))
             ORDER BY ss.created_at DESC",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(StakeholderSuggestionRow {
                id: row.get(0)?,
                account_id: row.get(1)?,
                person_id: row.get(2)?,
                suggested_name: row.get(3)?,
                suggested_email: row.get(4)?,
                suggested_role: row.get(5)?,
                suggested_engagement: row.get(6)?,
                source: row.get(7)?,
                status: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get a single stakeholder suggestion by ID (I652 phase 2).
    pub fn get_stakeholder_suggestion(
        &self,
        suggestion_id: i64,
    ) -> Result<Option<StakeholderSuggestionRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, person_id, suggested_name, suggested_email,
                    suggested_role, suggested_engagement, source, status, created_at
             FROM stakeholder_suggestions
             WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![suggestion_id], |row| {
            Ok(StakeholderSuggestionRow {
                id: row.get(0)?,
                account_id: row.get(1)?,
                person_id: row.get(2)?,
                suggested_name: row.get(3)?,
                suggested_email: row.get(4)?,
                suggested_role: row.get(5)?,
                suggested_engagement: row.get(6)?,
                source: row.get(7)?,
                status: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
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
    ///
    /// DOS-232 Codex follow-up: apply the same 0.70 accepted-confidence
    /// floor used by `get_meetings_for_account_with_prep` and the
    /// `get_total_meeting_count_for_account` helpers. Without it, the
    /// account chat LLM context, dossier markdown generation, and email
    /// prep enrichment all ingested speculative domain-match junctions
    /// (confidence < 0.70) as if they were real meetings — contaminating
    /// account reasoning with meetings the classifier never accepted.
    /// 0.70 matches the classifier promotion threshold
    /// (google_api/classify.rs).
    pub fn get_meetings_for_account(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id, mt.transcript_path
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1
               AND me.entity_type = 'account'
               AND me.confidence >= 0.70
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

    /// DOS-233 Codex fix: total count of meetings linked to an account above
    /// the accepted-confidence floor (0.70). Used by the About-this-dossier
    /// counts so active accounts don't stall at "10 meetings on record" —
    /// the preview list still caps at 10, but totals come from COUNT(*)
    /// without a LIMIT. Matches the visibility rule used by
    /// `get_meetings_for_account_with_prep`: include every junction above
    /// the floor regardless of `is_primary`.
    pub fn get_total_meeting_count_for_account(&self, account_id: &str) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                 FROM meeting_entities me
                 WHERE me.entity_id = ?1
                   AND me.entity_type = 'account'
                   AND me.confidence >= 0.70",
                params![account_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(DbError::from)
    }

    /// DOS-233 Codex fix: total count of transcripts for meetings linked to
    /// an account above the accepted-confidence floor. Joined through
    /// meeting_transcripts (a meeting only counts when it has a transcript
    /// row AND a non-null transcript_path).
    pub fn get_total_transcript_count_for_account(&self, account_id: &str) -> Result<i64, DbError> {
        self.conn
            .query_row(
                "SELECT COUNT(DISTINCT m.id)
                 FROM meetings m
                 INNER JOIN meeting_entities me ON m.id = me.meeting_id
                 INNER JOIN meeting_transcripts mt ON mt.meeting_id = m.id
                 WHERE me.entity_id = ?1
                   AND me.entity_type = 'account'
                   AND me.confidence >= 0.70
                   AND mt.transcript_path IS NOT NULL",
                params![account_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(DbError::from)
    }

    /// Get past meetings for an account with prep context (ADR-0063).
    /// Used only on account detail page where prep preview cards are needed.
    pub fn get_meetings_for_account_with_prep(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        // DOS-232 Codex fix: account-specific The Record / recentMeetings
        // queries must include EVERY meeting whose junction to this account
        // is above the accepted-confidence floor (0.70), regardless of the
        // `is_primary` flag. DOS-224 intentionally persists exactly one
        // primary per meeting even when multiple accounts share that meeting
        // — so a secondary account at confidence 0.80 on a meeting where
        // another account is the primary would previously be hidden from its
        // own timeline/dossier counts. `is_primary` stays as the
        // meeting-chip prominence signal (used elsewhere), not as an account
        // visibility gate. The 0.70 floor — the same threshold the
        // classifier uses to promote all-internal meetings (see
        // google_api/classify.rs) — still filters out speculative
        // domain-match junctions.
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id, mp.prep_context_json
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1
               AND me.entity_type = 'account'
               AND me.confidence >= 0.70
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
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id, m.description
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
               AND julianday(m.start_time) >= julianday('now')
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
                description: row.get(10)?,
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
        // JSON-typed columns: validate non-empty values parse as JSON to prevent
        // silently storing corrupt data that later fails parse-on-read.
        if matches!(field, "strategic_programs" | "company_overview") && !value.is_empty() {
            serde_json::from_str::<serde_json::Value>(value).map_err(|e| {
                DbError::Sqlite(rusqlite::Error::InvalidParameterName(format!(
                    "Invalid JSON for field '{}': {}",
                    field, e
                )))
            })?;
        }
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

                // Propagate account_type: if the child is internal, the parent should be too
                let child_type: String = self
                    .conn
                    .query_row(
                        "SELECT account_type FROM accounts WHERE id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .unwrap_or_else(|_| "customer".to_string());
                if child_type == "internal" {
                    self.conn.execute(
                        "UPDATE accounts SET account_type = 'internal', is_internal = 1, updated_at = ?2 WHERE id = ?1 AND account_type != 'internal'",
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
            "account_type" => {
                "UPDATE accounts SET account_type = ?1, is_internal = CASE WHEN ?1 = 'internal' THEN 1 ELSE 0 END, updated_at = ?3 WHERE id = ?2"
            }
            "user_health_sentiment" => {
                "UPDATE accounts SET user_health_sentiment = ?1, updated_at = ?3 WHERE id = ?2"
            }
            "sentiment_set_at" => {
                "UPDATE accounts SET sentiment_set_at = ?1, updated_at = ?3 WHERE id = ?2"
            }
            "tracker_path" => {
                "UPDATE accounts SET tracker_path = ?1, updated_at = ?3 WHERE id = ?2"
            }
            "notes" => {
                "UPDATE accounts SET notes = CASE WHEN ?1 = '' THEN NULL ELSE ?1 END, updated_at = ?3 WHERE id = ?2"
            }
            "strategic_programs" => {
                "UPDATE accounts SET strategic_programs = ?1, updated_at = ?3 WHERE id = ?2"
            }
            "company_overview" => {
                "UPDATE accounts SET company_overview = ?1, updated_at = ?3 WHERE id = ?2"
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

    /// Read the renewal stage for an account.
    pub fn get_account_renewal_stage(&self, account_id: &str) -> Result<Option<String>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT renewal_stage FROM accounts WHERE id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .optional()
            .map(|value| value.flatten())?)
    }

    /// Set the renewal stage for an account and update updated_at.
    pub fn set_account_renewal_stage(
        &self,
        account_id: &str,
        renewal_stage: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE accounts
             SET renewal_stage = ?1,
                 updated_at = ?3
             WHERE id = ?2",
            params![renewal_stage, account_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Persist provenance metadata for a tracked account field.
    pub fn set_account_field_provenance(
        &self,
        account_id: &str,
        field: &str,
        source: &str,
        updated_at: Option<&str>,
    ) -> Result<(), DbError> {
        let (source_col, updated_col) = match field {
            "arr" => ("arr_source", "arr_updated_at"),
            "lifecycle" => ("lifecycle_source", "lifecycle_updated_at"),
            "contract_end" => ("contract_end_source", "contract_end_updated_at"),
            "nps" => ("nps_source", "nps_updated_at"),
            _ => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("Field '{field}' does not support provenance"),
                )))
            }
        };

        let sql = format!(
            "UPDATE accounts SET {source_col} = ?1, {updated_col} = ?2, updated_at = ?4 WHERE id = ?3"
        );
        let provenance_updated_at = updated_at
            .map(str::to_string)
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        self.conn.execute(
            &sql,
            params![
                source,
                provenance_updated_at,
                account_id,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Fetch provenance metadata for tracked account vitals.
    pub fn get_account_field_provenance(
        &self,
        account_id: &str,
    ) -> Result<Vec<DbAccountFieldProvenance>, DbError> {
        let row = self.conn.query_row(
            "SELECT
                arr_source, arr_updated_at,
                lifecycle_source, lifecycle_updated_at,
                contract_end_source, contract_end_updated_at,
                nps_source, nps_updated_at
             FROM accounts
             WHERE id = ?1",
            params![account_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        )?;

        let mut result = Vec::new();
        let fields = [
            ("arr", row.0, row.1),
            ("lifecycle", row.2, row.3),
            ("contract_end", row.4, row.5),
            ("nps", row.6, row.7),
        ];
        for (field, source, updated_at) in fields {
            if let Some(source) = source {
                if !source.is_empty() {
                    result.push(DbAccountFieldProvenance {
                        field: field.to_string(),
                        source,
                        updated_at,
                    });
                }
            }
        }
        Ok(result)
    }

    /// Insert a lifecycle change log entry and return the new ID.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_lifecycle_change(
        &self,
        account_id: &str,
        previous_lifecycle: Option<&str>,
        new_lifecycle: &str,
        previous_stage: Option<&str>,
        new_stage: Option<&str>,
        previous_contract_end: Option<&str>,
        new_contract_end: Option<&str>,
        source: &str,
        confidence: f64,
        evidence: Option<&str>,
        health_score_before: Option<f64>,
        health_score_after: Option<f64>,
    ) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO lifecycle_changes (
                account_id, previous_lifecycle, new_lifecycle, previous_stage, new_stage,
                previous_contract_end, new_contract_end, source, confidence, evidence,
                health_score_before, health_score_after
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                account_id,
                previous_lifecycle,
                new_lifecycle,
                previous_stage,
                new_stage,
                previous_contract_end,
                new_contract_end,
                source,
                confidence,
                evidence,
                health_score_before,
                health_score_after,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Update the user response for a lifecycle change.
    pub fn set_lifecycle_change_response(
        &self,
        change_id: i64,
        user_response: &str,
        response_notes: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE lifecycle_changes
             SET user_response = ?1,
                 response_notes = ?2,
                 reviewed_at = datetime('now')
             WHERE id = ?3",
            params![user_response, response_notes, change_id],
        )?;
        Ok(())
    }

    /// Fetch recent lifecycle changes for an account, most recent first.
    pub fn get_account_lifecycle_changes(
        &self,
        account_id: &str,
        limit: usize,
    ) -> Result<Vec<DbLifecycleChange>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, account_id, previous_lifecycle, new_lifecycle, previous_stage, new_stage,
                previous_contract_end, new_contract_end, source, confidence, evidence,
                health_score_before, health_score_after, user_response, response_notes,
                created_at, reviewed_at
             FROM lifecycle_changes
             WHERE account_id = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![account_id, limit as i64], |row| {
            Ok(DbLifecycleChange {
                id: row.get(0)?,
                account_id: row.get(1)?,
                previous_lifecycle: row.get(2)?,
                new_lifecycle: row.get(3)?,
                previous_stage: row.get(4)?,
                new_stage: row.get(5)?,
                previous_contract_end: row.get(6)?,
                new_contract_end: row.get(7)?,
                source: row.get(8)?,
                confidence: row.get(9)?,
                evidence: row.get(10)?,
                health_score_before: row.get(11)?,
                health_score_after: row.get(12)?,
                user_response: row.get(13)?,
                response_notes: row.get(14)?,
                created_at: row.get(15)?,
                reviewed_at: row.get(16)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Fetch a single lifecycle change by ID.
    pub fn get_lifecycle_change(
        &self,
        change_id: i64,
    ) -> Result<Option<DbLifecycleChange>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, account_id, previous_lifecycle, new_lifecycle, previous_stage, new_stage,
                previous_contract_end, new_contract_end, source, confidence, evidence,
                health_score_before, health_score_after, user_response, response_notes,
                created_at, reviewed_at
             FROM lifecycle_changes
             WHERE id = ?1",
        )?;
        stmt.query_row(params![change_id], |row| {
            Ok(DbLifecycleChange {
                id: row.get(0)?,
                account_id: row.get(1)?,
                previous_lifecycle: row.get(2)?,
                new_lifecycle: row.get(3)?,
                previous_stage: row.get(4)?,
                new_stage: row.get(5)?,
                previous_contract_end: row.get(6)?,
                new_contract_end: row.get(7)?,
                source: row.get(8)?,
                confidence: row.get(9)?,
                evidence: row.get(10)?,
                health_score_before: row.get(11)?,
                health_score_after: row.get(12)?,
                user_response: row.get(13)?,
                response_notes: row.get(14)?,
                created_at: row.get(15)?,
                reviewed_at: row.get(16)?,
            })
        })
        .optional()
        .map_err(DbError::from)
    }

    /// Fetch recent lifecycle changes for dashboard briefing consumption.
    pub fn get_recent_lifecycle_changes(
        &self,
        limit: usize,
    ) -> Result<Vec<DbLifecycleChange>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, account_id, previous_lifecycle, new_lifecycle, previous_stage, new_stage,
                previous_contract_end, new_contract_end, source, confidence, evidence,
                health_score_before, health_score_after, user_response, response_notes,
                created_at, reviewed_at
             FROM lifecycle_changes
             WHERE created_at >= datetime('now', '-7 days')
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(DbLifecycleChange {
                id: row.get(0)?,
                account_id: row.get(1)?,
                previous_lifecycle: row.get(2)?,
                new_lifecycle: row.get(3)?,
                previous_stage: row.get(4)?,
                new_stage: row.get(5)?,
                previous_contract_end: row.get(6)?,
                new_contract_end: row.get(7)?,
                source: row.get(8)?,
                confidence: row.get(9)?,
                evidence: row.get(10)?,
                health_score_before: row.get(11)?,
                health_score_after: row.get(12)?,
                user_response: row.get(13)?,
                response_notes: row.get(14)?,
                created_at: row.get(15)?,
                reviewed_at: row.get(16)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get products for an account, ordered by source confidence then name.
    pub fn get_account_products(&self, account_id: &str) -> Result<Vec<DbAccountProduct>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, name, category, status, arr_portion, source, confidence, notes,
                    product_type, tier, billing_terms, arr, last_verified_at, data_source,
                    created_at, updated_at
             FROM account_products
             WHERE account_id = ?1
             ORDER BY confidence DESC, lower(name) ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountProduct {
                id: row.get(0)?,
                account_id: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                status: row.get(4)?,
                arr_portion: row.get(5)?,
                source: row.get(6)?,
                confidence: row.get(7)?,
                notes: row.get(8)?,
                product_type: row.get(9)?,
                tier: row.get(10)?,
                billing_terms: row.get(11)?,
                arr: row.get(12)?,
                last_verified_at: row.get(13)?,
                data_source: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Insert or update a product for an account using source-priority merge logic.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_account_product(
        &self,
        account_id: &str,
        name: &str,
        category: Option<&str>,
        status: &str,
        arr_portion: Option<f64>,
        source: &str,
        confidence: f64,
        notes: Option<&str>,
    ) -> Result<i64, DbError> {
        let source_priority = |value: &str| match value {
            "user_correction" => 3,
            "glean" => 2,
            _ => 1,
        };
        let existing = self.conn.query_row(
            "SELECT id, source FROM account_products WHERE account_id = ?1 AND lower(name) = lower(?2) LIMIT 1",
            params![account_id, name],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        ).optional()?;

        let now = Utc::now().to_rfc3339();
        match existing {
            Some((id, existing_source)) => {
                if source_priority(source) >= source_priority(&existing_source) {
                    self.conn.execute(
                        "UPDATE account_products
                         SET category = ?1,
                             status = ?2,
                             arr_portion = ?3,
                             source = ?4,
                             confidence = ?5,
                             notes = COALESCE(?6, notes),
                             updated_at = ?8,
                             name = ?7
                         WHERE id = ?9",
                        params![
                            category,
                            status,
                            arr_portion,
                            source,
                            confidence,
                            notes,
                            name,
                            now,
                            id,
                        ],
                    )?;
                }
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO account_products (
                        account_id, name, category, status, arr_portion, source, confidence, notes, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![account_id, name, category, status, arr_portion, source, confidence, notes, now],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    /// Update a specific product row.
    pub fn update_account_product(
        &self,
        product_id: i64,
        name: &str,
        status: Option<&str>,
        notes: Option<&str>,
        source: &str,
        confidence: f64,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE account_products
             SET name = ?1,
                 status = COALESCE(?2, status),
                 notes = COALESCE(?3, notes),
                 source = ?4,
                 confidence = ?5,
                 updated_at = ?6
             WHERE id = ?7",
            params![
                name,
                status,
                notes,
                source,
                confidence,
                Utc::now().to_rfc3339(),
                product_id,
            ],
        )?;
        Ok(())
    }

    /// Delete a product row.
    pub fn delete_account_product(&self, product_id: i64) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM account_products WHERE id = ?1",
            params![product_id],
        )?;
        Ok(())
    }

    /// I651: Upsert product classification from Glean/Salesforce.
    /// Uses (account_id, product_type, data_source) as the upsert key.
    /// Idempotent: calling twice with same data produces one row.
    pub fn upsert_product_classification(
        &self,
        account_id: &str,
        product_type: &str,
        tier: Option<&str>,
        arr: Option<f64>,
        billing_terms: Option<&str>,
        data_source: &str,
    ) -> Result<i64, DbError> {
        let now = Utc::now().to_rfc3339();
        let existing = self
            .conn
            .query_row(
                "SELECT id FROM account_products
             WHERE account_id = ?1 AND product_type = ?2 AND data_source = ?3 LIMIT 1",
                params![account_id, product_type, data_source],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        match existing {
            Some(id) => {
                // Update existing row
                self.conn.execute(
                    "UPDATE account_products
                     SET tier = ?1,
                         arr = ?2,
                         billing_terms = ?3,
                         last_verified_at = ?4,
                         updated_at = ?4,
                         status = 'active',
                         confidence = 0.95
                     WHERE id = ?5",
                    params![tier, arr, billing_terms, now, id],
                )?;
                Ok(id)
            }
            None => {
                // Insert new row
                self.conn.execute(
                    "INSERT INTO account_products (
                        account_id, name, category, status, arr_portion,
                        source, confidence, notes,
                        product_type, tier, billing_terms, arr, last_verified_at, data_source,
                        created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
                    params![
                        account_id,
                        product_type,                           // name
                        product_type,                           // category
                        "active",                               // status
                        arr,                                    // arr_portion
                        "glean",                                // source
                        0.95,                                   // confidence (high for Glean)
                        None::<&str>,                           // notes
                        product_type,                           // product_type
                        tier,                                   // tier
                        billing_terms,                          // billing_terms
                        arr,                                    // arr (product-specific)
                        now,                                    // last_verified_at
                        data_source,                            // data_source
                        now,                                    // created_at & updated_at
                    ],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
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

        // DOS-225: Path A — flip meetings to internal only when EVERY attendee
        // is accounted for as internal.
        //
        // The prior version joined `meeting_attendees` → `people` and required
        // "zero joined attendees with non-internal relationship". That was
        // broken: if a meeting had one internal attendee (joined) and one
        // external attendee that never got a `people` row (e.g., a Gmail
        // address or a new domain), the JOIN silently dropped the external
        // row and the aggregate said "all internal" → the meeting flipped.
        //
        // The fix requires full coverage. For each candidate meeting we read
        // `meetings.attendees` (canonical attendee JSON) and ensure EVERY
        // parseable email is either:
        //   (a) already resolved via `meeting_attendees` / `people` as
        //       `relationship = 'internal'`, OR
        //   (b) its domain matches a user-owned domain (config fallback).
        // Any unresolved external email blocks the flip. Meetings with no
        // attendees at all (attendees JSON empty / missing) are skipped —
        // we can't judge coverage without attendees.
        let user_domains_for_path_a: Vec<String> = crate::state::load_config()
            .map(|c| c.resolved_user_domains())
            .unwrap_or_default();

        let mut path_a_stmt = self.conn.prepare(
            "SELECT id, attendees FROM meetings
             WHERE meeting_type IN ('customer', 'external', 'one_on_one')
               AND attendees IS NOT NULL
               AND attendees != ''",
        )?;
        let path_a_rows = path_a_stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(path_a_stmt);

        let mut internal_emails_stmt = self.conn.prepare(
            "SELECT LOWER(p.email)
             FROM meeting_attendees ma
             JOIN people p ON ma.person_id = p.id
             WHERE ma.meeting_id = ?1
               AND p.relationship = 'internal'
               AND p.email IS NOT NULL
               AND p.email != ''",
        )?;

        let mut path_a_update_stmt = self
            .conn
            .prepare("UPDATE meetings SET meeting_type = 'internal' WHERE id = ?1")?;

        for (meeting_id, attendees_str) in path_a_rows {
            let attendees: Vec<String> = match serde_json::from_str::<Vec<String>>(&attendees_str) {
                Ok(arr) => arr,
                Err(_) => attendees_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            };

            // Only parseable emails participate in the coverage check.
            let emails: Vec<String> = attendees
                .iter()
                .filter(|a| a.contains('@'))
                .map(|a| a.to_lowercase())
                .collect();
            if emails.is_empty() {
                continue;
            }

            // Collect internal emails known via the junction.
            let internal_from_people: std::collections::HashSet<String> = internal_emails_stmt
                .query_map(params![meeting_id], |row| row.get::<_, String>(0))?
                .filter_map(Result::ok)
                .collect();

            // Every attendee email must be resolvable as internal — either
            // known-internal via people, or domain-owned via config.
            let all_internal = emails.iter().all(|email| {
                if internal_from_people.contains(email) {
                    return true;
                }
                let domain = email.split('@').nth(1).unwrap_or("");
                !domain.is_empty()
                    && user_domains_for_path_a
                        .iter()
                        .any(|d| !d.is_empty() && domain.eq_ignore_ascii_case(d))
            });

            if all_internal {
                path_a_update_stmt.execute(params![meeting_id])?;
                total += 1;
            }
        }

        // DOS-206: Path B — via raw attendee email strings vs user_domains.
        // The people-join query above misses meetings where attendees are
        // stored only in the `meetings.attendees` JSON / CSV blob and have
        // never been materialized into the `people` table. We need a second
        // sweep that parses the raw attendee strings and flips meetings
        // where every parseable email ends in a user-owned domain.
        //
        // The config's user_domains is the source of truth; we load it in
        // Rust and iterate, rather than trying to SQL-parse attendees.
        let user_domains: Vec<String> = crate::state::load_config()
            .map(|c| c.resolved_user_domains())
            .unwrap_or_default();

        if !user_domains.is_empty() {
            // Fetch candidate meetings that are currently customer/external/one_on_one.
            let mut stmt = self.conn.prepare(
                "SELECT id, attendees FROM meetings
                 WHERE meeting_type IN ('customer', 'external', 'one_on_one')
                   AND attendees IS NOT NULL
                   AND attendees != ''",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            drop(stmt);

            let mut update_stmt = self
                .conn
                .prepare("UPDATE meetings SET meeting_type = 'internal' WHERE id = ?1")?;

            for (id, attendees_str) in rows {
                let attendees: Vec<String> =
                    match serde_json::from_str::<Vec<String>>(&attendees_str) {
                        Ok(arr) => arr,
                        Err(_) => attendees_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                    };

                // Parseable emails only. If no emails at all, skip (can't judge).
                let emails: Vec<String> = attendees
                    .iter()
                    .filter(|a| a.contains('@'))
                    .map(|a| a.to_lowercase())
                    .collect();
                if emails.is_empty() {
                    continue;
                }

                // Every parseable email must end in a user_domain → all-internal.
                let all_internal = emails.iter().all(|e| {
                    let domain = e.split('@').nth(1).unwrap_or("");
                    user_domains
                        .iter()
                        .any(|d| !d.is_empty() && domain.eq_ignore_ascii_case(d))
                });

                if all_internal {
                    update_stmt.execute(params![id])?;
                    total += 1;
                }
            }
        }

        // Meetings classified as internal but ANY attendee is now external → customer
        total += self.conn.execute(
            "UPDATE meetings SET meeting_type = 'customer'
             WHERE meeting_type = 'internal'
               AND id IN (
                   SELECT DISTINCT ma.meeting_id
                   FROM meeting_attendees ma
                   JOIN people p ON ma.person_id = p.id
                   WHERE p.relationship = 'external'
               )",
            [],
        )?;

        // DOS-258 Tier 2: weak-primary flip. If a meeting is still labelled
        // `customer` but its linked primary account has ZERO stakeholder or
        // domain evidence connecting the attendee list to that account,
        // the primary was elected on weak signal (e.g. a pre-fix P5 title
        // match). Flip the classification back to `internal` so the UI
        // stops promising customer context it can't deliver.
        //
        // Title-derived types (qbr, training, all_hands, team_sync, personal,
        // external, one_on_one) are left alone — their classification does
        // not depend on the linked-account evidence strength.
        let weak_primary_rows: Vec<(String, String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT m.id, m.attendees, ler.entity_id \
                 FROM meetings m \
                 JOIN linked_entities_raw ler ON ler.owner_id = m.id \
                 WHERE m.meeting_type = 'customer' \
                   AND ler.owner_type = 'meeting' \
                   AND ler.role = 'primary' \
                   AND ler.entity_type = 'account' \
                   AND m.attendees IS NOT NULL AND m.attendees != ''",
            )?;
            let rows: Vec<_> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };

        let mut weak_flip_stmt = self
            .conn
            .prepare("UPDATE meetings SET meeting_type = 'internal' WHERE id = ?1")?;

        for (meeting_id, attendees_str, account_id) in weak_primary_rows {
            let attendees: Vec<String> = match serde_json::from_str::<Vec<String>>(&attendees_str) {
                Ok(arr) => arr,
                Err(_) => attendees_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            };
            let attendee_emails: Vec<String> = attendees
                .iter()
                .filter(|a| a.contains('@'))
                .map(|a| a.to_lowercase())
                .collect();
            if attendee_emails.is_empty() {
                continue;
            }

            // Load this account's registered domains once per meeting.
            let account_domains: Vec<String> = {
                let mut dom_stmt = self
                    .conn
                    .prepare("SELECT domain FROM account_domains WHERE account_id = ?1")?;
                let doms: Vec<String> = dom_stmt
                    .query_map(params![account_id], |row| row.get::<_, String>(0))?
                    .filter_map(Result::ok)
                    .map(|d| d.to_lowercase())
                    .collect();
                doms
            };

            // Domain evidence: any attendee whose domain maps to this account.
            let has_domain_match = attendee_emails.iter().any(|email| {
                email
                    .split('@')
                    .nth(1)
                    .map(|d| account_domains.iter().any(|ad| ad == d))
                    .unwrap_or(false)
            });

            if has_domain_match {
                continue;
            }

            // Stakeholder evidence: any attendee is an active stakeholder.
            let mut stake_stmt = self.conn.prepare(
                "SELECT 1 FROM account_stakeholders as_ \
                 JOIN people p ON p.id = as_.person_id \
                 WHERE as_.account_id = ?1 \
                   AND as_.status = 'active' \
                   AND LOWER(p.email) = ?2 \
                 LIMIT 1",
            )?;
            let mut has_stakeholder_match = false;
            for email in &attendee_emails {
                let exists: Option<i64> = stake_stmt
                    .query_row(params![account_id, email], |row| row.get(0))
                    .ok();
                if exists.is_some() {
                    has_stakeholder_match = true;
                    break;
                }
            }

            if !has_stakeholder_match {
                weak_flip_stmt.execute(params![meeting_id])?;
                total += 1;
            }
        }

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
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
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
                "SELECT COUNT(*) FROM meetings m
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
                "SELECT COUNT(*) FROM meetings m
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
                "SELECT MAX(m.start_time) FROM meetings m
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
                 WHERE project_id = ?1 AND status IN ('backlog', 'unstarted', 'started')",
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

    /// The column list shared by all account SELECT queries (I644).
    const ACCOUNT_COLUMNS: &'static str =
        "id, name, lifecycle, arr, health, contract_start, contract_end, \
         nps, tracker_path, parent_id, account_type, updated_at, archived, \
         keywords, keywords_extracted_at, metadata, commercial_stage, \
         arr_range_low, arr_range_high, \
         renewal_likelihood, renewal_likelihood_source, renewal_likelihood_updated_at, \
         renewal_model, renewal_pricing_method, \
         support_tier, support_tier_source, support_tier_updated_at, \
         active_subscription_count, \
         growth_potential_score, growth_potential_score_source, \
         icp_fit_score, icp_fit_score_source, \
         primary_product, \
         customer_status, customer_status_source, customer_status_updated_at, \
         company_overview, strategic_programs, notes, \
         user_health_sentiment, sentiment_set_at";

    /// Helper: prefix every column in ACCOUNT_COLUMNS with a table alias.
    fn account_columns_aliased(alias: &str) -> String {
        Self::ACCOUNT_COLUMNS
            .split(", ")
            .map(|col| format!("{}.{}", alias, col.trim()))
            .collect::<Vec<_>>()
            .join(", ")
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
            account_type: AccountType::from_db(
                &row.get::<_, String>(10)
                    .unwrap_or_else(|_| "customer".to_string()),
            ),
            updated_at: row.get(11)?,
            archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
            keywords: row.get(13).unwrap_or(None),
            keywords_extracted_at: row.get(14).unwrap_or(None),
            metadata: row.get(15).unwrap_or(None),
            commercial_stage: row.get(16).unwrap_or(None),
            // I644 fact columns (migration 082)
            arr_range_low: row.get(17).unwrap_or(None),
            arr_range_high: row.get(18).unwrap_or(None),
            renewal_likelihood: row.get(19).unwrap_or(None),
            renewal_likelihood_source: row.get(20).unwrap_or(None),
            renewal_likelihood_updated_at: row.get(21).unwrap_or(None),
            renewal_model: row.get(22).unwrap_or(None),
            renewal_pricing_method: row.get(23).unwrap_or(None),
            support_tier: row.get(24).unwrap_or(None),
            support_tier_source: row.get(25).unwrap_or(None),
            support_tier_updated_at: row.get(26).unwrap_or(None),
            active_subscription_count: row.get(27).unwrap_or(None),
            growth_potential_score: row.get(28).unwrap_or(None),
            growth_potential_score_source: row.get(29).unwrap_or(None),
            icp_fit_score: row.get(30).unwrap_or(None),
            icp_fit_score_source: row.get(31).unwrap_or(None),
            primary_product: row.get(32).unwrap_or(None),
            customer_status: row.get(33).unwrap_or(None),
            customer_status_source: row.get(34).unwrap_or(None),
            customer_status_updated_at: row.get(35).unwrap_or(None),
            // I644: dashboard.json fields promoted to DB (migration 083)
            company_overview: row.get(36).unwrap_or(None),
            strategic_programs: row.get(37).unwrap_or(None),
            notes: row.get(38).unwrap_or(None),
            // DOS-110: User health sentiment
            user_health_sentiment: row.get(39).unwrap_or(None),
            sentiment_set_at: row.get(40).unwrap_or(None),
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

    /// Archive or unarchive a project. Cascade: archiving a parent archives all children (I388).
    pub fn archive_project(&self, id: &str, archived: bool) -> Result<usize, DbError> {
        let val = if archived { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "UPDATE projects SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![val, now, id],
        )?;

        // If archiving a parent, cascade to children
        if archived {
            self.conn.execute(
                "UPDATE projects SET archived = 1, updated_at = ?1 WHERE parent_id = ?2",
                params![now, id],
            )?;
        }

        Ok(changed)
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
            let actions_moved = conn
                .execute(
                    "UPDATE actions SET account_id = ?2 WHERE account_id = ?1",
                    params![from_id, into_id],
                )
                .map_err(|e| e.to_string())?;

            // Reassign meeting_entities (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE meeting_entities SET entity_id = ?2
                 WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id, into_id],
            )
            .map_err(|e| e.to_string())?;
            // Clean up remaining dupes
            let meetings_moved = conn
                .execute(
                    "DELETE FROM meeting_entities WHERE entity_id = ?1 AND entity_type = 'account'",
                    params![from_id],
                )
                .map_err(|e| e.to_string())?;

            // Reassign account_stakeholder_roles (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE account_stakeholder_roles SET account_id = ?2
                 WHERE account_id = ?1",
                params![from_id, into_id],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM account_stakeholder_roles WHERE account_id = ?1",
                params![from_id],
            )
            .map_err(|e| e.to_string())?;
            // Reassign account_stakeholders (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE account_stakeholders SET account_id = ?2
                 WHERE account_id = ?1",
                params![from_id, into_id],
            )
            .map_err(|e| e.to_string())?;
            let people_moved = conn
                .execute(
                    "DELETE FROM account_stakeholders WHERE account_id = ?1",
                    params![from_id],
                )
                .map_err(|e| e.to_string())?;

            // Reassign account_events
            let events_moved = conn
                .execute(
                    "UPDATE account_events SET account_id = ?2 WHERE account_id = ?1",
                    params![from_id, into_id],
                )
                .map_err(|e| e.to_string())?;

            // Reassign signal_events
            conn.execute(
                "UPDATE OR IGNORE signal_events SET entity_id = ?2
                 WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id, into_id],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM signal_events WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id],
            )
            .map_err(|e| e.to_string())?;

            // Reassign content_index
            conn.execute(
                "UPDATE OR IGNORE content_index SET entity_id = ?2
                 WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id, into_id],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM content_index WHERE entity_id = ?1 AND entity_type = 'account'",
                params![from_id],
            )
            .map_err(|e| e.to_string())?;

            // Reassign account_domains (ignore dupes)
            conn.execute(
                "UPDATE OR IGNORE account_domains SET account_id = ?2
                 WHERE account_id = ?1",
                params![from_id, into_id],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM account_domains WHERE account_id = ?1",
                params![from_id],
            )
            .map_err(|e| e.to_string())?;

            // Reassign children
            let children_moved = conn
                .execute(
                    "UPDATE accounts SET parent_id = ?2 WHERE parent_id = ?1",
                    params![from_id, into_id],
                )
                .map_err(|e| e.to_string())?;

            // Archive source account
            conn.execute(
                "UPDATE accounts SET archived = 1 WHERE id = ?1",
                params![from_id],
            )
            .map_err(|e| e.to_string())?;

            Ok(MergeResult {
                actions_moved,
                meetings_moved,
                people_moved,
                events_moved,
                children_moved,
            })
        })
        .map_err(DbError::Migration)
    }

    /// Get archived accounts (top-level + children).
    pub fn get_archived_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts WHERE archived = 1 ORDER BY name",
            Self::ACCOUNT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get archived projects.
    pub fn get_archived_projects(&self) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date, tracker_path, parent_id,
                    updated_at, archived, keywords, keywords_extracted_at, metadata
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
                 JOIN meetings mh ON ma.meeting_id = mh.id
                 WHERE mh.start_time >= datetime('now', '-30 days')
                 GROUP BY person_id
             ) m30 ON m30.person_id = p.id
             LEFT JOIN (
                 SELECT person_id, COUNT(*) as cnt
                 FROM meeting_attendees ma
                 JOIN meetings mh ON ma.meeting_id = mh.id
                 WHERE mh.start_time >= datetime('now', '-90 days')
                 GROUP BY person_id
             ) m90 ON m90.person_id = p.id
             LEFT JOIN (
                 SELECT as_.person_id, GROUP_CONCAT(a.name, ', ') AS names
                 FROM account_stakeholders as_
                 JOIN accounts a ON a.id = as_.account_id
                 GROUP BY as_.person_id
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
                days_since_last_meeting: None,
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

    // =========================================================================
    // Source References (I644)
    // =========================================================================

    /// Get all source references for an account, ordered by field then most recent.
    pub fn get_account_source_refs(
        &self,
        account_id: &str,
    ) -> Result<Vec<DbAccountSourceRef>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, field, source_system, source_kind, source_value, observed_at
             FROM account_source_refs
             WHERE account_id = ?1
             ORDER BY field, observed_at DESC",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountSourceRef {
                id: row.get(0)?,
                account_id: row.get(1)?,
                field: row.get(2)?,
                source_system: row.get(3)?,
                source_kind: row.get(4)?,
                source_value: row.get(5)?,
                observed_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // =========================================================================
    // Technical Footprint (I649)
    // =========================================================================

    /// Upsert the technical footprint row for an account.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_account_technical_footprint(
        &self,
        account_id: &str,
        integrations_json: Option<&str>,
        usage_tier: Option<&str>,
        adoption_score: Option<f64>,
        active_users: Option<i64>,
        support_tier: Option<&str>,
        csat_score: Option<f64>,
        open_tickets: i64,
        services_stage: Option<&str>,
        source: &str,
    ) -> Result<(), DbError> {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        self.conn.execute(
            "INSERT INTO account_technical_footprint
                (account_id, integrations_json, usage_tier, adoption_score, active_users,
                 support_tier, csat_score, open_tickets, services_stage, source, sourced_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
             ON CONFLICT(account_id) DO UPDATE SET
                integrations_json = COALESCE(excluded.integrations_json, account_technical_footprint.integrations_json),
                usage_tier = COALESCE(excluded.usage_tier, account_technical_footprint.usage_tier),
                adoption_score = COALESCE(excluded.adoption_score, account_technical_footprint.adoption_score),
                active_users = COALESCE(excluded.active_users, account_technical_footprint.active_users),
                support_tier = COALESCE(excluded.support_tier, account_technical_footprint.support_tier),
                csat_score = COALESCE(excluded.csat_score, account_technical_footprint.csat_score),
                open_tickets = excluded.open_tickets,
                services_stage = COALESCE(excluded.services_stage, account_technical_footprint.services_stage),
                source = excluded.source,
                sourced_at = excluded.sourced_at,
                updated_at = excluded.updated_at",
            params![
                account_id,
                integrations_json,
                usage_tier,
                adoption_score,
                active_users,
                support_tier,
                csat_score,
                open_tickets,
                services_stage,
                source,
                now,
            ],
        )?;
        Ok(())
    }

    /// DOS-231 Codex fix: update a single column on `account_technical_footprint`.
    /// Whitelisted to the gap-row fields surfaced by `AccountTechnicalFootprint`
    /// in chapter variant — other columns (source, timestamps, integrations_json)
    /// are not user-editable through this path.
    ///
    /// Creates the row if it does not yet exist (common for accounts Glean has
    /// not enriched) so the first user write can succeed without a separate
    /// bootstrap. Bumps `updated_at`; stamps `source = 'user_edit'` and
    /// `sourced_at = now` so downstream signal emitters see a fresh user-
    /// authored footprint.
    pub fn update_technical_footprint_field(
        &self,
        account_id: &str,
        field: &str,
        value: &str,
    ) -> Result<(), DbError> {
        // Whitelist + column binding. We never interpolate `field` from the
        // caller into SQL directly.
        enum Kind {
            Text,
            Integer,
            Real,
        }
        let (column, kind) = match field {
            "usage_tier" => ("usage_tier", Kind::Text),
            "services_stage" => ("services_stage", Kind::Text),
            "support_tier" => ("support_tier", Kind::Text),
            "active_users" => ("active_users", Kind::Integer),
            "open_tickets" => ("open_tickets", Kind::Integer),
            "csat_score" => ("csat_score", Kind::Real),
            "adoption_score" => ("adoption_score", Kind::Real),
            other => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("update_technical_footprint_field: unsupported field '{other}'"),
                )));
            }
        };

        // Ensure a row exists first (idempotent). INSERT-OR-IGNORE bootstraps
        // a blank row only when none exists; existing rows (with real
        // `open_tickets`, `adoption_score`, etc.) are left untouched because
        // the full-row `upsert_account_technical_footprint` would clobber
        // `open_tickets` on the UPDATE branch (its UPSERT clause writes
        // `open_tickets = excluded.open_tickets` unconditionally, so the
        // sentinel `0` passed here would silently wipe real support data
        // whenever a user edits `usage_tier`, `csat_score`, or any other
        // non-`open_tickets` gap field — DOS-231 Codex follow-up).
        let bootstrap_now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        self.conn.execute(
            "INSERT OR IGNORE INTO account_technical_footprint
                (account_id, integrations_json, usage_tier, adoption_score, active_users,
                 support_tier, csat_score, open_tickets, services_stage, source, sourced_at, updated_at)
             VALUES (?1, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, 'user_edit', ?2, ?2)",
            params![account_id, bootstrap_now],
        )?;

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let sql = format!(
            "UPDATE account_technical_footprint
             SET {column} = ?1,
                 source = 'user_edit',
                 sourced_at = ?2,
                 updated_at = ?2
             WHERE account_id = ?3"
        );
        match kind {
            Kind::Text => {
                self.conn.execute(&sql, params![value, now, account_id])?;
            }
            Kind::Integer => {
                let parsed: i64 = value.parse().map_err(|_| {
                    DbError::Sqlite(rusqlite::Error::InvalidParameterName(format!(
                        "update_technical_footprint_field: '{value}' is not a valid integer for {column}"
                    )))
                })?;
                self.conn.execute(&sql, params![parsed, now, account_id])?;
            }
            Kind::Real => {
                let parsed: f64 = value.parse().map_err(|_| {
                    DbError::Sqlite(rusqlite::Error::InvalidParameterName(format!(
                        "update_technical_footprint_field: '{value}' is not a valid number for {column}"
                    )))
                })?;
                self.conn.execute(&sql, params![parsed, now, account_id])?;
            }
        }
        Ok(())
    }

    /// Get the technical footprint for an account, if present.
    pub fn get_account_technical_footprint(
        &self,
        account_id: &str,
    ) -> Result<Option<DbAccountTechnicalFootprint>, DbError> {
        use rusqlite::OptionalExtension;
        self.conn
            .query_row(
                "SELECT account_id, integrations_json, usage_tier, adoption_score, active_users,
                        support_tier, csat_score, open_tickets, services_stage, source, sourced_at, updated_at
                 FROM account_technical_footprint
                 WHERE account_id = ?1",
                params![account_id],
                |row| {
                    Ok(DbAccountTechnicalFootprint {
                        account_id: row.get(0)?,
                        integrations_json: row.get(1)?,
                        usage_tier: row.get(2)?,
                        adoption_score: row.get(3)?,
                        active_users: row.get(4)?,
                        support_tier: row.get(5)?,
                        csat_score: row.get(6)?,
                        open_tickets: row.get::<_, i64>(7)?,
                        services_stage: row.get(8)?,
                        source: row.get(9)?,
                        sourced_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                },
            )
            .optional()
            .map_err(DbError::from)
    }

    /// Get accounts with renewal_date (contract_end) in the past and no churn event.
    pub fn get_accounts_past_renewal(&self) -> Result<Vec<DbAccount>, DbError> {
        let sql = format!(
            "SELECT {} FROM accounts a \
             WHERE a.contract_end IS NOT NULL \
               AND a.contract_end < date('now') \
               AND a.archived = 0 \
               AND a.id NOT IN ( \
                   SELECT account_id FROM account_events WHERE event_type = 'churn' \
               ) \
             ORDER BY a.contract_end",
            Self::account_columns_aliased("a")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // =========================================================================
    // Account fact upserts (I644)
    // =========================================================================

    /// Source priority for fact writes: user > salesforce > zendesk > glean > ai.
    fn source_priority(source: &str) -> i32 {
        match source {
            "user" | "user_correction" => 4,
            "salesforce" => 3,
            "zendesk" => 2,
            "glean" => 1,
            "ai" | "inference" => 0,
            _ => 0,
        }
    }

    /// Allowed field names for `upsert_account_fact` (I644).
    /// Each entry is (field, has_source, has_updated_at).
    const FACT_FIELDS: &'static [(&'static str, bool, bool)] = &[
        ("arr_range_low", false, false),
        ("arr_range_high", false, false),
        ("renewal_likelihood", true, true),
        ("renewal_model", false, false),
        ("renewal_pricing_method", false, false),
        ("support_tier", true, true),
        ("active_subscription_count", false, false),
        ("growth_potential_score", true, false),
        ("icp_fit_score", true, false),
        ("primary_product", false, false),
        ("customer_status", true, true),
        ("commercial_stage", false, false),
        ("renewal_stage", false, false),
    ];

    /// Update a single account fact field with source tracking (I644).
    ///
    /// Only overwrites if the new source priority >= existing source priority.
    /// Source priority: user (4) > salesforce (3) > zendesk (2) > glean (1) > ai (0).
    ///
    /// Returns `Ok(true)` if the field was updated, `Ok(false)` if skipped due to
    /// lower source priority.
    pub fn upsert_account_fact(
        &self,
        account_id: &str,
        field: &str,
        value: &str,
        source: &str,
        observed_at: &str,
    ) -> Result<bool, DbError> {
        // Whitelist check
        let spec = Self::FACT_FIELDS
            .iter()
            .find(|(name, _, _)| *name == field)
            .ok_or_else(|| {
                DbError::Migration(format!("upsert_account_fact: unknown field '{}'", field))
            })?;
        let has_source = spec.1;
        let has_updated_at = spec.2;

        let new_priority = Self::source_priority(source);

        // Check existing source priority (if this field tracks source)
        if has_source {
            let source_col = format!("{}_source", field);
            let existing_source: Option<String> = self
                .conn
                .query_row(
                    &format!("SELECT {} FROM accounts WHERE id = ?1", source_col),
                    params![account_id],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();

            if let Some(ref existing) = existing_source {
                if Self::source_priority(existing) > new_priority {
                    return Ok(false);
                }
            }
        }

        // Build the SET clause dynamically
        let mut set_parts = vec![format!("{} = ?1", field)];
        let mut param_idx = 2; // ?1 is value
        if has_source {
            set_parts.push(format!("{}_source = ?{}", field, param_idx));
            param_idx += 1;
        }
        if has_updated_at {
            set_parts.push(format!("{}_updated_at = ?{}", field, param_idx));
            param_idx += 1;
        }
        set_parts.push(format!("updated_at = ?{}", param_idx));
        let account_param = param_idx + 1;

        let sql = format!(
            "UPDATE accounts SET {} WHERE id = ?{}",
            set_parts.join(", "),
            account_param
        );

        let now = Utc::now().to_rfc3339();
        // Build params vector matching the SET clause order
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(value.to_string())); // ?1 = value
        if has_source {
            param_values.push(Box::new(source.to_string()));
        }
        if has_updated_at {
            param_values.push(Box::new(observed_at.to_string()));
        }
        param_values.push(Box::new(now)); // updated_at
        param_values.push(Box::new(account_id.to_string())); // WHERE id

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let rows_changed = self.conn.execute(&sql, params_refs.as_slice())?;
        Ok(rows_changed > 0)
    }

    /// Write a source reference row (I644).
    ///
    /// Records provenance for a fact value: which system provided it, when it was
    /// observed, and an optional reference to the source record.
    pub fn upsert_account_source_ref(
        &self,
        ref_data: &AccountSourceRef<'_>,
    ) -> Result<(), DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT OR REPLACE INTO account_source_refs
                (id, account_id, field, source_system, source_kind, source_value,
                 observed_at, source_record_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                ref_data.account_id,
                ref_data.field,
                ref_data.source_system,
                ref_data.source_kind,
                ref_data.source_value,
                ref_data.observed_at,
                ref_data.reference_id,
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // DOS-27: Sentiment journal
    // =========================================================================

    /// Insert a sentiment journal entry (value + optional note + timestamp).
    /// Computed band and score at set-time are stored for divergence analysis.
    pub fn insert_sentiment_journal_entry(
        &self,
        account_id: &str,
        sentiment: &str,
        note: Option<&str>,
        computed_band: Option<&str>,
        computed_score: Option<f64>,
    ) -> Result<(), DbError> {
        let note_clean = note.and_then(|n| {
            let trimmed = n.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        self.conn.execute(
            "INSERT INTO user_sentiment_history
                (account_id, sentiment, note, computed_band, computed_score)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                account_id,
                sentiment,
                note_clean,
                computed_band,
                computed_score
            ],
        )?;
        Ok(())
    }

    /// Get sentiment journal entries for an account within the last N days.
    /// Returns entries ordered newest-first.
    pub fn get_sentiment_history(
        &self,
        account_id: &str,
        days: i64,
    ) -> Result<Vec<DbSentimentJournalEntry>, DbError> {
        let cutoff = format!("-{} days", days);
        let mut stmt = self.conn.prepare(
            "SELECT sentiment, note, computed_band, computed_score, set_at
             FROM user_sentiment_history
             WHERE account_id = ?1
               AND set_at >= datetime('now', ?2)
             ORDER BY set_at DESC",
        )?;
        let rows = stmt.query_map(params![account_id, cutoff], |row| {
            Ok(DbSentimentJournalEntry {
                sentiment: row.get(0)?,
                note: row.get(1)?,
                computed_band: row.get(2)?,
                computed_score: row.get(3)?,
                set_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// DOS-228 Fix 1: Most recent note attached to the account's CURRENT
    /// sentiment value. If the account has no current sentiment, or no note
    /// has ever been recorded against that sentiment value, returns None.
    ///
    /// Previously this returned the newest non-null note regardless of the
    /// current `user_health_sentiment`, so a user could change sentiment
    /// from `at_risk` (with a note) to `on_track` (no note) and the stale
    /// at_risk note would still surface in the UI. The journal entry itself
    /// is preserved for history — this accessor just filters by the value
    /// that is currently set on the account.
    pub fn get_latest_sentiment_note(
        &self,
        account_id: &str,
    ) -> Result<Option<(String, String)>, DbError> {
        // Look up the account's current sentiment. No sentiment → no note.
        let current_sentiment: Option<String> = self
            .conn
            .query_row(
                "SELECT user_health_sentiment FROM accounts WHERE id = ?1",
                params![account_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();

        let Some(sentiment) = current_sentiment else {
            return Ok(None);
        };

        let result = self.conn.query_row(
            "SELECT note, set_at FROM user_sentiment_history
             WHERE account_id = ?1 AND sentiment = ?2 AND note IS NOT NULL
             ORDER BY set_at DESC LIMIT 1",
            params![account_id, sentiment],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// DOS-269: Update the note on the newest sentiment journal row whose
    /// `sentiment` matches the account's current `user_health_sentiment`.
    /// This is the "Add more detail" path — user is augmenting the existing
    /// journal entry rather than creating a new one. Returns `true` if a row
    /// was updated, `false` when there is no matching history row (caller
    /// should fall back to a fresh `insert_sentiment_journal_entry`).
    pub fn update_latest_sentiment_note(
        &self,
        account_id: &str,
        note: Option<&str>,
    ) -> Result<bool, DbError> {
        let current_sentiment: Option<String> = self
            .conn
            .query_row(
                "SELECT user_health_sentiment FROM accounts WHERE id = ?1",
                params![account_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let Some(sentiment) = current_sentiment else {
            return Ok(false);
        };
        let latest_rowid: Option<i64> = self
            .conn
            .query_row(
                "SELECT rowid FROM user_sentiment_history
                 WHERE account_id = ?1 AND sentiment = ?2
                 ORDER BY set_at DESC LIMIT 1",
                params![account_id, sentiment],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let Some(rowid) = latest_rowid else {
            return Ok(false);
        };
        let note_clean = note.and_then(|n| {
            let trimmed = n.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        self.conn.execute(
            "UPDATE user_sentiment_history SET note = ?1 WHERE rowid = ?2",
            params![note_clean, rowid],
        )?;
        Ok(true)
    }

    // =========================================================================
    // DOS-269: triage_snoozes — per-card dismissal persistence for Health tab
    // Snooze / Confirm-resolved actions. Keyed on (entity, triage_key).
    // =========================================================================

    /// Upsert a snooze for a triage card. `snoozed_until` is an ISO-8601 UTC
    /// timestamp; rendering-time filter hides cards whose snoozed_until is
    /// still in the future.
    pub fn snooze_triage_item(
        &self,
        entity_type: &str,
        entity_id: &str,
        triage_key: &str,
        snoozed_until: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO triage_snoozes (entity_type, entity_id, triage_key, snoozed_until, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(entity_type, entity_id, triage_key) DO UPDATE SET
                snoozed_until = excluded.snoozed_until,
                resolved_at   = NULL,
                updated_at    = datetime('now')",
            params![entity_type, entity_id, triage_key, snoozed_until],
        )?;
        Ok(())
    }

    /// Mark a triage card resolved. Resolution is permanent for the lifetime
    /// of the card key (stable until re-enrichment emits a new one).
    pub fn resolve_triage_item(
        &self,
        entity_type: &str,
        entity_id: &str,
        triage_key: &str,
    ) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO triage_snoozes (entity_type, entity_id, triage_key, resolved_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(entity_type, entity_id, triage_key) DO UPDATE SET
                resolved_at = excluded.resolved_at,
                updated_at  = datetime('now')",
            params![entity_type, entity_id, triage_key, now],
        )?;
        Ok(())
    }

    /// List active triage suppressions for an entity — returns (triage_key,
    /// snoozed_until, resolved_at) tuples so the frontend can decide
    /// card-by-card whether to hide. Returns all rows; callers filter by
    /// expiration so a snooze that's aged out becomes visible again.
    pub fn list_triage_snoozes(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<(String, Option<String>, Option<String>)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT triage_key, snoozed_until, resolved_at
             FROM triage_snoozes
             WHERE entity_type = ?1 AND entity_id = ?2",
        )?;
        let rows = stmt.query_map(params![entity_type, entity_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Return the daily-bucketed computed health scores for an account over
    /// the last N days, for sparkline rendering.
    /// Newest-last so the sparkline reads left-to-right chronologically.
    pub fn get_health_score_sparkline(
        &self,
        account_id: &str,
        days: i64,
    ) -> Result<Vec<DbHealthSparklinePoint>, DbError> {
        let cutoff = format!("-{} days", days);
        let mut stmt = self.conn.prepare(
            "SELECT date(computed_at) AS day,
                    AVG(score) AS avg_score,
                    (SELECT band FROM health_score_history h2
                     WHERE h2.account_id = h1.account_id
                       AND date(h2.computed_at) = date(h1.computed_at)
                     ORDER BY h2.computed_at DESC LIMIT 1) AS last_band
             FROM health_score_history h1
             WHERE account_id = ?1
               AND computed_at >= datetime('now', ?2)
             GROUP BY date(computed_at)
             ORDER BY day ASC",
        )?;
        let rows = stmt.query_map(params![account_id, cutoff], |row| {
            Ok(DbHealthSparklinePoint {
                day: row.get(0)?,
                score: row.get(1)?,
                band: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // =========================================================================
    // DOS-228 Fix 3: risk_briefing_jobs — persisted status for async risk
    // briefing generation. One row per account; the row is upserted at each
    // lifecycle transition (enqueued → running → complete|failed).
    // =========================================================================

    /// Mark a risk briefing as enqueued for `account_id` with a fresh
    /// `attempt_id`. Overwrites any prior job — only the latest attempt is
    /// retained. The caller (service layer) owns the returned attempt_id and
    /// must present it on every subsequent transition for compare-and-set.
    ///
    /// DOS-228 Wave 0e Fix 2: the attempt_id column lets a superseding
    /// retry invalidate a prior lifecycle runner's updates instead of
    /// last-write-wins corruption when two retries race.
    pub fn upsert_risk_briefing_job_enqueued(
        &self,
        account_id: &str,
        attempt_id: &str,
    ) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO risk_briefing_jobs
                (account_id, status, enqueued_at, completed_at, error_message, attempt_id)
             VALUES (?1, 'enqueued', ?2, NULL, NULL, ?3)
             ON CONFLICT(account_id) DO UPDATE SET
                status        = 'enqueued',
                enqueued_at   = excluded.enqueued_at,
                completed_at  = NULL,
                error_message = NULL,
                attempt_id    = excluded.attempt_id",
            params![account_id, now, attempt_id],
        )?;
        Ok(())
    }

    /// Compare-and-set transition to `running`. Returns true if the row's
    /// `attempt_id` still matches `attempt_id` — i.e. this runner still owns
    /// the lifecycle. Returns false if superseded (another retry has
    /// stamped a newer attempt_id), in which case the caller must exit
    /// without further writes.
    pub fn mark_risk_briefing_job_running(
        &self,
        account_id: &str,
        attempt_id: &str,
    ) -> Result<bool, DbError> {
        let affected = self.conn.execute(
            "UPDATE risk_briefing_jobs
                SET status = 'running', completed_at = NULL, error_message = NULL
              WHERE account_id = ?1 AND attempt_id = ?2",
            params![account_id, attempt_id],
        )?;
        Ok(affected > 0)
    }

    /// Compare-and-set terminal transition to `complete`. Returns true if
    /// this runner's attempt is still current.
    pub fn mark_risk_briefing_job_complete(
        &self,
        account_id: &str,
        attempt_id: &str,
    ) -> Result<bool, DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        let affected = self.conn.execute(
            "UPDATE risk_briefing_jobs
                SET status = 'complete', completed_at = ?2, error_message = NULL
              WHERE account_id = ?1 AND attempt_id = ?3",
            params![account_id, now, attempt_id],
        )?;
        Ok(affected > 0)
    }

    /// Compare-and-set terminal transition to `failed` with a truncated
    /// error message (>2000 chars are clipped to keep rows compact).
    pub fn mark_risk_briefing_job_failed(
        &self,
        account_id: &str,
        attempt_id: &str,
        error_message: &str,
    ) -> Result<bool, DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        let truncated: String = error_message.chars().take(2_000).collect();
        let affected = self.conn.execute(
            "UPDATE risk_briefing_jobs
                SET status = 'failed', completed_at = ?2, error_message = ?3
              WHERE account_id = ?1 AND attempt_id = ?4",
            params![account_id, now, truncated, attempt_id],
        )?;
        Ok(affected > 0)
    }

    /// Current status of the risk-briefing job, if any. Used by
    /// `retry_risk_briefing` to reject retries while a job is running.
    pub fn get_risk_briefing_job_status(
        &self,
        account_id: &str,
    ) -> Result<Option<String>, DbError> {
        let result = self.conn.query_row(
            "SELECT status FROM risk_briefing_jobs WHERE account_id = ?1",
            params![account_id],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Read the current (latest) job row for `account_id`.
    pub fn get_risk_briefing_job(
        &self,
        account_id: &str,
    ) -> Result<Option<DbRiskBriefingJob>, DbError> {
        let result = self.conn.query_row(
            "SELECT account_id, status, enqueued_at, completed_at, error_message
               FROM risk_briefing_jobs
              WHERE account_id = ?1",
            params![account_id],
            |row| {
                Ok(DbRiskBriefingJob {
                    account_id: row.get(0)?,
                    status: row.get(1)?,
                    enqueued_at: row.get(2)?,
                    completed_at: row.get(3)?,
                    error_message: row.get(4)?,
                })
            },
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    // =========================================================================
    // DOS-228 Wave 0e Fix 3: health_recompute_pending — durable debounce
    // marker. Scheduling a recompute writes a row; a successful recompute
    // clears it. Startup drains whatever survived a crash so committed
    // field edits never silently lose their health recompute.
    // =========================================================================

    /// Mark a pending health recompute for `account_id`. Idempotent — stamps
    /// `requested_at` on each call so diagnostics can show staleness.
    pub fn mark_health_recompute_pending(&self, account_id: &str) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO health_recompute_pending (account_id, requested_at)
             VALUES (?1, ?2)
             ON CONFLICT(account_id) DO UPDATE SET requested_at = excluded.requested_at",
            params![account_id, now],
        )?;
        Ok(())
    }

    /// Clear the pending marker for `account_id`. Called after a successful
    /// recompute so the drain on next startup won't redo the work.
    pub fn clear_health_recompute_pending(&self, account_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM health_recompute_pending WHERE account_id = ?1",
            params![account_id],
        )?;
        Ok(())
    }

    /// List all account_ids with a pending health recompute. Ordered by
    /// requested_at so startup drains oldest-first.
    pub fn list_health_recompute_pending(&self) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT account_id FROM health_recompute_pending ORDER BY requested_at ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

/// DOS-228 Fix 3: Status of a risk-briefing generation job.
/// Exposed via `get_account_detail` so the UI can render a "generating…"
/// indicator, a "retry" affordance on failure, or the timestamp of the last
/// successful generation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbRiskBriefingJob {
    pub account_id: String,
    /// One of: `enqueued`, `running`, `complete`, `failed`.
    pub status: String,
    pub enqueued_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

// Note: `attempt_id` is an internal CAS token (DOS-228 Wave 0e Fix 2) and is
// intentionally NOT exposed on `DbRiskBriefingJob` — the frontend only needs
// `status`/`enqueuedAt`/`completedAt`/`errorMessage` to render progress and
// retry affordance, and hiding the token prevents UI code from accidentally
// making it part of a retry contract.

/// One sentiment journal entry for API exposure.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbSentimentJournalEntry {
    pub sentiment: String,
    pub note: Option<String>,
    pub computed_band: Option<String>,
    pub computed_score: Option<f64>,
    pub set_at: String,
}

/// One daily sparkline point for the Health sentiment hero.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbHealthSparklinePoint {
    pub day: String,
    pub score: f64,
    pub band: String,
}

#[cfg(test)]
mod stakeholders_full_tests {
    use crate::db::test_utils::test_db;

    #[test]
    fn get_account_stakeholders_full_hydrates_roles_from_join() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, archived)
             VALUES ('acc-roles-join', 'Stakeholder Roles Account', '2026-05-01', 0)",
            [],
        )
        .expect("seed account");

        conn.execute(
            "INSERT INTO people (id, name, email, organization, role, relationship, updated_at)
             VALUES
                ('p-alpha', 'Alpha Person', 'alpha@example.com', 'Example', 'VP Success', 'external', '2026-05-01'),
                ('p-beta', 'Beta Person', 'beta@example.com', 'Example', 'Director', 'external', '2026-05-01'),
                ('p-internal', 'Internal Person', 'internal@example.com', 'Example', 'Teammate', 'internal', '2026-05-01'),
                ('p-noise', 'Noise Person', 'noreply@example.com', 'Example', 'Bot', 'external', '2026-05-01')",
            [],
        )
        .expect("seed people");

        conn.execute(
            "INSERT INTO account_stakeholders
                (account_id, person_id, data_source, status, confidence, created_at)
             VALUES
                ('acc-roles-join', 'p-alpha', 'user', 'active', 1.0, '2026-05-01'),
                ('acc-roles-join', 'p-beta', 'glean', 'active', 0.8, '2026-05-01'),
                ('acc-roles-join', 'p-internal', 'user', 'active', 1.0, '2026-05-01'),
                ('acc-roles-join', 'p-noise', 'user', 'active', 1.0, '2026-05-01')",
            [],
        )
        .expect("seed stakeholders");

        conn.execute(
            "INSERT INTO account_stakeholder_roles
                (account_id, person_id, role, data_source, created_at, dismissed_at)
             VALUES
                ('acc-roles-join', 'p-alpha', 'champion', 'user', '2026-05-01', NULL),
                ('acc-roles-join', 'p-alpha', 'technical', 'ai', '2026-05-01', NULL),
                ('acc-roles-join', 'p-alpha', 'former_champion', 'ai', '2026-05-01', '2026-05-02')",
            [],
        )
        .expect("seed stakeholder roles");

        let stakeholders = db
            .get_account_stakeholders_full("acc-roles-join")
            .expect("stakeholders");

        assert_eq!(
            stakeholders
                .iter()
                .map(|stakeholder| stakeholder.person_id.as_str())
                .collect::<Vec<_>>(),
            vec!["p-alpha", "p-beta"]
        );

        let alpha = &stakeholders[0];
        assert_eq!(alpha.stakeholder_role, "champion,technical");
        assert_eq!(
            alpha
                .roles
                .iter()
                .map(|role| (role.role.as_str(), role.data_source.as_str()))
                .collect::<Vec<_>>(),
            vec![("champion", "user"), ("technical", "ai")]
        );

        let beta = &stakeholders[1];
        assert_eq!(beta.stakeholder_role, "associated");
        assert!(beta.roles.is_empty());
    }
}

#[cfg(test)]
mod dos258_reclassify_tests {
    use crate::db::test_utils::test_db;
    use rusqlite::params;

    /// DOS-258 Tier 2: a meeting labelled `customer` whose linked primary
    /// account has zero attendee-level evidence (no domain match, no active
    /// stakeholder) must flip back to `internal` on re-classification.
    #[test]
    fn reclassify_weak_primary_customer_flips_to_internal() {
        let db = test_db();

        // Seed: one account "WordPress VIP" with its own domain wpvip.com
        // that does NOT appear among the meeting attendees.
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('acc-wp', 'WordPress VIP', '2026-01-01', 0)",
                [],
            )
            .expect("seed account");
        db.conn_ref()
            .execute(
                "INSERT INTO account_domains (account_id, domain) VALUES ('acc-wp', 'wpvip.com')",
                [],
            )
            .expect("seed account_domains");

        // Meeting classified as customer but attendee list has no wpvip.com
        // domain and no stakeholder on acc-wp. Pre-DOS-258 P5 title-only
        // could have elected this — the reclassifier must correct it.
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, attendees, created_at, calendar_event_id) \
                 VALUES ('m-weak', 'WordPress VIP Plan', 'customer', '2026-04-20', '[\"me@company.com\",\"jane@example.test\"]', '2026-04-20', 'ev1')",
                [],
            )
            .expect("seed meeting");
        db.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw \
                 (owner_type, owner_id, entity_id, entity_type, role, source, rule_id, graph_version, created_at) \
                 VALUES ('meeting', 'm-weak', 'acc-wp', 'account', 'primary', 'rule:P5', 'P5', 0, '2026-04-20')",
                [],
            )
            .expect("seed linked_entities_raw");

        let updated = db
            .reclassify_meeting_types_from_attendees()
            .expect("reclassify");
        assert!(updated >= 1, "expected at least one update, got {updated}");

        let meeting_type: String = db
            .conn_ref()
            .query_row(
                "SELECT meeting_type FROM meetings WHERE id = 'm-weak'",
                [],
                |row| row.get(0),
            )
            .expect("fetch meeting_type");
        assert_eq!(
            meeting_type, "internal",
            "weak-primary customer meeting should flip back to internal"
        );

        // Now seed a stakeholder link for jane@example.test on acc-wp and re-flip
        // the meeting to customer, then verify the reclassifier preserves it.
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, name, email, relationship, updated_at) \
                 VALUES ('p-jane', 'Jane', 'jane@example.test', 'external', '2026-04-20')",
                [],
            )
            .expect("seed person");
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source, status, confidence, created_at) \
                 VALUES ('acc-wp', 'p-jane', 'test', 'active', 1.0, '2026-04-20')",
                [],
            )
            .expect("seed stakeholder");
        db.conn_ref()
            .execute(
                "UPDATE meetings SET meeting_type = 'customer' WHERE id = 'm-weak'",
                [],
            )
            .expect("reset");

        db.reclassify_meeting_types_from_attendees()
            .expect("reclassify 2");
        let mt2: String = db
            .conn_ref()
            .query_row(
                "SELECT meeting_type FROM meetings WHERE id = 'm-weak'",
                [],
                |row| row.get(0),
            )
            .expect("fetch meeting_type 2");
        assert_eq!(
            mt2, "customer",
            "with stakeholder evidence the customer label must stick"
        );

        let _ = params![0]; // silence unused import if no params! used above
    }
}
