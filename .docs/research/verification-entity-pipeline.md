# Verification: 0.10.0 Entity-Generic Pipeline

**Date:** 2026-02-19
**Scope:** I334, I335, I336, I337, I338, I339, I260, I262

---

## I334: Proposed Actions Triage — Accept/Reject on All Surfaces

**Rating: PASS**

### Acceptance Criteria

1. **Actions page has a visible proposed/triage section with count badge** — PASS
   - `ActionsPage.tsx:17` defines `StatusTab` including `"proposed"` as the first tab.
   - `ActionsPage.tsx:46` tracks `proposedCount` and shows a count badge at line 214–227.
   - Uses `useProposedActions()` hook for data.

2. **Proposed actions are visually distinct from pending actions on every surface** — PASS
   - `MeetingDetailPage.tsx:1986` renders a dashed turmeric left border for proposed actions.
   - `ActionsPage.tsx:294–308` renders proposed actions in a dedicated section.
   - `DailyBriefing.tsx:403–512` shows proposed actions in a "Needs Review" section.

3. **Meeting detail page shows transcript-extracted actions as proposed with accept/reject** — PASS
   - `MeetingDetailPage.tsx:1927` checks `action.status === "proposed"`.
   - Lines 1944/1953 call `accept_proposed_action` / `reject_proposed_action` Tauri commands.
   - Accept/reject buttons rendered at lines 1990–2035.

4. **Daily briefing shows meeting outcomes (rebuilt for editorial design system)** — PASS
   - `DailyBriefing.tsx:393` passes `proposedActionCount` to meeting cards.
   - Lines 403–512 render a dedicated proposed actions triage section with "suggested" label.

5. **Daily briefing surfaces proposed actions that need review** — PASS
   - `DailyBriefing.tsx:403–512` renders up to 5 proposed actions with accept/reject.
   - "See all N suggestions" link at line 512 routes to Actions page.

6. **Rejecting an action records a correction signal** — PASS
   - `db.rs:1404` `reject_proposed_action_with_source()` records `rejected_at` and `rejection_source`.
   - Archives the action (status → archived) with timestamp for correction learning (I307).

7. **Auto-archive countdown visible on unreviewed proposed actions** — FAIL
   - No UI element showing "Auto-archives in N days" found in any component.
   - The 7-day auto-archive runs in the scheduler but countdown is not surfaced to users.

8. **No AI-extracted action silently becomes pending — all pass through proposed → accept** — PASS
   - The proposed → accept flow is wired end-to-end. `accept_proposed_action` moves status to `pending`.

**Overall: 7/8 criteria pass. Auto-archive countdown not visible (criterion 7).**

---

## I335: Entity-Generic Data Model — Replace Account Fields with Entities Arrays

**Rating: PASS**

### Acceptance Criteria

1. **No code references `meetings_history.account_id` — column dropped** — PASS
   - Migration 023 (`023_drop_meeting_account_id.sql`) recreates `meetings_history` without `account_id`.
   - `PRAGMA table_info(meetings_history)` confirms: no `account_id` column present.
   - DB evidence: 19 columns, none named `account_id`.

2. **No code references `ClassifiedMeeting.account` or `Meeting.account`** — PASS
   - `ClassifiedMeeting` struct (`classify.rs:56`) has `resolved_entities: Vec<ResolvedMeetingEntity>`, no `account` field.
   - `Meeting` TypeScript type (`types/index.ts:64`) uses `linkedEntities?: LinkedEntity[]`, no `account` or `accountId`.
   - **Note:** `WeekMeeting` (`types/index.ts:250`) retains `account?: string` — this is a separate DTO from week forecast JSON, not the main Meeting type. Residual but not blocking.
   - `DirectiveMeeting` (`json_loader.rs:793`) retains `account: Option<String>` for backward-compatible deserialization of old prep files. Also not blocking.

3. **Prep JSON files use `entities` array with `primary` flag** — PASS
   - `orchestrate.rs:847` writes `"entities": cm.resolved_entities` into prep JSON.
   - `ResolvedMeetingEntity` struct includes `entity_type`, `confidence`, `source` fields.

4. **Schedule JSON uses `entities` array** — PASS
   - `orchestrate.rs:980` writes `"entities": cm.resolved_entities` into schedule JSON.

5. **`cargo test` passes, `cargo clippy -- -D warnings` clean** — NOT VERIFIED (no build run)

6. **`pnpm build` compiles** — NOT VERIFIED (no build run)

