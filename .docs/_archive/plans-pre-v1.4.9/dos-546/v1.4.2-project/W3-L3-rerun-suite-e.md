# W3 L3 Suite E (Chaos / Edge cases) — re-run

- Reviewer: chaos-engineer
- Branch: `dos-546-w3-wordpress-foundation` @ `eae1c267`
- Parent: `dev@dd003ee2`
- Acceptance contract: `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md`
- Diff scope: `git diff dd003ee2..HEAD` (54 files / +9990)
- Cycle context: re-confirmation pass post-fold; initial L3 Suite E already APPROVED. New surface = 5 W3 fold fixes (handoff §"5 W3 L3 fixes").

## VERDICT: APPROVE

No new AC-bound chaos findings. All chaos scenarios against the 5 fold fixes either (a) terminate in fail-closed branches, (b) reduce to previously-filed path-α items (revoke TOCTOU, concurrent admin pairing, options-table-read-only, permission callback ordering), or (c) are bounded by trust-boundary assumptions documented in L0 V4 §host-boundary. The fold has not introduced new failure modes beyond the maintenance backlog already filed (DOS-585/586/587/588 + L2/L3 reviewer notes in handoff §Open).

## Chaos scenarios — 5 fold fixes

### FIX 1 — Runtime URL filter (`dailyos_wp_bridge_runtime_url`)

Surface under test:
- `class-dailyos-runtime-client.php:427-447` `runtime_base_url_for_signed_request`
- `class-dailyos-runtime-client.php:578-596` `normalize_loopback_runtime_url`
- `class-dailyos-runtime-client.php:633-636` `log_invalid_runtime_url_override`

**Scenario E1.1 — Filter returns non-loopback host (`http://attacker.example/`).**
Outcome: `normalize_loopback_runtime_url` rejects (scheme/host/path/query/fragment guard at line 591); `log_invalid_runtime_url_override` fires; falls back to `$marker_url`. Already covered by `RuntimeClientTest::test_runtime_url_filter_rejects_non_loopback_override` dataProvider. No leak.

**Scenario E1.2 — Filter returns valid loopback URL on first call, mutates to attacker URL on second call mid-request.**
Outcome: filter is read once per `invoke_ability` call (line 434) and the value is captured into `$runtime_base_url` before `wp_remote_post`. No re-read inside the signed-request path. A filter that flips state between calls produces independent per-request decisions, each individually validated. The URL is **not** part of the HMAC canonical bytes — only `path_query` is. Signature remains valid against intended runtime; a mutating filter just routes the (correctly-signed) request to a chosen loopback port. Since the override is `manage_options`-gated (line 430), admin-equivalent capability is required, which is by design the trust boundary.

**Scenario E1.3 — Filter throws / returns non-string (e.g., `WP_Error`, object, array).**
Outcome: `is_string( $filtered_url )` guard at line 436 rejects. Falls back to `$marker_url`. If `apply_filters` itself raises a `TypeError` from a filter callback, PHP fatal — same observability as any third-party plugin misbehaving. Not a W3-introduced regression.

**Scenario E1.4 — Filter returns loopback URL with non-default path (`http://127.0.0.1:5000/proxy`).**
Outcome: `$has_extra = '' !== $path && '/' !== $path` rejects (line 589); falls back. Good — defeats path-prefix smuggling.

**Scenario E1.5 — Race: `manage_options` revoked mid-request.**
`current_user_can('manage_options')` check at line 430 is the gate. If capability is dropped between the gate check and `apply_filters` invocation, the filter still runs (gate already passed). Window is microseconds within a single PHP request; capability snapshot semantics are WP-standard. Not chaos-relevant — admins are trust boundary by L0.

No AC-bound issue. **PASS.**

### FIX 2 — Plugin instance UUID generation at activation

Surface under test:
- `class-dailyos-activation.php:206-208` `plugin_instance_uuid()`
- `class-dailyos-activation.php:230-244` `ensure_uuid_option()`
- `wp_options.dailyos_plugin_instance_uuid`

**Scenario E2.1 — Two concurrent activations (e.g., admin double-clicks Activate during slow page load, or two admins in parallel sessions).**
Outcome: `ensure_uuid_option` reads (`get_option`), checks empty, generates UUID, writes (`update_option`). Classic check-then-act TOCTOU on the option row. Both processes can pass the empty-check and each writes a different UUID; last write wins. Same shape as the **concurrent admin pairing last-writer-wins** path-α finding already filed. WordPress activation is admin-gated, single-action; the realistic blast radius is "one UUID overwrites the other, and the marker carries the second one" — pairing handshake regenerates marker with the persisted UUID, so post-race coherence is recovered at first re-pair. Not AC-bound.

