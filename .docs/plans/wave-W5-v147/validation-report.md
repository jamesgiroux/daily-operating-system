# v1.4.7 W5-B Validation Report

Status: mandatory W5-B axes passed locally on 2026-05-26.

Automated runner:

```bash
bash tests/v147_e2e/run.sh all
```

Implemented axes:

- `tool-shapes`: pass, 12/12 checks.
- `privacy`: pass, 12/12 checks.
- `continuity`: pass, 6/6 checks.
- `v145-fidelity`: pass, 10/10 checks.
- `host-selection`: pass, 1/1 check.
- `release-gate`: available as an explicit axis after the mandatory axes pass.

Current evidence:

- Mandatory harness command: `bash tests/v147_e2e/run.sh all`.
- Generated report status: `pass`.
- Tool coverage: 10 expected, 10 validated, none missing.
- Continuity: same-session response handles, prior-handle reuse, cross-client rejection, and expired-handle replacement passed.
- Release gate: not run in this checkpoint.
- Manual Claude Desktop dogfood: not captured in this checkpoint.

Evidence rule: generated JSON reports are written under `src-tauri/target/release-gate/` and intentionally include only axis status, command labels, durations, exit codes, counts, tool coverage, and continuity statuses. Raw prompts, outputs, entity names, claim text, source handles, domains, customer data, and local paths are omitted.
