# v1.5.0 Wave 2 - Proof Bundle

**Wave:** W2 - In-context edit and customize
**Date:** 2026-06-02
**Branch:** `codex/v1.5.0-w2`
**Base:** `public/dev` after W1 merge (`8255e20c`)

## Issue Map

| Issue | Scope | Evidence |
|---|---|---|
| W2-0 | Edit-mode pattern specs first | `.docs/design/patterns/CompositionEditMode.md`, `.docs/design/patterns/CompositionBlockToolbar.md`, `.docs/design/patterns/CompositionInserter.md`, `.docs/design/patterns/CompositionReorderHandle.md`, `.docs/design/patterns/CompositionInlineEdit.md` |
| W2-A | Layout overlay persistence | `src-tauri/src/migrations/275_composition_layout_overlays.sql`, `src-tauri/src/services/composition_layout.rs`, `src-tauri/src/commands/composition_layout.rs` |
| W2-B | First-party commands and reactive hook | `src-tauri/src/commands/composition_layout.rs`, `src/hooks/useChapterLayout.ts`, `src/services/composition/layoutOverlay.ts` |
| W2-C | Visibility precedence and core blocks | `src/hooks/useChapterLayout.ts`, `src/hooks/useChapterLayout.test.tsx` |
| W2-D | In-context Account edit mode | `src/pages/AccountDetailPage.tsx`, `src/pages/AccountDetailPage.module.css`, `package.json`, `pnpm-lock.yaml` |
| W2-E | Inline edit through feedback routes | `src/components/composition/CompositionInlineEdit.tsx`, `src/components/composition/blocks/BlockComponents.tsx`, `src/hooks/useIntelligenceCorrection.ts`, `src-tauri/src/services/feedback.rs` |
| W2-F | Settings Surfaces secondary panel | `src/pages/SettingsPage.tsx`, `src/pages/SettingsPage.module.css` |
| W2-G | Proof, L2, PR | This file; `.docs/reviews/v1.5.0-w2-l2-codex-cycle-1-2026-06-02.md`; `.docs/reviews/v1.5.0-w2-l2-codex-cycle-2-2026-06-02.md`; PR still pending |

## Acceptance Evidence

- Pattern docs define the canonical edit-mode shell, block toolbar, inserter, reorder handle, and inline edit behavior before the runtime implementation. The pattern index now links all five W2 patterns.
- Migration 275 creates `composition_layout_overlays` with primary key `(entity_type, surface_key)`. `overlay_schema_version` describes the JSON format and `layout_revision` is service-owned presentation/cache state; neither field is part of identity.
- Layout overlay mutations route through `services::composition_layout`. Tauri command handlers validate/forward inputs and do not write directly to the database.
- The layout service validates entity type, surface key, overlay JSON object shape, schema version, id format/length, payload size, entry count, variants, and control-character-free labels. Tests cover create/read/update/reset, invalid payloads, payload bounds, and idempotent migration replay.
- `useChapterLayout` merges `ProjectedComposition + LayoutOverlay` without changing the producer composition version. It filters stale ids, keeps same-entity-type preferences reusable across Account routes, and maintains `layout_revision` as presentation state.
- Core content is locked visible: `headline`, `masthead`, `lead`, and `account_overview` cannot be hidden by overlay state. Core section block order stays producer-owned even if stale overlay order exists.
- Non-core blocks and sections support hide/show, re-add, variants, and presentation labels. Empty non-core layouts retain core content plus reset/re-add affordances.
- Optimistic layout mutations are latest-wins. Older save/reset responses cannot overwrite a newer local mutation, and failed latest mutations roll back to the prior view.
- Account Detail adds `Customize` / `Done`, dnd-kit mouse and keyboard sorting, block toolbars, visibility switches, variant controls, locked explanations, an inserter for hidden blocks, and reset-to-default.
- Claim-backed inline editing appears only for projected edit routes with `feedback_allowed: true` and a claim ref. Committed edits send entity id/type, field binding, claim id, current value, corrected value, and `composition_inline_edit` source through the existing feedback/correction path.
- Display-only or non-feedback routes do not expose inline editing. Presentation labels remain overlay preferences; claim text edits do not write into overlay JSON.
- Settings adds a narrow Surfaces chapter for Account layout reset/summary without introducing W5 lifecycle-stage configuration or a settings redesign.
- The W2 commands are documented in `src-tauri/scripts/ability_surface_allowlist.txt`, and the existing surface drift lint passes.
- The existing Glean finalization parity test now normalizes floating-point health projection values before exact JSON comparison. This is a verification-gate stabilization only; it does not change runtime behavior.

## Validation Log

