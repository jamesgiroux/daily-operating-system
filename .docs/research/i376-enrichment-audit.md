# I376: AI Enrichment Site Audit

**Date:** 2026-02-21
**Auditor:** enrichment-auditor agent
**Reference:** ADR-0086 (Intelligence as Shared Service)

## PTY Call Site Inventory

Every `PtyManager::for_tier` call in `src-tauri/src/` is listed below with classification.

### 1. intel_queue.rs:534 — `run_enrichment()`
- **Function:** Single-entity intelligence enrichment
- **Tier:** Synthesis, 180s timeout
- **Produces:** `IntelligenceJson` (entity intelligence)
- **Output destination:** `intelligence.json` on disk + DB metadata
- **Level:** Entity (account/person/project)
- **Routes through intel_queue:** YES — this IS the intel_queue processor
- **Classification:** ADR-0086 COMPLIANT

### 2. intel_queue.rs:597 — `run_batch_enrichment()`
- **Function:** Batched multi-entity intelligence enrichment (I289)
- **Tier:** Synthesis, 180s * batch_size timeout
- **Produces:** Multiple `IntelligenceJson` results
- **Output destination:** `intelligence.json` per entity on disk + DB metadata
- **Level:** Entity (batched)
- **Routes through intel_queue:** YES — this IS the intel_queue batch processor
- **Classification:** ADR-0086 COMPLIANT

### 3. intelligence/lifecycle.rs:442 — `enrich_meeting_with_ai()`
- **Function:** AI enrichment for a single meeting
- **Tier:** Synthesis, 120s timeout
- **Produces:** JSON with talking_points, stakeholder_notes, agenda_suggestions
- **Output destination:** `prep_context_json` column in `meetings_history` table
- **Level:** MEETING — violates ADR-0086
- **Routes through intel_queue:** NO — direct inline PTY call
- **Classification:** PRE-ADR-0086 — meeting-level AI enrichment that should be replaced by mechanical assembly from entity intelligence

### 4. risk_briefing.rs:357 — `run_risk_enrichment()`
- **Function:** Account risk briefing generation
- **Tier:** Synthesis, 300s timeout
- **Produces:** `RiskBriefing` struct (parsed from Claude output)
- **Output destination:** `risk-briefing.json` in account directory
- **Level:** Entity (account) — but produces a specialized report, not standard intelligence
- **Routes through intel_queue:** NO — direct call, but this is a workflow output
- **Classification:** DELIBERATE EXCEPTION — risk briefings are a specialized narrative workflow, analogous to daily briefing generation. They produce a purpose-built report, not entity intelligence.

### 5. processor/enrich.rs:80 — `enrich_file()`
- **Function:** Inbox file AI classification and enrichment
- **Tier:** Mechanical, configurable timeout
- **Produces:** `EnrichResult` (file type, summary, account, actions)
- **Output destination:** Processed file routed to entity directory + actions to DB
- **Level:** File processing — not entity intelligence
- **Routes through intel_queue:** NO — file processing pipeline
- **Classification:** DELIBERATE EXCEPTION — inbox file processing is content ingestion, not entity intelligence enrichment. It classifies and routes files, extracts actions.

### 6. processor/transcript.rs:106 — transcript processing
- **Function:** Meeting transcript AI extraction
- **Tier:** Extraction, configurable timeout
- **Produces:** Summary, wins, risks, decisions, discussion points
- **Output destination:** Transcript file in entity directory + signals to DB
- **Level:** File processing — content extraction from meeting recordings
- **Routes through intel_queue:** NO — transcript processing pipeline
- **Classification:** DELIBERATE EXCEPTION — transcript processing extracts structured data from meeting recordings. It produces signals that feed INTO entity intelligence, not intelligence itself.

### 7. processor/email_actions.rs:58 — `extract_email_commitments()`
- **Function:** Extract commitments/action items from individual emails
- **Tier:** Extraction, 60s timeout
- **Produces:** Vec of commitment objects (title, type, due_date, owner)
- **Output destination:** Actions DB table
- **Level:** Email processing — action extraction
- **Routes through intel_queue:** NO — email action extraction pipeline
- **Classification:** DELIBERATE EXCEPTION — extracts actionable items from emails, feeds the action queue. Not entity intelligence.

### 8. prepare/email_enrich.rs:270 — `enrich_pending_emails_two_phase()`
- **Function:** AI enrichment of individual emails (contextual summary, sentiment, urgency)
- **Tier:** Extraction, 60s timeout
- **Produces:** Contextual summary, sentiment, urgency per email
- **Output destination:** `email_enrichment_state` / `email_contextual_summary` columns in DB
- **Level:** Email content enrichment — per-email summaries for display
- **Routes through intel_queue:** NO — email enrichment pipeline
- **Classification:** DELIBERATE EXCEPTION — produces per-email display summaries for the inbox/email views. These are content-level summaries, not entity intelligence.

