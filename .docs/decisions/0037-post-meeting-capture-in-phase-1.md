# ADR-0037: Post-meeting capture implemented in Phase 1

**Date:** 2026-02-06
**Status:** Accepted
**Supersedes:** [ADR-0016](0016-defer-post-meeting-to-phase-3.md)

## Context

ADR-0016 deferred post-meeting capture to Phase 3, reasoning that calendar integration needed to be stable first for meeting-end detection. During implementation:

1. Calendar hybrid overlay (ADR-0032) shipped in Phase 1, providing reliable meeting timing
2. The capture state machine (`capture.rs`) was built with transcript detection (Otter, Fireflies, Fathom, Read.ai) and configurable fallback prompts
3. `PostMeetingPrompt` is mounted globally; meeting cards show "Outcomes" for past meetings
4. ADR-0023 (post-meeting capture replaces /wrap) was accepted and implemented in the same phase

The prerequisite (stable calendar) was met earlier than expected, so the capture feature followed naturally.

## Decision

Post-meeting capture is part of Phase 1. ADR-0016's deferral no longer applies.

## Consequences

- Users can capture wins, risks, and actions immediately after meetings in MVP
- The feedback loop ADR-0016 identified as a loss ("users can't capture outcomes") is preserved
- I17 (captured outcomes don't resurface in briefings) becomes a higher-priority gap — the capture exists but the loop isn't closed
- Phase 3 scope shrinks — can focus on capture refinement and cross-meeting intelligence
