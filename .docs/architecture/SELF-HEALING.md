# Self-Healing Architecture

> Complete reference for DailyOS's automatic data repair, quality maintenance, and proactive intelligence systems.
>
> Last updated: 2026-03-04 | Covers: v0.16.0

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [System Overview](#2-system-overview)
3. [Hygiene Scanner](#3-hygiene-scanner-hygienrs)
4. [Self-Healing Intelligence Module](#4-self-healing-intelligence-module)
5. [Proactive Detection Engine](#5-proactive-detection-engine)
6. [Deduplication Strategy](#6-deduplication-strategy)
7. [Permission Model](#7-permission-model-autonomous-vs-user-gated)
8. [Signal Emission Audit](#8-signal-emission-audit)
9. [Known Gaps](#9-known-gaps)
10. [Relationship to Other Architecture Docs](#10-relationship-to-other-architecture-docs)

---

## 1. Design Principles

### The Chief of Staff Model

DailyOS self-heals like a chief of staff who fixes problems before the executive notices. The system:

- **Fixes what it can mechanically** — no AI cost for deterministic repairs (relationship classification, meeting count reconciliation, renewal rollovers)
- **Uses AI budget wisely** — proactive enrichment is budget-gated and priority-scored, not spray-and-pray
- **Never overwrites user decisions** — user-set data (source priority 4) is sacred. Self-healing only fills gaps and fixes machine-generated errors
- **Degrades gracefully** — if the AI budget is exhausted, embedding model fails, or PTY is unavailable, mechanical fixes still run
- **Operates invisibly** — the user should notice better data, not the repair process

### The Permission Hierarchy

Every self-healing action falls into one of three permission levels:

| Level | Actions | Gate |
|-------|---------|------|
| **Autonomous** | Mechanical fixes, relationship classification, meeting count reconciliation, renewal rollover, file summary extraction, name resolution from calendar | Always runs. No user visibility needed. |
| **Confident Autonomous** | Auto-merge duplicates (>= 0.95 confidence), co-attendance linking (3+ meetings), phantom account archival, empty shell archival | Runs automatically but logged. Higher confidence threshold required. |
| **Budget-Gated** | AI re-enrichment, gap-targeted enrichment, coherence repair | Runs only when `HygieneBudget` allows. Budget resets daily. |

---

## 2. System Overview

Three subsystems work together, triggered by the scheduler and signal bus:

```
Scheduler (60s poll loop)
    |
    +-- Hygiene Scanner (every 4h, configurable)
    |       |
    |       +-- Phase 0: Gap detection (counting)
    |       +-- Phase 1: Mechanical fixes (free)
    |       +-- Phase 1b: Account cleanup
    |       +-- Phase 2: Email/domain resolution
    |       +-- Phase 2b: Low-confidence match detection
    |       +-- Phase 2c: Attendee pattern mining
    |       +-- Phase 2d: Self-healing intelligence (dedup, co-attendance, names)
    |       +-- Phase 2e: Email cadence monitoring
    |       +-- Phase 3: AI-budgeted gap filling (self_healing::evaluate_portfolio)
    |
    +-- Pre-meeting readiness (every 30min)
    |       |
    |       +-- check_pre_meeting_refresh (scheduler.rs)
    |       +-- check_upcoming_meeting_readiness (hygiene.rs, post-calendar-poll)
    |
    +-- Proactive Detection Engine (piggyback on hygiene + pre-briefing)
            |
            +-- 9 detectors scanning for patterns
            +-- Deduped by fingerprint (7-day window)
            +-- Emit signals into the bus

Signal Bus
    |
    +-- evaluate_on_signal() — if trigger_score > 0.7, enqueue re-enrichment
    +-- propagation rules — cross-entity intel re-enrichment
    +-- prep invalidation — queued for next scheduler drain
```

### Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/hygiene.rs` | Hygiene scanner: all phases, gap detection, mechanical fixes |
| `src-tauri/src/self_healing/mod.rs` | Portfolio evaluation entry point |
| `src-tauri/src/self_healing/quality.rs` | Beta distribution quality scoring (entity_quality table) |
| `src-tauri/src/self_healing/remediation.rs` | Continuous priority scoring for enrichment queue |
| `src-tauri/src/self_healing/detector.rs` | Coherence check via embedding similarity |
| `src-tauri/src/self_healing/scheduler.rs` | Circuit breaker state machine, signal-driven evaluation |
| `src-tauri/src/self_healing/feedback.rs` | User correction → quality score + source weight updates |
| `src-tauri/src/proactive/engine.rs` | Proactive detection engine |
| `src-tauri/src/proactive/detectors.rs` | Individual detector implementations |
| `src-tauri/src/proactive/scanner.rs` | Scanner orchestration |
| `src-tauri/src/scheduler.rs` | Scheduler loop, prep drain, pre-meeting refresh |

---

## 3. Hygiene Scanner (`hygiene.rs`)

### Trigger and Scheduling

| Parameter | Value | Notes |
|-----------|-------|-------|
| Startup delay | 30 seconds | Let app initialize before scanning |
| Default interval | 4 hours | Configurable: 1, 2, 4, 8 hours |
| Overnight mode | 2-3 AM | 2x AI budget for deeper repair |
| Manual trigger | `run_hygiene_scan_now` command | UI can trigger immediately |
| Overlap prevention | Atomic `scan_running` flag | Prevents concurrent manual + background |
| Heavy work gate | `heavy_work_semaphore.try_acquire()` | Defers if PTY/embedding running |

### Phase 0: Gap Detection

Counts problems before any fixes run. Populates `HygieneReport` for UI display.

| Gap Type | Detection Query | What It Means |
|----------|----------------|---------------|
| `unnamed_people` | People with no space in name | Likely email-only, never resolved to a real name |
| `unknown_relationships` | `relationship = "unknown"` | Not classified as internal or external |
| `missing_intelligence` | No row in `entity_intelligence` | Entity exists but was never enriched |
| `stale_intelligence` | Not enriched in 14+ days | Intelligence is aging |
| `unsummarized_files` | `content_index` rows with no `extracted_at` | Files ingested but never summarized |
| `duplicate_people` | Name similarity within email domain groups | Potential duplicates to merge |
| `abandoned_quill_syncs` | Quill sync state = "abandoned" | Transcript sync gave up |
| `empty_shell_accounts` | No meetings/actions/people after 30+ days | Account was created but never used |

### Phase 1: Mechanical Fixes (Free, Instant)

All deterministic. No AI cost. No user interaction needed.

#### `fix_unknown_relationships()`
- **Detects:** People with `relationship = "unknown"`
- **Fixes:** Re-classifies as "internal" or "external" by matching email domain against user domains
- **Tables:** `people`
- **Signals:** None

#### `backfill_file_summaries()`
- **Detects:** `content_index` rows where `extracted_at IS NULL`
- **Fixes:** Reads files from disk, calls `intelligence::extract_and_summarize()`, writes summary. Marks deleted/failed files so they never re-enter the gap pool.
- **Batch cap:** 50 files per scan
- **Tables:** `content_index`
- **Signals:** None

#### `fix_meeting_counts()`
- **Detects:** `person.meeting_count` mismatches vs actual `COUNT(*) FROM meeting_attendees`
- **Fixes:** Recomputes via `recompute_person_meeting_count()`
- **Tables:** `people`
- **Signals:** None

#### `fix_renewal_rollovers()`
- **Detects:** Non-archived accounts with `contract_end` in the past and no churn event
- **Fixes:** Records implicit "renewal" event, advances `contract_end` by 12 months
- **Tables:** `accounts`, `account_events`
- **Signals:** None
- **Assumption:** If no churn event exists, the customer renewed. This is a reasonable default for CS-managed accounts.

#### `retry_abandoned_quill_syncs()`
- **Detects:** Quill sync rows with state "abandoned" between 7 and 14 days old
- **Fixes:** Resets to pending/retry state
- **Tables:** `quill_syncs`
- **Signals:** None

### Phase 1b: Account Cleanup

#### `archive_phantom_accounts()`
- **Detects:** Accounts named "Internal" with wrong `account_type` and zero activity
- **Fixes:** Sets `archived = 1`
- **Tables:** `accounts`
- **Permission:** Confident Autonomous (zero activity = safe to archive)

#### `relink_orphan_internal_accounts()`
- **Detects:** Internal accounts with `parent_id IS NULL` that aren't the root
- **Fixes:** Sets `parent_id` to root internal account
- **Tables:** `accounts`

#### `archive_empty_shell_accounts()`
- **Detects:** Non-archived accounts updated 30+ days ago with no meetings, actions, people, events, or email signals
- **Fixes:** Sets `archived = 1`
- **Tables:** `accounts`
- **Permission:** Confident Autonomous (30-day grace + zero activity)

### Phase 2: Email & Domain Resolution

#### `resolve_names_from_emails()`
- **Detects:** People in `get_unnamed_people()` whose emails appear in `_today/data/emails.json`
- **Fixes:** Extracts display name from `From:` header, updates `people.name`
- **Dependency:** Requires `emails.json` on disk from that day's briefing pipeline. If briefing hasn't run, no names resolve.
- **Tables:** `people`
- **Signals:** None
- **Known gap:** Depends on filesystem data (`emails.json`). Post-I513, this should read from the DB's email tables instead.

#### `auto_link_people_by_domain()`
- **Detects:** External people with no `entity_people` link whose email domain hints match an account name
- **Fixes:** Creates link via `link_person_to_entity()` with relationship "associated"
- **Tables:** `entity_people`
- **Signals:** None
- **Confidence:** Heuristic (domain hint match, first word, 3+ chars). Low precision but useful for bootstrapping.

#### `dedup_people_by_domain_alias()`
- **Detects:** People with same local-part email across sibling domains (e.g., `renan@wpvip.com` and `renan@a8c.com`)
- **Fixes:** Calls `merge_people()` — higher meeting count wins, transfers all references
- **Tables:** `people` and all FK-referencing tables
- **Signals:** None
- **Guard:** Only processes domains with siblings in `account_domains`

### Phase 2b-2e: Pattern Detection & Self-Healing Intelligence

#### `detect_low_confidence_matches()` (I305)
- **Detects:** Meeting-to-entity matches with confidence below threshold
- **Fixes:** Creates `entity_suggestions` for user review (NOT auto-applied)
- **Permission:** User-gated (creates suggestions, doesn't act)

#### `signals::patterns::mine_attendee_patterns()` (I307)
- **Detects:** Recurring attendee groups across meetings
- **Fixes:** Updates pattern tables for future signal generation

#### `fix_auto_merge_duplicates()` (I342)
- **Detects:** Duplicate people with confidence >= 0.95 from `detect_duplicate_people()`
- **Fixes:** Merges up to 10 pairs per scan via `merge_people()`
- **Tables:** `people` and all referencing tables
- **Signals:** Yes — emits `auto_merged` signal with source "hygiene"
- **BUG:** Uses `emit()` not `emit_and_propagate()` — no downstream propagation, prep invalidation, or re-enrichment triggered

#### `resolve_names_from_calendar()` (I342)
- **Detects:** People without a space in their name whose email exists in `attendee_display_names` with a proper display name
- **Fixes:** Updates `people.name` from calendar data
- **Tables:** `people`
- **Signals:** None

#### `fix_co_attendance_links()` (I342)
- **Detects:** Person+account pairs where the person attended 3+ meetings linked to that account but has no `entity_people` link
- **Fixes:** Creates link with relationship "co-attendee"
- **Confidence scaling:** 0.75 (3-4 meetings), 0.85 (5-9), 0.95 (10+)
- **Tables:** `entity_people`
- **Signals:** Yes — emits `account_linked` signal with source "hygiene"
- **BUG:** Uses `emit()` not `emit_and_propagate()` — no downstream propagation

#### `signals::cadence::compute_and_emit_cadence_anomalies()` (I319)
- **Detects:** Entities where current week's email count is <50% (gone_quiet) or >200% (activity_spike) of 30-day rolling average
- **Fixes:** Upserts `entity_email_cadence` table
- **Signals:** Yes — emits `cadence_anomaly` signals. These flow through the signal bus and can propagate.

### Phase 3: AI-Budgeted Gap Filling

#### `self_healing::evaluate_portfolio()`
- **Process:** Scans all non-blocked entities, computes trigger scores, enqueues top candidates to `intel_queue` at `ProactiveHygiene` priority
- **Budget gate:** `HygieneBudget::try_consume()` — stops when daily budget exhausted
- **Circuit breaker:** Skips coherence-blocked entities
- **Overnight mode:** 2x budget between 2-3 AM

### Pre-Meeting Readiness (Not Part of Hygiene Loop)

Two overlapping mechanisms ensure meetings have fresh intelligence:

| Mechanism | Location | Frequency | Window | Threshold |
|-----------|----------|-----------|--------|-----------|
| `check_pre_meeting_refresh()` | `scheduler.rs` | Every 30 min | 2 hours before meeting | `has_new_signals = 1` OR `last_enriched_at IS NULL` OR stale > 12h |
| `check_upcoming_meeting_readiness()` | `hygiene.rs` | After calendar polls | `hygiene_pre_meeting_hours` (default 12h) | Trigger score >= 0.4 |

These overlap by design — the scheduler catches the tight window, hygiene catches the broader window. Both enqueue to `intel_queue`.

---

## 4. Self-Healing Intelligence Module

### Quality Scoring (`quality.rs`)

Every entity has a `Beta(alpha, beta)` distribution in `entity_quality`:

```
quality_score = alpha / (alpha + beta)
```

- Starting prior: `Beta(1, 1)` = score 0.5 (neutral)
- Each successful enrichment: `alpha += 1` (score rises)
- Each user correction: `beta += 1` (score drops)
- Score of 0.45 = "low quality" threshold for gap detection

### Enrichment Priority Scoring (`remediation.rs`)

Replaces the old binary "14-day stale" check with a continuous weighted score:

```
trigger_score = imminence × 0.35
              + staleness × 0.25
              + quality_deficit × 0.20
              + importance × 0.10
              + signal_delta × 0.10
```

| Dimension | Computation |
|-----------|-------------|
| `imminence` | 1.0 if meeting <24h, 0.5 if <7d, 0.1 if >7d, 0.0 if none |
| `staleness` | days_since_enrichment / 14, capped at 1.0. NULL = 1.0 |
| `quality_deficit` | 1.0 - quality_score |
| `importance` | meeting_count_90d / 10, capped at 1.0 |
| `signal_delta` | signals_since_last_enrichment / 10, capped at 1.0 |

Entities scoring >= 0.25 are candidates for re-enrichment. The list is sorted descending and budget-gated.

### Coherence Detection (`detector.rs`)

After every successful enrichment:

1. Fetch entity's `executive_assessment` from `entity_intelligence`
2. Fetch last 20 meetings (90-day window) for that entity
3. Embed both using local embedding model
4. Compute cosine similarity (threshold: 0.30)
5. If similarity < 0.30: intelligence is incoherent with meeting history

Graceful degradation: if <2 meetings or no embedding model, returns score 1.0 (passes).

### Circuit Breaker (`scheduler.rs`)

Prevents infinite re-enrichment loops for consistently failing entities:

```
State Machine (entity_quality columns):

  First coherence failure
    → start 24h window, retry_count = 1, re-enqueue

  Each failure within 24h
    → increment retry_count, re-enqueue

  3 failures within 24h
    → TRIP: coherence_blocked = 1
    → Entity skipped in evaluate_portfolio()

  After 72h
    → AUTO-EXPIRE: reset state, allow one more retry

  Coherence passes after retry
    → CLEAR: reset all state
```

### Signal-Driven Evaluation (`scheduler.rs::evaluate_on_signal`)

When any signal fires for an entity:
- If `trigger_score > 0.7` AND entity not circuit-broken → enqueue at `ContentChange` priority
- This is faster than waiting for the 4-hour hygiene cycle

### Feedback Loop (`feedback.rs`)

User corrections close the loop:

```
User edits enriched field
  → record_enrichment_correction()
  → increment_beta() on entity_quality (lowers quality score)
  → upsert_signal_weight(source, entity_type, "enrichment_quality", alpha=0, beta=1)
  → Source penalized in Thompson Sampling weights
```

```
Enrichment completes successfully
  → record_enrichment_success()
  → increment_alpha() on entity_quality (raises quality score)
  → update last_enrichment_at
```

---

## 5. Proactive Detection Engine

### Architecture

`ProactiveEngine` runs 9 detectors that scan for patterns and emit signals. Runs piggyback on hygiene timing and pre-briefing preparation.

### Deduplication

Each detector produces a fingerprint (SHA-256 of detector name + entity + key fields). If the same fingerprint was emitted within 7 days, the insight is suppressed. Prevents alert fatigue.

### Detectors

| Detector | Profiles | What It Detects | Signal Emitted |
|----------|----------|-----------------|----------------|
| `detect_renewal_gap` | cs, executive | Account with renewal <= 60d + no meeting in 30d | `renewal_gap` |
| `detect_relationship_drift` | all | Person where 30d meeting rate < 50% of 90d average | `relationship_drift` |
| `detect_email_volume_spike` | all | Email volume > 200% of average for entity | `email_volume_spike` |
| `detect_meeting_load_forecast` | all | Upcoming week has significantly more meetings | `meeting_load_high` |
| `detect_stale_champion` | cs, executive | Champion with no meeting in 45+ days | `stale_champion` |
| `detect_action_cluster` | all | 5+ open actions for same entity | `proactive_action_cluster` |
| `detect_prep_coverage_gap` | all | Tomorrow's meetings with no prep | `prep_coverage_gap` |
| `detect_no_contact_accounts` | all | Active account with 0 meetings in 60 days | `no_contact_account` |
| `detect_renewal_proximity` | cs, sales, partnerships, executive | Renewal within 30/60/90 day thresholds | `renewal_proximity` |

### Signal Flow

Detector signals flow into the bus via `emit_signal()` and are stored in `proactive_insights`. They feed:
- The daily briefing (surfaced as "things to watch")
- Propagation rules (e.g., `rule_renewal_engagement_compound` consumes `renewal_proximity`)
- Self-healing trigger evaluation

---

## 6. Deduplication Strategy

Three separate dedup mechanisms with different scopes and confidence levels:

### Layer 1: Detection (`detect_duplicate_people`)

SQL-based name similarity within email domain groups. Returns candidate pairs with confidence scores. Used for both gap counting (Phase 0) and auto-merge (Phase 2d).

### Layer 2: Auto-Merge (`fix_auto_merge_duplicates`)

- **Threshold:** >= 0.95 confidence
- **Batch cap:** 10 merges per scan (prevents runaway merges)
- **Merge strategy:** Higher `meeting_count` wins as the surviving record
- **What transfers:** All FK references — `meeting_attendees`, `entity_people`, `person_relationships`, `signal_events`, `email_signals`
- **Permission:** Confident Autonomous (0.95 is very high — same name + same domain)

### Layer 3: Domain Alias Dedup (`dedup_people_by_domain_alias`)

- **Scope:** Same local-part email across sibling domains in `account_domains`
- **Example:** `renan@wpvip.com` + `renan@a8c.com` where both domains belong to the same account
- **Guard:** Only processes domains with registered siblings
- **Permission:** Autonomous (email identity match is deterministic)

### What's NOT Deduped

- **Accounts** — no account dedup exists. I198 (Account Merge) is user-initiated only.
- **Cross-domain people** — `john@acme.com` and `john.smith@acme.com` are not matched (different local parts)
- **Projects** — no project dedup exists

---

## 7. Permission Model: Autonomous vs User-Gated

### Autonomous Actions (System Acts Without Asking)

| Action | Confidence Basis | Reversible? |
|--------|-----------------|-------------|
| Classify unknown relationships (internal/external) | Email domain match against user domains | Yes (user can re-classify) |
| Recompute meeting counts | Deterministic COUNT(*) | Yes (re-runs on next scan) |
| Advance expired renewals by 12 months | No churn event = implicit renewal | Yes (user can edit date) |
| Resolve names from calendar display names | Email match in attendee_display_names | Yes (user can rename) |
| Resolve names from email From: headers | Email match in emails.json | Yes (user can rename) |
| Backfill file summaries | File exists in content_index | Yes (re-extractable) |
| Retry abandoned Quill syncs (7-14 days old) | Age-gated | Yes (will re-abandon if still failing) |
| Relink orphan internal accounts to root | Parent is null, type is internal | Yes (user can reparent) |
| Link people to accounts by domain hint | Email domain matches account name | Yes (user can unlink) |
| Link people to accounts by co-attendance (3+ meetings) | Meeting evidence | Yes (user can unlink) |
| Dedup people by domain alias | Same local-part, sibling domains | No (merge is destructive) |

### Confident Autonomous Actions (System Acts at High Confidence, Logged)

| Action | Confidence Threshold | Logging |
|--------|---------------------|---------|
| Auto-merge duplicate people | >= 0.95 | `auto_merged` signal emitted |
| Archive phantom "Internal" accounts | Zero activity | Archived flag set |
| Archive empty shell accounts | 30+ days, zero activity | Archived flag set |

### Budget-Gated Actions (System Acts If Budget Allows)

| Action | Budget Control | Priority |
|--------|---------------|----------|
| AI re-enrichment (proactive) | `HygieneBudget::try_consume()` | Trigger score determines order |
| AI re-enrichment (signal-driven) | Trigger score > 0.7 | `ContentChange` priority |
| AI re-enrichment (pre-meeting) | Trigger score >= 0.4 | `CalendarChange` priority |

### User-Gated Actions (System Suggests, User Decides)

| Action | How Surfaced |
|--------|-------------|
| Low-confidence entity match review | `entity_suggestions` table, surfaced in UI |
| Coherence-flagged entity review | `coherence_flagged` in entity_intelligence, visible in UI |

---

## 8. Signal Emission Audit

### What Emits Signals Correctly

| Mechanism | Signal | Emission Type | Propagation? |
|-----------|--------|--------------|-------------|
| Email cadence anomaly | `cadence_anomaly` | `emit_signal()` via bus | Yes (bus-level) |
| Proactive detectors | Various (`renewal_gap`, etc.) | `emit_signal()` via bus | Yes (bus-level) |
| Person enrichment (Gravatar) | `profile_discovered` | `emit_signal_and_propagate()` | Yes |
| User corrections | `user_correction` | `emit_signal_and_propagate()` | Yes |
| Transcript outcomes | `transcript_outcomes` | `emit_signal_and_propagate()` | Yes |
| Post-meeting email correlation | `pre_meeting_context` | `emit_signal_and_propagate()` | Yes |

### What Emits Signals WITHOUT Propagation (BUGS)

| Mechanism | Signal | Issue |
|-----------|--------|-------|
| `fix_auto_merge_duplicates()` | `auto_merged` | Uses `emit()` not `emit_and_propagate()`. Merged person's linked accounts don't get prep invalidation or re-enrichment. |
| `fix_co_attendance_links()` | `account_linked` | Uses `emit()` not `emit_and_propagate()`. New person-account link doesn't trigger account re-enrichment or prep invalidation. |

### What Emits NO Signals (Gaps)

| Mechanism | What Happens | What Should Happen |
|-----------|-------------|-------------------|
| `fix_unknown_relationships()` | Reclassifies internal/external | Should emit signal so entity resolution can re-evaluate |
| `resolve_names_from_calendar()` | Updates person name | Should emit `profile_discovered` so meeting preps update |
| `resolve_names_from_emails()` | Updates person name | Should emit `profile_discovered` |
| `auto_link_people_by_domain()` | Creates entity_people link | Should emit `account_linked` with propagation |
| `dedup_people_by_domain_alias()` | Merges people | Should emit `auto_merged` with propagation |
| `fix_renewal_rollovers()` | Advances renewal date | Should emit signal so renewal-related reports invalidate |
| `archive_empty_shell_accounts()` | Archives account | Could emit signal for audit trail |

---

## 9. Known Gaps

### 9.1 Signal emission inconsistency

The two bugs and seven missing-signal gaps listed in §8 mean that hygiene repairs don't ripple through the system. An auto-merge that combines two stakeholder records doesn't update the account's meeting prep or intelligence. A renewal rollover doesn't invalidate the Renewal Readiness report. This is the highest-priority fix.

### 9.2 `resolve_names_from_emails()` depends on filesystem

This function reads `_today/data/emails.json` from disk. Post-I513 (DB as sole source), this should read from the DB's email tables. If the briefing hasn't run that day, no email-based name resolution happens at all.

### 9.3 No Glean-powered gap filling

`semantic_gap_query()` in `intelligence/prompts.rs` generates search terms for missing intelligence (empty risks, empty wins). These terms are used in the enrichment prompt to guide the LLM, but they're never sent to Glean as proactive searches. An account with empty competitive context should trigger a Glean search for competitor mentions — today it just hopes the next enrichment cycle finds something.

### 9.4 Overnight scan writes to unmaintained file

`run_overnight_scan` writes `maintenance.json` for morning briefing reference. There is no evidence the briefing pipeline reads this file. The overnight scan's deeper repair work happens (2x budget), but the user is never informed of what was fixed.

### 9.5 Proactive detectors don't trigger Glean searches

The proactive engine detects patterns like `stale_champion` (champion not seen in 45+ days) or `no_contact_account` (no meetings in 60 days). These insights emit signals but don't trigger any proactive data gathering. In Glean mode, the system could query Glean for org changes, departures, or recent documents that explain the gap.

### 9.6 No self-healing for reports

Reports become stale when intelligence updates (`mark_reports_stale`), but they're never auto-regenerated. The user must manually click "Regenerate." For scheduled reports (Weekly Impact, Monthly Wrapped), auto-generation exists via the scheduler — but for account-scoped reports (SWOT, Account Health, EBR/QBR), staleness is only flagged, never resolved.

### 9.7 Hygiene report not surfaced well

`HygieneReport` is generated after every scan with gap counts and fix results. It's emitted as a Tauri event but there's no dedicated UI surface showing the user what the system fixed. The chief of staff model says this should be invisible, but for trust-building, a "system health" indicator showing "12 data quality issues fixed overnight" would be valuable.

### 9.8 entity_quality initialization race

`initialize_quality_scores()` runs at the start of `evaluate_portfolio()`. If new entities are created between initialization and the priority scan, they won't have quality rows. Not a critical bug (they'll be initialized next cycle) but a minor gap.

---

## 10. Relationship to Other Architecture Docs

| Document | What It Covers | How Self-Healing Relates |
|----------|---------------|------------------------|
| `PIPELINES.md` §6 | Background Schedulers (scheduler loop, self-healing subsystem, proactive engine) | Self-healing runs within the scheduler pipeline. §6 covers the execution model; this doc covers the logic. |
| `DATA-FLOWS.md` §4 | User Action → Signal → Propagation flow | Signal-driven self-healing (`evaluate_on_signal`) is a branch of this flow. |
| `LIFECYCLES.md` §4 | Intelligence lifecycle (coherence check, circuit breaker) | The intelligence lifecycle is the primary consumer of self-healing decisions. |
| `LIFECYCLES.md` §2 | Person lifecycle (enrichment flow) | Person enrichment feeds quality scoring and triggers hygiene repairs. |
| `DATA-MODEL.md` | Table definitions | `entity_quality`, `signal_weights`, `proactive_insights`, `proactive_scan_state` are the self-healing tables. |
