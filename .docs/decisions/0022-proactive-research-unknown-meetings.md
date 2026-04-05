# ADR-0022: Proactive research for unknown meetings

**Date:** 2026-02
**Status:** Accepted

## Context

External meetings with unknown attendees or companies require context. The user shouldn't have to manually research who they're meeting with.

## Decision

The system searches local archive + web for context on unknown external meetings. Per the prime directive: "The system operates. You leverage."

## Consequences

- User opens prep and sees company context even for first-time meetings
- Requires web search capability during Phase 2 enrichment
- May surface inaccurate information for common company names — AI should note confidence level
- Rejected: Ask user to fill in gaps (violates zero-guilt), skip unknown meetings (missed value)
