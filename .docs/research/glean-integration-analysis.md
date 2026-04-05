# Glean + DailyOS: Integration Analysis

**Date:** 2026-02-24
**Context:** VIP rolling out Glean in March 2026. James Giroux has been given early access. This document records the strategic analysis of how Glean and DailyOS relate, where the boundary sits, and what the integration looks like.

---

## What Glean Is

Glean is a $7.2B enterprise Work AI platform (Series F, June 2025; $200M ARR as of December 2025, doubled in nine months). Its architectural bet: enterprise knowledge must be pre-indexed, not queried at runtime.

### The Three Layers

**Enterprise Graph (third generation, September 2025):** Continuously-refreshed knowledge model indexing 100+ enterprise SaaS applications. Maps semantic relationships across disconnected sources — "Reddit the customer" vs "Reddit the platform" are never confused. Pre-computes multi-hop relational reasoning so that complex cross-source inferences happen at index time, not query time.

**Personal Graph:** Per-user layer capturing communication style (up to 5 profiles by content type/audience), work patterns, task/project associations, and collaboration patterns. Important nuance: this is about *work methodology and communication style*, not personal relationship history or CRM-type longitudinal data. It cannot answer "how has my relationship with this stakeholder evolved over 12 months."

**Agentic Engine (v2, Fall 2025):** 100+ native actions across Slack, Salesforce, Microsoft, Jira, GitHub, Google. Scheduled triggers, looping, version-controlled agents, no-code "vibe coding" creation. MCP directory with 20+ pre-loaded partner servers.

### Why Pre-Built Index Beats Federated Search

Glean's own published argument:
- Federated search is constrained by the slowest system and the weakest API's search capability
- No cross-source ranking without a unified model — you get separate fragments from Salesforce and Zendesk with no way to reason across them
- Iterative LLM reasoning to resolve ambiguity is expensive in tokens and latency
- Pre-built index delivers precisely-structured, high-relevance context to each LLM step — "the best way to get deterministic results out of an agent is to supply the exact right info to the LLM per agent step"

The one concession: index sync can introduce minutes-to-hours lag. Glean's counter: organizational knowledge doesn't change fast enough for this to matter in most enterprise use cases.

### MCP Server (Live, September 2025)

Glean's remote MCP server exposes two tools:
- `Search Glean` — natural language query against the full enterprise index, returns permission-filtered results
- `Read document` — retrieve a specific document from the index

Setup: 5 minutes, enterprise OAuth, centrally managed, permissions enforced at every call. Works with Claude Desktop, Cursor, VS Code, ChatGPT, and any MCP-compatible client.

---

## The Boundary: What Glean Does vs. What DailyOS Does

### The One-Line Version

**Glean answers "what does VIP know?"**
**DailyOS answers "what do YOU need to know, and what do you need to do about it?"**

### Structural Differences

| Dimension | Glean | DailyOS |
|-----------|-------|---------|
| Scope | Organization's collective knowledge | Individual professional's personal intelligence |
| Data source | Company systems (Salesforce, Slack, Zendesk, Confluence, Gong) | Personal signals (calendar, email, transcripts, contacts) |
| Interaction model | Reactive — you ask, it answers | Proactive — briefing ready before you open the app |
| Deployment | Enterprise provisioning, IT admin, $100K+ minimum | Self-provisioned, local machine, individual |
| Privacy model | Cloud-indexed, org-governed permissions | Local-first, nothing leaves your machine |
| Relationship intelligence | What the org's systems record about an account | Your personal interaction history, signals, commitments |
| Meeting prep | Surfaces org documents relevant to the meeting | Synthesizes your relationship context, signals, priorities |
| Personal context | Communication style matching | Professional context (value prop, priorities, playbooks) |

### What Glean Cannot Do That DailyOS Does

- "What did *I* promise this customer in our last call?" (transcript not indexed in org systems)
- "This person hasn't responded in 3 weeks — is this relationship cooling?" (individual relationship temperature from personal email patterns)
- "I have 4 meetings today — ranked by my quarterly priorities, here's what matters for each" (synthesized from calendar + personal email + relationship history + declared priorities)
- "My QBR is in 2 hours — here's a briefing from MY perspective, not VIP's" (deep account intelligence from individual signals)
- Proactive daily briefing with no prompting required

