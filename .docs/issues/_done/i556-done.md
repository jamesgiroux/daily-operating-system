# I556 — Report Content Pipeline: Meeting Summaries + Captures for Reports

**Version:** v1.0.0 Phase 4
**Depends on:** I555 (captures metadata — enables urgency-filtered capture queries)
**Type:** Enhancement — report data gathering
**Scope:** Backend report generators only. No frontend changes.

---

## Problem

Three report types are starved of meeting content:

### 1. Weekly Impact — titles only, no substance

`weekly_impact.rs` queries meetings for the reporting week but only gets `title` and `start_time`. It never reads `meeting_transcripts.summary` or `captures`. The LLM must infer what happened from meeting titles alone.

This means "priorities moved" items are fabricated from titles like "Acme Weekly Sync" rather than grounded in actual discussion outcomes. The report feels generic because it IS generic — it has no content to work with.

### 2. Monthly Wrapped — same problem at larger scale

`monthly_wrapped.rs` gets meeting titles, timestamps, and IDs. No summaries, no captures, no wins/risks. "Moments" and "hidden patterns" are hallucinated from meeting metadata.

### 3. EBR/QBR — asks for quotes it can't access

`ebr_qbr.rs` requests `customer_quote` (exact words from transcript) but the data path provides only the synthesized `executive_assessment` prose. If the entity intelligence pipeline didn't preserve a verbatim quote in its narrative, the LLM must hallucinate or return null.

After I554/I555 land, verbatim quotes are stored in `captures.evidence_quote`. These should flow into EBR/QBR reports.

---

## Solution

### 1. Weekly Impact — Add meeting summaries + captures

In `weekly_impact.rs` data gathering, after fetching the week's meetings, also fetch:

```sql
-- Meeting summaries for the week
SELECT m.id, m.title, m.start_time, mt.summary
FROM meetings m
LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
WHERE m.start_time BETWEEN ? AND ?
ORDER BY m.start_time

-- Captures from the week (wins, risks, decisions)
SELECT c.capture_type, c.content, c.sub_type, c.urgency, c.impact,
       c.evidence_quote, c.meeting_title, c.captured_at
FROM captures c
WHERE c.captured_at BETWEEN ? AND ?
ORDER BY c.captured_at
```

Inject into the prompt as:

```
## Meeting Content (this week)
- {date} | {title} | Summary: {summary or "no transcript processed"}

## Outcomes Captured (this week)
WINS:
- [{sub_type}] {content} — from {meeting_title} ({date}) {#"quote" if available}
RISKS:
- [{urgency}] {content} — from {meeting_title} ({date})
DECISIONS:
- {content} — from {meeting_title} ({date})
```

Update the prompt instructions:
```
- priorities_moved: cite specific meeting outcomes from the content above, not just meeting titles
- wins: reference actual captured wins with evidence
- watch: reference actual captured risks with urgency context
```

### 2. Monthly Wrapped — Add summaries + captures + champion health

Same pattern as Weekly Impact but for the full calendar month. Additionally, include:

```sql
-- Champion health assessments from the month
SELECT mch.meeting_id, m.title, mch.champion_name, mch.champion_status,
       mch.champion_evidence
FROM meeting_champion_health mch
JOIN meetings m ON m.id = mch.meeting_id
WHERE m.start_time BETWEEN ? AND ?

-- Interaction dynamics (engagement quality)
SELECT mid.meeting_id, m.title, mid.talk_balance_customer_pct,
       mid.question_density, mid.decision_maker_active
FROM meeting_interaction_dynamics mid
JOIN meetings m ON m.id = mid.meeting_id
WHERE m.start_time BETWEEN ? AND ?
```

This gives Monthly Wrapped grounded data for:
- `moments` — cite actual meeting outcomes, not inferred from titles
- `hidden_pattern` — use engagement dynamics and champion health trends
- `top_win` — reference the strongest captured win of the month

### 3. EBR/QBR — Customer quote pipeline

In `ebr_qbr.rs`, add a dedicated quote gathering step:

```sql
-- Verbatim customer quotes from recent captures (90 days)
SELECT c.evidence_quote, c.content, c.capture_type, c.meeting_title, c.captured_at
FROM captures c
WHERE c.account_id = ?
  AND c.evidence_quote IS NOT NULL
  AND c.evidence_quote != ''
ORDER BY c.captured_at DESC
LIMIT 10
```

Inject as a dedicated prompt section:

```
## Customer Quotes (verbatim from meetings)
- "{quote}" — {meeting_title}, {date} (context: {content})
```

Update EBR/QBR prompt instruction for `customer_quote`:
```
customer_quote: Select the most impactful verbatim customer quote from the Customer Quotes
section above. Use their exact words. If no suitable quotes are available, return null.
Do NOT fabricate or paraphrase — only use quotes marked as verbatim.
```

### 4. Account Health — Captures with urgency

In `account_health.rs`, supplement the existing data gathering with urgency-enriched captures:

```sql
SELECT c.capture_type, c.content, c.sub_type, c.urgency, c.impact,
       c.evidence_quote, c.captured_at
FROM captures c
WHERE c.account_id = ?
  AND c.captured_at > datetime('now', '-90 days')
ORDER BY c.captured_at DESC
```

