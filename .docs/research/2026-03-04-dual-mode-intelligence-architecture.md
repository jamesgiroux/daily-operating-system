# Dual-Mode Intelligence Architecture

**Date:** 2026-03-04
**Status:** Design proposal — informs v1.0.0 interface design and v1.1.0 Glean Agent planning
**Context:** DailyOS needs to support intelligence from both local computation (current) and remote Glean Agents (future). This document defines the architectural boundary: what DailyOS owns permanently vs. what becomes pluggable.

---

## The Thesis

DailyOS's value is NOT in intelligence computation. Glean can compute health scores, extract transcript insights, and map stakeholders too — often with richer org-level data than DailyOS has locally. DailyOS's moat is four things that no remote service replicates:

1. **Personal state machine** — what you've seen, corrected, dismissed, what changed since yesterday
2. **Consumption surface** — proactive morning briefing, meeting prep 30 minutes before the call, editorial magazine layout
3. **Orchestration** — when to refresh, what to prep, signal decay, enrichment triggers, the 6am pipeline that has your day ready at 8am
4. **Correction loop** — user corrections shape future output via Bayesian source weighting, not just prompt engineering

Intelligence computation — health scoring, transcript analysis, stakeholder discovery, competitive assessment — is an input to these four capabilities, not the capability itself. Whether that input comes from local processing or a Glean Agent, DailyOS's state machine, surfaces, orchestration, and correction loop work identically.

**Key distinction from ADR-0099:** ADR-0099 proposed "server is canonical, all data syncs to shared Postgres." This proposal is fundamentally different: **local is canonical, computation is pluggable.** DailyOS's core stays on the user's machine. The question is whether intelligence INPUTS come from local processing or remote agents — not where the canonical data lives. ADR-0099 was withdrawn because it violated "your brain shouldn't have a landlord." This proposal preserves that principle entirely.

---

## Data Flow: Three Modes

### Local Mode (v1.0.0 — current)

```
Calendar/Gmail ─→ Google API ─→ local processing ─→ SQLite
                                      │
Workspace files ─→ file processing ───┘
                                      │
              Claude Code PTY ────────┘
                                      │
                                      ▼
                              SQLite (canonical)
                                      │
                                      ▼
                          Briefings / Meeting Prep / Reports
```

All intelligence computed locally. LLM calls via Claude Code PTY. Health scoring from first-party data (meetings, emails, stakeholders). Glean provides search context but not structured intelligence.

### Remote Mode (v1.1.0+ — Glean Agents as primary)

```
Glean Agents ─→ structured JSON ─→ DailyOS stores in SQLite
    │                                       │
    ├─ Account Health Agent                 │
    ├─ Stakeholder Mapper Agent             │
    ├─ Call Analyzer Agent                  │
    ├─ Competitive Intel Agent              │
    └─ Portfolio Summarizer Agent           │
                                            ▼
                                    SQLite (canonical)
                                            │
                              ┌─────────────┼─────────────┐
                              ▼             ▼             ▼
                          Briefings    Meeting Prep    Reports
```

Glean Agents produce structured intelligence matching DailyOS's schema types (`OrgHealthData`, `CompetitiveInsight`, `SupportHealth`, etc.). DailyOS stores results in SQLite and applies personal context overlay (priorities, corrections, relationship history). The state machine, orchestration, and correction loop operate identically — they don't care where the input came from.

### Hybrid Mode (recommended target)

```
                    ┌─── Glean Agents (org-level) ───┐
                    │                                 │
Calendar/Gmail ──┐  │  OrgHealthData                  │
                 │  │  StakeholderGraph                │
Workspace files ─┤  │  CompetitiveInsight              │
                 │  │  TranscriptOutcomes              │
                 ▼  ▼                                  │
           SQLite (canonical)                          │
                 │                                     │
    Local computation (personal):                      │
    ├─ RelationshipDimensions (6 dims)                 │
    ├─ Personal interpretation of meetings             │
    ├─ Correction-weighted source reliability           │
    ├─ Signal momentum from first-party data           │
    └─ Prep invalidation + orchestration               │
                 │                                     │
                 ▼                                     │
         Merged intelligence                           │
         (org baseline + personal context)             │
                 │                                     │
                 ▼                                     │
     Briefings / Meeting Prep / Reports ──→ S3 publish ┘
```

Glean provides org-level baselines. DailyOS computes personal relationship dimensions locally. The health scoring engine (I499) merges both via ADR-0097's "One Score, Two Layers" pattern: org score as baseline, relationship dimensions as personal overlay, divergence detection when they disagree.

