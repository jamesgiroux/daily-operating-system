//! DB query and write methods for the entity linking engine.
//!
//! All methods are on `ActionDb`. Callers in services::entity_linking
//! use these directly; nothing in this module should be called from
//! outside that service.

use std::collections::HashMap;

use rusqlite::params;

use super::ActionDb;

// ---------------------------------------------------------------------------
// Write-side structs
// ---------------------------------------------------------------------------

pub struct LinkedEntityRawWrite {
    pub owner_type: String,
    pub owner_id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub role: String,
    pub source: String,
    pub rule_id: Option<String>,
    pub confidence: Option<f64>,
    pub evidence_json: Option<String>,
    pub graph_version: i64,
}

pub struct LinkingEvaluationWrite<'a> {
    pub owner_type: &'a str,
    pub owner_id: &'a str,
    pub trigger: &'a str,
    pub rule_id: Option<&'a str>,
    pub entity_id: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub role: Option<&'a str>,
    pub graph_version: i64,
    pub evidence_json: &'a str,
}

// ---------------------------------------------------------------------------
// Read-side structs
// ---------------------------------------------------------------------------

pub struct ThreadPrimaryLink {
    pub entity_id: String,
    pub entity_type: String,
    /// Domain list for the primary account (empty if entity_type != "account").
    pub account_domains: Vec<String>,
    /// Sender email of the parent message — used by P2's same-sender check.
    pub parent_sender_email: Option<String>,
}

pub struct PendingStakeholderRow {
    pub person_id: String,
    pub name: String,
    pub email: String,
    pub confidence: Option<f64>,
    pub data_source: String,
    pub created_at: String,
    /// Other accounts that share this person's email domain (multi-BU hint).
    /// Used by the review queue UI to surface "Also add to X?" notes (AC#13).
    pub sibling_account_hints: Vec<(String, String)>, // (account_id, account_name)
}

