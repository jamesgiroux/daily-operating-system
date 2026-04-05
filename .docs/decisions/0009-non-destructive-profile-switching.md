# ADR-0009: Non-destructive profile switching

**Date:** 2026-02
**Status:** Accepted

## Context

Users may switch between CSM and General profiles. What happens to their files?

## Decision

Switching profiles changes classification rules, sidebar items, and card metadata. It does NOT move, delete, or restructure files. PARA directories persist across switches.

## Consequences

- Zero risk of data loss on profile switch
- A CSM's Accounts/ directory still exists when they switch to General — it's just not surfaced in navigation
- Aligns with zero-guilt principle: switching is always safe
