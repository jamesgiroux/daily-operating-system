# ADR-0015: Defer inbox processing to Phase 2

**Date:** 2026-02
**Status:** Superseded by [ADR-0036](0036-inbox-processing-in-phase-1.md)

## Context

Inbox processing (file classification, routing, two-tier with/without AI) adds significant complexity. The core value is passive consumption of the morning briefing.

## Decision

Inbox processing is Phase 2 work. MVP shows the inbox contents but doesn't process them automatically.

## Consequences

- Reduces MVP scope and risk
- Users see `_inbox/` files but must manually process them in MVP
- Phase 2 can take a more considered approach to file watching (see risk R3)
