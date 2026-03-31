use super::*;
use crate::types::{AccountMilestone, AccountObjective};
use rusqlite::OptionalExtension;

#[derive(Debug, Clone)]
pub struct AutoCompletedMilestones {
    pub milestones: Vec<AccountMilestone>,
    pub objectives: Vec<AccountObjective>,
}

impl ActionDb {
    fn map_objective_row(row: &rusqlite::Row) -> rusqlite::Result<AccountObjective> {
        Ok(AccountObjective {
            id: row.get("id")?,
            account_id: row.get("account_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            status: row.get("status")?,
            target_date: row.get("target_date")?,
            completed_at: row.get("completed_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            source: row.get("source")?,
            sort_order: row.get("sort_order")?,
            milestones: Vec::new(),
            linked_actions: Vec::new(),
            linked_action_count: row
                .get::<_, Option<i32>>("linked_action_count")?
                .unwrap_or(0),
            completed_milestone_count: row
                .get::<_, Option<i32>>("completed_milestone_count")?
                .unwrap_or(0),
            total_milestone_count: row
                .get::<_, Option<i32>>("total_milestone_count")?
                .unwrap_or(0),
        })
    }

    fn map_milestone_row(row: &rusqlite::Row) -> rusqlite::Result<AccountMilestone> {
        Ok(AccountMilestone {
            id: row.get("id")?,
            objective_id: row.get("objective_id")?,
            account_id: row.get("account_id")?,
            title: row.get("title")?,
            status: row.get("status")?,
            target_date: row.get("target_date")?,
            completed_at: row.get("completed_at")?,
            auto_detect_signal: row.get("auto_detect_signal")?,
            completed_by: row.get("completed_by")?,
            completion_trigger: row.get("completion_trigger")?,
            sort_order: row.get("sort_order")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn get_account_objectives(
        &self,
        account_id: &str,
    ) -> Result<Vec<AccountObjective>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT o.*,
                    (SELECT COUNT(*) FROM action_objective_links aol WHERE aol.objective_id = o.id) AS linked_action_count,
                    (SELECT COUNT(*) FROM account_milestones m WHERE m.objective_id = o.id AND m.status = 'completed') AS completed_milestone_count,
                    (SELECT COUNT(*) FROM account_milestones m WHERE m.objective_id = o.id) AS total_milestone_count
             FROM account_objectives o
             WHERE o.account_id = ?1
             ORDER BY o.sort_order, o.created_at",
        )?;
        let objective_rows = stmt.query_map(params![account_id], Self::map_objective_row)?;
        let mut objectives = Vec::new();
        for row in objective_rows {
            objectives.push(row?);
        }

        for objective in &mut objectives {
            objective.milestones = self.get_objective_milestones(&objective.id)?;
            objective.linked_actions = self.get_linked_actions_for_objective(&objective.id)?;
        }

        Ok(objectives)
    }

    pub fn get_objective(&self, objective_id: &str) -> Result<Option<AccountObjective>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT o.*,
                    (SELECT COUNT(*) FROM action_objective_links aol WHERE aol.objective_id = o.id) AS linked_action_count,
                    (SELECT COUNT(*) FROM account_milestones m WHERE m.objective_id = o.id AND m.status = 'completed') AS completed_milestone_count,
                    (SELECT COUNT(*) FROM account_milestones m WHERE m.objective_id = o.id) AS total_milestone_count
             FROM account_objectives o
             WHERE o.id = ?1",
        )?;
        let mut objective = stmt
            .query_row(params![objective_id], Self::map_objective_row)
            .optional()?;

        if let Some(ref mut item) = objective {
            item.milestones = self.get_objective_milestones(&item.id)?;
            item.linked_actions = self.get_linked_actions_for_objective(&item.id)?;
        }

        Ok(objective)
    }

    pub fn create_objective(
        &self,
        account_id: &str,
        title: &str,
        description: Option<&str>,
        target_date: Option<&str>,
        source: &str,
    ) -> Result<AccountObjective, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let sort_order: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM account_objectives WHERE account_id = ?1",
            params![account_id],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO account_objectives (
                id, account_id, title, description, status, target_date, source, sort_order, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?8, ?8)",
            params![id, account_id, title, description, target_date, source, sort_order, now],
        )?;
        self.get_objective(&id)?.ok_or_else(|| {
            DbError::Migration("Objective created but could not be reloaded".to_string())
        })
    }

    pub fn update_objective(
        &self,
        objective_id: &str,
        title: Option<&str>,
        description: Option<&str>,
        target_date: Option<&str>,
        sort_order: Option<i32>,
        status: Option<&str>,
    ) -> Result<AccountObjective, DbError> {
        let existing = self
            .get_objective(objective_id)?
            .ok_or_else(|| DbError::Migration(format!("Objective not found: {objective_id}")))?;
        let next_title = title.unwrap_or(&existing.title);
        let next_description = if description.is_some() {
            description.map(str::to_string)
        } else {
            existing.description.clone()
        };
        let next_target_date = if target_date.is_some() {
            target_date
                .map(str::to_string)
                .filter(|v| !v.trim().is_empty())
        } else {
            existing.target_date.clone()
        };
        let next_sort_order = sort_order.unwrap_or(existing.sort_order);
        let next_status = status.unwrap_or(&existing.status);
        self.conn.execute(
            "UPDATE account_objectives
             SET title = ?1,
                 description = ?2,
                 target_date = ?3,
                 sort_order = ?4,
                 status = ?5,
                 updated_at = ?6
             WHERE id = ?7",
            params![
                next_title,
                next_description,
                next_target_date,
                next_sort_order,
                next_status,
                Utc::now().to_rfc3339(),
                objective_id
            ],
        )?;
        self.get_objective(objective_id)?.ok_or_else(|| {
            DbError::Migration("Objective updated but could not be reloaded".to_string())
        })
    }

    pub fn complete_objective(&self, objective_id: &str) -> Result<AccountObjective, DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE account_objectives
             SET status = 'completed', completed_at = ?2, updated_at = ?2
             WHERE id = ?1",
            params![objective_id, now],
        )?;
        self.get_objective(objective_id)?.ok_or_else(|| {
            DbError::Migration("Objective completed but could not be reloaded".to_string())
        })
    }

    pub fn abandon_objective(&self, objective_id: &str) -> Result<AccountObjective, DbError> {
        self.conn.execute(
            "UPDATE account_objectives
             SET status = 'abandoned', updated_at = ?2
             WHERE id = ?1",
            params![objective_id, Utc::now().to_rfc3339()],
        )?;
        self.get_objective(objective_id)?.ok_or_else(|| {
            DbError::Migration("Objective abandoned but could not be reloaded".to_string())
        })
    }

    pub fn delete_objective(&self, objective_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM account_objectives WHERE id = ?1",
            params![objective_id],
        )?;
        Ok(())
    }

    pub fn get_objective_milestones(
        &self,
        objective_id: &str,
    ) -> Result<Vec<AccountMilestone>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT *
             FROM account_milestones
             WHERE objective_id = ?1
             ORDER BY sort_order, created_at",
        )?;
        let rows = stmt.query_map(params![objective_id], Self::map_milestone_row)?;
        let mut milestones = Vec::new();
        for row in rows {
            milestones.push(row?);
        }
        Ok(milestones)
    }

    pub fn get_milestone(&self, milestone_id: &str) -> Result<Option<AccountMilestone>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT * FROM account_milestones WHERE id = ?1",
                params![milestone_id],
                Self::map_milestone_row,
            )
            .optional()?)
    }

    pub fn create_milestone(
        &self,
        objective_id: &str,
        title: &str,
        target_date: Option<&str>,
        auto_detect_signal: Option<&str>,
    ) -> Result<AccountMilestone, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let account_id: String = self.conn.query_row(
            "SELECT account_id FROM account_objectives WHERE id = ?1",
            params![objective_id],
            |row| row.get(0),
        )?;
        let sort_order: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM account_milestones WHERE objective_id = ?1",
            params![objective_id],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO account_milestones (
                id, objective_id, account_id, title, status, target_date, auto_detect_signal, sort_order, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?8)",
            params![id, objective_id, account_id, title, target_date, auto_detect_signal, sort_order, now],
        )?;
        self.get_milestone(&id)?.ok_or_else(|| {
            DbError::Migration("Milestone created but could not be reloaded".to_string())
        })
    }

    pub fn update_milestone(
        &self,
        milestone_id: &str,
        title: Option<&str>,
        target_date: Option<&str>,
        auto_detect_signal: Option<&str>,
        sort_order: Option<i32>,
        status: Option<&str>,
    ) -> Result<AccountMilestone, DbError> {
        let existing = self
            .get_milestone(milestone_id)?
            .ok_or_else(|| DbError::Migration(format!("Milestone not found: {milestone_id}")))?;
        let next_title = title.unwrap_or(&existing.title);
        let next_target_date = if target_date.is_some() {
            target_date
                .map(str::to_string)
                .filter(|v| !v.trim().is_empty())
        } else {
            existing.target_date.clone()
        };
        let next_auto_detect = if auto_detect_signal.is_some() {
            auto_detect_signal
                .map(str::to_string)
                .filter(|v| !v.trim().is_empty())
        } else {
            existing.auto_detect_signal.clone()
        };
        let next_sort_order = sort_order.unwrap_or(existing.sort_order);
        let next_status = status.unwrap_or(&existing.status);
        self.conn.execute(
            "UPDATE account_milestones
             SET title = ?1,
                 target_date = ?2,
                 auto_detect_signal = ?3,
                 sort_order = ?4,
                 status = ?5,
                 updated_at = ?6
             WHERE id = ?7",
            params![
                next_title,
                next_target_date,
                next_auto_detect,
                next_sort_order,
                next_status,
                Utc::now().to_rfc3339(),
                milestone_id
            ],
        )?;
        self.get_milestone(milestone_id)?.ok_or_else(|| {
            DbError::Migration("Milestone updated but could not be reloaded".to_string())
        })
    }

    pub fn complete_milestone(
        &self,
        milestone_id: &str,
    ) -> Result<(AccountMilestone, Option<AccountObjective>), DbError> {
        self.complete_milestone_with_metadata(milestone_id, None, None)
    }

    pub fn complete_milestone_with_metadata(
        &self,
        milestone_id: &str,
        completed_by: Option<&str>,
        completion_trigger: Option<&str>,
    ) -> Result<(AccountMilestone, Option<AccountObjective>), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE account_milestones
             SET status = 'completed',
                 completed_at = ?2,
                 completed_by = COALESCE(?3, completed_by),
                 completion_trigger = COALESCE(?4, completion_trigger),
                 updated_at = ?2
             WHERE id = ?1",
            params![milestone_id, now, completed_by, completion_trigger],
        )?;
        let milestone = self.get_milestone(milestone_id)?.ok_or_else(|| {
            DbError::Migration("Milestone not found after completion".to_string())
        })?;
        let objective = self.auto_complete_objective_if_ready(&milestone.objective_id)?;
        Ok((milestone, objective))
    }

    pub fn skip_milestone(
        &self,
        milestone_id: &str,
    ) -> Result<(AccountMilestone, Option<AccountObjective>), DbError> {
        self.conn.execute(
            "UPDATE account_milestones
             SET status = 'skipped', updated_at = ?2
             WHERE id = ?1",
            params![milestone_id, Utc::now().to_rfc3339()],
        )?;
        let milestone = self
            .get_milestone(milestone_id)?
            .ok_or_else(|| DbError::Migration("Milestone not found after skip".to_string()))?;
        let objective = self.auto_complete_objective_if_ready(&milestone.objective_id)?;
        Ok((milestone, objective))
    }

    pub fn delete_milestone(&self, milestone_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM account_milestones WHERE id = ?1",
            params![milestone_id],
        )?;
        Ok(())
    }

    fn auto_complete_objective_if_ready(
        &self,
        objective_id: &str,
    ) -> Result<Option<AccountObjective>, DbError> {
        let pending_count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM account_milestones
             WHERE objective_id = ?1 AND status = 'pending'",
            params![objective_id],
            |row| row.get(0),
        )?;
        let total_count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM account_milestones WHERE objective_id = ?1",
            params![objective_id],
            |row| row.get(0),
        )?;
        if total_count > 0 && pending_count == 0 {
            return self.complete_objective(objective_id).map(Some);
        }
        Ok(None)
    }

    pub fn link_action_to_objective(
        &self,
        action_id: &str,
        objective_id: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO action_objective_links (action_id, objective_id)
             VALUES (?1, ?2)",
            params![action_id, objective_id],
        )?;
        Ok(())
    }

    pub fn unlink_action_from_objective(
        &self,
        action_id: &str,
        objective_id: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM action_objective_links WHERE action_id = ?1 AND objective_id = ?2",
            params![action_id, objective_id],
        )?;
        Ok(())
    }

    pub fn get_linked_actions_for_objective(
        &self,
        objective_id: &str,
    ) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.title, a.priority, a.status, a.created_at, a.due_date, a.completed_at,
                    a.account_id, a.project_id, a.source_type, a.source_id, a.source_label,
                    a.context, a.waiting_on, a.updated_at, a.person_id, acc.name AS account_name
             FROM action_objective_links aol
             JOIN actions a ON a.id = aol.action_id
             LEFT JOIN accounts acc ON a.account_id = acc.id
             WHERE aol.objective_id = ?1
               AND a.status IN ('suggested', 'pending')
             ORDER BY a.priority, a.due_date",
        )?;
        let rows = stmt.query_map(params![objective_id], Self::map_action_row)?;
        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    pub fn reorder_objectives(
        &self,
        account_id: &str,
        ordered_ids: &[String],
    ) -> Result<(), DbError> {
        for (idx, objective_id) in ordered_ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE account_objectives
                 SET sort_order = ?1, updated_at = ?3
                 WHERE id = ?2 AND account_id = ?4",
                params![
                    idx as i32,
                    objective_id,
                    Utc::now().to_rfc3339(),
                    account_id
                ],
            )?;
        }
        Ok(())
    }

    pub fn reorder_milestones(
        &self,
        objective_id: &str,
        ordered_ids: &[String],
    ) -> Result<(), DbError> {
        for (idx, milestone_id) in ordered_ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE account_milestones
                 SET sort_order = ?1, updated_at = ?3
                 WHERE id = ?2 AND objective_id = ?4",
                params![
                    idx as i32,
                    milestone_id,
                    Utc::now().to_rfc3339(),
                    objective_id
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_success_plan_signals_json(
        &self,
        account_id: &str,
    ) -> Result<Option<String>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT success_plan_signals_json
                 FROM entity_assessment
                 WHERE entity_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .optional()
            .map(|value| value.flatten())?)
    }

    pub fn get_unconsumed_commitments(
        &self,
        account_id: &str,
    ) -> Result<
        Vec<(
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
        )>,
        DbError,
    > {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, owner, target_date, confidence, source
             FROM captured_commitments
             WHERE account_id = ?1 AND consumed = 0
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?;
        let mut commitments = Vec::new();
        for row in rows {
            commitments.push(row?);
        }
        Ok(commitments)
    }

    pub fn mark_commitments_consumed(&self, commitment_ids: &[String]) -> Result<(), DbError> {
        for commitment_id in commitment_ids {
            self.conn.execute(
                "UPDATE captured_commitments SET consumed = 1 WHERE id = ?1",
                params![commitment_id],
            )?;
        }
        Ok(())
    }

    /// Read assessment fields for objective suggestion fallback.
    /// Returns (success_metrics_json, open_commitments_json, risks_json).
    pub fn get_assessment_fallback_fields(
        &self,
        account_id: &str,
    ) -> Result<(Option<String>, Option<String>, Option<String>), DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT success_metrics, open_commitments, risks_json
             FROM entity_assessment
             WHERE entity_id = ?1",
        )?;
        let result = stmt.query_row(params![account_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        });
        match result {
            Ok(fields) => Ok(fields),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok((None, None, None)),
            Err(e) => Err(DbError::from(e)),
        }
    }

    pub fn complete_milestones_for_account_event(
        &self,
        account_id: &str,
        event_type: &str,
    ) -> Result<AutoCompletedMilestones, DbError> {
        self.complete_milestones_for_completion_trigger(
            account_id,
            event_type,
            Some("account_event"),
        )
    }

    /// Get recently auto-completed milestones (I628 AC5) for timeline display.
    /// Returns milestones with `completed_by IS NOT NULL` completed within the last N days.
    pub fn get_auto_completed_milestones(
        &self,
        account_id: &str,
        days: i32,
    ) -> Result<Vec<AccountMilestone>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, objective_id, account_id, title, status,
                    target_date, completed_at, auto_detect_signal,
                    completed_by, completion_trigger, sort_order,
                    created_at, updated_at
             FROM account_milestones
             WHERE account_id = ?1
               AND status = 'completed'
               AND completed_by IS NOT NULL
               AND completed_at >= datetime('now', ?2)
             ORDER BY completed_at DESC",
        )?;
        let days_param = format!("-{} days", days);
        let rows = stmt.query_map(params![account_id, days_param], Self::map_milestone_row)?;
        let mut milestones = Vec::new();
        for row in rows {
            milestones.push(row?);
        }
        Ok(milestones)
    }

    /// Find pending milestones that match a completion trigger, returning (id, title) pairs.
    /// Used for sub-0.8 confidence notation (I628 AC3) — notes the match without completing.
    pub fn find_milestones_for_trigger(
        &self,
        account_id: &str,
        completion_trigger: &str,
    ) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title
             FROM account_milestones
             WHERE account_id = ?1
               AND status = 'pending'
               AND (
                   auto_detect_signal = ?2
                   OR completion_trigger = ?2
               )
             ORDER BY sort_order, created_at",
        )?;
        let rows = stmt.query_map(params![account_id, completion_trigger], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn complete_milestones_for_completion_trigger(
        &self,
        account_id: &str,
        completion_trigger: &str,
        completed_by: Option<&str>,
    ) -> Result<AutoCompletedMilestones, DbError> {
        let milestone_ids: Vec<String> = {
            let mut stmt = self.conn.prepare(
                "SELECT id
                 FROM account_milestones
                 WHERE account_id = ?1
                   AND status = 'pending'
                   AND (
                       auto_detect_signal = ?2
                       OR completion_trigger = ?2
                   )
                 ORDER BY sort_order, created_at",
            )?;
            let rows = stmt.query_map(params![account_id, completion_trigger], |row| row.get(0))?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row?);
            }
            ids
        };

        let mut completed_milestones = Vec::new();
        let mut completed_objectives = Vec::new();
        for milestone_id in milestone_ids {
            let (milestone, objective) = self.complete_milestone_with_metadata(
                &milestone_id,
                completed_by,
                Some(completion_trigger),
            )?;
            completed_milestones.push(milestone);
            if let Some(objective) = objective {
                completed_objectives.push(objective);
            }
        }

        Ok(AutoCompletedMilestones {
            milestones: completed_milestones,
            objectives: completed_objectives,
        })
    }
}
