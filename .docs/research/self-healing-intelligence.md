# Self-Healing Intelligence — Research Reference

**Date:** 2026-02-22
**Context:** Informs v0.13.7 (intelligence self-healing redesign)
**Status:** Foundational — this is reference material, not a plan.

---

## What exists in the world

### Entity resolution and data quality

**Splink** (Ministry of Justice, open source) is the most directly analogous system: probabilistic record linkage using the Fellegi-Sunter model, SQLite backend, designed for millions of records but works on small datasets. Key insight: it separates the comparison function (how similar are these two records?) from the match decision (are they the same entity?). DailyOS's `dedup_people_by_domain_alias` does the same thing but only on email domain — missing name variants like "Jim Smith" vs. "James Smith."

**Clearbit/Apollo/Clay** all implement a four-stage enrichment cascade: structural validation (required fields present?) → signal-based inference (infer from existing data) → internal AI enrichment → external API enrichment. Each stage is only attempted if the previous was insufficient. DailyOS has the mechanical and AI stages but no **waterfall controller** tracking which stage has been attempted per entity and whether it should be retried.

**Google Enterprise Knowledge Graph** uses confidence-propagation through entity relationships. If a high-confidence entity (well-known company) is linked to a low-confidence entity (person), the confidence of the person's attributes is boosted. DailyOS has this partially via signal propagation but not for intelligence quality scores directly.

### Self-healing pipelines

**Great Expectations** (data quality framework) implements what it calls "expectation suites" — declarative rules about what the data should look like. When a row violates an expectation, a remediation action fires. For DailyOS: "if intelligence mentions a company name not in the entity's account_domains, flag it" is an expectation that would catch the Jefferies/Adobe Fonts bug.

**dbt's "Observe and Fix" pattern**: tests run after every pipeline transformation. Failures trigger a separate "healing" run that attempts to correct the specific error. DailyOS's hygiene loop approximates this but runs every 4 hours rather than on every pipeline output — missing the event-driven trigger that makes dbt's approach effective.

**Apache Flink enrichment strategies** identify three modes: query-based lookup (expensive), broadcast state (fast but memory-heavy), and CDC-based enrichment (changes propagate as events). DailyOS's signal bus is architecturally a CDC system. The hygiene loop should respond to signal events, not run on a timer.

### Anomaly detection in knowledge bases

**ADKGD (2025)** identifies two complementary anomaly types: structural anomalies (unexpected graph edges) and attribute anomalies (entity attributes inconsistent with neighborhood). For DailyOS, the Adobe Fonts case is an attribute coherence anomaly — the intelligence text is inconsistent with the entity's linked meetings and signals.

**Semantic entropy** (Nature 2024, Farquhar et al.) detects LLM hallucination by generating multiple completions, clustering by semantic similarity, and measuring entropy. High entropy = model is uncertain = likely hallucination. For DailyOS: compare the intelligence embedding against the centroid of linked meeting/email embeddings. High cosine distance = the intelligence content has diverged from what the entity's interactions are actually about.

**KnowGraph (ACM CCS 2024)** encodes domain knowledge as first-order logic rules and flags violations as anomalies. For DailyOS: "if account.health = 'at-risk' then signal_events should contain at least one recent risk signal" is a verifiable rule. Intelligence claiming a health state that has no signal support is a coherence violation.

---

## The four gaps in DailyOS's current hygiene

### 1. No quality scores — binary classification only

Current: `has_intelligence = true/false`, `stale > 14 days`.

Better: **Beta distribution per entity** — `quality_alpha` (evidence of good intelligence) and `quality_beta` (evidence of errors). The mean `alpha/(alpha+beta)` is the quality score; the variance measures uncertainty. A new entity starts at `Beta(1,1)` — maximum uncertainty, mean 0.5. After 10 enrichments with no corrections: `Beta(11,1)` — quality score 0.92. After 3 corrections out of 5 enrichments: `Beta(3,4)` — quality score 0.43, flag for re-enrichment.

This is Thompson Sampling applied to quality rather than source reliability — architecturally identical to what `sampling.rs` already implements for signal sources.

### 2. No semantic coherence checking

Current: intelligence is generated and stored. Nobody checks whether it makes sense in context.

The Jefferies meeting intelligence mentions "Adobe Fonts." The entity is linked to the Agentforce project. "Adobe Fonts" is nowhere in the meeting history, emails, or signals for this entity. This is a topic coherence failure — the AI generated content unrelated to the entity's actual context, likely because an earlier meeting had ambiguous entity resolution and contaminated the context.

