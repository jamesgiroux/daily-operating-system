# ENGINEERING DIRECTOR'S TECHNICAL AUDIT
## DailyOS — Comprehensive Architecture & Pipeline Review
### February 20, 2026

---

## EXECUTIVE SUMMARY

This codebase has **ambitious architecture with serious delivery gaps**. The team has built sophisticated infrastructure — Bayesian signal fusion, entity resolution cascades, intelligence lifecycle state machines — but the wiring between systems is incomplete. The app compiles, passes 886 tests, and renders beautiful editorial UI. But when you trace data end-to-end from Google Calendar to what the user actually sees, the pipeline is held together with silent fallbacks, empty arrays, and feature flags that are off by default.

**The core finding:** This product makes promises it can't keep. The meeting prep pipeline — the headline feature — marks meetings as "enriched" after a mechanical row-count, not actual AI enrichment. The signal intelligence system has 17 signal types but only 1 of 15 emission sites triggers propagation rules. Three of six propagation rules listen for signal types that are never emitted anywhere in the codebase.

The frontend TypeScript types describe a richer product than the Rust backend delivers.

---

## SEVERITY MATRIX

| Severity | Count | Category |
|----------|-------|----------|
| **CRITICAL** | 4 | Pipeline breaks where data is promised but not delivered |
| **HIGH** | 8 | Incomplete wiring, dead infrastructure, type mismatches |
| **MEDIUM** | 12 | DRY violations, SRP violations, silent degradation |
| **LOW** | 8 | Missing indexes, extra fields, test hygiene |

---

## 1. THE MEETING PREP PIPELINE IS FAKING IT

This is the product's core value proposition. Here's what's actually happening:

### CRITICAL: Intelligence Lifecycle Lies About State

`intelligence_lifecycle.rs:184` — `generate_meeting_intelligence()` only does a mechanical row-count (entity links, past meetings, signal events). No AI enrichment occurs. But at **line 282**, it sets `intelligence_state = "enriched"`.

The freshness logic in `orchestrate.rs:443` then checks this state to decide whether to re-run. A meeting marked "enriched" with no new signals is **never re-enriched**. The system does busy work (recounting the same DB rows) and reports it as intelligence generation.

**Fix required:** Either implement actual AI enrichment or rename the state to "assessed" so the state machine is honest.

### CRITICAL: First-Run Meetings Get No Intelligence

`orchestrate.rs:447` — `prepare_today()` looks up meetings in `meetings_history` by `calendar_event_id`. But meeting rows are only created by `prepare_week()` or the reconcile workflow. On a fresh install, or when a new meeting appears for the first time, the lookup returns `None` and the intelligence freshness check does nothing. The comment on that line literally says: "No meeting row yet -- nothing to refresh (reconcile creates rows)."

New meetings — the ones the user most needs prep for — fall through silently.

### CRITICAL: Enriched Prep Content Never Updates schedule.json

`deliver.rs:672` — Phase 2 writes `schedule.json` with whatever `prepSummary` is available at that time. Phase 3 runs `enrich_preps()` which updates individual prep JSON files with AI-generated talking points. But there is **no second `deliver_schedule()` call** after enrichment.

Result: The daily briefing page shows pre-enrichment summaries (potentially empty arrays) while the meeting detail page shows enriched content. The user sees different data depending on which page they're on.

### HIGH: Email Intelligence Gated Behind Off-By-Default Flags

`orchestrate.rs:170-251` — Email body access, commitment extraction, auto-archive, and semantic reclassification are all behind feature flags (`emailBodyAccess`, `autoArchiveEnabled`, `semanticEmailReclass`) that default to `false`.

The product philosophy says "AI produces, users consume. No prompts, no maintenance." But most email intelligence requires manual opt-in via Settings. This isn't a bug — it's a broken promise.

---

## 2. THE SIGNAL SYSTEM IS INFRASTRUCTURE WITHOUT DELIVERY

The team built Bayesian signal fusion, Thompson Sampling correction learning, and a six-rule propagation engine. The architecture is sound. But almost none of it reaches the user.

