# ADR-0030: Weekly prep generation with daily refresh

**Date:** 2026-02
**Status:** Proposed

## Context

Currently, `/today` generates meeting prep from scratch each morning. `/week` shows a calendar grid but no prep content. Every meeting on the Week page shows "Prep needed."

## Decision

Sunday evening: `/week` generates lightweight prep for all eligible meetings Mon-Fri, cached in `_today/data/week-cache/`.

Each morning: `/today` refreshes cached preps with latest data (new emails, actions) rather than creating from scratch. Week page status updates to "prep_ready."

**Staleness rule:** If cached prep is >48h old and the meeting is tomorrow, force full regeneration.

## Consequences

- User sees "Prep ready" on Wednesday meetings from Monday morning
- Daily briefing runs ~30% faster (refresh vs. create)
- Sunday run is invisible to user (Principle 9)
- Wednesday's prep from Sunday may miss Monday's email thread — the daily refresh mitigates this
- More complex data flow: two sources of prep that need merging
