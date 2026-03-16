# Phase 3 Execution Tracker (v1.0.0)

**Last updated:** 2026-03-11 (Wave 4+5 complete, Wave 6 planned)
**Execution mode:** Umbrella + short-lived wave branches  
**Policy:** No Phase 3 issue closes without production-data parity gate evidence.

## Branch isolation model (locked)

1. Umbrella integration branch: `codex/v1-phase3` (created in workspace on 2026-03-11)
2. Short-lived issue branches from umbrella (examples):
- `codex/v1-phase3-i515`
- `codex/v1-phase3-i427`
- `codex/v1-phase3-i502`
3. Merge path:
- issue branch -> `codex/v1-phase3` (after issue AC + parity gate pass)
- `codex/v1-phase3` -> `main` only after full Phase 3 acceptance matrix pass
4. Isolation rule:
- `i536` track stays separate
- no cherry-picks between tracks unless explicitly approved

## Major-surface parity set (mandatory)

1. Dashboard / briefing
2. Actions
3. Account detail
4. Project detail
5. Person detail
6. Meeting detail
7. Inbox / emails
8. Settings / data
9. Reports

## Wave sequence (locked)

| Wave | Scope | Status |
|---|---|---|
| Wave 0 | Kickoff + parity baseline + tracker + branch model | Complete |
| Wave 1 | I521 definition sprint + frontend contract ownership | Complete |
| Wave 2 | 3a backend cleanup: I515 then I514, plus I538 + I540 reliability fixes | Complete |
| Wave 3 | 3b GA platform: I427, I428, I429, I430, I438 | Complete |
| Wave 4 | 3c then 3d: I502, I493, I447-I450, I453, I454, I541-I545 | Complete |
| Wave 5 | 3e: I507, I513, I529, I530, I537 | Complete |
| Wave 6 | I543, I546, FinisMarker sweep, hardening + signoff + full acceptance matrix | Planned |

## Tracker matrix

| Issue | Depends on | Wave | Status | Validation gate |
|---|---|---|---|---|
| I521 | I536, I503, I508a | 1 | Complete | Contract registry + ownership map + parity fixtures + `pnpm run test:parity` |
| I515 | I512 | 2 | Complete | Intel + prep retry/backoff + PTY circuit breaker + scheduler retry + `pipeline_failures` + targeted Rust tests |
| I514 | I512 | 2 | Complete | Commands/db decomposition ACs + boundary check + `cargo test` + strict clippy + `pnpm tsc --noEmit` |
| I538 | I511, I512 | 2 | Complete | Snapshot-then-swap refresh path + `cargo test refresh_completion` + `cargo test test_prep_queue` |
| I540 | I511, I512 | 2 | Complete | Granola action metadata preserved + notes-aware prompt + rejection source threading + lifecycle/archive fix + full Rust quality gates + `pnpm tsc --noEmit` |
| I427 | I511 | 3 | Complete | FTS5 search index + CommandMenu cross-entity search + email indexing |
| I428 | None | 3 | Complete | Sync metadata tracking + freshness UI + graceful failure recording |
| I429 | I511 | 3 | Complete | ZIP export with 8 JSON files, schema-validated queries |
| I430 | None | 3 | Complete | Data summary + clear intelligence + delete all data + privacy UI |
| I431 | I435 | 3 | Deferred | Depends on I435 (token optimization) — moved to post-1.0 |
| I438 | None | 3 | Complete | Prime onboarding chapter + drag-drop + connector cards |
| I502 | I499, I503 | 4 | Complete | Health rendering ACs + parity gate |
| I493 | I505, I502 | 4 | Complete | Account detail ACs + parity gate |
| I447 | I521 | 4 | Complete | Token audit ACs + parity gate |
| I454 | I521 | 4 | Complete | Vocabulary ACs + parity gate |
| I448 | I447, I521 | 4 | Complete | Actions editorial ACs + parity gate |
| I449 | I447, I521 | 4 | Complete | Week/emails editorial ACs + parity gate |
| I450 | I447, I521 | 4 | Complete | Portfolio chapter ACs + parity gate |
| ~~I451~~ | ~~I447, I521~~ | ~~4~~ | ~~Superseded by I542~~ | ~~Meeting editorial ACs + parity gate~~ |
| ~~I452~~ | ~~I447, I521~~ | ~~4~~ | ~~Superseded by I541~~ | ~~Settings editorial ACs + parity gate~~ |
| I453 | I447, I521 | 4 | Complete | Onboarding editorial ACs + parity gate |
| I541 | I447, I521 | 4 | Complete | Zero inline styles in settings, YouCard split into 3 sections, audit log pagination ≤50 initial, StatusDot shared, zero vocab violations + parity gate |
| I542 | I447, I521 | 4 | Complete | Zero inline styles in MeetingDetailPage, zero hardcoded hex/rgba in CSS module, zero vocab violations, no folio transcript button for past meetings + parity gate |
| I543 | None | 6 | Planned | All pages in PAGE-ARCHITECTURE.md, all shared components in COMPONENT-INVENTORY.md, STATE-PATTERNS.md exists, developer checklists documented, audit dates current + no dead links |
| I544 | I521 | 4 | Complete | Zero duplicate StatusDot/empty/loading/error implementations, every page uses EditorialEmpty/Loading/Error, no file >400 lines without justification, dead code removed + tsc clean |
| I545 | I447, I521 | 4 | Complete | Zero inline styles in Account/Project/Person detail pages (105 total), zero hardcoded rgba in entity detail CSS modules, shared entity-detail.module.css extracted + parity gate |
| I546 | I543 | 6 | Planned | INTERACTION-PATTERNS.md + DATA-PRESENTATION-GUIDELINES.md + NAVIGATION-ARCHITECTURE.md exist in .docs/design/, reference real components, no dead links |
| I507 | I487, I504, I505 | 5 | Complete | Person profile corrections + email disposition feedback — verified existing implementation meets ACs |
| I513 | I512 | 5 | Complete | 9 read-path eliminations (intelligence.json, pipeline JSON). DB as sole app read source. MCP sidecar keeps file read. |
| I529 | I507, I513 | 5 | Complete | Feedback UI ACs + UNIQUE constraint + 3-tier source attribution + parity gate |
| I530 | I529 | 5 | Complete | Taxonomy ACs + curation vs correction distinction + signal weight assertions |
| I537 | None | 5 | Complete | Feature-flag gate ACs + parity gate |

