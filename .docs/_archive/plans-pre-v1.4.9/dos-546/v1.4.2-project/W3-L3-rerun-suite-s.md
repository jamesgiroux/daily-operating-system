# L3 Suite S — W3 trust boundary security review

**Worktree:** `/private/tmp/dailyos-w3-0` · **Branch head:** `eae1c267` · **Parent:** `dev@dd003ee2`
**Date:** 2026-05-13

## VERDICT: **CONDITIONAL** → BLOCKING for L3 closure

One acceptance-bound HIGH (HMAC timestamp format divergence between PHP signer and Rust verifier) blocks runtime interoperability promised by W3-A AC #4 (15-field byte-exact canonicalization match). Two additional findings are path-α maintenance (substrate-user adoption; ASCII-whitespace trim divergence). One observation (substrate user lacks `manage_options` for session-key retrieval) is likely an L4 functional blocker, not security.

The L0 packet directional decision #4 names PHP runtime transport with tests proving exact-byte canonicalization to match the W2 HMAC contract. The W2 HMAC contract requires RFC3339-Z. The plugin canonicalizes correctly for any string but does not emit a contract-valid timestamp at signing time. This violates the spirit of fix #4 and is what L3 exists to catch — integrated-state adversarial review, not unit-test parity.

## Attack chains attempted

| # | Chain | Outcome |
|---|---|---|
| 1 | HMAC canonicalization: PHP `time()` decimal vs Rust RFC3339-Z parse | **HIT — HIGH-1.** PHP emits Unix decimal; Rust requires `endsWith('Z')` + RFC3339. Live signed requests rejected before HMAC compare. Fixtures pass (treat timestamp as opaque string) but don't exercise the timestamp gate. |
| 2 | Content-type whitespace trim divergence | **HIT — LOW-1.** PHP `trim()` charset = `" \t\n\r\0\x0B"`; Rust `trim_ascii_whitespace` = AsciiWhitespace incl. `\x0c`, excl. NUL. Pathological content-types cause signature mismatch. Fail-closed. |
| 3 | Nonce format mismatch | **MISS.** PHP `bin2hex(random_bytes(16))` matches `is_lowercase_hex_nonce`. |
| 4 | Pairing marker forgery → audit attribution | **ACKNOWLEDGED.** Marker non-secret by L0 V4 (marker-as-heuristic). `actor_instance` forgeable via DB write but does not bypass HMAC/scope. Out of AC. |
| 5 | Pairing marker forgery → runtime URL spoofing | **MISS.** `normalize_loopback_runtime_url` rejects `0.0.0.0`, hostnames, `127.0.0.1.evil.com`, decimal-IP `2130706433`, `127.000.000.001`, `[::1]`. Userinfo `http://x@127.0.0.1:port` connects to `127.0.0.1` only — no spoofing. |
| 6 | Filter override bypass (non-admin filter) | **MISS.** `runtime-client.php:430` gates filter on `current_user_can('manage_options')`. |
| 7 | Nonce replay vs substrate | **MISS.** `hmac.rs:1296-1398` reserves nonce at request start, marks consumed after sig check, TTL covers stale + skew + slack. |
| 8 | MCP `tools/list` enumeration of dailyos abilities by foreign server | **MISS.** `mcp-server.php:217 filter_tools_list` strips dailyos-prefixed tools when `is_dailyos_server($server)` is false. |
| 9 | MCP scope bypass at `tool/call` | **MISS.** Per-tool permission callback AND-gates WP cap + scope check + `mcp_exposure === Invocable`. |
| 10 | Substrate user adoption / collision via pre-existing `dailyos_substrate` login | **HIT — MEDIUM-1.** `ensure_user()` adopts existing user without ownership check; `namespace_is_dirty()` does not scan `wp_users`. |
| 11 | MCP `wp_user_id` audit attribution forgery | **MISS.** wp_user_id canonicalized; forgery → HMAC reject. |
| 12 | Substrate user inability to retrieve session key | **OBS-1.** MCP invocation switches to substrate user (lacks `manage_options`) before `retrieve_session_material()` which requires `manage_options`. MCP-invoked abilities fail with `missing_session_key`. Fail-closed; L4 functional blocker. |

## Findings

| ID | Severity | File:line | AC-bound? | Description |
|---|---|---|---|---|
| HIGH-1 | HIGH | `wp/dailyos/includes/transport/class-dailyos-hmac-signer.php:129-130` vs `src-tauri/src/surface_runtime/hmac.rs:1035-1043` | **YES (W3-A AC #4 / L0 V4 §4)** | PHP `current_timestamp()` returns `(string) time()` (Unix decimal); Rust verifier rejects anything not ending `'Z'`. Every live signed invocation from the plugin will be rejected at timestamp-parse before HMAC compare. Fix: change PHP to emit `gmdate('Y-m-d\TH:i:s\Z', time())`. Update fixture vectors + expected canonical bytes + expected signature hex. Add a fixture exercising RFC3339-Z form end-to-end. |
| MEDIUM-1 | MEDIUM | `wp/dailyos/includes/mcp/class-dailyos-mcp-roles.php:71-95` + `class-dailyos-activation.php:88-113` | NO (path-α) | `ensure_user()` adopts pre-existing WP user with login `dailyos_substrate` without ownership verification; `namespace_is_dirty()` does not scan `wp_users`. Pre-existing attacker-controlled user becomes the MCP principal. Fix: extend dirty-check to wp_users; refuse adoption on legitimate fresh activation, create uniquely-suffixed user. |
| LOW-1 | LOW | `class-dailyos-hmac-signer.php:53` vs `hmac.rs:1172` | NO (path-α) | Trim character-set divergence (NUL/VT vs `\x0c`). Fail-closed; edge case. Align on RFC 7230 OWS `\t \n \r`. |
| OBS-1 | OBS (functional) | `class-dailyos-credential-store.php:174` ↔ `class-dailyos-mcp-server.php:372-378` | NO | Substrate user can't retrieve session credential due to `manage_options` gate; MCP invocation non-functional. Either elevate gate to accept `dailyos_invoke_mcp_ability`, or capture credential before user switch. Likely L4 surface-QA blocker. |

## Path-α targets (Linear maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`)

- MEDIUM-1 — substrate user adoption ownership check + activation dirty-check coverage for `wp_users`
- LOW-1 — HMAC content-type trim character-set alignment
- OBS-1 — substrate user session-credential retrieval path (functional, surfaces at L4)

## Sound controls (no findings)

Nonce replay store · timestamp-window enforcement · constant-time sig compare · 15-field identity canonicalization · MCP same-server tool-list filter · loopback-only runtime URL enforcement · `manage_options` gating on session-key filter and runtime URL override filter · HMAC + session-credential redaction wrappers · substrate-user fail-closed activation.

## Files audited

- `wp/dailyos/includes/transport/class-dailyos-hmac-signer.php`
- `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`
- `wp/dailyos/includes/transport/class-dailyos-credential-store.php`
- `wp/dailyos/includes/mcp/class-dailyos-mcp-roles.php`
- `wp/dailyos/includes/mcp/class-dailyos-mcp-server.php`
- `wp/dailyos/includes/mcp/class-dailyos-mcp-permission.php`
- `wp/dailyos/includes/class-dailyos-activation.php`
- `wp/dailyos/includes/services/class-dailyos-namespace-store.php`
- `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json`
- `src-tauri/src/surface_runtime/hmac.rs`
- `src-tauri/src/services/surface_pairing.rs`
- `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md`
