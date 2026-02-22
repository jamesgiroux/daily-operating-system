use super::*;

impl ActionDb {
    // =========================================================================
    // Emails (I368)
    // =========================================================================

    /// Insert or update an email record. Sets `last_seen_at` to now on every upsert.
    pub fn upsert_email(&self, email: &DbEmail) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO emails (
                    email_id, thread_id, sender_email, sender_name, subject, snippet,
                    priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                    last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                    contextual_summary, sentiment, urgency, user_is_last_sender,
                    last_sender_email, message_count, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                    ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
                 )
                 ON CONFLICT(email_id) DO UPDATE SET
                    thread_id = excluded.thread_id,
                    sender_email = excluded.sender_email,
                    sender_name = excluded.sender_name,
                    subject = excluded.subject,
                    snippet = excluded.snippet,
                    priority = excluded.priority,
                    is_unread = excluded.is_unread,
                    received_at = excluded.received_at,
                    last_seen_at = excluded.last_seen_at,
                    user_is_last_sender = excluded.user_is_last_sender,
                    last_sender_email = excluded.last_sender_email,
                    message_count = excluded.message_count,
                    updated_at = excluded.updated_at",
                params![
                    email.email_id,
                    email.thread_id,
                    email.sender_email,
                    email.sender_name,
                    email.subject,
                    email.snippet,
                    email.priority,
                    email.is_unread as i32,
                    email.received_at,
                    email.enrichment_state,
                    email.enrichment_attempts,
                    email.last_enrichment_at,
                    now,
                    email.resolved_at,
                    email.entity_id,
                    email.entity_type,
                    email.contextual_summary,
                    email.sentiment,
                    email.urgency,
                    email.user_is_last_sender as i32,
                    email.last_sender_email,
                    email.message_count,
                    now,
                    now,
                ],
            )
            .map_err(|e| format!("Failed to upsert email {}: {e}", email.email_id))?;
        Ok(())
    }

    /// Mark emails as resolved (vanished from inbox). Sets `resolved_at` to now.
    /// Returns the number of rows updated.
    pub fn mark_emails_resolved(&self, vanished_ids: &[String]) -> Result<usize, String> {
        if vanished_ids.is_empty() {
            return Ok(0);
        }
        let now = Utc::now().to_rfc3339();
        let placeholders: Vec<String> = (1..=vanished_ids.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "UPDATE emails SET resolved_at = '{}', updated_at = '{}' WHERE email_id IN ({}) AND resolved_at IS NULL",
            now, now,
            placeholders.join(", ")
        );
        let param_values: Vec<&dyn rusqlite::types::ToSql> =
            vanished_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let rows = self
            .conn
            .execute(&sql, param_values.as_slice())
            .map_err(|e| format!("Failed to mark emails resolved: {e}"))?;
        Ok(rows)
    }

    /// Unmark resolved emails that reappeared in inbox. Sets `resolved_at` to NULL.
    /// Returns the number of rows updated.
    pub fn unmark_resolved(&self, reappeared_ids: &[String]) -> Result<usize, String> {
        if reappeared_ids.is_empty() {
            return Ok(0);
        }
        let now = Utc::now().to_rfc3339();
        let placeholders: Vec<String> =
            (1..=reappeared_ids.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "UPDATE emails SET resolved_at = NULL, updated_at = '{}' WHERE email_id IN ({})",
            now,
            placeholders.join(", ")
        );
        let param_values: Vec<&dyn rusqlite::types::ToSql> =
            reappeared_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let rows = self
            .conn
            .execute(&sql, param_values.as_slice())
            .map_err(|e| format!("Failed to unmark resolved emails: {e}"))?;
        Ok(rows)
    }

    /// Get emails pending enrichment (state = 'pending' or 'failed', attempts < 3).
    pub fn get_pending_enrichment(&self, limit: usize) -> Result<Vec<DbEmail>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason
                 FROM emails
                 WHERE enrichment_state IN ('pending', 'failed')
                   AND enrichment_attempts < 3
                 ORDER BY created_at
                 LIMIT ?1",
            )
            .map_err(|e| format!("Failed to prepare pending enrichment query: {e}"))?;

        let rows = stmt
            .query_map(params![limit as i64], map_email_row)
            .map_err(|e| format!("Failed to query pending enrichment: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email row: {e}"))?);
        }
        Ok(results)
    }

    /// Set enrichment state and related fields for an email.
    pub fn set_enrichment_state(
        &self,
        email_id: &str,
        state: &str,
        enrichment: EmailEnrichmentUpdate<'_>,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE emails SET
                    enrichment_state = ?1,
                    enrichment_attempts = enrichment_attempts + 1,
                    last_enrichment_at = ?2,
                    contextual_summary = COALESCE(?3, contextual_summary),
                    entity_id = COALESCE(?4, entity_id),
                    entity_type = COALESCE(?5, entity_type),
                    sentiment = COALESCE(?6, sentiment),
                    urgency = COALESCE(?7, urgency),
                    updated_at = ?2
                 WHERE email_id = ?8",
                params![
                    state,
                    now,
                    enrichment.summary,
                    enrichment.entity_id,
                    enrichment.entity_type,
                    enrichment.sentiment,
                    enrichment.urgency,
                    email_id,
                ],
            )
            .map_err(|e| format!("Failed to set enrichment state for {email_id}: {e}"))?;
        Ok(())
    }

    /// Get all active (non-resolved) emails.
    pub fn get_all_active_emails(&self) -> Result<Vec<DbEmail>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason
                 FROM emails
                 WHERE resolved_at IS NULL
                 ORDER BY received_at DESC",
            )
            .map_err(|e| format!("Failed to prepare active emails query: {e}"))?;

        let rows = stmt
            .query_map([], map_email_row)
            .map_err(|e| format!("Failed to query active emails: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email row: {e}"))?);
        }
        Ok(results)
    }

    /// Get emails linked to a specific entity (for entity detail pages).
    pub fn get_emails_for_entity(&self, entity_id: &str) -> Result<Vec<DbEmail>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason
                 FROM emails
                 WHERE entity_id = ?1
                 ORDER BY received_at DESC",
            )
            .map_err(|e| format!("Failed to prepare entity emails query: {e}"))?;

        let rows = stmt
            .query_map(params![entity_id], map_email_row)
            .map_err(|e| format!("Failed to query entity emails: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email row: {e}"))?);
        }
        Ok(results)
    }

    /// Update thread position tracking for a thread.
    pub fn update_thread_position(
        &self,
        thread_id: &str,
        user_is_last_sender: bool,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE emails SET user_is_last_sender = ?1, updated_at = ?2
                 WHERE thread_id = ?3",
                params![user_is_last_sender as i32, now, thread_id],
            )
            .map_err(|e| format!("Failed to update thread position for {thread_id}: {e}"))?;
        Ok(())
    }

    /// Get email sync statistics for the sync status indicator.
    pub fn get_email_sync_stats(&self) -> Result<EmailSyncStats, String> {
        let last_fetch_at: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(last_seen_at) FROM emails",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to query last fetch time: {e}"))?;

        let total: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count active emails: {e}"))?;

        let enriched: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL AND enrichment_state = 'enriched'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count enriched emails: {e}"))?;

        let pending: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL AND enrichment_state IN ('pending', 'enriching')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count pending emails: {e}"))?;

        let failed: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL AND enrichment_state = 'failed'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count failed emails: {e}"))?;

        Ok(EmailSyncStats {
            last_fetch_at,
            total,
            enriched,
            pending,
            failed,
        })
    }

    /// Update the entity assignment for an email (I395 — user correction).
    pub fn update_email_entity(
        &self,
        email_id: &str,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE emails SET entity_id = ?1, entity_type = ?2, updated_at = ?3 WHERE email_id = ?4",
                rusqlite::params![entity_id, entity_type, now, email_id],
            )
            .map_err(|e| format!("Failed to update email entity for {email_id}: {e}"))?;
        Ok(())
    }

    /// Set the relevance score and reason for an email (I395).
    pub fn set_relevance_score(&self, email_id: &str, score: f64, reason: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE emails SET relevance_score = ?1, score_reason = ?2, updated_at = ?3 WHERE email_id = ?4",
                rusqlite::params![score, reason, now, email_id],
            )
            .map_err(|e| format!("Failed to set relevance score for {email_id}: {e}"))?;
        Ok(())
    }

    /// Get emails sorted by relevance score (highest first), with minimum score filter.
    pub fn get_emails_by_score(&self, min_score: f64, limit: usize) -> Result<Vec<DbEmail>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason
                 FROM emails
                 WHERE resolved_at IS NULL
                   AND relevance_score >= ?1
                 ORDER BY relevance_score DESC
                 LIMIT ?2",
            )
            .map_err(|e| format!("Failed to prepare scored emails query: {e}"))?;

        let rows = stmt
            .query_map(rusqlite::params![min_score, limit as i64], map_email_row)
            .map_err(|e| format!("Failed to query scored emails: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email row: {e}"))?);
        }
        Ok(results)
    }

    /// Get threads awaiting reply (unread, not resolved, user is not last sender).
    pub fn get_emails_awaiting_reply(&self) -> Result<Vec<DbEmail>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason
                 FROM emails
                 WHERE user_is_last_sender = 0
                   AND resolved_at IS NULL
                   AND is_unread = 1
                 ORDER BY received_at DESC",
            )
            .map_err(|e| format!("Failed to prepare awaiting reply query: {e}"))?;

        let rows = stmt
            .query_map([], map_email_row)
            .map_err(|e| format!("Failed to query awaiting reply: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email row: {e}"))?);
        }
        Ok(results)
    }
}

