# Intelligence Consistency Reference

**Status:** Active reference  
**Last updated:** 2026-03-03  
**Scope:** Contradiction prevention and trust diagnostics for `intelligence.json` generation

## Purpose

This is the troubleshooting reference for logical contradictions in briefing intelligence. It complements:

- `.docs/design/SIGNAL-SCORING-REFERENCE.md` (signal math and linking thresholds),
- `.docs/architecture/PIPELINES.md` (pipeline flow),
- `.docs/_archive/issues/i527.md` (guardrail contract).

## End-to-End Flow (Scoring + Context + Guardrails)

1. Signals are emitted and fused with weighted confidence (`signals/bus.rs`, `signals/fusion.rs`).
2. Entity links are persisted only when auto-link guardrails pass (`signals/event_trigger.rs`).
3. Intelligence prompt context is assembled from scoped DB facts (`intelligence/prompts.rs`).
4. LLM output is parsed into `IntelligenceJson` (`intelligence/prompts.rs`).
5. Consistency guardrails run before write (`intelligence/consistency.rs` + `intel_queue.rs`):
   - deterministic contradiction check,
   - deterministic repair pass,
   - one retry only for unresolved high-severity findings,
   - persist corrected/flagged status and findings.
6. Meeting briefing surfaces `consistencyStatus` transparently (`MeetingDetailPage.tsx`).

## Thresholds and Guardrails (Live Values)

### Entity Resolution and Persistence

From `.docs/design/SIGNAL-SCORING-REFERENCE.md` and code:

- Resolver bands:
  - `Resolved >= 0.85`
  - `ResolvedWithFlag 0.60-0.85`
  - `Suggestion 0.30-0.60`
- Auto-link persistence:
  - requires `Resolved` and `confidence >= 0.85`,
  - source-specific minimums:
    - `junction/keyword: 0.85`
    - `group_pattern: 0.88`
    - `attendee_vote: 0.93`
    - `embedding: 0.95`
    - `keyword_fuzzy: 0.96`
  - max one auto-linked entity per type (`account`, `project`, `person`).

### Prompt Context Retrieval

From `src-tauri/src/intelligence/prompts.rs`:

- Meeting/capture lookback: 90 days.
- File inclusion lookback: 90 days.
- File context budget: `MAX_CONTEXT_BYTES = 10_000`.
- Email signal context limit: `20` when next meeting exists, else `12`.
- Semantic retrieval weights in `search_entity_content(...)`: `0.7`, `0.3`.
- User context semantic threshold: `0.82` for entries and attachments.
- Verified stakeholder presence lines injected into prompt: top `8` by recency/attendance.

### Consistency Rules (I527)

From `src-tauri/src/intelligence/consistency.rs`:

- `ABSENCE_CONTRADICTION` (`high`):
  - claim text includes absence phrase (`never appeared/attended`),
  - person mention is present,
  - deterministic attendance evidence `attendance_count >= 1`.
- `NO_PROGRESS_CONTRADICTION` (`high`):
  - claim includes `no new progress signals`,
  - deterministic recent signals in last 14 days `>= 2`.
- `AUTHORITY_UNKNOWN_CONTRADICTION` (`medium`):
  - claim includes unknown authority/stake language,
  - person has role/title and/or attendance evidence.
- `CROSS_ENTITY_BLEED_SUSPECT` (`medium`):
  - stakeholder name not linked to target entity,
  - no attendance evidence in target entity meetings.

### Balanced Repair Policy

From `src-tauri/src/intel_queue.rs`:

1. deterministic repair runs first on all findings,
2. unresolved high-severity findings allow exactly one retry,
3. unresolved findings after retry are persisted with `consistencyStatus = flagged`,
4. write is not blocked; future refreshes can overwrite prior corrected/flagged output.

## Consistency Metadata Contract

`IntelligenceJson` (Rust + TS mirror) adds optional fields:

- `consistencyStatus: "ok" | "corrected" | "flagged"`
- `consistencyFindings: ConsistencyFinding[]`
- `consistencyCheckedAt: string`

`ConsistencyFinding` fields:

- `code`, `severity`, `fieldPath`, `claimText`, `evidenceText`, `autoFixed`.