Even with Glean's Personal Graph, it captures communication style and work patterns — not longitudinal relationship intelligence with specific stakeholders. If your account context lives in your head, your emails, and your meeting transcripts rather than in Salesforce, Glean doesn't know it.

### What Glean Does That DailyOS Doesn't (and shouldn't try to replicate)

- Enterprise-wide document and knowledge indexing
- Cross-tool synthesis (Zendesk + Gong + Confluence + Salesforce in one query)
- Organizational governance, audit trails, FedRAMP compliance
- Actions across company systems (update Salesforce, create Jira tickets)
- What colleagues across VIP have documented about shared accounts

---

## The Overlap Zone (The Honest Risk)

The zone of overlap is **meeting prep**. Glean is internally testing proactive meeting briefings — building a brief before you ask. If shipped, a VIP employee could receive a Glean-generated briefing for their Acme call drawing on Zendesk tickets, Gong summaries, and Salesforce data.

This is adjacent to DailyOS territory, but not the same thing. Glean's brief will be *VIP's knowledge about Acme*. DailyOS's brief is *your knowledge about Acme* — your relationship history with specific stakeholders, signals from this week's emails, coaching insights from previous calls, your value proposition framing against their specific pain signals, your priorities for this account this quarter.

The frame that resolves this: Glean is the company's institutional knowledge. DailyOS is your personal chief of staff who has read all your emails, attended all your meetings, and knows your professional goals. An executive has both — a chief of staff and access to company knowledge systems. They serve different purposes.

---

## The Integration Opportunity

This is the most important implication of Glean access.

### Current DailyOS enrichment sources for an entity (e.g., Acme account):
- Calendar signals (meeting history, attendance patterns)
- Gmail signals (email threads, commitment tracking, sentiment)
- Meeting transcripts (Quill/Granola outcomes)
- Clay enrichment (person data, title/company)
- Gravatar (profile photos)

### What Glean adds:
- All VIP's Zendesk tickets for this account
- Gong call summaries from any VIP rep who has spoken to them
- Confluence documentation about this account or their industry
- Salesforce notes added by colleagues
- Any internal Slack discussions mentioning this account

With a Glean MCP integration (I340), DailyOS's entity enrichment prompt for Acme could include a Glean-sourced knowledge block: *"Here's what VIP knows about this account across Zendesk, Gong, Salesforce, and internal documentation."* Combined with personal signals, that produces intelligence neither system can generate alone.

### The Architecture

```
Glean context (org knowledge about the entity)  ─────────┐
                                                           ├── DailyOS entity intelligence ──▶ Meeting prep
Personal signals (email, calendar, transcripts)  ─────────┘     + daily briefing
```

The integration flow:
1. DailyOS intel_queue picks up entity for enrichment
2. Before calling Claude, `search_user_context_glean(entity_name, entity_domain)` queries Glean MCP
3. Glean returns permission-filtered org knowledge about the entity (tickets, calls, docs)
4. Glean context is injected into the enrichment prompt as an additional knowledge block
5. Claude produces intelligence that synthesizes both org breadth and personal depth

### Implementation Notes (for I340)

Glean's MCP server is remote, OAuth-managed, enterprise-provisioned. The integration requires:
- Glean MCP connection in DailyOS settings (alongside Clay, Linear, etc.)
- A `search_user_context_glean(query: &str, limit: usize)` function in the enrichment pipeline
- Glean results injected as a "VIP organizational context" block in entity intelligence prompts
- Optionally: include in the user entity `/me` page as an attachments source (documents from Glean's index scoped to user's accounts)

Key constraint: Glean's `Search Glean` tool returns results filtered by the user's organizational permissions. This is correct behavior — a CSM should only see what they're allowed to see about an account.

---

## Strategic Framing for VIP

When explaining DailyOS vs. Glean to colleagues:

> "Glean gives VIP something we haven't had before: a single, intelligent view of how the organisation works, connected to the systems where we get things done."

> DailyOS gives *you* something different: a personal intelligence layer that understands your specific relationships, your priorities, and your workday — and prepares you before you have to ask. Glean makes VIP smarter. DailyOS makes you smarter.

