# I395 — Email Relevance Scoring — Signal-Driven Surfacing for Briefings

**Status:** Open (0.13.1)
**Priority:** P0
**Version:** 0.13.1
**Area:** Backend / Intelligence + Frontend / UX

## Summary

Enriched emails currently surface on the daily briefing and email page by mechanical priority (high/medium/low from keyword-based classification). A calendar acceptance notification from `noreply@` gets the same treatment as a renewal discussion from a key account contact. The AI enrichment pipeline (I367/I369) produces rich intelligence — contextual summaries, sentiment, urgency, entity resolution — but none of this feeds back into a scoring decision about *what's worth showing*.

This issue builds an email relevance scorer that uses the existing signal infrastructure (signal bus, Bayesian fusion, embedding-based relevance, time decay) to compute a per-email relevance score. The score determines what surfaces on the daily briefing (top 3-5 emails by score), how the email page is ordered (score descending, not mechanical priority), and what the "Worth Your Attention" label actually means.

The scorer is a **consumer** of existing signal infrastructure, not a new system. It composes:
- Entity linkage signals (from I372 email-entity compounding)
- Bayesian fusion (from `signals/fusion.rs`) for multi-signal confidence
- Embedding relevance (from `signals/relevance.rs`) for today's-meetings similarity
- Time decay (from `signals/decay.rs`) for freshness
- Keyword relevance for business-critical terms

The pattern follows `signals/callouts.rs` (callout generation from signals) and `focus_prioritization.rs` (multi-factor action scoring) — both are existing models for "score items, rank, surface the top ones."

## Architecture

### Shared Service: `signals/scoring.rs` (new)

A general-purpose entity-aware item scorer. Not email-specific — designed so actions, meetings, and future item types can use the same scoring infrastructure.

```rust
pub struct ScoringContext {
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub content_text: String,           // For embedding similarity
    pub urgency: Option<String>,        // From AI enrichment
    pub sentiment: Option<String>,      // From AI enrichment
    pub source_type: String,            // "email", "action", "meeting"
    pub created_at: String,             // For time decay
}

pub struct ScoringResult {
    pub total_score: f64,               // 0.0 - 1.0 normalized
    pub entity_score: f64,              // Entity linkage + signal compound
    pub relevance_score: f64,           // Embedding similarity to today
    pub urgency_score: f64,             // AI-assessed urgency
    pub keyword_score: f64,             // Business-critical term matches
    pub recency_score: f64,             // Time decay
    pub reason: String,                 // Human-readable scoring rationale
}
```

The scorer is a pure function: `score_item(db, model, context, todays_meetings) -> ScoringResult`. No side effects, no DB writes, no state mutation.

### Email Scoring: `signals/email_scoring.rs` (new)

Email-specific adapter that maps `DbEmail` → `ScoringContext` and applies email-specific rules:

- **Entity linkage** (+0.30 max): Has resolved entity? Is entity a known account vs unknown person? Does entity have active signals in `signal_events`? Uses `fusion::fuse_confidence` to compound entity signal strength.
- **Meeting relevance** (+0.25 max): Embedding similarity between email content and today's meeting titles/descriptions. Uses existing `relevance::rank_signals_by_relevance` pattern. An email about "Acme renewal" scores high when you have an Acme meeting today.
- **AI urgency** (+0.20 max): Direct from enrichment output. `high` = 0.20, `medium` = 0.08, `low` = 0.02.
- **Keyword relevance** (+0.15 max): Business-critical terms in contextual summary: renewal, expansion, contract, order form, escalation, churn, deadline, budget, executive, QBR. Weighted by term importance.
- **Recency** (+0.10 max): Time decay from `received_at`. Uses existing `decay::decayed_weight`. Today = 0.10, yesterday = 0.07, 3 days ago = 0.03.
- **Noise penalty** (-0.50): Calendar notifications, noreply addresses, newsletter patterns. Hard penalty that pushes score below any reasonable threshold.

**Threshold:** Emails scoring below 0.15 don't surface on the daily briefing. All enriched emails appear on the email page, but sorted by score.

### Integration Points

**Daily Briefing (`services/dashboard.rs`):**
- Replace mechanical "replies needed" with score-ranked emails
- Section label: "Worth Your Attention" (not "Replies Needed")
- Top 3 emails by score, minimum threshold 0.15
- Each item shows contextual summary (primary) + entity name + score rationale as metadata

