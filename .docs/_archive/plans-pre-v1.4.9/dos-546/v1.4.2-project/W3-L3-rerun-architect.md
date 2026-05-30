# W3 L3 architect-reviewer — re-run verdict

Date: 2026-05-13
Branch: `dos-546-w3-wordpress-foundation` @ `eae1c267`
Parent: `dev@dd003ee2` (W2 merged via PR #270)
Reviewer: architect-reviewer (subagent), retry after prior mid-execution error
Acceptance contract: `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md` (V4 unanimous APPROVE)

## 1. VERDICT

**APPROVE.**

Integrated W2+W3 state is architecturally sound. All prior P0/P1 root causes are resolved with file:line evidence below. No new AC-bound blocking findings. Two path-α observations filed for the maintenance project (not blocking).

## 2. Prior P0/P1 resolution — confirmed

### P0 — Hardcoded loopback runtime URL (was: `127.0.0.1:8765` baked into client)

Resolved. Evidence:

- `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:143-156, 427-447, 449-468, 578-596` — `signed_post()` reads `runtime_url` from the pairing marker via `runtime_base_url_for_signed_request()`. Handshake reads the port from the pairing-code query string via `runtime_base_url_for_pairing()`. Both routes converge on `normalize_loopback_runtime_url()` which enforces scheme=`http`, host=`127.0.0.1`, valid port, no query/fragment/path-beyond-root. The `dailyos_wp_bridge_runtime_url` filter is gated on `current_user_can('manage_options')` and re-validated through the same normalizer (line 430-444). Non-loopback overrides are logged + ignored without leaking the candidate (`log_invalid_runtime_url_override`, line 633-636).
- `wp/dailyos/includes/transport/class-dailyos-credential-store.php:73-94` — marker schema carries `runtime_url` as a normalized field, written at `save_marker()` time.
- Negative grep: `grep -rn "127.0.0.1:8765\|DEFAULT_RUNTIME_URL" wp/` returns zero hits.
- Test coverage: `wp/dailyos/tests/transport/RuntimeClientTest.php` (270 lines) covers marker-read, not-paired, loopback-accept, non-loopback-reject.

Architectural quality: failure mode is correct (`WP_Error('dailyos_not_paired')` returned as a typed error, not a silent default-URL fallback). The same normalization function handles pairing-code-derived URLs and admin filter overrides, eliminating divergent validation paths.

### P1 — `dailyos_nonce_sweep` cron event with no listener

Resolved. Evidence:

- `wp/dailyos/includes/class-dailyos-plugin.php:72` — `add_action('dailyos_nonce_sweep', [$this, 'sweep_presence_nonces'])`.
- `wp/dailyos/includes/class-dailyos-plugin.php:157` — `public function sweep_presence_nonces(): void {}` (no-op handler; full lifecycle deferred to W4-E per packet).
- `wp/dailyos/includes/class-dailyos-activation.php:64` — `wp_clear_scheduled_hook('dailyos_nonce_sweep')` on deactivation.
- `wp/dailyos/includes/class-dailyos-activation.php:277-279` — hourly schedule with `$offset` jitter to avoid stampedes.
- `wp/dailyos/tests/ActivationTest.php:153` — `assertNotFalse(has_action('dailyos_nonce_sweep'))`.

Architectural quality: handler is registered as a class method (not closure), giving WP a stable callable identity for future replacement; deactivation cleanup pairs with activation scheduling, no orphan cron rows after lifecycle transitions.

### P1 — `mcp_server_name` audit field rename to `actor_instance`

Resolved. Evidence:

- `wp/dailyos/includes/mcp/class-dailyos-mcp-audit.php:26` — `REQUIRED_KEYS` contains `actor_instance` (and `mcp_exposure_path`, the audit-exposure field). `mcp_server_name` is absent.
- `wp/dailyos/includes/mcp/class-dailyos-mcp-server.php:281` — emit site uses `'actor_instance' => $this->actor_instance()`.
- `wp/dailyos/includes/mcp/class-dailyos-mcp-server.php:517-525` — `actor_instance()` reads `plugin_instance_uuid` from the marker; returns empty string if absent (fail-loud audit signal).
- `wp/dailyos/tests/mcp/McpExposureNoneTest.php:207` — fixture uses `actor_instance => 'plugin-1'`.
- Negative grep: `grep -rn "mcp_server_name" wp/` returns zero hits.

Architectural quality: `actor_instance` is correctly bound to `plugin_instance_uuid` (generated at activation via `wp_generate_uuid4()`, persisted in `wp_options.dailyos_plugin_instance_uuid`). This aligns with the L0 packet V2 rename rationale: the prior `mcp_server_name` field was constant under the exclusivity decision and carried no forensic value; `actor_instance` distinguishes plugin instances across multi-install or rotation scenarios.

## 3. New L3 findings against integrated state

### AC-bound (blocking)

None.

### Path-α (theoretical hardening — file to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`, NOT blocking)

**PATH-α-1 [LOW] — `actor_instance` empty-string on detached audit emission.** `class-dailyos-mcp-server.php:520-522` returns `''` when the marker is absent or lacks `plugin_instance_uuid`. Audit consumers treating empty actor as "unknown caller" is correct, but the `DailyOS_Mcp_Audit::emit()` `REQUIRED_KEYS` check (`class-dailyos-mcp-audit.php:26`) does not assert non-empty — an audit row with `actor_instance=''` will pass. Under normal operation the marker is present at any code path that enumerates abilities (substrate user is created at activation, marker at pairing); empty-actor rows would only fire in pre-pairing or post-uninstall windows. Not an L0 acceptance violation: the packet specifies `actor_instance` as the rename target and the W1-A0 SurfaceClient audit helper owns the forensic-completeness contract upstream. Maintenance ticket recommendation: tighten `REQUIRED_KEYS` to a value-shape map (non-empty string for `actor_instance` when emission path is not pre-pairing).

**PATH-α-2 [LOW] — `runtime_base_url_for_signed_request` filter result revalidation cost on every signed request.** `class-dailyos-runtime-client.php:430-444` runs `apply_filters('dailyos_wp_bridge_runtime_url', ...)` + `normalize_loopback_runtime_url()` on every signed request when the actor has `manage_options`. The repeated parse is cheap (no I/O, single `wp_parse_url` call), but for high-frequency admin AJAX patterns this adds per-request overhead. Not an acceptance violation: the packet's V3 disambiguation explicitly says the per-request retrieval is the load-bearing path; admin filter is a debug-/operator-override hook, not a hot path. Maintenance ticket recommendation: memoize the normalized result per-request via a static property on the client when the filter result hasn't changed.

## 4. Architectural integrity statement — integrated W2+W3

**The integrated W2+W3 state is internally consistent and substrate-aligned.**

Trust-boundary architecture (load-bearing):
- **HMAC canonicalization is byte-identical across the WP→Rust boundary.** `wp/dailyos/includes/transport/class-dailyos-hmac-signer.php:43-77` produces the same 15-field length-prefixed canonical bytes as `src-tauri/src/surface_runtime/hmac.rs:829-891`. Domain separator (`DAILYOS-WP-BRIDGE-HMAC-V1`), field order (method → path_query → content_type → body → 9 identity fields → nonce → timestamp), and field encoding (`label:length\nvalue\n`) match exactly. The shared test fixture `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json` carries pre-computed `expected_canonical_bytes_b64` + `expected_signature_hex` for three vectors covering ASCII JSON, binary body w/ empty content-type, and multisite trimmed-content-type — providing a cross-implementation drift gate.
- **Credential lifetime contract honored.** `class-dailyos-credential-store.php:127-160, 167-195` retrieves session material via the `manage_options`-gated `dailyos_wp_bridge_session_key` filter, validates shape (`normalize_session_key_result`, line 222-252), wraps in `DailyOS_Session_Credential` + `DailyOS_Hmac_Key` value objects per-request, never persists to WP options/transients/post-meta. The grep-gate at `scripts/grep-gates.json:18-22` enforces this with a regex blocking `update_option|set_transient|update_post_meta|update_user_meta|wp_localize_script` paired with secret-named tokens.
- **MCP exposure is fail-closed at three layers.** (a) `wp_register_ability()` default `mcp_exposure: None` keeps abilities invisible to any generic adapter enumerator (negative fixture at `tests/mcp/McpExposureNoneTest.php`). (b) `class-dailyos-mcp-server.php:217-248` filters `mcp_adapter_tools_list` to deny enumeration of DailyOS abilities by non-DailyOS servers AND per-tool runs `DailyOS_Mcp_Permission::check` against the resolved scope set even at list time. (c) Tool invocation runs as the `dailyos_substrate` low-cap WP user via `switch_to_substrate_user()` regardless of caller identity (line 229, 263, 372, 387).
- **Namespace-vacancy invariant at activation.** `class-dailyos-activation.php:30-57` implements the four-state machine from packet V3 (marker present/absent × namespace clean/dirty), with `wp dailyos repair-namespace` recovery path for the dirty + no-marker case. Marker is a heuristic only — the V4 clarification that runtime-state comparison is the authoritative trust source is reflected in `marker_matches_prior_pair()` checking runtime-reported state (`class-dailyos-activation.php:120+`).

Workspace-boundary architecture:
- W3 is purely additive on the WP side; `git diff dd003ee2..HEAD -- 'src-tauri/**'` returns no Rust substrate deltas. W2 substrate at `dev@dd003ee2` remains the canonical signed-transport authority. No promotion path from WP DB → canonical substrate state was introduced (workspace-boundary release-gate invariant preserved).
- Renderer/detector handoff (W3-C → W4-A/W4-C) is wired through audit emission, not through synchronous UI calls — `class-dailyos-mcp-audit.php` emits divergence signals, W3-C does not own UI degradation. This matches packet V2 §"Renderer/detector handoff named explicitly".

Test-coverage discipline:
- PHPUnit 45 tests / 161 assertions per W3-handoff §"Current gates".
- Cross-implementation drift gates: HMAC vector fixture (PHP + Rust both consume), role-capability fixture (`tests/fixtures/role-capabilities.json`), SurfaceClient scope fixture (`tests/fixtures/surfaceclient-scopes.json`) — three fixtures pin the W2↔W3 contract surface and fail CI on drift.
- Grep-gates (`scripts/grep-gates.json`) enforce 5 architectural invariants: no raw `$wpdb` outside services, no filesystem writes outside `uploads/dailyos/`, no secret persistence, no `wp_remote_post` array-body (pre-serialization required for HMAC byte-exactness), no ephemeral issue refs in PHP comments.

Recommendation: **proceed to L2 (pre-PR) panel + PR open.**

## 5. References

- Acceptance contract: `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md` (V4, 2026-05-13 unanimous APPROVE)
- Wave-bundle handoff: `.docs/plans/dos-546/v1.4.2-project/W3-handoff.md`
- W3 fold commits: `68e1ed9a` (final polish), `870b5a3e` (partial: nonce_sweep + audit-field rename), `c1fdf2b4` (test alignment)
- Maintenance project for path-α: `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` "Codebase Maintenance & Production Quality"