> For a CSM: use Glean to understand what VIP knows about an account. Use DailyOS to understand what you know — and to have a briefing ready before your first meeting of the day, without opening either tool.

The combination is more powerful than either alone. The I340 Glean integration is the architectural expression of this: bringing VIP's org knowledge into your personal intelligence layer.

---

## The Three-Layer Context Model (HBR, February 2024)

*Source: "Context Is the New Competitive Advantage" — Harvard Business Review, February 2024. The article studies two B2B technology services firms with identical CRM processes but materially different execution patterns. The insight: systems of record capture outcomes, not how execution unfolded.*

### What the article gets right

The article's core thesis maps directly onto DailyOS's architecture. It argues: *"Systems of record capture outcomes. They rarely capture how execution unfolds."* That's exactly the gap DailyOS fills — the emails, the chat threads, the patterns of escalation and deference, the relationship temperature signals that never make it into Salesforce but shape whether the deal closes.

The article describes a sales director reading a deal in the CRM that looks healthy. But as she scans recent activity, she notices: a delivery manager asking about scope phasing in a chat thread, a solution architect quietly removing two modules from a pricing model, a customer email saying "budget alignment" rather than "legal review." None of that is in the CRM. Whether it matters depends on the organisation's accumulated execution context.

That's DailyOS in motion. Synthesising ephemeral signals — the email language patterns, the meeting cadence changes, the quiet scope adjustments — into pre-meeting intelligence before you need to look for it.

The article argues that organisational context is a sustainable competitive advantage because it is valuable, rare, hard to imitate, and non-substitutable. The same four criteria apply to the individual's context layer. The CSM who has 18 months of synthesised relationship history with a stakeholder has something genuinely hard to replicate — not because the tool is clever but because the context is theirs.

### What the article misses — and where DailyOS lives

The article frames this as an *organisational* problem. It recommends building a firm-wide context library: "a validated repository of recurring execution patterns linked to outcomes." This is what Glean does.

But it treats the sales director's instinct as an organisational pattern to be captured and distributed. It isn't. Her read on the "budget alignment" phrasing comes from her specific history — her prior deals, her particular relationships at this account, her personal pattern of when that language precedes slippage in *her* book of business, not the company's aggregate. The moment you try to capture that in a shared index, you lose the specificity that makes it signal rather than noise.

The article keeps circling this but never lands on it: **the most valuable context is irreducibly personal.** It can't be fully formalised into an org-level library without flattening the nuance. The individual's judgment is the thing.

### The three-layer model the article implies but doesn't name

The article proposes two layers (systems of record + context layer). There are actually three:

| Layer | What it captures | Owned by | Tool |
|-------|-----------------|----------|------|
| **Systems of record** | Outcomes and transactions | Organisation | Salesforce, Zendesk, Jira |
| **Organisational context** | How the org makes decisions; patterns across roles and deals | Organisation | Glean |
| **Individual context** | How this person works; their specific relationships, judgment patterns, and execution signals | The individual | **DailyOS** |

As AI model access is commoditised, context becomes the differentiator. But there are two types — organisational and individual. The individual layer may be the hardest to replicate, because it can't be indexed without the person's consent and can't be shared without losing what makes it valuable.

### The sharing implication

This is why the brief stays personal and reports are the sharing layer. The sales director's judgment about "budget alignment" phrasing would lose its value in a shared context library — and she'd stop writing honest observations once she knew colleagues and systems could see them. The sharing mechanism must be at the curated output layer (reports, intentionally published), not the signal layer.

See `PHILOSOPHY.md` for the full articulation of this principle.

---

## Reference: Glean Scale Context

- Founded: 2019 (Arvind Jain, ex-Google)
- Series F: $150M at $7.2B valuation (June 2025)
- ARR: $200M as of December 2025 (doubled in 9 months from $100M)
- Gartner: Emerging Leader in Generative AI Knowledge Management (2026)
- CNBC Disruptor 50 (2025)
- Named customers: Booking.com, Comcast, eBay, Infocalc, LinkedIn, Pinterest, Samsung, Zillow, Confluent
- Minimum deal size: $100K enterprise contract
