# v1.4.5 Workspace Memory Validation Harness

This directory is the W5-B validation orchestrator for DOS-476. It is intentionally separate from product substrate code: Rust integration tests own hermetic DB assertions, and this harness coordinates those tests, redaction checks, and release-gate evidence.

Authoritative plan:

- `.docs/plans/v1.4.5-workspace-memory/L0-packet-W5-B-DOS-476.md`

Current status:

- L0 passed locally on 2026-05-25.
- W5-A is folded into the active W5 validation PR; backfill and redaction axes have automated green checks on the rebased base.
- Signal propagation is green on the release-gate branch.
- Filesystem validation is green for registry-level path rejection, explicit ingestion size/format rejection, and conservative backfill hidden/managed/unsupported skips.
- Graph-audit has automated entity-seeded and inbox assignment evidence plus MCP placement handler registration evidence, but remains blocked at packet level until a successful MCP placement-to-claim fixture is covered.
- Full validation remains dependency-gated on the trust-band matrix, successful MCP placement execution, full context parity, and missing lifecycle actions.
- A blocked axis is not a pass. Interim reports may record `blocked`, but W5-B Done requires all mandatory axes to be green.

## Commands

```bash
bash tests/v146_validation/redaction_lint.sh --self-test
bash tests/v146_validation/redaction_lint.sh
bash tests/v146_validation/run.sh graph-audit
bash tests/v146_validation/run.sh filesystem
bash tests/v146_validation/run.sh signals
bash tests/v146_validation/run.sh redaction
```

`bash tests/v146_validation/run.sh all` is reserved for the final W5-B validation pass. Until the remaining substrate gaps are closed, it should produce blocked evidence and exit non-zero.

## Evidence Rules

Committed and Linear-ready evidence may include only:

- entity ordinals such as `entity_1`
- counts and booleans
- trust-band and reason-code distributions
- opaque HMAC handles generated with a local-only key
- command names and pass/fail/blocked statuses

Evidence must not include raw paths, filenames, claim text, file content, prompt text, output bodies, entity names, customer/company names, emails, domains, raw file IDs, raw source handles, raw hashes, or provenance blobs.
