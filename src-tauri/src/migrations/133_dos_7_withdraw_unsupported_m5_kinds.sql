-- DOS-7 L2 cycle-12 fix #2 + cycle-15 fix #2: withdraw any backfill
-- claim row whose subject_ref kind is not a supported SubjectRef
-- variant.
--
-- Originally targeted only m5 (linking_dismissals → owner_type
-- fall-through). Cycle-15 review caught that m6 (briefing_callouts),
-- m7 (nudge_dismissals), and m8 (triage_snoozes) all do
-- `upper(substr(entity_type, 1, 1)) || substr(entity_type, 2)` on
-- raw legacy entity_type without guarding — so an entity_type of
-- 'global' or 'multi' becomes a Global/Multi-shaped subject_ref,
-- bypassing the v1.4.0 spine restriction (ADR-0125) that
-- commit_claim enforces at the runtime write boundary.
--
-- This migration transitions all m1-m9 rows with unsupported kinds
-- to claim_state='withdrawn' + retraction_reason='unsupported_subject_kind'.
-- PRE-GATE + suppression + readers no longer match them; the
-- audit row survives. Operators can remediate by introducing a
-- SubjectRef variant and re-running rekey.

UPDATE intelligence_claims
SET claim_state = 'withdrawn',
    surfacing_state = 'dormant',
    retraction_reason = 'unsupported_subject_kind'
WHERE id GLOB 'm[1-9]-*'
  AND claim_state <> 'withdrawn'
  AND json_valid(subject_ref) = 1
  AND lower(json_extract(subject_ref, '$.kind')) NOT IN (
      'account', 'meeting', 'person', 'project', 'email'
  );
