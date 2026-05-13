# W3-0 WP HTTP API byte-exactness spike

DOS-563 / W3-0 transport validation for v1.4.2's W3-B (PHP runtime client
that HMAC-signs canonical request bytes and POSTs to a runtime endpoint).

If `wp_remote_post()` mutates the body between sign-time and wire-time,
HMAC verification breaks downstream. This harness measures that.

## Files

- `byte-exactness.php` — WP-CLI script. Reads `config.json`, POSTs four
  canonical JSON bodies via `wp_remote_post()`, and writes each input
  body to `<outdir>/case-N.input`.
- `receiver.py` — Host-side TCP receiver. Listens on `127.0.0.1:<port>`,
  captures the raw HTTP request bytes per connection, writes each to
  `<outdir>/case-N.raw`, replies with `HTTP/1.1 200 OK` Content-Length 2.
- `run-test.sh` — Harness. Stages the PHP inside Studio's site at
  `wp-content/uploads/dos-w3-0/`, writes `config.json`, starts the
  receiver, invokes `studio wp eval-file`, then compares
  `sha256(case-N.input)` against `sha256(body(case-N.raw))`.

## Run

```sh
./run-test.sh
```

Override defaults with env: `DOS_STUDIO_SITE`, `DOS_BYTE_EXACTNESS_PORT`.

## Bodies tested

1. **leading-trailing-json-whitespace** — space, tab, CRLF before `{` and
   trailing CRLF + tab + space after `}`. Confirms WP does not strip
   outer whitespace.
2. **unescaped-utf8-unicode** — combining accents, CJK, snowman, rocket
   emoji, RTL Hebrew. Confirms WP does not re-encode UTF-8.
3. **escaped-control-whitespace** — backslash-escaped tab/newline/CR,
   forward-slash escape, literal interior spaces.
4. **utf8-bom-and-multibyte** — leading UTF-8 BOM (`EF BB BF`) plus
   multi-byte chars. Confirms BOM is not stripped.

## Notes on the Studio sandbox

- Studio's PHP is Emscripten WASM (`uname=Emscripten ... wasm32`).
  `nc` is not on the sandbox PATH, so the receiver must run on the host.
- The sandbox does not inherit shell env vars; config is passed via
  `config.json` co-located with the PHP file.
- Host `~/Studio/<site>/X` is mounted as `/wordpress/X` inside the
  sandbox. The harness stages under `wp-content/uploads/dos-w3-0/` so
  both sides can read/write the same files.

## Result (2026-05-13)

```
Studio dailyos-dev, WordPress 6.9.4, PHP 8.4.20 (Emscripten WASM)

case-1 (leading-trailing-json-whitespace): PASS  len=131
case-2 (unescaped-utf8-unicode):           PASS  len=94
case-3 (escaped-control-whitespace):       PASS  len=114
case-4 (utf8-bom-and-multibyte):           PASS  len=86

VERDICT: PASS — sha256(input) == sha256(wire body) for all 4 cases.
```

Spot-checked hexdumps: leading `20 09 0D 0A` and leading BOM `EF BB BF`
both arrive intact on the wire. `Content-Length` matches the byte count
of the supplied body, with no transfer-encoding rewrite.

## Recommendation for W3-B

**WP HTTP API is sufficient.** `wp_remote_post()` preserves request body
bytes verbatim across UTF-8, BOM, CRLF, and JSON whitespace edge cases
in Studio's WP 6.9.4 / PHP 8.4.20 environment. HMAC-SHA256 over the
canonical body computed in PHP will validate against the same body
received by the runtime endpoint.

Caveats W3-B should still cover:
- Use `'body' => $bytes` with `$bytes` already serialized; do not pass
  an array, which triggers WP's `application/x-www-form-urlencoded`
  encoder.
- Set `Content-Type` explicitly; do not rely on WP defaults.
- Sign the exact byte string passed to `'body'`, not a re-serialized
  copy.
- Re-run this probe on the production PHP/WP stack used by the runtime
  host before locking in the wave's transport contract.