The fix: before serving intelligence to meeting prep, compute cosine similarity between the intelligence embedding and the centroid of embeddings for all linked meetings and emails. The `nomic-embed-text-v1.5` model already running in background task #1 supports this. Low similarity → `coherence_flagged` → re-enrichment triggered with explicit meeting context injected.

From TAD-Bench (2025): embedding-based text anomaly detection works best comparing a document against a reference corpus from the same domain. For DailyOS, the reference corpus is the entity's own meeting history.

### 3. No enrichment trigger function — hardcoded 14-day threshold

Current: `get_stale_entity_intelligence(14)` — binary, applies equally to an important renewal account 24h before a meeting and an archived account that hasn't had contact in months.

Better: a continuous trigger score:
```
trigger_score = (meeting_imminence × 0.4)
              + (staleness_rate × 0.3)
              + (entity_importance × 0.2)
              + (signal_delta × 0.1)
```

Where `meeting_imminence` is 1.0 if a meeting is within 24h, 0.0 if no meetings in 7 days; `staleness_rate` is days_since_enrichment / entity_specific_baseline; `signal_delta` is new signals since last enrichment. The 14-day threshold is a special case of this with all other terms zeroed.

From the enrichment literature: the decision to enrich is a cost/benefit trade — AI budget spent vs. meeting value at stake. Making this explicit enables the system to correctly prioritize: always enrich important entities before their meetings, deprioritize dormant entities.

### 4. No feedback closure

Current: user corrections are recorded as signals (`user_correction` signal type in `signal_events`) but are not plumbed back to update source reliability in the Thompson Sampling store.

Result: the system never learns that a specific enrichment source (e.g., Clay enrichment for a particular company type) produces unreliable data. The sampling weights don't change regardless of how many corrections accumulate.

The fix: when a user edits an AI-generated or Clay-enriched field, call `signals::sampling::update_source_reliability(source, entity_type, correction: true)`. This decrements the source's Beta distribution for that entity type. Over time, unreliable sources get lower sampling weights. The system gets better at knowing which sources to trust for which entities.

---

## What the nomic-embed-text model enables that's currently unused

The `nomic-embed-text-v1.5` model (task #1) runs in the background and generates embeddings stored in `content_embeddings`. It supports an 8,192-token context and a `clustering` task type for semantic grouping. Currently consumed by: 4 live read paths via `search_entity_content`.

**Unused capability 1 — intelligence coherence scoring:** Embed the `entity_intel.executive_assessment` text and compare cosine similarity against the centroid of embeddings for that entity's linked meeting titles/summaries. Low similarity = coherence failure. This requires no new model, no new API call — just a query against `content_embeddings`.

**Unused capability 2 — embedding-based person deduplication:** Embed all person names + titles, compute pairwise cosine similarity. Pairs with similarity > 0.85 are stronger merge candidates than email-domain matching alone catches. This would catch "Jim Smith" vs. "James Smith at Acme" — currently missed by `dedup_people_by_domain_alias`.

**Unused capability 3 — Jaccard attendee group matching:** Replace exact SHA256 group hashing in `signals/patterns.rs` with Jaccard similarity (intersection / union) of attendee email sets. Threshold at Jaccard > 0.7. This catches recurring meetings where one person joins occasionally without creating false negatives.

---

## What NOT to build

- **Graph Neural Networks** — cost-effective for 100K+ node graphs. DailyOS has 50–500 entities. k-NN and cosine distance achieve the same outcome with 100x less complexity.
- **Full Splink EM parameter training** — requires enough record pairs to train. With < 500 people, training data is too sparse. Use fixed weights informed by domain knowledge.
- **LLM-based entity resolution** — embedding + Jaro-Winkler achieves >95% precision for person dedup at this scale. Reserve LLM calls for genuinely ambiguous cases that structured matching can't resolve.
- **Web-search-based external validation** — Clay provides this via API. Building it in-house duplicates the integration and adds maintenance burden.

---

## Key references

- [Splink: Probabilistic record linkage, SQLite backend](https://github.com/moj-analytical-services/splink)
- [ADKGD: Dual-channel KG anomaly detection (2025)](https://arxiv.org/abs/2501.07078)
- [Semantic Entropy: Detecting hallucinations via cluster entropy — Nature 2024](https://www.nature.com/articles/s41586-024-07421-0)
- [KnowGraph: Logical-rule anomaly detection (ACM CCS 2024)](https://arxiv.org/abs/2410.08390)
- [TAD-Bench: Embedding-based text anomaly detection (2025)](https://arxiv.org/abs/2501.11960)
- [Thompson Sampling Tutorial — Stanford](https://web.stanford.edu/~bvr/pubs/TS_Tutorial.pdf)
- [Linfa: k-NN, DBSCAN, clustering in pure Rust](https://github.com/rust-ml/linfa)
