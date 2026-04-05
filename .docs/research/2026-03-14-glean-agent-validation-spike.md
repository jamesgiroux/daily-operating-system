# I559 Validation Spike Results — 2026-03-14

## Executive Summary

The Glean MCP `chat` tool can fully replace PTY calls to Claude Code for entity intelligence enrichment. It returns structured JSON matching our I508 schema, with data from Zendesk, Salesforce, Gong, Slack, and internal docs that local-only enrichment cannot access. Response time is 10-30 seconds vs 60-180 seconds for PTY calls.

**The Agents REST API is not needed.** The `chat` MCP tool IS the agent — it does multi-step reasoning, cross-source synthesis, and produces structured output on request. This simplifies I535 significantly: single transport (MCP), existing auth, no new HTTP client.

**Critical fix discovered:** OAuth scope was missing `mcp` — token was issued without MCP permission, causing all MCP calls to 401. Fixed by adding `mcp search chat agents people entities tools offline_access` to the scope request in `glean/oauth.rs`.

---

## MCP Tool Inventory

3 tools available on `https://automattic-be.glean.com/mcp/default`:

| Tool | Input | Purpose | Key for DailyOS |
|------|-------|---------|----------------|
| `chat` | `message` (string), `context` (string[] optional) | AI-powered multi-source synthesis. Searches across all connected apps, reads documents, reasons over results. | **Primary intelligence tool.** Replaces PTY calls. |
| `search` | `query` + filters (`app`, `after`, `before`, `from`, `owner`, `type`, `exhaustive`, etc.) | Keyword search across all connected apps. Returns document snippets. | Targeted document retrieval. Augments `chat`. |
| `read_document` | `urls` (string[]) | Full document content by URL. Batch-capable. | Deep-dive on specific documents found via search. |

### Connected Apps (from search tool schema)

airtable, announcements, answers, automattic.com, bynder, clubhouse, collections, concepts, customer, datadog, docs.parse.ly, docs.wpvip.com, enablement sites, evernote, field guide, gcp, gdatastudio, **gong**, googlesites, invision, jfrog, learn.wpvip.com, lucid, notion, **p2**, **salescloud**, **servicecloud**, smartsheet, tableau, testrail, trello, workflows, wpvip.com, **zendesk**, zeplin, zoom

**Key data sources for CS intelligence:** Salesforce (salescloud/servicecloud), Zendesk, Gong, P2 (internal comms), Zoom.

---

## Test Results

### Test 1: Support + Commercial Health (Dimension 5-6)
- **Status:** PASS — valid JSON
- **Data quality:** Zendesk ticket analysis (open count, trends, critical issues), expansion signals, product adoption assessment
- **Notable:** Identified recurring cache anomalies and SSL certificate issues from real Zendesk tickets

### Test 2: Stakeholder + Champion Mapping (Dimension 2)
- **Status:** PASS — valid JSON
- **Data quality:** Empty stakeholders for Blackstone external contacts (Glean may not have external contact data), but correctly identified no champion
- **Note:** Better for internal team mapping than external stakeholder discovery

### Test 3: Competitive + Strategic Intelligence (Dimension 1)
- **Status:** PASS — valid JSON, EXCEPTIONAL quality
- **Data quality:** Found 3 competitors (Cloudflare, Akamai, Contentful/Webflow/Drupal) with specific context about positioning, threat level, and strategic alignment
- **Notable:** Discovered beta testing opportunity ("Safe Publisher"), domain mapping initiative, and tech stack consolidation priority — all from internal docs

### Test 4: Value Delivered + Success Metrics (Dimension 4)
- **Status:** PASS — valid JSON
- **Data quality:** Found headless architecture value delivery, speed-to-production metrics, open commitments
- **Notable:** Specific enough to cite in a QBR

### Test 5: Gong Call Analysis (Transcript Replacement)
- **Status:** PASS — valid JSON
- **Data quality:** Found a real Gong call with participants, date, summary, sentiment
- **Limitation:** Gong snippet was thin (social chitchat call), so wins/risks/decisions were empty — but the schema was correct
- **Assessment:** Can supplement but not fully replace transcript extraction (we have the full transcript locally; Gong only has Gong-recorded calls)

### Test 6: People/Org Chart
- **Status:** PASS — valid JSON, EXCEPTIONAL quality
- **Data quality:** 10+ people with emails, job titles, managers, locations, and last interaction URLs (Zendesk ticket + Gong call links)
- **Notable:** This is dramatically better than our Clay/Gravatar enrichment for internal team mapping

### Test 7: Full Executive Assessment (Intel Queue PTY Replacement)
- **Status:** PASS — valid JSON
- **Data quality:** 4-paragraph narrative, working/notWorking/unknowns with specific evidence, health score 65/yellow/medium/stable
- **Notable:** Cites specific Zendesk tickets, deployment patterns, and recurring issues — grounded in real data