This lets the Account Health report distinguish RED risks from GREEN_WATCH concerns and reference specific customer quotes.

### 5. Book of Business — Aggregate captures

In `book_of_business.rs`, add:

```sql
-- Top wins and risks across all accounts (90 days), urgency-sorted
SELECT c.capture_type, c.content, c.sub_type, c.urgency, c.impact,
       c.evidence_quote, a.name as account_name, c.captured_at
FROM captures c
JOIN accounts a ON a.id = c.account_id
WHERE c.captured_at > datetime('now', '-90 days')
ORDER BY
  CASE c.urgency WHEN 'red' THEN 1 WHEN 'yellow' THEN 2 WHEN 'green_watch' THEN 3 ELSE 4 END,
  c.captured_at DESC
LIMIT 30
```

This gives the BoB report grounded wins/risks with account attribution and urgency context, rather than relying solely on truncated executive assessments.

---

## Files

| File | Changes |
|------|---------|
| `src-tauri/src/reports/weekly_impact.rs` | Add meeting summary + captures queries to data gathering. Inject into prompt. Update prompt instructions for grounded citations. |
| `src-tauri/src/reports/monthly_wrapped.rs` | Add meeting summary + captures + champion health + dynamics queries. Inject into prompt. Update prompt instructions. |
| `src-tauri/src/reports/ebr_qbr.rs` | Add verbatim quote query. Add "Customer Quotes" prompt section. Update `customer_quote` instruction to use only verified quotes. |
| `src-tauri/src/reports/account_health.rs` | Add urgency-enriched captures query. Inject sub-type and urgency context into prompt. |
| `src-tauri/src/reports/book_of_business.rs` | Add aggregate captures query with urgency sorting. Inject into prompt alongside executive assessments. |

---

## Context Budget Management

Meeting summaries and captures add to the prompt context. Budget constraints:

| Report | Current context | Added context | Budget |
|--------|----------------|---------------|--------|
| Weekly Impact | ~2K tokens (titles + signals) | +3-5K (summaries + captures) | Fine — well within model limits |
| Monthly Wrapped | ~3K tokens | +5-8K (month of summaries + captures + dynamics) | Fine — month is bounded |
| EBR/QBR | ~15-20K tokens (full intel context) | +1-2K (10 quotes) | Fine — small addition |
| Account Health | ~15-20K tokens | +1-2K (urgency-enriched captures) | Fine |
| Book of Business | ~20-30K tokens | +2-3K (30 aggregate captures) | Fine — captures replace some of the vagueness the LLM currently fills |

No model tier changes needed. All reports already use `ModelTier::Synthesis`.

---

## Out of Scope

- Frontend rendering of quotes or captures in reports (report renderers already show the output fields)
- Prompt definition changes (I554)
- Schema changes (I555)
- New report types

---

## Acceptance Criteria

1. Weekly Impact report `priorities_moved` items cite specific meeting outcomes (summaries or captures), not just meeting titles.
2. Weekly Impact prompt includes meeting summaries and week's captured wins/risks/decisions.
3. Monthly Wrapped `moments` cite verifiable events from the month's meeting data.
4. Monthly Wrapped prompt includes champion health trends and engagement dynamics.
5. EBR/QBR `customer_quote` field contains a verbatim quote from `captures.evidence_quote` when available. Returns null (not hallucinated text) when no quotes exist.
6. EBR/QBR prompt includes a "Customer Quotes" section with source attribution.
7. Account Health report distinguishes RED risks from GREEN_WATCH in its analysis.
8. Book of Business report includes urgency-sorted captures with account attribution.
9. All new queries are bounded (LIMIT clauses, date windows) to prevent context overflow.
10. Reports that ran before I555 (no captures metadata) still generate correctly — null sub_type/urgency handled gracefully.
11. Generate a Weekly Impact report for a week with 5+ meetings that have transcripts. Verify the report references specific discussion outcomes, not just "you had a meeting with Acme."

### Glean Data in Reports
12. Book of Business report reads `entity_assessment.org_health_json` for Glean-enriched accounts. `resolve_health_band()` uses Salesforce health band when available (more reliable than computed-only). Verify: BoB snapshot table for a Glean-enriched account shows Salesforce-sourced health band, not just local `entity_quality.health_score`.
13. Reports consuming Glean-sourced `captures` (with `source='glean_chat'`) render them identically to PTY-sourced captures. No source-based filtering — both Glean and PTY captures appear in report context.

### Mock Data
12. Mock data (`full` scenario) seeds `meeting_transcripts.summary` for at least 5 meetings within the last 7 days (for Weekly Impact) and 15 within the last 30 days (for Monthly Wrapped).
13. Mock data seeds `captures` with `evidence_quote` values for at least 3 entries (for EBR/QBR customer quote pipeline).
14. After applying `full` mock scenario, generating a Weekly Impact report produces `priorities_moved` items that cite mock meeting summaries — not just titles. Generating an EBR/QBR report for a mock account returns a `customer_quote` from mock `evidence_quote` data.