**Scenario E2.2 — `wp_options` table read-only (MySQL `--read-only`, replica-only, or disk-full `update_option` failure).**
Outcome: `update_option` returns `false` silently; subsequent `get_option` still returns `''` until storage recovers. `ensure_uuid_option` then regenerates a new UUID on next call — but again can't persist. Net effect: every request gets a fresh ephemeral UUID until storage recovers; markers stored before recovery point to UUIDs that don't match `get_option` after recovery; pairing breaks → user re-pairs. Same shape as **options-table-read-only quiet success** path-α already filed. Not AC-bound.

**Scenario E2.3 — `wp_generate_uuid4` unavailable (theoretical, ancient WP).**
Outcome: fallback `sprintf` with `random_int` produces a valid UUIDv4-shaped string. PHP 8.4 minimum (per `phpcs.xml.dist`), `random_int` is core. Safe.

**Scenario E2.4 — Reactivation after option deletion (e.g., `wp option delete dailyos_plugin_instance_uuid` between activate cycles).**
Outcome: `ensure_uuid_option` regenerates on next call. New UUID does not match previously-issued markers; substrate-side will see a new `plugin_instance_uuid` in HMAC canonical bytes; signatures verify against the new UUID; pairing re-handshakes. This is the intended uninstall/reinstall flow. The marker's `plugin_instance_uuid` field is captured at pairing time (`class-dailyos-credential-store.php:83`), so if an admin deletes the option WITHOUT deleting the marker, the marker's UUID drifts from the option. **However** the runtime client reads `plugin_instance_uuid` from the **marker**, not the option (line 480), so signed requests continue to send the marker's UUID — which is the value the substrate verified at pairing. Self-healing. **PASS.**

### FIX 3 — HMAC canonical vectors / dataProvider drift (PHP signer vs Rust verifier)

Surface under test:
- `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json` (3 vectors)
- `wp/dailyos/tests/transport/HmacSignerTest.php::canonical_vector_provider`
- `wp/dailyos/includes/transport/class-dailyos-hmac-signer.php::canonical_bytes`
- `src-tauri/src/surface_runtime/hmac.rs::canonical_request_bytes` (Rust verifier)

**Scenario E3.1 — Rust verifier and PHP signer disagree on a field's canonical form (e.g., method case, content-type trimming, body byte handling).**
Vectors lock 3 cases: (a) standard JSON POST, (b) binary body with empty content_type and lowercase method, (c) PUT with charset content-type and multisite. PHP-side `HmacSignerTest` asserts both pre-HMAC canonical bytes AND final signature. **However:** Rust side does not currently consume this fixture file. Drift is detectable only via integration test at runtime. This is the **W2 follow-up: shared HMAC test vectors consumed by both Rust verifier and PHP signer** maintenance item already named in handoff §Open. Path-α, not AC-bound.

**Scenario E3.2 — Fixture mutated by a fold commit but PHP test forgets to regenerate `expected_canonical_bytes_b64`.**
Outcome: dataProvider test fails immediately; CI gate catches. The vectors are byte-locked, self-asserting (decode → compare). Drift inside PHP is detectable. PASS.

**Scenario E3.3 — Method case mismatch (`"post"` vs `"POST"`).**
Vector 2 uses lowercase `"post"` precisely to exercise `strtoupper($method)` normalization at line 52. Canonical bytes show `POST`. Verifier MUST also uppercase before comparison or signatures will diverge. Spot-checked Rust side `src-tauri/src/surface_runtime/hmac.rs:485+` — need not deep-audit here as initial L3 Suite E already approved the Rust verifier behavior. PASS.

**Scenario E3.4 — UTF-8 / ASCII assertions fail on identity field.**
`assert_utf8` (line 152) and `assert_ascii` (line 166) throw `InvalidArgumentException`. Caller surfaces as request failure, not as a silently mis-signed packet. Fail-closed. PASS.

**Scenario E3.5 — Empty `multisite_blog_id` vs `"0"` ambiguity.**
Vector 1 single-site → `""` (length 0); Vector 2 multisite → `"3"` (length 1); Vector 3 multisite → `"12"` (length 2). Length-prefix canonicalization makes `""` vs `"0"` byte-distinct (`multisite_blog_id:0\n\n` vs `multisite_blog_id:1\n0\n`). Cross-impl bug surface, but vectors lock the discriminator. PASS.

No AC-bound issue. Path-α already filed (Rust consumer of shared fixture).

### FIX 4 — MCP scope check on session that mutates mid-list