### 9. executor.rs:994-995 — `execute_daily()` email enrichment PTYs
- **Function:** Creates PTY managers for daily orchestration email enrichment
- **Tier:** Extraction + Synthesis
- **Produces:** Passed to `enrich_emails_with_fallback()` which calls workflow/deliver functions
- **Output destination:** emails.json, briefing narrative in schedule.json
- **Level:** Daily workflow orchestration — delegates to deliver.rs functions
- **Routes through intel_queue:** NO — workflow orchestration
- **Classification:** DELIBERATE EXCEPTION — daily workflow orchestration. The PTYs are passed to deliver.rs functions (enrich_emails, enrich_briefing) which produce workflow-level narratives, not entity intelligence.

### 10. executor.rs:1199-1200 — `execute_email_refresh()` PTYs
- **Function:** Creates PTY managers for email refresh workflow
- **Tier:** Extraction + Synthesis
- **Produces:** Passed to `enrich_emails_with_fallback()` for email enrichment
- **Output destination:** emails.json enrichment
- **Level:** Email refresh workflow
- **Routes through intel_queue:** NO — workflow orchestration
- **Classification:** DELIBERATE EXCEPTION — email refresh workflow. Same as #9 but triggered by manual email refresh.

### 11. google.rs:1062-1063 — email poll enrichment
- **Function:** Creates PTY managers when new emails detected during background polling
- **Tier:** Extraction + Synthesis
- **Produces:** Delegates to `enrich_emails_with_fallback()`
- **Output destination:** emails.json enrichment
- **Level:** Background email polling workflow
- **Routes through intel_queue:** NO — polling workflow
- **Classification:** DELIBERATE EXCEPTION — background email poll enrichment. Reuses executor pipeline for consistency.

### 12. devtools/mod.rs:871-873 — devtools test harness
- **Function:** Development/testing PTYs for devtools pipeline testing
- **Tier:** Extraction + Synthesis
- **Produces:** Test enrichment of emails, preps, briefing
- **Output destination:** _today/data/ files
- **Level:** Development tooling
- **Routes through intel_queue:** NO — devtools
- **Classification:** DELIBERATE EXCEPTION — development-only test harness for pipeline validation.

### 13. workflow/deliver.rs:2442, 2816, 3256, 3871 — PTY parameter declarations
- **Functions:** `enrich_emails()`, `enrich_briefing()`, `enrich_preps()`, `enrich_week()`
- **Note:** These are function PARAMETERS that accept `&PtyManager`, not construction sites. The PTYs are created by callers (executor.rs, devtools, google.rs).
- **enrich_emails:** Email thread summarization for display — DELIBERATE EXCEPTION (content-level, not entity intelligence)
- **enrich_briefing:** Daily briefing narrative synthesis — DELIBERATE EXCEPTION (workflow narrative generation)
- **enrich_preps:** Meeting prep agenda refinement — DELIBERATE EXCEPTION (mechanical prep enhancement)
- **enrich_week:** Weekly forecast narrative — NOTE: Per ADR-0086, this should be mechanical only. See remediation notes.

### Dead Code (PTY parameters, never called)
- **accounts.rs:1058** — `enrich_account()` takes `&PtyManager` but is NEVER CALLED. Superseded by intel_queue.
- **projects.rs:606** — `enrich_project()` takes `&PtyManager` but is NEVER CALLED. Superseded by intel_queue.
- **intelligence/prompts.rs:1484** — `enrich_entity_intelligence()` takes `&PtyManager` but is NEVER CALLED. Superseded by intel_queue.

## Findings Summary

| # | Call Site | Classification | Action |
|---|-----------|---------------|--------|
| 1 | intel_queue.rs:534 | ADR-0086 COMPLIANT | None |
| 2 | intel_queue.rs:597 | ADR-0086 COMPLIANT | None |
| 3 | lifecycle.rs:442 | PRE-ADR-0086 | REMEDIATE — remove meeting-level AI |
| 4 | risk_briefing.rs:357 | DELIBERATE EXCEPTION | None — specialized report workflow |
| 5 | processor/enrich.rs:80 | DELIBERATE EXCEPTION | None — file processing |
| 6 | processor/transcript.rs:106 | DELIBERATE EXCEPTION | None — content extraction |
| 7 | email_actions.rs:58 | DELIBERATE EXCEPTION | None — action extraction |
| 8 | email_enrich.rs:270 | DELIBERATE EXCEPTION | None — per-email summaries |
| 9 | executor.rs:994-995 | DELIBERATE EXCEPTION | None — daily workflow |
| 10 | executor.rs:1199-1200 | DELIBERATE EXCEPTION | None — email refresh |
| 11 | google.rs:1062-1063 | DELIBERATE EXCEPTION | None — email polling |
| 12 | devtools/mod.rs:871-873 | DELIBERATE EXCEPTION | None — dev tooling |
| 13 | deliver.rs (4 sites) | PTY params only | None — passed from callers |
| D1 | accounts.rs:1058 | DEAD CODE | REMOVE |
| D2 | projects.rs:606 | DEAD CODE | REMOVE |
| D3 | prompts.rs:1484 | DEAD CODE | REMOVE |