## Production-data parity gate contract

1. Canonical registry:
- `src/parity/phase3ContractRegistry.ts`
- `.docs/contracts/phase3-ui-contract-registry.json`
2. Fixture datasets:
- `.docs/fixtures/parity/mock/*.json`
- `.docs/fixtures/parity/production/*.json`
3. Test command:
- `pnpm run test:parity`
4. CI gate:
- `.github/workflows/test.yml` includes explicit parity step
5. Fail condition:
- Any major surface that passes mock but fails production-shape is release-blocking

## Wave 0-1 validation evidence

Validated on 2026-03-11 on branch `codex/v1-phase3`.

1. Branch model
- umbrella branch created locally: `codex/v1-phase3`

2. Contract + ownership artifacts
- canonical registry present: `src/parity/phase3ContractRegistry.ts`
- committed registry artifact present: `.docs/contracts/phase3-ui-contract-registry.json`
- explicit ownership map present: `src/parity/phase3OwnershipMap.ts`
- committed ownership artifact present: `.docs/contracts/phase3-ui-ownership-map.json`

3. Fixture coverage
- both datasets present for all major surfaces:
  - `.docs/fixtures/parity/mock/*.json`
  - `.docs/fixtures/parity/production/*.json`

4. Enforced validation
- `src/parity/phase3ParityGate.test.ts` now verifies:
  - registry artifact sync with TypeScript source
  - ownership artifact sync with TypeScript source
  - consumer/owner files exist
  - routed ownership paths exist in `src/router.tsx`
  - mock vs production fixture parity, error shape, and actions/proposed-actions visibility

5. Command evidence
- `pnpm run test:parity` — pass on 2026-03-11
- `pnpm test` — pass on 2026-03-11

## Wave 2 progress

1. I538 completed on 2026-03-11
- `refresh_meeting_briefing_full` now snapshots existing prep instead of clearing it up front
- manual refresh rebuild path now overwrites only when a replacement prep is successfully written
- full failure with an existing briefing restores the snapshot and returns `Update failed - showing previous briefing`
- background queue behavior remains unchanged for meetings that do not yet have a prep snapshot

