# v1.5.0 Wave 0 — Proof Bundle

**Wave:** W0 — WordPress surface banking + ADR status lock
**Date:** 2026-06-01
**Branch:** `codex/v1.5.0-w0-l1`

## Issue Map

| Issue | Scope | Evidence |
|---|---|---|
| DOS-835 | Bank the reverted WordPress block surface and retire block-presence CI gates while preserving shared Composition substrate | `.docs/_archive/banked-wordpress-surface-2026-06-01/`, `.github/workflows/block-kit-integration.yml`, `.github/workflows/lint-frontend.yml`, `src-tauri/src/surface_runtime/mod.rs` |
| DOS-836 | Confirm the Composition contract is surface-generic and classify remaining assumptions | `.docs/plans/v1.5.0-w0-substrate-genericity-audit.md` |
| DOS-837 | Accept ADR-0130 and ADR-0122 Option A | `.docs/decisions/0130-surface-independent-composition-contract.md`, `.docs/decisions/0122-chapter-enrichment.md` |

## Acceptance Evidence

- WordPress block surface preserved, not deleted: 89 `block.json` files are banked under `.docs/_archive/banked-wordpress-surface-2026-06-01/wp/dailyos/blocks/`.
- Block-specific PHPUnit fixtures and Rust block-kit integration fixtures are banked beside the blocks, outside active plugin and Rust test paths.
- The obsolete W1 consumer-skeleton gate is removed from `lint-frontend.yml` and preserved under `.docs/_archive/banked-wordpress-surface-2026-06-01/retired-ci/`.
- `block-kit-integration.yml` is converted from per-block fixture CI into a banking invariant: it fails if active WP block files or block-kit fixtures reappear without restoring the CI gates, and verifies the banked block count remains 89.
- `wp-plugin.yml` now excludes `wp/dailyos/blocks/**` and `wp/dailyos/tests/blocks/**`; W0 verifies its grep and chrome-collision gates tolerate the parked destination.
- Shared Composition substrate remains active: `src-tauri/src/services/composition_render_orchestrator.rs` and `/v1/local/project-composition` stay in source.
- Local loopback smoke coverage proves `/v1/local/project-composition` invokes the producer as `Actor::User` and caches the rendered Composition through the `Actor::User` cache path after banking.
- ADR-0130 is `Accepted`; ADR-0122 is `Accepted — Option A`.
- CI lint drift found during full-suite validation was repaired without changing runtime behavior: the signal-policy lint now reads the archived W1 inventory, recommendation metadata writes route through the claim service boundary, DOS-7 fixture writes carry explicit exceptions, and the migration rollback has a durable best-effort rationale.

## DOS-836 Summary

The Composition contract is surface-generic enough for W1. Active producers return `AbilityOutput<Composition>` projected through `ProjectedComposition`; no `block.json`, Gutenberg metadata, PHP render function, or WordPress storage shape is required. Remaining account-overview resolver and composition-id parsing assumptions are W1 generalization work, not W0 blockers.

## Validation Log

| Check | Command | Result |
|---|---|---|
| Banked block count | `find .docs/_archive/banked-wordpress-surface-2026-06-01/wp/dailyos/blocks -name block.json \| wc -l` | 89 |
| Banked invariant | Local execution of `.github/workflows/block-kit-integration.yml` invariant script | Pass: active block files, block-test fixtures, and Rust fixture paths are absent; archive count is 89 |
| Retired skeleton gate | `rg -n "check_w1_consumer_skeleton" .github/workflows/lint-frontend.yml src-tauri/scripts` | Pass: no active matches |
| Shared substrate still active | `test -f src-tauri/src/services/composition_render_orchestrator.rs && test ! -d wp/dailyos/blocks` | Pass |
| Local composition smoke | `CARGO_TARGET_DIR=/tmp/dailyos-v150-w0-target CARGO_BUILD_JOBS=1 cargo test --manifest-path src-tauri/Cargo.toml w0_local_project_composition_route_resolves_composition_as_user -- --nocapture` | Pass: 1 test passed after L2 hardening; proves `/v1/local/project-composition` invokes, resolves, and caches as `Actor::User` |
| WP grep gates | `bash wp/dailyos/scripts/run-grep-gates.sh` | Pass |
| WP chrome collision gate | `bash wp/dailyos/scripts/check-chrome-block-collision.sh` | Pass: skipped cleanly because `wp/dailyos/blocks` is intentionally absent |
| WP PHPUnit | `cd wp/dailyos && ./vendor/bin/phpunit --no-coverage` | Pass: 227 tests, 985 assertions; 4 existing PHPUnit deprecations |
| Frontend typecheck | `pnpm tsc --noEmit` | Pass |
| Diff whitespace | `git diff --cached --check` | Pass |
| Rust clippy | `CARGO_TARGET_DIR=/tmp/dailyos-v150-w0-target CARGO_BUILD_JOBS=1 cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Pass after L2 hardening |
| Rust library tests | `CARGO_TARGET_DIR=/tmp/dailyos-v150-w0-target CARGO_BUILD_JOBS=1 cargo test --manifest-path src-tauri/Cargo.toml --lib` | Pass after L2 hardening: 3093 passed, 0 failed, 11 ignored |
| Rust integration and trybuild tests | `CARGO_TARGET_DIR=/tmp/dailyos-v150-w0-target CARGO_BUILD_JOBS=1 cargo test --quiet --manifest-path src-tauri/Cargo.toml --tests` | Pass before L2 hardening: exited 0 after the library harness, trybuild cases, and integration binaries completed cleanly |

Note: this repository has no root `Cargo.toml`; Rust validation uses the manifest-scoped equivalent under `src-tauri/Cargo.toml`. The post-L2 hardening patch touched test code and CI workflow logic; focused route smoke, clippy, and library tests were rerun after that patch.

## L2 Review Log

| Reviewer | Result | Notes |
|---|---|---|
| Correctness | PASS | No blocking correctness regressions in the W0-scoped diff. |
| API contract | PASS | No blocking API/surface contract findings; residual cache-hit expansion is non-blocking W1+ hardening. |
| Testing | BLOCKED -> PASS | Initial proof gap fixed by asserting the producer receives `AbilityContext.actor == Actor::User`; banked invariant now rejects any active `wp/dailyos/blocks` file, not only `block.json`. |
