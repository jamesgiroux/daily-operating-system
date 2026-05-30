# Packet B Cycle-2 Codex Consult
Verdict: CONDITIONAL APPROVE

## Validation 1 - Producer Commit And Cache Key
Claim recap: V1.1 preserves producer commit on cache miss and fixes the loop by keying cache lookup/store with `current_db_version`.
Evidence: current handler authorizes before cache lookup at `src-tauri/src/surface_runtime/mod.rs:2288`.
Evidence: current cache lookup still passes `request.composition_version` at `src-tauri/src/surface_runtime/mod.rs:2317`.
Evidence: current DB version read is after cache miss at `src-tauri/src/surface_runtime/mod.rs:2355`.
Evidence: producer input uses that DB version at `src-tauri/src/surface_runtime/mod.rs:2371`.
Evidence: producer invocation remains on miss at `src-tauri/src/surface_runtime/mod.rs:2378`.
Evidence: current cache store still passes `request.composition_version` at `src-tauri/src/surface_runtime/mod.rs:2425`.
Evidence: account overview producer commits during ability execution at `src-tauri/abilities-runtime/src/abilities/account_overview.rs:112`.
Evidence: live commit goes through `services::compositions::commit_composition` at `src-tauri/src/services/context.rs:130`.
Evidence: composition commit mutates `composition_versions` and emits a version event at `src-tauri/src/services/compositions.rs:277`.
Evidence: the orchestrator key remains `(composition_id, composition_version, scopes_canonical_id)` at `src-tauri/src/services/composition_render_orchestrator.rs:51`.
Evidence: `cache_lookup` and `cache_store` accept only a version value swap at `src-tauri/src/services/composition_render_orchestrator.rs:102` and `src-tauri/src/services/composition_render_orchestrator.rs:121`.
Verdict: APPROVE WITH PATH CORRECTION.
Finding: the request's `src-tauri/src/services/composition/mod.rs:2355` path is not present in this repo; the matching code is `src-tauri/src/surface_runtime/mod.rs:2355`, with the helper defined at `src-tauri/src/services/compositions.rs:149`.
Finding: implementation is clean: move the existing DB read before `src-tauri/src/surface_runtime/mod.rs:2317`, pass `current_db_version`, and store with `projection.composition_version.unwrap_or(current_db_version)`.

## Validation 2 - Reload Trigger Pattern
Claim recap: V1.1 keeps full `reload` deps and gates auto-reload with a derived `reloadTrigger`.
Evidence: `reload` currently reads `composition_id`, `composition_version`, and `cache_hint_token` in the POST body at `wp/dailyos/blocks/account-overview/edit.js:53`.
Evidence: current `reload` already has the full dependency list at `wp/dailyos/blocks/account-overview/edit.js:82`.
Evidence: current `useEffect([reload])` retriggers when that callback changes at `wp/dailyos/blocks/account-overview/edit.js:84`.
Evidence: successful reload writes `composition_version`, `watermarks`, and `cache_hint_token` at `wp/dailyos/blocks/account-overview/edit.js:62`.
Evidence: V1.1's trigger key is specified at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:293`.
Evidence: V1.1 explicitly excludes version/token from the trigger at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:304`.
Evidence: the packet's grep invariant requires `useEffect([reloadTrigger])` at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:658`.
Evidence: root ESLint enables `react-hooks/exhaustive-deps` as a warning for `src/**/*` at `eslint.config.js:74`; WP has a separate `lint:js` script at `wp/dailyos/package.json:8`.
Verdict: CONDITIONAL APPROVE.
Finding: no infinite render loop if implemented exactly: success writes do not change the proposed trigger, so the effect will not refire from version/token writes.
Finding: exhaustive-deps lint behavior for the WP script is UNVERIFIED; if `wp-scripts lint-js` treats omitted `reload` as blocking, use an explicit suppression with rationale or a ref-based reload runner.

## Validation 3 - Local Render Decharge
Claim recap: V1.1 adds `authorize_local_render` that preserves authorization checks while calling rate limiting with `charge_ability_scope=false`.
Evidence: current public `authorize` delegates to `authorize_for_path` at `src-tauri/src/bridges/surface_client.rs:260`.
Evidence: descriptor lookup, actor/mode/experimental gates, required scopes, and browser-direct guard occur before rate limiting at `src-tauri/src/bridges/surface_client.rs:301`.
Evidence: current `authorize_for_path` hardcodes `charge_ability_scope: true` at `src-tauri/src/bridges/surface_client.rs:387` and `src-tauri/src/bridges/surface_client.rs:409`.
Evidence: the request struct already has a `charge_ability_scope` boolean at `src-tauri/src/bridges/surface_client.rs:642`.
Evidence: identity candidates are always added at `src-tauri/src/bridges/surface_client.rs:766`.
Evidence: scope and ability candidates are gated by `charge_ability_scope` at `src-tauri/src/bridges/surface_client.rs:795`.
Evidence: an existing identity-only helper already constructs requests with `charge_ability_scope: false` at `src-tauri/src/bridges/surface_client.rs:936`.
Verdict: APPROVE WITH TYPE-SHAPE CORRECTION.
Finding: the packet's pseudo-code names `ChargeAbilityScope::Off`, but the actual implementation substrate is a boolean field; adding a bool parameter or small local enum to `authorize_for_path` is straightforward.
Finding: route replacement is localized because project-composition currently calls `authorize` at `src-tauri/src/surface_runtime/mod.rs:2288`.

