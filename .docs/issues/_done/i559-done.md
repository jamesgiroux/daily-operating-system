# I559 — Glean Agent Validation Spike

**Version:** v1.0.0 Phase 5
**Depends on:** None (exploration only — uses existing `GleanMcpClient` and Glean OAuth connection)
**Type:** Exploration — no production code
**Scope:** Validation report with go/no-go per agent role. GATE for I535.

---

## Problem

I535 (Glean Agent integration) was specced for v1.1.0 with 6 open questions that affect the implementation pattern. Now that we're pulling it into v1.0.0 Phase 5, these questions must be answered before writing production code. Additionally, we've never explored what tools our Glean MCP server actually exposes beyond `search` — there may be agent invocation tools already available through the MCP transport we already have working.

---

## What to Validate

### 1. MCP Tool Discovery

**Question:** What tools does our Glean MCP server actually expose?

**How to test:** Call `tools/list` on the existing MCP endpoint via `GleanMcpClient`:

```json
{ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} }
```

Document every tool name, its input schema, and what it returns. We currently only use `search` and `read_document`. There may be agent tools, people tools, or structured query tools we don't know about.

### 2. Auth Token Compatibility

**Question:** Does our existing MCP OAuth token (stored in Keychain under `com.dailyos.desktop.glean-auth`) support the Agents REST API (`/rest/api/v1/agents/runs/wait`)?

**How to test:**
- Extract the current access token from `GleanToken`
- Call the Agents API directly: `GET /rest/api/v1/agents/search` with Bearer auth
- If 401/403: need separate API token with `agents` scope → Settings UI impact
- If 200: reuse existing token → simpler integration

### 3. Agent JSON Output Control

**Question:** Can we instruct a Glean Agent to return structured JSON that maps to our I508 types?

**How to test:**
- Create a test agent in Glean Agent Builder with instructions: "Return your analysis as a JSON object matching this schema: { score: number, band: 'green'|'yellow'|'red', risks: [{ text: string, urgency: 'critical'|'watch'|'low' }], ... }"
- Call it via REST API with a real account name
- Assess: does the response parse as valid JSON? How reliably? Does the schema hold?
- If unreliable: need LLM-parse fallback (Strategy B from I535 spec)

### 4. Response Latency

**Question:** How long do purpose-built agents take to respond?

**How to test:**
- Time 5 agent runs with realistic inputs (real account names from our book)
- Record p50, p90, p99 latency
- If < 30s: blocking wait is fine (current `GleanMcpClient` pattern)
- If 30-60s: acceptable but need higher timeout
- If > 60s: need async enqueue-and-poll pattern instead of blocking wait

### 5. Active Connectors

**Question:** Which data sources are connected in our Glean instance?

**How to validate:**
- Check Glean Admin Console (or ask Glean admin)
- Document: Salesforce? Zendesk? Gong? HRIS/org directory? Slack? Confluence? Google Drive?
- Maps to agent role viability:
  - AccountHealthBaseline requires Salesforce + Zendesk
  - StakeholderMapper requires HRIS or org directory
  - CompetitiveIntelligence requires Gong + CRM
  - CallAnalysisSummary requires Gong
  - AccountActivityDigest requires multiple sources

### 6. Rate Limits

**Question:** What are the actual Agents API rate limits?

**How to test:**
- Send 10 agent calls in rapid succession
- Check for 429 responses
- Record: calls-per-minute allowed, cooldown period, per-user vs per-org limits
- Determines budget in I535 (currently 10 calls/enrichment cycle)

---

## Exploration: MCP-Based Agent Invocation

Beyond the REST API path (I535's current design), test whether Glean Agents can be invoked via MCP tools. If `tools/list` reveals agent tools, test:

1. Call the agent tool via existing `GleanMcpClient` JSON-RPC transport
2. Compare latency vs REST API
3. Assess JSON output quality
4. Determine if MCP path is simpler (reuses existing transport, no new HTTP client)

If MCP agent tools exist and work reliably, I535 could use MCP transport instead of REST — simpler code, single auth path, existing error handling.

---

## Exploration: PTY Call Replacement Assessment

For each of our current PTY calls (transcript extraction, entity enrichment, consistency repair), assess whether a Glean Agent could produce equivalent or better output:

| PTY Call | Could Glean Agent Replace? | Assessment Criteria |
|---|---|---|
| Transcript extraction (processor/transcript.rs) | Unlikely — needs raw transcript text, not org knowledge | Glean has Gong call analysis but we need our specific extraction format |
| Entity enrichment (intel_queue.rs) | **Partial** — Glean can fill org-level dimensions (competitive, stakeholder coverage, support health) that local-only enrichment can't | Test: does Glean agent output for "Acme Corp health" produce data that maps to I508 dimensions? |
| Consistency repair (intelligence/consistency.rs) | No — needs the specific intelligence JSON to repair | Local-only concern |

The key question: can Glean Agents produce structured data that maps to our `IntelligenceJson` sub-types (`CompetitiveInsight`, `OrgHealthData`, `StakeholderContact`, `SupportHealth`, `AdoptionSignals`)? If yes, they replace the "fill from thin local context" problem that makes enrichment sparse for accounts with few meetings/emails.

---

## Deliverable

A validation report (`.docs/research/glean-agent-validation-spike.md`) containing:

1. **MCP tool inventory** — every tool exposed by our Glean MCP server, with schemas
2. **Auth compatibility** — go/no-go for reusing MCP OAuth token with Agents API
3. **JSON output quality** — samples of agent JSON output, parsing success rate, schema adherence
4. **Latency report** — p50/p90/p99 for 5+ agent runs
5. **Connector inventory** — active data sources in our Glean instance
6. **Rate limit assessment** — observed limits and recommended budget
7. **MCP vs REST recommendation** — which transport path for I535
8. **PTY replacement assessment** — which enrichment dimensions could Glean Agents fill
9. **Go/no-go per agent role** — table of 5 agent roles with viability assessment

---

## Out of Scope

- Production code (no Rust changes, no frontend changes)
- Creating Glean Agents in Agent Builder (that's admin work, separate from this spike)
- Changing the existing Glean search integration
- I535 implementation (blocked on this spike's output)

---

## Acceptance Criteria

1. Validation report exists at `.docs/research/glean-agent-validation-spike.md`.
2. MCP tool inventory documents all available tools (not just `search`).
3. Auth compatibility tested — clear go/no-go on OAuth token reuse.
4. At least one agent called with a real account name. JSON output sample included.
5. Latency numbers recorded for 5+ agent runs.
6. Go/no-go table for all 5 agent roles with rationale per role.
7. Recommendation: MCP transport vs REST API for I535.
8. Recommendation: which PTY enrichment dimensions could be replaced/augmented by Glean Agents.
