# I340 — Glean Integration — Organisational Knowledge Enrichment

**Status:** Open
**Priority:** P1
**Version:** 0.15.1
**Area:** Backend / Connectors + Intelligence

## Summary

Glean is VIP's enterprise knowledge graph — indexing Salesforce, Zendesk, Gong, Confluence, Slack, and every other company system into a pre-built relational index. A DailyOS user at VIP has access, via Glean's MCP server, to everything the organisation knows about their accounts. This integration surfaces that organisational knowledge as an additional enrichment source when DailyOS builds entity intelligence. The result: intelligence that synthesises both the organisation's breadth (what VIP knows about Acme) and the individual's depth (what *you* know about Acme from your personal emails, calendar, and transcripts). Neither system produces this alone.

See `.docs/research/glean-integration-analysis.md` for full strategic analysis and architecture.

## Acceptance Criteria

### 1. Connector setup

A "Glean" connector card exists in Settings → Connectors. The user provides:
- Glean instance URL (e.g., `https://vip.glean.com`)
- Personal API token (from Glean Settings → Tokens; or OAuth if Glean's enterprise OAuth is configured)

"Test connection" fires a minimal `Search Glean` query and returns the connected instance name. Token stored in keychain. Verify: `SELECT * FROM connector_config WHERE connector = 'glean'` — row exists after setup.

### 2. Entity search function

`search_glean_for_entity(db, config, entity_name: &str, entity_domain: Option<&str>, limit: usize) -> Vec<GleanResult>` exists in `src-tauri/src/glean/` (new module). It:
- Constructs a search query combining entity name and domain: `"Acme Corp site:acme.com"` or just `"Acme Corp"`
- Calls Glean's search API with the user's token
- Returns a capped list of results (title, snippet, source app, URL) up to `limit`
- Times out after 5 seconds and returns empty list rather than failing enrichment
- Results are permission-filtered by Glean (the user only sees what they're allowed to see)

Verify: with Glean configured and a known account in the DB, call the function — results include Zendesk tickets, Gong summaries, or Salesforce notes about that account from VIP's systems.

### 3. Enrichment prompt injection

When `build_intelligence_prompt` assembles the entity enrichment prompt for an account, it calls `search_glean_for_entity` if Glean is configured and enabled. A "Organisational context from Glean" section is appended to the prompt with the top results (max 3–5 snippets, clearly attributed by source app). If Glean is not configured or returns empty, this section is omitted — no change to existing prompt behaviour.

Verify: with Glean configured and an account that has relevant Glean content, inspect the assembled enrichment prompt (DEBUG log) — it includes a "Organisational context from Glean" section with real snippets. The resulting `entity_intel.executive_assessment` references context that only exists in VIP's Glean index (e.g., a Zendesk ticket detail, a Gong call outcome).

### 4. Connector signal contract

After Glean enrichment contributes to entity intelligence, `emit_signal_and_propagate` is called with `source = 'glean'`. Verify: `SELECT source, entity_type FROM signal_events WHERE source = 'glean' ORDER BY created_at DESC LIMIT 5` — returns rows after an enrichment cycle that included Glean context.

### 5. Freshness and caching

Glean results for an entity are cached for 4 hours in a `glean_cache` table (`entity_id`, `query`, `results_json`, `fetched_at`). On subsequent enrichment cycles within the window, the cached result is used. After 4 hours, a fresh query fires. Verify: trigger two enrichment cycles within 4 hours for the same entity — only one Glean API call logged.

### 6. Rate limiting and graceful failure

If the Glean API returns 429 (rate limit) or any 5xx, `search_glean_for_entity` logs a warning and returns an empty list. Enrichment continues without Glean context. No crashes, no enrichment failures attributable to Glean being unavailable.

### 7. Settings card status

The Glean connector card shows: connected/disconnected status, "last queried" timestamp, entity count that has received Glean context in the last 7 days. A "Disconnect" button clears the keychain token and `connector_config` row.

## Dependencies

- Requires Glean account access with API token capability (VIP rolling out March 2026; access confirmed Feb 2026).
- Glean MCP server is live: exposes `Search Glean` and `Read document` tools. REST API is available as alternative if MCP transport is complex to wire.
- Benefits from I412 (user context in enrichment prompts) being complete — Glean context and user context both feed the same enrichment prompt builder.
- See `.docs/research/glean-integration-analysis.md` for MCP architecture details.

## Notes / Rationale

**Why this matters:** DailyOS's current enrichment sources (calendar, email, transcripts, Clay, Gravatar) are all personal signals. Glean is the first *organisational* signal source. An account enriched with both personal signals and Glean's organisational knowledge produces genuinely richer intelligence than either alone — the individual's relationship context combined with everything VIP knows about the account from Zendesk, Gong, Salesforce, and internal docs.

**The integration is additive, not competitive.** Glean knows what VIP knows about Acme. DailyOS knows what *you* know about Acme. The enrichment prompt with both produces: "here's what you know about this account, plus here's what your organisation knows — synthesised through your professional lens."

**MCP vs. REST API:** Glean's MCP server is the preferred integration path (no SDK, standard protocol, permission enforcement built in). If MCP transport proves complex in the Tauri/Rust context, Glean's REST search API (`POST /rest/api/v1/search`) is a direct alternative with the same permission model.

**Scope boundary:** This integration is read-only. DailyOS does not write to Glean, does not create documents in Glean's index, and does not take actions in Glean-connected systems. It is a context consumer, not an agent.