### HIGH: Propagation Engine Is Dead Infrastructure

Only **1 of ~15 signal emission sites** uses `emit_signal_and_propagate()` — Clay enrichment at `clay/enricher.rs:417`. Every other emitter (proactive detectors, email bridge, post-meeting, cadence anomalies) calls plain `emit_signal()`, bypassing propagation entirely.

### HIGH: 3 of 6 Propagation Rules Are Unreachable

| Rule | Listens For | Emitted Anywhere? |
|------|-------------|-------------------|
| `rule_overdue_actions` | `"action_overdue"` | **NO** — detectors emit `"proactive_action_cluster"` |
| `rule_champion_sentiment` | `"negative_sentiment"` | **NO** — nothing emits this |
| `rule_departure_renewal` | `"person_departed"` | **NO** — Clay emits `"company_change"` and `"title_change"` only |

These rules have passing tests (with manually-inserted test signals) but will **never fire** in production.

### HIGH: Prep Invalidation Never Called

`signals/invalidation.rs` — Fully implemented with tests. `check_and_invalidate_preps()` would regenerate stale meeting preps when new signals arrive. But it's never called from anywhere. The propagation engine calls `evaluate_hygiene_actions` but does NOT call this function.

### HIGH: `renewal_at_risk` Signal Is Invisible

`signals/rules.rs:300` emits `"renewal_at_risk"` through propagation. But `"renewal_at_risk"` is NOT in the `CALLOUT_SIGNAL_TYPES` array at `callouts.rs:36-52`. The signal goes into the database and stays there. The user never sees it.

### MEDIUM: `source_context` Field Is Write-Only

The `source_context` field on `SignalEvent` has a DB column, a struct field, and a dedicated insert function. But all three query locations hardcode `source_context: None` when reading back. No caller ever passes context through `emit_signal_with_context`.

---

## 3. FRONTEND-BACKEND TYPE CONTRACT IS BROKEN

### HIGH: `DashboardData` Missing `userDomains`

**Frontend** (`types/index.ts:251`) expects `userDomains?: string[]` on `DashboardData`. **Rust** (`types.rs:931-948`) has no such field. The field exists on `Config` but is never plumbed through.

**Impact:** Attendee domain grouping (internal vs external) silently fails on the dashboard. Every meeting shows all attendees as undifferentiated.

### MEDIUM: `DbAction` Missing `accountName`

**Frontend** (`types/index.ts:149`) expects `accountName?: string`. **Rust** (`db.rs:61-78`) has no `account_name` field — only `account_id`. The SQL query needs a LEFT JOIN to `accounts(name)`.

**Impact:** Action items across the app show no account context.

### MEDIUM: `IntelligenceQuality` Type Lie

**Frontend Meeting type** declares a 3-field subset (`level`, `hasNewSignals`, `lastEnriched`). Rust sends all 8 fields. TypeScript says `signalCount` doesn't exist, but it's there at runtime. Code accessing it would work despite the type system saying it shouldn't.

### MEDIUM: No `GranolaStatus` Frontend Type

Rust has a full `GranolaStatus` struct (`commands.rs:10321`). The frontend has no corresponding interface — presumably consuming it as `any`.

### MEDIUM: 43 Silent Error Swallowing Instances

43 instances of `.catch(() => {})` or equivalent silent error suppression across the frontend. The most critical:

- `useCalendar.ts` — `get_calendar_events` silently fails to empty. The primary data source for the daily view shows nothing with zero indication of failure.
- `WeekPage.tsx` — `get_live_proactive_suggestions` catches errors, sets `_liveError` (underscore-prefixed, intentionally dead state variable). Users never see suggestions failed.
- `WeekPage.tsx` — `get_meeting_timeline` silently falls back to empty array. The entire Timeline chapter disappears with no explanation.
- `DailyBriefing.tsx:229` — `complete_action` is fire-and-forget with `.catch(() => {})`. UI optimistically marks done, backend silently fails, action reappears on next load.
- `RiskBriefingPage.tsx` — `get_risk_briefing` silently sets to `null`. Backend error is indistinguishable from "never generated."
- `useGoogleAuth.ts` — `get_google_auth_status` silently fails. App may think Google is not configured when it is.

