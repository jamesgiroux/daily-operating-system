# L3 Suite S — W3 cycle-2 verdict

**Worktree:** `/private/tmp/dailyos-w3-0` · **Branch head:** `497dc109` · **Parent:** `dev@dd003ee2`
**Date:** 2026-05-13

## VERDICT: **APPROVE**

HIGH-1 fold lands cleanly. No new findings from the regenerated fixtures or the RFC3339-Z format. Cycle-1 sound controls preserved.

## HIGH-1 resolution evidence

- **PHP emitter fixed** — `wp/dailyos/includes/transport/class-dailyos-hmac-signer.php:134-136` — `current_timestamp()` now returns `gmdate('Y-m-d\TH:i:s\Z', time())`. Inline doc-comment (125-133) cross-references the Rust verifier gate, locking the contract in source.
- **Rust gate verified** — `src-tauri/src/surface_runtime/hmac.rs:1035-1044` — `parse_timestamp` requires `ends_with('Z')` + `chrono::DateTime::parse_from_rfc3339`. PHP output (`Y-m-d\TH:i:s\Z`, fixed 20 ASCII bytes, GMT-locked via `gmdate`) is a strict subset of RFC3339-Z. Parse succeeds.
- **Single call site** — `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:170` — `$timestamp` flows into both `sign_request(...)` and the `X-DailyOS-Timestamp` header — same value on both sides, no divergence path.
- **Fixtures regenerated** — `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json` lines 21, 44, 67 carry RFC3339-Z timestamps; `expected_canonical_bytes_b64` + `expected_signature_hex` regenerated through the signer itself.
- **Format gate test** — `wp/dailyos/tests/transport/HmacSignerTest.php:111-118` (`test_current_timestamp_matches_expected_format`) asserts `/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/` against live `current_timestamp()`.
- **Gates** — PHPUnit 45/161, PHPCS 0/0, grep-gates 0.

## New findings: **None**

Attack-chain considerations for the fold:
- **Parser ambiguity** — `gmdate` always emits the same 20-byte form regardless of timezone. No fractional seconds, no `+00:00` aliases (Rust `endsWith('Z')` rejects those). No parser-disagreement room.
- **Length-prefix confusion** — fixed 20-byte payload; canonical field is `timestamp:20\n<20 bytes>\n`. No operator-controlled length variation.
- **Opaque-byte invariant preserved** — `canonical_bytes` does not constrain timestamp format; `test_minimal_canonical_byte_sequence` still exercises `'1'` and verifies byte-exactness. Wire-format gate is at emitter + verifier only.
- **Regenerated signatures** — values recomputed by the signer under per-vector session keys; no HMAC-SHA-256 property weakened.
- **Other `time()` / `gmdate` call sites** (cron scheduling, settings-page age display, admin pairing display, credential `last_use_gmt`, test bootstrap) — none feed into wire-signed timestamps.

## Regression check vs cycle-1 sound controls

Verified intact: canonical field order + byte-length prefix + ASCII/UTF-8 assertions · signature prefix `v1=` + lowercase-hex · nonce / site-binding / site-nonce / site-url / wp_user_id / claim-identifier parsers · content-encoding rejection · replay nonce reserve-then-consume + TTL caps · MCP `tools/list` filter · per-tool scope AND-gate · runtime URL allowlist · filter-override admin gate.

The diff is scoped to the timestamp emitter + the PHPUnit fixture/regex; no other surfaces touched.

## Cycle-2 summary

Fold targeted, surgical, complete. HIGH-1 resolved at the only AC-bound channel (PHP→Rust signed-transport interop). Path-α residuals (MEDIUM-1 substrate user adoption, LOW-1 trim charset divergence, OBS-1 substrate session-key gate) already filed to maintenance:
- DOS-595 actor_instance non-empty
- DOS-596 runtime URL filter memoize
- DOS-597 substrate user adoption
- DOS-598 trim charset alignment
- DOS-599 substrate user session credential (High — L4 blocker flag)

**APPROVE — W3 wave-bundle cleared for L3 closure from Suite S.**
