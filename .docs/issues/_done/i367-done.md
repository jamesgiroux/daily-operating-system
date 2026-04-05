# I367 — Mandatory Email Enrichment — Every Email AI-Processed, Retry on Failure

**Status:** Open (0.13.1)
**Priority:** P0
**Version:** 0.13.1
**Area:** Backend / Pipeline

## Summary

AI enrichment of emails was previously gated behind a feature flag (`semanticEmailReclass`) and applied only to a subset of emails. Per ADR-0085, AI enrichment is mandatory — every email that enters DailyOS is processed, entity-resolved, and contextually synthesized. There is no "raw email" steady state. This issue removes the feature flag, makes enrichment the default and only path, and adds retry logic for enrichment failures with exponential backoff (max 3 attempts).

This issue also absorbs I357 (semantic email reclassification) — reclassification is part of mandatory enrichment, not a separate opt-in feature.

## Acceptance Criteria

From the v0.13.1 brief, verified with real Gmail data in the running app:

1. Connect Gmail for the first time. The initial fetch produces mechanically classified emails (bootstrap). Within 2 minutes, AI enrichment runs automatically — verify by checking that email summaries, sentiment, and entity resolution appear without any manual action.
2. Deliberately kill the Claude process during enrichment (simulate failure). On the next poll cycle, the failed emails are retried — verify they eventually get enriched.
3. Every email on the email page has either: a contextual summary (enriched) OR a "Processing" indicator (enrichment in progress). No email should show raw sender+subject with no intelligence context in the steady state.
4. The `semanticEmailReclass` feature flag is removed from the codebase. `grep -r "semanticEmailReclass" src-tauri/src/` returns 0 results.
5. Check the enrichment queue: emails in `failed` state are retried with exponential backoff (verify via logs showing retry attempts with increasing delay, max 3 attempts).
6. The enrichment state for each email is tracked: `pending` → `enriching` → `enriched` or `failed`. Verify by querying the emails table.

## Dependencies

- Depends on I368 (persist email metadata to SQLite) — enrichment state must be persisted.
- Foundational for I369 (contextual synthesis) — synthesis is the enrichment content; this issue is the enrichment pipeline infrastructure.
- Foundational for I372 (signal compounding) — signals flow from enrichment output.
- Absorbs I357 (semantic email reclassification).

## Notes / Rationale

ADR-0085 decision 1: "AI enrichment is mandatory, not optional." The `semanticEmailReclass` feature flag was scaffolding from a transitional period. Removing it simplifies the code (one path instead of two) and aligns behavior with product intent. The retry mechanism ensures enrichment failures are transient, not permanent — an email that fails to enrich once will succeed when the Claude process stabilizes.