---

## 4. DRY VIOLATIONS — CODE COPY-PASTE ACROSS THE CODEBASE

### Entity Sync Logic Tripled

The workspace sync pattern (read_dir, skip hidden, parse JSON, compare timestamps, upsert) is nearly identical across:
- `accounts.rs:574-729` — `sync_accounts_from_workspace`
- `projects.rs:383-500` — `sync_projects_from_workspace`
- `people.rs:410-499` — `sync_people_from_workspace`

This is a trait-based generic begging to be extracted.

### `test_db()` Helper Duplicated 11+ Times

The exact same test setup function (create tempdir, open ActionDb, mem::forget) is copy-pasted across 11 test modules in `signals/`, `proactive/`, `accounts.rs`, `projects.rs`.

### `resolve_entity_name()` Duplicated with Reversed Parameter Order

- `signals/callouts.rs:330` — `resolve_entity_name(db, entity_type, entity_id)`
- `proactive/detectors.rs:239` — `resolve_entity_name(db, entity_id, entity_type)`

Same logic, reversed API. This is how bugs happen.

### Attendee Email Parsing — 4 Implementations

CSV attendee splitting exists in `patterns.rs` (two variants), `email_bridge.rs`, and `post_meeting.rs`. The `patterns.rs` comment even says "same logic as entity_resolver."

### SignalEvent Row-Mapping — 3 Copies

Identical 10-column to struct mapping in `bus.rs`, `callouts.rs`, and `feedback.rs`. All three hardcode `source_context: None`.

---

## 5. SINGLE RESPONSIBILITY VIOLATIONS

### `entity_intel.rs` Is a Monolith

Too large to read in a single pass. Contains: 15 struct definitions, file I/O, JSON path navigation, migration logic, DB cache operations, intelligence context assembly, prompt building, markdown formatting, and content indexing. At least 6 distinct responsibilities.

### `bus.rs` Does 4+ Jobs

Type definitions, source tier configuration, signal emission, meeting propagation, signal querying, weight learning, and raw SQL methods. The ActionDb SQL should be in `db.rs`.

### `callouts.rs` Has a 160-Line Switch Statement

`build_callout_text()` at line 164 knows the internal JSON structure of every signal type. Each new signal type requires touching this function. This should be a trait impl or dispatch table.

### `rules.rs` Mixes Propagation and Hygiene

Lines 18-309 handle cross-entity propagation rules. Lines 362-529 handle data mutations (duplicate merging, name resolution). These are fundamentally different concerns.

---

## 6. DATABASE LAYER — MOSTLY SOLID

The DB layer is the strongest part of the codebase. Well-migrated (31 migrations, forward-compat guard, pre-migration backups, idempotency). Key notes:

### LOW RISK: Dynamic SQL Table Name

`db.rs:2310` — `format!("UPDATE {} SET metadata = ?1 WHERE id = ?2", table)` — validated by a whitelist match, but the pattern is fragile.

### MEDIUM: No FK Constraints on Junction Tables

`meeting_entities`, `meeting_attendees`, `captures`, `entity_people` have no foreign key constraints. Orphan rows accumulate when meetings or entities are deleted. The team wrote manual cascade logic in `backfill_meeting_identity()` to compensate, which is error-prone.

### Schema-Struct Alignment

All tables exist in migrations and are actively used. `DbAccount` and `DbAction` structs align with their schemas, though `DbAction` is missing `needs_decision`, `rejected_at`, `rejection_source` fields (written via raw SQL but never deserialized). Legacy `csm` and `champion` columns remain in `accounts` schema but are never read.

---

## 7. DEAD CODE INVENTORY

