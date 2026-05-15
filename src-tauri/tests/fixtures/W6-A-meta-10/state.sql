-- W6-A-meta-10: list_open_loops_extract_commitments / contradiction / transcript vs email conflict.
-- Minimum substrate: two commitment claims from different sources plus an unresolved contradiction edge.
CREATE TABLE IF NOT EXISTS intelligence_claims (
    id TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    field_path TEXT,
    topic_key TEXT,
    text TEXT NOT NULL,
    actor TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    metadata_json TEXT,
    claim_state TEXT NOT NULL DEFAULT 'active',
    surfacing_state TEXT NOT NULL DEFAULT 'active',
    expires_at TEXT,
    trust_score REAL,
    temporal_scope TEXT NOT NULL DEFAULT 'state',
    sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active',
    verification_reason TEXT
);

CREATE TABLE IF NOT EXISTS claim_contradictions (
    id TEXT PRIMARY KEY,
    primary_claim_id TEXT NOT NULL,
    contradicting_claim_id TEXT NOT NULL,
    branch_kind TEXT NOT NULL,
    detected_at TEXT NOT NULL,
    reconciliation_kind TEXT,
    reconciliation_note TEXT,
    reconciled_at TEXT,
    winner_claim_id TEXT,
    merged_claim_id TEXT
);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, actor, data_source,
    source_ref, source_asof, observed_at, provenance_json, metadata_json,
    claim_state, surfacing_state, expires_at, trust_score, temporal_scope,
    sensitivity, verification_state, verification_reason
)
VALUES
    (
        'claim-meta-10-transcript-commitment',
        '{"kind":"account","id":"account-meta-10-example"}',
        'open_loop',
        'commitment.deliverable',
        'account-meta-10-example:commitment:draft',
        'The transcript says a draft will be sent by Friday.',
        'agent:fixture',
        'meeting_transcript',
        '{"source_id":"source-meta-10-transcript"}',
        '2026-05-14T18:00:00Z',
        '2026-05-14T18:05:00Z',
        '{"source_id":"source-meta-10-transcript","commitment":"send_draft_by_friday"}',
        '{"contradicts_claim_id":"claim-meta-10-email-no-commitment"}',
        'active',
        'active',
        '2026-05-22T23:59:59Z',
        0.63,
        'state',
        'internal',
        'contested',
        'unresolved_contradiction'
    ),
    (
        'claim-meta-10-email-no-commitment',
        '{"kind":"account","id":"account-meta-10-example"}',
        'commitment_correction',
        'commitment.deliverable',
        'account-meta-10-example:commitment:draft',
        'A follow-up email says there is no commitment to send the draft this week.',
        'agent:fixture',
        'email',
        '{"source_id":"source-meta-10-email"}',
        '2026-05-15T09:00:00Z',
        '2026-05-15T09:05:00Z',
        '{"source_id":"source-meta-10-email","commitment":"no_draft_this_week"}',
        '{"contradicts_claim_id":"claim-meta-10-transcript-commitment"}',
        'active',
        'active',
        NULL,
        0.7,
        'state',
        'internal',
        'contested',
        'unresolved_contradiction'
    );

INSERT INTO claim_contradictions (
    id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at,
    reconciliation_kind, reconciliation_note, reconciled_at, winner_claim_id, merged_claim_id
)
VALUES (
    'contradiction-meta-10-commitment',
    'claim-meta-10-transcript-commitment',
    'claim-meta-10-email-no-commitment',
    'commitment_conflict',
    '2026-05-15T10:00:00Z',
    NULL,
    NULL,
    NULL,
    NULL,
    NULL
);
