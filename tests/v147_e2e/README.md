# v1.4.7 MCP v2 E2E Validation

This suite coordinates the W5-B release validation axes for MCP v2. It is
privacy-safe: reports include axis names, command labels, status, and counts,
not payload bodies, entity names, paths, claim text, source handles, or prompts.

Run one axis while iterating:

```bash
node tests/v147_e2e/run.mjs --axis host-selection
```

Run the full release validation:

```bash
node tests/v147_e2e/run.mjs --axis all
```
