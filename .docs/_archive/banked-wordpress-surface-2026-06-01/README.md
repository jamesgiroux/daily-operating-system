# Banked WordPress Surface Work

v1.5.0 W0 parks the WordPress block surface out of the active plugin build and CI path without deleting the work.

- Active block source before W0: `wp/dailyos/blocks/`
- Banked block source after W0: `.docs/_archive/banked-wordpress-surface-2026-06-01/wp/dailyos/blocks/`
- Banked block count: 89 `block.json` files.
- Banked WP block PHPUnit fixtures: `.docs/_archive/banked-wordpress-surface-2026-06-01/wp/dailyos/tests/blocks/`
- Banked block-kit Rust integration fixtures: `.docs/_archive/banked-wordpress-surface-2026-06-01/src-tauri/abilities-runtime/tests/`
- Retired W1 consumer-skeleton gate: `.docs/_archive/banked-wordpress-surface-2026-06-01/retired-ci/`
- Historical plan source: `.docs/_archive/plans-pre-v1.4.9/v1.4.4-wp-surface-migration/`

`.github/workflows/block-kit-integration.yml` is now a banking invariant instead of a per-block fixture runner: it fails if active WordPress blocks or block-kit fixtures reappear without restoring the CI gates, and it verifies the banked block count stays at 89. `.github/workflows/wp-plugin.yml` excludes `wp/dailyos/blocks/**` and `wp/dailyos/tests/blocks/**`; the banked archive path is intentionally outside the active plugin workflow.

The shared composition substrate remains active: `src-tauri/src/services/composition_render_orchestrator.rs` and `/v1/local/project-composition` are used by the v1.5.0 Tauri renderer path.
