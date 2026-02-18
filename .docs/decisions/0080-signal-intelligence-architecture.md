# ADR-0080: Signal Intelligence Architecture

**Status:** Proposed

**Date:** 2026-02-18

**Participants:** James Giroux, Claude Code

**Related:** I305 (Intelligent meeting-entity resolution), I260 (Proactive surfacing), ADR-0074 (Vector search)

---

## Context

DailyOS collects data from multiple sources — Google Calendar, Gmail, Clay, Gravatar, meeting transcripts, user corrections — but treats every signal equally. A domain match carries the same weight as an attendee pattern. A Clay enrichment about a job change sits alongside a Gravatar profile picture with no hierarchy. The system collects but doesn't learn.

The "it should just know" principle requires a system that:

1. **Fuses signals** from multiple sources into actionable intelligence
2. **Weights signals** by reliability, recency, and relevance
3. **Learns** from user corrections to improve over time
4. **Decays** stale intelligence automatically
5. **Compounds** weak signals into strong convictions
6. **Surfaces uncertainty** rather than guessing silently

This affects every entity type — meetings, accounts, projects, people, actions — not just meeting-entity resolution. The architecture must be universal.

### The Triggering Scenario

A user demos Agentforce (a project) to Jefferies (a customer). The system:
- Can't connect the meeting to the Agentforce project (no project auto-matching)
- Resolves to Salesforce (parent company) via domain matching instead of the tagged project
- Pulls Salesforce Security intelligence instead of Agentforce context
- Doesn't re-enrich when the user manually corrects the entity
- Doesn't learn from the correction to avoid the same mistake tomorrow

Every layer failed: resolution, context selection, correction handling, and learning.

---

## Decision

### 1. Signal Bus Architecture

Introduce a unified signal layer where all data sources produce typed, weighted, time-decaying signals. Signals are stored in a SQLite event log and consumed by entity resolution, intelligence enrichment, and proactive hygiene.

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Calendar    │  │    Gmail     │  │    Clay      │  │  Transcripts │
│  (meetings,  │  │  (threads,   │  │  (contacts,  │  │  (content,   │
│   attendees) │  │   signals)   │  │   companies) │  │   entities)  │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │                 │
       ▼                 ▼                 ▼                 ▼
┌──────────────────────────────────────────────────────────────────┐
│                        SIGNAL BUS                                │
│  signal_events table: (entity, type, source, value, confidence,  │
│                        weight, created_at, decay_rate)           │
└────────────────────────────┬─────────────────────────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
      ┌──────────────┐ ┌──────────┐ ┌────────────────┐
      │ Entity       │ │ Proactive│ │ Intelligence   │
      │ Resolution   │ │ Hygiene  │ │ Enrichment     │
      │ (I305)       │ │ (I260)   │ │ (existing)     │
      └──────────────┘ └──────────┘ └────────────────┘
              │              │              │
              ▼              ▼              ▼
      ┌──────────────────────────────────────────┐
      │           USER FEEDBACK LOOP             │
      │  Corrections → weight updates → learning │
      └──────────────────────────────────────────┘
```

### 2. Signal Schema

```sql
CREATE TABLE signal_events (
    id            TEXT PRIMARY KEY,
    entity_type   TEXT NOT NULL,       -- 'person', 'account', 'project', 'meeting'
    entity_id     TEXT NOT NULL,
    signal_type   TEXT NOT NULL,       -- 'entity_match', 'title_change', 'frequency_drop', etc.
    source        TEXT NOT NULL,       -- 'clay', 'gmail', 'calendar', 'gravatar', 'user', 'transcript'
    value         TEXT,                -- JSON payload (structured signal data)
    confidence    REAL DEFAULT 1.0,    -- raw source confidence [0, 1]
    created_at    TEXT NOT NULL,
    decay_half_life_days INTEGER DEFAULT 90,
    superseded_by TEXT                 -- ID of newer signal that replaces this one
);

