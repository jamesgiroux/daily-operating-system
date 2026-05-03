use super::*;

impl From<String> for DbError {
    fn from(message: String) -> Self {
        DbError::Migration(message)
    }
}

//: Cumulative cap on automatic stale-failed retries. Defined
/// at the DB layer because `get_email_sync_stats` needs it to compute the
/// `permanently_failed` subset, but the actual retry call lives in
/// `services::emails` (which references this same constant). Keeping a single
/// source of truth means the stats UI and the retry pass agree on which rows
/// are "permanently" failed.
pub const STALE_FAILED_MAX_AUTO_RETRIES: i32 = 5;

impl ActionDb {
    // =========================================================================
    // Emails
    // =========================================================================

    /// Insert or update an email record. Sets `last_seen_at` to now on every upsert.
    /// Preserves existing `enriched_at` timestamp if present, does not overwrite.
    ///
    /// After the per-row INSERT/UPDATE, this method also syncs the
    /// row's `entity_id` / `entity_type` / `sentiment` to whatever any other
    /// unresolved sibling in the same `thread_id` already has. The inbox is
    /// thread-collapsed by `received_at`, so a brand-new sibling carrying the
    /// auto-classifier's defaults would otherwise overwrite (visually) the
    /// user's prior correction on an older sibling. Forward propagation
    /// (correction-time cascade) lives in `update_email_entity` /
    /// `update_email_sentiment`; this is the backward propagation that catches
    /// the new arrival.
    ///
    /// Bounded by `resolved_at IS NULL` so archived siblings never participate
    /// — their entity/sentiment is historical and should not influence
    /// classification of newly arrived rows.
    ///
    /// Wrapped in `with_transaction` so the per-row write + sibling sync
    /// commit atomically; a partial commit would leave the new row with stale
    /// auto-classifier values that the user would then see as a revert.
    pub fn upsert_email(&self, email: &DbEmail) -> Result<(), DbError> {
        self.with_transaction(|tx| {
            let now = Utc::now().to_rfc3339();
            tx.conn
                .execute(
                    "INSERT INTO emails (
                        email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at, is_noise,
                        to_recipients, cc_recipients
                     ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                        ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28
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
                        updated_at = excluded.updated_at,
                        to_recipients = COALESCE(excluded.to_recipients, to_recipients),
                        cc_recipients = COALESCE(excluded.cc_recipients, cc_recipients),
                        -- never silently re-noise an email the user has rescued.
                        -- Once is_noise is cleared (via unsuppress_email), keep it cleared.
                        is_noise = MIN(emails.is_noise, excluded.is_noise)",
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
                        email.enriched_at,
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
                        email.is_noise as i32,
                        email.to_recipients,
                        email.cc_recipients,
                    ],
                )
                .map_err(|e| format!("Failed to upsert email {}: {e}", email.email_id))?;

            // backward thread-cascade. If any other unresolved sibling
            // in the same thread differs on entity_id / entity_type / sentiment,
            // adopt the sibling's most-recently-updated value. This is what
            // catches a freshly polled message that arrived AFTER a user
            // correction on an older sibling.
            //
            // Skipped when this row itself is resolved (archived) — a row
            // arriving already-resolved should not be retroactively
            // re-classified — and when the row has no thread_id (singleton).
            if email.resolved_at.is_none() {
                if let Some(thread_id) = email.thread_id.as_deref().filter(|t| !t.trim().is_empty()) {
                    // Pick the most-recently-updated unresolved sibling that
                    // has a non-null entity_id (preferring user corrections,
                    // which always bump updated_at).
                    let sibling_entity: Option<(Option<String>, Option<String>)> = tx
                        .conn
                        .query_row(
                            "SELECT entity_id, entity_type FROM emails
                             WHERE thread_id = ?1
                               AND email_id != ?2
                               AND resolved_at IS NULL
                               AND entity_id IS NOT NULL
                             ORDER BY updated_at DESC
                             LIMIT 1",
                            params![thread_id, email.email_id],
                            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
                        )
                        .ok();

                    if let Some((sib_entity_id, sib_entity_type)) = sibling_entity {
                        tx.conn
                            .execute(
                                "UPDATE emails SET entity_id = ?1, entity_type = ?2, updated_at = ?3
                                 WHERE email_id = ?4
                                   AND resolved_at IS NULL
                                   AND (entity_id IS NOT ?1 OR entity_type IS NOT ?2)",
                                params![sib_entity_id, sib_entity_type, now, email.email_id],
                            )
                            .map_err(|e| {
                                format!(
                                    "Failed to inherit thread entity for {}: {e}",
                                    email.email_id
                                )
                            })?;
                    }

                    // Same backward cascade for sentiment. Distinct from entity
                    // because sentiment can legitimately be NULL on a row
                    // (un-enriched), and we only want to inherit a non-null
                    // sibling value.
                    let sibling_sentiment: Option<String> = tx
                        .conn
                        .query_row(
                            "SELECT sentiment FROM emails
                             WHERE thread_id = ?1
                               AND email_id != ?2
                               AND resolved_at IS NULL
                               AND sentiment IS NOT NULL
                             ORDER BY updated_at DESC
                             LIMIT 1",
                            params![thread_id, email.email_id],
                            |row| row.get::<_, Option<String>>(0),
                        )
                        .ok()
                        .flatten();

                    if let Some(sib_sentiment) = sibling_sentiment {
                        tx.conn
                            .execute(
                                "UPDATE emails SET sentiment = ?1, updated_at = ?2
                                 WHERE email_id = ?3
                                   AND resolved_at IS NULL
                                   AND sentiment IS NOT ?1",
                                params![sib_sentiment, now, email.email_id],
                            )
                            .map_err(|e| {
                                format!(
                                    "Failed to inherit thread sentiment for {}: {e}",
                                    email.email_id
                                )
                            })?;
                    }
                }
            }

            Ok(())
        })
        .map_err(Into::into)
    }

    ///  rescue: clear the noise flag on an email so it surfaces again in
    /// inbox / Records. Used by the user-facing "this isn't noise" affordance
    /// (will wire the UI). Idempotent.
    pub fn unsuppress_email(&self, email_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE emails SET is_noise = 0, updated_at = ?1 WHERE email_id = ?2",
                params![now, email_id],
            )
            .map_err(|e| format!("Failed to unsuppress email {email_id}: {e}"))?;
        Ok(())
    }

    /// Mark emails as resolved (vanished from inbox). Sets `resolved_at` to now.
    /// Returns the number of rows updated.
    pub fn mark_emails_resolved(&self, vanished_ids: &[String]) -> Result<usize, DbError> {
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
        let param_values: Vec<&dyn rusqlite::types::ToSql> = vanished_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = self
            .conn
            .execute(&sql, param_values.as_slice())
            .map_err(|e| format!("Failed to mark emails resolved: {e}"))?;
        Ok(rows)
    }

    /// Unmark resolved emails that reappeared in inbox. Sets `resolved_at` to NULL.
    /// Returns the number of rows updated.
    pub fn unmark_resolved(&self, reappeared_ids: &[String]) -> Result<usize, DbError> {
        if reappeared_ids.is_empty() {
            return Ok(0);
        }
        let now = Utc::now().to_rfc3339();
        let placeholders: Vec<String> = (1..=reappeared_ids.len())
            .map(|i| format!("?{i}"))
            .collect();
        let sql = format!(
            "UPDATE emails SET resolved_at = NULL, updated_at = '{}' WHERE email_id IN ({})",
            now,
            placeholders.join(", ")
        );
        let param_values: Vec<&dyn rusqlite::types::ToSql> = reappeared_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = self
            .conn
            .execute(&sql, param_values.as_slice())
            .map_err(|e| format!("Failed to unmark resolved emails: {e}"))?;
        Ok(rows)
    }

    /// Get emails pending enrichment (state = 'pending' or 'failed', attempts < 3).
    pub fn get_pending_enrichment(&self, limit: usize) -> Result<Vec<DbEmail>, DbError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason,
                        pinned_at, commitments, questions, is_noise, to_recipients, cc_recipients
                 FROM emails
                 WHERE enrichment_state IN ('pending', 'pending_retry', 'failed')
                   AND enrichment_attempts < 3
                   AND is_noise = 0
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
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        // is_noise gets COALESCE-style "only update if AI gave a
        // verdict" semantics via a CASE expression — Some(bool) -> 0/1,
        // None -> keep existing column value.
        let is_noise_param: Option<i32> = enrichment.is_noise.map(|b| if b { 1 } else { 0 });
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
                    is_noise = COALESCE(?8, is_noise),
                    updated_at = ?2
                 WHERE email_id = ?9",
                params![
                    state,
                    now,
                    enrichment.summary,
                    enrichment.entity_id,
                    enrichment.entity_type,
                    enrichment.sentiment,
                    enrichment.urgency,
                    is_noise_param,
                    email_id,
                ],
            )
            .map_err(|e| format!("Failed to set enrichment state for {email_id}: {e}"))?;
        Ok(())
    }

    /// Get all active (non-resolved) emails.
    pub fn get_all_active_emails(&self) -> Result<Vec<DbEmail>, DbError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason,
                        pinned_at, commitments, questions, is_noise, to_recipients, cc_recipients
                 FROM emails
                 WHERE resolved_at IS NULL
                   AND is_noise = 0
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
    pub fn get_emails_for_entity(&self, entity_id: &str) -> Result<Vec<DbEmail>, DbError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason,
                        pinned_at, commitments, questions, is_noise, to_recipients, cc_recipients
                 FROM emails
                 WHERE entity_id = ?1 AND resolved_at IS NULL AND is_noise = 0
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
    ) -> Result<(), DbError> {
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
    pub fn get_email_sync_stats(&self) -> Result<EmailSyncStats, DbError> {
        let last_fetch_at: Option<String> = self
            .conn
            .query_row("SELECT MAX(last_seen_at) FROM emails", [], |row| row.get(0))
            .map_err(|e| format!("Failed to query last fetch time: {e}"))?;

        // noise emails are hidden from inbox/Records — counts must
        // reflect what the user sees, not the raw fetch volume.
        let total: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL AND is_noise = 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count active emails: {e}"))?;

        let enriched: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL AND is_noise = 0 AND enrichment_state = 'enriched'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count enriched emails: {e}"))?;

        let pending: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL AND is_noise = 0 AND enrichment_state IN ('pending', 'enriching')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count pending emails: {e}"))?;

        // `pending_retry` is a transitional state owned by the retry
        // service. It represents rows that were `failed` until the user
        // clicked Retry; they stay counted as failed in the UI so the
        // "Retry" notice remains visible until the in-flight refresh
        // confirms success (promoting them to `pending`) or fails
        // (rolling them back to `failed`).
        let failed: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE resolved_at IS NULL AND is_noise = 0 AND enrichment_state IN ('failed', 'pending_retry')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count failed emails: {e}"))?;

        // fetch the last successful Gmail fetch timestamp (separate from
        // per-row last_seen_at so the UI can tell "fetch healthy, enrichment stuck"
        // apart from "we can't reach Gmail").
        // propagate unexpected errors — swallowing them previously
        // masked schema drift (e.g. migration 094 never applied) as a silent
        // "never fetched" state.
        let last_successful_fetch_at = self.get_last_successful_fetch_at()?;

        // subset of `failed` that the system has stopped auto-retrying.
        // The `failed` count above includes rows still eligible for the next
        // refresh's stale-recovery promotion  — those don't need
        // user intervention. `permanently_failed` is what the user-facing
        // "couldn't be enriched" UX should display + act on.
        let permanently_failed: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails
                 WHERE resolved_at IS NULL
                   AND enrichment_state = 'failed'
                   AND COALESCE(auto_retry_count, 0) >= ?1",
                params![STALE_FAILED_MAX_AUTO_RETRIES],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count permanently failed emails: {e}"))?;

        Ok(EmailSyncStats {
            last_fetch_at,
            last_successful_fetch_at,
            total,
            enriched,
            pending,
            failed,
            permanently_failed,
        })
    }

    //: Record that a Gmail fetch just completed successfully.
    /// Writes to the singleton `email_sync_meta` row (migration 093/094).
    ///
    /// Previously this used a bare `UPDATE WHERE id = 1` without
    /// checking affected rows, so if the singleton seed row was ever missing
    /// (fresh install racing migrations, partial restore, manual DB edit),
    /// the write silently no-op'd and the UI showed "never fetched" forever.
    /// We now upsert so the row materializes on first call regardless of
    /// seed state, and assert exactly one row was written as a defense in
    /// depth against a future schema change (e.g. losing the `id = 1`
    /// singleton PK check).
    pub fn set_last_successful_fetch_at(&self) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self
            .conn
            .execute(
                "INSERT INTO email_sync_meta (id, last_successful_fetch_at, updated_at)
                 VALUES (1, ?1, ?1)
                 ON CONFLICT(id) DO UPDATE SET
                    last_successful_fetch_at = excluded.last_successful_fetch_at,
                    updated_at = excluded.updated_at",
                params![now],
            )
            .map_err(|e| format!("Failed to upsert email_sync_meta: {e}"))?;
        if rows != 1 {
            return Err(format!(
                "email_sync_meta upsert affected {rows} rows; expected 1 (schema drift?)"
            )
            .into());
        }
        Ok(())
    }

    /// Read the last successful Gmail fetch timestamp. `Ok(None)` means
    /// we've never completed a successful fetch (fresh install or the meta row
    /// was never seeded — which shouldn't happen post-migration-094).
    pub fn get_last_successful_fetch_at(&self) -> Result<Option<String>, DbError> {
        let result: Result<Option<String>, rusqlite::Error> = self.conn.query_row(
            "SELECT last_successful_fetch_at FROM email_sync_meta WHERE id = 1",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(ts) => Ok(ts),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to query last_successful_fetch_at: {e}").into()),
        }
    }

    /// Update the entity assignment for an email (user correction).
    /// Also cascades the change to `email_signals`: moves signals to the new entity,
    /// or deactivates them if entity_id is cleared.
    ///
    /// Cascades the entity assignment to every UNRESOLVED row in the
    /// same `thread_id`. The inbox UI is thread-collapsed (see
    /// `services::emails::collapse_to_latest_thread_emails`) and picks the row
    /// with the newest `received_at` as the visible representative. Without
    /// this cascade, a user-correction on the visible row was hidden as soon
    /// as a newer sibling arrived (silent Gmail poll) — the auto-classified
    /// `entity_id` on the new row would win the collapse and look like the
    /// edit had reverted. Mirrors `archive_email`'s thread-wide approach.
    ///
    /// Resolved (archived) rows are intentionally left alone so they keep
    /// the entity that was correct at the time they were archived.
    ///
    /// All writes (per-row entity update + per-row signal cascade) run in a
    /// single transaction so a partial cascade can't leave the thread in
    /// inconsistent state.
    pub fn update_email_entity(
        &self,
        email_id: &str,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
    ) -> Result<(), DbError> {
        self.with_transaction(|tx| {
            let now = chrono::Utc::now().to_rfc3339();

            // Collect the full set of rows to update: the email itself plus
            // every other unresolved row sharing its thread_id. We resolve
            // this up front so the signal cascade below can iterate the same
            // set without reading from a half-mutated table.
            let thread_id: Option<String> = tx
                .conn
                .query_row(
                    "SELECT thread_id FROM emails WHERE email_id = ?1",
                    rusqlite::params![email_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to load thread for {email_id}: {e}"))?;

            let cascade_ids: Vec<String> = match thread_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
            {
                Some(tid) => {
                    let mut stmt = tx
                        .conn
                        .prepare(
                            "SELECT email_id FROM emails
                             WHERE (email_id = ?1 OR thread_id = ?2)
                               AND resolved_at IS NULL",
                        )
                        .map_err(|e| {
                            format!("Failed to prepare thread cascade query for {email_id}: {e}")
                        })?;
                    let rows = stmt
                        .query_map(rusqlite::params![email_id, tid], |row| {
                            row.get::<_, String>(0)
                        })
                        .map_err(|e| {
                            format!("Failed to query thread cascade ids for {email_id}: {e}")
                        })?;
                    let mut ids = Vec::new();
                    for r in rows {
                        ids.push(r.map_err(|e| {
                            format!("Failed to read cascade id row for {email_id}: {e}")
                        })?);
                    }
                    if ids.is_empty() {
                        vec![email_id.to_string()]
                    } else {
                        ids
                    }
                }
                None => vec![email_id.to_string()],
            };

            // cascade the entity assignment to every unresolved row in
            // the thread. Bounded by `resolved_at IS NULL` so archived rows
            // keep their historical entity.
            tx.conn
                .execute(
                    "UPDATE emails SET entity_id = ?1, entity_type = ?2, updated_at = ?3
                     WHERE email_id = ?4 AND resolved_at IS NULL",
                    rusqlite::params![entity_id, entity_type, now, email_id],
                )
                .map_err(|e| format!("Failed to update email entity for {email_id}: {e}"))?;

            for sibling_id in cascade_ids.iter().filter(|id| id.as_str() != email_id) {
                tx.conn
                    .execute(
                        "UPDATE emails SET entity_id = ?1, entity_type = ?2, updated_at = ?3
                         WHERE email_id = ?4 AND resolved_at IS NULL",
                        rusqlite::params![entity_id, entity_type, now, sibling_id],
                    )
                    .map_err(|e| {
                        format!("Failed to cascade email entity to thread sibling {sibling_id}: {e}")
                    })?;
            }

            // Cascade to email_signals for every row in the thread set so the
            // signal store agrees with the new entity assignment everywhere.
            for cascade_email_id in &cascade_ids {
                match entity_id {
                    Some(new_entity_id) => {
                        // Move signals to the new entity. UPDATE OR IGNORE skips
                        // rows that would violate the unique constraint
                        // (email_id, entity_id, signal_type).
                        tx.conn
                            .execute(
                                "UPDATE OR IGNORE email_signals SET entity_id = ?1, entity_type = COALESCE(?2, entity_type)
                                 WHERE email_id = ?3 AND entity_id != ?1 AND deactivated_at IS NULL",
                                rusqlite::params![new_entity_id, entity_type, cascade_email_id],
                            )
                            .map_err(|e| format!("Failed to move email signals for {cascade_email_id}: {e}"))?;
                        // Delete any constraint-blocked duplicates that couldn't move
                        tx.conn
                            .execute(
                                "DELETE FROM email_signals
                                 WHERE email_id = ?1 AND entity_id != ?2 AND deactivated_at IS NULL",
                                rusqlite::params![cascade_email_id, new_entity_id],
                            )
                            .map_err(|e| {
                                format!("Failed to clean duplicate signals for {cascade_email_id}: {e}")
                            })?;
                    }
                    None => {
                        // Entity cleared — deactivate signals instead of deleting
                        tx.conn
                            .execute(
                                "UPDATE email_signals SET deactivated_at = ?1
                                 WHERE email_id = ?2 AND deactivated_at IS NULL",
                                rusqlite::params![now, cascade_email_id],
                            )
                            .map_err(|e| format!("Failed to deactivate signals for {cascade_email_id}: {e}"))?;
                    }
                }
            }

            Ok(())
        })
        .map_err(Into::into)
    }

    /// Update the sentiment of an email and cascade the same value to
    /// every unresolved row in the thread. See `update_email_entity` for the
    /// thread-collapse rationale — the same reversion pattern applies to
    /// sentiment, which is also a per-row column read through the
    /// thread-collapsed inbox view.
    pub fn update_email_sentiment(
        &self,
        email_id: &str,
        sentiment: Option<&str>,
    ) -> Result<(), DbError> {
        self.with_transaction(|tx| {
            let now = chrono::Utc::now().to_rfc3339();

            let thread_id: Option<String> = tx
                .conn
                .query_row(
                    "SELECT thread_id FROM emails WHERE email_id = ?1",
                    rusqlite::params![email_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to load thread for {email_id}: {e}"))?;

            match thread_id.as_deref().filter(|id| !id.trim().is_empty()) {
                Some(tid) => {
                    tx.conn
                        .execute(
                            "UPDATE emails SET sentiment = ?1, updated_at = ?2
                             WHERE (email_id = ?3 OR thread_id = ?4)
                               AND resolved_at IS NULL",
                            rusqlite::params![sentiment, now, email_id, tid],
                        )
                        .map_err(|e| format!("Failed to cascade sentiment for {email_id}: {e}"))?;
                }
                None => {
                    tx.conn
                        .execute(
                            "UPDATE emails SET sentiment = ?1, updated_at = ?2
                             WHERE email_id = ?3 AND resolved_at IS NULL",
                            rusqlite::params![sentiment, now, email_id],
                        )
                        .map_err(|e| format!("Failed to update sentiment for {email_id}: {e}"))?;
                }
            }

            Ok(())
        })
        .map_err(Into::into)
    }

    /// Mark an email as replied to by the user (reply debt).
    /// Sets `user_is_last_sender = 1` on the email row and the corresponding
    /// email_threads row (if any).
    pub fn mark_reply_sent(&self, email_id: &str) -> Result<Option<(String, String)>, DbError> {
        let now = chrono::Utc::now().to_rfc3339();

        let entity_info: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT entity_id, entity_type FROM emails WHERE email_id = ?1",
                rusqlite::params![email_id],
                |row| {
                    let eid: Option<String> = row.get(0)?;
                    let etype: Option<String> = row.get(1)?;
                    Ok(eid.zip(etype))
                },
            )
            .ok()
            .flatten();

        self.conn
            .execute(
                "UPDATE emails SET user_is_last_sender = 1, updated_at = ?1 WHERE email_id = ?2",
                rusqlite::params![now, email_id],
            )
            .map_err(|e| format!("Failed to mark reply sent for {email_id}: {e}"))?;

        self.conn
            .execute(
                "UPDATE email_threads SET user_is_last_sender = 1, updated_at = datetime('now')
                 WHERE thread_id = (SELECT thread_id FROM emails WHERE email_id = ?1)",
                rusqlite::params![email_id],
            )
            .map_err(|e| format!("Failed to update email_threads for {email_id}: {e}"))?;

        Ok(entity_info)
    }

    /// Archive an email thread by setting resolved_at on every row in the thread.
    /// The inbox UI is thread-collapsed, so archiving only one message can let an
    /// older unresolved message in the same thread leak back into view.
    pub fn archive_email(&self, email_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let thread_id = self.get_thread_id(email_id)?;

        self.conn
            .execute(
                "UPDATE emails
                 SET resolved_at = ?1, updated_at = ?1
                 WHERE email_id = ?2
                    OR (thread_id IS NOT NULL AND thread_id = ?3)",
                params![now, email_id, thread_id],
            )
            .map_err(|e| format!("Failed to archive email {email_id}: {e}"))?;
        Ok(())
    }

    /// Unarchive an email thread by clearing resolved_at on every row in the thread.
    pub fn unarchive_email(&self, email_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let thread_id = self.get_thread_id(email_id)?;

        self.conn
            .execute(
                "UPDATE emails
                 SET resolved_at = NULL, updated_at = ?1
                 WHERE email_id = ?2
                    OR (thread_id IS NOT NULL AND thread_id = ?3)",
                params![now, email_id, thread_id],
            )
            .map_err(|e| format!("Failed to unarchive email {email_id}: {e}"))?;
        Ok(())
    }

    /// Return all known email IDs in the same thread as the given email.
    /// Falls back to the single email ID when thread metadata is absent.
    pub fn get_thread_email_ids(&self, email_id: &str) -> Result<Vec<String>, DbError> {
        let thread_id = self.get_thread_id(email_id)?;
        let Some(thread_id) = thread_id.filter(|id| !id.trim().is_empty()) else {
            return Ok(vec![email_id.to_string()]);
        };

        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id
                 FROM emails
                 WHERE email_id = ?1 OR thread_id = ?2
                 ORDER BY received_at DESC",
            )
            .map_err(|e| format!("Failed to prepare thread email lookup for {email_id}: {e}"))?;

        let rows = stmt
            .query_map(params![email_id, thread_id], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query thread email IDs for {email_id}: {e}"))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| format!("Failed to read thread email ID: {e}"))?);
        }
        if ids.is_empty() {
            ids.push(email_id.to_string());
        }
        Ok(ids)
    }

    fn get_thread_id(&self, email_id: &str) -> Result<Option<String>, DbError> {
        self.conn
            .query_row(
                "SELECT thread_id FROM emails WHERE email_id = ?1",
                params![email_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to load thread for {email_id}: {e}").into())
    }

    /// Toggle pin on an email. If pinned, clears; if not pinned, sets to now.
    /// Returns the new pinned state (true = pinned).
    pub fn toggle_pin_email(&self, email_id: &str) -> Result<bool, DbError> {
        let now = Utc::now().to_rfc3339();
        let current_pinned: Option<String> = self
            .conn
            .query_row(
                "SELECT pinned_at FROM emails WHERE email_id = ?1",
                params![email_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to read pin state for {email_id}: {e}"))?;

        let is_now_pinned = current_pinned.is_none();
        if is_now_pinned {
            self.conn
                .execute(
                    "UPDATE emails SET pinned_at = ?1, updated_at = ?1 WHERE email_id = ?2",
                    params![now, email_id],
                )
                .map_err(|e| format!("Failed to pin email {email_id}: {e}"))?;
        } else {
            self.conn
                .execute(
                    "UPDATE emails SET pinned_at = NULL, updated_at = ?1 WHERE email_id = ?2",
                    params![now, email_id],
                )
                .map_err(|e| format!("Failed to unpin email {email_id}: {e}"))?;
        }
        Ok(is_now_pinned)
    }

    /// Set the relevance score and reason for an email.
    pub fn set_relevance_score(
        &self,
        email_id: &str,
        score: f64,
        reason: &str,
    ) -> Result<(), DbError> {
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
    pub fn get_emails_by_score(
        &self,
        min_score: f64,
        limit: usize,
    ) -> Result<Vec<DbEmail>, DbError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason,
                        pinned_at, commitments, questions, is_noise, to_recipients, cc_recipients
                 FROM emails
                 WHERE resolved_at IS NULL
                   AND is_noise = 0
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

    /// Get threads awaiting reply (not resolved, user is not last sender).
    /// Does NOT require is_unread — a thread can be read but still awaiting reply.
    pub fn get_emails_awaiting_reply(&self) -> Result<Vec<DbEmail>, DbError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, thread_id, sender_email, sender_name, subject, snippet,
                        priority, is_unread, received_at, enrichment_state, enrichment_attempts,
                        last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
                        contextual_summary, sentiment, urgency, user_is_last_sender,
                        last_sender_email, message_count, created_at, updated_at,
                        relevance_score, score_reason,
                        pinned_at, commitments, questions, is_noise, to_recipients, cc_recipients
                 FROM emails
                 WHERE user_is_last_sender = 0
                   AND resolved_at IS NULL
                   AND is_noise = 0
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

    /// Transition `failed` emails to the transitional `pending_retry`
    /// state while a user-initiated retry is in flight. Stamps `retry_batch_id`
    /// and `retry_started_at` so concurrent refreshes and crash-recovery can
    /// tell this batch's rows apart from any rows stranded by a prior run
    /// (Codex finding 2).
    ///
    /// Unlike a direct `failed -> pending` reset (the pre- behaviour),
    /// this keeps `enrichment_attempts` intact so we can roll back cleanly
    /// if the subsequent Gmail refresh fails, and the UI failed-count query
    /// continues to include these rows so the "Retry" notice stays visible
    /// until the refresh outcome is known.
    ///
    /// Returns the number of rows transitioned.
    pub fn mark_failed_for_retry(&self, batch_id: &str) -> Result<usize, DbError> {
        // wrap in `with_transaction` so the state flip + batch_id stamp
        // + started_at stamp commit atomically. Today this is a single UPDATE so
        // SQLite's implicit transaction is sufficient, but the retry primitive
        // is the documented seam where future signal emissions / audit rows
        // attach (Intelligence Loop check). A transaction boundary here means
        // those additions cannot leave a half-marked batch behind.
        self.with_transaction(|tx| {
            let now = Utc::now().to_rfc3339();
            tx.conn
                .execute(
                    "UPDATE emails
                     SET enrichment_state = 'pending_retry',
                         retry_batch_id = ?1,
                         retry_started_at = ?2,
                         updated_at = ?2
                     WHERE enrichment_state = 'failed' AND resolved_at IS NULL",
                    params![batch_id, now],
                )
                .map_err(|e| format!("Failed to mark emails for retry: {e}"))
        })
        .map_err(Into::into)
    }

    ///  Promote this batch's `pending_retry` rows
    /// to `pending` and zero `enrichment_attempts` so the enrichment pipeline
    /// can pick them up. MUST be called BEFORE the enrichment pass that is
    /// meant to process the retried rows — `get_pending_enrichment` filters
    /// `enrichment_attempts < 3`, so rows left in `pending_retry` at attempts=3
    /// are skipped entirely by enrichment. Pre-fix the UI reported "retrying,
    /// cleared" while zero work happened.
    ///
    /// Scoped to `batch_id` (Codex finding 2) so a finalize from refresh A
    /// cannot accidentally adopt rows owned by refresh B.
    ///
    /// Returns the number of rows transitioned.
    pub fn finalize_pending_retry_success(&self, batch_id: &str) -> Result<usize, DbError> {
        // transactional so the state flip + attempts reset + batch_id
        // clear commit together. Splitting these would let a crash mid-finalize
        // leave a row at state=pending with a stale batch_id pointing at this
        // refresh — invisible to both the retry and the stale-recovery passes.
        self.with_transaction(|tx| {
            let now = Utc::now().to_rfc3339();
            tx.conn
                .execute(
                    "UPDATE emails
                     SET enrichment_state = 'pending',
                         enrichment_attempts = 0,
                         retry_batch_id = NULL,
                         retry_started_at = NULL,
                         updated_at = ?1
                     WHERE enrichment_state = 'pending_retry'
                       AND retry_batch_id = ?2
                       AND resolved_at IS NULL",
                    params![now, batch_id],
                )
                .map_err(|e| format!("Failed to finalize retry (success): {e}"))
        })
        .map_err(Into::into)
    }

    /// Roll this batch's `pending_retry` rows back to `failed` after
    /// the Gmail refresh failed. The user's "Retry" notice reappears and they
    /// can try again. `enrichment_attempts` was never touched, so the row
    /// returns to exactly its pre-retry state. Scoped to `batch_id` (Codex
    /// finding 2) so concurrent refreshes cannot clobber each other.
    ///
    /// Returns the number of rows transitioned.
    pub fn rollback_pending_retry(&self, batch_id: &str) -> Result<usize, DbError> {
        // transactional rollback. If we're going to surface the
        // rollback error to the caller (Codex finding 2), the rollback itself
        // must be atomic — a partial rollback that reports failure would leave
        // some rows in `failed` and others in `pending_retry`, confusing both
        // the next retry attempt and the stale-recovery pass.
        self.with_transaction(|tx| {
            let now = Utc::now().to_rfc3339();
            tx.conn
                .execute(
                    "UPDATE emails
                     SET enrichment_state = 'failed',
                         retry_batch_id = NULL,
                         retry_started_at = NULL,
                         updated_at = ?1
                     WHERE enrichment_state = 'pending_retry'
                       AND retry_batch_id = ?2
                       AND resolved_at IS NULL",
                    params![now, batch_id],
                )
                .map_err(|e| format!("Failed to roll back retry: {e}"))
        })
        .map_err(Into::into)
    }

    ///  Roll back `pending_retry` rows stranded by
    /// a crashed or never-finalized refresh. Called at the start of every
    /// refresh so stale batches from a previous process are recovered before
    /// the current batch is stamped.
    ///
    /// A row counts as "stale" if it's in `pending_retry` and either:
    /// - has no `retry_batch_id` (migrated from the pre-batching schema, or
    ///   a write crashed between the state flip and the batch_id stamp), or
    /// - its `retry_started_at` is older than `stale_after_seconds`.
    ///
    /// Returns the number of rows rolled back to `failed`.
    pub fn rollback_stale_pending_retry(&self, stale_after_seconds: i64) -> Result<usize, DbError> {
        let now = Utc::now();
        let cutoff = now - chrono::Duration::seconds(stale_after_seconds);
        let now_iso = now.to_rfc3339();
        let cutoff_iso = cutoff.to_rfc3339();
        self.conn
            .execute(
                "UPDATE emails
                 SET enrichment_state = 'failed',
                     retry_batch_id = NULL,
                     retry_started_at = NULL,
                     updated_at = ?1
                 WHERE enrichment_state = 'pending_retry'
                   AND resolved_at IS NULL
                   AND (retry_batch_id IS NULL OR retry_started_at IS NULL OR retry_started_at < ?2)",
                params![now_iso, cutoff_iso],
            )
            .map_err(|e| format!("Failed to roll back stale retries: {e}").into())
    }

    /// Auto-retry stale `failed` emails on every refresh.
    ///
    /// The original failure mode: an enrichment error (PTY hiccup, transient
    /// API failure, classification edge case) bumped a row to `failed` and
    /// it stayed there forever. The user had to notice the "Retry" notice
    /// and click it manually for each batch. Failed rows accumulated and
    /// the inbox always looked partially broken.
    ///
    /// New behaviour: on every refresh, any `failed` row that is BOTH
    /// (a) older than `stale_after_seconds` (measured against
    ///     `last_enrichment_at`, falling back to `created_at`) and
    /// (b) under the cumulative `max_auto_retries` cap
    /// is silently promoted back to `pending` with `enrichment_attempts = 0`
    /// and `auto_retry_count` incremented. The next enrichment pass picks
    /// it up automatically — no user action, no UI notice.
    ///
    /// Rows at or above the cap stay in `failed` and surface in the
    /// "couldn't be enriched" UX  where the user decides between
    /// one more manual retry or skipping permanently.
    ///
    /// Staleness is measured against `last_enrichment_at` so we don't
    /// instantly re-attempt a brand-new failure. `created_at` is the
    /// fallback for rows that never recorded an enrichment attempt.
    ///
    /// Returns the number of rows promoted to `pending`.
    pub fn auto_retry_stale_failed(
        &self,
        stale_after_seconds: i64,
        max_auto_retries: i32,
    ) -> Result<usize, DbError> {
        self.with_transaction(|tx| {
            let now = Utc::now();
            let cutoff = now - chrono::Duration::seconds(stale_after_seconds);
            let now_iso = now.to_rfc3339();
            let cutoff_iso = cutoff.to_rfc3339();
            tx.conn
                .execute(
                    "UPDATE emails
                     SET enrichment_state = 'pending',
                         enrichment_attempts = 0,
                         auto_retry_count = COALESCE(auto_retry_count, 0) + 1,
                         updated_at = ?1
                     WHERE enrichment_state = 'failed'
                       AND resolved_at IS NULL
                       AND COALESCE(auto_retry_count, 0) < ?2
                       AND COALESCE(last_enrichment_at, created_at) < ?3",
                    params![now_iso, max_auto_retries, cutoff_iso],
                )
                .map_err(|e| format!("Failed to auto-retry stale failed emails: {e}"))
        })
        .map_err(Into::into)
    }

    /// Count failed rows that have exhausted automatic retries
    /// (`auto_retry_count >= cap`). These are the rows the user-facing
    /// "couldn't be enriched" UX surfaces — rows the system has stopped
    /// trying to fix on its own. Distinct from the broader failed-count
    /// which still includes rows eligible for the next refresh's
    /// auto-retry pass (and shouldn't bother the user yet).
    pub fn count_permanently_failed_emails(&self, max_auto_retries: i32) -> Result<usize, DbError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM emails
                 WHERE resolved_at IS NULL
                   AND enrichment_state = 'failed'
                   AND COALESCE(auto_retry_count, 0) >= ?1",
                params![max_auto_retries],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n as usize)
            .map_err(|e| format!("Failed to count permanently failed emails: {e}").into())
    }

    /// Return a small preview of permanently-failed emails so the
    /// "View details" affordance can show subjects + senders without the
    /// caller having to fetch the full email list. Capped at `limit` rows.
    pub fn list_permanently_failed_previews(
        &self,
        max_auto_retries: i32,
        limit: usize,
    ) -> Result<Vec<FailedEmailPreview>, DbError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT email_id, subject, sender_email, sender_name, last_enrichment_at,
                        COALESCE(auto_retry_count, 0)
                 FROM emails
                 WHERE resolved_at IS NULL
                   AND enrichment_state = 'failed'
                   AND COALESCE(auto_retry_count, 0) >= ?1
                 ORDER BY COALESCE(last_enrichment_at, created_at) DESC
                 LIMIT ?2",
            )
            .map_err(|e| format!("Failed to prepare failed-preview query: {e}"))?;
        let rows = stmt
            .query_map(params![max_auto_retries, limit as i64], |row| {
                Ok(FailedEmailPreview {
                    email_id: row.get(0)?,
                    subject: row.get(1)?,
                    sender_email: row.get(2)?,
                    sender_name: row.get(3)?,
                    last_enrichment_at: row.get(4)?,
                    auto_retry_count: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query failed-email previews: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Failed to read failed-preview row: {e}"))?);
        }
        Ok(out)
    }

    /// User-initiated "Skip" — mark these failed rows resolved so
    /// they leave the failed-count entirely. The Gmail message stays in
    /// the inbox; we just stop trying to enrich it and stop surfacing it
    /// as a failure. Returns rows affected.
    pub fn skip_failed_emails(&self, email_ids: &[String]) -> Result<usize, DbError> {
        if email_ids.is_empty() {
            return Ok(0);
        }
        self.with_transaction(|tx| {
            let now = Utc::now().to_rfc3339();
            let placeholders: Vec<String> = (2..=(email_ids.len() + 1))
                .map(|i| format!("?{i}"))
                .collect();
            let sql = format!(
                "UPDATE emails
                 SET resolved_at = ?1, updated_at = ?1
                 WHERE enrichment_state = 'failed'
                   AND resolved_at IS NULL
                   AND email_id IN ({})",
                placeholders.join(", ")
            );
            let mut params_vec: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(1 + email_ids.len());
            params_vec.push(&now);
            for id in email_ids {
                params_vec.push(id);
            }
            tx.conn
                .execute(&sql, params_vec.as_slice())
                .map_err(|e| format!("Failed to skip failed emails: {e}"))
        })
        .map_err(Into::into)
    }

    ///  Count rows that are either `failed` or
    /// stuck in `pending_retry`. Used by `retry_failed_emails` so a user
    /// clicking Retry still triggers a refresh even if all their rows
    /// were orphaned by a prior crashed refresh (the pre-fix count
    /// matched only `failed` and silently returned 0 in this case).
    pub fn count_retriable_emails(&self) -> Result<usize, DbError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM emails
                 WHERE enrichment_state IN ('failed', 'pending_retry')
                   AND resolved_at IS NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n as usize)
            .map_err(|e| format!("Failed to count retriable emails: {e}").into())
    }

    /// Mark an email as enriched, setting `enriched_at` to now.
    /// Used after successful enrichment to support Gate 0 deduplication.
    pub fn mark_email_enriched(&self, email_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE emails SET enriched_at = ?1, updated_at = ?1 WHERE email_id = ?2",
                params![now, email_id],
            )
            .map_err(|e| format!("Failed to mark email enriched {email_id}: {e}"))?;
        Ok(())
    }

    /// Get snapshot of email content (snippet + subject) for all provided email IDs.
    /// Used in Gate 0 to detect content changes for re-enrichment eligibility.
    pub fn get_email_snapshots(
        &self,
        email_ids: &[String],
    ) -> Result<HashMap<String, crate::workflow::email_filter::PriorEmailSnapshot>, DbError> {
        if email_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut result = HashMap::new();
        for email_id in email_ids {
            match self.conn.query_row(
                "SELECT snippet, subject FROM emails WHERE email_id = ?1",
                params![email_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            ) {
                Ok((snippet, subject)) => {
                    result.insert(
                        email_id.clone(),
                        crate::workflow::email_filter::PriorEmailSnapshot { snippet, subject },
                    );
                }
                Err(_) => {
                    // Email not found or query failed — skip it
                }
            }
        }

        Ok(result)
    }
}

/// Parameters for enrichment state updates (avoids too_many_arguments lint).
pub struct EmailEnrichmentUpdate<'a> {
    pub summary: Option<&'a str>,
    pub entity_id: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub sentiment: Option<&'a str>,
    pub urgency: Option<&'a str>,
    /// LLM-determined noise verdict. None = no opinion (don't
    /// change the deterministic value); Some(true) = AI says noise;
    /// Some(false) = AI says signal (overrides any prior is_noise=1).
    pub is_noise: Option<bool>,
}

/// Row mapper for emails SELECT queries (33 columns).
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
        enriched_at: row.get(12)?,
        last_seen_at: row.get(13)?,
        resolved_at: row.get(14)?,
        entity_id: row.get(15)?,
        entity_type: row.get(16)?,
        contextual_summary: row.get(17)?,
        sentiment: row.get(18)?,
        urgency: row.get(19)?,
        user_is_last_sender: row.get::<_, i32>(20)? != 0,
        last_sender_email: row.get(21)?,
        message_count: row.get(22)?,
        created_at: row.get(23)?,
        updated_at: row.get(24)?,
        relevance_score: row.get(25).ok(),
        score_reason: row.get(26).ok(),
        pinned_at: row.get(27).ok(),
        commitments: row.get(28).ok(),
        questions: row.get(29).ok(),
        // column added by migration 092. Default false on legacy rows.
        is_noise: row.get::<_, i32>(30).map(|v| v != 0).unwrap_or(false),
        // columns added by migration 120. Default None on legacy rows.
        to_recipients: row.get(31).ok(),
        cc_recipients: row.get(32).ok(),
    })
}
