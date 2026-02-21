# ADR-0085: Email as Intelligence Input

**Date:** 2026-02-21
**Status:** Accepted
**Supersedes:** ADR-0024 (email = AI triage, not email client), ADR-0029 (three-tier email priority)

## Context

ADR-0024 correctly established that DailyOS is not an email client. ADR-0029 built a three-tier classification system (high/medium/low) to sort emails by priority. Both decisions were sound for their stage of the product but they framed email as a **display surface** — something DailyOS shows to the user, just sorted more intelligently than Gmail.

This framing is incomplete. A user can see their raw emails in Gmail. Showing them the same emails sorted into three buckets, even with an optional AI summary, is not a meaningful product. What makes DailyOS valuable is the intelligence extracted from email and how it compounds with other signals — meeting context, entity history, relationship patterns — to produce contextual understanding the user can act on.

The chief of staff analogy: A great EA doesn't hand their executive a sorted stack of mail. They read the mail, connect it to what's happening in the business, and deliver a distillation: "Jack is confirming the Acme EBR agenda for Thursday. This aligns with the renewal discussion you had Tuesday. No new action items, but the tone suggests they're leaning positive on the expansion."

## Decision

**Email is an intelligence input, not a display surface.** Every email that enters DailyOS is AI-processed, entity-resolved, and contextually synthesized as a foundational system behavior — not an optional enrichment phase.

### Principles

1. **AI enrichment is mandatory, not optional.** Mechanical classification (rules-based priority sorting) exists only as a bootstrap for the initial Gmail sync. From the second poll cycle onward, every email is AI-processed. There is no "raw email" steady state. Feature flags that gate AI enrichment (like `semanticEmailReclass`) are removed — enrichment is always on.

2. **Entity resolution is the primary classification axis.** Priority tiers (high/medium/low) remain as a secondary signal for urgency, but the primary organization of email is by entity context: which account, project, or person does this email relate to? An email that can't be resolved to an entity is inherently lower value than one connected to an upcoming meeting with a known account.

3. **Contextual synthesis, not raw summaries.** An email summary that says "Jack sent a message about the EBR" is mechanical. A contextual synthesis says "Jack is confirming the Acme EBR agenda. This aligns with the renewal discussion from Tuesday. No new action items, but monitor tone." The difference is whether the system knows what it already knows about the entity and uses that knowledge to interpret the email.

4. **Email signals compound with entity intelligence.** When an email arrives, its extracted signals (commitment, sentiment, urgency, topic) should flow into the entity's signal graph. An email about Acme should update Acme's intelligence — not exist as a standalone data point. The email bridge to meetings (already partially built) is the right pattern; it needs to extend to all entity types, not just upcoming meetings.

5. **Inbox state is the source of truth.** DailyOS should reflect the user's actual Gmail inbox. If an email is archived in Gmail, it should disappear from DailyOS. If it's been read but not archived, it's still relevant. The Gmail query must be `in:inbox` (actual inbox state), not `is:unread newer_than:1d` (a narrow snapshot of recent unread mail).

6. **Email metadata is durable.** Email intelligence persists in the database, not in ephemeral JSON files. This enables: cross-day thread tracking, entity email history, enrichment state management, and consistent data for all frontend surfaces.

### What DailyOS shows

When a user sees an email in DailyOS, they see it because:
- It relates to an upcoming meeting with a known entity, OR
- It contains a signal relevant to an entity they track (commitment, risk, sentiment shift), OR
- It requires their response (ball in their court)

What they see is not "you have an email from Jack." It's a contextual distillation: what the email is about, which entity it relates to, what it means in context of what's already known, and whether action is needed. The three-tier priority system remains as urgency signaling but the primary value is the intelligence synthesis.

### What DailyOS does NOT do

- Replace the user's email client for reading or replying to emails
- Show a comprehensive list of all emails (that's what Gmail does)
- Store email bodies (only metadata and extracted intelligence)
- Send emails on behalf of the user

## Consequences

- AI enrichment cost increases (every email is processed, not just high-priority ones). Mitigated by using extraction-tier models and caching entity context.
- Enrichment latency becomes visible — emails appear in mechanical state briefly before synthesis completes. The UI must handle this transition gracefully (progressive enhancement, not loading spinners).
- Entity resolution quality directly affects email intelligence quality. Emails that can't be resolved to entities get generic summaries. This creates pressure to improve entity resolution (good pressure).
- The email pipeline becomes a critical path, not a nice-to-have. Enrichment failures must be retried, not silently dropped.
- I357 (semantic email reclassification as opt-in) is absorbed — reclassification is part of the mandatory enrichment pipeline, not a feature flag.
- ADR-0024's "reviewable archive manifest" for auto-archived emails remains valid but the emphasis shifts: the manifest shows what intelligence was extracted before archiving, not just what was archived.
- ADR-0029's three-tier system remains as urgency classification but is no longer the primary organizing principle for email. Entity context is primary; urgency is secondary.
