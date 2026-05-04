# I359 — Vocabulary-Driven Prompts — Inject All 7 Role Fields into Enrichment + Briefing Prompts

**Status:** Open (Parking Lot)
**Priority:** P2
**Version:** Unscheduled
**Area:** Intelligence

## Summary

Role presets (v0.11.0) define 7 vocabulary fields that configure how DailyOS frames intelligence for a given role (e.g., a CS manager uses "renewal" while a marketing manager uses "campaign ROI"). Currently, some of these fields are injected into AI prompts but not all 7. This issue ensures all 7 role vocabulary fields are injected into every enrichment prompt and briefing prompt — so the intelligence output naturally uses the terminology appropriate for the user's role without the user needing to tune anything.

## Acceptance Criteria

Not yet specified. At minimum: a survey of all AI prompts in the enrichment pipeline identifies which role vocabulary fields are currently injected; any fields that are not injected are added; the resulting intelligence output for a preset that has distinctive vocabulary (e.g., "Agency" preset with campaign terminology) reflects that vocabulary throughout.

## Dependencies

- Depends on the role preset system (v0.11.0, I309-I316).
- Related to I141 (AI content tagging) — tags should also use role vocabulary.

## Notes / Rationale

Parked because the preset vocabulary is partially injected and the marginal improvement from completing all 7 fields is real but not blocking any user-visible workflow. Carried forward from v0.11.0 where it was explicitly noted as remaining work.
