# ADR-0075: Conversational Interface Architecture

**Status:** Accepted
**Date:** 2026-02-14
**Deciders:** James, Claude

## Context

OpenClaw's chat interface via messaging platforms (145K GitHub stars, Feb 2026) validates user demand for conversational AI in operational intelligence tools. Users want to ask questions like "tell me about Nielsen" or "what's the latest on the Acme renewal" and get synthesized answers from their knowledge base.

DailyOS currently has no chat surface. The app is consumption-first (P7) — users read briefings, not query databases. But conversational exploration is complementary to consumption: the briefing tells you what you need to know proactively; chat lets you dig deeper reactively.

**Key tension:** Building an in-app chat panel is a significant UI investment that risks violating P7 (Consumption Over Production) if users spend more time chatting than reading briefings. But ignoring conversational interaction means losing a validated user need.

**Relevant existing decisions:**
- ADR-0027: MCP dual-mode — already establishes DailyOS as both MCP server (expose tools) and client (consume external data). Chat via MCP tools is a natural extension of the server role.
- ADR-0057: Entity intelligence architecture — entity intelligence data is already structured and queryable.
- ADR-0062: Briefing artifacts vs. live queries — defines the boundary between rendered documents and live query functions.

## Decision

### Phase 1: External via MCP (post-ship)

Claude Desktop (or any MCP-compatible client) calls DailyOS MCP tools to query entities. No in-app chat UI needed.

**MCP tools to expose:**

1. `chat_query_entity(entity_id, question)` — Returns intelligence summary, recent actions, upcoming meetings for an entity. Answers questions like "what's the latest on Acme?"
2. `chat_search_content(entity_id, query)` — Semantic search over entity files (uses ADR-0074 vector search). Answers questions like "what did we discuss about webhooks?"
3. `chat_get_briefing()` — Returns today's briefing data. Answers "what's on my plate today?"
4. `chat_list_entities(type)` — Returns accounts or projects with health/status summary.

**Why external first:**
- Zero UI investment — Claude Desktop already has a polished chat interface
- Validates demand — if users actually use MCP tools, we have evidence for in-app investment
- Aligns with ADR-0027 — MCP server mode is already planned
- Respects P7 — the app stays consumption-first; chat lives in a separate tool

### Session Memory

**SQLite `chat_sessions` table** (consistent with ADR-0048 three-tier model):

```sql
CREATE TABLE chat_sessions (
    id TEXT PRIMARY KEY,
    entity_id TEXT,              -- nullable (general chat not tied to entity)
    entity_type TEXT,            -- 'account' | 'project' | NULL
    session_start TEXT NOT NULL,
    session_end TEXT,            -- NULL if active
    turn_count INTEGER DEFAULT 0,
    last_message TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE chat_turns (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_index INTEGER NOT NULL,
    role TEXT NOT NULL,           -- 'user' | 'assistant'
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_entity ON chat_sessions(entity_id);
CREATE INDEX idx_turns_session ON chat_turns(session_id);
```

Session memory enables continuity: "Last time you asked about Acme, I mentioned the renewal risk. Since then, there's been a new email signal about expansion."

**Retention:** Per-entity, indefinite. Chat history is part of the entity knowledge base. Users can clear history per entity if needed.

### Scope

**Entity-focused initially.** Chat queries are anchored to entities:
- "Tell me about Nielsen" → entity intelligence summary
- "What's the latest on the Acme renewal?" → renewal metadata + recent signals
- "What did Sarah say in the last call?" → semantic search over transcripts

**Workflow-focused later.** Future expansion:
- "Help me prep for my 2pm meeting" → meeting prep assistant
- "Draft a follow-up email to Acme" → action execution
- "What should I focus on today?" → prioritized action list

### Phase 2: In-App Chat Panel (future, if validated)

Only build if Phase 1 MCP tools see significant usage. In-app chat would be:
- Side panel (not full page) — conversation alongside the current page context
- Entity-scoped by default — chat panel inherits context from the entity page you're viewing
- Keyboard shortcut activation (Cmd+K style, but for chat not commands)

**Not a ship blocker.** Phase 2 is future work dependent on user research.

## Consequences

### Easier
- **Zero UI investment for Phase 1** — MCP tools are backend-only, exposed via existing Tauri command pattern
- **Validates demand** — usage data from MCP tools informs whether in-app chat is worth building
- **Session continuity** — SQLite chat history enables contextual follow-up conversations
- **Complements briefings** — proactive (briefing) + reactive (chat) covers both user modes
- **Platform-agnostic** — any MCP client can use the tools, not just Claude Desktop

### Harder
- **MCP server implementation** — need to build and maintain MCP tool handlers
- **Session management** — tracking conversation state adds complexity
- **Context assembly** — each tool call needs to assemble relevant entity context efficiently
- **Latency** — tool calls that hit vector search (ADR-0074) add ~500ms per query

### Trade-offs
- Chose external-first over in-app: lower investment but less integrated UX
- Chose entity-focused over workflow-focused: simpler scope but limits initial utility
- Chose SQLite over filesystem for sessions: consistent with three-tier model but adds schema
- Chose indefinite retention over rolling window: richer context but more storage
