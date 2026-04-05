# Architecture First-Principles Review

**Date:** 2026-03-03
**Status:** Decision — informs ADR-0099 withdrawal and replacement
**Context:** After writing ADR-0099 (remote-first, server-canonical architecture), a first-principles review against PHILOSOPHY.md, VISION.md, PRODUCT-PRINCIPLES.md, and POSITIONING.md revealed that the proposed architecture violates DailyOS's core identity. Feedback from Glean system admin (Maeve Lander) further clarified the boundary between DailyOS and Glean.

---

## The Problem ADR-0099 Was Trying to Solve

Three real needs drove the remote-first proposal:

1. **Governance** — if an IC leaves, their intelligence data can't just live on an abandoned laptop with no controls
2. **Team views** — a VP/lead needs portfolio-level visibility across accounts they don't personally touch
3. **Org access** — intelligence that helps build the customer narrative needs to be accessible to the right people

These needs are real. The solution (server-canonical, full sync, shared Postgres) was wrong.

---

## How ADR-0099 Violates DailyOS Principles

### Principle 1: "The most valuable context is irreducibly personal"
> Individual context cannot be fully captured without degrading it; cannot be shared without transforming it.

ADR-0099 puts all intelligence in a shared Postgres database with RLS-filtered views. That's sharing at the raw intelligence layer — exactly what this principle warns against. Two ICs looking at the same account produce different intelligence because they have different priorities and relationships. Merging them into a shared `entity_assessment` table flattens the personal context.

### Principle 3: "Sharing happens at the output layer, never the signal layer"
> Signals → intelligence → brief: private. Intelligence → report: curated, shareable.

ADR-0099 syncs signals, intelligence, assessments, and quality scores to a shared server. That's sharing at the signal layer. The VP would see the IC's raw `entity_assessment` — not a curated report the IC chose to publish.

### Principle 5: "Your brain shouldn't have a landlord"
> Local-first, AI-native. Ownership, not tenancy.

ADR-0099's first principle is "Server is canonical." The user's local data becomes a cache of someone else's server. This directly contradicts the philosophy.

### Principle 7: "Context compounds; access does not"
> Compounding only works if context stays with the user.

If intelligence syncs to a shared DB, compounding happens in shared space. Personal context gets diluted.

### POSITIONING.md: "Not a collaboration tool"
> DailyOS is for the alone part.

ADR-0099 is a collaboration architecture — shared database, team model, org hierarchy.

### The Three-Layer Model
> Systems of record (org, Salesforce) → Organizational context (org, Glean) → Individual context (yours, DailyOS)

DailyOS IS Layer 3. ADR-0099 collapses it into Layer 2. That's Glean's job.

---

## Research Trail: The Answer Was Already There

The research docs from Feb 24 – Mar 1 maintained the correct boundary. ADR-0099 overshot.

**Feb 24 — Glean integration analysis:**
> "The sharing mechanism must be at the curated output layer (reports, intentionally published), not the signal layer."

**Feb 28 — Hook gap analysis:**
> Facts (Glean) → Interpretation (DailyOS) → Decision (Human).
> "Write health summaries to Glean-indexed location so VP's Glean can aggregate across team."

**Feb 28 — Portfolio layer research:**
> One-way upward intelligence flow. IC intelligence → remote storage → territory synthesis → VP synthesis.
> Governance via Google Workspace (admin-managed, DLP, audit trail, retention).

**Mar 1 — Portfolio intelligence architecture:**
> Three options evaluated. Option C (Hybrid) decided: Google Drive for human-browsable intelligence files + optional remote DB for structured queries/rollups/embeddings.
> "The sharing mechanism must be at the curated output layer."

**Mar 2 — ADR-0099:**
> "Server is canonical. All data syncs to shared Postgres."

The leap happened on Mar 2. The research said "output-layer sharing via Google Drive + optional structured DB for rollups." ADR-0099 said "put everything on a server." The research was more nuanced.