| Location | Dead Code | ~Lines |
|----------|-----------|--------|
| `entity_resolver.rs:737` | `resolve_entity_to_account_match` | 100 |
| `meeting_context.rs:727` | `resolve_account_from_db` (test-only) | 150 |
| `signals/bus.rs:92` | `emit_signal_with_context` (never called externally) | 20 |
| `signals/feedback.rs:123` | `get_correction_stats` (never called) | 20 |
| `signals/invalidation.rs:21` | `check_and_invalidate_preps` (never wired in) | 55 |
| `signals/callouts.rs:455` | `get_unsurfaced_callouts` (never called externally) | 30 |
| `signals/propagation.rs:27` | `DerivedSignal.rule_name` field (written, never read) | — |
| `email_context.rs:18` | `_event_id`, `_title` parameters (dead params) | — |
| Duplicated `find_account_dir_by_name` | Two independent copies | 60 |

**~435+ lines of unreachable production code** in the pipeline.

---

## 8. FUNCTIONS WITH TOO MANY PARAMETERS

9 functions exceed 5 parameters, with `emit_signal` variants taking up to 9. The signal emission family should use a builder struct:

```rust
struct SignalEmission {
    entity_type: String,
    entity_id: String,
    signal_type: String,
    source: String,
    value: Option<String>,
    confidence: f64,
    source_context: Option<String>,
}
```

---

## PRIORITIZED REMEDIATION PLAN

### P0 — Ship-Blocking (Do Before Any New Features)

1. **Fix first-run meeting gap** — Add meeting upsert in `prepare_today()` before intelligence check
2. **Re-deliver schedule.json after enrichment** — One function call in the executor
3. **Rename intelligence state** — "enriched" → "assessed" until real AI enrichment exists
4. **Add `userDomains` to `DashboardData`** — Single field, immediate impact

### P1 — Architectural Integrity (Next Sprint)

5. **Wire `emit_signal_and_propagate` into all emitters** or delete the propagation engine
6. **Wire `check_and_invalidate_preps`** into the signal pipeline
7. **Add `renewal_at_risk` to CALLOUT_SIGNAL_TYPES** or delete the rule
8. **Fix/delete the 3 unreachable propagation rules** — emit the signals they listen for, or remove them
9. **Add `accountName` to `DbAction`** via JOIN
10. **Fix silent error swallowing** — at minimum add console.error to the 43 catch-and-swallow sites; surface errors for data-loading calls

### P2 — Code Quality (Ongoing)

11. Extract entity sync trait from account/project/people sync
12. Create shared `test_db()` utility
13. Unify `resolve_entity_name` implementations
14. Unify attendee email parsing
15. Split `entity_intel.rs` into focused modules
16. Add FK constraints to junction tables

### P3 — Hygiene

17. Remove dead code (~435 lines)
18. Add expression indexes for `LOWER(name)` queries
19. Align all frontend types with Rust backend structs
20. Add `SignalEmission` builder struct

---

## VERDICT

The team has built real infrastructure — the signal bus, entity resolution, and migration framework are genuinely well-designed. But they've been building capabilities faster than they're connecting them. The result is a codebase where **the architecture diagram looks complete, but data doesn't flow through it end-to-end**.

The P0 items are straightforward fixes — we're talking about a missing upsert, a missing function call, a field rename, and a missing struct field. The P1 items require deciding what the signal system actually delivers vs. what's aspirational. The hardest conversation is P1: do we wire up the propagation engine or admit it was premature and strip it out?

No new feature work until P0 is clean.

---

## ADDENDUM — Live Testing Findings (2026-02-20 afternoon)

Discovered during hands-on testing of the 0.13.0 implementation with real data.

### Fixed During Testing

