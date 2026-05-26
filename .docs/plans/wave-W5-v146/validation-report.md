# v1.4.5 W5-B Validation Report

**Status:** blocked by upstream dependencies  
**Issue:** DOS-476  
**Packet:** `.docs/plans/v1.4.5-workspace-memory/L0-packet-W5-B-DOS-476.md`  
**Branch:** `codex/v1.4.5-w5-mcp-placement-validation`

## Harness Status

| Command | Status | Notes |
| --- | --- | --- |
| `bash tests/v146_validation/redaction_lint.sh --self-test` | pass | Positive and negative lint fixtures behaved as expected. |
| `bash tests/v146_validation/redaction_lint.sh` | pass | Current report draft is privacy-safe. |
| `bash tests/v146_validation/run.sh backfill` | pass | W5-A conservative backfill registration tests pass on the rebased base. |
| `bash tests/v146_validation/run.sh graph-audit` | blocked | Entity-seeded and inbox assignment provenance-chain fixtures, workspace graph audit projection tests, and MCP placement handler registration evidence pass; successful MCP placement-to-claim fixture remains incomplete. |
| `bash tests/v146_validation/run.sh filesystem` | pass | Registry path rejection, explicit ingestion size/format rejection, and conservative backfill hidden/managed/unsupported skips pass with privacy-safe evidence. |
| `bash tests/v146_validation/run.sh signals` | pass | `WorkspaceFileIngested -> EntityIntelligenceUpdated -> prep invalidation` evidence passes with privacy-safe payloads. |
| `bash tests/v146_validation/run.sh redaction` | pass | Redaction axis is green. |
| `bash tests/v146_validation/run.sh all` | blocked | Backfill, signals, filesystem, and redaction are green; graph-audit, trust, contexts, and lifecycle remain blocked. |
| `bash scripts/release-gate/run-v146-validation.sh` | blocked | Wrapper delegates to the W5-B runner and preserves the blocked exit status. |

## Dependency Gate

| Dependency | Status | Evidence |
| --- | --- | --- |
| W5-A backfill registration | green | Folded into active PR #389 after PR #388 closed unmerged; focused backfill and migration coverage pass. |
| W4 source-management and placement stack | partial | Source-management read/action substrate landed; MCP placement now has a registered v2 handler path, but successful placement fixture coverage remains incomplete. |
| Graph projection service | partial | Workspace graph projection unit tests pass and explicit ingestion provenance chains have zero local graph-audit gaps for direct entity-seeded and inbox assignment fixtures; successful MCP placement graph evidence remains incomplete. |
| Workspace extractor claim production | partial | Explicit note-like workspace ingestion commits through `commit_claim` with privacy-safe provenance for entity-seeded and inbox assignment paths; successful MCP placement-to-claim coverage is not complete. |
| Workspace lifecycle signal wiring | green | Explicit ingestion emits `workspace_file_ingested`, emits the `entity_intelligence_updated` middle hop, and invalidates affected prep through the middle-hop signal. |
| MCP placement path | partial | Registered-handler/gateway scope-denial execution is now covered; successful MCP placement execution through the live workspace intake path still needs a hermetic fixture. |
| Source lifecycle actions | partial | Current action contract exposes reingest, quarantine, and relink only; ignore/scratchpad plus archive/delete are missing. |

## Evidence Matrix

| Axis | Status | Current evidence |
| --- | --- | --- |
| Backfill registration safety | green | Focused W5-A backfill service/bin tests plus v268 migration coverage pass on the rebased base. |
| Explicit ingestion to claim provenance | blocked | `src-tauri/tests/v146_validation.rs` proves direct entity-seeded ingestion and inbox assignment create lifecycle, run, link, and claim rows through service APIs with zero graph-audit gaps; the MCP placement handler is registered, but the packet-level path matrix remains blocked on successful placement-to-claim fixture evidence. |
| Trust-band discipline | blocked | Full matrix still needs recent, stale, pending-review, and reingest-without-freshness cases through real recompute. |
| Signal propagation and prep invalidation | green | `src-tauri/tests/v146_validation.rs` proves workspace ingestion emits privacy-safe `workspace_file_ingested`, emits privacy-safe `entity_intelligence_updated`, and queues prep invalidation for the affected meeting. |
| Context inclusion and MCP/privacy parity | blocked | `dailyos.write.place_document` now has a registered MCP v2 handler and privacy-safe receipt metadata rendering, but full Tauri/MCP context parity and sensitivity sweep remain unimplemented. |
| Lifecycle actions and user correction | blocked | Source-management action contract currently exposes only reingest, quarantine, and relink; ignore/scratchpad and archive/delete remain unavailable. |
| Filesystem validation negative fixtures | green | Rust negative fixtures cover traversal, encoded traversal, outside absolute paths, workspace root equality, symlink escape, NUL input, non-UTF8 path byte-input rejection, hardlink rejection when supported, explicit ingestion oversized/unsupported-format rejection, and conservative backfill hidden/managed/unsupported skips. |
| Redaction lint | green | Self-test and current report scan passed. |

## Manual Evidence Contract

Manual real-workspace evidence may include only entity ordinals, counts, booleans, trust-band distributions, reason-code distributions, opaque HMAC handles, command names, and pass/fail/blocked statuses.

No raw paths, filenames, file contents, claim text, prompt text, output bodies, entity names, emails, domains, raw file IDs, raw source handles, raw content hashes, or provenance blobs may appear in this report.
