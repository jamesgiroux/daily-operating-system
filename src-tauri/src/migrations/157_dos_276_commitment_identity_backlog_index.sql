-- DOS-276 W4-A cycle-3: replace the title-only backlog commitment guard.
--
-- The old partial unique index keyed backlog commitments by (title, account_id)
-- only. That collapsed real same-title commitments when due date or owner
-- differed. Runtime aliasing now uses the same structural identity tuple as
-- derive_commitment_id: entity + normalized title + due + owner.

DROP INDEX IF EXISTS idx_actions_backlog_commitment_title_account_unique;

CREATE UNIQUE INDEX IF NOT EXISTS idx_actions_backlog_commitment_identity_account_unique
    ON actions(account_id, lower(trim(title)), COALESCE(due_date, ''), COALESCE(owner_raw, ''))
    WHERE action_kind = 'commitment'
      AND status = 'backlog'
      AND account_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_actions_backlog_commitment_identity_project_unique
    ON actions(project_id, lower(trim(title)), COALESCE(due_date, ''), COALESCE(owner_raw, ''))
    WHERE action_kind = 'commitment'
      AND status = 'backlog'
      AND project_id IS NOT NULL;
