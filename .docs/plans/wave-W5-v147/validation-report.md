# v1.4.7 W5-B Validation Report

Status: harness added, full release pass not yet run in this checkpoint.

Automated runner:

```bash
node tests/v147_e2e/run.mjs --axis all
```

Implemented axes:

- `host-selection`: runs the W5-A tool-selection eval.
- `tool-inventory`: validates registered handlers match the catalog and local stdio only advertises registered tools.
- `read-boundaries`: validates runtime-backed read presenters and workspace-memory redaction tests.
- `resource-privacy`: validates opaque resource handles and resource redaction.
- `write-boundaries`: validates note/action/action-status/place-document service routing and cursor-only receipts.
- `binary-smoke`: checks the `dailyos-mcp` binary with `--features mcp`.

Evidence rule: generated JSON reports are written under `src-tauri/target/v147_e2e/` and intentionally include only axis status, command labels, durations, exit codes, and output hashes.
