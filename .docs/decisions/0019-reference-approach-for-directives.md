# ADR-0019: Reference approach for directives

**Date:** 2026-02
**Status:** Accepted

## Context

The directive JSON (`.today-directive.json`) tells Claude what to enrich. It could embed all context inline or reference files for Claude to load selectively.

## Decision

Directive contains file references, not embedded content. Claude loads files selectively during Phase 2 based on the directive's refs. This gives Claude control over depth and keeps the directive small.

## Consequences

- Smaller directive files (~KB not ~MB)
- Claude can choose which files to read deeply vs. skim
- Requires that referenced files exist at enrichment time (they should — Phase 1 just created them)
- Rejected: Embedded context (large directives, fixed depth — Claude can't decide what matters)
