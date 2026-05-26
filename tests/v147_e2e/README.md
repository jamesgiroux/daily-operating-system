# v1.4.7 MCP v2 E2E Validation

This suite coordinates the W5-B release validation axes for MCP v2. It is
privacy-safe: reports include axis names, command labels, status, and counts,
not payload bodies, entity names, paths, claim text, source handles, or prompts.

Run one axis while iterating:

```bash
bash tests/v147_e2e/run.sh host-selection
```

Run the mandatory W5-B axes:

```bash
bash tests/v147_e2e/run.sh all
```

Run the release gate only after the mandatory axes are green:

```bash
bash tests/v147_e2e/run.sh release-gate
```

Axes:

- `tool-shapes`: catalog, handler registration, local stdio inventory, and submit handler smoke coverage.
- `privacy`: presenter/resource/tool error redaction boundaries.
- `continuity`: server-minted conversation handle reuse, rejection, and expiry behavior.
- `v145-fidelity`: workspace-memory placement, graph, and provenance contracts.
- `host-selection`: W5-A tool-selection eval over the production catalog.
- `release-gate`: hermetic release gate, run explicitly after the mandatory axes.
