# v1.4.7 W5-B Validation Report

Status: mandatory W5-B axes passed locally on the rebased branch on 2026-05-26; hermetic release gate failed outside the MCP-specific axes.

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

- Mandatory harness command: `node tests/v147_e2e/run.mjs --axis all-with-release-gate --out src-tauri/target/release-gate/v147-e2e-with-release-gate.json`.
- MCP axis status: five MCP-specific axes passed; combined report status is `fail` because the release-gate axis failed.
- Tool coverage: 10 expected, 10 validated, none missing.
- Continuity: same-session response handles, prior-handle reuse, cross-client rejection, and expired-handle replacement passed.
- Release gate: failed with exit code 2 after the five MCP axes passed. Release-gate evidence reported 12/21 mandatory invariants passing, with infra failures in legacy bundles and several non-MCP-specific invariant failures.
- Manual Claude Desktop dogfood: not captured in this checkpoint.

Evidence rule: generated JSON reports are written under `src-tauri/target/release-gate/` and intentionally include only axis status, command labels, durations, exit codes, counts, tool coverage, and continuity statuses. Raw prompts, outputs, entity names, claim text, source handles, domains, customer data, and local paths are omitted.
