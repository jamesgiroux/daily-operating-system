# I342 — Surface Restructure Phase 4

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0 (Phases 1–3 in v0.12.1, Phase 4 in v0.13.0)
**Area:** UX / Product

## Summary

A full restructuring of the major surfaces (Daily Briefing, Meeting Detail, Weekly Forecast, Actions page, Entity Detail pages) to match how users actually move through their day — as documented in `.docs/research/i342-surface-redesign.md`. The prior surface structure had accumulated sections that were architectural relics (Lead Story, Review, Priorities, Appendix with 9 sub-sections) rather than a coherent daily workflow. Phase 4 was the largest cleanup: removing the Lead Story pattern, establishing the Day Frame / Schedule / Attention / Finis structure, gutting the Meeting Detail appendix, and reorganizing Actions around meeting context rather than a flat list.

## Acceptance Criteria

From the v0.13.0 brief, verified surface by surface in the running app:

**Daily Briefing**
1. "Lead Story" section does not exist. The schedule section expands the next upcoming meeting inline (1-sentence context + prep status) — no separate featured meeting zone.
2. Section structure top to bottom: Day Frame → Schedule → Attention → Finis. No "Review" section. No "Priorities" section. No "Later This Week" subsection anywhere.
3. Day Frame contains: AI narrative line, capacity (hours free + meeting count), focus directive. Hero and Focus are not two separate sections.
4. Attention contains: 2-3 actions (meeting-relevant or overdue), proposed action triage (high-signal only), 3-4 urgent emails. Nothing else.
5. Key People list, Prep Grid, and entity chips do not appear on the daily briefing.

**Meeting Detail**
6. Deep Dive / Act III does not exist. Recent Wins, Open Items, Email Signals, Strategic Programs, Current State, Key Principles, Full Context, Questions to Surface — none of these sections render anywhere on the page.
7. The Appendix (all 9 sub-sections) does not exist on meeting detail. Entity-level content links to entity detail, not embedded here.
8. Only one FinisMarker renders — after Your Plan.
9. Prefill affordance is inside the Your Plan section, not the folio bar.
10. Transcript attachment CTA is in the page body, not duplicated in the folio bar.
11. "Before This Meeting" is a single merged list (readiness items + tracked actions) — not two separate sources rendered separately.

**Weekly Forecast**
12. "Open Time" / deep work blocks section does not exist.
13. "Commitments" chapter does not exist.
14. Prefill and Draft Agenda buttons do not appear on the week page.
15. Section structure: Compact week header → This Week density map (The Shape) → Meeting intelligence timeline (±7 days) → Finis. No narrative hero, no "The Three".
16. Past meetings in the timeline show outcomes, follow-ups generated, context seeds. Future meetings show intelligence quality indicator, prep gap, and days until.

**Actions**
17. Primary view organizes by upcoming meetings, not a flat list. Upcoming meetings with associated actions appear as groups ("Acme QBR · Thursday → 3 actions").
18. "Everything else" (actions with no meeting context) appears below meeting-relevant groups.
19. "Waiting" tab does not exist as a tab. "Waiting" exists only as a status badge on individual rows.
20. "All" tab does not exist.
21. Auto-expiry: a pending action that has existed for 30+ days without activity no longer appears in the Pending view.
22. FolioBar date is removed from the Actions page.
23. Search searches by title, account name, and context — not just accountId.

**Entity Detail**
24. Resolution keywords / "Matching Keywords" do not render anywhere on account, project, or person detail pages.
25. "Value Delivered" section does not exist on account detail.
26. "Portfolio Summary" section does not exist on account detail.
27. Company context appears in one location only (appendix), not in both hero and appendix.
28. "Meeting readiness callout" does not appear on entity detail pages — meeting detail owns this.
29. "Build Intelligence" button label is replaced. Acceptable labels: "Check for updates", "Refresh", or similar plain language.
30. Dead code confirmed deleted: `AppSidebar.tsx`, `WatchItem.tsx`, `ActionList.tsx`, `ActionItem.tsx`, `EmailList.tsx`.

**Design system violations (ship-blocking)**
31. V-001: Tooltip claiming "Actions pending for 30+ days are automatically archived" removed (no backend expiry existed).
32. V-007: `status-badge.tsx` color values reference CSS custom properties, no hardcoded hex or rgba values.
33. V-015: `MeetingDetailPage.tsx` inline `style={{}}` attributes migrated to CSS module (target ≤10 remaining instances for truly dynamic values).
34. V-016: `ActionsPage.tsx` "That's everything" footer replaced with `<FinisMarker />`.

## Dependencies

- Depends on I362 (shared meeting card), I363 (timeline data enrichment), I364 (timeline adoption) for the weekly forecast restructure.
- Informed by `.docs/research/i342-surface-redesign.md` (the full research document that drove this work).

## Notes / Rationale

The surface restructure was driven by a recognition that the accumulated sections didn't match how users actually behave. A chief-of-staff doesn't give you a 9-section appendix — they give you what you need before you walk in the room. Each cut in Phase 4 removed something that was architecturally present but not actually serving the user's daily workflow.
