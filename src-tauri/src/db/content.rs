use super::*;

impl ActionDb {
    // =========================================================================
    // Content Index (I124)
    // =========================================================================

    /// Upsert a content file record. Preserves existing `extracted_at` / `summary`
    /// when the incoming record has `None` for those fields (COALESCE pattern).
    pub fn upsert_content_file(&self, file: &DbContentFile) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO content_index (
                id, entity_id, entity_type, filename, relative_path, absolute_path,
                format, file_size, modified_at, indexed_at, extracted_at, summary,
                embeddings_generated_at, content_type, priority
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                filename = excluded.filename,
                relative_path = excluded.relative_path,
                absolute_path = excluded.absolute_path,
                format = excluded.format,
                file_size = excluded.file_size,
                modified_at = excluded.modified_at,
                indexed_at = excluded.indexed_at,
                extracted_at = COALESCE(excluded.extracted_at, content_index.extracted_at),
                summary = COALESCE(excluded.summary, content_index.summary),
                embeddings_generated_at = excluded.embeddings_generated_at,
                content_type = excluded.content_type,
                priority = excluded.priority",
            params![
                file.id,
                file.entity_id,
                file.entity_type,
                file.filename,
                file.relative_path,
                file.absolute_path,
                file.format,
                file.file_size,
                file.modified_at,
                file.indexed_at,
                file.extracted_at,
                file.summary,
                file.embeddings_generated_at,
                file.content_type,
                file.priority,
            ],
        )?;
        Ok(())
    }

    /// Get all indexed files for an entity, highest priority first, then most recently modified.
    pub fn get_entity_files(&self, entity_id: &str) -> Result<Vec<DbContentFile>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, filename, relative_path, absolute_path,
                    format, file_size, modified_at, indexed_at, extracted_at, summary,
                    embeddings_generated_at, content_type, priority
             FROM content_index WHERE entity_id = ?1
             ORDER BY priority DESC, modified_at DESC",
        )?;
        let rows = stmt.query_map(params![entity_id], |row| {
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

    /// Delete a single content file record by ID.
    pub fn delete_content_file(&self, id: &str) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM content_index WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete all content file records for an entity.
    pub fn delete_entity_files(&self, entity_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM content_index WHERE entity_id = ?1",
            params![entity_id],
        )?;
        Ok(())
    }

    /// Update extraction results for a content file: summary, content_type, and priority.
    pub fn update_content_extraction(
        &self,
        id: &str,
        extracted_at: &str,
        summary: Option<&str>,
        content_type: Option<&str>,
        priority: Option<i32>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE content_index SET extracted_at = ?1, summary = ?2,
                    content_type = COALESCE(?3, content_type),
                    priority = COALESCE(?4, priority)
             WHERE id = ?5",
            params![extracted_at, summary, content_type, priority, id],
        )?;
        Ok(())
    }

    /// Update the embeddings watermark for a content file.
    pub fn set_embeddings_generated_at(
        &self,
        id: &str,
        embeddings_generated_at: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE content_index
             SET embeddings_generated_at = ?1
             WHERE id = ?2",
            params![embeddings_generated_at, id],
        )?;
        Ok(())
    }

    /// Replace all embedding chunks for a file atomically.
    pub fn replace_content_embeddings_for_file(
        &self,
        content_file_id: &str,
        chunks: &[DbContentEmbedding],
    ) -> Result<(), DbError> {
        self.with_transaction(|tx| {
            tx.conn
                .execute(
                    "DELETE FROM content_embeddings WHERE content_file_id = ?1",
                    params![content_file_id],
                )
                .map_err(|e| format!("failed deleting prior embeddings: {e}"))?;

            for chunk in chunks {
                tx.conn
                    .execute(
                        "INSERT INTO content_embeddings (
                            id, content_file_id, chunk_index, chunk_text, embedding, created_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            chunk.id,
                            chunk.content_file_id,
                            chunk.chunk_index,
                            chunk.chunk_text,
                            chunk.embedding,
                            chunk.created_at,
                        ],
                    )
                    .map_err(|e| format!("failed inserting content embedding: {e}"))?;
            }

            Ok(())
        })
        .map_err(DbError::Migration)?;

        Ok(())
    }

    /// Files requiring embedding generation.
    pub fn get_files_needing_embeddings(
        &self,
        limit: usize,
    ) -> Result<Vec<DbContentFile>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, filename, relative_path, absolute_path,
                    format, file_size, modified_at, indexed_at, extracted_at, summary,
                    embeddings_generated_at, content_type, priority
             FROM content_index
             WHERE embeddings_generated_at IS NULL OR embeddings_generated_at < modified_at
             ORDER BY modified_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
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

    /// Returns all embedding chunks for an entity.
    pub fn get_entity_embedding_chunks(
        &self,
        entity_id: &str,
    ) -> Result<Vec<DbContentEmbedding>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT ce.id, ce.content_file_id, ce.chunk_index, ce.chunk_text, ce.embedding, ce.created_at
             FROM content_embeddings ce
             JOIN content_index ci ON ci.id = ce.content_file_id
             WHERE ci.entity_id = ?1
             ORDER BY ce.chunk_index ASC",
        )?;
        let rows = stmt.query_map(params![entity_id], |row| {
            Ok(DbContentEmbedding {
                id: row.get(0)?,
                content_file_id: row.get(1)?,
                chunk_index: row.get(2)?,
                chunk_text: row.get(3)?,
                embedding: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Returns all entities that have at least one indexed content file.
    pub fn get_entities_with_content(&self) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT entity_id, entity_type
             FROM content_index
             ORDER BY entity_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }


    // =========================================================================
    // Chat Sessions (Sprint 26)
    // =========================================================================

    pub fn create_chat_session(
        &self,
        id: &str,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
        session_start: &str,
        created_at: &str,
    ) -> Result<DbChatSession, DbError> {
        self.conn.execute(
            "INSERT INTO chat_sessions (
                id, entity_id, entity_type, session_start, session_end, turn_count, last_message, created_at
             ) VALUES (?1, ?2, ?3, ?4, NULL, 0, NULL, ?5)",
            params![id, entity_id, entity_type, session_start, created_at],
        )?;
        Ok(DbChatSession {
            id: id.to_string(),
            entity_id: entity_id.map(|s| s.to_string()),
            entity_type: entity_type.map(|s| s.to_string()),
            session_start: session_start.to_string(),
            session_end: None,
            turn_count: 0,
            last_message: None,
            created_at: created_at.to_string(),
        })
    }

    pub fn get_open_chat_session(
        &self,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
    ) -> Result<Option<DbChatSession>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, session_start, session_end, turn_count, last_message, created_at
             FROM chat_sessions
             WHERE session_end IS NULL
               AND (
                    (?1 IS NULL AND entity_id IS NULL AND entity_type IS NULL)
                    OR (entity_id = ?1 AND ((?2 IS NULL AND entity_type IS NULL) OR entity_type = ?2))
               )
             ORDER BY session_start DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![entity_id, entity_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(DbChatSession {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                session_start: row.get(3)?,
                session_end: row.get(4)?,
                turn_count: row.get(5)?,
                last_message: row.get(6)?,
                created_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_next_chat_turn_index(&self, session_id: &str) -> Result<i32, DbError> {
        let idx: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(turn_index) + 1, 0) FROM chat_turns WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(idx)
    }

    pub fn append_chat_turn(
        &self,
        id: &str,
        session_id: &str,
        turn_index: i32,
        role: &str,
        content: &str,
        timestamp: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO chat_turns (id, session_id, turn_index, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, session_id, turn_index, role, content, timestamp],
        )?;
        Ok(())
    }

    pub fn bump_chat_session_stats(
        &self,
        session_id: &str,
        turn_delta: i32,
        last_message: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE chat_sessions
             SET turn_count = turn_count + ?1,
                 last_message = COALESCE(?2, last_message)
             WHERE id = ?3",
            params![turn_delta, last_message, session_id],
        )?;
        Ok(())
    }

    pub fn get_chat_session_turns(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<DbChatTurn>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, turn_index, role, content, timestamp
             FROM chat_turns
             WHERE session_id = ?1
             ORDER BY turn_index ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![session_id, limit as i64], |row| {
            Ok(DbChatTurn {
                id: row.get(0)?,
                session_id: row.get(1)?,
                turn_index: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Compute temperature from an optional last-seen timestamp.
    pub(crate) fn compute_temperature(last_seen: &Option<String>) -> String {
        match last_seen {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        }
    }

    /// Compute trend from 30d and 90d frequencies.
    pub(crate) fn compute_trend(freq_30d: i32, freq_90d: i32) -> String {
        compute_trend(freq_30d, freq_90d)
    }

}