---

## What Stays Local (Regardless of Mode)

| Capability | Why it can't be remote |
|-----------|----------------------|
| Personal priorities (annual + quarterly) | Irreducibly personal. No org service has this. |
| User corrections + feedback | Shapes future output via Bayesian weighting. Must compound locally. |
| State tracking (seen/dismissed/changed) | Per-user interaction state. No shared surface. |
| Prep invalidation logic | "This meeting's briefing is stale because the stakeholder changed" — requires local signal bus. |
| Orchestration + scheduling | 6am pipeline, 30-min pre-meeting prep, signal decay. Local scheduler. |
| Relationship dimensions (I499) | Computed from first-party data: calendar, email, stakeholder links. |
| Editorial rendering | Magazine layout, Newsreader typography, finite documents. The surface. |
| Correction loop (signals/feedback.rs) | Bayesian source weighting. Penalizes bad sources, rewards good ones. |

---

## What Becomes Pluggable

| Intelligence input | Local provider | Remote provider (Glean Agent) |
|-------------------|---------------|------------------------------|
| Org health baseline | None or Glean search (sparse) | Account Health Agent (structured `OrgHealthData`) |
| Stakeholder discovery | AI inference from meetings | Stakeholder Mapper Agent (org directory, roles) |
| Transcript outcomes | Claude Code PTY on local files | Call Analyzer Agent (Gong via Glean connector) |
| Competitive context | Meeting mention extraction | Competitive Intel Agent (cross-org sources) |
| Support health | None (no local source) | Account Health Agent (Zendesk/Intercom via Glean) |
| Product adoption | None (no local source) | Account Health Agent (usage data if indexed) |
| Executive assessment | Claude Code PTY synthesis | Could be Glean Agent, but personal lens matters |

**Interface:** Each pluggable input maps to an existing type from I503/I508: `OrgHealthData`, `CompetitiveInsight`, `SupportHealth`, `AdoptionSignals`, `SatisfactionData`. The schema is already source-agnostic — `source: String` fields track provenance. No new types needed for remote mode. Same struct, different fill path.

**The IntelligenceProvider pattern (ADR-0091):** Currently scoped to LLM provider abstraction (Claude Code vs. Ollama vs. OpenAI). This naturally extends to the full intelligence pipeline:

```rust
trait IntelligenceProvider {
    // Current (ADR-0091): LLM call abstraction
    async fn assess(&self, prompt: &str) -> Result<String>;

    // Extended: structured intelligence input
    async fn get_org_health(&self, account_id: &str) -> Result<Option<OrgHealthData>>;
    async fn get_stakeholders(&self, account_id: &str) -> Result<Vec<StakeholderData>>;
    async fn get_transcript_outcomes(&self, meeting_id: &str) -> Result<Option<TranscriptSentiment>>;
}
```

v1.0.0 implements `LocalIntelligenceProvider`. v1.1.0 adds `GleanAgentProvider`. The consumer code (`intel_queue.rs`, `health_scoring.rs`) calls the trait, not the implementation.

---

## Transcript Ownership

In local mode, DailyOS processes transcripts directly (Quill/Granola files dropped into workspace dirs). In remote mode, Glean owns transcripts via Gong/Zoom connectors and provides structured outcomes:

| Mode | Transcript source | DailyOS receives |
|------|------------------|-----------------|
| Local | User-dropped .md/.txt files | Raw text → Claude Code PTY → `TranscriptSentiment` |
| Remote | Gong → Glean connector | Structured outcomes via Call Analyzer Agent |
| Hybrid | Both | Merge: Glean provides org-level facts, DailyOS adds personal interpretation |

In remote mode, local transcript processing (Quill/Granola) becomes unnecessary for accounts covered by Gong. DailyOS queries the Call Analyzer Agent for structured outcomes instead of processing raw text. Local processing remains available as fallback and for meetings not recorded in Gong.

---

## S3 Publication Path

```
DailyOS (local) ──→ structured artifacts ──→ governed S3 bucket ──→ Glean indexes ──→ org-searchable
                         │
                         ├─ Account health summaries (score + narrative + trend)
                         ├─ Stakeholder coverage assessments
                         ├─ CS reports (VP Account Review, Renewal Readiness, etc.)
                         └─ Portfolio-level rollups
```

Publication is at the output layer per Principle 3. Raw signals, personal corrections, and working intelligence never leave the device. Only curated summaries are published.

