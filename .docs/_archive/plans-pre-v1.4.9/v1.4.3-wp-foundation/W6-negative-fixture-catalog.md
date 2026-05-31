# W6 Negative Fixture Catalog

DOS-575 closes the v1.4.3 foundation release gate by pinning every trust boundary crossed by the WordPress foundation to a named negative fixture. The canonical runner is:

```sh
bash scripts/release-gate/run-w6-fixtures.sh
```

The runner writes `src-tauri/target/release-gate/w6-fixtures.json`. Any failed, missing, stale, or skipped fixture is a release-gate failure.

## WordPress MCP Exposure Boundary

| ID | Owner issue | Expected status / error | Proof command | Coverage |
|---|---|---|---|---|
| `w6-01-default-wp-mcp-no-dailyos` | DOS-575 | Default WordPress MCP enumerates zero `dailyos/*` abilities. | `cd wp/dailyos && vendor/bin/phpunit --filter test_generic_mcp_server_enumerates_zero_dailyos_tools tests/mcp/McpExposureNoneTest.php` | Existed |
| `w6-02-mcp-exposure-none-hidden` | DOS-575 | Ability declared `mcp_exposure: None` is absent from the allowlist and adapter registration. | `cd wp/dailyos && vendor/bin/phpunit --filter 'test_build_allowlist_excludes_none_and_disallowed_categories\|test_all_none_inventory_registers_no_dailyos_tools_with_adapter' tests/mcp/McpExposureNoneTest.php` | Existed |

## Frontend + Gutenberg Serialization Boundary

| ID | Owner issue | Expected status / error | Proof command | Coverage |
|---|---|---|---|---|
| `w6-03-frontend-js-no-dailyos-secrets` | DOS-575 | Block bundles and frontend sources contain no DailyOS transport secret-shaped fields, bearer tokens, or loopback runtime URLs. | `bash scripts/release-gate/check-no-frontend-secrets.sh` | Added |
| `w6-04-gutenberg-rejects-raw-runtime-payloads` | DOS-575 | Saved block content strips raw ability payloads, provenance JSON, `payload_json`, and unknown sensitive shapes before serialization. | `cd wp/dailyos && vendor/bin/phpunit --filter test_block_serialization_rejects_raw_ability_payloads_provenance_and_unknown_sensitive_shapes tests/PresenceNonceTest.php` | Added |
| `w6-11-payload-json-redaction` | LOCK-13 | `payload_json` is never echoed on nonce issue or verify response paths. | `cd wp/dailyos && vendor/bin/phpunit --filter 'test_verify_response_does_not_echo_payload_json_back_to_caller\|test_issue_response_does_not_echo_payload_json_back_to_caller' tests/FeedbackPayloadRedactionTest.php` | Existed |

## Projection + Feedback Nonce Boundary

| ID | Owner issue | Expected status / error | Proof command | Coverage |
|---|---|---|---|---|
| `w6-05-projection-tampered-typed-error` | DOS-575 | Tampered projection maps to HTTP 422 with typed `projection_tampered`. | `cargo test --manifest-path src-tauri/Cargo.toml --lib dos575_projection_tampered_maps_to_typed_http_error` | Added |
| `w6-06-stale-claim-version-feedback-409` | DOS-575 | Stale `claim_version` rejects with HTTP 409 / `claim_version_stale`; no `claim_feedback` mutation is written. | `cargo test --manifest-path src-tauri/Cargo.toml --lib dos571_fixture_claim_version_drift` | Existed, strengthened |
| `w6-07-cross-user-presence-nonce` | DOS-575 | Nonce issued for `wp_user_id=A` and verified as `wp_user_id=B` rejects with `wrong_user` / HTTP 403. | `cargo test --manifest-path src-tauri/Cargo.toml --lib dos575_cross_user_presence_nonce_rejected` | Added |
| `w6-08-presence-nonce-replay-rejected` | DOS-575 | Replay after successful verify rejects with `replayed` and emits `presence_nonce_rejected` reason `replayed`. | `cargo test --manifest-path src-tauri/Cargo.toml --lib dos683_e2e_replay_rejection_after_verify` | Existed |
| `w6-09-phase3-budget-charge-fail-closed` | DOS-719 | Phase-3 feedback write failure charges the failure budget and leaves the nonce consumed. | `cargo test --manifest-path src-tauri/Cargo.toml --lib dos719_phase_three_failure_charges_failure_budget_fail_closed` | Added |

## Plugin Storage Boundary

| ID | Owner issue | Expected status / error | Proof command | Coverage |
|---|---|---|---|---|
| `w6-10-direct-plugin-claim-table-write-lint` | DOS-575 | Plugin PHP contains no direct claim-table writes; self-test planted write is caught. | `bash scripts/release-gate/check-no-direct-plugin-claim-table-writes.sh --self-test` | Added |

## Runtime + Theme Lifecycle Boundary

| ID | Owner issue | Expected status / error | Proof command | Coverage |
|---|---|---|---|---|
| `w6-12-stock-theme-account-overview-render` | DOS-575 | `dailyos/account-overview` renders trust band and visible provenance with plugin-owned fallback markup/CSS under stock-theme conditions. | `cd wp/dailyos && vendor/bin/phpunit --filter test_stock_theme_account_overview_renders_trust_band_and_provenance_markup tests/blocks/AccountOverviewBlockTest.php` | Added |
| `w6-13-cold-start-stale-marker-notice` | DOS-575 | Tauri down + stale marker path renders `RuntimeUnavailableNotice`, not `is-empty`, and redacts transport detail. | `cd wp/dailyos && vendor/bin/phpunit --filter test_stale_marker_transport_error_renders_runtime_unavailable_notice_not_empty_state tests/blocks/AccountOverviewBlockTest.php` | Added |
| `w6-14-hot-tauri-restart-sentinel-discovery` | DOS-575 | Sentinel cache reset after hot Tauri restart picks up the new loopback port. | `cd wp/dailyos && vendor/bin/phpunit --filter test_runtime_sentinel_cache_resets_after_restart_and_uses_new_port tests/transport/RuntimeClientTest.php` | Added |
| `w6-15-hot-studio-restart-first-render` | DOS-575 | First render after Studio boot succeeds from the first runtime response. Live clean-machine Studio coverage remains DOS-577. | `cd wp/dailyos && vendor/bin/phpunit --filter test_hot_studio_restart_first_render_after_boot_succeeds tests/blocks/AccountOverviewBlockTest.php` | Added |

## Release Gate Integration

`src-tauri/src/release_gate.rs` treats `w6-fixtures.json` as mandatory hermetic evidence. With `run_tests=true`, the gate invokes `scripts/release-gate/run-w6-fixtures.sh`; with `--no-run-tests`, it reads `output-dir/w6-fixtures.json`. Every catalog ID above becomes a mandatory invariant. Missing IDs, stale binding, unknown statuses, failures, and skipped fixtures exit non-zero.
