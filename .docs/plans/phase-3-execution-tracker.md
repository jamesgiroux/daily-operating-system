# Phase 3 Execution Tracker (v1.0.0)

**Last updated:** 2026-03-11  
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
| Wave 3 | 3b GA platform: I427, I428, I429, I430, I431, I438 | Planned |
| Wave 4 | 3c then 3d: I502, I493, I447-I450, I453, I454, I541-I546 | Planned |
| Wave 5 | 3e: I529, I530, I537 | Planned |
| Wave 6 | Hardening + signoff + full acceptance matrix | Planned |

## Tracker matrix

| Issue | Depends on | Wave | Status | Validation gate |
|---|---|---|---|---|
| I521 | I536, I503, I508a | 1 | Complete | Contract registry + ownership map + parity fixtures + `pnpm run test:parity` |
| I515 | I512 | 2 | Complete | Intel + prep retry/backoff + PTY circuit breaker + scheduler retry + `pipeline_failures` + targeted Rust tests |
| I514 | I512 | 2 | Complete | Commands/db decomposition ACs + boundary check + `cargo test` + strict clippy + `pnpm tsc --noEmit` |
| I538 | I511, I512 | 2 | Complete | Snapshot-then-swap refresh path + `cargo test refresh_completion` + `cargo test test_prep_queue` |
| I540 | I511, I512 | 2 | Complete | Granola action metadata preserved + notes-aware prompt + rejection source threading + lifecycle/archive fix + full Rust quality gates + `pnpm tsc --noEmit` |
| I427 | I511 | 3 | Planned | Search latency + parity gate |
| I428 | None | 3 | Planned | Degraded-mode rendering + parity gate |
| I429 | I511 | 3 | Planned | Export correctness + parity gate |
| I430 | None | 3 | Planned | Settings/Data copy + destructive action guardrails + parity gate |
| I431 | I435 | 3 | Planned | Cost model correctness + parity gate |
| I438 | None | 3 | Planned | Onboarding prime flow + parity gate |
| I502 | I499, I503 | 4 | Planned | Health rendering ACs + parity gate |
| I493 | I505, I502 | 4 | Planned | Account detail ACs + parity gate |
| I447 | I521 | 4 | Planned | Token audit ACs + parity gate |
| I454 | I521 | 4 | Planned | Vocabulary ACs + parity gate |
| I448 | I447, I521 | 4 | Planned | Actions editorial ACs + parity gate |
| I449 | I447, I521 | 4 | Planned | Week/emails editorial ACs + parity gate |
| I450 | I447, I521 | 4 | Planned | Portfolio chapter ACs + parity gate |
| ~~I451~~ | ~~I447, I521~~ | ~~4~~ | ~~Superseded by I542~~ | ~~Meeting editorial ACs + parity gate~~ |
| ~~I452~~ | ~~I447, I521~~ | ~~4~~ | ~~Superseded by I541~~ | ~~Settings editorial ACs + parity gate~~ |
| I453 | I447, I521 | 4 | Planned | Onboarding editorial ACs + parity gate |
| I541 | I447, I521 | 4 | Planned | Zero inline styles in settings, YouCard split into 3 sections, audit log pagination ≤50 initial, StatusDot shared, zero vocab violations + parity gate |
| I542 | I447, I521 | 4 | Planned | Zero inline styles in MeetingDetailPage, zero hardcoded hex/rgba in CSS module, zero vocab violations, no folio transcript button for past meetings + parity gate |
| I543 | None | 4 | Planned | All pages in PAGE-ARCHITECTURE.md, all shared components in COMPONENT-INVENTORY.md, STATE-PATTERNS.md exists, developer checklists documented, audit dates current + no dead links |
| I544 | I521 | 4 | Planned | Zero duplicate StatusDot/empty/loading/error implementations, every page uses EditorialEmpty/Loading/Error, no file >400 lines without justification, dead code removed + tsc clean |
| I545 | I447, I521 | 4 | Planned | Zero inline styles in Account/Project/Person detail pages (105 total), zero hardcoded rgba in entity detail CSS modules, shared entity-detail.module.css extracted + parity gate |
| I546 | I543 | 4 | Planned | INTERACTION-PATTERNS.md + DATA-PRESENTATION-GUIDELINES.md + NAVIGATION-ARCHITECTURE.md exist in .docs/design/, reference real components, no dead links |
| I529 | I507, I513 | 5 | Planned | Feedback UI ACs + parity gate |
| I530 | I529 | 5 | Planned | Taxonomy ACs + signal weight assertions |
| I537 | None | 5 | Planned | Feature-flag gate ACs + parity gate |

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

## Release signoff criteria (Phase 3)

1. Every Phase 3 issue marked done has linked acceptance evidence.
2. `pnpm run test:parity` passes on umbrella before merge to `main`.
3. Full frontend tests pass (`pnpm test`).
4. Rust quality gates pass for backend waves (`cargo test`, strict clippy).
5. No unresolved parity exceptions for actions/proposed actions visibility on production-shaped data.
