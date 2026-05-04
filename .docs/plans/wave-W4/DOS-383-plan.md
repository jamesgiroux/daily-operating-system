---
v1 (2026-05-04) — initial L0 draft

# Implementation Plan: DOS-383

## Revision history
- v1 (2026-05-04) — initial L0 draft; repo citations refined after the required first write.

## 1. Contract restated

DOS-383 adds the external client replay framework that ADR-0104's `ExecutionMode::Evaluate` requires. Linear scope: (1) `ExternalReplayFixture` trait + JSON schema with request-keyed fixtures at repo-local `src-tauri/tests/fixtures/<bundle>/external_replay.json` (Linear shorthand: `tests/fixtures/<bundle>/external_replay.json`); (2) fixture-only client constructors replacing live `ExternalClients` in Evaluate; (3) lint `scripts/check_no_live_external_clients_in_eval.sh` blocking `reqwest::Client::new`, `Client::builder()`, live Glean/Slack/Gmail/REDACTED/Google constructors, `std::net`, and live-only typed client constructors in test/eval paths; (4) request-key canonicalization with deterministic hashing of HTTP method + URL + whitelisted headers + body; (5) integration into `ServiceContext::new_evaluate` call sites.

ADR pins: ADR-0104 lines 65-74 define mode-aware Glean/Slack/Gmail/REDACTED wrappers; lines 207-209 require external calls in Simulate/Evaluate to route to replay fixtures. DOS-216 consumes DOS-383 as a hard prerequisite (`.docs/plans/wave-W4/DOS-216-plan.md:18`, `:26`, `:53`, `:63`). Missing replay must fail closed via typed `ExternalReplayFixtureMissing`, never fall through to Live.

Mirror pattern: `ProviderError::ReplayFixtureMissing` (`src-tauri/src/intelligence/provider.rs:212-215`) and `ReplayProvider`'s lookup/miss path (`:258-263`, `:314-326`) already prove this shape for IntelligenceProvider replay. DOS-383 applies the same fail-closed shape to external HTTP clients.

## 2. Approach

Current substrate: `src-tauri/src/services/context.rs:184-225` has `ExternalClients { glean, slack, gmail, REDACTED }` handles. `ServiceContext::new_evaluate` currently accepts `&ExternalClients` and only documents a caller invariant that those handles are replay fixtures (`:373-387`). DOS-383 closes that gap by adding `ExternalClients::from_replay(...)` and updating eval fixture construction sites, including `src-tauri/src/bridges/eval.rs:124-127`, to pass fixture-only handles instead of bare default/live handles.

New module `src-tauri/src/services/external_replay/`:
- `mod.rs`: `ExternalReplayFixture` trait, `ExternalReplayFixtureMissing` typed error, fixture map loader
- `key.rs`: request-key canonicalization
- `schema.rs`: JSON schema load + validation
- `glean.rs`: Glean replay adapter
- `google.rs`: Gmail plus shared Google API/Drive/Calendar/Auth call surfaces
- `slack.rs`: Slack replay adapter; likely no current live constructor found in grep, but ADR reserves the wrapper
- `REDACTED.rs` or REDACTED-local naming equivalent: REDACTED/REDACTED replay adapter

Linear and Clay are not in ADR-0104's `ExternalClients` set today. Keep them in the lint inventory and add replay adapters only if an Evaluate ability actually routes through them.

Trait shape:
- `ExternalReplayFixture::lookup(&self, key: &RequestKey) -> Result<ReplayResponse, ExternalReplayFixtureMissing>`
- `ReplayResponse` owns status + headers + body
- `RequestKey` includes auth-scope id plus canonical request hash

Request key canonicalization (`key.rs`):
- HTTP method uppercased ASCII
- URL with query params sorted lexicographically
- whitelisted headers lowercased + sorted: `User-Agent`, `Content-Type`, `Accept`
- volatile headers stripped: `Date`, `Authorization`, `X-Request-Id`, trace ids, etc.
- auth-scope id (workspace/tenant) prefixed for cross-tenant collision prevention
- body bytes SHA-256
- final hash: SHA-256 of concatenated canonical bytes

`ExternalClients::from_replay` returns replay wrappers. Live constructors fail compile in the hermetic test crate via `#[cfg(not(any(test, feature = "harness-hermetic")))]` gating where practical, backed by the lint where compile gating cannot reach legacy modules.

Lint script `scripts/check_no_live_external_clients_in_eval.sh` greps in `src-tauri/tests/` and `src-tauri/src/abilities/**/*.rs` evaluate paths for:
- `reqwest::Client::new` and `Client::builder()`
- `GleanMcpClient::new`, `LinearClient::new`, `ClayClient::new`
- Slack/Gmail/REDACTED/Google live SDK constructor names that appear
- `std::net`, `TcpListener`, `TcpStream`, `UdpSocket`

