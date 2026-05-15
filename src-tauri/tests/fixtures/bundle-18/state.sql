CREATE TABLE bundle18_scenarios (
    scenario_id TEXT PRIMARY KEY,
    substrate_table TEXT NOT NULL,
    expected_state TEXT NOT NULL,
    notes TEXT NOT NULL
);

CREATE TABLE meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL DEFAULT 'external',
    start_time TEXT NOT NULL,
    end_time TEXT,
    attendees TEXT,
    created_at TEXT NOT NULL,
    calendar_event_id TEXT,
    description TEXT
);

CREATE TABLE people (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE meeting_attendees (
    meeting_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    PRIMARY KEY (meeting_id, person_id)
);

CREATE TABLE entities (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    tracker_path TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (id, entity_type)
);

CREATE TABLE meeting_entities (
    meeting_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    confidence REAL DEFAULT 0.95,
    is_primary INTEGER DEFAULT 1,
    PRIMARY KEY (meeting_id, entity_id, entity_type)
);

CREATE TABLE intelligence_claims (
    id TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    field_path TEXT,
    topic_key TEXT,
    text TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    item_hash TEXT,
    actor TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    metadata_json TEXT,
    claim_state TEXT NOT NULL DEFAULT 'active',
    surfacing_state TEXT NOT NULL DEFAULT 'active',
    demotion_reason TEXT,
    reactivated_at TEXT,
    retraction_reason TEXT,
    expires_at TEXT,
    superseded_by TEXT,
    trust_score REAL,
    trust_computed_at TEXT,
    trust_version INTEGER,
    thread_id TEXT,
    temporal_scope TEXT NOT NULL DEFAULT 'state',
    sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active',
    verification_reason TEXT,
    needs_user_decision_at TEXT,
    claim_version INTEGER NOT NULL DEFAULT 1,
    canonical_status TEXT NOT NULL DEFAULT 'live',
    non_semantic_mergeable BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE claim_feedback (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL,
    feedback_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    actor_id TEXT,
    payload_json TEXT,
    submitted_at TEXT NOT NULL,
    applied_at TEXT
);

CREATE TABLE concurrent_enrichment_attempts (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    claim_id TEXT NOT NULL,
    attempted_generation INTEGER NOT NULL,
    current_generation INTEGER NOT NULL,
    attempted_text TEXT NOT NULL,
    status TEXT NOT NULL,
    rejection_reason TEXT,
    attempted_at TEXT NOT NULL
);

CREATE TABLE generated_output_rejections (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    claim_id TEXT NOT NULL,
    attempted_generation INTEGER NOT NULL,
    current_generation INTEGER NOT NULL,
    rejection_reason TEXT NOT NULL,
    rejected_at TEXT NOT NULL,
    attempted_text TEXT NOT NULL
);

CREATE TABLE version_events (
    event_seq INTEGER PRIMARY KEY AUTOINCREMENT,
    cursor TEXT NOT NULL UNIQUE,
    event_kind TEXT NOT NULL,
    claim_id TEXT,
    previous_version INTEGER,
    current_version INTEGER NOT NULL,
    reason TEXT,
    scope_redacted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    actor_kind TEXT NOT NULL
);

CREATE TABLE user_agenda_items (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    meeting_id TEXT NOT NULL,
    body TEXT NOT NULL,
    owner TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    preserved_after_refresh INTEGER NOT NULL
);

CREATE TABLE user_notes (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    body TEXT NOT NULL,
    owner TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    preserved_after_refresh INTEGER NOT NULL
);

CREATE TABLE user_dismissals (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    surface TEXT NOT NULL,
    subject_ref TEXT NOT NULL,
    dismissed_key TEXT NOT NULL,
    owner TEXT NOT NULL,
    dismissed_at TEXT NOT NULL,
    preserved_after_refresh INTEGER NOT NULL
);

CREATE TABLE daily_readiness_child_reads (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    parent_ability TEXT NOT NULL,
    child_ability TEXT NOT NULL,
    composition_id TEXT NOT NULL,
    child_status TEXT NOT NULL,
    parent_render_state TEXT NOT NULL,
    warning_enum TEXT NOT NULL,
    reason TEXT NOT NULL
);

CREATE TABLE offline_source_snapshots (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_asof TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    offline_mode INTEGER NOT NULL,
    stale_age_hours INTEGER NOT NULL,
    warning_class TEXT NOT NULL
);

CREATE TABLE signal_events (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    invalidates_claim_id TEXT NOT NULL,
    emitted_at TEXT NOT NULL
);

CREATE TABLE invalidation_jobs (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    status TEXT NOT NULL,
    coalescing_key TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    ability_id TEXT NOT NULL,
    first_signal_id TEXT NOT NULL,
    latest_signal_id TEXT NOT NULL,
    raw_signal_count INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    stale_marker_json TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE refresh_jobs (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    subject_ref TEXT NOT NULL,
    refresh_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    created_claim_id TEXT,
    coalesced_into TEXT,
    completed_at TEXT
);

CREATE TABLE commitments (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    subject_ref TEXT NOT NULL,
    source_refresh_job_id TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    text TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE eval_replay_runs (
    id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    run_label TEXT NOT NULL,
    clock TEXT NOT NULL,
    seed INTEGER NOT NULL,
    provider_replay_key TEXT NOT NULL,
    canonical_output_json TEXT NOT NULL
);

INSERT INTO bundle18_scenarios (scenario_id, substrate_table, expected_state, notes)
VALUES
    ('user-correction-vs-concurrent-enrichment', 'intelligence_claims', 'user_correction_preserved', 'A current-thread sequential enrichment attempt is rejected after a user correction.'),
    ('older-generation-retry-rejected', 'intelligence_claims', 'generation_unchanged', 'Older generated output retry is rejected against the claim_version watermark.'),
    ('refresh-preserves-agenda-notes-dismissals', 'user_agenda_items,user_notes,user_dismissals', 'all_user_rows_preserved', 'Agenda, notes, and dismissals are distinct user-layer rows and all survive refresh.'),
    ('child-failure-parent-partial-warning', 'daily_readiness_child_reads', 'parent_degraded_with_optional_warning', 'Daily readiness parent marks optional child read failure.'),
    ('offline-mode-stale-age-field', 'offline_source_snapshots', 'offline_stale_warning_has_age', 'Offline stale state exposes stale_age_hours.'),
    ('signal-coalescing-preserves-invalidations', 'invalidation_jobs', 'invalidations_preserved', 'Two signals coalesce while retaining the invalidated claim id.'),
    ('duplicate-refresh-single-claim-row', 'refresh_jobs', 'one_claim_one_commitment', 'Duplicate refresh jobs produce one claim row and one commitment row.'),
    ('eval-determinism-canonical-equality', 'eval_replay_runs', 'canonical_json_equal', 'Two replay runs share clock, seed, provider replay, and canonical output.');

INSERT INTO meetings (id, title, meeting_type, start_time, end_time, attendees, created_at, calendar_event_id, description)
VALUES (
    'meeting-b18-sync-refresh',
    'Sync Refresh Validation',
    'external',
    '2026-05-15T16:00:00Z',
    '2026-05-15T16:30:00Z',
    '["person@example.com"]',
    '2026-05-15T08:00:00Z',
    'cal-b18-sync-refresh',
    'Synthetic meeting for sync, refresh, concurrency, and partial-failure validation.'
);

INSERT INTO people (id, email, name)
VALUES ('person-b18-example', 'person@example.com', 'Person Example');

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES ('meeting-b18-sync-refresh', 'person-b18-example');

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES ('account-b18-example', 'Account Bundle 18 Example', 'account', NULL, '2026-05-15T08:00:00Z');

INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
VALUES ('meeting-b18-sync-refresh', 'account-b18-example', 'account', 0.95, 1);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash,
    actor, data_source, source_ref, source_asof, observed_at, created_at, provenance_json,
    metadata_json, claim_state, surfacing_state, demotion_reason, reactivated_at,
    retraction_reason, expires_at, superseded_by, trust_score, trust_computed_at,
    trust_version, thread_id, temporal_scope, sensitivity, verification_state,
    verification_reason, needs_user_decision_at, claim_version, canonical_status,
    non_semantic_mergeable
)
VALUES
    (
        'claim-b18-user-correction-current',
        '{"kind":"account","id":"account-b18-example"}',
        'current_state',
        'implementation.summary',
        'account-b18-example:implementation-summary',
        'User corrected the implementation summary to say the account wants a written agenda before renewal planning.',
        'b18:user-correction:account-b18-example:implementation-summary',
        'hash-b18-user-correction-current',
        'user',
        'user_correction',
        '{"source_id":"source-b18-user-correction","provider":"user"}',
        '2026-05-15T10:15:00Z',
        '2026-05-15T10:15:00Z',
        '2026-05-15T10:15:00Z',
        '{"scenario_id":"user-correction-vs-concurrent-enrichment"}',
        '{"scenario_id":"user-correction-vs-concurrent-enrichment","user_authored":true,"concurrent_enrichment_attempt_id":"enrichment-b18-user-correction-stale"}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.98,
        '2026-05-15T10:16:00Z',
        1,
        NULL,
        'state',
        'user_only',
        'active',
        NULL,
        NULL,
        4,
        'live',
        FALSE
    ),
    (
        'claim-b18-generation-current',
        '{"kind":"account","id":"account-b18-example"}',
        'current_state',
        'risk.status',
        'account-b18-example:risk-status',
        'Generation 7 says the account risk is watch because the implementation owner changed.',
        'b18:generation:account-b18-example:risk-status',
        'hash-b18-generation-current',
        'agent:fixture',
        'glean',
        '{"source_id":"source-b18-generation-current","provider":"glean"}',
        '2026-05-15T11:00:00Z',
        '2026-05-15T11:01:00Z',
        '2026-05-15T11:01:00Z',
        '{"scenario_id":"older-generation-retry-rejected"}',
        '{"scenario_id":"older-generation-retry-rejected","generation":7,"retry_rejected_generation":6}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.88,
        '2026-05-15T11:02:00Z',
        1,
        NULL,
        'state',
        'internal',
        'active',
        NULL,
        NULL,
        7,
        'live',
        FALSE
    ),
    (
        'claim-b18-offline-stale-source',
        '{"kind":"account","id":"account-b18-example"}',
        'current_state',
        'offline.snapshot',
        'account-b18-example:offline-snapshot',
        'Offline mode is using the most recent cached account snapshot.',
        'b18:offline:account-b18-example:snapshot',
        'hash-b18-offline-stale-source',
        'agent:fixture',
        'glean',
        '{"source_id":"source-b18-offline-stale","provider":"glean","mode":"offline"}',
        '2026-05-13T11:18:00Z',
        '2026-05-15T12:18:00Z',
        '2026-05-15T12:18:00Z',
        '{"scenario_id":"offline-mode-stale-age-field"}',
        '{"scenario_id":"offline-mode-stale-age-field","offline_mode":true,"stale_age_hours":49}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.52,
        '2026-05-15T12:18:00Z',
        1,
        NULL,
        'state',
        'internal',
        'contested',
        'offline_stale',
        NULL,
        2,
        'live',
        FALSE
    ),
    (
        'claim-b18-coalesced-invalidated',
        '{"kind":"account","id":"account-b18-example"}',
        'current_state',
        'briefing.freshness',
        'account-b18-example:briefing-freshness',
        'The coalesced refresh invalidated the stale briefing and recomputed the current account context.',
        'b18:coalesced:account-b18-example:briefing-freshness',
        'hash-b18-coalesced-invalidated',
        'agent:fixture',
        'signals',
        '{"source_id":"source-b18-coalesced-invalidated","provider":"signals"}',
        '2026-05-15T12:12:00Z',
        '2026-05-15T12:13:00Z',
        '2026-05-15T12:13:00Z',
        '{"scenario_id":"signal-coalescing-preserves-invalidations"}',
        '{"scenario_id":"signal-coalescing-preserves-invalidations","invalidation_status":"refreshed_after_coalescing","raw_signal_count":2}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.9,
        '2026-05-15T12:14:00Z',
        1,
        NULL,
        'state',
        'internal',
        'active',
        NULL,
        NULL,
        5,
        'live',
        FALSE
    ),
    (
        'claim-b18-duplicate-refresh-canonical',
        '{"kind":"account","id":"account-b18-example"}',
        'current_state',
        'refresh.risk',
        'account-b18-example:duplicate-refresh-risk',
        'Duplicate refresh requests resolve to one canonical account risk claim.',
        'b18:duplicate-refresh:account-b18-example:risk',
        'hash-b18-duplicate-refresh-canonical',
        'agent:fixture',
        'refresh',
        '{"source_id":"source-b18-duplicate-refresh","provider":"refresh"}',
        '2026-05-15T12:16:00Z',
        '2026-05-15T12:17:00Z',
        '2026-05-15T12:17:00Z',
        '{"scenario_id":"duplicate-refresh-single-claim-row"}',
        '{"scenario_id":"duplicate-refresh-single-claim-row","refresh_job_ids":["refresh-b18-duplicate-a","refresh-b18-duplicate-b"],"deduplicated":true}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.86,
        '2026-05-15T12:18:00Z',
        1,
        NULL,
        'state',
        'internal',
        'active',
        NULL,
        NULL,
        1,
        'live',
        FALSE
    );

INSERT INTO claim_feedback (id, claim_id, feedback_type, actor, actor_id, payload_json, submitted_at, applied_at)
VALUES (
    'feedback-b18-user-correction',
    'claim-b18-user-correction-current',
    'correct',
    'user',
    NULL,
    '{"correction_text":"User corrected the implementation summary to say the account wants a written agenda before renewal planning.","render_policy":"UserAuthoredWins"}',
    '2026-05-15T10:15:00Z',
    '2026-05-15T10:15:00Z'
);

INSERT INTO concurrent_enrichment_attempts (
    id, scenario_id, claim_id, attempted_generation, current_generation,
    attempted_text, status, rejection_reason, attempted_at
)
VALUES (
    'enrichment-b18-user-correction-stale',
    'user-correction-vs-concurrent-enrichment',
    'claim-b18-user-correction-current',
    3,
    4,
    'Generated enrichment tries to restore the old implementation summary.',
    'pending',
    NULL,
    '2026-05-15T10:16:00Z'
);

INSERT INTO generated_output_rejections (
    id, scenario_id, claim_id, attempted_generation, current_generation,
    rejection_reason, rejected_at, attempted_text
)
VALUES (
    'rejection-b18-older-generation',
    'older-generation-retry-rejected',
    'claim-b18-generation-current',
    6,
    7,
    'stale_generation_rejected',
    '2026-05-15T11:03:00Z',
    'Generation 6 retry tried to restore the previous healthy risk state.'
);

INSERT INTO version_events (
    cursor, event_kind, claim_id, previous_version, current_version, reason,
    scope_redacted, created_at, actor_kind
)
VALUES (
    '11111111-aaaa-4818-88ab-cccccccccccc',
    'claim.write_rejected',
    'claim-b18-generation-current',
    6,
    7,
    'stale_generation_rejected',
    0,
    '2026-05-15T11:03:00Z',
    'agent'
);

INSERT INTO user_agenda_items (
    id, scenario_id, meeting_id, body, owner, updated_at, preserved_after_refresh
)
VALUES (
    'agenda-b18-preserved',
    'refresh-preserves-agenda-notes-dismissals',
    'meeting-b18-sync-refresh',
    'Start with the user-authored renewal agenda before generated risk topics.',
    'user',
    '2026-05-15T09:00:00Z',
    1
);

INSERT INTO user_notes (
    id, scenario_id, owner_type, owner_id, body, owner, updated_at, preserved_after_refresh
)
VALUES (
    'note-b18-preserved',
    'refresh-preserves-agenda-notes-dismissals',
    'meeting',
    'meeting-b18-sync-refresh',
    'User note: confirm whether the account wants the rollout sequence in writing.',
    'user',
    '2026-05-15T09:05:00Z',
    1
);

INSERT INTO user_dismissals (
    id, scenario_id, surface, subject_ref, dismissed_key, owner, dismissed_at,
    preserved_after_refresh
)
VALUES (
    'dismissal-b18-preserved',
    'refresh-preserves-agenda-notes-dismissals',
    'prepare_meeting',
    '{"kind":"account","id":"account-b18-example"}',
    'topic:old-implementation-summary',
    'user',
    '2026-05-15T09:10:00Z',
    1
);

INSERT INTO daily_readiness_child_reads (
    id, scenario_id, parent_ability, child_ability, composition_id, child_status,
    parent_render_state, warning_enum, reason
)
VALUES (
    'child-read-b18-optional-failed',
    'child-failure-parent-partial-warning',
    'get_daily_readiness',
    'get_entity_context',
    'daily-readiness:child:get-entity-context:account-b18-example',
    'failed_optional',
    'degraded',
    'OptionalComposedReadFailed',
    'Synthetic optional get_entity_context read failed during daily readiness composition.'
);

INSERT INTO offline_source_snapshots (
    id, scenario_id, source_id, source_asof, observed_at, offline_mode,
    stale_age_hours, warning_class
)
VALUES (
    'offline-b18-stale-source',
    'offline-mode-stale-age-field',
    'source-b18-offline-stale',
    '2026-05-13T11:18:00Z',
    '2026-05-15T12:18:00Z',
    1,
    49,
    'OfflineStale'
);

INSERT INTO signal_events (
    id, scenario_id, entity_id, signal_type, invalidates_claim_id, emitted_at
)
VALUES
    (
        'signal-b18-coalesce-a',
        'signal-coalescing-preserves-invalidations',
        'account-b18-example',
        'claim_trust_changed',
        'claim-b18-coalesced-invalidated',
        '2026-05-15T12:12:00Z'
    ),
    (
        'signal-b18-coalesce-b',
        'signal-coalescing-preserves-invalidations',
        'account-b18-example',
        'source_changed',
        'claim-b18-coalesced-invalidated',
        '2026-05-15T12:12:00Z'
    );

INSERT INTO invalidation_jobs (
    id, scenario_id, status, coalescing_key, subject_type, subject_id, ability_id,
    first_signal_id, latest_signal_id, raw_signal_count, payload_json,
    stale_marker_json, updated_at
)
VALUES (
    'invalidation-b18-coalesced',
    'signal-coalescing-preserves-invalidations',
    'completed',
    'claim_recompute:account:account-b18-example:get_entity_context',
    'account',
    'account-b18-example',
    'get_entity_context',
    'signal-b18-coalesce-a',
    'signal-b18-coalesce-b',
    2,
    '{"invalidated_claim_ids":["claim-b18-coalesced-invalidated"],"coalesced_signal_ids":["signal-b18-coalesce-a","signal-b18-coalesce-b"],"dropped_invalidations":[]}',
    '{"briefing_stale_before_refresh":true,"briefing_stale_after_refresh":false}',
    '2026-05-15T12:14:00Z'
);

INSERT INTO refresh_jobs (
    id, scenario_id, subject_ref, refresh_kind, status, dedup_key,
    created_claim_id, coalesced_into, completed_at
)
VALUES
    (
        'refresh-b18-duplicate-a',
        'duplicate-refresh-single-claim-row',
        '{"kind":"account","id":"account-b18-example"}',
        'manual_refresh',
        'completed',
        'b18:duplicate-refresh:account-b18-example:risk',
        'claim-b18-duplicate-refresh-canonical',
        NULL,
        '2026-05-15T12:17:00Z'
    ),
    (
        'refresh-b18-duplicate-b',
        'duplicate-refresh-single-claim-row',
        '{"kind":"account","id":"account-b18-example"}',
        'manual_refresh',
        'coalesced',
        'b18:duplicate-refresh:account-b18-example:risk',
        NULL,
        'refresh-b18-duplicate-a',
        '2026-05-15T12:17:00Z'
    );

INSERT INTO commitments (
    id, scenario_id, subject_ref, source_refresh_job_id, dedup_key, text, status, created_at
)
VALUES (
    'commitment-b18-duplicate-refresh-canonical',
    'duplicate-refresh-single-claim-row',
    '{"kind":"account","id":"account-b18-example"}',
    'refresh-b18-duplicate-a',
    'b18:duplicate-refresh:account-b18-example:commitment',
    'Follow up with the account owner after the refresh result.',
    'open',
    '2026-05-15T12:17:00Z'
);

INSERT INTO eval_replay_runs (
    id, scenario_id, run_label, clock, seed, provider_replay_key, canonical_output_json
)
VALUES
    (
        'eval-b18-run-a',
        'eval-determinism-canonical-equality',
        'first',
        '2026-05-15T12:18:00Z',
        180293,
        'bundle-18-v1-synthetic-fingerprint',
        '{"surface":"prepare_meeting","scenario_id":"eval-determinism-canonical-equality","source_claim_ids":["claim-b18-user-correction-current","claim-b18-generation-current"],"result":{"status":"degraded","offline":true}}'
    ),
    (
        'eval-b18-run-b',
        'eval-determinism-canonical-equality',
        'second',
        '2026-05-15T12:18:00Z',
        180293,
        'bundle-18-v1-synthetic-fingerprint',
        '{"scenario_id":"eval-determinism-canonical-equality","result":{"offline":true,"status":"degraded"},"source_claim_ids":["claim-b18-user-correction-current","claim-b18-generation-current"],"surface":"prepare_meeting"}'
    );