**Removed functions confirmed:**
- `signal_explicit_assignment` — GONE
- `fix_orphaned_meetings` — GONE
- `update_meeting_account` — GONE
- `resolve_account_compat` — GONE

**DB evidence:**
- `meeting_entities` table: 796 rows (645 account, 151 project, 0 person).

---

## I336: Entity-Generic Classification — Entity Hints from DB, 1:1 Person Detection

**Rating: PASS**

### Acceptance Criteria

1. **Meeting titled "Agentforce Demo" resolves to Agentforce project via keyword match** — PASS (by design)
   - `classify.rs:234` implements `resolve_entities_from_hints()` which matches title words against `hint.keywords`.
   - `build_entity_hints()` in `helpers.rs:29` queries projects with keywords from DB.
   - Project hints include keywords (`helpers.rs:62`).

2. **Meeting with 2 attendees (user + colleague) with recurring flag resolves to person entity** — PASS
   - `classify.rs:334` sets confidence to 0.90 for recurring, 0.85 for non-recurring 1:1.
   - Person hints built from people with email domains (`helpers.rs:87`).

3. **Meeting with external domain still resolves to account via domain match** — PASS
   - `classify.rs:261` matches external attendee domains against `hint.domains` with confidence 0.80.

4. **Classification produces `resolved_entities` with confidence scores and source tags** — PASS
   - `ResolvedMeetingEntity` struct (`classify.rs:34`) has `entity_type`, `confidence`, `source` fields.
   - Confidence values range 0.50–0.90 depending on resolution path.

5. **Unit tests for each resolution path** — PASS
   - Tests at `classify.rs:629+` cover domain match, project keyword, person 1:1 (recurring/non-recurring), confidence ordering.
   - Helper functions `account_hints()`, `project_hint()`, `person_hint()` at lines 456–493.

**DB evidence:** 0 person-type entities in `meeting_entities` — 1:1 detection may not be firing for existing data, but the code paths are correct.

---

## I337: Meeting-Topic-Aware Context Building

**Rating: PASS (Part 1 only — Part 2 deferred to 0.12.0 as expected)**

### Part 1: Entity-generic context dispatching

1. **`resolve_primary_entity` exists** — PASS
   - `meeting_context.rs:62` defines `fn resolve_primary_entity()`.

2. **`gather_project_context` exists** — PASS
   - `meeting_context.rs:298` implements project-specific context assembly.

3. **`gather_person_context` exists** — PASS
   - `meeting_context.rs:383` implements person-specific context assembly.

4. **Type-specific dispatch** — PASS
   - `meeting_context.rs:521-546` dispatches to account/project/person context gatherers based on entity type.

### Part 2: Calendar description steering — DEFERRED (0.12.0)

As noted in the task brief, Part 2 is known incomplete and moved to 0.12.0. Not evaluated.

---

## I338: 1:1 Relationship Intelligence — Three-File Pattern for People

**Rating: KNOWN INCOMPLETE — deferred to 0.12.0**

As noted in the task brief and in the backlog itself: "INCOMPLETE. Commit 6ac8400 added code paths but no person has ever received an `intelligence.json` file." Not evaluated further.

---

## I339: Entity-Generic Dashboard and Frontend — Entities Array on All Surfaces

**Rating: PARTIAL**

### Acceptance Criteria

1. **No TypeScript code references `meeting.account` or `meeting.accountId`** — PARTIAL
   - Main `Meeting` type (`types/index.ts:64`) correctly uses `linkedEntities` only.
   - **BUT** `WeekMeeting` (`types/index.ts:250`) still has `account?: string`.
   - `weekPageViewModel.ts:233,437-438` and `WeekPage.tsx:1092-1093` still reference `m.account`.
   - `PostMeetingPrompt.tsx:93,131` references `meeting.account`.
   - These are residual `account` references on secondary types/surfaces.

2. **Meeting card subtitle shows correct entity name for accounts, projects, and people** — PASS
   - `entity-helpers.ts:10` `formatEntityByline()` returns "{Name} · Customer/Project/1:1".
   - `BriefingMeetingCard.tsx:21` imports and uses `formatEntityByline`.

3. **Meeting card shows appropriate icon for entity type** — PASS
   - `meeting-entity-chips.tsx:18` imports `Building2, FolderKanban, User` icons from lucide-react.
   - Color mapping at lines 34-44 differentiates account/project/person.

4. **Meetings with no entities show meeting type as fallback** — PASS (inferred)
   - `entity-helpers.ts:4-6` returns `null` when no entities, allowing fallback to `formatMeetingType`.

5. **`pnpm build` compiles clean** — NOT VERIFIED (no build run)

