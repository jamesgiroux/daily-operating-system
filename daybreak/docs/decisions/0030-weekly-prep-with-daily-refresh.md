# ADR-0030: Weekly prep generation with daily refresh

**Date:** 2026-02
**Status:** Proposed

## Context

The daily briefing generates meeting prep from scratch each morning. The Week page shows a calendar grid with prep status badges, but all meetings show "Prep needed" because no prep content is generated for the weekly view.

Significant infrastructure exists: `prepare_week.py` (751 lines) fetches calendar events and classifies meetings. `deliver_week.py` (511 lines) transforms the directive into `week-overview.json`. The scheduler runs the weekly workflow Monday at 5 AM. `WeekPage.tsx` (599 lines) renders the full grid with status badges, action summaries, hygiene alerts, and focus areas. Types are defined across Rust and TypeScript.

The gap: Phase 2 AI enrichment for `/week` doesn't exist yet. All prep statuses default to `prep_needed` because `prepare_week.py` never sets them. No caching layer exists.

## Decision

Sunday evening: `/week` generates lightweight prep for all eligible meetings Mon-Fri, cached in `_today/data/week-cache/`.

Each morning: `/today` refreshes cached preps with latest data (new emails, actions) rather than creating from scratch. Week page status updates to "prep_ready."

**Staleness rule:** If cached prep is >48h old and the meeting is tomorrow, force full regeneration.

## Consequences

- User sees "Prep ready" on Wednesday meetings from Monday morning
- Daily briefing runs faster (refresh vs. create)
- Sunday run is invisible to user (Principle 9)
- Wednesday's prep from Sunday may miss Monday's email thread — daily refresh mitigates this
- More complex data flow: two sources of prep that need merging
- Depends on Phase 2/3 stability — the weekly pipeline skeleton works but AI enrichment hasn't been built
