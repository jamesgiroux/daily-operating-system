-- DOS-7 L2 cycle-12 fix #2: withdraw mechanism-5 backfill rows whose
-- subject_ref kind is not a supported SubjectRef variant.
--
-- Migration 131 m5 backfill mapped owner_type 'meeting' → 'Meeting'
-- and 'email' → 'Email' but fell through every other owner_type as
-- the literal kind. linking_dismissals.owner_type can include
-- 'email_thread' (and historically other custom kinds), producing
-- claim rows with subject_ref like {"kind":"email_thread",...}.
-- The runtime SubjectRef enum has no EmailThread variant, so
-- bump_for_subject and is_suppressed_via_claims can't process those
-- rows.
--
-- This migration transitions any such rows to claim_state='withdrawn'
-- with retraction_reason='unsupported_subject_kind' so PRE-GATE +
-- suppression no longer match them and the audit row survives.
-- Operators can later remediate by introducing a SubjectRef variant
-- and re-running rekey.

UPDATE intelligence_claims
SET claim_state = 'withdrawn',
    surfacing_state = 'dormant',
    retraction_reason = 'unsupported_subject_kind'
WHERE id GLOB 'm5-*'
  AND json_valid(subject_ref) = 1
  AND lower(json_extract(subject_ref, '$.kind')) NOT IN (
      'account', 'meeting', 'person', 'project', 'email'
  );