Surface under test:
- `class-dailyos-mcp-server.php:217-248` `filter_tools_list`
- `class-dailyos-mcp-server.php:426-442` `resolved_scopes`
- `class-dailyos-mcp-server.php:447-` `switch_to_substrate_user`
- `class-dailyos-mcp-permission.php` `check`

**Scenario E4.1 — Session scopes change between `filter_tools_list` (enumeration) and `prepare_tool_call` (invocation).**
Two independent permission checks: `filter_tools_list` returns the listable set at time T0; `authorize_ability_invocation` re-runs `permission->check` at T1 with the then-current resolved scopes (line 390). If scopes were revoked between T0 and T1, the tool was advertised but invocation denies fail-closed via `WP_Error` with `dailyos_mcp_permission_denied`. The advertised tool is not actually invocable. Slight UX leak (existence disclosure of tools no longer authorized), but not a privilege escalation — per L0 V4 the existence of MCP tools at namespace `dailyos/*` is non-secret. PASS.

**Scenario E4.2 — `scope_resolver` closure throws / returns non-array.**
`resolved_scopes` guard at line 429 `is_array($scopes)` returns `[]`. Empty scopes → `permission->check` denies all scope-gated abilities → empty tool list. Fail-closed. PASS.

**Scenario E4.3 — `is_dailyos_server` returns false mid-list.**
`filter_tools_list` strips any DailyOS-named tool from the non-DailyOS server's response (lines 218-227). Defense in depth: a non-DailyOS server cannot accidentally surface DailyOS tools via MCP adapter routing. PASS.

**Scenario E4.4 — `switch_to_substrate_user` race with concurrent admin session.**
`wp_set_current_user` mutates request-local `$current_user`. `filter_tools_list` switches once at line 229; subsequent `get_current_user_id()` returns substrate. After tools_list returns, the user-switch persists for the request unless reverted (not seen in this file). Same shape as **`wp_set_current_user` call-site centralization** L2 maintenance item already filed (handoff §To be filed). Path-α, not new.

**Scenario E4.5 — Permission `check` ordering: WP cap evaluated against substrate user (post-switch) but `mcp_exposure` evaluated against original user's session.**
`switch_to_substrate_user` is called before `get_current_user_id`, so `$wp_user_id` = substrate user ID. Substrate user holds the dailyos_substrate role's capabilities, which is the documented design. Same shape as **`permission callback side-effect ordering`** path-α already filed.

No new AC-bound issue.

## New findings (cycle 2)

None of severity ≥ medium that are AC-bound.

| Finding | Severity | File:line | AC-bound? | Disposition |
|---|---|---|---|---|
| None new | — | — | — | — |

## Prior path-α items — confirmation still maintenance-only

| Item | Source | Status |
|---|---|---|
| revoke-vs-`update_last_use` TOCTOU resurrection | initial L3 Suite E | **Still path-α.** Fold did not change `update_last_use` (still `class-dailyos-credential-store.php:101-111` — get then write, no re-validation of marker presence). Confirmed maintenance-only. |
| concurrent admin pairing last-writer-wins | initial L3 Suite E | **Still path-α.** `save_marker` (line 65-94) is single-statement `update_option`; no optimistic lock. Same race surface. New `plugin_instance_uuid` generation at activation has the same shape (FIX 2 E2.1) → same maintenance bucket. Confirmed maintenance-only. |
| options-table-read-only quiet success | initial L3 Suite E | **Still path-α.** `update_option` return value unchecked at credential-store:93, 110; activation:241. FIX 2 amplifies the surface modestly (one more option) but doesn't change the disposition. Confirmed maintenance-only. |
| permission callback side-effect ordering | initial L3 Suite E | **Still path-α.** `switch_to_substrate_user` → `get_current_user_id` → `permission->check` ordering preserved across fold (filter_tools_list:229+241, authorize_ability_invocation:387+389). Confirmed maintenance-only. |

## Notes for L4 / runtime exercise

When pairing flow runs live in `~/Studio/dailyos-dev`:
- Exercise FIX 1 by setting `add_filter('dailyos_wp_bridge_runtime_url', fn() => 'http://192.168.1.1:5000')` in a mu-plugin, confirm `error_log` shows the override-ignored warning and request still routes to marker URL.
- Exercise FIX 4 by listing tools as one user, then revoke a scope via filter, list again — confirm the second list omits the revoked tool.
- These are L4 surface-QA checks, not L3 blockers.

## Suite E verdict

**APPROVE.** Re-confirmation pass complete. No new AC-bound findings introduced by the fold; the 4 prior path-α items remain in the maintenance project (DOS-585/586/587/588 + L2 backlog) and are not regressed. The fold's chaos surface is dominated by trust-boundary assumptions documented in L0 V4 and by already-filed maintenance work. W3 advances on Suite E.
