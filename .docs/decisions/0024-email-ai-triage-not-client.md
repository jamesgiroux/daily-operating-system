# ADR-0024: Email = AI triage, not email client

**Date:** 2026-02
**Status:** Accepted

## Context

Email is a major input to the daily workflow but building an email client is a massive scope expansion.

## Decision

The app shows AI-curated summaries and suggested actions, not raw emails. Morning briefing auto-archives low-priority with a reviewable manifest. The app surfaces intelligence; the user's email client handles the actual replies.

## Consequences

- Dramatically smaller scope than an email client
- Users still need their email client open — DailyOS doesn't replace it, it triages for it
- AI classification errors could hide important emails — mitigated by the reviewable archive manifest
- Rejected: Build email client (scope creep), show all emails (information overload), no auto-archive (manual triage burden)