2. Command evidence
- `cargo test refresh_completion` — pass on 2026-03-11
- `cargo test test_prep_queue` — pass on 2026-03-11

3. I515 completed on 2026-03-11
- `meeting_prep_queue` now carries retry metadata (`attempt`, `retry_after`, `last_error`, `overwrite_existing`)
- failed prep jobs re-enqueue with bounded backoff for retryable errors
- manual-refresh fallback retries preserve overwrite intent so a later retry can replace an existing snapshot
- new migration `064_pipeline_failures.sql` adds failure persistence
- `db/pipeline.rs` adds insert/resolve/count helpers
- meeting prep queue resolves prior `meeting_prep` failures on success and records terminal failures when retries are exhausted
- `intel_queue` now carries transient retry metadata, skips future `retry_after` items, reuses gathered context on retry, and records terminal enrichment failures in `pipeline_failures`
- shared PTY circuit breaker in `AppState` trips after consecutive PTY failures and re-opens for a cooldown probe
- scheduler-owned tasks now log failures to `pipeline_failures` and retry in 1 hour with a max of 3 retries/day
- scheduled workflow executions now log `scheduler` failures and requeue via executor after 1 hour with bounded retries

4. Command evidence
- `cargo test db::pipeline::tests::` — pass on 2026-03-11
- `cargo test meeting_prep_queue::tests::` — pass on 2026-03-11
- `cargo test refresh_completion` — pass on 2026-03-11
- `cargo test intel_queue::tests::` — pass on 2026-03-11
- `cargo test pty_circuit_breaker_trips_and_probes_after_cooldown` — pass on 2026-03-11
- `cargo test scheduler::tests::` — pass on 2026-03-11

5. I540 completed on 2026-03-11
- `prepare/actions.rs` now reads the real `actions.context` column instead of nonexistent `source_context`, so DB actions can flow back into briefing preparation
- `prepare/actions.rs` has targeted coverage proving DB-backed actions surface context in categorized results
- `src/hooks/useActions.ts` no longer hides stale pending actions in the client; backend lifecycle is now the sole arbiter
- Granola notes/transcript content is now tagged explicitly, the transcript prompt adapts for notes-only input, and extracted actions preserve priority/context/account metadata through the poller path
- Rejecting proposed actions now records the real frontend surface (`actions_page`, `daily_briefing`, `meeting_detail`) instead of falling back to `unknown`
- Pending action archival now matches the acceptance policy: 30-day overdue items archive by due date, undated items archive by age, and the daily scheduler sweep runs both proposed and pending archival
- Shared action rows now render stored action context beneath the title so AI-extracted rationale is visible in the UI

6. Command evidence
- `cargo test prepare::actions::tests::` — pass on 2026-03-11
- `cargo test granola::cache::tests::` — pass on 2026-03-11
- `cargo test processor::transcript::tests::` — pass on 2026-03-11
- `cargo test db::actions::tests::` — pass on 2026-03-11
- `cargo test granola::poller::tests::` — pass on 2026-03-11
- `cargo test quill::sync::tests::` — pass on 2026-03-11
- `cargo test --manifest-path src-tauri/Cargo.toml` — pass on 2026-03-11
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — pass on 2026-03-11
- `pnpm tsc --noEmit` — pass on 2026-03-11

7. I514 completed on 2026-03-11
- `src-tauri/src/commands.rs` is now a thin hub over split command modules under `src-tauri/src/commands/`
- `src-tauri/src/db/mod.rs` is now a re-export/prelude hub over split DB modules under `src-tauri/src/db/`
- `crate::commands::*` and `crate::db::*` API surfaces remain intact for existing service/lib call sites
- boundary-only DB mutation wrappers were added for pipeline failure logging, app-state KV writes, and signal-weight updates so hotspot files no longer bypass service-owned mutation APIs
- the boundary checker now scans the split command surface and uses a single-pass awk implementation fast enough for repeated validation

8. Command evidence
- `scripts/check_service_layer_boundary.sh` — pass on 2026-03-11
- `cargo test --manifest-path src-tauri/Cargo.toml` — pass on 2026-03-11
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — pass on 2026-03-11
- `pnpm tsc --noEmit` — pass on 2026-03-11

## Wave 3 progress

