# v1.5.0 Wave 4 - Proof Bundle

**Wave:** W4 - Briefing and meeting composition producers
**Date:** 2026-06-03
**Branch:** `codex/v1.5.0-w4`
**Base:** `public/dev` after rebase on 2026-06-06 (`031cd68b`)

## Issue Map

| Issue | Scope | Evidence |
|---|---|---|
| W4-0 | L0 plan packet and review | `.docs/plans/v1.5.0-w4-l0-packet.md` |
| W4-A | Daily Briefing producer and composition id grammar | `src-tauri/abilities-runtime/src/abilities/daily_briefing_composition.rs`, `src-tauri/src/services/composition_render_orchestrator.rs` |
| W4-B | Meeting Detail producer and tokenized meeting identity | `src-tauri/abilities-runtime/src/abilities/meeting_detail_composition.rs`, `src-tauri/src/services/meetings.rs`, `src-tauri/src/commands/abilities.rs` |
| W4-C | Fallback projection and provenance hardening | `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs`, `src-tauri/abilities-runtime/src/abilities/provenance/ownership.rs` |
| W4-D | Context bridge inputs | `src-tauri/abilities-runtime/src/services/context.rs`, `src-tauri/src/services/context.rs` |
| W4-E | Tauri routed surface consumption | `src/components/dashboard/DailyBriefing.tsx`, `src/pages/MeetingDetailPage.tsx`, `src/hooks/useProjectedComposition.ts` |
| W4-F | Frontend composition contracts | `src/services/composition/contracts.ts`, `src/components/dashboard/DailyBriefing.module.css`, `src/pages/meeting-intel.module.css` |
| W4-G | Proof and publication gates | This file; `.docs/reviews/v1.5.0-w4-local-bounded-review-2026-06-03.md`; `.docs/proofs/v1.5.0/screenshots/w4-browser-composition-proof.json`; PR/push still pending explicit external approval |

## Acceptance Evidence

- The render orchestrator recognizes `dailyos/daily-briefing:briefing:local~{yyyy-mm-dd}` and `dailyos/meeting-detail:meeting:{meeting_token}` composition ids.
- The briefing scope grammar uses the stable `local` token instead of deriving identity from a workspace path in the frontend.
- Meeting Detail uses a service-owned token path. The route still accepts the local meeting id, then asks Tauri for a meeting composition token before projection.
- The meeting token service issues and resolves stable tokens, treats sanitized calendar-route ids as equivalent to stored calendar ids, and fails closed for unknown or stale tokens without exposing raw meeting/calendar identifiers in successful tokens or missing-meeting errors.
- The Meeting Detail snapshot reader verifies that any hydrated raw meeting id maps back to the server-issued `meeting_token` before returning meeting data, so direct User ability invocation cannot bind an unrelated raw meeting row to a valid-looking token subject.
- Daily Briefing and Meeting Detail both project producer-owned section/block output with fallback-policy version, trust bands, rendered provenance, source freshness, and claim refs.
- Meeting Detail uses `CompositionKind::Custom { type_id: "dailyos/meeting-detail" }` across prep and recap states instead of narrowing the surface to a prep-only kind.
- The Meeting Detail frontend consumes the projected composition before the legacy meeting intelligence chapters while preserving the finite editorial ending.
- The Daily Briefing frontend consumes the projected briefing composition and keeps existing schedule/action surface behavior below it.
- The Tauri ability surface allowlist includes the W4 command surface additions.
- Fallback projection accepts action-list titles and the global provenance ownership path no longer rejects global targets solely because they lack an entity-link row.

## Validation Log

| Check | Command | Result |
|---|---|---|
| Diff whitespace | `git diff --check` | Pass on current W4 HEAD after the 2026-06-03 final validation pass |
| Focused W4 Rust regressions | `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml daily_briefing_composition -- --nocapture`; `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml meeting_detail_composition -- --nocapture`; `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml account_overview_does_not_admit -- --nocapture` | Pass on current W4 HEAD: 1 Daily Briefing producer test, 1 Meeting Detail producer test, and 2 fallback projection admission-guard tests |
| Meeting token service regressions | `cargo test --manifest-path src-tauri/Cargo.toml meeting_composition_token -- --nocapture` | Pass on current W4 HEAD: 4 token service tests, including issue/resolve, generic missing-error, and unknown/stale fail-closed coverage |
| Meeting snapshot token guard | `cargo test --manifest-path src-tauri/Cargo.toml meeting_composition_snapshot_rejects_token_meeting_id_mismatch -- --nocapture` | Pass on current W4 HEAD: snapshot reader rejects mismatched token/raw-meeting-id pairs before returning meeting data |
| Rust clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Pass on current W4 HEAD during the 2026-06-03 final validation pass |
| Full Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` | Pass on current W4 HEAD during the 2026-06-03 final validation pass: main lib `3141 passed; 0 failed; 11 ignored`; integration tests and doc tests completed cleanly |
| Frontend typecheck | `pnpm tsc --noEmit` | Pass on current W4 HEAD |
| Frontend tests | `pnpm test` | Pass on current W4 HEAD: 62 files, 351 tests |
| Daily Briefing refresh regression | `pnpm test src/components/dashboard/DailyBriefing.test.tsx` | Pass on current W4 HEAD: 1 file, 14 tests, including projected composition force-refresh after action completion and after refreshed dashboard data lands |
| Focused frontend composition regressions | `pnpm vitest run src/hooks/useProjectedComposition.test.tsx src/components/composition/ReactBlockRenderer.test.tsx` | Pass on current W4 HEAD: 2 files, 29 tests |
| Focused frontend W4 regression set | `pnpm vitest run src/components/dashboard/DailyBriefing.test.tsx src/hooks/useProjectedComposition.test.tsx src/components/composition/ReactBlockRenderer.test.tsx` | Pass on current W4 HEAD: 3 files, 42 tests |
| Browser surface smoke | Playwright against local Vite dev server at `http://localhost:1420/` | Pass: Daily Briefing and Meeting Detail projected surfaces rendered with no actionable console errors or page errors |

