# Fixture Governance

DOS-216 fixtures under `src-tauri/tests/fixtures/` are committed only after anonymization. They must use synthetic account, person, source, and project identifiers such as `acct-test-N`, `person-test-N`, `src-test-*`, and `proj-test-*`; email domains must be `example.com` or a subdomain such as `acme.example.com`.

## Review Cadence

Run a stale-fixture review quarterly. The review removes fixtures that no longer exercise a live invariant, refreshes fixtures whose source lifecycle metadata is stale, and confirms each bundle still maps to an active regression class or release gate.

When a prompt template version changes, every affected fixture must be rebaselined in the same change or explicitly marked as an intentional pending rebaseline. The per-fixture prompt fingerprint baseline remains the source of truth; accepted regression counts are reporting metadata only.

When a source is revoked, the associated fixture must be regenerated from still-authorized synthetic inputs or removed. Do not mask revocation by editing expected outputs while keeping stale source payloads.

Developers must not share a fixture corpus with each other unless the receiving developer re-runs anonymization locally. Identity maps are local secrets, not review artifacts, and must never be committed.

## Lint Gate

`src-tauri/scripts/check_fixture_anonymization.sh` scans only `src-tauri/tests/fixtures/`. It blocks non-`example.com` emails, phone-like numbers, known customer/vendor names, the `REDACTED` scrub artifact, and in-tree `fixture_identity_map.json` files. A single-line exception is allowed only with:

```text
LINT-ALLOW: fixture-anonymization (justification: ...)
```

Use the exception only for synthetic lint tests or documented false positives. It does not make identity map files valid.

## Capture Mode

Capture mode is future local tooling for W5/W6, once real abilities have real replay outputs to capture. It is opt-in, never runs in CI, and must refuse to write identity maps under the repo.

Proposed CLI shape:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin fixture_capture -- \
  --capture-fixture \
  --ability get-entity-context \
  --scenario bundle-1-cross-entity \
  --out src-tauri/tests/fixtures/bundle-N \
  --identity-map-out ~/.dailyos/fixture-capture/<run-id>/fixture_identity_map.json \
  --tokenize-entity-names \
  --redact-email-phone \
  --redact-free-text
```

The capture tool must tokenize entity names before writing fixture files, redact email, phone, and free-text fields, and write the identity map out of tree. Candidate fixtures are not reviewable until the anonymization lint passes.
