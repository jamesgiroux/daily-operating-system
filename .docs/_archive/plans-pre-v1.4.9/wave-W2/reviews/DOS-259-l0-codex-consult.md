# L0 review — DOS-259 plan — codex consult mode

**Reviewer:** /codex consult
**Plan revision:** v1 (2026-04-28)
**Verdict:** REVISE

## Findings

### F1 — §9 omits required parity and concurrency evidence (severity: High)
The plan names fixture replay and two byte-identical parity tests, but it does not name a test for concurrent `.complete()` invocations and does not name the Trust Compiler shadow parity test required by the ticket's unchanged-behavior acceptance criterion.
Location in plan: §9 "Add `replay_provider_returns_canned_completion`, `evaluate_mode_never_invokes_live_provider`, `pty_claude_code_fixture_returns_expected_fingerprint_metadata`, `glean_provider_fixture_returns_expected_fingerprint_metadata`, `provider_selection_is_single_source_for_tier`, `manual_enrichment_parity_dev_fixture_byte_identical`, and `meeting_prep_parity_dev_fixture_byte_identical`."
What needs to change: Add specific test names for concurrent-invocation safety and Trust Compiler shadow byte-identical parity, e.g. `provider_complete_concurrent_invocations_all_succeed` and `trust_compiler_shadow_parity_dev_fixture_byte_identical`.

## Summary
REVISE: the implementation path is otherwise coherent, but §9 does not prove two explicit contract surfaces before coding. The gap is narrow and fixable in the plan without changing the architecture.

## Strengths
§1 correctly frames DOS-259 as a pure extraction refactor with no behavior change, and §2 covers the trait, `Completion`, PTY/Glean migration, caller `.text` migration, provider selection, and replay provider shape. §3 is principled for downstream v1.4.0 consumers, §6 correctly treats the Intelligence Loop schema checklist as N/A for this no-schema refactor, §7 captures the real DOS-209 coordination cost, and §10 surfaces real open questions rather than sneaking in major decisions.

## If REVISE
1. Add named §9 tests for Trust Compiler shadow byte-identical parity and concurrent `.complete()` safety.