Concrete live HTTP inventory from the requested `rg` pass:
- Google: `google_api/gmail.rs:112,185,340,424,485,544`; `google_drive/client.rs:37,110,147`; `google_api/calendar.rs:119,259`; `google_api/auth.rs:83,179`; `google_api/mod.rs:380`
- Glean: `context_provider/glean.rs:123,407`; `glean/oauth.rs:125,132,451,566`; `glean/mod.rs:145`
- Other live clients: `linear/client.rs:70`, `clay/client.rs:186`, `commands/integrations.rs:1402,3048`, `services/emails.rs:1197`
- `std::net::TcpListener`: `google_api/auth.rs:10`, `glean/oauth.rs:14`

`ServiceContext::new_evaluate(...)` can keep receiving `ExternalClients` if that remains the local substrate style, but all Evaluate construction must obtain it from `ExternalClients::from_replay`. Bare `ExternalClients::default()` in eval tests is acceptable only for tests that prove no external calls are reachable.

## 3. Key decisions

Request key includes auth-scope id (workspace/tenant). Suite-S should spot check that cross-tenant replay collision is structurally impossible.

Header whitelist is intentionally narrow. Volatile headers are stripped at canonicalization time so fixtures remain reproducible across recordings.

Fixture format is JSON-schema-validated once at load time, not per request.

`ExternalClients::from_replay` errors on unknown client types. The harness must declare the full set of clients it expects to replay.

Capture mode is opt-in CLI tooling, never CI. Identity-map redaction is written out-of-tree and auth tokens are never serialized to fixtures.

Local docs mention DOS-307 as folded into DOS-294 and DOS-308 as tombstone/quarantine work; neither is an external-client-hardening dependency unless Linear says otherwise during implementation.

## 4. Security

Cross-tenant replay collision is prevented by including auth-scope id in the request key. Test `external_replay_request_key_includes_auth_scope_for_tenant_isolation` locks this in.

Auth tokens never serialize to fixtures; capture mode redacts at recording time.

Lint blocks `std::net` + live constructors in eval paths so a typo in a new ability cannot silently call a real external service during harness runs.

The lint must cover raw reqwest constructors and typed wrapper constructors because current live code is split across Google, Glean, Linear, Clay, command, and service modules rather than centralized behind `ExternalClients`.

## 5. Performance

Replay lookup is in-memory `HashMap` O(1) keyed by request hash. Fixture parse cost is paid at harness startup and amortized over the fixture set.

Recommended per-response body cap is <=1MB so fixtures do not become a memory or diff-review footgun.

## 6. Coding standards

Services-only mutations hold: the replay layer never writes DB rows, files, signals, queues, or external systems.

Intelligence-loop 5-question check is N/A: replay is test infrastructure, not product surface.

## 7. Integration with parallel wave-mates

DOS-216/W4-B is the only direct consumer; its plan explicitly names DOS-383 as the hard prerequisite for external replay hermeticity.

DOS-217's `EvalAbilityBridge` indirectly benefits because `fixture_services` should carry replay-backed `ExternalClients`, not live/default handles.

ADR-0104 lines 65-74 + 207-209 alignment is required for merge.

## 8. Failure modes + rollback

Missing replay fixture: typed `ExternalReplayFixtureMissing` with full request key in the error message; never falls through to Live.

Lint regression: rollback is mechanical (revert script + re-run CI).

Capture mode failure: candidate fixture deleted, identity map preserved out-of-tree.

W1-B universal write fence honored: replay layer never writes to DB, files, signals, or external systems.

## 9. Test evidence to be produced

- `external_replay_fixture_loads_from_json`
- `external_replay_missing_returns_typed_error_with_request_key`
- `external_replay_request_key_canonicalization_is_deterministic`
- `external_replay_request_key_includes_auth_scope_for_tenant_isolation`
- `external_replay_request_key_strips_volatile_headers`
- `external_replay_request_key_sorts_query_params`
- `lint_blocks_live_reqwest_client_in_test_crate`
- `lint_blocks_live_reqwest_builder_in_test_crate`
- `lint_blocks_live_glean_constructor_in_test_crate`
- `lint_blocks_live_slack_constructor_in_test_crate`
- `lint_blocks_live_gmail_constructor_in_test_crate`
- `lint_blocks_live_google_drive_calendar_auth_constructors_in_test_crate`
- `lint_blocks_live_REDACTED_constructor_in_test_crate`
- `lint_blocks_live_linear_and_clay_constructors_in_eval_paths`
- `lint_blocks_std_net_in_test_crate`
- `service_context_new_evaluate_wires_replay_fixture`
- `harness_replay_miss_propagates_typed_error_via_external_clients`

## 10. Open questions

1. Should Linear/Clay be explicit replay adapters now, or lint-only until an Evaluate ability reaches those clients?
2. Should auth-scope id default to workspace path for local DailyOS, or require explicit fixture-scoped auth-scope?
3. Header whitelist: confirm `User-Agent` / `Content-Type` / `Accept` is sufficient; some integrations may need more.
---
