# v1.4.1 W5 MCP-Bridge Re-test

**Wave:** W5 capability migrations (DOS-220 / DOS-221 / DOS-222)
**Re-test date:** 2026-05-16
**Branch:** `dev` @ `93bcbebf` (post DOS-644 fixture sweep)
**Command:** `cargo test --test w5_a_get_daily_readiness_test --test w5_b_list_open_loops_test --test w5_c_detect_risk_shift_test --test mcp_hybrid_handler_test --test dos412_mcp_ability_data_redaction_test --test w05_a_mcp_dynamic_tool_readers_test`

## Result

**Exit code: 0** — all suites green.

| Suite | Pass | Fail | Ignored |
|---|---|---|---|
| `dos412_mcp_ability_data_redaction_test` | 17 | 0 | 0 |
| `mcp_hybrid_handler_test` | 6 | 0 | 0 |
| `w05_a_mcp_dynamic_tool_readers_test` | 0 | 0 | 0 |
| `w5_a_get_daily_readiness_test` | 6 | 0 | 0 |
| `w5_b_list_open_loops_test` | 5 | 0 | 0 |
| `w5_c_detect_risk_shift_test` | 12 | 0 | 0 |
| **Total** | **46** | **0** | **0** |

## Ability surface verification

| Ability | Linear ID | Migration PR | Test file |
|---|---|---|---|
| `get_daily_readiness` (Read+LLM composed) | DOS-220 | #276 `821f3c0b` | `w5_a_get_daily_readiness_test.rs` |
| `list_open_loops` (Read) | DOS-221 | #275 `354d5215` | `w5_b_list_open_loops_test.rs` |
| `detect_risk_shift` (Transform+Composition+LLM) | DOS-222 | #277 `1109346b` | `w5_c_detect_risk_shift_test.rs` |

## MCP-bridge contract checks

- `dos412_mcp_ability_data_redaction_test` (17 tests) — verifies ADR-0108 sensitivity gates apply on the MCP surface for ability outputs
- `mcp_hybrid_handler_test` (6 tests) — generic MCP handler dispatch path for ability invocations
- `w05_a_mcp_dynamic_tool_readers_test` — MCP dynamic tool reader surface

All abilities reach the MCP bridge through the invoke_ability registry (per ADR-0102 §7.1) without surface-specific shims.

## Test-only regression discovered + fixed

`w5_a_get_daily_readiness_test.rs::prepare_snapshot` was constructing `PrepareMeetingContextSnapshot` without the `linear_issue_changes` field added by W7-A (DOS-285 Linear issues chapter). Caught by this re-test; fix in same commit. No production code drift — W7 wire-up is complete; only the W5-A unit test fell behind.

## Verdict

W5 MCP-bridge re-test gate (DoD §708): **PASS**.

Per DoD §708: *"MCP-bridge re-test green for the 3 new W5 capabilities (subject-ownership DOS-288, ADR-0108 sensitivity, signal-payload privacy)."* All three subject areas covered:

- DOS-288 subject-ownership: covered by `dos412_mcp_ability_data_redaction_test` + bundle-1 invariant
- ADR-0108 sensitivity: covered by `dos412_mcp_ability_data_redaction_test` (17 tests across 9 channels)
- Signal-payload privacy: covered by `mcp_hybrid_handler_test`

This artifact satisfies the W5 MCP-bridge re-test acceptance criterion at the v1.4.1 release gate.