CREATE TABLE signal_weights (
    source        TEXT NOT NULL,
    entity_type   TEXT NOT NULL,
    signal_type   TEXT NOT NULL,
    alpha         REAL DEFAULT 1.0,    -- Beta distribution: successes + 1
    beta          REAL DEFAULT 1.0,    -- Beta distribution: failures + 1
    update_count  INTEGER DEFAULT 0,
    updated_at    TEXT NOT NULL,
    PRIMARY KEY (source, entity_type, signal_type)
);
```

### 3. Signal Weighting: Three Layers

**Layer 1 — Source hierarchy (static priors)**

Not all sources are equal. Establish a baseline hierarchy:

| Tier | Sources | Base weight | Rationale |
|------|---------|-------------|-----------|
| 1 | User corrections, user-authored fields | 1.0 | Direct intent — the user told us |
| 2 | First-party content (transcripts, notes) | 0.9 | Rich, timely, contextual |
| 3 | Behavioral patterns (meeting frequency, email responsiveness) | 0.8 | Observable, hard to fake |
| 4 | Third-party enrichment (Clay, Gravatar) | 0.6 | Useful context, not actionable alone |
| 5 | Heuristic inference (title matching, domain matching) | 0.4 | Fragile, frequently wrong |

**Layer 2 — Temporal decay**

Signals lose weight over time. A transcript from yesterday matters more than a Clay enrichment from 3 months ago.

```rust
fn decayed_weight(base_weight: f64, age_days: f64, half_life_days: f64) -> f64 {
    base_weight * (-age_days * (2.0_f64.ln()) / half_life_days).exp()
}
```

Half-life by source type:
- User corrections: 365 days (long memory)
- Transcripts: 60 days
- Calendar patterns: 30 days
- Clay/Gravatar: 90 days (refreshed periodically anyway)
- Heuristic matches: 7 days (should be superseded quickly)

**Layer 3 — Learned reliability (Thompson Sampling)**

Track per-source, per-entity-type reliability using a Beta distribution. When a signal leads to a user correction, update the distribution:

```rust
// User accepted the signal's suggestion → reward
weights.alpha += 1.0;

// User corrected the signal's suggestion → penalty
weights.beta += 1.0;

