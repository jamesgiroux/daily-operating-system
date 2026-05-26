# v1.4.7 Tool Selection Eval

This suite validates the MCP v2 tool-description corpus used by Claude Desktop
and other MCP hosts.

The prompt fixtures live in the versioned MCP taxonomy catalog at
`src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml` so tool copy and
selection expectations stay in one source of truth. The manifest in
`fixtures/tool-selection-manifest.json` pins the corpus path, fixture density,
and pass thresholds for the runner.

Run:

```bash
bash tests/v147_tool_eval/run.sh
```
