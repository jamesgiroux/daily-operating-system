# v1.4.4a W6 Retro

**Wave:** W6 release gate
**Date:** 2026-05-24
**Author:** Codex

## What Went Right

- The review agents found the right classes of issues before PR: security on MCP exposure/logging, performance on over-broad reads and reload storms, and testing on stale release-gate fixtures.
- The fixes stayed close to the existing substrate instead of inventing a cache layer: bound the readers, avoid unnecessary expansion, and keep sensitivity decisions on claim-backed data.
- Full `cargo test --manifest-path src-tauri/Cargo.toml` caught a real migration-fixture drift in the DOS-412 render-policy tests. The focused test had passed, but the full chain found that the old v143 fixture needed a minimal `emails` table for later migrations.
- The published Suite P run replaced the earlier smoke-only evidence and compared the current bench manifest against the v1.4.1 W8 baseline with zero regressions.

## What Went Wrong

- The first W6 fixture script shape was inherited from the old WordPress setup work. That would have created false confidence for v1.4.4a because it did not exercise the briefing/entity surfaces.
- Daily Briefing had a classic hidden cost: the displayed page was small, but the producer was expanding every meeting before pagination. That is the kind of issue that looks correct functionally and only shows up under realistic data volume.
- Entity detail had the same shape: apply a cap after the full read. The correct fix was to push the limit into the service reader.
- The first bounded related-subject implementation still capped per subject. Review caught that a parent with many children could still materialize too many rows, so the final fix uses one global SQL-bounded related-subject read.
- The first claim-subject index migration was too optimistic for legacy partial schemas. The DOS-412 drift fixture forced the right shape: guard the migration by actual columns, then create the index only when the full claim table exists.
- The first post-L2 bounded claim reader still filtered Agent/MCP prompt safety after the page cap. That could hide older safe claims behind newer confidential ones, so the final shape pushes prompt-safe filtering into the service query before limiting.

## Decisions

- Do not add a separate read cache database in W6. The immediate bottleneck was unbounded local reads and redundant expansion, not a missing architecture layer.
- Do not expose synthetic action open-loop claim text to MCP without backing claim sensitivity. Tauri can show useful local action rows, but MCP/tool contexts need explicit claim/provenance semantics.
- Treat route-level Suite P benchmarks as maintenance hardening after W6. The published substrate suite passes and the W6 regressions are covered, but route baselines should be added as their own bounded task.

## Follow-Up

- File maintenance work for route-level Suite P benchmarks across Daily Briefing, Meeting Briefing, entity detail, actions, and emails.
- Keep the W6 fixture runner current as W7 or follow-up work extends more surfaces.

## Final State

- W6 local release gate: passed.
- Suite S: passed.
- Suite E: passed.
- Suite P published: passed.
- TypeScript check: passed.
- Rust lib suite: passed single-threaded (`2846 passed`, `11 ignored`).
- Release-gate integration test: passed.
- Pre-push L2: passed after global-cap and Agent/MCP producer-test fixes.
- Post-rebase rotation-test parallelism drift: fixed with a test-only lock; targeted rotation cluster passed under parallel test threads.
