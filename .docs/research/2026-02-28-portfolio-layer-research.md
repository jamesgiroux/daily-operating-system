# Daily OS: Portfolio Intelligence Layer

## Summary of Discovery Conversation — February 28, 2026

---

## The Problem

Daily OS was built as a personal productivity tool for individual contributors — a zero-build, zero-friction briefing system that synthesizes account-level intelligence and tells you exactly what you need to know before your day starts. It works. The IC layer is solved.

But the organization has multiple layers of leadership above the IC — territory leads, vertical leads, and a VP of Accounts — and none of them have the same synthesized intelligence view for their scope. Today, the VP gets portfolio-level signals through fragmented systems (Gainsight, Salesforce, spreadsheets, tribal knowledge), and Gainsight specifically has become noisy, untrustworthy, and burdensome to maintain. The VP needs the same "Chief of Staff" briefing experience that Daily OS provides at the IC level, but aggregated across the entire book of business.

The core tension: how do we scale Daily OS upward without losing its founding principles — zero build, zero maintenance, 80% consumption / 20% production, editorial calm, and readable-first design?

---

## Key Insights Surfaced

### 1. The VP Briefing Has Three Buckets
Every day, the VP of Accounts is already asking three questions:
- **Portfolio Health** — Which accounts are at risk? Which are expanding? What's net revenue retention trending?
- **Exception Alerts** — What deviated from expected behavior? Usage drops, sentiment shifts, escalations brewing?
- **Strategic Opportunities** — Where's the biggest upside? Which accounts are ready for expansion? Where should I spend my time today?

These are validated as real, daily conversations — not theoretical.

### 2. Glean Solves the Data Unification Problem
Glean already functions as a knowledge graph unifying data across Salesforce, Gainsight, and other systems. Daily OS doesn't need to become a data aggregator. It needs to be a **signal interpreter** on top of Glean — crafting specific prompts and MCP calls to ask the right questions, then presenting answers in a Chief of Staff briefing format.

### 3. Numbers Without Narrative Are Noise
A health score of 87/100 is meaningless without the *why*. Daily OS must synthesize relationship intelligence alongside product telemetry to tell the story behind the number. This is what differentiates it from Gainsight, which became a wall of metrics nobody trusted.

### 4. The Hierarchy Is Fractal
The same intelligence architecture applies at every layer — just aggregated and filtered differently:
- **IC** → Account-level briefing (already built)
- **Territory/Vertical Lead** → Cross-account patterns within their scope
- **VP** → All verticals, full portfolio synthesis

Same sources, same briefing philosophy, different questions and different scope.

---

## Architecture Direction

### One-Way Intelligence Flow (Upward)
The agreed-upon direction is that intelligence flows **up** the hierarchy, not down:

1. Each IC's accounts generate individual `intelligence.json` files locally (already working)
2. These files sync to a remote storage layer
3. Remote synthesis aggregates IC-level intelligence into territory/vertical views
4. VP-level synthesis aggregates across all territories

Glean stays anchored at the account level. It does not run at every layer. The synthesis at each subsequent layer consumes only what's below it.

### Why One-Way Works
- **Avoids feedback loops** — Glean doesn't learn from Daily OS synthesis, preventing an intelligence echo chamber
- **Keeps Glean scoped** — It stays at the IC/account layer where it's most effective
- **Bidirectional awareness happens through conversation** — The VP's synthesis surfaces questions for 1-on-1s, and context flows back through dialogue, not data pipes
- **Meeting briefings become smarter** — When a VP sits down with a territory lead, the brief includes "here are the three signals from your book that got surfaced — let's talk about these"

---

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Start from the VP level and work down | If the VP is happy, everyone else falls in line |
| Glean as the primary signal source at the account level | Avoids building data plumbing; leverages existing knowledge graph |
| Daily OS as signal interpreter, not data aggregator | Stays true to 80/20 consumption-first philosophy |
| One-way (upward) intelligence flow to start | Simple, avoids echo chambers, validates the model before adding complexity |
| Narrative over numbers | Every metric must include the *why*, not just the *what* |
| Maintain zero-build, zero-maintenance ethos at every layer | Don't become Gainsight 2.0 |

---

## What's Left to Unpack

### Architecture & Infrastructure
- **Where does the centralized intelligence layer live?** — Remote storage for synced `intelligence.json` files needs to be defined
- **Authentication & multitenancy** — Moving from local-only to multi-user requires access control, user roles, and tenant isolation
- **Central database alongside local SQLite** — What's the sync model? Conflict resolution? Data freshness?
- **Vector embedding model** — Where does it live? How is it shared across roles without creating redundancy or inconsistency?
- **Sync triggers** — What causes an `intelligence.json` to push upstream? On refresh? On schedule? On change?

### Synthesis Design
- **What questions does each layer ask?** — The IC asks account-specific questions; the VP asks portfolio-pattern questions. These prompt libraries need to be defined per role.
- **How is synthesis structured at the portfolio level?** — Is it a single narrative? Bucketed by the three categories? Exception-driven?
- **How do meeting briefings incorporate upward signals?** — When a VP has a 1-on-1 with a territory lead, what does that enriched brief look like?

### Product & Design
- **Does the editorial calm design scale to portfolio views?** — Dashboards and aggregate views tend toward density; how do you maintain the magazine-spread aesthetic with more data?
- **Role-aware presentation** — How does Daily OS know who's looking at it and what scope to show?
- **Should anything flow back down eventually?** — Start one-way, but is there a future where VP-level context enriches IC briefings? And if so, how do you do that without creating noise?

### Organizational
- **Validation with the actual VP** — Run the three-bucket briefing structure against their real daily workflow
- **Where does Glean end and Daily OS begin?** — Clear boundaries needed to avoid scope overlap and duplicated effort

---

## Design Principles to Protect

As this scales, these non-negotiables must hold:

1. **Zero build** — No setup tax, no configuration burden
2. **80/20 consumption** — Reading first, acting second
3. **Editorial calm** — Readable, not scannable; beautiful, not cluttered
4. **Narrative over metrics** — Every number tells a story
5. **Chief of Staff framing** — "Here's what matters today, and here's why"
6. **No maintenance tax** — The system does the work, not the user