/// Parameters for enrichment state updates (avoids too_many_arguments lint).
pub struct EmailEnrichmentUpdate<'a> {
    pub summary: Option<&'a str>,
    pub entity_id: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub sentiment: Option<&'a str>,
    pub urgency: Option<&'a str>,
}

/// Row mapper for emails SELECT queries (26 columns).
fn map_email_row(row: &rusqlite::Row) -> rusqlite::Result<DbEmail> {
    Ok(DbEmail {
        email_id: row.get(0)?,
        thread_id: row.get(1)?,
        sender_email: row.get(2)?,
        sender_name: row.get(3)?,
        subject: row.get(4)?,
        snippet: row.get(5)?,
        priority: row.get(6)?,
        is_unread: row.get::<_, i32>(7)? != 0,
        received_at: row.get(8)?,
        enrichment_state: row.get(9)?,
        enrichment_attempts: row.get(10)?,
        last_enrichment_at: row.get(11)?,
        last_seen_at: row.get(12)?,
        resolved_at: row.get(13)?,
        entity_id: row.get(14)?,
        entity_type: row.get(15)?,
        contextual_summary: row.get(16)?,
        sentiment: row.get(17)?,
        urgency: row.get(18)?,
        user_is_last_sender: row.get::<_, i32>(19)? != 0,
        last_sender_email: row.get(20)?,
        message_count: row.get(21)?,
        created_at: row.get(22)?,
        updated_at: row.get(23)?,
        relevance_score: row.get(24).ok(),
        score_reason: row.get(25).ok(),
    })
}
