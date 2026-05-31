# v1.4.4a W6 Proof Bundle

**Wave:** W6 release gate for claim-backed briefing and entity-intelligence surfaces
**Status:** Local gate passed; pre-push L2 passed
**Date:** 2026-05-24
**Branch:** `codex/v1.4.4a-w6-release-gate`
**Base:** `public/dev` after W5/W6 rebase (`4f3337df`)

## Scope

W6 is the release-gate pass for v1.4.4a. The goal is not a new surface; the goal is proving the claim-backed read path is safe, bounded, and usable across the surfaces touched by W1-W5:

- Daily Briefing
- Meeting Briefing
- entity detail pages
- suggested actions and work/email-adjacent reads
- release-gate fixtures for foreground DB contention and no-bypass claim rendering

## Subagent Findings Closed

| Reviewer | Finding | Disposition |
|---|---|---|
| Security | Action-derived synthetic open-loop claims could become MCP-visible without a backing claim sensitivity decision. | Closed by suppressing synthetic action open-loop claim synthesis for MCP surfaces unless the item already has backing claim semantics. Added regression coverage. |
| Security | `noise_reason` logging could persist raw model rationale text. | Closed by logging only the boolean `is_noise` decision and whether a reason was present. |
| Performance | Daily Briefing expanded every meeting before pagination, creating avoidable claim/prep reads. | Closed by expanding only the current meeting, next meeting, and the requested upcoming page. Added bounded expansion regression. |
| Performance | Entity intelligence loaded all active context claims before applying a display cap. | Closed by adding a limited claim reader, a SQL anti-join dismissal path, and a v263 SubjectRef expression index so the service reads bounded rows. Added capped-reader and query-plan regressions. |
| Performance / Adversarial | The limited entity-context reader still applied the cap once per related subject, then globally truncated in memory. | Closed by routing limited related-subject reads through one batched SQL query with a single global `ORDER BY created_at DESC LIMIT`. Added a regression with multiple child subjects. |
| Adversarial | Agent/MCP prompt-safe claims could disappear when non-prompt-safe claims filled the SQL page before actor filtering. | Closed by adding a prompt-safe limited reader that applies sensitivity filtering before the cap for Agent/MCP reads. Added regression coverage with newer confidential claims ahead of an older internal claim. |
| Performance | Email refresh events could stack overlapping foreground reloads. | Closed by adding in-flight coalescing and debounced silent refresh. |
| Testing | The W6 fixture script still pointed at an old WordPress-era gate. | Closed by wiring the release gate to `scripts/release-gate/run-v144a-w6-fixtures.sh` and the v1.4.4a fixture catalog. |
| Testing | Prompt-safe-before-cap producer wiring covered MCP but not plain Agent. | Closed by parameterizing the producer regression across `Actor::Agent` and `Actor::McpClient`, and including that test in fixture 04. |
| Security | Cargo-filtered fixture commands could pass while running zero tests. | Closed by adding a filtered-test helper that fails closed when a fixture filter does not resolve to a real test. |
| Testing | Suite P evidence was smoke-only. | Closed by running published Suite P against `.docs/perf/baselines/scope-v1.4.1-W8.json`; 3/3 manifest benches passed with zero regressions. |

## Implementation Evidence

- `scripts/release-gate/run-v144a-w6-fixtures.sh` runs the v1.4.4a W6 local gate.
- `scripts/suite-s.sh` now builds the MCP stub before clippy, runs the active writer-path lint, and invokes `cargo audit` against `src-tauri/Cargo.lock`.
- Daily Briefing now sorts meetings once and bounds expensive expansion to visible/current records.
- `get_entity_intelligence` now calls bounded claim readers, including prompt-safe pre-cap reads for Agent/MCP actors.
- The live claims service uses globally bounded related-subject surface reads, anti-joins `claim_surface_dismissals`, and filters prompt-unsafe sensitivities before Agent/MCP page caps.
- v263 adds an expression index for semantic SubjectRef lookups and skips cleanly for legacy partial schemas that do not yet have the full claims table shape.
- MCP action rows no longer synthesize public prompt-visible claim text without a backing claim.
- Email refresh events are debounced and coalesced while a load is already in flight.
- The legacy DOS-412 render-policy migration fixture now includes the minimal `emails` table needed by later migrations.