---

## Glean System Admin Feedback (Maeve Lander, 2026-03-03)

### Key positions

1. **Lean on Glean for data aggregation, reasoning, and analysis.** Build Glean Agents callable as tools via MCP. DailyOS works with the output and presents it in editorial form, personalized to the user.

2. **Non-DailyOS users should get access to the same intelligence via Glean Agents.** The intelligence capability exists independent of DailyOS.

3. **DailyOS shouldn't be doing call analysis.** Glean absorbs transcripts (from Gong via connector) along with all other company context and provides insights. DailyOS presents the output in a way that adds value to Glean's raw text output.

4. **Individual intelligence files, not shared.** Glean's answers are already somewhat personalized. Shared intelligence files push toward DailyOS becoming a hosted web app — "a huge departure."

5. **The existential question:** "Worth considering that what we're trying to build isn't really DailyOS but rather, a presentation layer for Glean-based customer intelligence reporting?"

### Where Maeve is right

**Glean Agents as MCP tools.** This is the cleanest architecture for org-level intelligence. DailyOS already has `GleanContextProvider` calling Glean's MCP server. Extending to purpose-built Glean Agents (call analyzer, account health assessor, stakeholder mapper) is natural. The agent pattern means:
- DailyOS users get output rendered in editorial UI with personal context overlay
- Non-DailyOS users get the same intelligence through Glean's native interface
- Intelligence capability lives in the governed platform, not on individual laptops

**Gong data through Glean, not direct integration.** Gong → Glean connector → Glean has transcript + all org context → Glean Agent analyzes with full org graph → DailyOS consumes output. DailyOS trying to replicate Gong's analysis locally with only the transcript and none of the org context is strictly worse.

**Individual intelligence files.** Aligns with Principle 1 and the user's instinct about personal lens on data.

### Where Maeve goes too far

**"DailyOS shouldn't be doing ANY analysis on calls."** There are two kinds of call analysis:

1. **What was discussed** — topics, action items, sentiment, talk ratios. This IS Glean's lane. Glean has the transcript + all org context. DailyOS shouldn't replicate this.

2. **What this means for ME** — how this call moves my priorities, what changed in my relationship with this stakeholder, whether expansion signals I've been tracking over 3 calls are strengthening. This requires personal context (priorities, relationship history, strategic lens) that only lives locally. Glean doesn't have the user's context at this depth.

The synthesis is what matters: Glean Agent produces type 1 → DailyOS combines with personal context → produces intelligence richer than either alone.

**"Maybe it's a presentation layer for Glean."** This misses what makes DailyOS valuable:

1. **Proactive, not on-demand.** Glean waits for you to ask. DailyOS has your briefing ready before you open the app. Meeting prep done 30 minutes before the meeting. Continuous, anticipatory intelligence requires local processing on a schedule.

2. **Compounding personal context.** DailyOS gets better over time because it accumulates YOUR corrections, YOUR priorities, YOUR relationship history. Glean's personalization is access-scoped (what you can see), not context-scoped (what matters to you).

3. **The editorial experience.** Narrative-first, magazine aesthetic, finite documents. A Glean presentation layer would be chat or search results. DailyOS is neither.

4. **Zero-guilt, zero-maintenance.** A web app or Glean Agent requires the user to go ask. DailyOS's thesis is the user shouldn't have to ask.

If DailyOS becomes just a presentation layer for Glean, it loses points 1, 2, and 4. It becomes a nice skin.

**The WPVIP/Node app path.** A web app means: login every time, network dependency, server costs, no local Claude Desktop integration, no filesystem for transcript processing, no offline, and — the proactive intelligence loop breaks. The 6am enrichment pipeline that has your briefing ready at 8am requires a local process. A web app prepares only when the user opens the browser.