### The Feedback Loop Hazard

If DailyOS publishes to S3, Glean indexes S3, and DailyOS queries Glean for context, DailyOS may read back its own published output as if it were independent org intelligence. This creates a circular dependency: DailyOS's health assessment of Acme appears in Glean search results for Acme, reinforcing the same assessment on next enrichment.

**Solution:**
1. **doc_type tagging** — all published artifacts include `doc_type: "dailyos_published"` metadata. DailyOS's Glean queries filter `NOT doc_type:dailyos_published`.
2. **Exclusion filters** — `GleanContextProvider` maintains a list of DailyOS publication prefixes and excludes them from search results.
3. **Source tracking** — `DataSource` enum (ADR-0098) tags any data originating from DailyOS publications so the correction loop doesn't treat it as independent evidence.

---

## Implications for v1.0.0

v1.0.0 builds intelligence locally. But three design decisions ensure v1.1.0 remote mode requires no refactoring:

1. **Source-agnostic types.** I508's intelligence schema uses `source: String` on sub-structs. I503's `OrgHealthData` accepts data from any provider. I499's `compute_account_health()` takes `Option<OrgHealthData>` — None in local mode, populated in remote mode. No schema changes needed to swap providers.

2. **Trait-based interfaces.** The `IntelligenceProvider` trait (ADR-0091) abstracts LLM calls. Extend to abstract intelligence inputs. Consumer code calls the trait, not the implementation.

3. **Separation of computation and orchestration.** Health scoring (I499) computes dimensions from data. Orchestration (`intel_queue.rs`, `scheduler.rs`) decides when to compute and what to do with results. These are cleanly separated — swapping the computation backend doesn't touch orchestration.

**What v1.0.0 must NOT do:**
- Hard-code local file paths into intelligence interfaces
- Assume transcript text is always available (remote mode provides structured outcomes)
- Couple health scoring computation to the prompt that triggers it
- Skip `source` fields on intelligence types ("we'll add those later" means refactoring)

---

## Glean Agent Validation Gates

Before committing to remote mode in v1.1.0, the following must be validated:

| Gate | What to test | Pass criteria |
|------|-------------|---------------|
| MCP structured output | Can Glean Agents return typed JSON matching our schemas? | Response deserializes into `OrgHealthData` without field mapping |
| Latency | Batch enrichment: 20 accounts x 5 agents = 100 calls | P95 < 30s per agent call; total batch < 10min |
| Rate limits | Glean MCP API rate limits for batch operations | No throttling at 100 calls/batch |
| Agent-specific endpoints | Can we call specific agents vs. generic Glean search? | Named agent invocation via MCP tool call |
| Schema stability | Do agent output schemas remain stable across Glean updates? | Versioned schemas or backward-compatible evolution |
| Fallback | What happens when a Glean Agent is unavailable? | Graceful degradation to local mode per-dimension |
| Auth scope | Does DailyOS's Glean OAuth scope cover agent invocation? | Agent calls succeed with existing token |

A technical validation spike (not a full implementation) must confirm these before v1.1.0 planning begins. See first-principles review (2026-03-03) for the five proposed agents and their output schemas.

---

## Relationship to Existing Architecture

| Document | Relationship |
|----------|-------------|
| ADR-0099 (withdrawn) | ADR-0099 proposed server-canonical sync. This proposal keeps local canonical and makes computation pluggable. Different answers to the same question. |
| ADR-0091 (IntelligenceProvider) | Extended from LLM abstraction to full intelligence input abstraction. Same pattern, broader scope. |
| ADR-0097 (health scoring) | "One Score, Two Layers" maps directly: org layer is pluggable (local or Glean Agent), relationship layer stays local. |
| ADR-0098 (data governance) | `DataSource` enum tracks provenance regardless of computation mode. Purge-on-revocation applies to Glean Agent data too. |
| First-principles review (2026-03-03) | Established that DailyOS is Layer 3 (individual context). This proposal preserves that — remote agents provide Layer 2 inputs, DailyOS synthesizes into Layer 3. |

---

## Summary

DailyOS is not a presentation layer for Glean. DailyOS is a personal intelligence operating system that happens to accept inputs from multiple sources — including Glean Agents. The personal state machine, proactive consumption surface, orchestration engine, and correction loop are the product. Intelligence computation is a pluggable input. Build v1.0.0 locally with pluggable interfaces. Add Glean Agent backends in v1.1.0. The data model, rendering, and orchestration remain the same.
