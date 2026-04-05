# I341 — Product Vocabulary — System-Term Strings

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0 (completed after partial work in v0.12.1)
**Area:** UX

## Summary

DailyOS's user-facing copy contained internal system terms leaking into the UI — phrases like "Meeting Intelligence Report", "hasPrep", "Proposed" (for AI-suggested actions), and "AI Suggested" that sound like developer-speak rather than a polished product. This issue audited and replaced those strings with vocabulary appropriate for the editorial, chief-of-staff experience.

Work began in v0.12.1 and completed in v0.13.0.

## Acceptance Criteria

From the v0.13.0 brief, verified by navigating to each location in the running app:

| Violation | Location | Old text | Required text |
|-----------|----------|----------|---------------|
| V-002 | `DashboardEmpty.tsx:126` | "Generate Briefing" | "Prepare my day" |
| V-003 | `MeetingDetailPage.tsx:~575` | "Meeting Intelligence Report" | "Meeting Briefing" |
| V-004 | `MeetingDetailPage.tsx:~411` | "Prep not ready yet" | "Not ready yet" or "Context building" |
| V-008 | `DailyBriefing.tsx:357` + `BriefingMeetingCard.tsx:~508` | "Read full intelligence" | "Read full briefing" |
| V-009 | `ActionsPage.tsx:64` | "proposed" (tab label) | "Suggested" |
| V-010 | `ActionsPage.tsx:~535` | "AI Suggested" | "Suggested" |
| V-032 | All three hero components | "Account Intelligence" / "Project Intelligence" / "Person Intelligence" timestamp labels | Label removed — timestamp only, no label |

## Dependencies

- Partially completed in v0.12.1 (vocabulary pass Phases 1–3).
- Final strings resolved in v0.13.0 alongside I342 surface restructure.

## Notes / Rationale

Vocabulary is a product quality signal. "Meeting Intelligence Report" and "hasPrep" tell the user they're looking at internal scaffolding, not a polished chief-of-staff tool. The vocabulary audit was driven by `VIOLATIONS.md` which tracked every system-term string in the codebase.