// Sample reliability when making decisions
fn sample_reliability(alpha: f64, beta: f64) -> f64 {
    Beta::new(alpha, beta).unwrap().sample(&mut rng)
}
```

Over time, the system learns: "Clay signals are reliable for people at enterprise accounts (alpha=28, beta=2) but unreliable for personal contacts (alpha=3, beta=7)."

No ML framework needed. The `rand_distr` crate provides Beta distribution sampling.

### 4. Signal Fusion: Bayesian Combination

When multiple signals point to the same conclusion, they compound. When they disagree, confidence drops. Use naïve Bayesian fusion:

```rust
fn fuse_confidence(signals: &[(f64, f64)]) -> f64 {
    // signals: Vec<(confidence, weight)>
    // Returns combined confidence [0, 1]
    let mut log_odds = 0.0_f64;
    for &(confidence, weight) in signals {
        let c = confidence.clamp(0.01, 0.99);
        log_odds += weight * (c / (1.0 - c)).ln();
    }
    1.0 / (1.0 + (-log_odds).exp())
}
```

Three weak signals (0.4, 0.4, 0.3) fuse to ~0.65. Two strong signals (0.8, 0.9) fuse to ~0.97. One strong signal contradicted by another (0.9 vs 0.1) yields uncertainty (~0.5). This captures the "compounding" behavior the system needs.

### 5. Confidence Thresholds and Hygiene Integration

Fused confidence drives what the system does:

| Confidence | Action | User visibility |
|------------|--------|-----------------|
| **≥ 0.85** | Auto-link silently | None — "it just knows" |
| **0.6 – 0.85** | Auto-link + hygiene flag | Briefing shows entity; hygiene report says "verify" |
| **0.3 – 0.6** | Don't link; suggest in hygiene | "Untagged meeting — did you mean X?" |
| **< 0.3** | Ignore | No action, no noise |

This is the core mechanism that prevents both silent wrong guesses and constant user prompting.

### 6. Entity Resolution Cascade

For meeting → entity resolution specifically, run signals in priority order:

1. **Explicit links** — `meeting_entities` junction. Confidence: 1.0.
2. **Project keyword matching** — Title + description vs project `keywords` field. Uses BM25 for exact matching + embedding similarity for semantic matching.
3. **Attendee group patterns** — Co-occurrence from historical `meeting_entities` + `meeting_attendees`. "When A + B + C meet, 90% of the time it's about entity X."
4. **Attendee entity voting** — Existing person → entity links, majority vote.
5. **Calendar description mining** — Parse body for entity name mentions. Uses `strsim` (Jaro-Winkler) for fuzzy company name matching.
6. **Email thread correlation** — Check `email_signals` for threads mentioning participants + entity names in 48 hours before meeting.
7. **Title/domain heuristics** — Existing normalized string matching. Lowest confidence.

Each signal produces `(entity_id, entity_type, confidence, source)`. Fuse per-entity, pick highest confidence per type (one account, one project). Apply thresholds above.

### 7. Learning from Corrections

When a user changes a meeting's entity assignment:

1. Record in `entity_resolution_feedback`:
```sql
CREATE TABLE entity_resolution_feedback (
    id              TEXT PRIMARY KEY,
    meeting_id      TEXT NOT NULL,
    old_entity_id   TEXT,
    new_entity_id   TEXT,
    signal_source   TEXT,        -- which signal produced the wrong answer
    corrected_at    TEXT NOT NULL
);
```

2. Update `signal_weights` — increment `beta` (failure count) for the signal source that produced the wrong answer.

3. Check for pattern: if the same correction happens N times for the same attendee group, auto-learn the mapping.

4. **Re-enrich** — Invalidate the meeting's prep file and re-queue with the correct entity's intelligence context.

### 8. Cross-Entity Signal Propagation

Signals don't just affect the entity they describe. They ripple:

| Signal | Direct effect | Propagation |
|--------|--------------|-------------|
| Clay: person changed jobs | Update person record | Flag all linked accounts for review |
| Meeting frequency drops 50% | Account engagement warning | Escalate if renewal within 90 days |
| Transcript mentions "churn" | Meeting flagged | Account risk signal, action created |
| Three overdue actions on one project | Project health warning | Account risk if project is customer-facing |
| Email sentiment turns negative | Email signal on person | Account risk if person is champion |

Propagation rules are declarative, stored in code (not a rules engine). Each rule is a function: `signal_event → Vec<derived_signal_event>`.

### 9. Technology Choices

**Use what we have:**
- SQLite for signal storage and weight persistence (no new infrastructure)
- fastembed reranker (already bundled via ONNX Runtime) for signal relevance scoring
- Existing embedding model for semantic entity matching

**Add minimally:**
- `strsim` (0.11) — Jaro-Winkler / Levenshtein for fuzzy entity name matching. ~50KB binary impact.
- `bm25` (0.3) — Keyword search complement to vector search for exact entity name matching. ~30KB.
- `rand_distr` (0.4) — Beta distribution for Thompson Sampling weight learning. Likely already a transitive dependency.

**Explicitly not adding:**
- `linfa` / `smartcore` — ML frameworks are overkill at this scale. Bayesian fusion + Thompson Sampling handle signal weighting without feature matrices or training pipelines.
- `candle` / `burn` — Local LLM inference adds 1-3GB model weight for marginal gain over the existing Claude Code integration for AI enrichment.
- Graph databases — SQLite relationship tables handle 200 entities and 2000 meetings without a dedicated graph engine.
- Event sourcing frameworks (`cqrs-es`, `eventually-rs`) — Designed for distributed systems. SQLite event log is simpler and sufficient.

**Rationale:** At the scale of personal data (50-200 entities, 500-2000 meetings), simple statistical methods outperform complex ML. The bottleneck is signal coverage and data quality, not model sophistication. Every dependency added is a maintenance burden in a native desktop app — keep the stack minimal.

### 10. Implementation Phases

| Phase | Scope | Dependencies | Effort |
|-------|-------|-------------|--------|
| **Phase 1: Foundation** | `signal_events` table, signal schema, basic Bayesian fusion, project keyword matching, re-enrichment on entity correction | `strsim` | 1 sprint |
| **Phase 2: Learning** | `signal_weights` table, Thompson Sampling, user correction feedback loop, confidence thresholds in hygiene | `rand_distr` | 1 sprint |
| **Phase 3: Expansion** | Calendar description mining, BM25 keyword search, attendee group pattern detection, email thread correlation | `bm25` | 1-2 sprints |
| **Phase 4: Propagation** | Cross-entity signal rules, fastembed reranker for signal relevance, derived signals, proactive surfacing integration (I260) | None new | 1-2 sprints |

Phase 1 is I305. Phases 2-4 extend the architecture across the full entity model.

---

## Consequences

### Positive

- **"It should just know" becomes achievable.** The signal bus + Bayesian fusion + learned weights means the system gets smarter with use, not just with more data sources.
- **Uncertainty is surfaced, not hidden.** Confidence thresholds prevent silent wrong guesses. Low-confidence matches become hygiene suggestions, preserving user trust.
- **Integrations contribute proportionally.** Clay and Gravatar add value without adding noise because their signals are weighted by learned reliability, not treated as gospel.
- **No cloud dependency.** Everything runs locally — SQLite storage, statistical math, optional ONNX inference. Aligns with P5 (Local-First, Always).
- **Minimal new dependencies.** Three small crates (`strsim`, `bm25`, `rand_distr`) totaling <100KB binary impact. No ML frameworks, no graph databases.

### Negative

- **Complexity budget.** The signal bus adds a new architectural layer. Every integration must now produce typed signals, not just write to entity fields directly.
- **Cold start problem.** New users have no correction history, so learned weights default to static priors. The system only gets smart after the user interacts for a few weeks.
- **Debugging opacity.** When the system makes a wrong call, tracing *why* through signal fusion + learned weights + temporal decay is harder than tracing a simple if/else chain. Good logging and a "show me why you decided this" diagnostic view are essential.
- **Migration cost.** Existing enrichment pipelines (Clay, Gravatar, hygiene) need to be retrofitted to emit signals rather than write to entity fields directly. This is incremental but touches many files.

### Risks

- **Over-engineering.** At 50-200 entities, a lookup table might outperform Bayesian fusion. Start simple, add sophistication only when simple breaks.
- **Learning rate.** With ~5-10 meetings per day and corrections on maybe 1-2, the Thompson Sampling weights converge slowly. May take weeks to learn meaningful signal preferences. Mitigate with reasonable static priors.
- **Signal spam.** If every integration emits signals for every change, the event log grows fast. Implement signal deduplication and supersession (newer signal of same type replaces older one).

---

## Alternatives Considered

### A. Full ML Pipeline (linfa/smartcore)

Train a logistic regression model on (signal features) → (correct entity) using historical meeting-entity pairs. Retrain periodically.

**Rejected because:** The dataset is too small for reliable ML. 500-2000 meetings with sparse corrections means the model would overfit. Bayesian fusion with learned priors achieves similar accuracy with better interpretability and no training pipeline.

### B. Local LLM for Entity Extraction (candle)

Run a quantized local language model to extract entity mentions from meeting titles, descriptions, and transcripts.

**Rejected because:** Adds 1-3GB model download, significant memory overhead, and inference latency — all for a task that BM25 keyword matching + embedding similarity handle adequately. The existing Claude Code integration already provides LLM-quality entity extraction during enrichment workflows.

### C. External ML Service

Send signals to a cloud API (OpenAI, Anthropic) for classification and fusion.

**Rejected because:** Violates P5 (Local-First, Always). Adds latency, cost, and privacy concerns for an operation that runs on every meeting every morning.

### D. Rules Engine

Define explicit if/then rules for every signal combination: "if attendee = X AND title contains Y, then entity = Z."

**Rejected because:** Doesn't scale. Every new integration or signal type requires new rules. Bayesian fusion handles novel signal combinations automatically. Rules are the fallback for propagation (Section 8) where the logic is genuinely domain-specific.

---

## References

- [fastembed-rs rerankers](https://github.com/Anush008/fastembed-rs) — BAAI/bge-reranker-base for signal relevance scoring
- [strsim](https://github.com/rapidfuzz/strsim-rs) — Jaro-Winkler, Levenshtein for fuzzy entity matching
- [bm25 crate](https://crates.io/crates/bm25) — Keyword search for exact entity name matching
- [Thompson Sampling](https://en.wikipedia.org/wiki/Thompson_sampling) — Beta-binomial model for online weight learning
- [Bayesian signal fusion](https://en.wikipedia.org/wiki/Naive_Bayes_classifier) — Log-odds combination for multi-source confidence
- ADR-0074: Vector search entity content (existing embedding infrastructure)
- I305: Intelligent meeting-entity resolution (first implementation of this architecture)
- I260: Proactive surfacing (consumer of signal intelligence)
- ADR-0081: Event-driven meeting intelligence (meeting intelligence as a signal consumer)

## 2026-02-18 Alignment Note (ADR-0081)

ADR-0081 (Event-Driven Meeting Intelligence, 0.13.0) makes **meeting intelligence a primary consumer** of the signal bus. The architecture diagram in Section 1 should be read as:

```
Signal Bus Consumers:
  - Entity Resolution (I305)
  - Proactive Hygiene (I260)
  - Intelligence Enrichment (existing)
  - Meeting Intelligence (I332 — new)
```

When signals arrive (email, transcript, calendar change, entity update), affected meeting intelligence records are marked "has new signals" (I332). This is the bridge between the generic signal infrastructure (this ADR) and the meeting-specific intelligence lifecycle (ADR-0081). I308 (event-driven signal processing) should implement meeting intelligence as a first-class signal consumer alongside entity resolution and hygiene.
