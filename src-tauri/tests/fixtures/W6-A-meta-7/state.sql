-- W6-A-meta-7: detect_risk_shift / contradiction / contradicting risk signals.
-- Minimum substrate: two fresh risk claims plus a contradiction edge.
CREATE TABLE IF NOT EXISTS entities (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    tracker_path TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (id, entity_type)
);

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

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES ('account-meta-7-example', 'Account Meta 7 Example', 'account', NULL, '2026-05-15T10:00:00Z');

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, actor, data_source,
    source_ref, source_asof, observed_at, provenance_json, metadata_json,
    trust_score, temporal_scope, sensitivity, verification_state, verification_reason
)
VALUES
    (
        'claim-meta-7-support-risk',
        '{"kind":"account","id":"account-meta-7-example"}',
        'risk_signal',
        'risk.support',
        'account-meta-7-example:risk:current',
        'A support note says the account escalated implementation risk today.',
        'agent:fixture',
        'support',
        '{"source_id":"source-meta-7-support"}',
        '2026-05-15T09:00:00Z',
        '2026-05-15T09:05:00Z',
        '{"source_id":"source-meta-7-support","risk_direction":"increasing"}',
        '{"contradicts_claim_id":"claim-meta-7-email-stable"}',
        0.68,
        'state',
        'internal',
        'contested',
        'unresolved_contradiction'
    ),
    (
        'claim-meta-7-email-stable',
        '{"kind":"account","id":"account-meta-7-example"}',
        'risk_signal',
        'risk.sponsor',
        'account-meta-7-example:risk:current',
        'A sponsor email says the account is stable and has no active implementation concern.',
        'agent:fixture',
        'email',
        '{"source_id":"source-meta-7-email"}',
        '2026-05-15T09:30:00Z',
        '2026-05-15T09:35:00Z',
        '{"source_id":"source-meta-7-email","risk_direction":"stable"}',
        '{"contradicts_claim_id":"claim-meta-7-support-risk"}',
        0.66,
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
    'contradiction-meta-7-risk',
    'claim-meta-7-support-risk',
    'claim-meta-7-email-stable',
    'risk_signal_conflict',
    '2026-05-15T10:00:00Z',
    NULL,
    NULL,
    NULL,
    NULL,
    NULL
);
