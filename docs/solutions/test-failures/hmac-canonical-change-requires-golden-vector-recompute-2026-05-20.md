---
title: "Changes to HMAC canonical signing input require deterministic recompute of golden vector fixtures (expected_canonical_bytes_b64 + expected_signature_hex)"
problem_type: test_failure
track: knowledge
module: wp/dailyos/tests/fixtures/hmac_canonical_vectors.json + tests/transport/HmacSignerTest.php + tests/transport/RuntimeClientTest.php
tags: [hmac, canonical-bytes, golden-vector, fixture-recompute, signed-transport, dos-742]
date: 2026-05-20
related_linear: DOS-742
related_memories: []
---

## Context

DOS-742 added a new `x-dailyos-request-id` field to `canonical_request_bytes` (Rust `src-tauri/src/surface_runtime/hmac.rs`) + mirrored at `class-dailyos-hmac-signer.php`. The field is included after `timestamp` in the canonical ordering.

PR #337 cycle-1 CI failed on all 4 PHPUnit matrices (PHP 8.1/8.2/8.3/8.4) with assertions like:

```
Failed asserting that two strings are identical.
--- Expected
+++ Actual
@@ @@
-'10fdfd8121abab5e814a89f8eaa19171e999885d1066c64a3b58eb21978cb6a6'
+'3c0fb5e0921a281caa50ace4cd4a519981ef6b0edb11374abc3d153aeef7d4af'
```

The golden vector fixtures at `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json` carry both:
- `expected_canonical_bytes_b64` — base64 of the canonical bytes the signer is expected to produce
- `expected_signature_hex` — HMAC-SHA256 of those bytes with `session_key_hex`

When canonical changes, both values must update. The signature value can't be hand-edited; it must be **deterministically recomputed**.

## Recompute script

```python
import json, base64, hmac, hashlib

with open('wp/dailyos/tests/fixtures/hmac_canonical_vectors.json') as f:
    vectors = json.load(f)

for vec in vectors:
    raw = base64.b64decode(vec['expected_canonical_bytes_b64'])
    # Append the new field — exact bytes per canonical schema change.
    # Format: "<name>:<len>\n<value>\n" where len is the byte length of value.
    # Empty value: "<name>:0\n\n"
    new_raw = raw + b'request_id:0\n\n'
    vec['expected_canonical_bytes_b64'] = base64.b64encode(new_raw).decode()

    key = bytes.fromhex(vec['session_key_hex'])
    vec['expected_signature_hex'] = hmac.new(key, new_raw, hashlib.sha256).hexdigest()

with open('wp/dailyos/tests/fixtures/hmac_canonical_vectors.json', 'w') as f:
    json.dump(vectors, f, indent=4)
    f.write('\n')
```

3 vectors in the file (`json_post_single_site`, `binary_body_empty_content_type`, `trimmed_content_type_multisite`) all needed update.

## Inline test assertions also need update

Beyond the JSON fixtures, two PHPUnit files had inline canonical-byte expectations:

### `wp/dailyos/tests/transport/HmacSignerTest.php`

```php
$expected = ...
    . "nonce:1\nn\n"
    . "timestamp:1\n1\n"
    . "request_id:0\n\n";  // ← added
```

### `wp/dailyos/tests/transport/RuntimeClientTest.php`

```php
$expected_signature = ( new DailyOS_Hmac_Signer() )->sign_request(
    ...
    $headers['X-DailyOS-Timestamp'],
    $headers['X-DailyOS-Request-Id']   // ← added (new param)
);
```

## Signals to detect before this breaks CI

Watch for changes to:

1. **`canonical_request_bytes` in Rust** (`src-tauri/src/surface_runtime/hmac.rs`).
2. **`canonical_bytes` method in PHP** (`wp/dailyos/includes/transport/class-dailyos-hmac-signer.php`).
3. **`CanonicalRequest` struct fields** in Rust.
4. **`ParsedSigningHeaders` extraction** (adding optional or required headers).

Any of those triggers fixture recompute. Forgetting it produces PHPUnit red on cycle 1 of the PR.

## Resolution

PR #337 cycle-2 updated all 3 JSON fixtures + 2 inline test assertions. CI cycle-2 all green. Merged 2026-05-20.

## Don't do this

- **Don't hand-edit `expected_signature_hex`.** It's deterministic from key + canonical bytes; any manual edit will be wrong.
- **Don't update only the JSON fixtures and forget the inline `HmacSignerTest` / `RuntimeClientTest` expectations.** They use the same canonical schema but encode it inline.

## Do this

- **Use the Python recompute script** above (or equivalent) immediately after changing canonical input. Commit fixtures alongside the canonical-change commit.
- **Add a comment near the canonical-bytes function** linking back to this entry so future contributors know about the fixture obligation.

## Cross-references

- Retro: `.docs/plans/v1.4.3-wp-foundation/retro.md` §"Cycle-2 PHPUnit failures on PR #337 from HMAC canonical change"
- Substrate files: `src-tauri/src/surface_runtime/hmac.rs`, `wp/dailyos/includes/transport/class-dailyos-hmac-signer.php`
- Fixture file: `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json`