**However:** There's a case for a lightweight web view for the VP/lead persona who doesn't process transcripts or do enrichment — they consume portfolio-level intelligence that ICs publish. That's not "DailyOS becomes a web app." It's "DailyOS has a web-based portfolio view for consumers of published intelligence."

---

## What Local Gives You That Remote Doesn't

1. **Claude Desktop / Claude Code filesystem access.** Claude Desktop operates on local files. MCP tools read local directories. The workflow — drop a transcript, Claude processes it, intelligence updates — depends on a local filesystem. A web app can't do this without upload friction (violates zero-guilt).

2. **Personal lens on data.** Enrichment runs the user's priorities, context entries, and value proposition through the LLM. Running this server-side means either everyone shares one prompt (loses personalization) or the server stores everyone's personal context (expensive, complex, and now the server has your personal context).

3. **Zero maintenance, zero accounts.** Open the app. It works. No login, no team setup, no org provisioning. The guilt loop starts the moment someone has to "set up their workspace" on a server.

4. **Transcript processing through personal lens.** The original value: transcripts → your priorities → work product (e.g., expansion slide deck using account's own words). Two ICs produce different intelligence from the same transcript because they have different priorities.

5. **Offline-first is real.** Not just airplane mode — "I'm in a meeting and need prep right now and the network is slow." Local means instant.

---

## The Architecture That Serves the Need

### Design

**DailyOS stays local-first and personal.** The core product — proactive briefings, meeting prep, intelligence synthesis through personal lens, editorial UI — runs on the user's machine. Non-negotiable.

**Glean becomes the primary org-level intelligence source.** Instead of DailyOS doing all analysis locally, it calls Glean Agents for org-level intelligence (call analysis, account health baselines, competitive context, stakeholder mapping from org data). DailyOS synthesizes Glean's org-level output with the user's personal context. The `GleanContextProvider` evolves from "search Glean for context" to "call Glean Agents for specific analysis."

**Intelligence stays personal.** Each user has their own intelligence.json (or DB equivalent after workspace file elimination). It's the synthesis of Glean's org knowledge + the user's personal context. Two users looking at the same account get different intelligence.

**Publication is at the output layer.** DailyOS publishes curated outputs — reports, account health summaries — to a governed location (Google Drive Shared folder, indexed by Glean). This is intentional publishing, not automatic syncing. The VP consumes published outputs through Glean or through a lightweight portfolio reader.

**Glean Agents serve non-DailyOS users.** The same intelligence prompts that power DailyOS reports can be deployed as Glean Agents. A non-DailyOS user queries Glean directly. A DailyOS user gets the same output plus personal context overlay plus editorial presentation. DailyOS's moat is the personal lens and proactive experience, not exclusive access to the analysis.

**The optional remote DB is for portfolio rollups, not canonical data.** If/when the VP needs structured portfolio queries (average health, renewal pipeline), a lightweight read-only DB serves queries over published outputs. It's a query layer, not a canonical store.

### How this maps to the three-layer model

| Layer | Owner | Tool | What lives here |
|-------|-------|------|----------------|
| Systems of Record | Org | Salesforce, Gong, Zendesk | Raw data (deals, calls, tickets) |
| Organizational Context | Org | Glean + Glean Agents | Org knowledge graph, cross-source analysis, governed access |
| Individual Context | User | DailyOS (local) | Personal intelligence, priorities, relationship history, corrections |
| Published Outputs | Org (governed) | Google Drive / Glean index | Reports, health summaries — curated, intentional, one-way |

### What analysis lives where

| Analysis type | Where | Why |
|--------------|-------|-----|
| Call transcription + topics + sentiment | Glean (via Gong connector) | Org-level fact extraction. Glean has full org context. |
| Account health baseline (org-level) | Glean Agent | Uses org data (support tickets, product usage, NPS). |
| Stakeholder mapping from org data | Glean Agent | Org directory, reporting lines, role data. |
| Competitive intelligence | Glean Agent | Across all org sources. |
| "What this call means for MY priorities" | DailyOS (local) | Requires personal context (priorities, strategy, relationship history). |
| Health score through personal lens | DailyOS (local) | Combines Glean baseline + personal relationship dimensions. |
| Meeting prep / briefing | DailyOS (local) | Proactive, scheduled, personalized. |
| Reports (VP Account Review, etc.) | DailyOS (local) → published | Generated locally with personal context, published for org consumption. |

---

## Impact on Planning

### ADR-0099: Withdraw and replace

ADR-0099 should be withdrawn. A new ADR should:
1. Reaffirm local-first as the canonical architecture
2. Define the publication model (what, where, when, by whom)
3. Define the governance boundary (org controls published outputs, user controls local data)
4. Position Google Drive as the governed publication layer (extends I426)
5. Position Glean Agents as the org-level intelligence source (extends GleanContextProvider)
6. Scope optional remote DB as a query service over published outputs, not canonical store

### v1.0.0: Scope shrinks to local rearchitecture

The local-architecture cleanup is still valid and still needed:
- **Workspace file elimination** — DB as sole local data layer. #1 bug fix.
- **ServiceLayer** — mandatory mutation path. Good engineering.
- **Schema decomposition** — clean data model.
- **Module decomposition** — god modules broken up.
- **Pipeline reliability** — saga pattern, error boundaries.
- **Frontend cleanup** — ghost components, type alignment.
- **Intelligence foundation** — health scoring, relationships, 6-dimension schema.

What gets removed from v1.0.0:
- ~~Supabase provisioning~~ — no Supabase
- ~~Sync engine~~ — no full sync; replaced by publication mechanism
- ~~Auth / org model / RLS / onboarding~~ — no auth layer in the app
- ~~Server-side embeddings~~ — embeddings stay local
- ~~Admin panel~~ — governance is Google Workspace / Glean
- ~~Conflict resolution / offline redesign / online detection~~ — no sync, no conflicts

What gets added:
- **Publication engine** — write curated outputs to Google Drive Shared folder after enrichment
- **Portfolio reader** — read published intelligence from Shared Drive for portfolio synthesis
- **Glean Agent integration** — call Glean Agents for org-level analysis via MCP
- Extends I426 (Google Drive connector, already built) for publication
- Extends GleanContextProvider for agent calls

### Issues to withdraw (created for ADR-0099)

| Issue | Status |
|-------|--------|
| I510 (Supabase provisioning) | Withdraw |
| I516 (Sync engine) | Withdraw — replace with publication mechanism |
| I517 (Supabase Auth) | Withdraw |
| I518 (Org + territory model) | Withdraw |
| I519 (RLS policies) | Withdraw |
| I520 (Auth-first onboarding) | Withdraw |
| I522 (Server-side embeddings) | Withdraw |
| I523 (Admin panel) | Withdraw |
| I524 (Conflict resolution) | Withdraw |
| I525 (Offline mode redesign) | Withdraw |
| I526 (Online/offline detection) | Withdraw |

Issues that remain valid:
| Issue | Status |
|-------|--------|
| I511 (Schema decomposition) | Keep — local schema cleanup |
| I512 (ServiceLayer) | Keep — remove SyncAction generation, keep mandatory mutation path + signal emission |
| I513 (Workspace file elimination) | Keep — DB as sole local data layer |
| I514 (Module decomposition) | Keep |
| I515 (Pipeline reliability) | Keep |
| I521 (Frontend structural cleanup) | Keep |

### New issues needed

1. **Publication engine** — write curated outputs (reports, health summaries) to Google Drive Shared folder
2. **Portfolio reader** — read published intelligence from Shared Drive, synthesize portfolio view
3. **Glean Agent integration** — extend GleanContextProvider to call Glean Agents for org-level analysis
4. **Glean Agent development** — build call analyzer, account health, stakeholder mapper agents in Glean

---

## Resolved Questions (2026-03-03)

### 1. How much analysis shifts to Glean Agents vs. stays local?

**Position:** Org-level facts/analysis → Glean Agents. Personal interpretation/synthesis → local DailyOS. When both are needed, Glean provides the baseline and DailyOS synthesizes with personal context.

| Analysis | Where | Rationale |
|----------|-------|-----------|
| Call topics, action items, sentiment, talk ratios | Glean (via Gong connector) | Org-level fact extraction. Glean has transcript + all org context. |
| Account health baseline (support tickets, NPS, product usage) | Glean Agent | Org data DailyOS doesn't have access to. |
| Stakeholder mapping from org directory | Glean Agent | Reporting lines, roles, department — org data. |
| Competitive intelligence | Glean Agent | Cross-org sources. |
| Transcript dynamics (I509) | **Split.** Glean extracts raw dynamics. DailyOS interprets what they mean for the user's priorities. | Raw extraction is org-level; personal interpretation requires local context. |
| Relationship inference (I504) | **Split.** Glean provides org-graph relationships (reports-to, department). DailyOS infers relationship quality from personal interaction history. | Org structure = Glean. Relationship quality = personal. |
| Co-attendance inference (I506) | Local | Calendar-based — DailyOS already has the data. |
| Health score through personal lens | Local | Combines Glean baseline + personal relationship dimensions. |
| Meeting prep / briefing | Local | Proactive, scheduled, personalized. |
| "What this call means for MY priorities" | Local | Core personal interpretation loop. |

**Key dependency:** I508 (intelligence schema redesign) is a prerequisite for Glean Agent design. You can't define agent output schemas without knowing your own schema. This is a v1.0.0 → v1.1.0 dependency gate.

**Key constraint:** Glean Agents are generic placeholders until validated. The 5 proposed agents need serious design work in collaboration with the Glean admin. A technical validation spike is required before v1.1.0 planning: test MCP capabilities, structured output support, latency, rate limits.

### 2. Auto-publish or manual publish?

**Position:** Auto-publish with 24-hour review window.

- Principle 3 (sharing is intentional) argues manual. Zero-guilt (maintenance belongs to the machine) argues auto.
- Synthesis: publish automatically after enrichment. User gets notification: "3 account summaries ready to publish." They can review, edit, or retract. After 24h, auto-publish. Default is publish — user only intervenes if something is wrong.
- Settings toggle: "Auto-publish after enrichment" (default on) vs. "Review before publish" (manual queue).
- This mirrors how the briefing works: system does the work proactively, user consumes.

### 3. Does the VP use DailyOS or just Glean?

**Position:** The VP uses Glean. Portfolio Glean Agent is the primary VP surface.

- The VP doesn't process transcripts or run enrichment. They consume portfolio-level intelligence.
- Glean is already the VP's daily tool. Adding another app creates adoption friction.
- A Glean Agent for portfolio intelligence means the VP queries their existing workflow.
- I492 (Portfolio Health page) de-prioritizes — optional, not a blocker for v1.1.0.
- This eliminates the need for auth, org model, or RLS in DailyOS.

**Nuance (from user feedback):** People increasingly work in Claude Desktop, not Glean. Glean is seen as a chat/query surface, not a work surface. The VP persona needs validation: do they actually use Glean as their primary tool, or do they want a dedicated surface? This doesn't change the architecture (publication model works either way) but affects whether I492 stays deferred or gets prioritized.

### 4. What Glean Agents need to be built?

**Position:** Five agents, built by Glean admin team, consumed by DailyOS via MCP:

| Agent | Purpose | DailyOS consumes as |
|-------|---------|-------------------|
| Account Health Baseline | Support tickets, NPS, product usage → structured health score | Input to I499 health scoring (Glean baseline layer) |
| Stakeholder Mapper | Org directory, reporting lines, role data → structured stakeholder graph | Input to I505 (Glean stakeholder intelligence) |
| Call Analyzer | Gong transcripts → topics, action items, sentiment, key moments | Input to meeting prep (replaces local transcript analysis for org-level facts) |
| Competitive Intelligence | Cross-org competitor mentions → structured competitive landscape | Input to enrichment prompts |
| Portfolio Summarizer | Reads published IC intelligence → portfolio synthesis | VP-facing, consumed via Glean directly |

**Key constraints:**
- These don't need to be built before v1.0.0. The existing `GleanContextProvider` (search-based) continues to work.
- These are Glean-side artifacts — we define the prompts and output schemas, Maeve's team deploys them. We don't control the timeline.
- A technical validation spike must confirm Glean's MCP API supports agent-specific endpoints, structured JSON output, and acceptable latency for batch enrichment (20 accounts × 5 agents = 100 calls).

### 5. What gets published vs. stays private?

**Position:** Curated outputs that serve the org narrative → published. Raw signals, personal interpretation, working intelligence → private.

| Content | Published? | Rationale |
|---------|-----------|-----------|
| Reports (VP Account Review, Renewal Readiness, etc.) | Yes | Explicitly designed for org consumption. |
| Account health summary (score + rationale + trend) | Yes | VP needs this for portfolio visibility. One-paragraph synthesis. |
| Stakeholder coverage assessment | Yes | "We know X, Y, Z. No exec sponsor identified." Useful for team alignment. |
| Raw intelligence dimensions (6-dimension scores) | No | Personal assessment. Two ICs score differently. |
| Signals | No | Internal data pipeline. |
| Meeting prep / briefing | No | Personal preparation. Contains user's strategic lens. |
| User priorities / context entries | No | Most personal data in the system. |
| Corrections / feedback | No | Personal calibration. |
| Entity assessment (full) | No | Contains personal interpretation. Published health summary is the curated version. |

**Format:** Structured markdown files in Google Drive Shared folder, one per account per output type. Filename convention: `{account-slug}/{output-type}-{date}.md`. Glean indexes the Shared Drive. Human-readable if opened directly.

**New concept (from user feedback):** Org-aligned work products — standardized success plans, QBR decks — designed by AI, edited/approved/published by user. These are **living documents**, not report snapshots. This pattern needs its own design work: document lifecycle (draft → review → approved → published → updated), structured data for tracking (objectives, milestones), customer-facing sharing mechanism. Potentially evolves I497 (Success Plan report) into something richer.

### 6. Optional remote DB — when and what for?

**Position:** Defer to v1.2.0+. Evaluate after portfolio Glean Agent ships.

- Primary use case is portfolio rollups (average health, renewal pipeline, declining accounts).
- If VP uses Glean, these queries go through Portfolio Summarizer Agent reading published files from Google Drive.
- Google Drive + Glean indexing may be sufficient for v1.1.0.
- A remote DB becomes necessary only if: (a) VP needs real-time dashboards, (b) query complexity exceeds Glean's file-parsing capability, or (c) 50+ accounts makes file-based reading too slow.
- Don't build infrastructure ahead of the need.

---

## Remaining Work Streams

Identified during deep scrutiny of this plan (2026-03-03):

1. **Glean validation spike** — test MCP capabilities, structured output, latency, rate limits. Required before v1.1.0 planning.
2. **Publication design doc** — output schema, versioning, freshness, Google Drive write API (new OAuth scopes), governance model. Required before v1.1.0 implementation.
3. **I509 rewrite** — scope conflicts with Glean analysis boundary. Needs reframe or split.
4. **Org work product design** — success plans as living documents, not report snapshots. New feature stream for v1.1.0+.
5. **Phase 1 specs** — migration spec for I511 (schema decomp), ServiceLayer API design for I512. Required before v1.0.0 implementation.
6. **v1.1.0 version brief** — number TBD issues, define MVP scope, resolve I492 placement. After v1.0.0 ships.
