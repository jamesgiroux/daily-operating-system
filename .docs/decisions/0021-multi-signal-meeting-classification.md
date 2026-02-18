# ADR-0021: Multi-signal meeting classification

**Date:** 2026-02
**Status:** Accepted

## Context

Meetings need classification (customer, internal, one-on-one, all-hands, etc.) for prep prioritization and UI treatment. Calendar titles alone are unreliable.

## Decision

Classification uses multiple signals in priority order: attendee count → title keywords → attendee domain cross-reference → internal heuristics. Uses OAuth domain for internal/external detection. All Hands threshold: 50+ attendees.

## Consequences

- More accurate classification than title-only approaches
- Requires Google Calendar OAuth to detect internal vs. external domains
- Classification rules are currently hardcoded — ADR-0026 extension architecture enables profile-specific overrides later
- See `MEETING-TYPES.md` for the full algorithm specification
