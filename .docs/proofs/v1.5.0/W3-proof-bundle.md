# v1.5.0 Wave 3 - Proof Bundle

**Wave:** W3 - Forward detail surfaces
**Date:** 2026-06-02
**Branch:** `codex/v1.5.0-w3`
**Base:** `public/dev` after W2 merge

## Issue Map

| Issue | Scope | Evidence |
|---|---|---|
| W3-0 | L0 plan packet and review | `.docs/plans/v1.5.0-w3-l0-packet.md`, `.docs/reviews/v1.5.0-w3-l0-cycle-1.md`, `.docs/reviews/v1.5.0-w3-l0-cycle-2.md` |
| W3-A | Subject-keyed producer registry | `src-tauri/src/services/composition_render_orchestrator.rs` |
| W3-B | Project overview producer | `src-tauri/abilities-runtime/src/abilities/project_overview.rs`, `src-tauri/src/services/context.rs` |
| W3-C | Person overview producer | `src-tauri/abilities-runtime/src/abilities/person_overview.rs`, `src-tauri/src/services/context.rs` |
| W3-D | Action detail producer | `src-tauri/abilities-runtime/src/abilities/action_detail.rs`, `src-tauri/src/services/context.rs` |
| W3-E | Action claim subject support | `src-tauri/abilities-runtime/src/types.rs`, `src-tauri/abilities-runtime/src/abilities/provenance/subject.rs`, `src-tauri/src/services/claims.rs`, `src-tauri/src/db/claim_invalidation.rs`, `src-tauri/src/migrations/276_action_claim_version.sql` |
| W3-F | Detail page projection routing | `src/hooks/useProjectedComposition.ts`, `src/hooks/useChapterLayout.ts`, `src/pages/ProjectDetailEditorial.tsx`, `src/pages/PersonDetailEditorial.tsx`, `src/pages/ActionDetailPage.tsx` |
| W3-G | Signal invalidation | `src-tauri/src/services/projects.rs`, `src-tauri/src/services/people.rs`, `src-tauri/src/services/actions.rs`, `src-tauri/src/services/linear.rs` |
| W3-H | Renderer/action feedback contract | `src/components/composition/ReactBlockRenderer.tsx`, `src/components/composition/blocks/BlockComponents.tsx`, `src/components/composition/CompositionInlineEdit.tsx`, `src/services/composition/contracts.ts` |

## Acceptance Evidence

- The composition orchestrator now resolves producers by subject identity rather than account-only routing. Project, person, and action composition ids map to `dailyos/project-overview:project:{id}`, `dailyos/person-overview:person:{id}`, and `dailyos/action-detail:action:{id}`.
- The composition id parser rejects malformed ids, empty parts, extra delimiters, unknown producers, and producer/subject mismatches before execution.
- Project and person overview producers accept orchestrator identity inputs while preserving subject validation, fallback projection, trust/provenance metadata, and renderable section/block output.
- Action Detail is modeled as `ProducerSubject::Action`, not as a generic entity kind. Action provenance uses `SubjectRef::Action(action_id)`.
- Action claims can be read through the claim service and invalidation version checks. Migration 276 adds action `claim_version` storage so action subject claim reads participate in the same invalidation discipline as account/project/person/meeting subjects.
- Legacy entity-intelligence projection stays fail-closed for action subjects; Action Detail reads claim-backed context through the W3 action producer instead of adding an unsupported generic canonical action subject.
- Project, person, action, and Linear mutation paths emit detail-surface invalidation signals when W3-visible fields change.
- Project Detail and Person Detail render projected compositions first and keep their existing controls, hierarchy/network/appendix sections, and Linear chapter below the projected surface.
- Action Detail renders the projected action composition first and preserves the existing editable action controls below it.
- `useChapterLayout` supports action as a non-persisted layout subject, so W2 account/project/person overlays do not create action overlay writes.

## Validation Log

