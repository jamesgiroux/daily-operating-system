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
| `bash tests/v146_validation/run.sh backfill` | pass | W5-A conservative backfill registration tests pass on the rebased base. |
| `bash tests/v146_validation/run.sh graph-audit` | blocked | Partial explicit ingestion provenance-chain fixture and workspace graph audit projection tests pass; full path matrix remains incomplete. |
| `bash tests/v146_validation/run.sh filesystem` | blocked | Partial negative path validation fixtures pass and leave workspace lifecycle/run/link/claim tables untouched; full negative fixture matrix remains incomplete. |
| `bash tests/v146_validation/run.sh signals` | blocked | Partial signal evidence passes; the literal `WorkspaceFileIngested -> EntityIntelligenceUpdated -> prep invalidation` chain is still not present. |
| `bash tests/v146_validation/run.sh redaction` | pass | Redaction axis is green. |
| `bash tests/v146_validation/run.sh all` | blocked | Backfill and redaction are green; graph-audit, trust, signals, contexts, lifecycle, and filesystem remain blocked. |
| `bash scripts/release-gate/run-v146-validation.sh` | blocked | Wrapper delegates to the W5-B runner and preserves the blocked exit status. |

## Dependency Gate

| Dependency | Status | Evidence |
| --- | --- | --- |
| W5-A backfill registration | green | Folded into active PR #389 after PR #388 closed unmerged; focused backfill and migration coverage pass. |
| W4 source-management and placement stack | partial | Source-management read/action substrate landed, but the MCP placement handler remains a placeholder. |
| Graph projection service | partial | Workspace graph projection unit tests pass and explicit ingestion provenance chain has no local gaps for the direct pipeline fixture; full path matrix remains incomplete. |
| Workspace extractor claim production | partial | Explicit note-like workspace ingestion commits through `commit_claim` with privacy-safe provenance; entity-intake, inbox, and MCP placement path coverage is not complete. |
| Workspace lifecycle signal wiring | partial | Workspace signal emission and direct prep invalidation evidence pass; the required `EntityIntelligenceUpdated` middle hop is absent. |
| MCP placement path | blocked | Actual MCP/gateway or registered-handler execution is required; catalog presence is not enough. |
| Source lifecycle actions | partial | Current action contract exposes reingest, quarantine, and relink only; ignore/scratchpad plus archive/delete are missing. |

## Evidence Matrix

| Axis | Status | Current evidence |
| --- | --- | --- |
| Backfill registration safety | green | Focused W5-A backfill service/bin tests plus v268 migration coverage pass on the rebased base. |
| Explicit ingestion to claim provenance | blocked | `src-tauri/tests/v146_validation.rs` proves direct explicit ingestion creates lifecycle, run, link, and claim rows through service APIs; packet-level path matrix remains blocked on entity-intake, inbox, and MCP placement evidence. |
| Trust-band discipline | blocked | Full matrix still needs recent, stale, pending-review, and reingest-without-freshness cases through real recompute. |
| Signal propagation and prep invalidation | blocked | Partial evidence proves workspace ingestion emits privacy-safe `workspace_file_ingested` and queues prep invalidation; required middle-hop chain remains absent. |
| Context inclusion and MCP/privacy parity | blocked | `dailyos.write.place_document` is cataloged, but the MCP v2 handler is still a placeholder. |
| Lifecycle actions and user correction | blocked | Source-management action contract currently exposes only reingest, quarantine, and relink; ignore/scratchpad and archive/delete remain unavailable. |
| Filesystem validation negative fixtures | blocked | Rust negative fixtures cover traversal, encoded traversal, outside absolute paths, workspace root equality, symlink escape, NUL input, and hardlink rejection when supported; oversized, non-UTF8, managed/hidden, and unsupported-file cases remain to be automated in this axis. |
| Redaction lint | green | Self-test and current report scan passed. |

## Manual Evidence Contract

Manual real-workspace evidence may include only entity ordinals, counts, booleans, trust-band distributions, reason-code distributions, opaque HMAC handles, command names, and pass/fail/blocked statuses.

No raw paths, filenames, file contents, claim text, prompt text, output bodies, entity names, emails, domains, raw file IDs, raw source handles, raw content hashes, or provenance blobs may appear in this report.