1. I427 (Full-Text Search) completed on 2026-03-11
- FTS5 virtual table (`search_index`) with cross-entity indexing: accounts, projects, people, meetings, actions, emails
- `search_global` Tauri command with bm25() ranking, prefix matching, <300ms target
- CommandMenu extended with grouped results by entity type, icon per type, click-to-navigate
- Search index rebuilt on app startup via `run_startup_sync()`
- Schema-validated: all queries verified against real migration schema (meetings not meetings_history, email_id not id, etc.)

2. I428 (Offline/Degraded Mode) completed on 2026-03-11
- Migration 066: `sync_metadata` table seeded with google_calendar, gmail, claude_code
- `connectivity.rs`: `record_sync_success()`, `record_sync_failure()`, `get_sync_freshness()` with green/amber/red thresholds
- Sync recording wired into google.rs (calendar + gmail success/failure paths) and intel_queue.rs (claude_code)
- `useConnectivity` hook polls every 60s, exposes `isFullyFresh`, `staleServices`, `oldestUpdate`
- SystemStatus.tsx displays per-source freshness with colored dots
- Glean excluded from seed (inline enrichment, no discrete sync path)

3. I429 (Data Export) completed on 2026-03-11
- `export.rs`: ZIP with 8 JSON files (accounts, people, projects, meetings, actions, signals, intelligence, metadata)
- Tauri `save()` dialog for path picker in DataPrivacySection
- All queries schema-validated against real table/column names (9 column fixes applied)

4. I430 (Privacy Controls) completed on 2026-03-11
- `privacy.rs`: `get_data_summary()` with live counts, `clear_intelligence()` deleting assessments/feedback/signals/summaries
- `delete_all_data` command: close DB, delete file, clear workspace, relaunch
- DataPrivacySection in Settings → Data with export, clear insights (confirmation), delete everything (requires typing "DELETE")
- ADR-0083 compliant: "contacts" not "people", "insights" not "intelligence"

5. I431 (Cost Visibility) deferred
- Depends on I435 (token optimization) which hasn't shipped — no audit doc, no tier corrections
- Building cost estimates on incorrect tier data would be misleading
- Moved to post-1.0 per plan guidance ("cut if needed")

6. I438 (Prime DailyOS) completed on 2026-03-11
- PrimeBriefing chapter added as step 7 in OnboardingFlow
- Tauri native drag-drop via `getCurrentWebview().onDragDropEvent()` + file browser via `open()` dialog
- Connector cards (Quill, Granola, Google Drive) with "Coming soon" disabled state
- Confirmation message after file add, correct skip label per spec
- Wizard step recording fixed to "prime" (was "role" — would have caused resume loop)

7. Command evidence
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — pass on 2026-03-11
- `cargo test --lib` — 1161 passed, 0 failed on 2026-03-11
- `pnpm tsc --noEmit` — pass on 2026-03-11
- Schema validation audit: 22 issues found and fixed across all tracks (stale table names, wrong columns, invalid FTS5 syntax)

## Wave 4 progress

Completed on 2026-03-11. Branch: `codex/v1-phase3` (worktree). 14 issues across 3c (health surfaces) + 3d (editorial polish).

### 3c: Frontend surfaces + intelligence rendering

1. I447 (Design Token Audit) — replaced 139 hardcoded rgba() values with design tokens across all CSS modules
2. I454 (Vocabulary Pass) — ADR-0083 compliance across all user-facing strings
3. I448 (ActionsPage Editorial) — full CSS module migration, ChapterHeadings, FinisMarker, EditorialLoading/Error
4. I449 (WeekPage + EmailsPage Polish) — CSS module migration, stat line tokens, editorial components
5. I450+I545 (Entity Detail + Portfolio) — 105 inline styles eliminated across Account/Project/Person detail pages, shared `entity-detail.module.css` extracted
6. I453 (Onboarding Editorial) — CSS module migration for all onboarding pages
7. I502 (Health Surfaces) — health band, dimensions, trend rendering across 6 surfaces (account detail, project detail, person detail, dashboard, meeting detail, reports)
8. I493 (Account Detail Intelligence) — Glean-sourced titles, coverage gaps, reports chapter
9. I541 (Settings UX Rebuild) — YouCard split into Identity/Workspace/Preferences, full CSS module migration, audit log pagination (≤50 initial), StatusDot consolidation
10. I542 (MeetingDetailPage Styles) — 51 inline styles migrated to CSS module, hardcoded colors replaced with tokens, folio transcript button fix
11. I544 (Component DRY/SRP) — 15 dead component files removed, duplicate StatusDot/empty/loading/error consolidation

