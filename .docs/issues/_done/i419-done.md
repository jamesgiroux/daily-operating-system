# I419 — Monthly Wrapped — Celebratory Narrative Impact Report

**Status:** Open
**Priority:** P1
**Version:** 0.15.0
**Area:** Backend / Reports

---

## Summary

The monthly counterpart to the Weekly Impact Report — bigger scope, more narrative, and explicitly celebratory. Think Spotify Wrapped for professional work: a synthesis of the month that helps the user see their contribution clearly, celebrate real wins, and face one honest observation. Auto-generates on the 1st of each month covering the prior month. Shareable as a PDF. The format is narrative-first, not dashboard-first — it tells a story about what the user accomplished, framed by what they declared matters to them.

---

## Report Sections

1. **The month in one sentence** — AI-synthesized headline. "You advanced 4 of 5 quarterly priorities, closed 2 renewals, and deepened executive relationships at 3 key accounts."
2. **Top wins** — the 3–5 most significant things that happened. Specific, named, citable: "Cox B2B: new BU added (annual priority)" not "you did good work." Each win sourced to a real signal or meeting.
3. **Priority progress** — each annual priority shown with a visual indicator of whether it moved this month. Each quarterly priority shown as complete / in-progress / no activity. No synthetic progress percentages — just honest movement or no movement.
4. **The honest miss** — one observation about what didn't move that should have. The account on a declared priority that went quiet. The commitment that aged. This is not a guilt mechanism — it's the coaching insight a good manager would offer. It's one item, stated directly: "Jefferies: listed as a quarterly priority — no meetings or signals this month."
5. **By the numbers** — meetings attended, signals processed, accounts touched this month vs. prior month. Volume tells a story.
6. **Relationships deepened** — the accounts and people where engagement increased this month vs. prior period. Pulled from meeting frequency changes and positive sentiment trends.

---

## Acceptance Criteria

1. `generate_report(user_entity_id, 'monthly_wrapped')` produces a report covering the calendar month immediately prior to generation. `report_type = 'monthly_wrapped'`. Stored in `reports` table under `entity_type = 'user'`.
2. The report auto-generates on the 1st of each month covering the prior month. Verify: on February 1st, `SELECT report_type, json_extract(content_json, '$.period') FROM reports WHERE report_type = 'monthly_wrapped' ORDER BY generated_at DESC LIMIT 1` — period is "January 2026" (or the prior month).
3. **Top wins quality gate:** Each item in `top_wins` must have a `source` field referencing a real meeting ID or signal ID from the reporting month AND must relate to a declared annual or quarterly priority where possible. At least one win must be priority-linked if any priorities were declared. Generic wins without source citations are a failure criterion.
4. **Priority progress is honest:** If a quarterly priority had zero meeting or signal activity during the month, it appears in the "no activity" state — it is NOT omitted to make the report look better. Verify: add a quarterly priority, generate a report for a month with no activity on that priority — the priority appears in the "no activity" group.
5. **The honest miss is present and specific:** The report always includes exactly one honest miss (if any priority had no activity — otherwise the section is omitted). The miss names the specific priority/account/commitment. It is never generic ("you could have done more"). Verify by reading `content_json.honest_miss` — it contains a priority text and account name where relevant.
6. The report renders on the `/me` page in the "My Impact" section, below the most recent Weekly report. Labelled by month ("January 2026"). Up to 12 months of reports are listed; older ones are accessible via archive.
7. **PDF export is polished and personal.** The Monthly Wrapped PDF is designed for personal reflection and potential sharing — not the clinical look of the CS reports. It uses the editorial design system but with a warmer tone: the headline stat is prominent, wins are featured, the honest miss is visually distinct but not harsh. File under 2MB. Verify: export a Wrapped PDF and confirm it reads like a personal achievement summary, not a business intelligence report.
8. The report includes a comparison where history exists: "This month vs. last month: +3 meetings, -2 open commitments, +1 account health improvement." Only shown if a prior month's report exists.
9. `cargo test` passes. Generation handles gracefully when user entity has no declared priorities — still produces a factual summary from activity data with a note that priorities would personalise the framing.

---

## Design Decisions

1. **Relationships deepened** — Uses meeting frequency delta month-over-month for linked people entities. Person A: 2 meetings in December, 4 in January = relationship deepened. Simple, factual, no sentiment analysis. The signal bus already tracks `meeting_attendee` signals per person. If a person entity has a new `relationship_depth` field populated during the month (from enrichment), that's a bonus signal but not required.

2. **Honest miss selection** — Pick the priority with the highest declared importance (annual over quarterly, first-listed over later-listed — the user ordered them intentionally) that had zero activity. If all no-activity priorities are quarterly, pick the first one. Deterministic selection respecting the user's own ranking. No AI selection needed.

3. **Narrative tone** — The prompt should frame the report as a personal achievement summary written by a supportive chief of staff. Celebratory where earned, honest where warranted, never guilt-inducing. Think Spotify Wrapped energy applied to professional accomplishment. The honest miss is coaching, not criticism.

---

## Dependencies

- **Blocked by I397** — report infrastructure must exist.
- **Blocked by I411** — user entity priorities must exist.
- **Build I418 first** — Weekly Impact Report should be built before Monthly Wrapped to validate the personal report framing at a smaller scope before tackling the larger narrative format.
