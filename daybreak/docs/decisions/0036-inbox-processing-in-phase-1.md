# ADR-0036: Inbox processing implemented in Phase 1

**Date:** 2026-02-06
**Status:** Accepted
**Supersedes:** [ADR-0015](0015-defer-inbox-to-phase-2.md)

## Context

ADR-0015 deferred inbox processing to Phase 2 to reduce MVP scope. During Phase 1 implementation, inbox processing was built alongside the core briefing workflow because:

1. The processor pipeline (classifier, router, enrichment hooks) shared infrastructure with governance logic (ADR-0025)
2. File watching (`watcher.rs`) was needed for inbox count display anyway
3. The `InboxBatch` workflow fit naturally into the scheduler alongside `Today` and `Archive`

The feature exists and works: files dropped in `_inbox/` are classified, routed, and enriched via the standard processor pipeline.

## Decision

Inbox processing is part of Phase 1. ADR-0015's deferral no longer applies.

## Consequences

- MVP includes automatic inbox processing — users don't need to manually handle `_inbox/` files
- Risk R3 (file watcher reliability on macOS) is now a Phase 1 concern, not Phase 2
- The processor pipeline is exercised earlier, surfacing integration issues sooner
- Phase 2 scope shrinks — can focus on refinement rather than initial implementation
