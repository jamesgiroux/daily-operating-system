# v1.4.5 W5-B Validation Report

**Status:** blocked by upstream dependencies  
**Issue:** DOS-476  
**Packet:** `.docs/plans/v1.4.5-workspace-memory/L0-packet-W5-B-DOS-476.md`  
**Branch:** `codex/v1.4.5-w5-e2e-validation`

## Harness Status

| Command | Status | Notes |
| --- | --- | --- |
| `bash tests/v146_validation/redaction_lint.sh --self-test` | pass | Positive and negative lint fixtures behaved as expected. |
| `bash tests/v146_validation/redaction_lint.sh` | pass | Current report draft is privacy-safe. |
| `bash tests/v146_validation/run.sh redaction` | pass | Redaction axis is green. |
| `bash tests/v146_validation/run.sh all` | blocked | Writes blocked evidence because upstream mandatory axes are not available yet. |
| `bash scripts/release-gate/run-v146-validation.sh` | blocked | Wrapper delegates to the W5-B runner and preserves the blocked exit status. |

## Dependency Gate

| Dependency | Status | Evidence |
| --- | --- | --- |
| W5-A backfill registration | open | PR #388 is open. |
| W4 source-management and placement stack | blocked | PR #385 is open and unstable. |
| Graph projection service | blocked | Required before hermetic graph zero-gap assertions can pass. |
| Workspace extractor claim production | blocked | Required before explicit ingestion axes can pass. |
| Workspace lifecycle signal wiring | blocked | Required before the literal signal-chain axis can pass. |
| MCP placement path | blocked | Actual MCP/gateway or registered-handler path is required. |
| Source lifecycle actions | blocked | Each named action must be service-callable before Axis 6 can pass. |

## Evidence Matrix

| Axis | Status | Current evidence |
| --- | --- | --- |
| Backfill registration safety | blocked | Harness shell created; final assertions wait for W5-A on the W5-B base. |
| Explicit ingestion to claim provenance | blocked | Requires merged explicit ingestion and graph projection paths. |
| Trust-band discipline | blocked | Requires trust recompute and promotion/reingest semantics on the merged base. |
| Signal propagation and prep invalidation | blocked | Requires the literal `WorkspaceFileIngested -> EntityIntelligenceUpdated -> prep invalidation` chain. |
| Context inclusion and MCP/privacy parity | blocked | Requires actual MCP/gateway or registered-handler execution. |
| Lifecycle actions and user correction | blocked | Requires all named service actions. |
| Filesystem validation negative fixtures | blocked | Rust negative fixtures are not implemented yet. |
| Redaction lint | green | Self-test and current report scan passed. |

## Manual Evidence Contract

Manual real-workspace evidence may include only entity ordinals, counts, booleans, trust-band distributions, reason-code distributions, opaque HMAC handles, command names, and pass/fail/blocked statuses.

No raw paths, filenames, file contents, claim text, prompt text, output bodies, entity names, emails, domains, raw file IDs, raw source handles, raw content hashes, or provenance blobs may appear in this report.