| Check | Command | Result |
|---|---|---|
| Diff whitespace | `git diff --check` | Pass |
| Action producer tests | `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml action_detail -- --nocapture` | Pass: 6 tests |
| Project/person overview tests | `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml overview -- --nocapture` | Pass: 24 unit tests plus 3 fallback projection tests |
| App-side project composition tests | `cargo test --manifest-path src-tauri/Cargo.toml project_composition -- --nocapture` | Pass: 4 tests |
| Claim version entity mapping | `cargo test --manifest-path src-tauri/Cargo.toml bump_each_entity_kind_targets_correct_table -- --nocapture` | Pass |
| Action claim read regression | `cargo test --manifest-path src-tauri/Cargo.toml load_claims_active_accepts_action_subjects -- --nocapture` | Pass |
| Mock data schema validator | `cargo test --manifest-path src-tauri/Cargo.toml test_mock_data_insert_statements_match_current_schema -- --nocapture` | Pass |
| Reference fidelity audit | `python3 .docs/design/_audits/audit-reference.py` | Pass: Action Detail reference returned to `clean`; unrelated existing reference debt unchanged |
| Rust clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Pass |
| Full Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` | Pass: main lib `3134 passed; 0 failed; 11 ignored`; integration and doc tests completed cleanly |
| Frontend typecheck | `pnpm tsc --noEmit` | Pass |
| Detail mutation projection refresh regressions | `pnpm vitest run src/hooks/useProjectedComposition.test.tsx src/pages/ActionDetailPage.test.tsx src/pages/ProjectDetailEditorial.test.tsx src/pages/PersonDetailEditorial.test.tsx` | Pass: 4 files, 30 tests |
| Frontend tests | `pnpm test` | Pass: 62 files, 345 tests |
| Pre-commit hook | `.githooks/pre-commit` | Pass |

Note: the full Rust test run emits pre-existing unused-import warnings in `tests/dos567_fixture_backfill_and_composition_versions.rs`; they do not fail clippy or tests.

## Replica Smoke

Replica smoke used `DAILYOS_DB_MODE=replica` and the replica database path `~/.dailyos/dailyos-replica.db`. The command exercised the live W3 projection path with a temporary local helper that was removed before this proof bundle.

| Surface | Result | Sections | Root blocks | Rendered provenance | Unknown blocks | Served from cache |
|---|---:|---:|---:|---:|---:|---:|
| Project overview | Pass | 8 | 8 | present | 0 | false |
| Person overview | Pass | 8 | 8 | present | 0 | false |
| Action detail | Pass | 7 | 7 | present | 0 | false |

No live replica content is included in this proof bundle. Only structural counts from the smoke are recorded.

## L4 Surface Evidence

Browser evidence used the local Vite dev server at `http://localhost:1420/` with sanitized synthetic Tauri responses. Live replica data was not copied into screenshots.

| Surface | Composition id asserted | Sections asserted | Controls asserted | Screenshot |
|---|---|---|---|---|
| Project Detail | `dailyos/project-overview:project:project-l4` | `headline`, `portfolio`, `trajectory`, `the-horizon`, `the-landscape`, `the-room`, `whats-next`, `the-record` | `Project controls` with `Save` | `/private/tmp/dailyos-v150-w3-l4/project-main.png` |
| Person Detail | `dailyos/person-overview:person:person-l4` | `headline`, `the-dynamic`, `their-orbit`, `their-network`, `the-landscape`, `open-threads`, `the-record`, `the-work` | `Person controls` with `Save` | `/private/tmp/dailyos-v150-w3-l4/person-main.png` |
| Action Detail | `dailyos/action-detail:action:action-l4` | `headline`, `status`, `priority`, `context`, `reference`, `linear`, `action-bar` | `Action controls` with `Mark Complete` | `/private/tmp/dailyos-v150-w3-l4/action-main.png` |

Additional L4 assertions:

- `main[data-composition-id]` was visible for each route.
- No `No project/person/action composition` empty fallback rendered.
- No `No renderable blocks are available`, `controls unavailable`, `composition failed`, or error-boundary text rendered.
- Onboarding/update/telemetry splash chrome was suppressed in the fixture state and did not appear in the screenshots.
- Page errors were empty after the Tauri event lifecycle shim included listener cleanup.
- Shell-only unhandled-command warnings from the browser harness were filtered as non-actionable; actionable console findings were empty.

## Intelligence Loop Check

| Question | W3 answer |
|---|---|
| Claim model | Project/person/action detail surfaces consume claim-backed producer outputs. Action is represented as an action subject for read/provenance/invalidation paths rather than display-only frontend data. |
| Provenance + trust | Producers emit rendered provenance and trust-band-aware projected blocks. Action provenance uses `SubjectRef::Action(action_id)` instead of global/user/unknown attribution. |
| Signals + invalidation | Project, person, action, and Linear mutation paths emit W3-visible invalidation signals. Action subjects now have claim-version state for invalidation jobs. |
| Runtime + surfaces | Tauri detail routes consume subject-keyed projected compositions through `useProjectedComposition` and `useChapterLayout`. Generic legacy action entity-intelligence projection remains fail-closed. |
| Feedback loop | Renderer feedback/inline edit contracts include action entity type support so claim-backed projected blocks can send corrections with the right surface subject. Presentation controls remain separate from claim mutation. |

## L2 Readiness

| Gate | Status | Notes |
|---|---|---|
| L0 review cycles | Passed | Cycle 2 accepted the W3 plan with action-as-subject, parser rejection, W3 signal emissions, and pre-L2 L4 requirements. |
| Validation suite | Passed | Rust, clippy, frontend typecheck, frontend tests, focused producer/claim tests, and diff whitespace all passed. |
| Replica smoke | Passed | Structural projection smoke passed against replica mode; no live data is included here. |
| L4 surface proof | Passed | Sanitized browser assertions and screenshots captured for project, person, and action detail surfaces. |
| L2 review | Passed | Cycle 3 passed with no blockers after stale projection refresh was remediated across Action, Project, Person, and Person relationship mutations. One nonblocking path-alpha test-depth item was routed to maintenance as `DOS-849`. |
