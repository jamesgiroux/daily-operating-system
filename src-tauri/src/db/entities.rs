use super::*;

impl ActionDb {
    // =========================================================================
    // Entities (ADR-0045)
    // =========================================================================

    /// Insert or update a profile-agnostic entity.
    pub fn upsert_entity(&self, entity: &DbEntity) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                entity_type = excluded.entity_type,
                tracker_path = excluded.tracker_path,
                updated_at = excluded.updated_at",
            params![
                entity.id,
                entity.name,
                entity.entity_type.as_str(),
                entity.tracker_path,
                entity.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Fetch an entity by ID.
    pub fn get_entity(&self, id: &str) -> Result<Option<DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, tracker_path, updated_at
             FROM entities WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let et: String = row.get(2)?;
            Ok(DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Touch `updated_at` on an entity as a last-contact signal.
    ///
    /// Matches by ID or by case-insensitive name. Returns `true` if a row
    /// was updated, `false` if no entity matched.
    pub fn touch_entity_last_contact(&self, name: &str) -> Result<bool, DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE entities SET updated_at = ?1
             WHERE id = ?2 OR LOWER(name) = LOWER(?2)",
            params![now, name],
        )?;
        Ok(rows > 0)
    }

    /// List entities of a given type.
    pub fn get_entities_by_type(&self, entity_type: &str) -> Result<Vec<DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, tracker_path, updated_at
             FROM entities WHERE entity_type = ?1
             ORDER BY name",
        )?;

        let rows = stmt.query_map(params![entity_type], |row| {
            let et: String = row.get(2)?;
            Ok(DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// Upsert an entity row that mirrors a CS account.
    ///
    /// Called from `upsert_account()` to keep the entity layer in sync.
    pub fn ensure_entity_for_account(&self, account: &DbAccount) -> Result<(), DbError> {
        let entity = DbEntity {
            id: account.id.clone(),
            name: account.name.clone(),
            entity_type: EntityType::Account,
            tracker_path: account.tracker_path.clone(),
            updated_at: account.updated_at.clone(),
        };
        self.upsert_entity(&entity)
    }


}
