# ADR-0029: Three-tier email priority with AI-enriched context

**Date:** 2026-02
**Status:** Proposed

## Context

`prepare_today.py` classifies emails into 3 tiers (high/medium/low) but `deliver_today.py` had collapsed to 2 (high/normal). Medium-priority emails (internal colleagues, meeting-related) were silently buried.

## Decision

Keep three tiers throughout the full pipeline:
- **High:** Full card with AI context (summary, conversation arc, recommended action). Emails needing a response today.
- **Medium:** Compact visible row. "Glance at these when you have time."
- **Low:** Auto-archived with a one-line manifest entry. "These were auto-archived."

Phase 2 enrichment writes structured data back to JSON, not just markdown. User-configurable rules are Phase 4+ (extension point per ADR-0026).

## Consequences

- Users can review medium-priority emails instead of them vanishing
- Three tiers through the stack: Python classification → JSON → Rust types → TypeScript → UI
- Adds a collapsible UI section for medium-priority emails
- Classification rules are currently keyword-based — future: per-sender overrides, custom domains