| Check | Command | Result |
|---|---|---|
| Diff whitespace | `git diff --check` | Pass |
| Frontend typecheck | `pnpm tsc --noEmit` | Pass |
| Frontend tests | `pnpm test` | Pass: 59 files, 318 tests |
| W2 hook and inline edit tests | `pnpm vitest run src/hooks/useChapterLayout.test.tsx src/components/composition/CompositionInlineEdit.test.tsx` | Pass after L2 remediation: 2 files, 14 tests |
| W2 hook L2 remediation regressions | `pnpm vitest run src/hooks/useChapterLayout.test.tsx` | Pass: 1 file, 11 tests |
| Layout overlay service tests | `cargo test --manifest-path src-tauri/Cargo.toml composition_layout -- --nocapture` | Pass: 4 tests |
| Inline correction Rust regression | `cargo test --manifest-path src-tauri/Cargo.toml composition_inline_correction_uses_current_value_snapshot -- --nocapture` | Pass |
| Surface drift lint | `cargo test --manifest-path src-tauri/Cargo.toml --test dos217_surface_drift_lint_test` | Pass |
| Rust clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Pass |
| Full Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` | Pass: main lib `3129 passed; 0 failed; 11 ignored`; integration and doc tests completed cleanly |

Note: the full Rust test run emits pre-existing unused-import warnings in `tests/dos567_fixture_backfill_and_composition_versions.rs`; they do not fail clippy or tests.

## Replica Smoke

The dev build path discipline was checked before W2 proof because the replica/production migration issue was recently fixed in a parallel session.

| Check | Evidence |
|---|---|
| Debug mode default | `src-tauri/src/db/core.rs` resolves debug default to `DbMode::Replica` when unset |
| Replica DB path | `~/.dailyos/dailyos-replica.db` |
| Production DB path | `~/.dailyos/dailyos.db` |
| Non-live guard | `guard_path_for_mode` denies opening the production DB path in non-live modes |
| Before smoke stat | Replica `49778688 1780425992`; production `54992896 1780425876` |
| Smoke command | `DAILYOS_DB_MODE=replica cargo run --manifest-path src-tauri/Cargo.toml --bin workspace_graph_audit -- --format human --fail-on-gaps=false` |
| Smoke output | `workspace graph audit`; `gap_count: 0` |
| After smoke stat | Replica `49778688 1780425992`; production `54992896 1780425876` |

The smoke opened the replica-mode path and did not mutate the production database. No migration was pending in that command path, as shown by unchanged mtimes.

## Focused Coverage

| Area | Evidence |
|---|---|
| Overlay identity | Service tests and migration prove the key is only `(entity_type, surface_key)` and migration 275 is replay-safe. |
| Input validation | Rust tests reject invalid entity types, surface keys, non-object overlay JSON, schema-version mismatches, unsupported variants, oversized payloads, oversized entry sets, invalid ids, and invalid labels. |
| Command ownership | Commands are in `commands::composition_layout`; persistence code is in `services::composition_layout`. |
| Latest-wins optimistic mutations | `useChapterLayout.test.tsx` covers failed older save after newer mutation and older reset response after newer save. |
| L2 cycle 1 remediation | `.docs/reviews/v1.5.0-w2-l2-codex-cycle-1-2026-06-02.md` records blocking AC findings for stale initial load overwrite and overlapping failed save rollback. |
| L2 cycle 2 pass | `.docs/reviews/v1.5.0-w2-l2-codex-cycle-2-2026-06-02.md` records the targeted pass after adding persisted-baseline rollback and stale-load mutation guards. |
| Route reuse | Hook tests cover account-to-account reuse for the same entity type while stale overlay ids disappear when the projection changes. |
| Core lock | Hook tests cover locked sections/blocks and stale attempts to hide core content. |
| Empty reset state | Hook tests cover hidden non-core content while core blocks remain visible and reset affordance state remains available. |
| Inline feedback path | React and Rust tests cover current-value feedback submission and backend persistence of `previous_value`, `corrected_value`, and `source_system`. |

## Intelligence Loop Check

| Question | W2 answer |
|---|---|
| Claim model | Layout overlays are user presentation preferences, not claims. Claim-backed text edits continue through the feedback/correction path. |
| Provenance + trust | Overlay state does not alter source attribution, source freshness, trust scoring, or sensitivity. Visible blocks keep trust/provenance from the projected composition. |
| Signals + invalidation | Overlay saves update local presentation state only. They do not bump producer composition versions or claim versions. |
| Runtime + surfaces | Tauri Account Detail consumes overlays through first-party commands and `useChapterLayout`. MCP/headless surfaces are unchanged in W2. |
| Feedback loop | Inline edits submit corrections with current and corrected values. Hide/show/reorder/variant choices remain layout preferences and do not create claim dismissals or source-reliability changes. |

## Open Before W2 PR

| Gate | Status | Notes |
|---|---|---|
| L2 review cycles | Passed in cycle 2 | Cycle 1 found two AC blockers in `useChapterLayout`; both were remediated and cycle 2 found no remaining W2 blockers. |
| Routed visual/L4 proof | Pending | W2 has unit/integration coverage for edit mode contracts. Native/browser proof for hide, reorder, variant, inline edit, reset, keyboard reorder, and persistence should be captured before or during PR review if local Tauri/browser tooling permits. |
| PR | Pending | Open one W2 PR to `dev` after L2 passes, commit, and push. |