**Email Page (`services/emails.rs`):**
- Score all active emails, sort by score descending
- Email page groups: "Worth Your Attention" (score > 0.40), "Monitoring" (0.15 - 0.40), "Low Signal" (< 0.15)
- Intelligence hierarchy preserved: contextual summary is the content, subject is metadata

**Signal Compounding (`signals/email_bridge.rs`):**
- Existing email signals (sentiment, urgency, commitment) feed into entity signal graph
- Scorer reads those compounded signals back when scoring the next batch of emails
- This creates a feedback loop: email signals strengthen entity intelligence, which strengthens future email scoring

### What's Reused (Not Built)

| Component | File | How It's Used |
|-----------|------|--------------|
| Signal bus | `signals/bus.rs` | Read entity signals for compound scoring |
| Bayesian fusion | `signals/fusion.rs` | Fuse multiple entity signals into confidence |
| Embedding relevance | `signals/relevance.rs` | Cosine similarity to today's meetings |
| Time decay | `signals/decay.rs` | Freshness weighting |
| Entity name resolution | `helpers.rs` | `resolve_entity_name` for display |
| Callout generation | `signals/callouts.rs` | Pattern for "query signals → score → rank → surface" |
| Focus prioritization | `focus_prioritization.rs` | Pattern for multi-factor scoring with reasons |

## Acceptance Criteria

Verified with real Gmail data in the running app after a full `pnpm dev` restart and at least one enrichment cycle.

1. **Scoring function exists and produces real scores.** Query: `SELECT email_id, relevance_score FROM emails WHERE relevance_score IS NOT NULL` returns rows. Scores range from 0.0 to 1.0. Emails from known contacts at known accounts score higher than emails from unknown senders. Calendar notifications score below 0.10.

2. **Daily briefing shows scored emails, not mechanical "replies needed."** The briefing email section is labeled "Worth Your Attention" (not "Replies Needed"). It shows the top 3 emails by relevance score, minimum threshold 0.15. Each item displays the contextual summary as primary content and includes the resolved entity name. No calendar notifications, no noreply addresses, no newsletters appear in this section.

3. **Email page is sorted by relevance score.** Open the email page. The first email listed is the highest-scored. A renewal discussion from a key account appears above an internal newsletter. The score determines order, not the mechanical high/medium/low classification.

4. **Entity linkage affects score.** An email from `jack@acme.com` where Acme is a tracked account with active signals scores higher than an email from `random@unknown.com`. Verify by comparing `relevance_score` for entity-linked vs unlinked emails in the DB.

5. **Meeting relevance affects score.** When you have a meeting with Acme today, emails about Acme score higher than emails about unrelated topics. Verify: emails whose contextual summary mentions an entity with a meeting today have higher `relevance_score` than emails about entities with no meetings today.

6. **Noise emails score near zero.** Calendar notifications (`Accepted:`, `Declined:`), `noreply@` senders, and `comment-reply@` addresses score below 0.10. They still appear at the bottom of the email page but never surface on the daily briefing.

7. **Scoring runs as part of the enrichment pipeline.** After email enrichment completes, scores are computed and persisted to the `emails.relevance_score` column. No separate manual trigger needed. Verify: after a poll cycle, newly enriched emails have non-null `relevance_score`.

8. **Score rationale is human-readable.** Each scored email has a `score_reason` field explaining why it scored the way it did. Example: "Acme account (entity match) + renewal keyword + meeting today + high urgency." This appears in the email metadata on the email page.

## Dependencies

- Depends on I367 (mandatory enrichment) — scoring consumes enrichment output
- Depends on I368 (DB persistence) — scores are persisted to `emails` table
- Depends on I372 (signal compounding) — entity signals feed into scoring
- Uses existing signal bus, fusion, relevance, decay infrastructure — no new infra needed

## Notes / Rationale

The thesis of v0.13.1 is "email as intelligence input." Without scoring, enrichment is intelligence that goes unused — the AI knows that an email is about a renewal, but the surfacing logic treats it the same as a newsletter. Scoring closes the loop: enrichment produces intelligence, scoring consumes it to decide what matters, and the user sees only what's worth their attention.

The scorer is deliberately designed as a shared service (`signals/scoring.rs`) so that the same infrastructure can score actions, meetings, and future item types. The email-specific adapter (`signals/email_scoring.rs`) maps email data into the general scoring context. This avoids building a one-off email scorer that can't be reused.