## Diagnostic SQL

### 1) Find likely false "never appeared" claims with attendance evidence

```sql
SELECT
  ei.entity_id,
  ei.executive_assessment,
  p.name AS stakeholder_name,
  COUNT(DISTINCT ma.meeting_id) AS attendance_count
FROM entity_intelligence ei
JOIN meeting_entities me ON me.entity_id = ei.entity_id
JOIN meeting_attendees ma ON ma.meeting_id = me.meeting_id
JOIN people p ON p.id = ma.person_id
WHERE LOWER(COALESCE(ei.executive_assessment, '')) LIKE '%never appeared%'
GROUP BY ei.entity_id, ei.executive_assessment, p.name
HAVING attendance_count >= 1
ORDER BY attendance_count DESC;
```

### 2) Find "no new progress signals" claims despite recent signal activity

```sql
SELECT
  ei.entity_id,
  ei.entity_type,
  ei.executive_assessment,
  COUNT(se.id) AS signals_14d
FROM entity_intelligence ei
JOIN signal_events se
  ON se.entity_id = ei.entity_id
 AND se.entity_type = ei.entity_type
WHERE se.superseded_by IS NULL
  AND se.created_at >= datetime('now', '-14 days')
  AND LOWER(COALESCE(ei.executive_assessment, '')) LIKE '%no new progress signals%'
GROUP BY ei.entity_id, ei.entity_type, ei.executive_assessment
HAVING COUNT(se.id) >= 2
ORDER BY signals_14d DESC;
```

### 3) Stakeholder bleed triage from `stakeholder_insights_json`

```sql
WITH insight_names AS (
  SELECT
    ei.entity_id,
    LOWER(TRIM(json_extract(j.value, '$.name'))) AS stakeholder_name
  FROM entity_intelligence ei,
       json_each(COALESCE(ei.stakeholder_insights_json, '[]')) AS j
  WHERE json_extract(j.value, '$.name') IS NOT NULL
),
linked_names AS (
  SELECT
    ep.entity_id,
    LOWER(TRIM(p.name)) AS stakeholder_name
  FROM entity_people ep
  JOIN people p ON p.id = ep.person_id
),
attendance_names AS (
  SELECT
    me.entity_id,
    LOWER(TRIM(p.name)) AS stakeholder_name,
    COUNT(DISTINCT ma.meeting_id) AS attendance_count
  FROM meeting_entities me
  JOIN meeting_attendees ma ON ma.meeting_id = me.meeting_id
  JOIN people p ON p.id = ma.person_id
  GROUP BY me.entity_id, LOWER(TRIM(p.name))
)
SELECT
  i.entity_id,
  i.stakeholder_name
FROM insight_names i
LEFT JOIN linked_names l
  ON l.entity_id = i.entity_id
 AND l.stakeholder_name = i.stakeholder_name
LEFT JOIN attendance_names a
  ON a.entity_id = i.entity_id
 AND a.stakeholder_name = i.stakeholder_name
WHERE l.stakeholder_name IS NULL
  AND COALESCE(a.attendance_count, 0) = 0
ORDER BY i.entity_id, i.stakeholder_name;
```

## Worked Example: Janus / Matt Wickham

Observed failure:
- intelligence text claimed Matt Wickham "has never appeared in a recorded meeting",
- meeting history already showed attendance.

Expected behavior after I527:
1. `build_fact_context` records Matt attendance count and last-seen timestamp.
2. `check_consistency` raises `ABSENCE_CONTRADICTION` (high).
3. deterministic repair rewrites the absolute false absence phrasing.
4. if no high findings remain, persist with `consistencyStatus = corrected`.
5. meeting briefing shows corrected banner:
   - "Context auto-corrected against meeting records."
6. later refreshes are still allowed to replace content with improved synthesis.

## Operational Triage Checklist

1. Confirm target meeting entity links are correct in `meeting_entities`.
2. Validate attendance evidence exists for named stakeholders.
3. Validate recent signal count window before accepting "no progress" language.
4. Inspect `consistencyStatus` and `consistencyFindings` on the resulting intelligence row.
5. If flagged persists across refreshes, inspect prompt grounding section and linked-entity bleed at source.
