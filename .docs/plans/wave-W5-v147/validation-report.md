# v1.4.7 W5-B Validation Report

Status: harness added, full release pass not yet run in this checkpoint.

Automated runner:

```bash
bash tests/v147_e2e/run.sh all
```

Implemented axes:

- `tool-shapes`: validates catalog load, registered handlers, local stdio inventory, grant refresh, and submit handler smoke coverage.
- `privacy`: validates presenter/resource/tool-error redaction and MCP ability-data privacy tests.
- `continuity`: validates first-call envelopes, response handles, conversation-revoked shape, same-client prior-handle reuse, cross-client rejection, and expired-handle replacement.
- `v145-fidelity`: validates workspace-memory placement registration, placement graph writes, graph unit coverage, and source provenance.
- `host-selection`: runs the W5-A tool-selection eval.
- `release-gate`: available as an explicit axis after the mandatory axes pass.

Evidence rule: generated JSON reports are written under `src-tauri/target/release-gate/` and intentionally include only axis status, command labels, durations, exit codes, counts, tool coverage, and continuity statuses. Raw prompts, outputs, entity names, claim text, source handles, domains, customer data, and local paths are omitted.
