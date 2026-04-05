# ADR-0014: MVP scope definition

**Date:** 2026-02
**Status:** Accepted

## Context

Need to define the minimum set of features that proves the core value proposition: "Open the app. Your day is ready."

## Decision

MVP = F1 (Morning Briefing) + F7 (Dashboard) + F6 (System Tray) + F3 (Background Archive). Everything else is deferred.

Explicitly out of MVP:
- File watching / inbox processing (ADR-0015)
- Post-meeting capture (ADR-0016)
- Weekly planning flow
- Preferences UI (ADR-0003)
- Onboarding wizard

## Consequences

- Small, focused scope that can be validated quickly
- Core loop is: scheduler triggers briefing → user reads dashboard → archive cleans up at midnight
- No terminal required for the daily workflow — that's the key differentiator from the CLI
