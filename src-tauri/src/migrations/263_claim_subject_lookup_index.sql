-- v1.4.4a W6: keep claim-backed surface reads bounded by subject.
--
-- Readers intentionally match SubjectRef semantics instead of exact JSON text:
-- key order and kind casing can differ across producers. This expression
-- index supports the json_extract lookup shape used by the surface readers.
CREATE INDEX IF NOT EXISTS idx_claims_subject_kind_id_lifecycle_created
ON intelligence_claims (
    lower(json_extract(subject_ref, '$.kind')),
    json_extract(subject_ref, '$.id'),
    claim_state,
    surfacing_state,
    claim_type,
    created_at DESC
)
WHERE json_valid(subject_ref) = 1;

CREATE INDEX IF NOT EXISTS idx_claims_subject_kind_id_lifecycle_untyped_created
ON intelligence_claims (
    lower(json_extract(subject_ref, '$.kind')),
    json_extract(subject_ref, '$.id'),
    claim_state,
    surfacing_state,
    created_at DESC
)
WHERE json_valid(subject_ref) = 1;