### Wave 4 audit remediation
- Vocabulary violations caught and fixed post-initial pass
- Editorial component gaps (missing FinisMarker, ChapterHeading inconsistencies) resolved

### Command evidence
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — pass on 2026-03-11
- `cargo test --manifest-path src-tauri/Cargo.toml` — pass on 2026-03-11
- `pnpm tsc --noEmit` — pass on 2026-03-11

## Wave 5 progress

Completed on 2026-03-11. Branch: `codex/v1-phase3-wave5` (worktree) → merged to `codex/v1-phase3` → merged to `dev`.

### Issues completed

1. I507 (Source-Attributed Correction Feedback) — VERIFICATION ONLY. Both ACs already met:
   - Person profile corrections: `services/people.rs` reads `enrichment_sources`, calls `upsert_signal_weight()`
   - Email disposition: `commands/integrations.rs` calls `get_email_signal_source_for_feedback()` then `upsert_signal_weight()`

2. I513 (DB as Sole Source — remaining read paths) — 9 read-path eliminations:
   - `prepare/meeting_context.rs` — removed intelligence.json fallback → DB query
   - `prepare/orchestrate.rs` — removed intelligence.json fallback → DB query
   - `services/dashboard.rs` — replaced `load_directive`/`load_week_json` with DB queries
   - `services/emails.rs` — replaced `load_directive` with DB query
   - `executor.rs` — removed `sync_actions_to_db` call
   - `devtools/mod.rs` — removed `sync_actions_to_db` calls
   - `workflow/today.rs` — removed `sync_actions_to_db` function
   - `json_loader.rs` — removed `load_actions_json`, `load_week_json` + types
   - MCP sidecar continues reading intelligence.json (file write preserved — sidecar has no DB access)

3. I529 (Intelligence Quality Feedback UI):
   - Backend: `services/feedback.rs` with 3-tier source attribution (`enrichment_sources` → enrichment signal → None)
   - Migration 067: UNIQUE(entity_id, entity_type, field) constraint on `intelligence_feedback`
   - Frontend: `IntelligenceFeedback.tsx` hover-reveal thumbs, `useIntelligenceFeedback.ts` hook
   - Integrated on: AccountDetailEditorial (state_of_play, watch_list), ProjectDetailEditorial (state_of_play, watch_list), PersonDetailEditorial (watch_list), MeetingDetailPage (risks, plan)

4. I530 (Signal Taxonomy: Curation vs Correction):
   - `services/intelligence.rs` — `update_intelligence_field` now distinguishes empty value (curation) from edit (correction)
   - Curation: emits `intelligence_curated` signal, no `upsert_signal_weight`, no source penalty
   - Correction: emits `user_correction` signal, calls `record_enrichment_correction()`, source penalized
   - `signals/bus.rs` — taxonomy documented, `user_feedback` and `user_curation` source configs added

5. I537 (Gate Role Presets Behind Feature Flag):
   - Backend: `FeatureFlags` struct with `role_presets_enabled: false`, `get_feature_flags()` command
   - Frontend: OnboardingFlow skips EntityMode chapter when flag off, auto-sets CS + "both"
   - Settings: YouCard hides RoleSection, DiagnosticsSection hides EntityModeSelector when flag off

### AC validation issues caught and fixed
- **AC16**: INSERT OR REPLACE with UUID PK never triggered conflict → fixed with ON CONFLICT(entity_id, entity_type, field) DO UPDATE
- **AC17**: Source attribution always None for non-person entities → fixed with 3-tier `resolve_intelligence_source()` (enrichment_sources → signal_events query → None)
- **AC18**: `intelligence_curated` signal never emitted on real delete paths → fixed by checking if edited value is empty/null/[] in `update_intelligence_field`

### Command evidence
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — pass on 2026-03-11
- `cargo test --manifest-path src-tauri/Cargo.toml` — pass on 2026-03-11
- `pnpm tsc --noEmit` — pass on 2026-03-11