### Test 8: Account Discovery from Email Identity
- **Status:** PASS — valid JSON
- **Data quality:** 12 accounts found across Gong (7) and Zendesk (5) with evidence and role attribution
- **Accounts found:** Nielsen, BaT, Janus Henderson, Cox, Heroku, AirBnB, Salesforce, Blackstone, Credit Karma, Newsweek, CEI, Akismet
- **Notable:** Knows role per account (TAM, co-owner, technical side)

### Test 9: Full Onboarding Bootstrap
- **Status:** PASS — valid JSON
- **Data quality:** 2 Salesforce-owned accounts with health bands, renewal data ($105K at 30% probability), stakeholders, last activity
- **Notable:** Single call produces user profile + account list + per-account health snapshots + stakeholders

---

## Auth Findings

| Question | Answer |
|---|---|
| OAuth token compatibility | Works once `mcp` scope is included. Was missing — only `openid profile email` was requested. |
| Agents REST API | 404 — not available on this Glean instance. Not needed — `chat` tool is sufficient. |
| Required scopes | `openid profile email mcp search chat agents people entities tools offline_access` |
| Token refresh | DCR client registrations can expire. Refresh token failed after 8 days. May need periodic re-auth. |

---

## Latency

| Call Type | Latency |
|---|---|
| `tools/list` | 78-82ms |
| `search` | 37-41ms |
| `chat` (simple) | 10-15s |
| `chat` (complex multi-source) | 20-40s |
| `chat` (full I508 schema) | 30-60s |

All within acceptable enrichment pipeline timeouts (current PTY timeout: 180s).

---

## Go/No-Go Per Intelligence Role

| Role | Go/No-Go | Rationale |
|---|---|---|
| Account Health Baseline | **GO** | Full health assessment with support data, commercial context, product adoption |
| Stakeholder Mapper | **GO** (internal) / **PARTIAL** (external) | Excellent for internal team + org chart. Limited external contact visibility. |
| Competitive Intelligence | **GO** | Exceptional — finds competitors from Gong, docs, and internal discussions |
| Call Analysis Summary | **GO** (supplement) | Good for Gong-recorded calls. Cannot replace local transcript extraction for non-Gong calls. |
| Account Activity Digest | **GO** | Cross-source recent activity synthesis works well |
| Account Discovery | **GO** | Can bootstrap account list from user identity — game changer for onboarding |
| Full Intelligence Enrichment | **GO** | Can produce complete IntelligenceJson-compatible output with real cross-source data |

---

## Recommendations

### I535 Redesign: MCP `chat` Tool, Not REST Agents API

I535 should be redesigned around the `chat` MCP tool:
- Reuse existing `GleanMcpClient` transport (already handles auth, timeouts, JSON-RPC)
- No new HTTP client, no agent registry, no agent IDs in Settings
- Structured prompts per intelligence dimension, responses parsed into I508 types
- `context` parameter enables multi-turn refinement if needed

### Onboarding Revolution

With Glean connected, onboarding becomes:
1. User authenticates with Glean (OAuth)
2. Background call: "Find all accounts for {email}" → account list with roles
3. User confirms/adjusts the list
4. Background enrichment: one `chat` call per account → full intelligence
5. User opens app to a fully populated book of business

No Claude Code subscription needed. No manual account entry. No waiting for meetings to accumulate context.

### Dual-Mode Intelligence (Updated)

| Capability | Local (Claude Code PTY) | Remote (Glean MCP chat) |
|---|---|---|
| Entity enrichment | Full IntelligenceJson from local context | Full IntelligenceJson from org knowledge graph |
| Transcript extraction | Full extraction from raw transcript text | Gong call summaries only (supplement, not replacement) |
| Health scoring | 6 algorithmic dimensions from local data | Org-level health baseline + support health from Zendesk |
| People enrichment | Clay + Gravatar | Org chart + internal team + Salesforce contacts |
| Account discovery | Manual entry | Automatic from Salesforce/Gong/Zendesk |
| Competitive intel | From meeting transcripts only | From Gong + Slack + internal docs |

### Signal + Health Scoring Integration

Glean-sourced intelligence should flow through the Intelligence Loop:
- `chat` responses parsed into I508 types → written to `entity_assessment`
- Glean data emits signals with `source = "glean"` at confidence 0.7
- Health scoring dimensions augmented with Glean data (support tickets → `supportHealth` dimension, Salesforce data → `financialProximity`)
- Bayesian source weights track Glean reliability separately from local sources
- User corrections on Glean-sourced data adjust `signal_weights` for `"glean"` source

---

## Files Modified During Spike

| File | Change | Permanent? |
|---|---|---|
| `src-tauri/src/glean/oauth.rs` | Added `mcp search chat agents people entities tools offline_access` to OAuth scopes | **YES** — critical fix, was blocking all MCP calls |
| `src-tauri/src/commands/integrations.rs` | Added `dev_explore_glean_tools` temporary command | No — remove after spike |
| `src-tauri/src/lib.rs` | Registered `dev_explore_glean_tools` command | No — remove after spike |
