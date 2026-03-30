use super::*;

/// Result of a find-or-create person operation (I652).
#[derive(Debug)]
pub enum PersonResolution {
    /// Existing person found by email (primary or alias). Definitive match.
    FoundByEmail(DbPerson),
    /// Existing person found by name similarity. Needs user confirmation.
    FoundByName {
        person: DbPerson,
        confidence: f32,
        reason: String,
    },
    /// No match found — a new person was created.
    Created(DbPerson),
}

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

    /// Create a person from minimal Glean-sourced fields (I505).
    ///
    /// Idempotent: if the email already exists, returns Ok without changes
    /// (the existing person is left untouched — use `update_person_profile` for updates).
    pub fn create_person_minimal(
        &self,
        id: &str,
        email: &str,
        name: Option<&str>,
        title: Option<&str>,
        department: Option<&str>,
        location: Option<&str>,
    ) -> Result<(), DbError> {
        // Check if email already exists — idempotent guard
        if self.get_person_by_email(email)?.is_some() {
            return Ok(());
        }

        let now = chrono::Utc::now().to_rfc3339();

        // Build enrichment_sources JSON for each non-null field
        let mut sources = serde_json::Map::new();
        let source_entry = serde_json::json!({"source": "glean", "at": &now});
        if name.is_some() {
            sources.insert("name".into(), source_entry.clone());
        }
        if title.is_some() {
            sources.insert("role".into(), source_entry.clone());
        }
        if department.is_some() {
            sources.insert("organization".into(), source_entry.clone());
        }
        if location.is_some() {
            sources.insert("company_hq".into(), source_entry);
        }

        let enrichment_sources = if sources.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(sources).to_string())
        };

        let person = super::types::DbPerson {
            id: id.to_string(),
            email: email.to_lowercase(),
            name: name.unwrap_or(email).to_string(),
            organization: department.map(|s| s.to_string()),
            role: title.map(|s| s.to_string()),
            relationship: "unknown".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: Some(now.clone()),
            meeting_count: 0,
            updated_at: now,
            archived: false,
            linkedin_url: None,
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: None,
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: location.map(|s| s.to_string()),
            last_enriched_at: None,
            enrichment_sources,
        };

        self.upsert_person(&person)?;
        Ok(())
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
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                    linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                    company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
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
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived,
                    p.linkedin_url, p.twitter_handle, p.phone, p.photo_url, p.bio, p.title_history,
                    p.company_industry, p.company_size, p.company_hq, p.last_enriched_at, p.enrichment_sources
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
        let user_domains_lower: Vec<String> =
            user_domains.iter().map(|d| d.to_lowercase()).collect();
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

    // =========================================================================
    // Person resolution: find-or-create with email + name dedup (I652)
    // =========================================================================

    /// Find an existing person by email (primary + aliases) or name similarity,
    /// or create a new one. This is the canonical entry point for stakeholder
    /// suggestion acceptance and meeting attendee reconciliation.
    ///
    /// Resolution order:
    /// 1. Exact email match (primary `people.email`)
    /// 2. Alias email match (`person_emails` table)
    /// 3. Domain-alias match (`local_part@sibling_domain`)
    /// 4. Name similarity match (confidence ≥ 0.60 within same org/domain)
    /// 5. Create new person
    ///
    /// Steps 1-3 return `FoundByEmail` — the match is definitive.
    /// Step 4 returns `FoundByName` — the caller should confirm with the user.
    /// Step 5 returns `Created` — no match found.
    pub fn find_or_create_person(
        &self,
        email: Option<&str>,
        name: &str,
        organization: Option<&str>,
        relationship: &str,
        user_domains: &[String],
    ) -> Result<PersonResolution, DbError> {
        // --- Step 1-3: Email-based resolution (definitive) ---
        if let Some(email) = email {
            let email_lower = email.to_lowercase();

            // Step 1: Primary email match
            if let Some(person) = self.get_person_by_email(&email_lower)? {
                return Ok(PersonResolution::FoundByEmail(person));
            }

            // Step 2: Alias email match
            if let Some(person) = self.get_person_by_email_or_alias(&email_lower)? {
                return Ok(PersonResolution::FoundByEmail(person));
            }

            // Step 3: Domain-alias match (e.g., john@wpvip.com → john@a8c.com)
            let siblings = self.get_sibling_domains_for_email(&email_lower, user_domains)?;
            if !siblings.is_empty() {
                if let Some(person) = self.find_person_by_domain_alias(&email_lower, &siblings)? {
                    return Ok(PersonResolution::FoundByEmail(person));
                }
            }

            // No email match — fall through to name matching, then create
        }

        // --- Step 4: Name-based similarity (needs user confirmation) ---
        if !name.is_empty() && !name.contains('@') {
            let candidates = self.find_person_candidates_by_name(name, organization)?;
            if let Some((person, confidence, reason)) = candidates.into_iter().next() {
                return Ok(PersonResolution::FoundByName { person, confidence, reason });
            }
        }

        // --- Step 5: Create new person ---
        if let Some(email) = email {
            let id = crate::util::slugify(&format!(
                "{}-{}",
                name,
                &email[..email.find('@').unwrap_or(email.len())]
            ));
            let now = chrono::Utc::now().to_rfc3339();
            let person = DbPerson {
                id: id.clone(),
                email: email.to_lowercase(),
                name: name.to_string(),
                organization: organization.map(|s| s.to_string()),
                role: None,
                relationship: relationship.to_string(),
                notes: None,
                tracker_path: None,
                last_seen: None,
                first_seen: Some(now.clone()),
                meeting_count: 0,
                updated_at: now,
                archived: false,
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
            };
            self.upsert_person(&person)?;
            return Ok(PersonResolution::Created(person));
        }

        // Name only, no email — cannot create a person (email is NOT NULL)
        Err(DbError::Migration(
            "Cannot create person without email address".to_string(),
        ))
    }

    /// Find existing people whose names are similar to the given name.
    /// Returns matches sorted by confidence descending (highest first).
    ///
    /// Uses the same scoring algorithm as `hygiene/detectors::score_name_similarity`:
    /// - 0.95: Exact normalized name match
    /// - 0.70: Same first name + last initial
    /// - 0.60: First 3 chars of first+last match
    ///
    /// Optionally filters by organization to reduce false positives.
    pub fn find_person_candidates_by_name(
        &self,
        name: &str,
        organization: Option<&str>,
    ) -> Result<Vec<(DbPerson, f32, String)>, DbError> {
        // Get first+last parts for targeted SQL query
        let name_lower = name.trim().to_lowercase();
        let parts: Vec<&str> = name_lower.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(Vec::new());
        }

        // Query candidates using LIKE on the first name part (reduces scan)
        let first_part = parts[0];
        let pattern = format!("{}%", first_part);

        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                    linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                    company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
             FROM people
             WHERE archived = 0 AND LOWER(name) LIKE ?1
             ORDER BY name
             LIMIT 50",
        )?;
        let candidates: Vec<DbPerson> = stmt
            .query_map(params![pattern], Self::map_person_row)?
            .filter_map(|r| r.ok())
            .collect();

        let mut results: Vec<(DbPerson, f32, String)> = Vec::new();

        for person in candidates {
            // Score name similarity using the same algorithm as hygiene detectors
            if let Some((confidence, reason)) =
                crate::hygiene::detectors::score_name_similarity(name, &person.name)
            {
                // Only include matches ≥ 0.60
                if confidence >= 0.60 {
                    // Boost confidence if organizations match
                    let org_boost = match (organization, person.organization.as_deref()) {
                        (Some(org), Some(p_org))
                            if !org.is_empty()
                                && org.to_lowercase() == p_org.to_lowercase() =>
                        {
                            0.05
                        }
                        _ => 0.0,
                    };
                    let final_confidence = (confidence + org_boost).min(1.0);
                    results.push((person, final_confidence, reason));
                }
            }
        }

        // Sort by confidence descending
        results.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Get a person by ID.
    pub fn get_person(&self, id: &str) -> Result<Option<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                    linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                    company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
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
                            tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                            linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                            company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
                     FROM people WHERE relationship = ?1 AND archived = 0 ORDER BY name",
                )?;
                let rows = stmt.query_map(params![rel], Self::map_person_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, email, name, organization, role, relationship, notes,
                            tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                            linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                            company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
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
                       JOIN meetings m ON m.id = ma.meeting_id
                       WHERE m.start_time >= date('now', '-30 days') GROUP BY ma.person_id
                   ) cnt30 ON cnt30.person_id = p.id
                   LEFT JOIN (
                       SELECT ma.person_id, COUNT(*) AS c FROM meeting_attendees ma
                       JOIN meetings m ON m.id = ma.meeting_id
                       WHERE m.start_time >= date('now', '-90 days') GROUP BY ma.person_id
                   ) cnt90 ON cnt90.person_id = p.id
                   LEFT JOIN (
                       SELECT ma.person_id, MAX(m.start_time) AS max_start FROM meeting_attendees ma
                       JOIN meetings m ON m.id = ma.meeting_id
                       WHERE m.start_time <= datetime('now') GROUP BY ma.person_id
                   ) last_m ON last_m.person_id = p.id
                   LEFT JOIN (
                       SELECT as_.person_id, GROUP_CONCAT(a.name, ', ') AS names
                       FROM account_stakeholders as_
                       JOIN accounts a ON a.id = as_.account_id
                       GROUP BY as_.person_id
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
            let days_since_last_meeting = last_meeting
                .as_deref()
                .and_then(crate::db::types::days_since_iso);

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
                days_since_last_meeting,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get people linked to an entity (account/project).
    /// Routes to account_stakeholders for accounts, entity_members for projects.
    pub fn get_people_for_entity(&self, entity_id: &str) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived,
                    p.linkedin_url, p.twitter_handle, p.phone, p.photo_url, p.bio, p.title_history,
                    p.company_industry, p.company_size, p.company_hq, p.last_enriched_at, p.enrichment_sources
             FROM people p
             WHERE p.id IN (
                 SELECT as_.person_id FROM account_stakeholders as_ WHERE as_.account_id = ?1
                 UNION
                 SELECT em.person_id FROM entity_members em WHERE em.entity_id = ?1
             )
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map(params![entity_id], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get entities linked to a person.
    /// UNION across account_stakeholders (accounts) and entity_members (projects/other).
    pub fn get_entities_for_person(
        &self,
        person_id: &str,
    ) -> Result<Vec<crate::entity::DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, e.entity_type, e.tracker_path, e.updated_at
             FROM entities e
             WHERE e.id IN (
                 SELECT as_.account_id FROM account_stakeholders as_ WHERE as_.person_id = ?1
                 UNION
                 SELECT em.entity_id FROM entity_members em WHERE em.person_id = ?1
             )
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
    /// Routes to account_stakeholders for accounts, entity_members for projects/other.
    pub fn link_person_to_entity(
        &self,
        person_id: &str,
        entity_id: &str,
        rel: &str,
    ) -> Result<(), DbError> {
        // Check if entity_id is an account
        let is_account: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1)",
                params![entity_id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if is_account {
            self.conn.execute(
                "INSERT INTO account_stakeholders (account_id, person_id)
                 VALUES (?1, ?2)
                 ON CONFLICT(account_id, person_id) DO NOTHING",
                params![entity_id, person_id],
            )?;
            self.conn.execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(account_id, person_id, role) DO NOTHING",
                params![entity_id, person_id, rel],
            )?;
        } else {
            self.conn.execute(
                "INSERT OR IGNORE INTO entity_members (entity_id, person_id, relationship_type)
                 VALUES (?1, ?2, ?3)",
                params![entity_id, person_id, rel],
            )?;
        }
        Ok(())
    }

    /// Unlink a person from an entity.
    /// Deletes from both tables (only one will match per entity_id).
    pub fn unlink_person_from_entity(
        &self,
        person_id: &str,
        entity_id: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM account_stakeholder_roles WHERE account_id = ?1 AND person_id = ?2",
            params![entity_id, person_id],
        )?;
        self.conn.execute(
            "DELETE FROM account_stakeholders WHERE account_id = ?1 AND person_id = ?2",
            params![entity_id, person_id],
        )?;
        self.conn.execute(
            "DELETE FROM entity_members WHERE entity_id = ?1 AND person_id = ?2",
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
                    "SELECT start_time FROM meetings WHERE id = ?1",
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
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived,
                    p.linkedin_url, p.twitter_handle, p.phone, p.photo_url, p.bio, p.title_history,
                    p.company_industry, p.company_size, p.company_hq, p.last_enriched_at, p.enrichment_sources
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
            "SELECT DISTINCT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, mt.summary, m.created_at,
                    m.calendar_event_id, mt.transcript_path
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             LEFT JOIN meeting_attendees ma ON m.id = ma.meeting_id
             LEFT JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE ma.person_id = ?1
                OR (me.entity_type = 'person' AND me.entity_id = ?1)
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

    /// Compute person-level signals (meeting frequency, temperature, trend).
    pub fn get_person_signals(&self, person_id: &str) -> Result<PersonSignals, DbError> {
        let count_30d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT m.id) FROM meetings m
                 WHERE m.start_time >= date('now', '-30 days')
                   AND (
                        EXISTS (
                            SELECT 1 FROM meeting_attendees ma
                            WHERE ma.meeting_id = m.id AND ma.person_id = ?1
                        )
                        OR EXISTS (
                            SELECT 1 FROM meeting_entities me
                            WHERE me.meeting_id = m.id
                              AND me.entity_type = 'person'
                              AND me.entity_id = ?1
                        )
                   )",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let count_90d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT m.id) FROM meetings m
                 WHERE m.start_time >= date('now', '-90 days')
                   AND (
                        EXISTS (
                            SELECT 1 FROM meeting_attendees ma
                            WHERE ma.meeting_id = m.id AND ma.person_id = ?1
                        )
                        OR EXISTS (
                            SELECT 1 FROM meeting_entities me
                            WHERE me.meeting_id = m.id
                              AND me.entity_type = 'person'
                              AND me.entity_id = ?1
                        )
                   )",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let last_meeting: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(m.start_time) FROM meetings m
                 WHERE m.start_time <= datetime('now')
                   AND (
                        EXISTS (
                            SELECT 1 FROM meeting_attendees ma
                            WHERE ma.meeting_id = m.id AND ma.person_id = ?1
                        )
                        OR EXISTS (
                            SELECT 1 FROM meeting_entities me
                            WHERE me.meeting_id = m.id
                              AND me.entity_type = 'person'
                              AND me.entity_id = ?1
                        )
                   )",
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
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                    linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                    company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
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

    /// Stamp/override provenance for a single person field in `enrichment_sources`.
    pub fn set_person_field_source(
        &self,
        person_id: &str,
        field: &str,
        source: &str,
    ) -> Result<(), DbError> {
        if field.trim().is_empty() {
            return Err(DbError::Migration(
                "set_person_field_source: field cannot be empty".to_string(),
            ));
        }
        if source_priority(source) == 0 {
            return Err(DbError::Migration(format!(
                "set_person_field_source: unknown source '{}'",
                source
            )));
        }

        let current_sources_json: Option<String> = self
            .conn
            .query_row(
                "SELECT enrichment_sources FROM people WHERE id = ?1",
                params![person_id],
                |row| row.get(0),
            )
            .map_err(|e| DbError::Migration(format!("read enrichment_sources: {}", e)))?;

        let mut sources: std::collections::HashMap<String, FieldSource> = current_sources_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();

        let now = Utc::now().to_rfc3339();
        sources.insert(
            field.to_string(),
            FieldSource {
                source: source.to_string(),
                at: now.clone(),
            },
        );
        let sources_json = serde_json::to_string(&sources).unwrap_or_else(|_| "{}".to_string());
        self.conn.execute(
            "UPDATE people SET enrichment_sources = ?1, updated_at = ?2 WHERE id = ?3",
            params![sources_json, now, person_id],
        )?;
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
            linkedin_url: row.get(13)?,
            twitter_handle: row.get(14)?,
            phone: row.get(15)?,
            photo_url: row.get(16)?,
            bio: row.get(17)?,
            title_history: row.get(18)?,
            company_industry: row.get(19)?,
            company_size: row.get(20)?,
            company_hq: row.get(21)?,
            last_enriched_at: row.get(22)?,
            enrichment_sources: row.get(23)?,
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
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                    linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                    company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
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
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                    linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                    company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
             FROM people
             WHERE relationship = 'unknown'
             ORDER BY meeting_count DESC",
        )?;
        let rows = stmt.query_map([], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Entities with content files in content_index but no assessment cache row
    /// or NULL enriched_at. Returns (entity_id, entity_type) pairs.
    pub fn get_entities_without_intelligence(&self) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT ci.entity_id, ci.entity_type
             FROM content_index ci
             LEFT JOIN entity_assessment ei ON ei.entity_id = ci.entity_id
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
             FROM entity_assessment ei
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
            tx.conn
                .execute(
                    "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id)
                 SELECT meeting_id, ?1 FROM meeting_attendees WHERE person_id = ?2",
                    params![keep_id, remove_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM meeting_attendees WHERE person_id = ?1",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;

            // 2. Transfer account_stakeholders links
            tx.conn
                .execute(
                    "INSERT OR IGNORE INTO account_stakeholders (account_id, person_id, data_source)
                 SELECT account_id, ?1, data_source FROM account_stakeholders WHERE person_id = ?2",
                    params![keep_id, remove_id],
                )
                .map_err(|e| e.to_string())?;
            // 2a. Transfer account_stakeholder_roles links
            tx.conn
                .execute(
                    "INSERT OR IGNORE INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 SELECT account_id, ?1, role, data_source FROM account_stakeholder_roles WHERE person_id = ?2",
                    params![keep_id, remove_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM account_stakeholder_roles WHERE person_id = ?1",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM account_stakeholders WHERE person_id = ?1",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;

            // 2b. Transfer entity_members links
            tx.conn
                .execute(
                    "INSERT OR IGNORE INTO entity_members (entity_id, person_id, relationship_type)
                 SELECT entity_id, ?1, relationship_type FROM entity_members WHERE person_id = ?2",
                    params![keep_id, remove_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM entity_members WHERE person_id = ?1",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;

            // 3. Transfer actions
            tx.conn
                .execute(
                    "UPDATE actions SET person_id = ?1 WHERE person_id = ?2",
                    params![keep_id, remove_id],
                )
                .map_err(|e| e.to_string())?;

            // 4. Delete removed person's assessment cache
            tx.conn
                .execute(
                    "DELETE FROM entity_assessment WHERE entity_id = ?1",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;

            // 5. Delete removed person's entity row
            tx.conn
                .execute(
                    "DELETE FROM entities WHERE id = ?1 AND entity_type = 'person'",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;

            // 6. Delete removed person's content_index rows
            tx.conn
                .execute(
                    "DELETE FROM content_index WHERE entity_id = ?1",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;

            // 6b. Transfer email aliases from removed person to kept person
            tx.conn
                .execute(
                    "UPDATE OR IGNORE person_emails SET person_id = ?1 WHERE person_id = ?2",
                    params![keep_id, remove_id],
                )
                .map_err(|e| e.to_string())?;
            // Clean up any that couldn't be transferred (duplicate email for same person)
            tx.conn
                .execute(
                    "DELETE FROM person_emails WHERE person_id = ?1",
                    params![remove_id],
                )
                .map_err(|e| e.to_string())?;
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
                    tx.conn
                        .execute(
                            "UPDATE people SET notes = ?1, updated_at = ?2 WHERE id = ?3",
                            params![merged_notes, now, keep_id],
                        )
                        .map_err(|e| e.to_string())?;
                }
            }

            Ok(())
        })
        .map_err(DbError::Migration)
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
            tx.conn
                .execute(
                    "DELETE FROM meeting_attendees WHERE person_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM account_stakeholder_roles WHERE person_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM account_stakeholders WHERE person_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM entity_members WHERE person_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "UPDATE actions SET person_id = NULL WHERE person_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM entity_assessment WHERE entity_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM entities WHERE id = ?1 AND entity_type = 'person'",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM content_index WHERE entity_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute(
                    "DELETE FROM person_emails WHERE person_id = ?1",
                    params![person_id],
                )
                .map_err(|e| e.to_string())?;
            tx.conn
                .execute("DELETE FROM people WHERE id = ?1", params![person_id])
                .map_err(|e| e.to_string())?;

            Ok(())
        })
        .map_err(DbError::Migration)
    }

    /// Unified person profile update — single write path for all enrichment sources.
    ///
    /// Checks source priority for each field, writes allowed fields to `people`,
    /// updates provenance in `enrichment_sources`, records an `enrichment_log`
    /// audit entry, and returns which fields were actually written.
    pub fn update_person_profile(
        &self,
        person_id: &str,
        fields: &ProfileUpdate,
        source: &str,
    ) -> Result<ProfileUpdateResult, DbError> {
        let conn = &self.conn;

        // Read current enrichment_sources
        let current_sources_json: Option<String> = conn
            .query_row(
                "SELECT enrichment_sources FROM people WHERE id = ?1",
                rusqlite::params![person_id],
                |row| row.get(0),
            )
            .map_err(|e| DbError::Migration(format!("read enrichment_sources: {}", e)))?;

        let mut sources: std::collections::HashMap<String, FieldSource> = current_sources_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();

        let csj = current_sources_json.as_deref();
        let mut updated: Vec<String> = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        // Check each field against source priority
        let candidates: Vec<(&str, &Option<String>)> = vec![
            ("linkedin_url", &fields.linkedin_url),
            ("twitter_handle", &fields.twitter_handle),
            ("phone", &fields.phone),
            ("photo_url", &fields.photo_url),
            ("bio", &fields.bio),
            ("title_history", &fields.title_history),
            ("organization", &fields.organization),
            ("role", &fields.role),
            ("company_industry", &fields.company_industry),
            ("company_size", &fields.company_size),
            ("company_hq", &fields.company_hq),
        ];

        // Build SET clauses dynamically for allowed fields
        let mut set_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        for (field_name, value) in &candidates {
            if let Some(val) = value {
                if !val.is_empty() && can_write_field(csj, field_name, source) {
                    set_clauses.push(format!("{} = ?", field_name));
                    params.push(Box::new(val.clone()));
                    sources.insert(
                        field_name.to_string(),
                        FieldSource {
                            source: source.to_string(),
                            at: now.clone(),
                        },
                    );
                    updated.push(field_name.to_string());
                }
            }
        }

        if updated.is_empty() {
            // Still stamp last_enriched_at so this person isn't re-queued
            let _ = conn.execute(
                "UPDATE people SET last_enriched_at = ?1 WHERE id = ?2",
                rusqlite::params![now, person_id],
            );
            return Ok(ProfileUpdateResult {
                fields_updated: vec![],
            });
        }

        // Always update enrichment_sources, last_enriched_at, updated_at
        let sources_json = serde_json::to_string(&sources).unwrap_or_else(|_| "{}".to_string());
        set_clauses.push("enrichment_sources = ?".to_string());
        params.push(Box::new(sources_json));
        set_clauses.push("last_enriched_at = ?".to_string());
        params.push(Box::new(now.clone()));
        set_clauses.push("updated_at = ?".to_string());
        params.push(Box::new(now));

        // WHERE clause
        params.push(Box::new(person_id.to_string()));

        let sql = format!("UPDATE people SET {} WHERE id = ?", set_clauses.join(", "));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| DbError::Migration(format!("update_person_profile: {}", e)))?;

        // Audit trail
        let log_id = format!("el-{}", uuid::Uuid::new_v4());
        let fields_json = serde_json::to_string(&updated).unwrap_or_else(|_| "[]".to_string());

        let _ = conn.execute(
            "INSERT INTO enrichment_log
                (id, entity_type, entity_id, source, event_type, fields_updated, created_at)
             VALUES (?1, 'person', ?2, ?3, 'enrichment', ?4, datetime('now'))",
            rusqlite::params![log_id, person_id, source, fields_json],
        );

        Ok(ProfileUpdateResult {
            fields_updated: updated,
        })
    }
}

// ---------------------------------------------------------------------------
// Enrichment types (shared by all sources)
// ---------------------------------------------------------------------------

/// Fields that can be updated on a person profile from any enrichment source.
#[derive(Debug, Clone, Default)]
pub struct ProfileUpdate {
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub phone: Option<String>,
    pub photo_url: Option<String>,
    pub bio: Option<String>,
    pub title_history: Option<String>,
    pub organization: Option<String>,
    pub role: Option<String>,
    pub company_industry: Option<String>,
    pub company_size: Option<String>,
    pub company_hq: Option<String>,
}

/// Result of a profile update — which fields were actually written.
#[derive(Debug, Clone)]
pub struct ProfileUpdateResult {
    pub fields_updated: Vec<String>,
}

/// Per-field provenance record stored in the `enrichment_sources` JSON column.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldSource {
    pub source: String,
    pub at: String,
}

/// Returns the numeric priority for a given enrichment source.
/// Higher values win: User (4) > Clay (3) > Glean/Google/Gravatar (2) > AI (1).
pub fn source_priority(source: &str) -> u8 {
    match source {
        "user" => 4,
        "clay" => 3,
        "glean" | "google" => 2,
        "gravatar" => 2,
        "ai" => 1,
        _ => 0,
    }
}

/// Checks whether a source is allowed to write a field given the current
/// provenance map. Returns `true` when no higher-priority source has already
/// written the field.
pub fn can_write_field(current_sources_json: Option<&str>, field: &str, source: &str) -> bool {
    let new_priority = source_priority(source);
    if new_priority == 0 {
        return false;
    }
    let sources: std::collections::HashMap<String, FieldSource> = current_sources_json
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    match sources.get(field) {
        Some(existing) => source_priority(&existing.source) <= new_priority,
        None => true,
    }
}
