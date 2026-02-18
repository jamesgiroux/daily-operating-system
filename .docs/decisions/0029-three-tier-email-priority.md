# ADR-0029: Three-tier email priority with AI-enriched context

**Date:** 2026-02
**Status:** Accepted

## Context

Email triage needs to surface what matters without hiding what doesn't. A two-tier system (high/normal) forced a binary: either an email demands attention or it vanishes. Medium-priority emails (internal colleagues, meeting-related) were silently buried. Low-priority emails (newsletters, GitHub notifications) were mixed in with useful-but-not-urgent messages.

## Decision

Three tiers flow through the entire stack — Python classification, JSON output, Rust types, TypeScript types, UI rendering:

- **Needs Attention** (high): Full card with AI enrichment — summary, recommended action, conversation arc. Emails requiring a response today. Sources: customer domains, account domain matches, urgency keywords in subject.
- **Worth a Look** (medium): Compact visible row with summary. Internal colleagues, meeting-related, and anything that doesn't match high or low signals. This is the default classification.
- **FYI** (low): Collapsed by default, expandable. Newsletters, automated notifications, GitHub. One-line display (sender + subject only, no enrichment).

AI enrichment (Phase 2) writes structured fields back to JSON: `summary`, `recommendedAction`, `conversationArc`, `emailType`. Enrichment is merged by fuzzy subject matching in `deliver_today.py`.

Classification rules are keyword-based in `prepare_today.py:classify_email_priority()`. User-configurable rules are Phase 4+ (extension point per ADR-0026).

## Consequences

- Three tiers are implemented across the full stack: `prepare_today.py` → `emails.json` → Rust `EmailPriority` enum → TypeScript `EmailPriority` type → `EmailsPage.tsx`
- Dashboard shows high-priority emails only (max 3); email page shows all three tiers
- FYI section is collapsed by default — respects zero-guilt (no unread count, no obligation)
- Medium is the fallback default — emails that don't match high or low signals land here
- Schema file (`templates/schemas/emails.schema.json`) must be updated to reflect three tiers and enrichment fields
