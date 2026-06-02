# v1.5.0 Wave 1 - Proof Bundle

**Wave:** W1 - Account Detail, end-to-end reference
**Date:** 2026-06-01
**Branch:** `codex/v1.5.0-w1`
**Base:** `public/dev` after PR #427 merge (`cc0f5bf4`)

## Issue Map

| Issue | Scope | Evidence |
|---|---|---|
| W1-0 | Account variant-D reference/spec gate | `.docs/design/surfaces/AccountDetailPage.md`, `.docs/design/reference/surfaces/account.html`, `.docs/design/_audits/surface-manifest.json`, `.docs/design/INVENTORY.md` |
| W1-A | Shared projection service + first-party Tauri command | `src-tauri/src/commands/abilities.rs`, `src-tauri/src/services/composition_render_orchestrator.rs`, `src-tauri/src/surface_runtime/mod.rs` |
| W1-B | Sectioned projection + provenance-resolution contract | `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs`, `src-tauri/abilities-runtime/tests/dos570_fallback_projection.rs` |
| W1-C | React `BlockRenderer` and block component library | `src/services/composition/contracts.ts`, `src/hooks/useProjectedComposition.ts`, `src/components/composition/` |
| W1-D | Variant-D `dailyos/account-overview` producer | `src-tauri/abilities-runtime/src/abilities/account_overview.rs`, `src-tauri/abilities-runtime/src/services/context.rs`, `src-tauri/src/services/context.rs` |
| W1-E | Replace routed Account template with composed scroll | `src/pages/AccountDetailPage.tsx`, `src/pages/AccountDetailPage.module.css` |
| W1-F | Proof, L2, PR | This file; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-1-2026-06-01.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-2-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-3-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-4-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-5-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-6-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-7-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-8-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-l2-codex-cycle-9-2026-06-02.md`; `.docs/reviews/v1.5.0-w1-static-review-2026-06-01.md`; native proof app launch verified; native screenshot capture remains blocked by local macOS capture/tooling permissions |

## Acceptance Evidence

- The Account reference is now the composed-scroll variant-D surface: no Health/Context/Work tabs, all 11 W1 section ids represented, trust/provenance/freshness affordances shown, empty/degraded states present, and finite ending retained.
- `ProjectedComposition` now carries a safe `sections` outline in addition to the flat `blocks` list. The outline is derived after fallback projection and contains only section id, order, label, layout, salience, and safe block references.
- The Tauri app uses a first-party `get_projected_composition` command through `invoke()`, with `BridgeSurface::TauriApp` and `SurfaceKind::TauriApp`, rather than fetching the local HTTP loopback route.
- The existing `/v1/local/project-composition` loopback path remains active and continues to use the shared render orchestrator path.
- Projection cache identity now includes surface kind and fallback-policy version. Singleflight guards serialize same-key cache misses, and stale-version retry remains as a defensive fallback.
- `dailyos/account-overview` now emits the W1 section set: `headline`, `outlook`, `state-of-play`, `the-room`, `whats-next`, `watch-list`, `value-commitments`, `strategic-landscape`, `the-record`, `the-work`, and `reports`.
- The Account producer owns empty and degraded blocks. The routed React page no longer builds a dossier from independent frontend Account queries.
- `AccountCompositionSnapshotReadHandle` provides service-owned Account inputs for producer use. Snapshot fields carry sensitivity, provenance kind, source label/ref, source as-of, and trust status where applicable; confidential/user-only snapshot fields are excluded.
- Claim refs, field bindings, provenance refs, trust bands, source freshness, and fallback banners are preserved through producer -> projection -> renderer.
- Account producer provenance now stays under the hard envelope budget for high-cardinality overview claim sets by using parent-path block attributions instead of expanding large source-ref lists onto every leaf.
- Mixed claim+snapshot sections preserve both evidence paths: claim blocks remain renderable, and snapshot-backed fields append as sourced evidence instead of disappearing when claims exist.
- Recommendation claims now render as work actions in the composed Account `whats-next` and `the-work` sections instead of being dropped as ignored claims, and those ActionList blocks preserve block-level trust/freshness metadata for the renderer shell.
- React composition contracts define `ProjectedComposition`, `ProjectedSection`, `ProjectedBlock`, rendered provenance, trust-band normalization, and the 19 known W1 block types.
- `ReactBlockRenderer` dispatches all 19 known block types and renders unknown/fallback blocks through the projected safe type with a non-dismissible degraded banner. Tests assert exact renderer coverage, no raw diagnostic leakage, and no raw block id/original custom type DOM attributes.
- `AccountDetailPage.tsx` now derives folio chapters from projected sections and renders `ReactBlockRenderer`; `AccountViewSwitcher` and hidden mounted tab views are no longer used by the route.
- Gate failures found during validation were repaired in substrate-owned paths: W1 signal-policy inventory was restored, Glean provider debug filenames no longer use direct wall-clock reads, recommendation metadata mutation routes through the claims service, DOS-7 fixture exceptions are explicit, and migration rollback cleanup has a durable best-effort rationale.

## Validation Log

| Check | Command | Result |
|---|---|---|
| Diff whitespace | `git diff --check` | Pass after final proof update |
| Frontend typecheck | `pnpm tsc --noEmit` | Pass |
| Frontend tests | `pnpm exec vitest run src/components/composition/ReactBlockRenderer.test.tsx src/hooks/useProjectedComposition.test.tsx` | Pass after L2 cycle 6 remediation: 2 files, 15 tests |
| Abilities runtime clippy | `cargo clippy --manifest-path src-tauri/abilities-runtime/Cargo.toml --all-targets -- -D warnings` | Pass |
| Account overview producer regressions | `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml account_overview -- --nocapture` | Pass after L2 cycle 8 remediation: 11 account overview tests plus 3 filtered fallback projection tests, including `recommendation_claims_render_as_work_actions`, ActionList shell metadata projection, `high_cardinality_overview_claims_keep_provenance_under_hard_budget`, `missing_account_snapshot_rejects_before_composition_commit`, and producer projection compatibility |
| Fallback projection regressions | `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --test dos570_fallback_projection -- --test-threads=1` | Pass after L2 cycle 4 remediation: 21 fallback projection tests, including account-object and composite-root allowlist coverage |
| Abilities runtime provenance regression | `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml prepare_meeting_source_asof_from_child_composition_is_reachable -- --test-threads=1` | Pass after preserving entity-intelligence child source rows for `prepare_meeting` |
| Abilities runtime full tests | `cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml -- --test-threads=1` | Pass before PR #427 rebase: lib `488 passed; 0 failed`; integration, trybuild, temporal, and doc tests completed cleanly |
| Rust clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Pass |
| Projected-composition cache metric helper | `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::abilities::tests::projected_composition_metrics_only_count_producer_runs -- --exact` | Pass after L2 cycle 2 remediation |
| Devtools mock-data write discipline | `cargo test --manifest-path src-tauri/Cargo.toml --lib devtools::tests::test_mock_data_does_not_write_frozen_entity_context_table -- --exact` | Pass after routing full-scenario entity-context mock rows through user-note claim commits instead of the frozen legacy table |
| Recommendation surfacing conflict regression | `cargo test --manifest-path src-tauri/Cargo.toml services::recommendations::surfacing::tests::material_baseline_uses_subject_action_render_for_duplicate_claims -- --test-threads=1` | Pass after PR #427 rebase and conflict resolution |
| Full Tauri tests | `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1` | Pass before PR #427 rebase: main lib `3096 passed; 0 failed; 11 ignored`; integration tests and doc tests completed cleanly; doc tests `1 passed; 1 ignored` |
| Replica native startup smoke | `DAILYOS_DB_MODE=replica pnpm tauri dev` | Pass after rebasing onto PR #427: schema 274 opened without database recovery; only known `tmutil addexclusion` privilege warning emitted during observed startup window |
| Mock native startup isolation | `DAILYOS_DB_MODE=mock pnpm tauri dev`; `lsof -p <dailyos-pid>`; CoreGraphics window list; `/v1/surface/health` curl | Pass for startup/isolation: dev binary PID `67695` opened `/Users/jamesgiroux/.dailyos/dailyos-dev.db`, exposed a healthy native surface endpoint on `127.0.0.1:51799`, and had an onscreen `DailyOS` window. Visual capture is still blocked by local macOS automation/screenshot permissions, so this is not a substitute for the W1 routed visual gate. |
| Native proof app launch | `pnpm tauri build --debug --config /tmp/dailyos-w1-tauri-proof-noexternal.json`; `open -n "src-tauri/target/debug/bundle/macos/DailyOS W1 Proof.app" --args --mock`; CoreGraphics window list | Partial pass: frontend `tsc && vite build` passed, Rust debug app binary built, and a distinct `DailyOS W1 Proof` mock-mode app process/window appeared with bundle id `com.dailyos.desktop.w1proof` and 1280x800 bounds. Bundling failed after creating the app because Tauri still tried to copy unrelated missing external bin `target/debug/release_gate`. The proof app launched anyway with `--mock`; no account-overview provenance-size validation warning reappeared after the producer fix. Window capture still failed (`screencapture -l`: `could not create image from window`; `screencapture -R`: `could not create image from rect`; Computer Use `get_app_state`: `cgWindowNotFound`). |
| Supplemental browser visual | Playwright browser render of `/accounts/mock-acme-corp` with mocked `window.__TAURI_INTERNALS__.invoke("get_projected_composition")` | Supplemental only: desktop and mobile screenshots plus JSON saved under `.docs/proofs/v1.5.0/screenshots/`. Both render all 11 W1 sections, visible fallback state, no startup/What's New/lock/telemetry overlays, no horizontal overflow, and no page or console errors. This does not satisfy the native Tauri visual proof gate because IPC is mocked. |

Note: the full Rust test run emits a pre-existing unused-import warning in `tests/dos567_fixture_backfill_and_composition_versions.rs`; it does not fail clippy or tests.

## Focused Coverage

| Area | Evidence |
|---|---|
| Account producer section set | `account_overview` tests cover variant-D section order, absence of legacy sections, snapshot-backed fields, sensitive field exclusion, provenance source attachment, and projection compatibility. |
| Account producer provenance budget | `high_cardinality_overview_claims_keep_provenance_under_hard_budget` covers a 160-claim overview composition, asserts the serialized provenance envelope remains below the 1 MiB hard budget, and validates every block `ProvenanceRef` against the canonical envelope. |
| Sectioned projection | `dos570_fallback_projection` tests cover safe section outline projection and fallback-policy cache key behavior. |
| Render orchestrator cache | `composition_render_orchestrator` tests cover cache miss on surface-kind change, fallback-policy version change, scope change, cache round trip, and singleflight guard behavior. |
| Tauri transport | `get_projected_composition` command path validates account composition ids, actor ownership, Tauri surface/render policy, cache hinting, and projected-response shape. |
| Frontend command hook | `useProjectedComposition.test.tsx` asserts one Tauri `invoke()` load per account, command payload shape, rendered provenance return, stale response suppression, account-change data clearing, synchronous stale-data/provenance hiding during navigation, first-load error surfacing, and loading preservation when navigating away from a failed account. |
| Frontend block dispatch | `ReactBlockRenderer.test.tsx` asserts renderer exhaustiveness for the 19 known block types, verifies fallback banner behavior without raw diagnostics, and covers account snapshot degradation rendering without exposing internal degradation reasons. |
| Static review remediation | `.docs/reviews/v1.5.0-w1-static-review-2026-06-01.md` records fixes for section indexes after dropped unknown blocks, navigation stale-data handling, mixed claim+snapshot evidence, raw fallback DOM attributes, and render-orchestrator doc drift. |
| Surface design drift | `dos217_surface_drift_lint_test` passes after documenting the W1 command in the ability surface allowlist. |
| Signal policy drift | `dos235_signal_policy_registry_lint_test` passes with the W1-B channel inventory restored under `.docs/plans/wave-W1/`. |
| Provider clock/RNG lint | `dos259_lint_wiring_test` passes after replacing the Glean provider debug filename timestamp with a process-local atomic counter. |
| Claim writer discipline | `dos7_d4_lint_test` passes after routing recommendation feedback metadata mutation through `services::claims` and adding explicit migration/test exceptions. |
| Schema 274 dependency | PR #427 merged to `dev` as `cc0f5bf4`; W1 was fast-forwarded to that base so replica DBs at schema 274 open cleanly. |
| L2 cycle 1 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-1-2026-06-01.md` records the blocking findings and the applied fixes for shared projection service ownership, versionless singleflight, provenance rendering, Rust-anchored renderer exhaustiveness, and pending routed proof. |
| L2 cycle 2 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-2-2026-06-02.md` records the request-changes findings and remediations for missing-account rejection, `dailyos/text` generic fallback rendering, parent/detail provenance status resolution, and cache-hit ability metrics. |
| L2 cycle 3 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-3-2026-06-02.md` records the request-changes findings and remediations for account-object projection allowlisting and synchronous stale account composition hiding. |
| L2 cycle 4 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-4-2026-06-02.md` records the request-changes findings and remediations for first-load error surfacing and the class-wide removal of account-overview composite-root projection admissions. |
| L2 cycle 5 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-5-2026-06-02.md` records the request-changes findings and remediations for visible account snapshot degradation and stale-error navigation loading. |
| L2 cycle 6 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-6-2026-06-02.md` records the request-changes finding and remediation for rendering correction affordances from feedback-allowed composition edit routes. |
| L2 cycle 7 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-7-2026-06-02.md` records the request-changes finding and remediation for routing recommendation claims into composed Account work sections with feedback bindings intact. |
| L2 cycle 8 remediation | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-8-2026-06-02.md` records the request-changes finding and remediation for preserving block-level trust/freshness metadata on recommendation and commitment ActionList blocks. |
| L2 cycle 9 pass | `.docs/reviews/v1.5.0-w1-l2-codex-cycle-9-2026-06-02.md` records the passing L2 verdict after re-reviewing the staged W1 diff. Reviewer-side Rust and Vitest reruns were blocked by local write permissions, while TypeScript passed and prior focused Rust/Vitest validations remain the remediation proof. |
| Child composition provenance | `prepare_meeting_source_asof_from_child_composition_is_reachable` passes after preserving non-redacted child entity-intelligence envelope sources on composed provenance, so parent compositions can still reach child `source_asof` evidence. |
| Supplemental routed render artifact | `.docs/proofs/v1.5.0/screenshots/w1-browser-mocked-account-proof.json` records desktop/mobile route checks for the W1 Account page with generic mock data: all 11 section ids, projected block dispatch, visible fallback banner, overlay absence, no overflow, and clean page/console error arrays. |

## Intelligence Loop Check

| Question | W1 answer |
|---|---|
| Claim model | W1 does not add new claim tables or claim types. It consumes existing active claims and only uses snapshot fields through explicit sensitivity/provenance wrappers. |
| Provenance + trust | Blocks preserve claim refs, field bindings, provenance refs, trust bands, source freshness, and degraded fallback state. React renders trust/provenance; it does not compute or invent it. |
| Signals + invalidation | Composition versions, cache keys, fallback-policy versions, and route/command cache behavior remain substrate-owned. Signal policy inventory covers the W1 channel surface. |
| Runtime + surfaces | Tauri consumes through `invoke()`; the W0 local-loopback route remains active; MCP/headless surfaces are not expanded in W1. |
| Feedback loop | Claim-backed blocks retain claim/field targets for existing correction, dismissal, corroboration, and contradiction flows. Display-only intelligence is not introduced. |

## Open Before W1 PR

| Gate | Status | Notes |
|---|---|---|
| Routed visual/L4 proof | Blocked by local macOS UI automation/capture access | Need live routed Account evidence with generic fixture data across desktop/mobile, including dense, sparse, stale/trust-caution, parent, and unknown-fallback states per W1 §3.2. Static reference screenshots exist but are not sufficient for PR proof, and browser-only localhost proof is insufficient because the W1 route depends on native Tauri `invoke()`. Native mock-mode startup, DB isolation, proof app launch, and window presence are verified; screenshot capture remains blocked. CoreGraphics reports the proof app window, but Computer Use returns `cgWindowNotFound`, `screencapture -l` returns `could not create image from window`, `screencapture -R` returns `could not create image from rect`, full-screen capture returns a black image, and System Events coordinate automation fails with accessibility/automation errors. Capture still requires user-side permission/tooling help or a different native capture harness. |
| L2 review cycles | Passed in cycle 9 | Cycles 1-8 requested changes and all findings were remediated. Static adjunct findings are also remediated. Cycle 9 found no blocking issues. |
| PR | Pending | Open one W1 PR to `dev` after commit/push. PR must disclose that native visual capture remains blocked even though native startup/window presence and supplemental browser proof are documented. |