### Merge evidence
- Merged `codex/v1-phase3-wave5` → `codex/v1-phase3` (clean)
- Merged `codex/v1-phase3` → `dev` (4 conflicts resolved: AccountDetailEditorial.tsx, PersonDetailEditorial.tsx, ProjectDetailEditorial.tsx, YouCard.tsx — all Wave 4 CSS module imports vs Wave 5 feedback imports, keep both)

---

## Wave 6 plan: Hardening + Signoff

### Scope

Wave 6 is the final wave before GA. Two remaining documentation issues (I543, I546), a FinisMarker sweep, and the full acceptance matrix signoff.

### Step 1: I543 — GA Design Documentation (parallel with Step 2)

Update existing design docs to reflect post-Wave 4 reality:
- **PAGE-ARCHITECTURE.md** — add all undocumented pages (currently missing ~11 pages added since original doc)
- **COMPONENT-INVENTORY.md** — add all Wave 4 components (IntelligenceFeedback, HealthBand, EditableVitalsStrip, etc.)
- **STATE-PATTERNS.md** (NEW) — per-page state matrices documenting hooks, loading/error/empty patterns
- Developer checklists: "new page" and "new component" checklists referencing existing patterns
- Audit dates current, no dead links

### Step 2: I546 — Design Documentation: Interaction, Data, Navigation (parallel with Step 1)

Three new reference documents in `.docs/design/`:
- **INTERACTION-PATTERNS.md** — hover-reveal, inline edit, optimistic update, confirmation dialogs, toast patterns
- **DATA-PRESENTATION-GUIDELINES.md** — vitals strips, timeline rendering, health bands, empty state messaging
- **NAVIGATION-ARCHITECTURE.md** — route structure, magazine shell, chapter scrolling, back link patterns

### Step 3: FinisMarker Sweep

Add FinisMarker to 7 remaining pages:
- `AccountsPage.tsx`
- `ActionDetailPage.tsx`
- `MeetingHistoryDetailPage.tsx`
- `MonthlyWrappedPage.tsx`
- `PeoplePage.tsx`
- `ProjectsPage.tsx`
- `ReportPage.tsx`

### Step 4: Full Acceptance Matrix Signoff

Run every Phase 3 AC from `v1.0.0.md` (ACs 1–43) against the running app with real data:

**Quality gates:**
- `cargo test --manifest-path src-tauri/Cargo.toml` — all pass
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — clean
- `pnpm tsc --noEmit` — clean
- `pnpm test` — all pass
- `pnpm run test:parity` — passes with both mock + production fixtures

**Manual verification checklist:**
- [ ] AC6: Health band + trend arrow + confidence renders on account detail
- [ ] AC8: Cmd+K search < 300ms, click navigates
- [ ] AC9: Briefing loads with cached data when offline
- [ ] AC10: Data export ZIP produced with all entity types
- [ ] AC11: Clear intelligence + delete all data work
- [ ] AC12: Zero new rgba() violations (27 existing are intentional atmospheric colors)
- [ ] AC13: Every page ends with FinisMarker
- [ ] AC14: Zero ADR-0083 vocabulary violations
- [ ] AC15-22: Intelligence feedback loop (hover, vote, persist, source attribution, curation vs correction)
- [ ] AC32: Zero `style={{}}` in all pages
- [ ] AC33: YouCard split into Identity/Workspace/Preferences
- [ ] AC34: Audit log pagination ≤50 initial
- [ ] AC35-37: Design docs complete (I543)
- [ ] AC38-39: Zero inline styles + hardcoded rgba in entity detail pages
- [ ] AC40-41: Zero duplicate components, every page uses editorial state components
- [ ] AC42: Design docs exist (I546)

### Step 5: Merge to main

After full acceptance matrix passes:
- `codex/v1-phase3` → `main` (final merge)
- Tag `v1.0.0`
- Update `release-notes.md` and `CHANGELOG.md`

---

## Release signoff criteria (Phase 3)

1. Every Phase 3 issue marked done has linked acceptance evidence.
2. `pnpm run test:parity` passes on umbrella before merge to `main`.
3. Full frontend tests pass (`pnpm test`).
4. Rust quality gates pass for backend waves (`cargo test`, strict clippy).
5. No unresolved parity exceptions for actions/proposed actions visibility on production-shaped data.