## Validation 4 - Error Envelope And Emittable Codes
Claim recap: V1.1 maps typed runtime/transport errors by nested `error.code`.
Evidence: runtime error responses use nested `{"error":{...}}` at `src-tauri/src/surface_runtime/mod.rs:3514`.
Evidence: non-2xx WP transport responses are rewrapped with `ok=false` at `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:393`.
Evidence: WP transport emits `runtime_request_failed`, `runtime_invalid_json`, and `runtime_http_error` at `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:379`.
Evidence: `rate_limited` is emitted from the project-composition auth failure path at `src-tauri/src/surface_runtime/mod.rs:2300` and constructed at `src-tauri/src/surface_runtime/mod.rs:3426`.
Evidence: `session_not_found` is constructed at `src-tauri/src/surface_runtime/mod.rs:3361`.
Evidence: `session_requires_repair` is a `SurfacePairingError` code at `src-tauri/src/services/surface_pairing.rs:277` and is surfaced through `from_pairing_error` at `src-tauri/src/surface_runtime/mod.rs:3330`.
Evidence: projection consistency codes are mapped at `src-tauri/src/surface_runtime/mod.rs:3010`, `src-tauri/src/surface_runtime/mod.rs:3016`, and `src-tauri/src/surface_runtime/mod.rs:3043`.
Evidence: `BridgeSurfaceError` variants do not include `consistency_failure` in the current enum at `src-tauri/src/bridges/types.rs:298`.
Verdict: CONDITIONAL APPROVE.
Finding: the two-channel envelope is verified.
Finding: `consistency_failure` is UNVERIFIED as an emittable runtime code in the current Rust surface; keep it only as a renderer fail-safe fixture, or add the producer of that code to the packet.
Finding: AC #14 lists `missing_expected_claim_version` and `mid_flight_mutation` at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:623`, but fixture #14's 11-code list omits both at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:648`.

## Validation 5 - PHP Wrapper Preservation
Claim recap: V1.1 preserves `dailyos_account_overview_render(array $attributes): string` for front-end render and six existing fixtures.
Evidence: `render.php` requires the renderer and calls `dailyos_account_overview_render($attributes)` at `wp/dailyos/blocks/account-overview/render.php:20`.
Evidence: current wrapper signature and single runtime call live at `wp/dailyos/blocks/account-overview/render-functions.php:32` and `wp/dailyos/blocks/account-overview/render-functions.php:55`.
Evidence: preview route currently makes one runtime call at `wp/dailyos/includes/class-dailyos-plugin.php:587`.
Evidence: preview then re-enters the wrapper via `render_block_with_filter` at `wp/dailyos/includes/class-dailyos-plugin.php:612` and `wp/dailyos/includes/class-dailyos-plugin.php:682`.
Evidence: the six cited tests call the wrapper at `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:52`, `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:61`, `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:96`, `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:131`, `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:169`, and `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:189`.
Verdict: APPROVE.
Finding: V1.1's proposed pure helper preserves the wrapper contract and removes only the preview re-entry; this should keep the existing fixtures green if the wrapper signature and one-fetch behavior remain unchanged.

## Validation 6 - AC To Fixture Mapping
Claim recap: V1.1 says 16 acceptance criteria map to 16 fixtures.
Evidence: AC #1-#16 are listed at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:607`.
Evidence: fixture #1-#16 are listed at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:635`.
Verdict: CONDITIONAL APPROVE.
Finding: mapping is not actually 1:1. AC #1 needs fixture #1 and fixture #2, while fixture #7 is an important cache-hit guard but has no standalone AC line.
Finding: AC #4 is only inferred from fixture #4's grep shape; add an execution assertion that the success `setAttributes` write does not schedule a second request.
Finding: AC #10 and AC #11 are covered only inside fixture #16's hands-on log, not by isolated automated fixtures.
Finding: AC #12 and AC #13 are editor-state requirements, but fixture #13 is described as a PHP integration response-shape test; the packet itself says last-good preview is editor-side state at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:597`.
Finding: AC #14 and fixture #14 disagree on code count as noted in Validation 4.
Finding: AC #15 is split across fixtures #9 and #11, and AC #16 is partly source-review/invariant territory rather than a single fixture.

## Summary
The core V1.1 implementation is feasible: preserve producer commit on cache miss, move the current DB-version read before cache lookup, swap the version argument passed into the existing orchestrator API, and add a local-render authorization variant using the existing `charge_ability_scope` boolean.
Conditional approval depends on packet cleanup before implementation: correct the path/type-shape inaccuracies, resolve the unverified `consistency_failure` code, align AC #14 with the fixture matrix, and replace the claimed 16-to-16 mapping with an explicit AC-to-fixture table that covers editor-state behavior directly.
No source files or the V1.1 packet were modified by this review.
