-- DOS: stakeholder-role soft-delete (dismissal tombstone)
--
-- Before this column, the remove-role UX (× on a role pill) ran a hard
-- DELETE on account_stakeholder_roles. With no record of the removal,
-- the next enrichment cycle's intel_queue would re-INSERT the role
-- with data_source='ai' because its existence check returned no row.
-- User intent "I do not want this role on this person" was lost every
-- time AI re-surfaced the role.
--
-- Soft-delete fix: when a user removes a role, we UPDATE
-- data_source='user' AND dismissed_at=now instead of deleting. The
-- row stays in the table as a tombstone; reads filter dismissed_at
-- IS NULL so the UI doesn't see it; the intel_queue existence check
-- returns data_source='user' and skips the re-insert. If the same
-- user later re-adds the role, add_stakeholder_role's ON CONFLICT
-- clause clears dismissed_at and reactivates the row.

ALTER TABLE account_stakeholder_roles ADD COLUMN dismissed_at TEXT;