## Replica Smoke

Replica-mode validation used the local replica database path and exercised the live W4 projection path. No live data is included in this proof bundle.

| Surface | Result | Rendered provenance | Trust bands | Unknown blocks | Dropped unknown blocks |
|---|---|---|---|---:|---:|
| Daily Briefing | Pass | present | present | 0 | 0 |
| Meeting Detail | Pass | present | present | 0 | 0 |

## L4 Surface Evidence

Browser evidence used the local Vite dev server at `http://localhost:1420/` with sanitized synthetic Tauri responses. Live replica data was not copied into screenshots.

| Surface | Composition id asserted | Blocks asserted | Additional assertions | Screenshot |
|---|---|---:|---|---|
| Daily Briefing | `dailyos/daily-briefing:briefing:local~2026-06-03` | 3 | Three provenance labels, no briefing fallback text, proof copy visible | `.docs/proofs/v1.5.0/screenshots/w4-daily-briefing-composition-desktop-1440x900.png` |
| Meeting Detail | `dailyos/meeting-detail:meeting:mtg_0123456789abcdef` | 3 | Fallback-policy version `1`, Finis marker present, no composition-unavailable text, proof copy visible | `.docs/proofs/v1.5.0/screenshots/w4-meeting-detail-composition-desktop-1440x900.png` |

Additional L4 assertions:

- Browser proof JSON is recorded at `.docs/proofs/v1.5.0/screenshots/w4-browser-composition-proof.json`.
- `get_projected_composition` fired for both Daily Briefing and Meeting Detail.
- `get_meeting_composition_token` fired before the meeting projection path.
- Onboarding, update, and telemetry splash chrome was suppressed in the fixture state and did not appear in the screenshots.
- `actionableConsole` and `pageErrors` were both empty in the final smoke result.

## Intelligence Loop Check

| Question | W4 answer |
|---|---|
| Claim model | W4 adds no new claim table or claim type. Briefing and meeting compositions consume existing claim-backed and snapshot-backed context through producer-owned blocks rather than ad-hoc frontend-only intelligence. |
| Provenance + trust | Producers emit rendered provenance, field attribution, trust bands, source labels, and source-as-of metadata. The frontend renders those values and does not compute trust locally. |
| Signals + invalidation | W4 uses existing projection cache/version behavior and meeting/briefing refresh paths. Briefing and meeting projected compositions force-refresh after preserved mutations, and Daily Briefing refreshes its projection when refreshed dashboard data lands. The meeting token resolver fails closed when identity is stale or unknown. |
| Runtime + surfaces | Tauri Daily Briefing and Meeting Detail routes consume the projected composition path. MCP/headless surfaces are not expanded in W4. |
| Feedback loop | Claim refs and field bindings remain attached to projected blocks so existing correction/dismissal/corroboration flows can target claim-backed content. Display-only legacy meeting chapters remain separate from claim mutation. |

## Open Gates

| Gate | Status | Notes |
|---|---|---|
| L0 review | Passed | W4 L0 packet records cycle-2 approval after briefing scope, meeting token, and custom composition-kind fixes. |
| L1 validation | Passed | Final validation passed on current W4 HEAD after the latest focused local L2 hardening: clippy, full Rust tests, TypeScript, frontend tests, W4 producer regressions, meeting token service, meeting snapshot token guard, projection, Daily Briefing refresh, and frontend composition regressions are green. |
| L4 surface proof | Passed | Sanitized browser assertions and screenshots captured for Daily Briefing and Meeting Detail. |
| Local pre-L2 bounded review | Resolved | `.docs/reviews/v1.5.0-w4-local-bounded-review-2026-06-03.md` records W4-0 token-service proof/error-privacy/snapshot-token-binding and W4-4 projected-composition refresh findings, all resolved locally. |
| L2 review | Passed | Official Codex L2 ran on 2026-06-03 with private diff/repo context export approved in-thread. Three P2 findings were fixed during L2; the final bounded rerun returned P1=0 and P2=0, and the W4 commit carries `L2-status: passed`. |
| PR/push | Approved | Publishing `codex/v1.5.0-w4` to the external GitHub remote was approved in-thread on 2026-06-06. |
