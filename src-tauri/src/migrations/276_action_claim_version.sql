-- Add per-action claim_version so first-class Action subjects participate
-- in the same claim invalidation and repair machinery as Account, Project,
-- Person, Meeting, and Email subjects.

ALTER TABLE actions ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