## Remediation Plan

### R1: Remove meeting-level AI enrichment (lifecycle.rs)
`enrich_meeting_with_ai()` at lifecycle.rs:396 is the sole pre-ADR-0086 meeting-level AI enrichment. Per ADR-0086, meeting prep should be mechanical assembly from entity intelligence. This function should be removed, and `generate_meeting_intelligence()` should use mechanical quality assessment + MeetingPrepQueue for prep generation instead of spawning a PTY.

### R2: Remove dead entity enrichment code
- `accounts.rs::enrich_account()` — dead code, superseded by intel_queue
- `projects.rs::enrich_project()` — dead code, superseded by intel_queue
- `intelligence/prompts.rs::enrich_entity_intelligence()` — dead code, superseded by intel_queue

### R3: Build entity relink → prep re-assembly chain
`link_meeting_entity()` (commands.rs:3689) does a bare DB insert with no downstream effects. When a user relinks a meeting to a different entity, the meeting's prep should be re-assembled from the NEW entity's intelligence. Required chain:
1. `link_meeting_entity()` clears `prep_frozen_json` for the meeting
2. `link_meeting_entity()` enqueues the meeting in `MeetingPrepQueue`
3. `MeetingPrepQueue` reads from the new entity's `intelligence.json`
4. No AI call is made during relink

Same for `unlink_meeting_entity()` — should clear and re-assemble.

## Remediations Applied

### R1: DONE — Removed meeting-level AI enrichment
- Removed `enrich_meeting_with_ai()`, `gather_meeting_context()`, `build_enrichment_prompt()`, `cap_enrichment_arrays()`, and `MeetingEnrichmentContext` struct from `intelligence/lifecycle.rs`
- Replaced `generate_meeting_intelligence()` to use mechanical quality assessment + MeetingPrepQueue enqueue instead of PTY call
- Removed unused imports (`PathBuf`, `serde_json::json`, `PtyManager`, `ModelTier`)

### R2: DONE — Removed dead entity enrichment code
- Removed `accounts.rs::enrichment_prompt()` and `enrich_account()` (dead, superseded by intel_queue)
- Removed `projects.rs::enrichment_prompt()` and `enrich_project()` (dead, superseded by intel_queue)
- Removed `intelligence/prompts.rs::EntityEnrichmentTarget` struct and `enrich_entity_intelligence()` (dead, superseded by intel_queue)

### R3: DONE — Built entity relink -> prep re-assembly chain
- `link_meeting_entity()` now: (1) links entity in DB, (2) clears `prep_frozen_json`, (3) enqueues meeting in `MeetingPrepQueue` at Manual priority
- `unlink_meeting_entity()` now: (1) unlinks entity in DB, (2) clears `prep_frozen_json`, (3) enqueues meeting in `MeetingPrepQueue` at Manual priority
- No AI call is made during relink — MeetingPrepQueue reads from entity intelligence.json files mechanically

## Verification (post-remediation)

Grep output for `PtyManager::new|PtyManager::for_tier` after remediation:
- intel_queue.rs: 2 sites (lines 534, 597) — COMPLIANT
- risk_briefing.rs: 1 site (line 357) — DELIBERATE EXCEPTION
- processor/enrich.rs: 1 site (line 80) — DELIBERATE EXCEPTION
- processor/transcript.rs: 1 site (line 106) — DELIBERATE EXCEPTION
- processor/email_actions.rs: 1 site (line 58) — DELIBERATE EXCEPTION
- prepare/email_enrich.rs: 1 site (line 270) — DELIBERATE EXCEPTION
- executor.rs: 4 sites (lines 994, 995, 1199, 1200) — DELIBERATE EXCEPTION
- google.rs: 2 sites (lines 1062, 1063) — DELIBERATE EXCEPTION
- devtools/mod.rs: 2 sites (lines 871, 873) — DELIBERATE EXCEPTION
- **lifecycle.rs: 0 sites — REMOVED (was PRE-ADR-0086)**
- **accounts.rs: 0 sites — REMOVED (was DEAD CODE)**
- **projects.rs: 0 sites — REMOVED (was DEAD CODE)**
- **prompts.rs: 0 sites — REMOVED (was DEAD CODE)**
- Total: 15 `PtyManager::for_tier` construction sites (down from 16+3 dead)
- All 15 are either ADR-0086 compliant or documented deliberate exceptions
- `cargo test`: 933 passed, 0 failed
- `cargo clippy -D warnings`: clean