## Validation

All commands below passed locally from the W6 worktree:

```bash
cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib daily_briefing_expansion_ids_are_bounded_to_visible_page_current_and_next
cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib daily_briefing_producer_expands_only_current_next_and_requested_page
cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib daily_briefing_entity_sections_omit_open_loops_for_aggregate_pass
cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib upcoming_meetings_cursor_roundtrip
cargo test --manifest-path src-tauri/Cargo.toml --lib action_open_loop_synthesis_is_not_mcp_visible_without_claim_sensitivity
cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_surface_limited_reader_caps_visible_claims
cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_surface_limited_reader_applies_global_cap_across_related_subjects
cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_prompt_claim_reader_filters_sensitivity_before_cap
cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_subject_lookup_uses_expression_index
cargo test --manifest-path src-tauri/Cargo.toml --lib migration_263_adds_claim_subject_lookup_index
cargo test --manifest-path src-tauri/Cargo.toml --lib mcp_action_db_reader_filters_prompt_unsafe_claims_before_page_cap
cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib agent_and_mcp_producer_read_prompt_safe_claims_before_page_cap
cargo test --manifest-path src-tauri/Cargo.toml --features release-gate --test release_gate_hermetic_test
cargo test --manifest-path src-tauri/Cargo.toml --test dos412_render_policy_test
cargo test --manifest-path src-tauri/Cargo.toml --test dos168_mcp_v2_migration_smoke_test
pnpm tsc --noEmit
pnpm test -- src/pages/EmailsPage.test.tsx src/services/entity-intelligence/invoke.test.ts
scripts/release-gate/run-v144a-w6-fixtures.sh
scripts/suite-s.sh --scope v1.4.4a-W6 --out src-tauri/target/release-gate/v1.4.4a-suite-s.json
scripts/suite-e.sh --scope v1.4.4a-W6 --out src-tauri/target/release-gate/v1.4.4a-suite-e.json
scripts/suite-p.sh --mode published --scope v1.4.4a-W6 --run-id v1.4.4a-w6-published --out .docs/perf/runs/v1.4.4a-w6-published/record.json --baseline .docs/perf/baselines/scope-v1.4.1-W8.json
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1
```

Suite notes:

- Suite S: passed.
- Suite E: passed.
- Suite P: published mode passed; bench count 3, missing 0, regressions 0, comparator `.docs/perf/baselines/scope-v1.4.1-W8.json`.
- Pre-push L2: migration, security, performance, adversarial, and testing reviewers passed after the final global-cap and Agent/MCP producer-test fixes.
- Rust lib suite: passed single-threaded after the DOS-412 legacy fixture drift fix (`2846 passed`, `11 ignored`). A normal full `cargo test --manifest-path src-tauri/Cargo.toml` attempt terminated with SIGBUS during the lib harness before reporting a deterministic assertion failure; the lib suite was rerun single-threaded to completion, and the focused release-gate integration test passed.
- Post-rebase pre-commit surfaced parallel-test interference in key-rotation/global DB-service tests; fixed with a test-only rotation lock. Targeted parallel rotation cluster passed with `cargo test --manifest-path src-tauri/Cargo.toml --lib rotation -- --test-threads=8`.

## Release-Gate Verdict

W6 closes the known local release-gate gaps for v1.4.4a:

- The main claim-backed surfaces render through bounded readers.
- MCP prompt exposure does not get synthetic action text without backing claim sensitivity.
- The foreground email refresh path avoids overlapping reload storms.
- The release-gate fixtures now validate the v1.4.4a surfaces rather than stale WordPress setup work.

Residual maintenance item: add route-level Suite P benchmarks for claim-backed Tauri surfaces beyond the current substrate benchmark manifest.
