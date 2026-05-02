-- DOS-7 L2 cycle-3 fix: add per-entity claim_version to the emails table
-- so SubjectRef::Email participates in the same per-entity invalidation
-- machinery as Account/Meeting/Person/Project.
--
-- Why: cycle-2's email-dismissal shadow-write workaround routed claims
-- to an Account subject with a `email_id::item_text` text prefix hack.
-- That broke parity with the SQL backfill (m3 in migration 130) which
-- writes Email-subject rows. Cycle-3 audit showed the workaround does
-- not stop AI re-surfacing across the cutover boundary; the fix is to
-- model Email as a real claim subject.

ALTER TABLE emails ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