| Item | Root Cause | Fix |
|------|-----------|-----|
| `lean_events` stripping attendees before directive write | `orchestrate.rs` had explicit "strip attendees" step that rebuilt events with only id/summary/start/end | Added attendees, names, rsvp, description to lean_events |
| `calendarAttendees` in schedule.json but not on Meeting objects | `JsonMeeting` didn't have the field, `load_schedule_json` didn't map it, `Meeting` Rust struct didn't have it | Added fields to all three: JsonMeeting, Meeting struct, loader mapping |
| Attendee count reading `prep.stakeholders` instead of `calendarAttendees` | Frontend `BriefingMeetingCard` hardcoded `meeting.prep?.stakeholders?.length` | Changed to `meeting.calendarAttendees?.length` with fallback |
| `DashboardData` missing `user_domains` | Rust struct and both construction sites had no field | Added field + populated from config |
| `ensure_meeting_in_history` not storing attendees/description | INSERT and change detection only checked title + start_time | Added attendees + description to insert, update, and change detection |
| Entity resolution: junction not definitive | `resolve_meeting_entities` ran ALL signal sources even with junction links | Junction gate: return immediately when junction entries exist |
| RFC3339 timestamps | `datetime('now')` produced bare timestamps parsed as local time by JS | Switched to `chrono::Utc::now().to_rfc3339()` |

### Systemic Issue: "Compiles Clean" ≠ "Works"

The 0.13.0 implementation was reported as complete after passing `cargo clippy`, `tsc --noEmit`, and 915 Rust tests. But when tested with real data:
- Data existed in schedule.json but was silently dropped at every intermediate deserialization step
- Fields were added to output structs but never to input structs
- The directive pipeline had an explicit "strip attendees" step that was never identified during code review
- Agent-reported work was verified against compilation, not data flow

**Lesson:** Every pipeline change must be verified by inspecting the actual generated files (schedule.json, directive JSON, prep files) with real data. Compilation and type-checking verify structure, not behavior.

### Open — Attendee Hydration from People DB

**Priority:** P1

**Problem:** The Room now correctly shows calendar invitees (not AI-enriched stakeholders), but names are email-prefix-parsed (`james.giroux`, `amy.gerber`) instead of full names from the people database.

**What exists:** `hydrate_attendee_context()` in `commands.rs` already matches attendee emails to person entities for the meeting detail page. This hydration needs to run for schedule meetings too.

**Fix:** In `get_dashboard_data` or `deliver_schedule`, for each meeting's `calendar_attendees`, look up each email in the `people` table. Replace parsed name with DB person's full name, add title, personId, relationship. Unknown attendees keep email-parsed names with an unresolved indicator.

**Trickle-down:** Person-entity links from attendee hydration feed entity resolution — if a person is linked to an account, that strengthens meeting→account association. This is the mechanical gate from the 0.13.0 architecture.

**Acceptance:** Calendar attendees show full names from people DB. Unknown attendees show email-parsed names. The Room groups correctly by internal/external using person relationship data, not just email domain matching.

### Open — Freshness Indicator for Mechanical-Only State

**Priority:** P1

**Problem:** Meetings with calendar data but no AI enrichment show "Sparse" which is ambiguous — doesn't distinguish "no data at all" from "mechanical data present, enrichment pending."

**Fix:** Either add a "Detected" badge state, or enhance "Sparse" tooltip: "Calendar data only — intelligence building." The intelligence lifecycle already tracks this (`intelligence_state = "detected"` vs `"enriched"`), it just needs to surface in the badge.

### Open — AI Enrichment Prompt Scoping

**Priority:** P2

**Problem:** AI enrichment includes all attendee information and synthesizes about any entity it recognizes — Jefferies/Agentforce data appears on Cox meetings because attendees from those companies are present.

**Fix:** Scoping instruction in the AI prompt: "This meeting is about {entity_name}. Only surface intelligence relevant to this entity." File: wherever the AI prompt template is built in the enrichment pipeline.

### Open — BU-Aware Entity Resolution

**Priority:** P2

**Problem:** Attendee → Person → BU → Account chain isn't used as an entity resolution signal.

**Fix:** In the entity resolver's inference step, when an attendee maps to a person with a BU link, add the BU's parent account as a 0.85 confidence signal.

### Open — Multi-Entity People (CSMs) Hypothesis Reinforcement

**Priority:** P3

**Problem:** Without a manual junction link, a CSM linked to 5 accounts still introduces 5 competing entity candidates.

**Fix:** Hypothesis-and-reinforce model: form hypothesis from strongest signal, then multi-entity attendees reinforce if linked to the hypothesis, stay neutral otherwise.
