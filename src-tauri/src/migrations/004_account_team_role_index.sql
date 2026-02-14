-- Sprint 21 code review: index for account_team ORDER BY role queries
CREATE INDEX IF NOT EXISTS idx_account_team_account_role
ON account_team(account_id, role);