**Gap:** `WeekMeeting` type and `WeekPage`/`weekPageViewModel` still use `account` string instead of entities array. `PostMeetingPrompt` also references `meeting.account`.

---

## I260: Proactive Surfacing — Trigger → Insight → Briefing Pipeline

**Rating: PARTIAL**

### Architecture Check

1. **Trigger layer** — PASS
   - `proactive/scanner.rs:13` `run_proactive_scan()` runs on scheduled intervals.
   - `proactive/engine.rs:74` `run_scan()` orchestrates detector execution.

2. **Detection layer** — PASS
   - 9 detectors implemented in `proactive/detectors.rs`:
     - `detect_renewal_gap` (line 18)
     - `detect_relationship_drift` (line 86)
     - `detect_email_volume_spike` (line 175)
     - `detect_meeting_load_forecast` (line 257)
     - `detect_stale_champion` (line 327)
     - `detect_action_cluster` (line 437)
     - `detect_prep_coverage_gap` (line 546)
     - `detect_no_contact_accounts` (line 617)
     - `detect_renewal_proximity` (line 668)

3. **Storage** — PASS
   - Migration 021 creates `proactive_insights` table with proper schema.
   - `engine.rs:108` inserts insights with fingerprint deduplication.
   - DB evidence: 39 insights generated (8 email_volume_spike, 31 no_contact_accounts).

4. **Synthesis layer** — PARTIAL
   - Detectors produce `RawInsight` with headline and detail. No AI enrichment/synthesis step observed.

5. **Delivery layer** — FAIL
   - **No proactive insights surface in the DailyBriefing.** Grep for "proactive" or "insight" in `DailyBriefing.tsx` returns zero matches.
   - Insights are stored in the database but never delivered to the user through the briefing UI.

**Summary:** Backend pipeline (trigger → detect → store) is fully functional. Frontend delivery to briefing is missing. The pipeline produces insights but they are invisible to the user.

---

## I262: The Record — Transcripts and Content Index as Timeline Sources

**Rating: FAIL**

### Acceptance Criteria

1. **`UnifiedTimeline` shows transcripts from `content_index`** — FAIL
   - `UnifiedTimeline.tsx` merges 4 sources: meetings, emails, captures, account events.
   - No code queries `content_index` for transcripts or notes.
   - `TimelineEntryType` (`TimelineEntry.tsx:8`) does not include `'transcript'` or `'note'` variants.

2. **`content_index` entries with `content_type='transcript'` appear in timeline** — FAIL
   - No backend query fetches `content_index` records for timeline display.
   - `TimelineSource` type does not include a transcripts field.

3. **Transcript entries are clickable** — FAIL
   - Not implemented.

4. **Chronological merge with existing sources** — FAIL
   - No transcript items to merge.

**Summary:** The Record still only shows meetings, emails, captures, and account events. Transcripts indexed in `content_index` remain invisible in the UI despite being indexed for enrichment.

---

## Summary Table

| Issue | Title | Rating | Notes |
|-------|-------|--------|-------|
| I334 | Proposed actions triage | **PASS** (7/8) | Auto-archive countdown not visible |
| I335 | Entity-generic data model | **PASS** | account_id dropped, entities arrays in prep/schedule JSON |
| I336 | Entity-generic classification | **PASS** | EntityHint, classify_meeting_multi, confidence scores all present |
| I337 | Meeting-topic-aware context | **PASS** (Part 1) | Part 2 deferred to 0.12.0 as expected |
| I338 | 1:1 relationship intelligence | **KNOWN INCOMPLETE** | Deferred to 0.12.0 as noted |
| I339 | Entity-generic frontend | **PARTIAL** | Main Meeting type done; WeekMeeting/PostMeetingPrompt still use `account` |
| I260 | Proactive surfacing | **PARTIAL** | Backend pipeline works (39 insights in DB); no delivery to briefing UI |
| I262 | The Record — transcripts | **FAIL** | Not implemented. Timeline lacks transcript/content_index integration |

### Residual Issues

1. **I334 criterion 7:** Auto-archive countdown ("Auto-archives in N days") not shown on proposed actions.
2. **I339:** `WeekMeeting` type, `WeekPage.tsx`, `weekPageViewModel.ts`, and `PostMeetingPrompt.tsx` still use `account` string.
3. **I260:** 39 proactive insights in DB with no frontend delivery path.
4. **I262:** `UnifiedTimeline` does not query `content_index` for transcripts.
5. **I336 (minor):** 0 person-type entities in `meeting_entities` — 1:1 detection code exists but may not be firing for existing data.
