# DailyOS Async Pipeline Reference

> Comprehensive trace of every background/async pipeline in the DailyOS Tauri app.
> Each pipeline documents: flow diagram, DB tables touched, error handling quality,
> concurrency model, and testing gaps.
>
> Last updated: 2026-03-02

---

## Table of Contents

1. [Pipeline 1: Intelligence Enrichment](#pipeline-1-intelligence-enrichment)
2. [Pipeline 2: Signal Bus](#pipeline-2-signal-bus)
3. [Pipeline 3: Meeting Prep](#pipeline-3-meeting-prep)
4. [Pipeline 4: Transcript Processing](#pipeline-4-transcript-processing)
5. [Pipeline 5: Report Generation](#pipeline-5-report-generation)
6. [Pipeline 6: Background Schedulers](#pipeline-6-background-schedulers)
7. [Pipeline 7: Google API Integration](#pipeline-7-google-api-integration)

---

## Pipeline 1: Intelligence Enrichment

**Location**: `src-tauri/src/intel_queue.rs`, `src-tauri/src/intelligence/`

### Flow Diagram

```
Trigger (signal, manual, calendar, hygiene)
    |
    v
IntelligenceQueue::enqueue()          -- intel_queue.rs:97
    |  [debounce: 30s for ContentChange/ProactiveHygiene]
    |  [dedup: same entity_id => keep higher priority]
    v
run_intel_processor() loop             -- intel_queue.rs:257
    |  [polls every 5s, adaptive sleep, wake on Notify]
    |  [dev mode check: skip if dev sandbox active]
    v
Phase 0: dequeue_batch(adaptive_size)  -- intel_queue.rs:290
    |  [Active=1, Idle=2, Background=3 batch size]
    |  [TTL check: skip if enriched within 7200s unless Manual]
    v
Phase 1: gather_enrichment_input()     -- intel_queue.rs:613
    |  [opens OWN DB connection (split-lock pattern)]
    |  [reads entity row, workspace files, builds prompt]
    |  [reads existing intelligence.json, source manifest]
    v
Phase 2: run_enrichment() via PTY      -- intel_queue.rs:373
    |  [acquires heavy_work_semaphore (limits to 1 concurrent)]
    |  [spawns Claude Code subprocess, 30-120s]
    |  [ModelTier::Synthesis, nice priority 5]
    |  [single entity: direct call; multi: batch with delimiters]
    v
parse_intelligence_response()          -- intelligence/mod.rs
    |  [validates JSON structure, IntelligenceJson]
    |  [I470: retry up to 2x on validation failure]
    v
Phase 3: write_enrichment_results()    -- intel_queue.rs:494
    |  [writes intelligence.json to entity dir on disk]
    |  [updates entity_intelligence row in DB]
    |  [marks reports stale via reports/invalidation.rs]
    v
Phase 4: Post-enrichment                -- intel_queue.rs:503
    |  [emits "intelligence-updated" Tauri event]
    |  [invalidates + requeues meeting preps for linked meetings]
    |  [self-healing: coherence check (I409/I410)]
    |  [audit log: entity_enrichment_completed]
    v
Done
```

### Priority Levels

| Priority | Value | Trigger |
|----------|-------|---------|
| `ProactiveHygiene` | 0 | Self-healing portfolio sweep, budget-gated |
| `ContentChange` | 1 | Workspace file changes in entity directory |
| `CalendarChange` | 2 | Calendar changes affecting entity meetings |
| `Manual` | 3 | User clicks "Refresh Intelligence" |

### Data Touched

| Step | Table/File | Read/Write |
|------|-----------|------------|
| gather_enrichment_input | `accounts`, `projects`, `people` | Read |
| gather_enrichment_input | `entity_intelligence` | Read |
| gather_enrichment_input | `intelligence.json` (disk) | Read |
| gather_enrichment_input | `content_files` | Read |
| run_enrichment | PTY subprocess (Claude Code) | External |
| write_enrichment_results | `intelligence.json` (disk) | Write |
| write_enrichment_results | `entity_intelligence` | Write |
| write_enrichment_results | `reports` | Write (mark stale) |
| post-enrichment | `meetings_history` | Write (prep invalidation) |
| post-enrichment | `entity_quality` | Write (coherence check) |
| post-enrichment | `audit_log` (disk) | Write |

### Error Handling

| Step | Failure Mode | Handling | Tag |
|------|-------------|----------|-----|
| DB lock poisoned | Mutex poisoned | Returns early from gather | `[HANDLED]` |
| Entity not found | No row in accounts/projects/people | Logs warning, skips entity | `[LOGGED ONLY]` |
| PTY spawn failure | Claude Code not installed or crashes | Logs warning, entity tracked for retry | `[HANDLED]` |
| PTY timeout | AI takes >120s | Categorized as "timeout", retried up to 2x | `[HANDLED]` |
| Parse failure | AI returns invalid JSON | Validation retry (I470), up to MAX_VALIDATION_RETRIES=2 | `[HANDLED]` |
| Write failure | Disk write or DB write fails | Logs warning, skips to next entity | `[LOGGED ONLY]` |
| Coherence check | Embedding model unavailable | Gracefully passes with score 1.0 | `[HANDLED]` |
| heavy_work_semaphore closed | Semaphore dropped | Processor loop exits entirely | `[HANDLED]` |

### Concurrency

- **Self-concurrent**: No. Single processor loop. Queue is `Mutex<VecDeque>`.
- **With other pipelines**: `heavy_work_semaphore` (permits=1) serializes PTY calls across intel_queue and other heavy operations.
- **DB access**: Split-lock pattern. Opens own `ActionDb::open()` connection for reads, brief `state.db.lock()` for writes. UI stays responsive during 30-120s PTY.
- **Batch processing**: Adaptive batch size (1-3) based on user activity level.

### Testing

- **Queue operations**: Tested (enqueue, dequeue, dedup, priority ordering, batch dequeue). `[TESTED]`
- **TTL check**: `enrichment_age_check()` tested. `[TESTED]`
- **Full processor loop**: Not unit-tested (requires AppState + PTY). `[UNTESTED]`
- **Write results + side effects**: Not unit-tested. `[UNTESTED]`
- **Retry logic (I470)**: Not directly tested (requires mock PTY failure). `[UNTESTED]`

---

## Pipeline 2: Signal Bus

**Location**: `src-tauri/src/signals/` (bus.rs, propagation.rs, decay.rs, feedback.rs, fusion.rs, invalidation.rs, rules.rs, sampling.rs)

### Flow Diagram

```
Event source (transcript, calendar, email, user correction, proactive detector)
    |
    v
emit_signal()                           -- bus.rs:139
    |  [inserts into signal_events table]
    |  [flags future meetings with has_new_signals=1]
    v
    +-- emit_signal_and_propagate()      -- bus.rs:180
    |       |
    |       v
    |   PropagationEngine::propagate()   -- propagation.rs:77
    |       |  [evaluates all registered rules]
    |       |  [inserts derived signals + signal_derivations]
    |       |  [cross-entity targets: enqueues intel enrichment at ProactiveHygiene]
    |       |  [evaluates hygiene actions (rules.rs)]
    |       |  [checks prep invalidation (invalidation.rs)]
    |       v
    |   Derived signals emitted
    |
    +-- emit_signal_propagate_and_evaluate()  -- bus.rs:231
            |  [= emit_signal_and_propagate + self-healing evaluation]
            |  [calls evaluate_on_signal() if trigger_score > 0.7]
            v
        Entity re-enrichment enqueued (IntelligenceQueue)
```

### Signal Tier Weights (ADR-0080)

| Tier | Sources | Base Weight | Half-Life |
|------|---------|-------------|-----------|
| 1 (highest) | `user_correction`, `explicit` | 1.0 | 365 days |
| 1 | `transcript`, `notes` | 0.9 | 60 days |
| 2 | `attendee`, `attendee_vote`, `email_thread`, `junction` | 0.8 | 30 days |
| 2 | `group_pattern` | 0.75 | 60 days |
| 2 | `proactive`, `glean`, `glean_search`, `glean_org` | 0.7 | 3-60 days |
| 3 | `clay`, `gravatar` | 0.6 | 90 days |
| 4 (lowest) | `keyword`, `keyword_fuzzy`, `heuristic`, `embedding` | 0.4 | 7 days |

### Propagation Rules

| Rule | Source Signal | Derived Signal | Target |
|------|-------------|----------------|--------|
| `rule_person_job_change` | Person title_change/company_change | stakeholder_change | Linked accounts |
| `rule_overdue_actions` | Action overdue | engagement_warning | Linked account |
| `rule_champion_sentiment` | Transcript sentiment negative on champion | champion_risk | Account |
| `rule_departure_renewal` | Person departure + account has renewal | renewal_risk_escalation | Account |
| `rule_renewal_engagement_compound` | Multiple negative signals near renewal | renewal_risk_escalation | Account |
| `rule_person_network` | Person profile discovered | person_network_updated | Related people |
| `rule_hierarchy_up` | Child entity signal | Propagated signal | Parent account |
| `rule_hierarchy_down` | Parent entity signal | Propagated signal | Child entities |
| `rule_person_profile_discovered` | Clay/Gravatar enrichment | profile_discovered | Person + accounts |

### Decay Model

```
decayed_weight = base_weight * 2^(-age_days / half_life_days)
```
- **Implementation**: `signals/decay.rs` (pure math, no DB)
- **Age calculation**: `age_days_from_now()` parses RFC3339 or SQLite datetime format
- **Fusion**: Weighted log-odds Bayesian combination (`signals/fusion.rs`)

### Feedback Loop (User Corrections)

```
User corrects entity assignment
    |
    v
feedback::record_correction()           -- feedback.rs:23
    |  [finds which signal source led to wrong entity]
    |  [inserts entity_resolution_feedback row]
    v
update_weights_from_correction()         -- feedback.rs:110
    |  [penalize wrong source: beta++ in signal_weights]
    |  [reward correct sources: alpha++ in signal_weights]
    v
get_learned_reliability()                -- bus.rs:320
    |  [reads signal_weights(alpha, beta, update_count)]
    |  [if update_count >= 5: Thompson Sampling]
    |  [else: uninformative prior 0.5]
    v
Affects future signal fusion
```

### Prep Invalidation (Signal-Driven)

```
Signal emitted with confidence >= 0.70
    |
    v
check_and_invalidate_preps()             -- invalidation.rs:22
    |  [checks signal_type is invalidating type]
    |  [queries meetings within 48h linked to entity]
    v
Push meeting_id to prep_invalidation_queue
    |  [scheduler drains queue every 1 min]
    v
Trigger Today workflow (regenerates preps)
```

**Invalidating signal types**: `stakeholder_change`, `champion_risk`, `renewal_risk_escalation`, `engagement_warning`, `project_health_warning`, `title_change`, `company_change`, `pre_meeting_context`, `stakeholders_updated`, `team_member_added`, `team_member_removed`, `transcript_outcomes`

### Data Touched

| Operation | Table | Read/Write |
|-----------|-------|------------|
| emit_signal | `signal_events` | Write |
| emit_signal | `meetings_history` | Write (has_new_signals flag) |
| propagate | `signal_events` | Write (derived) |
| propagate | `signal_derivations` | Write |
| propagate | `meeting_entities` | Read |
| feedback | `entity_resolution_feedback` | Write |
| feedback | `signal_weights` | Write (alpha/beta) |
| invalidation | `meeting_entities`, `meetings_history` | Read |

### Error Handling

| Step | Failure Mode | Handling | Tag |
|------|-------------|----------|-----|
| emit_signal DB write | SQLite error | Returns `Err(DbError)` | `[HANDLED]` |
| Flag future meetings | SQL error on UPDATE | Silently ignored (`let _ =`) | `[SILENT FAILURE]` |
| Propagation rule panics | Rule fn returns bad data | Each rule is `fn`, panics crash propagation | `[UNTESTED]` |
| Prep invalidation query | SQL error | Logs warning, returns early | `[LOGGED ONLY]` |
| Prep queue lock poisoned | Mutex poison | Silently returns | `[SILENT FAILURE]` |

### Concurrency

- **Signal emission**: Synchronous within the caller's context. No dedicated thread.
- **Propagation**: Runs inline with emission in `emit_signal_and_propagate()`.
- **DB**: All signal operations use the caller's DB connection (no split-lock).
- **Prep invalidation queue**: `Mutex<Vec<String>>` drained by scheduler every minute.

### Testing

- **Source weights**: Tested. `[TESTED]`
- **Half-lives**: Tested. `[TESTED]`
- **emit + get signals**: Tested. `[TESTED]`
- **Supersede exclusion**: Tested. `[TESTED]`
- **Propagation engine**: Tested (empty engine, with mock rule, derivation insert). `[TESTED]`
- **Decay math**: Tested (half-life, zero age, double half-life, edge cases). `[TESTED]`
- **Feedback recording + weight updates**: Tested. `[TESTED]`
- **Prep invalidation**: Tested (low confidence skip, irrelevant type skip, with meeting). `[TESTED]`
- **Fusion (log-odds combination)**: Tested. `[TESTED]`
- **Thompson Sampling**: Tested. `[TESTED]`
- **Full propagation chain with real rules**: Not integration-tested. `[UNTESTED]`
- **emit_signal_propagate_and_evaluate chain**: Not tested. `[UNTESTED]`

---

## Pipeline 3: Meeting Prep

**Location**: `src-tauri/src/meeting_prep_queue.rs`, `src-tauri/src/prepare/`, `src-tauri/src/intelligence/lifecycle.rs`

### Flow Diagram

```
Trigger
    |
    +-- App boot: sweep_meetings_needing_prep()       -- meeting_prep_queue.rs:184
    |       [queries future meetings with entities but no prep_frozen_json]
    |       [enqueues all at Background priority]
    |
    +-- Day change: scheduler calls sweep again        -- scheduler.rs:87
    |
    +-- Signal-driven invalidation: prep_invalidation_queue drained
    |       [scheduler drains queue, triggers Today workflow]
    |
    +-- Manual refresh: user clicks refresh
    |       [enqueues at Manual priority]
    |
    +-- generate_meeting_intelligence()                -- lifecycle.rs:185
    |       [enqueues at Manual priority]
    |
    v
MeetingPrepQueue::enqueue()                            -- meeting_prep_queue.rs:72
    |  [debounce: 60s for Background/PageLoad]
    |  [dedup: same meeting_id => keep higher priority]
    |  [Manual bypasses debounce]
    v
run_meeting_prep_processor() loop                      -- meeting_prep_queue.rs:247
    |  [polls every 5s, adaptive sleep, wake on Notify]
    |  [dev mode check]
    |  [periodic debounce pruning]
    v
dequeue() highest-priority request
    v
generate_mechanical_prep() [spawn_blocking]            -- meeting_prep_queue.rs:341
    |
    |  Phase 1: Load meeting from DB (own connection)
    |  Phase 2: Check if prep already exists (skip if fresh)
    |  Phase 3: Build classified meeting JSON
    |  Phase 4: gather_meeting_context_single()        -- prepare/meeting_context.rs
    |       [entity intelligence, account dashboards,
    |        open actions, meeting history, signals]
    |       [optional: embedding-based signal relevance]
    |  Phase 5: build_prep_json_public()               -- workflow/deliver.rs
    |       [converts DirectiveMeetingContext to FullMeetingPrep JSON]
    |       [adds filePath, timeRange defaults for deserialization]
    |  Phase 6: Write prep_frozen_json to DB
    |       [does NOT set prep_frozen_at (owned by AI workflow)]
    |  Phase 7: Update intelligence_state to "enriched"
    v
Emit "prep-ready" Tauri event
    |
    v
Frontend receives event, reloads meeting detail
```

### Priority Levels

| Priority | Value | Trigger |
|----------|-------|---------|
| `Background` | 0 | Boot sweep, weekly workflow |
| `PageLoad` | 1 | User opens Week page, meeting has no prep |
| `Manual` | 2 | User clicks Refresh on meeting detail |

### Load Order (load_meeting_prep_from_sources)

1. `prep_frozen_json` from DB (highest priority)
2. Disk files from `_today/data/preps/*.json` (fallback)

**Critical caveat**: Stale skeleton prep files on disk (5 fields, no intelligence) deserialize successfully into `FullMeetingPrep` with all content as `None`, preventing richer DB sources from loading. DB sources are checked first.

### Data Touched

| Step | Table/File | Read/Write |
|------|-----------|------------|
| sweep_meetings_needing_prep | `meetings_history`, `meeting_entities` | Read |
| generate_mechanical_prep | `meetings_history` | Read |
| gather_meeting_context_single | `entity_intelligence`, `accounts`, `actions`, `signal_events`, `meeting_entities` | Read |
| gather_meeting_context_single | Entity `intelligence.json` (disk) | Read |
| build_prep_json_public | N/A (in-memory transform) | N/A |
| Write result | `meetings_history.prep_frozen_json` | Write |
| Update state | `meetings_history.intelligence_state` | Write |
| Audit | `audit_log` (disk) | Write |

### Error Handling

| Step | Failure Mode | Handling | Tag |
|------|-------------|----------|-----|
| DB lock poisoned | Mutex poison on sweep | Logs warning, returns | `[LOGGED ONLY]` |
| Meeting not found | Deleted between enqueue and process | Returns `Err`, logged | `[HANDLED]` |
| Config lock poisoned | RwLock poison | Returns `Err`, logged | `[HANDLED]` |
| gather_meeting_context failure | SQL or file I/O error | Propagates as Err | `[HANDLED]` |
| serde deserialization | DirectiveMeetingContext parse failure | `unwrap_or_default()` — silent empty context | `[SILENT FAILURE]` |
| spawn_blocking panic | Tokio task panics | Caught by match on JoinError, logged | `[HANDLED]` |
| prep_frozen_json write | SQLite write error | Returns Err, logged | `[HANDLED]` |

### Concurrency

- **Self-concurrent**: No. Single processor loop with `Mutex<VecDeque>` queue.
- **With intel_queue**: Independent. Does NOT hold `heavy_work_semaphore` (mechanical only, no PTY).
- **DB access**: Split-lock via `ActionDb::open()`. Does not block UI.
- **Blocking work**: Runs `generate_mechanical_prep` via `tokio::task::spawn_blocking` to avoid blocking tokio runtime.

### Testing

- **Queue operations**: Tested (enqueue, dequeue, dedup, priority ordering, debounce, manual bypass, prune). `[TESTED]`
- **generate_mechanical_prep**: Not unit-tested (requires full AppState + DB with meetings). `[UNTESTED]`
- **sweep_meetings_needing_prep**: Not unit-tested. `[UNTESTED]`
- **Integration with gather_meeting_context**: Not tested end-to-end. `[UNTESTED]`
- **prep_frozen_json deserialization round-trip**: Not tested. `[UNTESTED]`

---

## Pipeline 4: Transcript Processing

**Location**: `src-tauri/src/processor/transcript.rs`, `src-tauri/src/processor/mod.rs`

### Flow Diagram

```
User drops transcript file into _inbox/
    |
    v
process_file()                            -- processor/mod.rs:54
    |  [validates path within inbox]
    |  [detects format, extracts text]
    |  [classifies file]
    v
classify_file() returns Classification::Transcript
    |  OR: user manually triggers process_transcript()
    v
process_transcript()                      -- processor/transcript.rs:38
    |
    |  Step 1: Read source file
    |  Step 2: Route to account dir or archive
    |       [validate account exists in DB]
    |       [build YAML frontmatter]
    |       [write file with frontmatter]
    |  Step 3: Build prompt + invoke Claude
    |       [truncate_transcript(): tail-biased, keep first 3K + last 57K]
    |       [ModelTier::Extraction, 180s timeout]
    |       [injection protection: encode_high_risk_field, wrap_user_data]
    |  Step 4: Parse response
    |       [parse_enrichment_response() extracts sections]
    |       [SUMMARY, DISCUSSION, ANALYSIS, ACTIONS, WINS, RISKS, DECISIONS]
    |  Step 4a: Extract actions to SQLite
    |       [parse_action_metadata() for priority, @account, due date, #context]
    |       [upsert_action_if_not_completed()]
    |  Step 4b: Store captures (wins, risks, decisions)
    |       [insert_capture() for each]
    |  Step 4c: Emit transcript_outcomes signal
    |       [confidence 0.75, source "transcript"]
    |  Step 5: Run post-enrichment hooks
    |  Step 6: Log to processing_log
    |  Step 7: Append wins to impact log (atomic append)
    v
TranscriptResult returned to caller
```

### Inbox Processing (Non-Transcript)

```
Inbox batch workflow (scheduled or manual)
    |
    v
process_all()                             -- processor/mod.rs:384
    |  [scans _inbox/ directory]
    |  [skips directories and hidden files]
    v
process_file() per file                   -- processor/mod.rs:54
    |  [classify → route → log]
    |
    +-- Classification::MeetingNotes
    |       [route to account dir or archive]
    |       [I474: try_match_to_meeting() — match to historical meeting]
    |       [emit transcript_outcomes signal if matched]
    |
    +-- Classification::ActionItems
    |       [route + extract_and_sync_actions()]
    |
    +-- Classification::Unknown
    |       [NeedsEnrichment — left in inbox for AI]
    |
    +-- Classification::NeedsEntity
            [left in inbox, suggested entity name returned]
```

### Data Touched

| Step | Table/File | Read/Write |
|------|-----------|------------|
| Route to account dir | `accounts` | Read (validate account exists) |
| Write transcript | Disk (Accounts/*/Call-Transcripts/) | Write |
| Invoke Claude | PTY subprocess | External |
| Extract actions | `actions` | Write |
| Store captures | `captures` | Write |
| Emit signal | `signal_events`, `meetings_history` | Write |
| Post-enrichment hooks | Various | Read/Write |
| Processing log | `processing_log` | Write |
| Impact log | Disk (_today/90-impact-log.md) | Append |
| Audit trail | Disk (_audit/) | Write |

### Error Handling

| Step | Failure Mode | Handling | Tag |
|------|-------------|----------|-----|
| File read failure | File deleted or permissions | Returns TranscriptResult with status "error" | `[HANDLED]` |
| Directory creation | mkdir fails | Returns error result | `[HANDLED]` |
| File write failure | Disk full or permissions | Returns error result | `[HANDLED]` |
| PTY spawn failure | Claude not available | Returns partial success (file routed, no AI) | `[HANDLED]` |
| Parse failure | AI returns garbage | Empty sections, debug message with raw output | `[HANDLED]` |
| Action extraction | DB write fails per action | Logs warning per action, continues | `[LOGGED ONLY]` |
| Capture insertion | DB write fails | Silently ignored (`let _ =`) | `[SILENT FAILURE]` |
| Signal emission | DB write fails | Silently ignored (`let _ =`) | `[SILENT FAILURE]` |
| Hook failure | Hook returns error | Logged, does not block pipeline | `[LOGGED ONLY]` |
| Processing log write | DB error | Logs warning | `[LOGGED ONLY]` |
| Impact log append | File I/O error | Silently ignored (`let _ =`) | `[SILENT FAILURE]` |

### Concurrency

- **Self-concurrent**: No. `process_transcript()` is synchronous, called from command handler.
- **With other pipelines**: No locks held during processing. PTY call is blocking.
- **Batch processing**: `process_all()` processes files sequentially (no parallelism).

### Testing

- **Prompt building**: Tested (with/without account, null fields, injection encoding). `[TESTED]`
- **Transcript truncation**: Tested (short, long). `[TESTED]`
- **Frontmatter generation**: Tested (with/without account). `[TESTED]`
- **Destination routing**: Tested (with account, without account). `[TESTED]`
- **Slugify**: Tested. `[TESTED]`
- **Decision parsing**: Tested. `[TESTED]`
- **Inbox file processing**: Tested (plaintext, markdown, unsupported, CSV). `[TESTED]`
- **User attachment processing**: Tested. `[TESTED]`
- **Full process_transcript with real AI**: Not tested. `[UNTESTED]`
- **Action extraction from AI output**: Not directly tested. `[UNTESTED]`
- **Signal emission from transcript**: Not tested. `[UNTESTED]`

---

## Pipeline 5: Report Generation

**Location**: `src-tauri/src/reports/` (mod.rs, generator.rs, invalidation.rs, swot.rs, account_health.rs, ebr_qbr.rs, weekly_impact.rs, monthly_wrapped.rs, risk.rs)

### Flow Diagram

```
Frontend command: generate_report(entity_id, entity_type, report_type)
    |
    v
Phase 1: Gather input (brief DB lock)
    |  [read entity_intelligence, signals, meeting history]
    |  [compute intel_hash (SHA-256 of enriched_at + assessment)]
    |  [check existing report: skip if hash matches (not stale)]
    |  [build report-specific prompt]
    v
Phase 2: Run PTY generation (NO DB lock)
    |  [run_report_generation()]            -- generator.rs:26
    |  [ModelTier::Synthesis, 300s timeout, nice 10]
    |  [audit trail written]
    v
Phase 3: Write result to DB
    |  [upsert_report() — INSERT ON CONFLICT UPDATE]
    |  [sets is_stale = 0, updates intel_hash]
    v
Report returned to frontend
```

### Report Types

| Type | File | Trigger |
|------|------|---------|
| `swot` | swot.rs | Manual (user generates from account page) |
| `account_health` | account_health.rs | Manual |
| `ebr_qbr` | ebr_qbr.rs | Manual |
| `weekly_impact` | weekly_impact.rs | Auto (Monday via scheduler) + manual |
| `monthly_wrapped` | monthly_wrapped.rs | Auto (1st of month via scheduler) + manual |
| `risk_briefing` | risk.rs | Manual |

### Staleness Model

```
intel_hash = SHA-256(entity_id + entity_type + enriched_at + executive_assessment)[:16]

On report read:
  current_hash = compute_intel_hash(entity_id, entity_type, db)
  if current_hash != stored_hash:
      is_stale = true

On entity enrichment:
  mark_reports_stale(db, entity_id)  -- invalidation.rs
  [sets is_stale = 1 for all entity reports]
```

### Data Touched

| Step | Table/File | Read/Write |
|------|-----------|------------|
| Gather input | `entity_intelligence` | Read |
| Gather input | `signal_events` | Read |
| Gather input | `meetings_history`, `meeting_entities` | Read |
| Gather input | `accounts`, `projects` | Read |
| Gather input | `reports` (check existing) | Read |
| Run generation | PTY subprocess | External |
| Write result | `reports` | Write (upsert) |
| Staleness invalidation | `reports` | Write (mark stale) |
| Audit | Disk (_audit/) | Write |

### Error Handling

| Step | Failure Mode | Handling | Tag |
|------|-------------|----------|-----|
| Entity not found | No row in entity_intelligence | Returns error string to frontend | `[HANDLED]` |
| PTY failure | Claude not available or timeout | Returns error string | `[HANDLED]` |
| DB write failure | SQLite error on upsert | Returns error string | `[HANDLED]` |
| Hash computation | No entity_intelligence row | Returns empty hash (default) | `[HANDLED]` |
| Report save (user edit) | SQL error | Returns error string | `[HANDLED]` |

### Concurrency

- **Self-concurrent**: Reports can be generated concurrently for different entities. No entity-level locking.
- **Two-phase pattern**: DB lock held only during gather (Phase 1) and write (Phase 3). PTY runs without lock.
- **With intel_queue**: Intel enrichment invalidates reports (marks stale). Race condition: if report generation runs while enrichment writes, the hash may be computed on stale data. Mitigated by rechecking hash on write.

### Testing

- **Report types parsing**: Not explicitly tested. `[UNTESTED]`
- **Intel hash computation**: Not tested. `[UNTESTED]`
- **Upsert/query operations**: Not tested. `[UNTESTED]`
- **Staleness detection**: Not tested. `[UNTESTED]`
- **Full generation flow**: Not tested (requires PTY). `[UNTESTED]`
- **Report invalidation on enrichment**: Not tested. `[UNTESTED]`

---

## Pipeline 6: Background Schedulers

**Location**: `src-tauri/src/scheduler.rs`, `src-tauri/src/self_healing/`, `src-tauri/src/proactive/`

### Scheduler Flow Diagram

```
Scheduler::run()                           -- scheduler.rs:66
    |  [polls every 60s]
    |  [dev mode check: skip if dev sandbox active]
    |
    +-- Day change detection
    |       [compares Local::now().date_naive() to last_date]
    |       [emits "day-changed" Tauri event]
    |       [sweep_meetings_needing_prep()]
    |       [Monday: auto-generate weekly_impact report]
    |       [1st of month: auto-generate monthly_wrapped report]
    |
    +-- Sleep/wake detection
    |       [if time jumped > 5 minutes since last check]
    |       [check_missed_jobs() with grace periods]
    |       [daily: 2h grace, weekly: 24h grace]
    |
    +-- Check and run due jobs
    |       [Today workflow: cron-scheduled daily briefing]
    |       [Archive workflow: cron-scheduled archival]
    |       [InboxBatch workflow: cron-scheduled inbox processing]
    |       [Week workflow: cron-scheduled weekly forecast]
    |
    +-- Drain prep invalidation queue (every 1 min)
    |       [triggers Today workflow to regenerate invalidated preps]
    |
    +-- Auto-archive stale proposed actions (every 24h)
    |       [auto_archive_old_proposed(7 days)]
    |
    +-- Pre-meeting auto-refresh (every 30 min)
    |       [meetings starting within 2h with new signals or stale intel]
    |       [calls generate_meeting_intelligence() per meeting]
    |
    +-- Post-meeting email correlation (every 30 min)
            [correlate_post_meeting_emails_with_engine()]
```

### Workflow Schedules

| Workflow | Default Cron | Purpose |
|----------|-------------|---------|
| `Today` | Configurable (e.g., `0 8 * * 1-5`) | Daily briefing pipeline |
| `Archive` | Configurable | Archive stale data |
| `InboxBatch` | Configurable | Process inbox files |
| `Week` | Configurable | Weekly forecast generation |

### Self-Healing Subsystem

```
evaluate_portfolio()                       -- self_healing/mod.rs:21
    |  [called from hygiene Phase 3]
    |  [initialize_quality_scores() — ensure every entity has quality row]
    |  [prioritize_enrichment_queue() — sorted by trigger score]
    |  [budget check: HygieneBudget::try_consume()]
    |  [circuit breaker check per entity]
    v
IntelligenceQueue::enqueue(ProactiveHygiene)
    v

on_enrichment_complete()                   -- self_healing/scheduler.rs:66
    |  [run_coherence_check() — embedding similarity]
    |       [cosine similarity of assessment vs meeting corpus]
    |       [threshold: 0.30]
    |  [if passed: clear coherence state]
    |  [if failed: emit entity_coherence_flagged signal]
    v
manage_circuit_breaker()                   -- self_healing/scheduler.rs:113
    |  [first failure: start window, retry]
    |  [< 24h, 3 retries: trip breaker (block entity)]
    |  [> 24h < 72h: reset count, new window]
    |  [> 72h: auto-expire, reset, re-enqueue]
    v
Entity blocked or re-enqueued
```

### Proactive Detection Engine

```
ProactiveEngine::run_scan()                -- proactive/engine.rs:74
    |  [runs registered detectors matching user profile]
    |  [dedup by fingerprint (SHA-256, 7-day window)]
    |  [emit signal via bus::emit_signal()]
    |  [insert into proactive_insights table]
    |  [update proactive_scan_state]
    v

Default detectors (9 total):
  - detect_renewal_gap (cs, executive)
  - detect_relationship_drift (all)
  - detect_email_volume_spike (all)
  - detect_meeting_load_forecast (all)
  - detect_stale_champion (cs, executive)
  - detect_action_cluster (all)
  - detect_prep_coverage_gap (all)
  - detect_no_contact_accounts (all)
  - detect_renewal_proximity (cs, sales, partnerships, executive)
```

### Data Touched

| Component | Tables | Read/Write |
|-----------|--------|------------|
| Scheduler | `meetings_history`, `actions` | Read + Write |
| Scheduler | Workflow execution state | Read + Write |
| Self-healing | `entity_quality` | Read + Write |
| Self-healing | `entity_intelligence` | Read |
| Self-healing | `meetings_history` (for corpus) | Read |
| Proactive | Various entity tables | Read |
| Proactive | `proactive_insights` | Read + Write |
| Proactive | `proactive_scan_state` | Write |
| Proactive | `signal_events` | Write (via emit) |

### Error Handling

| Step | Failure Mode | Handling | Tag |
|------|-------------|----------|-----|
| Scheduler cron parse | Invalid expression | Returns `ExecutionError::ConfigurationError` | `[HANDLED]` |
| Scheduler send failure | mpsc channel closed | Logs error | `[LOGGED ONLY]` |
| DB open for auto-archive | Connection failure | Logs warning, skips | `[LOGGED ONLY]` |
| Pre-meeting refresh | DB or generation failure | Logs warning per meeting, continues | `[LOGGED ONLY]` |
| Post-meeting correlation | DB open or query failure | Logs warning | `[LOGGED ONLY]` |
| Weekly impact auto-gen | Service error | Logs warning | `[LOGGED ONLY]` |
| Monthly wrapped auto-gen | Service error | Logs warning | `[LOGGED ONLY]` |
| Coherence check | Embedding model unavailable | Passes with score 1.0 (graceful skip) | `[HANDLED]` |
| Circuit breaker SQL | Query fails | `unwrap_or(0)` — default to not blocked | `[SILENT FAILURE]` |
| Proactive detector panic | Detector fn panics | Crashes entire scan (no catch) | `[UNTESTED]` |
| Proactive signal emit | DB error | Returns Err, stops scan | `[HANDLED]` |

### Concurrency

- **Scheduler loop**: Single async loop. All workflows dispatched via mpsc channel.
- **Self-healing**: Synchronous within the caller context (hygiene phase or signal evaluation).
- **Proactive scans**: Sequential detector evaluation, no parallelism.
- **Pre-meeting refresh**: Sequential per-meeting processing.
- **Budget gating**: `HygieneBudget` is atomic, limits daily enrichments.

### Testing

- **Cron parsing**: Tested (weekdays 8am, midnight, invalid). `[TESTED]`
- **Next run time**: Tested. `[TESTED]`
- **Circuit breaker**: Tested (default, blocked, reset, trips after max retries, first failure, blocked evaluation). `[TESTED]`
- **Proactive engine**: Tested (empty, mock detector, dedup, profile filtering). `[TESTED]`
- **Fingerprint determinism**: Tested. `[TESTED]`
- **Scheduler loop**: Not tested (requires full AppState). `[UNTESTED]`
- **Sleep/wake detection**: Not tested. `[UNTESTED]`
- **Day change handling**: Not tested. `[UNTESTED]`
- **Missed job grace periods**: Not tested. `[UNTESTED]`
- **Pre-meeting refresh**: Not tested. `[UNTESTED]`
- **Self-healing coherence check with real embeddings**: Not tested. `[UNTESTED]`

---

## Pipeline 7: Google API Integration

**Location**: `src-tauri/src/google.rs`, `src-tauri/src/google_api/` (mod.rs, auth.rs, calendar.rs, gmail.rs, token_store.rs)

### OAuth Flow Diagram

```
User clicks "Connect Google" in Settings
    |
    v
run_consent_flow()                         -- google_api/auth.rs:30
    |
    |  1. Load credentials
    |       [~/.dailyos/google/credentials.json (dev override)]
    |       [embedded_credentials() (production)]
    |  2. Generate PKCE verifier + challenge
    |  3. Generate OAuth state parameter
    |  4. Bind TcpListener on random localhost port
    |  5. Build auth URL with scopes, PKCE, state
    |  6. Open browser (open::that)
    |  7. Wait for redirect callback
    |       [validate state parameter]
    |  8. Exchange auth code for tokens
    |       [POST to token_uri with PKCE verifier]
    |  9. Fetch user email (Gmail profile → OAuth userinfo fallback)
    | 10. Save token to storage
    |       [macOS Keychain (primary) or file fallback]
    | 11. Send success HTML to browser
    v
Return authenticated email address
```

### Token Refresh

```
get_valid_access_token()                   -- google_api/mod.rs:435
    |
    |  1. load_token() from storage backend
    |  2. is_token_expired()?
    |       [check expiry field, 60s buffer]
    |       [no expiry = assume expired]
    |  3. If expired: refresh_access_token()
    |       [acquire TOKEN_REFRESH_MUTEX (serialize concurrent refreshes)]
    |       [POST refresh_token to token_uri]
    |       [parse new access_token + expires_in]
    |       [save_token() to persist]
    |  4. Return valid access token
    v
```

### Calendar Sync

```
Calendar polling (triggered by Today/Week workflows)
    |
    v
fetch_calendar_events()                    -- google_api/calendar.rs
    |  [GET calendar/v3/calendars/primary/events]
    |  [pagination: follows nextPageToken]
    |  [parameters: timeMin, timeMax, maxResults, singleEvents=true]
    |  [parse attendees, organizer, description, location]
    |  [normalize to GoogleCalendarEvent]
    v
classify_meeting()                         -- google_api/classify.rs
    |  [10-rule classification system]
    |  [determines MeetingType: Customer, Internal, 1on1, etc.]
    v
Store in meetings_history                  -- db.rs
    |  [upsert with calendar_event_id as key]
    |  [entity resolution via prepare/entity_resolver.rs]
    v
Signal emissions for calendar changes
```

### Gmail Sync

```
Gmail polling (triggered by Today/InboxBatch workflows)
    |
    v
fetch_inbox_emails()                       -- google_api/gmail.rs
    |  [GET gmail/v1/users/me/messages?q=in:inbox]
    |  [pagination: up to 5 pages]
    |  [per message: GET format=metadata]
    |  [extract: from, subject, snippet, date, labels]
    |  [detect unread status via UNREAD label]
    v
Email signal processing                   -- signals/email_bridge.rs
    |  [classify emails by entity association]
    |  [emit email signals (inbound/outbound)]
    |  [score emails for relevance]
    v
Store email signals in DB
```

### Retry Policy

```
send_with_retry()                          -- google_api/mod.rs:183
    |  [max_attempts: 3]
    |  [initial_backoff: 250ms]
    |  [max_backoff: 2000ms]
    |  [exponential backoff with jitter]
    |
    |  Retryable conditions:
    |    - 429 Too Many Requests
    |    - 408 Request Timeout
    |    - 5xx Server Error
    |    - Transport errors (timeout, connect)
    |
    |  Honors Retry-After header (capped at 30s)
    v
```

### Data Touched

| Operation | Table/File | Read/Write |
|-----------|-----------|------------|
| Token storage | macOS Keychain / token.json | Read + Write |
| Credentials | credentials.json (disk) / embedded | Read |
| Calendar events | `meetings_history` | Write (upsert) |
| Calendar events | `meeting_entities` | Write |
| Email fetch | `email_items` | Write |
| Email signals | `signal_events` | Write |

### Error Handling

| Step | Failure Mode | Handling | Tag |
|------|-------------|----------|-----|
| Credentials not found | No file, no embedded secret | `GoogleApiError::CredentialsNotFound` | `[HANDLED]` |
| Token not found | No keychain entry, no file | `GoogleApiError::TokenNotFound` | `[HANDLED]` |
| Token expired + no refresh_token | Refresh impossible | `GoogleApiError::AuthExpired` | `[HANDLED]` |
| Refresh failed (invalid_grant) | Refresh token revoked | `GoogleApiError::AuthExpired` — user must re-auth | `[HANDLED]` |
| API error (non-retryable) | 400, 403, 404 | `GoogleApiError::ApiError` with status + message | `[HANDLED]` |
| API error (retryable) | 429, 5xx | Retry with exponential backoff, up to 3 attempts | `[HANDLED]` |
| Transport error | Timeout, connection refused | Retry for timeout/connect, else propagate | `[HANDLED]` |
| OAuth state mismatch | CSRF detected | Error response to browser, `OAuthStateMismatch` returned | `[HANDLED]` |
| Browser open failure | No default browser | Logs warning, returns URL for manual copy | `[LOGGED ONLY]` |
| Token save failure | Keychain write error | Logs error, error response to browser | `[HANDLED]` |
| Concurrent refresh race | Multiple threads refresh | `TOKEN_REFRESH_MUTEX` serializes refreshes | `[HANDLED]` |

### Concurrency

- **Token refresh**: Serialized via `tokio::sync::Mutex` (one refresh at a time).
- **API calls**: Multiple API calls can run concurrently. Each gets its own access token check.
- **Calendar + Gmail**: Can poll concurrently (different endpoints).
- **OAuth flow**: Single flow at a time (user-driven, browser-based).

### Testing

- **Token roundtrip**: Tested (serialize/deserialize). `[TESTED]`
- **Python format compat**: Tested (field aliases). `[TESTED]`
- **access_token alias**: Tested. `[TESTED]`
- **Token expiry detection**: Tested (no expiry, future, past). `[TESTED]`
- **Credentials parsing**: Tested (with/without secret). `[TESTED]`
- **Auth URL construction**: Tested (PKCE + state included). `[TESTED]`
- **Retry policy**: Not directly tested. `[UNTESTED]`
- **Token refresh flow**: Not tested (requires HTTP mocking). `[UNTESTED]`
- **Calendar event fetching**: Not tested (requires API mocking). `[UNTESTED]`
- **Gmail fetching**: Not tested (requires API mocking). `[UNTESTED]`
- **Calendar classification**: Tested separately in classify.rs. `[TESTED]`
- **End-to-end OAuth flow**: Not tested (requires browser). `[UNTESTED]`

---

## Cross-Pipeline Interactions

```
                     +-----------------+
                     |   Google APIs   |
                     |  (Calendar +    |
                     |   Gmail)        |
                     +--------+--------+
                              |
                    calendar events, emails
                              |
                              v
+-------------------+    +---------+    +-------------------+
|   Scheduler       |--->| Signal  |--->| Intel Queue       |
| (cron, day change,|    |  Bus    |    | (PTY enrichment)  |
|  pre-meeting,     |    |         |    |                   |
|  post-meeting)    |    +----+----+    +--------+----------+
+-------------------+         |                  |
        |                     |            intelligence.json
        |                     v                  |
        |              +------+-------+          v
        |              | Propagation  |   +------+----------+
        |              |   Engine     |   | Meeting Prep    |
        |              | (9 rules)    |   | Queue           |
        |              +------+-------+   | (mechanical)    |
        |                     |           +--------+--------+
        v                     v                    |
+-------------------+  +-----------+       prep_frozen_json
| Transcript        |  | Self-     |               |
| Processing        |  | Healing   |               v
| (inbox/manual)    |  | (quality, |        +------+-------+
+-------------------+  |  circuit  |        |   Reports    |
                       |  breaker) |        |  (two-phase  |
                       +-----------+        |   PTY)       |
                                            +--------------+
```

### Key Dependencies

1. **Signal Bus** is the central nervous system. Nearly every pipeline emits or consumes signals.
2. **Intel Queue** consumes signals (via self-healing trigger) and produces intelligence that Meeting Prep and Reports consume.
3. **Meeting Prep Queue** is downstream of both Intel Queue (intelligence.json changes) and Signal Bus (invalidation).
4. **Scheduler** orchestrates all timed triggers and drains the prep invalidation queue.
5. **Google APIs** feed data into the system. Token refresh is a prerequisite for all Google operations.
6. **Reports** are consumers of entity intelligence. They become stale when intelligence is refreshed.

### Shared Resources

| Resource | Type | Pipelines |
|----------|------|-----------|
| `heavy_work_semaphore` | Semaphore(1) | Intel Queue, embedding inference |
| `state.db` | Mutex<Option<ActionDb>> | All (for writes) |
| `ActionDb::open()` | Independent connection | Intel Queue, Meeting Prep (split-lock reads) |
| `prep_invalidation_queue` | Mutex<Vec<String>> | Signal Bus (write), Scheduler (drain) |
| `TOKEN_REFRESH_MUTEX` | tokio::sync::Mutex | Google API (serialize refreshes) |
| `state.intel_queue` | IntelligenceQueue | Intel Queue, Signal Bus, Self-Healing, Proactive |
| `state.meeting_prep_queue` | MeetingPrepQueue | Meeting Prep, Scheduler, Lifecycle |
| `audit_log` | Mutex | Intel Queue, Meeting Prep |