// ---------------------------------------------------------------------------
// impl ActionDb
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Read the current entity graph snapshot version (O(1)).
    pub fn get_entity_graph_version(&self) -> Result<i64, String> {
        self.conn_ref()
            .query_row(
                "SELECT version FROM entity_graph_version WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("get_entity_graph_version: {e}"))
    }

    /// Batch read linked entities for a list of meetings from the `linked_entities`
    /// view (DOS-258). Mirrors the single-owner query used by the meeting detail
    /// page so the three list/dashboard surfaces no longer read stale rows from
    /// the legacy `meeting_entities` junction table.
    ///
    /// The `linked_entities` view already hides `user_dismissed` tombstones.
    /// Returns a map `meeting_id -> Vec<LinkedEntity>` sorted by role
    /// (primary, related, auto_suggested) then name ASC. Legacy `is_primary`
    /// and `suggested` fields are derived from `role` for back-compat with the
    /// frontend chip renderer, while the new `role` and `applied_rule` fields
    /// carry the engine's deterministic output.
    pub fn get_linked_entities_map_for_meetings(
        &self,
        meeting_ids: &[String],
    ) -> Result<HashMap<String, Vec<crate::types::LinkedEntity>>, String> {
        if meeting_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders: Vec<String> = (0..meeting_ids.len())
            .map(|i| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT lr.owner_id, lr.entity_id, lr.entity_type, lr.role, \
                    lr.confidence, lr.rule_id, \
                    COALESCE(acc.name, proj.name, p.name, lr.entity_id) AS name \
             FROM linked_entities lr \
             LEFT JOIN accounts acc \
                  ON lr.entity_type = 'account' AND acc.id = lr.entity_id \
             LEFT JOIN projects proj \
                  ON lr.entity_type = 'project' AND proj.id = lr.entity_id \
             LEFT JOIN people p \
                  ON lr.entity_type = 'person' AND p.id = lr.entity_id \
             WHERE lr.owner_type = 'meeting' AND lr.owner_id IN ({}) \
             ORDER BY lr.owner_id ASC, \
               CASE lr.role WHEN 'primary' THEN 0 \
                            WHEN 'related' THEN 1 \
                            ELSE 2 END, \
               name ASC",
            placeholders.join(", ")
        );
        let conn = self.conn_ref();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("prepare get_linked_entities_map_for_meetings: {e}"))?;
        let params: Vec<&dyn rusqlite::types::ToSql> = meeting_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let mut rows = stmt
            .query(params.as_slice())
            .map_err(|e| format!("get_linked_entities_map_for_meetings query: {e}"))?;
        let mut map: HashMap<String, Vec<crate::types::LinkedEntity>> = HashMap::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("get_linked_entities_map_for_meetings row: {e}"))?
        {
            let meeting_id: String = row.get(0).unwrap_or_default();
            let entity_id: String = row.get(1).unwrap_or_default();
            let entity_type: String = row.get(2).unwrap_or_default();
            let role: String = row.get(3).unwrap_or_default();
            let confidence: Option<f64> = row.get(4).ok();
            let rule_id: Option<String> = row.get(5).ok();
            let name: String = row.get(6).unwrap_or_default();
            map.entry(meeting_id)
                .or_default()
                .push(crate::types::LinkedEntity {
                    id: entity_id,
                    name,
                    entity_type,
                    confidence: confidence.unwrap_or(0.95),
                    is_primary: role == "primary",
                    suggested: role == "auto_suggested",
                    role: Some(role),
                    applied_rule: rule_id,
                });
        }
        Ok(map)
    }

    /// Return (entity_id, entity_type) pairs that the user has dismissed for this owner.
    pub fn get_linking_dismissals(
        &self,
        owner_type: &str,
        owner_id: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let conn = self.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT entity_id, entity_type FROM linking_dismissals \
                 WHERE owner_type = ?1 AND owner_id = ?2",
            )
            .map_err(|e| format!("prepare get_linking_dismissals: {e}"))?;
        let mut rows = stmt
            .query(params![owner_type, owner_id])
            .map_err(|e| format!("get_linking_dismissals query: {e}"))?;
        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("get_linking_dismissals row: {e}"))?
        {
            results.push((
                row.get::<_, String>(0)
                    .map_err(|e| format!("get_linking_dismissals col0: {e}"))?,
                row.get::<_, String>(1)
                    .map_err(|e| format!("get_linking_dismissals col1: {e}"))?,
            ));
        }
        Ok(results)
    }

    /// Return the user-override link (source='user') for an owner, if any.
    pub fn get_user_override_link(
        &self,
        owner_type: &str,
        owner_id: &str,
    ) -> Result<Option<(String, String)>, String> {
        self.conn_ref()
            .query_row(
                "SELECT entity_id, entity_type FROM linked_entities_raw \
                 WHERE owner_type = ?1 AND owner_id = ?2 AND source = 'user' \
                   AND role = 'primary' LIMIT 1",
                params![owner_type, owner_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| format!("get_user_override_link: {e}"))
    }

    /// Find the current primary link for any other email in the same thread.
    /// Used by P2 thread inheritance.
    pub fn get_thread_primary_link(
        &self,
        thread_id: &str,
        exclude_email_id: &str,
    ) -> Result<Option<ThreadPrimaryLink>, String> {
        let conn = self.conn_ref();
        // Check linked_entities_raw first (new system). Also fetch the parent
        // sender_email so P2 can apply the "same sender" check.
        let row = conn
            .query_row(
                "SELECT ler.entity_id, ler.entity_type, e.sender_email \
                 FROM linked_entities_raw ler \
                 JOIN emails e ON e.email_id = ler.owner_id \
                 WHERE ler.owner_type = 'email' \
                   AND ler.role = 'primary' \
                   AND ler.source != 'user_dismissed' \
                   AND e.thread_id = ?1 \
                   AND ler.owner_id != ?2 \
                 ORDER BY e.received_at DESC LIMIT 1",
                params![thread_id, exclude_email_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("get_thread_primary_link: {e}"))?;

        // Fall back to legacy emails.entity_id
        let (entity_id, entity_type, parent_sender_email) = match row {
            Some(r) => r,
            None => {
                let legacy = conn
                    .query_row(
                        "SELECT entity_id, entity_type, sender_email FROM emails \
                         WHERE thread_id = ?1 AND email_id != ?2 \
                           AND entity_id IS NOT NULL \
                         ORDER BY received_at DESC LIMIT 1",
                        params![thread_id, exclude_email_id],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1).unwrap_or_else(|_| "account".to_string()),
                                row.get::<_, Option<String>>(2)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|e| format!("get_thread_primary_link legacy: {e}"))?;
                match legacy {
                    Some(r) => r,
                    None => return Ok(None),
                }
            }
        };

        // Fetch account domains if entity_type is 'account'
        let account_domains = if entity_type == "account" {
            let mut stmt2 = conn
                .prepare("SELECT domain FROM account_domains WHERE account_id = ?1")
                .map_err(|e| format!("prepare account_domains: {e}"))?;
            let mut rows2 = stmt2
                .query(params![entity_id])
                .map_err(|e| format!("account_domains query: {e}"))?;
            let mut domains = Vec::new();
            while let Some(row) = rows2.next().map_err(|e| format!("account_domains row: {e}"))? {
                if let Ok(d) = row.get::<_, String>(0) {
                    domains.push(d);
                }
            }
            domains
        } else {
            vec![]
        };

        Ok(Some(ThreadPrimaryLink {
            entity_id,
            entity_type,
            account_domains,
            parent_sender_email,
        }))
    }

    /// Find the user-set primary for the first event in a series. Used by P3.
    pub fn get_series_primary_link(
        &self,
        series_id: &str,
        exclude_meeting_id: &str,
    ) -> Result<Option<(String, String)>, String> {
        self.conn_ref()
            .query_row(
                "SELECT ler.entity_id, ler.entity_type \
                 FROM linked_entities_raw ler \
                 JOIN meetings m ON m.id = ler.owner_id \
                 WHERE ler.owner_type = 'meeting' \
                   AND ler.role = 'primary' \
                   AND ler.source = 'user' \
                   AND m.calendar_event_id LIKE ?1 \
                   AND ler.owner_id != ?2 \
                 ORDER BY m.start_time ASC LIMIT 1",
                params![format!("{series_id}%"), exclude_meeting_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| format!("get_series_primary_link: {e}"))
    }

    /// Return all non-archived account_ids on which `person_id` is an **active**
    /// stakeholder. Drives the P4a stakeholder-inference rule: stakeholder
    /// membership is stronger evidence than attendee-domain matching.
    ///
    /// Only `status='active'` rows count. Pending, dismissed and archived
    /// stakeholders are excluded because they do not represent a confirmed
    /// relationship.
    pub fn lookup_active_stakeholder_accounts_for_person(
        &self,
        person_id: &str,
    ) -> Result<Vec<String>, String> {
        let conn = self.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT as_.account_id \
                 FROM account_stakeholders as_ \
                 JOIN accounts a ON a.id = as_.account_id \
                 WHERE as_.person_id = ?1 \
                   AND as_.status = 'active' \
                   AND a.archived = 0 \
                 ORDER BY as_.account_id",
            )
            .map_err(|e| format!("prepare lookup_active_stakeholder_accounts_for_person: {e}"))?;
        let mut rows = stmt
            .query(params![person_id])
            .map_err(|e| format!("lookup_active_stakeholder_accounts_for_person query: {e}"))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("lookup_active_stakeholder_accounts_for_person row: {e}"))?
        {
            out.push(
                row.get::<_, String>(0)
                    .map_err(|e| format!("stakeholder_accounts col0: {e}"))?,
            );
        }
        Ok(out)
    }

    /// True if person is a stakeholder on 2+ accounts (multi-account-active check for P4b).
    pub fn is_person_multi_account_active(&self, person_id: &str) -> Result<bool, String> {
        let count: i64 = self
            .conn_ref()
            .query_row(
                "SELECT count(DISTINCT account_id) FROM account_stakeholders \
                 WHERE person_id = ?1 AND status IN ('active', 'pending_review')",
                params![person_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("is_person_multi_account_active: {e}"))?;
        Ok(count >= 2)
    }

    /// Return a map of account_id → link count for a sender email (for P10).
    pub fn count_sender_account_links(
        &self,
        sender_email: &str,
        days: i32,
    ) -> Result<Vec<(String, i64)>, String> {
        let cutoff = format!("-{days} days");
        let conn = self.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT entity_id, count(*) as cnt \
                 FROM emails \
                 WHERE sender_email = ?1 \
                   AND entity_type = 'account' \
                   AND entity_id IS NOT NULL \
                   AND received_at >= datetime('now', ?2) \
                 GROUP BY entity_id \
                 ORDER BY cnt DESC",
            )
            .map_err(|e| format!("prepare count_sender_account_links: {e}"))?;
        let mut rows = stmt
            .query(params![sender_email, cutoff])
            .map_err(|e| format!("count_sender_account_links query: {e}"))?;
        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("count_sender_account_links row: {e}"))?
        {
            results.push((
                row.get::<_, String>(0).map_err(|e| format!("sender_links col0: {e}"))?,
                row.get::<_, i64>(1).map_err(|e| format!("sender_links col1: {e}"))?,
            ));
        }
        Ok(results)
    }

    /// Fetch all entity names + keywords for P5 title matching.
    /// Returns Vec<(id, entity_type, name, keywords_json)>.
    pub fn get_entities_for_title_match(
        &self,
    ) -> Result<Vec<(String, String, String, Option<String>)>, String> {
        let conn = self.conn_ref();
        let mut results = Vec::new();

        let mut stmt = conn
            .prepare(
                "SELECT id, 'account', name, keywords FROM accounts WHERE archived = 0 \
                 UNION ALL \
                 SELECT id, 'project', name, keywords FROM projects WHERE archived = 0",
            )
            .map_err(|e| format!("prepare get_entities_for_title_match: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("get_entities_for_title_match query: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("get_entities_for_title_match row: {e}"))?
        {
            results.push((
                row.get::<_, String>(0).map_err(|e| format!("title_match col0: {e}"))?,
                row.get::<_, String>(1).map_err(|e| format!("title_match col1: {e}"))?,
                row.get::<_, String>(2).map_err(|e| format!("title_match col2: {e}"))?,
                row.get::<_, Option<String>>(3).map_err(|e| format!("title_match col3: {e}"))?,
            ));
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Write methods
    // -----------------------------------------------------------------------

    /// Upsert a row into linked_entities_raw.
    pub fn upsert_linked_entity_raw(&self, row: &LinkedEntityRawWrite) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw \
                 (owner_type, owner_id, entity_id, entity_type, role, source, \
                  rule_id, confidence, evidence_json, graph_version, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
                 ON CONFLICT(owner_type, owner_id, entity_id, entity_type) DO UPDATE SET \
                   role = excluded.role, \
                   source = excluded.source, \
                   rule_id = excluded.rule_id, \
                   confidence = excluded.confidence, \
                   evidence_json = excluded.evidence_json, \
                   graph_version = excluded.graph_version",
                params![
                    row.owner_type,
                    row.owner_id,
                    row.entity_id,
                    row.entity_type,
                    row.role,
                    row.source,
                    row.rule_id,
                    row.confidence,
                    row.evidence_json,
                    row.graph_version,
                    now,
                ],
            )
            .map(|_| ())
            .map_err(|e| format!("upsert_linked_entity_raw: {e}"))
    }

    /// Delete auto-resolution rows for an owner.
    ///
    /// Preserves:
    ///   - source='user'           — explicit user overrides (P1)
    ///   - source='user_dismissed' — dismissal tombstones (dismissal-wins-race)
    ///
    /// Without preserving user_dismissed, a concurrent recompute could delete
    /// the tombstone and then re-insert the dismissed entity on its next pass.
    pub fn delete_auto_links_for_owner(
        &self,
        owner_type: &str,
        owner_id: &str,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "DELETE FROM linked_entities_raw \
                 WHERE owner_type = ?1 AND owner_id = ?2 \
                   AND source NOT IN ('user', 'user_dismissed')",
                params![owner_type, owner_id],
            )
            .map(|_| ())
            .map_err(|e| format!("delete_auto_links_for_owner: {e}"))
    }

    /// Mark a specific link as dismissed (source='user_dismissed').
    pub fn set_link_user_dismissed(
        &self,
        owner_type: &str,
        owner_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "UPDATE linked_entities_raw SET source = 'user_dismissed' \
                 WHERE owner_type = ?1 AND owner_id = ?2 \
                   AND entity_id = ?3 AND entity_type = ?4",
                params![owner_type, owner_id, entity_id, entity_type],
            )
            .map(|_| ())
            .map_err(|e| format!("set_link_user_dismissed: {e}"))
    }

    /// Append a row to entity_linking_evaluations (append-only audit).
    pub fn insert_linking_evaluation(&self, ev: &LinkingEvaluationWrite) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "INSERT INTO entity_linking_evaluations \
                 (owner_type, owner_id, link_trigger, rule_id, entity_id, \
                  entity_type, role, graph_version, evidence_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    ev.owner_type,
                    ev.owner_id,
                    ev.trigger,
                    ev.rule_id,
                    ev.entity_id,
                    ev.entity_type,
                    ev.role,
                    ev.graph_version,
                    ev.evidence_json,
                ],
            )
            .map(|_| ())
            .map_err(|e| format!("insert_linking_evaluation: {e}"))
    }

    /// Write a dismissal row to linking_dismissals.
    pub fn upsert_linking_dismissal(
        &self,
        owner_type: &str,
        owner_id: &str,
        entity_id: &str,
        entity_type: &str,
        dismissed_by: Option<&str>,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO linking_dismissals \
                 (owner_type, owner_id, entity_id, entity_type, dismissed_by, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![owner_type, owner_id, entity_id, entity_type, dismissed_by, now],
            )
            .map(|_| ())
            .map_err(|e| format!("upsert_linking_dismissal: {e}"))
    }

    /// Remove a dismissal row (undo dismiss).
    pub fn delete_linking_dismissal(
        &self,
        owner_type: &str,
        owner_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "DELETE FROM linking_dismissals \
                 WHERE owner_type = ?1 AND owner_id = ?2 \
                   AND entity_id = ?3 AND entity_type = ?4",
                params![owner_type, owner_id, entity_id, entity_type],
            )
            .map(|_| ())
            .map_err(|e| format!("delete_linking_dismissal: {e}"))
    }

    /// Queue a person as a pending stakeholder suggestion for an account.
    /// INSERT OR IGNORE — does not overwrite if already exists in any status.
    pub fn suggest_stakeholder_pending(
        &self,
        account_id: &str,
        person_id: &str,
        data_source: &str,
        confidence: f64,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO account_stakeholders \
                 (account_id, person_id, data_source, confidence, status) \
                 VALUES (?1, ?2, ?3, ?4, 'pending_review')",
                params![account_id, person_id, data_source, confidence],
            )
            .map(|_| ())
            .map_err(|e| format!("suggest_stakeholder_pending: {e}"))
    }

    /// Promote a pending_review suggestion to active.
    pub fn confirm_stakeholder(&self, account_id: &str, person_id: &str) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "UPDATE account_stakeholders SET status = 'active' \
                 WHERE account_id = ?1 AND person_id = ?2 AND status = 'pending_review'",
                params![account_id, person_id],
            )
            .map(|_| ())
            .map_err(|e| format!("confirm_stakeholder: {e}"))
    }

    /// Dismiss a pending_review suggestion.
    pub fn dismiss_stakeholder_suggestion(
        &self,
        account_id: &str,
        person_id: &str,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "UPDATE account_stakeholders SET status = 'dismissed' \
                 WHERE account_id = ?1 AND person_id = ?2 AND status = 'pending_review'",
                params![account_id, person_id],
            )
            .map(|_| ())
            .map_err(|e| format!("dismiss_stakeholder_suggestion: {e}"))
    }

    /// True if person already has an active or pending stakeholder row on account.
    pub fn is_stakeholder_on_account(
        &self,
        account_id: &str,
        person_id: &str,
    ) -> Result<bool, String> {
        let count: i64 = self
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM account_stakeholders \
                 WHERE account_id = ?1 AND person_id = ?2 \
                   AND status IN ('active', 'pending_review')",
                params![account_id, person_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("is_stakeholder_on_account: {e}"))?;
        Ok(count > 0)
    }

    /// Return pending stakeholder suggestions for an account (for the review queue UI).
    pub fn get_pending_stakeholder_suggestions(
        &self,
        account_id: &str,
    ) -> Result<Vec<PendingStakeholderRow>, String> {
        let conn = self.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT s.person_id, p.name, p.email, s.confidence, s.data_source, s.created_at \
                 FROM account_stakeholders s \
                 JOIN people p ON p.id = s.person_id \
                 WHERE s.account_id = ?1 AND s.status = 'pending_review' \
                 ORDER BY s.confidence DESC, s.created_at DESC",
            )
            .map_err(|e| format!("prepare get_pending_stakeholder_suggestions: {e}"))?;
        let mut rows = stmt
            .query(params![account_id])
            .map_err(|e| format!("get_pending_stakeholder_suggestions query: {e}"))?;
        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("get_pending_stakeholder_suggestions row: {e}"))?
        {
            let person_id: String = row.get(0).map_err(|e| format!("pending_stk col0: {e}"))?;
            let name: String = row.get(1).map_err(|e| format!("pending_stk col1: {e}"))?;
            let email: String = row.get(2).map_err(|e| format!("pending_stk col2: {e}"))?;
            let confidence: Option<f64> = row.get(3).map_err(|e| format!("pending_stk col3: {e}"))?;
            let data_source: String = row.get(4).map_err(|e| format!("pending_stk col4: {e}"))?;
            let created_at: String = row.get(5).map_err(|e| format!("pending_stk col5: {e}"))?;

            // AC#13 multi-BU: find other accounts sharing this person's email domain.
            let sibling_account_hints = self.get_sibling_accounts_for_email(&email, account_id);

            results.push(PendingStakeholderRow {
                person_id,
                name,
                email,
                confidence,
                data_source,
                created_at,
                sibling_account_hints,
            });
        }
        Ok(results)
    }

    /// Queue a child email for deferred thread inheritance (P2 late-arrival).
    pub fn enqueue_thread_inheritance(
        &self,
        thread_id: &str,
        child_email_id: &str,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO pending_thread_inheritance \
                 (thread_id, child_owner_type, child_owner_id) \
                 VALUES (?1, 'email', ?2)",
                params![thread_id, child_email_id],
            )
            .map(|_| ())
            .map_err(|e| format!("enqueue_thread_inheritance: {e}"))
    }

    /// Pop and return child emails waiting for a thread's primary to be set.
    pub fn drain_thread_inheritance_queue(
        &self,
        thread_id: &str,
    ) -> Result<Vec<String>, String> {
        let mut children = Vec::new();
        {
            let conn = self.conn_ref();
            let mut stmt = conn
                .prepare(
                    "SELECT child_owner_id FROM pending_thread_inheritance \
                     WHERE thread_id = ?1",
                )
                .map_err(|e| format!("prepare drain_thread_inheritance: {e}"))?;
            let mut rows = stmt
                .query(params![thread_id])
                .map_err(|e| format!("drain_thread_inheritance query: {e}"))?;
            while let Some(row) = rows
                .next()
                .map_err(|e| format!("drain_thread_inheritance row: {e}"))?
            {
                if let Ok(id) = row.get::<_, String>(0) {
                    children.push(id);
                }
            }
        };
        if !children.is_empty() {
            self.conn_ref()
                .execute(
                    "DELETE FROM pending_thread_inheritance WHERE thread_id = ?1",
                    params![thread_id],
                )
                .map_err(|e| format!("drain_thread_inheritance delete: {e}"))?;
        }
        Ok(children)
    }

    /// Return other accounts that share the person's email domain (AC#13 multi-BU hint).
    /// Excludes the primary account and archived accounts. Returns at most 5 hints.
    fn get_sibling_accounts_for_email(
        &self,
        email: &str,
        exclude_account_id: &str,
    ) -> Vec<(String, String)> {
        let domain = match email.rsplit_once('@').map(|(_, d)| d.to_lowercase()) {
            Some(d) => d,
            None => return vec![],
        };
        let conn = self.conn_ref();
        let mut stmt = match conn.prepare(
            "SELECT DISTINCT a.id, a.name \
             FROM account_domains ad \
             JOIN accounts a ON a.id = ad.account_id \
             WHERE ad.domain = ?1 \
               AND a.id != ?2 \
               AND a.archived = 0 \
             LIMIT 5",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let mut rows = match stmt.query(params![domain, exclude_account_id]) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        let mut hints = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let id: String = row.get(0).unwrap_or_default();
            let name: String = row.get(1).unwrap_or_default();
            if !id.is_empty() {
                hints.push((id, name));
            }
        }
        hints
    }

    /// Fetch minimal email fields for thread-inheritance re-evaluation.
    ///
    /// Returns the fields that email_adapter::build_context needs to rebuild
    /// a LinkingContext. Other DbEmail fields are set to safe defaults.
    pub fn get_email_by_id_for_linking(
        &self,
        email_id: &str,
    ) -> Result<Option<crate::db::types::DbEmail>, String> {
        self.conn_ref()
            .query_row(
                "SELECT email_id, thread_id, sender_email, sender_name, subject,
                        snippet, priority, is_unread, received_at,
                        enrichment_state, entity_id, entity_type,
                        user_is_last_sender, last_sender_email,
                        message_count, created_at, updated_at,
                        to_recipients, cc_recipients
                 FROM emails WHERE email_id = ?1",
                params![email_id],
                |row| {
                    Ok(crate::db::types::DbEmail {
                        email_id:            row.get(0)?,
                        thread_id:           row.get(1)?,
                        sender_email:        row.get(2)?,
                        sender_name:         row.get(3)?,
                        subject:             row.get(4)?,
                        snippet:             row.get(5)?,
                        priority:            row.get(6)?,
                        is_unread:           row.get(7).unwrap_or(false),
                        received_at:         row.get(8)?,
                        enrichment_state:    row.get(9).unwrap_or_default(),
                        enrichment_attempts: 0,
                        last_enrichment_at:  None,
                        enriched_at:         None,
                        last_seen_at:        None,
                        resolved_at:         None,
                        entity_id:           row.get(10)?,
                        entity_type:         row.get(11)?,
                        contextual_summary:  None,
                        sentiment:           None,
                        urgency:             None,
                        user_is_last_sender: row.get(12).unwrap_or(false),
                        last_sender_email:   row.get(13)?,
                        message_count:       row.get(14).unwrap_or(1),
                        created_at:          row.get(15).unwrap_or_default(),
                        updated_at:          row.get(16).unwrap_or_default(),
                        relevance_score:     None,
                        score_reason:        None,
                        pinned_at:           None,
                        commitments:         None,
                        questions:           None,
                        is_noise:            false,
                        // DOS-258: populated so manual_set_primary can rebuild a
                        // LinkingContext with real email participants (P4b/P4c/P4d
                        // fire, and the stakeholder-domain backfill sees real
                        // attendee domains).
                        to_recipients:       row.get(17).ok(),
                        cc_recipients:       row.get(18).ok(),
                    })
                },
            )
            .optional()
            .map_err(|e| format!("get_email_by_id_for_linking: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Helper: rusqlite optional extension (mirrors db/people.rs usage)
// ---------------------------------------------------------------------------

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
