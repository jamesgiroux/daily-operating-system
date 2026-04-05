# Signal Scoring Reference

**Status:** Active reference
**Last updated:** 2026-03-02
**Scope:** Meeting entity resolution, auto-linking, and briefing context scoping

## Why this doc exists

This document is the operational reference for debugging cross-entity bleed in meeting briefings.
It describes the live scoring model, thresholds, where each threshold is enforced in code, and how to investigate mis-links.

Primary code paths:
- `src-tauri/src/prepare/entity_resolver.rs`
- `src-tauri/src/signals/fusion.rs`
- `src-tauri/src/signals/bus.rs`
- `src-tauri/src/signals/event_trigger.rs`
- `src-tauri/src/prepare/meeting_context.rs`

## Signal Sources and Raw Confidence

Entity resolution signals currently emit these base confidences:

- `junction` (manual/explicit meeting link): `0.95`
- `attendee_vote`: `0.5 + 0.4 * (votes / total_attendees)`, capped at `0.90`
- `group_pattern`: learned confidence from `attendee_group_patterns` table (up to `0.85`)
- `keyword` exact match: `0.80` (name) or `0.65` (keyword list)
- `keyword_fuzzy`: `0.55`
- `embedding`: `0.4 + 0.4 * cosine_similarity`, with similarity gate `> 0.75`

Reference: `src-tauri/src/prepare/entity_resolver.rs`

## Source Weights and Decay (Weighted Fusion)

The resolver uses weighted log-odds fusion with:

`effective_weight = source_base_weight * temporal_decay * learned_reliability`

Base source weights (`signals/bus.rs`):

- `user_correction`, `explicit`: `1.0`
- `transcript`, `notes`: `0.9`
- `attendee`, `attendee_vote`, `email_thread`, `junction`: `0.8`
- `group_pattern`: `0.75`
- `proactive`, `glean*`: `0.7`
- `clay`, `gravatar`: `0.6`
- `keyword`, `keyword_fuzzy`, `heuristic`, `embedding`: `0.4`
- default: `0.5`

Default half-life days:

- `user_correction`, `explicit`: `365`
- `transcript`, `notes`: `60`
- `attendee*`, `junction`: `30`
- `group_pattern`: `60`
- `proactive`: `3`
- `glean*`: `60`
- `clay`, `gravatar`: `90`
- `keyword*`, `heuristic`, `embedding`: `7`
- default: `30`

References:
- `src-tauri/src/signals/bus.rs`
- `src-tauri/src/signals/decay.rs`
- `src-tauri/src/signals/fusion.rs`

## Resolver Outcome Thresholds

Resolver output bands (`entity_resolver.rs`):

- `Resolved`: `>= 0.85`
- `ResolvedWithFlag`: `0.60–0.85`
- `Suggestion`: `0.30–0.60`
- `NoMatch`: `< 0.30`

These are classification thresholds, not persistence thresholds.

## Persistence Guardrails (Auto-link)

Auto-linking into `meeting_entities` is stricter than resolver output.

As of 2026-03-02 (`signals/event_trigger.rs`):

- Only `Resolved` outcomes are eligible for auto-link.
- `ResolvedWithFlag` is not auto-linked.
- Auto-link requires `confidence >= 0.85` and source-specific minimum confidence:
  - `junction`, `keyword`: `0.85`
  - `group_pattern`: `0.88`
  - `attendee_vote`: `0.93`
  - `embedding`: `0.95`
  - `keyword_fuzzy`: `0.96`
  - default fallback: `0.95`
- At most one auto-linked entity per entity type (`account`, `project`, `person`) is persisted for a meeting.

This prevents low-specificity sources from mass-linking multiple accounts/projects to a single meeting.

## Prep Context Selection Guardrails

Meeting prep context consumes resolver outcomes but uses additional scoping (`prepare/meeting_context.rs`):

- Primary entity minimum confidence is `0.75`.
- `ResolvedWithFlag` candidates are only considered when:
  - `confidence >= 0.75`, and
  - source is one of: `junction`, `keyword`, `group_pattern`.
- For customer/external meeting types (`customer`, `qbr`, `partnership`, `training`, `external`), account/project entities are preferred over person entities when available.

This reduces cases where a high-confidence person link displaces the intended account context for customer-facing meetings.

## Scoped Data Requirements (Anti-Bleed)

Account context queries must use stable `entity_id` (not display name):

- `get_captures_for_account(account_id, ...)`
- `get_account_actions(account_id)`
- `get_meeting_history(account_id, ...)`

If account name is used accidentally, scoped retrieval can fail, causing sparse context and fallback behavior that increases bleed risk.

Reference: `prepare/meeting_context.rs` (`gather_account_context`).

## Troubleshooting Playbook

When a briefing includes unrelated entities:

1. Inspect linked entities on the meeting:
```sql
SELECT meeting_id, entity_id, entity_type
FROM meeting_entities
WHERE meeting_id = ?;
```

2. Inspect entity-resolution signal history for candidate entities:
```sql
SELECT entity_type, entity_id, signal_type, source, confidence, created_at, value
FROM signal_events
WHERE signal_type IN ('entity_resolution', 'entity_resolved')
  AND superseded_by IS NULL
ORDER BY created_at DESC
LIMIT 200;
```

3. Check whether weak-source links were persisted (should not happen with current guardrails).

4. Validate that context assembly uses the intended entity ID and not a name string.

5. Rebuild affected meeting prep after unlinking bad entities:
- unlink incorrect `meeting_entities` rows
- trigger meeting briefing refresh

## Known Failure Modes

- Multi-account auto-linking from ambiguous attendee patterns.
- Person entity winning primary-context selection for customer meetings.
- Sparse account context caused by ID/name mismatch in DB lookups.
- Historical contamination: previously bad links can continue influencing entity intelligence until cleaned.

## Remediation Guidance

For contamination already in data:

- Remove incorrect rows from `meeting_entities`.
- Re-run prep generation for affected meetings.
- Re-enrich affected entities to refresh `intelligence.json` from corrected context.

For prevention (already implemented):

- stricter auto-link gating
- one-per-type auto-link selection
- stricter primary entity selection
- account ID-scoped context queries

## Related ADRs

- `.docs/decisions/0080-signal-intelligence-architecture.md`
- `.docs/decisions/0081-event-driven-meeting-intelligence.md`
- `.docs/decisions/0086-intelligence-as-shared-service.md`
- `.docs/decisions/0095-dual-mode-context-architecture.md`
- `.docs/decisions/0096-glean-mode-local-footprint.md`
