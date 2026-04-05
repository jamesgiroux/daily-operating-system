# I418 — Weekly Impact Report — Personal Operational Look-Back

**Status:** Open
**Priority:** P1
**Version:** 0.15.0
**Area:** Backend / Reports

---

## Summary

A weekly report generated from the user entity's perspective — not about account health, but about what the user accomplished. Auto-generates every Monday using the previous 7 days of data. Lives on the `/me` page under a "My Impact" section alongside the declared context sections. Uses the same `reports` table and `generate_report` infrastructure from I397. Input: the user's annual/quarterly priorities (from `user_entity`) + the last 7 days of `signal_events`, `meetings_history`, and `entity_intel` changes. Output: a first-person operational look-back framed through the user's own priorities.

---

## Report Sections

1. **Priorities moved** — for each annual/quarterly priority the user declared, did anything happen this week that advances it? Concrete evidence from signals and meetings, not generic statements. If a priority is linked to an account entity, check that account's signal activity.
2. **Wins this week** — the specific things that went well: commitments received, positive signals, accounts that improved health, meetings that advanced something. Pulled from `value_delivered` signals, positive sentiment, completed commitments.
3. **What you did** — volume: meetings attended, signals processed, accounts touched, actions completed. Not the quality assessment — just the activity picture.
4. **Watch** — one to three items that need attention: accounts that were silent on a declared priority, open commitments that are aging, signals that didn't resolve.
5. **Carry forward** — open quarterly priorities with no signal activity this week. Not a guilt mechanism — just visibility.

---

## Acceptance Criteria

1. `generate_report(user_entity_id, 'weekly_impact')` produces a report covering the last 7 calendar days. The `reports` table stores it under `entity_type = 'user'` and `entity_type_id = 1`. `report_type = 'weekly_impact'`.
2. The report auto-generates every Monday. A scheduler entry (or hygiene scan trigger) checks: does a `weekly_impact` report exist for the current week? If not, enqueues generation. Verify: on Monday, `SELECT generated_at FROM reports WHERE report_type = 'weekly_impact' ORDER BY generated_at DESC LIMIT 1` — timestamp is from this week.
3. **Priorities moved is the quality gate:** At least one item in the Priorities section must reference a real signal, meeting ID, or commitment that occurred in the reporting period AND is linked to a declared priority. Generic statements ("you worked on Cox this week") are a failure criterion. Verify by reading `content_json.priorities_moved[0]` — it must contain a `source` field pointing to a real event ID.
4. The report renders on the `/me` page under a "My Impact" section — a new chapter below the declared context sections. It shows the most recent weekly report with a "Last week" label and a "View previous" affordance for older reports.
5. All text sections are editable inline (draft state only — not persisted). A "Share as PDF" export produces a clean single-page personal summary, under 1MB. No external DailyOS terminology in the export (no "signal_events", "entity_intel").
6. If the user has no declared priorities (both `annual_priorities` and `quarterly_priorities` are empty), the Priorities moved section shows a prompt: "Add priorities on your profile to see how your week connected to what matters." It does not hallucinate priority alignment.
7. Invalidation: the weekly report is invalidated (`is_stale = 1`) if new signals arrive for entities linked to the user's priorities after generation. This is rare but ensures the report reflects the full week if late-arriving data lands before Monday's next generation.

---

## Design Decisions

1. **Week boundary** — Generation runs on Monday covering the prior Monday through Sunday (7 calendar days). The scheduler checks on Monday: does a `weekly_impact` report exist with `generated_at` in the current week? If not, enqueue generation covering the prior Mon-Sun period.

2. **Post-generation invalidation** — If new signals arrive for entities linked to the user's priorities after the weekly report was generated (but still within the same week), mark the report `is_stale = 1`. Implementation: the signal bus propagation engine checks if the affected entity_id matches any priority-linked entity for the current week's report. This is an edge case — most weeks the report generates once and stays fresh.

## Dependencies

- **Blocked by I397** — report infrastructure (`reports` table, `generate_report` command) must exist.
- **Blocked by I411** — user entity priorities must exist and be populated before this report can do priority-linked analysis.
- **Benefits significantly from I414** — signal scoring weights make priority-linked signals easy to identify; report quality degrades gracefully without it.
